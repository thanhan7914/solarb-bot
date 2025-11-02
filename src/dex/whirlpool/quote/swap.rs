use super::{
    AMOUNT_EXCEEDS_MAX_U64, ARITHMETIC_OVERFLOW, CoreError, FeeRateManager,
    INVALID_ADAPTIVE_FEE_INFO, INVALID_SQRT_PRICE_LIMIT_DIRECTION, MAX_SQRT_PRICE, MIN_SQRT_PRICE,
    SQRT_PRICE_LIMIT_OUT_OF_BOUNDS, TickArraySequence, ZERO_TRADABLE_AMOUNT,
    sqrt_price_to_tick_index,
    state::{
        Whirlpool,
        oracle::{AdaptiveFeeInfo, Oracle},
        tick::Tick,
        tick_array::TickArrays,
    },
    tick_index_to_sqrt_price, try_apply_swap_fee, try_apply_transfer_fee, try_get_amount_delta_a,
    try_get_amount_delta_b, try_get_max_amount_with_slippage_tolerance,
    try_get_min_amount_with_slippage_tolerance, try_get_next_sqrt_price_from_a,
    try_get_next_sqrt_price_from_b, try_reverse_apply_swap_fee, try_reverse_apply_transfer_fee,
    types::{
        swap::{ExactInSwapQuote, ExactOutSwapQuote},
        token::TransferFee,
    },
};

/// Computes the exact input or output amount for a swap transaction.
///
/// # Arguments
/// - `token_in`: The input token amount.
/// - `specified_token_a`: If `true`, the input token is token A. Otherwise, it is token B.
/// - `slippage_tolerance`: The slippage tolerance in basis points.
/// - `whirlpool`: The whirlpool state.
/// - `oracle`: The oracle data for the whirlpool.
/// - `tick_arrays`: The tick arrays needed for the swap.
/// - `timestamp`: The timestamp for the swap.
/// - `transfer_fee_a`: The transfer fee for token A.
/// - `transfer_fee_b`: The transfer fee for token B.
///
/// # Returns
/// The exact input or output amount for the swap transaction.
#[allow(clippy::too_many_arguments)]
pub fn swap_quote_by_input_token(
    token_in: u64,
    specified_token_a: bool,
    slippage_tolerance_bps: u16,
    whirlpool: Whirlpool,
    oracle: Option<Oracle>,
    // tick_arrays: TickArrays,
    tick_arrays: [Option<super::state::tick_array::TickArray>; 5],
    timestamp: u64,
    transfer_fee_a: Option<TransferFee>,
    transfer_fee_b: Option<TransferFee>,
) -> Result<ExactInSwapQuote, CoreError> {
    let (transfer_fee_in, transfer_fee_out) = if specified_token_a {
        (transfer_fee_a, transfer_fee_b)
    } else {
        (transfer_fee_b, transfer_fee_a)
    };
    let token_in_after_fee =
        try_apply_transfer_fee(token_in.into(), transfer_fee_in.unwrap_or_default())?;

    // let tick_array_options = tick_arrays.into_array();
    let tick_sequence = TickArraySequence::new(tick_arrays, whirlpool.tick_spacing)?;

    let swap_result = compute_swap(
        token_in_after_fee.into(),
        0,
        whirlpool,
        tick_sequence,
        specified_token_a,
        true,
        timestamp,
        oracle.map(|oracle| oracle.into()),
    )?;

    let (token_in_after_fees, token_est_out_before_fee) = if specified_token_a {
        (swap_result.token_a, swap_result.token_b)
    } else {
        (swap_result.token_b, swap_result.token_a)
    };

    let token_in =
        try_reverse_apply_transfer_fee(token_in_after_fees, transfer_fee_in.unwrap_or_default())?;

    let token_est_out = try_apply_transfer_fee(
        token_est_out_before_fee,
        transfer_fee_out.unwrap_or_default(),
    )?;

    let token_min_out =
        try_get_min_amount_with_slippage_tolerance(token_est_out, slippage_tolerance_bps)?;

    Ok(ExactInSwapQuote {
        token_in,
        token_est_out,
        token_min_out,
        trade_fee: swap_result.trade_fee,
        trade_fee_rate_min: swap_result.applied_fee_rate_min,
        trade_fee_rate_max: swap_result.applied_fee_rate_max,
    })
}

