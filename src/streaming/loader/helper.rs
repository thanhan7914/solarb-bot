use super::*;
use anyhow::anyhow;
use spl_token::{solana_program::program_pack::Pack, state::Account as TokenAccount};

#[inline]
pub fn get_reserve_amount(pk: &Pubkey) -> u64 {
    global_data::get_account(&pk)
        .and_then(|data| {
            if let AccountDataType::ReserveAccount(reserve) = data {
                Some(reserve.amount)
            } else {
                None
            }
        })
        .unwrap_or(0)
}

#[inline]
pub fn get_account(pk: &Pubkey) -> Result<Account> {
    if let Some(AccountDataType::Account(account)) = global_data::get_account(&pk) {
        return Ok(account);
    }

    Err(anyhow!("Failed to load account"))
}

#[inline]
pub fn get_token_account(pk: &Pubkey) -> Result<TokenAccount> {
    if let Some(AccountDataType::TokenAccount(account)) = global_data::get_account(&pk) {
        return Ok(account);
    }

    Err(anyhow!("Failed to load token account"))
}

#[inline]
pub fn token_account_to_account(token_acc: &TokenAccount) -> Account {
    let mut data = vec![0u8; TokenAccount::LEN];
    TokenAccount::pack(token_acc.clone(), &mut data).unwrap();

    Account {
        lamports: 0,
        data,
        owner: token_acc.owner,
        executable: false,
        rent_epoch: 0,
    }
}
