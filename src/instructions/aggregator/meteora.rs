use super::{METEORA_DAMM_ID, METEORA_DLMM_ID};
use crate::{
    arb::{MeteoraDammv2Data, MeteoraDlmmData},
    instructions::util::bins_to_remaining_accounts,
    dex::meteora,
    onchain::get_associated_token_address,
    token_program,
};
use anchor_client::solana_sdk::{instruction::AccountMeta, pubkey::Pubkey};

pub fn build_dlmm_accounts(
    payer: &Pubkey,
    pool_address: Pubkey,
    data: &MeteoraDlmmData,
    current_account_in: &Pubkey,
) -> (u8, Vec<AccountMeta>, Pubkey) {
    let token_x_account = get_associated_token_address(payer, &data.lb_pair.token_x_mint);
    let token_y_account = get_associated_token_address(payer, &data.lb_pair.token_y_mint);

    let (token_in_account, token_out_account) = if current_account_in == &token_x_account {
        (token_x_account, token_y_account)
    } else {
        (token_y_account, token_x_account)
    };

    let mut accounts = vec![
        AccountMeta::new_readonly(meteora::dlmm::program_id(), false),
        AccountMeta::new_readonly(meteora::dlmm::program_id(), false),
        AccountMeta::new(pool_address, false),
        AccountMeta::new_readonly(meteora::dlmm::event_authority(), false),
        AccountMeta::new(data.lb_pair.oracle, false),
        AccountMeta::new(meteora::dlmm::program_id(), false),
        AccountMeta::new(data.lb_pair.reserve_x, false),
        AccountMeta::new(data.lb_pair.reserve_y, false),
        AccountMeta::new_readonly(data.lb_pair.token_x_mint, false),
        AccountMeta::new_readonly(data.lb_pair.token_y_mint, false),
        AccountMeta::new(token_in_account, false),
        AccountMeta::new(token_out_account, false),
        AccountMeta::new_readonly(token_program(), false),
        AccountMeta::new_readonly(token_program(), false),
    ];

    let remaining_accounts = bins_to_remaining_accounts(&data.bin_arrays, true);
    accounts.extend(remaining_accounts);

    (METEORA_DLMM_ID, accounts, token_out_account)
}

pub fn build_damm_accounts(
    payer: &Pubkey,
    pool_address: Pubkey,
    data: &MeteoraDammv2Data,
    current_account_in: &Pubkey,
) -> (u8, Vec<AccountMeta>, Pubkey) {
    let (pool_authority, _) = meteora::damm::DammV2PDA::get_pool_authority().unwrap();
    let (event_authority, _) = meteora::damm::DammV2PDA::get_event_authority().unwrap();
    let token_x_account = get_associated_token_address(payer, &data.pool_state.token_a_mint);
    let token_y_account = get_associated_token_address(payer, &data.pool_state.token_b_mint);

    let (token_in_account, token_out_account) = if current_account_in == &token_x_account {
        (token_x_account, token_y_account)
    } else {
        (token_y_account, token_x_account)
    };

    let accounts = vec![
        AccountMeta::new_readonly(meteora::damm::program_id(), false),
        AccountMeta::new_readonly(pool_authority, false),
        AccountMeta::new(pool_address, false),
        AccountMeta::new_readonly(event_authority, false),
        AccountMeta::new(meteora::damm::program_id(), false),
        AccountMeta::new(token_in_account, false),
        AccountMeta::new(token_out_account, false),
        AccountMeta::new(data.pool_state.token_a_vault, false),
        AccountMeta::new(data.pool_state.token_b_vault, false),
        AccountMeta::new_readonly(data.pool_state.token_a_mint, false),
        AccountMeta::new_readonly(data.pool_state.token_b_mint, false),
        AccountMeta::new_readonly(token_program(), false),
        AccountMeta::new_readonly(token_program(), false),
    ];

    (METEORA_DAMM_ID, accounts, token_out_account)
}
