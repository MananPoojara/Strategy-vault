//! Joe Corona, "Professional Options Trading College" (tradingmarkets.com), Week 5 lesson notes "VIX, VXN, and Direction/Volatility Combinations"
//! (research share Trading Courses,Seminars,Videos/Joe Corona - Professional Options Trading College (Videos & Manuals 11.2 GB), $2995 (tradingmarkets.com)/Week 5/Week 5 Notes.pdf).
//! Book id `jcvix`. Lane K3 (25 Nov 2026, R4 #409 row F1248, #400). Independent check authors/conv-1123/k3w/jcvix_ref.py.
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};
use super::rbtie::target_trades;

pub const BOOK: &str = "jcvix";
pub const REVIEWED: &[&str] = &["jcvix_sma10"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const NOTE: &str = "[Adapted: 4 unstated details, R4 #400] Corona's lesson-5 slides give the rule in a box marked 'My interpretation': the VIX closes 10% above its 10-day SMA -> buy the index on the close; exit when the VIX crosses back below its 10-day SMA; the mirror (VIX closes 10% below the SMA -> sell the index, exit when it crosses back above). Unstated, one per item: (1) the 10-day SMA of the VIX closes includes today's close; (2) 'crosses back below/above' = the first close at or below (long) / at or above (short) the SMA, exited on that close, and '10% above/below' is read as 10% or more for entry; (3) one position, no scaling in ('keep some powder dry, scale in' and the time stops 3/5/10 bars and the extra day 'for a further stretch' are optional extras in the notes, not applied; the printed tables' 'bars since crossover 0' row is this reading); (4) a day with no VIX bar leaves the position unchanged. The 10-day SMA runs over the VIX closes of the instrument's own trading days. Options adaptation, not counted: the course trades the signal with bull vertical and ratio call spreads and long puts; no option prices are held, so those option trades are not modelled and the card trades the underlying index. The slides' backtest tables carry no dates, so no printed figure is compared.";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef { id: "jcvix_sma10", name: "Corona VIX stretch: buy the index when the VIX closes 10% above its 10-day SMA, sell when 10% below, exit on the SMA cross", group: "Corona · Professional Options Trading College (Week 5)", session: 0, dir: "Long/Short",
        status: Status::Live, needs_volume: false, needs_vix: true, needs_series: &[], book: BOOK,
        rules: &[
            "E|\"VIX / VXN closes 10% above its 10-day SMA. Buy on the close (or reasonably close)\" (Week 5 Notes, 'Rules (buys)')",
            "X|\"Exit when VIX / VXN crosses back below its 10-day SMA\" (the printed tables list the 0- and 1-bar-after-crossover variants and 3/5/10-bar time stops)",
            "E|\"VIX / VXN closes 10% below its 10-day SMA. Sell Index on the close (or reasonably close)\" ('Rules (sells)')",
            "X|\"Exit when VIX / VXN crosses back above 10-day SMA\"",
        ], notes: NOTE, claim: None }]
}
pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> { reviewed(id).then_some(("jcvix", "Corona VIX stretch", "10% / 10-day SMA")) }
pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    reviewed(id).then(|| vec![ParamDef { key: "pct", label: "Stretch: VIX % beyond its 10-day SMA", def: 10., min: 3., max: 20., step: 1. }])
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from > to || to >= x.c.len() { return None; }
    let pct = p.get("pct").copied().unwrap_or(10.0);
    let t = pct / 100.0;
    let mut tgt = vec![0i8; to + 1];
    let mut pos = 0i8;
    for i in 0..=to {
        if x.vix.has_bar(i) {
            let s = x.vix.stretch_at(i);
            pos = match pos {
                1 if s <= 0.0 => 0,   // closes at or below the SMA
                -1 if s >= 0.0 => 0,  // closes at or above the SMA (the text's 'at or above')
                0 if s >= t => 1,     // '10% or more' above
                0 if s <= -t => -1,
                p => p,
            };
        }
        tgt[i] = pos;
    }
    Some(target_trades(x, &tgt, from, to, false, "VIX stretch signal"))
}

pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book == BOOK && reviewed(def.id),
    label: "Joe Corona's slides run the rule on the S&P 500 with the VIX, trading it with option spreads (not modelled: no option prices are held, the card trades the underlying) (and the Nasdaq-100 with the VXN); it runs on SPX and SPY with the VIX, and on NIFTY 50 and NIFTY BANK with India VIX as a declared adaptation (R4 #376).",
    basis: "Joe Corona, Professional Options Trading College, Week 5 Notes (Lesson 5: VIX, VXN and direction/volatility combinations, 'Rules' slides)",
};
