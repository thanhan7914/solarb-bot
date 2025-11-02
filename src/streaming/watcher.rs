use anchor_client::solana_sdk::{account::Account, pubkey::Pubkey};
use anyhow::Result;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;
use tokio::sync::mpsc;
use tracing::info;
use yellowstone_grpc_proto::geyser::{SubscribeUpdateAccountInfo, subscribe_update};
use crate::arb;

use super::*;

#[derive(Debug, Clone)]
pub struct AccountUpdateEvent {
    pub pubkey: Pubkey,
    pub data: AccountDataType,
    pub slot: u64,
    pub receive_time: Instant,
}

pub type EventSender = mpsc::UnboundedSender<AccountUpdateEvent>;
pub type EventReceiver = mpsc::UnboundedReceiver<AccountUpdateEvent>;

#[derive(Debug)]
pub struct WatcherStats {
    pub total_updates: AtomicU64,
    pub successful_parses: AtomicU64,
    pub failed_parses: AtomicU64,
}

impl Default for WatcherStats {
    fn default() -> Self {
        Self {
            total_updates: AtomicU64::new(0),
            successful_parses: AtomicU64::new(0),
            failed_parses: AtomicU64::new(0),
        }
    }
}

pub struct DataWatcher {
    grpc_client: GrpcClient,
    event_sender: EventSender,
    stats: Arc<WatcherStats>,
}

fn subscribe_account_to_account(info: &SubscribeUpdateAccountInfo) -> Account {
    let owner_arr: [u8; 32] = info.owner.as_slice().try_into().expect("owner 32 bytes");
    Account {
        lamports:   info.lamports as u64,
        data:       info.data.clone(),
        owner:      Pubkey::new_from_array(owner_arr),
        executable: info.executable,
        rent_epoch: info.rent_epoch as u64,
    }
}

impl DataWatcher {
    pub fn new(config: GrpcConfig) -> (Self, EventReceiver) {
        let (event_sender, event_receiver) = mpsc::unbounded_channel();

        let watcher = Self {
            grpc_client: GrpcClient::new(config),
            event_sender,
            stats: Arc::new(WatcherStats::default()),
        };

        (watcher, event_receiver)
    }

    pub async fn start(&mut self) -> Result<()> {
        info!("Starting ultra-fast data watcher with intelligent batching...");

        let event_sender = self.event_sender.clone();
        let stats = Arc::clone(&self.stats);

        self.grpc_client
            .start_subscription(move |update, receive_time| {
                // Direct processing without spawning tasks
                Self::process_update_fast(update, &event_sender, &stats, receive_time);
            })
            .await?;

        info!("Ultra-fast data watcher started");
        Ok(())
    }

    #[inline(always)]
    fn process_update_fast(
        update: &yellowstone_grpc_proto::geyser::SubscribeUpdate,
        event_sender: &EventSender,
        stats: &Arc<WatcherStats>,
        receive_time: Instant,
    ) {
        // Increment counter atomically
        stats.total_updates.fetch_add(1, Ordering::Relaxed);

        // Only process account updates
        if let Some(subscribe_update::UpdateOneof::Account(account_update)) = &update.update_oneof {
            if let Some(account) = &account_update.account {
                let pubkey = Pubkey::try_from(account.pubkey.as_slice()).unwrap();

                // Parse and store in one step
                if let Some(data) = parse_account(&pubkey, &subscribe_account_to_account(account)) {
                    // Store immediately
                    ACCOUNT_DATA.insert(pubkey, data.clone());
                    polling::get_and_set_price(&pubkey);

                    // Check arbitrage relevance with fast type detection
                    if Self::is_arbitrage_relevant(&pubkey) {
                        arb::processor::find_from_pool(pubkey);
                        let event = AccountUpdateEvent {
                            pubkey,
                            data,
                            slot: account_update.slot,
                            receive_time,
                        };

                        // Non-blocking send
                        let _ = event_sender.send(event);
                    }

                    stats.successful_parses.fetch_add(1, Ordering::Relaxed);
                } else {
                    stats.failed_parses.fetch_add(1, Ordering::Relaxed);
                }
            }
        }
    }

