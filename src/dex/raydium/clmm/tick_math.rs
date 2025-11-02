use super::q_math::*;

pub struct PriceConstants;

impl PriceConstants {
    // sqrt(1.0001) = 1.00004999875...
    pub const SQRT_1_0001: f64 = 1.00004999875062;

    // 1.0001^(tick_spacing) for common spacings
    pub const PRICE_TICK_SPACING_1: f64 = 1.0001;
    pub const PRICE_TICK_SPACING_10: f64 = 1.001000450120027;
    pub const PRICE_TICK_SPACING_60: f64 = 1.006018054162;
    pub const PRICE_TICK_SPACING_200: f64 = 1.020201340026755;

    // Maximum and minimum tick values
    pub const MIN_TICK: i32 = -443636;
    pub const MAX_TICK: i32 = 443636;

    // Corresponding sqrt prices (using safe constants)
    pub const MIN_SQRT_PRICE_X64: u64 = 4295048016;
    pub const MAX_SQRT_PRICE_X64: u128 = 79226673515401279992447579055;
}

pub fn is_valid_tick(tick: i32) -> bool {
    tick >= PriceConstants::MIN_TICK && tick <= PriceConstants::MAX_TICK
}

/// Get next initialized tick given tick spacing
pub fn get_next_tick(tick: i32, tick_spacing: u16, up: bool) -> i32 {
    let spacing = tick_spacing as i32;
    if up {
        ((tick / spacing) + 1) * spacing
    } else {
        ((tick / spacing) - 1) * spacing
    }
}

/// Calculate liquidity from amounts and price range
pub fn liquidity_from_amounts(
    sqrt_price_x128: u128,
    sqrt_price_lower_x128: u128,
    sqrt_price_upper_x128: u128,
    amount_0: u64,
    amount_1: u64,
) -> u128 {
    // Convert Q64.64 (stored as u128) to f64 safely
    let sqrt_price = sqrt_price_x128 as f64 / Q64_64_SCALE;
    let sqrt_price_lower = sqrt_price_lower_x128 as f64 / Q64_64_SCALE;
    let sqrt_price_upper = sqrt_price_upper_x128 as f64 / Q64_64_SCALE;

    let liquidity_0 =
        amount_0 as f64 * sqrt_price * sqrt_price_upper / (sqrt_price_upper - sqrt_price);
    let liquidity_1 = amount_1 as f64 / (sqrt_price - sqrt_price_lower);

    liquidity_0.min(liquidity_1) as u128
}

pub fn tick_to_price(tick: i32) -> f64 {
    // Base case: 1.0001^tick
    // For performance, we can use precomputed values for common tick ranges

    if tick == 0 {
        return 1.0;
    }

    // Use bit manipulation for powers of 2
    if tick > 0 {
        1.0001_f64.powi(tick)
    } else {
        1.0 / 1.0001_f64.powi(-tick)
    }
}

/// Convert sqrt_price_x128 to tick (inverse operation for u128)
pub fn sqrt_price_x128_to_tick(sqrt_price_x128: u128) -> i32 {
    // Convert Q64.64 (stored as u128) to actual sqrt price
    let sqrt_price = sqrt_price_x128 as f64 / Q64_64_SCALE;
    let price = sqrt_price * sqrt_price;

    // Calculate tick = log_1.0001(price)
    // Using change of base: log_1.0001(price) = ln(price) / ln(1.0001)
    let tick = price.ln() / 1.0001_f64.ln();
    tick.round() as i32
}

/// Convert sqrt_price_x64 to tick (legacy function for backward compatibility)
pub fn sqrt_price_x64_to_tick(sqrt_price_x64: u64) -> i32 {
    sqrt_price_x128_to_tick(sqrt_price_x64 as u128)
}
