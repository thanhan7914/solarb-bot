use crate::byte_reader::ByteReader;
use anchor_client::solana_sdk::pubkey::Pubkey;
use anyhow::{Result, anyhow};
use std::str::FromStr;

pub mod curve;
pub mod pda;
pub mod util;

#[cfg(feature = "devnet")]
pub const RAYDIUM_CPMM_PROGRAM_ID: &str = "CPMDWBwJDtYax9qW7AyRuVC19Cc4L4Vcy4n2BHAbHkCW";

#[cfg(not(feature = "devnet"))]
pub const RAYDIUM_CPMM_PROGRAM_ID: &str = "CPMMoo8L3F4NbTegBCKVNunggL7H1ZpdTHKxQB5qKP1C";

pub const POOL_DISCRIMINATOR: [u8; 8] = [247, 237, 227, 245, 215, 195, 222, 70];

pub fn program_id() -> Pubkey {
    Pubkey::from_str(RAYDIUM_CPMM_PROGRAM_ID).unwrap()
}

pub fn authority() -> Pubkey {
    Pubkey::from_str("GpMZbSM2GgvTKHJirzeGfMFoaZ8UR2X7F4v8vHTvxFbL").unwrap()
}

#[derive(Debug, Clone)]
pub struct PoolState {
    pub amm_config: Pubkey,
    pub pool_creator: Pubkey,
    pub token_0_vault: Pubkey,
    pub token_1_vault: Pubkey,
    pub lp_mint: Pubkey,
    pub token_0_mint: Pubkey,
    pub token_1_mint: Pubkey,
    pub token_0_program: Pubkey,
    pub token_1_program: Pubkey,
    pub observation_key: Pubkey,
    pub auth_bump: u8,
    pub status: u8,
    pub lp_mint_decimals: u8,
    pub mint_0_decimals: u8,
    pub mint_1_decimals: u8,
    pub lp_supply: u64,
    pub protocol_fees_token_0: u64,
    pub protocol_fees_token_1: u64,
    pub fund_fees_token_0: u64,
    pub fund_fees_token_1: u64,
    pub open_time: u64,
    pub recent_epoch: u64,
    pub padding: [u64; 31],
}

impl PoolState {
    pub fn deserialize(data: &[u8]) -> Result<Self> {
        let mut reader = ByteReader::new(data);

        // Skip the discriminator (first 8 bytes) - [247,237,227,245,215,195,222,70]
        reader.skip(8)?;

        // Read 10 pubkeys (10 * 32 = 320 bytes) - NO PADDING between fields in packed struct
        let amm_config = reader.read_pubkey()?;
        let pool_creator = reader.read_pubkey()?;
        let token_0_vault = reader.read_pubkey()?;
        let token_1_vault = reader.read_pubkey()?;
        let lp_mint = reader.read_pubkey()?;
        let token_0_mint = reader.read_pubkey()?;
        let token_1_mint = reader.read_pubkey()?;
        let token_0_program = reader.read_pubkey()?;
        let token_1_program = reader.read_pubkey()?;
        let observation_key = reader.read_pubkey()?;

        // Read 5 u8 fields (5 bytes) - NO PADDING in packed struct
        let auth_bump = reader.read_u8()?;
        let status = reader.read_u8()?;
        let lp_mint_decimals = reader.read_u8()?;
        let mint_0_decimals = reader.read_u8()?;
        let mint_1_decimals = reader.read_u8()?;

        // NO ALIGNMENT PADDING - packed struct means fields are laid out consecutively
        // Offset at this point: 8 + 320 + 5 = 333 bytes

        // Read u64 fields directly (no alignment needed in packed struct)
        let lp_supply = reader.read_u64()?;
        let protocol_fees_token_0 = reader.read_u64()?;
        let protocol_fees_token_1 = reader.read_u64()?;
        let fund_fees_token_0 = reader.read_u64()?;
        let fund_fees_token_1 = reader.read_u64()?;
        let open_time = reader.read_u64()?;
        let recent_epoch = reader.read_u64()?;

        // Offset at this point: 333 + (7 * 8) = 333 + 56 = 389 bytes
        // Remaining for padding: 637 - 389 = 248 bytes = 31 u64 values

        // Read padding array (exactly 31 u64 values = 248 bytes)
        let mut padding = [0u64; 31];
        for i in 0..31 {
            padding[i] = reader.read_u64()?;
        }

        Ok(PoolState {
            amm_config,
            pool_creator,
            token_0_vault,
            token_1_vault,
            lp_mint,
            token_0_mint,
            token_1_mint,
            token_0_program,
            token_1_program,
            observation_key,
            auth_bump,
            status,
            lp_mint_decimals,
            mint_0_decimals,
            mint_1_decimals,
            lp_supply,
            protocol_fees_token_0,
            protocol_fees_token_1,
            fund_fees_token_0,
            fund_fees_token_1,
            open_time,
            recent_epoch,
            padding,
        })
    }

