use crate::{global, instructions};
use anchor_client::{
    solana_client::nonblocking::rpc_client::RpcClient,
    solana_sdk::{
        address_lookup_table::{AddressLookupTableAccount, state::AddressLookupTable},
        commitment_config::CommitmentLevel,
        pubkey::Pubkey,
        signature::Keypair,
        signer::Signer,
    },
};
use anyhow::{Result, anyhow};
use futures::future::try_join_all;
use spl_token::solana_program::program_pack::Pack;
use spl_token::state::Account as TokenAccount;
use std::sync::Arc;

pub mod send;

pub async fn get_token_amount(rpc_client: &RpcClient, token_account: &Pubkey) -> Result<u64> {
    let account_info = rpc_client.get_account(&token_account).await?;
    let token_account = TokenAccount::unpack(&account_info.data)
        .map_err(|e| anyhow!("Failed to unpack token account: {}", e))?;

    Ok(token_account.amount)
}

pub async fn get_ata_token_amount(wallet: &Pubkey, mint: &Pubkey) -> Result<u64> {
    let rpc_client = global::get_rpc_client();
    let ata_account = instructions::util::get_associated_token_address(wallet, mint);
    let amount = get_token_amount(&rpc_client, &ata_account).await;
    amount
}

pub async fn get_wsol_amount(wallet: &Pubkey) -> Result<u64> {
    get_ata_token_amount(wallet, &global::WSOL).await
}

pub async fn fetch_alt_accounts(
    alt_pubkeys: &[Pubkey],
) -> Result<Vec<(Pubkey, AddressLookupTableAccount)>> {
    let rpc_client = global::get_rpc_client();
    let alt_future: Vec<_> = alt_pubkeys
        .iter()
        .map(|&alt_pubkey| fetch_alt_account(rpc_client.clone(), alt_pubkey))
        .collect();

    let alt_accounts = try_join_all(alt_future).await.expect("Failed to load ALT");

    if alt_accounts.is_empty() && !alt_pubkeys.is_empty() {
        return Err(anyhow::anyhow!("Failed to load any ALT accounts"));
    }

    let result: Vec<(Pubkey, AddressLookupTableAccount)> = alt_pubkeys
        .iter()
        .cloned()
        .zip(alt_accounts.into_iter())
        .collect();

    Ok(result)
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

pub async fn create_ata_token(mint: &Pubkey) -> Result<Pubkey> {
    let payer = global::get_keypair();
    create_ata_token_with_payer(payer, mint, Some(CommitmentLevel::Processed)).await
}

pub async fn create_ata_token_with_payer(
    payer: Arc<Keypair>,
    mint: &Pubkey,
    preflight_commitment: Option<CommitmentLevel>,
) -> Result<Pubkey> {
    let owner = global::get_pubkey();
    let ata = get_associated_token_address(&owner, mint);
    let rpc = global::get_rpc_client();

    match rpc.get_account(&ata).await {
        std::result::Result::Ok(_) => {}
        Err(_) => {
            println!("ATA not exists. Creating {} - mint {}", ata.to_string(), mint);
            let ix = crate::instructions::token::create_ata_token_instruction(
                &payer.pubkey(),
                &owner,
                mint,
            )?;

            if let Some(_) =
                send::send_transaction_with_payer(payer, &[ix], Some(false), preflight_commitment)
                    .await
                    .ok()
            {
                return Ok(ata);
            } else {
                return Err(anyhow!("Can't create ata {} token", ata));
            }
        }
    }

    Ok(ata)
}

pub async fn check_ata_token(mint: &Pubkey) -> Result<bool> {
    let owner = global::get_pubkey();
    let ata = get_associated_token_address(&owner, mint);
    let rpc = global::get_rpc_client();

    match rpc.get_account(&ata).await {
        std::result::Result::Ok(_) => Ok(true),
        Err(_) => Ok(false),
    }
}

pub fn get_associated_token_address(wallet: &Pubkey, mint: &Pubkey) -> Pubkey {
    let ata_address = spl_associated_token_account::get_associated_token_address(wallet, mint);
    ata_address
}

pub fn get_ata_token_address(wallet: &Pubkey, mint: &Pubkey, program: &Pubkey) -> Pubkey {
    spl_associated_token_account::get_associated_token_address_with_program_id(
        wallet, mint, program,
    )
}
