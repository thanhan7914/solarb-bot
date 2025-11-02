use crate::{
    cache::Cache,
    clock_mint,
    config::Config,
    global, onchain,
    pool_index::{self, TokenPool},
    dex::pumpfun::PumpAmmReader,
    streaming::{
        grpc::{GrpcClient, GrpcConfig},
        watcher::DataWatcher,
    },
    dex::whirlpool,
};
use anchor_client::solana_sdk::{
    account::Account, address_lookup_table::AddressLookupTableAccount, pubkey::Pubkey,
};
use anyhow::{Ok, Result};
use dashmap::DashMap;
use dlmm_interface::LbPairAccount;
use once_cell::sync::Lazy;
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio::time::Duration;
use tracing::{info, warn};

pub mod blockhash;
pub mod commander;
pub mod global_data;
pub mod grpc;
pub mod loader;
pub mod monitor;
pub mod parser;
pub mod polling;
pub mod pool_loader;
pub mod price;
pub mod processor;
pub mod typedefs;
pub mod updater;
pub mod util;
pub mod watcher;

pub use loader::*;
pub use parser::parse_account;
pub use typedefs::*;

static ACCOUNT_TYPE_MAP: once_cell::sync::Lazy<Arc<DashMap<Pubkey, AccountTypeInfo>>> =
    Lazy::new(|| Arc::new(DashMap::new()));

static ACCOUNT_DATA: once_cell::sync::Lazy<Arc<DashMap<Pubkey, AccountDataType>>> =
    once_cell::sync::Lazy::new(|| Arc::new(DashMap::new()));

static PRICE_DATA: once_cell::sync::Lazy<Arc<DashMap<Pubkey, (Pubkey, f64)>>> =
    once_cell::sync::Lazy::new(|| Arc::new(DashMap::new()));

static MINT_DATA: once_cell::sync::Lazy<Arc<DashMap<Pubkey, Account>>> =
    once_cell::sync::Lazy::new(|| Arc::new(DashMap::new()));

// mapping mint -> lookup table
pub static PK_TO_ALT: Lazy<Cache<Pubkey, Pubkey>> = once_cell::sync::Lazy::new(|| Cache::new());

// mapping alt_pk -> lookup table data
pub static ALT_DATA: Lazy<Cache<Pubkey, AddressLookupTableAccount>> =
    once_cell::sync::Lazy::new(|| Cache::new());

const CLOCK_ACCOUNT: &str = "SysvarC1ock11111111111111111111111111111111";

pub async fn start(conf: Config) -> Result<mpsc::UnboundedSender<WatcherCommand>> {
    let config = GrpcConfig {
        endpoint: conf.grpc.url.to_string(),
        x_token: conf.grpc.token,
        batch_interval_ms: 50,        // Batch every 50ms cho ultra-fast
        max_batch_size: 100,          // Max 100 changes before force flush
        connection_timeout_ms: 15000, // 15s timeout
    };

    println!("{:?}", config);
    let (mut watcher, event_receiver) = DataWatcher::new(config);
    if conf.grpc.enabled {
        watcher.start().await?;
        watcher.add_account(String::from(CLOCK_ACCOUNT));
    }

    let (cmd_tx, cmd_rx) = mpsc::unbounded_channel::<WatcherCommand>();
    let cmd_tx_monitor = cmd_tx.clone();
    let cmd_tx_updater = cmd_tx.clone();
    tokio::spawn(processor::signal_receiver(event_receiver, cmd_tx_updater));
    tokio::spawn(commander::run_command_processor(cmd_rx, watcher));
    tokio::spawn(monitor::watch(cmd_tx_monitor, 10));

    Ok(cmd_tx)
}

fn retrieve_alt_pk(mint: &Pubkey) -> Option<Pubkey> {
    PK_TO_ALT.get(mint)
}

pub fn retrieve_alt_from_alt_pk(alt_pk: &Pubkey) -> Option<AddressLookupTableAccount> {
    ALT_DATA.get(alt_pk)
}

pub fn retrieve_alt(mint: &Pubkey) -> Option<AddressLookupTableAccount> {
    if let Some(alt_pk) = retrieve_alt_pk(mint) {
        retrieve_alt_from_alt_pk(&alt_pk)
    } else {
        None
    }
}

pub async fn store_lookup_table(alt_pk: &Pubkey) -> Result<()> {
    let rpc_client = global::get_rpc_client();
    let alt_accounts = onchain::fetch_alt_account(rpc_client, *alt_pk).await?;
    ALT_DATA.forever(*alt_pk, alt_accounts);
    Ok(())
}

pub fn store_mint_alt(mint: Pubkey, alt_pk: Pubkey) {
    PK_TO_ALT.forever(mint, alt_pk);
}

pub fn has_alt_pk(mint: &Pubkey) -> bool {
    PK_TO_ALT.has(mint)
}

pub fn count_accounts() -> usize {
    ACCOUNT_DATA.len()
}
