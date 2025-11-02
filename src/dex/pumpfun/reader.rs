use super::typedefs::{AmmPool, BondingCurve, GlobalConfig, PoolReserves};
use anchor_client::solana_client::nonblocking::rpc_client::RpcClient;
use anchor_client::solana_sdk::pubkey::Pubkey;
use anyhow::{Ok, Result};
use std::{str::FromStr, sync::Arc};
use tokio::join;

// Discriminators
const POOL_DISCRIMINATOR: [u8; 8] = [241, 154, 109, 4, 17, 177, 109, 188];
const GLOBAL_CONFIG_DISCRIMINATOR: [u8; 8] = [149, 8, 156, 202, 160, 252, 176, 217];
const BONDING_CURVE_DISCRIMINATOR: [u8; 8] = [23, 183, 248, 55, 96, 216, 172, 96];

pub struct PumpAmmReader {
    program_id: Pubkey,
    rpc_client: Arc<RpcClient>,
}

impl PumpAmmReader {
    pub fn new(rpc_url: &str) -> Result<Self> {
        let program_id = super::program_id();
        let rpc_client = Arc::new(RpcClient::new(rpc_url.to_string()));

        Ok(Self {
            program_id,
            rpc_client,
        })
    }

    pub fn new_with_client(rpc_client: Arc<RpcClient>) -> Result<Self> {
        let program_id = super::program_id();
        Ok(Self {
            program_id,
            rpc_client,
        })
    }

    pub async fn read_pool(&self, pool_address: &str) -> Result<AmmPool> {
        let pool_pubkey = Pubkey::from_str(pool_address)?;
        let account = self.rpc_client.get_account(&pool_pubkey).await?;
        // Verify discriminator
        if account.data.len() < 8 {
            return Err(anyhow::anyhow!(
                "Account data too short: {} bytes",
                account.data.len()
            ));
        }

        let discriminator = &account.data[0..8];

        if discriminator != POOL_DISCRIMINATOR {
            return Err(anyhow::anyhow!(
                "Invalid Pool discriminator: expected {:?}, got {:?}",
                POOL_DISCRIMINATOR,
                discriminator
            ));
        }

        let pool = PumpAmmReader::parse_pool_data(&account.data[8..])?;

        Ok(pool)
    }

    pub fn parse_pool_data(data: &[u8]) -> Result<AmmPool> {
        // Pool struct by IDL:
        // pool_bump: u8 (1 byte)
        // index: u16 (2 bytes)
        // creator: Pubkey (32 bytes)
        // base_mint: Pubkey (32 bytes)
        // quote_mint: Pubkey (32 bytes)
        // lp_mint: Pubkey (32 bytes)
        // pool_base_token_account: Pubkey (32 bytes)
        // pool_quote_token_account: Pubkey (32 bytes)
        // lp_supply: u64 (8 bytes)
        // coin_creator: Pubkey (32 bytes)
        // Total: 1 + 2 + 32*7 + 8 = 235 bytes minimum

        if data.len() < 235 {
            return Err(anyhow::anyhow!(
                "Pool data too short: {} bytes, expected at least 235",
                data.len()
            ));
        }

        let mut offset = 0;

        // pool_bump: u8
        let pool_bump = data[offset];
        offset += 1;

        // index: u16
        let index = u16::from_le_bytes([data[offset], data[offset + 1]]);
        offset += 2;

        // creator: Pubkey (32 bytes)
        let creator =
            Pubkey::new_from_array(data[offset..offset + 32].try_into().map_err(|_| {
                anyhow::anyhow!("Failed to parse creator pubkey at offset {}", offset)
            })?);
        offset += 32;

        // base_mint: Pubkey (32 bytes)
        let base_mint =
            Pubkey::new_from_array(data[offset..offset + 32].try_into().map_err(|_| {
                anyhow::anyhow!("Failed to parse base_mint pubkey at offset {}", offset)
            })?);
        offset += 32;

        // quote_mint: Pubkey (32 bytes)
        let quote_mint =
            Pubkey::new_from_array(data[offset..offset + 32].try_into().map_err(|_| {
                anyhow::anyhow!("Failed to parse quote_mint pubkey at offset {}", offset)
            })?);
        offset += 32;

        // lp_mint: Pubkey (32 bytes)
        let lp_mint =
            Pubkey::new_from_array(data[offset..offset + 32].try_into().map_err(|_| {
                anyhow::anyhow!("Failed to parse lp_mint pubkey at offset {}", offset)
            })?);
        offset += 32;

        // pool_base_token_account: Pubkey (32 bytes)
        let pool_base_token_account =
            Pubkey::new_from_array(data[offset..offset + 32].try_into().map_err(|_| {
                anyhow::anyhow!(
                    "Failed to parse pool_base_token_account at offset {}",
                    offset
                )
            })?);
        offset += 32;

        // pool_quote_token_account: Pubkey (32 bytes)
        let pool_quote_token_account =
            Pubkey::new_from_array(data[offset..offset + 32].try_into().map_err(|_| {
                anyhow::anyhow!(
                    "Failed to parse pool_quote_token_account at offset {}",
                    offset
                )
            })?);
        offset += 32;

        // lp_supply: u64 (8 bytes)
        let lp_supply = u64::from_le_bytes(
            data[offset..offset + 8]
                .try_into()
                .map_err(|_| anyhow::anyhow!("Failed to parse lp_supply at offset {}", offset))?,
        );
        offset += 8;

        // coin_creator: Pubkey (32 bytes)
        let coin_creator =
            Pubkey::new_from_array(data[offset..offset + 32].try_into().map_err(|_| {
                anyhow::anyhow!("Failed to parse coin_creator at offset {}", offset)
            })?);

        Ok(AmmPool {
            pool_bump,
            index,
            creator,
            base_mint,
            quote_mint,
            lp_mint,
            pool_base_token_account,
            pool_quote_token_account,
            lp_supply,
            coin_creator,
        })
    }

