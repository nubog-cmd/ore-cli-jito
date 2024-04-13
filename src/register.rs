use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{commitment_config::CommitmentConfig, signature::Signer};
use crate::{utils::proof_pubkey, Miner};
use solana_sdk::signer::keypair::Keypair;
impl Miner {
    pub async fn register(&self) {
        // let mut signers_to_use = Vec::new();
        let mut accounts_needed = Vec::new();
        let client = RpcClient::new_with_commitment(self.cluster.clone(), CommitmentConfig::processed());

        let mut signers = Vec::new();

        for signer in self.signers() {
            let address = proof_pubkey(signer.pubkey());
            let pubkey = signer.pubkey();
        
            if client.get_account(&address).await.is_err() {
                accounts_needed.push(signer);
                println!("{} is not registered, pubkey: {}", address, pubkey);
            } else {
                println!("{} already registered, pubkey: {}", address, pubkey);
            }
        }
        

        signers.extend(accounts_needed.iter());

        let ixs: Vec<_> = accounts_needed.iter().map(|signer: &Keypair| {
            ore::instruction::register(signer.pubkey())
        }).collect();

        if ixs.is_empty() {
            println!("No new wallets to register, returning.");
            return;
        } else {
            println!("Registering {} new wallets with {} signers", accounts_needed.len(), signers.len());
        }

        // Sign and send transaction with the appropriate signers.
        let uuid = self.send_and_confirm(&ixs, &signers)
            .await
            .expect("Transaction failed");

        println!("Bundle sent wth uuid {}", uuid);
    }
}