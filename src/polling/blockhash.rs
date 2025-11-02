use crate::global;
use anchor_client::{solana_client::nonblocking::rpc_client::RpcClient, solana_sdk::hash::Hash};
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::time::Duration;
use tracing::{error, info};

static BLOCKHASH: once_cell::sync::Lazy<Arc<RwLock<Option<Hash>>>> =
    once_cell::sync::Lazy::new(|| Arc::new(RwLock::new(None)));

pub async fn blockhash_refresher(rpc_client: Arc<RpcClient>, refresh_interval: Duration) {
    info!("starting blockhash refresher");

    loop {
        match rpc_client.get_latest_blockhash().await {
            std::result::Result::Ok(blockhash) => {
                {
                    let mut global_blockhash = BLOCKHASH.write().await;
                    *global_blockhash = Some(blockhash);
                }
                // info!("Blockhash refreshed: {}", blockhash);
            }
            Err(e) => {
                error!("Failed to refresh blockhash: {:?}", e);
            }
        }
        tokio::time::sleep(refresh_interval).await;
    }
}

pub async fn get_current_blockhash() -> Option<Hash> {
    let blockhash = BLOCKHASH.read().await;
    *blockhash
}

pub fn start_blockhash_refresher(delay: u64) {
    let refresh_interval = tokio::time::Duration::from_secs(delay);
    tokio::spawn(async move {
        blockhash_refresher(global::get_rpc_client(), refresh_interval).await;
    });
}
