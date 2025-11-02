use crate::byte_reader::ByteReader;
use anchor_client::solana_sdk::pubkey::Pubkey;
use anyhow::{Result, anyhow};
use std::str::FromStr;

pub mod big_num;
pub mod fixed_point_64;
pub mod full_math;
pub mod liquidity_math;
pub mod pda;
pub mod q_math;
pub mod sqrt_price_math;
pub mod swap_math;
pub mod swap_util;
pub mod tick_array;
pub mod tick_array_bitmap_extension;
pub mod tick_array_bitmap_math;
pub mod tick_math;
pub mod unsafe_math;
pub mod util;

use big_num::{U256, U1024};
use tick_array::TickArrayState;
use tick_array_bitmap_extension::TickArrayBitmapExtension;

#[cfg(feature = "devnet")]
pub const RAYDIUM_CLMM_PROGRAM_ID: &str = "devi51mZmdwUJGU9hjN27vEz64Gps7uUefqxg27EAtH";

#[cfg(not(feature = "devnet"))]
pub const RAYDIUM_CLMM_PROGRAM_ID: &str = "CAMMCzo5YL8w4VFF8KVHrK22GGUsp5VTaW7grrKgrWqK";

pub const POOL_DISCRIMINATOR: [u8; 8] = [247, 237, 227, 245, 215, 195, 222, 70];

pub fn program_id() -> Pubkey {
    Pubkey::from_str(RAYDIUM_CLMM_PROGRAM_ID).unwrap()
}

#[derive(Debug, Clone)]
pub struct RewardInfo {
    pub reward_state: u8,
    pub open_time: u64,
    pub end_time: u64,
    pub last_update_time: u64,
    pub emissions_per_second_x64: u128,
    pub reward_total_emissioned: u64,
    pub reward_claimed: u64,
    pub token_mint: Pubkey,
    pub token_vault: Pubkey,
    pub authority: Pubkey,
    pub reward_growth_global_x64: u128,
}

#[derive(Debug, Clone)]
pub struct PoolState {
    pub bump: [u8; 1],
    pub amm_config: Pubkey,
    pub owner: Pubkey,
    pub token_mint_0: Pubkey,
    pub token_mint_1: Pubkey,
    pub token_vault_0: Pubkey,
    pub token_vault_1: Pubkey,
    pub observation_key: Pubkey,
    pub mint_decimals_0: u8,
    pub mint_decimals_1: u8,
    pub tick_spacing: u16,
    pub liquidity: u128,
    pub sqrt_price_x64: u128,
    pub tick_current: i32,
    pub padding3: u16,
    pub padding4: u16,
    pub fee_growth_global_0_x64: u128,
    pub fee_growth_global_1_x64: u128,
    pub protocol_fees_token_0: u64,
    pub protocol_fees_token_1: u64,
    pub swap_in_amount_token_0: u128,
    pub swap_out_amount_token_1: u128,
    pub swap_in_amount_token_1: u128,
    pub swap_out_amount_token_0: u128,
    pub status: u8,
    pub padding: [u8; 7],
    pub reward_infos: [RewardInfo; 3],
    pub tick_array_bitmap: [u64; 16],
    pub total_fees_token_0: u64,
    pub total_fees_claimed_token_0: u64,
    pub total_fees_token_1: u64,
    pub total_fees_claimed_token_1: u64,
    pub fund_fees_token_0: u64,
    pub fund_fees_token_1: u64,
    pub open_time: u64,
    pub recent_epoch: u64,
    pub padding1: [u64; 24],
    pub padding2: [u64; 32],
}

impl RewardInfo {
    pub fn deserialize(reader: &mut ByteReader) -> Result<Self> {
        let reward_state = reader.read_u8()?;
        let open_time = reader.read_u64()?;
        let end_time = reader.read_u64()?;
        let last_update_time = reader.read_u64()?;
        let emissions_per_second_x64 = reader.read_u128()?;
        let reward_total_emissioned = reader.read_u64()?;
        let reward_claimed = reader.read_u64()?;
        let token_mint = reader.read_pubkey()?;
        let token_vault = reader.read_pubkey()?;
        let authority = reader.read_pubkey()?;
        let reward_growth_global_x64 = reader.read_u128()?;

        Ok(RewardInfo {
            reward_state,
            open_time,
            end_time,
            last_update_time,
            emissions_per_second_x64,
            reward_total_emissioned,
            reward_claimed,
            token_mint,
            token_vault,
            authority,
            reward_growth_global_x64,
        })
    }
}

