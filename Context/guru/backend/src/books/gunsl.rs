//! Gunslinger (TradeStation code, vendor not named): `T:/Gunslinger System for TS/Code.rar` -> `Code.txt`.
//! `T:` = research drive `USB HDD 1/TradeStation Systems & Indicators`. Book id "gunsl". Notes: library/gunsl/README.md.
use crate::books::ajmisc::{back, first_ok, highest, lowest, ord, stdev_pop, Sim, Ty, K};
use crate::connors::{sma, Ctx, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "gunsl";
const ID: &str = "gunsl_gunslinger";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef { id: ID, name: "Gunslinger (open +/- 0.81 x 2-day swing-range band, stop and reverse)", group: "Gunslinger (Code.txt)", session: 0, dir: "Long/Short", status: Status::Live,
        needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK, rules: &[
        "F|\"value1=highest(value12,2); value2=lowest(value22,2); value3=highest(value55,2); value4=lowest(value55,2); value5=value1-value4; value6=value3-value2\" (value12/22/55 = the day's high/low/close)",
        "F|\"value7=average(value5,8)+.45*stddev(value5,8); value8=average(value6,8)+.45*stddev(value6,9); value20=(value7+value8)/2\" (stddev(value6,9): 9 in the code, kept)",
        "E|\"Sell Short Next Bar at value44-.81*(value20+thrustTax) stop\" (value44 = open of tomorrow, thrustTax 0.6)",
        "E|\"Buy Next Bar at value44+.81*(value20-thrustTax) stop\"",
        "X|Stop and reverse; no other exit in the code."],
        notes: "Reading of the code: the entry conditions condition38/39/48/49 are 'value87 = value87' etc., which are always true (the variables start at 0), and the min((O-L)/(H-L),(H-O)/(H-L)) < .5 test only feeds those variables, so both stops are placed every bar. \
intraDayHigh/Low reset on each new date = the daily high/low on daily bars. stddev = TradeStation StdDev (population). thrustTax x100 when the first bar's open > 10000 ('Fix for FutureSource'): not triggered on NIFTY 50 (1995 opens ~1000) or SPX (1983 ~140). \
No market or bar size named: daily cash indices (instrument swap). Fills: vault TradeStation daily-bar convention (ajmisc.rs header). India: instrument swap; index shorts hypothetical.",
        claim: None }]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> { (id == ID).then_some((ID, "Gunslinger", "code")) }

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    (id == ID).then(|| vec![p("mult", "Entry multiple (.81)", 0.81, 0.2, 2., 0.01), p("thrust_tax", "thrustTax", 0.6, 0., 5., 0.1)])
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if id != ID || from >= to || to >= x.r.len() { return None; }
    let g = |k: &str, d: f64| p.get(k).copied().unwrap_or(d);
    let m = g("mult", 0.81);
    let (v1, v2, v3, v4) = (highest(&x.h, 2), lowest(&x.l, 2), highest(&x.c, 2), lowest(&x.c, 2));
    let v5: Vec<f64> = v1.iter().zip(&v4).map(|(a, b)| a - b).collect();
    let v6: Vec<f64> = v3.iter().zip(&v2).map(|(a, b)| a - b).collect();
    let (a5, s5, a6, s6) = (sma(&v5, 8), stdev_pop(&v5, 8), sma(&v6, 8), stdev_pop(&v6, 9));
    let v20: Vec<f64> = (0..x.c.len()).map(|i| (a5[i] + 0.45 * s5[i] + a6[i] + 0.45 * s6[i]) / 2.).collect();
    let st = first_ok(x.c.len(), from, &[&v20, &back(&v5, 9), &back(&v6, 9)]);
    if st >= to { return Some(vec![]); }
    let tt = g("thrust_tax", 0.6) * if x.o[st] > 10000. { 100. } else { 1. };
    let mut s = Sim::new();
    for i in st..to {
        let o1 = x.o[i + 1];
        s.bar(x, i + 1, &[ord(K::Short, Ty::Stop(o1 - m * (v20[i] + tt)), "open - .81(value20+tax) stop"), ord(K::Buy, Ty::Stop(o1 + m * (v20[i] - tt)), "open + .81(value20-tax) stop")]);
    }
    Some(s.done(x, to))
}
