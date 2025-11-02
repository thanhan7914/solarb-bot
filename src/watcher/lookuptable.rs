use anchor_client::solana_sdk::pubkey::Pubkey;
use anyhow::Result;
use base64::Engine;
use dashmap::DashMap;
use parking_lot::RwLock;
use serde_json::{Value, json};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tracing::warn;

#[derive(Debug, Clone)]
pub struct LookupTableCacheEntry {
    pub accounts: Vec<Pubkey>,
    pub cached_at: Instant,
    pub ttl: Duration,
}

impl LookupTableCacheEntry {
    pub fn new(accounts: Vec<Pubkey>) -> Self {
        Self {
            accounts,
            cached_at: Instant::now(),
            ttl: Duration::from_secs(2 * 60 * 60),
        }
    }

    pub fn is_expired(&self) -> bool {
        self.cached_at.elapsed() > self.ttl
    }

    pub fn remaining_ttl(&self) -> Duration {
        self.ttl.saturating_sub(self.cached_at.elapsed())
    }
}

#[derive(Debug)]
pub struct LookupTableCache {
    cache: Arc<DashMap<Pubkey, LookupTableCacheEntry>>,
    rpc_endpoint: String,
    stats: Arc<RwLock<CacheStats>>,
}

#[derive(Debug, Default, Clone)]
pub struct CacheStats {
    pub total_requests: u64,
    pub cache_hits: u64,
    pub cache_misses: u64,
    pub rpc_calls: u64,
    pub cache_size: usize,
}

impl CacheStats {
    pub fn hit_rate(&self) -> f64 {
        if self.total_requests == 0 {
            0.0
        } else {
            self.cache_hits as f64 / self.total_requests as f64
        }
    }
}

impl LookupTableCache {
    pub fn new(rpc_endpoint: String) -> Self {
        let cache = Arc::new(DashMap::new());

        let cache_clone = cache.clone();
        tokio::spawn(async move {
            Self::cleanup_task(cache_clone).await;
        });

        Self {
            cache,
            rpc_endpoint,
            stats: Arc::new(RwLock::new(CacheStats::default())),
        }
    }

    pub async fn get_lookup_table_accounts(
        &self,
        lookup_table_key: &Pubkey,
    ) -> Result<Vec<Pubkey>, Box<dyn std::error::Error>> {
        {
            let mut stats = self.stats.write();
            stats.total_requests += 1;
            stats.cache_size = self.cache.len();
        }

        if let Some(entry) = self.cache.get(lookup_table_key) {
            if !entry.is_expired() {
                {
                    let mut stats = self.stats.write();
                    stats.cache_hits += 1;
                }

                return Ok(entry.accounts.clone());
            } else {
                self.cache.remove(lookup_table_key);
            }
        }

        {
            let mut stats = self.stats.write();
            stats.cache_misses += 1;
            stats.rpc_calls += 1;
        }

        let accounts = self.fetch_lookup_table_from_rpc(lookup_table_key).await?;

        let cache_entry = LookupTableCacheEntry::new(accounts.clone());
        self.cache.insert(*lookup_table_key, cache_entry);

        Ok(accounts)
    }

    async fn fetch_lookup_table_from_rpc(
        &self,
        lookup_table_key: &Pubkey,
    ) -> Result<Vec<Pubkey>, Box<dyn std::error::Error>> {
        let client = reqwest::Client::new();

        let request = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "getAccountInfo",
            "params": [
                lookup_table_key.to_string(),
                {
                    "encoding": "base64",
                    "commitment": "confirmed"
                }
            ]
        });

        let response: Value = client
            .post(&self.rpc_endpoint)
            .json(&request)
            .send()
            .await?
            .json()
            .await?;

        let mut accounts = Vec::new();

        if let Some(result) = response.get("result") {
            if let Some(value) = result.get("value") {
                if value.is_null() {
                    return Ok(accounts);
                }

                if let Some(data) = value.get("data") {
                    if let Some(data_array) = data.as_array() {
                        if let Some(data_str) = data_array.get(0).and_then(|d| d.as_str()) {
                            if let Ok(decoded) =
                                base64::engine::general_purpose::STANDARD.decode(data_str)
                            {
                                accounts = self.parse_lookup_table_data(&decoded)?;
                            }
                        }
                    }
                }
            }
        }

        Ok(accounts)
    }

    fn parse_lookup_table_data(
        &self,
        data: &[u8],
    ) -> Result<Vec<Pubkey>, Box<dyn std::error::Error>> {
        let mut accounts = Vec::new();

        // Lookup table format:
        // - Discriminator: 8 bytes
        // - DeactivationSlot: 8 bytes
        // - LastExtendedSlot: 8 bytes
        // - LastExtendedSlotStartIndex: 1 byte
        // - Authority: 33 bytes (Option<Pubkey>)
        // - Padding: 7 bytes
        // - Addresses: Vec<Pubkey>

        if data.len() < 56 {
            return Ok(accounts);
        }

        let addresses_data = &data[56..];
        let num_addresses = addresses_data.len() / 32;

        for i in 0..num_addresses {
            let start = i * 32;
            let end = start + 32;

            if end <= addresses_data.len() {
                let pubkey_bytes: [u8; 32] = addresses_data[start..end]
                    .try_into()
                    .map_err(|_| "Invalid pubkey bytes")?;

                accounts.push(Pubkey::new_from_array(pubkey_bytes));
            }
        }

        Ok(accounts)
    }

    pub async fn preload_lookup_tables(
        &self,
        lookup_tables: &[Pubkey],
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut tasks = Vec::new();
        for &lookup_table in lookup_tables {
            let cache = self.clone();
            tasks.push(tokio::spawn(async move {
                if let Err(e) = cache.get_lookup_table_accounts(&lookup_table).await {
                    warn!("Failed to preload lookup table {}: {}", lookup_table, e);
                }
            }));
        }

        for task in tasks {
            let _ = task.await;
        }

        Ok(())
    }

    pub fn clear_cache(&self) {
        let count = self.cache.len();
        self.cache.clear();

        let mut stats = self.stats.write();
        *stats = CacheStats::default();
    }

    pub fn get_stats(&self) -> CacheStats {
        let mut stats = self.stats.read().clone();
        stats.cache_size = self.cache.len();
        stats
    }

    async fn cleanup_task(cache: Arc<DashMap<Pubkey, LookupTableCacheEntry>>) {
        let mut cleanup_interval = tokio::time::interval(Duration::from_secs(30 * 60));

        loop {
            cleanup_interval.tick().await;

            let mut expired_keys = Vec::new();

            for entry in cache.iter() {
                if entry.value().is_expired() {
                    expired_keys.push(*entry.key());
                }
            }

            let mut removed_count = 0;
            for key in expired_keys {
                if cache.remove(&key).is_some() {
                    removed_count += 1;
                }
            }
        }
    }

    fn clone(&self) -> Self {
        Self {
            cache: self.cache.clone(),
            rpc_endpoint: self.rpc_endpoint.clone(),
            stats: self.stats.clone(),
        }
    }
}
