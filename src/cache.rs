use dashmap::DashMap;
use std::{
    hash::Hash,
    time::{Duration, Instant},
};

#[derive(Debug, Clone)]
enum CacheEntry<V> {
    Temporary(V, Instant, Duration), // value, inserted_at, ttl
    Permanent(V),
}

impl<V> CacheEntry<V> {
    fn value(&self) -> &V {
        match self {
            CacheEntry::Temporary(v, _, _) => v,
            CacheEntry::Permanent(v) => v,
        }
    }

    fn is_expired(&self) -> bool {
        match self {
            CacheEntry::Temporary(_, inserted, ttl) => inserted.elapsed() >= *ttl,
            CacheEntry::Permanent(_) => false, // Never expires
        }
    }

    fn ttl_remaining(&self) -> Option<Duration> {
        match self {
            CacheEntry::Temporary(_, inserted, ttl) => {
                let elapsed = inserted.elapsed();
                if elapsed < *ttl {
                    Some(*ttl - elapsed)
                } else {
                    Some(Duration::ZERO) // Expired
                }
            }
            CacheEntry::Permanent(_) => None, // No TTL
        }
    }
}

#[derive(Debug)]
pub struct Cache<K, V>
where
    K: Eq + Hash + Clone,
    V: Clone,
{
    inner: DashMap<K, CacheEntry<V>>,
}

impl<K, V> Cache<K, V>
where
    K: Eq + Hash + Clone,
    V: Clone,
{
    pub fn new() -> Self {
        Self {
            inner: DashMap::new(),
        }
    }

    pub fn set(&self, key: K, value: V, ttl: Duration) {
        self.inner
            .insert(key, CacheEntry::Temporary(value, Instant::now(), ttl));
    }

    pub fn forever(&self, key: K, value: V) {
        self.inner.insert(key, CacheEntry::Permanent(value));
    }

    pub fn get(&self, key: &K) -> Option<V> {
        self.inner.get(key).and_then(|entry| {
            if entry.value().is_expired() {
                // Lazy eviction of expired item
                drop(entry);
                self.inner.remove(key);
                None
            } else {
                Some(entry.value().value().clone())
            }
        })
    }

    pub fn has(&self, key: &K) -> bool {
        self.get(key).is_some()
    }

    pub fn is_permanent(&self, key: &K) -> bool {
        self.inner
            .get(key)
            .map(|entry| matches!(entry.value(), CacheEntry::Permanent(_)))
            .unwrap_or(false)
    }

    pub fn ttl_remaining(&self, key: &K) -> Option<Duration> {
        self.inner.get(key)?.ttl_remaining()
    }

    pub fn forget(&self, key: &K) {
        self.inner.remove(key);
    }

    pub fn purge_expired(&self) {
        self.inner.retain(|_, entry| !entry.is_expired());
    }

    pub fn stats(&self) -> CacheStats {
        let total_entries = self.inner.len();
        let mut expired_count = 0;
        let mut permanent_count = 0;

        // Count expired and permanent entries
        for entry in self.inner.iter() {
            match entry.value() {
                CacheEntry::Permanent(_) => permanent_count += 1,
                CacheEntry::Temporary(_, inserted, ttl) => {
                    if inserted.elapsed() >= *ttl {
                        expired_count += 1;
                    }
                }
            }
        }

        CacheStats {
            total_entries,
            expired_entries: expired_count,
            permanent_entries: permanent_count,
            fresh_entries: total_entries - expired_count,
        }
    }

    pub fn clear(&self) {
        self.inner.clear();
    }

    pub fn len(&self) -> usize {
        self.inner.len()
    }

    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    pub fn get_or_insert_with<F>(&self, key: K, ttl: Duration, f: F) -> V
    where
        F: FnOnce() -> V,
    {
        if let Some(value) = self.get(&key) {
            value
        } else {
            let value = f();
            self.set(key, value.clone(), ttl);
            value
        }
    }

    pub fn get_or_insert_forever<F>(&self, key: K, f: F) -> V
    where
        F: FnOnce() -> V,
    {
        if let Some(value) = self.get(&key) {
            value
        } else {
            let value = f();
            self.forever(key, value.clone());
            value
        }
    }

    pub async fn get_or_insert_with_async<F, Fut>(&self, key: K, ttl: Duration, f: F) -> V
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = V>,
    {
        if let Some(value) = self.get(&key) {
            value
        } else {
            let value = f().await;
            self.set(key, value.clone(), ttl);
            value
        }
    }

    pub fn update_ttl(&self, key: &K, new_ttl: Duration) -> bool {
        if let Some(mut entry) = self.inner.get_mut(key) {
            match entry.value_mut() {
                CacheEntry::Temporary(value, _, _ttl) => {
                    *entry = CacheEntry::Temporary(value.clone(), Instant::now(), new_ttl);
                    true
                }
                CacheEntry::Permanent(_) => false, // Can't change permanent entry TTL
            }
        } else {
            false
        }
    }

    pub fn make_temporary(&self, key: &K, ttl: Duration) -> bool {
        if let Some(mut entry) = self.inner.get_mut(key) {
            match entry.value() {
                CacheEntry::Permanent(value) => {
                    let value = value.clone();
                    *entry = CacheEntry::Temporary(value, Instant::now(), ttl);
                    true
                }
                CacheEntry::Temporary(_, _, _) => false, // Already temporary
            }
        } else {
            false
        }
    }

    pub fn make_permanent(&self, key: &K) -> bool {
        if let Some(mut entry) = self.inner.get_mut(key) {
            match entry.value() {
                CacheEntry::Temporary(value, _, _) => {
                    let value = value.clone();
                    *entry = CacheEntry::Permanent(value);
                    true
                }
                CacheEntry::Permanent(_) => false, // Already permanent
            }
        } else {
            false
        }
    }
}

impl<K, V> Default for Cache<K, V>
where
    K: Eq + Hash + Clone,
    V: Clone,
{
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
pub struct CacheStats {
    pub total_entries: usize,
    pub expired_entries: usize,
    pub permanent_entries: usize,
    pub fresh_entries: usize,
}

impl CacheStats {
    pub fn hit_ratio(&self) -> f64 {
        if self.total_entries == 0 {
            0.0
        } else {
            self.fresh_entries as f64 / self.total_entries as f64
        }
    }

    pub fn permanent_ratio(&self) -> f64 {
        if self.total_entries == 0 {
            0.0
        } else {
            self.permanent_entries as f64 / self.total_entries as f64
        }
    }
}
