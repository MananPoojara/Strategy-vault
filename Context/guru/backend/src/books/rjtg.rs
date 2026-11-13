//! Ryan Jones, *The Trading Game* (Wiley 1999; same text sold as *Money Management*). Source + audit: library/rjtg/.
//! The book is about position sizing (Fixed Fractional, Fixed Ratio - not strategies). Its one trading method is
//! "A Simple Trading Method" (ch.13 p.186-190): slope-of-average pullback entry at the next open, stop and reverse.
use crate::books::rjmil::slope_sign;
use crate::connors::{Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "rjtg";

const NOTE: &str = "Book market: eight futures (Bonds, Swiss Franc, D-Mark, Eurodollar, Crude Oil, Wheat, JY, 10-Year Notes; p.188-189), test period and the X/Y used for the printed results not stated. \
Defaults X=20, Y=3 are the book's only numbers (the worked example, p.186-190: 'For example, if X = 20 and Y = 3'); the book says X and Y may be anything that fits the logic. \
Runs on the indices (instrument swap). Signals on the close, fills at the next open (p.187 'buy on the open the next day'). Always in once the first setup fires; a same-direction setup while already in that direction is ignored. \
NOT APPLIED: the book's 'set protective stop' ('unless the market goes against the position too far', p.186, 190) - its size is never stated, so no number can be copied; trades end only on the reverse setup.";

pub fn catalogue() -> Vec<StrategyDef> {
    let d = |id, name, group, dir, status, rules, notes| StrategyDef {
        id, name, group, session: 0, dir, status, needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK, rules, notes, claim: None,
    };
    vec![
        d("rjtg_simple", "A Simple Trading Method (rising average + pullback, stop and reverse)", "Ch.13 The System · A Simple Trading Method", "Long/Short", Status::NotTestable("the book uses a protective stop on every trade but never gives its size, and X=20 / Y=3 are only its worked example; running it without the stop would change the book's rule, and the vault does not invent one"), &[
            "E|Buys: \"1. Requires the average close of X days to be greater than the same average close Y days ago. 2. Requires the close to be less than the close Y days ago. 3. Requires the close to be greater than the close Y + X days ago. If these three conditions are met, the method will buy on the open the next day.\" (p.187)",
            "E|Sells: \"1. Requires the average close of X days to be less than the same average close Y days ago. 2. Requires the close to be greater than the close Y days ago. 3. Requires the close to be less than the close Y + X days ago. If these three conditions are met, the method will sell on the open the next day.\" (p.187)",
            "X|\"This is a trend-following method and reverses. There are no exit rules other than a reversing situation and a set protective stop.\" (p.186) - reverse at the next open on the opposite setup.",
            "X|[Not applied] protective stop: size not stated anywhere in the book (p.186, 190).",
        ], NOTE),
        d("rjtg_ma_cross_example", "10/40 moving average crossover (optimization illustration)", "Ch.14 Optimization", "Long/Short", Status::NotTestable(
            "duplicate / illustration: a plain 10/40 SMA crossover used only to illustrate over-optimization (p.191-193); the vault already has SMA crossover systems (e.g. kftsm_hh_cross, cov_tfm_ma_cross_50_75 with editable lengths)"), &[
            "E|\"If the 10-day moving average crosses above the 40-day moving average, you buy. If the 10-day moving average crosses below the 40-day moving average; you sell.\" (p.191-192)"],
            "Bond-futures example only; not one of the author's methods."),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    match id {
        "rjtg_simple" => Some(("rjtg_simple", "Trading Game simple method", "Book rules")),
        "rjtg_ma_cross_example" => Some(("rjtg_simple", "Trading Game simple method", "MA cross illustration")),
        _ => None,
    }
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    match id {
        "rjtg_simple" => Some(vec![
            p("x", "X: average length (book example 20)", 20.0, 5.0, 100.0, 1.0),
            p("y", "Y: pullback lookback (book example 3)", 3.0, 1.0, 20.0, 1.0),
        ]),
        _ => None,
    }
}

fn tr(dir: f64, i: usize, px: f64, sig: usize) -> Trade {
    Trade { dir, legs: vec![Leg { i, px, w: 1.0 }], exit_i: i, exit_px: px, reason: String::new(), entry_kind: "open", signal_i: sig, open: false, exits: vec![] }
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if id != "rjtg_simple" { return None; }
    let schema = params(id)?;
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).unwrap().def);
    let n = x.c.len();
    if n == 0 || to >= n || from > to { return Some(vec![]); }
    let xn = (g("x") as usize).max(1);
    let y = (g("y") as usize).max(1);
    let c = &x.c;
    let mut out = vec![];
    let mut cur: Option<Trade> = None;
    let mut pending: Option<(f64, usize)> = None;
    for t in 0..=to {
        if let Some((d, s)) = pending.take() {
            if let Some(mut o) = cur.take() {
                o.exit_i = t; o.exit_px = x.o[t]; o.signal_i = s; o.reason = "Reverse setup: next open".into(); out.push(o);
            }
            cur = Some(tr(d, t, x.o[t], s));
        }
        if t == to || t < xn + y { continue; }
        let Some(sl) = slope_sign(c, t, xn, y) else { continue };
        let buy = sl > 0 && c[t] < c[t - y] && c[t] > c[t - y - xn];
        let sell = sl < 0 && c[t] > c[t - y] && c[t] < c[t - y - xn];
        let d = if buy { 1.0 } else if sell { -1.0 } else { continue };
        if cur.as_ref().map_or(true, |o| o.dir != d) { pending = Some((d, t)); }
    }
    if let Some(mut o) = cur { o.exit_i = to; o.exit_px = c[to]; o.signal_i = to; o.reason = "open".into(); o.open = true; out.push(o); }
    out.retain(|t| t.legs[0].i >= from);
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;
    #[test]
    fn rising_average_pullback_buys_next_open() {
        // steady rise, then a 3-day dip below close[t-3] but above close[t-23] -> buy at the next open
        let mut r: Vec<Row> = (0..40).map(|i| { let v = 100.0 + i as f64; Row { ticker: "T".into(), date: format!("2000-01-{:02}", 1 + i % 28), open: v, high: v + 1.0, low: v - 1.0, close: v, volume: None } }).collect();
        r[30].close = 126.0; // < close[27]=127, > close[7]=107, sma20 still rising
        r[31].open = 125.5;
        let x = Ctx::new(&r, &r, &[]);
        let mut p = Params::new(); p.insert("x".into(), 20.0); p.insert("y".into(), 3.0);
        let ts = run("rjtg_simple", &x, 0, 39, &p).unwrap();
        assert_eq!((ts[0].dir, ts[0].legs[0].i, ts[0].legs[0].px), (1.0, 31, 125.5));
    }
}
