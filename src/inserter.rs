use crate::{
    global,
    pool_index::{TokenPool, add_pool},
    dex::raydium,
    streaming::{self, AccountDataType, AccountTypeInfo, global_data},
    util, dex::whirlpool, wsol_mint,
};
use anchor_client::solana_sdk::pubkey::Pubkey;
use anyhow::Result;
use std::str::FromStr;

pub const LAMPORTS_PER_SOL: u64 = 1_000_000_000;
const MIN_WSOL_LIQ: u64 = 10 * LAMPORTS_PER_SOL; // 10 SOL

#[inline]
fn mul_div_floor_u128(a: u128, num: u128, den: u128) -> Option<u128> {
    if den == 0 {
        return None;
    }
    Some(a.saturating_mul(num) / den)
}

#[inline]
fn f64_to_ratio(price: f64) -> Option<(u128, u128)> {
    if !price.is_finite() || price <= 0.0 {
        return None;
    }
    const SCALE: u128 = 1_000_000_000;
    let num = (price * SCALE as f64).floor() as u128;
    Some((num, SCALE))
}

async fn _check_whirlpool_liquidity(
    token_pool: &TokenPool,
    pool_data: &AccountDataType,
) -> Result<bool> {
    let wsol_mint = wsol_mint();
    if !(token_pool.mint_a == wsol_mint || token_pool.mint_b == wsol_mint) {
        return Ok(true);
    }

    if let AccountDataType::Whirlpool(data) = pool_data {
        let price_ab = data.get_price();
        if !price_ab.is_finite() || price_ab <= 0.0 {
            return Ok(false);
        }

        let (wsol_vault, other_vault, other_to_wsol_ratio): (Pubkey, Pubkey, (u128, u128)) =
            if token_pool.mint_a == wsol_mint {
                let (num, den) =
                    f64_to_ratio(price_ab).ok_or_else(|| anyhow::anyhow!("bad price"))?;
                // 1 B * (den/num) A
                (data.token_vault_a, data.token_vault_b, (den, num))
            } else {
                let (num, den) =
                    f64_to_ratio(price_ab).ok_or_else(|| anyhow::anyhow!("bad price"))?;
                (data.token_vault_b, data.token_vault_a, (num, den))
            };

        let rpc = global::get_rpc_client();
        let vault_pks = vec![wsol_vault, other_vault];
        let vaults = rpc.get_multiple_accounts(&vault_pks).await?;

        let (Some(wsol_acc), Some(other_acc)) = (
            &vaults.get(0).and_then(|x| x.as_ref()),
            &vaults.get(1).and_then(|x| x.as_ref()),
        ) else {
            return Ok(false);
        };

        let wsol_amount: u64 = util::parse_token_amount(&wsol_acc.data)?;
        if wsol_amount < MIN_WSOL_LIQ {
            return Ok(false);
        }

        let other_amount: u64 = util::parse_token_amount(&other_acc.data)?;
        let (num, den) = other_to_wsol_ratio;
        let other_as_wsol_u128 = mul_div_floor_u128(other_amount as u128, num, den).unwrap_or(0);
        let other_as_wsol =
            u64::try_from(other_as_wsol_u128.min(u128::from(u64::MAX))).unwrap_or(u64::MAX);

        if other_as_wsol < MIN_WSOL_LIQ {
            return Ok(false);
        }

        Ok(true)
    } else {
        Ok(true)
    }
}

pub async fn add(token_pool: TokenPool, pool_data: AccountDataType) -> Result<Vec<Pubkey>> {
    match &pool_data {
        AccountDataType::Whirlpool(_) => {
            let valid = _check_whirlpool_liquidity(&token_pool, &pool_data).await?;
            if !valid {
                return Ok(vec![]);
            }
        }
        _ => {}
    }

    if add_pool(token_pool.clone()) {
        return insert_pool_info(&token_pool, pool_data).await;
    }

    Ok(vec![])
}

