use crate::math::BASIS_POINT_MAX;
use crate::safe_math::*;
use crate::{byte_reader::ByteReader, math::ONE_Q64};
use anchor_client::solana_sdk::pubkey::Pubkey;
use anyhow::{Result, anyhow};
use std::str::FromStr;

pub mod constants;
pub mod curve;
pub mod fee;
pub mod pda;
pub mod u128x128_math;
pub mod util;
pub mod util_math;

use constants::Q64_64_SCALE;
pub use curve::*;
pub use fee::*;
pub use pda::*;
pub use u128x128_math::*;
pub use util::*;
pub use util_math::*;

pub const PROGRAM_ID: &str = "cpamdpZCGKUy5JxQXB4dcpGPiikHawvSWAd6mEn1sGG";
pub const POOL_DISCRIMINATOR: [u8; 8] = [241, 154, 109, 4, 17, 177, 109, 188];

pub fn program_id() -> Pubkey {
    Pubkey::from_str(PROGRAM_ID).unwrap()
}

pub fn event_authority() -> Pubkey {
    Pubkey::from_str("3rmHSu74h1ZcmAisVcWerTCiRDQbUrBKmcwptYGjHfet").unwrap()
}

pub fn pool_authority() -> Pubkey {
    Pubkey::from_str("HLnpSz9h2S4hiLQ43rnSD9XkcUThA7B8hQMKmDaiTLcC").unwrap()
}

#[derive(Clone, Debug)]
pub struct BaseFeeStruct {
    pub cliff_fee_numerator: u64,
    pub fee_scheduler_mode: u8,
    pub padding_0: [u8; 5],
    pub number_of_period: u16,
    pub period_frequency: u64,
    pub reduction_factor: u64,
    pub padding_1: u64,
}

impl BaseFeeStruct {
    pub fn get_max_base_fee_numerator(&self) -> u64 {
        self.cliff_fee_numerator
    }
    pub fn get_min_base_fee_numerator(&self) -> Result<u64> {
        // trick to force current_point < activation_point
        self.get_current_base_fee_numerator(0, 1)
    }
    pub fn get_current_base_fee_numerator(
        &self,
        current_point: u64,
        activation_point: u64,
    ) -> Result<u64> {
        if self.period_frequency == 0 {
            return Ok(self.cliff_fee_numerator);
        }
        // can trade before activation point, so it is alpha-vault, we use min fee
        let period = if current_point < activation_point {
            self.number_of_period.into()
        } else {
            let period = current_point
                .safe_sub(activation_point)?
                .safe_div(self.period_frequency)?;
            period.min(self.number_of_period.into())
        };
        let fee_scheduler_mode = FeeSchedulerMode::try_from(self.fee_scheduler_mode)
            .map_err(|_| anyhow!("TypeCast Failed"))?;

        match fee_scheduler_mode {
            FeeSchedulerMode::Linear => {
                let fee_numerator = self
                    .cliff_fee_numerator
                    .safe_sub(period.safe_mul(self.reduction_factor.into())?)?;
                Ok(fee_numerator)
            }
            FeeSchedulerMode::Exponential => {
                let period = u16::try_from(period).map_err(|_| anyhow!("Math overflow"))?;
                let fee_numerator =
                    get_fee_in_period(self.cliff_fee_numerator, self.reduction_factor, period)?;
                Ok(fee_numerator)
            }
        }
    }
}

#[derive(Clone, Debug)]
pub struct DynamicFeeStruct {
    pub initialized: u8,
    pub padding: [u8; 7],
    pub max_volatility_accumulator: u32,
    pub variable_fee_control: u32,
    pub bin_step: u16,
    pub filter_period: u16,
    pub decay_period: u16,
    pub reduction_factor: u16,
    pub last_update_timestamp: u64,
    pub bin_step_u128: u128,
    pub sqrt_price_reference: u128,
    pub volatility_accumulator: u128,
    pub volatility_reference: u128,
}

