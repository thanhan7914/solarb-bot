use super::{RAYDIUM_AMM_ID, RAYDIUM_CLMM_ID, RAYDIUM_CPMM_ID};
use crate::{
    arb::{RaydiumAmmData, RaydiumClmmData, RaydiumCpmmData},
    memo_program,
    onchain::get_associated_token_address,
    dex::raydium, token_2022_program, token_program,
};
use anchor_client::solana_sdk::{instruction::AccountMeta, pubkey::Pubkey};

pub fn build_amm_accounts(
    payer: &Pubkey,
    pool_address: Pubkey,
    data: &RaydiumAmmData,
    current_account_in: &Pubkey,
) -> (u8, Vec<AccountMeta>, Pubkey) {
    let token_x_account = get_associated_token_address(payer, &data.pool_state.pc_mint);
    let token_y_account = get_associated_token_address(payer, &data.pool_state.coin_mint);
    let (amm_authority, _) = raydium::amm::derive_amm_authority().unwrap();
    let vault_signer = data
        .pool_state
        .derive_vault_signer(data.market_state.vault_signer_nonce)
        .unwrap();

    let (account_in, account_out) = if current_account_in == &token_x_account {
        (token_x_account, token_y_account)
    } else {
        (token_y_account, token_x_account)
    };

    let accounts = vec![
        AccountMeta::new_readonly(raydium::amm::program_id(), false),
        AccountMeta::new(amm_authority, false),
        AccountMeta::new(pool_address, false),
        AccountMeta::new(data.pool_state.open_orders, false),
        AccountMeta::new(data.pool_state.token_coin, false),
        AccountMeta::new(data.pool_state.token_pc, false),
        AccountMeta::new(raydium::amm::openbook_id(), false),
        AccountMeta::new(data.pool_state.market, false),
        AccountMeta::new(data.market_state.bids, false),
        AccountMeta::new(data.market_state.asks, false),
        AccountMeta::new(data.market_state.event_q, false),
        AccountMeta::new(data.market_state.coin_vault, false),
        AccountMeta::new(data.market_state.pc_vault, false),
        AccountMeta::new(vault_signer, false),
        AccountMeta::new(account_in, false),
        AccountMeta::new(account_out, false),
        AccountMeta::new_readonly(token_program(), false),
    ];

    (RAYDIUM_AMM_ID, accounts, account_out)
}

pub fn build_cpmm_accounts(
    payer: &Pubkey,
    pool_address: Pubkey,
    data: &RaydiumCpmmData,
    current_account_in: &Pubkey,
) -> (u8, Vec<AccountMeta>, Pubkey) {
    let (authority, _) = raydium::cpmm::pda::derive_authority().unwrap();
    let (observation_state, _) =
        raydium::cpmm::pda::derive_observation_state(&pool_address).unwrap();
    let token_x_account = get_associated_token_address(payer, &data.pool_state.token_0_mint);
    let token_y_account = get_associated_token_address(payer, &data.pool_state.token_1_mint);

    let (token_in_account, token_out_account, vault_in, vault_out, token_in, token_out) =
        if current_account_in == &token_x_account {
            (
                token_x_account,
                token_y_account,
                data.pool_state.token_0_vault,
                data.pool_state.token_1_vault,
                data.pool_state.token_0_mint,
                data.pool_state.token_1_mint,
            )
        } else {
            (
                token_y_account,
                token_x_account,
                data.pool_state.token_1_vault,
                data.pool_state.token_0_vault,
                data.pool_state.token_1_mint,
                data.pool_state.token_0_mint,
            )
        };

    let accounts = vec![
        AccountMeta::new_readonly(raydium::cpmm::program_id(), false),
        AccountMeta::new_readonly(data.pool_state.amm_config, false),
        AccountMeta::new(pool_address, false),
        AccountMeta::new_readonly(authority, false),
        AccountMeta::new(token_in_account, false),
        AccountMeta::new(token_out_account, false),
        AccountMeta::new(vault_in, false),
        AccountMeta::new(vault_out, false),
        AccountMeta::new_readonly(token_program(), false),
        AccountMeta::new_readonly(token_program(), false),
        AccountMeta::new_readonly(token_in, false),
        AccountMeta::new_readonly(token_out, false),
        AccountMeta::new(observation_state, false),
    ];

    (RAYDIUM_CPMM_ID, accounts, token_out_account)
}

pub fn build_clmm_accounts(
    payer: &Pubkey,
    pool_address: Pubkey,
    data: &RaydiumClmmData,
    current_account_in: &Pubkey,
) -> (u8, Vec<AccountMeta>, Pubkey) {
    // let (observation_state, _) =
    //     raydium::clmm::pda::derive_observation_state(&pool_address).unwrap();
    let (bitmap_ext, _) =
        raydium::clmm::pda::derive_tick_array_bitmap_extension(&pool_address).unwrap();
    let token_x_account = get_associated_token_address(payer, &data.pool_state.token_mint_0);
    let token_y_account = get_associated_token_address(payer, &data.pool_state.token_mint_1);
    let observation_state = data.pool_state.observation_key;

    let (a_to_b, token_in_account, token_out_account, vault_in, vault_out, token_in, token_out) =
        if current_account_in == &token_x_account {
            (
                true,
                token_x_account,
                token_y_account,
                data.pool_state.token_vault_0,
                data.pool_state.token_vault_1,
                data.pool_state.token_mint_0,
                data.pool_state.token_mint_1,
            )
        } else {
            (
                false,
                token_y_account,
                token_x_account,
                data.pool_state.token_vault_1,
                data.pool_state.token_vault_0,
                data.pool_state.token_mint_1,
                data.pool_state.token_mint_0,
            )
        };

    let mut accounts = vec![
        AccountMeta::new_readonly(raydium::clmm::program_id(), false),
        AccountMeta::new_readonly(data.pool_state.amm_config, false),
        AccountMeta::new(pool_address, false),
        AccountMeta::new(token_in_account, false),
        AccountMeta::new(token_out_account, false),
        AccountMeta::new(vault_in, false),
        AccountMeta::new(vault_out, false),
        AccountMeta::new(observation_state, false),
        AccountMeta::new_readonly(token_program(), false),
        AccountMeta::new_readonly(token_2022_program(), false),
        AccountMeta::new_readonly(memo_program(), false),
        AccountMeta::new_readonly(token_in, false),
        AccountMeta::new_readonly(token_out, false),
        AccountMeta::new(bitmap_ext, false),
    ];

    let ticks = if a_to_b {
        data.right_ticks.clone()
    } else {
        data.left_ticks.clone()
    };

    let remaining_accounts: Vec<AccountMeta> = ticks
        .into_iter()
        .map(|tick| {
            AccountMeta::new(
                raydium::clmm::pda::derive_tick_array(&pool_address, tick.start_tick_index)
                    .unwrap()
                    .0,
                false,
            )
        })
        .collect();

    accounts.extend(remaining_accounts);

    (RAYDIUM_CLMM_ID, accounts, token_out_account)
}
