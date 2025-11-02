use crate::{
    dex::{meteora, pumpfun, raydium, solfi, vertigo, whirlpool},
    pool_index::TokenPoolType,
};
use anchor_client::solana_sdk::{account::Account, pubkey::Pubkey};
use dlmm_interface::{BinArray, LbPair};
use std::collections::{HashMap, VecDeque};

#[derive(Debug)]
pub struct SwapRoutes {
    pub routes: Vec<PoolType>,
    pub profit: i64,
    pub amount_in: u64,
    pub threshold: u64,
    pub mint: Pubkey,
}

#[derive(Debug, Clone)]
pub enum PoolType {
    Meteora(Pubkey, MeteoraDlmmData),
    Pump(Pubkey, PumpAmmData),
    MeteoraDammv2(Pubkey, MeteoraDammv2Data),
    Vertigo(Pubkey, VertigoData),
    RaydiumAmm(Pubkey, RaydiumAmmData),
    RaydiumCpmm(Pubkey, RaydiumCpmmData),
    RaydiumClmm(Pubkey, RaydiumClmmData),
    Whirlpool(Pubkey, WhirlpoolData),
    Solfi(Pubkey, SolfiData),
}

#[derive(Debug, Clone)]
pub struct PoolPair {
    pub from_pool: Box<PoolType>,
    pub to_pool: Box<PoolType>,
    pub price_difference: f64,
    pub profit: i64,
    pub amount_in: u64,
    pub threshold: u64,
    pub mint: Pubkey,
}

#[derive(Debug, Clone)]
pub struct PoolInfo {
    pub pool_type: Box<PoolType>,
    pub price: f64,
    pub index: usize,
}

#[derive(Debug, Clone)]
pub struct MeteoraDlmmData {
    pub pool_address: Pubkey,
    pub lb_pair: LbPair,
    pub mint_x_account: Account,
    pub mint_y_account: Account,
    pub bin_arrays: HashMap<Pubkey, BinArray>,
}

#[derive(Debug, Clone)]
pub struct PumpAmmData {
    pub pool_address: Pubkey,
    pub pool: pumpfun::AmmPool,
    pub reserves: pumpfun::PoolReserves,
}

#[derive(Debug, Clone)]
pub struct VertigoData {
    pub pool_address: Pubkey,
    pub pool_state: vertigo::Pool,
}

#[derive(Debug, Clone)]
pub struct MeteoraDammv2Data {
    pub pool_address: Pubkey,
    pub pool_state: meteora::damm::Pool,
}

#[derive(Debug, Clone)]
pub struct RaydiumAmmData {
    pub pool_address: Pubkey,
    pub pool_state: raydium::amm::AmmInfo,
    pub market_state: raydium::amm::serum::MarketState,
    pub vaults: raydium::amm::PoolVaults,
}

#[derive(Debug, Clone)]
pub struct RaydiumCpmmData {
    pub pool_address: Pubkey,
    pub pool_state: raydium::cpmm::PoolState,
    pub amm_config: raydium::cpmm::AmmConfig,
    pub vaults: raydium::cpmm::PoolReserves,
}

#[derive(Debug, Clone)]
pub struct RaydiumClmmData {
    pub pool_address: Pubkey,
    pub pool_state: raydium::clmm::PoolState,
    pub tick_array_bitmap_ext: raydium::clmm::tick_array_bitmap_extension::TickArrayBitmapExtension,
    pub left_ticks: VecDeque<raydium::clmm::tick_array::TickArrayState>,
    pub right_ticks: VecDeque<raydium::clmm::tick_array::TickArrayState>,
}

#[derive(Debug, Clone)]
pub struct WhirlpoolData {
    pub pool_address: Pubkey,
    pub pool_state: whirlpool::state::Whirlpool,
    pub oracle: Option<whirlpool::state::oracle::Oracle>,
    pub tick_data: [(Pubkey, whirlpool::state::TickArray); 5],
}

#[derive(Debug, Clone)]
pub struct SolfiData {
    pub pool_address: Pubkey,
    pub pool_state: solfi::Pool,
    pub reserves: solfi::PoolReserves,
}

#[derive(Clone, Debug)]
pub struct Hop {
    pub from: Pubkey,
    pub to: Pubkey,
    pub pool: Pubkey,
    pub pool_type: TokenPoolType,
    pub rate: f64,
}

#[derive(Clone, Debug)]
pub struct Route {
    pub start: Pubkey,
    pub hops: Vec<Hop>,
    pub product: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SourceType {
    MultiHops,
    PoolPair,
}

pub struct ProfitableRoute {
    pub route: SwapRoutes,
    pub quote_time: tokio::time::Instant,
    pub sent_time: tokio::time::Instant,
}
