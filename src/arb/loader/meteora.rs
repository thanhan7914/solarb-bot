use super::*;
use crate::dex::meteora;

pub struct MeteoraLoader;

impl MeteoraLoader {
    pub async fn load_dlmm(
        rpc_client: Arc<RpcClient>,
        pool_address: Pubkey,
    ) -> Result<MeteoraDlmmData> {
        let lb_pair_account = rpc_client.get_account(&pool_address).await?;
        let lb_pair = LbPairAccount::deserialize(&lb_pair_account.data).unwrap().0;

        // 3 bin arrays to left, and right is enough to cover most of the swap, and stay under 1.4m CU constraint.
        // Get 3 bin arrays to the left from the active bin
        let left_bin_array_pubkeys =
            get_bin_array_pubkeys_for_swap(pool_address, &lb_pair, None, true, 3).unwrap();

        // Get 3 bin arrays to the right the from active bin
        let right_bin_array_pubkeys =
            get_bin_array_pubkeys_for_swap(pool_address, &lb_pair, None, false, 3).unwrap();

        let bin_array_pubkeys = left_bin_array_pubkeys
            .into_iter()
            .chain(right_bin_array_pubkeys)
            .collect::<Vec<Pubkey>>();

        let mut all_keys = Vec::with_capacity(2 + bin_array_pubkeys.len());
        all_keys.extend_from_slice(&[lb_pair.token_x_mint, lb_pair.token_y_mint]);
        all_keys.extend_from_slice(&bin_array_pubkeys);

        let accounts = rpc_client.get_multiple_accounts(&all_keys).await?;

        let mut iter = accounts.into_iter();
        let mint_x_account = iter.next().unwrap().unwrap();
        let mint_y_account = iter.next().unwrap().unwrap();

        let bin_arrays = iter
            .zip(bin_array_pubkeys)
            .map(|(account, key)| {
                (
                    key,
                    BinArrayAccount::deserialize(&account.unwrap().data)
                        .unwrap()
                        .0,
                )
            })
            .collect::<HashMap<_, _>>();

        Ok(MeteoraDlmmData {
            pool_address,
            lb_pair,
            mint_x_account,
            mint_y_account,
            bin_arrays,
        })
    }

    pub async fn load_damm(
        rpc_client: Arc<RpcClient>,
        pool_address: Pubkey,
    ) -> Result<MeteoraDammv2Data> {
        let pool_state = meteora::damm::util::fetch_pool_account(rpc_client, &pool_address).await?;

        Ok(MeteoraDammv2Data {
            pool_address,
            pool_state,
        })
    }
}
