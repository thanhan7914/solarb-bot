use super::u128x128_math::{Rounding, mul_div_u256};
use crate::safe_math::SafeMath;
use anyhow::{Result, anyhow};
use ruint::aliases::U256;

pub const RESOLUTION: u8 = 64;

pub fn get_initialize_amounts(
    sqrt_min_price: u128,
    sqrt_max_price: u128,
    sqrt_price: u128,
    liquidity: u128,
) -> Result<(u64, u64)> {
    // BASE TOKEN
    let amount_a =
        get_delta_amount_a_unsigned(sqrt_price, sqrt_max_price, liquidity, Rounding::Up)?;
    // QUOTE TOKEN
    let amount_b =
        get_delta_amount_b_unsigned(sqrt_min_price, sqrt_price, liquidity, Rounding::Up)?;
    Ok((amount_a, amount_b))
}

/// Gets the delta amount_a for given liquidity and price range
///
/// # Formula
///
/// * `Δa = L * (1 / √P_lower - 1 / √P_upper)`
/// * i.e. `L * (√P_upper - √P_lower) / (√P_upper * √P_lower)`
pub fn get_delta_amount_a_unsigned(
    lower_sqrt_price: u128,
    upper_sqrt_price: u128,
    liquidity: u128,
    round: Rounding,
) -> Result<u64> {
    let result = get_delta_amount_a_unsigned_unchecked(
        lower_sqrt_price,
        upper_sqrt_price,
        liquidity,
        round,
    )?;
    return Ok(result.try_into().map_err(|_| anyhow!("TypeCast Failed"))?);
}

/// * i.e. `L * (√P_upper - √P_lower) / (√P_upper * √P_lower)`
pub fn get_delta_amount_a_unsigned_unchecked(
    lower_sqrt_price: u128,
    upper_sqrt_price: u128,
    liquidity: u128,
    round: Rounding,
) -> Result<U256> {
    if upper_sqrt_price < lower_sqrt_price {
        return Err(anyhow!("Upper sqrt price less than lower sqrt price"));
    }

    let numerator_1 = U256::from(liquidity);
    let numerator_2 = U256::from(upper_sqrt_price - lower_sqrt_price);

    let denominator = U256::from(lower_sqrt_price).safe_mul(U256::from(upper_sqrt_price))?;

    assert!(denominator > U256::ZERO);
    let result = mul_div_u256(numerator_1, numerator_2, denominator, round)
        .ok_or_else(|| anyhow!("Math overflow"))?;
    return Ok(result);
}

/// Gets the delta amount_b for given liquidity and price range
/// * `Δb = L (√P_upper - √P_lower)`
pub fn get_delta_amount_b_unsigned(
    lower_sqrt_price: u128,
    upper_sqrt_price: u128,
    liquidity: u128,
    round: Rounding,
) -> Result<u64> {
    let result = get_delta_amount_b_unsigned_unchecked(
        lower_sqrt_price,
        upper_sqrt_price,
        liquidity,
        round,
    )?;
    return Ok(result.try_into().map_err(|_| anyhow!("TypeCast Failed"))?);
}

//Δb = L (√P_upper - √P_lower)
pub fn get_delta_amount_b_unsigned_unchecked(
    lower_sqrt_price: u128,
    upper_sqrt_price: u128,
    liquidity: u128,
    round: Rounding,
) -> Result<U256> {
    let liquidity = U256::from(liquidity);
    let delta_sqrt_price = U256::from(upper_sqrt_price - lower_sqrt_price);
    let prod = liquidity.safe_mul(delta_sqrt_price)?;

    match round {
        Rounding::Up => {
            let denominator = U256::from(1).safe_shl((RESOLUTION as usize) * 2)?;
            let result = prod.div_ceil(denominator);
            Ok(result)
        }
        Rounding::Down => {
            let (result, _) = prod.overflowing_shr((RESOLUTION as usize) * 2);
            Ok(result)
        }
    }
}

/// Gets the next sqrt price given an input amount of token_a or token_b
/// Throws if price or liquidity are 0, or if the next price is out of bounds
pub fn get_next_sqrt_price_from_input(
    sqrt_price: u128,
    liquidity: u128,
    amount_in: u64,
    a_for_b: bool,
) -> Result<u128> {
    if liquidity <= 0 || sqrt_price <= 0 {
        return Ok(0);
    }

    // round to make sure that we don't pass the target price
    if a_for_b {
        get_next_sqrt_price_from_amount_a_rounding_up(sqrt_price, liquidity, amount_in)
    } else {
        get_next_sqrt_price_from_amount_b_rounding_down(sqrt_price, liquidity, amount_in)
    }
}

/// Gets the next sqrt price √P' given a delta of token_a
///
/// Always round up because
/// 1. In the exact output case, token 0 supply decreases leading to price increase.
/// Move price up so that exact output is met.
/// 2. In the exact input case, token 0 supply increases leading to price decrease.
/// Do not round down to minimize price impact. We only need to meet input
/// change and not guarantee exact output.
///
/// Use function for exact input or exact output swaps for token 0
///
/// # Formula
///
/// * `√P' = √P * L / (L + Δx * √P)`
/// * If Δx * √P overflows, use alternate form `√P' = L / (L/√P + Δx)`
///
/// # Proof
///
/// For constant L,
///
///  L = x * √P
///  x' = x + Δx
///  x' * √P' = x * √P
///  (x + Δx) * √P' = x * √P
///  √P' = (x * √P) / (x + Δx)
///  x = L/√P
///  √P' = √P * L / (L + Δx * √P)
///
pub fn get_next_sqrt_price_from_amount_a_rounding_up(
    sqrt_price: u128,
    liquidity: u128,
    amount: u64,
) -> Result<u128> {
    if amount == 0 {
        return Ok(sqrt_price);
    }
    let sqrt_price = U256::from(sqrt_price);
    let liquidity = U256::from(liquidity);

    let product = U256::from(amount).safe_mul(sqrt_price)?;
    let denominator = liquidity.safe_add(U256::from(product))?;
    let result = mul_div_u256(liquidity, sqrt_price, denominator, Rounding::Up)
        .ok_or_else(|| anyhow!("Math overflow"))?;
    return Ok(result.try_into().map_err(|_| anyhow!("TypeCast Failed"))?);
}

/// Gets the next sqrt price given a delta of token_b
///
/// Always round down because
/// 1. In the exact output case, token 1 supply decreases leading to price decrease.
/// Move price down by rounding down so that exact output of token 0 is met.
/// 2. In the exact input case, token 1 supply increases leading to price increase.
/// Do not round down to minimize price impact. We only need to meet input
/// change and not gurantee exact output for token 0.
///
///
/// # Formula
///
/// * `√P' = √P + Δy / L`
///
pub fn get_next_sqrt_price_from_amount_b_rounding_down(
    sqrt_price: u128,
    liquidity: u128,
    amount: u64,
) -> Result<u128> {
    let quotient = U256::from(amount)
        .safe_shl((RESOLUTION * 2) as usize)?
        .safe_div(U256::from(liquidity))?;

    let result = U256::from(sqrt_price).safe_add(quotient)?;
    Ok(result.try_into().map_err(|_| anyhow!("TypeCast Failed"))?)
}
