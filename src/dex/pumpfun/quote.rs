use anyhow::{anyhow, Result};
use anchor_client::solana_sdk::pubkey::Pubkey;
use super::util::{ceil_div, fee};

#[derive(Debug, Clone)]
pub struct BuyBaseInputResult {
    pub internal_quote_amount: u128,
    pub ui_quote: u128,
    pub max_quote: u128,
}

#[derive(Debug, Clone)]
pub struct BuyQuoteInputResult {
    pub base: u128,
    pub internal_quote_without_fees: u128,
    pub max_quote: u128,
}

#[derive(Debug, Clone)]
pub struct SellBaseInputResult {
    pub ui_quote: u128,
    pub min_quote: u128,
    pub internal_quote_amount_out: u128,
}

#[derive(Debug, Clone)]
pub struct SellQuoteInputResult {
    pub internal_raw_quote: u128,
    pub base: u128,
    pub min_quote: u128,
}

pub fn buy_base_input_internal(
    base: u128,
    slippage: f64, // 1.0 => 1%
    base_reserve: u128,
    quote_reserve: u128,
    lp_fee_bps: u128,
    protocol_fee_bps: u128,
    coin_creator_fee_bps: u128,
    coin_creator: Pubkey,
) -> Result<BuyBaseInputResult> {
    // -----------------------------------------------------
    // 1) Basic validations
    // -----------------------------------------------------
    if base_reserve == 0 || quote_reserve == 0 {
        return Err(anyhow!("Invalid input: 'base_reserve' or 'quote_reserve' cannot be zero."));
    }
    
    if base > base_reserve {
        return Err(anyhow!("Cannot buy more base tokens than the pool reserves."));
    }

    // -----------------------------------------------------
    // 2) Calculate the raw quote needed (Raydium-like formula)
    //    quote_amount_in = ceil_div(quote_reserve * base, base_reserve - base)
    // -----------------------------------------------------
    let numerator = quote_reserve
        .checked_mul(base)
        .ok_or_else(|| anyhow!("Math overflow in numerator calculation"))?;
    
    let denominator = base_reserve
        .checked_sub(base)
        .ok_or_else(|| anyhow!("Math underflow in denominator calculation"))?;

    if denominator == 0 {
        return Err(anyhow!("Pool would be depleted; denominator is zero."));
    }

    let quote_amount_in = ceil_div(numerator, denominator)?;

    // -----------------------------------------------------
    // 3) Calculate fees
    //    - LP Fee = floor((quote_amount_in * lp_fee_bps) / 10000)
    //    - Protocol Fee = floor((quote_amount_in * protocol_fee_bps) / 10000)
    // -----------------------------------------------------
    let lp_fee = fee(quote_amount_in, lp_fee_bps)?;
    let protocol_fee = fee(quote_amount_in, protocol_fee_bps)?;
    
    let coin_creator_fee = if coin_creator == Pubkey::default() {
        0
    } else {
        fee(quote_amount_in, coin_creator_fee_bps)?
    };

    let total_quote = quote_amount_in
        .checked_add(lp_fee)
        .and_then(|x| x.checked_add(protocol_fee))
        .and_then(|x| x.checked_add(coin_creator_fee))
        .ok_or_else(|| anyhow!("Math overflow in total_quote calculation"))?;

    // -----------------------------------------------------
    // 4) Calculate maxQuote with slippage
    //    If slippage=1.0 => factor = (1 + 1/100) = 1.01
    // -----------------------------------------------------
    let precision = 1_000_000_000u128; // For slippage calculations
    let slippage_factor = ((1.0 + slippage / 100.0) * precision as f64) as u128;

    // max_quote = total_quote * slippage_factor / precision
    let max_quote = total_quote
        .checked_mul(slippage_factor)
        .and_then(|x| x.checked_div(precision))
        .ok_or_else(|| anyhow!("Math overflow in max_quote calculation"))?;

    Ok(BuyBaseInputResult {
        internal_quote_amount: quote_amount_in,
        ui_quote: total_quote,
        max_quote,
    })
}

const MAX_FEE_BASIS_POINTS: u128 = 10_000; // Assuming MAX_FEE_BASIS_POINTS is 10,000 (100%)

