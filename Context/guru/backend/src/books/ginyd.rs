//! Ginyard NR11 system (ginyard.nu): `T:/Ginyard System for TS (ginyard.nu)/Code.rar` -> `Code.txt`; input notes in
//! `Ginyard.doc` (same folder). `T:` = research drive `USB HDD 1/TradeStation Systems & Indicators`. Book id "ginyd".
//! Notes: library/ginyd/README.md.
use crate::books::ajmisc::{avg_true_range, back, first_ok, highest, lowest, ord, Sim, Ty, K, POINT};
use crate::connors::{sma, Ctx, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "ginyd";
const ID: &str = "ginyd_nr11";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef { id: ID, name: "Ginyard NR11 (narrowest range of 11 bars, 50-day MA side, $ stop + adaptive exit)", group: "Ginyard (Code.txt + Ginyard.doc)", session: 0, dir: "Long/Short", status: Status::NotTestable("its stops and filters are in dollars per contract and the code never states the contract point value; reading $1 as 1 index point is our choice, not the vendor's"),
        needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK, rules: &[
        "F|\"Condition1 = RANGE < LOWEST(RANGE, NR)[1]\" (NR 11)",
        "E|\"If Condition1 and CLOSE > AVERAGE(CLOSE, 50) then Buy (\"NR11long\") Next Bar HIGH + 500 POINT STOP\"",
        "E|\"If Condition1 and CLOSE < AVERAGE(CLOSE, 50) then Sell Short (\"NR11short\") Next Bar LOW - 500 POINT STOP\"",
        "X|\"OrderPrice = ENTRYPRICE - (RiskCalc / BIGPOINTVALUE); Sell (\"$SL\") NEXT BAR OrderPrice STOP\" (DollarRisk 500 per position); short: \"Buy to Cover (\"$RS\") NEXT BAR ENTRYPRICE + SXRiskCalc/BIGPOINTVALUE STOP\"",
        "F|\"If CLOSE > AVGENTRYPRICE + (profittarget * atrr) and numofbars > daysfromentry then ptl = TRUE\" (profittarget 1, atrr = AvgTrueRange(14), daysfromentry 0); short mirror pts; both reset when flat",
        "X|\"If (mp = 1) and (ptl) then base = LOWEST(LOW, baselen); stoppoint = base + ((numofbars * atrr) * increment); Sell Next Bar stoppoint STOP\" (baselen 10, increment 0.05); short mirror"],
        notes: "Ginyard.doc explains the inputs (DollarRisk = 'dollars at risk on opening of trade'; adaptive exit 'starting point' and 'acceleration'). 500 POINT = 500 x 1/PriceScale = 5.00 index points (help.tradestation.com, Point). \
Dollar stops use p.bpv = 1 (TradeStation BigPointValue of a cash index): 500 index points, far wider than on a futures contract; p.bpv sets a futures point value instead. \
numofbars counts bars in the current market position (flat included); ptl/pts are cleared only on a flat bar, so a direct reversal keeps the flag (literal). AVGENTRYPRICE = 0 when flat. \
No market or bar size named: daily cash indices (instrument swap). Fills: vault TradeStation daily-bar convention (ajmisc.rs header). India: instrument swap; index shorts hypothetical.",
        claim: None }]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> { (id == ID).then_some((ID, "Ginyard NR11", "code")) }

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    (id == ID).then(|| vec![p("nr", "NR", 11., 3., 30., 1.), p("entry_pts", "Entry offset in points (500 x 0.01)", 500., 0., 5000., 50.), p("dollar_risk", "DollarRisk / SXDollarRisk ($)", 500., 50., 99999., 50.),
        p("bpv", "BigPointValue (1 = cash index)", 1., 1., 500., 1.), p("atrlen", "atrlen", 14., 5., 50., 1.), p("baselen", "baselen", 10., 3., 50., 1.), p("increment", "increment", 0.05, 0., 0.5, 0.01),
        p("profittarget", "profittarget (ATRs)", 1., 0., 5., 0.25), p("daysfromentry", "daysfromentry", 0., 0., 20., 1.)])
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if id != ID || from >= to || to >= x.r.len() { return None; }
    let g = |k: &str, d: f64| p.get(k).copied().unwrap_or(d);
    let nr = g("nr", 11.).max(1.) as usize;
    let (off, risk, bpv, inc, pt, dfe) = (g("entry_pts", 500.) * POINT, g("dollar_risk", 500.), g("bpv", 1.), g("increment", 0.05), g("profittarget", 1.), g("daysfromentry", 0.));
    let rg: Vec<f64> = (0..x.c.len()).map(|i| x.h[i] - x.l[i]).collect();
    let lr = back(&lowest(&rg, nr), 1);
    let ma = sma(&x.c, 50);
    let atr = avg_true_range(x, g("atrlen", 14.).max(1.) as usize);
    let bl = g("baselen", 10.).max(1.) as usize;
    let (lb, hb) = (lowest(&x.l, bl), highest(&x.h, bl));
    let mut s = Sim::new();
    let (mut mp1, mut nbars, mut ptl, mut pts) = (0i8, 0f64, false, false);
    for i in first_ok(x.c.len(), from, &[&lr, &ma, &atr, &lb])..to {
        let (c, mp, ep) = (x.c[i], s.pos, s.ep());
        let mut q = vec![];
        let c1 = rg[i] < lr[i];
        if c1 && c > ma[i] { q.push(ord(K::Buy, Ty::Stop(x.h[i] + off), "NR11long")); }
        if c1 && c < ma[i] { q.push(ord(K::Short, Ty::Stop(x.l[i] - off), "NR11short")); }
        if mp == -1 { q.push(ord(K::XS, Ty::Stop(ep + risk / bpv), "$RS")); }
        if mp == 1 { q.push(ord(K::XL, Ty::Stop(ep - risk / bpv), "$SL")); }
        if mp1 != mp { nbars = 1.; } else { nbars += 1.; }
        if c > ep + pt * atr[i] && nbars > dfe { ptl = true; }
        if c < ep - pt * atr[i] && nbars > dfe { pts = true; }
        if mp == 1 && ptl { q.push(ord(K::XL, Ty::Stop(lb[i] + nbars * atr[i] * inc), "adaptive exit")); }
        if mp == -1 && pts { q.push(ord(K::XS, Ty::Stop(hb[i] - nbars * atr[i] * inc), "adaptive exit")); }
        if mp == 0 { ptl = false; pts = false; }
        mp1 = mp;
        s.bar(x, i + 1, &q);
    }
    Some(s.done(x, to))
}
