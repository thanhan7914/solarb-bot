use anchor_client::solana_sdk::pubkey::Pubkey;
use std::str::FromStr;

pub const METEORA_DLMM_PROGRAM_ID: &str = "LBUZKhRxPF3XUpBCjp4YzTKgLccjZhTSDM9YuVaPwxo";
use once_cell::sync::Lazy;
use std::collections::HashMap;

pub mod damm;

static STEP_RATIO_CACHE: Lazy<HashMap<u16, f64>> = Lazy::new(|| {
    let mut cache = HashMap::new();
    // Common bin steps in Meteora
    for bin_step in [1, 5, 10, 15, 20, 25, 50, 70, 75, 80, 100, 200, 500, 1000] {
        cache.insert(bin_step, 1.0 + (bin_step as f64) / 10_000.0);
    }
    cache
});

pub mod dlmm {
    use super::*;

    const DLMM_EVENT_AUTHORITY: &str = "D1ZN9Wj1fRSUQfCjhvnu1hqDMT7hzjzBBpi12nVniYD6";
    pub const POOL_DISCRIMINATOR: [u8; 8] = [33, 11, 49, 98, 181, 101, 177, 13];

    #[inline]
    pub fn program_id() -> Pubkey {
        Pubkey::from_str(METEORA_DLMM_PROGRAM_ID).unwrap()
    }

    #[inline]
    pub fn event_authority() -> Pubkey {
        Pubkey::from_str(DLMM_EVENT_AUTHORITY).unwrap()
    }
}

pub mod utils {
    use super::*;

    #[inline]
    fn fast_powi(base: f64, mut exp: i32) -> f64 {
        if exp == 0 {
            return 1.0;
        }

        let mut result = 1.0;
        let mut current_power = if exp < 0 {
            exp = -exp;
            1.0 / base
        } else {
            base
        };

        // Binary exponentiation
        while exp > 0 {
            if exp & 1 == 1 {
                result *= current_power;
            }
            current_power *= current_power;
            exp >>= 1;
        }

        result
    }

    #[inline]
    pub fn compute_price(active_id: i32, bin_step: u16) -> f64 {
        // Handle common cases quickly
        if active_id == 0 {
            return 1.0;
        }

        // Use lookup table for common bin steps
        if let Some(&step_ratio) = STEP_RATIO_CACHE.get(&bin_step) {
            return fast_powi(step_ratio, active_id);
        }

        // Fallback for uncommon bin steps
        let step_ratio = 1.0 + (bin_step as f64) / 10_000.0;
        fast_powi(step_ratio, active_id)
    }

    #[inline]
    pub fn derive_event_authority() -> Pubkey {
        let program_id = Pubkey::from_str(METEORA_DLMM_PROGRAM_ID).unwrap();

        // Event authority PDA
        let (event_authority, _) =
            Pubkey::find_program_address(&[b"__event_authority"], &program_id);

        event_authority
    }
}