    pub async fn read_global_config(&self) -> Result<GlobalConfig> {
        // Derive GlobalConfig PDA by IDL
        let (global_config_pubkey, _) =
            Pubkey::find_program_address(&[b"global_config"], &self.program_id);

        let account = self.rpc_client.get_account(&global_config_pubkey).await?;

        // Verify discriminator
        if account.data.len() < 8 {
            return Err(anyhow::anyhow!("GlobalConfig data too short"));
        }

        let discriminator = &account.data[0..8];
        if discriminator != GLOBAL_CONFIG_DISCRIMINATOR {
            return Err(anyhow::anyhow!("Invalid GlobalConfig discriminator"));
        }

        let config = self.parse_global_config_data(&account.data[8..])?;

        Ok(config)
    }

    fn parse_global_config_data(&self, data: &[u8]) -> Result<GlobalConfig> {
        // GlobalConfig struct:
        // admin: Pubkey (32 bytes)
        // lp_fee_basis_points: u64 (8 bytes)
        // protocol_fee_basis_points: u64 (8 bytes)
        // disable_flags: u8 (1 byte)
        // protocol_fee_recipients: [Pubkey; 8] (8 * 32 = 256 bytes)
        // coin_creator_fee_basis_points: u64 (8 bytes)
        // Total: 32 + 8 + 8 + 1 + 256 + 8 = 313 bytes

        if data.len() < 313 {
            return Err(anyhow::anyhow!(
                "GlobalConfig data too short: {} bytes, expected 313",
                data.len()
            ));
        }

        let mut offset = 0;

        // admin: Pubkey (32 bytes)
        let admin = Pubkey::new_from_array(data[offset..offset + 32].try_into().unwrap());
        offset += 32;

        // lp_fee_basis_points: u64 (8 bytes)
        let lp_fee_basis_points = u64::from_le_bytes(data[offset..offset + 8].try_into().unwrap());
        offset += 8;

        // protocol_fee_basis_points: u64 (8 bytes)
        let protocol_fee_basis_points =
            u64::from_le_bytes(data[offset..offset + 8].try_into().unwrap());
        offset += 8;

        // disable_flags: u8 (1 byte)
        let disable_flags = data[offset];
        offset += 1;

        // protocol_fee_recipients: [Pubkey; 8] (8 * 32 = 256 bytes)
        let mut protocol_fee_recipients = [Pubkey::default(); 8];
        for i in 0..8 {
            protocol_fee_recipients[i] =
                Pubkey::new_from_array(data[offset..offset + 32].try_into().unwrap());
            offset += 32;
        }

        // coin_creator_fee_basis_points: u64 (8 bytes)
        let coin_creator_fee_basis_points =
            u64::from_le_bytes(data[offset..offset + 8].try_into().unwrap());

        Ok(GlobalConfig {
            admin,
            lp_fee_basis_points,
            protocol_fee_basis_points,
            disable_flags,
            protocol_fee_recipients,
            coin_creator_fee_basis_points,
        })
    }

