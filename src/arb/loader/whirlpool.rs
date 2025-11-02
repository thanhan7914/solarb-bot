use super::*;
use crate::dex::whirlpool;

pub struct WhirlpoolLoader;

impl WhirlpoolLoader {
    pub async fn load_whirlpool(
        rpc_client: Arc<RpcClient>,
        pool_address: Pubkey,
    ) -> Result<WhirlpoolData> {
        let pool_state =
            whirlpool::util::fetch_and_deserialize_whirlpool(rpc_client.clone(), &pool_address)
                .await?;
        let oracle =
            whirlpool::util::fetch_and_deserialize_oracle(rpc_client.clone(), &pool_address).await;
        let tick_data: [(Pubkey, whirlpool::state::TickArray); 5] =
            whirlpool::util::fetch_tick_arrays_or_default(rpc_client, pool_address, &pool_state)
                .await?;

        Ok(WhirlpoolData {
            pool_address,
            pool_state,
            oracle,
            tick_data,
        })
    }
}
