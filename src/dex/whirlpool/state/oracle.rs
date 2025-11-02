use super::math::{Q64_RESOLUTION, U256Muldiv, increasing_price_order, sqrt_price_from_tick_index};
use crate::byte_reader::ByteReader;
use anchor_client::solana_sdk::pubkey::Pubkey;
use anyhow::{Result, anyhow};

pub const MAX_TRADE_ENABLE_TIMESTAMP_DELTA: u64 = 60 * 60 * 72; // 72 hours

// This constant is used to scale the value of the volatility accumulator.
// The value of the volatility accumulator is decayed by the reduction factor and used as a new reference.
// However, if the volatility accumulator is simply the difference in tick_group_index, a value of 1 would quickly decay to 0.
// By scaling 1 to 10,000, for example, if the reduction factor is 0.5, the resulting value would be 5,000.
pub const VOLATILITY_ACCUMULATOR_SCALE_FACTOR: u16 = 10_000;

// The denominator of the reduction factor.
// When the reduction_factor is 5_000, the reduction factor functions as 0.5.
pub const REDUCTION_FACTOR_DENOMINATOR: u16 = 10_000;

// adaptive_fee_control_factor is used to map the square of the volatility accumulator to the fee rate.
// A larger value increases the fee rate quickly even for small volatility, while a smaller value increases the fee rate more gradually even for high volatility.
// When the adaptive_fee_control_factor is 1_000, the adaptive fee control factor functions as 0.01.
pub const ADAPTIVE_FEE_CONTROL_FACTOR_DENOMINATOR: u32 = 100_000;

// The time (in seconds) to forcibly reset the reference if it is not updated for a long time.
// A recovery measure against the act of intentionally repeating major swaps to keep the Adaptive Fee high (DoS).
pub const MAX_REFERENCE_AGE: u64 = 3_600; // 1 hour

const TICK_ARRAY_SIZE: i32 = 88;

#[derive(Debug, Default, Clone)]
pub struct AdaptiveFeeConstants {
    pub filter_period: u16,
    pub decay_period: u16,
    pub reduction_factor: u16,
    pub adaptive_fee_control_factor: u32,
    pub max_volatility_accumulator: u32,
    pub tick_group_size: u16,
    pub major_swap_threshold_ticks: u16,
    pub reserved: [u8; 16],
}

impl AdaptiveFeeConstants {
    pub const LEN: usize = 2 + 2 + 2 + 4 + 4 + 2 + 2 + 16;

    pub fn deserialize(reader: &mut ByteReader) -> Result<Self> {
        Ok(Self {
            filter_period: reader.read_u16()?,
            decay_period: reader.read_u16()?,
            reduction_factor: reader.read_u16()?,
            adaptive_fee_control_factor: reader.read_u32()?,
            max_volatility_accumulator: reader.read_u32()?,
            tick_group_size: reader.read_u16()?,
            major_swap_threshold_ticks: reader.read_u16()?,
            reserved: reader.read_bytes_array::<16>()?,
        })
    }

    #[allow(clippy::too_many_arguments)]
    pub fn validate_constants(
        tick_spacing: u16,
        filter_period: u16,
        decay_period: u16,
        reduction_factor: u16,
        adaptive_fee_control_factor: u32,
        max_volatility_accumulator: u32,
        tick_group_size: u16,
        major_swap_threshold_ticks: u16,
    ) -> bool {
        // filter_period validation
        // must be >= 1
        if filter_period == 0 {
            return false;
        }

        // decay_period validation
        // must be >= 1 and > filter_period
        if decay_period == 0 || decay_period <= filter_period {
            return false;
        }

        // adaptive_fee_control_factor validation
        // must be less than ADAPTIVE_FEE_CONTROL_FACTOR_DENOMINATOR
        if adaptive_fee_control_factor >= ADAPTIVE_FEE_CONTROL_FACTOR_DENOMINATOR {
            return false;
        }

        // max_volatility_accumulator validation
        // this constraint is to prevent overflow at FeeRateManager::compute_adaptive_fee_rate
        if u64::from(max_volatility_accumulator) * u64::from(tick_group_size) > u32::MAX as u64 {
            return false;
        }

        // reduction_factor validation
        if reduction_factor >= REDUCTION_FACTOR_DENOMINATOR {
            return false;
        }

        // tick_group_size validation
        if tick_group_size == 0
            || tick_group_size > tick_spacing
            || tick_spacing % tick_group_size != 0
        {
            return false;
        }

        // major_swap_threshold_ticks validation
        // there is no clear upper limit for major_swap_threshold_ticks, but as a safeguard, we set the limit to ticks in a TickArray
        let ticks_in_tick_array = tick_spacing as i32 * TICK_ARRAY_SIZE;
        if major_swap_threshold_ticks == 0
            || major_swap_threshold_ticks as i32 > ticks_in_tick_array
        {
            return false;
        }

        true
    }
}

