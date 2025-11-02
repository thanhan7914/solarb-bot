use anchor_client::solana_sdk::pubkey::Pubkey;
use anyhow::Result;

/// Suffix for amm authority seed
const AUTHORITY_AMM: &'static [u8] = b"amm authority";
/// Suffix for amm associated seed
const AMM_ASSOCIATED_SEED: &'static [u8] = b"amm_associated_seed";
/// Suffix for target associated seed
const TARGET_ASSOCIATED_SEED: &'static [u8] = b"target_associated_seed";
/// Suffix for amm open order associated seed
const OPEN_ORDER_ASSOCIATED_SEED: &'static [u8] = b"open_order_associated_seed";
/// Suffix for coin vault associated seed
const COIN_VAULT_ASSOCIATED_SEED: &'static [u8] = b"coin_vault_associated_seed";
/// Suffix for pc vault associated seed
const PC_VAULT_ASSOCIATED_SEED: &'static [u8] = b"pc_vault_associated_seed";
/// Suffix for lp mint associated seed
const LP_MINT_ASSOCIATED_SEED: &'static [u8] = b"lp_mint_associated_seed";

#[derive(Clone, Copy, Debug)]
pub struct AmmKeys {
    pub amm_pool: Pubkey,
    pub amm_coin_mint: Pubkey,
    pub amm_pc_mint: Pubkey,
    pub amm_authority: Pubkey,
    pub amm_target: Pubkey,
    pub amm_coin_vault: Pubkey,
    pub amm_pc_vault: Pubkey,
    pub amm_lp_mint: Pubkey,
    pub amm_open_order: Pubkey,
    pub market: Pubkey,
    pub nonce: u8,
}

pub fn get_associated_address_and_bump_seed(
    info_id: &Pubkey,
    market_address: &Pubkey,
    associated_seed: &[u8],
    program_id: &Pubkey,
) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[
            &info_id.to_bytes(),
            &market_address.to_bytes(),
            &associated_seed,
        ],
        program_id,
    )
}

pub fn derive_amm_authority() -> Result<(Pubkey, u8)> {
    let (amm_authority, nonce) =
        Pubkey::find_program_address(&[AUTHORITY_AMM], &super::program_id());
    Ok((amm_authority, nonce))
}

pub fn get_amm_pda_keys(
    market: &Pubkey,
    coin_mint: &Pubkey,
    pc_mint: &Pubkey,
) -> Result<AmmKeys> {
    let amm_program = super::program_id();
    let amm_pool = get_associated_address_and_bump_seed(
        &amm_program,
        &market,
        AMM_ASSOCIATED_SEED,
        &amm_program,
    )
    .0;
    let (amm_authority, nonce) = Pubkey::find_program_address(&[AUTHORITY_AMM], &amm_program);
    let amm_open_order = get_associated_address_and_bump_seed(
        &amm_program,
        &market,
        OPEN_ORDER_ASSOCIATED_SEED,
        &amm_program,
    )
    .0;
    let amm_lp_mint = get_associated_address_and_bump_seed(
        &amm_program,
        &market,
        LP_MINT_ASSOCIATED_SEED,
        &amm_program,
    )
    .0;
    let amm_coin_vault = get_associated_address_and_bump_seed(
        &amm_program,
        &market,
        COIN_VAULT_ASSOCIATED_SEED,
        &amm_program,
    )
    .0;
    let amm_pc_vault = get_associated_address_and_bump_seed(
        &amm_program,
        &market,
        PC_VAULT_ASSOCIATED_SEED,
        &amm_program,
    )
    .0;
    let amm_target = get_associated_address_and_bump_seed(
        &amm_program,
        &market,
        TARGET_ASSOCIATED_SEED,
        &amm_program,
    )
    .0;

    Ok(AmmKeys {
        amm_pool,
        amm_target,
        amm_coin_vault,
        amm_pc_vault,
        amm_lp_mint,
        amm_open_order,
        amm_coin_mint: *coin_mint,
        amm_pc_mint: *pc_mint,
        amm_authority,
        market: *market,
        nonce,
    })
}
