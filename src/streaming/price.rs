// price updater
use super::*;
use anyhow::Result;
use futures::future::join_all;
use tokio;
use tracing::{error, info};

async fn updater(pools: &[Arc<TokenPool>]) -> Result<()> {
    for pool in pools {
        let pubkey = pool.pool;
        if let Some(pool_type) = pool.to_pool_type() {
            let (atob, _) = pool_type.get_price(&pool.mint_a);
            global_data::update_price(&pubkey, pool.mint_a, atob);
        }
    }

    Ok(())
}

async fn sync_epoch() -> Result<()> {
    let all_pools: Vec<Arc<TokenPool>> = pool_index::get_all_pools();
    if all_pools.is_empty() {
        return Ok(());
    }

    let chunks: Vec<Vec<Arc<TokenPool>>> =
        all_pools.chunks(100).map(|chunk| chunk.to_vec()).collect();

    let tasks: Vec<_> = chunks.iter().map(|chunk| updater(chunk)).collect();
    join_all(tasks).await;

    Ok(())
}

pub fn sync_price() -> Result<()> {
    let mut interval = tokio::time::interval(tokio::time::Duration::from_millis(100));
    tokio::spawn(async move {
        info!("Begin sync_price thread...");

        loop {
            interval.tick().await;
            if let Err(e) = sync_epoch().await {
                error!("sync_price failed: {}", e);
            }
        }
    });

    Ok(())
}
