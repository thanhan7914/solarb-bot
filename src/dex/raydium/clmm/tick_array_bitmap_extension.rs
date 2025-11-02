use super::{
    big_num::U512,
    tick_array::{MAX_TICK, MIN_TICK, TICK_ARRAY_SIZE, TickArrayState},
};
use crate::{byte_reader::ByteReader, dex::raydium::clmm::tick_array_bitmap_math};
use anchor_client::solana_sdk::pubkey::Pubkey;
use anyhow::Result;

pub const TICK_ARRAY_BITMAP_SIZE: i32 = 512;
pub type TickArryBitmap = [u64; 8];

pub fn max_tick_in_tickarray_bitmap(tick_spacing: u16) -> i32 {
    i32::from(tick_spacing) * TICK_ARRAY_SIZE * TICK_ARRAY_BITMAP_SIZE
}

#[derive(Debug, Clone, Copy)]
pub struct TickArrayBitmapExtension {
    pub pool_id: Pubkey,
    pub positive_tick_array_bitmap: [[u64; 8]; 14],
    pub negative_tick_array_bitmap: [[u64; 8]; 14],
}

impl TickArrayBitmapExtension {
    pub fn deserialize(data: &[u8]) -> Result<Self> {
        let mut reader = ByteReader::new(data);

        // Skip the discriminator (first 8 bytes)
        reader.skip(8)?;

        // Read pool_id (32 bytes)
        let pool_id = reader.read_pubkey()?;

        // Read positive_tick_array_bitmap (14 arrays of 8 u64 = 14 * 8 * 8 = 896 bytes)
        let mut positive_tick_array_bitmap = [[0u64; 8]; 14];
        for i in 0..14 {
            for j in 0..8 {
                positive_tick_array_bitmap[i][j] = reader.read_u64()?;
            }
        }

        // Read negative_tick_array_bitmap (14 arrays of 8 u64 = 14 * 8 * 8 = 896 bytes)
        let mut negative_tick_array_bitmap = [[0u64; 8]; 14];
        for i in 0..14 {
            for j in 0..8 {
                negative_tick_array_bitmap[i][j] = reader.read_u64()?;
            }
        }

        Ok(TickArrayBitmapExtension {
            pool_id,
            positive_tick_array_bitmap,
            negative_tick_array_bitmap,
        })
    }

    pub fn is_positive_tick_array_initialized(&self, array_index: usize, bit_index: usize) -> bool {
        if array_index >= 14 || bit_index >= 512 {
            // 8 * 64 = 512 bits per array
            return false;
        }

        let word_index = bit_index / 64;
        let bit_position = bit_index % 64;

        if word_index >= 8 {
            return false;
        }

        (self.positive_tick_array_bitmap[array_index][word_index] & (1u64 << bit_position)) != 0
    }

    pub fn is_negative_tick_array_initialized(&self, array_index: usize, bit_index: usize) -> bool {
        if array_index >= 14 || bit_index >= 512 {
            // 8 * 64 = 512 bits per array
            return false;
        }

        let word_index = bit_index / 64;
        let bit_position = bit_index % 64;

        if word_index >= 8 {
            return false;
        }

        (self.negative_tick_array_bitmap[array_index][word_index] & (1u64 << bit_position)) != 0
    }

    pub fn set_positive_tick_array_initialized(&mut self, array_index: usize, bit_index: usize) {
        if array_index >= 14 || bit_index >= 512 {
            return;
        }

        let word_index = bit_index / 64;
        let bit_position = bit_index % 64;

        if word_index >= 8 {
            return;
        }

        self.positive_tick_array_bitmap[array_index][word_index] |= 1u64 << bit_position;
    }

    pub fn set_negative_tick_array_initialized(&mut self, array_index: usize, bit_index: usize) {
        if array_index >= 14 || bit_index >= 512 {
            return;
        }

        let word_index = bit_index / 64;
        let bit_position = bit_index % 64;

        if word_index >= 8 {
            return;
        }

        self.negative_tick_array_bitmap[array_index][word_index] |= 1u64 << bit_position;
    }

    pub fn count_positive_initialized_arrays(&self) -> u32 {
        let mut count = 0;
        for array in &self.positive_tick_array_bitmap {
            for &word in array {
                count += word.count_ones();
            }
        }
        count
    }

    pub fn count_negative_initialized_arrays(&self) -> u32 {
        let mut count = 0;
        for array in &self.negative_tick_array_bitmap {
            for &word in array {
                count += word.count_ones();
            }
        }
        count
    }

    fn get_bitmap_offset(tick_index: i32, tick_spacing: u16) -> Result<usize> {
        let ticks_in_one_bitmap = max_tick_in_tickarray_bitmap(tick_spacing);
        let mut offset = tick_index.abs() / ticks_in_one_bitmap - 1;
        if tick_index < 0 && tick_index.abs() % ticks_in_one_bitmap == 0 {
            offset -= 1;
        }
        Ok(offset as usize)
    }

