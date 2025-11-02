use anyhow::{Result, anyhow};
use dashmap::DashMap;
use futures_util::StreamExt;
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::{Duration, Instant};
use tokio::sync::mpsc;
use tokio::time::interval;
use tokio_stream::wrappers::ReceiverStream;
use tracing::{debug, error, info, warn};
use yellowstone_grpc_proto::geyser::{
    CommitmentLevel, SubscribeRequest, SubscribeRequestFilterAccounts, SubscribeUpdate,
    geyser_client::GeyserClient,
};

use tonic::Request;
use tonic::metadata::MetadataValue;
use tonic::service::Interceptor;

#[derive(Clone)]
struct TokenInterceptor {
    token: String,
}

impl Interceptor for TokenInterceptor {
    fn call(&mut self, mut req: Request<()>) -> Result<Request<()>, tonic::Status> {
        req.metadata_mut().insert(
            "x-token",
            MetadataValue::try_from(self.token.clone()).unwrap(),
        );
        Ok(req)
    }
}

#[derive(Debug, Clone)]
pub struct GrpcConfig {
    pub endpoint: String,
    pub x_token: Option<String>,
    pub batch_interval_ms: u64, // Batch updates every X ms
    pub max_batch_size: usize,  // Max changes before force update
    pub connection_timeout_ms: u64,
}

impl Default for GrpcConfig {
    fn default() -> Self {
        Self {
            endpoint: "http://127.0.0.1:10000".to_string(),
            x_token: None,
            batch_interval_ms: 100,
            max_batch_size: 50,
            connection_timeout_ms: 15000,
        }
    }
}

#[derive(Debug, Default)]
struct PendingChanges {
    accounts_to_add: Vec<String>,
    accounts_to_remove: Vec<String>,
    programs_to_add: Vec<String>,
    programs_to_remove: Vec<String>,
    change_count: usize,
}

impl PendingChanges {
    fn is_empty(&self) -> bool {
        self.change_count == 0
    }

    fn clear(&mut self) {
        self.accounts_to_add.clear();
        self.accounts_to_remove.clear();
        self.programs_to_add.clear();
        self.programs_to_remove.clear();
        self.change_count = 0;
    }

    fn add_account(&mut self, account: String) {
        self.accounts_to_add.push(account);
        self.change_count += 1;
    }

    fn remove_account(&mut self, account: String) {
        self.accounts_to_remove.push(account);
        self.change_count += 1;
    }

    fn add_program(&mut self, program: String) {
        self.programs_to_add.push(program);
        self.change_count += 1;
    }
}

#[derive(Debug)]
pub struct SubscriptionState {
    pub accounts: DashMap<String, ()>,
    pub programs: DashMap<String, ()>,
    pub is_running: AtomicBool,
    pub last_update_slot: AtomicU64,
    pub pending_changes: parking_lot::Mutex<PendingChanges>, // Fast mutex
    pub last_batch_time: std::sync::Mutex<Instant>,
}

impl Default for SubscriptionState {
    fn default() -> Self {
        Self {
            accounts: DashMap::new(),
            programs: DashMap::new(),
            is_running: AtomicBool::new(false),
            last_update_slot: AtomicU64::new(0),
            pending_changes: parking_lot::Mutex::new(PendingChanges::default()),
            last_batch_time: std::sync::Mutex::new(Instant::now()),
        }
    }
}

/// Commands
#[derive(Debug, Clone)]
pub enum SubscriptionCommand {
    FlushBatch, // Force flush pending changes
    Stop,
}

pub struct GrpcClient {
    config: GrpcConfig,
    pub subscription_state: Arc<SubscriptionState>,
    subscription_control: Option<mpsc::UnboundedSender<SubscriptionCommand>>,
}

impl GrpcClient {
    pub fn new(config: GrpcConfig) -> Self {
        Self {
            config,
            subscription_state: Arc::new(SubscriptionState::default()),
            subscription_control: None,
        }
    }

    pub fn add_account(&self, account: String) -> bool {
        // Check if already exists
        if self.subscription_state.accounts.contains_key(&account) {
            return false; // No change needed
        }

        // Add to actual state
        let was_new = self
            .subscription_state
            .accounts
            .insert(account.clone(), ())
            .is_none();

        if was_new {
            // Add to pending changes for batching
            let mut pending = self.subscription_state.pending_changes.lock();
            pending.add_account(account);

            if self.should_flush_batch(&pending) {
                drop(pending); // Release lock before flush
                self.flush_batch_now();
            }
        }

        was_new
    }

    pub fn remove_account(&self, account: String) -> bool {
        let was_removed = self.subscription_state.accounts.remove(&account).is_some();

        if was_removed {
            let mut pending = self.subscription_state.pending_changes.lock();
            pending.remove_account(account);

            if self.should_flush_batch(&pending) {
                drop(pending);
                self.flush_batch_now();
            }
        }

        was_removed
    }