/// Computes the exact input or output amount for a swap transaction.
///
/// # Arguments
/// - `token_out`: The output token amount.
/// - `specified_token_a`: If `true`, the output token is token A. Otherwise, it is token B.
/// - `slippage_tolerance`: The slippage tolerance in basis points.
/// - `whirlpool`: The whirlpool state.
/// - `oracle`: The oracle data for the whirlpool.
/// - `tick_arrays`: The tick arrays needed for the swap.
/// - `timestamp`: The timestamp for the swap.
/// - `transfer_fee_a`: The transfer fee for token A.
/// - `transfer_fee_b`: The transfer fee for token B.
///
/// # Returns
/// The exact input or output amount for the swap transaction.
#[allow(clippy::too_many_arguments)]
pub fn swap_quote_by_output_token(
    token_out: u64,
    specified_token_a: bool,
    slippage_tolerance_bps: u16,
    whirlpool: Whirlpool,
    oracle: Option<Oracle>,
    // tick_arrays: TickArrays,
    tick_arrays: [Option<super::state::tick_array::TickArray>; 5],
    timestamp: u64,
    transfer_fee_a: Option<TransferFee>,
    transfer_fee_b: Option<TransferFee>,
) -> Result<ExactOutSwapQuote, CoreError> {
    let (transfer_fee_in, transfer_fee_out) = if specified_token_a {
        (transfer_fee_b, transfer_fee_a)
    } else {
        (transfer_fee_a, transfer_fee_b)
    };
    let token_out_before_fee =
        try_reverse_apply_transfer_fee(token_out, transfer_fee_out.unwrap_or_default())?;

    // let tick_array_options = tick_arrays.into_array();
    let tick_sequence = TickArraySequence::new(tick_arrays, whirlpool.tick_spacing)?;

    let swap_result = compute_swap(
        token_out_before_fee.into(),
        0,
        whirlpool,
        tick_sequence,
        !specified_token_a,
        false,
        timestamp,
        oracle.map(|oracle| oracle.into()),
    )?;

    let (token_out_before_fee, token_est_in_after_fee) = if specified_token_a {
        (swap_result.token_a, swap_result.token_b)
    } else {
        (swap_result.token_b, swap_result.token_a)
    };

    let token_out =
        try_apply_transfer_fee(token_out_before_fee, transfer_fee_out.unwrap_or_default())?;

    let token_est_in = try_reverse_apply_transfer_fee(
        token_est_in_after_fee,
        transfer_fee_in.unwrap_or_default(),
    )?;

    let token_max_in =
        try_get_max_amount_with_slippage_tolerance(token_est_in, slippage_tolerance_bps)?;

    Ok(ExactOutSwapQuote {
        token_out,
        token_est_in,
        token_max_in,
        trade_fee: swap_result.trade_fee,
        trade_fee_rate_min: swap_result.applied_fee_rate_min,
        trade_fee_rate_max: swap_result.applied_fee_rate_max,
    })
}

pub struct SwapResult {
    pub token_a: u64,
    pub token_b: u64,
    pub trade_fee: u64,
    pub applied_fee_rate_min: u32,
    pub applied_fee_rate_max: u32,
}

