use super::{VERTIGO_BUY_ID, VERTIGO_SELL_ID};
use crate::{
    arb::VertigoData, memo_program, onchain::get_associated_token_address, token_program, dex::vertigo,
};
use anchor_client::solana_sdk::{instruction::AccountMeta, pubkey::Pubkey};

pub fn build_vertigo_accounts(
    payer: &Pubkey,
    pool_address: Pubkey,
    data: &VertigoData,
    current_account_in: &Pubkey,
) -> (u8, Vec<AccountMeta>, Pubkey) {
    let token_x_account = get_associated_token_address(payer, &data.pool_state.mint_a);
    let token_y_account = get_associated_token_address(payer, &data.pool_state.mint_b);
    let (vault_x, _) =
        vertigo::pda::derive_token_vault(&pool_address, &data.pool_state.mint_a).unwrap();
    let (vault_y, _) =
        vertigo::pda::derive_token_vault(&pool_address, &data.pool_state.mint_b).unwrap();

    let accounts = vec![
        AccountMeta::new_readonly(vertigo::program_id(), false),
        AccountMeta::new_readonly(data.pool_state.owner, false),
        AccountMeta::new(pool_address, false),
        AccountMeta::new_readonly(data.pool_state.mint_a, false),
        AccountMeta::new_readonly(data.pool_state.mint_b, false),
        AccountMeta::new(token_x_account, false),
        AccountMeta::new(token_y_account, false),
        AccountMeta::new(vault_x, false),
        AccountMeta::new(vault_y, false),
        AccountMeta::new_readonly(token_program(), false),
        AccountMeta::new_readonly(token_program(), false),
        AccountMeta::new_readonly(memo_program(), false),
    ];

    let (dex_id, token_out_account) = if current_account_in == &token_x_account {
        (VERTIGO_BUY_ID, token_y_account)
    } else {
        (VERTIGO_SELL_ID, token_x_account)
    };

    (dex_id, accounts, token_out_account)
}
