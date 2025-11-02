use crate::{
    arb::{Hop, PoolType, Route, route::HopVecExt},
    global,
    streaming::{self, AccountDataType, global_data},
    token_program, wsol_mint,
};
use anchor_client::solana_sdk::pubkey::Pubkey;
use dashmap::DashMap;
use once_cell::sync::Lazy;
use std::{collections::HashSet, str::FromStr, sync::Arc};

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum TokenPoolType {
    Dlmm,
    Dammv2,
    PumpAmm,
    RaydiumAmm,
    RaydiumCpmm,
    RaydiumClmm,
    Whirlpool,
    Vertigo,
    Solfi,
}

#[derive(Debug, Clone)]
pub struct TokenPool {
    pub pool_type: TokenPoolType,
    pub mint_a: Pubkey,
    pub mint_b: Pubkey,
    pub pool: Pubkey,
}

impl TokenPool {
    pub fn is_native_token_pool(&self) -> bool {
        if let Some(AccountDataType::Account(mint_a_account)) =
            global_data::get_account(&self.mint_a)
        {
            let token_program = token_program();
            if let Some(AccountDataType::Account(mint_b_account)) =
                global_data::get_account(&self.mint_b)
            {
                return mint_a_account.owner == token_program
                    && mint_b_account.owner == token_program;
            }
        }

        false
    }

    pub fn is_pumpfun_pool(&self) -> bool {
        match self.pool_type {
            TokenPoolType::PumpAmm => true,
            _ => false,
        }
    }

    pub fn to_pool_type(&self) -> Option<PoolType> {
        match self.pool_type {
            TokenPoolType::PumpAmm => {
                if let Some(amm_pool) = streaming::PumpfunLoader::get_pump_amm(&self.pool) {
                    return Some(PoolType::Pump(self.pool, amm_pool));
                }
            }
            TokenPoolType::Dlmm => {
                if let Some(dlmm_pool) = streaming::MeteoraLoader::get_dlmm(&self.pool) {
                    return Some(PoolType::Meteora(self.pool, dlmm_pool));
                }
            }
            TokenPoolType::Dammv2 => {
                if let Some(damm) = streaming::MeteoraLoader::get_damm(&self.pool) {
                    return Some(PoolType::MeteoraDammv2(self.pool, damm));
                }
            }
            TokenPoolType::RaydiumAmm => {
                if let Some(clmm) = streaming::RaydiumLoader::get_amm(&self.pool) {
                    return Some(PoolType::RaydiumAmm(self.pool, clmm));
                }
            }
            TokenPoolType::RaydiumCpmm => {
                if let Some(cpmm) = streaming::RaydiumLoader::get_cpmm(&self.pool) {
                    return Some(PoolType::RaydiumCpmm(self.pool, cpmm));
                }
            }
            TokenPoolType::RaydiumClmm => {
                if let Some(clmm) = streaming::RaydiumLoader::get_clmm(&self.pool) {
                    return Some(PoolType::RaydiumClmm(self.pool, clmm));
                }
            }
            TokenPoolType::Whirlpool => {
                if let Some(whirlpool) = streaming::WhirlpoolLoader::get_whirlpool(&self.pool) {
                    return Some(PoolType::Whirlpool(self.pool, whirlpool));
                }
            }
            TokenPoolType::Vertigo => {
                if let Some(vertigo) = streaming::VertigoLoader::get_vertigo(&self.pool) {
                    return Some(PoolType::Vertigo(self.pool, vertigo));
                }
            }
            TokenPoolType::Solfi => {
                if let Some(solfi) = streaming::SolfiLoader::get_solfi(&self.pool) {
                    return Some(PoolType::Solfi(self.pool, solfi));
                }
            }
        }

        None
    }

