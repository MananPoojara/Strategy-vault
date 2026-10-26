//! Active Trader Magazine, 2001 volume: the monthly "Trading System Lab" columns (Mar-Dec 2001; the Jan/Feb 2001 issue
//! is not on the research drive). Book id `atm01`. Source copies: Books/Active Trader Magazine/2001/tradingsystemslabMM01.djvu
//! (+ 2001 06/08/11 full issues). Transcription, testability table and checks: library/atm01/README.md.
//!
//! Engine and fill conventions: atm00 `sim` (levels at the level or the open on a gap; price exits from the bar after
//! entry; stop before target; nearer-the-open entry when both sides fill on one bar). Columns tested the 30 Dow or 30
//! Nasdaq-100 stocks in one account with fixed-fractional sizing; the vault runs one index/stock at a time, full
//! equity per trade (sizing noted, not modelled). India: NIFTY 50, or basket:nifty50:<SYMBOL> for the stock systems.
//! Duplicates already in the vault (not repeated): Relative Strength Bands (Mar 2001) = strmm_rsbands_orig; Rotation
//! (May 2001) = strmm_rotation_orig; Harris 3L-R variation with 4%/12% exits (Nov 2001) = strmm_harris_orig.
use super::atm00::{hi_n, levels, lo_n, sim, Ord};
use crate::connors::{sma, Claim, Ctx, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "atm01";
/// Admitted ids (authors/activetrader2/admission/SOURCE-CONTRACT.md).
pub const REVIEWED: &[&str] = &["atm01_five_for_five", "atm01_old_faithful", "atm01_vwa", "atm01_bad_compromise", "atm01_td_carrie", "atm01_deep_pockets"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

macro_rules! def {
    ($id:expr, $name:expr, $g:expr, $dir:expr, $st:expr, $vol:expr, $rules:expr, $notes:expr, $claim:expr) => {
        StrategyDef { id: $id, name: $name, group: $g, session: 0, dir: $dir, status: $st, needs_volume: $vol, needs_vix: false,
            needs_series: &[], book: BOOK, rules: $rules, notes: $notes, claim: $claim }
    };
}
fn cl(t: &'static str, w: &'static str, a: &'static str, d: &'static str, p: &'static str) -> Option<Claim> {
    Some(Claim { trades: t, win: w, avg: a, hold: d, period: p })
}

/// TradeStation AvgPrice = (O + H + L + C) / 4 (the Dec 2001 column defines the day's "average price" this way).
fn avg_price(x: &Ctx) -> Vec<f64> { (0..x.c.len()).map(|i| (x.o[i] + x.h[i] + x.l[i] + x.c[i]) / 4.0).collect() }

/// Meander boundaries computed on bar i (Oct 2000 EasyLanguage): the n x 4 percentage changes of O, H, L, C of the last
/// n bars from each previous close; mean and population SD; level = C x (1 + mean -/+ k x SD) for the next bar.
fn meander(x: &Ctx, n: usize, k: f64) -> (Vec<f64>, Vec<f64>) {
    let len = x.c.len();
    let (mut lo, mut hi) = (vec![f64::NAN; len], vec![f64::NAN; len]);
    for i in n..len {
        let mut v = Vec::with_capacity(4 * n);
        for j in 0..n {
            let (b, pc) = (i - j, x.c[i - j - 1]);
            for p in [x.o[b], x.h[b], x.l[b], x.c[b]] { v.push((p - pc) / pc); }
        }
        let m = v.iter().sum::<f64>() / v.len() as f64;
        let sd = (v.iter().map(|z| (z - m).powi(2)).sum::<f64>() / v.len() as f64).sqrt();
        lo[i] = x.c[i] * (1.0 + m - k * sd);
        hi[i] = x.c[i] * (1.0 + m + k * sd);
    }
    (lo, hi)
}

/// July 2001 volume-weighted average. AV = n-day average volume, RV = (AV - LAV) / (HAV - LAV) over the same n days;
/// printed formula VWA = (1 - RV) x C + RV x PVWA (alt = true: the column's other wording RV x C + (1 - RV) x PVWA).
/// Recursion starts from PVWA = 0 at the first bar with RV defined; HAV = LAV leaves VWA unchanged (book silent).
fn vwa(x: &Ctx, n: usize, alt: bool) -> Vec<f64> {
    let vol: Vec<f64> = x.r.iter().map(|r| r.volume.unwrap_or(f64::NAN)).collect();
    let av = sma(&vol, n);
    let len = x.c.len();
    let mut out = vec![f64::NAN; len];
    let mut prev = f64::NAN;
    for i in 0..len {
        if i + 1 < 2 * n - 1 || av[i + 1 - n..=i].iter().any(|v| !v.is_finite()) { prev = f64::NAN; continue; }
        let w = &av[i + 1 - n..=i];
        let (hv, lv) = (w.iter().cloned().fold(f64::MIN, f64::max), w.iter().cloned().fold(f64::MAX, f64::min));
        let p = if prev.is_finite() { prev } else { 0.0 };
        out[i] = if hv > lv {
            let rv = (av[i] - lv) / (hv - lv);
            if alt { rv * x.c[i] + (1.0 - rv) * p } else { (1.0 - rv) * x.c[i] + rv * p }
        } else if prev.is_finite() { prev } else { f64::NAN };
        prev = out[i];
    }
    out
}

pub fn catalogue() -> Vec<StrategyDef> {
    use Status::*;
    vec![
        def!("atm01_five_for_five", "Five for five (Apr 2001 TSL)", "Trading System Lab · Apr 2001", "Long", Live, false,
            &["F|\"1. Prepare to go long if yesterday's 100-day moving average is higher than it was five days ago.\" (TSL p.110)",
              "E|\"2. Go long with a stop order at yesterday's high if: 1) yesterday's high and low are higher than the previous day's high and low, respectively; 2) yesterday's close is higher than yesterday's open and 3) today's open is higher than yesterday's close.\" (TSL p.110)",
              "X|\"1. (Stop-loss) On the entry day, exit with a loss on the close if the market closes below the lowest low of the previous two days. 2. On the entry day, exit with a profit on the close if the market closes above the highest high of the previous two days by an amount equal to the distance between the entry point and the lowest low of the previous two days.\" (TSL p.110)",
              "X|\"3. For all other days: a. Exit with a trailing stop (win or loss) at the lowest low of the previous two days, or b. Exit with a limit order at the highest high of the previous two days by an amount equal to the distance between the entry point and the lowest low of the two days leading into the trade.\" (TSL p.110)",
              "X|\"it is necessary to be conservative and assume the market initially moved against the position, triggering the trailing stop order before the profit-taking limit order\" (TSL p.111)"],
            "Book: 30 Dow stocks, Jul 1991-Dec 2000, 2% risk, $20 costs: 2,849 trades, 42% winners, avg $286, 4.2 days (TSL p.111). India: NIFTY 50 / basket:nifty50 stocks.",
            cl("2,849", "42%", "$286", "4.2 days", "30 Dow stocks 7/1991-12/2000")),
        def!("atm01_old_faithful", "Old faithful: MAS trend + Meander entry (Jun 2001 TSL)", "Trading System Lab · Jun 2001", "Long/Short", Live, false,
            &["F|\"The MAS indicator is the difference between today's 80-day moving average value and its value 11 days ago. When today's moving average value is higher than it was 11 days ago, the trend is considered to be up\" (TSL Jun 2001 p.66)",
              "F|Meander (Oct 2000 p.31 EasyLanguage): VS = (O, H, L, C of each of the last 5 bars - previous close) / previous close (20 values); VSLow/VSHigh = C x (1 + AvgVS -/+ StdVS x 1), StdVS = population SD, plotted one bar ahead",
              "E|\"1. Go long if the MAS indicator signals an uptrend and the price crosses below the one standard deviation boundary of the Meander indicator (an oversold signal). 2. Go short if the MAS indicator signals a downtrend and the price crosses above the one standard deviation boundary\" (TSL Jun 2001 p.66) - limit order at yesterday's boundary (open beyond it = fill at the open); MAS of yesterday's close",
              "X|\"3. Exit all trades on the close after two days, counting the day for the entry as the first day in the trade, no matter how late in the day the entry took place.\" (TSL Jun 2001 p.66)"],
            "Book: six spot currencies vs USD, Jan 1995-Mar 2001, no costs, results in %: 1,653 trades, 55.0% winners, avg 0.05%, 2.0 days (TSL Jun 2001 p.67). Not an index; not compared.", None),
        def!("atm01_vwa", "Volume-weighted average reversal (Jul 2001 TSL)", "Trading System Lab · Jul 2001", "Long/Short", Live, true,
            &["F|\"RV=(AV-LAV)/(HAV-LAV)\" with AV = average volume over the nine-day look-back, HAV/LAV its highest/lowest over the same nine days; \"VWA=(1-RV)*C+RV*PVWA\" (TSL p.72) - p.formula=1 uses the column's other wording RV*C+(1-RV)*PVWA (p.73)",
              "E|\"Prepare to go long if today's closing price is below VWA and if VWA is lower today than yesterday. Prepare to go short if today's closing price is above VWA and if VWA is greater today than yesterday. Enter long tomorrow if the market trades above the highest high of the previous two days ... Enter short tomorrow if the market trades below the lowest low of the previous two days.\" (TSL p.72)",
              "X|\"Exit all trades: 1) on the close, if the market does not follow through on its move that triggered the trade by also closing above (below for short trades) the VWA; 2) if the market moves against you more than 4 percent.\" (TSL p.72) - close test applied every day incl. the entry day",
              "F|\"we took the liberty of defining the trend using hindsight and allowed only long trades up until April 1990, and only short trades thereafter\" (TSL p.73) - default; p.side: 1 both, 2 long only, 3 short only"],
            "Book: 30 Dow stocks, Sep 1991-Apr 2001, 1% risk, $20 costs: 2,740 trades, 39% winners, avg $74, 4.0 days (TSL p.73). Needs volume (SPX has it; check NIFTY 50 coverage).",
            cl("2,740", "39%", "$74", "4.0 days", "30 Dow stocks 9/1991-4/2001")),
        def!("atm01_bad_compromise", "A bad compromise: inside-average consolidation (Aug 2001 TSL)", "Trading System Lab · Aug 2001", "Long/Short", Live, false,
            &["F|\"1. Prepare to go long if today's high and low are inside the average high and low prices of the last five days and average price of the last five days is above the average price of the preceding five days. 1a. [short] ... below\" (TSL p.62)",
              "E|\"2. Enter long tomorrow if the market opens above today's closing price. 2a. Enter short tomorrow if the market opens below today's closing price.\" (TSL p.62)",
              "X|\"3. Exit long trades if the market trades below the average low of the last five days. 3a. Exit short trades if the market trades above the average high of the last five days. 4. Exit all trades if the market moves against the position more than 4 percent.\" (TSL p.62)",
              "F|Read: \"average price\" = TradeStation AvgPrice (O+H+L+C)/4 as the Dec 2001 column defines the day's average price (p.ap_close=1: close); averages as of the prior close for the exit levels"],
            "Book: 30 Dow stocks, Sep 1991-Apr 2001, 5% risk, $20 costs: 901 trades, 32% winners, avg -$10, 6.3 days (TSL p.63).",
            cl("901", "32%", "-$10", "6.3 days", "30 Dow stocks 9/1991-4/2001")),
        def!("atm01_td_carrie", "DeMark variation: TD Carrie with 4%/12%/5-day exits (Sep 2001 TSL)", "Trading System Lab · Sep 2001", "Long/Short", Live, false,
            &["E|\"1. Prepare to go long today if a. the true high from four days ago is higher than the high from either two, three or five days ago, and b. yesterday's close is lower than the close two days ago, and c. today's open is lower than the high four days ago.\" (TSL p.100) - true high = max(high, previous close)",
              "E|\"2. Prepare to go short today if a. the true low from four days ago is lower than the low from either two, three or five days ago, and b. yesterday's close is higher than the close two days ago, and c. today's open is higher than the low four days ago\" (TSL p.100)",
              "E|\"3. Go long today with a stop order at the true high of four days ago, plus 0.1 percent. 4. Go short today with a stop order at the true low of four days ago, minus 0.1 percent.\" (TSL p.100)",
              "X|\"6. Exit all trades with a loss if the market moves against the position by 4 percent or more. 7. Exit all trades with a profit if the market moves in favor of the position by 12 percent or more. 8. Exit all trades after five days, counting the day for the entry as day one\" (TSL p.100)",
              "E|Rule c uses the plain high/low four days ago (rule text); p.open_true=1 uses the true high/low (logic paragraph wording)"],
            "Book: 30 Nasdaq-100 stocks, Nov 1991-Jun 2001, 2% risk, $10 costs: 3,529 trades, 39.4% winners, avg $158, 3.0 days (TSL p.101). TD Carrie itself is listed (not testable, no exits) under tdnmtt_systems; these exits are the column's own.",
            cl("3,529", "39.4%", "$158", "3.0 days", "30 Nasdaq-100 stocks 11/1991-6/2001")),
        def!("atm01_beta", "Beta value trend system (Oct 2001 TSL)", "Trading System Lab · Oct 2001", "Long/Short",
            NotTestable("the beta calculation (lookback, return interval, which index) is not given - only \"the beta calculations are totally unoptimized\" (TSL Oct 2001 p.82); the stock's beta also needs the index series beside each stock"), false,
            &["E|\"1. Prepare to go long today if the broader market is in an uptrend, as indicated by the appropriate index trading above its 63-day (three-month) moving average, and the stock has a beta greater than 1. 2. [short] ... below its 63-day moving average, and the stock has a beta below 1. 3. Go long today with a limit order at the lowest low of the previous five days. 4. Go short today with a limit order at the highest high of the previous five days.\" (TSL Oct 2001 p.82)",
              "X|\"5. Exit all trades with a loss if the market moves against the position four percent or more. 6. Exit all trades with a profit if the market moves in favor of the position 12 percent or more.\" (TSL Oct 2001 p.82)"],
            "Book: 30 Nasdaq-100 stocks, Dec 1991-Jul 2001: 2,716 trades, 39.4% winners, avg $3.", None),
        def!("atm01_deep_pockets", "Deep pockets: average-price reversal (Dec 2001 TSL)", "Trading System Lab · Dec 2001", "Long/Short", Live, false,
            &["F|\"Typical price is defined as (Open+High+Low+Close)/4\" (TSL Dec 2001 p.58)",
              "E|\"1. Go long tomorrow if the market opens below today's average price and then trades above it, provided that today's average price is lower than the average price five days ago and lower than today's close (reverse the rules for short trades).\" (TSL Dec 2001 p.58)",
              "X|\"3. Exit all trades with a profit or loss if the market trades below the previous day's low. 4. Exit all trades with a profit if the market trades above the highest high of the last five days. 5. Exit all remaining trades after five days, counting the day of entry as day one.\" (TSL Dec 2001 p.58)"],
            "Book: 30 Nasdaq-100 stocks, Feb 1992-Sep 2001, 0.25% risk, $20 costs, $1 million start: 12,720 trades, 37.9% winners, avg $228, 1.6 days (TSL Dec 2001 p.59).",
            cl("12,720", "37.9%", "$228", "1.6 days", "30 Nasdaq-100 stocks 2/1992-9/2001")),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    let d = catalogue().into_iter().find(|d| d.id == id)?;
    Some(("atm01_tsl", "Active Trader · Trading System Lab 2001", d.name))
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    Some(match id {
        "atm01_five_for_five" => vec![p("ma", "Trend MA days", 100., 20., 300., 10.), p("slope", "MA compared with days earlier", 5., 1., 20., 1.),
            p("n", "Stop / target lookback days", 2., 1., 10., 1.)],
        "atm01_old_faithful" => vec![p("ma", "MAS moving average days", 80., 20., 200., 5.), p("lag", "MAS lag days", 11., 1., 40., 1.),
            p("bars", "Meander bars (x4 prices)", 5., 2., 20., 1.), p("sd", "Meander standard deviations", 1., 0.5, 3., 0.25),
            p("hold", "Exit on close of day (entry day = 1)", 2., 1., 10., 1.)],
        "atm01_vwa" => vec![p("n", "VWA look-back days", 9., 3., 40., 1.), p("trig", "Breakout days", 2., 1., 10., 1.), p("stop_pct", "Stop loss %", 4., 1., 10., 0.5),
            p("side", "Direction: 0 book hindsight (long to Apr 1990, short after), 1 both, 2 long, 3 short", 0., 0., 3., 1.),
            p("formula", "VWA: 0 printed (1-RV)*C+RV*PVWA, 1 RV*C+(1-RV)*PVWA", 0., 0., 1., 1.)],
        "atm01_bad_compromise" => vec![p("n", "Average days", 5., 2., 20., 1.), p("stop_pct", "Stop loss %", 4., 1., 10., 0.5),
            p("ap_close", "Average price: 0 (O+H+L+C)/4, 1 close", 0., 0., 1., 1.)],
        "atm01_td_carrie" => vec![p("pct", "Breakout beyond true high/low %", 0.1, 0., 1., 0.05), p("stop_pct", "Stop loss %", 4., 1., 10., 0.5),
            p("tgt_pct", "Profit target %", 12., 2., 30., 1.), p("hold", "Exit on close of day (entry day = 1)", 5., 1., 20., 1.),
            p("open_true", "Rule c reference: 0 high/low 4 days ago, 1 true high/low", 0., 0., 1., 1.)],
        "atm01_deep_pockets" => vec![p("lag", "Average price compared with days ago", 5., 1., 20., 1.), p("hh", "Profit exit: highest high of days", 5., 1., 20., 1.),
            p("hold", "Exit on close of day (entry day = 1)", 5., 1., 20., 1.)],
        _ => return None,
    })
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !id.starts_with("atm01_") { return None; }
    let schema = params(id)?;
    if from >= to || to >= x.r.len() { return Some(vec![]); }
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).map_or(f64::NAN, |d| d.def));
    let u = |k: &str| g(k).round().max(1.0) as usize;
    let (c, o, h, l) = (&x.c, &x.o, &x.h, &x.l);
    Some(match id {
        "atm01_five_for_five" => {
            let (m, lag, n) = (sma(c, u("ma")), u("slope"), u("n"));
            sim(x, from, to, &|t| {
                if t < lag + n + 1 { return vec![]; }
                let y = t - 1;
                if m[y] > m[y - lag] && h[y] > h[y - 1] && l[y] > l[y - 1] && c[y] > o[y] && o[t] > c[y] { vec![(1.0, Ord::Stop(h[y]))] } else { vec![] }
            }, &|t, q| {
                let d = q.epx - lo_n(x, q.ei - 1, n); // entry minus lowest low of the days leading into the trade
                if t == q.ei {
                    if c[t] < lo_n(x, t - 1, n) { return Some((c[t], "entry-day close below 2-day low")); }
                    if c[t] > hi_n(x, t - 1, n) + d { return Some((c[t], "entry-day close above 2-day high + risk")); }
                    return None;
                }
                levels(x, t, 1.0, Some(lo_n(x, t - 1, n)), Some(hi_n(x, t - 1, n) + d))
            })
        }
        "atm01_old_faithful" => {
            let m = sma(c, u("ma"));
            let lag = u("lag");
            let (vlo, vhi) = meander(x, u("bars"), g("sd"));
            let hold = u("hold");
            sim(x, from, to, &|t| {
                let y = t - 1;
                if y < lag || !m[y - lag].is_finite() { return vec![]; }
                let mas = m[y] - m[y - lag];
                if mas > 0.0 { vec![(1.0, Ord::Limit(vlo[y]))] } else if mas < 0.0 { vec![(-1.0, Ord::Limit(vhi[y]))] } else { vec![] }
            }, &|t, q| (t + 1 - q.ei >= hold).then(|| (c[t], "time exit on close")))
        }
        "atm01_vwa" => {
            let (n, trig, sp) = (u("n"), u("trig"), g("stop_pct") / 100.);
            let side = g("side").round() as i64;
            let w = vwa(x, n, g("formula") >= 0.5);
            let allow = |s: usize, d: f64| match side {
                0 => if x.r[s].date.as_str() < "1990-04-01" { d > 0.0 } else { d < 0.0 },
                1 => true, 2 => d > 0.0, _ => d < 0.0,
            };
            sim(x, from, to, &|t| {
                let s = t - 1;
                if s < trig || !(w[s].is_finite() && w[s - 1].is_finite()) { return vec![]; }
                let mut v = vec![];
                if c[s] < w[s] && w[s] < w[s - 1] && allow(s, 1.0) { v.push((1.0, Ord::Stop(hi_n(x, s, trig)))); }
                if c[s] > w[s] && w[s] > w[s - 1] && allow(s, -1.0) { v.push((-1.0, Ord::Stop(lo_n(x, s, trig)))); }
                v
            }, &|t, q| {
                if t > q.ei { if let Some(e) = levels(x, t, q.dir, Some(q.epx * (1.0 - q.dir * sp)), None) { return Some(e); } }
                let fail = if q.dir > 0.0 { !(c[t] > w[t]) } else { !(c[t] < w[t]) };
                fail.then(|| (c[t], "no follow-through close vs VWA"))
            })
        }
        "atm01_bad_compromise" => {
            let n = u("n");
            let ap = if g("ap_close") >= 0.5 { c.clone() } else { avg_price(x) };
            let (aph, apl, app) = (sma(h, n), sma(l, n), sma(&ap, n));
            let sp = g("stop_pct") / 100.;
            sim(x, from, to, &|t| {
                let s = t - 1;
                if s < 2 * n || !app[s - n].is_finite() { return vec![]; }
                // strict comparisons with a 1e-9 relative tolerance: a high equal to the average is not inside (float noise)
                let e = |v: f64| v.abs() * 1e-9;
                if !(h[s] < aph[s] - e(aph[s]) && l[s] > apl[s] + e(apl[s])) { return vec![]; }
                if app[s] > app[s - n] + e(app[s]) && o[t] > c[s] { vec![(1.0, Ord::Open)] }
                else if app[s] < app[s - n] - e(app[s]) && o[t] < c[s] { vec![(-1.0, Ord::Open)] } else { vec![] }
            }, &|t, q| {
                if t == q.ei { return None; }
                let pct = q.epx * (1.0 - q.dir * sp);
                let stop = if q.dir > 0.0 { apl[t - 1].max(pct) } else { aph[t - 1].min(pct) };
                levels(x, t, q.dir, Some(stop), None)
            })
        }
        "atm01_td_carrie" => {
            let (pct, sp, tp, hold) = (g("pct") / 100., g("stop_pct") / 100., g("tgt_pct") / 100., u("hold"));
            let ot = g("open_true") >= 0.5;
            sim(x, from, to, &|t| {
                if t < 6 { return vec![]; }
                let th4 = h[t - 4].max(c[t - 5]);
                let tl4 = l[t - 4].min(c[t - 5]);
                let mut v = vec![];
                let (oref_h, oref_l) = if ot { (th4, tl4) } else { (h[t - 4], l[t - 4]) };
                if (th4 > h[t - 2] || th4 > h[t - 3] || th4 > h[t - 5]) && c[t - 1] < c[t - 2] && o[t] < oref_h { v.push((1.0, Ord::Stop(th4 * (1.0 + pct)))); }
                if (tl4 < l[t - 2] || tl4 < l[t - 3] || tl4 < l[t - 5]) && c[t - 1] > c[t - 2] && o[t] > oref_l { v.push((-1.0, Ord::Stop(tl4 * (1.0 - pct)))); }
                v
            }, &|t, q| {
                if t > q.ei { if let Some(e) = levels(x, t, q.dir, Some(q.epx * (1.0 - q.dir * sp)), Some(q.epx * (1.0 + q.dir * tp))) { return Some(e); } }
                (t + 1 - q.ei >= hold).then(|| (c[t], "time exit on close (day 5)"))
            })
        }
        "atm01_deep_pockets" => {
            let ap = avg_price(x);
            let (lag, hn, hold) = (u("lag"), u("hh"), u("hold"));
            sim(x, from, to, &|t| {
                let s = t - 1;
                if s < lag.max(hn) { return vec![]; }
                if ap[s] < ap[s - lag] && ap[s] < c[s] && o[t] < ap[s] { vec![(1.0, Ord::Stop(ap[s]))] }
                else if ap[s] > ap[s - lag] && ap[s] > c[s] && o[t] > ap[s] { vec![(-1.0, Ord::Stop(ap[s]))] } else { vec![] }
            }, &|t, q| {
                if t > q.ei {
                    let (stop, tgt) = if q.dir > 0.0 { (l[t - 1], hi_n(x, t - 1, hn)) } else { (h[t - 1], lo_n(x, t - 1, hn)) };
                    if let Some(e) = levels(x, t, q.dir, Some(stop), Some(tgt)) { return Some(e); }
                }
                (t + 1 - q.ei >= hold).then(|| (c[t], "time exit on close (day 5)"))
            })
        }
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;
    #[test]
    fn meander_matches_hand_computation() {
        // two bars of constant prices: all 4 changes from the previous close are 0 -> levels equal the close
        let r: Vec<Row> = (0..3).map(|i| Row { ticker: "T".into(), date: format!("2001-01-0{}", i + 1), open: 100., high: 100., low: 100., close: 100., volume: None }).collect();
        let x = Ctx::new(&r, &r, &[]);
        let (lo, hi) = meander(&x, 2, 1.0);
        assert_eq!((lo[2], hi[2]), (100.0, 100.0));
        // one bar: O=101,H=104,L=99,C=102 vs prev close 100 -> changes .01,.04,-.01,.02: mean .015, pop sd .01802776
        let mut r2 = r.clone();
        r2[2] = Row { ticker: "T".into(), date: "2001-01-03".into(), open: 101., high: 104., low: 99., close: 102., volume: None };
        let x2 = Ctx::new(&r2, &r2, &[]);
        let (lo2, _) = meander(&x2, 1, 1.0);
        let sd = ((0.005f64.powi(2) + 0.025f64.powi(2) + 0.025f64.powi(2) + 0.005f64.powi(2)) / 4.0).sqrt();
        assert!((lo2[2] - 102.0 * (1.0 + 0.015 - sd)).abs() < 1e-9);
    }
}
