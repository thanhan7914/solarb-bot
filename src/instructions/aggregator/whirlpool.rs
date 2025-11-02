use super::WHIRLPOOL_ID;
use crate::{arb::WhirlpoolData, onchain::get_associated_token_address, token_program, dex::whirlpool};
use anchor_client::solana_sdk::{instruction::AccountMeta, pubkey::Pubkey};

pub fn build_whirlpool_accounts(
    payer: &Pubkey,
    pool_address: Pubkey,
    data: &WhirlpoolData,
    current_account_in: &Pubkey,
) -> (u8, Vec<AccountMeta>, Pubkey) {
    let token_x_account = get_associated_token_address(payer, &data.pool_state.token_mint_a);
    let token_y_account = get_associated_token_address(payer, &data.pool_state.token_mint_b);
    let (oracle, _) = whirlpool::state::pda::derive_oracle_address(&pool_address).unwrap();

    let accounts = vec![
        AccountMeta::new_readonly(whirlpool::program_id(), false),
        AccountMeta::new(pool_address, false),
        AccountMeta::new_readonly(token_program(), false),
        AccountMeta::new(token_x_account, false),
        AccountMeta::new(data.pool_state.token_vault_a, false),
        AccountMeta::new(token_y_account, false),
        AccountMeta::new(data.pool_state.token_vault_b, false),
        AccountMeta::new(data.tick_data[0].0, false),
        AccountMeta::new(data.tick_data[1].0, false),
        AccountMeta::new(data.tick_data[2].0, false),
        AccountMeta::new(oracle, false),
    ];

    let token_out_account = if current_account_in == &token_x_account {
        token_y_account
    } else {
        token_x_account
    };

    (WHIRLPOOL_ID, accounts, token_out_account)
}
