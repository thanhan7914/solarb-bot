use crate::{
    arb::{PoolType, SwapRoutes},
    associated_token_program, global,
    onchain::get_associated_token_address,
    system_program,
};
use anchor_client::solana_sdk::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
};
use anyhow::Result;
use std::str::FromStr;

mod constants;
mod meteora;
mod pumpfun;
mod raydium;
mod solfi;
mod vertigo;
mod whirlpool;

use constants::*;
use meteora::*;
use pumpfun::*;
use raydium::*;
use solfi::*;
use vertigo::*;
use whirlpool::*;

pub fn program_id() -> Pubkey {
    Pubkey::from_str(PROGRAM_ID).unwrap()
}

pub fn route(swap: SwapRoutes, fee: u64) -> Result<Instruction> {
    let payer = global::get_pubkey();
    let user_base_account = get_associated_token_address(&payer, &swap.mint);
    let mut accounts: Vec<AccountMeta> = vec![
        AccountMeta::new(payer, true),
        AccountMeta::new(user_base_account, false),
        AccountMeta::new_readonly(system_program(), false),
        AccountMeta::new_readonly(associated_token_program(), false),
    ];

    let mut routes: Vec<u8> = Vec::with_capacity(swap.routes.len() * 2);
    let mut remaining_accounts: Vec<AccountMeta> = Vec::new();
    let mut current_account_in = user_base_account;

    for route in swap.routes {
        let (dex_id, route_accounts, token_out_account) = match route {
            PoolType::Pump(address, data) => {
                build_pump_accounts(&payer, address, &data, &current_account_in)
            }
            PoolType::Meteora(address, data) => {
                build_dlmm_accounts(&payer, address, &data, &current_account_in)
            }
            PoolType::MeteoraDammv2(address, data) => {
                build_damm_accounts(&payer, address, &data, &current_account_in)
            }
            PoolType::RaydiumAmm(address, data) => {
                build_amm_accounts(&payer, address, &data, &current_account_in)
            }
            PoolType::RaydiumCpmm(address, data) => {
                build_cpmm_accounts(&payer, address, &data, &current_account_in)
            }
            PoolType::RaydiumClmm(address, data) => {
                build_clmm_accounts(&payer, address, &data, &current_account_in)
            }
            PoolType::Whirlpool(address, data) => {
                build_whirlpool_accounts(&payer, address, &data, &current_account_in)
            }
            PoolType::Vertigo(address, data) => {
                build_vertigo_accounts(&payer, address, &data, &current_account_in)
            }
            PoolType::Solfi(address, data) => {
                build_solfi_accounts(&payer, address, &data, &current_account_in)
            }
        };

        // Add route metadata
        routes.push(dex_id);
        routes.push(route_accounts.len() as u8);
        remaining_accounts.extend(route_accounts);

        // Update input account for next route
        current_account_in = token_out_account;
    }

    accounts.extend(remaining_accounts);

    let amount_in: u64 = swap.amount_in as u64;
    let threshold: u64 = swap.threshold + 1_000_000;

    // Build instruction data
    let mut data = ROUTE_DISCRIMINATOR.to_vec();
    data.extend_from_slice(&(routes.len() as u32).to_le_bytes());
    data.extend_from_slice(&routes);
    data.extend_from_slice(&amount_in.to_le_bytes());
    data.extend_from_slice(&threshold.to_le_bytes());
    data.extend_from_slice(&fee.to_le_bytes());

    let instruction = Instruction {
        program_id: program_id(),
        accounts,
        data,
    };

    Ok(instruction)
}
