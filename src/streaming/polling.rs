use super::*;
use crate::{arb::ata_worker, default_lta, global, streaming::watcher::AccountUpdateEvent};
use anchor_client::solana_sdk::{commitment_config::CommitmentConfig, pubkey::Pubkey};
use anyhow::Result;
use futures::future::join_all;
use std::sync::Arc;
use tokio;
use tokio::sync::mpsc;
use tracing::{error, info};

pub type EventSender = mpsc::UnboundedSender<AccountUpdateEvent>;
pub type EventReceiver = mpsc::UnboundedReceiver<AccountUpdateEvent>;

pub struct PollingWatcher {
    event_sender: EventSender,
}

pub fn get_and_set_price(pool_pk: &Pubkey) {
    match pool_index::get(pool_pk) {
        Some(pool) => {
            if let Some(pool_type) = pool.to_pool_type() {
                let (atob, _) = pool_type.get_price(&pool.mint_a);
                global_data::update_price(pool_pk, pool.mint_a, atob);
            }
        }
        None => {}
    }
}

impl PollingWatcher {
    pub fn new() -> (Self, EventReceiver) {
        let (event_sender, event_receiver) = mpsc::unbounded_channel();

        let watcher = Self { event_sender };

        (watcher, event_receiver)
    }

    pub fn get_sender(&self) -> EventSender {
        self.event_sender.clone()
    }

    async fn fetch_unit(pubkeys: &[Pubkey], event_sender: &EventSender) -> Result<()> {
        let rpc = global::get_rpc_client();
        let accounts = match rpc
            .get_multiple_accounts_with_commitment(pubkeys, CommitmentConfig::processed())
            .await
        {
            std::result::Result::Ok(accounts) => accounts,
            Err(e) => {
                error!("Failed to fetch {} accounts: {}", pubkeys.len(), e);
                return Err(e.into());
            }
        };

        for (pubkey, account_option) in pubkeys.iter().zip(accounts.value.iter()) {
            match account_option {
                Some(account) => {
                    if let Some(data) = parse_account(pubkey, account) {
                        ACCOUNT_DATA.insert(pubkey.clone(), data.clone());
                        get_and_set_price(pubkey);

                        let event = AccountUpdateEvent {
                            pubkey: *pubkey,
                            data,
                            slot: accounts.context.slot,
                            receive_time: std::time::Instant::now(),
                        };

                        if let Err(_) = event_sender.send(event) {
                            error!("Failed to send account update event for {}", pubkey);
                        }
                    }
                }
                None => {}
            }
        }

        Ok(())
    }

    fn get_all_pubkeys() -> Vec<Pubkey> {
        ACCOUNT_DATA.iter().map(|entry| *entry.key()).collect()
    }

    async fn fetch(event_sender: &EventSender) -> Result<()> {
        let pubkeys = Self::get_all_pubkeys();

        if pubkeys.is_empty() {
            return Ok(());
        }

        let chunks: Vec<&[Pubkey]> = pubkeys.chunks(20).collect();
        let tasks: Vec<_> = chunks
            .into_iter()
            .map(|chunk| Self::fetch_unit(chunk, event_sender))
            .collect();

        join_all(tasks).await;
        // _write_token_pools_debug(&pool_index::get_all_pools(), "pools.txt")?;

        Ok(())
    }
}

fn _write_token_pools_debug(pools: &Vec<Arc<TokenPool>>, filename: &str) -> Result<()> {
    let pools_data: Vec<&TokenPool> = pools.iter().map(|arc| arc.as_ref()).collect();
    std::fs::write(filename, format!("{:#?}", pools_data))?;
    Ok(())
}

pub async fn start(ms: u64) -> Result<EventReceiver> {
    let (watcher, event_receiver) = PollingWatcher::new();
    let event_sender = watcher.get_sender();
    let mut interval = tokio::time::interval(tokio::time::Duration::from_millis(ms));

    store_lookup_table(&default_lta()).await?;
    global_data::add_account_type(clock_mint(), AccountTypeInfo::Clock);
    price::sync_price()?;
    ata_worker::AtaWorker::get_or_init();

    tokio::spawn(async move {
        info!("Begin polling watcher...");

        loop {
            interval.tick().await;
            if let Err(e) = PollingWatcher::fetch(&event_sender).await {
                error!("Polling fetch failed: {}", e);
            }
        }
    });

    Ok(event_receiver)
}
