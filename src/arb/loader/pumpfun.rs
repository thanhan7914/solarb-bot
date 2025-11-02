use super::*;

pub struct PumpfunLoader;

impl PumpfunLoader {
    pub async fn load_pumpfun(
        rpc_client: Arc<RpcClient>,
        pool_address: Pubkey,
    ) -> Result<PumpAmmData> {
        let reader = PumpAmmReader::new_with_client(rpc_client)?;
        let pool = reader.read_pool(&pool_address.to_string()).await?;
        let reserves = reader.get_pool_reserves(&pool).await?;
        Ok(PumpAmmData {
            pool_address,
            pool,
            reserves,
        })
    }
}