impl PoolState {
    /// Discriminator: [247,237,227,245,215,195,222,70]
    pub fn deserialize(data: &[u8]) -> Result<Self> {
        let mut reader = ByteReader::new(data);

        // Skip the discriminator (first 8 bytes)
        reader.skip(8)?;

        // Read bump (1 byte array)
        let bump = [reader.read_u8()?];

        // Read pubkeys (8 * 32 = 256 bytes)
        let amm_config = reader.read_pubkey()?;
        let owner = reader.read_pubkey()?;
        let token_mint_0 = reader.read_pubkey()?;
        let token_mint_1 = reader.read_pubkey()?;
        let token_vault_0 = reader.read_pubkey()?;
        let token_vault_1 = reader.read_pubkey()?;
        let observation_key = reader.read_pubkey()?;

        // Read mint decimals and tick_spacing (4 bytes total)
        let mint_decimals_0 = reader.read_u8()?;
        let mint_decimals_1 = reader.read_u8()?;
        let tick_spacing = reader.read_u16()?;

        // Read liquidity (16 bytes)
        let liquidity = reader.read_u128()?;

        // Read sqrt_price_x64 (16 bytes) - this is u128 according to IDL
        let sqrt_price_x64 = reader.read_u128()?;

        // Read tick_current and padding (8 bytes total)
        let tick_current_bytes = reader.read_u32()?;
        let tick_current = tick_current_bytes as i32; // Convert u32 to i32
        let padding3 = reader.read_u16()?;
        let padding4 = reader.read_u16()?;

        // Read fee growth globals (32 bytes total)
        let fee_growth_global_0_x64 = reader.read_u128()?;
        let fee_growth_global_1_x64 = reader.read_u128()?;

        // Read protocol fees (16 bytes total)
        let protocol_fees_token_0 = reader.read_u64()?;
        let protocol_fees_token_1 = reader.read_u64()?;

        // Read swap amounts (64 bytes total)
        let swap_in_amount_token_0 = reader.read_u128()?;
        let swap_out_amount_token_1 = reader.read_u128()?;
        let swap_in_amount_token_1 = reader.read_u128()?;
        let swap_out_amount_token_0 = reader.read_u128()?;

        // Read status and padding (8 bytes total)
        let status = reader.read_u8()?;
        let mut padding = [0u8; 7];
        for i in 0..7 {
            padding[i] = reader.read_u8()?;
        }

        // Read reward_infos array (3 RewardInfo structs)
        let mut reward_infos = Vec::new();
        for _ in 0..3 {
            reward_infos.push(RewardInfo::deserialize(&mut reader)?);
        }
        let reward_infos: [RewardInfo; 3] = reward_infos
            .try_into()
            .map_err(|_| anyhow!("Failed to convert reward_infos vector to array"))?;

        // Read tick_array_bitmap (16 u64 = 128 bytes)
        let mut tick_array_bitmap = [0u64; 16];
        for i in 0..16 {
            tick_array_bitmap[i] = reader.read_u64()?;
        }

        // Read fee totals (32 bytes total)
        let total_fees_token_0 = reader.read_u64()?;
        let total_fees_claimed_token_0 = reader.read_u64()?;
        let total_fees_token_1 = reader.read_u64()?;
        let total_fees_claimed_token_1 = reader.read_u64()?;

        // Read fund fees (16 bytes total)
        let fund_fees_token_0 = reader.read_u64()?;
        let fund_fees_token_1 = reader.read_u64()?;

        // Read timestamps (16 bytes total)
        let open_time = reader.read_u64()?;
        let recent_epoch = reader.read_u64()?;

        // Read padding1 (24 u64 = 192 bytes)
        let mut padding1 = [0u64; 24];
        for i in 0..24 {
            padding1[i] = reader.read_u64()?;
        }

        // Read padding2 (32 u64 = 256 bytes)
        let mut padding2 = [0u64; 32];
        for i in 0..32 {
            padding2[i] = reader.read_u64()?;
        }

        Ok(PoolState {
            bump,
            amm_config,
            owner,
            token_mint_0,
            token_mint_1,
            token_vault_0,
            token_vault_1,
            observation_key,
            mint_decimals_0,
            mint_decimals_1,
            tick_spacing,
            liquidity,
            sqrt_price_x64,
            tick_current,
            padding3,
            padding4,
            fee_growth_global_0_x64,
            fee_growth_global_1_x64,
            protocol_fees_token_0,
            protocol_fees_token_1,
            swap_in_amount_token_0,
            swap_out_amount_token_1,
            swap_in_amount_token_1,
            swap_out_amount_token_0,
            status,
            padding,
            reward_infos,
            tick_array_bitmap,
            total_fees_token_0,
            total_fees_claimed_token_0,
            total_fees_token_1,
            total_fees_claimed_token_1,
            fund_fees_token_0,
            fund_fees_token_1,
            open_time,
            recent_epoch,
            padding1,
            padding2,
        })
    }

