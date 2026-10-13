//! Joseph Man-Joe Leung and Terence Tai-Leung Chong, "An empirical comparison of moving average envelopes and
//! Bollinger Bands", Applied Economics Letters 2003, 10, 339-341. Book id `mjlbb`. Source: research drive
//! "USB HDD 2/tRADING KNO-2/Joseph Man-Joe Leung - An Empirical Comparison of Moving Average Envelopes & Bollinger
//! Bands (Article)/". Daily closing index levels, long only, no transaction costs, no accumulation.
//! The paper is a grid (N = 10, 20, 50, 250; MAE k = 3%, 5%; BB k = 2 sd); every tested cell is its own strategy so
//! each keeps the paper's literal parameters and its printed Dow Jones result.
//! Bollinger SD (class C, fitted to the paper's own Table 1): the paper names the bands "Bollinger Bands with k
//! standard deviations" but prints eq. (3) as sqrt( sum_{i=t-N+1..t} [P(i) - SMA_N(i)]^2 / N ), deviations from each
//! day's own SMA. On the paper's DJIA sample the conventional population SD around SMA_N(t) gives 67/48/21/2
//! transactions for N = 10/20/50/250 (paper 68/49/25/3); eq. (3) as printed gives 38/25/11/0. Default = conventional
//! (`sd_form` = 1); the printed eq. (3) is the labelled option `sd_form` = 0.
use crate::connors::{claim, sma, Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "mjlbb";

const RULES_MAE: &[&str] = &[
    "F|eq.(1) SMA_N(t) = sum_{i=t-N+1..t} P(i) / N; eq.(2) MAE_N^k%(t) = SMA_N(t) x (1 +/- k%) (p.339)",
    "E|\"Buy : P(t-1) < MAE_low(t-1) and P(t) > MAE_low(t)\" - \"a buy signal will be generated when the price crosses the lower bound from below\" (p.340)",
    "X|\"Sell : P(t-1) > MAE_up(t-1) and P(t) < MAE_up(t)\" - \"a sell signal will be generated when the price penetrates the upper bound from above\" (p.340)",
    "F|\"short selling is not allowed, and transactions cannot be accumulated, i.e., two consecutive buying actions are not allowed\"; transaction cost and dividends negligible; return per transaction S(j)/B(j) at the daily closing index (p.340)",
];
const RULES_BB: &[&str] = &[
    "F|eq.(3) BB_N^k(t) = SMA_N(t) +/- k x sqrt( sum_{i=t-N+1..t} [P(i) - SMA_N(i)]^2 / N ) (p.339); \"Bollinger Bands with 2 standard deviations\" (p.340). Default uses the conventional SD around SMA_N(t), which reproduces Table 1 (DJIA 67/48/21/2 vs printed 68/49/25/3 transactions); eq.(3) as printed (38/25/11/0) is option sd_form = 0",
    "E|\"Buy : P(t-1) < BB_low(t-1) and P(t) > BB_low(t)\" (p.340)",
    "X|\"Sell : P(t-1) > BB_up(t-1) and P(t) < BB_up(t)\" (p.340)",
    "F|Long only, no accumulation, no costs; transactions at the daily close (p.340)",
];

struct Cell { id: &'static str, name: &'static str, bb: bool, n: f64, k: f64, dj: &'static str, trades: &'static str }

const CELLS: &[Cell] = &[
    Cell { id: "mjlbb_mae3_10", name: "MAE 10-day 3%", bb: false, n: 10., k: 3., dj: "6%", trades: "25" },
    Cell { id: "mjlbb_mae5_10", name: "MAE 10-day 5%", bb: false, n: 10., k: 5., dj: "3%", trades: "5" },
    Cell { id: "mjlbb_bb2_10", name: "Bollinger 10-day 2 sd", bb: true, n: 10., k: 2., dj: "11%", trades: "68" },
    Cell { id: "mjlbb_mae3_20", name: "MAE 20-day 3%", bb: false, n: 20., k: 3., dj: "8%", trades: "31" },
    Cell { id: "mjlbb_mae5_20", name: "MAE 20-day 5%", bb: false, n: 20., k: 5., dj: "6%", trades: "8" },
    Cell { id: "mjlbb_bb2_20", name: "Bollinger 20-day 2 sd", bb: true, n: 20., k: 2., dj: "9%", trades: "49" },
    Cell { id: "mjlbb_mae3_50", name: "MAE 50-day 3%", bb: false, n: 50., k: 3., dj: "7%", trades: "26" },
    Cell { id: "mjlbb_mae5_50", name: "MAE 50-day 5%", bb: false, n: 50., k: 5., dj: "4%", trades: "12" },
    Cell { id: "mjlbb_bb2_50", name: "Bollinger 50-day 2 sd", bb: true, n: 50., k: 2., dj: "6%", trades: "25" },
    Cell { id: "mjlbb_mae3_250", name: "MAE 250-day 3%", bb: false, n: 250., k: 3., dj: "2%", trades: "7" },
    Cell { id: "mjlbb_mae5_250", name: "MAE 250-day 5%", bb: false, n: 250., k: 5., dj: "2%", trades: "4" },
    Cell { id: "mjlbb_bb2_250", name: "Bollinger 250-day 2 sd", bb: true, n: 250., k: 2., dj: "3%", trades: "3" },
];

const NOTE: &str = "Paper tests 11 stock indices 1985-2000 (Dow Jones Industrials 1/1/1985-29/12/2000); here run on any index. Claim = the paper's Dow Jones annual rate of return [prod(1+r_j)]^(250/T) - 1 with the number of transactions (round trips) in brackets (Table 1, p.341). Long only; a buy while long and a sell while flat are ignored; the last position stays open (the paper counts completed transactions only).";

fn cell(id: &str) -> Option<&'static Cell> { CELLS.iter().find(|c| c.id == id) }

pub fn catalogue() -> Vec<StrategyDef> {
    CELLS.iter().map(|c| StrategyDef {
        id: c.id, name: c.name, group: "Leung & Chong (2003) Table 1", session: 0, dir: "Long", status: Status::Live,
        needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK, rules: if c.bb { RULES_BB } else { RULES_MAE }, notes: NOTE,
        claim: claim(c.trades, "-", c.dj, "annual rate of return (DJIA)", "1/1/1985-29/12/2000, Dow Jones Industrials"),
    }).collect()
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    let c = cell(id)?;
    Some(if c.bb { ("mjlbb_bb", "Leung-Chong Bollinger Band cross-back", c.name) } else { ("mjlbb_mae", "Leung-Chong MA envelope cross-back", c.name) })
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let c = cell(id)?;
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    let mut v = vec![p("n", "N (SMA days)", c.n, 5., 300., 5.)];
    if c.bb {
        v.push(p("k", "k (standard deviations)", c.k, 1., 3., 0.5));
        v.push(p("sd_form", "SD form: 1 = conventional around SMA_N(t) (reproduces Table 1), 0 = eq.(3) as printed", 1., 0., 1., 1.));
    } else {
        v.push(p("k", "k (% envelope)", c.k, 1., 10., 1.));
    }
    Some(v)
}

/// (lower, upper) bands per bar.
pub(crate) fn bands(c: &[f64], n: usize, k: f64, bb: bool, sd_form: u8) -> (Vec<f64>, Vec<f64>) {
    let m = sma(c, n);
    let len = c.len();
    let (mut lo, mut up) = (vec![f64::NAN; len], vec![f64::NAN; len]);
    for t in 0..len {
        if m[t].is_nan() { continue; }
        let w = if bb {
            if t + 1 < n { continue; }
            let base = |i: usize| if sd_form == 1 { m[t] } else { m[i] };
            let ss: f64 = (t + 1 - n..=t).map(|i| (c[i] - base(i)).powi(2)).sum();
            if ss.is_nan() { continue; }
            k * (ss / n as f64).sqrt()
        } else {
            m[t] * k / 100.0
        };
        lo[t] = m[t] - w;
        up[t] = m[t] + w;
    }
    (lo, up)
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    let c0 = cell(id)?;
    let n = p.get("n").copied().unwrap_or(c0.n).round().max(2.0) as usize;
    let k = p.get("k").copied().unwrap_or(c0.k);
    let sd_form = p.get("sd_form").copied().unwrap_or(1.0).round() as u8;
    let to = to.min(x.c.len().saturating_sub(1));
    let (lo, up) = bands(&x.c, n, k, c0.bb, sd_form);
    let c = &x.c;
    let mut out = Vec::new();
    let mut cur: Option<Trade> = None;
    for t in from.max(1)..=to {
        let buy = c[t - 1] < lo[t - 1] && c[t] > lo[t];
        let sell = c[t - 1] > up[t - 1] && c[t] < up[t];
        if let Some(mut q) = cur.take() {
            if sell {
                q.exit_i = t; q.exit_px = c[t]; q.signal_i = t; q.reason = "Close crosses back below the upper band".into();
                out.push(q);
            } else {
                cur = Some(q);
            }
        } else if buy {
            cur = Some(Trade { dir: 1.0, legs: vec![Leg { i: t, px: c[t], w: 1.0 }], exit_i: t, exit_px: c[t], reason: String::new(),
                entry_kind: "close", signal_i: t, open: false, exits: vec![] });
        }
    }
    if let Some(mut q) = cur {
        q.exit_i = to; q.exit_px = c[to]; q.signal_i = to; q.reason = "open".into(); q.open = true;
        out.push(q);
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn envelope_cross_back() {
        let c = vec![100., 100., 100., 96., 99., 100., 104., 102.];
        let (lo, up) = bands(&c, 3, 3., false, 0);
        // t=3: sma 98.667 lo 95.71 -> 96 above; check a manual cross: c[3]=96 vs lo[3]
        assert!((lo[2] - 97.0).abs() < 1e-9 && (up[2] - 103.0).abs() < 1e-9);
        let (lo, _) = bands(&c, 3, 2., true, 1);
        assert!((lo[2] - 100.0).abs() < 1e-9); // zero deviation
    }
}
