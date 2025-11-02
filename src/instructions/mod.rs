use anchor_client::{
    solana_client::nonblocking::rpc_client::RpcClient,
    solana_client::rpc_config::RpcSendTransactionConfig,
    solana_sdk::{
        address_lookup_table::{AddressLookupTableAccount, state::AddressLookupTable},
        commitment_config::{CommitmentConfig, CommitmentLevel},
        hash::Hash,
        instruction::{AccountMeta, Instruction},
        message::{VersionedMessage, v0},
        pubkey::Pubkey,
        signature::{Keypair, Signature},
        signer::Signer,
        transaction::{Transaction, VersionedTransaction},
    },
};
use anyhow::{Ok, Result};
use bincode::serialize;
use bs58;
use dlmm_interface::BinArray;
use futures::future::try_join_all;
use std::collections::HashMap;
use std::{rc::Rc, sync::Arc};

pub mod cu;
pub use cu::*;
pub mod flashloan;
pub use flashloan::*;
pub mod aggregator;
pub mod token;

pub mod util {
    use super::*;

    pub fn bins_to_remaining_accounts(
        bin_arrays: &HashMap<Pubkey, BinArray>,
        writable: bool,
    ) -> Vec<AccountMeta> {
        let keys: Vec<Pubkey> = bin_arrays.keys().cloned().collect();

        keys.into_iter()
            .map(|k| {
                if writable {
                    AccountMeta::new(k, false) // writable, non-signer
                } else {
                    AccountMeta::new_readonly(k, false) // read-only
                }
            })
            .collect()
    }

    pub fn get_associated_token_address(wallet: &Pubkey, mint: &Pubkey) -> Pubkey {
        let ata_address = spl_associated_token_account::get_associated_token_address(wallet, mint);
        ata_address
    }

    pub async fn get_latest_blockhash(rpc_url: String) -> Result<Hash> {
        let rpc_client = RpcClient::new(rpc_url.to_string());
        Ok(rpc_client.get_latest_blockhash().await?)
    }

    pub async fn fetch_alt_accounts(
        rpc_url: String,
        alt_pubkeys: &[Pubkey],
    ) -> Result<Vec<AddressLookupTableAccount>> {
        let rpc_client = Arc::new(RpcClient::new(rpc_url.to_string()));

        let alt_future: Vec<_> = alt_pubkeys
            .iter()
            .map(|&alt_pubkey| fetch_alt_account(rpc_client.clone(), alt_pubkey))
            .collect();

        let alt_accounts = try_join_all(alt_future).await.expect("Failed to load ALT");

        if alt_accounts.is_empty() && !alt_pubkeys.is_empty() {
            return Err(anyhow::anyhow!("Failed to load any ALT accounts"));
        }

        Ok(alt_accounts)
    }

    pub async fn fetch_alt_account(
        rpc_client: Arc<RpcClient>,
        alt_pubkey: Pubkey,
    ) -> Result<AddressLookupTableAccount> {
        let account_data = rpc_client.get_account(&alt_pubkey).await?;
        let address_lookup_table = AddressLookupTable::deserialize(&account_data.data)?;
        let alt_account = AddressLookupTableAccount {
            key: alt_pubkey,
            addresses: address_lookup_table.addresses.to_vec(),
        };

        Ok(alt_account)
    }

    pub async fn send_transaction_and_confirm(
        rpc_url: String,
        payer: Rc<Keypair>,
        instructions: &[Instruction],
    ) -> Result<Signature> {
        let rpc_client = Arc::new(RpcClient::new(rpc_url.to_string()));
        let (recent, _) = rpc_client
            .get_latest_blockhash_with_commitment(CommitmentConfig::processed())
            .await?;
        let tx = Transaction::new_signed_with_payer(
            instructions,
            Some(&payer.pubkey()),
            &[&*payer],
            recent,
        );
        let signature = rpc_client.send_and_confirm_transaction(&tx).await?;
        Ok(signature)
    }

