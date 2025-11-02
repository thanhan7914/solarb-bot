use super::tick_array_bitmap_extension::TickArrayBitmapExtension;
use super::*;
use anchor_client::solana_client::nonblocking::rpc_client::RpcClient;
use std::sync::Arc;

pub async fn fetch_pool_state(
    rpc_client: Arc<RpcClient>,
    pool_pubkey: &Pubkey,
) -> Result<PoolState> {
    let account_data = rpc_client
        .get_account_data(pool_pubkey)
        .await
        .map_err(|e| anyhow!("Failed to fetch account data: {}", e))?;

    PoolState::deserialize(&account_data)
}

pub async fn fetch_bitmap_extension_state(
    rpc_client: Arc<RpcClient>,
    bitmap_ext: &Pubkey,
) -> Result<TickArrayBitmapExtension> {
    let account_data = rpc_client
        .get_account_data(bitmap_ext)
        .await
        .map_err(|e| anyhow!("Failed to fetch account data: {}", e))?;

    TickArrayBitmapExtension::deserialize(&account_data)
}
