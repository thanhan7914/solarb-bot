use anyhow::Result;
use serde::Deserialize;
use std::fs;
use toml;

#[derive(Debug, Deserialize, Clone)]
pub struct Config {
    pub rpc: Rpc,
    pub grpc: Grpc,
    pub bot: BotConfig,
    pub watcher: Watcher,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Rpc {
    pub url: String,
    pub websocket_url: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Grpc {
    pub url: String,
    pub token: Option<String>,
    pub enabled: bool,
}

#[derive(Debug, Deserialize, Clone)]
pub struct BotConfig {
    pub mint: String,
    pub minimum_profit: u64,
    pub optimization_method: String,
    pub max_hops: u8,
    pub price_threshold: f64,
    pub optimization_amount_percent: u8,
    pub routes_batch_size: u32,
    pub enabled_slippage: bool,
    pub slippage_bps: u64,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Watcher {
    pub only_succeed: bool,
    pub only_failed: bool,
    pub max_pools: u32,
    pub max_routes: u32,
}

pub fn read_config(path: &str) -> Result<Config> {
    let content = fs::read_to_string(path)?;
    let config: Config = toml::from_str(&content)?;
    Ok(config)
}
