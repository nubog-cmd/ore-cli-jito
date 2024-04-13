use std::{
    io::{stdout, Write}, sync::{atomic::AtomicBool, Arc, Mutex}, vec
};

use ore::{self, state::Bus, BUS_ADDRESSES, BUS_COUNT, EPOCH_DURATION};
use rand::Rng;
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{
    commitment_config::CommitmentConfig,  keccak::{hashv, Hash as KeccakHash}, signature::{Keypair, Signer},
};
use futures::future::join_all;
use std::thread;

use crate::{
    Miner,
    utils::{get_clock_account, get_proof, get_treasury},
};

// Odds of being selected to submit a reset tx
const RESET_ODDS: u64 = 20;

impl Miner {
    pub async fn mine(&self, threads: u64) {
        // Register, if needed.
        self.register().await;
        let mut rng = rand::thread_rng();

        // why tf
        // stdout.write_all(b"\x1b[2J\x1b[3J\x1b[H").ok();

        for signer in self.signers() {
            println!("{}: Starting to Mine", signer.pubkey());
        }

        // Start mining loop
        'mining_loop:loop {
            // Fetch account states for all 5 keypairs
            let treasury = get_treasury(self.cluster.clone()).await;

            let self_arc = Arc::new(self);
            let vec_signers = self.signers(); 
            let signers: Vec<&Keypair> = vec_signers.iter().collect(); // Convert

            let proofs_futures: Vec<_> = signers.iter()
                .map(|signer| get_proof(self_arc.cluster.clone(), signer.pubkey()))
                .collect();

            let proofs = join_all(proofs_futures).await;

            let proofs_clone = proofs.clone();

            // Commenting out the parallel processing code
            
            let handles: Vec<_> = signers.iter().zip(proofs.into_iter())
                .map(|(signer, proof)| {
                    let signer = signer.insecure_clone();
                    let proof = proof.clone();
                    let difficulty = treasury.difficulty.into();
                    thread::spawn(move || find_next_hash_par(signer, proof.hash.into(), difficulty, threads))
                })
                .collect();

            let results_with_nonces: Vec<_> = handles.into_iter().map(|handle| handle.join().unwrap()).collect();
            

            // Sequential processing
            /* let results_with_nonces: Vec<_> = signers.into_iter().zip(proofs.into_iter())
                .map(|(signer, proof)| {
                    let signer = signer.insecure_clone();
                    let proof = proof.clone();
                    let difficulty = treasury.difficulty.into();
                    find_next_hash_par(signer, proof.hash.into(), difficulty, threads)
                })
                .collect(); */
            let results: Vec<_> = results_with_nonces.iter().map(|(result, _)| result.clone()).collect();
            let nonces: Vec<_> = results_with_nonces.iter().map(|(_, nonce)| nonce.clone()).collect();

            let reward_rate = (treasury.reward_rate as f64) / (10f64.powf(ore::TOKEN_DECIMALS as f64));

            let rewards: Vec<_> = proofs_clone.into_iter()
                .map(|proof| (proof.claimable_rewards as f64) / (10f64.powf(ore::TOKEN_DECIMALS as f64)))
                .collect();
            
            for signer in self.signers() {
                println!("Wallet: {}", signer.pubkey());
            }
            for reward in rewards {
                println!("Claimable: {} ORE", reward);
            }
            println!("Reward rate: {} ORE", reward_rate);
            println!("Enable JitoTip: {}", self.jito_enable);
            println!("JitoTip Fee: {}", self.jito_fee);

            // Escape sequence that clears the screen and the scrollback buffer
            println!("\nMining for valid hashes...");


            loop {
                // Reset epoch, if needed
                let treasury = get_treasury(self.cluster.clone()).await;
                let clock = get_clock_account(self.cluster.clone()).await;
                let threshold = treasury.last_reset_at.saturating_add(EPOCH_DURATION);
                if clock.unix_timestamp.ge(&threshold) {
                    // There are a lot of miners right now, so randomly select into submitting tx
                    if rng.gen_range(0..RESET_ODDS).eq(&0) {
                        println!("Sending epoch reset transaction...");
                        let reset_ixs: Vec<_> = signers.iter().map(|signer| {
                            ore::instruction::reset(signer.pubkey())
                        }).collect();
                        let uuid = self.send_and_confirm(&reset_ixs, &signers).await.ok();
                        println!("Bundle sent wth uuid {:?}\n", uuid);
                    }
                    continue 'mining_loop
                }

                // Submit mine request.
                let bus = self.find_bus_id(treasury.reward_rate).await;
                let bus_rewards = (bus.rewards as f64) / (10f64.powf(ore::TOKEN_DECIMALS as f64));
                println!("Sending on bus {} ({} ORE)", bus.id, bus_rewards);

                let mut mine_ixs = vec![];
                for (i, signer) in signers.iter().enumerate() {
                    mine_ixs.push(ore::instruction::mine(
                        signer.pubkey(),
                        BUS_ADDRESSES[bus.id as usize],
                        results[i].into(),
                        nonces[i],
                    ));
                }
                let ixs = mine_ixs;

                match self
                    .send_and_confirm(&ixs, &signers)
                    .await
                {
                    Ok(sig) => {
                        println!("Bundle sent with uuid {}\n", sig);
                        break;
                    }
                    Err(_err) => {
                        println!("send_and_confirm Error: {}", _err.to_string());

                        if Miner::should_break_loop(&_err.to_string()) {
                            continue 'mining_loop;
                        }
                    }
                }
            }
        }
    }

    async fn find_bus_id(&self, reward_rate: u64) -> Bus {
        let mut rng = rand::thread_rng();
        loop {
            let bus_id = rng.gen_range(0..BUS_COUNT);
            if let Ok(bus) = self.get_bus(bus_id).await {
                if bus.rewards.gt(&reward_rate.saturating_mul(4)) {
                    return bus;
                }
            }
        }
    }

    fn _find_next_hash(signer: Keypair, hash: KeccakHash, difficulty: KeccakHash) -> (KeccakHash, u64) {
        let mut next_hash: KeccakHash;
        let mut nonce = 0u64;
        loop {
            next_hash = hashv(&[
                hash.to_bytes().as_slice(),
                signer.pubkey().to_bytes().as_slice(),
                nonce.to_le_bytes().as_slice(),
            ]);
            if next_hash.le(&difficulty) {
                break;
            } else {
                println!("Invalid hash: {} Nonce: {:?}", next_hash.to_string(), nonce);
            }
            nonce += 1;
        }
        (next_hash, nonce)
    }

    

    pub async fn get_ore_display_balance(&self, signer: Keypair) -> String {
        let client =
            RpcClient::new_with_commitment(self.cluster.clone(), CommitmentConfig::processed());
        let token_account_address = spl_associated_token_account::get_associated_token_address(
            &signer.pubkey(),
            &ore::MINT_ADDRESS,
        );
        match client.get_token_account(&token_account_address).await {
            Ok(token_account) => {
                if let Some(token_account) = token_account {
                    token_account.token_amount.ui_amount_string
                } else {
                    "0.00".to_string()
                }
            }
            Err(_) => "Err".to_string(),
        }
    }
    pub fn should_break_loop(err_msg: &str) -> bool {
        err_msg.contains("custom program error: 0x3")
    }
}

