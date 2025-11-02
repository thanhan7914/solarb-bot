use crate::byte_reader::ByteReader;
use anchor_client::solana_sdk::pubkey::Pubkey;
use anyhow::{Result, anyhow};
use std::str::FromStr;

pub mod util;
pub mod pda;

const VERTIGO_ID: &str = "vrTGoBuy5rYSxAfV3jaRJWHH6nN9WK4NRExGxsk1bCJ";
pub const POOL_DISCRIMINATOR: [u8; 8] = [241, 154, 109, 4, 17, 177, 109, 188];

pub fn program_id() -> Pubkey {
    Pubkey::from_str(VERTIGO_ID).unwrap()
}

#[derive(Debug, Clone)]
pub struct FeeParams {
    pub normalization_period: u64,
    pub decay: f64,
    pub reference: u64,
    pub royalties_bps: u16,
    pub privileged_swapper: Option<Pubkey>,
}

#[derive(Debug, Clone)]
pub struct Pool {
    pub enabled: bool,
    pub owner: Pubkey,
    pub mint_a: Pubkey,
    pub mint_b: Pubkey,
    pub token_a_reserves: u128,
    pub token_b_reserves: u128,
    pub shift: u128,
    pub royalties: u64,
    pub vertigo_fees: u64,
    pub bump: u8,
    pub fee_params: FeeParams,
}

impl Pool {
    pub fn deserialize(data: &[u8]) -> Result<Self> {
        let mut reader = ByteReader::new(data);

        // Skip discriminator (8 bytes)
        reader.skip(8)?;

        let enabled = reader.read_u8()? != 0;
        let owner = reader.read_pubkey()?;
        let mint_a = reader.read_pubkey()?;
        let mint_b = reader.read_pubkey()?;
        let token_a_reserves = reader.read_u128()?;
        let token_b_reserves = reader.read_u128()?;
        let shift = reader.read_u128()?;
        let royalties = reader.read_u64()?;
        let vertigo_fees = reader.read_u64()?;
        let bump = reader.read_u8()?;

        // Read FeeParams
        let fee_params = FeeParams {
            normalization_period: reader.read_u64()?,
            decay: f64::from_bits(reader.read_u64()?),
            reference: reader.read_u64()?,
            royalties_bps: reader.read_u16()?,
            privileged_swapper: {
                let has_privileged = reader.read_u8()? != 0;
                if has_privileged {
                    Some(reader.read_pubkey()?)
                } else {
                    None
                }
            },
        };

        Ok(Pool {
            enabled,
            owner,
            mint_a,
            mint_b,
            token_a_reserves,
            token_b_reserves,
            shift,
            royalties,
            vertigo_fees,
            bump,
            fee_params,
        })
    }

    pub fn calculate_buy_amount_out(&self, amount_a_in: u64, current_slot: u64) -> Result<u64> {
        if !self.enabled {
            return Err(anyhow!("Pool is disabled"));
        }

        if amount_a_in == 0 {
            return Ok(0);
        }

        let fee_rate = self.calculate_fee_rate(current_slot);
        let fee_amount = ((amount_a_in as u128) * (fee_rate as u128) / 10000) as u64;
        let amount_after_fee = amount_a_in.saturating_sub(fee_amount);

        // AMM constant product formula: x * y = k
        // Với shift: (x + shift) * (y + shift) = k
        let reserve_a = self.token_a_reserves;
        let reserve_b = self.token_b_reserves;
        let shift = self.shift;

        // k = (reserve_a + shift) * (reserve_b + shift)
        let k = (reserve_a + shift)
            .checked_mul(reserve_b + shift)
            .ok_or_else(|| anyhow!("Math overflow in k calculation"))?;

        // new_reserve_a = reserve_a + amount_after_fee
        let new_reserve_a = reserve_a
            .checked_add(amount_after_fee as u128)
            .ok_or_else(|| anyhow!("Math overflow in new_reserve_a calculation"))?;

        // new_reserve_b = k / (new_reserve_a + shift) - shift
        let new_reserve_b_with_shift = k
            .checked_div(new_reserve_a + shift)
            .ok_or_else(|| anyhow!("Division by zero"))?;

        if new_reserve_b_with_shift <= shift {
            return Err(anyhow!("Insufficient liquidity"));
        }

        let new_reserve_b = new_reserve_b_with_shift - shift;

        // amount_out = reserve_b - new_reserve_b
        let amount_out = reserve_b
            .checked_sub(new_reserve_b)
            .ok_or_else(|| anyhow!("Insufficient output"))?;

        if amount_out > u64::MAX as u128 {
            return Err(anyhow!("Amount out exceeds u64 max"));
        }

        Ok(amount_out as u64)
    }

