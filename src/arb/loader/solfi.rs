use super::*;
use crate::dex::solfi;

pub struct SolfiLoader;

impl SolfiLoader {
    pub async fn load_solfi(
        rpc_client: Arc<RpcClient>,
        pool_address: Pubkey,
    ) -> Result<SolfiData> {
        let pool_state =
            solfi::fetch_and_deserialize_pool(rpc_client.clone(), &pool_address).await?;
        let vaults = pool_state.fetch_vaults(rpc_client).await?;

        Ok(SolfiData {
            pool_address: pool_address,
            pool_state: pool_state,
            reserves: vaults,
        })
    }
}
