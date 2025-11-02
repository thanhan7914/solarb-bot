use super::*;

pub fn profitable_route(
    route: Route,
    clock: &Clock,
    min_amount_in: u64,
    max_amount_in: u64,
    epsilon: u64,
    adjust_slippage: bool,
) -> Option<SwapRoutes> {
    let token = route.start;
    let pool_vec: Vec<PoolType> = route.to_vec_owned()?;

    // let min_profit = swap_compute(clock, &pool_vec, min_amount_in, &token, false)
    //     .unwrap_or(negative_u64(min_amount_in));

    // if min_profit <= 0 {
    //     return None;
    // }

    let mut a = min_amount_in;
    let mut b = max_amount_in;
    let mut iter = 0usize;
    let max_iter = 200;

    while a < b && b - a > epsilon && iter < max_iter {
        // third = floor((b - a) / 3)
        let range = b - a;
        let third = range / 3;
        if third == 0 {
            break;
        }

        // m1 = a + third; m2 = b - third (ensure m1 < m2)
        let m1 = a + third;
        let m2 = b - third;
        if m1 >= m2 {
            break;
        }

        let f1 =
            swap_compute(clock, &pool_vec, m1, &token, adjust_slippage).unwrap_or(negative_u64(min_amount_in));
        let f2 =
            swap_compute(clock, &pool_vec, m2, &token, adjust_slippage).unwrap_or(negative_u64(min_amount_in));

        // If f1 < f2, the max is right m1 => drop [a, m1]
        // else [m2, b]
        if f1 < f2 {
            a = m1;
        } else {
            b = m2;
        }

        iter += 1;
    }

    let optimal_amount_in = adjust_amount_in(a);
    let final_profit =
        swap_compute(clock, &pool_vec, optimal_amount_in, &token, false).unwrap_or(-1);

    let (amount_in, threshold) = compute_threshold(&route.hops[0], optimal_amount_in)?;

    Some(SwapRoutes {
        routes: pool_vec,
        profit: final_profit,
        amount_in,
        threshold,
        mint: route.start,
    })
}