impl DynamicFeeStruct {
    // we approximate Px / Py = (1 + b) ^ delta_bin  = 1 + b * delta_bin (if b is too small)
    // Ex: (1+1/10000)^ 5000 / (1+5000 * 1/10000) = 1.1 (10% diff if sqrt_price diff is (1+1/10000)^ 5000 = 1.64 times)
    pub fn get_delta_bin_id(
        bin_step_u128: u128,
        sqrt_price_a: u128,
        sqrt_price_b: u128,
    ) -> Result<u128> {
        let (upper_sqrt_price, lower_sqrt_price) = if sqrt_price_a > sqrt_price_b {
            (sqrt_price_a, sqrt_price_b)
        } else {
            (sqrt_price_b, sqrt_price_a)
        };

        let price_ratio: u128 =
            safe_shl_div_cast(upper_sqrt_price, lower_sqrt_price, 64, Rounding::Down)?;

        let delta_bin_id = price_ratio.safe_sub(ONE_Q64)?.safe_div(bin_step_u128)?;

        Ok(delta_bin_id.safe_mul(2)?)
    }
    pub fn update_volatility_accumulator(&mut self, sqrt_price: u128) -> Result<()> {
        let delta_price =
            Self::get_delta_bin_id(self.bin_step_u128, sqrt_price, self.sqrt_price_reference)?;

        let volatility_accumulator = self
            .volatility_reference
            .safe_add(delta_price.safe_mul(BASIS_POINT_MAX.into())?)?;

        self.volatility_accumulator = std::cmp::min(
            volatility_accumulator,
            self.max_volatility_accumulator.into(),
        );
        Ok(())
    }

    pub fn update_references(
        &mut self,
        sqrt_price_current: u128,
        current_timestamp: u64,
    ) -> Result<()> {
        let elapsed = current_timestamp.safe_sub(self.last_update_timestamp)?;
        // Not high frequency trade
        if elapsed >= self.filter_period as u64 {
            // Update sqrt of last transaction
            self.sqrt_price_reference = sqrt_price_current;
            // filter period < t < decay_period. Decay time window.
            if elapsed < self.decay_period as u64 {
                let volatility_reference = self
                    .volatility_accumulator
                    .safe_mul(self.reduction_factor.into())?
                    .safe_div(BASIS_POINT_MAX.into())?;

                self.volatility_reference = volatility_reference;
            }
            // Out of decay time window
            else {
                self.volatility_reference = 0;
            }
        }
        Ok(())
    }

    pub fn is_dynamic_fee_enable(&self) -> bool {
        self.initialized != 0
    }

    pub fn get_variable_fee(&self) -> Result<u128> {
        if self.is_dynamic_fee_enable() {
            let square_vfa_bin: u128 = self
                .volatility_accumulator
                .safe_mul(self.bin_step.into())?
                .checked_pow(2)
                .unwrap();
            // Variable fee control, volatility accumulator, bin step are in basis point unit (10_000)
            // This is 1e20. Which > 1e9. Scale down it to 1e9 unit and ceiling the remaining.
            let v_fee = square_vfa_bin.safe_mul(self.variable_fee_control.into())?;

            let scaled_v_fee = v_fee.safe_add(99_999_999_999)?.safe_div(100_000_000_000)?;

            Ok(scaled_v_fee)
        } else {
            Ok(0)
        }
    }
}

#[derive(Clone, Debug)]
pub struct PoolFeesStruct {
    pub base_fee: BaseFeeStruct,
    pub protocol_fee_percent: u8,
    pub partner_fee_percent: u8,
    pub referral_fee_percent: u8,
    pub padding_0: [u8; 5],
    pub dynamic_fee: DynamicFeeStruct,
    pub padding_1: [u64; 2],
}

impl PoolFeesStruct {
    // in numerator
    pub fn get_total_trading_fee(&self, current_point: u64, activation_point: u64) -> Result<u128> {
        let base_fee_numerator = self
            .base_fee
            .get_current_base_fee_numerator(current_point, activation_point)?;
        let total_fee_numerator = self
            .dynamic_fee
            .get_variable_fee()?
            .safe_add(base_fee_numerator.into())?;
        Ok(total_fee_numerator)
    }