    pub fn add_program(&self, program: String) -> bool {
        if self.subscription_state.programs.contains_key(&program) {
            return false;
        }

        let was_new = self
            .subscription_state
            .programs
            .insert(program.clone(), ())
            .is_none();

        if was_new {
            let mut pending = self.subscription_state.pending_changes.lock();
            pending.add_program(program);

            if self.should_flush_batch(&pending) {
                drop(pending);
                self.flush_batch_now();
            }
        }

        was_new
    }

    pub fn batch_update(
        &self,
        add_accounts: Vec<String>,
        remove_accounts: Vec<String>,
        add_programs: Vec<String>,
        remove_programs: Vec<String>,
    ) -> bool {
        let mut any_changes = false;
        let mut pending = self.subscription_state.pending_changes.lock();

        // Process all changes in one lock
        for account in add_accounts {
            if self
                .subscription_state
                .accounts
                .insert(account.clone(), ())
                .is_none()
            {
                pending.add_account(account);
                any_changes = true;
            }
        }

        for account in remove_accounts {
            if self.subscription_state.accounts.remove(&account).is_some() {
                pending.remove_account(account);
                any_changes = true;
            }
        }

        for program in add_programs {
            if self
                .subscription_state
                .programs
                .insert(program.clone(), ())
                .is_none()
            {
                pending.add_program(program);
                any_changes = true;
            }
        }

        for program in remove_programs {
            if self.subscription_state.programs.remove(&program).is_some() {
                pending.programs_to_remove.push(program);
                pending.change_count += 1;
                any_changes = true;
            }
        }

        if any_changes {
            drop(pending);
            self.flush_batch_now();
        }

        any_changes
    }

    fn should_flush_batch(&self, pending: &PendingChanges) -> bool {
        // Flush if too many changes
        if pending.change_count >= self.config.max_batch_size {
            return true;
        }

        // Flush if too much time passed
        if let Ok(last_time) = self.subscription_state.last_batch_time.lock() {
            let elapsed = last_time.elapsed();
            if elapsed >= Duration::from_millis(self.config.batch_interval_ms) {
                return true;
            }
        }

        false
    }

    fn flush_batch_now(&self) {
        if let Some(sender) = &self.subscription_control {
            let _ = sender.send(SubscriptionCommand::FlushBatch);
        }
    }

    pub async fn start_subscription<F>(&mut self, processor: F) -> Result<()>
    where
        F: Fn(&SubscribeUpdate, Instant) + Send + Sync + Clone + 'static,
    {
        let (cmd_tx, cmd_rx) = mpsc::unbounded_channel();
        self.subscription_control = Some(cmd_tx);

        let config = self.config.clone();
        let subscription_state = Arc::clone(&self.subscription_state);

        subscription_state.is_running.store(true, Ordering::Relaxed);

        self.start_batch_timer().await;

        tokio::spawn(async move {
            Self::run_subscription(config, subscription_state, processor, cmd_rx).await;
        });

        Ok(())
    }

    async fn start_batch_timer(&self) {
        let subscription_control = self.subscription_control.clone();
        let interval_ms = self.config.batch_interval_ms;

        if let Some(sender) = subscription_control {
            tokio::spawn(async move {
                let mut interval = interval(Duration::from_millis(interval_ms));

                loop {
                    interval.tick().await;
                    let _ = sender.send(SubscriptionCommand::FlushBatch);
                }
            });
        }
    }

    async fn run_subscription<F>(
        config: GrpcConfig,
        subscription_state: Arc<SubscriptionState>,
        processor: F,
        mut cmd_rx: mpsc::UnboundedReceiver<SubscriptionCommand>,
    ) where
        F: Fn(&SubscribeUpdate, Instant) + Send + Sync + Clone + 'static,
    {
        loop {
            match Self::run_single_subscription(
                &config,
                Arc::clone(&subscription_state),
                processor.clone(),
                &mut cmd_rx,
            )
            .await
            {
                Ok(()) => break,
                Err(e) => {
                    error!("Subscription failed: {}, retrying...", e);
                    tokio::time::sleep(Duration::from_millis(1000)).await;
                }
            }
        }
    }

