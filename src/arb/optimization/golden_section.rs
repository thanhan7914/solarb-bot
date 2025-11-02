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

    // let min_profit = swap_compute(clock, &pool_refs, min_amount_in, &token, false)
    //     .unwrap_or(negative_u64(min_amount_in));

    // if min_profit <= 0 {
    //     return None;
    // }

    // Golden-section search MAX
    // c = b - (b-a)/phi  ~ a + 0.382*(b-a)
    // d = a + (b-a)/phi  ~ a + 0.618*(b-a)
    let phi = (1.0 + 5.0_f64.sqrt()) / 2.0;
    let invphi = 1.0 / phi; // ≈ 0.618
    let invphi2 = 1.0 - invphi; // ≈ 0.382

    let mut a = min_amount_in;
    let mut b = max_amount_in;

    let mut c = a + (((b - a) as f64) * invphi2) as u64;
    let mut d = a + (((b - a) as f64) * invphi) as u64;
    if c == d {
        // ensure d != c
        d = (d + 1).min(b);
        if c == d && d > a {
            c = d - 1;
        }
    }

    let mut fc =
        swap_compute(clock, &pool_vec, c, &token, adjust_slippage).unwrap_or(negative_u64(b));
    let mut fd =
        swap_compute(clock, &pool_vec, d, &token, adjust_slippage).unwrap_or(negative_u64(b));

    let mut iters = 0usize;
    let max_iters = 128; 
    // Loop until b - a <= epsilon
    while b - a > epsilon && iters < max_iters {
        iters += 1;
        if fc < fd {
            // Use the left to c
            a = c;
            c = d;
            fc = fd;

            // re-calculate new d (reuse c)
            d = a + (((b - a) as f64) * invphi) as u64;
            if d <= c {
                d = (c + 1).min(b);
            }
            fd = swap_compute(clock, &pool_vec, d, &token, adjust_slippage)
                .unwrap_or(negative_u64(b));
        } else {
            // Use the left to d
            b = d;
            d = c;
            fd = fc;

            // re-calculate new c (reuse d)
            c = a + (((b - a) as f64) * invphi2) as u64;
            if c >= d {
                c = d.saturating_sub(1).max(a);
            }
            fc = swap_compute(clock, &pool_vec, c, &token, adjust_slippage)
                .unwrap_or(negative_u64(b));
        }

        if b <= a || b - a <= epsilon {
            break;
        }
    }

    let optimal_amount_in = adjust_amount_in(a);
    let final_profit =
        swap_compute(clock, &pool_vec, optimal_amount_in, &token, false).unwrap_or(-1);

    let (amount_in, threshold) = compute_threshold(&route.hops[0], optimal_amount_in)?;

    Some(SwapRoutes {
        routes: pool_vec,
        profit: final_profit,
        amount_in: amount_in,
        threshold: threshold,
        mint: route.start,
    })
}
