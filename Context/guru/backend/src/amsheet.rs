//! The 36 AmiBroker result-list columns of one optimization combination, in the order of the research team's optimization sheet
//! (Results-feature handoff section 4.4). Unit mode: initial capital 1, so the nine money columns stay blank (NaN) exactly as the team's file leaves them;
//! every other column is in AmiBroker units (percent numbers). Definitions are the "report" set: 365-day years, the drawdown peak starts at the first
//! bar, an open trade is included and marked at the last close, exposure runs from the entry bar to the bar before the exit, bars held count both the
//! entry and the exit bar, a trade's profit is the equity before it times its return, Sharpe of trades rf 5 %, Ulcer performance rf 5.4 %.
//! Mirrors frontend/react/src/compat/report-stats.js, which draws the same numbers on the page.

pub const COLUMNS: usize = 36;
/// starting money of the money columns (same as the report's `capital`)
pub const CAPITAL: f64 = 1_000_000.0;

/// one closed or open trade: bar numbers are relative to the first bar of the run window
pub struct TradeIn {
    pub dir: f64,
    pub entry: usize,
    pub exit: usize,
    pub ret: f64,
    pub avg_px: f64,
    pub exit_px: f64,
    /// stock-universe runs: the trade's own profit as a share of the starting equity and its weight of equity (0 / None = single-instrument run, compounded from `ret`)
    pub profit: Option<f64>,
    pub weight: f64,
}

/// days since 1970-01-01 of "YYYY-MM-DD"
fn days(d: &str) -> f64 {
    let (y, m, dd): (i64, i64, i64) = (d[0..4].parse().unwrap_or(1970), d[5..7].parse().unwrap_or(1), d[8..10].parse().unwrap_or(1));
    let (y, m) = if m <= 2 { (y - 1, m + 9) } else { (y, m - 3) };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let doy = (153 * m + 2) / 5 + dd - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    (era * 146097 + doe - 719468) as f64
}

fn mean(v: &[f64]) -> f64 {
    if v.is_empty() { f64::NAN } else { v.iter().sum::<f64>() / v.len() as f64 }
}

