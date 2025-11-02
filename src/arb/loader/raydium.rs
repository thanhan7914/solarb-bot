use super::*;
use crate::dex::raydium;

pub struct RaydiumLoader;

impl RaydiumLoader {
    pub async fn load_amm(
        rpc_client: Arc<RpcClient>,
        pool_address: Pubkey,
    ) -> Result<RaydiumAmmData> {
        let pool = raydium::amm::util::fetch_amm_account(rpc_client.clone(), &pool_address).await?;
        let serum =
            raydium::amm::util::fetch_market_state(rpc_client.clone(), &pool.market).await?;
        let vaults = raydium::amm::util::fetch_vaults(rpc_client, &pool).await?;

        Ok(RaydiumAmmData {
            pool_address: pool_address,
            pool_state: pool,
            market_state: serum,
            vaults,
        })
    }

    pub async fn load_cpmm(
        rpc_client: Arc<RpcClient>,
        pool_address: Pubkey,
    ) -> Result<RaydiumCpmmData> {
        let pool_state =
            raydium::cpmm::util::fetch_pool_state(rpc_client.clone(), &pool_address).await?;
        let pool_reserves =
            raydium::cpmm::util::fetch_pool_reserves(rpc_client.clone(), &pool_state).await?;
        let amm_config =
            raydium::cpmm::util::fetch_amm_config_state(rpc_client, &pool_state.amm_config).await?;

        Ok(RaydiumCpmmData {
            pool_address: pool_address,
            pool_state,
            amm_config,
            vaults: pool_reserves,
        })
    }

    pub async fn load_clmm(
        rpc_client: Arc<RpcClient>,
        pool_address: Pubkey,
    ) -> Result<RaydiumClmmData> {
        let pool_state =
            raydium::clmm::util::fetch_pool_state(rpc_client.clone(), &pool_address).await?;

        let bitmap_ext = raydium::clmm::pda::derive_tick_array_bitmap_extension(&pool_address)?.0;
        let bitmap_state =
            raydium::clmm::util::fetch_bitmap_extension_state(rpc_client.clone(), &bitmap_ext)
                .await?;

        let left_ticks = raydium::clmm::swap_util::load_cur_and_next_five_tick_array(
            rpc_client.clone(),
            pool_address,
            &pool_state,
            &bitmap_state,
            false,
        )
        .await;

        let right_ticks = raydium::clmm::swap_util::load_cur_and_next_five_tick_array(
            rpc_client,
            pool_address,
            &pool_state,
            &bitmap_state,
            true,
        )
        .await;

        Ok(RaydiumClmmData {
            pool_address,
            pool_state: pool_state,
            tick_array_bitmap_ext: bitmap_state,
            left_ticks,
            right_ticks,
        })
    }
}
