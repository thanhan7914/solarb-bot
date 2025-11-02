use super::*;

pub struct CLMMSeeds;

impl CLMMSeeds {
    pub const POOL: &'static [u8] = b"pool";
    pub const POOL_VAULT: &'static [u8] = b"pool_vault";
    pub const OBSERVATION: &'static [u8] = b"observation";
    pub const TICK_ARRAY: &'static [u8] = b"tick_array";
    pub const POSITION: &'static [u8] = b"position";
    pub const AMM_CONFIG: &'static [u8] = b"amm_config";
    pub const OPERATION: &'static [u8] = b"operation";
    pub const POOL_REWARD_VAULT: &'static [u8] = b"pool_reward_vault";
    pub const POOL_TICK_ARRAY_BITMAP_EXTENSION: &'static [u8] = b"pool_tick_array_bitmap_extension";
}

pub fn derive_tick_array_bitmap_extension(pool_id: &Pubkey) -> Result<(Pubkey, u8)> {
    let tickarray_bitmap_extension = Pubkey::find_program_address(
        &[
            CLMMSeeds::POOL_TICK_ARRAY_BITMAP_EXTENSION,
            pool_id.to_bytes().as_ref(),
        ],
        &super::program_id(),
    );

    Ok(tickarray_bitmap_extension)
}

pub fn derive_pool_state(
    amm_config: &Pubkey,
    token_mint_0: &Pubkey,
    token_mint_1: &Pubkey,
) -> Result<(Pubkey, u8)> {
    let (pool_pda, bump) = Pubkey::find_program_address(
        &[
            CLMMSeeds::POOL,
            amm_config.as_ref(),
            token_mint_0.as_ref(),
            token_mint_1.as_ref(),
        ],
        &super::program_id(),
    );

    Ok((pool_pda, bump))
}

pub fn derive_token_vault(pool_state: &Pubkey, token_mint: &Pubkey) -> Result<(Pubkey, u8)> {
    let (vault_pda, bump) = Pubkey::find_program_address(
        &[
            CLMMSeeds::POOL_VAULT,
            pool_state.as_ref(),
            token_mint.as_ref(),
        ],
        &super::program_id(),
    );

    Ok((vault_pda, bump))
}

pub fn derive_observation_state(pool_state: &Pubkey) -> Result<(Pubkey, u8)> {
    let (observation_pda, bump) = Pubkey::find_program_address(
        &[CLMMSeeds::OBSERVATION, pool_state.as_ref()],
        &super::program_id(),
    );

    Ok((observation_pda, bump))
}

pub fn derive_tick_array(pool_address: &Pubkey, index: i32) -> Result<(Pubkey, u8)> {
    let (tick_array_pk, bump) = Pubkey::find_program_address(
        &[
            CLMMSeeds::TICK_ARRAY,
            pool_address.to_bytes().as_ref(),
            &index.to_be_bytes(),
        ],
        &program_id(),
    );

    Ok((tick_array_pk, bump))
}
