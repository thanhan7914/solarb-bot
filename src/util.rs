use anchor_client::solana_sdk::clock::Clock;
use anchor_client::{
    solana_client::nonblocking::rpc_client::RpcClient, solana_sdk::pubkey::Pubkey,
};
use anyhow::{Result, anyhow};
use rand::Rng;
use spl_token::solana_program::program_pack::Pack;
use spl_token::state::Account as TokenAccount;
use std::sync::Arc;

const AMOUNT_OFFSET: usize = 64;
const TEN_THOUSAND: u128 = 10000;

pub async fn get_clock(rpc_client: &RpcClient) -> Result<Clock> {
    let clock_account = rpc_client
        .get_account(&anchor_client::solana_sdk::sysvar::clock::ID)
        .await?;

    let clock_state: Clock = bincode::deserialize(clock_account.data.as_ref())?;

    Ok(clock_state)
}

pub async fn get_token_amount(rpc_client: Arc<RpcClient>, token_account: &Pubkey) -> Result<u64> {
    let data = rpc_client.get_account(&token_account).await?.data;
    parse_token_amount(&data)
}

pub fn parse_token_amount(data: &[u8]) -> Result<u64> {
    // Try normal deserialization first (SPL Token v1)
    match TokenAccount::unpack(&data.as_ref()) {
        Ok(token) => Ok(token.amount),
        Err(_) => {
            // Fallback: read amount manually from raw data (token-2022 or other custom format)
            if data.len() < AMOUNT_OFFSET + 8 {
                return Err(anyhow!("Invalid Account Data"));
            }

            let amount_bytes = &data[AMOUNT_OFFSET..AMOUNT_OFFSET + 8];
            let amount = u64::from_le_bytes(amount_bytes.try_into().unwrap());
            Ok(amount)
        }
    }
}

pub fn ternary_search<F>(mut l: u64, mut r: u64, eps: u64, f: F) -> u64
where
    F: Fn(u64) -> i64,
{
    while r - l > eps {
        let m1 = l + (r - l) / 3;
        let m2 = r - (r - l) / 3;

        if f(m1) < f(m2) {
            l = m1;
        } else {
            r = m2;
        }
    }

    let m = (l + r) / 2;
    let candidates = [l, m, r];

    *candidates.iter().max_by_key(|&&x| f(x)).unwrap()
}

pub fn pairwise<T: Clone>(data: &[T]) -> Vec<(T, T)> {
    let mut result = Vec::new();

    for i in 0..data.len() {
        for j in i + 1..data.len() {
            result.push((data[i].clone(), data[j].clone()));
        }
    }

    result
}

pub fn apply_slippage(amount_out: u128, slippage: f64) -> Result<u128> {
    let precision = 1_000_000_000u128; // For safe integer math
    // (1 - slippage/100) => e.g. slippage=1 => factor= 0.99
    let slippage_factor = ((1.0 - slippage / 100.0) * precision as f64) as u128;

    // min_quote = final_quote * (1 - slippage/100)
    let min_quote = amount_out
        .checked_mul(slippage_factor)
        .and_then(|x| x.checked_div(precision))
        .ok_or_else(|| anyhow!("Math overflow in min_quote calculation"))?;

    Ok(min_quote)
}

pub fn amount_with_slippage(amount: u64, slippage_bps: u64, up_towards: bool) -> Result<u64> {
    let amount = amount as u128;
    let slippage_bps = slippage_bps as u128;
    let amount_with_slippage = if up_towards {
        amount
            .checked_mul(slippage_bps.checked_add(TEN_THOUSAND).unwrap())
            .unwrap()
            .checked_div(TEN_THOUSAND)
            .unwrap()
    } else {
        amount
            .checked_mul(TEN_THOUSAND.checked_sub(slippage_bps).unwrap())
            .unwrap()
            .checked_div(TEN_THOUSAND)
            .unwrap()
    };
    u64::try_from(amount_with_slippage)
        .map_err(|_| anyhow!("failed to cast u128 -> u64 from {}", amount_with_slippage))
}

pub fn rand_u32(min: u32, max: u32) -> u32 {
    let mut rng = rand::thread_rng();
    rng.gen_range(min..=max)
}