    pub fn get_fee_on_amount(
        &self,
        amount: u64,
        has_referral: bool,
        current_point: u64,
        activation_point: u64,
    ) -> Result<FeeOnAmountResult> {
        let trade_fee_numerator = self.get_total_trading_fee(current_point, activation_point)?;
        let trade_fee_numerator =
            if trade_fee_numerator > (constants::fee::MAX_FEE_NUMERATOR as u128) {
                constants::fee::MAX_FEE_NUMERATOR
            } else {
                trade_fee_numerator.try_into().unwrap()
            };
        let lp_fee: u64 = safe_mul_div_cast_u64(
            amount,
            trade_fee_numerator,
            constants::fee::FEE_DENOMINATOR,
            Rounding::Up,
        )?;
        // update amount
        let amount = amount.safe_sub(lp_fee)?;

        let protocol_fee = safe_mul_div_cast_u64(
            lp_fee,
            self.protocol_fee_percent.into(),
            100,
            Rounding::Down,
        )?;
        // update lp fee
        let lp_fee = lp_fee.safe_sub(protocol_fee)?;

        let referral_fee = if has_referral {
            safe_mul_div_cast_u64(
                protocol_fee,
                self.referral_fee_percent.into(),
                100,
                Rounding::Down,
            )?
        } else {
            0
        };

        let protocol_fee_after_referral_fee = protocol_fee.safe_sub(referral_fee)?;
        let partner_fee = safe_mul_div_cast_u64(
            protocol_fee_after_referral_fee,
            self.partner_fee_percent.into(),
            100,
            Rounding::Down,
        )?;

        let protocol_fee = protocol_fee_after_referral_fee.safe_sub(partner_fee)?;

        Ok(FeeOnAmountResult {
            amount,
            lp_fee,
            protocol_fee,
            partner_fee,
            referral_fee,
        })
    }
}

#[derive(Clone, Debug)]
pub struct PoolMetrics {
    pub total_lp_a_fee: u128,
    pub total_lp_b_fee: u128,
    pub total_protocol_a_fee: u64,
    pub total_protocol_b_fee: u64,
    pub total_partner_a_fee: u64,
    pub total_partner_b_fee: u64,
    pub total_position: u64,
    pub padding: u64,
}

#[derive(Clone, Debug)]
pub struct RewardInfo {
    pub initialized: u8,
    pub reward_token_flag: u8,
    pub _padding_0: [u8; 6],
    pub _padding_1: [u8; 8],
    pub mint: Pubkey,
    pub vault: Pubkey,
    pub funder: Pubkey,
    pub reward_duration: u64,
    pub reward_duration_end: u64,
    pub reward_rate: u128,
    pub reward_per_token_stored: [u8; 32],
    pub last_update_time: u64,
    pub cumulative_seconds_with_empty_liquidity_reward: u64,
}

#[derive(Clone, Debug)]
pub struct Pool {
    pub pool_fees: PoolFeesStruct,
    pub token_a_mint: Pubkey,
    pub token_b_mint: Pubkey,
    pub token_a_vault: Pubkey,
    pub token_b_vault: Pubkey,
    pub whitelisted_vault: Pubkey,
    pub partner: Pubkey,
    pub liquidity: u128,
    pub _padding: u128,
    pub protocol_a_fee: u64,
    pub protocol_b_fee: u64,
    pub partner_a_fee: u64,
    pub partner_b_fee: u64,
    pub sqrt_min_price: u128,
    pub sqrt_max_price: u128,
    pub sqrt_price: u128,
    pub activation_point: u64,
    pub activation_type: u8,
    pub pool_status: u8,
    pub token_a_flag: u8,
    pub token_b_flag: u8,
    pub collect_fee_mode: u8,
    pub pool_type: u8,
    pub _padding_0: [u8; 2],
    pub fee_a_per_liquidity: [u8; 32],
    pub fee_b_per_liquidity: [u8; 32],
    pub permanent_lock_liquidity: u128,
    pub metrics: PoolMetrics,
    pub creator: Pubkey,
    pub _padding_1: [u64; 6],
    pub reward_infos: [RewardInfo; 2],
}

impl Pool {
    pub const DISCRIMINATOR: [u8; 8] = [241, 154, 109, 4, 17, 177, 109, 188];

