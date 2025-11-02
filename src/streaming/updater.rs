use super::{
    ACCOUNT_DATA, ACCOUNT_TYPE_MAP, AccountDataType, AccountTypeInfo, WatcherCommand, global_data,
    util, watcher::AccountUpdateEvent,
};
use crate::{
    global,
    pool_index::{self},
    dex::{raydium, whirlpool}
};
use anchor_client::solana_sdk::pubkey::Pubkey;
use anyhow::{Ok, Result};
use commons::get_bin_array_pubkeys_for_swap;
use dlmm_interface::{BinArray, BinArrayAccount, LbPair};
use std::collections::HashMap;
use tokio::{sync::mpsc, time::Duration};
use tracing::{error, info, warn};

pub async fn signal_receiver(
    mut event_receiver: mpsc::UnboundedReceiver<AccountUpdateEvent>,
    command: mpsc::UnboundedSender<WatcherCommand>,
) {
    info!("Starting updater...");

    while let Some(event) = event_receiver.recv().await {
        let command_clone = command.clone();

        tokio::spawn(async move {
            match pool_index::get(&event.pubkey) {
                Some(pool) => {
                    if let Some(pool_type) = pool.to_pool_type() {
                        let (atob, _) = pool_type.get_price(&pool.mint_a);
                        global_data::update_price(&event.pubkey, pool.mint_a, atob);
                    }
                }
                None => {}
            }

            match &event.data {
                &AccountDataType::DlmmPair(lb_pair) => {
                    // Add bin arrays if needed
                    if let std::result::Result::Ok(bin_arrays) =
                        get_dlmm_bin_array_keys(event.pubkey, &lb_pair)
                    {
                        let new_keys: Vec<String> = bin_arrays
                            .iter()
                            .filter(|key| !ACCOUNT_DATA.contains_key(key))
                            .map(|key| key.to_string())
                            .collect();

                        if !new_keys.is_empty() {
                            let _ = add_bin_array_accounts(&bin_arrays).await;
                            if let Err(e) =
                                command_clone.send(WatcherCommand::BatchAdd { accounts: new_keys })
                            {
                                error!("Failed to send watcher command: {}", e);
                                // Note: Can't break from spawned task, just return
                                return;
                            }
                        }
                    }
                }
                &AccountDataType::RaydiumClmmPool(ref pool_state) => {
                    match super::loader::get_bitmap_ext(&event.pubkey) {
                        Some(bitmap_state) => {
                            let left_ticks =
                                raydium::clmm::swap_util::get_cur_and_next_five_tick_array(
                                    event.pubkey,
                                    &pool_state,
                                    &bitmap_state,
                                    false,
                                );
                            let right_ticks =
                                raydium::clmm::swap_util::get_cur_and_next_five_tick_array(
                                    event.pubkey,
                                    &pool_state,
                                    &bitmap_state,
                                    true,
                                );
                            let ticks = util::merge(&[&left_ticks, &right_ticks]);
                            let new_keys = nonexists_pubkeys(&ticks);
                            if !new_keys.is_empty() {
                                global_data::add_accounts_type_str(
                                    &new_keys,
                                    AccountTypeInfo::RaydiumTickArrayState,
                                );
                                if let Err(e) = command_clone
                                    .send(WatcherCommand::BatchAdd { accounts: new_keys })
                                {
                                    error!("Failed to send watcher command: {}", e);
                                    return;
                                }
                            }
                        }
                        None => {}
                    }
                }
                &AccountDataType::Whirlpool(ref pool_state) => {
                    match whirlpool::util::get_tick_arrays_or_default(event.pubkey, &pool_state) {
                        std::result::Result::Ok(tick_arrays) => {
                            let new_keys = nonexists_pubkeys(&tick_arrays);
                            if !new_keys.is_empty() {
                                global_data::add_accounts_type_str(
                                    &new_keys,
                                    AccountTypeInfo::WhirlpoolTickArray,
                                );
                                if let Err(e) = command_clone
                                    .send(WatcherCommand::BatchAdd { accounts: new_keys })
                                {
                                    error!("Failed to send watcher command: {}", e);
                                    return;
                                }
                            }
                        }
                        Err(_) => {}
                    }
                }
                _ => {}
            }
        });
    }

    info!("Updater stopped");
}

#[inline]
fn get_dlmm_bin_array_keys(address: Pubkey, lb_pair: &LbPair) -> Result<Vec<Pubkey>> {
    let left_bins = get_bin_array_pubkeys_for_swap(address, lb_pair, None, true, 3)?;
    let right_bins = get_bin_array_pubkeys_for_swap(address, lb_pair, None, false, 3)?;

    Ok(util::concat(&left_bins, &right_bins))
}

async fn add_bin_array_accounts(pubkeys: &[Pubkey]) -> Result<()> {
    let rpc_client = global::get_rpc_client();

    let timeout_duration = Duration::from_secs(30);
    let accounts =
        tokio::time::timeout(timeout_duration, rpc_client.get_multiple_accounts(pubkeys))
            .await
            .map_err(|_| anyhow::anyhow!("RPC timeout after 30s"))?
            .map_err(|e| anyhow::anyhow!("RPC error: {}", e))?;

    for (pubkey, account) in pubkeys.iter().zip(accounts.into_iter()) {
        if let Some(data) = account {
            match BinArrayAccount::deserialize(&data.data) {
                std::result::Result::Ok(bin_array) => {
                    ACCOUNT_TYPE_MAP.insert(*pubkey, AccountTypeInfo::BinArray);
                    ACCOUNT_DATA.insert(*pubkey, AccountDataType::BinArray(bin_array.0));
                }
                Err(e) => {
                    warn!("Failed to deserialize bin array for {}: {}", pubkey, e);
                }
            }
        } else {
            warn!("Account {} not found", pubkey);
        }
    }

    Ok(())
}

#[inline]
pub fn get_bin_arrays(pubkeys: &[Pubkey]) -> Option<HashMap<Pubkey, BinArray>> {
    let mut bin_arrays = HashMap::with_capacity(pubkeys.len());

    for pk in pubkeys {
        if let Some(AccountDataType::BinArray(bin_array)) = global_data::get_account(&pk) {
            bin_arrays.insert(*pk, bin_array);
        }
    }

    if !bin_arrays.is_empty() {
        Some(bin_arrays)
    } else {
        None
    }
}

#[inline]
fn nonexists_pubkeys(pubkeys: &[Pubkey]) -> Vec<String> {
    let new_keys: Vec<String> = pubkeys
        .iter()
        .filter(|key| !ACCOUNT_DATA.contains_key(key))
        .map(|key| key.to_string())
        .collect();

    new_keys
}
