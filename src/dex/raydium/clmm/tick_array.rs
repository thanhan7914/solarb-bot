use super::big_num::U128;
use crate::byte_reader::ByteReader;
use anchor_client::solana_sdk::pubkey::Pubkey;
use anyhow::{Result, anyhow};

pub const TICK_ARRAY_SIZE_USIZE: usize = 60;
pub const TICK_ARRAY_SIZE: i32 = 60;
/// The minimum tick
pub const MIN_TICK: i32 = -443636;
/// The minimum tick
pub const MAX_TICK: i32 = -MIN_TICK;

/// The minimum value that can be returned from #get_sqrt_price_at_tick. Equivalent to get_sqrt_price_at_tick(MIN_TICK)
pub const MIN_SQRT_PRICE_X64: u128 = 4295048016;
/// The maximum value that can be returned from #get_sqrt_price_at_tick. Equivalent to get_sqrt_price_at_tick(MAX_TICK)
pub const MAX_SQRT_PRICE_X64: u128 = 79226673521066979257578248091;

// Number 64, encoded as a U128
const NUM_64: U128 = U128([64, 0]);

const BIT_PRECISION: u32 = 16;

/// Calculates 1.0001^(tick/2) as a U64.64 number representing
/// the square root of the ratio of the two assets (token_1/token_0)
///
/// Calculates result as a U64.64
/// Each magic factor is `2^64 / (1.0001^(2^(i - 1)))` for i in `[0, 18)`.
///
/// Throws if |tick| > MAX_TICK
///
/// # Arguments
/// * `tick` - Price tick
///
pub fn get_sqrt_price_at_tick(tick: i32) -> Result<u128, anchor_lang::error::Error> {
    let abs_tick = tick.abs() as u32;

    // i = 0
    let mut ratio = if abs_tick & 0x1 != 0 {
        U128([0xfffcb933bd6fb800, 0])
    } else {
        // 2^64
        U128([0, 1])
    };
    // i = 1
    if abs_tick & 0x2 != 0 {
        ratio = (ratio * U128([0xfff97272373d4000, 0])) >> NUM_64
    };
    // i = 2
    if abs_tick & 0x4 != 0 {
        ratio = (ratio * U128([0xfff2e50f5f657000, 0])) >> NUM_64
    };
    // i = 3
    if abs_tick & 0x8 != 0 {
        ratio = (ratio * U128([0xffe5caca7e10f000, 0])) >> NUM_64
    };
    // i = 4
    if abs_tick & 0x10 != 0 {
        ratio = (ratio * U128([0xffcb9843d60f7000, 0])) >> NUM_64
    };
    // i = 5
    if abs_tick & 0x20 != 0 {
        ratio = (ratio * U128([0xff973b41fa98e800, 0])) >> NUM_64
    };
    // i = 6
    if abs_tick & 0x40 != 0 {
        ratio = (ratio * U128([0xff2ea16466c9b000, 0])) >> NUM_64
    };
    // i = 7
    if abs_tick & 0x80 != 0 {
        ratio = (ratio * U128([0xfe5dee046a9a3800, 0])) >> NUM_64
    };
    // i = 8
    if abs_tick & 0x100 != 0 {
        ratio = (ratio * U128([0xfcbe86c7900bb000, 0])) >> NUM_64
    };
    // i = 9
    if abs_tick & 0x200 != 0 {
        ratio = (ratio * U128([0xf987a7253ac65800, 0])) >> NUM_64
    };
    // i = 10
    if abs_tick & 0x400 != 0 {
        ratio = (ratio * U128([0xf3392b0822bb6000, 0])) >> NUM_64
    };
    // i = 11
    if abs_tick & 0x800 != 0 {
        ratio = (ratio * U128([0xe7159475a2caf000, 0])) >> NUM_64
    };
    // i = 12
    if abs_tick & 0x1000 != 0 {
        ratio = (ratio * U128([0xd097f3bdfd2f2000, 0])) >> NUM_64
    };
    // i = 13
    if abs_tick & 0x2000 != 0 {
        ratio = (ratio * U128([0xa9f746462d9f8000, 0])) >> NUM_64
    };
    // i = 14
    if abs_tick & 0x4000 != 0 {
        ratio = (ratio * U128([0x70d869a156f31c00, 0])) >> NUM_64
    };
    // i = 15
    if abs_tick & 0x8000 != 0 {
        ratio = (ratio * U128([0x31be135f97ed3200, 0])) >> NUM_64
    };
    // i = 16
    if abs_tick & 0x10000 != 0 {
        ratio = (ratio * U128([0x9aa508b5b85a500, 0])) >> NUM_64
    };
    // i = 17
    if abs_tick & 0x20000 != 0 {
        ratio = (ratio * U128([0x5d6af8dedc582c, 0])) >> NUM_64
    };
    // i = 18
    if abs_tick & 0x40000 != 0 {
        ratio = (ratio * U128([0x2216e584f5fa, 0])) >> NUM_64
    }

    // Divide to obtain 1.0001^(2^(i - 1)) * 2^32 in numerator
    if tick > 0 {
        ratio = U128::MAX / ratio;
    }

    Ok(ratio.as_u128())
}

