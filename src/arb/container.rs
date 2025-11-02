use super::*;
use dashmap::{DashMap, Entry};
use parking_lot::Mutex;
use std::{
    collections::BinaryHeap,
    sync::{Arc, OnceLock},
};

#[derive(Clone)]
pub struct RouteStore {
    map: Arc<DashMap<u64, (i64, ProfitableRoute)>>,
    heap: Arc<Mutex<BinaryHeap<(i64, u64)>>>,
}

impl RouteStore {
    pub fn new() -> Self {
        Self {
            map: Arc::new(DashMap::new()),
            heap: Arc::new(Mutex::new(BinaryHeap::new())),
        }
    }

    #[inline]
    pub fn insert(&self, key: u64, weight: i64, route: ProfitableRoute) {
        self.map.insert(key, (weight, route));
        self.heap.lock().push((weight, key));
    }

    #[inline]
    pub fn smart_insert(&self, key: u64, weight: i64, route: ProfitableRoute) {
        match self.map.entry(key) {
            Entry::Occupied(mut occ) => {
                if weight > occ.get().0 {
                    occ.insert((weight, route));
                    self.heap.lock().push((weight, key));
                }
            }
            Entry::Vacant(vac) => {
                vac.insert((weight, route));
                self.heap.lock().push((weight, key));
            }
        }
    }

    pub fn pop_top_n(&self, n: usize) -> Vec<ProfitableRoute> {
        let mut out = Vec::with_capacity(n);

        while out.len() < n {
            let k = {
                let mut heap = self.heap.lock();
                match heap.pop() {
                    Some((_w, k)) => k,
                    None => break,
                }
            };

            if let Some((_key, (_weight, route))) = self.map.remove(&k) {
                out.push(route);
            } else {
                // key stale; skip
            }
        }
        out
    }

    pub fn drain(&self, n: usize) -> Vec<ProfitableRoute> {
        let mut out = Vec::with_capacity(n);
        let mut old_heap = {
            let mut guard = self.heap.lock();
            std::mem::take(&mut *guard)
        };

        while out.len() < n {
            let k = {
                match old_heap.pop() {
                    Some((_w, k)) => k,
                    None => break,
                }
            };

            if let Some((_key, (_weight, route))) = self.map.remove(&k) {
                out.push(route);
            } else {
            }
        }

        self.map.clear();

        out
    }

    pub fn drain_v1(&self, n: usize) -> Vec<ProfitableRoute> {
        let routes = self.pop_top_n(n);
        self.clear();
        routes
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.map.len()
    }

    pub fn clear(&self) {
        self.map.clear();
        self.heap.lock().clear();
    }

    pub fn clean_weight(&self) {
        let old_heap = {
            let mut guard = self.heap.lock();
            std::mem::take(&mut *guard)
        };

        let mut cleaned = BinaryHeap::with_capacity(old_heap.len());
        for (w, k) in old_heap.into_iter() {
            if let Some(entry) = self.map.get(&k) {
                let (stored_w, _route_ref) = entry.value();
                if *stored_w == w {
                    cleaned.push((w, k));
                }
            }
        }

        if !cleaned.is_empty() {
            let mut guard = self.heap.lock();
            guard.extend(cleaned.into_iter());
        }
    }
}

static ROUTE_STORE: OnceLock<RouteStore> = OnceLock::new();

impl RouteStore {
    pub fn global() -> &'static RouteStore {
        ROUTE_STORE.get_or_init(|| RouteStore::new())
    }
}

#[inline]
fn _to_scaled(weight: f64) -> i64 {
    (weight * 10_000.0) as i64
}

pub struct RouteContainer;

impl RouteContainer {
    #[inline]
    pub fn insert(route: ProfitableRoute) {
        let key = route.route.to_hash();
        // RouteStore::global().insert(key, _to_scaled(route.product), route);
        RouteStore::global().insert(key, route.route.profit, route);
    }

    #[inline]
    pub fn smart_insert(route: ProfitableRoute) {
        let key = route.route.to_mint_hash();
        // RouteStore::global().insert(key, _to_scaled(route.product), route);
        RouteStore::global().smart_insert(key, route.route.profit, route);
    }

    #[inline]
    pub fn insert_with_weight(route: ProfitableRoute, weight: i64) {
        let key = route.route.to_hash();
        RouteStore::global().insert(key, weight, route);
    }

    #[inline]
    pub fn pop_top_n(n: usize) -> Vec<ProfitableRoute> {
        RouteStore::global().pop_top_n(n)
    }

    pub fn drain(n: usize) -> Vec<ProfitableRoute> {
        RouteStore::global().drain(n)
    }

    #[inline]
    pub fn count() -> usize {
        RouteStore::global().len()
    }

    #[inline]
    pub fn clear() {
        RouteStore::global().clear();
    }

    #[inline]
    pub fn clean_weight() {
        RouteStore::global().clean_weight();
    }
}
