//! Mark Crisp, "Cashing in with Commodities" (stressfreetrading.com e-book, 67 pdf pages, no printed page numbers; cites are
//! pdf pages). Book id `mccic`. Reading: claude-continuation-20261020/authors/crisp/agents/cashcomm/.
//! Live: the weekly two-week breakout with the 2.5 x 30-day ATR stop, weekly-low trail and 50%/75% profit protection.
//! The 1-2-3 entries are incomplete, option selling needs option prices, the CRB / freight "secrets" are judgement.
use crate::connors::{Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};
use crate::books::mpts::{period_ends, sma};

pub const BOOK: &str = "mccic";
pub const REVIEWED: &[&str] = &["mccic_2wk"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const NOTE: &str = "Weekly bars are calendar weeks (Monday-Sunday) built from the daily bars. A two-week high is a weekly high that the next two weekly highs do not exceed (an equal high counts, p.21); it becomes the buy level when the second of those weeks closes, and the newest such level replaces older ones. A daily high above the level fills a buy stop at the level, or at the open if the day opens above it; mirror for sells at two-week lows. [Declared readings, not from the book] The newest level only; a level is used up once a day trades beyond it (also while in a trade); no entry on a day that breaks both levels; while in a trade new breakouts are ignored (p.20: 'not necessarily a stop and reverse system'); the order is not 'brought in' below the high (the book leaves that to judgement, p.27). Initial stop 2.5 x the 30-day average daily true range of the day before the fill (p.30), live from the next day. At each weekly close after the entry day: once the close shows at least 2R profit (R = initial risk) the stop protects 50% of the open profit, at 5R 75% (p.38-39), and the weekly-low trail stops ('Forget about the weekly lows', p.38); before that the stop goes one tick below the weekly low (p.38: 'a fraction below'). The stop never loosens (p.43). A stop fills at the stop or at a gapping open. One unit, no 4% sizing. The book trades commodity futures (crude oil, bean oil, sugar, cocoa); index, ETF and stock runs are a declared application; cash-index shorts are hypothetical.";

pub fn catalogue() -> Vec<StrategyDef> {
    use Status::*;
    let nt = |id, name, group, why, rules| StrategyDef { id, name, group, session: 0, dir: "Long/Short", status: NotTestable(why), needs_volume: false, needs_vix: false,
        needs_series: &[], book: BOOK, rules, notes: "Read in full (authors/crisp/agents/cashcomm).", claim: None };
    vec![
        StrategyDef { id: "mccic_2wk", name: "Weekly two-week breakout with ATR stop, weekly-low trail and profit protection", group: "Two-week breakout (pdf p.17-43)",
            session: 0, dir: "Long/Short", status: Live, needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK,
            rules: &[
                "F|\"The two-week rule is: 'A high or low which is not EXCEEDED for at least two weeks.'\"; \"a bar which is equal to the break out bar does count\" (p.21); \"Keep to the weekly charts\" (p.18)",
                "E|\"The two-week break out is simply buying or selling on the breakout of a minimum of a two-week high or low\" (p.18)",
                "X|Initial stop: \"I set a 30-DAY ATR\"; \"That figure is 2,5 X ATR\" (p.30)",
                "X|\"Keep trailing your stop to exit the trade a fraction below the weekly bar. It is adjusted just once per week\" (p.38)",
                "X|\"When crude oil on a weekly basis closes at or above a price, which gives us a paper profit of [2R] or more, then protect 50% of those profits ... Forget about the weekly lows\" (p.38); \"When the weekly close is equal to or greater than 5 X R ... then protect 75% of those profits\" (p.39); \"The stop can never move up (when going short) or move down (when going long)\" (p.43)",
            ],
            notes: NOTE, claim: None },
        nt("mccic_123", "1-2-3 entries", "1-2-3 (pdf p.44-50)", "points 1, 2 and 3 are picked by eye and the entry offset is 'your choice'", &["E|Buy above point 2 after a higher low at point 3"]),
        nt("mccic_options", "Selling options on the trend", "Options (pdf p.51-55)", "needs option prices (none in the vault)", &["E|Sell puts in an uptrend"]),
        nt("mccic_secrets", "CRB and Baltic freight 'secrets'", "Secrets (pdf p.56-59)", "chart reading of other markets (CRB, Baltic freight index); no entry or exit", &["E|Background judgement"]),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    Some(match id {
        "mccic_2wk" => ("mccic_2wk", "Crisp two-week breakout", "2 weeks, 2.5 ATR(30)"),
        _ if id.starts_with("mccic_") => ("mccic_nt", "Crisp Cashing in with Commodities (not testable)", "-"),
        _ => return None,
    })
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    (id == "mccic_2wk").then(|| vec![
        ParamDef { key: "weeks", label: "Weeks the high/low must hold (book: at least 2)", def: 2., min: 1., max: 6., step: 1. },
        ParamDef { key: "atr_mult", label: "Initial stop, x ATR (book: 2.5)", def: 2.5, min: 0.5, max: 6., step: 0.5 },
        ParamDef { key: "atr", label: "ATR days (book: 30)", def: 30., min: 5., max: 60., step: 1. },
    ])
}

/// Simple average of the daily true range (high-low widened by the previous close).
fn atr(x: &Ctx, n: usize) -> Vec<f64> {
    let tr: Vec<f64> = (0..x.c.len()).map(|i| if i == 0 { x.h[0] - x.l[0] } else {
        (x.h[i] - x.l[i]).max((x.h[i] - x.c[i - 1]).abs()).max((x.l[i] - x.c[i - 1]).abs()) }).collect();
    sma(&tr, n)
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if id != "mccic_2wk" || from > to || to >= x.r.len() { return None; }
    let g = |k: &str, d: f64| p.get(k).copied().unwrap_or(d);
    let nw = g("weeks", 2.).round().max(1.) as usize;
    let mult = g("atr_mult", 2.5);
    let a = atr(x, g("atr", 30.).round().max(1.) as usize);
    let tick = x.market_tick();
    // weekly bars: (end index, high, low, close)
    let ends = period_ends(x, false);
    let mut weeks: Vec<(usize, f64, f64, f64)> = vec![];
    let mut start = 0;
    for i in 0..x.c.len() {
        if ends[i] {
            let (hi, lo) = (x.h[start..=i].iter().cloned().fold(f64::MIN, f64::max), x.l[start..=i].iter().cloned().fold(f64::MAX, f64::min));
            weeks.push((i, hi, lo, x.c[i])); start = i + 1;
        }
    }
    // levels confirmed at the close of bar i (applied from bar i+1)
    let mut new_buy = vec![None::<f64>; x.c.len()];
    let mut new_sell = vec![None::<f64>; x.c.len()];
    for k in 0..weeks.len() {
        if k + nw >= weeks.len() { break; }
        let e = weeks[k + nw].0;
        if (1..=nw).all(|j| weeks[k + j].1 <= weeks[k].1) { new_buy[e] = Some(weeks[k].1); }
        if (1..=nw).all(|j| weeks[k + j].2 >= weeks[k].2) { new_sell[e] = Some(weeks[k].2); }
    }
    let week_end_at: std::collections::HashMap<usize, (f64, f64, f64)> = weeks.iter().map(|w| (w.0, (w.1, w.2, w.3))).collect();
    let (mut buy, mut sell) = (None::<f64>, None::<f64>);
    let mut out = vec![];
    // open trade: (trade, stop, risk, protected)
    let mut cur: Option<(Trade, f64, f64, bool)> = None;
    for i in 0..=to {
        if i > 0 {
            if let Some(v) = new_buy[i - 1] { buy = Some(v); }
            if let Some(v) = new_sell[i - 1] { sell = Some(v); }
        }
        let hit_b = buy.is_some_and(|b| x.h[i] > b);
        let hit_s = sell.is_some_and(|s| x.l[i] < s);
        let (lb, ls) = (buy, sell);
        if hit_b { buy = None; }
        if hit_s { sell = None; }
        if i < from { continue; }
        if let Some((mut t, stop, risk, prot)) = cur.take() {
            let d = t.dir;
            if (d > 0. && x.l[i] <= stop) || (d < 0. && x.h[i] >= stop) {
                let px = if d > 0. { x.o[i].min(stop) } else { x.o[i].max(stop) };
                t.exit_i = i; t.exit_px = px; t.signal_i = i;
                t.reason = (if prot { "profit-protection stop" } else { "stop" }).into();
                out.push(t);
                continue;
            }
            let (mut stop, mut prot) = (stop, prot);
            if let Some(&(wh, wl, wc)) = week_end_at.get(&i) {
                let e = t.legs[0].px;
                let pr = d * (wc - e);
                let cand = if pr >= 5. * risk { prot = true; Some(e + d * 0.75 * pr) }
                    else if pr >= 2. * risk { prot = true; Some(e + d * 0.5 * pr) }
                    else if !prot { Some(if d > 0. { wl - tick } else { wh + tick }) } else { None };
                if let Some(cv) = cand { stop = if d > 0. { stop.max(cv) } else { stop.min(cv) }; }
            }
            cur = Some((t, stop, risk, prot));
            continue;
        }
        if hit_b == hit_s || i == 0 || !a[i - 1].is_finite() { continue; }
        let risk = mult * a[i - 1];
        let (d, px) = if hit_b { (1., x.o[i].max(lb.unwrap())) } else { (-1., x.o[i].min(ls.unwrap())) };
        let t = Trade { dir: d, legs: vec![Leg { i, px, w: 1. }], exit_i: i, exit_px: px, reason: String::new(), entry_kind: "stop", signal_i: i, open: false, exits: vec![] };
        // weekly updates start with the first week that closes after the entry day
        cur = Some((t, px - d * risk, risk, false));
    }
    if let Some((mut t, _, _, _)) = cur { t.exit_i = to; t.exit_px = x.c[to]; t.signal_i = to; t.reason = "open".into(); t.open = true; out.push(t); }
    Some(out)
}

// Instrument admission: Crisp (mccic) 2026-10-24 block, moved from instrument_policy.rs on 31 Oct 2026 (lane ST).
// Admission only for books::mccic::REVIEWED (authors/crisp/admission/SOURCE-CONTRACT.md); kept in one block to merge.
// Runs on actual indices, ETFs and stocks, US and India (index_etf_or_stock, nifty50 basket, declared application).
pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book==crate::books::mccic::BOOK && crate::books::mccic::reviewed(def.id),
    label: "Cashing in with Commodities trades this breakout on commodity futures (crude oil, bean oil, sugar, cocoa); index, ETF and stock runs (US and India) are a declared application, not the author's test.",
    basis: "Mark Crisp, Cashing in with Commodities (pdf p.17-43): weekly two-week breakout, buy/sell stop at the two-week high/low, 2.5 x 30-day ATR initial stop, weekly-low trail, 50% / 75% profit protection at 2R / 5R, reviewed in authors/crisp/admission/SOURCE-CONTRACT.md with independent chronological ledgers. Which level, gap fills, one-tick trail offset and ignoring breakouts while in a trade are declared readings. One unit; no 4% sizing. Cash-index shorts are hypothetical exposure.",
};