async fn insert_pool_info(
    token_pool: &TokenPool,
    pool_data: AccountDataType,
) -> Result<Vec<Pubkey>> {
    let mut vec_keys: Vec<Pubkey> = vec![token_pool.pool, token_pool.mint_a, token_pool.mint_b];
    let account_data = pool_data.clone();
    let rpc_client = global::get_rpc_client();
    global_data::add_accounts_type(
        &[token_pool.mint_a, token_pool.mint_b],
        AccountTypeInfo::Account,
    );

    match pool_data {
        AccountDataType::AmmPair(pool_state) => {
            vec_keys.push(pool_state.pool_base_token_account);
            vec_keys.push(pool_state.pool_quote_token_account);
            global_data::add_accounts_type(
                &[
                    pool_state.pool_base_token_account,
                    pool_state.pool_quote_token_account,
                ],
                AccountTypeInfo::ReserveAccount,
            );
            global_data::add_accounts(token_pool.pool, account_data, AccountTypeInfo::AmmPair);
        }
        AccountDataType::DlmmPair(pool_state) => {
            let bin_array_pubkeys =
                streaming::loader::get_dlmm_bin_array_keys(token_pool.pool, &pool_state)?;
            global_data::add_accounts_type(&bin_array_pubkeys, AccountTypeInfo::BinArray);
            vec_keys.extend(bin_array_pubkeys);
            global_data::add_accounts(token_pool.pool, account_data, AccountTypeInfo::DlmmPair);
        }
        AccountDataType::Dammv2Pool(_) => {
            global_data::add_accounts(token_pool.pool, account_data, AccountTypeInfo::Dammv2Pool);
        }
        AccountDataType::RaydiumAmmPool(pool_state) => {
            vec_keys.extend(vec![
                pool_state.token_coin,
                pool_state.token_pc,
                pool_state.market,
            ]);
            global_data::add_accounts_type(
                &[pool_state.token_coin, pool_state.token_pc],
                AccountTypeInfo::ReserveAccount,
            );

            let raw_data = rpc_client.get_account_data(&pool_state.market).await?;
            if let Ok(data) = raydium::amm::serum::MarketState::deserialize(&raw_data) {
                global_data::add_accounts(
                    pool_state.market,
                    AccountDataType::RaydiumAmmMakertState(data),
                    AccountTypeInfo::RaydiumAmmMarketState,
                );
            }
            global_data::add_accounts(
                token_pool.pool,
                account_data,
                AccountTypeInfo::RaydiumAmmPool,
            );
        }
        AccountDataType::RaydiumCpmmPool(pool_state) => {
            vec_keys.extend(vec![
                pool_state.token_0_vault,
                pool_state.token_1_vault,
                pool_state.amm_config,
            ]);
            global_data::add_accounts_type(
                &[pool_state.token_0_vault, pool_state.token_1_vault],
                AccountTypeInfo::ReserveAccount,
            );

            let amm_config = rpc_client.get_account_data(&pool_state.amm_config).await?;
            if let Ok(data) = raydium::cpmm::AmmConfig::deserialize(&amm_config) {
                global_data::add_accounts(
                    pool_state.amm_config,
                    AccountDataType::RaydiumCpmmAmmConfig(data),
                    AccountTypeInfo::RaydiumCpmmAmmConfig,
                );
            }
            global_data::add_accounts(
                token_pool.pool,
                account_data,
                AccountTypeInfo::RaydiumCpmmPool,
            );
        }
        AccountDataType::RaydiumClmmPool(pool_state) => {
            let bitmap_ext =
                raydium::clmm::pda::derive_tick_array_bitmap_extension(&token_pool.pool)
                    .unwrap()
                    .0;

            let bitmap_state =
                raydium::clmm::util::fetch_bitmap_extension_state(rpc_client.clone(), &bitmap_ext)
                    .await?;
            let left_ticks = raydium::clmm::swap_util::get_cur_and_next_five_tick_array(
                token_pool.pool,
                &pool_state,
                &bitmap_state,
                false,
            );
            let right_ticks = raydium::clmm::swap_util::get_cur_and_next_five_tick_array(
                token_pool.pool,
                &pool_state,
                &bitmap_state,
                true,
            );
            let ticks = streaming::util::merge(&[&left_ticks, &right_ticks]);

            vec_keys.push(bitmap_ext);
            vec_keys.extend(&ticks);
            global_data::add_accounts_type(&ticks, AccountTypeInfo::RaydiumTickArrayState);
            global_data::add_accounts(
                bitmap_ext,
                AccountDataType::RaydiumTickArrayBitmapExt(bitmap_state),
                AccountTypeInfo::RaydiumTickArrayBitmapExt,
            );
            global_data::add_accounts(
                token_pool.pool,
                account_data,
                AccountTypeInfo::RaydiumClmmPool,
            );
        }
        AccountDataType::Whirlpool(pool_state) => {
            let oracle_address = whirlpool::state::pda::derive_oracle_address(&token_pool.pool)
                .unwrap()
                .0;
            let tick_data =
                whirlpool::util::get_tick_arrays_or_default(token_pool.pool, &pool_state).unwrap();
            vec_keys.push(oracle_address);
            global_data::add_accounts_type(&tick_data, AccountTypeInfo::WhirlpoolTickArray);
            vec_keys.extend(&tick_data);
            global_data::add_account_type(oracle_address, AccountTypeInfo::WhirlpoolOracle);
            global_data::add_accounts(token_pool.pool, account_data, AccountTypeInfo::Whirlpool);
        }
        AccountDataType::VertigoPool(_) => {
            global_data::add_accounts(token_pool.pool, account_data, AccountTypeInfo::VertigoPool);
        }
        AccountDataType::SolfiPool(pool_state) => {
            vec_keys.extend([pool_state.vault_a, pool_state.vault_b]);
            global_data::add_accounts_type(
                &[pool_state.vault_a, pool_state.vault_b],
                AccountTypeInfo::ReserveAccount,
            );
            global_data::add_accounts(token_pool.pool, account_data, AccountTypeInfo::SolfiPool);
        }
        _ => {}
    }

    Ok(vec_keys)
}
