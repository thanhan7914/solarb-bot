use super::SOLFI_ID;
use crate::{arb::SolfiData, onchain::get_associated_token_address, dex::solfi, token_program};
use anchor_client::solana_sdk::{instruction::AccountMeta, pubkey::Pubkey, sysvar};

pub fn build_solfi_accounts(
    payer: &Pubkey,
    pool_address: Pubkey,
    data: &SolfiData,
    current_account_in: &Pubkey,
) -> (u8, Vec<AccountMeta>, Pubkey) {
    let token_x_account = get_associated_token_address(payer, &data.pool_state.mint_a);
    let token_y_account = get_associated_token_address(payer, &data.pool_state.mint_b);

    let accounts = vec![
        AccountMeta::new_readonly(solfi::program_id(), false),
        AccountMeta::new(pool_address, false),
        AccountMeta::new(data.pool_state.vault_a, false),
        AccountMeta::new(data.pool_state.vault_b, false),
        AccountMeta::new(token_x_account, false),
        AccountMeta::new(token_y_account, false),
        AccountMeta::new_readonly(token_program(), false),
        AccountMeta::new_readonly(sysvar::instructions::id(), false),
    ];

    let token_out_account = if current_account_in == &token_x_account {
        token_y_account
    } else {
        token_x_account
    };

    (SOLFI_ID, accounts, token_out_account)
}