    fn get_bitmap(&self, tick_index: i32, tick_spacing: u16) -> Result<(usize, TickArryBitmap)> {
        let offset = Self::get_bitmap_offset(tick_index, tick_spacing)?;
        if tick_index < 0 {
            Ok((offset, self.negative_tick_array_bitmap[offset]))
        } else {
            Ok((offset, self.positive_tick_array_bitmap[offset]))
        }
    }

    pub fn check_tick_array_is_initialized(
        &self,
        tick_array_start_index: i32,
        tick_spacing: u16,
    ) -> Result<(bool, i32)> {
        let (_, tickarray_bitmap) = self.get_bitmap(tick_array_start_index, tick_spacing)?;

        let tick_array_offset_in_bitmap =
            Self::tick_array_offset_in_bitmap(tick_array_start_index, tick_spacing);

        if U512(tickarray_bitmap).bit(tick_array_offset_in_bitmap as usize) {
            return Ok((true, tick_array_start_index));
        }
        Ok((false, tick_array_start_index))
    }

    /// Search for the first initialized bit in bitmap according to the direction, if found return ture and the tick array start index,
    /// if not, return false and tick boundary index
    pub fn next_initialized_tick_array_from_one_bitmap(
        &self,
        last_tick_array_start_index: i32,
        tick_spacing: u16,
        zero_for_one: bool,
    ) -> Result<(bool, i32)> {
        let multiplier = TickArrayState::tick_count(tick_spacing);
        let next_tick_array_start_index = if zero_for_one {
            last_tick_array_start_index - multiplier
        } else {
            last_tick_array_start_index + multiplier
        };
        let min_tick_array_start_index =
            TickArrayState::get_array_start_index(MIN_TICK, tick_spacing);
        let max_tick_array_start_index =
            TickArrayState::get_array_start_index(MAX_TICK, tick_spacing);

        if next_tick_array_start_index < min_tick_array_start_index
            || next_tick_array_start_index > max_tick_array_start_index
        {
            return Ok((false, next_tick_array_start_index));
        }

        let (_, tickarray_bitmap) = self.get_bitmap(next_tick_array_start_index, tick_spacing)?;

        Ok(Self::next_initialized_tick_array_in_bitmap(
            tickarray_bitmap,
            next_tick_array_start_index,
            tick_spacing,
            zero_for_one,
        ))
    }

    pub fn next_initialized_tick_array_in_bitmap(
        tickarray_bitmap: TickArryBitmap,
        next_tick_array_start_index: i32,
        tick_spacing: u16,
        zero_for_one: bool,
    ) -> (bool, i32) {
        let (bitmap_min_tick_boundary, bitmap_max_tick_boundary) =
            tick_array_bitmap_math::get_bitmap_tick_boundary(next_tick_array_start_index, tick_spacing);

        let tick_array_offset_in_bitmap =
            Self::tick_array_offset_in_bitmap(next_tick_array_start_index, tick_spacing);
        if zero_for_one {
            // tick from upper to lower
            // find from highter bits to lower bits
            let offset_bit_map = U512(tickarray_bitmap)
                << (TICK_ARRAY_BITMAP_SIZE - 1 - tick_array_offset_in_bitmap);

            let next_bit = if offset_bit_map.is_zero() {
                None
            } else {
                Some(u16::try_from(offset_bit_map.leading_zeros()).unwrap())
            };

            if next_bit.is_some() {
                let next_array_start_index = next_tick_array_start_index
                    - i32::from(next_bit.unwrap()) * TickArrayState::tick_count(tick_spacing);
                return (true, next_array_start_index);
            } else {
                // not found til to the end
                return (false, bitmap_min_tick_boundary);
            }
        } else {
            // tick from lower to upper
            // find from lower bits to highter bits
            let offset_bit_map = U512(tickarray_bitmap) >> tick_array_offset_in_bitmap;

            let next_bit = if offset_bit_map.is_zero() {
                None
            } else {
                Some(u16::try_from(offset_bit_map.trailing_zeros()).unwrap())
            };
            if next_bit.is_some() {
                let next_array_start_index = next_tick_array_start_index
                    + i32::from(next_bit.unwrap()) * TickArrayState::tick_count(tick_spacing);
                return (true, next_array_start_index);
            } else {
                // not found til to the end
                return (
                    false,
                    bitmap_max_tick_boundary - TickArrayState::tick_count(tick_spacing),
                );
            }
        }
    }

    pub fn tick_array_offset_in_bitmap(tick_array_start_index: i32, tick_spacing: u16) -> i32 {
        let m = tick_array_start_index.abs() % max_tick_in_tickarray_bitmap(tick_spacing);
        let mut tick_array_offset_in_bitmap = m / TickArrayState::tick_count(tick_spacing);
        if tick_array_start_index < 0 && m != 0 {
            tick_array_offset_in_bitmap = TICK_ARRAY_BITMAP_SIZE - tick_array_offset_in_bitmap;
        }
        tick_array_offset_in_bitmap
    }
}