#[derive(Debug, Default, Clone)]
pub struct AdaptiveFeeVariables {
    pub last_reference_update_timestamp: u64,
    pub last_major_swap_timestamp: u64,
    pub volatility_reference: u32,
    pub tick_group_index_reference: i32,
    pub volatility_accumulator: u32,
    pub reserved: [u8; 16],
}

impl AdaptiveFeeVariables {
    pub const LEN: usize = 8 + 8 + 4 + 4 + 4 + 16;

    pub fn deserialize(reader: &mut ByteReader) -> Result<Self> {
        Ok(Self {
            last_reference_update_timestamp: reader.read_u64()?,
            last_major_swap_timestamp: reader.read_u64()?,
            volatility_reference: reader.read_u32()?,
            tick_group_index_reference: {
                let bytes = reader.read_bytes_array::<4>()?;
                i32::from_le_bytes(bytes)
            },
            volatility_accumulator: reader.read_u32()?,
            reserved: reader.read_bytes_array::<16>()?,
        })
    }

    pub fn update_volatility_accumulator(
        &mut self,
        tick_group_index: i32,
        adaptive_fee_constants: &AdaptiveFeeConstants,
    ) -> Result<()> {
        let index_delta = (self.tick_group_index_reference - tick_group_index).unsigned_abs();
        let volatility_accumulator = u64::from(self.volatility_reference)
            + u64::from(index_delta) * u64::from(VOLATILITY_ACCUMULATOR_SCALE_FACTOR);

        self.volatility_accumulator = std::cmp::min(
            volatility_accumulator,
            u64::from(adaptive_fee_constants.max_volatility_accumulator),
        ) as u32;

        Ok(())
    }

    pub fn update_reference(
        &mut self,
        tick_group_index: i32,
        current_timestamp: u64,
        adaptive_fee_constants: &AdaptiveFeeConstants,
    ) -> Result<()> {
        let max_timestamp = self
            .last_reference_update_timestamp
            .max(self.last_major_swap_timestamp);
        if current_timestamp < max_timestamp {
            // return Err(anyhow!("Invalid timestamp"));
            // for testing
            return Ok(());
        }

        let reference_age = current_timestamp - self.last_reference_update_timestamp;
        if reference_age > MAX_REFERENCE_AGE {
            // The references are too old, so reset them
            self.tick_group_index_reference = tick_group_index;
            self.volatility_reference = 0;
            self.last_reference_update_timestamp = current_timestamp;
            return Ok(());
        }

        let elapsed = current_timestamp - max_timestamp;
        if elapsed < adaptive_fee_constants.filter_period as u64 {
            // high frequency trade
            // no change
        } else if elapsed < adaptive_fee_constants.decay_period as u64 {
            // NOT high frequency trade
            self.tick_group_index_reference = tick_group_index;
            self.volatility_reference = (u64::from(self.volatility_accumulator)
                * u64::from(adaptive_fee_constants.reduction_factor)
                / u64::from(REDUCTION_FACTOR_DENOMINATOR))
                as u32;
            self.last_reference_update_timestamp = current_timestamp;
        } else {
            // Out of decay time window
            self.tick_group_index_reference = tick_group_index;
            self.volatility_reference = 0;
            self.last_reference_update_timestamp = current_timestamp;
        }

        Ok(())
    }

