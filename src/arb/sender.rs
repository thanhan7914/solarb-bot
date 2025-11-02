use super::*;
use crate::arb::ata_worker::AtaWorker;
use crate::polling::blockhash;
use crate::streaming::global_data;
use crate::{default_lta, global, streaming, transaction};
use anchor_client::solana_sdk::{
    address_lookup_table::AddressLookupTableAccount, signature::Signature,
};
use anyhow::Result;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tokio;
use tokio::time::Instant;
use tracing::{error, info, warn};

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
struct ArbitrageKey {
    hash: u64,
    amount_in: u64,
    // profit_range: i64,
}

impl ArbitrageKey {
    fn from_swap_route(swap: &SwapRoutes) -> Self {
        Self {
            hash: swap.to_mint_hash(),
            amount_in: (swap.amount_in / 10_000_000) * 10_000_000,
            // profit_range: (swap.profit / 10_000_000) * 10_000_000,
        }
    }
}

lazy_static::lazy_static! {
    static ref RATE_LIMITER: Arc<Mutex<HashMap<ArbitrageKey, tokio::time::Instant>>> =
        Arc::new(Mutex::new(HashMap::new()));
}

const RATE_LIMIT_DURATION: tokio::time::Duration = tokio::time::Duration::from_secs(60);

fn should_allow_transaction(arb_key: &ArbitrageKey) -> bool {
    let mut rate_limiter = RATE_LIMITER.lock().unwrap();
    let now = tokio::time::Instant::now();

    match rate_limiter.get(arb_key) {
        Some(last_time) => {
            if now.duration_since(*last_time) >= RATE_LIMIT_DURATION {
                rate_limiter.insert(arb_key.clone(), now);
                true
            } else {
                false
            }
        }
        None => {
            rate_limiter.insert(arb_key.clone(), now);
            true
        }
    }
}

fn collect_alt_accounts(swap: &SwapRoutes) -> Option<Vec<AddressLookupTableAccount>> {
    let mut alt_accounts: Vec<AddressLookupTableAccount> =
        Vec::with_capacity(swap.routes.len() + 1);
    if let Some(default_lta_data) = streaming::retrieve_alt_from_alt_pk(&default_lta()) {
        alt_accounts.push(default_lta_data);
    }

    for pool in &swap.routes {
        if let Some(alt_data) = streaming::retrieve_alt(pool.get_address()) {
            alt_accounts.push(alt_data);
        }
    }

    if alt_accounts.len() > 0 {
        Some(alt_accounts)
    } else {
        None
    }
}

#[allow(unreachable_code)]
#[inline]
pub async fn send_arb(swap: SwapRoutes) -> Option<Signature> {
    let blockhash = blockhash::get_current_blockhash().await.unwrap();
    if let Some(alt_accounts) = collect_alt_accounts(&swap) {
        transaction::build_and_send(
            blockhash,
            swap,
            &alt_accounts,
            global::get_base_mint_amount(),
        )
        .await
    } else {
        error!("Can't load ALT");
        None
    }
}

#[allow(unused_variables)]
pub async fn do_arb_v2(profitable_route: ProfitableRoute) -> Result<bool> {
    let swap = profitable_route.route;
    let quote_time = profitable_route.quote_time.elapsed();
    let receive_time = profitable_route.sent_time.elapsed();
    let now = tokio::time::Instant::now();

    if swap.routes.len() < 2 {
        return Ok(false);
    }

    if !AtaWorker::create_mints(&swap.routes) {
        warn!("Creating mints..., skip");
        return Ok(false);
    }

    let arb_key = ArbitrageKey::from_swap_route(&swap);

    if should_allow_transaction(&arb_key) {
        if let Some(signature) = send_arb(swap).await {
            // if true {
            info!(
                "Quote time ({:?} / {:?}) - sent time {:?} - total time {:?}",
                quote_time,
                receive_time,
                now.elapsed(),
                profitable_route.quote_time.elapsed()
            );
            Ok(true)
        } else {
            warn!("Failed to send transaction.");
            Ok(false)
        }
    } else {
        Ok(false)
    }
}

#[allow(unused_variables)]
pub async fn do_arb(swap: SwapRoutes, now: tokio::time::Instant) -> Result<bool> {
    let quote_time = now.elapsed();

    if swap.routes.len() < 2 {
        return Ok(false);
    }

    if !AtaWorker::create_mints(&swap.routes) {
        warn!("Creating mints..., skip");
        return Ok(false);
    }

    if swap.profit > global::get_minimum_profit() as i64 {
        let arb_key = ArbitrageKey::from_swap_route(&swap);

        if should_allow_transaction(&arb_key) {
            // let clock = global_data::get_clock().unwrap();
            // let profit = swap_compute(&clock, &swap.routes, swap.amount_in, &swap.mint, true)?;
            // println!(" swap {} -> {}", swap.amount_in, profit);
            if let Some(signature) = send_arb(swap).await {
                // if true {
                info!(
                    "Quote time {:?} - sent time {:?} - total time {:?}",
                    quote_time,
                    (now.elapsed() - quote_time),
                    now.elapsed()
                );
                Ok(true)
            } else {
                warn!("Failed to send transaction.");
                Ok(false)
            }
        } else {
            Ok(false)
        }
    } else {
        Ok(false)
    }
}

pub async fn check_and_send_swap(
    swap: SwapRoutes,
    receive_time: Instant,
    source: SourceType,
) -> Result<()> {
    let amount_in = swap.amount_in;
    let org_profit = swap.profit;
    if let Some(clock) = global_data::get_clock() {
        let profit = swap_compute(
            &clock,
            &swap.routes,
            swap.amount_in,
            &swap.mint,
            global::enabled_slippage(),
        )
        .unwrap_or(-1);
        if profit > 0 {
            if let std::result::Result::Ok(sent) = do_arb(swap, receive_time).await {
                if sent {
                    info!(
                        "From {:?} - amount in {} -> {} ({})",
                        source, amount_in, profit, org_profit
                    );
                }
            }
        }
    }

    Ok(())
}

pub async fn send_route(route: Route, receive_time: Instant, source: SourceType) -> Result<()> {
    let time = Instant::now();
    if let Some(clock) = global_data::get_clock() {
        if let Some(swap) = optimization::find_profitable_route(route.clone(), &clock) {
            let amount_in = swap.amount_in;
            let profit = swap.profit;
            let optimization_time = time.elapsed();
            if let std::result::Result::Ok(sent) = do_arb(swap, receive_time).await {
                if sent {
                    // info!("{:#?}", route);
                    info!(
                        "From {:?} - weight {} - optimization time {:?} - handle time {:?} - amount in {} -> {}",
                        source,
                        route.product,
                        optimization_time,
                        time.elapsed(),
                        amount_in,
                        profit
                    );
                }
            }
        }
    }

    Ok(())
}

pub fn check_route(route: &Route, min_profit: u64) -> Option<SwapRoutes> {
    if let Some(clock) = global_data::get_clock() {
        if let Some(swap) = optimization::find_profitable_route(route.clone(), &clock) {
            if swap.profit > min_profit as i64 {
                return Some(swap);
            }
        }
    }

    None
}
