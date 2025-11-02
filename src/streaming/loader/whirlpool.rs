use super::*;
use crate::{
    arb::WhirlpoolData,
    dex::whirlpool::{
        self,
        state::{TickArray, oracle::Oracle},
    },
};

pub struct WhirlpoolLoader;

impl WhirlpoolLoader {
    pub fn get_whirlpool(pool_address: &Pubkey) -> Option<WhirlpoolData> {
        if let Some(AccountDataType::Whirlpool(pool_state)) = global_data::get_account(pool_address)
        {
            let oracle = get_oracle(&pool_address);
            let tick_arrays =
                whirlpool::util::get_tick_arrays_or_default(*pool_address, &pool_state).unwrap();
            let ticks = get_tick_arrays(&pool_state, &tick_arrays);
            let tick_data_op: Option<[(Pubkey, TickArray); 5]> = ticks.try_into().ok();
            if let Some(tick_data) = tick_data_op {
                Some(WhirlpoolData {
                    pool_address: *pool_address,
                    pool_state,
                    oracle,
                    tick_data,
                })
            } else {
                println!("Failed to convert tick_arrays data");
                None
            }
        } else {
            None
        }
    }
}

#[inline]
fn get_oracle(pool_address: &Pubkey) -> Option<Oracle> {
    let (oracle_address, _) = whirlpool::state::pda::derive_oracle_address(pool_address).unwrap();
    match global_data::get_account(&oracle_address) {
        Some(AccountDataType::WhirlpoolOracle(oracle)) => Some(oracle),
        _ => None,
    }
}

#[inline]
fn get_tick_arrays(
    whirlpool: &whirlpool::state::Whirlpool,
    pubkeys: &[Pubkey],
) -> Vec<(Pubkey, TickArray)> {
    let mut tick_arrays = Vec::with_capacity(pubkeys.len());
    let tick_array_start_index = whirlpool::get_tick_array_start_tick_index(
        whirlpool.tick_current_index,
        whirlpool.tick_spacing,
    );
    let offset = whirlpool.tick_spacing as i32 * whirlpool::TICK_ARRAY_SIZE as i32;

    let tick_array_indexes = [
        tick_array_start_index,
        tick_array_start_index + offset,
        tick_array_start_index + offset * 2,
        tick_array_start_index - offset,
        tick_array_start_index - offset * 2,
    ];

    let mut index: usize = 0;
    for pk in pubkeys {
        if let Some(AccountDataType::WhirlpoolTickArray(tick_array)) = global_data::get_account(&pk)
        {
            tick_arrays.push((*pk, tick_array));
        } else {
            let tick_array = whirlpool::util::uninitialized_tick_array(tick_array_indexes[index]);
            tick_arrays.push((*pk, tick_array));
        }

        index += 1;
    }

    tick_arrays
}