/// Computes the amounts of tokens A and B based on the current Whirlpool state and tick sequence.
///
/// # Arguments
/// - `token_amount`: The input or output amount specified for the swap. Must be non-zero.
/// - `sqrt_price_limit`: The price limit for the swap represented as a square root.
///    If set to `0`, it defaults to the minimum or maximum sqrt price based on the direction of the swap.
/// - `whirlpool`: The current state of the Whirlpool AMM, including liquidity, price, and tick information.
/// - `tick_sequence`: A sequence of ticks used to determine price levels during the swap process.
/// - `a_to_b`: Indicates the direction of the swap:
///    - `true`: Swap from token A to token B.
///    - `false`: Swap from token B to token A.
/// - `specified_input`: Determines if the input amount is specified:
///    - `true`: `token_amount` represents the input amount.
///    - `false`: `token_amount` represents the output amount.
/// - `timestamp`: A timestamp used to calculate the adaptive fee rate.
/// - `adaptive_fee_info`: An optional `AdaptiveFeeInfo` struct containing information about the adaptive fee rate.
/// # Returns
/// A `Result` containing a `SwapResult` struct if the swap is successful, or an `ErrorCode` if the computation fails.
/// # Notes
/// - This function doesn't take into account slippage tolerance.
/// - This function doesn't take into account transfer fee extension.
#[allow(clippy::too_many_arguments)]
pub fn compute_swap<const SIZE: usize>(
    token_amount: u64,
    sqrt_price_limit: u128,
    whirlpool: Whirlpool,
    tick_sequence: TickArraySequence<SIZE>,
    a_to_b: bool,
    specified_input: bool,
    timestamp: u64,
    adaptive_fee_info: Option<AdaptiveFeeInfo>,
) -> Result<SwapResult, CoreError> {
    let sqrt_price_limit = if sqrt_price_limit == 0 {
        if a_to_b {
            MIN_SQRT_PRICE
        } else {
            MAX_SQRT_PRICE
        }
    } else {
        sqrt_price_limit
    };

    if !(MIN_SQRT_PRICE..=MAX_SQRT_PRICE).contains(&sqrt_price_limit) {
        return Err(SQRT_PRICE_LIMIT_OUT_OF_BOUNDS);
    }

    if a_to_b && sqrt_price_limit >= whirlpool.sqrt_price
        || !a_to_b && sqrt_price_limit <= whirlpool.sqrt_price
    {
        return Err(INVALID_SQRT_PRICE_LIMIT_DIRECTION);
    }

    if token_amount == 0 {
        return Err(ZERO_TRADABLE_AMOUNT);
    }

    let mut amount_remaining = token_amount;
    let mut amount_calculated = 0u64;
    let mut current_sqrt_price = whirlpool.sqrt_price;
    let mut current_tick_index = whirlpool.tick_current_index;
    let mut current_liquidity = whirlpool.liquidity;
    let mut trade_fee = 0u64;

    let base_fee_rate = whirlpool.fee_rate;
    let mut applied_fee_rate_min: Option<u32> = None;
    let mut applied_fee_rate_max: Option<u32> = None;

    if whirlpool.is_initialized_with_adaptive_fee() != adaptive_fee_info.is_some() {
        return Err(INVALID_ADAPTIVE_FEE_INFO);
    }

    let mut fee_rate_manager = FeeRateManager::new(
        a_to_b,
        whirlpool.tick_current_index, // note:  -1 shift is acceptable
        timestamp,
        base_fee_rate,
        &adaptive_fee_info,
    )
    .unwrap();

    while amount_remaining > 0 && sqrt_price_limit != current_sqrt_price {
        let (next_tick, next_tick_index) = if a_to_b {
            tick_sequence.prev_initialized_tick(current_tick_index)?
        } else {
            tick_sequence.next_initialized_tick(current_tick_index)?
        };
        let next_tick_sqrt_price: u128 = tick_index_to_sqrt_price(next_tick_index.into()).into();
        let target_sqrt_price = if a_to_b {
            next_tick_sqrt_price.max(sqrt_price_limit)
        } else {
            next_tick_sqrt_price.min(sqrt_price_limit)
        };

        loop {
            fee_rate_manager.update_volatility_accumulator();

            let total_fee_rate = fee_rate_manager.get_total_fee_rate();
            applied_fee_rate_min = Some(
                applied_fee_rate_min
                    .unwrap_or(total_fee_rate)
                    .min(total_fee_rate),
            );
            applied_fee_rate_max = Some(
                applied_fee_rate_max
                    .unwrap_or(total_fee_rate)
                    .max(total_fee_rate),
            );

            let (bounded_sqrt_price_target, adaptive_fee_update_skipped) = fee_rate_manager
                .get_bounded_sqrt_price_target(target_sqrt_price, current_liquidity);

            let step_quote = compute_swap_step(
                amount_remaining,
                total_fee_rate,
                current_liquidity,
                current_sqrt_price,
                bounded_sqrt_price_target,
                a_to_b,
                specified_input,
            )?;

            trade_fee += step_quote.fee_amount;

            if specified_input {
                amount_remaining = amount_remaining
                    .checked_sub(step_quote.amount_in)
                    .ok_or(ARITHMETIC_OVERFLOW)?
                    .checked_sub(step_quote.fee_amount)
                    .ok_or(ARITHMETIC_OVERFLOW)?;
                amount_calculated = amount_calculated
                    .checked_add(step_quote.amount_out)
                    .ok_or(ARITHMETIC_OVERFLOW)?;
            } else {
                amount_remaining = amount_remaining
                    .checked_sub(step_quote.amount_out)
                    .ok_or(ARITHMETIC_OVERFLOW)?;
                amount_calculated = amount_calculated
                    .checked_add(step_quote.amount_in)
                    .ok_or(ARITHMETIC_OVERFLOW)?
                    .checked_add(step_quote.fee_amount)
                    .ok_or(ARITHMETIC_OVERFLOW)?;
            }

            if step_quote.next_sqrt_price == next_tick_sqrt_price {
                current_liquidity = get_next_liquidity(current_liquidity, next_tick, a_to_b);
                current_tick_index = if a_to_b {
                    next_tick_index - 1
                } else {
                    next_tick_index
                }
            } else if step_quote.next_sqrt_price != current_sqrt_price {
                current_tick_index =
                    sqrt_price_to_tick_index(step_quote.next_sqrt_price.into()).into();
            }

            current_sqrt_price = step_quote.next_sqrt_price;

            if !adaptive_fee_update_skipped {
                fee_rate_manager.advance_tick_group();
            } else {
                fee_rate_manager.advance_tick_group_after_skip(
                    current_sqrt_price,
                    next_tick_sqrt_price,
                    next_tick_index,
                );
            }

            // do while loop
            if amount_remaining == 0 || current_sqrt_price == target_sqrt_price {
                break;
            }
        }
    }

    let swapped_amount = token_amount - amount_remaining;

    let token_a = if a_to_b == specified_input {
        swapped_amount
    } else {
        amount_calculated
    };
    let token_b = if a_to_b == specified_input {
        amount_calculated
    } else {
        swapped_amount
    };

    fee_rate_manager.update_major_swap_timestamp(
        timestamp,
        whirlpool.sqrt_price,
        current_sqrt_price,
    );

    Ok(SwapResult {
        token_a,
        token_b,
        trade_fee,
        applied_fee_rate_min: applied_fee_rate_min.unwrap_or(base_fee_rate as u32),
        applied_fee_rate_max: applied_fee_rate_max.unwrap_or(base_fee_rate as u32),
    })
}

