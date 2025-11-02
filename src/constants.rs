use anchor_client::solana_sdk::pubkey::Pubkey;
use std::str::FromStr;

const WSOL: &str = "So11111111111111111111111111111111111111112";
const USDC: &str = "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v";
const SYSTEM_PROGRAM: &str = "11111111111111111111111111111111";
const ASSOCIATED_TOKEN_PROGRAM: &str = "ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL";
const TOKEN_PROGRAM: &str = "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA";
const TOKEN_2022_PROGRAM: &str = "TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb";
const MEMO_PROGRAM: &str = "MemoSq4gqABAXKb96qnH8TysNcWxMyWCqXgDLGmfcHr";
const CLOCK: &str = "SysvarC1ock11111111111111111111111111111111";
const FEE_PROGRAM: &str = "pfeeUxB6jkeY1Hxd7CsFCAjcbHA9rWtchMGdZ6VojVZ";
const LOOKUP_TABLE_ADDRESS: &str = "4sKLJ1Qoudh8PJyqBeuKocYdsZvxTcRShUt9aKqwhgvC";

pub fn default_lta() -> Pubkey {
    Pubkey::from_str(LOOKUP_TABLE_ADDRESS).unwrap()
}

pub fn wsol_mint() -> Pubkey {
    Pubkey::from_str(WSOL).unwrap()
}

pub fn usdc_mint() -> Pubkey {
    Pubkey::from_str(USDC).unwrap()
}

pub fn system_program() -> Pubkey {
    Pubkey::from_str(SYSTEM_PROGRAM).unwrap()
}

pub fn associated_token_program() -> Pubkey {
    Pubkey::from_str(ASSOCIATED_TOKEN_PROGRAM).unwrap()
}

pub fn token_program() -> Pubkey {
    Pubkey::from_str(TOKEN_PROGRAM).unwrap()
}

pub fn token_2022_program() -> Pubkey {
    Pubkey::from_str(TOKEN_2022_PROGRAM).unwrap()
}

pub fn memo_program() -> Pubkey {
    Pubkey::from_str(MEMO_PROGRAM).unwrap()
}

pub fn clock_mint() -> Pubkey {
    Pubkey::from_str(CLOCK).unwrap()
}

pub fn fee_program() -> Pubkey {
    Pubkey::from_str(FEE_PROGRAM).unwrap()
}
