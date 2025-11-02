use super::*;
use crate::byte_reader::ByteReader;
use anchor_client::solana_sdk::pubkey::Pubkey;
use anyhow::{Result, anyhow};

pub mod oracle;
pub mod pda;
pub mod tick;
pub mod tick_array;

pub use tick_array::*;

#[derive(Debug, Clone)]
pub struct WhirlpoolRewardInfo {
    pub mint: Pubkey,
    pub vault: Pubkey,
    pub authority: Pubkey,
    pub emissions_per_second_x64: u128,
    pub growth_global_x64: u128,
}

impl WhirlpoolRewardInfo {
    pub fn deserialize(reader: &mut ByteReader) -> Result<Self> {
        Ok(Self {
            mint: reader.read_pubkey()?,
            vault: reader.read_pubkey()?,
            authority: reader.read_pubkey()?,
            emissions_per_second_x64: reader.read_u128()?,
            growth_global_x64: reader.read_u128()?,
        })
    }
}

#[derive(Debug, Clone)]
pub struct Whirlpool {
    pub whirlpools_config: Pubkey,
    pub whirlpool_bump: [u8; 1],
    pub tick_spacing: u16,
    pub fee_tier_index_seed: [u8; 2],
    pub fee_rate: u16,
    pub protocol_fee_rate: u16,
    pub liquidity: u128,
    pub sqrt_price: u128,
    pub tick_current_index: i32,
    pub protocol_fee_owed_a: u64,
    pub protocol_fee_owed_b: u64,
    pub token_mint_a: Pubkey,
    pub token_vault_a: Pubkey,
    pub fee_growth_global_a: u128,
    pub token_mint_b: Pubkey,
    pub token_vault_b: Pubkey,
    pub fee_growth_global_b: u128,
    pub reward_last_updated_timestamp: u64,
    pub reward_infos: [WhirlpoolRewardInfo; 3],
}

impl Whirlpool {
    pub fn deserialize(data: &[u8]) -> Result<Self> {
        if data.len() < 8 {
            return Err(anyhow!("Data too short for account discriminator"));
        }

        let mut reader = ByteReader::new(&data[8..]);

        Ok(Self {
            whirlpools_config: reader.read_pubkey()?,
            whirlpool_bump: reader.read_bytes_array::<1>()?,
            tick_spacing: reader.read_u16()?,
            fee_tier_index_seed: reader.read_bytes_array::<2>()?,
            fee_rate: reader.read_u16()?,
            protocol_fee_rate: reader.read_u16()?,
            liquidity: reader.read_u128()?,
            sqrt_price: reader.read_u128()?,
            tick_current_index: reader.read_i32()?,
            protocol_fee_owed_a: reader.read_u64()?,
            protocol_fee_owed_b: reader.read_u64()?,
            token_mint_a: reader.read_pubkey()?,
            token_vault_a: reader.read_pubkey()?,
            fee_growth_global_a: reader.read_u128()?,
            token_mint_b: reader.read_pubkey()?,
            token_vault_b: reader.read_pubkey()?,
            fee_growth_global_b: reader.read_u128()?,
            reward_last_updated_timestamp: reader.read_u64()?,
            reward_infos: [
                WhirlpoolRewardInfo::deserialize(&mut reader)?,
                WhirlpoolRewardInfo::deserialize(&mut reader)?,
                WhirlpoolRewardInfo::deserialize(&mut reader)?,
            ],
        })
    }

    pub fn fee_tier_index(&self) -> u16 {
        u16::from_le_bytes(self.fee_tier_index_seed)
    }

    pub fn is_initialized_with_adaptive_fee(&self) -> bool {
        self.fee_tier_index() != self.tick_spacing
    }

    pub fn get_price(&self) -> f64 {
        if self.sqrt_price == 0 {
            return 0.0;
        }
        // sqrt_price_x64 is Q64.64 format stored as u128
        // Convert to f64 and square it
        let sqrt_price = self.sqrt_price as f64 / Q64_64_SCALE;
        sqrt_price * sqrt_price
    }
}

#[derive(Debug, Clone)]
pub struct PositionRewardInfo {
    pub growth_inside_checkpoint: u128,
    pub amount_owed: u64,
}

impl PositionRewardInfo {
    pub fn deserialize(reader: &mut ByteReader) -> Result<Self> {
        Ok(Self {
            growth_inside_checkpoint: reader.read_u128()?,
            amount_owed: reader.read_u64()?,
        })
    }
}

