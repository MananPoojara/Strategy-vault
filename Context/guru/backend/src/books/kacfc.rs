//! Lane K82 (13 Nov 2026; authors/titles-1030/K82.tsv; reader row authors/o2/O2.tsv): Kevin Adams, "Cracking The Forex Code"
//! (e-book, crackingforexcode.com, no date; research share 'USB HDD 2/tRADING KNO-2/Kevin Adams - Cracking The Forex Code
//! (crackingforexcode.com)/Cracking The Forex Code(Kevin Adams nd).pdf', 28 PDF pages; the printed page number is the PDF page - 1).
//! Book id `kacfc`. Coded: System 2 (CFC2, EMA 5/12 cross with the Parabolic SAR on the same side, exit when the SAR flips, p.20-24)
//! as a daily application (R4 #67, #114: the 50-pip stop and the 50-pip trailing alternative are held). Held: System 1 (its exits
//! are pip amounts only).
//! Independent reference: authors/titles-1030/K82w/k82_ref.py (written from the pages, not from this file).
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};
use super::as51ts::psar;
use super::msws::ema;
use super::tselhs::Book;

pub const REVIEWED: &[&str] = &["kacfc_cfc2"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const NOTE: &str = "[Declared readings] The book's system is for 1-hour forex charts and says it can be used on 4-hour and daily charts if the stop, target and trailing stop are tweaked (p.20); it runs here on daily bars of indices, ETFs and stocks, US and India, as a declared application (R4 #67, #114). The 50-pip protective stop and the 50-pip trailing-stop alternative are pip amounts and are held (R4 #19, #88): the trade exits only on the SAR flip. EMA = exponential average of closes (alpha 2/(n+1)), seeded with the first close of the data. Parabolic SAR = Wilder's standard 0.02 / 0.02 / 0.2, computed as TradingView's ta.sar. Both conditions must hold on the same bar: the EMA 5 crosses the EMA 12 on that bar (below today, at or above yesterday, for a short) and that bar's SAR dot is on the other side of price (above for a short, below for a long); the order is filled at that bar's close. Exit at the close of the first bar whose SAR dot is on the trade's side ('when the Parabolic SAR dot appears below the price candlestick' for a short); a new signal on that bar is taken after the exit. No performance figures are printed (chart examples only). The book calls the 12-period average 'EMA60' once in its indicator list (p.20, a typo); the rules use EMA12.";

pub fn catalogue() -> Vec<StrategyDef> {
    let d = |id, name, group, status, rules, notes| StrategyDef { id, name, group, session: 0, dir: "Long/Short", status, needs_volume: false, needs_vix: false,
        needs_series: &[], book: "kacfc", rules, notes, claim: None };
    vec![
        d("kacfc_cfc2", "Adams CFC2: EMA 5 crosses EMA 12 with the Parabolic SAR on the same side; exit when the SAR flips", "System 2 (CFC2, p.20-24)", Status::Live,
          &["E|\"We enter a short (sell) trade when these two conditions happen: 1. EMA5 crosses below EMA12. 2. Parabolic SAR dot appears above the price candlestick.\" (p.20); long: \"EMA5 crosses above EMA12. Parabolic SAR dot appears below the price candlestick.\" (p.24)",
            "X|\"Profit Target: We exit the trade when the Parabolic SAR dot appears below the price candlestick.\" (short, p.20; long mirror p.24). Held: \"We place a protective stop loss at -50 pips\"; \"Alternatively, we can also use a trailing stop of 50 pips\" (pip amounts, R4 #19/#88)",
            "F|\"This system is used for 1-hour timeframe. You can also use it for 4-hour and daily timeframes. But in that case, you have to tweak the stop loss, profit target, trailing stop appropriately.\" (p.20)"],
          NOTE),
        d("kacfc_cfc1", "Adams CFC1: EMA 5 > 15 > 60 with the 15 and 60 rising, buy the touch of the EMA 60", "System 1 (CFC1, p.3-19)", Status::NotTestable("Held: every exit is a pip amount (stop -40 pips, half off at +40 pips, then a 40-pip trailing stop, p.3-4), and the book says the stop, target and trail must be 'tweaked' for daily charts without saying how (p.3); pip exits are held (R4 #19, #88). 'Pointing up' (slope) and 'touch the EMA60' are not defined further."),
          &["E|\"EMA60 and EMA15 are both pointing up. EMA5 is above EMA15. EMA15 is above EMA60 ... We then wait for the price to fall back, and touch the EMA60. When that happens, we enter with two lots.\" (p.3)",
            "X|\"We place a protective stop loss at -40 pips ... When we have 40 pips in profit we exit one lot. Move stop loss to break even for the second lot. We then use a trailing stop of 40 pips for that lot.\" (p.3-4)"],
          "Kevin Adams, Cracking The Forex Code (e-book, no date); printed pages cited."),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    match id {
        "kacfc_cfc2" => Some(("kacfc_cfc2", "Adams CFC2 EMA 5/12 + Parabolic SAR", "5/12")),
        "kacfc_cfc1" => catalogue().into_iter().find(|d| d.id == id).map(|d| (d.id, d.name, "")),
        _ => None,
    }
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    reviewed(id).then(|| vec![p("fast", "Fast EMA, bars (book 5)", 5., 1., 50., 1.), p("slow", "Slow EMA, bars (book 12)", 12., 2., 100., 1.)])
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.r.len() { return None; }
    let g = |k: &str, d: f64| p.get(k).copied().unwrap_or(d).round().max(1.) as usize;
    let (a, b) = (ema(&x.c, g("fast", 5.)), ema(&x.c, g("slow", 12.)));
    let (sar, below, _) = psar(&x.h, &x.l, &x.c, 0.02, 0.02, 0.2);
    let mut pos = Book::new();
    for t in from.max(1)..=to {
        if !sar[t].is_finite() { continue; }
        let side = pos.side();
        if (side > 0.0 && !below[t]) || (side < 0.0 && below[t]) { pos.exit(t, x.c[t], "Parabolic SAR flipped"); }
        if !(a[t - 1].is_finite() && b[t - 1].is_finite()) || pos.side() != 0.0 { continue; }
        if a[t] > b[t] && a[t - 1] <= b[t - 1] && below[t] { pos.enter(1.0, t, x.c[t], "close"); }
        else if a[t] < b[t] && a[t - 1] >= b[t - 1] && !below[t] { pos.enter(-1.0, t, x.c[t], "close"); }
    }
    Some(pos.done(to, x.c[to]))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn cfc2_enters_on_the_cross_with_the_sar_and_exits_on_the_flip() {
        let r = super::super::sctraub::tests::bars(2500);
        let x = Ctx::new(&r, &r, &[]);
        let (_, below, _) = psar(&x.h, &x.l, &x.c, 0.02, 0.02, 0.2);
        let t = run("kacfc_cfc2", &x, 0, r.len() - 1, &Params::new()).unwrap();
        assert!(t.iter().any(|z| z.dir > 0.) && t.iter().any(|z| z.dir < 0.));
        for z in &t {
            assert_eq!(below[z.legs[0].i], z.dir > 0.);
            if !z.open { assert_eq!(below[z.exit_i], z.dir < 0.); for k in z.legs[0].i..z.exit_i { assert_eq!(below[k], z.dir > 0.); } }
        }
        for w in t.windows(2) { assert!(w[1].legs[0].i >= w[0].exit_i); }
    }
}

// Instrument admission: Adams CFC2 daily application 2026-11-13 (lane K82).
pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book == "kacfc" && reviewed(def.id),
    label: "Kevin Adams's system is for 1-hour forex charts ('also 4-hour and daily'); daily bars of indices, ETFs and stocks, US and India, are a declared application with the pip stop held.",
    basis: "Cracking The Forex Code (e-book) p.20-24; R4 #67, #114; independent check authors/titles-1030/K82w/k82_ref.py",
};
