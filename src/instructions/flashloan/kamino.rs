use crate::{onchain::get_associated_token_address, token_program, usdc_mint, wsol_mint};
use anchor_client::solana_sdk::pubkey::Pubkey;
use anchor_client::solana_sdk::{
    instruction::{AccountMeta, Instruction},
    sysvar,
};
use std::str::FromStr;

const PROGRAME_ID: &str = "KLend2g3cP87fffoy8q1mQqGKjrxjC8boSyAYavgmjD";

#[derive(Clone)]
pub struct KaminoReserve {
    pub lending_market: Pubkey,
    pub authority: Pubkey,
    pub reserve: Pubkey,
    pub reserve_supply: Pubkey,
    pub mint: Pubkey,
    pub reserve_liquidity_fee_receiver: Pubkey,
}

pub fn program_id() -> Pubkey {
    Pubkey::from_str(PROGRAME_ID).unwrap()
}

pub fn wsol_reserve() -> KaminoReserve {
    KaminoReserve {
        lending_market: Pubkey::from_str("H6rHXmXoCQvq8Ue81MqNh7ow5ysPa1dSozwW3PU1dDH6").unwrap(),
        authority: Pubkey::from_str("Dx8iy2o46sK1DzWbEcznqSKeLbLVeu7otkibA3WohGAj").unwrap(),
        reserve: Pubkey::from_str("6gTJfuPHEg6uRAijRkMqNc9kan4sVZejKMxmvx2grT1p").unwrap(),
        reserve_supply: Pubkey::from_str("ywaaLvG7t1vXJo8sT3UzE8yzzZtxLM7Fmev64Jbooye").unwrap(),
        mint: wsol_mint(),
        reserve_liquidity_fee_receiver: Pubkey::from_str(
            "EQ7hw63aBS7aPQqXsoxaaBxiwbEzaAiY9Js6tCekkqxf",
        )
        .unwrap(),
    }
}

pub fn usdc_reserve() -> KaminoReserve {
    KaminoReserve {
        lending_market: Pubkey::from_str("7u3HeHxYDLhnCoErrtycNokbQYbWGzLs6JSDqGAv5PfF").unwrap(),
        authority: Pubkey::from_str("9DrvZvyWh1HuAoZxvYWMvkf2XCzryCpGgHqrMjyDWpmo").unwrap(),
        reserve: Pubkey::from_str("D6q6wuQSrifJKZYpR1M8R4YawnLDtDsMmWM1NbBmgJ59").unwrap(),
        reserve_supply: Pubkey::from_str("Bgq7trRgVMeq33yt235zM2onQ4bRDBsY5EWiTetF4qw6").unwrap(),
        mint: usdc_mint(),
        reserve_liquidity_fee_receiver: Pubkey::from_str(
            "BbDUrk1bVtSixgQsPLBJFZEF7mwGstnD5joA1WzYvYFX",
        )
        .unwrap(),
    }
}

pub fn find_reserve(mint: &Pubkey) -> Option<KaminoReserve> {
    let wsol = wsol_mint();
    let usdc = usdc_mint();

    match mint {
        m if m == &wsol => Some(wsol_reserve()),
        m if m == &usdc => Some(usdc_reserve()),
        _ => None,
    }
}

pub fn flash_borrow_reserve_liquidity(
    payer: &Pubkey,
    kamino_reserve: KaminoReserve,
    borrow_amount: u64,
) -> Instruction {
    let user_destination_liquidity = get_associated_token_address(payer, &kamino_reserve.mint);

    let mut data = vec![0x87, 0xe7, 0x34, 0xa7, 0x07, 0x34, 0xd4, 0xc1];
    data.extend_from_slice(&borrow_amount.to_le_bytes());

    Instruction {
        program_id: program_id(),
        accounts: vec![
            AccountMeta::new_readonly(*payer, true),
            AccountMeta::new_readonly(kamino_reserve.authority, false),
            AccountMeta::new_readonly(kamino_reserve.lending_market, false),
            AccountMeta::new(kamino_reserve.reserve, false),
            AccountMeta::new_readonly(kamino_reserve.mint, false),
            AccountMeta::new(kamino_reserve.reserve_supply, false),
            AccountMeta::new(user_destination_liquidity, false),
            AccountMeta::new(kamino_reserve.reserve_liquidity_fee_receiver, false),
            AccountMeta::new_readonly(program_id(), false),
            AccountMeta::new_readonly(program_id(), false),
            AccountMeta::new_readonly(sysvar::instructions::id(), false),
            AccountMeta::new_readonly(token_program(), false),
        ],
        data,
    }
}

pub fn flash_repay_reserve_liquidity(
    payer: &Pubkey,
    kamino_reserve: KaminoReserve,
    borrow_amount: u64,
    borrow_index: u8,
) -> Instruction {
    let user_destination_liquidity = get_associated_token_address(payer, &kamino_reserve.mint);

    let mut data = vec![0xb9, 0x75, 0x00, 0xcb, 0x60, 0xf5, 0xb4, 0xba];
    data.extend_from_slice(&borrow_amount.to_le_bytes());
    data.extend_from_slice(&borrow_index.to_le_bytes());

    Instruction {
        program_id: program_id(),
        accounts: vec![
            AccountMeta::new_readonly(*payer, true),
            AccountMeta::new_readonly(kamino_reserve.authority, false),
            AccountMeta::new_readonly(kamino_reserve.lending_market, false),
            AccountMeta::new(kamino_reserve.reserve, false),
            AccountMeta::new_readonly(kamino_reserve.mint, false),
            AccountMeta::new(kamino_reserve.reserve_supply, false),
            AccountMeta::new(user_destination_liquidity, false),
            AccountMeta::new(kamino_reserve.reserve_liquidity_fee_receiver, false),
            AccountMeta::new_readonly(program_id(), false),
            AccountMeta::new_readonly(program_id(), false),
            AccountMeta::new_readonly(sysvar::instructions::id(), false),
            AccountMeta::new_readonly(token_program(), false),
        ],
        data,
    }
}
