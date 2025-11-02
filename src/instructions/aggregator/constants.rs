#[cfg(feature = "devnet")]
pub const PROGRAM_ID: &str = "EgR8hCtVKyTmsrdiegp113w1KfCYYxGnsFDADT2nozkk";

#[cfg(not(feature = "devnet"))]
pub const PROGRAM_ID: &str = "95HBHFnJWW3ZRo491Mnuhz4cx52N9nDSRzR6q5P5N7ZX";

pub const ROUTE_DISCRIMINATOR: [u8; 8] = [229, 23, 203, 151, 122, 227, 173, 42];
pub const PUMP_BUY_ID: u8 = 0;
pub const PUMP_SELL_ID: u8 = 1;
pub const METEORA_DLMM_ID: u8 = 2;
pub const METEORA_DAMM_ID: u8 = 3;
pub const RAYDIUM_AMM_ID: u8 = 4;
pub const RAYDIUM_CPMM_ID: u8 = 5;
pub const RAYDIUM_CLMM_ID: u8 = 6;
pub const WHIRLPOOL_ID: u8 = 7;
pub const VERTIGO_BUY_ID: u8 = 8;
pub const VERTIGO_SELL_ID: u8 = 9;
pub const SOLFI_ID: u8 = 10;
