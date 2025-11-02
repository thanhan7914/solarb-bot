use super::*;
use crate::{
    arb::PoolType,
    dex::{meteora, pumpfun, raydium, whirlpool},
    pool_index::TokenPoolType,
    streaming::global_data,
    wsol_mint,
};
use anchor_client::solana_sdk::{clock::Clock, pubkey::Pubkey};
use anyhow::Result;
use commons::quote as dlmm_quote;
use std::panic::{AssertUnwindSafe, catch_unwind};

impl PoolType {
    // return price and quote_mint
    pub fn get_price(&self, base_mint: &Pubkey) -> (f64, &Pubkey) {
        match self {
            PoolType::Meteora(_, data) => {
                let price =
                    meteora::utils::compute_price(data.lb_pair.active_id, data.lb_pair.bin_step);
                if &data.lb_pair.token_x_mint == base_mint {
                    (price, &data.lb_pair.token_y_mint)
                } else {
                    (1.0 / price, &data.lb_pair.token_x_mint)
                }
            }
            PoolType::Pump(_, data) => {
                let base_amount = data.reserves.base_amount;
                let quote = data.reserves.quote_amount as f64;
                let base = base_amount as f64;

                if &data.pool.base_mint == base_mint {
                    (quote / base, &data.pool.quote_mint)
                } else {
                    (base / quote, &data.pool.base_mint)
                }
            }
            PoolType::MeteoraDammv2(_, data) => {
                let price = data.pool_state.get_price();
                if &data.pool_state.token_a_mint == base_mint {
                    (price, &data.pool_state.token_b_mint)
                } else {
                    (1.0 / price, &data.pool_state.token_a_mint)
                }
            }
            PoolType::RaydiumAmm(_, data) => {
                let pc_vault = data.vaults.pc_vault_amount as f64;
                let coin_vault = data.vaults.coin_vault_amount as f64;

                if &data.pool_state.coin_mint == base_mint {
                    (pc_vault / coin_vault, &data.pool_state.pc_mint)
                } else {
                    (coin_vault / pc_vault, &data.pool_state.coin_mint)
                }
            }
            PoolType::RaydiumCpmm(_, data) => {
                let token_0_amount = data.vaults.token_0_amount as f64;
                let token_1_amount = data.vaults.token_1_amount as f64;

                if &data.pool_state.token_0_mint == base_mint {
                    (
                        token_1_amount / token_0_amount,
                        &data.pool_state.token_1_mint,
                    )
                } else {
                    (
                        token_0_amount / token_1_amount,
                        &data.pool_state.token_0_mint,
                    )
                }
            }
            PoolType::RaydiumClmm(_, data) => {
                // price token 1 / token 0
                let price = data.pool_state.get_price();
                if &data.pool_state.token_mint_0 == base_mint {
                    (price, &data.pool_state.token_mint_1)
                } else {
                    (1.0 / price, &data.pool_state.token_mint_0)
                }
            }
            PoolType::Whirlpool(_, data) => {
                // price token 1 / token 0
                let price = data.pool_state.get_price();
                if &data.pool_state.token_mint_a == base_mint {
                    (price, &data.pool_state.token_mint_b)
                } else {
                    (1.0 / price, &data.pool_state.token_mint_a)
                }
            }
            PoolType::Vertigo(_, data) => {
                if &data.pool_state.mint_a == base_mint {
                    (data.pool_state.get_price_a_in_b(), &data.pool_state.mint_b)
                } else {
                    (data.pool_state.get_price_b_in_a(), &data.pool_state.mint_a)
                }
            }
            PoolType::Solfi(_, data) => {
                if &data.pool_state.mint_a == base_mint {
                    (data.reserves.get_price_a_in_b(), &data.pool_state.mint_b)
                } else {
                    (data.reserves.get_price_b_in_a(), &data.pool_state.mint_a)
                }
            }
        }
    }

    #[inline]
    pub fn compute_price(&self, mint_in: &Pubkey, amount_in: u64) -> (f64, u64) {
        let clock = match global_data::get_clock() {
            Some(c) => c,
            None => return (0.0, 0),
        };

        let amount_out: u64 = catch_unwind(AssertUnwindSafe(|| {
            self.compute_swap(&clock, mint_in, amount_in)
        }))
        .ok()
        .and_then(|r| r.ok())
        .map(|v| v.max(0) as u64)
        .unwrap_or(0);

        (amount_out as f64 / amount_in as f64, amount_out)
    }

