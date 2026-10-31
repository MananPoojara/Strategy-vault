//! Martin J. Pring, "Technical Analysis Explained" (2nd ed., McGraw-Hill 1985; scan, 214 pdf pages; the Spanish
//! "Analisis Tecnico Explicado" is a 1989 translation of the same edition). Book id `mptae`.
//! Reading: claude-continuation-20261020/authors/pring/agents/mptae/. Live: the price-only rules with complete entries
//! and exits; Coppock's buy indicator has no exit and runs with the user's standard exits (labelled).
use crate::connors::{atr, Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};
use crate::books::mpts::sma;

pub const BOOK: &str = "mptae";

/// Admitted after the 2026-10-23 reading and independent ledgers (authors/pring/admission/SOURCE-CONTRACT.md).
pub const REVIEWED: &[&str] = &["mptae_ma40w", "mptae_1030w", "mptae_russell", "mptae_tw9m", "mptae_coppock"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const LONG: &str = "Long or cash by default: the book speaks of bull and bear signals, never of short selling; p.short = 1 reverses into shorts [option]. Weekly bar = last trading day of each Monday-based week; monthly = last trading day of the month (the last data bar never counts as a month end). Simple averages unless stated. Fill at the signal bar's close [Not from the book: no fill is stated]. Equal values give no new signal [declared]. The book's market is the DJIA (and a trade-weighted currency index for mptae_tw9m); US and Indian indices, ETFs and stocks are a declared application.";

pub fn catalogue() -> Vec<StrategyDef> {
    use Status::*;
    let d = |id, name, group, rules, notes| StrategyDef { id, name, group, session: 0, dir: "Long", status: Live, needs_volume: false, needs_vix: false,
        needs_series: &[], book: BOOK, rules, notes, claim: None };
    let nt = |id, name, group, why, rules| StrategyDef { id, name, group, session: 0, dir: "Long", status: NotTestable(why), needs_volume: false, needs_vix: false,
        needs_series: &[], book: BOOK, rules, notes: "Read in full (authors/pring/agents/mptae).", claim: None };
    vec![
        d("mptae_ma40w", "40-week moving-average crossover (primary trend)", "Ch.7 · Moving Averages (p.89-92, 156-157)",
          &["E|\"A change from a rising to a declining market is signaled when the index moves below its moving average. The reverse set of conditions will confirm the termination of a declining trend.\" (p.91)",
            "F|\"For primary trends, experiments with 20-, 30-, and 40-week moving averages from 1897 to 1967 have shown the 40-week period to be the most helpful\" (p.92); weekly closes (\"plotting the moving average on the final week\", p.91)",
            "X|Sell when the weekly close moves below the 40-week average. p.n: 10, 13 or 30 weeks are the book's intermediate spans (p.92, 158)"],
          LONG),
        d("mptae_1030w", "10- and 30-week averages with the slope condition", "Ch.7 · Moving Averages (p.95-97)",
          &["F|\"Two averages which have been found reliable in determining primary market moves are the 10- and 30-week moving averages, when used together ... the weekly closing price ... is used\" (p.95-97)",
            "X|\"Signals are given when the 10-week average moves below the 30-week average and when the 30-week average itself is declining\" (p.97)",
            "E|\"It is not assumed to be reversed until both averages are rising simultaneously, with the 10-week higher than the 30-week moving average\" (p.97)"],
          "Rising / declining = higher / lower than the previous week's value [declared]. Otherwise as the other ids: long or cash, p.short = 1 reverses; fill at the signal week's close [Not from the book]; declared application on US and Indian indices, ETFs and stocks."),
        d("mptae_russell", "Russell 13/50-week moving-average major signals", "Ch.12 · Moving Averages and Primary Trends (p.158)",
          &["E|\"Major signals are given when the 13-week average crosses above (or below) the 50-week average.\" (Chart 12.4, p.158, a system devised by Richard Russell)",
            "X|Sell when the 13-week average crosses below the 50-week average; the 4/13-week 'alerts' are warnings, not trades"],
          LONG),
        d("mptae_tw9m", "Index vs its 9-month average confirmed by the 12-month ROC vs its 9-month average", "Ch.27 · Currencies (p.354-355)",
          &["E|\"Buy signals are given whenever the trade-weighted index crosses above its 9-month moving average and this is confirmed by a crossover of the 12-month ROC index above its 9-month moving average.\" (p.354)",
            "X|\"Such signals remain in force until both crossovers occur in a negative way. A sell signal is not triggered if the trade-weighted index alone crosses below its moving average\" (p.354)"],
          LONG),
        d("mptae_coppock", "Coppock long-term momentum buy (10-month weighted average of 11- and 14-month ROC) with standard exits", "Ch.8 · Rate of Change (p.116-117)",
          &["F|\"the smoothing is a 10-month weighted average of 11- and 14-month rates of change of DJIA monthly closing prices\" (p.116)",
            "E|\"this index has been found useful for market bottoms rather than tops, so the momentum curve takes on significance only when it falls below the 0 reference line and then rises\" (p.116): buy at the month-end close when the curve turns up from below zero",
            "X|[Not from the book] the book gives no exit; the user's standard exits: p.exit 0 = close after p.hold daily bars (10 default; 3 / 5), 1 = trailing stop 2 x ATR(5) behind the best price since entry, 2 = first up close"],
          "Option exit 3 (Elliott Middleton, 'The Coppock Curve', S&C V.12:11, 1994, PDF p.1-3): the same curve and turn-up buy, held for N months ('this allocation should be maintained for about two years'; he measured returns 6, 12, 18 and 24 months after each buy) and sold at that month-end close; his test used monthly average S&P 500 data (1960-1993), so with this exit the curve is computed on each month's average daily close by default (month-end closes are the option); a buy while holding does not add or extend [declared]; he notes it fails on the Japanese market. Curve = linearly weighted 10-month average of (11-month ROC + 14-month ROC), ROC = 100 (close / close n months ago - 1) [the sum has the same zero line and turns as the average]. Turn up = this month higher than last month after last month was lower than the month before, last month below zero [declared]. Month-end entries at the close; exits on daily bars. ATR trail: from the bar after entry, stop = best of the entry close and the highs since, minus 2 x Wilder ATR(5) of the previous bar; filled at the stop or a lower open. Long only. Declared application on US and Indian indices, ETFs and stocks (the book: DJIA)."),
        nt("mptae_ema14m", "DJIA 14-month EMA with the 14-month EMA of the 12-month ROC", "Ch.12 (p.159-160)",
           "the text says a price crossover of the EMA confirmed by a change in direction of the smoothed ROC (p.159), the chart caption says a reversal of the EMA itself confirmed by the ROC average (p.160), and neither says what a mixed state means",
           &["E|\"a 14-month exponential moving average for the DJIA and a 14-month exponential moving average for a 12-month rate-of-change index\" (p.159)"]),
        nt("mptae_gm4m", "General Motors 4-month rule", "Ch.12 (p.164-167)",
           "reads General Motors shares to time the whole market; the vault's GM history starts with the new company (2010) and the book does not say whether peaks are closes or intraday highs",
           &["E|\"if in a bull market General Motors fails to make a new high within 4 calendar months ... of its previous peak, the bullish trend of the market has reversed\" (p.167)"]),
        nt("mptae_data_rules", "Fed Funds 10-week MA + 13-week ROC, Members' Short Index 85/65, Dow theory, breadth, sentiment", "Ch.3-27",
           "Fed Funds rate, NYSE member short sales, up/down volume and sentiment surveys are not in the vault; Dow theory, patterns, trendlines and cycles are judgement. The pound system (p.361-363) is the Triple Indicator (mpts_triple) and Gartley's advanced average (p.92-95) is mpmm_gartley_25a3",
           &["E|See notes"]),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    Some(match id {
        "mptae_ma40w" => ("mptae_ma", "Pring TA Explained MA rules", "40-week MA"),
        "mptae_1030w" => ("mptae_ma", "Pring TA Explained MA rules", "10/30 weeks with slope"),
        "mptae_russell" => ("mptae_ma", "Pring TA Explained MA rules", "Russell 13/50 weeks"),
        "mptae_tw9m" => ("mptae_ma", "Pring TA Explained MA rules", "9-month MA + ROC confirmation"),
        "mptae_coppock" => ("mptae_mom", "Pring TA Explained momentum", "Coppock buy, standard exits"),
        _ if id.starts_with("mptae_") => ("mptae_nt", "Pring TA Explained (not testable)", "-"),
        _ => return None,
    })
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    let short = p("short", "Also go short on sell signals (1)", 0., 0., 1., 1.);
    Some(match id {
        "mptae_ma40w" => vec![p("n", "MA (weeks)", 40., 2., 60., 1.), short],
        "mptae_1030w" => vec![p("fast", "Fast MA (weeks)", 10., 2., 26., 1.), p("slow", "Slow MA (weeks)", 30., 5., 60., 1.), short],
        "mptae_russell" => vec![p("fast", "Fast MA (weeks)", 13., 2., 26., 1.), p("slow", "Slow MA (weeks)", 50., 10., 80., 1.), short],
        "mptae_tw9m" => vec![p("ma", "MA (months)", 9., 2., 24., 1.), p("roc", "ROC (months)", 12., 3., 24., 1.), short],
        "mptae_coppock" => vec![p("exit", "Exit: 0 = after hold bars, 1 = 2 x ATR(5) trail, 2 = first up close [0-2 not from the book]; 3 = Middleton: hold N months (S&C V.12:11, 1994)", 0., 0., 3., 1.),
                                p("hold", "[Not from the book] bars held (3 / 5 / 10)", 10., 3., 10., 1.),
                                p("months", "Exit 3: months held (Middleton 'about two years' = 24; he also measured 6 / 12 / 18)", 24., 1., 36., 1.),
                                p("avg", "Exit 3: curve on 1 = monthly average closes (Middleton's test, default), 0 = month-end closes", 1., 0., 1., 1.)],
        _ => return None,
    })
}

fn sign(a: f64, b: f64) -> Option<i8> {
    if !(a.is_finite() && b.is_finite()) { None } else if a > b { Some(1) } else if a < b { Some(-1) } else { None }
}

/// Values of f over the period-end closes, written back on those bars (NaN elsewhere).
fn on_ends(ends: &[bool], v: &[f64], f: impl Fn(&[f64]) -> Vec<f64>) -> Vec<f64> {
    let idx: Vec<usize> = (0..ends.len()).filter(|&i| ends[i]).collect();
    let r = f(&idx.iter().map(|&i| v[i]).collect::<Vec<_>>());
    let mut out = vec![f64::NAN; ends.len()];
    for (k, &i) in idx.iter().enumerate() { out[i] = r[k]; }
    out
}

fn wma(v: &[f64], n: usize) -> Vec<f64> {
    let den = (n * (n + 1)) as f64 / 2.;
    (0..v.len()).map(|i| if i + 1 < n { f64::NAN } else { (0..n).map(|k| (n - k) as f64 * v[i - k]).sum::<f64>() / den }).collect()
}

fn open(dir: f64, i: usize, px: f64) -> Trade {
    Trade { dir, legs: vec![Leg { i, px, w: 1. }], exit_i: i, exit_px: px, reason: String::new(), entry_kind: "close", signal_i: i, open: false, exits: vec![] }
}

/// Trades on changes of the wanted side (history before `from` counts; entries allowed through `to`).
fn states(x: &Ctx, from: usize, to: usize, st: &[Option<i8>], long_only: bool, why: [&str; 2]) -> Vec<Trade> {
    let (mut out, mut cur, mut last) = (vec![], None::<Trade>, None::<i8>);
    for i in 0..=to {
        let s = st[i];
        let change = matches!((s, last), (Some(a), Some(b)) if a != b);
        if s.is_some() { last = s; }
        if i < from || !change { continue; }
        let sd = s.unwrap();
        if let Some(mut t) = cur.take() { t.exit_i = i; t.exit_px = x.c[i]; t.signal_i = i; t.reason = (if sd > 0 { why[0] } else { why[1] }).into(); out.push(t); }
        if sd > 0 || !long_only { cur = Some(open(sd as f64, i, x.c[i])); }
    }
    if let Some(mut t) = cur { t.exit_i = to; t.exit_px = x.c[to]; t.signal_i = to; t.reason = "open".into(); t.open = true; out.push(t); }
    out
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    let schema = params(id)?;
    if from > to || to >= x.r.len() { return None; }
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).unwrap().def);
    let u = |k: &str| g(k).round().max(1.) as usize;
    let long_only = g_short(&schema, p);
    let (c, n) = (&x.c, x.c.len());
    let wk = crate::books::mpts::period_ends(x, false);
    let mo = crate::books::mpts::period_ends(x, true);
    Some(match id {
        "mptae_ma40w" => {
            let m = on_ends(&wk, c, |v| sma(v, u("n")));
            states(x, from, to, &(0..n).map(|i| sign(c[i], m[i])).collect::<Vec<_>>(), long_only, ["weekly close crossed above the average", "weekly close crossed below the average"])
        }
        "mptae_russell" => {
            let (f, s) = (on_ends(&wk, c, |v| sma(v, u("fast"))), on_ends(&wk, c, |v| sma(v, u("slow"))));
            states(x, from, to, &(0..n).map(|i| sign(f[i], s[i])).collect::<Vec<_>>(), long_only, ["13-week average crossed above the 50-week", "13-week average crossed below the 50-week"])
        }
        "mptae_1030w" => {
            let (f, s) = (on_ends(&wk, c, |v| sma(v, u("fast"))), on_ends(&wk, c, |v| sma(v, u("slow"))));
            let idx: Vec<usize> = (0..n).filter(|&i| wk[i]).collect();
            let mut st = vec![None; n];
            for k in 1..idx.len() {
                let (i, j) = (idx[k], idx[k - 1]);
                if [f[i], s[i], f[j], s[j]].iter().all(|v| v.is_finite()) {
                    if f[i] > s[i] && f[i] > f[j] && s[i] > s[j] { st[i] = Some(1) } else if f[i] < s[i] && s[i] < s[j] { st[i] = Some(-1) }
                }
            }
            states(x, from, to, &st, long_only, ["both averages rising, 10-week above", "10-week below a declining 30-week"])
        }
        "mptae_tw9m" => {
            let (mn, rn) = (u("ma"), u("roc"));
            let m = on_ends(&mo, c, |v| sma(v, mn));
            let r = on_ends(&mo, c, |v| (0..v.len()).map(|i| if i >= rn { v[i] / v[i - rn] } else { f64::NAN }).collect());
            let rm = on_ends(&mo, &r, |v| sma(v, mn));
            let st: Vec<Option<i8>> = (0..n).map(|i| match (sign(c[i], m[i]), sign(r[i], rm[i])) { (Some(a), Some(b)) if a == b => Some(a), _ => None }).collect();
            states(x, from, to, &st, long_only, ["price and ROC above their 9-month averages", "price and ROC below their 9-month averages"])
        }
        "mptae_coppock" => coppock(x, from, to, &mo, g("exit").round() as i64, u("hold"), u("months"), g("avg") >= 0.5),
        _ => return None,
    })
}

