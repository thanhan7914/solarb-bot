use anchor_client::solana_client::nonblocking::rpc_client::RpcClient;
use std::sync::Arc;

use crate::dex::raydium::amm::serum::MarketState;

use super::*;

pub async fn fetch_amm_account(
    rpc_client: Arc<RpcClient>,
    amm_address: &Pubkey,
) -> Result<AmmInfo> {
    let account = rpc_client
        .get_account(amm_address)
        .await
        .map_err(|e| anyhow!("Failed to fetch AMM account: {}", e))?;

    AmmInfo::deserialize(&account.data)
}

pub async fn fetch_market_state(
    rpc_client: Arc<RpcClient>,
    market_address: &Pubkey,
) -> Result<MarketState> {
    let account = rpc_client
        .get_account(market_address)
        .await
        .map_err(|e| anyhow!("Failed to fetch AMM account: {}", e))?;

    MarketState::deserialize(&account.data)
}

pub async fn fetch_multiple_amm_accounts(
    rpc_client: Arc<RpcClient>,
    amm_addresses: &[Pubkey],
) -> Result<Vec<Option<AmmInfo>>> {
    let accounts = rpc_client
        .get_multiple_accounts(amm_addresses)
        .await
        .map_err(|e| anyhow!("Failed to fetch accounts: {}", e))?;

    let mut amms = Vec::new();
    for account_opt in accounts {
        match account_opt {
            Some(account) => match AmmInfo::deserialize(&account.data) {
                Ok(amm) => amms.push(Some(amm)),
                Err(_) => amms.push(None),
            },
            None => amms.push(None),
        }
    }

    Ok(amms)
}

pub async fn fetch_vaults(rpc_client: Arc<RpcClient>, amm: &AmmInfo) -> Result<PoolVaults> {
    let accounts = rpc_client
        .get_multiple_accounts(&[amm.token_coin, amm.token_pc])
        .await?
        .into_iter()
        .collect::<Vec<_>>();

    let vault_a_data = &accounts[0].as_ref().unwrap().data;
    let vault_b_data = &accounts[1].as_ref().unwrap().data;

    Ok(PoolVaults {
        coin_vault_amount: crate::util::parse_token_amount(&vault_a_data)?,
        pc_vault_amount: crate::util::parse_token_amount(&vault_b_data)?,
        coin_vault: amm.token_coin,
        pc_vault: amm.token_pc,
    })
}