    pub fn vault_amount_without_fee(&self, vault_0: u64, vault_1: u64) -> (u64, u64) {
        (
            vault_0
                .checked_sub(self.protocol_fees_token_0 + self.fund_fees_token_0)
                .unwrap_or(1),
            vault_1
                .checked_sub(self.protocol_fees_token_1 + self.fund_fees_token_1)
                .unwrap_or(1),
        )
    }
}

#[derive(Debug, Clone)]
pub struct PoolReserves {
    pub token_0_vault: Pubkey,
    pub token_0_amount: u64,
    pub token_1_vault: Pubkey,
    pub token_1_amount: u64,
}

#[derive(Default, Debug, Clone)]
pub struct AmmConfig {
    /// Bump to identify PDA
    pub bump: u8,
    /// Status to control if new pool can be create
    pub disable_create_pool: bool,
    /// Config index
    pub index: u16,
    /// The trade fee, denominated in hundredths of a bip (10^-6)
    pub trade_fee_rate: u64,
    /// The protocol fee
    pub protocol_fee_rate: u64,
    /// The fund fee, denominated in hundredths of a bip (10^-6)
    pub fund_fee_rate: u64,
    /// Fee for create a new pool
    pub create_pool_fee: u64,
    /// Address of the protocol fee owner
    pub protocol_owner: Pubkey,
    /// Address of the fund fee owner
    pub fund_owner: Pubkey,
    // pub padding: [u64; 16],
}

impl AmmConfig {
    pub fn deserialize(data: &[u8]) -> Result<Self> {
        let mut reader = ByteReader::new(data);

        // Skip the discriminator (first 8 bytes) - [247,237,227,245,215,195,222,70]
        reader.skip(8)?;
        let bump = reader.read_u8()?;
        let disable_create_pool = reader.read_u8()?;
        let index = reader.read_u16()?;
        let trade_fee_rate = reader.read_u64()?;
        let protocol_fee_rate = reader.read_u64()?;
        let fund_fee_rate = reader.read_u64()?;
        let create_pool_fee = reader.read_u64()?;
        let protocol_owner = reader.read_pubkey()?;
        let fund_owner = reader.read_pubkey()?;

        Ok(AmmConfig {
            bump,
            disable_create_pool: disable_create_pool == 0,
            index,
            trade_fee_rate,
            protocol_fee_rate,
            fund_fee_rate,
            create_pool_fee,
            protocol_owner,
            fund_owner,
            // padding,
        })
    }
}

#[derive(Debug, Clone)]
pub struct SwapOutput {
    pub amount_specified: u64,
    pub other_amount_threshold: u64,
    pub fund_fee: u128,
    pub trade_fee: u128,
    pub protocol_fee: u128,
}

pub fn swap_calculate(
    amm_config_state: &AmmConfig,
    pool_state: &PoolState,
    pool_reserves: &PoolReserves,
    amount_specified: u64,
    a_to_b: bool,
) -> Result<SwapOutput> {
    let (total_token_0_amount, total_token_1_amount) = pool_state
        .vault_amount_without_fee(pool_reserves.token_0_amount, pool_reserves.token_1_amount);

    let (total_input_token_amount, total_output_token_amount) = if a_to_b {
        (total_token_0_amount, total_token_1_amount)
    } else {
        (total_token_1_amount, total_token_0_amount)
    };

    // TODO: sub transfer fees
    let actual_amount_in = amount_specified;
    let result = curve::CurveCalculator::swap_base_input(
        u128::from(actual_amount_in),
        u128::from(total_input_token_amount),
        u128::from(total_output_token_amount),
        amm_config_state.trade_fee_rate,
        amm_config_state.protocol_fee_rate,
        amm_config_state.fund_fee_rate,
    )
    .ok_or(anyhow!("Zero Trading Token"))
    .unwrap();

    let amount_out = u64::try_from(result.destination_amount_swapped).unwrap();
    // TODO: calc transfer fee
    let transfer_fee = 0;
    let amount_received = amount_out.checked_sub(transfer_fee).unwrap();

    Ok(SwapOutput {
        amount_specified: amount_specified,
        other_amount_threshold: amount_received,
        fund_fee: result.fund_fee,
        trade_fee: result.trade_fee,
        protocol_fee: result.protocol_fee,
    })
}
