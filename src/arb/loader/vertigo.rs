use super::*;
use crate::dex::vertigo;

pub struct VertigoLoader;

impl VertigoLoader {
    pub async fn load_vertigo(
        rpc_client: Arc<RpcClient>,
        pool_address: Pubkey,
    ) -> Result<VertigoData> {
        let pool_state =
            vertigo::util::fetch_and_deserialize_pool(rpc_client, &pool_address).await?;

        Ok(VertigoData {
            pool_address,
            pool_state,
        })
    }
}
