//! Donald Pendergast, "Making A Good System Great" (the MFI 5-5 system), Technical Analysis of Stocks & Commodities, January
//! 2011, pp.24-30, sidebars "MetaStock Code For MFI 5-5 System" and "TradeSim Enterprise Code For MFI 5 Long Only" (research
//! share Trading Knowledge I/Stocks & Commodities 2010 -2011 (traders.com)/SC2011-01.pdf, OCR of PDF p.24-30). Book id
//! `scpenmfi`. Lane K19 (titles loop, 2 Nov 2026), to-code row 301; independent check authors/titles-1030/K19w/k19_ref.py.
use crate::connors::{rsi, sma, Ctx, ParamDef, Params, Status, StrategyDef, Trade};
use super::tselhs::{india, Book};

pub const REVIEWED: &[&str] = &["scpenmfi_55"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const NOTE: &str = "[Declared readings] The printed code is the default (decision 133): 'EntryTrigger = Ref((Cross( MFI(5),10)),-1)' (MetaStock Cross(A, B) = A crosses above B: MFI(5) rises through 10) and 'ExitTrigger = Ref(Cross( 75, RSI(5)),-1)' (RSI(5) falls back through 75), both at the next session's open. The text describes the opposite crossings ('dropped below a value of 10', 'rises to a value above 75'): option text = 1. MFI(5) = MetaStock money flow index of the typical price (H+L+C)/3 x volume over 5 days (100 when there is no negative flow, 50 when there is none at all); RSI(5) is Wilder's. 'a fixed stop-loss of 5% for every trade; no trailing stop' (the OCR of the TradeSim stop line is unreadable): 5% below the entry price, checked on each day's low from the entry day, filled at the stop or at the open below it. The exploration filter 'CLOSE > 7.50 AND Mov(VOLUME,50,SIMPLE) > 400000' is option filt = 1 on the signal day (the $7.50 test US only, decision 58); the portfolio caps of the tests (4-9 positions, 1-2 new a day) are not part of the single-instrument signal. Article: 100 large-cap US stocks, 1/4/2001-7/29/2010, TradeSim portfolios: test 1 (9 positions, 1 new a day) 1,258 trades, 47% winners; test 8 (4 positions, 2 a day) 737 trades, 46.73%. Runs on indices, ETFs and stocks with volume, US and India, are a declared application.";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef { id: "scpenmfi_55", name: "Pendergast MFI 5-5: buy the next open after MFI(5) crosses 10, sell the next open after RSI(5) crosses back through 75, 5% stop", group: "S&C: Making A Good System Great (Pendergast, Jan 2011)", session: 0, dir: "Long", status: Status::Live,
        needs_volume: true, needs_vix: false, needs_series: &[], book: "scpenmfi", rules: &[
            "E|\"EntryTrigger = Ref((Cross( MFI(5 ),10)),-1); EntryPrice = OPEN;\" (TradeSim sidebar, PDF p.25)",
            "X|\"ExitTrigger = Ref(Cross( 75, RSI(5)),-1); ExitPrice = OPEN;\" (sidebar); \"I also programmed in a fixed stop-loss of 5% for every trade; no trailing stop was used\" (p.25)",
            "F|\"Filter colA > 7.50 AND Mov(VOLUME,50,SIMPLE) > 400000\"; \"All trade signal entries and exits take place at the next session's opening price\" (p.25)"],
        notes: NOTE, claim: None }]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    reviewed(id).then_some(("scpenmfi_55", "Pendergast MFI 5-5 (S&C)", "MFI 10 / RSI 75, 5% stop"))
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    reviewed(id).then(|| vec![p("text", "1 = the text's crossings (MFI below 10, RSI above 75)", 0., 0., 1., 1.), p("stop", "Fixed stop, % below entry (5)", 5., 0.5, 20., 0.5),
        p("filt", "1 = price > $7.50 (US) and 50-day volume > 400,000 on the signal day", 0., 0., 1., 1.)])
}

