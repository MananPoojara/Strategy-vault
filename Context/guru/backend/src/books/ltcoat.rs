//! Coat Tails (longtermtrading.com, "Strategy: Coat Tails"; entries labelled tt30): `T:/Coat Tails System for TS
//! (lomgtermtrading.com)/Open Code.rar` -> `Open Code.txt`. `T:` = research drive `USB HDD 1/TradeStation Systems & Indicators`.
//! Book id "ltcoat". Notes: library/ltcoat/README.md. Related but not identical: lgtt30_tt30 (Gettess TT30, printed code).
use crate::books::ajmisc::{first_ok, highest, lowest, ord, Sim, Ty, K};
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "ltcoat";
const ID: &str = "ltcoat_coat_tails";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef { id: ID, name: "Coat Tails (3-day range breakout from the open, profitable-open exit)", group: "Coat Tails (Open Code.txt)", session: 0, dir: "Long/Short", status: Status::Live,
        needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK, rules: &[
        "F|\"Value2 = Highest(high,3) - Lowest(close,3); Value3 = Highest(close,3) - Lowest(low,3); Value4 = Maxlist(Value2,Value3); lePrice = Open next bar + Value4/2\"",
        "E|\"If MarketPosition <> +1 then Buy tomorrow at lePrice stop\"",
        "F|Short: \"Value8 = Minlist(Value6,Value7); sePrice = Open next bar - Value8/2\" (the SMALLER of the two ranges)",
        "E|\"If MarketPosition <> -1 then Sell Short tomorrow at sePrice stop\"",
        "X|Long: on the entry bar \"lxPTarget = High[0]; lxPStop = Low[1]\"; \"If Open next bar > Entryprice then Sell tomorrow at market\"; \"Sell Next Bar at lxPTarget Limit\"; \"Sell Next Bar at lxPStop Stop\"",
        "X|Short: \"sxPTarget = Low[0]; sxPStop = High[1]\"; \"If Open next bar < Entryprice then Buy to Cover tomorrow at market\"; cover limit at sxPTarget, cover stop at sxPStop"],
        notes: "The TT30 'Kris-Kross' idea (cf. lgtt30_tt30) as sold by longtermtrading.com, with differences that make it a separate system: the short stop uses the smaller of the two 3-day ranges, there is no 'goodgap' exit and no price rounding, and only the entry in the direction already held is blocked. \
'Open next bar' is known when the stop is placed at the open. No bar size or market named: run on daily cash indices (instrument swap). Fills: vault TradeStation daily-bar convention (ajmisc.rs header). India: instrument swap; index shorts hypothetical.",
        claim: None }]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> { (id == ID).then_some((ID, "Coat Tails (longtermtrading.com)", "code")) }

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    (id == ID).then(|| vec![ParamDef { key: "len", label: "Range lookback (code 3)", def: 3., min: 1., max: 10., step: 1. }, ParamDef { key: "frac", label: "Range fraction (code 1/2)", def: 0.5, min: 0.1, max: 2., step: 0.05 }])
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if id != ID || from >= to || to >= x.r.len() { return None; }
    let g = |k: &str, d: f64| p.get(k).copied().unwrap_or(d);
    let (n, fr) = (g("len", 3.).max(1.) as usize, g("frac", 0.5));
    let (hh, lc, hc, ll) = (highest(&x.h, n), lowest(&x.c, n), highest(&x.c, n), lowest(&x.l, n));
    let mut s = Sim::new();
    let (mut lt, mut ls, mut st, mut ss) = (0f64, 0f64, 0f64, 0f64);
    for i in first_ok(x.c.len(), from.max(1), &[&hh])..to {
        let (o1, mp) = (x.o[i + 1], s.pos);
        let (v2, v3) = (hh[i] - lc[i], hc[i] - ll[i]);
        let mut q = vec![];
        if mp != 1 { q.push(ord(K::Buy, Ty::Stop(o1 + v2.max(v3) * fr), "le.TT30")); }
        if mp == 1 {
            if s.bse(i) == 0 { lt = x.h[i]; ls = x.l[i - 1]; }
            if o1 > s.ep() { q.push(ord(K::XL, Ty::Mkt, "Lx.ProfOp")); }
            q.push(ord(K::XL, Ty::Lim(lt), "lx.pTarget"));
            q.push(ord(K::XL, Ty::Stop(ls), "lx.pStop"));
        }
        if mp != -1 { q.push(ord(K::Short, Ty::Stop(o1 - v2.min(v3) * fr), "se.TT30")); }
        if mp == -1 {
            if s.bse(i) == 0 { st = x.l[i]; ss = x.h[i - 1]; }
            if o1 < s.ep() { q.push(ord(K::XS, Ty::Mkt, "Sx.ProfOp")); }
            q.push(ord(K::XS, Ty::Lim(st), "sx.pTarget"));
            q.push(ord(K::XS, Ty::Stop(ss), "sx.pStop"));
        }
        s.bar(x, i + 1, &q);
    }
    Some(s.done(x, to))
}