    /// Deserialize pool account data
    pub fn deserialize(data: &[u8]) -> Result<Self> {
        // Check discriminator
        if data.len() < 8 {
            return Err(anyhow!("Account data too short"));
        }

        if data[0..8] != Self::DISCRIMINATOR {
            return Err(anyhow!("Invalid account discriminator"));
        }

        // Skip discriminator and start reading
        let pool_data = &data[8..];
        let mut reader = ByteReader::new(pool_data);

        // Read PoolFeesStruct
        let pool_fees = Self::read_pool_fees(&mut reader)?;

        // Read main pool fields
        let token_a_mint = reader.read_pubkey()?;
        let token_b_mint = reader.read_pubkey()?;
        let token_a_vault = reader.read_pubkey()?;
        let token_b_vault = reader.read_pubkey()?;
        let whitelisted_vault = reader.read_pubkey()?;
        let partner = reader.read_pubkey()?;
        let liquidity = reader.read_u128()?;
        let _padding = reader.read_u128()?;
        let protocol_a_fee = reader.read_u64()?;
        let protocol_b_fee = reader.read_u64()?;
        let partner_a_fee = reader.read_u64()?;
        let partner_b_fee = reader.read_u64()?;
        let sqrt_min_price = reader.read_u128()?;
        let sqrt_max_price = reader.read_u128()?;
        let sqrt_price = reader.read_u128()?;
        let activation_point = reader.read_u64()?;
        let activation_type = reader.read_u8()?;
        let pool_status = reader.read_u8()?;
        let token_a_flag = reader.read_u8()?;
        let token_b_flag = reader.read_u8()?;
        let collect_fee_mode = reader.read_u8()?;
        let pool_type = reader.read_u8()?;
        let _padding_0 = reader.read_bytes_array::<2>()?;
        let fee_a_per_liquidity = reader.read_bytes_array::<32>()?;
        let fee_b_per_liquidity = reader.read_bytes_array::<32>()?;
        let permanent_lock_liquidity = reader.read_u128()?;

        // Read PoolMetrics
        let metrics = Self::read_pool_metrics(&mut reader)?;

        let creator = reader.read_pubkey()?;

        // Read padding_1 (6 u64s)
        let mut _padding_1 = [0u64; 6];
        for i in 0..6 {
            _padding_1[i] = reader.read_u64()?;
        }

        // Read RewardInfos (2 entries)
        let mut reward_infos = Vec::new();
        for _ in 0..2 {
            reward_infos.push(Self::read_reward_info(&mut reader)?);
        }

        Ok(Pool {
            pool_fees,
            token_a_mint,
            token_b_mint,
            token_a_vault,
            token_b_vault,
            whitelisted_vault,
            partner,
            liquidity,
            _padding,
            protocol_a_fee,
            protocol_b_fee,
            partner_a_fee,
            partner_b_fee,
            sqrt_min_price,
            sqrt_max_price,
            sqrt_price,
            activation_point,
            activation_type,
            pool_status,
            token_a_flag,
            token_b_flag,
            collect_fee_mode,
            pool_type,
            _padding_0,
            fee_a_per_liquidity,
            fee_b_per_liquidity,
            permanent_lock_liquidity,
            metrics,
            creator,
            _padding_1,
            reward_infos: [reward_infos[0].clone(), reward_infos[1].clone()],
        })
    }

    fn read_base_fee(reader: &mut ByteReader) -> Result<BaseFeeStruct> {
        Ok(BaseFeeStruct {
            cliff_fee_numerator: reader.read_u64()?,
            fee_scheduler_mode: reader.read_u8()?,
            padding_0: reader.read_bytes_array::<5>()?,
            number_of_period: reader.read_u16()?,
            period_frequency: reader.read_u64()?,
            reduction_factor: reader.read_u64()?,
            padding_1: reader.read_u64()?,
        })
    }

    fn read_dynamic_fee(reader: &mut ByteReader) -> Result<DynamicFeeStruct> {
        Ok(DynamicFeeStruct {
            initialized: reader.read_u8()?,
            padding: reader.read_bytes_array::<7>()?,
            max_volatility_accumulator: reader.read_u32()?,
            variable_fee_control: reader.read_u32()?,
            bin_step: reader.read_u16()?,
            filter_period: reader.read_u16()?,
            decay_period: reader.read_u16()?,
            reduction_factor: reader.read_u16()?,
            last_update_timestamp: reader.read_u64()?,
            bin_step_u128: reader.read_u128()?,
            sqrt_price_reference: reader.read_u128()?,
            volatility_accumulator: reader.read_u128()?,
            volatility_reference: reader.read_u128()?,
        })
    }

    fn read_pool_fees(reader: &mut ByteReader) -> Result<PoolFeesStruct> {
        let base_fee = Self::read_base_fee(reader)?;

        Ok(PoolFeesStruct {
            base_fee,
            protocol_fee_percent: reader.read_u8()?,
            partner_fee_percent: reader.read_u8()?,
            referral_fee_percent: reader.read_u8()?,
            padding_0: reader.read_bytes_array::<5>()?,
            dynamic_fee: Self::read_dynamic_fee(reader)?,
            padding_1: [reader.read_u64()?, reader.read_u64()?],
        })
    }