// Private functions

fn get_next_liquidity(current_liquidity: u128, next_tick: Option<&Tick>, a_to_b: bool) -> u128 {
    let liquidity_net = next_tick.map(|tick| tick.liquidity_net).unwrap_or(0);
    let liquidity_net_unsigned = liquidity_net.unsigned_abs();
    if a_to_b {
        if liquidity_net < 0 {
            current_liquidity + liquidity_net_unsigned
        } else {
            current_liquidity - liquidity_net_unsigned
        }
    } else if liquidity_net < 0 {
        current_liquidity - liquidity_net_unsigned
    } else {
        current_liquidity + liquidity_net_unsigned
    }
}

struct SwapStepQuote {
    amount_in: u64,
    amount_out: u64,
    next_sqrt_price: u128,
    fee_amount: u64,
}

fn compute_swap_step(
    amount_remaining: u64,
    fee_rate: u32,
    current_liquidity: u128,
    current_sqrt_price: u128,
    target_sqrt_price: u128,
    a_to_b: bool,
    specified_input: bool,
) -> Result<SwapStepQuote, CoreError> {
    // Any error that is not AMOUNT_EXCEEDS_MAX_U64 is not recoverable
    let initial_amount_fixed_delta = try_get_amount_fixed_delta(
        current_sqrt_price,
        target_sqrt_price,
        current_liquidity,
        a_to_b,
        specified_input,
    );
    let is_initial_amount_fixed_overflow =
        initial_amount_fixed_delta == Err(AMOUNT_EXCEEDS_MAX_U64);

    let amount_calculated = if specified_input {
        try_apply_swap_fee(amount_remaining.into(), fee_rate)?
    } else {
        amount_remaining
    };

    let next_sqrt_price =
        if !is_initial_amount_fixed_overflow && initial_amount_fixed_delta? <= amount_calculated {
            target_sqrt_price
        } else {
            try_get_next_sqrt_price(
                current_sqrt_price,
                current_liquidity,
                amount_calculated,
                a_to_b,
                specified_input,
            )?
        };

    let is_max_swap = next_sqrt_price == target_sqrt_price;

    let amount_unfixed_delta = try_get_amount_unfixed_delta(
        current_sqrt_price,
        next_sqrt_price,
        current_liquidity,
        a_to_b,
        specified_input,
    )?;

    // If the swap is not at the max, we need to readjust the amount of the fixed token we are using
    let amount_fixed_delta = if !is_max_swap || is_initial_amount_fixed_overflow {
        try_get_amount_fixed_delta(
            current_sqrt_price,
            next_sqrt_price,
            current_liquidity,
            a_to_b,
            specified_input,
        )?
    } else {
        initial_amount_fixed_delta?
    };

    let (amount_in, mut amount_out) = if specified_input {
        (amount_fixed_delta, amount_unfixed_delta)
    } else {
        (amount_unfixed_delta, amount_fixed_delta)
    };

    // Cap output amount if using output
    if !specified_input && amount_out > amount_remaining {
        amount_out = amount_remaining;
    }

    let fee_amount = if specified_input && !is_max_swap {
        amount_remaining - amount_in
    } else {
        let pre_fee_amount = try_reverse_apply_swap_fee(amount_in.into(), fee_rate)?;
        pre_fee_amount - amount_in
    };

    Ok(SwapStepQuote {
        amount_in,
        amount_out,
        next_sqrt_price,
        fee_amount,
    })
}

