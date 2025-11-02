use crate::{byte_reader::ByteReader};
use anchor_client::solana_sdk::pubkey::Pubkey;
use anyhow::{Result, anyhow};
use std::str::FromStr;

pub mod math;
pub mod pda;
pub mod serum;
pub mod util;

pub use math::*;
pub use pda::*;

#[cfg(feature = "devnet")]
pub const PROGRAM_ID: &str = "HWy1jotHpo6UqeQxx49dpYYdQB8wj9Qk9MdxwjLvDHB8";

#[cfg(feature = "devnet")]
pub const OPENBOOK_ID: &str = "EoTcMgcDRTJVZDMZWBoU6rhYHZfkNTVEAfz3uUJRcYGj";

#[cfg(not(feature = "devnet"))]
pub const PROGRAM_ID: &str = "675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8";

#[cfg(not(feature = "devnet"))]
pub const OPENBOOK_ID: &str = "srmqPvymJeFKQ4zGQed1GFppgkRHL9kaELCbyksJtPX";

pub const POOL_DISCRIMINATOR: [u8; 8] = [6, 0, 0, 0, 0, 0, 0, 0];

pub fn program_id() -> Pubkey {
    Pubkey::from_str(PROGRAM_ID).unwrap()
}

pub fn openbook_id() -> Pubkey {
    Pubkey::from_str(OPENBOOK_ID).unwrap()
}

pub fn authority() -> Pubkey {
    Pubkey::from_str("5Q544fKrFoe6tsEbD7S8EmxGTJYAKtTVhAW5Q5pge4j1").unwrap()
}

#[derive(Clone, Debug)]
pub struct Fees {
    pub min_separate_numerator: u64,
    pub min_separate_denominator: u64,
    pub trade_fee_numerator: u64,
    pub trade_fee_denominator: u64,
    pub pnl_numerator: u64,
    pub pnl_denominator: u64,
    pub swap_fee_numerator: u64,
    pub swap_fee_denominator: u64,
}

#[derive(Clone, Debug)]
pub struct OutPutData {
    pub need_take_pnl_coin: u64,
    pub need_take_pnl_pc: u64,
    pub total_pnl_pc: u64,
    pub total_pnl_coin: u64,
    pub pool_open_time: u64,
    pub punish_pc_amount: u64,
    pub punish_coin_amount: u64,
    pub orderbook_to_init_time: u64,
    pub swap_coin_in_amount: u128,
    pub swap_pc_out_amount: u128,
    pub swap_take_pc_fee: u64,
    pub swap_pc_in_amount: u128,
    pub swap_coin_out_amount: u128,
    pub swap_take_coin_fee: u64,
}

#[derive(Clone, Debug)]
pub struct AmmInfo {
    pub status: u64,
    pub nonce: u64,
    pub order_num: u64,
    pub depth: u64,
    pub coin_decimals: u64,
    pub pc_decimals: u64,
    pub state: u64,
    pub reset_flag: u64,
    pub min_size: u64,
    pub vol_max_cut_ratio: u64,
    pub amount_wave: u64,
    pub coin_lot_size: u64,
    pub pc_lot_size: u64,
    pub min_price_multiplier: u64,
    pub max_price_multiplier: u64,
    pub sys_decimal_value: u64,
    pub fees: Fees,
    pub out_put: OutPutData,
    pub token_coin: Pubkey,
    pub token_pc: Pubkey,
    pub coin_mint: Pubkey,
    pub pc_mint: Pubkey,
    pub lp_mint: Pubkey,
    pub open_orders: Pubkey,
    pub market: Pubkey,
    pub serum_dex: Pubkey,
    pub target_orders: Pubkey,
    pub withdraw_queue: Pubkey,
    pub token_temp_lp: Pubkey,
    pub amm_owner: Pubkey,
    pub lp_amount: u64,
    pub client_order_id: u64,
    pub padding: [u64; 2],
}

#[derive(Debug, Clone)]
pub struct PoolVaults {
    pub coin_vault_amount: u64,
    pub pc_vault_amount: u64,
    pub coin_vault: Pubkey,
    pub pc_vault: Pubkey,
}