/// MetaStock MFI(n): typical price x volume summed by the direction of the typical price.
pub fn mfi(x: &Ctx, vol: &[f64], n: usize) -> Vec<f64> {
    let len = x.r.len();
    let tp: Vec<f64> = (0..len).map(|i| (x.h[i] + x.l[i] + x.c[i]) / 3.0).collect();
    let mut out = vec![f64::NAN; len];
    for i in n..len {
        if !vol[i + 1 - n..=i].iter().all(|v| v.is_finite()) { continue; }
        let (mut pos, mut neg) = (0.0, 0.0);
        for j in i + 1 - n..=i {
            if tp[j] > tp[j - 1] { pos += tp[j] * vol[j]; } else if tp[j] < tp[j - 1] { neg += tp[j] * vol[j]; }
        }
        out[i] = if neg == 0.0 { if pos == 0.0 { 50.0 } else { 100.0 } } else { 100.0 - 100.0 / (1.0 + pos / neg) };
    }
    out
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.r.len() { return None; }
    let g = |k: &str, d: f64| p.get(k).copied().unwrap_or(d);
    let (text, st, filt) = (g("text", 0.) > 0.5, g("stop", 5.), g("filt", 0.) > 0.5);
    let vol: Vec<f64> = x.r.iter().map(|r| r.volume.filter(|v| v.is_finite() && *v > 0.0).unwrap_or(f64::NAN)).collect();
    let (m, r, v50, us) = (mfi(x, &vol, 5), rsi(&x.c, 5), sma(&vol, 50), !india(x));
    let ok = |a: f64, b: f64| a.is_finite() && b.is_finite();
    let entry = |t: usize| t > 0 && ok(m[t - 1], m[t]) && if text { m[t] < 10.0 && m[t - 1] >= 10.0 } else { m[t] > 10.0 && m[t - 1] <= 10.0 }
        && (!filt || ((!us || x.c[t] > 7.5) && v50[t] > 400000.0));
    let exit = |t: usize| t > 0 && ok(r[t - 1], r[t]) && if text { r[t] > 75.0 && r[t - 1] <= 75.0 } else { r[t] < 75.0 && r[t - 1] >= 75.0 };
    let mut pos = Book::new();
    let (mut pend, mut stop) = (None::<bool>, f64::NAN);
    for i in from..=to {
        match pend.take() {
            Some(true) => { pos.enter(1.0, i, x.o[i], "open"); stop = x.o[i] * (1.0 - st / 100.0); }
            Some(false) => pos.exit(i, x.o[i], "RSI(5) exit"),
            None => {}
        }
        if pos.side() > 0.0 && x.l[i] <= stop { pos.exit(i, x.o[i].min(stop), "5% stop"); }
        if i < to { if pos.side() > 0.0 { if exit(i) { pend = Some(false); } } else if entry(i) { pend = Some(true); } }
    }
    Some(pos.done(to, x.c[to]))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn mfi_range_and_stop() {
        let r = super::super::sctraub::tests::bars(2500);
        let x = Ctx::new(&r, &r, &[]);
        let vol: Vec<f64> = r.iter().map(|z| z.volume.unwrap()).collect();
        let m = mfi(&x, &vol, 5);
        assert!(m[5..].iter().all(|v| (0.0..=100.0).contains(v)));
        for text in [0., 1.] {
            let t = run("scpenmfi_55", &x, 0, r.len() - 1, &[("text".to_string(), text)].into_iter().collect()).unwrap();
            assert!(!t.is_empty());
            for z in &t { assert!(z.exit_px >= z.legs[0].px * 0.95 - 1e-9 || z.exit_px == x.o[z.exit_i]); }
        }
    }
}

// Instrument admission: S&C Pendergast MFI 5-5 2026-11-02 (titles lane K19).
pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book == "scpenmfi" && reviewed(def.id),
    label: "Pendergast tested the MFI 5-5 system on 100 large-cap US stocks (2001-2010) in TradeSim portfolios. Runs on single indices, ETFs and stocks with volume, US and India, are a declared application.",
    basis: "Technical Analysis of Stocks & Commodities, January 2011, pp.24-30 (Making A Good System Great; MetaStock and TradeSim sidebars, OCR of PDF p.24-30); independent check authors/titles-1030/K19w/k19_ref.py",
};