    async fn run_single_subscription<F>(
        config: &GrpcConfig,
        subscription_state: Arc<SubscriptionState>,
        processor: F,
        cmd_rx: &mut mpsc::UnboundedReceiver<SubscriptionCommand>,
    ) -> Result<()>
    where
        F: Fn(&SubscribeUpdate, Instant) + Send + Sync + 'static,
    {
        info!("Starting subscription...");

        let channel = tonic::transport::Channel::from_shared(config.endpoint.clone())?
            .timeout(Duration::from_millis(config.connection_timeout_ms))
            .keep_alive_while_idle(true)
            .connect()
            .await?;

        let mut client = GeyserClient::with_interceptor(
            channel,
            TokenInterceptor {
                token: config.x_token.clone().unwrap_or_default(),
            },
        );
        let (stream_tx, mut stream_rx) = mpsc::channel(8);

        // Send initial request
        let initial_request = Self::build_request(&subscription_state);
        stream_tx.send(initial_request).await?;

        let request_stream = ReceiverStream::new(stream_rx);
        let mut response_stream = client.subscribe(request_stream).await?.into_inner();

        info!("Subscription started");

        let mut update_count = 0u64;

        loop {
            tokio::select! {
                // HIGHEST PRIORITY: Process updates
                message = response_stream.next() => {
                    let receive_time = Instant::now();
                    match message {
                        Some(Ok(update)) => {
                            update_count += 1;

                            // Update slot
                            if let Some(slot) = Self::extract_slot(&update) {
                                subscription_state.last_update_slot.store(slot, Ordering::Relaxed);
                            }

                            // Process immediately
                            if update.update_oneof.is_some() {
                                processor(&update, receive_time);
                            }

                            // Debug info every 1000 updates
                            if update_count % 1000 == 0 {
                                debug!("Processed {} updates", update_count);
                            }
                        }
                        Some(Err(e)) => {
                            error!("Stream error: {}", e);
                            return Err(anyhow!("Stream error: {}", e));
                        }
                        None => {
                            warn!("Stream ended");
                            return Err(anyhow!("Stream ended"));
                        }
                    }
                }

                // LOWER PRIORITY: Handle commands
                cmd = cmd_rx.recv() => {
                    match cmd {
                        Some(SubscriptionCommand::FlushBatch) => {
                            // Check if there are actually pending changes
                            let has_changes = {
                                let pending = subscription_state.pending_changes.lock();
                                !pending.is_empty()
                            };

                            if has_changes {
                                let new_request = Self::build_request(&subscription_state);

                                // Apply pending changes to actual subscription
                                Self::apply_pending_changes(&subscription_state);

                                if stream_tx.send(new_request).await.is_err() {
                                    return Err(anyhow!("Failed to send batch update"));
                                }

                                // Update last flush time
                                if let Ok(mut last_time) = subscription_state.last_batch_time.lock() {
                                    *last_time = Instant::now();
                                }

                                debug!("Flushed batch changes");
                            }
                        }
                        Some(SubscriptionCommand::Stop) => {
                            info!("Stopping subscription");
                            return Ok(());
                        }
                        None => return Ok(()),
                    }
                }
            }
        }
    }

    fn apply_pending_changes(subscription_state: &Arc<SubscriptionState>) {
        let mut pending = subscription_state.pending_changes.lock();
        // Changes already applied to main state in add/remove methods
        // Just clear pending list
        pending.clear();
    }

    fn build_request(subscription_state: &Arc<SubscriptionState>) -> SubscribeRequest {
        let mut accounts_filter = HashMap::new();

        // Get current accounts
        if !subscription_state.accounts.is_empty() {
            let accounts: Vec<String> = subscription_state
                .accounts
                .iter()
                .map(|entry| entry.key().clone())
                .collect();

            if !accounts.is_empty() {
                accounts_filter.insert(
                    "accounts".to_string(),
                    SubscribeRequestFilterAccounts {
                        account: accounts,
                        owner: vec![],
                        filters: vec![],
                    },
                );
            }
        }

        // Get current programs
        if !subscription_state.programs.is_empty() {
            let programs: Vec<String> = subscription_state
                .programs
                .iter()
                .map(|entry| entry.key().clone())
                .collect();

            if !programs.is_empty() {
                accounts_filter.insert(
                    "programs".to_string(),
                    SubscribeRequestFilterAccounts {
                        account: vec![],
                        owner: programs,
                        filters: vec![],
                    },
                );
            }
        }

        SubscribeRequest {
            slots: HashMap::new(),
            accounts: accounts_filter,
            transactions: HashMap::new(),
            transactions_status: HashMap::new(),
            blocks: HashMap::new(),
            blocks_meta: HashMap::new(),
            entry: HashMap::new(),
            commitment: Some(CommitmentLevel::Processed as i32),
            accounts_data_slice: vec![],
            ping: None,
        }
    }

    fn extract_slot(update: &SubscribeUpdate) -> Option<u64> {
        match &update.update_oneof {
            Some(yellowstone_grpc_proto::geyser::subscribe_update::UpdateOneof::Account(
                account_update,
            )) => Some(account_update.slot),
            Some(yellowstone_grpc_proto::geyser::subscribe_update::UpdateOneof::Slot(
                slot_update,
            )) => Some(slot_update.slot),
            _ => None,
        }
    }

    pub fn get_metrics(&self) -> SubscriptionMetrics {
        let pending = self.subscription_state.pending_changes.lock();

        SubscriptionMetrics {
            accounts_count: self.subscription_state.accounts.len(),
            programs_count: self.subscription_state.programs.len(),
            pending_changes: pending.change_count,
            last_update_slot: self
                .subscription_state
                .last_update_slot
                .load(Ordering::Relaxed),
            is_running: self.subscription_state.is_running.load(Ordering::Relaxed),
        }
    }

    pub fn force_immediate_update(&self) {
        self.flush_batch_now();
    }

    pub async fn stop(&self) -> Result<()> {
        self.subscription_state
            .is_running
            .store(false, Ordering::Relaxed);
        if let Some(sender) = &self.subscription_control {
            sender.send(SubscriptionCommand::Stop)?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct SubscriptionMetrics {
    pub accounts_count: usize,
    pub programs_count: usize,
    pub pending_changes: usize,
    pub last_update_slot: u64,
    pub is_running: bool,
}
