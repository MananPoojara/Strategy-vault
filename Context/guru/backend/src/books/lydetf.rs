//! Tom Lydon, The ETF Trend Following Playbook (FT Press 2009). One rule: long above the 200-day
//! moving average, cash below; Lydon uses the exponential average (p.27) and prints Keller Partners'
//! S&P 500 long/cash 200-day EMA results (p.34-35). Page refs are printed pages (PDF = printed + 19).
//! Sources: library/lydetf/.
use crate::books::apta::{ema_first, run_long};
use crate::connors::{claim, sma, Ctx, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "lydetf";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef {
        id: "lydetf_ema200", name: "200-day EMA trend following (long / cash)", group: "Ch 3 · Spotting Trends", session: 0, dir: "Long",
        status: Status::Live, needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK,
        rules: &[
            "E|\"1. Buy when a position moves above the 200-day moving average.\" (p.25)",
            "X|\"2. Sell when a position moves below the 200-day moving average.\" (p.25); \"If a position falls below its 200-day moving average, sell it.\" (p.28)",
            "F|\"For my own purposes, I use the exponential moving average\"; \"EMA = (P * a) + (Previous EMA * (1 - a)), a = 2/(1+N)\" (p.27); Keller test: \"switched an S&P 500 index portfolio to cash every time that the index value dipped below its 200-day exponential average (EMA), and then reinvested it every time the index crossed back up\" (p.32)",
            "F|[Read, B] Daily close vs the EMA of daily closes, fill at that close (time of day not given); EMA started from the first close (seed not given). p.sma=1 = simple average (not the book default).",
        ],
        notes: "Keller's results assume 2.0%/yr interest on cash (p.34); the vault's cash earns 0. SPX in the vault starts 1983, so the 1970-2008 period is compared on 1983-2008 only. The SMA variant equals apta_ma200_dji.",
        claim: claim("", "", "$100,000 -> $1,776,000 (7.86%/yr; max DD -18.8%)", "", "S&P 500 1970-2008 (1930-1970: $1,345,000, 7.51%/yr, DD -35.1%)"),
    }]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    (id == "lydetf_ema200").then_some(("lydetf_ma", "Lydon 200-day trend following", "EMA"))
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    (id == "lydetf_ema200").then(|| vec![
        ParamDef { key: "n", label: "Moving average days", def: 200.0, min: 20.0, max: 300.0, step: 10.0 },
        ParamDef { key: "sma", label: "Simple (1) instead of exponential (0) average", def: 0.0, min: 0.0, max: 1.0, step: 1.0 },
    ])
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    let schema = params(id)?;
    let n = x.r.len();
    if n == 0 || from > to || to >= n {
        return Some(vec![]);
    }
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).map_or(0.0, |d| d.def));
    let len = g("n").round().max(1.0) as usize;
    let ma = if g("sma") >= 0.5 { sma(&x.c, len) } else { ema_first(&x.c, len) };
    let c = &x.c;
    let idx: Vec<usize> = (0..n).collect();
    Some(run_long(x, &idx, from, to, |_, i, holding| {
        if !ma[i].is_finite() {
            None
        } else if !holding && c[i] > ma[i] {
            Some("close above the 200-day average".into())
        } else if holding && c[i] < ma[i] {
            Some("close below the 200-day average".into())
        } else {
            None
        }
    }))
}
