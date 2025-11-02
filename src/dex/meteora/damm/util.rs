use super::*;
use anchor_client::{
    solana_client::nonblocking::rpc_client::RpcClient, solana_sdk::pubkey::Pubkey,
};
use anyhow::{Result, anyhow};
use std::sync::Arc;

pub async fn fetch_pool_account(client: Arc<RpcClient>, pool_address: &Pubkey) -> Result<Pool> {
    let account = client
        .get_account(pool_address)
        .await
        .map_err(|e| anyhow!("Failed to fetch account: {}", e))?;

    Pool::deserialize(&account.data)
}

pub async fn fetch_multiple_pool_accounts(
    client: Arc<RpcClient>,
    pool_addresses: &[Pubkey],
) -> Result<Vec<Option<Pool>>> {
    let accounts = client
        .get_multiple_accounts(pool_addresses)
        .await
        .map_err(|e| anyhow!("Failed to fetch accounts: {}", e))?;

    let mut pools = Vec::new();
    for account_opt in accounts {
        match account_opt {
            Some(account) => match Pool::deserialize(&account.data) {
                Ok(pool) => pools.push(Some(pool)),
                Err(_) => pools.push(None),
            },
            None => pools.push(None),
        }
    }

    Ok(pools)
}