/// Calculates the greatest tick value such that get_sqrt_price_at_tick(tick) <= ratio
/// Throws if sqrt_price_x64 < MIN_SQRT_RATIO or sqrt_price_x64 > MAX_SQRT_RATIO
///
/// Formula: `i = log base(√1.0001) (√P)`
pub fn get_tick_at_sqrt_price(sqrt_price_x64: u128) -> Result<i32, anchor_lang::error::Error> {
    // Determine log_b(sqrt_ratio). First by calculating integer portion (msb)
    let msb: u32 = 128 - sqrt_price_x64.leading_zeros() - 1;
    let log2p_integer_x32 = (msb as i128 - 64) << 32;

    // get fractional value (r/2^msb), msb always > 128
    // We begin the iteration from bit 63 (0.5 in Q64.64)
    let mut bit: i128 = 0x8000_0000_0000_0000i128;
    let mut precision = 0;
    let mut log2p_fraction_x64 = 0;

    // Log2 iterative approximation for the fractional part
    // Go through each 2^(j) bit where j < 64 in a Q64.64 number
    // Append current bit value to fraction result if r^2 Q2.126 is more than 2
    let mut r = if msb >= 64 {
        sqrt_price_x64 >> (msb - 63)
    } else {
        sqrt_price_x64 << (63 - msb)
    };

    while bit > 0 && precision < BIT_PRECISION {
        r *= r;
        let is_r_more_than_two = r >> 127 as u32;
        r >>= 63 + is_r_more_than_two;
        log2p_fraction_x64 += bit * is_r_more_than_two as i128;
        bit >>= 1;
        precision += 1;
    }
    let log2p_fraction_x32 = log2p_fraction_x64 >> 32;
    let log2p_x32 = log2p_integer_x32 + log2p_fraction_x32;

    // 14 bit refinement gives an error margin of 2^-14 / log2 (√1.0001) = 0.8461 < 1
    // Since tick is a decimal, an error under 1 is acceptable

    // Change of base rule: multiply with 2^16 / log2 (√1.0001)
    let log_sqrt_10001_x64 = log2p_x32 * 59543866431248i128;

    // tick - 0.01
    let tick_low = ((log_sqrt_10001_x64 - 184467440737095516i128) >> 64) as i32;

    // tick + (2^-14 / log2(√1.001)) + 0.01
    let tick_high = ((log_sqrt_10001_x64 + 15793534762490258745i128) >> 64) as i32;

    Ok(if tick_low == tick_high {
        tick_low
    } else if get_sqrt_price_at_tick(tick_high).unwrap() <= sqrt_price_x64 {
        tick_high
    } else {
        tick_low
    })
}

#[derive(Debug, Clone, Default, Copy)]
pub struct TickState {
    pub tick: i32,
    pub liquidity_net: i128,
    pub liquidity_gross: u128,
    // pub fee_growth_outside_0_x64: u128,
    // pub fee_growth_outside_1_x64: u128,
    // pub reward_growths_outside_x64: [u128; 3],
    // pub padding: [u32; 13],
}

#[derive(Debug, Clone)]
pub struct TickArrayState {
    // pub pool_id: Pubkey,
    pub start_tick_index: i32,
    pub ticks: [TickState; 60],
    // pub initialized_tick_count: u8,
    // pub recent_epoch: u64,
    // pub padding: [u8; 107],
}

