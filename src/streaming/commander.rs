use std::time::Instant;
use tokio::sync::mpsc;
use tracing::{error, info};

use super::*;
use crate::{pool_index, streaming::watcher::DataWatcher};

pub async fn run_command_processor(
    mut cmd_rx: mpsc::UnboundedReceiver<WatcherCommand>,
    mut watcher: DataWatcher,
) {
    let mut command_count = 0u64;
    let mut batch_buffer = Vec::with_capacity(10);
    let start_time = Instant::now();

    info!("Starting commander processing loop...");

    while let Some(command) = cmd_rx.recv().await {
        command_count += 1;

        match command {
            WatcherCommand::AddAccount(pubkey) => {
                if watcher.add_account(pubkey.clone()) {
                    if command_count % 10 == 0 {
                        info!("Added account: {}", pubkey);
                    }
                }
            }
            WatcherCommand::RemoveAccount(pubkey) => {
                if watcher.remove_account(pubkey.clone()) {
                    if command_count % 10 == 0 {
                        info!("Removed account: {}", pubkey);
                    }
                }
            }
            WatcherCommand::AddHotAccount(pubkey) => {
                if watcher.add_hot_account(pubkey.clone()) {
                    info!("Added HOT arbitrage account: {}", pubkey);
                }
            }
            WatcherCommand::BatchAdd { accounts } => {
                watcher.add_accounts(accounts.clone());
            }
            WatcherCommand::BatchRemove { accounts } => {
                let removed = watcher.remove_accounts(accounts.clone());
                info!("Batch removed {} accounts", removed);
            }
            WatcherCommand::BatchUpdate {
                add_accounts,
                remove_accounts,
            } => {
                let changed = watcher.batch_update(add_accounts, remove_accounts, vec![], vec![]);
                if changed {
                    info!("Batch update completed");
                }
            }
            WatcherCommand::DiscoverNew { accounts } => {
                batch_buffer.extend(accounts);

                if batch_buffer.len() >= 5 {
                    let new_accounts = batch_buffer.drain(..).collect();
                    let added = watcher.add_accounts(new_accounts);
                    info!("Discovery batch: added {} accounts", added);
                }
            }
            WatcherCommand::RemoveOld { account } => {
                watcher.remove_account(account);
            }
            WatcherCommand::EmergencyCleanup { accounts } => {
                let removed = watcher.remove_accounts(accounts.clone());
                info!("Emergency cleanup: removed {} accounts", removed);
            }
            WatcherCommand::GetMetrics => {
                print_metrics(&watcher, start_time, command_count);
            }
            WatcherCommand::Stop => {
                info!("Received stop command");
                if let Err(e) = watcher.stop().await {
                    error!("Error stopping watcher: {}", e);
                }
                break;
            }
        }
    }

    info!("Commander loop stopped after {} commands", command_count);
}

#[inline]
fn print_metrics(watcher: &DataWatcher, start_time: Instant, command_count: u64) {
    let metrics = watcher.get_metrics();
    let uptime = start_time.elapsed();

    info!(
        "GRPC - METRICS -\
        Updates: {} -\
        Pools: {} -\
        Accounts: {} -\
        Programs: {} -\
        Pending: {} -\
        Success Rate: {}% -\
        Uptime: {:.1}s -\
        Commands: {}",
        metrics.total_updates,
        pool_index::count(),
        metrics.accounts_count,
        metrics.programs_count,
        metrics.pending_changes,
        if metrics.total_updates > 0 {
            (metrics.successful_parses * 100) / metrics.total_updates
        } else {
            0
        },
        uptime.as_secs_f64(),
        command_count
    );
}
