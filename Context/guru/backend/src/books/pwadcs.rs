//! DCS II (pwafutures.com, Arlington TX): `T:/DCS II System for TS (pwafutures.com)/Open Code.rar` -> `Open Code.txt`
//! (TradeStation 2000i code). `T:` = research drive `USB HDD 1/TradeStation Systems & Indicators`. Book id "pwadcs".
//! Notes: library/pwadcs/README.md.
use crate::books::ajmisc::{back, highest, lowest, ord, Sim, Ty, K, POINT};
use crate::connors::{sma, Ctx, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "pwadcs";
const ID: &str = "pwadcs_dcs2";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef { id: ID, name: "DCS II (5-bar average CCI(30) leaves +/-100, 9-bar breakout stop, tightening channel exit)", group: "DCS II (Open Code.txt)", session: 0, dir: "Long/Short", status: Status::Live,
        needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK, rules: &[
        "F|\"AvgCCI = Average(CCI(CCILen), AvgLen); BSetup = AvgCCI Crosses Above -100; SSetup = AvgCCI Crosses Below 100\" (CCILen 30, AvgLen 5)",
        "E|\"IF AvgCCI > -100 AND MRO(BSetup, EntryBar, 1) <> -1 Then Buy Next Bar at Highest(High, HiLoLen)[1] + 1 Point Stop\" (EntryBar 4, HiLoLen 9)",
        "E|\"IF AvgCCI < 100 AND MRO(SSetup, EntryBar, 1) <> -1 Then Sell Next Bar at Lowest(Low, HiLoLen)[1] - 1 Point Stop\" (TS2000i: Sell = short entry, reverses a long)",
        "F|\"IF MarketPosition = 1 AND High > Highest(High, HiLoLen)[1] Then IF TrailStop > 1 Then TrailStop = TrailStop - 1\" (short mirror with Low); \"IF BarsSinceEntry = 0 Then TrailStop = HiLoLen\"",
        "X|\"ExitLong Next Bar at Lowest(Low, TrailStop) - 1 Point Stop; ExitShort Next Bar at Highest(High, TrailStop) + 1 Point Stop\""],
        notes: "CCI = TradeStation CCI: (typical price - its Length-bar average) / (0.015 x mean absolute deviation). MRO(Test, 4, 1) <> -1 = the setup was true on this bar or one of the 3 before (help.tradestation.com, MRO). 'Crosses above' = above now and at or below on the prior bar. \
1 Point = 1/PriceScale = 0.01. TrailStop is reset to 9 on every bar with BarsSinceEntry = 0 (entry bar, or flat). DCS II is listed among Futures Truth's top systems in Ruggiero's book (putg_vendor_systems, not testable there); this is its code. \
Run on daily cash indices (instrument swap). Fills: vault TradeStation daily-bar convention (ajmisc.rs header). India: instrument swap; index shorts hypothetical.",
        claim: None }]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> { (id == ID).then_some((ID, "DCS II (pwafutures.com)", "code")) }

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    (id == ID).then(|| vec![p("cci_len", "CCILen", 30., 5., 60., 1.), p("avg_len", "AvgLen", 5., 1., 20., 1.), p("hilo_len", "HiLoLen", 9., 3., 30., 1.), p("entry_bar", "EntryBar", 4., 1., 10., 1.)])
}

/// TradeStation CCI(n).
pub fn cci(x: &Ctx, n: usize) -> Vec<f64> {
    let tp: Vec<f64> = (0..x.c.len()).map(|i| x.h[i] + x.l[i] + x.c[i]).collect();
    let m = sma(&tp, n);
    (0..tp.len()).map(|i| if m[i].is_nan() { f64::NAN } else {
        let md = tp[i + 1 - n..=i].iter().map(|v| (v - m[i]).abs()).sum::<f64>() / n as f64;
        if md == 0. { 0. } else { (tp[i] - m[i]) / (0.015 * md) }
    }).collect()
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if id != ID || from >= to || to >= x.r.len() { return None; }
    let g = |k: &str, d: f64| p.get(k).copied().unwrap_or(d);
    let (cl, al, hl, eb) = (g("cci_len", 30.).max(1.) as usize, g("avg_len", 5.).max(1.) as usize, g("hilo_len", 9.).max(1.) as usize, g("entry_bar", 4.).max(1.) as usize);
    let ac = sma(&cci(x, cl), al);
    let (hh, ll) = (back(&highest(&x.h, hl), 1), back(&lowest(&x.l, hl), 1));
    let n = x.c.len();
    let bset: Vec<bool> = (0..n).map(|i| i > 0 && ac[i] > -100. && ac[i - 1] <= -100.).collect();
    let sset: Vec<bool> = (0..n).map(|i| i > 0 && ac[i] < 100. && ac[i - 1] >= 100.).collect();
    let mro = |v: &[bool], i: usize| (0..eb).any(|k| i >= k && v[i - k]);
    let st = (from..n).find(|&i| i >= 1 && ac[i - 1].is_finite() && hh[i].is_finite()).unwrap_or(n);
    let mut s = Sim::new();
    let mut trail = 0usize;
    for i in st..to {
        let mut q = vec![];
        if ac[i] > -100. && mro(&bset, i) { q.push(ord(K::Buy, Ty::Stop(hh[i] + POINT), "Highest(H,9)[1] + 1 point")); }
        if ac[i] < 100. && mro(&sset, i) { q.push(ord(K::Short, Ty::Stop(ll[i] - POINT), "Lowest(L,9)[1] - 1 point")); }
        if s.pos == 1 && x.h[i] > hh[i] && trail > 1 { trail -= 1; }
        if s.pos == -1 && x.l[i] < ll[i] && trail > 1 { trail -= 1; }
        if s.bse(i) == 0 { trail = hl; }
        if trail >= 1 && i + 1 >= trail {
            let w = i + 1 - trail;
            q.push(ord(K::XL, Ty::Stop(x.l[w..=i].iter().copied().fold(f64::MAX, f64::min) - POINT), "trailing Lowest(L,TrailStop) - 1 point"));
            q.push(ord(K::XS, Ty::Stop(x.h[w..=i].iter().copied().fold(f64::MIN, f64::max) + POINT), "trailing Highest(H,TrailStop) + 1 point"));
        }
        s.bar(x, i + 1, &q);
    }
    Some(s.done(x, to))
}
