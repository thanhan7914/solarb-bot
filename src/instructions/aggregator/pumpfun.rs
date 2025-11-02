use super::{PUMP_BUY_ID, PUMP_SELL_ID};
use crate::{arb::PumpAmmData, fee_program, onchain::get_associated_token_address, dex::pumpfun, token_program};
use anchor_client::solana_sdk::{instruction::AccountMeta, pubkey::Pubkey};

pub fn build_pump_accounts(
    payer: &Pubkey,
    pool_address: Pubkey,
    data: &PumpAmmData,
    current_account_in: &Pubkey,
) -> (u8, Vec<AccountMeta>, Pubkey) {
    let pdas = pumpfun::derive_pdas(&data.pool, payer).unwrap();
    let user_base_account = get_associated_token_address(payer, &data.pool.base_mint);
    let user_quote_account = get_associated_token_address(payer, &data.pool.quote_mint);
    let (fee_account, _) = pumpfun::protocol_fee_account(&token_program(), &data.pool.quote_mint);

    let mut accounts = vec![
        AccountMeta::new_readonly(pumpfun::program_id(), false),
        AccountMeta::new_readonly(pumpfun::global_config(), false),
        AccountMeta::new_readonly(pool_address, false),
        AccountMeta::new_readonly(pdas.event_authority, false),
        AccountMeta::new_readonly(pumpfun::protocol_fee(), false),
        AccountMeta::new(fee_account, false),
        AccountMeta::new(pdas.coin_creator_vault_ata, false),
        AccountMeta::new_readonly(pdas.coin_creator_vault_authority, false),
        AccountMeta::new_readonly(data.pool.base_mint, false),
        AccountMeta::new_readonly(data.pool.quote_mint, false),
        AccountMeta::new(user_base_account, false),
        AccountMeta::new(user_quote_account, false),
        AccountMeta::new(data.pool.pool_base_token_account, false),
        AccountMeta::new(data.pool.pool_quote_token_account, false),
        AccountMeta::new_readonly(token_program(), false),
        AccountMeta::new_readonly(token_program(), false),
    ];

    let (dex_id, token_out_account, extend_accounts) = if current_account_in == &user_base_account {
        (PUMP_SELL_ID, user_quote_account, vec![
            AccountMeta::new(pdas.fee_config, false),
            AccountMeta::new_readonly(fee_program(), false),
        ])
    } else {
        (PUMP_BUY_ID, user_base_account, vec![
            AccountMeta::new(pdas.global_volume_accumulator, false),
            AccountMeta::new(pdas.user_volume_accumulator, false),
            AccountMeta::new(pdas.fee_config, false),
            AccountMeta::new_readonly(fee_program(), false),
        ])
    };

    accounts.extend(extend_accounts);

    (dex_id, accounts, token_out_account)
}
