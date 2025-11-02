use super::*;
use std::sync::Arc;
use tokio::{
    sync::{Semaphore, TryAcquireError, mpsc},
    task::JoinSet,
    time::{Instant},
};
use tracing::{error, info, warn};

const SEMAPHORE_PERMITS: usize = 20;

#[derive(Debug)]
pub struct ArbitrageEvent {
    pub swap_routes: Vec<SwapRoutes>,
    pub receive_time: Instant,
    pub source: SourceType,
}

pub type ArbitrageEventSender = mpsc::UnboundedSender<ArbitrageEvent>;

async fn signal_receiver(mut event_receiver: mpsc::UnboundedReceiver<ArbitrageEvent>) {
    info!("Starting arbitrage processor...");
    let sem = Arc::new(Semaphore::new(SEMAPHORE_PERMITS));
    let mut set = JoinSet::new();

    while let Some(event) = event_receiver.recv().await {
        for swap in event.swap_routes {
            let permit = match sem.clone().try_acquire_owned() {
                std::result::Result::Ok(p) => p,
                Err(TryAcquireError::NoPermits) => {
                    warn!("All semaphore permits in use, dropping route");
                    continue;
                }
                Err(TryAcquireError::Closed) => {
                    error!("Semaphore closed, shutting down...");
                    break;
                }
            };

            let receive_time = event.receive_time;
            set.spawn(async move {
                let _permit = permit;
                let route_start = Instant::now();
                let amount_in = swap.amount_in;
                let profit = swap.profit;

                if let std::result::Result::Ok(sent) = sender::do_arb(swap, receive_time).await {
                    if sent {
                        // info!("{:#?}", route);
                        info!(
                            "From {:?} - amount in {} -> {} ({:?})",
                            event.source, amount_in, profit, route_start.elapsed()
                        );
                    }
                }
            });
        }
    }

    info!("Waiting for {} remaining tasks to complete...", set.len());
    while let Some(result) = set.join_next().await {
        if let Err(e) = result {
            error!("Task cleanup error: {:?}", e);
        }
    }

    info!("Arbitrage processor stopped");
}

pub fn create() -> ArbitrageEventSender {
    let (command_tx, processor_receiver) = mpsc::unbounded_channel::<ArbitrageEvent>();
    tokio::spawn(signal_receiver(processor_receiver));
    command_tx
}
