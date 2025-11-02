use super::*;
use crate::{
    arb::{RaydiumAmmData, RaydiumClmmData, RaydiumCpmmData},
    dex::raydium::{amm, clmm, cpmm},
};
use std::collections::VecDeque;

pub struct RaydiumLoader;

impl RaydiumLoader {
    pub fn get_amm(pool_address: &Pubkey) -> Option<RaydiumAmmData> {
        if let Some(AccountDataType::RaydiumAmmPool(pool_state)) =
            global_data::get_account(pool_address)
        {
            if let Some(market_state) = get_market_state(&pool_state.market) {
                let coin_vault = pool_state.token_coin;
                let pc_vault = pool_state.token_pc;
                let coin_vault_amount = get_reserve_amount(&coin_vault);
                let pc_vault_amount = get_reserve_amount(&pc_vault);
                let vaults = amm::PoolVaults {
                    coin_vault,
                    coin_vault_amount,
                    pc_vault,
                    pc_vault_amount,
                };

                Some(RaydiumAmmData {
                    pool_address: *pool_address,
                    pool_state,
                    market_state,
                    vaults,
                })
            } else {
                None
            }
        } else {
            None
        }
    }

    pub fn get_cpmm(pool_address: &Pubkey) -> Option<RaydiumCpmmData> {
        if let Some(AccountDataType::RaydiumCpmmPool(pool_state)) =
            global_data::get_account(pool_address)
        {
            let amm_config = match get_amm_config(&pool_state.amm_config) {
                Some(config) => config,
                None => {
                    eprintln!(
                        "[get_cpmm] Failed to get amm_config for pool {:?}",
                        pool_state.amm_config
                    );
                    return None;
                }
            };
            let token_0_vault = pool_state.token_0_vault;
            let token_1_vault = pool_state.token_1_vault;
            let token_0_amount = get_reserve_amount(&token_0_vault);
            let token_1_amount = get_reserve_amount(&token_1_vault);
            let vaults = cpmm::PoolReserves {
                token_0_vault,
                token_0_amount,
                token_1_vault,
                token_1_amount,
            };

            Some(RaydiumCpmmData {
                pool_address: *pool_address,
                pool_state,
                amm_config,
                vaults,
            })
        } else {
            None
        }
    }

    pub fn get_clmm(pool_address: &Pubkey) -> Option<RaydiumClmmData> {
        if let Some(AccountDataType::RaydiumClmmPool(pool_state)) =
            global_data::get_account(pool_address)
        {
            let tick_array_bitmap_ext_op = get_bitmap_ext(pool_address);
            if let Some(tick_array_bitmap_ext) = tick_array_bitmap_ext_op {
                let left_ticks =
                    get_tick_arrays(pool_address, &pool_state, &tick_array_bitmap_ext, false);
                let right_ticks =
                    get_tick_arrays(pool_address, &pool_state, &tick_array_bitmap_ext, true);

                Some(RaydiumClmmData {
                    pool_address: *pool_address,
                    pool_state,
                    tick_array_bitmap_ext,
                    left_ticks,
                    right_ticks,
                })
            } else {
                None
            }
        } else {
            None
        }
    }
}

#[inline]
pub fn get_bitmap_ext(
    pool_address: &Pubkey,
) -> Option<clmm::tick_array_bitmap_extension::TickArrayBitmapExtension> {
    let (bitmap_ext, _) = clmm::pda::derive_tick_array_bitmap_extension(&pool_address).unwrap();
    match global_data::get_account(&bitmap_ext) {
        Some(AccountDataType::RaydiumTickArrayBitmapExt(data)) => Some(data),
        _ => None,
    }
}

#[inline]
fn get_market_state(market: &Pubkey) -> Option<amm::serum::MarketState> {
    match global_data::get_account(market) {
        Some(AccountDataType::RaydiumAmmMakertState(data)) => Some(data),
        _ => None,
    }
}

#[inline]
fn get_amm_config(config: &Pubkey) -> Option<cpmm::AmmConfig> {
    match global_data::get_account(config) {
        Some(AccountDataType::RaydiumCpmmAmmConfig(data)) => Some(data),
        _ => None,
    }
}

#[inline]
fn get_tick_arrays(
    pool_address: &Pubkey,
    pool_state: &clmm::PoolState,
    tick_array_bitmap_ext: &clmm::tick_array_bitmap_extension::TickArrayBitmapExtension,
    a_to_b: bool,
) -> VecDeque<clmm::tick_array::TickArrayState> {
    let tick_pks = clmm::swap_util::get_cur_and_next_five_tick_array(
        *pool_address,
        &pool_state,
        &tick_array_bitmap_ext,
        a_to_b,
    );
    let mut tick_arrays = VecDeque::new();
    for tick_pk in tick_pks {
        if let Some(AccountDataType::RaydiumTickArrayState(tick_array_state)) =
            global_data::get_account(&tick_pk)
        {
            tick_arrays.push_back(tick_array_state);
        }
    }
    tick_arrays
}
