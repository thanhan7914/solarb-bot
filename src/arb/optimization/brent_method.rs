use super::*;

/// Brent's method (maximize) for profit(amount_in)
pub fn profitable_route(
    route: Route,
    clock: &Clock,
    min_amount_in: u64,
    max_amount_in: u64,
    epsilon: u64,
    adjust_slippage: bool,
) -> Option<SwapRoutes> {
    const BAD: i64 = i64::MIN / 2;

    if min_amount_in == 0 || max_amount_in <= min_amount_in {
        return None;
    }

    let token = route.start;
    let pool_vec: Vec<PoolType> = route.to_vec_owned()?;

    let eval = |x_u64: u64| -> i64 {
        if x_u64 == 0 {
            return BAD;
        }
        swap_compute(clock, &pool_vec, x_u64, &token, adjust_slippage).unwrap_or(BAD)
    };

    // ---- Brent's method (maximize) on [a, b] ----
    // Convert to find minimize on g(x) = -f(x)
    let a0 = min_amount_in as f64;
    let b0 = max_amount_in as f64;

    // Brent parameters
    let phi = (1.0 + 5f64.sqrt()) / 2.0;
    let invphi = 1.0 / phi;
    let tol = epsilon.max(1) as f64;

    // initital data
    let mut a = a0;
    let mut b = b0;

    // choose x between [a,b], w,v = x
    let mut x = 0.5 * (a + b);
    let mut w = x;
    let mut v = x;

    // g(x) = -profit(x)
    let mut fx = -eval(x.round() as u64) as f64;
    let mut fw = fx;
    let mut fv = fx;

    let mut d: f64 = 0.0;
    let mut e: f64 = 0.0;

    loop {
        let m = 0.5 * (a + b);
        if (b - a) <= tol {
            break;
        }

        let mut u;
        let mut ok_parabolic = false;

        if e.abs() > 1e-12 {
            // nội suy (x,fx), (w,fw), (v,fv)
            // u = x - 0.5 * [(x-w)^2*(fx-fv) - (x-v)^2*(fx-fw)] / [(x-w)*(fx-fv) - (x-v)*(fx-fw)]
            let (xw, xv) = (x - w, x - v);
            let (fw_fv, fw_fx, fv_fx) = (fw - fv, fw - fx, fv - fx);
            let denom = 2.0 * (xw * fv_fx - xv * fw_fx);
            if denom.abs() > 1e-12 {
                let numer = (xw * xw) * fv_fx - (xv * xv) * fw_fx;
                let mut u_par = x - numer / denom;

                // u_par phải nằm trong (a,b) và di chuyển đủ tốt
                // Nếu không, fallback golden
                if u_par > a && u_par < b {
                    // giới hạn bước không quá 0.5 * e để tránh bước quá tham
                    if (u_par - x).abs() < 0.5 * e.abs() {
                        ok_parabolic = true;
                        u = u_par;
                    } else {
                        // fallback: golden section về phía tốt hơn
                        u = if x < m {
                            x + invphi * (b - x)
                        } else {
                            x - invphi * (x - a)
                        };
                    }
                } else {
                    // fallback nếu u_par ra ngoài đoạn
                    u = if x < m {
                        x + invphi * (b - x)
                    } else {
                        x - invphi * (x - a)
                    };
                }
            } else {
                // không thể nội suy parabol tin cậy
                u = if x < m {
                    x + invphi * (b - x)
                } else {
                    x - invphi * (x - a)
                };
            }
        } else {
            // chưa có e tốt → dùng golden section
            u = if x < m {
                x + invphi * (b - x)
            } else {
                x - invphi * (x - a)
            };
        }

        // Rời rạc hoá: round về u64 (ít nhất chênh 1)
        let mut u_i = u.round() as i64;
        let x_i = x.round() as i64;
        if u_i == x_i {
            // ép lệch tối thiểu 1 đơn vị về phía “nhảy”
            u_i += if u > x { 1 } else { -1 };
        }
        // clamp về [a,b]
        u_i = u_i.clamp(a.ceil() as i64, b.floor() as i64);
        let u_u64 = if u_i < 0 { 0u64 } else { u_i as u64 };

        // Đánh giá g(u) = -profit(u)
        let fu = -eval(u_u64) as f64;

        // Cập nhật cửa sổ [a,b]
        if fu <= fx {
            // u tốt hơn x (nhớ: minimize g)
            if u as f64 >= x {
                a = x
            } else {
                b = x
            }
            v = w;
            fv = fw;
            w = x;
            fw = fx;
            x = u_i as f64;
            fx = fu;
        } else {
            if u as f64 >= x {
                b = u as f64
            } else {
                a = u as f64
            }
            if fu <= fw || (w - x).abs() < f64::EPSILON {
                v = w;
                fv = fw;
                w = u_i as f64;
                fw = fu;
            } else if fu <= fv || (v - x).abs() < f64::EPSILON || (v - w).abs() < f64::EPSILON {
                v = u_i as f64;
                fv = fu;
            }
        }

        // cập nhật e cho lần sau (độ dài bước tối thiểu gần nhất)
        e = (if ok_parabolic {
            (u as f64 - x).abs()
        } else {
            invphi * (b - a)
        })
        .max(1.0);
        // d không cần thiết ở bản rời rạc, giữ lại nếu bạn muốn logging
        d = e;
    }

    let optimal_amount_in =
        adjust_amount_in(x.round().clamp(min_amount_in as f64, max_amount_in as f64) as u64);
    let final_profit = eval(optimal_amount_in);

    if final_profit <= 0 {
        return None;
    }

    let (amount_in, threshold) = compute_threshold(&route.hops[0], optimal_amount_in)?;

    Some(SwapRoutes {
        routes: pool_vec,
        profit: final_profit,
        amount_in: amount_in,
        threshold: threshold,
        mint: route.start,
    })
}
