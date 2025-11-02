use crate::{
    arb::SwapRoutes,
    global,
    instructions::{self, flashloan},
    onchain,
    util::rand_u32,
};
use anchor_client::{
    solana_client::rpc_config::RpcSendTransactionConfig,
    solana_sdk::{
        address_lookup_table::AddressLookupTableAccount, commitment_config::CommitmentLevel,
        hash::Hash, signature::Signature, transaction::VersionedTransaction,
    },
};
use tracing::{error, info};

fn adjust_cu_price(profit: i64) -> u64 {
    match profit {
        p if p < 50_000 => 5_000,
        p if p < 1_000_000 => 10_000,
        p if p < 5_000_000 => 15_000,
        p if p < 50_000_000 => 20_000,
        p if p < 100_000_000 => 50_000,
        p if p < 500_000_000 => 200_000,
        p if p < 1_000_000_000 => 500_000,
        p if p < 5_000_000_000 => 800_000,
        _ => 1_000_000, //
    }
}

pub async fn build_and_send(
    blockhash: Hash,
    swap_data: SwapRoutes,
    alt_accounts: &Vec<AddressLookupTableAccount>,
    user_base_amount: u64,
) -> Option<Signature> {
    let profit = swap_data.profit;
    let amount_in = if swap_data.threshold > 0 {
        swap_data.threshold
    } else {
        swap_data.amount_in
    };
    let mint = swap_data.mint;
    let mut ixs = vec![instructions::cu::price_instruction(adjust_cu_price(
        swap_data.profit,
    ))];
    let route_len: u32 = swap_data.routes.len() as u32;
    let swap_ix = instructions::aggregator::route(swap_data, 0).unwrap();
    let mut cu_limit = rand_u32(300_000, 350_000);
    let extra_cu: u32 = (route_len - 2) * 120_000;
    cu_limit += extra_cu;

    if amount_in > user_base_amount {
        match flashloan::kamino::find_reserve(&mint) {
            Some(kamino_reserve) => {
                // enable flashloan
                let payer = global::get_pubkey();
                let flashloan_index = (ixs.len() as u8) + 1;
                ixs.push(flashloan::kamino::flash_borrow_reserve_liquidity(
                    &payer,
                    kamino_reserve.clone(),
                    amount_in,
                ));
                ixs.push(swap_ix);
                ixs.push(flashloan::kamino::flash_repay_reserve_liquidity(
                    &payer,
                    kamino_reserve,
                    amount_in,
                    flashloan_index,
                ));
                cu_limit += 80_000;
            }
            None => {
                ixs.push(swap_ix);
            }
        }
    } else {
        ixs.push(swap_ix);
    }

    ixs.insert(0, instructions::cu::limit_instruction(cu_limit));

    let signature = match onchain::send::send_arb_tx(blockhash, &ixs, &alt_accounts).await {
        std::result::Result::Ok(sig) => {
            info!("Transaction hash {}", sig.to_string());
            Some(sig)
        }
        Err(e) => {
            error!("An error occus {}", e);
            None
        }
    };

    info!("Amount in {} SOL -> profit {} SOL", amount_in, profit);

    return signature;
}
