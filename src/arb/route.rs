use super::*;
use crate::streaming::global_data;
use ahash::AHasher;
use std::hash::{Hash, Hasher};

impl Route {
    pub fn to_hash(&self) -> u64 {
        self.hops.to_hash()
    }
}

impl SwapRoutes {
    pub fn to_hash(&self) -> u64 {
        let mut h = AHasher::default();
        for hop in &self.routes {
            hop.get_address().hash(&mut h);
        }
        h.finish()
    }

    pub fn to_mint_hash(&self) -> u64 {
        let mut h = AHasher::default();
        for hop in &self.routes {
            let (mint_x, mint_y) = hop.get_mints();
            let (mint_a, mint_b) = if mint_x < mint_y {
                (mint_x, mint_y)
            } else {
                (mint_y, mint_x)
            };
            mint_a.hash(&mut h);
            mint_b.hash(&mut h);
        }
        h.finish()
    }
}

impl Hop {
    #[inline]
    pub fn get_price(&self) -> f64 {
        if let Some((mint_a, atob)) = global_data::get_price(&self.pool) {
            if &self.from == &mint_a {
                atob
            } else {
                1f64 / atob
            }
        } else {
            0f64
        }
    }
}

pub trait HopVecExt {
    fn to_hash(&self) -> u64;
    fn product(&self) -> f64;
}

impl HopVecExt for Vec<Hop> {
    fn to_hash(&self) -> u64 {
        let mut h = AHasher::default();
        for hop in self {
            hop.pool.hash(&mut h);
            hop.from.hash(&mut h);
            hop.to.hash(&mut h);
        }
        h.finish()
    }

    fn product(&self) -> f64 {
        let mut p: f64 = 1f64;
        for hop in self {
            p *= hop.get_price();
        }

        p
    }
}
