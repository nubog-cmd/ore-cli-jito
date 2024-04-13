use std::str::FromStr;

use solana_client::nonblocking::rpc_client::RpcClient;
use solana_program::pubkey::Pubkey;
use solana_sdk::{commitment_config::CommitmentConfig, signature::Signer};

use crate::Miner;

impl Miner {
    pub async fn balance(&self, address: Option<String>) {
        let signer = &self.signers()[0];
        let address = if let Some(address) = address {
            if let Ok(address) = Pubkey::from_str(&address) {
                address
            } else {
                println!("Invalid address: {:?}", address);
                return;
            }
        } else {
            signer.pubkey()
        };
        let client =
            RpcClient::new_with_commitment(self.cluster.clone(), CommitmentConfig::processed());
        let token_account_address = spl_associated_token_account::get_associated_token_address(
            &address,
            &ore::MINT_ADDRESS,
        );
        match client.get_token_account(&token_account_address).await {
            Ok(token_account) => {
                if let Some(token_account) = token_account {
                    println!("{:} ORE", token_account.token_amount.ui_amount_string);
                } else {
                    println!("Account not found");
                }
            }
            Err(err) => {
                println!("{:?}", err);
            }
        }
    }
}
