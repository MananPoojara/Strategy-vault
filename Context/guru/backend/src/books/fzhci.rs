//! Frank J. Fabozzi, Roland Füss and Dieter G. Kaiser (eds.), "The Handbook of Commodity Investing" (Wiley 2008). Book id
//! `fzhci`. Reading: claude-continuation-20261020/authors/fabozzi/agents/hci/. Five complete price-only rules: Lhabitant's
//! price/average cross (Ch.16), the Erb-Harvey 12-month GSCI momentum rule (Ch.8), and Park & Irwin's dual average,
//! outside channel and Alexander filter definitions (Ch.40). Term-structure, cross-sectional momentum, spread and
//! contract-life rules need futures curves or contract histories.
use crate::connors::{Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};
use crate::books::mpts::{period_ends, sma};

pub const BOOK: &str = "fzhci";
pub const REVIEWED: &[&str] = &["fzhci_ma15", "fzhci_mom12", "fzhci_dmac", "fzhci_chan40", "fzhci_filter"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

macro_rules! note { ($s:expr) => { concat!($s, " [Declared conventions, not from the book] Indicators are built from the start of the data file; a run starts flat and takes the first change of position inside the window; an open trade is marked at the window's last close. No costs. The chapters describe commodity futures; index, ETF and stock runs are a declared application; cash-index shorts are hypothetical.") }; }

pub fn catalogue() -> Vec<StrategyDef> {
    use Status::*;
    let live = |id, name, group, rules, notes| StrategyDef { id, name, group, session: 0, dir: "Long/Short", status: Live, needs_volume: false,
        needs_vix: false, needs_series: &[], book: BOOK, rules, notes, claim: None };
    let held = |id, name, group, st, rules| StrategyDef { id, name, group, session: 0, dir: "Long/Short", status: st, needs_volume: false,
        needs_vix: false, needs_series: &[], book: BOOK, rules, notes: "Held after the 2026-10-24 reading (authors/fabozzi/agents/hci).", claim: None };
    vec![
        live("fzhci_ma15", "Price crosses its 15-day moving average, long and short", "Ch.16 Lhabitant (p.395-398)",
            &["F|\"a 15-day moving average takes the last 15 closing prices, adds them up, and divides the result by 15\" (p.395)",
              "E|\"buy when the current price crosses the moving average from below\" (p.396)",
              "X|\"sell when the current price crosses the moving average from above\" (p.396); CTA trend followers trade up or down (p.394): the sell reverses to short"],
            note!("Daily closes, filled at the signal close [book silent]. Default 15 days (Exhibit 16.2); p.n = 60 (Exhibit 16.3) or the 20/40/200-day spans of p.397-398. A close equal to the average is no cross. p.short = 0 trades long only. Exhibits use gold and crude oil futures 2005-2007.")),
        live("fzhci_mom12", "12-month momentum: long if the past year was up, short if down (monthly)", "Ch.8 (p.221, Erb and Harvey)",
            &["E|\"goes long the index for one month if the previous one-year's excess return was positive\" (p.221)",
              "X|\"and going short the index if the prior one-year's excess return was negative\" (p.221); re-decided every month"],
            note!("Decided at each month-end close (last trading day of the month) against the month-end close 12 months earlier, and filled at that close. The book uses the GSCI excess-return index (futures over T-bills); a price series is a declared application and no T-bill or collateral return is added. An unchanged year keeps the position. Book result: 13.47% a year after up years, -5.49% after down years, 1969-2004 (not claimed here).")),
        live("fzhci_dmac", "Dual moving-average crossover, next-open entry, always long or short (10/40)", "Ch.40 Park & Irwin (p.914, 921)",
            &["F|\"SMA over s days ... LMA over l days ... s < l\" (p.914)",
              "E|\"Go long at P(o,t+1) if SMA(t) > LMA(t), where P(o,t+1) is the open at time t+1\" (p.914)",
              "X|\"Go short at P(o,t+1) if SMA(t) < LMA(t)\" (p.914)"],
            note!("The chapter leaves s and l open ('3. Parameters: s, l'); default 10/40, the pair Tomek and Querin tested (Exhibit 40.1, p.921); p.s / p.l = 3/10 runs their other pair. Equal averages keep the position. A signal on the last bar of the window is not filled.")),
        live("fzhci_chan40", "Outside price channel breakout, always long or short (40 days)", "Ch.40 Park & Irwin (p.915, 927)",
            &["F|\"HH(t) = max{P(h,t-1), ..., P(h,t-n+1)}, LL(t) = min{P(l,t-1), ..., P(l,t-n+1)}\" (p.915)",
              "E|\"Go long at P(c,t) if P(c,t) > HH(t), where P(c,t) is the close at time t\" (p.915)",
              "X|\"Go short at P(c,t) if P(c,t) < LL(t)\" (p.915); reverse on the opposite breakout"],
            note!("n = 40 days, 'a representative parameter' (Irwin et al., p.927); as printed the channel is the n-1 bars before today. p.n changes it.")),
        live("fzhci_filter", "Alexander filter on closes, always long or short (3%)", "Ch.40 Park & Irwin (p.916-919)",
            &["F|\"High extreme point (HEP) = the highest close obtained while in a long trade. Low extreme point (LEP) = the lowest close obtained while in a short trade. x = the percent filter size\" (p.917)",
              "E|\"Go long on the close, if today's close rises x% above the LEP\" (p.917)",
              "X|\"Go short on the close, if today's close falls x% below the HEP\" (p.917)"],
            note!("Default x = 3%, the size used in both cited studies (p.918-919); p.x = 1.5 or 5 are the others. 'Rises x% above' = close at least LEP x (1 + x/100). Before the first trade the lowest and highest closes since the start of the data file act as LEP and HEP [book silent]; the extreme point restarts at the reversal close.")),
        held("fzhci_term", "Term-structure (backwardation / contango) strategies", "Ch.3, 8, 18", NeedsData("needs commodity futures curves (near and far contracts)"), &["E|Long backwardated, short contango commodities"]),
        held("fzhci_xsmom", "Cross-sectional commodity momentum", "Ch.8", NeedsData("ranks 12 commodity futures; needs those futures series"), &["E|Long the past winners, short the losers"]),
        held("fzhci_spread", "Cootner July/December wheat spread and Houthakker contract-life stops", "Ch.17, 40 (p.407, 918)", NeedsData("needs individual contract-month histories"), &["E|Seasonal spread between contract months"]),
        held("fzhci_osc", "RSI 30/70, stochastics and other moving-average variants", "Ch.16, 40", NotTestable("no lengths, levels or exits are given"), &["E|Oversold / overbought readings"]),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    Some(match id {
        "fzhci_ma15" => ("fzhci_ma15", "Lhabitant price/average cross", "15"),
        "fzhci_mom12" => ("fzhci_mom12", "Erb-Harvey 12-month momentum", "12 months"),
        "fzhci_dmac" => ("fzhci_dmac", "Park & Irwin dual average", "10/40"),
        "fzhci_chan40" => ("fzhci_chan40", "Park & Irwin outside channel", "40"),
        "fzhci_filter" => ("fzhci_filter", "Park & Irwin Alexander filter", "3%"),
        _ if id.starts_with("fzhci_") => ("fzhci_nt", "Handbook of Commodity Investing (held)", "-"),
        _ => return None,
    })
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    Some(match id {
        "fzhci_ma15" => vec![p("n", "Average days (book: 15; 60, 20, 40, 200 named)", 15., 2., 250., 1.), p("short", "Also go short (1) / long only (0)", 1., 0., 1., 1.)],
        "fzhci_mom12" => vec![p("n", "Lookback months (book: 12)", 12., 1., 24., 1.)],
        "fzhci_dmac" => vec![p("s", "Short average (book example: 10 or 3)", 10., 1., 100., 1.), p("l", "Long average (book example: 40 or 10)", 40., 2., 250., 1.)],
        "fzhci_chan40" => vec![p("n", "Channel days (book: 40)", 40., 2., 250., 1.)],
        "fzhci_filter" => vec![p("x", "Filter % (book: 3; 1.5 and 5 tested)", 3., 0.5, 20., 0.5)],
        _ => return None,
    })
}

/// Position after each bar's close (+1 / -1, None before the first signal).
fn positions(x: &Ctx, id: &str, p: &Params) -> Vec<Option<i8>> {
    let g = |k: &str, d: f64| p.get(k).copied().unwrap_or(d);
    let ni = |k: &str, d: f64| g(k, d).round().max(1.) as usize;
    let c = &x.c;
    let n = c.len();
    let keep = |raw: Vec<i8>| { let mut cur = None; raw.into_iter().map(|r| { if r != 0 { cur = Some(r); } cur }).collect::<Vec<_>>() };
    let cmp = |a: f64, b: f64| if !(a.is_finite() && b.is_finite()) || a == b { 0 } else if a > b { 1 } else { -1 };
    match id {
        "fzhci_ma15" => { let m = sma(c, ni("n", 15.)); keep((0..n).map(|i| cmp(c[i], m[i])).collect()) }
        "fzhci_dmac" => { let (a, b) = (sma(c, ni("s", 10.)), sma(c, ni("l", 40.))); keep((0..n).map(|i| cmp(a[i], b[i])).collect()) }
        "fzhci_chan40" => {
            let k = ni("n", 40.).max(2) - 1;
            keep((0..n).map(|i| if i < k { 0 } else {
                let hh = x.h[i - k..i].iter().cloned().fold(f64::MIN, f64::max);
                let ll = x.l[i - k..i].iter().cloned().fold(f64::MAX, f64::min);
                if c[i] > hh { 1 } else if c[i] < ll { -1 } else { 0 } }).collect())
        }
        "fzhci_mom12" => {
            let ends = period_ends(x, true);
            let idx: Vec<usize> = (0..n).filter(|&i| ends[i]).collect();
            let k = ni("n", 12.);
            let mut raw = vec![0i8; n];
            for j in k..idx.len() { raw[idx[j]] = cmp(c[idx[j]], c[idx[j - k]]); }
            keep(raw)
        }
        "fzhci_filter" => {
            let f = g("x", 3.) / 100.;
            let (mut pos, mut hep, mut lep) = (0i8, c[0], c[0]);
            (0..n).map(|i| {
                let v = c[i];
                if pos >= 0 { hep = hep.max(v); }
                if pos <= 0 { lep = lep.min(v); }
                if pos <= 0 && v >= lep * (1. + f) { pos = 1; hep = v; }
                else if pos >= 0 && v <= hep * (1. - f) { pos = -1; lep = v; }
                (pos != 0).then_some(pos)
            }).collect()
        }
        _ => vec![],
    }
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from > to || to >= x.r.len() { return None; }
    let st = positions(x, id, p);
    let long_only = id == "fzhci_ma15" && p.get("short").copied().unwrap_or(1.) < 0.5;
    let next_open = id == "fzhci_dmac";
    let (mut out, mut cur) = (vec![], None::<Trade>);
    for i in from.max(1)..=to {
        let (s, prev) = (st[i], st[i - 1]);
        if !matches!((s, prev), (Some(a), Some(b)) if a != b) { continue; }
        let (fi, px, kind) = if next_open { if i == to { continue; } (i + 1, x.o[i + 1], "open") } else { (i, x.c[i], "close") };
        let d = s.unwrap();
        if let Some(mut t) = cur.take() {
            t.exit_i = fi; t.exit_px = px; t.signal_i = i; t.reason = (if d > 0 { "signal turned long" } else { "signal turned short" }).into(); out.push(t);
        }
        if d > 0 || !long_only {
            cur = Some(Trade { dir: d as f64, legs: vec![Leg { i: fi, px, w: 1. }], exit_i: fi, exit_px: px, reason: String::new(),
                entry_kind: kind, signal_i: i, open: false, exits: vec![] });
        }
    }
    if let Some(mut t) = cur { t.exit_i = to; t.exit_px = x.c[to]; t.signal_i = to; t.reason = "open".into(); t.open = true; out.push(t); }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn catalogue_admits_exactly_the_reviewed_ids() {
        let live: Vec<_> = catalogue().into_iter().filter(|d| d.status == Status::Live).map(|d| d.id).collect();
        assert_eq!(live, REVIEWED);
    }
}

// Instrument admission: Fabozzi 2026-10-24 block, moved from instrument_policy.rs on 31 Oct 2026 (lane ST).
// Admission only for the module REVIEWED lists (authors/fabozzi/admission/SOURCE-CONTRACT.md); kept in one block to merge.
// Runs on actual indices, ETFs and stocks, US and India (index_etf_or_stock, nifty50 basket, declared application).
pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| matches!(def.book,"fzhci") && (crate::books::fzhci::reviewed(def.id)),
    label: "The Handbook of Commodity Investing describes these rules on commodity futures (gold, crude oil, grains, the GSCI); index, ETF and stock runs (US and India) are a declared application, not the authors' test.",
    basis: "Fabozzi, Füss and Kaiser (eds.), The Handbook of Commodity Investing (2008): Lhabitant's price/15-day average cross (Ch.16 p.395-398), the Erb-Harvey 12-month momentum rule (Ch.8 p.221), Park and Irwin's dual moving-average (next-open), outside-channel and Alexander filter definitions (Ch.40 p.914-921), reviewed in authors/fabozzi/admission/SOURCE-CONTRACT.md with independent chronological ledgers. Close fills where the book is silent; defaults are book-printed study settings. No excess-return (T-bill) component or costs. Cash-index shorts are hypothetical exposure.",
};
