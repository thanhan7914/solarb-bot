use std::sync::Arc;

use crate::global;
use anchor_client::{
    solana_client::rpc_config::RpcSendTransactionConfig,
    solana_sdk::{
        address_lookup_table::AddressLookupTableAccount,
        commitment_config::{CommitmentConfig, CommitmentLevel},
        hash::Hash,
        instruction::Instruction,
        message::{VersionedMessage, v0},
        signature::{Keypair, Signature},
        signer::Signer,
        transaction::{Transaction, VersionedTransaction},
    },
};
use anyhow::Result;

pub async fn send_arb_tx(
    blockhash: Hash,
    instructions: &[Instruction],
    alt_accounts: &[AddressLookupTableAccount],
) -> Result<Signature> {
    let payer = global::get_keypair();
    let wallet = global::get_pubkey();
    // Create v0 message with ALT
    let message = v0::Message::try_compile(&wallet, instructions, &alt_accounts, blockhash)?;

    // Create versioned transaction
    let versioned_message = VersionedMessage::V0(message);
    let versioned_tx = VersionedTransaction::try_new(versioned_message, &[&*payer])?;

    // Send transaction
    let rpc = global::get_rpc_client();
    let signature = rpc
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

pub async fn send_transaction(
    instructions: &[Instruction],
    skip_preflight: Option<bool>,
) -> Result<Signature> {
    let payer = global::get_keypair();
    send_transaction_with_payer(payer, instructions, skip_preflight, Some(CommitmentLevel::Processed)).await
}

pub async fn send_transaction_with_payer(
    payer: Arc<Keypair>,
    instructions: &[Instruction],
    skip_preflight: Option<bool>,
    preflight_commitment: Option<CommitmentLevel>
) -> Result<Signature> {
    let rpc_client = global::get_rpc_client();
    let (recent, _) = rpc_client
        .get_latest_blockhash_with_commitment(CommitmentConfig::processed())
        .await?;
    let tx =
        Transaction::new_signed_with_payer(instructions, Some(&payer.pubkey()), &[&*payer], recent);
    let signature = rpc_client
        .send_transaction_with_config(
            &tx,
            RpcSendTransactionConfig {
                skip_preflight: skip_preflight.unwrap_or(true),
                preflight_commitment: preflight_commitment,
                max_retries: Some(3),
                ..Default::default()
            },
        )
        .await?;
    Ok(signature)
}
