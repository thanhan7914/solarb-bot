use anyhow::{Result, anyhow};
use crate::{math::{pow, BASIS_POINT_MAX, ONE_Q64, SCALE_OFFSET}, safe_math::*};

#[derive(Debug, PartialEq)]
pub struct FeeOnAmountResult {
    pub amount: u64,
    pub lp_fee: u64,
    pub protocol_fee: u64,
    pub partner_fee: u64,
    pub referral_fee: u64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
// https://www.desmos.com/calculator/oxdndn2xdx
pub enum FeeSchedulerMode {
    // fee = cliff_fee_numerator - passed_period * reduction_factor
    Linear,
    // fee = cliff_fee_numerator * (1-reduction_factor/10_000)^passed_period
    Exponential,
}

impl TryFrom<u8> for FeeSchedulerMode {
    type Error = anyhow::Error;

    fn try_from(value: u8) -> Result<Self> {
        match value {
            0 => Ok(FeeSchedulerMode::Linear),
            1 => Ok(FeeSchedulerMode::Exponential),
            _ => Err(anyhow!("Invalid fee_scheduler_mode value: {}", value)),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum CollectFeeMode {
    /// Both token, in this mode only out token is collected
    BothToken,
    /// Only token B, we just need token B, because if user want to collect fee in token A, they just need to flip order of tokens
    OnlyB,
}

impl TryFrom<u8> for CollectFeeMode {
    type Error = anyhow::Error;

    fn try_from(value: u8) -> Result<Self> {
        match value {
            0 => Ok(CollectFeeMode::BothToken),
            1 => Ok(CollectFeeMode::BothToken),
            2 => Ok(CollectFeeMode::OnlyB),
            _ => Err(anyhow!("Invalid collect_fee_mode value: {}", value)),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum TradeDirection {
    /// Input token A, output token B
    AtoB,
    /// Input token B, output token A
    BtoA,
}

#[derive(Default, Debug)]
pub struct FeeMode {
    pub fees_on_input: bool,
    pub fees_on_token_a: bool,
    pub has_referral: bool,
}

impl FeeMode {
    pub fn get_fee_mode(
        collect_fee_mode: u8,
        trade_direction: TradeDirection,
        has_referral: bool,
    ) -> Result<FeeMode> {
        let collect_fee_mode = CollectFeeMode::try_from(collect_fee_mode)?;

        let (fees_on_input, fees_on_token_a) = match (collect_fee_mode, trade_direction) {
            // When collecting fees on output token
            (CollectFeeMode::BothToken, TradeDirection::AtoB) => (false, false),
            (CollectFeeMode::BothToken, TradeDirection::BtoA) => (false, true),

            // When collecting fees on tokenB
            (CollectFeeMode::OnlyB, TradeDirection::AtoB) => (false, false),
            (CollectFeeMode::OnlyB, TradeDirection::BtoA) => (true, false),
        };

        Ok(FeeMode {
            fees_on_input,
            fees_on_token_a,
            has_referral,
        })
    }
}

pub fn get_fee_in_period(
    cliff_fee_numerator: u64,
    reduction_factor: u64,
    passed_period: u16,
) -> Result<u64> {
    if reduction_factor == 0 {
        return Ok(cliff_fee_numerator);
    }
    // Make bin_step into Q64x64, and divided by BASIS_POINT_MAX. If bin_step = 1, we get 0.0001 in Q64x64
    let bps = u128::from(reduction_factor)
        .safe_shl(SCALE_OFFSET.into())?
        .safe_div(BASIS_POINT_MAX.into())?;
    let base = ONE_Q64.safe_sub(bps)?;
    let result = pow(base, passed_period.into()).ok_or_else(|| anyhow!("Math overflow"))?;

    let (fee, _) = result
        .safe_mul(cliff_fee_numerator.into())?
        .overflowing_shr(SCALE_OFFSET);

    let fee_numerator = u64::try_from(fee).map_err(|_| anyhow!("TypeCast Failed"))?;
    Ok(fee_numerator)
}
