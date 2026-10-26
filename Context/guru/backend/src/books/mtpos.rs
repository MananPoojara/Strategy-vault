//! Michael C. Thomsett, "Put Option Strategies for Smarter Trading" (FT Press 2009; printed page = pdf page - 13). Book id
//! `mtpos`. Reading: claude-continuation-20261020/authors/thomsett/agents/pos/. Live: the Ch.4 three-day swing set-up on
//! the stock itself. The option versions need option prices; the same set-up in Winning with Stocks is not counted again.
use crate::connors::{Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "mtpos";
pub const REVIEWED: &[&str] = &["mtpos_swing3"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

pub fn catalogue() -> Vec<StrategyDef> {
    use Status::*;
    let held = |id, name, group, st, rules| StrategyDef { id, name, group, session: 0, dir: "Long/Short", status: st, needs_volume: false, needs_vix: false,
        needs_series: &[], book: BOOK, rules, notes: "Held after the 2026-10-24 reading (authors/thomsett/agents/pos).", claim: None };
    vec![
        StrategyDef { id: "mtpos_swing3", name: "Three-day swing set-up: buy the reversal day after a 3-day downtrend, sell at the opposite set-up", group: "Ch.4 · Swing trading (p.65-69, 73-74)",
            session: 0, dir: "Long/Short", status: Live, needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK,
            rules: &[
                "F|\"Swing traders define an uptrend as three or more consecutive days ... the closing price is higher than the previous day's closing price; and second, the day's lowest price is higher than the previous day's closing price\" (p.65); \"A downtrend is the opposite\" (p.66)",
                "F|\"A reversal day ... The uptrend stops as soon as a downward-moving day occurs. This simply means that the stock's closing price is lower than its open, or vice versa\" (p.66)",
                "E|\"buy shares of stock at the bottom of the downtrend and then sell at the top\" (p.69): a reversal day (close above the open) after three or more downtrend days",
                "X|Sell at the top: the sell set-up (a reversal day after three or more uptrend days); \"the exit set-up does not usually serve as an entry set-up going in the opposite direction\" (p.69)",
            ],
            notes: "[Declared readings, not from the book] The reversal day must come right after the third (or later) trend day; the narrow-range day and high volume are 'and/or' confirmations without numbers and are not used. Entry and exit at the close of the set-up day. No stop and no time exit (the book gives none; a long without a sell set-up stays open). p.tt = 1 uses the figure note's higher-high / higher-low trend test (p.65; also Winning with Stocks p.80) instead of the text's close-and-low test. Long only by default, as the book recommends puts over shorting; p.short = 1 adds the mirror short. The book shows US stocks (YHOO, JNJ, TGT); index and ETF runs and Indian stocks are a declared application.",
            claim: None },
        held("mtpos_options", "Option versions of the swing trade, spreads, straddles, synthetics, uncovered puts", "Ch.4-8", NeedsData("needs option prices (none in the vault)"), &["E|Buy puts at the top of the swing"]),
        held("mtpos_charts", "Chapter 9 chart reading", "Ch.9", NotTestable("chart interpretation by eye"), &["E|Judgement"]),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    Some(match id {
        "mtpos_swing3" => ("mtpos_swing3", "Thomsett swing set-up", "3 days"),
        _ if id.starts_with("mtpos_") => ("mtpos_nt", "Thomsett Put Option Strategies (held)", "-"),
        _ => return None,
    })
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    (id == "mtpos_swing3").then(|| vec![
        ParamDef { key: "days", label: "Trend days before the reversal (book: 3 or more)", def: 3., min: 2., max: 6., step: 1. },
        ParamDef { key: "tt", label: "Trend test: 0 close and low vs prior close (text), 1 higher high and higher low (figure note)", def: 0., min: 0., max: 1., step: 1. },
        ParamDef { key: "short", label: "Also trade the short side (0 = long only, default)", def: 0., min: 0., max: 1., step: 1. },
    ])
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if id != "mtpos_swing3" || from > to || to >= x.r.len() { return None; }
    let g = |k: &str, d: f64| p.get(k).copied().unwrap_or(d);
    let need = g("days", 3.).round().max(1.) as usize;
    let hl = g("tt", 0.) >= 0.5;
    let shorts = g("short", 0.) >= 0.5;
    let n = x.c.len();
    let (mut up, mut dn) = (vec![0usize; n], vec![0usize; n]);
    for i in 1..n {
        let (u, d) = if hl { (x.h[i] > x.h[i - 1] && x.l[i] > x.l[i - 1], x.h[i] < x.h[i - 1] && x.l[i] < x.l[i - 1]) }
            else { (x.c[i] > x.c[i - 1] && x.l[i] > x.c[i - 1], x.c[i] < x.c[i - 1] && x.h[i] < x.c[i - 1]) };
        up[i] = if u { up[i - 1] + 1 } else { 0 };
        dn[i] = if d { dn[i - 1] + 1 } else { 0 };
    }
    let (mut out, mut cur) = (vec![], None::<Trade>);
    for j in from.max(1)..=to {
        let buy = dn[j - 1] >= need && x.c[j] > x.o[j];
        let sell = up[j - 1] >= need && x.c[j] < x.o[j];
        if let Some(mut t) = cur.take() {
            if (t.dir > 0. && sell) || (t.dir < 0. && buy) {
                t.exit_i = j; t.exit_px = x.c[j]; t.signal_i = j;
                t.reason = (if t.dir > 0. { "sell set-up" } else { "buy set-up" }).into(); out.push(t);
            } else { cur = Some(t); }
            continue;
        }
        let d = if buy { 1. } else if sell && shorts { -1. } else { continue };
        cur = Some(Trade { dir: d, legs: vec![Leg { i: j, px: x.c[j], w: 1. }], exit_i: j, exit_px: x.c[j], reason: String::new(), entry_kind: "close", signal_i: j, open: false, exits: vec![] });
    }
    if let Some(mut t) = cur { t.exit_i = to; t.exit_px = x.c[to]; t.signal_i = to; t.reason = "open".into(); t.open = true; out.push(t); }
    Some(out)
}
