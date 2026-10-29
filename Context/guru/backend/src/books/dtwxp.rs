//! Trend XP System for TS (dream-trading.com). Source (research drive, read in place, read-only):
//! `USB HDD 1/TradeStation Systems & Indicators/Trend XP System for TS (dream-trading.com)/Open Code.rar` -> `Open Code.txt`
//! (extracted with the read-only libarchive reader; data only). No manual in the folder. Book id `dtwxp`.
//! Notes: library/dtwxp/README.md. Engine: books::rrsim.
use crate::books::rrsim::{g, o, pd, run_ts, sdef, A, P};
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "dtwxp";
/// Admitted ids (authors/lanea/vendor2/decisions.json).
pub const REVIEWED_LANEA: &[&str] = &["dtwxp_trend_xp"];
pub fn reviewed_lanea(id: &str) -> bool { REVIEWED_LANEA.contains(&id) }
const ID: &str = "dtwxp_trend_xp";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![sdef(BOOK, ID, "Trend XP (2-bar momentum turns, always in)", "Trend XP (Open Code.txt)", "Long/Short", Status::Live, &[
        "F|\"MoFactor = Momentum(Close, 2)\" (= Close - Close[2])",
        "E|\"BuyCondition = MoFactor > MoFactor[1] AND MoFactor > MoFactor[2]; IF BuyCondition = TRUE THEN Buy (\\\"Buy\\\") Cons Contracts Next Bar at Market\"",
        "E|\"SellCondition = MoFactor < MoFactor[1] AND MoFactor < MoFactor[2]; IF SellCondition = TRUE THEN Sell Short (\\\"Sell\\\") Cons Contracts Next Bar at Market\"",
        "X|No exit orders: a position closes only when the opposite entry reverses it",
        "F|\"INPUT: Multi(0)\": Multi = 0 -> Cons = 1 contract (the Multi = 1 volatility sizing is off by default)",
    ], "Open Code.txt is the complete strategy (no stops, no targets). Timeframe is not stated in the folder; run on daily bars. \
Fills: market orders at the next open; Buy/Sell Short reverse (TradeStation). India: instrument swap (NIFTY 50 instead of the futures it was sold for); index shorts hypothetical.")]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    (id == ID).then_some(("dtwxp", "Trend XP (dream-trading.com)", "code literal"))
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    (id == ID).then(|| vec![pd("mom_len", "Momentum length", 2., 1., 20., 1.)])
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if id != ID || from >= to || to >= x.r.len() { return None; }
    let n = g(p, "mom_len", 2.).max(1.) as usize;
    let mo: Vec<f64> = (0..x.c.len()).map(|i| if i >= n { x.c[i] - x.c[i - n] } else { f64::NAN }).collect();
    Some(run_ts(x, from, to, |i, _st, _now, next| {
        if i < 2 { return; }
        if mo[i] > mo[i - 1] && mo[i] > mo[i - 2] { next.push(o(A::Buy, P::Mkt, "Buy")); }
        if mo[i] < mo[i - 1] && mo[i] < mo[i - 2] { next.push(o(A::Sell, P::Mkt, "Sell")); }
    }))
}