fn try_get_amount_fixed_delta(
    current_sqrt_price: u128,
    target_sqrt_price: u128,
    current_liquidity: u128,
    a_to_b: bool,
    specified_input: bool,
) -> Result<u64, CoreError> {
    if a_to_b == specified_input {
        try_get_amount_delta_a(
            current_sqrt_price.into(),
            target_sqrt_price.into(),
            current_liquidity.into(),
            specified_input,
        )
    } else {
        try_get_amount_delta_b(
            current_sqrt_price.into(),
            target_sqrt_price.into(),
            current_liquidity.into(),
            specified_input,
        )
    }
}

fn try_get_amount_unfixed_delta(
    current_sqrt_price: u128,
    target_sqrt_price: u128,
    current_liquidity: u128,
    a_to_b: bool,
    specified_input: bool,
) -> Result<u64, CoreError> {
    if specified_input == a_to_b {
        try_get_amount_delta_b(
            current_sqrt_price.into(),
            target_sqrt_price.into(),
            current_liquidity.into(),
            !specified_input,
        )
    } else {
        try_get_amount_delta_a(
            current_sqrt_price.into(),
            target_sqrt_price.into(),
            current_liquidity.into(),
            !specified_input,
        )
    }
}

fn try_get_next_sqrt_price(
    current_sqrt_price: u128,
    current_liquidity: u128,
    amount_calculated: u64,
    a_to_b: bool,
    specified_input: bool,
) -> Result<u128, CoreError> {
    if specified_input == a_to_b {
        try_get_next_sqrt_price_from_a(
            current_sqrt_price.into(),
            current_liquidity.into(),
            amount_calculated.into(),
            specified_input,
        )
        .map(|x| x.into())
    } else {
        try_get_next_sqrt_price_from_b(
            current_sqrt_price.into(),
            current_liquidity.into(),
            amount_calculated.into(),
            specified_input,
        )
        .map(|x| x.into())
    }
}
