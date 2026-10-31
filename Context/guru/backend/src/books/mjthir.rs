//! Mark G. Johnson, "Thirteen" MACD system for a commodity-futures portfolio (code dated 24 May 2001, readme 2 Jun 2001).
//! Source: research drive TradeStation Systems & Indicators/Mark Johnson - Thirteen System for TS/thirteen.go (Trading
//! Recipes code); copy in Books/Mark G Johnson/Thirteen System/. Book id "mjthir". Notes: library/mjthir/README.md.
//! The EasyLanguage text files (Mgj_13.txt, mj13_mm.txt) are only inside a RAR archive (no RAR tool), so thirteen.go is the source.
use crate::connors::{Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "mjthir";
/// Admitted ids (authors/lanea/elmtrader/decisions.json).
pub const REVIEWED_LANEA: &[&str] = &["mjthir_thirteen"];
pub fn reviewed_lanea(id: &str) -> bool { REVIEWED_LANEA.contains(&id) }

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef {
        id: "mjthir_thirteen", name: "Thirteen: sign of MACD(13,130), always in, reverse at the open", group: "thirteen.go (Trading Recipes)",
        session: 0, dir: "Long/Short", status: Status::Live, needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK,
        rules: &[
            "F|\"COL1 = COL1[1] + 1\" (count of days since the start of the data); \"COL2 = MACD[C,13,130]\" (thirteen.go lines 8-9)",
            "E|\"IF (COL1 > 100) AND (COL2[1] > 0) THEN BUYOPEN\" / \"IF (COL1 > 100) AND (COL2[1] < 0) THEN SELLOPEN\" (lines 15-16): yesterday's MACD sign, filled at today's open",
            "X|\"IF (LONG > 0) AND (COL2[1] < 0) THEN REVERSE = OPEN\" / \"IF (SHORT > 0) AND (COL2[1] > 0) THEN REVERSE = OPEN\" (lines 25-26)",
            "F|Bet size \"MEMORY[1] = (.007 * EQUITY) / MANAGER[1]\" with MANAGER = POINTVALUE x EMA15(true range) (lines 11-12, 19-22): not modelled (one full position per trade, percentage returns)",
        ],
        notes: "MACD[C,13,130] read as EMA(13) - EMA(130) of closes, smoothing 2/(n+1), seeded with the first close [the Trading Recipes EMA seeding is not stated in the file; only the sign is used]. A MACD of exactly 0 gives no order. Bars are counted from the first bar of the loaded data (COL1 > 100 = bar index >= 100). Signals are taken from the selected start onwards (flat at the start). Book result (thirteen.xls): 21-market futures portfolio with the 0.7% bet-size rule, CAGR 77%/yr, max drawdown 38.5%; not comparable with one index. India: instrument swap; index shorts are hypothetical.",
        claim: None,
    }]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    (id == "mjthir_thirteen").then_some(("mjthir_thirteen", "Mark Johnson Thirteen (MACD 13/130)", "thirteen.go literal"))
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    (id == "mjthir_thirteen").then(|| vec![p("fast", "MACD fast EMA", 13., 5., 30., 1.), p("slow", "MACD slow EMA", 130., 50., 250., 10.),
        p("warm", "Entries only after COL1 > this bar count", 100., 0., 300., 10.)])
}

/// EMA with smoothing 2/(n+1), seeded with the first value.
pub fn ema(v: &[f64], n: usize) -> Vec<f64> {
    let k = 2.0 / (n as f64 + 1.0);
    let mut out = vec![f64::NAN; v.len()];
    let mut e = f64::NAN;
    for (i, &x) in v.iter().enumerate() {
        e = if e.is_nan() { x } else { e + k * (x - e) };
        out[i] = e;
    }
    out
}

