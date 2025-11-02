use super::*;
use crate::{arb::SolfiData, dex::solfi::PoolReserves};

pub struct SolfiLoader;

impl SolfiLoader {
    pub fn get_solfi(pool_address: &Pubkey) -> Option<SolfiData> {
        if let Some(AccountDataType::SolfiPool(pool_state)) = global_data::get_account(pool_address)
        {
            let vault_a = pool_state.vault_a;
            let vault_b = pool_state.vault_b;
            let vault_a_amount = get_reserve_amount(&vault_a);
            let vault_b_amount = get_reserve_amount(&vault_b);

            Some(SolfiData {
                pool_address: *pool_address,
                pool_state,
                reserves: PoolReserves {
                    vault_a,
                    vault_a_amount,
                    vault_b,
                    vault_b_amount,
                },
            })
        } else {
            None
        }
    }
}
