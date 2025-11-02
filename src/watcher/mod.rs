use crate::{
    config::Config,
    global, pool_index,
    streaming::{AccountDataType, WatcherCommand},
};
use anchor_client::solana_sdk::pubkey::Pubkey;
use anyhow::Result;
use crossbeam::queue::SegQueue;
use dashmap::DashMap;
use futures_util::{SinkExt, StreamExt};
use lockfree::stack::Stack;
use once_cell::sync::Lazy;
use parking_lot::RwLock;
use serde_json::{Value, json};
use std::{collections::HashSet, sync::Arc, time::Duration};
use tokio::sync::mpsc;
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};
use tracing::{debug, error, info, warn};

mod account_data_type;
pub mod constants;
mod lookuptable;
mod parser;
mod processor;
mod transaction;

pub static SIG_QUEUE: Lazy<Arc<Stack<String>>> = Lazy::new(|| Arc::new(Stack::new()));
pub static POOL_QUEUE: Lazy<Arc<SegQueue<(Pubkey, AccountDataType, Option<Pubkey>)>>> =
    Lazy::new(|| Arc::new(SegQueue::new()));

#[derive(Debug, Clone)]
pub struct ProgramInfo {
    pub program_id: Pubkey,
    pub name: String,
    pub description: Option<String>,
    pub is_dex: bool,
}

#[derive(Debug, Clone)]
pub struct TransactionInfo {
    pub signature: String,
    pub slot: u64,
    pub block_time: Option<i64>,
    pub program_ids: Vec<Pubkey>,
    pub success: bool,
    pub fee: Option<u64>,
    pub logs: Vec<String>,
    pub accounts: Vec<Pubkey>,
    pub err: Option<Value>,
}

pub struct SolanaTransactionWatcher {
    programs: Arc<RwLock<Vec<ProgramInfo>>>,
    processed_signatures: Arc<DashMap<String, bool>>,
    transaction_cache: Arc<DashMap<String, TransactionInfo>>,
    ws_endpoint: String,
    subscription_ids: Arc<DashMap<String, u64>>,
    connection_healthy: Arc<std::sync::atomic::AtomicBool>,
}

impl SolanaTransactionWatcher {
    pub fn new(ws_endpoint: String) -> Self {
        Self {
            programs: Arc::new(RwLock::new(Vec::new())),
            processed_signatures: Arc::new(DashMap::new()),
            transaction_cache: Arc::new(DashMap::new()),
            ws_endpoint,
            subscription_ids: Arc::new(DashMap::new()),
            connection_healthy: Arc::new(std::sync::atomic::AtomicBool::new(false)),
        }
    }

    pub fn add_program(
        &self,
        program_id: Pubkey,
        name: String,
        description: Option<String>,
        is_dex: bool,
    ) {
        let name_clone = name.clone();
        let program_info = ProgramInfo {
            program_id,
            name,
            description,
            is_dex,
        };

        self.programs.write().push(program_info);
        info!(
            "✅ Added program to watch list: {} ({})",
            program_id, name_clone
        );
    }

    pub fn add_programs(&self, programs: Vec<(Pubkey, String, Option<String>, bool)>) {
        let mut program_list = self.programs.write();

        for (program_id, name, description, is_dex) in programs {
            let name_clone = name.clone();
            let program_info = ProgramInfo {
                program_id,
                name,
                description,
                is_dex,
            };
            program_list.push(program_info);
            info!(
                "✅ Added program to watch list: {} ({})",
                program_id, name_clone
            );
        }
    }

