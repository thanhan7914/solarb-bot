use super::*;
use crate::arb::{MeteoraDammv2Data, MeteoraDlmmData};
use commons::get_bin_array_pubkeys_for_swap;
use dlmm_interface::{BinArray, LbPair};
use std::collections::HashMap;

pub struct MeteoraLoader;

impl MeteoraLoader {
    pub fn get_dlmm(pool_address: &Pubkey) -> Option<MeteoraDlmmData> {
        if let Some(AccountDataType::DlmmPair(lb_pair)) = global_data::get_account(&pool_address) {
            let mint_x_account = helper::get_account(&lb_pair.token_x_mint).ok()?;
            let mint_y_account = helper::get_account(&lb_pair.token_y_mint).ok()?;

            let bin_array_keys = get_dlmm_bin_array_keys(*pool_address, &lb_pair).ok()?;
            let bin_arrays = get_bin_arrays(&bin_array_keys)?;

            Some(MeteoraDlmmData {
                pool_address: *pool_address,
                lb_pair,
                mint_x_account,
                mint_y_account,
                bin_arrays,
            })
        } else {
            None
        }
    }

    pub fn get_damm(pool_address: &Pubkey) -> Option<MeteoraDammv2Data> {
        if let Some(AccountDataType::Dammv2Pool(pool_state)) =
            global_data::get_account(&pool_address)
        {
            Some(MeteoraDammv2Data {
                pool_address: *pool_address,
                pool_state,
            })
        } else {
            None
        }
    }
}

#[inline]
pub fn get_dlmm_bin_array_keys(address: Pubkey, lb_pair: &LbPair) -> Result<Vec<Pubkey>> {
    let left_bins = get_bin_array_pubkeys_for_swap(address, lb_pair, None, true, 3)?;
    let right_bins = get_bin_array_pubkeys_for_swap(address, lb_pair, None, false, 3)?;

    Ok(util::concat(&left_bins, &right_bins))
}

#[inline]
pub fn get_bin_arrays(pubkeys: &[Pubkey]) -> Option<HashMap<Pubkey, BinArray>> {
    let mut bin_arrays = HashMap::with_capacity(pubkeys.len());

    for pk in pubkeys {
        if let Some(AccountDataType::BinArray(bin_array)) = global_data::get_account(&pk) {
            bin_arrays.insert(*pk, bin_array);
        }
    }

    if !bin_arrays.is_empty() {
        Some(bin_arrays)
    } else {
        None
    }
}