fn find_next_hash_par(
    signer: Keypair,
    hash: KeccakHash,
    difficulty: KeccakHash,
    threads: u64,
) -> (KeccakHash, u64) {
    let found_solution = Arc::new(AtomicBool::new(false));
    let solution = Arc::new(Mutex::<(KeccakHash, u64)>::new((
        KeccakHash::new_from_array([0; 32]),
        0,
    )));
    let pubkey = signer.pubkey();
    let thread_handles: Vec<_> = (0..threads)
        .map(|i| {
            std::thread::spawn({
                let found_solution = found_solution.clone();
                let solution = solution.clone();
                let mut stdout = stdout();
                move || {
                    let n = u64::MAX.saturating_div(threads).saturating_mul(i);
                    let mut next_hash: KeccakHash;
                    let mut nonce: u64 = n;
                    loop {
                        next_hash = hashv(&[
                            hash.to_bytes().as_slice(),
                            pubkey.to_bytes().as_slice(),
                            nonce.to_le_bytes().as_slice(),
                        ]);
                        if nonce % 10_000 == 0 {
                            if found_solution.load(std::sync::atomic::Ordering::Relaxed) {
                                return;
                            }
                            if n == 0 {
                                stdout
                                    .write_all(
                                        format!("\r{}", next_hash.to_string()).as_bytes(),
                                    )
                                    .ok();
                            }
                        }
                        if next_hash.le(&difficulty) {
                            stdout
                                .write_all(format!("\r{}", next_hash.to_string()).as_bytes())
                                .ok();
                            found_solution.store(true, std::sync::atomic::Ordering::Relaxed);
                            let mut w_solution = solution.lock().expect("failed to lock mutex");
                            *w_solution = (next_hash, nonce);
                            return;
                        }
                        nonce += 1;
                    }
                }
            })
        })
        .collect();

    for thread_handle in thread_handles {
        thread_handle.join().unwrap();
    }

    let r_solution = solution.lock().expect("Failed to get lock");
    *r_solution
}