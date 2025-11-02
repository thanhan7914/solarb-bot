use anchor_client::solana_sdk::pubkey::Pubkey;
use anyhow::Result;

pub struct RaydiumSeeds;

impl RaydiumSeeds {
    /// Authority seed for vault and LP mint authority
    /// From IDL: "vault_and_lp_mint_auth_seed"
    pub const AUTHORITY: &'static [u8] = b"vault_and_lp_mint_auth_seed";

    /// AMM Config seed
    /// From IDL: "amm_config"
    pub const AMM_CONFIG: &'static [u8] = b"amm_config";

    /// Pool seed for pool state PDA
    pub const POOL: &'static [u8] = b"pool";

    /// Pool vault seed
    pub const POOL_VAULT: &'static [u8] = b"pool_vault";

    /// Pool LP mint seed
    pub const POOL_LP_MINT: &'static [u8] = b"pool_lp_mint";

    /// Observation seed for oracle data
    pub const OBSERVATION: &'static [u8] = b"observation";
}

pub fn derive_authority() -> Result<(Pubkey, u8)> {
    let (authority_pda, bump) =
        Pubkey::find_program_address(&[RaydiumSeeds::AUTHORITY], &super::program_id());

    Ok((authority_pda, bump))
}

pub fn derive_amm_config(index: u16) -> Result<(Pubkey, u8)> {
    let (config_pda, bump) = Pubkey::find_program_address(
        &[RaydiumSeeds::AMM_CONFIG, &index.to_le_bytes()],
        &super::program_id(),
    );

    Ok((config_pda, bump))
}

pub fn derive_pool_state_pda(
    amm_config: &Pubkey,
    token_0_mint: &Pubkey,
    token_1_mint: &Pubkey,
) -> Result<(Pubkey, u8)> {
    let (pool_pda, bump) = Pubkey::find_program_address(
        &[
            RaydiumSeeds::POOL,
            amm_config.as_ref(),
            token_0_mint.as_ref(),
            token_1_mint.as_ref(),
        ],
        &super::program_id(),
    );

    Ok((pool_pda, bump))
}

pub fn derive_token_vault(pool_state: &Pubkey, token_mint: &Pubkey) -> Result<(Pubkey, u8)> {
    let (vault_pda, bump) = Pubkey::find_program_address(
        &[
            RaydiumSeeds::POOL_VAULT,
            pool_state.as_ref(),
            token_mint.as_ref(),
        ],
        &super::program_id(),
    );

    Ok((vault_pda, bump))
}

pub fn derive_lp_mint(pool_state: &Pubkey) -> Result<(Pubkey, u8)> {
    let (lp_mint_pda, bump) = Pubkey::find_program_address(
        &[RaydiumSeeds::POOL_LP_MINT, pool_state.as_ref()],
        &super::program_id(),
    );

    Ok((lp_mint_pda, bump))
}

pub fn derive_observation_state(pool_state: &Pubkey) -> Result<(Pubkey, u8)> {
    let (observation_pda, bump) = Pubkey::find_program_address(
        &[RaydiumSeeds::OBSERVATION, pool_state.as_ref()],
        &super::program_id(),
    );

    Ok((observation_pda, bump))
}
