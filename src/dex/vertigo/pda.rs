use anchor_client::solana_sdk::pubkey::Pubkey;
use anyhow::Result;

pub fn derive_token_vault(pool_state: &Pubkey, token_mint: &Pubkey) -> Result<(Pubkey, u8)> {
    let (vault, bump) = Pubkey::find_program_address(
        &[pool_state.as_ref(), token_mint.as_ref()],
        &super::program_id(),
    );

    Ok((vault, bump))
}
