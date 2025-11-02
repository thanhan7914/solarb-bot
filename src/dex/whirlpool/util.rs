use super::{
    get_tick_array_start_tick_index, program_id,
    state::pda::get_tick_array_address,
    state::{
        FeeTier, Position, TickArray, Whirlpool, WhirlpoolsConfig, oracle::Oracle, tick::Tick,
    },
    types::tick_array::{TICK_ARRAY_SIZE, TICK_ARRAY_SIZE_USIZE},
};
use anchor_client::{
    solana_client::nonblocking::rpc_client::RpcClient, solana_sdk::pubkey::Pubkey,
};
use anyhow::Result;
use std::{iter::zip, sync::Arc};

pub async fn fetch_and_deserialize_whirlpool(
    rpc_client: Arc<RpcClient>,
    whirlpool_address: &Pubkey,
) -> Result<Whirlpool> {
    let account = rpc_client.get_account(whirlpool_address).await?;
    Whirlpool::deserialize(&account.data)
}

pub async fn fetch_and_deserialize_tick_array(
    rpc_client: Arc<RpcClient>,
    tick_array_address: &Pubkey,
) -> Result<TickArray> {
    let account = rpc_client.get_account(tick_array_address).await?;
    TickArray::deserialize(&account.data)
}

pub async fn fetch_and_deserialize_position(
    rpc_client: Arc<RpcClient>,
    position_address: &Pubkey,
) -> Result<Position> {
    let account = rpc_client.get_account(position_address).await?;
    Position::deserialize(&account.data)
}

pub async fn fetch_and_deserialize_config(
    rpc_client: Arc<RpcClient>,
    config_address: &Pubkey,
) -> Result<WhirlpoolsConfig> {
    let account = rpc_client.get_account(config_address).await?;
    WhirlpoolsConfig::deserialize(&account.data)
}

pub async fn fetch_and_deserialize_fee_tier(
    rpc_client: Arc<RpcClient>,
    fee_tier_address: &Pubkey,
) -> Result<FeeTier> {
    let account = rpc_client.get_account(fee_tier_address).await?;
    FeeTier::deserialize(&account.data)
}

pub async fn fetch_oracle(
    rpc_client: Arc<RpcClient>,
    oracle_address: Pubkey,
    whirlpool: &Whirlpool,
) -> Result<Option<Oracle>> {
    // no need to fetch oracle for non-adaptive fee whirlpools
    if whirlpool.tick_spacing == u16::from_le_bytes(whirlpool.fee_tier_index_seed) {
        return Ok(None);
    }
    let oracle_info = rpc_client.get_account(&oracle_address).await?;
    Ok(Some(Oracle::deserialize(&oracle_info.data)?))
}

pub async fn fetch_and_deserialize_oracle(
    rpc_client: Arc<RpcClient>,
    whirlpool_address: &Pubkey,
) -> Option<Oracle> {
    let oracle_address = super::state::pda::derive_oracle_address(whirlpool_address)
        .unwrap()
        .0;
    println!("oracle address {}", oracle_address);
    match rpc_client.get_account(&oracle_address).await {
        std::result::Result::Ok(account) => Some(Oracle::deserialize(&account.data).unwrap()),
        Err(_) => None,
    }
}

pub fn uninitialized_tick_array(start_tick_index: i32) -> TickArray {
    TickArray {
        start_tick_index,
        ticks: [Tick::default(); TICK_ARRAY_SIZE_USIZE],
        whirlpool: program_id(),
    }
}

pub async fn fetch_tick_arrays_or_default(
    rpc: Arc<RpcClient>,
    whirlpool_address: Pubkey,
    whirlpool: &Whirlpool,
) -> Result<[(Pubkey, TickArray); 5]> {
    let tick_array_start_index =
        get_tick_array_start_tick_index(whirlpool.tick_current_index, whirlpool.tick_spacing);
    let offset = whirlpool.tick_spacing as i32 * TICK_ARRAY_SIZE as i32;

    let tick_array_indexes = [
        tick_array_start_index,
        tick_array_start_index + offset,
        tick_array_start_index + offset * 2,
        tick_array_start_index - offset,
        tick_array_start_index - offset * 2,
    ];

    let tick_array_addresses: Vec<Pubkey> = tick_array_indexes
        .iter()
        .map(|&x| get_tick_array_address(&whirlpool_address, x).map(|y| y.0))
        .collect::<Result<Vec<Pubkey>, _>>()?;

    let tick_array_infos = rpc.get_multiple_accounts(&tick_array_addresses).await?;
    let maybe_tick_arrays: Vec<Option<TickArray>> = tick_array_infos
        .iter()
        .map(|account_option| {
            account_option
                .as_ref()
                .and_then(|account| TickArray::deserialize(&account.data).ok())
        })
        .collect();

    let tick_arrays: Vec<TickArray> = maybe_tick_arrays
        .iter()
        .enumerate()
        .map(|(i, x)| {
            x.clone()
                .unwrap_or(uninitialized_tick_array(tick_array_indexes[i]))
        })
        .collect::<Vec<TickArray>>();

    let result: [(Pubkey, TickArray); 5] = zip(tick_array_addresses, tick_arrays)
        .collect::<Vec<(Pubkey, TickArray)>>()
        .try_into()
        .map_err(|_| "Failed to convert tick arrays to array".to_string())
        .unwrap();

    Ok(result)
}

pub fn get_tick_arrays_or_default(
    whirlpool_address: Pubkey,
    whirlpool: &Whirlpool,
) -> Result<Vec<Pubkey>> {
    let tick_array_start_index =
        get_tick_array_start_tick_index(whirlpool.tick_current_index, whirlpool.tick_spacing);
    let offset = whirlpool.tick_spacing as i32 * TICK_ARRAY_SIZE as i32;

    let tick_array_indexes = [
        tick_array_start_index,
        tick_array_start_index + offset,
        tick_array_start_index + offset * 2,
        tick_array_start_index - offset,
        tick_array_start_index - offset * 2,
    ];

    let tick_array_addresses: Vec<Pubkey> = tick_array_indexes
        .iter()
        .map(|&x| get_tick_array_address(&whirlpool_address, x).map(|y| y.0))
        .collect::<Result<Vec<Pubkey>, _>>()?;

    Ok(tick_array_addresses)
}
