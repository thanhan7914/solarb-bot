use super::ACCOUNT_TYPE_MAP;
use crate::dex::{meteora, pumpfun, raydium, solfi, vertigo, whirlpool};
use anchor_client::solana_sdk::{account::Account, clock::Clock};
use anchor_lang::prelude::Pubkey;
use dlmm_interface::{BinArray, LbPair};
use spl_token::state::Account as TokenAccount;

#[derive(Debug, Clone)]
pub enum AccountDataType {
    DlmmPair(LbPair),
    BinArray(BinArray),
    AmmPair(pumpfun::AmmPool),
    Account(Account),
    Clock(Clock),
    TokenAccount(TokenAccount),
    ReserveAccount(TokenAccount),
    Dammv2Pool(meteora::damm::Pool),
    RaydiumAmmPool(raydium::amm::AmmInfo),
    RaydiumAmmMakertState(raydium::amm::serum::MarketState),
    RaydiumCpmmPool(raydium::cpmm::PoolState),
    RaydiumCpmmAmmConfig(raydium::cpmm::AmmConfig),
    RaydiumClmmPool(raydium::clmm::PoolState),
    RaydiumTickArrayBitmapExt(raydium::clmm::tick_array_bitmap_extension::TickArrayBitmapExtension),
    RaydiumTickArrayState(raydium::clmm::tick_array::TickArrayState),
    SolfiPool(solfi::Pool),
    VertigoPool(vertigo::Pool),
    Whirlpool(whirlpool::state::Whirlpool),
    WhirlpoolOracle(whirlpool::state::oracle::Oracle),
    WhirlpoolTickArray(whirlpool::state::TickArray),
    Unknown(Vec<u8>),
    Empty,
}

impl AccountDataType {
    #[inline(always)]
    pub const fn to_label(&self) -> &'static str {
        match self {
            AccountDataType::DlmmPair(_) => "DlmmPair",
            AccountDataType::BinArray(_) => "BinArray",
            AccountDataType::AmmPair(_) => "AmmPair",
            AccountDataType::Account(_) => "Account",
            AccountDataType::Clock(_) => "Clock",
            AccountDataType::TokenAccount(_) => "TokenAccount",
            AccountDataType::ReserveAccount(_) => "ReserveAccount",
            AccountDataType::Dammv2Pool(_) => "Dammv2Pool",
            AccountDataType::RaydiumAmmPool(_) => "RaydiumAmmPool",
            AccountDataType::RaydiumAmmMakertState(_) => "RaydiumAmmMakertState",
            AccountDataType::RaydiumCpmmPool(_) => "RaydiumCpmmPool",
            AccountDataType::RaydiumCpmmAmmConfig(_) => "RaydiumCpmmAmmConfig",
            AccountDataType::RaydiumClmmPool(_) => "RaydiumClmmPool",
            AccountDataType::RaydiumTickArrayBitmapExt(_) => "RaydiumTickArrayBitmapExt",
            AccountDataType::RaydiumTickArrayState(_) => "RaydiumTickArrayState",
            AccountDataType::SolfiPool(_) => "SolfiPool",
            AccountDataType::VertigoPool(_) => "VertigoPool",
            AccountDataType::Whirlpool(_) => "Whirlpool",
            AccountDataType::WhirlpoolOracle(_) => "WhirlpoolOracle",
            AccountDataType::WhirlpoolTickArray(_) => "WhirlpoolTickArray",
            AccountDataType::Unknown(_) => "Unknown",
            AccountDataType::Empty => "Empty",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AccountTypeInfo {
    DlmmPair,
    BinArray,
    AmmPair,
    Account,
    Clock,
    TokenAccount,
    ReserveAccount,
    ProgramAccount,
    Dammv2Pool,
    RaydiumAmmPool,
    RaydiumAmmMarketState,
    RaydiumCpmmPool,
    RaydiumCpmmAmmConfig,
    RaydiumClmmPool,
    RaydiumTickArrayBitmapExt,
    RaydiumTickArrayState,
    SolfiPool,
    VertigoPool,
    Whirlpool,
    WhirlpoolOracle,
    WhirlpoolTickArray,
    Unknown,
}

impl AccountTypeInfo {
    #[inline(always)]
    pub fn from_pubkey(pubkey: &Pubkey) -> Self {
        ACCOUNT_TYPE_MAP
            .get(pubkey)
            .map(|entry| *entry.value())
            .unwrap_or(Self::Unknown)
    }
}

#[derive(Debug, Clone)]
pub enum WatcherCommand {
    AddAccount(String),
    RemoveAccount(String),
    AddHotAccount(String),
    BatchAdd {
        accounts: Vec<String>,
    },
    BatchRemove {
        accounts: Vec<String>,
    },
    BatchUpdate {
        add_accounts: Vec<String>,
        remove_accounts: Vec<String>,
    },
    DiscoverNew {
        accounts: Vec<String>,
    },
    RemoveOld {
        account: String,
    },
    EmergencyCleanup {
        accounts: Vec<String>,
    },
    GetMetrics,
    Stop,
}
