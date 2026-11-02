//! Roger Darley, "Optimizing With Hilbert Indicators" (Stocks & Commodities, Nov 2000) and that issue's Traders' Tips
//! (research share USB HDD 1/MetaStock/@Systems & Indicators/@Misc/- Metastock Systems & Indicators/Hilbert Indicators +
//! Info/hillbert_metastock.doc). Book id `rdhcb`. One rule: the Hilbert Channel Breakout System with its printed
//! default inputs (15-bar entry and exit channels). Lane K6 (titles loop, 31 Oct 2026); independent check
//! authors/titles-1030/K6w/k6_ref.py (header). Second card `rdhcb_hilbert`: the adaptive Hilbert-period form with the article's
//! GE inputs (research share Trading Knowledge I/Stocks & Commodities DVD 11.26 (1982 - 2008) (traders.com)/VOLUMES/V18/C11/
//! 100Opt.pdf and 109TIPS.pdf); lane K19 (2 Nov 2026), to-code row 268, independent check authors/titles-1030/K19w/k19_ref.py.
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};
use super::pbwts::{highest, lowest};
use super::rbtie::mk;

/// Admitted ids (authors/titles-1030/K6w/k6_ref.py (header)).
pub const REVIEWED: &[&str] = &["rdhcb_channel", "rdhcb_hilbert"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const NOTE: &str = "[Declared readings] The printed inputs EntryVal 15 and ExitVal 15 are not 0, so the channels use fixed 15-bar look-backs and Ehlers' HilbertPeriod is not used (the adaptive form EntryK x Period, used when EntryVal = 0, is not coded here: the article's 'optimum' K ranges are not fixed values). EntryChannel = the highest high of the 15 bars before the current one, ExitChannel = the lowest low of the 15 bars before it. 'Buy' and 'ExitLong' without an order type are TradeStation's next-bar market orders: filled at the next open. One long position; the exit test starts on the entry bar. Equis' MetaStock long/short reversal version is not coded. Daily bars; indices, ETFs and US/India stocks are a declared application; no results printed.";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef { id: "rdhcb_channel", name: "Hilbert Channel Breakout: buy above the 15-bar high, out below the 15-bar low, next open", group: "Darley Hilbert Channel Breakout",
        session: 0, dir: "Long", status: Status::Live, needs_volume: false, needs_vix: false, needs_series: &[], book: "rdhcb", rules: &[
            "F|\"Inputs: Price((H+L)/2), EntryVal(15), EntryK(0), ExitVal(15), ExitK(0)\"; EntryChannel = Highest(High, EntryLookBack)[1], ExitChannel = Lowest(Low, ExitLookBack)[1]",
            "E|\"If MarketPosition = 0 and High > EntryChannel then Buy\"",
            "X|\"If MarketPosition = 1 and Low < ExitChannel then ExitLong\""],
        notes: NOTE, claim: None },
    StrategyDef { id: "rdhcb_hilbert", name: "Hilbert Channel Breakout, adaptive: entry channel 0.1 x and exit channel 3.4 x Ehlers' Hilbert cycle period, next open", group: "Darley Hilbert Channel Breakout",
        session: 0, dir: "Long", status: Status::Live, needs_volume: false, needs_vix: false, needs_series: &[], book: "rdhcb", rules: &[
            "F|Period = HilbertPeriod((H+L)/2) (Ehlers 7/22/00 function, S&C V.18:11 Traders' Tips p.106-108); \"If EntryVal <> 0 then EntryLookBack = EntryVal else EntryLookBack = EntryK*Period; if EntryLookBack < 1 then EntryLookBack = 1\" (exit likewise); \"for count = 1 to EntryLookBack begin if EntryChannel < High[count] then EntryChannel = High[count]\"",
            "E|\"If MarketPosition = 0 and High > EntryChannel then Buy\" with EntryK 0.1 (the article's GE optimisation, Figures 5-7)",
            "X|\"If MarketPosition = 1 and Low < ExitChannel then ExitLong\" with ExitK 3.4 (Figures 5-7)"],
        notes: NOTE_HILBERT, claim: None }]
}

