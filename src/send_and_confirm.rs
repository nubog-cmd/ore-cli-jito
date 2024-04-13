use std::{
    str,
    sync::Arc,
};
use thiserror::Error;
use anyhow::Result;
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_program::instruction::Instruction;
use solana_sdk::{
    pubkey, commitment_config::CommitmentConfig, hash::Hash, message::{v0, VersionedMessage}, signature::{Keypair, Signer}, system_instruction::transfer, transaction::VersionedTransaction
};
use jito_protos::{
    auth::{auth_service_client::AuthServiceClient, Role},
    bundle::Bundle,
    convert::proto_packet_from_versioned_tx,
    searcher::{
        searcher_service_client::SearcherServiceClient, SendBundleRequest, SendBundleResponse,
    },
};
use tonic::{
    codegen::InterceptedService,
    transport,
    transport::{Channel, Endpoint},
    Response, Status,
};

use crate::token_authenticator::ClientInterceptor;
use crate::Miner;

#[derive(Debug, Error)]
pub enum BlockEngineConnectionError {
    #[error("transport error {0}")]
    TransportError(#[from] transport::Error),
    #[error("client error {0}")]
    ClientError(#[from] Status),
}

pub type BlockEngineConnectionResult<T> = Result<T, BlockEngineConnectionError>;


const CHUNK_SIZE: usize = 5;

impl Miner {
    pub async fn send_and_confirm(
        &self,
        ixs: &[Instruction],
        signers: &[&Keypair],
    ) -> Result<String, Box<dyn std::error::Error>> {
        let feepayer = &self.feepayer();
        let auth: Arc<Keypair> = Arc::new(self.auth());
        let total_chunks = (ixs.len() + CHUNK_SIZE - 1) / CHUNK_SIZE;
        //println!("Total chunks: {}", total_chunks);
        let client = RpcClient::new_with_commitment(self.cluster.to_owned(), CommitmentConfig::processed());
        let (hash, _slot) = client.get_latest_blockhash_with_commitment(CommitmentConfig::confirmed()).await?;
        let mut versioned_txs = Vec::new();

        for (index, (ixs_chunk, signers_chunk)) in ixs.chunks(CHUNK_SIZE).zip(signers.chunks(CHUNK_SIZE)).enumerate() {
            let mut vec_ixs = Vec::from(ixs_chunk);
            let mut vec_signers = signers_chunk.to_vec();

            vec_signers.insert(0, feepayer); // Always include the feepayer to sign FIRSTTT


            //println!("Ixs len: {}", vec_ixs.len());
            //println!("Signing with accts:");
            //for signer in &vec_signers {
                //println!("{}", signer.pubkey()); // Print the public key of each signer
            //}

            if index + 1 == total_chunks {
                // This is the last chunk in the txs
                let jito_tip_ix = transfer(
                    &feepayer.pubkey(),
                    &pubkey!("96gYZGLnJYVFmbjzopPSU6QiEV5fGqZNyN9nmNhvrZU5"),
                    self.jito_fee,
                );
                vec_ixs.push(jito_tip_ix); // Push the jito tip instruction
            }

            // Create a versioned transaction for this chunk with respective signers and ixs
            let versioned_tx = self.create_vtx(
                hash,
                &vec_ixs,
                &vec_signers
            ).await?;

            // Print tx size
            //let serialized: Vec<u8> = bincode::serialize(&versioned_tx).unwrap();
            //println!("Vtxn is {} bytes.", serialized.len());

            versioned_txs.push(versioned_tx);
        }

        // Get searcher client
        let mut searcher_client = self.get_searcher_client(&self.be_url, &auth)
            .await?;

        // Send the bundle of versioned transactions
        let send_response = self.send_bundle_no_wait(&versioned_txs, &mut searcher_client)
            .await?;
        
        Ok(send_response.into_inner().uuid)
    }

    async fn create_vtx(
        &self,
        hash: Hash,
        ixs: &[Instruction],
        payers: &[&Keypair],
    ) -> Result<VersionedTransaction, Box<dyn std::error::Error>> {
        let tx = VersionedTransaction::try_new(
            VersionedMessage::V0(v0::Message::try_compile(
                &self.feepayer().pubkey(),
                ixs,
                &[],
                hash,
            )?),
            payers,
        )?;
    
        Ok(tx)
    }

    async fn get_searcher_client(
        &self,
        block_engine_url: &str,
        auth_keypair: &Arc<Keypair>,
    ) -> BlockEngineConnectionResult<
        SearcherServiceClient<InterceptedService<Channel, ClientInterceptor>>,
    > {
        let auth_channel = self.create_grpc_channel(block_engine_url).await?;
        let client_interceptor = ClientInterceptor::new(
            AuthServiceClient::new(auth_channel),
            auth_keypair,
            Role::Searcher,
        )
        .await?;
    
        let searcher_channel = self.create_grpc_channel(block_engine_url).await?;
        let searcher_client =
            SearcherServiceClient::with_interceptor(searcher_channel, client_interceptor);
        Ok(searcher_client)
    }

    async fn create_grpc_channel(&self, url: &str) -> BlockEngineConnectionResult<Channel> {
        let mut endpoint = Endpoint::from_shared(url.to_string()).expect("invalid url");
        if url.starts_with("https") {
            endpoint = endpoint.tls_config(tonic::transport::ClientTlsConfig::new())?;
        }
        Ok(endpoint.connect().await?)
    }

    pub async fn send_bundle_no_wait(
        &self,
        transactions: &[VersionedTransaction],
        searcher_client: &mut SearcherServiceClient<InterceptedService<Channel, ClientInterceptor>>,
    ) -> Result<Response<SendBundleResponse>, Status> {
        // convert them to packets + send over
        let packets: Vec<_> = transactions
            .iter()
            .map(proto_packet_from_versioned_tx)
            .collect();
    
        searcher_client
            .send_bundle(SendBundleRequest {
                bundle: Some(Bundle {
                    header: None,
                    packets,
                }),
            })
            .await
    }
}