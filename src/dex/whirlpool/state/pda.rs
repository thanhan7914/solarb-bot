use super::program_id;
use anchor_client::solana_sdk::pubkey::Pubkey;
use anyhow::Result;

pub mod seeds {
    pub const WHIRLPOOL: &[u8] = b"whirlpool";
    pub const TICK_ARRAY: &[u8] = b"tick_array";
    pub const POSITION: &[u8] = b"position";
    pub const ORACLE: &[u8] = b"oracle";
    pub const FEE_TIER: &[u8] = b"fee_tier";
    pub const TOKEN_BADGE: &[u8] = b"token_badge";
    pub const CONFIG_EXTENSION: &[u8] = b"config_extension";
    pub const POSITION_BUNDLE: &[u8] = b"position_bundle";
}

pub fn derive_whirlpool_address(
    whirlpools_config: &Pubkey,
    token_mint_a: &Pubkey,
    token_mint_b: &Pubkey,
    tick_spacing: u16,
) -> Result<(Pubkey, u8)> {
    let tick_spacing_bytes = tick_spacing.to_le_bytes();

    let (address, bump) = Pubkey::find_program_address(
        &[
            seeds::WHIRLPOOL,
            whirlpools_config.as_ref(),
            token_mint_a.as_ref(),
            token_mint_b.as_ref(),
            &tick_spacing_bytes,
        ],
        &super::program_id(),
    );

    Ok((address, bump))
}

pub fn derive_oracle_address(whirlpool: &Pubkey) -> Result<(Pubkey, u8)> {
    let (address, bump) =
        Pubkey::find_program_address(&[seeds::ORACLE, whirlpool.as_ref()], &super::program_id());

    Ok((address, bump))
}

pub fn derive_tick_array_address(
    whirlpool: &Pubkey,
    start_tick_index: i32,
) -> Result<(Pubkey, u8)> {
    let start_tick_bytes = start_tick_index.to_le_bytes();

    let (address, bump) = Pubkey::find_program_address(
        &[seeds::TICK_ARRAY, whirlpool.as_ref(), &start_tick_bytes],
        &super::program_id(),
    );

    Ok((address, bump))
}

pub fn derive_position_address(position_mint: &Pubkey) -> Result<(Pubkey, u8)> {
    let (address, bump) = Pubkey::find_program_address(
        &[seeds::POSITION, position_mint.as_ref()],
        &super::program_id(),
    );

    Ok((address, bump))
}

pub fn derive_fee_tier_address(
    whirlpools_config: &Pubkey,
    tick_spacing: u16,
) -> Result<(Pubkey, u8)> {
    let tick_spacing_bytes = tick_spacing.to_le_bytes();

    let (address, bump) = Pubkey::find_program_address(
        &[
            seeds::FEE_TIER,
            whirlpools_config.as_ref(),
            &tick_spacing_bytes,
        ],
        &super::program_id(),
    );

    Ok((address, bump))
}

pub fn derive_token_badge_address(
    whirlpools_config: &Pubkey,
    token_mint: &Pubkey,
) -> Result<(Pubkey, u8)> {
    let (address, bump) = Pubkey::find_program_address(
        &[
            seeds::TOKEN_BADGE,
            whirlpools_config.as_ref(),
            token_mint.as_ref(),
        ],
        &super::program_id(),
    );

    Ok((address, bump))
}

pub fn derive_config_extension_address(whirlpools_config: &Pubkey) -> Result<(Pubkey, u8)> {
    let (address, bump) = Pubkey::find_program_address(
        &[seeds::CONFIG_EXTENSION, whirlpools_config.as_ref()],
        &super::program_id(),
    );

    Ok((address, bump))
}

pub fn derive_position_bundle_address(position_bundle_mint: &Pubkey) -> Result<(Pubkey, u8)> {
    let (address, bump) = Pubkey::find_program_address(
        &[seeds::POSITION_BUNDLE, position_bundle_mint.as_ref()],
        &super::program_id(),
    );

    Ok((address, bump))
}

pub fn get_tick_array_address(whirlpool: &Pubkey, start_tick_index: i32) -> Result<(Pubkey, u8)> {
    let start_tick_index_str = start_tick_index.to_string();

    let (address, bump) = Pubkey::find_program_address(
        &[
            b"tick_array",
            whirlpool.as_ref(),
            start_tick_index_str.as_bytes(),
        ],
        &program_id(),
    );

    Ok((address, bump))
}