impl AmmInfo {
    pub fn deserialize(data: &[u8]) -> Result<Self> {
        let mut reader = ByteReader::new(data);

        let status = reader.read_u64()?;
        let nonce = reader.read_u64()?;
        let order_num = reader.read_u64()?;
        let depth = reader.read_u64()?;
        let coin_decimals = reader.read_u64()?;
        let pc_decimals = reader.read_u64()?;
        let state = reader.read_u64()?;
        let reset_flag = reader.read_u64()?;
        let min_size = reader.read_u64()?;
        let vol_max_cut_ratio = reader.read_u64()?;
        let amount_wave = reader.read_u64()?;
        let coin_lot_size = reader.read_u64()?;
        let pc_lot_size = reader.read_u64()?;
        let min_price_multiplier = reader.read_u64()?;
        let max_price_multiplier = reader.read_u64()?;
        let sys_decimal_value = reader.read_u64()?;

        let fees = Self::read_fees(&mut reader)?;
        let out_put = Self::read_output_data(&mut reader)?;

        let token_coin = reader.read_pubkey()?;
        let token_pc = reader.read_pubkey()?;
        let coin_mint = reader.read_pubkey()?;
        let pc_mint = reader.read_pubkey()?;
        let lp_mint = reader.read_pubkey()?;
        let open_orders = reader.read_pubkey()?;
        let market = reader.read_pubkey()?;
        let serum_dex = reader.read_pubkey()?;
        let target_orders = reader.read_pubkey()?;
        let withdraw_queue = reader.read_pubkey()?;
        let token_temp_lp = reader.read_pubkey()?;
        let amm_owner = reader.read_pubkey()?;

        let lp_amount = reader.read_u64()?;
        let client_order_id = reader.read_u64()?;

        let mut padding = [0u64; 2];
        for i in 0..2 {
            padding[i] = reader.read_u64()?;
        }

        Ok(AmmInfo {
            status,
            nonce,
            order_num,
            depth,
            coin_decimals,
            pc_decimals,
            state,
            reset_flag,
            min_size,
            vol_max_cut_ratio,
            amount_wave,
            coin_lot_size,
            pc_lot_size,
            min_price_multiplier,
            max_price_multiplier,
            sys_decimal_value,
            fees,
            out_put,
            token_coin,
            token_pc,
            coin_mint,
            pc_mint,
            lp_mint,
            open_orders,
            market,
            serum_dex,
            target_orders,
            withdraw_queue,
            token_temp_lp,
            amm_owner,
            lp_amount,
            client_order_id,
            padding,
        })
    }

    fn read_fees(reader: &mut ByteReader) -> Result<Fees> {
        Ok(Fees {
            min_separate_numerator: reader.read_u64()?,
            min_separate_denominator: reader.read_u64()?,
            trade_fee_numerator: reader.read_u64()?,
            trade_fee_denominator: reader.read_u64()?,
            pnl_numerator: reader.read_u64()?,
            pnl_denominator: reader.read_u64()?,
            swap_fee_numerator: reader.read_u64()?,
            swap_fee_denominator: reader.read_u64()?,
        })
    }

    fn read_output_data(reader: &mut ByteReader) -> Result<OutPutData> {
        Ok(OutPutData {
            need_take_pnl_coin: reader.read_u64()?,
            need_take_pnl_pc: reader.read_u64()?,
            total_pnl_pc: reader.read_u64()?,
            total_pnl_coin: reader.read_u64()?,
            pool_open_time: reader.read_u64()?,
            punish_pc_amount: reader.read_u64()?,
            punish_coin_amount: reader.read_u64()?,
            orderbook_to_init_time: reader.read_u64()?,
            swap_coin_in_amount: reader.read_u128()?,
            swap_pc_out_amount: reader.read_u128()?,
            swap_take_pc_fee: reader.read_u64()?,
            swap_pc_in_amount: reader.read_u128()?,
            swap_coin_out_amount: reader.read_u128()?,
            swap_take_coin_fee: reader.read_u64()?,
        })
    }

    pub fn derive_vault_signer(&self, vault_signer_nonce: u64) -> Result<Pubkey> {
        match Pubkey::create_program_address(
            &[self.market.as_ref(), &vault_signer_nonce.to_le_bytes()],
            &openbook_id(),
        ) {
            Ok(pda) => Ok(pda),
            Err(e) => Err(anyhow!("Failed to create vault signer PDA: {}", e)),
        }
    }
}
