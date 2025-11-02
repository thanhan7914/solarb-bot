use super::*;
use crate::{
    global::{self, get_config},
    math,
    pool_index::TokenPoolType,
    dex::pumpfun::quote,
};

pub mod brent_method;
pub mod golden_section;
pub mod ternary_search;

pub fn compute_threshold(first_hop: &Hop, amount_in: u64) -> Option<(u64, u64)> {
    let (final_amount_in, threshold) = match first_hop.pool_type {
        TokenPoolType::PumpAmm => {
            if let Some(pool_type) = first_hop.to_pool_type() {
                match pool_type {
                    PoolType::Pump(_, ref data) => {
                        let buy_quote = quote::buy_quote_input_internal(
                            amount_in as u128,
                            1.0f64,
                            data.reserves.base_amount as u128,
                            data.reserves.quote_amount as u128,
                            20,
                            5,
                            80,
                            data.pool.coin_creator,
                        )
                        .ok()?;
                        (buy_quote.base as u64, amount_in + 1_000_000_000)
                    }
                    _ => (amount_in, 0),
                }
            } else {
                (amount_in, 0)
            }
        }
        _ => (amount_in, 0),
    };

    Some((final_amount_in, threshold))
}

pub fn adjust_amount_in(amount_in: u64) -> u64 {
    let percent = get_config().bot.optimization_amount_percent as u64;
    (amount_in / 100) * percent
}

pub fn profitable_route(
    route: Route,
    clock: &Clock,
    min_amount_in: u64,
    max_amount_in: u64,
    epsilon: u64,
    adjust_slippage: bool,
) -> Option<SwapRoutes> {
    let swap_op = match get_config().bot.optimization_method.as_str() {
        "brent_method" => brent_method::profitable_route(
            route,
            clock,
            min_amount_in,
            max_amount_in,
            epsilon,
            adjust_slippage,
        ),
        "golden_section" => golden_section::profitable_route(
            route,
            clock,
            min_amount_in,
            max_amount_in,
            epsilon,
            adjust_slippage,
        ),
        "ternary" => ternary_search::profitable_route(
            route,
            clock,
            min_amount_in,
            max_amount_in,
            epsilon,
            adjust_slippage,
        ),
        other => {
            eprintln!("Unknown optimization method: {}", other);
            None
        }
    };

    if let Some(swap) = swap_op {
        let mul = math::div_or_zero(math::to_possible_u64(swap.profit), swap.amount_in);
        if mul > 5 && swap.amount_in < 10_000_000 {
            None
        } else {
            Some(swap)
        }
    } else {
        None
    }
}

pub fn find_profitable_route(route: Route, clock: &Clock) -> Option<SwapRoutes> {
    let min_amount_in = 50_000;
    let max_amount_in = 100_000_000_000;
    let epsilon = 100_000;
    let enabled_slippage = global::enabled_slippage();
    profitable_route(
        route,
        clock,
        min_amount_in,
        max_amount_in,
        epsilon,
        enabled_slippage,
    )
}