const NOTE_HILBERT: &str = "[Declared readings] The article's optimised Hilbert inputs on GE (10/27/1987-8/4/2000), EntryK 0.1 and ExitK 3.4 with EntryVal = ExitVal = 0, are the default (decision 73); the fixed 10/45 bars that 'pop up quickly' in its first step are option entval = 10, exitval = 45. HilbertPeriod is Ehlers' printed function on the bar midpoint (H+L)/2: computed from the 6th bar of the data (CurrentBar > 5), earlier values 0, ArcTangent in degrees, period limited to 0.67-1.5 x the previous value and to 6-50 bars, then smoothed 0.2/0.8. The look-back loops 'for count = 1 to K x Period' run over whole bars, so a look-back is the integer part of K x Period (at least 1). No signals in the first 200 bars of the data (the article's 'Max number of bars strategy will reference' = 200). The printed ExitChannel start value 100,000 is read as 'no limit'. 'Buy' and 'ExitLong' are next-bar market orders (next open); one long position. Article: GE, 30 trades over 12.5 years, $3,570 net per share after $0.30 costs, profit factor 4.21 (fixed 10/45: $2,466, 3.13). Runs on indices, ETFs and US/India stocks are a declared application.";

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    match id { "rdhcb_channel" => Some(("rdhcb_channel", "Darley Hilbert channel breakout", "15/15 bars")), "rdhcb_hilbert" => Some(("rdhcb_channel", "Darley Hilbert channel breakout", "Hilbert period, K 0.1 / 3.4")), _ => None }
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    match id {
        "rdhcb_channel" => Some(vec![p("entry", "EntryVal, entry channel bars (printed 15)", 15., 2., 100., 1.), p("exit", "ExitVal, exit channel bars (printed 15)", 15., 2., 100., 1.)]),
        "rdhcb_hilbert" => Some(vec![p("entk", "EntryK x Hilbert period (GE optimum 0.1)", 0.1, 0.1, 1., 0.1), p("exitk", "ExitK x Hilbert period (GE optimum 3.4)", 3.4, 1.5, 4.5, 0.1),
            p("entval", "EntryVal, fixed entry bars (0 = adaptive; article 10)", 0., 0., 100., 1.), p("exitval", "ExitVal, fixed exit bars (0 = adaptive; article 45)", 0., 0., 200., 1.)]),
        _ => None,
    }
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.r.len() { return None; }
    if id == "rdhcb_hilbert" { return Some(hilbert_channel(x, from, to, p)); }
    let schema = params(id)?;
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).map_or(f64::NAN, |d| d.def));
    let (en, ex) = (highest(&x.h, g("entry").round().max(1.) as usize), lowest(&x.l, g("exit").round().max(1.) as usize));
    let (mut out, mut cur) = (vec![], None::<(usize, f64)>);
    let mut i = from.max(1);
    while i < to {
        match cur {
            None => if x.h[i] > en[i - 1] { cur = Some((i + 1, x.o[i + 1])); },
            Some((e, ep)) => if x.l[i] < ex[i - 1] { out.push(mk(1., e, ep, "open", i + 1, x.o[i + 1], "low below the exit channel", false, i)); cur = None; },
        }
        i += 1;
    }
    if let Some((e, ep)) = cur { out.push(mk(1., e, ep, "open", to, x.c[to], "open", true, to)); }
    Some(out)
}

/// Ehlers' HilbertPeriod (EasyLanguage function of 7/22/00, S&C V.18:11 Traders' Tips) on `price`: series variables start at 0
/// and the core runs from the 6th bar (CurrentBar > 5); a reference before the data reads 0.
pub fn hilbert_period(price: &[f64]) -> Vec<f64> {
    let n = price.len();
    let (mut sm, mut de, mut q1, mut i1) = (vec![0.0; n], vec![0.0; n], vec![0.0; n], vec![0.0; n]);
    let (mut i2, mut q2, mut re, mut im, mut per) = (vec![0.0; n], vec![0.0; n], vec![0.0; n], vec![0.0; n], vec![0.0; n]);
    let at = |v: &[f64], i: usize, k: usize| if i >= k { v[i - k] } else { 0.0 };
    for i in 5..n {
        let pp = at(&per, i, 1);
        sm[i] = (4.0 * price[i] + 3.0 * price[i - 1] + 2.0 * price[i - 2] + price[i - 3]) / 10.0;
        de[i] = (0.25 * sm[i] + 0.75 * at(&sm, i, 2) - 0.75 * at(&sm, i, 4) - 0.25 * at(&sm, i, 6)) * (0.046 * pp + 0.332);
        q1[i] = (0.25 * de[i] + 0.75 * at(&de, i, 2) - 0.75 * at(&de, i, 4) - 0.25 * at(&de, i, 6)) * (0.046 * pp + 0.332);
        i1[i] = at(&de, i, 3);
        let ji = 0.25 * i1[i] + 0.75 * at(&i1, i, 2) - 0.75 * at(&i1, i, 4) - 0.25 * at(&i1, i, 6);
        let jq = 0.25 * q1[i] + 0.75 * at(&q1, i, 2) - 0.75 * at(&q1, i, 4) - 0.25 * at(&q1, i, 6);
        i2[i] = 0.15 * (i1[i] - jq) + 0.85 * at(&i2, i, 1);
        q2[i] = 0.15 * (q1[i] + ji) + 0.85 * at(&q2, i, 1);
        let r = i2[i] * at(&i2, i, 1) + q2[i] * at(&q2, i, 1);
        let m = i2[i] * at(&q2, i, 1) - q2[i] * at(&i2, i, 1);
        re[i] = 0.2 * r + 0.8 * at(&re, i, 1);
        im[i] = 0.2 * m + 0.8 * at(&im, i, 1);
        let mut p = pp;
        if im[i] != 0.0 && re[i] != 0.0 { p = 360.0 / ((im[i] / re[i]).atan() * 180.0 / std::f64::consts::PI); }
        if p > 1.5 * pp { p = 1.5 * pp; }
        if p < 0.67 * pp { p = 0.67 * pp; }
        if p < 6.0 { p = 6.0; }
        if p > 50.0 { p = 50.0; }
        per[i] = 0.2 * p + 0.8 * pp;
    }
    per
}