    fn read_pool_metrics(reader: &mut ByteReader) -> Result<PoolMetrics> {
        Ok(PoolMetrics {
            total_lp_a_fee: reader.read_u128()?,
            total_lp_b_fee: reader.read_u128()?,
            total_protocol_a_fee: reader.read_u64()?,
            total_protocol_b_fee: reader.read_u64()?,
            total_partner_a_fee: reader.read_u64()?,
            total_partner_b_fee: reader.read_u64()?,
            total_position: reader.read_u64()?,
            padding: reader.read_u64()?,
        })
    }

    fn read_reward_info(reader: &mut ByteReader) -> Result<RewardInfo> {
        Ok(RewardInfo {
            initialized: reader.read_u8()?,
            reward_token_flag: reader.read_u8()?,
            _padding_0: reader.read_bytes_array::<6>()?,
            _padding_1: reader.read_bytes_array::<8>()?,
            mint: reader.read_pubkey()?,
            vault: reader.read_pubkey()?,
            funder: reader.read_pubkey()?,
            reward_duration: reader.read_u64()?,
            reward_duration_end: reader.read_u64()?,
            reward_rate: reader.read_u128()?,
            reward_per_token_stored: reader.read_bytes_array::<32>()?,
            last_update_time: reader.read_u64()?,
            cumulative_seconds_with_empty_liquidity_reward: reader.read_u64()?,
        })
    }

    pub fn get_price(&self) -> f64 {
        if self.sqrt_price == 0 {
            return 0.0;
        }
        // sqrt_price_x64 is Q64.64 format stored as u128
        // Convert to f64 and square it
        let sqrt_price = self.sqrt_price as f64 / Q64_64_SCALE;
        sqrt_price * sqrt_price
    }

    pub fn get_swap_result(
        &self,
        amount_in: u64,
        fee_mode: &FeeMode,
        trade_direction: TradeDirection,
        current_point: u64,
    ) -> Result<SwapResult> {
        let mut actual_protocol_fee = 0;
        let mut actual_lp_fee = 0;
        let mut actual_referral_fee = 0;
        let mut actual_partner_fee = 0;

        let actual_amount_in = if fee_mode.fees_on_input {
            let FeeOnAmountResult {
                amount,
                lp_fee,
                protocol_fee,
                partner_fee,
                referral_fee,
            } = self.pool_fees.get_fee_on_amount(
                amount_in,
                fee_mode.has_referral,
                current_point,
                self.activation_point,
            )?;

            actual_protocol_fee = protocol_fee;
            actual_lp_fee = lp_fee;
            actual_referral_fee = referral_fee;
            actual_partner_fee = partner_fee;

            amount
        } else {
            amount_in
        };

        let SwapAmount {
            output_amount,
            next_sqrt_price,
        } = match trade_direction {
            TradeDirection::AtoB => self.get_swap_result_from_a_to_b(actual_amount_in),
            TradeDirection::BtoA => self.get_swap_result_from_b_to_a(actual_amount_in),
        }?;

        let actual_amount_out = if fee_mode.fees_on_input {
            output_amount
        } else {
            let FeeOnAmountResult {
                amount,
                lp_fee,
                protocol_fee,
                partner_fee,
                referral_fee,
            } = self.pool_fees.get_fee_on_amount(
                output_amount,
                fee_mode.has_referral,
                current_point,
                self.activation_point,
            )?;
            actual_protocol_fee = protocol_fee;
            actual_lp_fee = lp_fee;
            actual_referral_fee = referral_fee;
            actual_partner_fee = partner_fee;
            amount
        };

        Ok(SwapResult {
            output_amount: actual_amount_out,
            next_sqrt_price,
            lp_fee: actual_lp_fee,
            protocol_fee: actual_protocol_fee,
            partner_fee: actual_partner_fee,
            referral_fee: actual_referral_fee,
        })
    }
    fn get_swap_result_from_a_to_b(&self, amount_in: u64) -> Result<SwapAmount> {
        // finding new target price
        let next_sqrt_price =
            get_next_sqrt_price_from_input(self.sqrt_price, self.liquidity, amount_in, true)?;

        if next_sqrt_price < self.sqrt_min_price {
            return Err(anyhow!("PriceRangeViolent"));
        }

        // finding output amount
        let output_amount = get_delta_amount_b_unsigned(
            next_sqrt_price,
            self.sqrt_price,
            self.liquidity,
            Rounding::Down,
        )?;

        Ok(SwapAmount {
            output_amount,
            next_sqrt_price,
        })
    }

