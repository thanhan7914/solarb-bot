use crate::associated_token_program;
use anchor_client::solana_sdk::pubkey::Pubkey;
use std::str::FromStr;

const PROGRAM_ID: &str = "pAMMBay6oceH9fJKBRHGP5D4bD4sWpmSwMn52FMfXEA";
const GLOBAL_CONFIG: &str = "ADyA8hdefvWN2dbGGWFotbzWxrAvLW83WG6QCVXvJKqw";
pub const PUMP_FEE_WALLET: &str = "JCRGumoE9Qi5BBgULTgdgTLjSgkCMSbF62ZZfGs84JeU";
pub const POOL_DISCRIMINATOR: [u8; 8] = [241, 154, 109, 4, 17, 177, 109, 188];

#[cfg(feature = "devnet")]
const PROTOCOL_FEE: &str = "Freijj9xKLefjrb5fHgT6KMbYG1XBP2mA83tqeXYUMYM";

#[cfg(not(feature = "devnet"))]
const PROTOCOL_FEE: &str = "FWsW1xNtWscwNmKv6wVsU1iTzRN6wmmk3MjxRP5tT7hz";

pub fn fee_wallet() -> Pubkey {
    Pubkey::from_str(PUMP_FEE_WALLET).unwrap()
}

pub fn program_id() -> Pubkey {
    Pubkey::from_str(PROGRAM_ID).unwrap()
}

pub fn protocol_fee() -> Pubkey {
    Pubkey::from_str(PROTOCOL_FEE).unwrap()
}

pub fn protocol_fee_account(quote_token_program: &Pubkey, quote_mint: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[
            protocol_fee().as_ref(),
            quote_token_program.as_ref(),
            quote_mint.as_ref(),
        ],
        &associated_token_program(),
    )
}

pub fn global_config() -> Pubkey {
    Pubkey::from_str(GLOBAL_CONFIG).unwrap()
}

pub mod typedefs;
pub use typedefs::*;
pub mod reader;
pub use reader::*;
pub mod util;
pub use util::*;
pub mod quote;
pub use quote::*;
pub mod pda;
pub use pda::*;
