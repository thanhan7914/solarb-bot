use anyhow::{Ok, Result};
use tracing::info;
use tracing_subscriber;

pub mod arb;
pub mod byte_reader;
pub mod cache;
pub mod config;
pub mod constants;
pub mod dex;
pub mod global;
pub mod inserter;
pub mod instructions;
pub mod io;
pub mod math;
pub mod metric;
pub mod onchain;
pub mod polling;
pub mod pool_index;
pub mod safe_math;
pub mod streaming;
pub mod transaction;
pub mod util;
pub mod watcher;

pub use constants::*;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    info!("Solarb client runing...");
    let conf = config::read_config("config.toml").unwrap();
    let _ = global::prepare_data(None, &conf.bot.mint).await;
    println!("Mainnet wallet {}", global::get_pubkey());
    let base_mint = global::get_base_mint().as_ref().clone();
    let base_mint_ata_amount = global::get_base_mint_amount();
    println!("Base mint {} - amount {}", base_mint, base_mint_ata_amount);

    {
        let command_tx = streaming::start(conf.clone()).await?;
        let command_tx_2 = command_tx.clone();
        watcher::monitoring(conf, Some(command_tx), 3).await?;
        let event_receiver = streaming::polling::start(10_000).await?;

        tokio::spawn(streaming::updater::signal_receiver(
            event_receiver,
            command_tx_2,
        ));

        polling::blockhash::start_blockhash_refresher(1);
        metric::start(60);
        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
        arb::processor::finding(100)?;

        tokio::signal::ctrl_c()
            .await
            .expect("Failed to listen for ctrl-c");

        info!("Shutting down...");
    }

    Ok(())
}