    #[inline]
    pub fn other_mint(&self, from: Pubkey) -> Option<Pubkey> {
        if self.mint_a == from {
            Some(self.mint_b)
        } else if self.mint_b == from {
            Some(self.mint_a)
        } else {
            None
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MintPairKey(Pubkey, Pubkey);

impl MintPairKey {
    pub fn new(a: Pubkey, b: Pubkey) -> Self {
        if a < b { Self(a, b) } else { Self(b, a) }
    }
}

struct PoolIndex {
    by_pool: DashMap<Pubkey, Arc<TokenPool>>,
    by_mint: DashMap<Pubkey, Vec<Pubkey>>,
    by_pair: DashMap<MintPairKey, Vec<Pubkey>>,
    routes: DashMap<u64, Route>,
    route_by_mint: DashMap<Pubkey, Vec<Route>>,
}

impl PoolIndex {
    fn new() -> Self {
        Self {
            by_pool: DashMap::new(),
            by_mint: DashMap::new(),
            by_pair: DashMap::new(),
            routes: DashMap::new(),
            route_by_mint: DashMap::new(),
        }
    }

    pub fn insert(&self, pool: TokenPool) -> bool {
        let pool_key = pool.pool;

        if self.by_pool.contains_key(&pool_key) {
            return false;
        }

        let arc_pool = Arc::new(pool.clone());
        self.by_pool.insert(pool_key, arc_pool);
        self.by_mint.entry(pool.mint_a).or_default().push(pool_key);
        self.by_mint.entry(pool.mint_b).or_default().push(pool_key);

        let pair_key = MintPairKey::new(pool.mint_a, pool.mint_b);
        self.by_pair.entry(pair_key).or_default().push(pool_key);

        // let time = tokio::time::Instant::now();
        let routes = self._generate_routes();
        // println!("Generate routes {:?} - count {}", time.elapsed(), routes.len());
        for route in routes {
            let hash = route.to_hash();
            self.routes.insert(hash, route.clone());
            self._index_route(pool.mint_a, route.clone());
            self._index_route(pool.mint_b, route);
        }

        true
    }

    fn _index_route(&self, mint: Pubkey, route: Route) {
        if mint == wsol_mint() {
            return;
        }

        let hash = route.to_hash();
        let mut entry_a = self.route_by_mint.entry(mint).or_insert_with(Vec::new);

        if !entry_a.iter().any(|r| r.to_hash() == hash) {
            entry_a.push(route);
        }
    }

    pub fn remove(&self, pool_key: &Pubkey) -> Option<Arc<TokenPool>> {
        if let Some((_, pool)) = self.by_pool.remove(pool_key) {
            // Clean up mint indices
            if let Some(mut mint_a_pools) = self.by_mint.get_mut(&pool.mint_a) {
                mint_a_pools.retain(|&p| p != *pool_key);
            }
            if let Some(mut mint_b_pools) = self.by_mint.get_mut(&pool.mint_b) {
                mint_b_pools.retain(|&p| p != *pool_key);
            }

            // Clean up pair index
            let pair_key = MintPairKey::new(pool.mint_a, pool.mint_b);
            if let Some(mut pair_pools) = self.by_pair.get_mut(&pair_key) {
                pair_pools.retain(|&p| p != *pool_key);
            }

            Some(pool)
        } else {
            None
        }
    }

    fn _generate_routes(&self) -> Vec<Route> {
        let base_mint: Pubkey = *global::get_base_mint().as_ref();
        let bot_config = &global::get_config().bot;
        let max_hops: usize = bot_config.max_hops as usize;

        if max_hops == 0 {
            return Vec::new();
        }

        if self.by_mint.get(&base_mint).is_none() {
            return Vec::new();
        }

        // DFS state
        let mut routes: Vec<Route> = Vec::new();
        let mut used_pools: HashSet<Pubkey> = HashSet::new();
        let mut path: Vec<Hop> = Vec::with_capacity(max_hops);
        let mut seen_signatures: HashSet<u64> = HashSet::new();

        fn dfs(
            cur_mint: Pubkey,
            depth: usize,
            max_hops: usize,
            by_mint: &DashMap<Pubkey, Vec<Pubkey>>,
            by_pool: &DashMap<Pubkey, Arc<TokenPool>>,
            used_pools: &mut HashSet<Pubkey>,
            path: &mut Vec<Hop>,
            routes: &mut Vec<Route>,
            seen_signatures: &mut HashSet<u64>,
            base_mint: Pubkey,
        ) {
            if depth > 0 && cur_mint == base_mint {
                if depth <= max_hops {
                    let product = path.iter().fold(1.0_f64, |acc, h| acc * h.rate);
                    let sig = path.to_hash();
                    if seen_signatures.insert(sig) {
                        routes.push(Route {
                            start: base_mint,
                            hops: path.clone(),
                            product,
                        });
                    }
                }
                return;
            }

            if depth == max_hops {
                return;
            }

            let Some(pool_keys_guard) = by_mint.get(&cur_mint) else {
                return;
            };

            for pool_key in pool_keys_guard.iter() {
                if used_pools.contains(pool_key) {
                    continue;
                }

                let Some(pool_guard) = by_pool.get(pool_key) else {
                    continue;
                };
                let p: &TokenPool = &pool_guard;

                let Some(next_mint) = p.other_mint(cur_mint) else {
                    continue;
                };

                used_pools.insert(p.pool);
                path.push(Hop {
                    from: cur_mint,
                    to: next_mint,
                    pool: p.pool,
                    pool_type: p.pool_type,
                    rate: 1.0_f64,
                });

                dfs(
                    next_mint,
                    depth + 1,
                    max_hops,
                    by_mint,
                    by_pool,
                    used_pools,
                    path,
                    routes,
                    seen_signatures,
                    base_mint,
                );

                // backtrack
                path.pop();
                used_pools.remove(&p.pool);
            }
        }

        dfs(
            base_mint,
            0,
            max_hops,
            &self.by_mint,
            &self.by_pool,
            &mut used_pools,
            &mut path,
            &mut routes,
            &mut seen_signatures,
            base_mint,
        );

        routes
    }
}

static POOL_INDEX: Lazy<Arc<PoolIndex>> = Lazy::new(|| Arc::new(PoolIndex::new()));

pub fn add_pool(pool: TokenPool) -> bool {
    POOL_INDEX.insert(pool)
}

pub fn remove_pool(pool_key: &Pubkey) -> Option<Arc<TokenPool>> {
    POOL_INDEX.remove(pool_key)
}

pub fn find_by_mint(mint: &Pubkey) -> Vec<Pubkey> {
    POOL_INDEX
        .by_mint
        .get(mint)
        .map(|v| v.clone())
        .unwrap_or_default()
}

pub fn find_by_pair(mint_a: &Pubkey, mint_b: &Pubkey) -> Vec<Pubkey> {
    POOL_INDEX
        .by_pair
        .get(&MintPairKey::new(*mint_a, *mint_b))
        .map(|v| v.clone())
        .unwrap_or_default()
}

pub fn get(pool: &Pubkey) -> Option<Arc<TokenPool>> {
    POOL_INDEX.by_pool.get(pool).map(|v| v.clone())
}

pub fn get_all(pools: &[Pubkey]) -> Vec<Option<Arc<TokenPool>>> {
    pools.iter().map(|pool_key| get(pool_key)).collect()
}

pub fn pool_count() -> usize {
    POOL_INDEX.by_pool.len()
}

pub fn native_pool_count() -> usize {
    get_all_native_token_pools().len()
}

pub fn get_all_pools() -> Vec<Arc<TokenPool>> {
    POOL_INDEX
        .by_pool
        .iter()
        .map(|entry| entry.value().clone())
        .collect()
}

pub fn get_all_native_token_pools() -> Vec<Arc<TokenPool>> {
    POOL_INDEX
        .by_pool
        .iter()
        .map(|entry| entry.value().clone())
        .filter(|token| token.is_native_token_pool())
        .collect()
}

pub fn count_invalid_pools() -> i32 {
    let all_pools = get_all_pools();
    let mut invalid_count: i32 = 0;
    for pool in &all_pools {
        if pool.to_pool_type().is_none() {
            invalid_count += 1;
        }
    }

    invalid_count
}

pub fn get_relevent_pools(pool_pk: &Pubkey) -> Vec<Arc<TokenPool>> {
    let mut pools: Vec<Arc<TokenPool>> = vec![];
    if let Some(pool) = get(pool_pk) {
        let relevant_pool_pks = find_by_pair(&pool.mint_a, &pool.mint_b);

        for pk in relevant_pool_pks {
            if let Some(token_pool) = get(&pk) {
                pools.push(token_pool);
            }
        }
    }

    pools
}

pub fn count() -> usize {
    POOL_INDEX.by_pool.len()
}

pub fn has_pool(pool_key: &Pubkey) -> bool {
    POOL_INDEX.by_pool.contains_key(&pool_key)
}

pub fn routes_count() -> usize {
    POOL_INDEX.routes.len()
}

pub fn routes() -> Vec<Route> {
    POOL_INDEX
        .routes
        .iter()
        .map(|entry| entry.value().clone())
        .collect()
}

pub fn get_routes_by_mint(mint: &Pubkey) -> Vec<Route> {
    POOL_INDEX
        .route_by_mint
        .get(mint)
        .map(|v| v.clone())
        .unwrap_or_default()
}

pub fn is_reach_max() -> bool {
    let watcher_config = global::get_watcher_config();
    let max_pools: usize = watcher_config.max_pools as usize;
    let max_routes: usize = watcher_config.max_routes as usize;
    count() > max_pools || routes_count() > max_routes
}
