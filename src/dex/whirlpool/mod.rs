use anchor_client::solana_sdk::pubkey::Pubkey;
use std::str::FromStr;

pub mod constants;
pub mod math;
pub mod quote;
pub mod state;
pub mod state_math;
pub mod types;
pub mod util;

pub use constants::*;
pub use state_math::*;

pub const PROGRAM_ID: &str = "whirLbMiicVdio4qvUfM5KAg6Ct8VwpYzGff3uctyCc";
pub const POOL_DISCRIMINATOR: [u8; 8] = [63, 149, 209, 12, 225, 128, 99, 9];

pub fn program_id() -> Pubkey {
    Pubkey::from_str(PROGRAM_ID).unwrap()
}
