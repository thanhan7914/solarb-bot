/// Mathematical constants for Q-format numbers
pub const Q64_64_SCALE: f64 = 18446744073709551616.0; // 2^64 as f64
// 2^128 is too large for u128 literal, so use smaller representation
pub const Q128_128_SCALE: f64 = 340282366920938463463374607431768211456.0; // 2^128 as f64

/// Convert Q64.64 (stored as u64) to f64
pub fn q64_64_to_f64(value: u64) -> f64 {
    value as f64 / Q64_64_SCALE
}

/// Convert Q64.64 (stored as u128) to f64
pub fn q64_64_u128_to_f64(value: u128) -> f64 {
    value as f64 / Q64_64_SCALE
}

/// Convert f64 to Q64.64 (with bounds checking)
pub fn f64_to_q64_64(value: f64) -> Option<u64> {
    if value < 0.0 || value >= 1.0 {
        return None;
    }
    Some((value * Q64_64_SCALE) as u64)
}

/// Multiply two Q64.64 numbers (result is Q128.128, then scaled back)
pub fn q64_64_mul(a: u64, b: u64) -> u64 {
    let result = (a as u128) * (b as u128);
    // Divide by 2^64 to get back to Q64.64
    (result >> 64) as u64
}

/// Square a Q64.64 number (u64 input)
pub fn q64_64_square(value: u64) -> u128 {
    let val_u128 = value as u128;
    val_u128 * val_u128
}

/// Square a Q64.64 number (u128 input)
pub fn q64_64_u128_square(value: u128) -> u128 {
    // For u128 input representing Q64.64, we can square directly
    // but need to be careful about overflow
    value * value
}

/// Convert Q128.128 to f64 (for price calculations)
pub fn q128_128_to_f64(value: u128) -> f64 {
    value as f64 / Q128_128_SCALE
}

/// Safely get high and low parts of u128 for division
pub fn u128_to_f64_precise(value: u128) -> f64 {
    // Split u128 into high and low u64 parts for better precision
    let high = (value >> 64) as u64;
    let low = (value & 0xFFFFFFFFFFFFFFFF) as u64;
    (high as f64) * Q64_64_SCALE + (low as f64)
}