    /// Get current price as f64 (token1/token0)
    /// Using safe Q-format conversion for u128 sqrt_price_x64
    pub fn get_price(&self) -> f64 {
        if self.sqrt_price_x64 == 0 {
            return 0.0;
        }
        // sqrt_price_x64 is Q64.64 format stored as u128
        // Convert to f64 and square it
        let sqrt_price = self.sqrt_price_x64 as f64 / q_math::Q64_64_SCALE;
        sqrt_price * sqrt_price
    }

    /// Get current price with decimal adjustment (more accurate for trading)
    pub fn get_price_with_decimals(&self, token0_decimals: u8, token1_decimals: u8) -> f64 {
        let raw_price = self.get_price();
        if raw_price == 0.0 {
            return 0.0;
        }
        let decimal_adjustment = 10f64.powi((token1_decimals as i32) - (token0_decimals as i32));
        raw_price * decimal_adjustment
    }

    /// Get price as rational number for exact calculations
    pub fn get_price_exact(&self) -> (u128, f64) {
        // For u128 sqrt_price, we can compute the square directly
        let sqrt_price_squared = (self.sqrt_price_x64 as u128).pow(2);
        (sqrt_price_squared, q_math::Q128_128_SCALE)
    }

    /// High precision price calculation
    pub fn get_price_precise(&self) -> f64 {
        if self.sqrt_price_x64 == 0 {
            return 0.0;
        }
        // More precise calculation using u128 arithmetic
        let sqrt_price_squared = (self.sqrt_price_x64 as u128).pow(2);
        // Use precise conversion for better accuracy
        q_math::u128_to_f64_precise(sqrt_price_squared) / q_math::Q128_128_SCALE
    }

    /// Get current tick price using precomputed constants
    pub fn get_tick_price(&self) -> f64 {
        tick_math::tick_to_price(self.tick_current)
    }

    /// Convert sqrt_price_x64 to tick (reverse calculation)
    pub fn sqrt_price_to_tick(&self) -> i32 {
        tick_math::sqrt_price_x128_to_tick(self.sqrt_price_x64)
    }

    /// Get total fees accumulated
    pub fn get_total_fees(&self) -> (u64, u64) {
        (
            self.protocol_fees_token_0 + self.fund_fees_token_0,
            self.protocol_fees_token_1 + self.fund_fees_token_1,
        )
    }

    pub fn get_tick_array_offset(&self, tick_array_start_index: i32) -> Result<usize> {
        let tick_array_offset_in_bitmap = tick_array_start_index
            / TickArrayState::tick_count(self.tick_spacing)
            + tick_array_bitmap_extension::TICK_ARRAY_BITMAP_SIZE;
        Ok(tick_array_offset_in_bitmap as usize)
    }

