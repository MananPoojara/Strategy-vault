//! R. Sullivan, A. Timmermann & H. White, "Data-Snooping, Technical Trading Rule Performance, and the
//! Bootstrap", UCSD Discussion Paper 97-31 (Dec 1997; J. Finance 1999). Book id `stwds`.
//! Source copy: Books/Ryan Sullivan/DATA-SNOOPING, TECHNICAL TRADING RULE PERFORMANCE, AND THE BOOTSTRAP.PDF.
//! Live = the best rules the paper names (Tables I-II, VI) from its rule families (Sec. III, Appendix 1),
//! each with the named parameters as defaults. The paper's 26-rule BLL universe is module `bllstr`.
//! Signal function S(chi_t) uses closes up to t and earns y_t+1 (eq. 2-3): fills at the signal day's close.
use crate::connors::{Claim, Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "stwds";
/// Admitted ids (authors/sullivan/admission/SOURCE-CONTRACT.md).
pub const REVIEWED: &[&str] = &["stwds_ma5", "stwds_ma5_b001", "stwds_ma2_b001", "stwds_obv2", "stwds_obv30_75", "stwds_sr5_b005_c5", "stwds_ch20_x075_c5", "stwds_ch200_x15_c50", "stwds_ch20_x01_c10", "stwds_filter_12_10"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const NOTE: &str = "Paper data: DJIA daily closes 1897-1996 (BLL's data) and S&P 500 futures 1984-1996; positions 1 long, 0 neutral, -1 short (p.6), no dividends, no costs. Closing prices only. India = run on the index instead of the DJIA.";
const MA_R: &[&str] = &[
    "F|\"The moving average for a particular day is calculated as the arithmetic average of prices over the previous n days, including the current day\" (fn 12, p.12)",
    "E|Gartley: \"long commitments are retained as long as the price trend remains above the moving average ... short positions are held as long as the price trend remains below the moving average\" (p.11-12): long when price (or fast MA) > slow MA x (1 + b), short when < slow MA x (1 - b)",
    "F|\"The fixed percentage band filter requires that the buy or sell signal exceed the moving average by a fixed multiplicative amount, b\" (p.12); inside the band = neutral (0, \"out of the market\", p.6)",
    "X|Position changes at the close the signal changes (exit / reverse); no stop",
];
const OBV_R: &[&str] = &[
    "F|\"The on-balance volume (OBV) indicator ... keeping a running total of the indicator each day and adding the entire amount of daily volume when the closing price increases, and subtracting the daily volume when the closing price decreases\" (p.13)",
    "E|\"We then apply a moving average of n days to the OBV indicator ... The OBV trading rules employed are the same as for the moving average trading rules, except in this case the value of interest is the OBV indicator rather than price\" (p.13): long when OBV (fast MA of OBV) > n-day MA of OBV, short when below",
    "X|Position changes at the close the signal changes; no stop",
];
const SR_R: &[&str] = &[
    "E|\"buy when the closing price exceeds the maximum price over the previous n days, and sell when the closing price is less than the minimum price over the previous n days\" (p.12), with band b: close > max x (1 + b) / close < min x (1 - b)",
    "X|\"positions can be held for a prespecified number of days, c\" (p.13); \"c = number of days a position is held, ignoring all other signals during that time\" (App. 1): exit at the close c days after entry",
];
const CH_R: &[&str] = &[
    "F|\"A channel ... can be said to occur when the high over the previous n days is within x percent of the low over the previous n days, not including the current price\" (p.13); App. 1: \"x = difference between the high price and the low price (x x high price) required to form a channel\" -> default (max - min) <= x x max (closes)",
    "E|\"buy when the closing price exceeds the channel, and to sell when the price moves below the channel\" (p.13)",
    "X|\"Long and short positions are held for a fixed number of days, c\" (p.13), other signals ignored meanwhile (App. 1)",
];
const FILTER_R: &[&str] = &[
    "E|Fama-Blume: \"If the daily closing price of a particular security moves up at least x per cent, buy and hold the security until its price moves down at least x per cent from a subsequent high, at which time simultaneously sell and go short\" (p.10-11)",
    "F|\"a subsequent high is the highest closing price achieved while holding a particular long position. Likewise, a subsequent low is the lowest closing price achieved while holding a particular short position\" (p.11)",
    "X|\"a neutral position ... liquidating a long position when the price decreases y percent from the previous high, and covering a short position when the price increases y percent from the previous low\" (p.11); x = 0.12, y = 0.10 (Table I, sub-period 5)",
];

macro_rules! d {
    ($id:expr, $name:expr, $group:expr, $status:expr, $rules:expr, $vol:expr, $claim:expr) => {
        StrategyDef { id: $id, name: $name, group: $group, session: 0, dir: "Long/Short", status: $status, needs_volume: $vol, needs_vix: false,
            needs_series: &[], book: BOOK, rules: $rules, notes: NOTE, claim: $claim }
    };
}

pub fn catalogue() -> Vec<StrategyDef> {
    let full = "Table I-II · Best rules, full universe (7,846 rules)";
    vec![
        d!("stwds_ma5", "5-day moving average (best mean return 1897-1986, 1897-1996, 1915-38)", full, Status::Live, MA_R, false,
           Some(Claim { trades: "6,310 (3,155 long / 3,155 short)", win: "2,501 / 6,310 (39.6%)", avg: "0.29% per trade; 17.2%/yr", hold: "4.3 days", period: "DJIA 1897-1996 (Table V); 1987-1996: 2.8%/yr (p.21)" })),
        d!("stwds_ma5_b001", "5-day moving average, 0.001 band (best Sharpe 1897-1986/1996, 1915-38)", full, Status::Live, MA_R, false, None),
        d!("stwds_ma2_b001", "2-day moving average, 0.001 band (best Sharpe 1939-62, 1962-86)", full, Status::Live, MA_R, false, None),
        d!("stwds_sr5_b005_c5", "5-day support & resistance, 0.005 band, 5-day hold (best mean 1897-1914)", full, Status::Live, SR_R, false, None),
        d!("stwds_obv2", "2-day on-balance volume average (best mean 1939-62, 1962-86)", full, Status::Live, OBV_R, true, None),
        d!("stwds_obv30_75", "30 and 75-day on-balance volume (best mean, S&P 500 futures 1984-96)", full, Status::Live, OBV_R, true,
           Some(Claim { trades: "43 (22 long / 21 short)", win: "22 / 43", avg: "3.00% per trade; 9.4%/yr", hold: "76.5 days", period: "S&P 500 futures 1984-1996 (Table VI)" })),
        d!("stwds_ch20_x075_c5", "20-day channel, 0.075 width, 5-day hold (best Sharpe 1897-1914)", full, Status::Live, CH_R, false, None),
        d!("stwds_ch200_x15_c50", "200-day channel, 0.150 width, 50-day hold (best Sharpe 1987-96)", full, Status::Live, CH_R, false, None),
        d!("stwds_ch20_x01_c10", "20-day channel, 0.01 width, 10-day hold (best Sharpe S&P futures 1984-96)", full, Status::Live, CH_R, false, None),
        d!("stwds_filter_12_10", "Filter rule x = 0.12, y = 0.10 (best mean 1987-96)", full, Status::Live, FILTER_R, false,
           Some(Claim { trades: "6 (4 long / 2 short)", win: "4 / 6", avg: "34.38% per trade; 14.4%/yr", hold: "411.7 days", period: "DJIA 1987-1996 (Table VI)" })),
        d!("stwds_cumwealth", "Recursive cumulative-wealth rule (follow yesterday's best rule)", "Table V-VI · Recursive rule",
           Status::NotTestable("needs the full 7,846-rule universe, whose 'e' alternative-extrema rules, time-delay filter and band/hold combinations are not specified exactly (App. 1 gives counts, not the combination logic)"),
           &["E|\"on each day ... we first determine the best-performing trading rule to date. That is, we find the rule with the greatest cumulative wealth ... and then follow the signal of that rule on the following day\" (p.18)"], false,
           Some(Claim { trades: "6,160", win: "2,476 / 6,160", avg: "0.26% per trade; 14.9%/yr", hold: "4.2 days", period: "DJIA 1897-1996 (Table V)" })),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    Some(match id {
        "stwds_ma5" => ("stwds_ma", "STW moving-average rules", "5-day MA"),
        "stwds_ma5_b001" => ("stwds_ma", "STW moving-average rules", "5-day MA · 0.001 band"),
        "stwds_ma2_b001" => ("stwds_ma", "STW moving-average rules", "2-day MA · 0.001 band"),
        "stwds_obv2" => ("stwds_obv", "STW on-balance volume rules", "2-day OBV"),
        "stwds_obv30_75" => ("stwds_obv", "STW on-balance volume rules", "30/75-day OBV"),
        "stwds_sr5_b005_c5" => ("stwds_sr", "STW support & resistance", "5-day · 0.005 band · 5-day hold"),
        "stwds_ch20_x075_c5" => ("stwds_ch", "STW channel break-outs", "20-day · 0.075 · 5-day hold"),
        "stwds_ch200_x15_c50" => ("stwds_ch", "STW channel break-outs", "200-day · 0.15 · 50-day hold"),
        "stwds_ch20_x01_c10" => ("stwds_ch", "STW channel break-outs", "20-day · 0.01 · 10-day hold"),
        "stwds_filter_12_10" => ("stwds_filter", "STW filter rules", "x 0.12 · y 0.10"),
        "stwds_cumwealth" => ("stwds_cumwealth", "STW recursive best-rule", "Not testable"),
        _ => return None,
    })
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    let ma = |f: f64, s: f64, b: f64| vec![p("fast", "Fast average, days (1 = price)", f, 1., 50., 1.), p("slow", "Moving average, days", s, 2., 250., 1.), p("band_pct", "Band b, %", b, 0., 5., 0.1)];
    let ch = |n: f64, x: f64, c: f64| vec![p("n", "Channel days", n, 5., 250., 5.), p("width_pct", "Channel width x, %", x, 0.5, 15., 0.5), p("hold", "Holding days c", c, 5., 50., 5.),
        p("width_base", "Width measured on: 0 = high (App. 1), 1 = low (text p.13)", 0., 0., 1., 1.)];
    Some(match id {
        "stwds_ma5" => ma(1., 5., 0.),
        "stwds_ma5_b001" => ma(1., 5., 0.1),
        "stwds_ma2_b001" => ma(1., 2., 0.1),
        "stwds_obv2" => ma(1., 2., 0.),
        "stwds_obv30_75" => ma(30., 75., 0.),
        "stwds_sr5_b005_c5" => vec![p("n", "Support/resistance days", 5., 5., 250., 5.), p("band_pct", "Band b, %", 0.5, 0., 5., 0.1), p("hold", "Holding days c", 5., 5., 50., 5.)],
        "stwds_ch20_x075_c5" => ch(20., 7.5, 5.),
        "stwds_ch200_x15_c50" => ch(200., 15., 50.),
        "stwds_ch20_x01_c10" => ch(20., 1., 10.),
        "stwds_filter_12_10" => vec![p("x_pct", "Filter x (initiate), %", 12., 0.5, 50., 0.5), p("y_pct", "Filter y (liquidate), %", 10., 0.5, 20., 0.5)],
        _ => return None,
    })
}

fn entry(dir: f64, i: usize, px: f64) -> Trade {
    Trade { dir, legs: vec![Leg { i, px, w: 1. }], exit_i: i, exit_px: px, reason: String::new(), entry_kind: "close", signal_i: i, open: false, exits: vec![] }
}
fn exit(out: &mut Vec<Trade>, cur: &mut Option<Trade>, i: usize, px: f64, why: String) {
    if let Some(mut t) = cur.take() { t.exit_i = i; t.exit_px = px; t.signal_i = i; t.reason = why; out.push(t); }
}
/// Strict a > b decided so that values equal in exact arithmetic (a close exactly on its average or band, missed by float
/// rounding) are equal (review 25 Oct 2026); "at least x per cent" = not strictly below.
fn gt(a: f64, b: f64) -> bool { a - b > 1e-12 * a.abs().max(b.abs()) }
fn sma(v: &[f64], n: usize, i: usize) -> f64 { v[i + 1 - n..=i].iter().sum::<f64>() / n as f64 }

/// State rule: desired position at each close (None = no change yet).
fn states(x: &Ctx, from: usize, to: usize, mut want: impl FnMut(usize) -> Option<f64>) -> Vec<Trade> {
    let (mut out, mut cur) = (vec![], None);
    for i in from..=to {
        let Some(d) = want(i) else { continue };
        if cur.as_ref().map_or(0., |t: &Trade| t.dir) == d { continue; }
        exit(&mut out, &mut cur, i, x.c[i], "Signal changed · close".into());
        if d != 0. && i < to { cur = Some(entry(d, i, x.c[i])); }
    }
    finish(out, cur, x, to)
}
/// Fixed-hold rule: a signal opens a position held `hold` days; signals during the hold are ignored.
fn held(x: &Ctx, from: usize, to: usize, hold: usize, sig: impl Fn(usize) -> f64) -> Vec<Trade> {
    let (mut out, mut cur): (Vec<Trade>, Option<Trade>) = (vec![], None);
    for i in from..=to {
        // The c holding days are t+1..t+c; a signal on the exit day t+c falls inside them and is ignored.
        if cur.as_ref().is_some_and(|t| i == t.legs[0].i + hold) { exit(&mut out, &mut cur, i, x.c[i], format!("{hold}-day holding period · close")); continue; }
        if cur.is_some() || i == to { continue; }
        let d = sig(i);
        if d != 0. { cur = Some(entry(d, i, x.c[i])); }
    }
    finish(out, cur, x, to)
}
fn finish(mut out: Vec<Trade>, cur: Option<Trade>, x: &Ctx, to: usize) -> Vec<Trade> {
    if let Some(mut t) = cur { t.exit_i = to; t.exit_px = x.c[to]; t.signal_i = to; t.reason = "open".into(); t.open = true; out.push(t); }
    out
}
fn extremes(c: &[f64], i: usize, n: usize) -> (f64, f64) {
    let w = &c[i - n..i];
    (w.iter().copied().fold(f64::MIN, f64::max), w.iter().copied().fold(f64::MAX, f64::min))
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    let schema = params(id)?;
    if from > to || to >= x.c.len() { return None; }
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).map_or(0., |d| d.def));
    let c = &x.c;
    Some(match id {
        "stwds_ma5" | "stwds_ma5_b001" | "stwds_ma2_b001" | "stwds_obv2" | "stwds_obv30_75" => {
            let (f, s, b) = (g("fast").max(1.) as usize, g("slow").max(1.) as usize, g("band_pct") / 100.);
            // OBV restarts at 0 after any bar without volume (NIFTY has none before 2000); an average is only
            // valid when its whole window lies after the last missing bar, so no volume is ever guessed.
            let mut seg = vec![0usize; c.len()];
            let v: Vec<f64> = if id.starts_with("stwds_obv") {
                if !x.has_volume { return None; }
                let mut o = vec![f64::NAN; c.len()];
                let mut start = usize::MAX;
                for i in 0..c.len() {
                    match x.r[i].volume {
                        None => { o[i] = f64::NAN; start = usize::MAX; }
                        Some(_) if start == usize::MAX => { o[i] = 0.; start = i; }
                        Some(vol) => o[i] = o[i - 1] + if c[i] > c[i - 1] { vol } else if c[i] < c[i - 1] { -vol } else { 0. },
                    }
                    seg[i] = start;
                }
                o
            } else { c.clone() };
            states(x, from, to, |i| {
                if i + 1 < f.max(s) { return None; }
                if seg[i] == usize::MAX || i + 1 - f.max(s) < seg[i] { return Some(0.); }
                let (a, m) = (sma(&v, f, i), sma(&v, s, i));
                if !(a.is_finite() && m.is_finite()) { return Some(0.); }
                // OBV can be negative: the band is measured on |MA| so "exceed by b" keeps its direction.
                let w = b * m.abs();
                Some(if gt(a, m + w) { 1. } else if gt(m - w, a) { -1. } else { 0. })
            })
        }
        "stwds_sr5_b005_c5" => {
            let (n, b, hold) = (g("n").max(1.) as usize, g("band_pct") / 100., g("hold").max(1.) as usize);
            held(x, from, to, hold, |i| {
                if i < n { return 0.; }
                let (hi, lo) = extremes(c, i, n);
                if gt(c[i], hi * (1. + b)) { 1. } else if gt(lo * (1. - b), c[i]) { -1. } else { 0. }
            })
        }
        "stwds_ch20_x075_c5" | "stwds_ch200_x15_c50" | "stwds_ch20_x01_c10" => {
            let (n, xw, hold, on_low) = (g("n").max(1.) as usize, g("width_pct") / 100., g("hold").max(1.) as usize, g("width_base") >= 0.5);
            held(x, from, to, hold, |i| {
                if i < n { return 0.; }
                let (hi, lo) = extremes(c, i, n);
                let channel = if on_low { !gt(hi, lo * (1. + xw)) } else { !gt(hi - lo, xw * hi) };
                if !channel { 0. } else if c[i] > hi { 1. } else if c[i] < lo { -1. } else { 0. }
            })
        }
        "stwds_filter_12_10" => {
            let (xf, yf) = (g("x_pct") / 100., g("y_pct") / 100.);
            let (mut pos, mut hi, mut lo) = (0f64, c[from], c[from]);
            states(x, from, to, |i| {
                if i == from { return None; }
                let px = c[i];
                match pos as i8 {
                    0 => {
                        hi = hi.max(px); lo = lo.min(px);
                        if !gt(lo * (1. + xf), px) { pos = 1.; hi = px; } else if !gt(px, hi * (1. - xf)) { pos = -1.; lo = px; }
                    }
                    1 => { hi = hi.max(px); if !gt(px, hi * (1. - yf)) { pos = 0.; lo = px; } }
                    _ => { lo = lo.min(px); if !gt(lo * (1. + yf), px) { pos = 0.; hi = px; } }
                }
                Some(pos)
            })
        }
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;
    fn rows(cl: &[f64]) -> Vec<Row> {
        cl.iter().enumerate().map(|(i, &c)| Row { ticker: "T".into(), date: format!("{:04}-01-01", 2000 + i), open: c, high: c, low: c, close: c, volume: Some(1.) }).collect()
    }
    #[test]
    fn filter_goes_neutral_on_y_and_short_needs_x_from_the_long_high() {
        // 100 -> 112 (long) -> 120 high -> 108 (-10%: neutral) -> 105.6 (-12% from 120: short) -> 116.2 (+10% from low: neutral)
        let r = rows(&[100., 112.5, 120., 107.9, 105.5, 116.5, 116.5]); let x = Ctx::new(&r, &r, &[]);
        let ts = run("stwds_filter_12_10", &x, 0, 6, &Params::new()).unwrap();
        assert_eq!(ts.len(), 2);
        assert_eq!((ts[0].dir, ts[0].legs[0].i, ts[0].exit_i), (1., 1, 3));
        assert_eq!((ts[1].dir, ts[1].legs[0].i, ts[1].exit_i), (-1., 4, 5));
    }
    #[test]
    fn channel_requires_narrow_range_and_holds_c_days() {
        let mut cl = vec![100., 101., 100.5, 100., 101.]; cl.extend([102., 103., 104., 105., 106., 107., 108., 109., 110., 111., 112.]);
        let r = rows(&cl); let x = Ctx::new(&r, &r, &[]);
        let p = Params::from([("n".into(), 5.), ("width_pct".into(), 2.), ("hold".into(), 5.)]);
        let ts = run("stwds_ch20_x075_c5", &x, 0, 15, &p).unwrap();
        assert_eq!((ts[0].legs[0].i, ts[0].exit_i), (5, 10));
    }
}
