use super::Pool;
use anchor_client::{
    solana_client::nonblocking::rpc_client::RpcClient, solana_sdk::pubkey::Pubkey,
};
use anyhow::Result;
use std::sync::Arc;

pub async fn fetch_and_deserialize_pool(
    rpc_client: Arc<RpcClient>,
    pool_address: &Pubkey,
) -> Result<Pool> {
    let account = rpc_client.get_account(pool_address).await?;
    Pool::deserialize(&account.data)
}