    pub async fn read_bonding_curve(&self, bonding_curve_address: &str) -> Result<BondingCurve> {
        let bonding_curve_pubkey = Pubkey::from_str(bonding_curve_address)?;
        let account = self.rpc_client.get_account(&bonding_curve_pubkey).await?;

        // Verify discriminator
        if account.data.len() < 8 {
            return Err(anyhow::anyhow!("BondingCurve data too short"));
        }

        let discriminator = &account.data[0..8];
        if discriminator != BONDING_CURVE_DISCRIMINATOR {
            return Err(anyhow::anyhow!("Invalid BondingCurve discriminator"));
        }

        let bonding_curve = self.parse_bonding_curve_data(&account.data[8..])?;

        Ok(bonding_curve)
    }

    fn parse_bonding_curve_data(&self, data: &[u8]) -> Result<BondingCurve> {
        // BondingCurve struct:
        // virtual_token_reserves: u64 (8 bytes)
        // virtual_sol_reserves: u64 (8 bytes)
        // real_token_reserves: u64 (8 bytes)
        // real_sol_reserves: u64 (8 bytes)
        // token_total_supply: u64 (8 bytes)
        // complete: bool (1 byte)
        // creator: Pubkey (32 bytes)
        // Total: 5*8 + 1 + 32 = 73 bytes

        if data.len() < 73 {
            return Err(anyhow::anyhow!("BondingCurve data too short"));
        }

        let mut offset = 0;

        let virtual_token_reserves =
            u64::from_le_bytes(data[offset..offset + 8].try_into().unwrap());
        offset += 8;

        let virtual_sol_reserves = u64::from_le_bytes(data[offset..offset + 8].try_into().unwrap());
        offset += 8;

        let real_token_reserves = u64::from_le_bytes(data[offset..offset + 8].try_into().unwrap());
        offset += 8;

        let real_sol_reserves = u64::from_le_bytes(data[offset..offset + 8].try_into().unwrap());
        offset += 8;

        let token_total_supply = u64::from_le_bytes(data[offset..offset + 8].try_into().unwrap());
        offset += 8;

        let complete = data[offset] != 0;
        offset += 1;

        let creator = Pubkey::new_from_array(data[offset..offset + 32].try_into().unwrap());

        Ok(BondingCurve {
            virtual_token_reserves,
            virtual_sol_reserves,
            real_token_reserves,
            real_sol_reserves,
            token_total_supply,
            complete,
            creator,
        })
    }

    pub async fn get_pool_reserves(&self, pool: &AmmPool) -> Result<PoolReserves> {
        let base_pubkey = pool.pool_base_token_account;
        let quote_pubkey = pool.pool_quote_token_account;
        let (base_account_res, quote_account_res) = join!(
            self.rpc_client.get_account_data(&base_pubkey),
            self.rpc_client.get_account_data(&quote_pubkey),
        );
        let base_account_data = base_account_res?;
        let quote_account_data = quote_account_res?;

        let base_amount = crate::util::parse_token_amount(&base_account_data)?;
        let quote_amount = crate::util::parse_token_amount(&quote_account_data)?;

        Ok(PoolReserves {
            base_amount: base_amount,
            quote_amount: quote_amount,
            base_mint: base_pubkey,
            quote_mint: quote_pubkey,
        })
    }
}

pub async fn read_amm_reserves(
    rpc_client: Arc<RpcClient>,
    base_token_account: Pubkey,
    quote_token_account: Pubkey,
) -> Result<PoolReserves> {
    let accounts = rpc_client
        .get_multiple_accounts(&[base_token_account, quote_token_account])
        .await?;

    let mut iter = accounts.into_iter();
    let base_account = iter.next().unwrap().unwrap();
    let quote_account = iter.next().unwrap().unwrap();

    let base_token_amount = crate::util::parse_token_amount(&base_account.data.as_ref())?;
    let quote_token_amount = crate::util::parse_token_amount(&quote_account.data.as_ref())?;

    Ok(PoolReserves {
        base_amount: base_token_amount,
        quote_amount: quote_token_amount,
        base_mint: base_token_account,
        quote_mint: quote_token_account,
    })
}
