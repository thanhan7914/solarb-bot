use crate::{arb, pool_index, streaming, wsol_mint};
use tokio::time;
use tracing::info;

pub fn start(delay_seconds: u64) {
    let mut interval = time::interval(time::Duration::from_secs(delay_seconds));
    tokio::spawn(async move {
        info!("Log starting...");
        loop {
            interval.tick().await;
            let total_accounts = streaming::count_accounts();
            let now = time::Instant::now();
            let all = pool_index::get_all_pools();
            let els_time = now.elapsed();
            let native_pool_count = pool_index::native_pool_count();
            let pool_count = pool_index::pool_count();
            let wsol_p_count = pool_index::find_by_mint(&wsol_mint()).len();
            let route_count = pool_index::routes_count();

            info!(
                "{} watched accounts, {} pools, {} wsol pools, {} invalid pools, {} token pools, {} route counts",
                total_accounts,
                pool_count,
                wsol_p_count,
                pool_index::count_invalid_pools(),
                native_pool_count,
                route_count
            );
        }
    });
}