    pub fn compute_swap(
        &self,
        clock: &Clock,
        mint_in: &Pubkey,
        current_amount: u64,
    ) -> Result<u64> {
        let current_timestamp = clock.unix_timestamp as u64;
        let current_slot = clock.slot;

        let (amount_out, _mint_out) = match self {
            PoolType::Pump(_, data) => {
                if mint_in != &wsol_mint() {
                    let sell_quote = pumpfun::quote::sell_base_input_internal(
                        current_amount as u128,
                        0f64,
                        data.reserves.base_amount as u128,
                        data.reserves.quote_amount as u128,
                        20,
                        5,
                        80,
                        data.pool.coin_creator,
                    )?;

                    (sell_quote.min_quote as u64, &data.pool.quote_mint)
                } else {
                    let buy_quote = pumpfun::quote::buy_quote_input_internal(
                        current_amount as u128,
                        0f64,
                        data.reserves.base_amount as u128,
                        data.reserves.quote_amount as u128,
                        20,
                        5,
                        80,
                        data.pool.coin_creator,
                    )?;

                    (buy_quote.base as u64, &data.pool.base_mint)
                }
            }
            PoolType::Meteora(address, data) => {
                let quote = dlmm_quote::quote_exact_in(
                    *address,
                    &data.lb_pair,
                    current_amount,
                    &data.lb_pair.token_y_mint != mint_in,
                    data.bin_arrays.clone(),
                    None,
                    clock,
                    &data.mint_x_account,
                    &data.mint_y_account,
                )?;

                let token_out_mint = if &data.lb_pair.token_x_mint == mint_in {
                    &data.lb_pair.token_y_mint
                } else {
                    &data.lb_pair.token_x_mint
                };

                (quote.amount_out, token_out_mint)
            }
            PoolType::MeteoraDammv2(_, data) => {
                let quote = meteora::damm::get_quote(
                    &data.pool_state,
                    current_timestamp,
                    current_slot,
                    current_amount,
                    &data.pool_state.token_a_mint == mint_in,
                    false,
                )?;

                let token_out_mint = if &data.pool_state.token_a_mint == mint_in {
                    &data.pool_state.token_b_mint
                } else {
                    &data.pool_state.token_a_mint
                };

                (quote.output_amount, token_out_mint)
            }
            PoolType::RaydiumAmm(_, data) => {
                let (swap_direction, token_out_mint) = if mint_in == &data.pool_state.coin_mint {
                    (
                        raydium::amm::SwapDirection::Coin2PC,
                        &data.pool_state.pc_mint,
                    )
                } else {
                    (
                        raydium::amm::SwapDirection::PC2Coin,
                        &data.pool_state.coin_mint,
                    )
                };

                let quote = raydium::amm::swap_compute(
                    &data.pool_state,
                    &data.vaults,
                    swap_direction,
                    current_amount,
                    true,
                    0,
                )?;

                (quote, token_out_mint)
            }
            PoolType::RaydiumCpmm(_, data) => {
                let (a_to_b, token_out_mint) = if &data.pool_state.token_0_mint == mint_in {
                    (true, &data.pool_state.token_1_mint)
                } else {
                    (false, &data.pool_state.token_0_mint)
                };

                let quote = raydium::cpmm::swap_calculate(
                    &data.amm_config,
                    &data.pool_state,
                    &data.vaults,
                    current_amount,
                    a_to_b,
                )?;

                (quote.other_amount_threshold, token_out_mint)
            }
            PoolType::RaydiumClmm(_, data) => {
                let (a_to_b, token_out_mint) = if &data.pool_state.token_mint_0 == mint_in {
                    (true, &data.pool_state.token_mint_1)
                } else {
                    (false, &data.pool_state.token_mint_0)
                };

                let mut tick_clone = if a_to_b {
                    data.right_ticks.clone()
                } else {
                    data.left_ticks.clone()
                };
                let (amount_out, _) =
                    raydium::clmm::swap_util::get_out_put_amount_and_remaining_accounts(
                        current_amount,
                        None,
                        a_to_b,
                        true,
                        0,
                        &data.pool_state,
                        &data.tick_array_bitmap_ext,
                        &mut tick_clone,
                    )
                    .unwrap_or_default();

                (amount_out, token_out_mint)
            }
            PoolType::Whirlpool(_, data) => {
                let (a_to_b, token_out_mint) = if &data.pool_state.token_mint_a == mint_in {
                    (true, &data.pool_state.token_mint_b)
                } else {
                    (false, &data.pool_state.token_mint_a)
                };

                let tick_arrays = data
                    .tick_data
                    .clone()
                    .map(|(_, tick_array)| Some(tick_array));
                let quote = whirlpool::quote::swap_quote_by_input_token(
                    current_amount,
                    a_to_b,
                    0,
                    data.pool_state.clone(),
                    data.oracle.clone(),
                    tick_arrays,
                    current_timestamp,
                    None,
                    None,
                )
                .unwrap_or_default();

                (quote.token_min_out, token_out_mint)
            }
            PoolType::Vertigo(_, data) => {
                let (amount_out, token_out_mint) = if &data.pool_state.mint_a == mint_in {
                    let amount_out = data
                        .pool_state
                        .calculate_buy_amount_out(current_amount, current_slot)?;
                    (amount_out, &data.pool_state.mint_b)
                } else {
                    let amount_out = data
                        .pool_state
                        .calculate_sell_amount_in(current_amount, current_slot)?;
                    (amount_out, &data.pool_state.mint_a)
                };

                (amount_out, token_out_mint)
            }
            PoolType::Solfi(_, data) => {
                let (a_to_b, token_out_mint) = if &data.pool_state.mint_a == mint_in {
                    (true, &data.pool_state.mint_b)
                } else {
                    (false, &data.pool_state.mint_a)
                };

                let amount_out = data.reserves.swap_quote(current_amount, a_to_b);

                (amount_out, token_out_mint)
            }
        };

        Ok(amount_out)
    }

