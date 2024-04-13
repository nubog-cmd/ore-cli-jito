use ore::TREASURY_ADDRESS;
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{commitment_config::CommitmentConfig, signature::Signer};

use crate::Miner;

impl Miner {
    pub async fn initialize(&self) {
        // Return early if program is initialized
        // PUSH ALL NEW KEYPAIRS
        let signer = self.signer();
        let signer2 = self.signer2();
        let signer3 = self.signer3();
        let signer4 = self.signer4();
        let signer5 = self.signer5();
        let client =
            RpcClient::new_with_commitment(self.cluster.clone(), CommitmentConfig::processed());
        if client.get_account(&TREASURY_ADDRESS).await.is_ok() {
            return;
        }

        // Sign and send transaction.
        let ix1 = ore::instruction::initialize(signer.pubkey());
        let ix2 = ore::instruction::initialize(signer2.pubkey());
        let ix3 = ore::instruction::initialize(signer3.pubkey());
        let ix4 = ore::instruction::initialize(signer4.pubkey());
        let ix5 = ore::instruction::initialize(signer5.pubkey());
        self.send_and_confirm(&[ix1, ix2, ix3, ix4, ix5], false, false)
            .await
            .expect("Transaction failed");
    }
}
