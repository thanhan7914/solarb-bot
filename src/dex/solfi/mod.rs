use crate::{byte_reader::ByteReader, onchain::get_associated_token_address};
use anchor_client::{
    solana_client::nonblocking::rpc_client::RpcClient, solana_sdk::pubkey::Pubkey,
};
use anyhow::Result;
use std::{str::FromStr, sync::Arc};

pub mod instruction;

const PROGRAM_ID: &str = "SoLFiHG9TfgtdUXUjWAxi3LtvYuFyDLVhBWxdMZxyCe";
pub const POOL_DISCRIMINATOR: [u8; 8] = [240, 0, 0, 0, 0, 0, 0, 0];

pub fn program_id() -> Pubkey {
    Pubkey::from_str(PROGRAM_ID).unwrap()
}

#[derive(Debug, Clone)]
pub struct Pool {
    pub market: Pubkey,
    pub mint_a: Pubkey,
    pub mint_b: Pubkey,
    pub vault_a: Pubkey,
    pub vault_b: Pubkey,
}

impl Pool {
    pub fn new(market: &Pubkey, mint_a: &Pubkey, mint_b: &Pubkey) -> Self {
        Pool {
            market: *market,
            mint_a: *mint_a,
            mint_b: *mint_b,
            vault_a: get_associated_token_address(market, &mint_a),
            vault_b: get_associated_token_address(market, &mint_b),
        }
    }

    pub fn deserialize(market: &Pubkey, data: &[u8]) -> Result<Self> {
        let mut reader = ByteReader::new(&data);

        reader.skip(2664)?;

        let mint_a = reader.read_pubkey()?;
        let mint_b = reader.read_pubkey()?;
        let vault_a = get_associated_token_address(market, &mint_a);
        let vault_b = get_associated_token_address(market, &mint_b);

        Ok(Self {
            market: *market,
            mint_a,
            mint_b,
            vault_a,
            vault_b,
        })
    }

    pub async fn fetch_vaults(&self, rpc_client: Arc<RpcClient>) -> Result<PoolReserves> {
        let accounts = rpc_client
            .get_multiple_accounts(&[self.vault_a, self.vault_b])
            .await?
            .into_iter()
            .collect::<Vec<_>>();

        let vault_a_data = &accounts[0].as_ref().unwrap().data;
        let vault_b_data = &accounts[1].as_ref().unwrap().data;

        Ok(PoolReserves {
            vault_a_amount: crate::util::parse_token_amount(&vault_a_data)?,
            vault_b_amount: crate::util::parse_token_amount(&vault_b_data)?,
            vault_a: self.vault_a,
            vault_b: self.vault_b,
        })
    }
}

#[derive(Debug, Clone)]
pub struct PoolReserves {
    pub vault_a_amount: u64,
    pub vault_b_amount: u64,
    pub vault_a: Pubkey,
    pub vault_b: Pubkey,
}

impl PoolReserves {
    pub fn swap_quote(&self, amount_in: u64, a_to_b: bool) -> u64 {
        if a_to_b {
            self.calculate_swap_a_to_b(amount_in)
        } else {
            self.calculate_swap_b_to_a(amount_in)
        }
    }

    pub fn calculate_swap_a_to_b(&self, amount_a_in: u64) -> u64 {
        if amount_a_in == 0 || self.vault_a_amount == 0 || self.vault_b_amount == 0 {
            return 0;
        }

        // Fee 0.3% (997/1000)
        let amount_a_in_with_fee = (amount_a_in as u128 * 997) / 1000;

        // Constant product: x * y = k
        // amount_out = (amount_in * reserve_out) / (reserve_in + amount_in)
        let numerator = amount_a_in_with_fee * self.vault_b_amount as u128;
        let denominator = self.vault_a_amount as u128 + amount_a_in_with_fee;

        if denominator == 0 {
            return 0;
        }

        (numerator / denominator) as u64
    }

    pub fn calculate_swap_b_to_a(&self, amount_b_in: u64) -> u64 {
        if amount_b_in == 0 || self.vault_a_amount == 0 || self.vault_b_amount == 0 {
            return 0;
        }

        // Fee 0.3% (997/1000)
        let amount_b_in_with_fee = (amount_b_in as u128 * 997) / 1000;

        let numerator = amount_b_in_with_fee * self.vault_a_amount as u128;
        let denominator = self.vault_b_amount as u128 + amount_b_in_with_fee;

        if denominator == 0 {
            return 0;
        }

        (numerator / denominator) as u64
    }

    pub fn get_price_a_in_b(&self) -> f64 {
        if self.vault_a_amount == 0 {
            return 0.0;
        }
        self.vault_b_amount as f64 / self.vault_a_amount as f64
    }

    pub fn get_price_b_in_a(&self) -> f64 {
        if self.vault_b_amount == 0 {
            return 0.0;
        }
        self.vault_a_amount as f64 / self.vault_b_amount as f64
    }

    pub fn calculate_amount_a_in_for_b_out(&self, amount_b_out: u64) -> u64 {
        if amount_b_out == 0 || amount_b_out >= self.vault_b_amount {
            return 0;
        }

        // amount_in = (reserve_in * amount_out) / ((reserve_out - amount_out) * 997/1000)
        let numerator = self.vault_a_amount as u128 * amount_b_out as u128 * 1000;
        let denominator = (self.vault_b_amount - amount_b_out) as u128 * 997;

        if denominator == 0 {
            return 0;
        }

        ((numerator / denominator) + 1) as u64 // +1 để round up
    }

    pub fn calculate_amount_b_in_for_a_out(&self, amount_a_out: u64) -> u64 {
        if amount_a_out == 0 || amount_a_out >= self.vault_a_amount {
            return 0;
        }

        let numerator = self.vault_b_amount as u128 * amount_a_out as u128 * 1000;
        let denominator = (self.vault_a_amount - amount_a_out) as u128 * 997;

        if denominator == 0 {
            return 0;
        }

        ((numerator / denominator) + 1) as u64
    }
}

pub async fn fetch_and_deserialize_pool(
    rpc_client: Arc<RpcClient>,
    pool_address: &Pubkey,
) -> Result<Pool> {
    let account = rpc_client.get_account(pool_address).await?;
    Pool::deserialize(&pool_address, &account.data)
}