    #[inline]
    pub fn get_address(&self) -> &Pubkey {
        match self {
            PoolType::Meteora(address, _)
            | PoolType::Pump(address, _)
            | PoolType::MeteoraDammv2(address, _)
            | PoolType::Vertigo(address, _)
            | PoolType::RaydiumAmm(address, _)
            | PoolType::RaydiumCpmm(address, _)
            | PoolType::RaydiumClmm(address, _)
            | PoolType::Whirlpool(address, _)
            | PoolType::Solfi(address, _) => address,
        }
    }

    #[inline]
    pub fn get_other_mint(&self, mint: &Pubkey) -> Pubkey {
        match self {
            PoolType::Meteora(_, data) => {
                if &data.lb_pair.token_x_mint == mint {
                    data.lb_pair.token_y_mint
                } else {
                    data.lb_pair.token_x_mint
                }
            }
            PoolType::Pump(_, data) => {
                if &data.pool.base_mint == mint {
                    data.pool.quote_mint
                } else {
                    data.pool.base_mint
                }
            }
            PoolType::MeteoraDammv2(_, data) => {
                if &data.pool_state.token_a_mint == mint {
                    data.pool_state.token_b_mint
                } else {
                    data.pool_state.token_a_mint
                }
            }
            PoolType::RaydiumAmm(_, data) => {
                if &data.pool_state.pc_mint == mint {
                    data.pool_state.coin_mint
                } else {
                    data.pool_state.pc_mint
                }
            }
            PoolType::RaydiumCpmm(_, data) => {
                if &data.pool_state.token_0_mint == mint {
                    data.pool_state.token_1_mint
                } else {
                    data.pool_state.token_0_mint
                }
            }
            PoolType::RaydiumClmm(_, data) => {
                if &data.pool_state.token_mint_0 == mint {
                    data.pool_state.token_mint_1
                } else {
                    data.pool_state.token_mint_0
                }
            }
            PoolType::Whirlpool(_, data) => {
                if &data.pool_state.token_mint_a == mint {
                    data.pool_state.token_mint_b
                } else {
                    data.pool_state.token_mint_a
                }
            }
            PoolType::Vertigo(_, data) => {
                if &data.pool_state.mint_a == mint {
                    data.pool_state.mint_b
                } else {
                    data.pool_state.mint_a
                }
            }
            PoolType::Solfi(_, data) => {
                if &data.pool_state.mint_a == mint {
                    data.pool_state.mint_b
                } else {
                    data.pool_state.mint_a
                }
            }
        }
    }

