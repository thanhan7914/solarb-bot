use crate::{
    global, dex::{meteora, pumpfun}, pool_index::{TokenPool, TokenPoolType}, streaming::AccountDataType
};
use anchor_client::solana_sdk::pubkey::Pubkey;

impl AccountDataType {
    pub fn to_mints(&self) -> Option<(Pubkey, Pubkey)> {
        match self {
            AccountDataType::DlmmPair(pool_state) => {
                Some((pool_state.token_x_mint, pool_state.token_y_mint))
            }
            AccountDataType::Dammv2Pool(pool_state) => {
                Some((pool_state.token_a_mint, pool_state.token_b_mint))
            }
            AccountDataType::AmmPair(pool_state) => {
                Some((pool_state.base_mint, pool_state.quote_mint))
            }
            AccountDataType::RaydiumAmmPool(pool_state) => {
                Some((pool_state.pc_mint, pool_state.coin_mint))
            }
            AccountDataType::RaydiumCpmmPool(pool_state) => {
                Some((pool_state.token_0_mint, pool_state.token_1_mint))
            }
            AccountDataType::RaydiumClmmPool(pool_state) => {
                Some((pool_state.token_mint_0, pool_state.token_mint_1))
            }
            AccountDataType::Whirlpool(pool_state) => {
                Some((pool_state.token_mint_a, pool_state.token_mint_b))
            }
            AccountDataType::VertigoPool(pool_state) => {
                Some((pool_state.mint_a, pool_state.mint_b))
            }
            AccountDataType::SolfiPool(pool_state) => Some((pool_state.mint_a, pool_state.mint_b)),
            _ => None,
        }
    }

    pub fn to_token_pool(&self, pool: Pubkey) -> Option<TokenPool> {
        match self {
            AccountDataType::DlmmPair(pool_state) => Some(TokenPool {
                pool_type: TokenPoolType::Dlmm,
                mint_a: pool_state.token_x_mint,
                mint_b: pool_state.token_y_mint,
                pool,
            }),
            AccountDataType::Dammv2Pool(pool_state) => Some(TokenPool {
                pool_type: TokenPoolType::Dammv2,
                mint_a: pool_state.token_a_mint,
                mint_b: pool_state.token_b_mint,
                pool,
            }),
            AccountDataType::AmmPair(pool_state) => Some(TokenPool {
                pool_type: TokenPoolType::PumpAmm,
                mint_a: pool_state.base_mint,
                mint_b: pool_state.quote_mint,
                pool,
            }),
            AccountDataType::RaydiumAmmPool(pool_state) => Some(TokenPool {
                pool_type: TokenPoolType::RaydiumAmm,
                mint_a: pool_state.pc_mint,
                mint_b: pool_state.coin_mint,
                pool,
            }),
            AccountDataType::RaydiumCpmmPool(pool_state) => Some(TokenPool {
                pool_type: TokenPoolType::RaydiumCpmm,
                mint_a: pool_state.token_0_mint,
                mint_b: pool_state.token_1_mint,
                pool,
            }),
            AccountDataType::RaydiumClmmPool(pool_state) => Some(TokenPool {
                pool_type: TokenPoolType::RaydiumClmm,
                mint_a: pool_state.token_mint_0,
                mint_b: pool_state.token_mint_1,
                pool,
            }),
            AccountDataType::Whirlpool(pool_state) => Some(TokenPool {
                pool_type: TokenPoolType::Whirlpool,
                mint_a: pool_state.token_mint_a,
                mint_b: pool_state.token_mint_b,
                pool,
            }),
            AccountDataType::VertigoPool(pool_state) => Some(TokenPool {
                pool_type: TokenPoolType::Vertigo,
                mint_a: pool_state.mint_a,
                mint_b: pool_state.mint_b,
                pool,
            }),
            AccountDataType::SolfiPool(pool_state) => Some(TokenPool {
                pool_type: TokenPoolType::Solfi,
                mint_a: pool_state.mint_a,
                mint_b: pool_state.mint_b,
                pool,
            }),
            _ => None,
        }
    }

    pub fn get_relevant_accounts(&self, pool: Pubkey) -> Vec<Pubkey> {
        match self {
            AccountDataType::DlmmPair(pool_state) => {
                vec![
                    pool,
                    meteora::dlmm::event_authority(),
                    pool_state.oracle,
                    pool_state.reserve_x,
                    pool_state.reserve_y,
                ]
            }
            AccountDataType::Dammv2Pool(pool_state) => {
                vec![
                    pool,
                    meteora::damm::DammV2PDA::get_pool_authority().unwrap().0,
                    meteora::damm::DammV2PDA::get_event_authority().unwrap().0,
                    pool_state.token_a_vault,
                    pool_state.token_b_vault,
                ]
            }
            AccountDataType::AmmPair(pool_state) => {
                let pdas = pumpfun::derive_pdas(&pool_state, &global::get_pubkey()).unwrap();
                vec![
                    pool,
                    pool_state.pool_base_token_account,
                    pool_state.pool_quote_token_account,
                    pdas.event_authority,
                    pdas.coin_creator_vault_ata,
                    pdas.coin_creator_vault_authority,
                ]
            }
            AccountDataType::RaydiumAmmPool(pool_state) => {
                vec![pool, pool_state.open_orders]
            }
            AccountDataType::RaydiumCpmmPool(_pool_state) => {
                vec![pool]
            }
            AccountDataType::RaydiumClmmPool(_pool_state) => {
                vec![pool]
            }
            AccountDataType::Whirlpool(_pool_state) => {
                vec![pool]
            }
            AccountDataType::VertigoPool(_pool_state) => {
                vec![pool]
            }
            AccountDataType::SolfiPool(_pool_state) => {
                vec![pool]
            }
            _ => vec![pool],
        }
    }
}