    pub async fn start_watching(&self) -> Result<()> {
        info!("🚀 Starting Solana transaction watcher via WebSocket...");

        let programs = self.programs.read().clone();
        if programs.is_empty() {
            warn!("⚠️ No programs to watch. Please add programs first.");
            return Ok(());
        }

        info!("📋 Watching {} programs:", programs.len());
        for program in &programs {
            info!("   - {} ({})", program.program_id, program.name);
        }

        // Add connection timeout
        let connect_future = connect_async(&self.ws_endpoint);
        let (ws_stream, _) = tokio::time::timeout(Duration::from_secs(15), connect_future)
            .await
            .map_err(|_| anyhow::anyhow!("WebSocket connection timeout after 15 seconds"))?
            .map_err(|e| anyhow::anyhow!("WebSocket connection failed: {}", e))?;

        let (mut write, mut read) = ws_stream.split();
        info!("✅ Connected to WebSocket: {}", self.ws_endpoint);

        // Mark connection as healthy
        self.connection_healthy
            .store(true, std::sync::atomic::Ordering::Relaxed);

        // Start heartbeat task using shared channel
        let (heartbeat_tx, mut heartbeat_rx) = mpsc::unbounded_channel::<()>();
        let heartbeat_handle = {
            let healthy = self.connection_healthy.clone();

            tokio::spawn(async move {
                let mut interval = tokio::time::interval(Duration::from_secs(25));
                let mut ping_count = 0u32;

                loop {
                    interval.tick().await;

                    if !healthy.load(std::sync::atomic::Ordering::Relaxed) {
                        break;
                    }

                    // Signal main loop to send ping
                    if heartbeat_tx.send(()).is_err() {
                        break;
                    }

                    ping_count += 1;
                    debug!("💓 Heartbeat signal sent ({})", ping_count);
                }
                debug!("🔄 Heartbeat task ended");
            })
        };

        // Subscribe to programs in smaller batches
        let mut subscription_id = 1u64;
        let program_chunks: Vec<_> = programs.chunks(3).collect(); // Max 3 subscriptions at once

        for (chunk_idx, chunk) in program_chunks.iter().enumerate() {
            info!(
                "📡 Subscribing to batch {}/{} ({} programs)",
                chunk_idx + 1,
                program_chunks.len(),
                chunk.len()
            );

            for program in chunk.iter() {
                let subscribe_request = json!({
                    "jsonrpc": "2.0",
                    "id": subscription_id,
                    "method": "logsSubscribe",
                    "params": [
                        {
                            "mentions": [program.program_id.to_string()]
                        },
                        {
                            "commitment": "processed"
                        }
                    ]
                });

                write
                    .send(Message::Text(subscribe_request.to_string().into()))
                    .await?;
                self.subscription_ids
                    .insert(program.program_id.to_string(), subscription_id);

                info!(
                    "📡 Subscribed to logs for program: {} (ID: {})",
                    program.program_id, subscription_id
                );
                subscription_id += 1;

                tokio::time::sleep(Duration::from_millis(300)).await;
            }

            // Longer delay between batches
            if chunk_idx < program_chunks.len() - 1 {
                tokio::time::sleep(Duration::from_millis(1000)).await;
            }
        }

        info!("✅ All subscriptions completed. Listening for transactions...");

        // Message processing loop with timeout detection
        let mut last_message_time = std::time::Instant::now();
        let message_timeout = Duration::from_secs(90); // 1.5 minutes without any message
        let mut ping_counter = 0u32;

        loop {
            tokio::select! {
                // Handle heartbeat signals
                _ = heartbeat_rx.recv() => {
                    ping_counter += 1;
                    let ping_data = ping_counter.to_le_bytes().to_vec().into();

                    if let Err(e) = write.send(Message::Ping(ping_data)).await {
                        warn!("💔 Heartbeat failed: {}", e);
                        self.connection_healthy.store(false, std::sync::atomic::Ordering::Relaxed);
                        break;
                    }
                    debug!("💓 Heartbeat sent ({})", ping_counter);
                }

                // Handle incoming WebSocket messages
                message_result = tokio::time::timeout(Duration::from_secs(5), read.next()) => {
                    match message_result {
                        Ok(Some(Ok(Message::Text(text)))) => {
                            last_message_time = std::time::Instant::now();
                            if let Err(e) = self.handle_websocket_message(&text).await {
                                error!("❌ Error handling WebSocket message: {}", e);
                            }
                        }
                        Ok(Some(Ok(Message::Close(frame)))) => {
                            warn!("🔌 WebSocket connection closed: {:?}", frame);
                            break;
                        }
                        Ok(Some(Ok(Message::Ping(data)))) => {
                            last_message_time = std::time::Instant::now();
                            debug!("🏓 Received ping, sending pong");
                            if let Err(e) = write.send(Message::Pong(data)).await {
                                error!("❌ Failed to send pong: {}", e);
                                break;
                            }
                        }
                        Ok(Some(Ok(Message::Pong(data)))) => {
                            last_message_time = std::time::Instant::now();
                            debug!("🏓 Received pong: {} bytes", data.len());
                        }
                        Ok(Some(Err(e))) => {
                            error!("❌ WebSocket error: {}", e);
                            break;
                        }
                        Ok(None) => {
                            warn!("🔌 WebSocket stream ended");
                            break;
                        }
                        Err(_) => {
                            // Timeout occurred, check if we haven't received messages for too long
                            if last_message_time.elapsed() > message_timeout {
                                warn!("⏰ No messages received for {:?}, assuming connection is dead", message_timeout);
                                break;
                            }
                            // Continue the loop for normal timeouts
                        }
                        _ => {
                            debug!("📨 Received other message type");
                        }
                    }
                }
            }
        }

        // Cleanup
        heartbeat_handle.abort();
        self.connection_healthy
            .store(false, std::sync::atomic::Ordering::Relaxed);

        Err(anyhow::anyhow!("WebSocket connection ended"))
    }