impl TickState {
    pub fn deserialize(reader: &mut ByteReader) -> Result<Self> {
        let tick = reader.read_u32()? as i32; // Read as u32 then cast to i32

        // Read liquidity_net as i128 (16 bytes)
        let liquidity_net_bytes = reader.read_bytes(16)?;
        let liquidity_net = i128::from_le_bytes(
            liquidity_net_bytes
                .try_into()
                .map_err(|_| anyhow!("Failed to convert liquidity_net bytes"))?,
        );

        // Read liquidity_gross as u128 (16 bytes)
        let liquidity_gross = reader.read_u128()?;

        // Read fee growth fields (16 bytes each)
        let fee_growth_outside_0_x64 = reader.read_u128()?;
        let fee_growth_outside_1_x64 = reader.read_u128()?;

        // Read reward_growths_outside_x64 array (3 * 16 = 48 bytes)
        let mut reward_growths_outside_x64 = [0u128; 3];
        for i in 0..3 {
            reward_growths_outside_x64[i] = reader.read_u128()?;
        }

        // Read padding array (13 * 4 = 52 bytes)
        let mut padding = [0u32; 13];
        for i in 0..13 {
            padding[i] = reader.read_u32()?;
        }

        Ok(TickState {
            tick,
            liquidity_net,
            liquidity_gross,
            // fee_growth_outside_0_x64,
            // fee_growth_outside_1_x64,
            // reward_growths_outside_x64,
            // padding,
        })
    }

    pub fn is_initialized(self) -> bool {
        self.liquidity_gross != 0
    }

    pub fn get_tick(&self) -> i32 {
        self.tick
    }

    pub fn get_liquidity_net(&self) -> i128 {
        self.liquidity_net
    }

    pub fn get_liquidity_gross(&self) -> u128 {
        self.liquidity_gross
    }

    /// Common checks for a valid tick input.
    /// A tick is valid if it lies within tick boundaries
    pub fn check_is_out_of_boundary(tick: i32) -> bool {
        tick < MIN_TICK || tick > MAX_TICK
    }
}

impl TickArrayState {
    pub fn deserialize(data: &[u8]) -> Result<Self> {
        let mut reader = ByteReader::new(data);

        // Skip the discriminator (first 8 bytes)
        reader.skip(8)?;

        // Read pool_id (32 bytes)
        let pool_id = reader.read_pubkey()?;

        // Read start_tick_index (4 bytes)
        let start_tick_index = reader.read_u32()? as i32;

        // Read ticks array (60 TickState structs)
        let mut ticks = Vec::new();
        for _ in 0..60 {
            ticks.push(TickState::deserialize(&mut reader)?);
        }
        let ticks: [TickState; 60] = ticks
            .try_into()
            .map_err(|_| anyhow!("Failed to convert ticks vector to array"))?;

        // Read initialized_tick_count (1 byte)
        let initialized_tick_count = reader.read_u8()?;

        // Read recent_epoch (8 bytes)
        let recent_epoch = reader.read_u64()?;

        // Read padding (107 bytes)
        let mut padding = [0u8; 107];
        for i in 0..107 {
            padding[i] = reader.read_u8()?;
        }

        Ok(TickArrayState {
            // pool_id,
            start_tick_index,
            ticks,
            // initialized_tick_count,
            // recent_epoch,
            // padding,
        })
    }

    pub fn get_tick_range(&self, tick_spacing: u16) -> (i32, i32) {
        let start = self.start_tick_index;
        let end = start + (60 * tick_spacing as i32) - tick_spacing as i32;
        (start, end)
    }

    pub fn find_tick(&self, tick_index: i32, tick_spacing: u16) -> Option<&TickState> {
        let array_index = ((tick_index - self.start_tick_index) / tick_spacing as i32) as usize;
        if array_index < 60 {
            Some(&self.ticks[array_index])
        } else {
            None
        }
    }

    pub fn get_initialized_ticks(&self) -> Vec<&TickState> {
        self.ticks
            .iter()
            .filter(|tick| tick.is_initialized())
            .collect()
    }

