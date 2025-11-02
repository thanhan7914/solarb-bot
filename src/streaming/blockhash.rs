use anchor_client::solana_sdk::hash::Hash;
use anyhow::Result;
use futures_util::StreamExt;
use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use tracing::debug;
use yellowstone_grpc_proto::geyser::{
    CommitmentLevel, SubscribeRequest, SubscribeRequestFilterBlocksMeta,
    geyser_client::GeyserClient, subscribe_update::UpdateOneof,
};

// GLOBAL BLOCKHASH
static GLOBAL_BLOCKHASH: RwLock<String> = parking_lot::const_rwlock(String::new());
static GLOBAL_SLOT: AtomicU64 = AtomicU64::new(0);

#[inline]
pub fn get_blockhash() -> String {
    GLOBAL_BLOCKHASH.read().clone()
}

#[inline]
pub fn get_blockhash_hash() -> Option<Hash> {
    let blockhash_str = GLOBAL_BLOCKHASH.read().clone();
    if blockhash_str.len() != 44 {
        // Base58 hash is always 44 chars
        return None;
    }

    bs58::decode(&blockhash_str)
        .into_vec()
        .ok()
        .and_then(|bytes| {
            if bytes.len() == 32 {
                Some(Hash::new_from_array(bytes.try_into().unwrap()))
            } else {
                None
            }
        })
}

#[inline]
pub fn get_slot() -> u64 {
    GLOBAL_SLOT.load(Ordering::Relaxed)
}

#[inline]
pub fn is_ready() -> bool {
    GLOBAL_BLOCKHASH.read().len() == 44
}

pub struct BlockhashTracker {
    stop_sender: Option<mpsc::UnboundedSender<()>>,
}

impl BlockhashTracker {
    pub fn stop(&mut self) {
        if let Some(sender) = self.stop_sender.take() {
            let _ = sender.send(());
        }
    }
}

impl Drop for BlockhashTracker {
    fn drop(&mut self) {
        self.stop();
    }
}

pub async fn start_tracking(endpoint: &str) -> Result<BlockhashTracker> {
    let (stop_tx, mut stop_rx) = mpsc::unbounded_channel();
    let endpoint = endpoint.to_string();

    // Spawn background task
    tokio::spawn(async move {
        loop {
            if stop_rx.try_recv().is_ok() {
                break;
            }

            if let Err(_) = run_tracker(&endpoint, &mut stop_rx).await {
                tokio::time::sleep(Duration::from_millis(500)).await;
            }
        }
        debug!("BlockhashTracker stopped");
    });

    let tracker = BlockhashTracker {
        stop_sender: Some(stop_tx),
    };

    // Wait for first blockhash (with timeout)
    for _ in 0..60 {
        // 30s timeout
        if is_ready() {
            debug!("BlockhashTracker ready");
            return Ok(tracker);
        }
        tokio::time::sleep(Duration::from_millis(500)).await;
    }

    Err(anyhow::anyhow!("Timeout waiting for blockhash"))
}

async fn run_tracker(endpoint: &str, stop_rx: &mut mpsc::UnboundedReceiver<()>) -> Result<()> {
    // Fast connection
    let channel = tonic::transport::Channel::from_shared(endpoint.to_string())?
        .timeout(Duration::from_secs(5))
        .tcp_keepalive(Some(Duration::from_secs(30)))
        .connect()
        .await?;

    let mut client = GeyserClient::new(channel);
    let (stream_tx, stream_rx) = mpsc::channel(4);

    // Minimal subscription - only block_meta
    let mut blocks_meta_filter = HashMap::new();
    blocks_meta_filter.insert(
        "blocks_meta".to_string(),
        SubscribeRequestFilterBlocksMeta {},
    );

    let request = SubscribeRequest {
        slots: HashMap::new(),
        accounts: HashMap::new(),
        transactions: HashMap::new(),
        transactions_status: HashMap::new(),
        blocks: HashMap::new(),
        blocks_meta: blocks_meta_filter,
        entry: HashMap::new(),
        commitment: Some(CommitmentLevel::Confirmed as i32),
        accounts_data_slice: vec![],
        ping: None,
    };

    stream_tx.send(request).await?;
    let request_stream = ReceiverStream::new(stream_rx);
    let mut response_stream = client.subscribe(request_stream).await?.into_inner();

    loop {
        tokio::select! {
            // HIGHEST PRIORITY: Process updates
            message = response_stream.next() => {
                match message {
                    Some(Ok(update)) => {
                        if let Some(UpdateOneof::BlockMeta(block_meta)) = &update.update_oneof {
                            GLOBAL_SLOT.store(block_meta.slot, Ordering::Relaxed);

                            if block_meta.blockhash.len() == 44 {
                                let current = GLOBAL_BLOCKHASH.read();
                                if *current != block_meta.blockhash {
                                    drop(current);
                                    *GLOBAL_BLOCKHASH.write() = block_meta.blockhash.clone();
                                }
                            }
                        }
                    }
                    Some(Err(_)) => return Err(anyhow::anyhow!("Stream error")),
                    None => return Err(anyhow::anyhow!("Stream ended")),
                }
            }

            // LOWER PRIORITY: Check stop signal
            _ = stop_rx.recv() => return Ok(()),
        }
    }
}
