use crate::arb::PoolType;
use crate::streaming::{AccountDataType, global_data};
use crate::{global, onchain, pool_index::TokenPool};
use crate::{pool_index, usdc_mint, wsol_mint};
use anchor_client::solana_sdk::{
    commitment_config::{CommitmentConfig, CommitmentLevel},
    pubkey::Pubkey,
};
use anyhow::Result;
use dashmap::DashMap;
use futures::future::join_all;
use std::collections::HashSet;
use std::sync::Arc;
use std::time::Duration;
use std::{collections::HashMap, sync::OnceLock};
use tokio::sync::mpsc;
use tracing::{error, info, warn};

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
struct AtaKey(pub Pubkey);

#[derive(Debug)]
enum AtaCmd {
    EnsureMany { mints: Vec<Pubkey> },
    Shutdown,
}

pub struct AtaWorker {
    tx: mpsc::UnboundedSender<AtaCmd>,
}

static ATA_WORKER: OnceLock<AtaWorker> = OnceLock::new();
static IN_FLIGHT: OnceLock<DashMap<AtaKey, ()>> = OnceLock::new();
static DONE_CACHE: OnceLock<DashMap<AtaKey, ()>> = OnceLock::new();

impl AtaWorker {
    pub fn get_or_init() -> &'static AtaWorker {
        ATA_WORKER.get_or_init(|| {
            info!("Initializing ATA Worker singleton");
            let (tx, mut rx) = mpsc::unbounded_channel::<AtaCmd>();

            let in_flight = IN_FLIGHT.get_or_init(|| DashMap::new());
            let done_cache = DONE_CACHE.get_or_init(|| DashMap::new());
            done_cache.insert(AtaKey(wsol_mint()), ());
            done_cache.insert(AtaKey(usdc_mint()), ());

            tokio::spawn(async move {
                info!("ATA Worker started");

                while let Some(cmd) = rx.recv().await {
                    match cmd {
                        AtaCmd::EnsureMany { mints } => {
                            process_ensure_many(mints, in_flight, done_cache).await;
                        }
                        AtaCmd::Shutdown => {
                            info!("ATA Worker shutting down");
                            break;
                        }
                    }
                }

                info!("ATA Worker stopped");
            });

            tokio::spawn(async move {
                info!("ATA Worker checker started...");
                let mut interval = tokio::time::interval(tokio::time::Duration::from_millis(500));

                loop {
                    interval.tick().await;
                    if let Err(e) = sync_epoch().await {
                        error!("check token process failed: {}", e);
                    }
                }
            });

            AtaWorker { tx }
        })
    }

    pub fn request_many(&self, mints: Vec<Pubkey>) {
        if mints.is_empty() {
            return;
        }

        let cmd = AtaCmd::EnsureMany { mints };

        if let Err(_) = self.tx.send(cmd) {
            error!("Failed to send ATA request - worker may be shut down");
        }
    }

    pub fn is_ata_ready(&self, mint: &Pubkey) -> bool {
        let done_cache = DONE_CACHE.get().unwrap();
        let key = AtaKey(*mint);
        done_cache.contains_key(&key)
    }

    pub fn is_ata_ready_or_inflight(&self, mint: &Pubkey) -> bool {
        let done_cache = DONE_CACHE.get().unwrap();
        let inflight_cache = IN_FLIGHT.get().unwrap();
        let key = AtaKey(*mint);
        done_cache.contains_key(&key) || inflight_cache.contains_key(&key)
    }

    pub async fn shutdown(&self) {
        let _ = self.tx.send(AtaCmd::Shutdown);
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
}

impl AtaWorker {
    pub fn create_mints(pools: &[PoolType]) -> bool {
        let mut missing: Vec<Pubkey> = Vec::with_capacity(pools.len() * 2);
        for pool in pools {
            let (mint_a, mint_b) = pool.get_mints();

            if !Self::check_ata_ready(&mint_a) {
                missing.push(mint_a);
            }

            if !Self::check_ata_ready(&mint_b) {
                missing.push(mint_b);
            }
        }

        let is_created_all = missing.is_empty();

        if !is_created_all {
            Self::request_ata_creation(missing);
        }

        is_created_all
    }

    pub fn request_ata_creation(mints: Vec<Pubkey>) {
        Self::get_or_init().request_many(mints);
    }

    pub fn check_ata_ready(mint: &Pubkey) -> bool {
        Self::get_or_init().is_ata_ready(mint)
    }

    pub fn check_ata_ready_or_inflight(mint: &Pubkey) -> bool {
        Self::get_or_init().is_ata_ready_or_inflight(mint)
    }

