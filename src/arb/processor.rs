use crate::{
    arb::{
        ProfitableRoute, Route, route::HopVecExt, safe_swap_compute, sender,
        container::RouteContainer,
    },
    global, pool_index,
    streaming::global_data,
    wsol_mint,
};
use anchor_client::solana_sdk::clock::Clock;
use anchor_client::solana_sdk::pubkey::Pubkey;
use anyhow::Result;
use rayon::prelude::*;
use std::{
    panic::{AssertUnwindSafe, catch_unwind},
    sync::Arc,
    thread::{self},
};
use tokio::{
    sync::Semaphore,
    time::{Duration, MissedTickBehavior},
};
use tracing::info;

pub fn send_routes(batch_size: usize) {
    info!("Start thread send routes - batch size {}", batch_size);

    tokio::spawn(async move {
        let mut ticker = tokio::time::interval(Duration::from_millis(1));
        ticker.set_missed_tick_behavior(MissedTickBehavior::Delay);
        let sem = Arc::new(Semaphore::new(batch_size));

        loop {
            ticker.tick().await;

            let len = RouteContainer::count();
            if len == 0 {
                continue;
            }

            let swaps = RouteContainer::drain(batch_size);
            for swap in swaps {
                if let Ok(permit) = sem.clone().try_acquire_owned() {
                    tokio::spawn(async move {
                        let _permit = permit;
                        let _ = sender::do_arb_v2(swap).await;
                    });
                } else {
                    break;
                }
            }
        }
    });
}

fn find_profitable_route(
    clock: &Clock,
    routes: &[Route],
    base_mint: Pubkey,
    amount_in: u64,
    epsilon: f64,
) {
    routes
        .par_iter()
        .filter(|route| route.hops.product() >= epsilon)
        .filter_map(|r| {
            let pools = r.to_vec_owned()?;
            match safe_swap_compute(clock, &pools, amount_in, &base_mint, false) {
                Ok(p) if p > 0 => Some(r),
                _ => None,
            }
        })
        .for_each(|r| {
            let quote_time = tokio::time::Instant::now();
            let min_profit = global::get_minimum_profit();
            let quote_result = catch_unwind(AssertUnwindSafe(|| sender::check_route(r, min_profit)));
            if let Ok(Some(swap)) = quote_result {
                RouteContainer::smart_insert(ProfitableRoute {
                    route: swap,
                    quote_time: quote_time,
                    sent_time: tokio::time::Instant::now(),
                });
            }
        });
}

fn find_routes(base_mint: Pubkey, epsilon: f64, delay_ms: u64) {
    loop {
        thread::sleep(std::time::Duration::from_millis(delay_ms));

        let amount_in = 50_000;
        let clock = global_data::get_clock().unwrap();
        let mut routes = pool_index::routes();
        fastrand::shuffle(&mut routes);
        find_profitable_route(&clock, &routes, base_mint, amount_in, epsilon);
    }
}

pub fn find_from_pool(pool_address: Pubkey) {
    tokio::task::spawn_blocking(move || {
        if let Some(pool) = pool_index::get(&pool_address) {
            let mint = if pool.mint_a == wsol_mint() {
                pool.mint_b
            } else {
                pool.mint_a
            };

            let bot_config = &global::get_config().bot;
            let epsilon = 1f64 + bot_config.price_threshold;
            let base_mint = global::get_base_mint().as_ref().clone();
            let amount_in = 50_000;
            let clock = global_data::get_clock().unwrap();
            let routes = pool_index::get_routes_by_mint(&mint);
            find_profitable_route(&clock, &routes, base_mint, amount_in, epsilon);
        }
    });
}

pub fn finding(delay_ms: u64) -> Result<()> {
    let bot_config = &global::get_config().bot;
    let routes_batch_size = bot_config.routes_batch_size;
    let epsilon = 1f64 + bot_config.price_threshold;
    let base_mint = global::get_base_mint().as_ref().clone();
    send_routes(routes_batch_size as usize);
    find_routes(base_mint, epsilon, delay_ms);

    Ok(())
}
