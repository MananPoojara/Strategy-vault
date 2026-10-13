//! Peter B. Waite, "Abalone User Manual" (c)2003, tradingsystemstore.com (31 pdf pages; TradeStation code p.7-8).
//! Source: research drive MetaStock/@Systems & Indicators/Abalone for MS + Manual (tradingsystemstore.com)/Abalone User Manual.pdf;
//! copy in Books/Peter B Waite/Abalone/. Book id "pwabal". Notes: library/pwabal/README.md.
use crate::connors::{Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "pwabal";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef {
        id: "pwabal_abalone", name: "Abalone: 25/50-day SMAs rising (falling) 2 days, exit when the 25-day turns 2 days", group: "Trading System Rules (p.3-8)",
        session: 0, dir: "Long/Short", status: Status::Live, needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK,
        rules: &[
            "E|\"If (MarketPosition = 0) and Average(Close,25) > Average(Close,25)[1] and Average(Close,50) > Average(Close,50)[1] and Average(Close,25)[1] > Average(Close,25)[2] and Average(Close,50)[1] > Average(Close,50)[2] then Buy (\"LE\") at Market;\" (p.7); \"buy long tomorrow at the open\" (p.4)",
            "E|Short: the same with every '>' as '<', \"Sell (\"SE\") at Market\" (p.7); \"sell short tomorrow at the open\" (p.5)",
            "X|\"If (MarketPosition = +1) and Average(Close,25) < Average(Close,25)[1] and Average(Close,25)[1] < Average(Close,25)[2] then ExitLong (\"LX\") at Market;\" (p.7); \"exit tomorrow at the open\" (p.6)",
            "X|\"If (MarketPosition = -1) and Average(Close,25) > Average(Close,25)[1] and Average(Close,25)[1] > Average(Close,25)[2] then ExitShort (\"SX\") at Market;\" (p.8)",
            "F|\"There are no hard stops with Abalone\" (p.2); \"for all computations we use closing prices only\" (p.6)",
        ],
        notes: "Entries only when flat (MarketPosition = 0 at the signal close), so an exit and a new entry never share a signal bar; a new entry can be signalled on the close of the bar whose open closed the previous trade. Book results are futures portfolios in dollars (e.g. 4 markets Oct 1979-Oct 2002: 617 trades, 40% profitable, p.9-10; 20 markets: 3021 trades, 39%, p.17-18); no index result is printed. India: instrument swap; index shorts are hypothetical.",
        claim: None,
    }]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    (id == "pwabal_abalone").then_some(("pwabal_abalone", "Abalone (Waite)", "manual TradeStation code"))
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    (id == "pwabal_abalone").then(|| vec![p("fast", "Mid-term SMA (entry and exit)", 25., 10., 50., 5.), p("slow", "Long-term SMA (entry)", 50., 30., 100., 5.)])
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if id != "pwabal_abalone" || from >= to || to >= x.r.len() { return None; }
    let g = |k: &str, d: f64| (p.get(k).copied().unwrap_or(d).max(1.)) as usize;
    let (a, b) = (x.sma_n(g("fast", 25.)), x.sma_n(g("slow", 50.)));
    let up = |m: &[f64], i: usize| i >= 2 && m[i] > m[i - 1] && m[i - 1] > m[i - 2];
    let dn = |m: &[f64], i: usize| i >= 2 && m[i] < m[i - 1] && m[i - 1] < m[i - 2];
    let mut out = vec![];
    let mut cur: Option<Trade> = None;
    let mut pending: Option<(f64, usize)> = None; // 0 = exit, +-1 = entry; signal bar
    for i in from..=to {
        if let Some((d, s)) = pending.take() {
            if d == 0. {
                let mut t = cur.take().unwrap();
                t.exit_i = i; t.exit_px = x.o[i]; t.signal_i = s;
                t.reason = "25-day SMA turned against the trade 2 days · next open".into();
                out.push(t);
            } else {
                cur = Some(Trade { dir: d, legs: vec![Leg { i, px: x.o[i], w: 1. }], exit_i: i, exit_px: x.o[i], reason: String::new(), entry_kind: "open", signal_i: s, open: false, exits: vec![] });
            }
        }
        if i == to { break; }
        match cur.as_ref().map(|t| t.dir) {
            Some(d) if d > 0. && dn(&a, i) => pending = Some((0., i)),
            Some(d) if d < 0. && up(&a, i) => pending = Some((0., i)),
            Some(_) => {}
            None if up(&a, i) && up(&b, i) => pending = Some((1., i)),
            None if dn(&a, i) && dn(&b, i) => pending = Some((-1., i)),
            None => {}
        }
    }
    if let Some(mut t) = cur { t.exit_i = to; t.exit_px = x.c[to]; t.signal_i = to; t.reason = "open".into(); t.open = true; out.push(t); }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;
    #[test]
    fn enters_next_open_after_two_rising_days_and_exits_after_two_falling_days() {
        let mut r: Vec<Row> = (0..120).map(|i| Row { ticker: "T".into(), date: format!("{:04}", i), open: 100., high: 200., low: 1., close: 100., volume: None }).collect();
        // rising closes from bar 60 make both SMAs rise from bar 60; SMA25 rises on 60 and 61 -> signal on 61
        for k in 60..80 { r[k].close = 100. + (k - 59) as f64; }
        for k in 80..120 { r[k].close = 50.; }
        r[62].open = 111.;
        let x = Ctx::new(&r, &r, &[]);
        let ts = run("pwabal_abalone", &x, 0, 119, &Params::new()).unwrap();
        assert_eq!(ts[0].dir, 1.); assert_eq!(ts[0].legs[0].i, 62); assert_eq!(ts[0].legs[0].px, 111.);
        // closes collapse at bar 80: SMA25 falls on 80 and 81 -> exit at the open of 82
        assert_eq!(ts[0].exit_i, 82); assert_eq!(ts[0].signal_i, 81);
    }
}
