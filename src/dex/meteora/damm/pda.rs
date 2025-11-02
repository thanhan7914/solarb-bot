use anchor_client::solana_sdk::pubkey::Pubkey;
use anyhow::Result;

pub struct DammV2PDA;

impl DammV2PDA {
    /// DammV2 Program ID
    pub const PROGRAM_ID: &'static str = "cpamdpZCGKUy5JxQXB4dcpGPiikHawvSWAd6mEn1sGG";

    /// Pool Authority seed từ IDL
    /// Seed: "pool_authority"
    pub const POOL_AUTHORITY_SEED: &'static [u8] = b"pool_authority";

    /// Event Authority seed từ IDL  
    /// Seed: "__event_authority" (được encode thành bytes trong IDL)
    pub const EVENT_AUTHORITY_SEED: &'static [u8] = b"__event_authority";

    /// Pool Authority derive from:
    /// - Seeds: ["pool_authority"]
    /// - Program: DammV2 Program ID
    pub fn get_pool_authority() -> Result<(Pubkey, u8)> {
        let program_id = Self::PROGRAM_ID
            .parse::<Pubkey>()
            .map_err(|e| anyhow::anyhow!("Invalid program ID: {}", e))?;

        let (pda, bump) = Pubkey::find_program_address(&[Self::POOL_AUTHORITY_SEED], &program_id);

        Ok((pda, bump))
    }

    /// Tính Event Authority PDA  
    /// Event Authority được derive từ:
    /// - Seeds: ["__event_authority"]
    /// - Program: DammV2 Program ID
    pub fn get_event_authority() -> Result<(Pubkey, u8)> {
        let program_id = Self::PROGRAM_ID
            .parse::<Pubkey>()
            .map_err(|e| anyhow::anyhow!("Invalid program ID: {}", e))?;

        let (pda, bump) = Pubkey::find_program_address(&[Self::EVENT_AUTHORITY_SEED], &program_id);

        Ok((pda, bump))
    }

    /// Tính Token Vault PDA cho pool
    /// Token Vault được derive từ:
    /// - Seeds: ["token_vault", token_mint, pool_address]
    /// - Program: DammV2 Program ID
    pub fn get_token_vault(token_mint: &Pubkey, pool_address: &Pubkey) -> Result<(Pubkey, u8)> {
        let program_id = Self::PROGRAM_ID
            .parse::<Pubkey>()
            .map_err(|e| anyhow::anyhow!("Invalid program ID: {}", e))?;

        let (pda, bump) = Pubkey::find_program_address(
            &[b"token_vault", token_mint.as_ref(), pool_address.as_ref()],
            &program_id,
        );

        Ok((pda, bump))
    }

    /// Tính Position PDA
    /// Position được derive từ:
    /// - Seeds: ["position", position_nft_mint]
    /// - Program: DammV2 Program ID
    pub fn get_position_pda(position_nft_mint: &Pubkey) -> Result<(Pubkey, u8)> {
        let program_id = Self::PROGRAM_ID
            .parse::<Pubkey>()
            .map_err(|e| anyhow::anyhow!("Invalid program ID: {}", e))?;

        let (pda, bump) =
            Pubkey::find_program_address(&[b"position", position_nft_mint.as_ref()], &program_id);

        Ok((pda, bump))
    }

    /// Tính Position NFT Account PDA
    /// Position NFT Account được derive từ:
    /// - Seeds: ["position_nft_account", position_nft_mint]
    /// - Program: DammV2 Program ID
    pub fn get_position_nft_account(position_nft_mint: &Pubkey) -> Result<(Pubkey, u8)> {
        let program_id = Self::PROGRAM_ID
            .parse::<Pubkey>()
            .map_err(|e| anyhow::anyhow!("Invalid program ID: {}", e))?;

        let (pda, bump) = Pubkey::find_program_address(
            &[b"position_nft_account", position_nft_mint.as_ref()],
            &program_id,
        );

        Ok((pda, bump))
    }

    /// Tính Config PDA
    /// Config được derive từ:
    /// - Seeds: ["config", index]
    /// - Program: DammV2 Program ID
    pub fn get_config_pda(index: u64) -> Result<(Pubkey, u8)> {
        let program_id = Self::PROGRAM_ID
            .parse::<Pubkey>()
            .map_err(|e| anyhow::anyhow!("Invalid program ID: {}", e))?;

        let (pda, bump) =
            Pubkey::find_program_address(&[b"config", &index.to_le_bytes()], &program_id);

        Ok((pda, bump))
    }

    /// Tính Reward Vault PDA
    /// Reward Vault được derive từ:
    /// - Seeds: ["reward_vault", pool_address, reward_index]
    /// - Program: DammV2 Program ID
    pub fn get_reward_vault(pool_address: &Pubkey, reward_index: u8) -> Result<(Pubkey, u8)> {
        let program_id = Self::PROGRAM_ID
            .parse::<Pubkey>()
            .map_err(|e| anyhow::anyhow!("Invalid program ID: {}", e))?;

        let (pda, bump) = Pubkey::find_program_address(
            &[b"reward_vault", pool_address.as_ref(), &[reward_index]],
            &program_id,
        );

        Ok((pda, bump))
    }

    /// Tính Token Badge PDA
    /// Token Badge được derive từ:
    /// - Seeds: ["token_badge", token_mint]
    /// - Program: DammV2 Program ID
    pub fn get_token_badge(token_mint: &Pubkey) -> Result<(Pubkey, u8)> {
        let program_id = Self::PROGRAM_ID
            .parse::<Pubkey>()
            .map_err(|e| anyhow::anyhow!("Invalid program ID: {}", e))?;

        let (pda, bump) =
            Pubkey::find_program_address(&[b"token_badge", token_mint.as_ref()], &program_id);

        Ok((pda, bump))
    }

    /// Tính Claim Fee Operator PDA
    /// Claim Fee Operator được derive từ:
    /// - Seeds: ["cf_operator", operator_pubkey]
    /// - Program: DammV2 Program ID
    pub fn get_claim_fee_operator(operator: &Pubkey) -> Result<(Pubkey, u8)> {
        let program_id = Self::PROGRAM_ID
            .parse::<Pubkey>()
            .map_err(|e| anyhow::anyhow!("Invalid program ID: {}", e))?;

        let (pda, bump) =
            Pubkey::find_program_address(&[b"cf_operator", operator.as_ref()], &program_id);

        Ok((pda, bump))
    }

    /// Helper để decode seed bytes từ IDL
    /// Event Authority seed trong IDL là: [95,95,101,118,101,110,116,95,97,117,116,104,111,114,105,116,121]
    /// Pool Authority seed trong IDL là: [112,111,111,108,95,97,117,116,104,111,114,105,116,121]
    pub fn decode_seed_from_idl_bytes(bytes: &[u8]) -> String {
        String::from_utf8_lossy(bytes).to_string()
    }
}
