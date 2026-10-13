//! John Tolan (Scientific Trading Solutions, trendchannel.com), "The Trendchannel Operating Manual" with the
//! "Trendchannel System Formula" in TradeStation 6 EasyLanguage (1999-2002), plus "Trendchannel Trading Guide" and
//! "TradingGuideTC" (performance notes). Book id `tcrsi`. Source (read in place, copy pending): research drive
//! "USB HDD 1/TradeStation Systems & Indicators/TrendChannel Pro System for TS + Manual (trendchannel.com)/
//! TrendChannel Operating Manual.pdf" (pp.1-5 of the printed web page).
//! RSI(close, 34) is Wilder's (the manual's own formula: first average = 34-day sum / 34, then (prev x 33 + today) / 34),
//! which is the engine's `rsi`. Orders are "at the market on the next day's open"; end-of-day settlement prices.
//! The EasyLanguage covers a short with "Rsi(close,34) crosses above 50" while the prose says "if Short and if RSI > 50";
//! a short is opened only below 45, so the first close above 50 is always a cross: both readings give the same trades.
use crate::connors::{claim, rsi, Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "tcrsi";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef {
        id: "tcrsi_trendchannel", name: "Trendchannel RSI(34) 55/45/50 trend system", group: "Trendchannel Operating Manual", session: 0, dir: "Long/Short",
        status: Status::NotTestable("duplicate of tchnl_rsi34 (the manual's RSI(34) TrendChannel formula): identical trades on NIFTY 50 and SPX, so it is listed once"), needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK,
        rules: &[
            "F|\"Set the daily bar RSI Parameter at 34. In other words, the 2 input values are: Close, 34.\" RSI per the manual: first average up/down close = 34-day sum / 34, then (previous x 33 + today) / 34",
            "E|\"Long signal: If RSI > 55, Then, buy at-the-market on the next day's open.\" EL: \"If MarketPosition<1 and Rsi(close,34)> 55 then Buy (\\\"LE\\\") tomorrow at the market;\"",
            "E|\"Short Signal: If RSI < 45, Then, sell at-the-market on the next day's open.\" EL: \"If MarketPosition>-1 and Rsi(close,34) < 45 then Sell Short (\\\"SE\\\") tomorrow at the market;\"",
            "X|\"Long liquidation signal: If Long and if RSI < 50, Then, sell at-the-market on the next day's open.\" EL: \"If MarketPosition>0 and Rsi(close,34) < 50 then Sell(\\\"LX\\\") tomorrow at the market;\"",
            "X|\"Short liquidation signal: If Short and if RSI > 50, Then, buy at-the-market on the next day's open.\" EL: \"If MarketPosition<0 and Rsi(close,34) crosses above 50 then Buy to Cover(\\\"SX\\\") tomorrow at the market;\"",
            "F|\"No position is held while the market is in a SIDE trend.\" \"Intra-day price penetration does not signal a trend change. No intra-day stops are used.\" Futures roll-forward trades are not modelled (index has no contract months)",
        ],
        notes: "Futures system (24 markets, Futures Truth test 11/1968-6/1999) run on the index (instrument swap, one unit, percentage returns). Published results are portfolio totals with futures roll trades included, so no index-level comparison exists; only the signal rate (2,671 trade signals over 24 markets and ~30.6 years = ~3.6 per market-year) is comparable.",
        claim: claim("2,671 signals (4,631 incl. rolls), 24 futures markets", "40.85% (incl. rolls)", "W/L 2.13, PF 1.5", "74% time in market", "11/1968-6/1999, Futures Truth (Excalibur), $75 per trade"),
    }]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    (id == "tcrsi_trendchannel").then_some(("tcrsi", "Trendchannel (Tolan)", "RSI 34 55/45/50"))
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    (id == "tcrsi_trendchannel").then(|| vec![
        p("len", "RSI length", 34., 5., 60., 1.), p("up", "Long when RSI above", 55., 50., 70., 1.),
        p("dn", "Short when RSI below", 45., 30., 50., 1.), p("mid", "Exit at RSI back through", 50., 40., 60., 1.),
    ])
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    let defs = params(id)?;
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| defs.iter().find(|d| d.key == k).map(|d| d.def).unwrap_or(f64::NAN));
    let to = to.min(x.c.len().saturating_sub(1));
    let r = rsi(&x.c, g("len").round().max(2.) as usize);
    let (up, dn, mid) = (g("up"), g("dn"), g("mid"));
    let (o, c) = (&x.o, &x.c);
    let mut out = Vec::new();
    let mut cur: Option<Trade> = None;
    let open_t = |dir: f64, i: usize, sig: usize| Trade { dir, legs: vec![Leg { i, px: o[i], w: 1.0 }], exit_i: i, exit_px: o[i], reason: String::new(), entry_kind: "open", signal_i: sig, open: false, exits: vec![] };
    for t in from.max(1)..to {
        if !r[t].is_finite() { continue; }
        let e = t + 1;
        let mp = cur.as_ref().map_or(0.0, |q| q.dir);
        // EasyLanguage order of evaluation at the close of t; all orders fill at the open of t + 1
        let le = mp < 1.0 && r[t] > up;
        let se = mp > -1.0 && r[t] < dn;
        let lx = mp > 0.0 && r[t] < mid;
        let sx = mp < 0.0 && r[t] > mid && r[t - 1] <= mid;
        let target = if le { 1.0 } else if se { -1.0 } else if lx || sx { 0.0 } else { mp };
        if target != mp {
            if let Some(mut q) = cur.take() {
                q.exit_i = e; q.exit_px = o[e]; q.signal_i = t;
                q.reason = match (q.dir > 0., target) { (true, t) if t < 0. => "reversed: RSI below 45", (true, _) => "RSI back below 50 (side trend)", (false, t) if t > 0. => "reversed: RSI above 55", _ => "RSI crossed above 50 (side trend)" }.into();
                out.push(q);
            }
            if target != 0.0 { cur = Some(open_t(target, e, t)); }
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
    use crate::Row;
    #[test]
    fn long_then_side_then_short() {
        // closes rise (RSI 100), then fall hard: long on the open after the first bar, exit/reverse later
        let mut c: Vec<f64> = (0..40).map(|i| 100. + i as f64).collect();
        c.extend((0..40).map(|i| 139. - 2. * i as f64));
        let rows: Vec<Row> = c.iter().enumerate().map(|(i, &v)| Row { ticker: "T".into(), date: format!("2000-01-{:02}", i % 28 + 1), open: v, high: v + 1., low: v - 1., close: v, volume: None }).collect();
        let x = Ctx::new(&rows, &rows, &[]);
        let t = run("tcrsi_trendchannel", &x, 0, rows.len() - 1, &Params::new()).unwrap();
        assert_eq!(t[0].dir, 1.0);
        assert_eq!(t[0].legs[0].i, 35); // RSI(34) first defined at bar 34
        assert!(t.iter().any(|q| q.dir < 0.));
    }
}