    pub fn contains_tick(&self, tick_index: i32, tick_spacing: u16) -> bool {
        let (start, end) = self.get_tick_range(tick_spacing);
        tick_index >= start && tick_index <= end
    }

    pub fn tick_count(tick_spacing: u16) -> i32 {
        TICK_ARRAY_SIZE * i32::from(tick_spacing)
    }

    /// Get next initialized tick in tick array, `current_tick_index` can be any tick index, in other words, `current_tick_index` not exactly a point in the tickarray,
    /// and current_tick_index % tick_spacing maybe not equal zero.
    /// If price move to left tick <= current_tick_index, or to right tick > current_tick_index
    pub fn next_initialized_tick(
        &mut self,
        current_tick_index: i32,
        tick_spacing: u16,
        zero_for_one: bool,
    ) -> Result<Option<&mut TickState>> {
        let current_tick_array_start_index =
            TickArrayState::get_array_start_index(current_tick_index, tick_spacing);
        if current_tick_array_start_index != self.start_tick_index {
            return Ok(None);
        }
        let mut offset_in_array =
            (current_tick_index - self.start_tick_index) / i32::from(tick_spacing);

        if zero_for_one {
            while offset_in_array >= 0 {
                if self.ticks[offset_in_array as usize].is_initialized() {
                    return Ok(self.ticks.get_mut(offset_in_array as usize));
                }
                offset_in_array = offset_in_array - 1;
            }
        } else {
            offset_in_array = offset_in_array + 1;
            while offset_in_array < TICK_ARRAY_SIZE {
                if self.ticks[offset_in_array as usize].is_initialized() {
                    return Ok(self.ticks.get_mut(offset_in_array as usize));
                }
                offset_in_array = offset_in_array + 1;
            }
        }
        Ok(None)
    }

    /// Base on swap directioin, return the first initialized tick in the tick array.
    pub fn first_initialized_tick(&mut self, zero_for_one: bool) -> Result<&mut TickState> {
        if zero_for_one {
            let mut i = TICK_ARRAY_SIZE - 1;
            while i >= 0 {
                if self.ticks[i as usize].is_initialized() {
                    return Ok(self.ticks.get_mut(i as usize).unwrap());
                }
                i = i - 1;
            }
        } else {
            let mut i = 0;
            while i < TICK_ARRAY_SIZE_USIZE {
                if self.ticks[i].is_initialized() {
                    return Ok(self.ticks.get_mut(i).unwrap());
                }
                i = i + 1;
            }
        }

        Err(anyhow!("InvalidTickArray"))
    }

    /// Base on swap directioin, return the next tick array start index.
    pub fn next_tick_arrary_start_index(&self, tick_spacing: u16, zero_for_one: bool) -> i32 {
        let ticks_in_array = TICK_ARRAY_SIZE * i32::from(tick_spacing);
        if zero_for_one {
            self.start_tick_index - ticks_in_array
        } else {
            self.start_tick_index + ticks_in_array
        }
    }

    /// Input an arbitrary tick_index, output the start_index of the tick_array it sits on
    pub fn get_array_start_index(tick_index: i32, tick_spacing: u16) -> i32 {
        let ticks_in_array = TickArrayState::tick_count(tick_spacing);
        let mut start = tick_index / ticks_in_array;
        if tick_index < 0 && tick_index % ticks_in_array != 0 {
            start = start - 1
        }
        start * ticks_in_array
    }

    pub fn check_is_valid_start_index(tick_index: i32, tick_spacing: u16) -> bool {
        if TickState::check_is_out_of_boundary(tick_index) {
            if tick_index > MAX_TICK {
                return false;
            }
            let min_start_index = TickArrayState::get_array_start_index(MIN_TICK, tick_spacing);
            return tick_index == min_start_index;
        }
        tick_index % TickArrayState::tick_count(tick_spacing) == 0
    }
}

impl Default for TickArrayState {
    #[inline]
    fn default() -> TickArrayState {
        TickArrayState {
            // pool_id: Pubkey::default(),
            ticks: [TickState::default(); TICK_ARRAY_SIZE_USIZE],
            start_tick_index: 0,
            // initialized_tick_count: 0,
            // recent_epoch: 0,
            // padding: [0; 107],
        }
    }
}
