use tokio::sync::mpsc;
use tokio::time::{Duration, interval};
use tracing::info;

use super::*;

pub async fn watch(command: mpsc::UnboundedSender<WatcherCommand>, delay_seconds: u64) {
    let mut interval = interval(Duration::from_secs(delay_seconds));

    info!("Starting performance monitoring...");

    loop {
        interval.tick().await;

        let total_accounts = ACCOUNT_DATA.len();

        if total_accounts > 0 {
            let arbitrage_pairs = get_all_pair_prices();
            info!(
                "Performance: {} accounts, {} pairs tracked",
                total_accounts,
                arbitrage_pairs.len()
            );
        }

        let _ = command.send(WatcherCommand::GetMetrics);
    }
}

pub fn get_all_pair_prices() -> Vec<(Pubkey, i32)> {
    ACCOUNT_DATA
        .iter()
        .filter_map(|entry| match entry.value() {
            AccountDataType::DlmmPair(pair) => Some((entry.key().clone(), pair.active_id)),
            _ => None,
        })
        .collect()
}