/// The Hilbert Channel Breakout signal with adaptive (K x Period) or fixed (Val) look-backs; next-open fills.
fn hilbert_channel(x: &Ctx, from: usize, to: usize, p: &Params) -> Vec<Trade> {
    let g = |k: &str, d: f64| p.get(k).copied().unwrap_or(d);
    let (ek, xk, ev, xv) = (g("entk", 0.1), g("exitk", 3.4), g("entval", 0.).round(), g("exitval", 0.).round());
    let mid: Vec<f64> = (0..x.r.len()).map(|i| (x.h[i] + x.l[i]) / 2.0).collect();
    let per = hilbert_period(&mid);
    let look = |val: f64, k: f64, i: usize| { let lb = if val != 0.0 { val } else { k * per[i] }; lb.max(1.0).floor() as usize };
    super::sctraub::next_open(x, from, to, "low below the exit channel", |t, side| {
        if t < 200 { return side; }
        if side == 0.0 { let n = look(ev, ek, t); if x.h[t] > super::lc3::hi(&x.h, t - n, t - 1) { return 1.0; } }
        else { let n = look(xv, xk, t); if x.l[t] < super::lc3::lo(&x.l, t - n, t - 1) { return 0.0; } }
        side
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;
    #[test]
    fn channel_breakout_next_open() {
        let b = [(10., 9.), (10., 9.), (10., 9.), (11., 9.5), (11.5, 10.), (10.5, 8.5), (10., 9.)];
        let r: Vec<Row> = b.iter().enumerate().map(|(i, &(h, l))| Row { ticker: "T".into(), date: format!("2001-01-{:02}", i + 1), open: l + 0.5, high: h, low: l, close: h - 0.5, volume: None }).collect();
        let x = Ctx::new(&r, &r, &[]);
        let p: Params = [("entry".to_string(), 3.), ("exit".to_string(), 3.)].into_iter().collect();
        let t = run("rdhcb_channel", &x, 0, r.len() - 1, &p).unwrap();
        // bar 3: high 11 > 10 -> buy bar 4 open 10.5; bar 5: low 8.5 < 9 -> out at bar 6 open 9.5
        assert_eq!(t.iter().map(|z| (z.legs[0].i, z.legs[0].px, z.exit_i, z.exit_px)).collect::<Vec<_>>(), vec![(4, 10.5, 6, 9.5)]);
        assert!(run("rdhcb_x", &x, 0, 6, &p).is_none());
    }
    #[test]
    fn hilbert_period_bounds_and_trades() {
        let r = super::super::sctraub::tests::bars(1500);
        let x = Ctx::new(&r, &r, &[]);
        let per = hilbert_period(&x.c);
        assert!(per[300..].iter().all(|&v| (6.0..=50.0).contains(&v)));
        let t = run("rdhcb_hilbert", &x, 0, r.len() - 1, &Params::new()).unwrap();
        assert!(t.len() > 3 && t.iter().all(|z| z.dir > 0. && z.legs[0].i > 200));
    }
}

// Instrument admission: Roger Darley (Hilbert Channel Breakout) 2026-10-31 (titles lane K6).
// Admission only for the module REVIEWED lists (authors/titles-1030/K6w/k6_ref.py (header)).
// Runs on actual indices, ETFs and stocks, US and India (index_etf_or_stock, nifty50 basket, declared application).
pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book == "rdhcb" && crate::books::rdhcb::reviewed(def.id),
    label: "A magazine system for any market with no printed results; indices, ETFs and US/India stocks are a declared application.",
    basis: "Roger Darley, 'Optimizing With Hilbert Indicators', S&C Nov 2000, and Traders' Tips (Hilbert Channel Breakout EasyLanguage), hillbert_metastock.doc",
};
