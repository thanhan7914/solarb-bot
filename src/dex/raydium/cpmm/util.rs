use super::*;
use anchor_client::solana_client::nonblocking::rpc_client::RpcClient;
use std::sync::Arc;
use tokio::join;

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

pub async fn fetch_pool_reserves(
    rpc_client: Arc<RpcClient>,
    pool_state: &PoolState,
) -> Result<PoolReserves> {
    let (vault_0, vault_1) = join!(
        rpc_client.get_account_data(&pool_state.token_0_vault),
        rpc_client.get_account_data(&pool_state.token_1_vault),
    );

    let vault_0_amount = crate::util::parse_token_amount(&vault_0?)?;
    let vault_1_amount = crate::util::parse_token_amount(&vault_1?)?;

    Ok(PoolReserves {
        token_0_vault: pool_state.token_0_vault,
        token_0_amount: vault_0_amount,
        token_1_vault: pool_state.token_1_vault,
        token_1_amount: vault_1_amount,
    })
}

pub async fn fetch_amm_config_state(
    rpc_client: Arc<RpcClient>,
    amm_config_pubkey: &Pubkey,
) -> Result<AmmConfig> {
    let account_data = rpc_client
        .get_account_data(amm_config_pubkey)
        .await
        .map_err(|e| anyhow!("Failed to fetch account data: {}", e))?;

    AmmConfig::deserialize(&account_data)
}
