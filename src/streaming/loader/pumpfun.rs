use super::*;
use crate::{arb::PumpAmmData, dex::pumpfun::PoolReserves};

pub struct PumpfunLoader;

impl PumpfunLoader {
    pub fn get_pump_amm(pool_address: &Pubkey) -> Option<PumpAmmData> {
        if let Some(AccountDataType::AmmPair(amm_pool)) = global_data::get_account(pool_address) {
            let base_mint = amm_pool.pool_base_token_account;
            let quote_mint = amm_pool.pool_quote_token_account;
            let base_amount = get_reserve_amount(&base_mint);
            let quote_amount = get_reserve_amount(&quote_mint);

            Some(PumpAmmData {
                pool_address: *pool_address,
                pool: amm_pool,
                reserves: PoolReserves {
                    base_amount,
                    quote_amount,
                    base_mint,
                    quote_mint,
                },
            })
        } else {
            None
        }
    }
}