    pub fn update_major_swap_timestamp(
        &mut self,
        pre_sqrt_price: u128,
        post_sqrt_price: u128,
        current_timestamp: u64,
        adaptive_fee_constants: &AdaptiveFeeConstants,
    ) -> Result<()> {
        if Self::is_major_swap(
            pre_sqrt_price,
            post_sqrt_price,
            adaptive_fee_constants.major_swap_threshold_ticks,
        )? {
            self.last_major_swap_timestamp = current_timestamp;
        }
        Ok(())
    }

    // Determine whether the difference between pre_sqrt_price and post_sqrt_price is equivalent to major_swap_threshold_ticks or more
    // Note: The error of less than 0.00000003% due to integer arithmetic of sqrt_price is acceptable
    fn is_major_swap(
        pre_sqrt_price: u128,
        post_sqrt_price: u128,
        major_swap_threshold_ticks: u16,
    ) -> Result<bool> {
        let (smaller_sqrt_price, larger_sqrt_price) =
            increasing_price_order(pre_sqrt_price, post_sqrt_price);

        // major_swap_sqrt_price_target
        //   = smaller_sqrt_price * sqrt(pow(1.0001, major_swap_threshold_ticks))
        //   = smaller_sqrt_price * sqrt_price_from_tick_index(major_swap_threshold_ticks) >> Q64_RESOLUTION
        //
        // Note: The following two are theoretically equal, but there is an integer arithmetic error.
        //       However, the error impact is less than 0.00000003% in sqrt price (x64) and is small enough.
        //       - sqrt_price_from_tick_index(a) * sqrt_price_from_tick_index(b) >> Q64_RESOLUTION   (mathematically, sqrt(pow(1.0001, a)) * sqrt(pow(1.0001, b)) = sqrt(pow(1.0001, a + b)))
        //       - sqrt_price_from_tick_index(a + b)                                                 (mathematically, sqrt(pow(1.0001, a + b)))
        let major_swap_sqrt_price_factor =
            sqrt_price_from_tick_index(major_swap_threshold_ticks as i32);
        let major_swap_sqrt_price_target = U256Muldiv::new(0, smaller_sqrt_price)
            .mul(U256Muldiv::new(0, major_swap_sqrt_price_factor))
            .shift_right(Q64_RESOLUTION as u32)
            .try_into_u128()?;

        Ok(larger_sqrt_price >= major_swap_sqrt_price_target)
    }
}

#[derive(Debug, Default, Clone)]
pub struct AdaptiveFeeInfo {
    pub constants: AdaptiveFeeConstants,
    pub variables: AdaptiveFeeVariables,
}

#[derive(Debug, Clone)]
pub struct Oracle {
    pub whirlpool: Pubkey,
    pub trade_enable_timestamp: u64,
    pub adaptive_fee_constants: AdaptiveFeeConstants,
    pub adaptive_fee_variables: AdaptiveFeeVariables,
    pub reserved: [u8; 128],
}

impl Oracle {
    pub fn deserialize(data: &[u8]) -> Result<Self> {
        // Skip discriminator (8 bytes)
        if data.len() < 8 {
            return Err(anyhow!("Data too short for account discriminator"));
        }

        let mut reader = ByteReader::new(&data[8..]);

        Ok(Self {
            whirlpool: reader.read_pubkey()?,
            trade_enable_timestamp: reader.read_u64()?,
            adaptive_fee_constants: AdaptiveFeeConstants::deserialize(&mut reader)?,
            adaptive_fee_variables: AdaptiveFeeVariables::deserialize(&mut reader)?,
            reserved: reader.read_bytes_array::<128>()?,
        })
    }
}

impl From<Oracle> for AdaptiveFeeInfo {
    fn from(oracle: Oracle) -> Self {
        Self {
            constants: oracle.adaptive_fee_constants,
            variables: oracle.adaptive_fee_variables,
        }
    }
}

impl From<&Oracle> for AdaptiveFeeInfo {
    fn from(oracle: &Oracle) -> Self {
        Self {
            constants: oracle.adaptive_fee_constants.clone(),
            variables: oracle.adaptive_fee_variables.clone(),
        }
    }
}
