use crate::{
    dex::{meteora, pumpfun, raydium, solfi, vertigo, whirlpool},
    streaming::AccountDataType,
};
use anchor_client::solana_sdk::account::Account;
use dlmm_interface::LbPairAccount;

pub fn get_pool_type(account: &Account) -> AccountDataType {
    if account.data.len() < 8 {
        return AccountDataType::Empty;
    }

    let data = &account.data;
    let owner = &account.owner;

    if *owner == meteora::dlmm::program_id() {
        if data[0..8] == meteora::dlmm::POOL_DISCRIMINATOR {
            if let Ok(data) = LbPairAccount::deserialize(data) {
                return AccountDataType::DlmmPair(data.0);
            }
        }
        return AccountDataType::Empty;
    }

    if *owner == meteora::damm::program_id() {
        if data[0..8] == meteora::damm::POOL_DISCRIMINATOR {
            if let Ok(data) = meteora::damm::Pool::deserialize(data) {
                return AccountDataType::Dammv2Pool(data);
            }
        }
        return AccountDataType::Empty;
    }

    if *owner == pumpfun::program_id() {
        if data[0..8] == pumpfun::POOL_DISCRIMINATOR {
            if let Ok(pool) = pumpfun::PumpAmmReader::parse_pool_data(&data[8..]) {
                return AccountDataType::AmmPair(pool);
            }
        }
        return AccountDataType::Empty;
    }

    if *owner == raydium::amm::program_id() {
        if data[0..8] == raydium::amm::POOL_DISCRIMINATOR {
            if let Ok(data) = raydium::amm::AmmInfo::deserialize(data) {
                return AccountDataType::RaydiumAmmPool(data);
            }
        }
        return AccountDataType::Empty;
    }

    if *owner == raydium::cpmm::program_id() {
        if data[0..8] == raydium::cpmm::POOL_DISCRIMINATOR {
            if let Ok(data) = raydium::cpmm::PoolState::deserialize(data) {
                return AccountDataType::RaydiumCpmmPool(data);
            }
        }
        return AccountDataType::Empty;
    }

    if *owner == raydium::clmm::program_id() {
        if data[0..8] == raydium::clmm::POOL_DISCRIMINATOR {
            if let Ok(data) = raydium::clmm::PoolState::deserialize(data) {
                return AccountDataType::RaydiumClmmPool(data);
            }
        }
        return AccountDataType::Empty;
    }

    if *owner == whirlpool::program_id() {
        if data[0..8] == whirlpool::POOL_DISCRIMINATOR {
            if let Ok(data) = whirlpool::state::Whirlpool::deserialize(data) {
                return AccountDataType::Whirlpool(data);
            }
        }
        return AccountDataType::Empty;
    }

    if *owner == vertigo::program_id() {
        if data[0..8] == vertigo::POOL_DISCRIMINATOR {
            if let Ok(data) = vertigo::Pool::deserialize(data) {
                return AccountDataType::VertigoPool(data);
            }
        }
        return AccountDataType::Empty;
    }

    if *owner == solfi::program_id() {
        if data[0..8] == solfi::POOL_DISCRIMINATOR {
            if let Ok(data) = solfi::Pool::deserialize(owner, data) {
                return AccountDataType::SolfiPool(data);
            }
        }
        return AccountDataType::Empty;
    }

    AccountDataType::Empty
}