    pub async fn start_watching_with_retry(&self) -> Result<()> {
        let mut retry_count = 0;
        const MAX_RETRIES: u32 = 20; // Increased max retries
        const MAX_BACKOFF_SECS: u64 = 120; // Max 2 minutes backoff

        loop {
            match self.start_watching().await {
                Ok(_) => {
                    info!("✅ WebSocket session completed normally");
                    retry_count = 0; // Reset retry count on successful connection
                }
                Err(e) => {
                    retry_count += 1;
                    error!("❌ WebSocket error (attempt {}): {}", retry_count, e);

                    if retry_count >= MAX_RETRIES {
                        error!("🚫 Max retries ({}) exceeded, giving up", MAX_RETRIES);
                        return Err(e);
                    }

                    // Exponential backoff with cap
                    let backoff_secs = std::cmp::min(2_u64.pow(retry_count), MAX_BACKOFF_SECS);
                    let delay = Duration::from_secs(backoff_secs);

                    warn!(
                        "🔄 Reconnecting in {:?}... (attempt {}/{})",
                        delay, retry_count, MAX_RETRIES
                    );
                    tokio::time::sleep(delay).await;
                }
            }
        }
    }

    async fn handle_websocket_message(&self, text: &str) -> Result<()> {
        let response: Value = serde_json::from_str(text)?;

        if let Some(result) = response.get("result") {
            if result.is_number() {
                let sub_id = result.as_u64().unwrap_or(0);
                debug!("✅ Subscription confirmed with ID: {}", sub_id);
                return Ok(());
            }
        }

        if let Some(method) = response.get("method") {
            if method == "logsNotification" {
                if let Some(params) = response.get("params") {
                    if !pool_index::is_reach_max() {
                        self.process_logs_notification(params).await?;
                    }
                }
            }
        }

        Ok(())
    }

    async fn process_logs_notification(&self, params: &Value) -> Result<()> {
        if let Some(result) = params.get("result") {
            if let Some(value) = result.get("value") {
                let signature = value
                    .get("signature")
                    .and_then(|s| s.as_str())
                    .unwrap_or("")
                    .to_string();

                if signature.is_empty() {
                    return Ok(());
                }

                if self.processed_signatures.contains_key(&signature) {
                    return Ok(());
                }

                self.processed_signatures.insert(signature.clone(), true);

                let slot = result
                    .get("context")
                    .and_then(|c| c.get("slot"))
                    .and_then(|s| s.as_u64())
                    .unwrap_or(0);

                let logs = value
                    .get("logs")
                    .and_then(|l| l.as_array())
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|v| v.as_str())
                            .map(|s| s.to_string())
                            .collect::<Vec<String>>()
                    })
                    .unwrap_or_default();

                // if self.count_dex_from_logs(&logs) <= 1 {
                //     return Ok(());
                // }

                let err = value.get("err").cloned();
                let success = err.is_none() || err.as_ref().unwrap().is_null();

                if global::only_watch_succeed_tx() && !success {
                    return Ok(());
                }

                if global::only_watch_failed_tx() && success {
                    return Ok(());
                }

                let tx_info = TransactionInfo {
                    signature: signature.clone(),
                    slot,
                    block_time: None,
                    program_ids: self.extract_program_ids_from_logs(&logs),
                    success,
                    fee: None,
                    logs,
                    accounts: Vec::new(),
                    err,
                };

