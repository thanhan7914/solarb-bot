use crate::{
    config::{Config, Watcher, read_config},
    io,
};
use anchor_client::{
    solana_client::nonblocking::rpc_client::RpcClient,
    solana_sdk::{
        commitment_config::CommitmentConfig, pubkey::Pubkey, signature::Keypair, signer::Signer,
    },
};
use anyhow::Result;
use std::{
    path::Path,
    str::FromStr,
    sync::{
        Arc, OnceLock,
        atomic::{AtomicU64, Ordering},
    },
};

#[cfg(feature = "devnet")]
lazy_static::lazy_static! {
    static ref CONFIG:Config = read_config("config_dev.toml").unwrap();
    static ref RPC: Arc<RpcClient> = Arc::new(
        RpcClient::new_with_commitment(
            CONFIG.rpc.url.to_string(),
            CommitmentConfig::processed()
        )
    );
}

#[cfg(not(feature = "devnet"))]
lazy_static::lazy_static! {
    static ref CONFIG: Config = read_config("config.toml").unwrap();
    static ref RPC: Arc<RpcClient> = Arc::new(
        RpcClient::new_with_commitment(
            CONFIG.rpc.url.to_string(),
            CommitmentConfig::processed()
        )
    );
}

pub const WSOL: Pubkey = Pubkey::new_from_array([
    6, 155, 136, 87, 254, 171, 129, 132, 251, 104, 127, 99, 70, 24, 192, 53, 218, 196, 57, 220, 26,
    235, 59, 85, 152, 160, 240, 0, 0, 0, 0, 1,
]);

pub fn get_rpc_client() -> Arc<RpcClient> {
    RPC.clone()
}

pub fn get_config() -> &'static Config {
    &CONFIG
}

pub fn only_watch_succeed_tx() -> bool {
    let config = get_config();
    let watcher = config.watcher.clone();
    watcher.only_succeed
}

pub fn only_watch_failed_tx() -> bool {
    let config = get_config();
    let watcher = config.watcher.clone();
    watcher.only_failed
}

pub fn get_watcher_config() -> Watcher {
    let config = get_config();
    let watcher = config.watcher.clone();
    watcher
}

pub fn enabled_slippage() -> bool {
    let config = get_config();
    let bot = config.bot.clone();
    bot.enabled_slippage
}

pub fn get_slippage_bps() -> u64 {
    let config = get_config();
    let bot = config.bot.clone();
    bot.slippage_bps
}

pub fn new_rpc(rpc_endpoint: &str) -> Arc<RpcClient> {
    Arc::new(RpcClient::new_with_commitment(
        rpc_endpoint.to_string(),
        CommitmentConfig::processed(),
    ))
}

static GLOBAL_KEYPAIR: OnceLock<Arc<Keypair>> = OnceLock::new();
static GLOBAL_PAYER: OnceLock<Arc<Keypair>> = OnceLock::new();
static BASE_MINT: OnceLock<Arc<Pubkey>> = OnceLock::new();
static MINT_ATA_AMOUNT: AtomicU64 = AtomicU64::new(0);
static MINIMUM_PROFIT: AtomicU64 = AtomicU64::new(1000);

#[inline]
pub fn get_base_mint_amount() -> u64 {
    MINT_ATA_AMOUNT.load(Ordering::Relaxed)
}

#[inline]
pub fn get_base_mint() -> Arc<Pubkey> {
    BASE_MINT.get().expect("BASE_MINT not initialized").clone()
}

#[inline]
pub fn get_minimum_profit() -> u64 {
    MINIMUM_PROFIT.load(Ordering::Relaxed)
}

#[inline]
pub fn get_keypair() -> Arc<Keypair> {
    GLOBAL_KEYPAIR
        .get()
        .expect("Keypair not initialized")
        .clone()
}

#[inline]
pub fn get_pubkey() -> Pubkey {
    GLOBAL_KEYPAIR
        .get()
        .expect("Keypair not initialized")
        .pubkey()
}

pub fn get_payer() -> Arc<Keypair> {
    GLOBAL_PAYER
        .get()
        .expect("Payer keypair not initialized")
        .clone()
}

fn load_keypair_with_fallback(wallet_path: Option<&str>) -> Arc<Keypair> {
    let real_path = match wallet_path {
        Some(val) => val,
        None => "./wallet.json",
    };

    if Path::new(real_path).exists() {
        Arc::new(io::load_keypair(real_path).unwrap())
    } else {
        Arc::new(io::load_keypair("./wallet.json").unwrap())
    }
}

pub async fn prepare_data(wallet_path: Option<&str>, mint_str: &str) -> Result<()> {
    let mint = Pubkey::from_str(mint_str)?;
    BASE_MINT
        .set(Arc::new(mint))
        .map_err(|_| anyhow::anyhow!("Base mint already initialized"))?;
    let real_path = match wallet_path {
        Some(val) => val,
        None => "./wallet.json",
    };
    println!("Load wallet from {}", real_path);
    let payer = Arc::new(io::load_keypair(real_path).unwrap());
    GLOBAL_KEYPAIR
        .set(payer)
        .map_err(|_| anyhow::anyhow!("Global keypair already initialized"))?;
    let amount = crate::onchain::get_ata_token_amount(&get_pubkey(), &mint).await?;
    MINT_ATA_AMOUNT.store(amount, Ordering::Relaxed);
    MINIMUM_PROFIT.store(CONFIG.bot.minimum_profit, Ordering::Relaxed);

    let payer = load_keypair_with_fallback(Some("./payer"));
    GLOBAL_PAYER
        .set(payer)
        .map_err(|_| anyhow::anyhow!("Global GLOBAL_PAYER already initialized"))?;

    Ok(())
}
