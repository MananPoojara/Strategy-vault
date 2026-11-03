//! ASC Trend EA (ASCTRENDTS 2006) with the ASCTrend arrow indicator (Ramdass conversion), MetaTrader 4 code from the Forexmentor
//! Metatrader Toolkit (research share USB HDD 2/tRADING KNO-2/_Forexmentor.com/Alan Benefield - Mastering Metatrader 4 in 90 Minutes &
//! Members Site/BONUS 1 - Metatrader Toolkit/ASC Trend EA(2).mq4 and ASCTrend.mq4). Book id `mtasc`. Lane K18 (titles loop,
//! 2 Nov 2026), to-code row 323; independent check authors/titles-1030/K18w/k18_ref.py.
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};
use super::sctraub::next_open;

pub const REVIEWED: &[&str] = &["mtasc_trend"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const NOTE: &str = "[Declared readings] MetaTrader 4 expert advisor with no timeframe of its own, run on daily bars as a declared application (lead decision 114). ASCTrend indicator as printed (RISK 3): Range = average of High - Low over the last 10 bars; MRO2 = |Close[k+3] - Close[k]| >= 4.6 x Range for some k = 0..5 bars back; the Williams %R period is 4 when MRO2 holds, else 3 + 2 x RISK = 9 (the printed MRO1 test is overwritten by the MRO2 line, so it has no effect); v = 100 - |WPR|. A down arrow when v < 33 - RISK and, going back over the bars with v inside [30, 70], the first bar outside is above 70; an up arrow mirrored. The EA acts on the most recent arrow at or before the last closed bar: flat -> open in its direction; a long closes on a down arrow, a short on an up arrow (EnableCloseByASCTrend true) and the next tick opens the other side, read here as a reversal at the next open. The indicator is computed over the full history (its CountBars 300 is a chart limit); a bar whose %R range is zero has no value and no arrow. HELD: the 20-pip stops, 80-pip targets and 20-pip trailing stops (pip amounts, decisions 19 and 88) and the 08:00-18:00 hour filter (clock time, no meaning on daily bars). Lot sizing ignored (one position). No results are printed. Runs on indices, ETFs and stocks, US and India, are a declared application.";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef { id: "mtasc_trend", name: "ASC Trend EA: ASCTrend Williams %R arrows, always in the direction of the latest arrow (stop and reverse)", group: "MT4 EA: ASC Trend (Forexmentor toolkit)", session: 0, dir: "Long/Short", status: Status::Live,
        needs_volume: false, needs_vix: false, needs_series: &[], book: "mtasc", rules: &[
            "F|ASCTrend.mq4: value10 = 3 + RISK*2; x1 = 67 + RISK; x2 = 33 - RISK; Range = avg(|High - Low|, 10); if (MRO2 > -1) value11 = 4 else value11 = value10; value2 = 100 - |iWPR(value11)|",
            "E|Down arrow: value2 < x2 and the first earlier value outside [x2, x1] is > x1; up arrow: value2 > x1 and the first earlier value outside the band is < x2. EA: no position -> OpenSell on the latest down arrow, OpenBuy on the latest up arrow",
            "X|EnableCloseByASCTrend: CloseBuy on a down arrow, CloseSell on an up arrow (then the other side opens). Pip stop 20 / target 80 / trailing 20 held (pip amounts)."],
        notes: NOTE, claim: None }]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> { reviewed(id).then_some(("mtasc_trend", "ASC Trend EA (MT4)", "RISK 3")) }

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    reviewed(id).then(|| vec![p("risk", "RISK (3)", 3., 0., 20., 1.)])
}

/// ASCTrend arrows: +1 up arrow, -1 down arrow, 0 none.
pub fn arrows(x: &Ctx, risk: f64) -> Vec<i8> {
    let n = x.c.len();
    let (x1, x2) = (67. + risk, 33. - risk);
    let p10 = (3. + 2. * risk).max(1.) as usize;
    let mut v = vec![f64::NAN; n];
    let mut out = vec![0i8; n];
    for i in 9..n {
        let range = (0..10).map(|k| (x.h[i - k] - x.l[i - k]).abs()).sum::<f64>() / 10.; // MQL order: newest bar first
        let mro2 = (0..6).any(|k| i >= k + 3 && (x.c[i - k - 3] - x.c[i - k]).abs() >= range * 4.6);
        let per = if mro2 { 4 } else { p10 };
        if i + 1 < per { continue; }
        let hh = (i + 1 - per..=i).map(|j| x.h[j]).fold(f64::MIN, f64::max);
        let ll = (i + 1 - per..=i).map(|j| x.l[j]).fold(f64::MAX, f64::min);
        if hh - ll <= 0. { continue; }
        v[i] = 100. - (-100. * (hh - x.c[i]) / (hh - ll)).abs();
        let back = |i: usize| -> f64 { let mut j = i; while j > 0 { j -= 1; if !(v[j] >= x2 && v[j] <= x1) { return v[j]; } } f64::NAN };
        if v[i] < x2 && back(i) > x1 { out[i] = -1; }
        if v[i] > x1 && back(i) < x2 { out[i] = 1; }
    }
    out
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.r.len() { return None; }
    let a = arrows(x, p.get("risk").copied().unwrap_or(3.).round());
    let mut last = 0.0;
    let mut k = 0;
    Some(next_open(x, from, to, "reversed", |t, side| {
        while k <= t { if a[k] != 0 { last = a[k] as f64; } k += 1; }
        if last != 0.0 { last } else { side }
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn asc_trend_reverses() {
        let r = super::super::sctraub::tests::bars(2000);
        let x = Ctx::new(&r, &r, &[]);
        let t = run("mtasc_trend", &x, 0, r.len() - 1, &Params::new()).unwrap();
        assert!(t.len() > 3 && t.iter().any(|z| z.dir > 0.) && t.iter().any(|z| z.dir < 0.));
        for w in t.windows(2) { assert_eq!(w[0].exit_i, w[1].legs[0].i); assert_eq!(w[0].dir, -w[1].dir); }
        assert!(run("mtasc_x", &x, 0, 5, &Params::new()).is_none());
    }
}

// Instrument admission: ASC Trend EA 2026-11-02 (titles lane K18).
pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book == "mtasc" && reviewed(def.id),
    label: "MetaTrader 4 forex expert advisor with no published test. Runs on daily indices, ETFs and stocks, US and India, are a declared application.",
    basis: "Forexmentor Metatrader Toolkit, ASC Trend EA(2).mq4 and ASCTrend.mq4 (printed code); independent check authors/titles-1030/K18w/k18_ref.py",
};