    #[inline(always)]
    fn is_arbitrage_relevant(pubkey: &Pubkey) -> bool {
        // Checks based on known patterns
        // DEX accounts, token accounts, etc.
        return false;

        let account_type = AccountTypeInfo::from_pubkey(pubkey);
        match account_type {
            AccountTypeInfo::AmmPair
            | AccountTypeInfo::DlmmPair
            | AccountTypeInfo::Dammv2Pool
            | AccountTypeInfo::RaydiumAmmPool
            | AccountTypeInfo::RaydiumCpmmPool
            | AccountTypeInfo::RaydiumClmmPool
            | AccountTypeInfo::Whirlpool
            | AccountTypeInfo::VertigoPool
            | AccountTypeInfo::SolfiPool => true,
            _ => false,
        }
    }

    pub fn add_accounts(&self, accounts: Vec<String>) -> usize {
        let mut added_count = 0;
        for account in accounts {
            if self.grpc_client.add_account(account) {
                added_count += 1;
            }
        }
        added_count
    }

    pub fn remove_accounts(&self, accounts: Vec<String>) -> usize {
        let mut removed_count = 0;
        for account in accounts {
            if self.grpc_client.remove_account(account) {
                removed_count += 1;
            }
        }
        removed_count
    }

    pub fn add_programs(&self, programs: Vec<String>) -> usize {
        let mut added_count = 0;
        for program in programs {
            if self.grpc_client.add_program(program) {
                added_count += 1;
            }
        }
        added_count
    }

    pub fn batch_update(
        &self,
        add_accounts: Vec<String>,
        remove_accounts: Vec<String>,
        add_programs: Vec<String>,
        remove_programs: Vec<String>,
    ) -> bool {
        self.grpc_client
            .batch_update(add_accounts, remove_accounts, add_programs, remove_programs)
    }

    pub fn is_watching(&self, account: &str) -> bool {
        self.grpc_client
            .subscription_state
            .accounts
            .contains_key(account)
    }

    #[inline(always)]
    pub fn get_account_data(&self, pubkey: &Pubkey) -> Option<AccountDataType> {
        ACCOUNT_DATA.get(pubkey).map(|entry| entry.value().clone())
    }

    pub fn get_metrics(&self) -> WatcherMetrics {
        let grpc_metrics = self.grpc_client.get_metrics();
        let (total, success, failed) = self.get_stats();

        WatcherMetrics {
            total_updates: total,
            successful_parses: success,
            failed_parses: failed,
            accounts_count: grpc_metrics.accounts_count,
            programs_count: grpc_metrics.programs_count,
            pending_changes: grpc_metrics.pending_changes,
            last_update_slot: grpc_metrics.last_update_slot,
            is_running: grpc_metrics.is_running,
        }
    }

    pub fn last_update_slot(&self) -> u64 {
        self.grpc_client.get_metrics().last_update_slot
    }

    pub fn get_stats(&self) -> (u64, u64, u64) {
        (
            self.stats.total_updates.load(Ordering::Relaxed),
            self.stats.successful_parses.load(Ordering::Relaxed),
            self.stats.failed_parses.load(Ordering::Relaxed),
        )
    }

    pub async fn stop(&mut self) -> Result<()> {
        info!("Stopping data watcher...");
        self.grpc_client.stop().await?;
        info!("Data watcher stopped");
        Ok(())
    }

    pub fn clear_all(&self) {
        // Clear from GrpcClient
        self.grpc_client.subscription_state.accounts.clear();
        self.grpc_client.subscription_state.programs.clear();

        // Force immediate update
        self.grpc_client.force_immediate_update();
    }

    pub fn force_immediate_update(&self) {
        self.grpc_client.force_immediate_update();
    }
}

#[derive(Debug, Clone)]
pub struct WatcherMetrics {
    pub total_updates: u64,
    pub successful_parses: u64,
    pub failed_parses: u64,
    pub accounts_count: usize,
    pub programs_count: usize,
    pub pending_changes: usize,
    pub last_update_slot: u64,
    pub is_running: bool,
}

impl DataWatcher {
    pub fn add_account(&self, account: String) -> bool {
        self.grpc_client.add_account(account)
    }

    pub fn remove_account(&self, account: String) -> bool {
        self.grpc_client.remove_account(account)
    }

    pub fn add_program(&self, program: String) -> bool {
        self.grpc_client.add_program(program)
    }

    pub fn add_hot_account(&self, account: String) -> bool {
        let added = self.add_account(account);
        if added {
            self.force_immediate_update();
        }
        added
    }

    pub fn cleanup_cold_accounts(&self, cold_accounts: Vec<String>) -> usize {
        info!("Cleaning up {} cold accounts", cold_accounts.len());
        self.remove_accounts(cold_accounts)
    }
}
