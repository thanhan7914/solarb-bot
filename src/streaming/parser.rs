use super::{AccountDataType, AccountTypeInfo};
use crate::dex::{meteora, pumpfun::PumpAmmReader, raydium, solfi, vertigo, whirlpool};
use anchor_client::solana_sdk::{account::Account, clock::Clock, pubkey::Pubkey};
use dlmm_interface::{BinArrayAccount, LbPairAccount};
use spl_token::{solana_program::program_pack::Pack, state::Account as TokenAccount};

#[inline]
pub fn parse_account(pubkey: &Pubkey, account: &Account) -> Option<AccountDataType> {
    let account_type = AccountTypeInfo::from_pubkey(pubkey);
    let raw_data: &[u8] = &account.data;

    match account_type {
        AccountTypeInfo::DlmmPair => {
            if let Ok(data) = LbPairAccount::deserialize(raw_data) {
                return Some(AccountDataType::DlmmPair(data.0));
            }
        }
        AccountTypeInfo::BinArray => {
            if let Ok(data) = BinArrayAccount::deserialize(raw_data) {
                return Some(AccountDataType::BinArray(data.0));
            }
        }
        AccountTypeInfo::AmmPair => {
            if let Ok(pool) = PumpAmmReader::parse_pool_data(&raw_data[8..]) {
                return Some(AccountDataType::AmmPair(pool));
            }
        }
        AccountTypeInfo::Account => {
            return Some(AccountDataType::Account(account.clone()));
        }
        AccountTypeInfo::ReserveAccount => {
            if let Ok(token) = TokenAccount::unpack(raw_data) {
                return Some(AccountDataType::ReserveAccount(token));
            }
        }
        AccountTypeInfo::TokenAccount => {
            if let Ok(token) = TokenAccount::unpack(raw_data) {
                return Some(AccountDataType::TokenAccount(token));
            }
        }
        AccountTypeInfo::Clock => {
            if let Ok(clock) = bincode::deserialize::<Clock>(raw_data) {
                return Some(AccountDataType::Clock(clock));
            }
        }
        AccountTypeInfo::Dammv2Pool => {
            if let Ok(data) = meteora::damm::Pool::deserialize(raw_data) {
                return Some(AccountDataType::Dammv2Pool(data));
            }
        }
        AccountTypeInfo::RaydiumAmmPool => {
            if let Ok(data) = raydium::amm::AmmInfo::deserialize(raw_data) {
                return Some(AccountDataType::RaydiumAmmPool(data));
            }
        }
        AccountTypeInfo::RaydiumAmmMarketState => {
            if let Ok(data) = raydium::amm::serum::MarketState::deserialize(raw_data) {
                return Some(AccountDataType::RaydiumAmmMakertState(data));
            }
        }
        AccountTypeInfo::RaydiumCpmmPool => {
            if let Ok(data) = raydium::cpmm::PoolState::deserialize(raw_data) {
                return Some(AccountDataType::RaydiumCpmmPool(data));
            }
        }
        AccountTypeInfo::RaydiumCpmmAmmConfig => {
            if let Ok(data) = raydium::cpmm::AmmConfig::deserialize(raw_data) {
                return Some(AccountDataType::RaydiumCpmmAmmConfig(data));
            }
        }
        AccountTypeInfo::RaydiumClmmPool => {
            if let Ok(data) = raydium::clmm::PoolState::deserialize(raw_data) {
                return Some(AccountDataType::RaydiumClmmPool(data));
            }
        }
        AccountTypeInfo::RaydiumTickArrayBitmapExt => {
            if let Ok(data) =
                raydium::clmm::tick_array_bitmap_extension::TickArrayBitmapExtension::deserialize(
                    raw_data,
                )
            {
                return Some(AccountDataType::RaydiumTickArrayBitmapExt(data));
            }
        }
        AccountTypeInfo::RaydiumTickArrayState => {
            if let Ok(data) = raydium::clmm::tick_array::TickArrayState::deserialize(raw_data) {
                return Some(AccountDataType::RaydiumTickArrayState(data));
            }
        }
        AccountTypeInfo::SolfiPool => {
            if let Ok(data) = solfi::Pool::deserialize(pubkey, raw_data) {
                return Some(AccountDataType::SolfiPool(data));
            }
        }
        AccountTypeInfo::VertigoPool => {
            if let Ok(data) = vertigo::Pool::deserialize(raw_data) {
                return Some(AccountDataType::VertigoPool(data));
            }
        }
        AccountTypeInfo::Whirlpool => {
            if let Ok(data) = whirlpool::state::Whirlpool::deserialize(raw_data) {
                return Some(AccountDataType::Whirlpool(data));
            }
        }
        AccountTypeInfo::WhirlpoolOracle => {
            if let Ok(data) = whirlpool::state::oracle::Oracle::deserialize(raw_data) {
                return Some(AccountDataType::WhirlpoolOracle(data));
            }
        }
        AccountTypeInfo::WhirlpoolTickArray => {
            if let Ok(data) = whirlpool::state::TickArray::deserialize(raw_data) {
                return Some(AccountDataType::WhirlpoolTickArray(data));
            }
        }
        _ => {}
    }

    Some(AccountDataType::Unknown(raw_data.to_vec()))
}
