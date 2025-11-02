use super::{AmmPool, PoolPDAs};
use anchor_client::solana_sdk::pubkey::Pubkey;
use anyhow::Result;

pub fn derive_event_authority() -> Result<(Pubkey, u8)> {
    let event_authority =
        Pubkey::find_program_address(&[b"__event_authority"], &super::program_id());
    Ok(event_authority)
}

pub fn derive_coin_creator_vault_authority(coin_creator: &Pubkey) -> Result<(Pubkey, u8)> {
    let cc_vault = Pubkey::find_program_address(
        &[b"creator_vault", coin_creator.as_ref()],
        &super::program_id(),
    );
    Ok(cc_vault)
}

pub fn derive_coin_creator_vault_ata(
    coin_creator_vault_authority: &Pubkey,
    quote_mint: &Pubkey,
) -> Result<(Pubkey, u8)> {
    let cc_vault_ata = Pubkey::find_program_address(
        &[
            coin_creator_vault_authority.as_ref(),
            crate::token_program().as_ref(),
            quote_mint.as_ref(),
        ],
        &crate::associated_token_program(),
    );
    Ok(cc_vault_ata)
}

pub fn derive_global_config() -> Result<(Pubkey, u8)> {
    let global_config = Pubkey::find_program_address(&[b"global_config"], &super::program_id());
    Ok(global_config)
}

pub fn derive_global_volume_accumulator() -> Result<(Pubkey, u8)> {
    let global_volume_accumulator =
        Pubkey::find_program_address(&[b"global_volume_accumulator"], &super::program_id());
    Ok(global_volume_accumulator)
}

pub fn derive_user_volume_accumulator(user: &Pubkey) -> Result<(Pubkey, u8)> {
    let user_volume_accumulator = Pubkey::find_program_address(
        &[b"user_volume_accumulator", user.as_ref()],
        &super::program_id(),
    );
    Ok(user_volume_accumulator)
}

pub fn derive_fee_config() -> Result<(Pubkey, u8)> {
    // Fee program address
    let fee_program = crate::fee_program();

    // Hardcoded seed from IDL
    let fee_config_seed = [
        12, 20, 222, 252, 130, 94, 198, 118, 148, 37, 8, 24, 187, 101, 64, 101, 244, 41, 141, 49,
        86, 213, 113, 180, 212, 248, 9, 12, 24, 233, 168, 99,
    ];

    let fee_config = Pubkey::find_program_address(&[b"fee_config", &fee_config_seed], &fee_program);
    Ok(fee_config)
}

pub fn derive_global_incentive_token_account(
    global_volume_accumulator: &Pubkey,
    mint: &Pubkey,
    token_program: &Pubkey,
) -> Result<(Pubkey, u8)> {
    let global_incentive_token_account = Pubkey::find_program_address(
        &[
            global_volume_accumulator.as_ref(),
            token_program.as_ref(),
            mint.as_ref(),
        ],
        &crate::associated_token_program(),
    );
    Ok(global_incentive_token_account)
}

pub fn derive_user_ata(
    user: &Pubkey,
    mint: &Pubkey,
    token_program: &Pubkey,
) -> Result<(Pubkey, u8)> {
    let user_ata = Pubkey::find_program_address(
        &[user.as_ref(), token_program.as_ref(), mint.as_ref()],
        &crate::associated_token_program(),
    );
    Ok(user_ata)
}

pub fn derive_protocol_fee_recipient_token_account(
    protocol_fee_recipient: &Pubkey,
    quote_mint: &Pubkey,
    quote_token_program: &Pubkey,
) -> Result<(Pubkey, u8)> {
    let protocol_fee_recipient_token_account = Pubkey::find_program_address(
        &[
            protocol_fee_recipient.as_ref(),
            quote_token_program.as_ref(),
            quote_mint.as_ref(),
        ],
        &crate::associated_token_program(),
    );
    Ok(protocol_fee_recipient_token_account)
}

pub fn derive_pdas(pool: &AmmPool, user: &Pubkey) -> Result<PoolPDAs> {
    let (event_authority, _) = derive_event_authority()?;
    let (coin_creator_vault_authority, _) =
        derive_coin_creator_vault_authority(&pool.coin_creator)?;
    let (coin_creator_vault_ata, _) =
        derive_coin_creator_vault_ata(&coin_creator_vault_authority, &pool.quote_mint)?;
    let (global_config, _) = derive_global_config()?;
    let (global_volume_accumulator, _) = derive_global_volume_accumulator()?;
    let (user_volume_accumulator, _) = derive_user_volume_accumulator(user)?;
    let (fee_config, _) = derive_fee_config()?;

    Ok(PoolPDAs {
        event_authority,
        coin_creator_vault_authority,
        coin_creator_vault_ata,
        global_config,
        global_volume_accumulator,
        user_volume_accumulator,
        fee_config,
    })
}