    #[inline]
    pub fn get_mints(&self) -> (Pubkey, Pubkey) {
        match self {
            PoolType::Meteora(_, data) => (data.lb_pair.token_x_mint, data.lb_pair.token_y_mint),
            PoolType::Pump(_, data) => (data.pool.base_mint, data.pool.quote_mint),
            PoolType::MeteoraDammv2(_, data) => {
                (data.pool_state.token_a_mint, data.pool_state.token_b_mint)
            }
            PoolType::RaydiumAmm(_, data) => (data.pool_state.pc_mint, data.pool_state.coin_mint),
            PoolType::RaydiumCpmm(_, data) => {
                (data.pool_state.token_0_mint, data.pool_state.token_1_mint)
            }
            PoolType::RaydiumClmm(_, data) => {
                (data.pool_state.token_mint_0, data.pool_state.token_mint_1)
            }
            PoolType::Whirlpool(_, data) => {
                (data.pool_state.token_mint_a, data.pool_state.token_mint_b)
            }
            PoolType::Vertigo(_, data) => (data.pool_state.mint_a, data.pool_state.mint_b),
            PoolType::Solfi(_, data) => (data.pool_state.mint_a, data.pool_state.mint_b),
        }
    }

    #[inline]
    pub fn to_pool_type(&self) -> TokenPoolType {
        match self {
            PoolType::Meteora(_, _) => TokenPoolType::Dlmm,
            PoolType::Pump(_, _) => TokenPoolType::PumpAmm,
            PoolType::MeteoraDammv2(_, _) => TokenPoolType::Dammv2,
            PoolType::RaydiumAmm(_, _) => TokenPoolType::RaydiumAmm,
            PoolType::RaydiumCpmm(_, _) => TokenPoolType::RaydiumCpmm,
            PoolType::RaydiumClmm(_, _) => TokenPoolType::RaydiumClmm,
            PoolType::Whirlpool(_, _) => TokenPoolType::Whirlpool,
            PoolType::Vertigo(_, _) => TokenPoolType::Vertigo,
            PoolType::Solfi(_, _) => TokenPoolType::Solfi,
        }
    }
}

impl From<MeteoraDlmmData> for PoolType {
    fn from(data: MeteoraDlmmData) -> Self {
        PoolType::Meteora(data.pool_address, data)
    }
}

impl From<PumpAmmData> for PoolType {
    fn from(data: PumpAmmData) -> Self {
        PoolType::Pump(data.pool_address, data)
    }
}

impl From<MeteoraDammv2Data> for PoolType {
    fn from(data: MeteoraDammv2Data) -> Self {
        PoolType::MeteoraDammv2(data.pool_address, data)
    }
}

impl From<VertigoData> for PoolType {
    fn from(data: VertigoData) -> Self {
        PoolType::Vertigo(data.pool_address, data)
    }
}

impl From<RaydiumAmmData> for PoolType {
    fn from(data: RaydiumAmmData) -> Self {
        PoolType::RaydiumAmm(data.pool_address, data)
    }
}

impl From<RaydiumCpmmData> for PoolType {
    fn from(data: RaydiumCpmmData) -> Self {
        PoolType::RaydiumCpmm(data.pool_address, data)
    }
}

impl From<RaydiumClmmData> for PoolType {
    fn from(data: RaydiumClmmData) -> Self {
        PoolType::RaydiumClmm(data.pool_address, data)
    }
}

impl From<WhirlpoolData> for PoolType {
    fn from(data: WhirlpoolData) -> Self {
        PoolType::Whirlpool(data.pool_address, data)
    }
}

impl From<SolfiData> for PoolType {
    fn from(data: SolfiData) -> Self {
        PoolType::Solfi(data.pool_address, data)
    }
}