    pub fn calculate_sell_amount_out(&self, amount_b_in: u64, current_slot: u64) -> Result<u64> {
        if !self.enabled {
            return Err(anyhow!("Pool is disabled"));
        }

        if amount_b_in == 0 {
            return Ok(0);
        }

        let fee_rate = self.calculate_fee_rate(current_slot);
        let fee_amount = ((amount_b_in as u128) * (fee_rate as u128) / 10000) as u64;
        let amount_after_fee = amount_b_in.saturating_sub(fee_amount);

        // AMM constant product formula với shift
        let reserve_a = self.token_a_reserves;
        let reserve_b = self.token_b_reserves;
        let shift = self.shift;

        // k = (reserve_a + shift) * (reserve_b + shift)
        let k = (reserve_a + shift)
            .checked_mul(reserve_b + shift)
            .ok_or_else(|| anyhow!("Math overflow in k calculation"))?;

        // new_reserve_b = reserve_b + amount_after_fee
        let new_reserve_b = reserve_b
            .checked_add(amount_after_fee as u128)
            .ok_or_else(|| anyhow!("Math overflow in new_reserve_b calculation"))?;

        // new_reserve_a = k / (new_reserve_b + shift) - shift
        let new_reserve_a_with_shift = k
            .checked_div(new_reserve_b + shift)
            .ok_or_else(|| anyhow!("Division by zero"))?;

        if new_reserve_a_with_shift <= shift {
            return Err(anyhow!("Insufficient liquidity"));
        }

        let new_reserve_a = new_reserve_a_with_shift - shift;

        // amount_out = reserve_a - new_reserve_a
        let amount_out = reserve_a
            .checked_sub(new_reserve_a)
            .ok_or_else(|| anyhow!("Insufficient output"))?;

        if amount_out > u64::MAX as u128 {
            return Err(anyhow!("Amount out exceeds u64 max"));
        }

        Ok(amount_out as u64)
    }

    fn calculate_fee_rate(&self, current_slot: u64) -> u16 {
        let reference_slot = self.fee_params.reference;
        let normalization_period = self.fee_params.normalization_period;
        let decay = self.fee_params.decay;
        let royalties_bps = self.fee_params.royalties_bps;

        if current_slot <= reference_slot {
            return 10000; // 100% fee
        }

        let slots_passed = current_slot - reference_slot;

        if slots_passed >= normalization_period {
            return royalties_bps; // Base fee
        }

        // Exponential decay: fee = base_fee + (10000 - base_fee) * exp(-decay * slots_passed / normalization_period)
        let normalized_time = slots_passed as f64 / normalization_period as f64;
        let decay_factor = (-decay * normalized_time).exp();
        let dynamic_fee = royalties_bps as f64 + (10000.0 - royalties_bps as f64) * decay_factor;

        dynamic_fee.round() as u16
    }