/// `dates` and `close` and `equity` hold one value per bar of the run window; returns COLUMNS values (NaN = blank cell)
pub fn row(dates: &[&str], close: &[f64], equity: &[f64], trades: &[TradeIn]) -> Vec<f64> {
    let nan = f64::NAN;
    let n = equity.len();
    if n < 3 || dates.len() < n || trades.is_empty() {
        let mut v = vec![nan; COLUMNS];
        v[20] = trades.len() as f64;
        return v;
    }
    let years = (days(dates[n - 1]) - days(dates[0])) / 365.0;
    let net = equity[n - 1] / equity[0] - 1.0;
    let cagr = if years > 0.0 { (equity[n - 1] / equity[0]).powf(1.0 / years) - 1.0 } else { nan };
    let exposure = trades.iter().map(|t| (t.exit.saturating_sub(t.entry)) as f64 * if t.weight > 0.0 { t.weight } else { 1.0 }).sum::<f64>() / n as f64;
    // equity curve: drawdown, Ulcer, regression of equity on the bar number
    let (mut peak, mut max_dd, mut max_abs, mut u2) = (equity[0], 0.0f64, 0.0f64, 0.0f64);
    for &e in equity {
        peak = peak.max(e);
        let dd = e / peak - 1.0;
        max_dd = max_dd.min(dd);
        max_abs = max_abs.max(peak - e);
        u2 += (100.0 * dd).powi(2);
    }
    let nf = n as f64;
    let (xm, ym) = ((nf - 1.0) / 2.0, equity.iter().sum::<f64>() / nf);
    let (mut sxx, mut sxy) = (0.0, 0.0);
    for (k, &e) in equity.iter().enumerate() {
        sxx += (k as f64 - xm).powi(2);
        sxy += (k as f64 - xm) * (e - ym);
    }
    let slope = sxy / sxx;
    let icpt = ym - slope * xm;
    let sse: f64 = equity.iter().enumerate().map(|(k, &e)| (e - (icpt + slope * k as f64)).powi(2)).sum();
    let se = (sse / (nf - 2.0)).sqrt();
    let bars_per_year = if years > 0.0 { nf / years } else { nan };
    let ulcer = (u2 / nf).sqrt();
    let rar = cagr / exposure;
    let abs_dd = max_dd.abs();

    // trades: profit = equity before the trade times its return; bars held count entry and exit bar
    let mut growth = 1.0;
    let (mut win_p, mut loss_p) = (0.0, 0.0);
    let (mut win_r, mut loss_r, mut win_b, mut loss_b, mut all_b) = (Vec::new(), Vec::new(), Vec::new(), Vec::new(), Vec::new());
    let mut rets = Vec::new();
    let mut max_trade_dd = 0.0f64;
    for t in trades {
        let profit = t.profit.unwrap_or(growth * t.ret);
        if t.profit.is_none() { growth *= 1.0 + t.ret; }
        let held = (t.exit - t.entry + 1) as f64;
        all_b.push(held);
        rets.push(t.ret * 100.0);
        if t.ret > 0.0 { win_p += profit; win_r.push(t.ret); win_b.push(held); } else { loss_p += profit; loss_r.push(t.ret); loss_b.push(held); }
        // worst peak-to-valley inside the trade: entry fill, the closes up to the bar before the exit, the exit fill
        let mark = |p: f64| if t.dir > 0.0 { p / t.avg_px } else { 2.0 - p / t.avg_px };
        let (mut pk, mut dd) = (mark(t.avg_px), 0.0f64);
        // without the instrument's closes (a stock universe) only the two fills are known
        let path = std::iter::once(t.avg_px).chain((t.entry..t.exit.min(close.len())).map(|k| close[k])).chain(std::iter::once(t.exit_px));
        for p in path {
            let m = mark(p);
            pk = pk.max(m);
            dd = dd.min(m / pk - 1.0);
        }
        max_trade_dd = max_trade_dd.min(dd);
    }
    let nt = trades.len() as f64;
    let held = mean(&all_b);
    let sd = { let m = mean(&rets); (rets.iter().map(|r| (r - m).powi(2)).sum::<f64>() / rets.len() as f64).sqrt() };
    let sharpe = if sd > 0.0 && held > 0.0 && bars_per_year.is_finite() {
        let f = bars_per_year / held;
        (mean(&rets) * f - 5.0) / (sd * f.sqrt())
    } else { nan };
    let profit_factor = if loss_p < 0.0 { win_p / -loss_p } else if win_r.is_empty() { 0.0 } else { nan };
    let payoff = if !win_r.is_empty() && !loss_r.is_empty() && loss_p < 0.0 { (win_p / win_r.len() as f64) / (-loss_p / loss_r.len() as f64) } else { nan };
    let pct = |c: usize| 100.0 * c as f64 / nt;
    vec![
        CAPITAL * net,                                              // Net Profit (money)
        100.0 * net, 100.0 * exposure, 100.0 * cagr, 100.0 * rar,    // Net % Profit, Exposure %, CAR, RAR
        nan, 100.0 * max_trade_dd,                                    // Max. Trade Drawdown (money), Max. Trade % Drawdown
        -CAPITAL * max_abs, 100.0 * max_dd,                          // Max. Sys Drawdown (money), Max. Sys % Drawdown
        if max_abs > 0.0 { net / max_abs } else { nan },              // Recovery Factor
        cagr / abs_dd, rar / abs_dd,                                  // CAR/MDD, RAR/MDD
        profit_factor, payoff,
        CAPITAL * se,                                               // Standard Error (money)
        slope * bars_per_year / se, ulcer, (100.0 * cagr - 5.4) / ulcer, sharpe, slope * sxx.sqrt() / se / nf, // RRR, Ulcer, UPI, Sharpe, K-Ratio
        nt,
        CAPITAL * net / nt, mean(&rets), held,                        // Avg Profit/Loss (money), Avg % Profit/Loss, Avg Bars Held
        win_r.len() as f64, pct(win_r.len()), CAPITAL * win_p, if win_r.is_empty() { nan } else { CAPITAL * win_p / win_r.len() as f64 }, 100.0 * mean(&win_r), mean(&win_b),   // winners: #, %, total, avg (money), avg %, bars
        loss_r.len() as f64, pct(loss_r.len()), CAPITAL * loss_p, if loss_r.is_empty() { nan } else { CAPITAL * loss_p / loss_r.len() as f64 }, 100.0 * mean(&loss_r), mean(&loss_b), // losers
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn t(entry: usize, exit: usize, entry_px: f64, exit_px: f64, ret: f64) -> TradeIn {
        TradeIn { dir: 1.0, entry, exit, ret, avg_px: entry_px, exit_px, profit: None, weight: 0.0 }
    }

    #[test]
    fn matches_the_hand_computed_report_example() {
        // same 8 bars and 3 trades (+10 %, -5 %, +20 %) as the frontend test in tests/report-stats.test.ts
        let dates = ["2020-01-01", "2020-01-02", "2020-01-03", "2020-01-06", "2020-01-07", "2020-01-08", "2020-01-09", "2020-01-10"];
        let close = [100.0, 110.0, 108.0, 100.0, 95.0, 120.0, 114.0, 120.0];
        let equity = [1.0, 1.1, 1.1, 1.1, 1.045, 1.045, 1.045, 1.254];
        let trades = [t(0, 1, 100.0, 110.0, 0.1), t(2, 4, 108.0, 102.6, -0.05), t(5, 7, 100.0, 120.0, 0.2)];
        let r = row(&dates, &close, &equity, &trades);
        assert_eq!(r.len(), COLUMNS);
        assert!((r[1] - 25.4).abs() < 1e-9, "net % {}", r[1]);
        assert!((r[2] - 100.0 * 5.0 / 8.0).abs() < 1e-9, "exposure {}", r[2]);
        assert!((r[8] - 100.0 * (1.045 / 1.1 - 1.0)).abs() < 1e-9, "max sys dd {}", r[8]);
        assert!((r[12] - (0.1 + 1.045 * 0.2) / (1.1 * 0.05)).abs() < 1e-9, "profit factor {}", r[12]);
        assert_eq!(r[20], 3.0);
        assert!((r[22] - 100.0 * (0.1 - 0.05 + 0.2) / 3.0).abs() < 1e-9, "avg % {}", r[22]);
        assert!((r[23] - (2.0 + 3.0 + 3.0) / 3.0).abs() < 1e-9, "avg bars {}", r[23]);
        assert_eq!((r[24], r[30]), (2.0, 1.0));
        assert!((r[25] - 200.0 / 3.0).abs() < 1e-9);
        assert!((r[6] - 100.0 * (100.0 / 108.0 - 1.0)).abs() < 1e-9, "max trade dd {}", r[6]);
        // money columns = capital x the per-1 result; only Max. Trade Drawdown (money) stays blank
        assert!(r[5].is_nan());
        assert!((r[0] - CAPITAL * 0.254).abs() < 1e-6, "net profit {}", r[0]);
        assert!((r[21] - CAPITAL * 0.254 / 3.0).abs() < 1e-6, "avg profit {}", r[21]);
        assert!((r[26] - CAPITAL * (0.1 + 1.045 * 0.2)).abs() < 1e-6, "winners total {}", r[26]);
        assert!((r[27] - CAPITAL * (0.1 + 1.045 * 0.2) / 2.0).abs() < 1e-6, "winners avg {}", r[27]);
        assert!((r[32] - CAPITAL * (-1.1 * 0.05)).abs() < 1e-6, "losers total {}", r[32]);
        assert!((r[26] + r[32] - r[0]).abs() < 1e-6, "winners + losers = net profit");
        assert!((r[7] + CAPITAL * (1.1 - 1.045)).abs() < 1e-6, "max sys drawdown {}", r[7]);
    }

    #[test]
    fn a_day_count_is_calendar_days() {
        assert_eq!(days("2020-01-03") - days("2020-01-01"), 2.0);
        assert_eq!(days("2021-01-01") - days("2020-01-01"), 366.0);
    }

    #[test]
    fn no_trades_gives_blank_columns_and_zero_trades() {
        let r = row(&["2020-01-01", "2020-01-02", "2020-01-03"], &[1.0, 1.0, 1.0], &[1.0, 1.0, 1.0], &[]);
        assert_eq!(r[20], 0.0);
        assert!(r[1].is_nan());
    }
}
