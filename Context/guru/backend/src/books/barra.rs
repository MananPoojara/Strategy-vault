//! Baramundi (TradeStation code; `T:/Baramundi System for TS/Open Code.rar` -> `Open Code.txt`) and the identical
//! Intervest-ES strategy (`T:/Intervest ES System for TS (intervestsecurities.com)/Open_Code.rar` -> `Open_Code.txt`:
//! same inputs 48/2/12 and logic, TS6 'Sell'/'Buy to Cover' wording). `T:` = research drive
//! `USB HDD 1/TradeStation Systems & Indicators`. Book id "barra". Notes: library/barra/README.md.
use crate::books::ajmisc::{back, first_ok, highest, lowest, ord, Sim, Ty, K};
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "barra";
const ID: &str = "barra_baramundi";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef { id: ID, name: "Baramundi / Intervest-ES (second 48-bar breakout bar, buy back at the channel)", group: "Baramundi (Open Code.txt) = Intervest-ES (Open_Code.txt)", session: 0, dir: "Long", status: Status::Live,
        needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK, rules: &[
        "F|\"If (LCounter >= 1 AND High > HighChannel) OR (LCounter = 0 AND High > Highest(High,Length)[1]) then LCounter = LCounter+1; SCounter = 0; If LCounter = 1 then HighChannel = Highest(High,Length)[1]\" (Length 48)",
        "F|\"If (SCounter >= 1 AND Low < LowChannel) OR (SCounter = 0 AND Low < Lowest(Low,Length)[1]) then SCounter = SCounter+1; LCounter = 0; If SCounter = 1 then LowChannel = Lowest(Low,length)[1]\"",
        "E|\"If LCounter >= Numbr then Buy next bar at HighChannel Stop\" (Numbr 2)",
        "X|\"If SCounter >= Numbr then ExitLong next bar at LowChannel Stop\" (Intervest-ES: 'Sell next bar at LowChannel Stop')",
        "F|\"If MarketPosition = 1 then HighChannel = 0; LCounter = 0\"",
        "X|\"If MarketPosition = 1 then ExitLong next bar at Lowest(Low,StopBars) stop\" (StopBars 12)"],
        notes: "Long only: neither file has a short entry (the ExitShort / Buy to Cover lines can never act). Intervest-ES is the same code under another vendor name (inputs 48/2/12), so it is one strategy here. \
Neither file names a bar size; Intervest-ES names the E-mini S&P. Run on daily cash indices (instrument swap). Fills: vault TradeStation daily-bar convention (ajmisc.rs header).",
        claim: None }]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> { (id == ID).then_some((ID, "Baramundi / Intervest-ES", "code")) }

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    (id == ID).then(|| vec![p("length", "Length", 48., 10., 150., 1.), p("numbr", "Numbr", 2., 1., 6., 1.), p("stopbars", "StopBars", 12., 3., 50., 1.)])
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if id != ID || from >= to || to >= x.r.len() { return None; }
    let g = |k: &str, d: f64| p.get(k).copied().unwrap_or(d);
    let (len, nb, sb) = (g("length", 48.).max(1.) as usize, g("numbr", 2.).max(1.) as i64, g("stopbars", 12.).max(1.) as usize);
    let (hh, ll, lsb) = (back(&highest(&x.h, len), 1), back(&lowest(&x.l, len), 1), lowest(&x.l, sb));
    let mut s = Sim::new();
    let (mut hc, mut lc, mut lcn, mut scn) = (0f64, 9999f64, 0i64, 0i64);
    for i in first_ok(x.c.len(), from, &[&hh, &lsb])..to {
        let (h, l) = (x.h[i], x.l[i]);
        if (lcn >= 1 && h > hc) || (lcn == 0 && h > hh[i]) { lcn += 1; scn = 0; if lcn == 1 { hc = hh[i]; } }
        if (scn >= 1 && l < lc) || (scn == 0 && l < ll[i]) { scn += 1; lcn = 0; if scn == 1 { lc = ll[i]; } }
        let mut q = vec![];
        if lcn >= nb { q.push(ord(K::Buy, Ty::Stop(hc), "HighChannel stop")); }
        if scn >= nb { q.push(ord(K::XL, Ty::Stop(lc), "LowChannel stop")); }
        if s.pos == 1 { hc = 0.; lcn = 0; q.push(ord(K::XL, Ty::Stop(lsb[i]), "Lowest(Low,12) stop")); }
        s.bar(x, i + 1, &q);
    }
    Some(s.done(x, to))
}