    fn get_swap_result_from_b_to_a(&self, amount_in: u64) -> Result<SwapAmount> {
        // finding new target price
        let next_sqrt_price =
            get_next_sqrt_price_from_input(self.sqrt_price, self.liquidity, amount_in, false)?;

        if next_sqrt_price > self.sqrt_max_price {
            return Err(anyhow!("PriceRangeViolent"));
        }
        // finding output amount
        let output_amount = get_delta_amount_a_unsigned(
            self.sqrt_price,
            next_sqrt_price,
            self.liquidity,
            Rounding::Down,
        )?;

        Ok(SwapAmount {
            output_amount,
            next_sqrt_price,
        })
    }

    pub fn update_pre_swap(&mut self, current_timestamp: u64) -> Result<()> {
        if self.pool_fees.dynamic_fee.is_dynamic_fee_enable() {
            self.pool_fees
                .dynamic_fee
                .update_references(self.sqrt_price, current_timestamp)?;
        }
        Ok(())
    }

    pub fn update_post_swap(&mut self, old_sqrt_price: u128, current_timestamp: u64) -> Result<()> {
        if self.pool_fees.dynamic_fee.is_dynamic_fee_enable() {
            self.pool_fees
                .dynamic_fee
                .update_volatility_accumulator(self.sqrt_price)?;

            // update only last_update_timestamp if bin is crossed
            let delta_price = DynamicFeeStruct::get_delta_bin_id(
                self.pool_fees.dynamic_fee.bin_step_u128,
                old_sqrt_price,
                self.sqrt_price,
            )?;
            if delta_price > 0 {
                self.pool_fees.dynamic_fee.last_update_timestamp = current_timestamp;
            }
        }
        Ok(())
    }
}

#[derive(Debug, PartialEq)]
pub struct SwapResult {
    pub output_amount: u64,
    pub next_sqrt_price: u128,
    pub lp_fee: u64,
    pub protocol_fee: u64,
    pub partner_fee: u64,
    pub referral_fee: u64,
}

pub struct SwapAmount {
    output_amount: u64,
    next_sqrt_price: u128,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum ActivationType {
    Slot,
    Timestamp,
}

impl TryFrom<u8> for ActivationType {
    type Error = anyhow::Error;

    fn try_from(value: u8) -> Result<Self> {
        match value {
            0 => Ok(ActivationType::Slot),
            1 => Ok(ActivationType::Timestamp),
            _ => Err(anyhow!("Invalid activation_type value: {}", value)),
        }
    }
}

pub fn get_quote(
    pool: &Pool,
    current_timestamp: u64,
    current_slot: u64,
    actual_amount_in: u64,
    a_to_b: bool,
    has_referral: bool,
) -> Result<SwapResult> {
    let result = if pool.pool_fees.dynamic_fee.is_dynamic_fee_enable() {
        let mut pool = pool.clone();
        pool.update_pre_swap(current_timestamp)?;
        get_internal_quote(
            &pool,
            current_timestamp,
            current_slot,
            actual_amount_in,
            a_to_b,
            has_referral,
        )
    } else {
        get_internal_quote(
            pool,
            current_timestamp,
            current_slot,
            actual_amount_in,
            a_to_b,
            has_referral,
        )
    };

    result
}

fn get_internal_quote(
    pool: &Pool,
    current_timestamp: u64,
    current_slot: u64,
    actual_amount_in: u64,
    a_to_b: bool,
    has_referral: bool,
) -> Result<SwapResult> {
    let activation_type = ActivationType::try_from(pool.activation_type)?;
    let current_point = match activation_type {
        ActivationType::Slot => current_slot,
        ActivationType::Timestamp => current_timestamp,
    };

    let trade_direction = if a_to_b {
        TradeDirection::AtoB
    } else {
        TradeDirection::BtoA
    };

    let fee_mode = &FeeMode::get_fee_mode(pool.collect_fee_mode, trade_direction, has_referral)?;

    let swap_result =
        pool.get_swap_result(actual_amount_in, fee_mode, trade_direction, current_point)?;

    Ok(swap_result)
}
