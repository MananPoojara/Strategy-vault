//! Andromeda (andromedafutures.com, TradeStation 2000i+): `T:/Andromeda Tradestation Systems + Manual (andromedafutures.com)/
//! Open Code.rar` -> `Open Code.txt`; manual `Andromeda Trading Manual.pdf` (p.2-6: daily back-adjusted futures, orders at the
//! next open, MaxBarsBack 120, Initial Risk Limit $4000). `T:` = research drive `USB HDD 1/TradeStation Systems & Indicators`.
//! Book id "andro". Notes: library/andro/README.md.
use crate::books::ajmisc::{back, first_ok, highest, lowest, ord, stdev_pop, Sim, Ty, K};
use crate::connors::{sma, Ctx, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "andro";
const ID: &str = "andro_andromeda";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef { id: ID, name: "Andromeda (50-close breakout beyond the 100-day 1-sigma band, staged MA exits)", group: "Andromeda (Open Code.txt + Trading Manual)", session: 0, dir: "Long/Short", status: Status::Live,
        needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK, rules: &[
        "F|\"Value1=Average(Close,100); Value2=Highest(Close,Donchian_Entry)[1]; Value3=Lowest(Close,Donchian_Entry)[1]\" (Donchian_Entry 50); \"Upper_Band = Value1 + stddev(Close,100); Lower_Band = Value1 - stddev(Close,100)\"",
        "E|\"If MarketPosition = 0 and Close > Value2 and Close > Upper_Band and (Close - Value1)*BigPointValue < Initial_Risk_Limit then Buy Next Bar at Market\" (Initial_Risk_Limit 4000)",
        "E|\"If MarketPosition = 0 and Close < Value3 and Close < Lower_Band and (Value1 - Close)*BigPointValue < Initial_Risk_Limit then Sell Short Next Bar at Market\"",
        "X|Long: \"(Close - EntryPrice)*BigPointValue > 10000\" (LX 10K); \"BarsSinceEntry(0) > 99 and Close < Average(Close,25)\"; \"> 74 and Close < Average(Close,50)\"; \"> 49 and Close < Average(Close,75)\"; \"Close < Value1\" (LX 100MA) -> Sell Next Bar at Market",
        "X|Short: mirror (SX 10K, SX 25MA/50MA/75MA, SX 100MA) -> Buy to Cover Next Bar at Market"],
        notes: "[Literal code; R4 #293 (1), (3)] Runs only on Norgate continuous futures (back-adjusted daily bars, as the manual requires: 'back-adjusted continuous contract files', CSI Unfair Advantage) of The Trading Plan's 8-market portfolio: Swiss franc, crude oil, 10-year T-note, corn, Japanese yen, US Dollar Index, Eurodollar, cotton (the 16-market portfolio's municipal bonds are not held, so it is not run as a whole). BigPointValue = the contract's US$ per point on the bar's date (futures::MARKETS, exchange specifications), so the $4,000 Initial Risk Limit and the $10,000 profit exit are the vendor's dollar rules. All orders at the next open; stddev = TradeStation StdDev (population). Norgate continuous futures, ends 2020-09-11; roll method not stated by the vendor. No costs in the engine (the manual deducts $100 per trade). p.risk = the manual's other Initial Risk Limits ($2,000 / $3,000 in its small and mid accounts).",
        claim: None }]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> { (id == ID).then_some((ID, "Andromeda (andromedafutures.com)", "code")) }

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    (id == ID).then(|| vec![p("risk", "Initial_Risk_Limit ($)", 4000., 500., 99999., 500.), p("donchian", "Donchian_Entry", 50., 10., 150., 5.),
        p("profit", "Profit exit ($)", 10000., 1000., 100000., 1000.), p("bpv", "BigPointValue (1 = cash index)", 1., 1., 500., 1.)])
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if id != ID || from >= to || to >= x.r.len() { return None; }
    let g = |k: &str, d: f64| p.get(k).copied().unwrap_or(d);
    let (risk, prof) = (g("risk", 4000.), g("profit", 10000.));
    // lane PF: on a futures context BigPointValue is the contract's US$ per point on the bar's date (#293 (3)); p.bpv otherwise
    let mk = crate::futures::ctx_market(x);
    let bpv_at = |i: usize| mk.map_or(g("bpv", 1.), |m| crate::futures::usd_per_point(m, &x.r[i].date).unwrap_or(f64::NAN));
    let dn = g("donchian", 50.).max(1.) as usize;
    let v1 = sma(&x.c, 100);
    let (v2, v3) = (back(&highest(&x.c, dn), 1), back(&lowest(&x.c, dn), 1));
    let (v4, v5, v6) = (sma(&x.c, 25), sma(&x.c, 50), sma(&x.c, 75));
    let sd = stdev_pop(&x.c, 100);
    let n = x.c.len();
    let mut s = Sim::new();
    for i in first_ok(n, from, &[&v1, &v2, &sd])..to {
        let (c, mp, bse, ep, bpv) = (x.c[i], s.pos, s.bse(i), s.ep(), bpv_at(i));
        let (up, lo) = (v1[i] + sd[i], v1[i] - sd[i]);
        let mut q = vec![];
        if mp == 0 && c > v2[i] && c > up && (c - v1[i]) * bpv < risk { q.push(ord(K::Buy, Ty::Mkt, "LE")); }
        if mp == 0 && c < v3[i] && c < lo && (v1[i] - c) * bpv < risk { q.push(ord(K::Short, Ty::Mkt, "SE")); }
        if mp == 1 {
            if (c - ep) * bpv > prof { q.push(ord(K::XL, Ty::Mkt, "LX 10K")); }
            if bse > 99 && c < v4[i] { q.push(ord(K::XL, Ty::Mkt, "LX 25MA")); }
            if bse > 74 && c < v5[i] { q.push(ord(K::XL, Ty::Mkt, "LX 50MA")); }
            if bse > 49 && c < v6[i] { q.push(ord(K::XL, Ty::Mkt, "LX 75MA")); }
            if c < v1[i] { q.push(ord(K::XL, Ty::Mkt, "LX 100MA")); }
        }
        if mp == -1 {
            if (ep - c) * bpv > prof { q.push(ord(K::XS, Ty::Mkt, "SX 10K")); }
            if bse > 99 && c > v4[i] { q.push(ord(K::XS, Ty::Mkt, "SX 25MA")); }
            if bse > 74 && c > v5[i] { q.push(ord(K::XS, Ty::Mkt, "SX 50MA")); }
            if bse > 49 && c > v6[i] { q.push(ord(K::XS, Ty::Mkt, "SX 75MA")); }
            if c > v1[i] { q.push(ord(K::XS, Ty::Mkt, "SX 100MA")); }
        }
        s.bar(x, i + 1, &q);
    }
    Some(s.done(x, to))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;
    #[test]
    fn dollar_risk_filter_uses_the_contract_point_value() {
        // Unit-test fixture only (no market data): corn (US$50 per point) flat at 300 for 130 bars, then a close of 380.
        // (C - SMA100) = 80 - 0.8 = 79.2 points = US$3,960 < 4,000 -> buy next open; at a US$3,000 limit it is filtered out.
        let n = 140;
        let px = |i: usize| if i < 130 { 300. } else { 380. };
        let r: Vec<Row> = (0..n).map(|i| Row { ticker: "Corn (Norgate &ZC)".into(), date: format!("{}-{:02}-{:02}", 1990 + i / 250, 1 + (i % 250) / 21, 1 + i % 21), open: px(i), high: px(i), low: px(i), close: px(i), volume: None }).collect();
        let x = Ctx::new(&r, &r, &[]);
        let t = run(ID, &x, 0, n - 1, &Params::new()).unwrap();
        assert_eq!((t.len(), t[0].dir, t[0].legs[0].i), (1, 1.0, 131));
        let mut p = Params::new(); p.insert("risk".into(), 3000.);
        assert!(run(ID, &x, 0, n - 1, &p).unwrap().is_empty());
    }
}