fn g_short(schema: &[ParamDef], p: &Params) -> bool {
    schema.iter().any(|d| d.key == "short") && p.get("short").copied().unwrap_or(0.) < 0.5
}

/// Coppock curve on month-end closes; buy at the month-end close of an upturn from below zero; standard exits.
pub fn coppock_curve(m: &[f64]) -> Vec<f64> {
    let roc = |k: usize| -> Vec<f64> { (0..m.len()).map(|i| if i >= k { 100. * (m[i] / m[i - k] - 1.) } else { f64::NAN }).collect() };
    let (a, b) = (roc(11), roc(14));
    wma(&(0..m.len()).map(|i| a[i] + b[i]).collect::<Vec<_>>(), 10)
}

/// Exit 3 is lane K9's Middleton option (to-code row 161, R4 decision 42; independent check authors/titles-1030/K9w/k9_ref.py):
/// with avg the curve uses each month's average daily close; entry still at the signal month's last close; out at the close of
/// the month-end `months` months later; signals while holding are ignored.
fn coppock(x: &Ctx, from: usize, to: usize, mo: &[bool], exit: i64, hold: usize, months: usize, avg: bool) -> Vec<Trade> {
    let idx: Vec<usize> = (0..x.c.len()).filter(|&i| mo[i]).collect();
    let mv: Vec<f64> = if exit == 3 && avg {
        idx.iter().enumerate().map(|(k, &e)| { let s = if k == 0 { 0 } else { idx[k - 1] + 1 }; x.c[s..=e].iter().sum::<f64>() / (e + 1 - s) as f64 }).collect()
    } else { idx.iter().map(|&i| x.c[i]).collect() };
    let cv = coppock_curve(&mv);
    let mut mnum = vec![0usize; x.c.len()];
    let mut k = 0;
    for i in 0..x.c.len() { mnum[i] = k; if mo[i] { k += 1; } }
    let mut buy = vec![false; x.c.len()];
    for k in 2..idx.len() {
        if [cv[k], cv[k - 1], cv[k - 2]].iter().all(|v| v.is_finite()) && cv[k - 1] < 0. && cv[k - 1] < cv[k - 2] && cv[k] > cv[k - 1] { buy[idx[k]] = true; }
    }
    let a = atr(x.r, 5);
    let (mut out, mut cur) = (vec![], None::<Trade>);
    for i in from..=to {
        if let Some(t) = cur.as_ref() {
            let e = t.legs[0].i;
            let done = match exit {
                1 if i > e => {
                    let best = (e + 1..i).map(|k| x.h[k]).fold(t.legs[0].px, f64::max);
                    let stop = best - 2. * a[i - 1];
                    (stop.is_finite() && x.l[i] <= stop).then(|| (x.o[i].min(stop), "2 x ATR(5) trailing stop"))
                }
                2 if i > e => (x.c[i] > x.c[i - 1]).then(|| (x.c[i], "first up close")),
                0 if i >= e + hold => Some((x.c[i], "bars held")),
                3 if mo[i] && mnum[i] >= mnum[e] + months => Some((x.c[i], "Middleton: months held")),
                _ => None,
            };
            if let Some((px, why)) = done {
                let mut t = cur.take().unwrap(); t.exit_i = i; t.exit_px = px; t.signal_i = i; t.reason = why.into(); out.push(t);
                continue;
            }
        }
        if cur.is_none() && buy[i] { cur = Some(open(1., i, x.c[i])); }
    }
    if let Some(mut t) = cur { t.exit_i = to; t.exit_px = x.c[to]; t.signal_i = to; t.reason = "open".into(); t.open = true; out.push(t); }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn coppock_weights_and_zero_line() {
        let m: Vec<f64> = (0..40).map(|i| 100. + i as f64).collect();
        let cv = coppock_curve(&m);
        assert!(cv[22].is_nan() && cv[23].is_finite() && cv[23] > 0.);
        assert!((wma(&[1., 2., 3.], 3)[2] - 14. / 6.).abs() < 1e-12);
    }
}
