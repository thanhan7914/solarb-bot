use std::str::FromStr;

use super::*;
use crate::clock_mint;
use anchor_client::solana_sdk::clock::Clock;

pub fn get_clock() -> Option<Clock> {
    let account = ACCOUNT_DATA
        .get(&clock_mint())
        .map(|entry| entry.value().clone())?;

    if let AccountDataType::Clock(clock) = account {
        Some(clock)
    } else {
        None
    }
}

pub fn get_account(pubkey: &Pubkey) -> Option<AccountDataType> {
    ACCOUNT_DATA.get(pubkey).map(|entry| entry.value().clone())
}

pub fn get_mint_account(pubkey: &Pubkey) -> Option<Account> {
    MINT_DATA.get(pubkey).map(|entry| entry.value().clone())
}

pub fn get_account_type(pubkey: &Pubkey) -> AccountTypeInfo {
    AccountTypeInfo::from_pubkey(pubkey)
}

pub fn has_account_of_type(pubkey: &Pubkey, account_type: AccountTypeInfo) -> bool {
    AccountTypeInfo::from_pubkey(pubkey) == account_type
}

pub fn get_accounts_of_type(account_type: AccountTypeInfo) -> Vec<Pubkey> {
    ACCOUNT_TYPE_MAP
        .iter()
        .filter(|entry| *entry.value() == account_type)
        .map(|entry| entry.key().clone())
        .collect()
}

pub fn account_count_by_type() -> std::collections::HashMap<AccountTypeInfo, usize> {
    let mut counts = std::collections::HashMap::new();

    for entry in ACCOUNT_TYPE_MAP.iter() {
        let account_type = *entry.value();
        *counts.entry(account_type).or_insert(0) += 1;
    }

    counts
}

fn _safe_insert_type(pubkey: Pubkey, account_type: AccountTypeInfo) {
    ACCOUNT_TYPE_MAP.insert(pubkey, account_type);
    if !ACCOUNT_DATA.contains_key(&pubkey) {
        ACCOUNT_DATA.insert(pubkey, AccountDataType::Empty);
    }
}

#[inline]
pub fn add_accounts_type(accounts: &[Pubkey], account_type: AccountTypeInfo) {
    for account in accounts {
        // DashMap's insert is already optimized - just insert directly
        // insert() overwrites if exists, or use try_insert() to avoid overwrite
        _safe_insert_type(*account, account_type);
    }
}

#[inline]
pub fn add_accounts_type_str(accounts: &[String], account_type: AccountTypeInfo) {
    for account in accounts {
        // DashMap's insert is already optimized - just insert directly
        // insert() overwrites if exists, or use try_insert() to avoid overwrite
        let pk = Pubkey::from_str(&account).unwrap();
        _safe_insert_type(pk, account_type);
    }
}

#[inline]
pub fn add_account_type(key: Pubkey, account_type: AccountTypeInfo) {
    _safe_insert_type(key, account_type);
}

#[inline]
pub fn add_accounts(key: Pubkey, account: AccountDataType, account_type: AccountTypeInfo) {
    ACCOUNT_TYPE_MAP.insert(key, account_type);
    ACCOUNT_DATA.insert(key, account);
}

pub fn account_count() -> usize {
    ACCOUNT_DATA.len()
}

pub fn clear_all() {
    ACCOUNT_DATA.clear();
    ACCOUNT_TYPE_MAP.clear();
}

#[inline]
pub fn nonexists_pubkeys(pubkeys: &[Pubkey]) -> Vec<String> {
    let new_keys: Vec<String> = pubkeys
        .iter()
        .filter(|key| !ACCOUNT_DATA.contains_key(key))
        .map(|key| key.to_string())
        .collect();

    new_keys
}

#[inline]
pub fn update_price(pubkey: &Pubkey, from_mint: Pubkey, atob: f64) {
    PRICE_DATA.insert(*pubkey, (from_mint, atob));
}

#[inline]
pub fn get_price(pubkey: &Pubkey) -> Option<(Pubkey, f64)> {
    PRICE_DATA.get(pubkey).map(|entry| entry.value().clone())
}
