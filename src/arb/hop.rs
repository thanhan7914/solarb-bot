use super::*;
use crate::{pool_index::TokenPoolType, streaming};

impl Hop {
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
}

impl Route {
    #[inline]
    pub fn to_vec_owned(&self) -> Option<Vec<PoolType>> {
        let mut pools = Vec::with_capacity(self.hops.len());
        for hop in &self.hops {
            let pool_type = hop.to_pool_type()?;
            pools.push(pool_type);
        }
        Some(pools)
    }
}