    pub fn set_ata_state(mint: Pubkey, state: bool) {
        let done_cache = DONE_CACHE.get().unwrap();
        let key = AtaKey(mint);

        if state {
            done_cache.insert(key, ());
        } else {
            done_cache.remove(&key);
        }
    }
}

impl Drop for AtaWorker {
    fn drop(&mut self) {
        let _ = self.tx.send(AtaCmd::Shutdown);
    }
}

async fn process_ensure_many(
    mints: Vec<Pubkey>,
    in_flight: &DashMap<AtaKey, ()>,
    done_cache: &DashMap<AtaKey, ()>,
) {
    let unique_mints = deduplicate_mints(&mints, in_flight, done_cache);

    if unique_mints.is_empty() {
        return;
    }

    // info!("Processing {} unique ATA creations", unique_mints.len());

    for mint in unique_mints {
        let key = AtaKey(mint);

        match check_and_create_ata(&mint).await {
            Ok(_) => {
                in_flight.remove(&key);
                done_cache.insert(key, ());

                tokio::time::sleep(Duration::from_millis(10)).await;
            }
            Err(e) => {
                in_flight.remove(&key);
                warn!("ATA creation failed for {}: {:?}", mint, e);
                tokio::time::sleep(Duration::from_millis(50)).await;
            }
        }
    }
}

async fn check_and_create_ata(mint: &Pubkey) -> Result<()> {
    if let Some(AccountDataType::Account(account)) = global_data::get_account(mint) {
        if account.owner == crate::token_program() {
            if !AtaWorker::check_ata_ready(&mint) {
                let _ata = onchain::create_ata_token_with_payer(
                    global::get_payer(),
                    mint,
                    Some(CommitmentLevel::Confirmed),
                )
                .await?;
            }
        } else {
            println!("Skip token 2022 {}", mint);
        }
    }

    Ok(())
}

fn deduplicate_mints(
    mints: &[Pubkey],
    in_flight: &DashMap<AtaKey, ()>,
    done_cache: &DashMap<AtaKey, ()>,
) -> Vec<Pubkey> {
    let mut unique = Vec::with_capacity(mints.len());
    let mut seen: HashSet<Pubkey> = HashSet::with_capacity(mints.len());

    for &mint in mints {
        let key = AtaKey(mint);

        if done_cache.contains_key(&key) {
            continue;
        }

        if in_flight.insert(key, ()).is_none() && seen.insert(mint) {
            unique.push(mint);
        }
    }

    unique
}

async fn updater(pools: &[Arc<TokenPool>]) -> Result<()> {
    let owner = global::get_pubkey();
    let mut ata_vec: Vec<Pubkey> = Vec::with_capacity(pools.len() * 2);
    let mut token_map: HashMap<Pubkey, Pubkey> = HashMap::new();
    for pool in pools {
        let ata_mint_a = onchain::get_associated_token_address(&owner, &pool.mint_a);
        let ata_mint_b = onchain::get_associated_token_address(&owner, &pool.mint_b);
        ata_vec.push(ata_mint_a);
        ata_vec.push(ata_mint_b);
        token_map.insert(ata_mint_a, pool.mint_a);
        token_map.insert(ata_mint_b, pool.mint_b);
    }

    let rpc = global::get_rpc_client();
    let accounts = match rpc
        .get_multiple_accounts_with_commitment(&ata_vec, CommitmentConfig::confirmed())
        .await
    {
        std::result::Result::Ok(accounts) => accounts,
        Err(e) => {
            error!("Failed to fetch {} accounts: {}", ata_vec.len(), e);
            return Err(e.into());
        }
    };

    for (pubkey, account_option) in ata_vec.iter().zip(accounts.value.iter()) {
        let mint_op = token_map.get(pubkey);
        if let Some(mint) = mint_op {
            match account_option {
                Some(_) => {
                    AtaWorker::set_ata_state(*mint, true);
                }
                None => {
                    AtaWorker::set_ata_state(*mint, false);
                }
            }
        }
    }

    Ok(())
}

async fn sync_epoch() -> Result<()> {
    let all_pools: Vec<Arc<TokenPool>> = pool_index::get_all_pools();
    if all_pools.is_empty() {
        return Ok(());
    }

    let chunks: Vec<Vec<Arc<TokenPool>>> =
        all_pools.chunks(50).map(|chunk| chunk.to_vec()).collect();

    let tasks: Vec<_> = chunks.iter().map(|chunk| updater(chunk)).collect();

    join_all(tasks).await;

    Ok(())
}
