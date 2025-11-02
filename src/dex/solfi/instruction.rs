use super::Pool;
use crate::onchain::get_associated_token_address;
use anchor_client::solana_sdk::pubkey::Pubkey;
use anchor_client::solana_sdk::{
    instruction::{AccountMeta, Instruction},
    sysvar,
};

fn create_instruction_data(discriminator: u8, amount_in: u64, a_to_b: bool) -> Vec<u8> {
    let mut buffer = Vec::with_capacity(9);
    buffer.push(discriminator);
    buffer.extend_from_slice(&amount_in.to_le_bytes());
    buffer.resize(17, 0);
    buffer.push(if a_to_b { 0 } else { 1 });
    buffer
}

pub fn create_swap_ix(
    market: &Pubkey,
    user: &Pubkey,
    pool: &Pool,
    amount: u64,
    a_to_b: bool,
) -> Instruction {
    Instruction {
        program_id: crate::dex::solfi::program_id(),
        accounts: vec![
            AccountMeta::new(*user, true),
            AccountMeta::new(*market, false),
            AccountMeta::new(pool.vault_a, false),
            AccountMeta::new(pool.vault_b, false),
            AccountMeta::new(get_associated_token_address(user, &pool.mint_a), false),
            AccountMeta::new(get_associated_token_address(user, &pool.mint_b), false),
            AccountMeta::new_readonly(spl_token::id(), false),
            AccountMeta::new_readonly(sysvar::instructions::id(), false),
        ],
        data: create_instruction_data(7, amount, a_to_b),
    }
}
