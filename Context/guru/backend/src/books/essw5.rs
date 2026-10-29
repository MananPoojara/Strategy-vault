//! Erik Skyba, CMT, "Swing 5 - Relative Strength Index Strategy" (TradeStation Labs, Technical Focus, 15 June 2010; saved
//! web page, book id `essw5`). Reading: lane K2, claude-continuation-20261020/authors/skyba (admission/sw5_ref.py).
//! Live: the whole strategy ('Defining Strategy Rules' and 'Strategy Inputs'): RSI(5) under 30 above the 200-day EMA,
//! or under 25 below it on a long black candle; out when RSI(5) crosses above 40. Book: SPY daily 5/2000-5/2010, 100
//! shares; other indices, ETFs and US/India stocks are a declared application.
use crate::books::apta::ema_first;
use crate::connors::{rsi, Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};
/// Admitted ids (authors/skyba/admission/sw5_ref.py (header)).
pub const REVIEWED: &[&str] = &["essw5_rsi5"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const NOTE: &str = "[Declared readings] Signals on the daily close, filled at the next day's open (TradeStation strategy orders; the page does not state the order). RSI = Wilder's 5-day RSI; the 200-day exponential average starts from the first close (TradeStation XAverage). Condition 2's long black candle uses 0.70 x the day's range as the rule text says twice; the page's input table lists PercentofBar = 0.75 (option). 'Crosses above 40' = RSI above 40 today and at or below 40 yesterday. One position at a time, equal weight (the page traded 100 shares); a position still open is marked at the last close.";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef { id: "essw5_rsi5", name: "Swing 5 RSI: buy RSI(5) under 30 above the 200-day EMA, or under 25 below it on a long black candle; sell when RSI(5) crosses 40",
        group: "Skyba Swing 5 RSI", session: 0, dir: "Long", status: Status::Live, needs_volume: false, needs_vix: false, needs_series: &[], book: "essw5", rules: &[
            "E|\"Buy Condition 1 - if the closing price is greater than its 200 day exponential moving average and the Relative Strength Index < 30\" (RSI Length 5)",
            "E|\"Buy Condition 2 - if the closing price is less than its 200 day exponential moving average and the Relative Strength Index < 25 and absolute(open - close) > .70 * (High - Low) and close < open\"",
            "X|\"Exit a long position when the Relative Strength Index crosses above 40\" (Sell_Level 40)",
            "F|Inputs printed: OSLevel 25 / 30, Sell_Level 40, EMALength 200, PercentofBar 0.75, RSI Length 5; tested on SPY daily 5/15/2000-5/13/2010, 100 shares"],
        notes: NOTE, claim: None }]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    (id == "essw5_rsi5").then_some(("essw5_rsi5", "Skyba Swing 5 RSI", "RSI 5, EMA 200, 30/25, exit 40"))
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    (id == "essw5_rsi5").then(|| vec![
        p("pct", "Black-candle body share of the range (rule text 0.70; input table 0.75)", 0.70, 0.1, 1., 0.05),
        p("os_above", "RSI buy level above the EMA (30)", 30., 5., 50., 1.), p("os_below", "RSI buy level below the EMA (25)", 25., 5., 50., 1.),
        p("exit", "RSI exit level (40)", 40., 20., 80., 1.)])
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.r.len() { return None; }
    let schema = params(id)?;
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).map_or(f64::NAN, |d| d.def));
    let (o, h, l, c) = (&x.o, &x.h, &x.l, &x.c);
    let (r, e) = (rsi(c, 5), ema_first(c, 200));
    let (pct, up_lvl, dn_lvl, ex) = (g("pct"), g("os_above"), g("os_below"), g("exit"));
    let (mut out, mut pos) = (vec![], None::<(usize, f64)>);
    for t in from.max(1)..to {
        if let Some((ei, ep)) = pos {
            if r[t] > ex && r[t - 1] <= ex {
                out.push(Trade { dir: 1., legs: vec![Leg { i: ei, px: ep, w: 1. }], exit_i: t + 1, exit_px: o[t + 1], reason: "RSI crossed above the exit level".into(),
                    entry_kind: "open", signal_i: t, open: false, exits: vec![] });
                pos = None;
            }
            continue;
        }
        let buy1 = c[t] > e[t] && r[t] < up_lvl;
        let buy2 = c[t] < e[t] && r[t] < dn_lvl && (o[t] - c[t]).abs() > pct * (h[t] - l[t]) && c[t] < o[t];
        if buy1 || buy2 { pos = Some((t + 1, o[t + 1])); }
    }
    if let Some((ei, ep)) = pos {
        out.push(Trade { dir: 1., legs: vec![Leg { i: ei, px: ep, w: 1. }], exit_i: to, exit_px: c[to], reason: "open".into(), entry_kind: "open", signal_i: to, open: true, exits: vec![] });
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn ids_and_params() {
        assert!(reviewed("essw5_rsi5") && !reviewed("essw5_x"));
        assert_eq!(params("essw5_rsi5").unwrap()[0].def, 0.70);
    }
}
