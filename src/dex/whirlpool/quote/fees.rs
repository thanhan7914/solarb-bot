use anyhow::Result;
use ethnum::U256;

use super::{
    AMOUNT_EXCEEDS_MAX_U64, ARITHMETIC_OVERFLOW, state::{Whirlpool, Position, tick::Tick}, try_apply_transfer_fee,
    types::fees::CollectFeesQuote, types::token::TransferFee,
};

/// Calculate fees owed for a position
///
/// # Paramters
/// - `whirlpool`: The whirlpool state
/// - `position`: The position state
/// - `tick_lower`: The lower tick state
/// - `tick_upper`: The upper tick state
/// - `transfer_fee_a`: The transfer fee for token A
/// - `transfer_fee_b`: The transfer fee for token B
///
/// # Returns
/// - `CollectFeesQuote`: The fees owed for token A and token B
#[allow(clippy::too_many_arguments)]
pub fn collect_fees_quote(
    whirlpool: Whirlpool,
    position: Position,
    tick_lower: Tick,
    tick_upper: Tick,
    transfer_fee_a: Option<TransferFee>,
    transfer_fee_b: Option<TransferFee>,
) -> Result<CollectFeesQuote> {
    let mut fee_growth_below_a: u128 = tick_lower.fee_growth_outside_a;
    let mut fee_growth_above_a: u128 = tick_upper.fee_growth_outside_a;
    let mut fee_growth_below_b: u128 = tick_lower.fee_growth_outside_b;
    let mut fee_growth_above_b: u128 = tick_upper.fee_growth_outside_b;

    if whirlpool.tick_current_index < position.tick_lower_index {
        fee_growth_below_a = whirlpool
            .fee_growth_global_a
            .wrapping_sub(fee_growth_below_a);
        fee_growth_below_b = whirlpool
            .fee_growth_global_b
            .wrapping_sub(fee_growth_below_b);
    }

    if whirlpool.tick_current_index >= position.tick_upper_index {
        fee_growth_above_a = whirlpool
            .fee_growth_global_a
            .wrapping_sub(fee_growth_above_a);
        fee_growth_above_b = whirlpool
            .fee_growth_global_b
            .wrapping_sub(fee_growth_above_b);
    }

    let fee_growth_inside_a = whirlpool
        .fee_growth_global_a
        .wrapping_sub(fee_growth_below_a)
        .wrapping_sub(fee_growth_above_a);

    let fee_growth_inside_b = whirlpool
        .fee_growth_global_b
        .wrapping_sub(fee_growth_below_b)
        .wrapping_sub(fee_growth_above_b);

    let fee_growth_delta_a = fee_growth_inside_a.wrapping_sub(position.fee_growth_checkpoint_a);

    let fee_growth_delta_b = fee_growth_inside_b.wrapping_sub(position.fee_growth_checkpoint_b);

    let fee_owed_delta_a: U256 = <U256>::from(fee_growth_delta_a)
        .checked_mul(position.liquidity.into())
        .ok_or(ARITHMETIC_OVERFLOW)
        .unwrap()
        >> 64;

    let fee_owed_delta_b: U256 = <U256>::from(fee_growth_delta_b)
        .checked_mul(position.liquidity.into())
        .ok_or(ARITHMETIC_OVERFLOW)
        .unwrap()
        >> 64;

    let fee_owed_delta_a: u64 = fee_owed_delta_a
        .try_into()
        .map_err(|_| AMOUNT_EXCEEDS_MAX_U64)
        .unwrap();
    let fee_owed_delta_b: u64 = fee_owed_delta_b
        .try_into()
        .map_err(|_| AMOUNT_EXCEEDS_MAX_U64)
        .unwrap();

    let withdrawable_fee_a = position.fee_owed_a + fee_owed_delta_a;
    let withdrawable_fee_b = position.fee_owed_b + fee_owed_delta_b;

    let fee_owed_a =
        try_apply_transfer_fee(withdrawable_fee_a, transfer_fee_a.unwrap_or_default()).unwrap();
    let fee_owed_b =
        try_apply_transfer_fee(withdrawable_fee_b, transfer_fee_b.unwrap_or_default()).unwrap();

    Ok(CollectFeesQuote {
        fee_owed_a,
        fee_owed_b,
    })
}