fn calculate_quote_amount_out(
    user_quote_amount_out: u128,
    lp_fee_basis_points: u128,
    protocol_fee_basis_points: u128,
    coin_creator_fee_basis_points: u128,
) -> Result<u128> {
    // Calculate the total fee basis points
    let total_fee_basis_points = lp_fee_basis_points
        .checked_add(protocol_fee_basis_points)
        .and_then(|x| x.checked_add(coin_creator_fee_basis_points))
        .ok_or_else(|| anyhow!("Math overflow in total_fee_basis_points calculation"))?;
    
    // Calculate the denominator
    let denominator = MAX_FEE_BASIS_POINTS
        .checked_sub(total_fee_basis_points)
        .ok_or_else(|| anyhow!("Math underflow in denominator calculation"))?;
    
    // Calculate the quote_amount_out
    let numerator = user_quote_amount_out
        .checked_mul(MAX_FEE_BASIS_POINTS)
        .ok_or_else(|| anyhow!("Math overflow in numerator calculation"))?;
    
    ceil_div(numerator, denominator)
}

pub fn sell_base_input_internal(
    base: u128,
    slippage: f64, // e.g. 1.0 => 1% slippage tolerance
    base_reserve: u128,
    quote_reserve: u128,
    lp_fee_bps: u128,
    protocol_fee_bps: u128,
    coin_creator_fee_bps: u128,
    coin_creator: Pubkey,
) -> Result<SellBaseInputResult> {
    // -----------------------------------------
    // 1) Basic validations
    // -----------------------------------------
    if base_reserve == 0 || quote_reserve == 0 {
        return Err(anyhow!("Invalid input: 'base_reserve' or 'quote_reserve' cannot be zero."));
    }

    // -----------------------------------------
    // 2) Calculate the raw quote output (no fees)
    //    This matches a typical constant-product formula for selling base to get quote:
    //      quote_amount_out = floor( (quote_reserve * base) / (base_reserve + base) )
    // -----------------------------------------
    let numerator = quote_reserve
        .checked_mul(base)
        .ok_or_else(|| anyhow!("Math overflow in quote_amount_out numerator"))?;
    
    let denominator = base_reserve
        .checked_add(base)
        .ok_or_else(|| anyhow!("Math overflow in quote_amount_out denominator"))?;

    let quote_amount_out = numerator / denominator; // floor division

    // -----------------------------------------
    // 3) Calculate fees
    //    LP fee and protocol fee are both taken from 'quote_amount_out'
    // -----------------------------------------
    let lp_fee = fee(quote_amount_out, lp_fee_bps)?;
    let protocol_fee = fee(quote_amount_out, protocol_fee_bps)?;
    let coin_creator_fee = if coin_creator == Pubkey::default() {
        0
    } else {
        fee(quote_amount_out, coin_creator_fee_bps)?
    };

    // Subtract fees to get the actual user receive
    let final_quote = quote_amount_out
        .checked_sub(lp_fee)
        .and_then(|x| x.checked_sub(protocol_fee))
        .and_then(|x| x.checked_sub(coin_creator_fee))
        .ok_or_else(|| anyhow!("Fees exceed total output; final quote is negative."))?;

    // -----------------------------------------
    // 4) Calculate minQuote with slippage
    //    - If slippage=1 => 1%, we allow receiving as low as 99% of final_quote
    // -----------------------------------------
    let precision = 1_000_000_000u128; // For safe integer math
    // (1 - slippage/100) => e.g. slippage=1 => factor= 0.99
    let slippage_factor = ((1.0 - slippage / 100.0) * precision as f64) as u128;

    // min_quote = final_quote * (1 - slippage/100)
    let min_quote = final_quote
        .checked_mul(slippage_factor)
        .and_then(|x| x.checked_div(precision))
        .ok_or_else(|| anyhow!("Math overflow in min_quote calculation"))?;

    Ok(SellBaseInputResult {
        ui_quote: final_quote, // actual tokens user receives after fees
        min_quote, // minimum acceptable tokens after applying slippage
        internal_quote_amount_out: quote_amount_out,
    })
}

