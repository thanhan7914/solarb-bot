#![allow(non_snake_case)]

/// This constant is used to scale the value of the volatility accumulator.
pub const VOLATILITY_ACCUMULATOR_SCALE_FACTOR: u16 = 10_000;

/// The denominator of the reduction factor.
pub const REDUCTION_FACTOR_DENOMINATOR: u16 = 10_000;

/// adaptive_fee_control_factor is used to map the square of the volatility accumulator to the fee rate.
pub const ADAPTIVE_FEE_CONTROL_FACTOR_DENOMINATOR: u32 = 100_000;

/// The time (in seconds) to forcibly reset the reference if it is not updated for a long time.
pub const MAX_REFERENCE_AGE: u64 = 3_600;

/// max fee rate should be controlled by max_volatility_accumulator, so this is a hard limit for safety.
/// Fee rate is represented as hundredths of a basis point.
pub const FEE_RATE_HARD_LIMIT: u32 = 100_000; // 10%
