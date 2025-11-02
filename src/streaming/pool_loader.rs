use crate::{
    arb::PoolType,
    pool_index::{self, TokenPoolType},
};
use anchor_client::solana_sdk::pubkey::Pubkey;

#[inline]
pub fn retrieve_pool_type(pool_pk: &Pubkey) -> Option<Box<PoolType>> {
    if let Some(token_pool) = pool_index::get(pool_pk) {
        match token_pool.pool_type {
            TokenPoolType::PumpAmm => {
                if let Some(amm_pool) = super::PumpfunLoader::get_pump_amm(&token_pool.pool) {
                    Some(Box::new(PoolType::Pump(token_pool.pool, amm_pool)))
                } else {
                    None
                }
            }
            TokenPoolType::Dlmm => {
                if let Some(dlmm_pool) = super::MeteoraLoader::get_dlmm(&token_pool.pool) {
                    Some(Box::new(PoolType::Meteora(token_pool.pool, dlmm_pool)))
                } else {
                    None
                }
            }
            TokenPoolType::Dammv2 => {
                if let Some(damm) = super::MeteoraLoader::get_damm(&token_pool.pool) {
                    Some(Box::new(PoolType::MeteoraDammv2(token_pool.pool, damm)))
                } else {
                    None
                }
            }
            TokenPoolType::RaydiumAmm => {
                if let Some(clmm) = super::RaydiumLoader::get_amm(&token_pool.pool) {
                    Some(Box::new(PoolType::RaydiumAmm(token_pool.pool, clmm)))
                } else {
                    None
                }
            }
            TokenPoolType::RaydiumCpmm => {
                if let Some(cpmm) = super::RaydiumLoader::get_cpmm(&token_pool.pool) {
                    Some(Box::new(PoolType::RaydiumCpmm(token_pool.pool, cpmm)))
                } else {
                    None
                }
            }
            TokenPoolType::RaydiumClmm => {
                if let Some(clmm) = super::RaydiumLoader::get_clmm(&token_pool.pool) {
                    Some(Box::new(PoolType::RaydiumClmm(token_pool.pool, clmm)))
                } else {
                    None
                }
            }
            TokenPoolType::Whirlpool => {
                if let Some(whirlpool) = super::WhirlpoolLoader::get_whirlpool(&token_pool.pool) {
                    Some(Box::new(PoolType::Whirlpool(token_pool.pool, whirlpool)))
                } else {
                    None
                }
            }
            TokenPoolType::Vertigo => {
                if let Some(vertigo) = super::VertigoLoader::get_vertigo(&token_pool.pool) {
                    Some(Box::new(PoolType::Vertigo(token_pool.pool, vertigo)))
                } else {
                    None
                }
            }
            TokenPoolType::Solfi => {
                if let Some(solfi) = super::SolfiLoader::get_solfi(&token_pool.pool) {
                    Some(Box::new(PoolType::Solfi(token_pool.pool, solfi)))
                } else {
                    None
                }
            }
        }
    } else {
        None
    }
}

#[inline]
pub fn get_pool_price(pool_pk: &Pubkey, base_mint: &Pubkey) -> Option<f64> {
    if let Some(token_pool) = pool_index::get(pool_pk) {
        match token_pool.pool_type {
            TokenPoolType::PumpAmm => {
                if let Some(amm_pool) = super::PumpfunLoader::get_pump_amm(&token_pool.pool) {
                    Some(PoolType::Pump(token_pool.pool, amm_pool).get_price(base_mint).0)
                } else {
                    None
                }
            }
            TokenPoolType::Dlmm => {
                if let Some(dlmm_pool) = super::MeteoraLoader::get_dlmm(&token_pool.pool) {
                    Some(PoolType::Meteora(token_pool.pool, dlmm_pool).get_price(base_mint).0)
                } else {
                    None
                }
            }
            TokenPoolType::Dammv2 => {
                if let Some(damm) = super::MeteoraLoader::get_damm(&token_pool.pool) {
                    Some(PoolType::MeteoraDammv2(token_pool.pool, damm).get_price(base_mint).0)
                } else {
                    None
                }
            }
            TokenPoolType::RaydiumAmm => {
                if let Some(clmm) = super::RaydiumLoader::get_amm(&token_pool.pool) {
                    Some(PoolType::RaydiumAmm(token_pool.pool, clmm).get_price(base_mint).0)
                } else {
                    None
                }
            }
            TokenPoolType::RaydiumCpmm => {
                if let Some(cpmm) = super::RaydiumLoader::get_cpmm(&token_pool.pool) {
                    Some(PoolType::RaydiumCpmm(token_pool.pool, cpmm).get_price(base_mint).0)
                } else {
                    None
                }
            }
            TokenPoolType::RaydiumClmm => {
                if let Some(clmm) = super::RaydiumLoader::get_clmm(&token_pool.pool) {
                    Some(PoolType::RaydiumClmm(token_pool.pool, clmm).get_price(base_mint).0)
                } else {
                    None
                }
            }
            TokenPoolType::Whirlpool => {
                if let Some(whirlpool) = super::WhirlpoolLoader::get_whirlpool(&token_pool.pool) {
                    Some(PoolType::Whirlpool(token_pool.pool, whirlpool).get_price(base_mint).0)
                } else {
                    None
                }
            }
            TokenPoolType::Vertigo => {
                if let Some(vertigo) = super::VertigoLoader::get_vertigo(&token_pool.pool) {
                    Some(PoolType::Vertigo(token_pool.pool, vertigo).get_price(base_mint).0)
                } else {
                    None
                }
            }
            TokenPoolType::Solfi => {
                if let Some(solfi) = super::SolfiLoader::get_solfi(&token_pool.pool) {
                    Some(PoolType::Solfi(token_pool.pool, solfi).get_price(base_mint).0)
                } else {
                    None
                }
            }
        }
    } else {
        None
    }
}