pub fn sell_quote_input_internal(
    quote: u128,
    slippage: f64, // e.g. 1.0 => 1% slippage tolerance
    base_reserve: u128,
    quote_reserve: u128,
    lp_fee_bps: u128,
    protocol_fee_bps: u128,
    coin_creator_fee_bps: u128,
    coin_creator: Pubkey,
) -> Result<SellQuoteInputResult> {
    // -----------------------------------------
    // 1) Basic validations
    // -----------------------------------------
    if base_reserve == 0 || quote_reserve == 0 {
        return Err(anyhow!("Invalid input: 'base_reserve' or 'quote_reserve' cannot be zero."));
    }
    if quote > quote_reserve {
        return Err(anyhow!("Cannot receive more quote tokens than the pool quote reserves."));
    }

    // -----------------------------------------
    // 2) Calculate the fees included in the quote
    // -----------------------------------------
    let coin_creator_fee_bps_actual = if coin_creator == Pubkey::default() {
        0
    } else {
        coin_creator_fee_bps
    };

    let raw_quote = calculate_quote_amount_out(
        quote,
        lp_fee_bps,
        protocol_fee_bps,
        coin_creator_fee_bps_actual,
    )?;

    // -----------------------------------------
    // 3) Calculate the base amount needed for the raw quote output
    //    Invert the constant product formula:
    //    base_amount_in = ceil((base_reserve * raw_quote) / (quote_reserve - raw_quote))
    // -----------------------------------------
    if raw_quote >= quote_reserve {
        return Err(anyhow!("Invalid input: Desired quote amount exceeds available reserve."));
    }

    let numerator = base_reserve
        .checked_mul(raw_quote)
        .ok_or_else(|| anyhow!("Math overflow in base_amount_in numerator"))?;
    
    let denominator = quote_reserve
        .checked_sub(raw_quote)
        .ok_or_else(|| anyhow!("Math underflow in base_amount_in denominator"))?;

    let base_amount_in = ceil_div(numerator, denominator)?;

    // -----------------------------------------
    // 4) Calculate minQuote with slippage
    //    - If slippage=1 => 1%, we allow receiving as low as 99% of the desired quote
    // -----------------------------------------
    let precision = 1_000_000_000u128; // For slippage calculations
    let slippage_factor = ((1.0 - slippage / 100.0) * precision as f64) as u128;

    let min_quote = quote
        .checked_mul(slippage_factor)
        .and_then(|x| x.checked_div(precision))
        .ok_or_else(|| anyhow!("Math overflow in min_quote calculation"))?;

    Ok(SellQuoteInputResult {
        internal_raw_quote: raw_quote,
        base: base_amount_in, // amount of base tokens required to get the desired quote
        min_quote, // minimum acceptable tokens after applying slippage
    })
}

pub fn buy_quote_input_internal(
    quote: u128,
    slippage: f64, // 1.0 => 1%
    base_reserve: u128,
    quote_reserve: u128,
    lp_fee_bps: u128,
    protocol_fee_bps: u128,
    coin_creator_fee_bps: u128,
    coin_creator: Pubkey,
) -> Result<BuyQuoteInputResult> {
    // -----------------------------------------------------
    // 1) Basic validations
    // -----------------------------------------------------
    if base_reserve == 0 || quote_reserve == 0 {
        return Err(anyhow!("Invalid input: 'base_reserve' or 'quote_reserve' cannot be zero."));
    }

    // -----------------------------------------------------
    // 2) Calculate total fee basis points and denominator
    // -----------------------------------------------------
    let coin_creator_fee_bps_actual = if coin_creator == Pubkey::default() {
        0
    } else {
        coin_creator_fee_bps
    };

    let total_fee_bps = lp_fee_bps
        .checked_add(protocol_fee_bps)
        .and_then(|x| x.checked_add(coin_creator_fee_bps_actual))
        .ok_or_else(|| anyhow!("Math overflow in total_fee_bps calculation"))?;

    let denominator = 10_000u128
        .checked_add(total_fee_bps)
        .ok_or_else(|| anyhow!("Math overflow in denominator calculation"))?;

    // -----------------------------------------------------
    // 3) Calculate effective quote amount
    // -----------------------------------------------------
    let effective_quote = quote
        .checked_mul(10_000)
        .and_then(|x| x.checked_div(denominator))
        .ok_or_else(|| anyhow!("Math overflow in effective_quote calculation"))?;

    // -----------------------------------------------------
    // 4) Calculate the base tokens received using effective_quote
    //    base_amount_out = floor(base_reserve * effective_quote / (quote_reserve + effective_quote))
    // -----------------------------------------------------
    let numerator = base_reserve
        .checked_mul(effective_quote)
        .ok_or_else(|| anyhow!("Math overflow in numerator calculation"))?;

    let denominator_effective = quote_reserve
        .checked_add(effective_quote)
        .ok_or_else(|| anyhow!("Math overflow in denominator_effective calculation"))?;

    if denominator_effective == 0 {
        return Err(anyhow!("Pool would be depleted; denominator is zero."));
    }

    let base_amount_out = numerator / denominator_effective;

    // -----------------------------------------------------
    // 5) Calculate maxQuote with slippage
    //    If slippage=1.0 => factor = (1 + 1/100) = 1.01
    // -----------------------------------------------------
    let precision = 1_000_000_000u128; // For slippage calculations
    let slippage_factor = ((1.0 + slippage / 100.0) * precision as f64) as u128;

    // max_quote = quote * slippage_factor / precision
    let max_quote = quote
        .checked_mul(slippage_factor)
        .and_then(|x| x.checked_div(precision))
        .ok_or_else(|| anyhow!("Math overflow in max_quote calculation"))?;

    Ok(BuyQuoteInputResult {
        base: base_amount_out,
        internal_quote_without_fees: effective_quote,
        max_quote,
    })
}