    pub async fn send_transaction(
        rpc_url: String,
        payer: Arc<Keypair>,
        instructions: &[Instruction],
    ) -> Result<Signature> {
        let rpc_client = Arc::new(RpcClient::new(rpc_url.to_string()));
        let (recent, _) = rpc_client
            .get_latest_blockhash_with_commitment(CommitmentConfig::processed())
            .await?;
        let tx = Transaction::new_signed_with_payer(
            instructions,
            Some(&payer.pubkey()),
            &[&*payer],
            recent,
        );
        let signature = rpc_client
            .send_transaction_with_config(
                &tx,
                RpcSendTransactionConfig {
                    skip_preflight: true,
                    preflight_commitment: Some(CommitmentLevel::Processed),
                    max_retries: Some(3),
                    ..Default::default()
                },
            )
            .await?;
        Ok(signature)
    }

    pub async fn send_transaction_with_alt(
        rpc_url: String,
        payer: Arc<Keypair>,
        instructions: &[Instruction],
        alt_accounts: &[AddressLookupTableAccount],
    ) -> Result<Signature> {
        let rpc_client = Arc::new(RpcClient::new_with_commitment(
            rpc_url.to_string(),
            CommitmentConfig::processed(),
        ));
        let (recent_blockhash, _) = rpc_client
            .get_latest_blockhash_with_commitment(CommitmentConfig::processed())
            .await?;
        // Create v0 message with ALT
        let message = v0::Message::try_compile(
            &payer.pubkey(),
            instructions,
            &alt_accounts,
            recent_blockhash,
        )?;

        // Create versioned transaction
        let versioned_message = VersionedMessage::V0(message);
        let versioned_tx = VersionedTransaction::try_new(versioned_message, &[&*payer])?;

        // Send transaction
        let signature = rpc_client
            .send_transaction_with_config(
                &versioned_tx,
                RpcSendTransactionConfig {
                    skip_preflight: true,
                    preflight_commitment: Some(CommitmentLevel::Processed),
                    max_retries: Some(3),
                    ..Default::default()
                },
            )
            .await?;
        Ok(signature)
    }

    pub async fn create_ata_token(
        rpc_url: String,
        payer: Arc<Keypair>,
        owner: &Pubkey,
        mint: &Pubkey,
    ) -> Result<Pubkey> {
        let ata = get_associated_token_address(owner, mint);
        let rpc = RpcClient::new(rpc_url.to_string());

        match rpc.get_account(&ata).await {
            std::result::Result::Ok(_) => {}
            Err(_) => {
                println!("ATA not exists. Creating {}", ata.to_string());
                let ix = super::token::create_ata_token_instruction(&payer.pubkey(), owner, mint)?;

                send_transaction(rpc_url, payer, &[ix]).await?;
            }
        }

        Ok(ata)
    }

    pub async fn create_transaction_bs58(
        rpc_url: String,
        payer: Arc<Keypair>,
        instructions: &[Instruction],
    ) -> Result<String> {
        let rpc_client = Arc::new(RpcClient::new(rpc_url.to_string()));
        let (recent, _) = rpc_client
            .get_latest_blockhash_with_commitment(CommitmentConfig::processed())
            .await?;
        let transaction = Transaction::new_signed_with_payer(
            instructions,
            Some(&payer.pubkey()),
            &[&*payer],
            recent,
        );
        let bytes = serialize(&transaction)?;

        Ok(bs58::encode(bytes).into_string())
    }

    pub async fn create_versioned_transaction_bs58(
        rpc_url: String,
        payer: Arc<Keypair>,
        instructions: &[Instruction],
        alt_accounts: &[AddressLookupTableAccount],
    ) -> Result<String> {
        let rpc_client = Arc::new(RpcClient::new(rpc_url.to_string()));
        let (recent_blockhash, _) = rpc_client
            .get_latest_blockhash_with_commitment(CommitmentConfig::processed())
            .await?;
        let message = v0::Message::try_compile(
            &payer.pubkey(),
            instructions,
            &alt_accounts,
            recent_blockhash,
        )?;

        let versioned_message = VersionedMessage::V0(message);
        let versioned_tx = VersionedTransaction::try_new(versioned_message, &[&*payer])?;
        let bytes = serialize(&versioned_tx)?;

        Ok(bs58::encode(bytes).into_string())
    }
}
