//! AxSym (i8i.com): `T:/AxSym for TS (i8i.com)/Open Code.rar` -> `Open Code.txt` (TradeStation code; no manual in the folder).
//! `T:` = research drive `USB HDD 1/TradeStation Systems & Indicators`. Book id "axsym". Notes: library/axsym/README.md.
use crate::books::ajmisc::{ord, Sim, Ty, K};
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "axsym";
const ID: &str = "axsym_gap_breakout";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef { id: ID, name: "AxSym (opening gap > 30% of range -> open +/- range stop, entry-day high/low exits)", group: "AxSym (Open Code.txt)", session: 0, dir: "Long/Short", status: Status::Live,
        needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK, rules: &[
        "F|\"Value1 = .3*Range; Value2 = close - open of tomorrow; Value3 = open of tomorrow - close; Value6 = Open of tomorrow + (Buycent/100)*Range; Value7 = Open of tomorrow - (Sellcent/100)*Range\" (Buycent = Sellcent = 100)",
        "E|\"If MarketPosition = 0 and Value2 > Value1 then Buy tomorrow at Value6 stop\"; \"If MarketPosition = -1 and Value2 > Value1 then Buy (\"Reverse Buy\") tomorrow at Value6 stop\"",
        "E|\"If MarketPosition = 0 and Value3 > Value1 then Sell Short tomorrow at Value7 stop\"; \"If MarketPosition = +1 and Value3 > Value1 then Sell Short (\"Reverse Sell\") tomorrow at Value7 stop\"",
        "F|\"If MarketPosition = +1 (or -1) and BarsSinceEntry = 0 then EntryDayHigh = High; EntryDayLow = Low\"",
        "X|Long: \"Sell (\"PROFIT long\") Next Bar at EntryDayHigh Limit; Sell (\"long STOP\") Next Bar at EntryDayLow Stop\"",
        "X|Short: \"Buy to Cover (\"PROFIT short\") Next Bar at EntryDayLow Limit; Buy to Cover (\"short STOP\") Next Bar at EntryDayHigh Stop\""],
        notes: "Reading of the code: an opening gap DOWN of more than 30% of today's range arms a buy stop one range above tomorrow's open (and a gap up arms a sell-short stop one range below); \
the 'Reentry' orders in the code apply to the direction already held and do nothing (no pyramiding). 'open of tomorrow' is the next bar's open, known when the stop is placed at the open. \
No bar size or market named: run on daily cash indices (instrument swap). Fills: vault TradeStation daily-bar convention (ajmisc.rs header). India: instrument swap; index shorts hypothetical.",
        claim: None }]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> { (id == ID).then_some((ID, "AxSym (i8i.com)", "code")) }

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    (id == ID).then(|| vec![p("gap_frac", "Gap filter (fraction of range, code .3)", 0.3, 0., 1., 0.05), p("buycent", "Buycent", 100., 10., 200., 10.), p("sellcent", "Sellcent", 100., 10., 200., 10.)])
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if id != ID || from >= to || to >= x.r.len() { return None; }
    let g = |k: &str, d: f64| p.get(k).copied().unwrap_or(d);
    let (gf, bc, sc) = (g("gap_frac", 0.3), g("buycent", 100.), g("sellcent", 100.));
    let mut s = Sim::new();
    let (mut edh, mut edl) = (0f64, 0f64);
    for i in from..to {
        let (rg, c, o1, mp) = (x.h[i] - x.l[i], x.c[i], x.o[i + 1], s.pos);
        let (v1, v2, v3) = (gf * rg, c - o1, o1 - c);
        let (v6, v7) = (o1 + bc / 100. * rg, o1 - sc / 100. * rg);
        let mut q = vec![];
        if mp <= 0 && v2 > v1 { q.push(ord(K::Buy, Ty::Stop(v6), if mp == 0 { "Buy" } else { "Reverse Buy" })); }
        if mp >= 0 && v3 > v1 { q.push(ord(K::Short, Ty::Stop(v7), if mp == 0 { "Sell" } else { "Reverse Sell" })); }
        if mp != 0 && s.bse(i) == 0 { edh = x.h[i]; edl = x.l[i]; }
        if mp == 1 { q.push(ord(K::XL, Ty::Lim(edh), "PROFIT long")); q.push(ord(K::XL, Ty::Stop(edl), "long STOP")); }
        if mp == -1 { q.push(ord(K::XS, Ty::Lim(edl), "PROFIT short")); q.push(ord(K::XS, Ty::Stop(edh), "short STOP")); }
        s.bar(x, i + 1, &q);
    }
    Some(s.done(x, to))
}