fn open(dir: f64, i: usize, px: f64, sig: usize) -> Trade {
    Trade { dir, legs: vec![Leg { i, px, w: 1. }], exit_i: i, exit_px: px, reason: String::new(), entry_kind: "open", signal_i: sig, open: false, exits: vec![] }
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if id != "mjthir_thirteen" || from >= to || to >= x.r.len() { return None; }
    let g = |k: &str, d: f64| p.get(k).copied().unwrap_or(d).max(0.) as usize;
    let (fast, slow, warm) = (g("fast", 13.).max(1), g("slow", 130.).max(1), g("warm", 100.));
    let (ef, es) = (ema(&x.c, fast), ema(&x.c, slow));
    let mut out = vec![];
    let mut cur: Option<Trade> = None;
    for i in from + 1..=to {
        let m = ef[i - 1] - es[i - 1];
        let want = if m > 0. { 1. } else if m < 0. { -1. } else { continue };
        match cur.as_ref() {
            Some(t) if t.dir == want => {}
            Some(_) => {
                let mut t = cur.take().unwrap();
                t.exit_i = i; t.exit_px = x.o[i]; t.signal_i = i - 1; t.reason = "MACD sign flipped · reverse at the open".into();
                out.push(t);
                cur = Some(open(want, i, x.o[i], i - 1));
            }
            None if i >= warm => cur = Some(open(want, i, x.o[i], i - 1)),
            None => {}
        }
    }
    if let Some(mut t) = cur { t.exit_i = to; t.exit_px = x.c[to]; t.signal_i = to; t.reason = "open".into(); t.open = true; out.push(t); }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;
    #[test]
    fn reverses_at_the_open_after_the_macd_sign_flips() {
        let mut r: Vec<Row> = (0..400).map(|i| Row { ticker: "T".into(), date: format!("{:04}", i), open: 100., high: 101., low: 99., close: 100. + i as f64 * 0.1, volume: None }).collect();
        for (k, row) in r[300..].iter_mut().enumerate() { row.close = 130. - k as f64; row.open = 130. - k as f64; }
        let x = Ctx::new(&r, &r, &[]);
        let ts = run("mjthir_thirteen", &x, 0, 399, &Params::new()).unwrap();
        assert_eq!(ts[0].dir, 1.); assert_eq!(ts[0].legs[0].i, 100); // COL1 > 100
        let m = |i: usize| ema(&x.c, 13)[i] - ema(&x.c, 130)[i];
        let flip = (300..399).find(|&i| m(i) < 0.).unwrap();
        assert_eq!(ts[0].exit_i, flip + 1); assert_eq!(ts[0].exit_px, r[flip + 1].open);
        assert_eq!(ts[1].dir, -1.); assert!(ts[1].open);
    }
}

// Instrument admission: Lane A elmtrader 2026-10-26 (lane B) block, moved from instrument_policy.rs on 31 Oct 2026 (lane ST).
// Admission only for the module REVIEWED lists (authors/lanea/elmtrader/decisions.json); kept in one block to merge.
// Runs on actual indices, ETFs and stocks, US and India (index_etf_or_stock, nifty50 basket, declared application).
pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| (def.book=="mjthir" && crate::books::mjthir::reviewed_lanea(def.id)) || (def.book=="pwabal" && crate::books::pwabal::reviewed_lanea(def.id)) || (def.book=="jwbent" && crate::books::jwbent::reviewed_lanea(def.id)) || (def.book=="fcfail" && crate::books::fcfail::reviewed_lanea(def.id)) || (def.book=="kfaber" && crate::books::kfaber::reviewed_lanea(def.id)) || (def.book=="mstt" && crate::books::mstt::reviewed_lanea(def.id)) || (def.book=="ts247" && crate::books::ts247::reviewed_lanea(def.id)) || (def.book=="fsi23" && crate::books::fsi23::reviewed_lanea(def.id)),
    label: "Vendor / newsletter system code with no per-market test the vault can compare; runs on indices, ETFs and single stocks (US and India) are a declared application, one unit per trade.",
    basis: "Vendor and newsletter system codes (M. G. Johnson, P. B. Waite, J. Wolf, F. Corrias, Aberration copy, Equis Traders' Tips, B. Castelo, 24-7 System, ForexSuperInvestor RSI 2.3)",
};