                self.transaction_cache
                    .insert(signature.clone(), tx_info.clone());

                SIG_QUEUE.push(signature);
            }
        }

        Ok(())
    }

    fn extract_program_ids_from_logs(&self, logs: &[String]) -> Vec<Pubkey> {
        let mut program_ids = Vec::new();
        let programs = self.programs.read();

        for log in logs {
            for program in programs.iter() {
                let program_str = program.program_id.to_string();
                if log.contains(&program_str) {
                    if !program_ids.contains(&program.program_id) {
                        program_ids.push(program.program_id);
                    }
                }
            }
        }

        program_ids
    }

    pub fn get_stats(&self) -> (usize, usize, usize) {
        let programs_count = self.programs.read().len();
        let processed_count = self.processed_signatures.len();
        let cached_count = self.transaction_cache.len();

        (programs_count, processed_count, cached_count)
    }

    pub fn clear_cache(&self) {
        self.processed_signatures.clear();
        self.transaction_cache.clear();
    }

    pub fn is_healthy(&self) -> bool {
        self.connection_healthy
            .load(std::sync::atomic::Ordering::Relaxed)
    }
}

pub async fn start_batch_processing(
    rpc_endpoint: &str,
    num_workers: usize,
    batch_size: usize,
) -> Result<()> {
    let rpc_endpoint = rpc_endpoint.to_string();

    let shared_lookup_cache = Arc::new(lookuptable::LookupTableCache::new(rpc_endpoint.clone()));
    let mut handles = Vec::new();

    for worker_id in 0..num_workers {
        let rpc_endpoint_clone = rpc_endpoint.clone();
        let lookup_cache_clone = shared_lookup_cache.clone();

        let handle = tokio::spawn(async move {
            process_queue_batch_worker(
                worker_id,
                &rpc_endpoint_clone,
                batch_size,
                lookup_cache_clone,
            )
            .await
        });

        handles.push(handle);
    }

    for handle in handles {
        if let Err(e) = handle.await {
            eprintln!("Batch worker task failed: {}", e);
        }
    }

    Ok(())
}

async fn process_queue_batch_worker(
    worker_id: usize,
    rpc_endpoint: &str,
    batch_size: usize,
    shared_lookup_cache: Arc<lookuptable::LookupTableCache>,
) -> Result<()> {
    loop {
        let mut batch = Vec::new();

        for _ in 0..batch_size {
            if let Some(signature) = SIG_QUEUE.pop() {
                batch.push(signature);
            } else {
                break;
            }
        }

        if batch.is_empty() {
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
            continue;
        }

        if pool_index::is_reach_max() {
            warn!("Stop the process_queue_batch_worker {}", worker_id);
            break;
        }

        let tasks: Vec<_> = batch
            .into_iter()
            .map(|signature| {
                let rpc_endpoint_clone = rpc_endpoint.to_string();
                let lookup_cache_clone = shared_lookup_cache.clone();

                tokio::spawn(async move {
                    process_single_signature(
                        worker_id,
                        &signature,
                        &rpc_endpoint_clone,
                        &lookup_cache_clone,
                    )
                    .await
                })
            })
            .collect();

        for task in tasks {
            if let Err(e) = task.await {
                eprintln!("Worker {}: Batch task failed: {}", worker_id, e);
            }
        }
    }

    Ok(())
}

