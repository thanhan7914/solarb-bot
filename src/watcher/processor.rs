use super::POOL_QUEUE;
use crate::{
    global::{self, get_base_mint},
    inserter,
    pool_index::{self, TokenPool},
    streaming::{self, AccountDataType, WatcherCommand, global_data},
    wsol_mint,
};
use anchor_client::solana_sdk::pubkey::Pubkey;
use anyhow::Result;
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::error;

const ENABLED_LOG: bool = false;

pub async fn handle_batch_process(
    command: mpsc::UnboundedSender<WatcherCommand>,
    num_workers: usize,
    batch_size: usize,
) -> Result<()> {
    let command = Arc::new(command);
    let mut handles = Vec::new();

    for worker_id in 0..num_workers {
        let command_clone = command.clone();

        let handle =
            tokio::spawn(async move { batch_worker(worker_id, command_clone, batch_size).await });

        handles.push(handle);
    }

    for (worker_id, handle) in handles.into_iter().enumerate() {
        if let Err(e) = handle.await {
            error!("❌ Batch worker {} failed: {}", worker_id, e);
        }
    }

    Ok(())
}

async fn batch_worker(
    worker_id: usize,
    command: Arc<mpsc::UnboundedSender<WatcherCommand>>,
    batch_size: usize,
) -> Result<()> {
    loop {
        let mut batch = Vec::new();

        for _ in 0..batch_size {
            if let Some(item) = POOL_QUEUE.pop() {
                batch.push(item);
            } else {
                break;
            }
        }

        if batch.is_empty() {
            tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
            continue;
        }

        let tasks: Vec<_> = batch
            .into_iter()
            .enumerate()
            .filter(|(_, (pool_pk, _, _))| !pool_index::has_pool(&pool_pk))
            .map(|(idx, (pool_pk, pool_data, alt_op))| {
                let command_clone = command.clone();

                tokio::spawn(async move {
                    if let Err(e) =
                        process_pool_item(worker_id, idx, pool_pk, pool_data, alt_op, command_clone)
                            .await
                    {
                        error!("❌ Worker {} item {} failed: {}", worker_id, idx, e);
                    }
                })
            })
            .collect();

        for (idx, task) in tasks.into_iter().enumerate() {
            if let Err(e) = task.await {
                error!("❌ Worker {} batch task {} panicked: {}", worker_id, idx, e);
            }
        }
    }
}

async fn process_pool_item(
    worker_id: usize,
    item_idx: usize,
    pool_pk: Pubkey,
    pool_data: crate::streaming::AccountDataType,
    alt_op: Option<Pubkey>,
    command: Arc<mpsc::UnboundedSender<WatcherCommand>>,
) -> Result<()> {
    if !streaming::has_alt_pk(&pool_pk) {
        if let Some(alt_pk) = alt_op {
            streaming::store_lookup_table(&alt_pk).await?;
            streaming::store_mint_alt(pool_pk, alt_pk);
        }
    }

    if pool_index::has_pool(&pool_pk) {
        return Ok(());
    }

    if ENABLED_LOG {
        println!(
            "Worker Id {} item {} pool {:?}",
            worker_id,
            item_idx,
            pool_data.to_token_pool(pool_pk)
        );
    }

    let base_mint = get_base_mint().as_ref().clone();
    if let Some(token_pool) = pool_data.to_token_pool(pool_pk) {
        if is_native_pool(&token_pool).await? {
            if base_mint == wsol_mint() || !token_pool.is_pumpfun_pool() {
                let new_keys = inserter::add(token_pool, pool_data).await?;
                let pk_as_str = streaming::util::pubkeys_to_strings(&new_keys);

                if let Err(e) = command.send(WatcherCommand::BatchAdd {
                    accounts: pk_as_str,
                }) {
                    error!(
                        "❌ Worker {} item {}: Failed to send watcher command: {}",
                        worker_id, item_idx, e
                    );
                    return Err(e.into());
                }
            }
        }
    }

    Ok(())
}

async fn is_native_pool(pool: &TokenPool) -> Result<bool> {
    let token_program = crate::token_program();
    if let (Some(AccountDataType::Account(a)), Some(AccountDataType::Account(b))) = (
        global_data::get_account(&pool.mint_a),
        global_data::get_account(&pool.mint_b),
    ) {
        return Ok(a.owner == token_program && b.owner == token_program);
    }

    let rpc = global::get_rpc_client();
    let accounts = rpc
        .get_multiple_accounts(&[pool.mint_a, pool.mint_b])
        .await?;

    Ok(accounts
        .iter()
        .all(|opt| matches!(opt, Some(acc) if acc.owner == token_program)))
}

pub async fn run_process(command: mpsc::UnboundedSender<WatcherCommand>) -> Result<()> {
    handle_batch_process(command, 10, 5).await
}