    pub fn calculate_buy_amount_in(&self, amount_b_out: u64, current_slot: u64) -> Result<u64> {
        if !self.enabled {
            return Err(anyhow!("Pool is disabled"));
        }

        if amount_b_out == 0 {
            return Ok(0);
        }

        let reserve_a = self.token_a_reserves;
        let reserve_b = self.token_b_reserves;
        let shift = self.shift;

        if amount_b_out as u128 >= reserve_b {
            return Err(anyhow!("Insufficient liquidity"));
        }

        // k = (reserve_a + shift) * (reserve_b + shift)
        let k = (reserve_a + shift)
            .checked_mul(reserve_b + shift)
            .ok_or_else(|| anyhow!("Math overflow in k calculation"))?;

        // new_reserve_b = reserve_b - amount_b_out
        let new_reserve_b = reserve_b - amount_b_out as u128;

        // new_reserve_a = k / (new_reserve_b + shift) - shift
        let new_reserve_a_with_shift = k
            .checked_div(new_reserve_b + shift)
            .ok_or_else(|| anyhow!("Division by zero"))?;

        if new_reserve_a_with_shift <= shift {
            return Err(anyhow!("Insufficient liquidity"));
        }

        let new_reserve_a = new_reserve_a_with_shift - shift;

        // amount_in_before_fee = new_reserve_a - reserve_a
        let amount_in_before_fee = new_reserve_a
            .checked_sub(reserve_a)
            .ok_or_else(|| anyhow!("Invalid calculation"))?;

        // Tính fee và amount_in thực tế
        let fee_rate = self.calculate_fee_rate(current_slot);

        // amount_in_before_fee = amount_in * (1 - fee_rate/10000)
        // => amount_in = amount_in_before_fee / (1 - fee_rate/10000)
        let fee_multiplier = 10000 - fee_rate as u128;
        let amount_in = (amount_in_before_fee * 10000)
            .checked_div(fee_multiplier)
            .ok_or_else(|| anyhow!("Division by zero in fee calculation"))?;

        if amount_in > u64::MAX as u128 {
            return Err(anyhow!("Amount in exceeds u64 max"));
        }

        Ok(amount_in as u64)
    }

    pub fn calculate_sell_amount_in(&self, amount_a_out: u64, current_slot: u64) -> Result<u64> {
        if !self.enabled {
            return Err(anyhow!("Pool is disabled"));
        }

        if amount_a_out == 0 {
            return Ok(0);
        }

        let reserve_a = self.token_a_reserves;
        let reserve_b = self.token_b_reserves;
        let shift = self.shift;

        if amount_a_out as u128 >= reserve_a {
            return Err(anyhow!("Insufficient liquidity"));
        }

        // k = (reserve_a + shift) * (reserve_b + shift)
        let k = (reserve_a + shift)
            .checked_mul(reserve_b + shift)
            .ok_or_else(|| anyhow!("Math overflow in k calculation"))?;

        // new_reserve_a = reserve_a - amount_a_out
        let new_reserve_a = reserve_a - amount_a_out as u128;

        // new_reserve_b = k / (new_reserve_a + shift) - shift
        let new_reserve_b_with_shift = k
            .checked_div(new_reserve_a + shift)
            .ok_or_else(|| anyhow!("Division by zero"))?;

        if new_reserve_b_with_shift <= shift {
            return Err(anyhow!("Insufficient liquidity"));
        }

        let new_reserve_b = new_reserve_b_with_shift - shift;

        // amount_in_before_fee = new_reserve_b - reserve_b
        let amount_in_before_fee = new_reserve_b
            .checked_sub(reserve_b)
            .ok_or_else(|| anyhow!("Invalid calculation"))?;

        // Tính fee và amount_in thực tế
        let fee_rate = self.calculate_fee_rate(current_slot);
        let fee_multiplier = 10000 - fee_rate as u128;
        let amount_in = (amount_in_before_fee * 10000)
            .checked_div(fee_multiplier)
            .ok_or_else(|| anyhow!("Division by zero in fee calculation"))?;

        if amount_in > u64::MAX as u128 {
            return Err(anyhow!("Amount in exceeds u64 max"));
        }

        Ok(amount_in as u64)
    }

    pub fn get_price_a_in_b(&self) -> f64 {
        if self.token_a_reserves == 0 {
            return 0.0;
        }

        let reserve_a = self.token_a_reserves as f64;
        let reserve_b = self.token_b_reserves as f64;
        let shift = self.shift as f64;

        // Price = (reserve_b + shift) / (reserve_a + shift)
        (reserve_b + shift) / (reserve_a + shift)
    }

    pub fn get_price_b_in_a(&self) -> f64 {
        if self.token_b_reserves == 0 {
            return 0.0;
        }

        let reserve_a = self.token_a_reserves as f64;
        let reserve_b = self.token_b_reserves as f64;
        let shift = self.shift as f64;

        // Price = (reserve_a + shift) / (reserve_b + shift)
        (reserve_a + shift) / (reserve_b + shift)
    }
}