async fn process_single_signature(
    _worker_id: usize,
    signature: &str,
    rpc_endpoint: &str,
    shared_lookup_cache: &Arc<lookuptable::LookupTableCache>,
) -> Result<()> {
    let (details, alt_accounts) =
        transaction::fetch_transaction_details(rpc_endpoint, signature).await?;

    if details.is_arbitrage {
        let details = transaction::fetch_accounts_from_alt(
            details,
            alt_accounts.clone(),
            shared_lookup_cache,
        )
        .await?;

        let mut excludes = HashSet::new();
        excludes.extend(constants::PROGRAMS_TO_WATCH.iter().map(|account| account.0));
        for account in &details.signer_token_balance_changes {
            excludes.insert(account.mint);
            excludes.insert(account.owner);
            excludes.insert(account.account);
        }

        let accounts: Vec<Pubkey> = details
            .all_accounts
            .iter()
            .filter(|account_info| {
                !account_info.is_signer
                    && account_info.is_writable
                    && !excludes.contains(&account_info.pubkey)
            })
            .map(|account_info| account_info.pubkey)
            .collect::<HashSet<_>>()
            .into_iter()
            .collect();

        let rpc_client = global::get_rpc_client();
        let account_data = rpc_client.get_multiple_accounts(&accounts).await?;
        let alt_pks = transaction::extract_pubkeys(alt_accounts);
        let mut pool_data: Vec<(Pubkey, AccountDataType, Option<Pubkey>)> = vec![];

        for (account_info_op, pubkey) in account_data.iter().zip(accounts.iter()) {
            match account_info_op {
                Some(account) => match parser::get_pool_type(&account) {
                    AccountDataType::Empty => {}
                    pool_type => {
                        let alt_address = find_alt_address(
                            shared_lookup_cache,
                            &alt_pks,
                            &pool_type.get_relevant_accounts(*pubkey),
                        )
                        .await;
                        pool_data.push((*pubkey, pool_type, alt_address));
                    }
                },
                None => {}
            }
        }

        if pool_data.len() < 2 {
            return Ok(());
        }

        for pool in pool_data {
            if !pool_index::has_pool(&pool.0) {
                POOL_QUEUE.push(pool);
            }
        }
    }

    Ok(())
}

async fn find_alt_address(
    lookup_table_cache: &Arc<lookuptable::LookupTableCache>,
    alt_pks: &[Pubkey],
    search: &[Pubkey],
) -> Option<Pubkey> {
    for alt_pk in alt_pks {
        match lookup_table_cache.get_lookup_table_accounts(&alt_pk).await {
            Ok(lookup_accounts) => {
                let set_pubkeys: HashSet<Pubkey> = lookup_accounts.into_iter().collect();
                for pk in search {
                    if set_pubkeys.contains(pk) {
                        return Some(*alt_pk);
                    }
                }
            }
            Err(_) => {}
        }
    }

    None
}

async fn begin_watch_unit(
    ws_endpoint: String,
    programs: &[(Pubkey, String, Option<String>, bool)],
) -> Result<()> {
    let watcher = SolanaTransactionWatcher::new(ws_endpoint);
    watcher.add_programs(programs.to_vec());

    let watcher_arc = Arc::new(watcher);
    let stats_watcher = watcher_arc.clone();
    let health_watcher = watcher_arc.clone();

    tokio::spawn(async move {
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(30));
        loop {
            interval.tick().await;
            let (programs, processed, cached) = stats_watcher.get_stats();
            let health_status = if stats_watcher.is_healthy() {
                "🟢 HEALTHY"
            } else {
                "🔴 UNHEALTHY"
            };
            info!(
                "📊 Stats - {} | Programs: {}, Processed TXs: {}, Cached: {}, Pools: {}, Pool Queue: {} ",
                health_status,
                programs,
                processed,
                cached,
                pool_index::count(),
                POOL_QUEUE.len(),
            );
        }
    });

    tokio::spawn(async move {
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(60));
        loop {
            interval.tick().await;
            if !health_watcher.is_healthy() {
                warn!("🚨 WebSocket connection is unhealthy for over 1 minute!");
            }
        }
    });

    match watcher_arc.start_watching_with_retry().await {
        Ok(_) => info!("✅ Transaction watcher finished successfully"),
        Err(e) => error!("❌ Transaction watcher failed permanently: {}", e),
    }

    Ok(())
}

pub async fn monitoring(
    conf: Config,
    command_op: Option<mpsc::UnboundedSender<WatcherCommand>>,
    chunk_size: usize,
) -> Result<()> {
    let rpc_endpoint = conf.rpc.url.to_string();

    tokio::spawn(async move {
        let _ = start_batch_processing(&rpc_endpoint, 10, 5).await;
    });

    if let Some(command) = command_op {
        tokio::spawn(async move {
            let _ = processor::run_process(command).await;
        });
    }

    for programs in constants::PROGRAMS_TO_WATCH.clone().chunks(chunk_size) {
        let websocket_url = conf.rpc.websocket_url.to_string();
        let programs = programs.to_vec();

        tokio::spawn(async move {
            if let Err(e) = begin_watch_unit(websocket_url, &programs).await {
                eprintln!("Error in watch unit: {:?}", e);
            }
        });
    }

    Ok(())
}