    pub fn get_first_initialized_tick_array(
        &self,
        tickarray_bitmap_extension: &Option<TickArrayBitmapExtension>,
        zero_for_one: bool,
    ) -> Result<(bool, i32)> {
        let (is_initialized, start_index) =
            if self.is_overflow_default_tickarray_bitmap(vec![self.tick_current]) {
                tickarray_bitmap_extension
                    .as_ref()
                    .unwrap()
                    .check_tick_array_is_initialized(
                        TickArrayState::get_array_start_index(self.tick_current, self.tick_spacing),
                        self.tick_spacing,
                    )?
            } else {
                tick_array_bitmap_math::check_current_tick_array_is_initialized(
                    U1024(self.tick_array_bitmap),
                    self.tick_current,
                    self.tick_spacing.into(),
                )?
            };
        if is_initialized {
            return Ok((true, start_index));
        }

        let next_start_index_op = self.next_initialized_tick_array_start_index(
            tickarray_bitmap_extension,
            TickArrayState::get_array_start_index(self.tick_current, self.tick_spacing),
            zero_for_one,
        )?;

        if let Some(next_start_index) = next_start_index_op {
            Ok((false, next_start_index))
        } else {
            Err(anyhow!("Can't get first tick array"))
        }
    }

    pub fn next_initialized_tick_array_start_index(
        &self,
        tickarray_bitmap_extension: &Option<TickArrayBitmapExtension>,
        mut last_tick_array_start_index: i32,
        zero_for_one: bool,
    ) -> Result<Option<i32>> {
        last_tick_array_start_index =
            TickArrayState::get_array_start_index(last_tick_array_start_index, self.tick_spacing);

        loop {
            let (is_found, start_index) =
                tick_array_bitmap_math::next_initialized_tick_array_start_index(
                    U1024(self.tick_array_bitmap),
                    last_tick_array_start_index,
                    self.tick_spacing,
                    zero_for_one,
                );
            if is_found {
                return Ok(Some(start_index));
            }
            last_tick_array_start_index = start_index;

            if tickarray_bitmap_extension.is_none() {
                return Err(anyhow!("MissingTickArrayBitmapExtensionAccount"));
            }

            let (is_found, start_index) = tickarray_bitmap_extension
                .as_ref()
                .unwrap()
                .next_initialized_tick_array_from_one_bitmap(
                    last_tick_array_start_index,
                    self.tick_spacing,
                    zero_for_one,
                )?;
            if is_found {
                return Ok(Some(start_index));
            }
            last_tick_array_start_index = start_index;

            if last_tick_array_start_index < tick_array::MIN_TICK
                || last_tick_array_start_index > tick_array::MAX_TICK
            {
                return Ok(None);
            }
        }
    }

    pub fn is_overflow_default_tickarray_bitmap(&self, tick_indexs: Vec<i32>) -> bool {
        let (min_tick_array_start_index_boundary, max_tick_array_index_boundary) =
            self.tick_array_start_index_range();
        for tick_index in tick_indexs {
            let tick_array_start_index =
                TickArrayState::get_array_start_index(tick_index, self.tick_spacing);
            if tick_array_start_index >= max_tick_array_index_boundary
                || tick_array_start_index < min_tick_array_start_index_boundary
            {
                return true;
            }
        }
        false
    }

    // the range of tick array start index that default tickarray bitmap can represent
    // if tick_spacing = 1, the result range is [-30720, 30720)
    pub fn tick_array_start_index_range(&self) -> (i32, i32) {
        // the range of ticks that default tickarrary can represent
        let mut max_tick_boundary =
            tick_array_bitmap_math::max_tick_in_tickarray_bitmap(self.tick_spacing);
        let mut min_tick_boundary = -max_tick_boundary;
        if max_tick_boundary > tick_array::MAX_TICK {
            max_tick_boundary =
                TickArrayState::get_array_start_index(tick_array::MAX_TICK, self.tick_spacing);
            // find the next tick array start index
            max_tick_boundary = max_tick_boundary + TickArrayState::tick_count(self.tick_spacing);
        }
        if min_tick_boundary < tick_array::MIN_TICK {
            min_tick_boundary =
                TickArrayState::get_array_start_index(tick_array::MIN_TICK, self.tick_spacing);
        }
        (min_tick_boundary, max_tick_boundary)
    }
}
