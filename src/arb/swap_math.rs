use crate::{
    arb::PoolType,
    dex::{meteora, pumpfun, raydium, whirlpool},
    global,
    math::subtract_as_i64,
    util::amount_with_slippage,
    wsol_mint,
};
use anchor_client::solana_sdk::{clock::Clock, pubkey::Pubkey};
use anyhow::Result;
use commons::quote as dlmm_quote;
use std::panic::{AssertUnwindSafe, catch_unwind};

pub fn safe_swap_compute(
    clock: &Clock,
    routes: &[PoolType],
    amount_in: u64,
    mint: &Pubkey,
    adjust_slippage: bool,
) -> Result<i64> {
    match catch_unwind(AssertUnwindSafe(|| {
        swap_compute(&clock, routes, amount_in, mint, adjust_slippage)
    })) {
        Ok(v) => v,
        Err(_) => Ok(0),
    }
}

pub fn swap_compute(
    clock: &Clock,
    routes: &[PoolType],
    amount_in: u64,
    mint: &Pubkey,
    adjust_slippage: bool,
) -> Result<i64> {
    let mut current_amount = amount_in;
    let mut next_token_in = mint;
    let current_timestamp = clock.unix_timestamp as u64;
    let current_slot = clock.slot;
    let slippage_bps = global::get_slippage_bps();

    for route in routes {
        if current_amount <= 0 {
            return Ok(0);
        }

        (current_amount, next_token_in) = match route {
            PoolType::Pump(_, data) => {
                if next_token_in != &wsol_mint() {
                    let sell_quote = pumpfun::quote::sell_base_input_internal(
                        current_amount as u128,
                        0f64,
                        data.reserves.base_amount as u128,
                        data.reserves.quote_amount as u128,
                        20,
                        5,
                        80,
                        data.pool.coin_creator,
                    )?;

                    (sell_quote.min_quote as u64, &data.pool.quote_mint)
                } else {
                    let buy_quote = pumpfun::quote::buy_quote_input_internal(
                        current_amount as u128,
                        0f64,
                        data.reserves.base_amount as u128,
                        data.reserves.quote_amount as u128,
                        20,
                        5,
                        80,
                        data.pool.coin_creator,
                    )?;

                    (buy_quote.base as u64, &data.pool.base_mint)
                }
            }
            PoolType::Meteora(address, data) => {
                let quote = dlmm_quote::quote_exact_in(
                    *address,
                    &data.lb_pair,
                    current_amount,
                    &data.lb_pair.token_y_mint != next_token_in,
                    data.bin_arrays.clone(),
                    None,
                    clock,
                    &data.mint_x_account,
                    &data.mint_y_account,
                )?;

                let token_out_mint = if &data.lb_pair.token_x_mint == next_token_in {
                    &data.lb_pair.token_y_mint
                } else {
                    &data.lb_pair.token_x_mint
                };

                if quote.failed {
                    // println!("Meteora compute failed {}", address);
                    return Ok(0);
                }

                (quote.amount_out, token_out_mint)
            }
            PoolType::MeteoraDammv2(_, data) => {
                let quote = meteora::damm::get_quote(
                    &data.pool_state,
                    current_timestamp,
                    current_slot,
                    current_amount,
                    &data.pool_state.token_a_mint == next_token_in,
                    false,
                )?;

                let token_out_mint = if &data.pool_state.token_a_mint == next_token_in {
                    &data.pool_state.token_b_mint
                } else {
                    &data.pool_state.token_a_mint
                };

                (quote.output_amount, token_out_mint)
            }
            PoolType::RaydiumAmm(_, data) => {
                let (swap_direction, token_out_mint) =
                    if next_token_in == &data.pool_state.coin_mint {
                        (
                            raydium::amm::SwapDirection::Coin2PC,
                            &data.pool_state.pc_mint,
                        )
                    } else {
                        (
                            raydium::amm::SwapDirection::PC2Coin,
                            &data.pool_state.coin_mint,
                        )
                    };

                let quote = raydium::amm::swap_compute(
                    &data.pool_state,
                    &data.vaults,
                    swap_direction,
                    current_amount,
                    true,
                    0,
                )?;

                (quote, token_out_mint)
            }
            PoolType::RaydiumCpmm(_, data) => {
                let (a_to_b, token_out_mint) = if &data.pool_state.token_0_mint == next_token_in {
                    (true, &data.pool_state.token_1_mint)
                } else {
                    (false, &data.pool_state.token_0_mint)
                };

                let quote = raydium::cpmm::swap_calculate(
                    &data.amm_config,
                    &data.pool_state,
                    &data.vaults,
                    current_amount,
                    a_to_b,
                )?;

                (quote.other_amount_threshold, token_out_mint)
            }
            PoolType::RaydiumClmm(_, data) => {
                let (a_to_b, token_out_mint) = if &data.pool_state.token_mint_0 == next_token_in {
                    (true, &data.pool_state.token_mint_1)
                } else {
                    (false, &data.pool_state.token_mint_0)
                };

                let mut tick_clone = if a_to_b {
                    data.right_ticks.clone()
                } else {
                    data.left_ticks.clone()
                };
                let (amount_out, _) =
                    raydium::clmm::swap_util::get_out_put_amount_and_remaining_accounts(
                        current_amount,
                        None,
                        a_to_b,
                        true,
                        0,
                        &data.pool_state,
                        &data.tick_array_bitmap_ext,
                        &mut tick_clone,
                    )
                    .unwrap_or_default();

                (amount_out, token_out_mint)
            }
            PoolType::Whirlpool(_, data) => {
                let (a_to_b, token_out_mint) = if &data.pool_state.token_mint_a == next_token_in {
                    (true, &data.pool_state.token_mint_b)
                } else {
                    (false, &data.pool_state.token_mint_a)
                };

                let tick_arrays = data
                    .tick_data
                    .clone()
                    .map(|(_, tick_array)| Some(tick_array));
                let quote = whirlpool::quote::swap_quote_by_input_token(
                    current_amount,
                    a_to_b,
                    0,
                    data.pool_state.clone(),
                    data.oracle.clone(),
                    tick_arrays,
                    current_timestamp,
                    None,
                    None,
                )
                .unwrap_or_default();

                (quote.token_min_out, token_out_mint)
            }
            PoolType::Vertigo(_, data) => {
                let (amount_out, token_out_mint) = if &data.pool_state.mint_a == next_token_in {
                    let amount_out = data
                        .pool_state
                        .calculate_buy_amount_out(current_amount, current_slot)?;
                    (amount_out, &data.pool_state.mint_b)
                } else {
                    let amount_out = data
                        .pool_state
                        .calculate_sell_amount_in(current_amount, current_slot)?;
                    (amount_out, &data.pool_state.mint_a)
                };

                (amount_out, token_out_mint)
            }
            PoolType::Solfi(_, data) => {
                let (a_to_b, token_out_mint) = if &data.pool_state.mint_a == next_token_in {
                    (true, &data.pool_state.mint_b)
                } else {
                    (false, &data.pool_state.mint_a)
                };

                let amount_out = data.reserves.swap_quote(current_amount, a_to_b);

                (amount_out, token_out_mint)
            }
        };

        if adjust_slippage {
            current_amount = amount_with_slippage(current_amount, slippage_bps, false)?;
        }
    }

    Ok(subtract_as_i64(current_amount, amount_in))
}