#[derive(Debug, Clone)]
pub struct Position {
    pub whirlpool: Pubkey,
    pub position_mint: Pubkey,
    pub liquidity: u128,
    pub tick_lower_index: i32,
    pub tick_upper_index: i32,
    pub fee_growth_checkpoint_a: u128,
    pub fee_owed_a: u64,
    pub fee_growth_checkpoint_b: u128,
    pub fee_owed_b: u64,
    pub reward_infos: [PositionRewardInfo; 3],
}

impl Position {
    pub fn deserialize(data: &[u8]) -> Result<Self> {
        // Skip discriminator (8 bytes)
        if data.len() < 8 {
            return Err(anyhow!("Data too short for account discriminator"));
        }

        let mut reader = ByteReader::new(&data[8..]);

        Ok(Self {
            whirlpool: reader.read_pubkey()?,
            position_mint: reader.read_pubkey()?,
            liquidity: reader.read_u128()?,
            tick_lower_index: {
                let bytes = reader.read_bytes_array::<4>()?;
                i32::from_le_bytes(bytes)
            },
            tick_upper_index: {
                let bytes = reader.read_bytes_array::<4>()?;
                i32::from_le_bytes(bytes)
            },
            fee_growth_checkpoint_a: reader.read_u128()?,
            fee_owed_a: reader.read_u64()?,
            fee_growth_checkpoint_b: reader.read_u128()?,
            fee_owed_b: reader.read_u64()?,
            reward_infos: [
                PositionRewardInfo::deserialize(&mut reader)?,
                PositionRewardInfo::deserialize(&mut reader)?,
                PositionRewardInfo::deserialize(&mut reader)?,
            ],
        })
    }
}

#[derive(Debug, Clone)]
pub struct WhirlpoolsConfig {
    pub fee_authority: Pubkey,
    pub collect_protocol_fees_authority: Pubkey,
    pub reward_emissions_super_authority: Pubkey,
    pub default_protocol_fee_rate: u16,
}

impl WhirlpoolsConfig {
    pub fn deserialize(data: &[u8]) -> Result<Self> {
        if data.len() < 8 {
            return Err(anyhow!("Data too short for account discriminator"));
        }

        let mut reader = ByteReader::new(&data[8..]);

        Ok(Self {
            fee_authority: reader.read_pubkey()?,
            collect_protocol_fees_authority: reader.read_pubkey()?,
            reward_emissions_super_authority: reader.read_pubkey()?,
            default_protocol_fee_rate: reader.read_u16()?,
        })
    }
}

#[derive(Debug, Clone)]
pub struct FeeTier {
    pub whirlpools_config: Pubkey,
    pub tick_spacing: u16,
    pub default_fee_rate: u16,
}

impl FeeTier {
    pub fn deserialize(data: &[u8]) -> Result<Self> {
        if data.len() < 8 {
            return Err(anyhow!("Data too short for account discriminator"));
        }

        let mut reader = ByteReader::new(&data[8..]);

        Ok(Self {
            whirlpools_config: reader.read_pubkey()?,
            tick_spacing: reader.read_u16()?,
            default_fee_rate: reader.read_u16()?,
        })
    }
}

#[derive(Debug, Clone)]
pub struct PositionBundle {
    pub position_bundle_mint: Pubkey,
    pub position_bitmap: [u8; 32],
}

impl PositionBundle {
    pub fn deserialize(data: &[u8]) -> Result<Self> {
        if data.len() < 8 {
            return Err(anyhow!("Data too short for account discriminator"));
        }

        let mut reader = ByteReader::new(&data[8..]);

        Ok(Self {
            position_bundle_mint: reader.read_pubkey()?,
            position_bitmap: reader.read_bytes_array::<32>()?,
        })
    }
}

#[derive(Debug, Clone)]
pub struct TokenBadge {
    pub whirlpools_config: Pubkey,
    pub token_mint: Pubkey,
}

impl TokenBadge {
    pub fn deserialize(data: &[u8]) -> Result<Self> {
        if data.len() < 8 {
            return Err(anyhow!("Data too short for account discriminator"));
        }

        let mut reader = ByteReader::new(&data[8..]);

        Ok(Self {
            whirlpools_config: reader.read_pubkey()?,
            token_mint: reader.read_pubkey()?,
        })
    }
}
