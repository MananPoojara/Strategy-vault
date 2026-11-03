//! Robert M. Barnes, "Timing is Everything" (INO workshop workbook; research share Trading Knowledge II/@INO (ino.com)/
//! Robert M.Barnes - Timing is Everything/workbook.pdf, image-only, read by OCR). Book id `rbtie`. Live: Part 1, the
//! Directional Relative Volatility (DRV) stop-and-reverse (PDF p.5-7, Table 1.7 p.16), and Part 2, the Volume-Price
//! method, trend version with the contrary version as an option (PDF p.23-25, Tables 2.6-2.7 p.30-31). Not coded:
//! Part 3 'Time x Price' (needs the futures UNIT per market; the index rows of Table 3.5 are unreadable).
//! Lane K6 (titles loop, 31 Oct 2026); independent check authors/titles-1030/K6w/k6_ref.py (header).
use crate::connors::{Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};

/// Admitted ids (authors/titles-1030/K6w/k6_ref.py (header)).
pub const REVIEWED: &[&str] = &["rbtie_drv", "rbtie_volume_price"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const NOTE_DRV: &str = "[Declared readings] P = the daily close; MC and MMC start at 0 on the first day of the test window ('for day 1, by convention, MC(1) = 0', p.6) and the first 20 days are not traded ('at least 10, and perhaps 20 days', p.7). Long when DRV >= C, short when DRV <= -C, otherwise the position is kept: the workbook's trade tables (p.11) reverse from long to short with each exit date equal to the next entry date, so the system is always in after its first signal. Signal and fill at the close of the signal day. Defaults = Table 1.7's S&P 500 futures optimum 1984-93 (A = 0.02, C = 0.18; NYSE futures the same). Not coded: the tables' 'size' and 'after time' stops (E 99999, G 3000, H 9999), which the workbook does not define and which are futures dollar values. Book: S&P futures; indices, ETFs and US/India stocks here are a declared application.";
const NOTE_VP: &str = "[Declared readings] MV and M start at that day's volume and close on the first day of the test window and the first 20 days are not traded (the warm-up the workbook gives for DRV, p.7). Trend version (default, p.25): with volume expansion, (MV(i) - MV(i-1))/MV(i-1) >= C, go long when (M(i) - M(i-1))/M(i-1) >= D and short when (M(i-1) - M(i))/M(i-1) >= D; close the position when volume growth tapers off, read as the same relative volume change falling below C; signals and fills at the close; flat between trades; a volume expansion with the opposite price move reverses. Contrary version (mode 1, p.25): on volume expansion take the position opposite to the price move and hold it until the opposite signal (the workbook's 'reversal only mode'); Table 2.7's S&P values for it are A 0.07, C 0.04, D 0. Defaults = Table 2.6's S&P 500 futures trend optimum (June 1984-1993): A 0.02, C 0.012, D 0.0006. Not coded: the 'size' and 'after time' stops (undefined futures dollar values). Needs daily volume (ETFs and stocks; the SPX index file has no volume since 25 Sep 2026). Book: S&P futures; ETFs and stocks here are a declared application.";

pub fn catalogue() -> Vec<StrategyDef> {
    let d = |id, name, vol, rules, notes| StrategyDef { id, name, group: "Barnes Timing is Everything", session: 0, dir: "Long/Short", status: Status::Live,
        needs_volume: vol, needs_vix: false, needs_series: &[], book: "rbtie", rules, notes, claim: None };
    vec![
        d("rbtie_drv", "Directional Relative Volatility: always in, long at DRV >= C, short at DRV <= -C", false, &[
            "F|\"MC(i) = A x (P(i) - P(i-1)) + (1 - A) x MC(i-1)\" (p.6); \"MMC(i) = A x abs(P(i) - P(i-1)) + (1 - A) x MMC(i-1)\" (p.7); \"DRV(i) = MC(i)/MMC(i)\" (p.7)",
            "E|\"1. Buy when DRV >= C  2. Sell when DRV <= -C\" (p.5); trade tables reverse long/short (p.11)",
            "X|Stop and reverse: the opposite signal closes the position and opens the other side (trade tables p.11, exit date = next entry date)",
            "F|Table 1.7 S&P: A = 0.02, C = 0.18 (p.16); \"make sure a number of days go by before actual trading can be started ... at least 10, and perhaps 20 days\" (p.7)"], NOTE_DRV),
        d("rbtie_volume_price", "Volume-Price method: volume expansion with a price move, out when volume growth tapers off", true, &[
            "F|\"MV(i) = A x V(i) + (1 - A) x MV(i-1)\"; \"M(i) = A x C(i) + (1 - A) x M(i-1)\" (p.23)",
            "E|\"If (MV(i) - MV(i-1))/MV(i-1) >= C ... (M(i) - M(i-1))/M(i-1) >= D means go long\"; the mirror goes short (trend method, p.25)",
            "X|\"we will close out the position, long or short, for this trend strategy, when volume growth has tapered off, or when MV(i) - MV(i-1) < C\" (p.25)",
            "F|Table 2.6 S&P trend optimum: A 0.02, C 0.012, D 0.0006; contrary version Table 2.7 S&P: A 0.07, C 0.04, D 0 (p.30-31)"], NOTE_VP),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    match id {
        "rbtie_drv" => Some(("rbtie_drv", "Barnes DRV", "A 0.02, C 0.18, stop and reverse")),
        "rbtie_volume_price" => Some(("rbtie_volume_price", "Barnes Volume-Price", "trend version, A 0.02")),
        _ => None,
    }
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    let warm = || p("warm", "Days not traded at the start (book: 10 to 20)", 20., 0., 60., 1.);
    match id {
        "rbtie_drv" => Some(vec![p("a", "A, weight of today's change (Table 1.7 S&P 0.02)", 0.02, 0.01, 1., 0.01),
            p("c", "C, DRV level (Table 1.7 S&P 0.18)", 0.18, 0.01, 1., 0.01), warm()]),
        "rbtie_volume_price" => Some(vec![p("a", "A, average weight (Table 2.6 S&P 0.02)", 0.02, 0.01, 1., 0.01),
            p("c", "C, relative volume-average rise (Table 2.6 S&P 0.012)", 0.012, 0., 0.2, 0.001),
            p("d", "D, relative price-average move (Table 2.6 S&P 0.0006)", 0.0006, 0., 0.05, 0.0001),
            p("mode", "0 = trend version (default), 1 = contrary version, reversal only", 0., 0., 1., 1.), warm()]),
        _ => None,
    }
}

pub fn mk(d: f64, ei: usize, ep: f64, kind: &'static str, xi: usize, xp: f64, why: &str, open: bool, sig: usize) -> Trade {
    Trade { dir: d, legs: vec![Leg { i: ei, px: ep, w: 1. }], exit_i: xi, exit_px: xp, reason: why.into(), entry_kind: kind, signal_i: sig, open, exits: vec![] }
}

/// Target position per bar, decided at that bar's close (1 long, -1 short, 0 flat), turned into trades inside [from, to].
/// The position changes only on a bar where the target changes, so a window never opens in the middle of an old signal.
/// Fill at that close, or (next_open) at the next bar's open (no fill after `to`). A position still open is marked at c[to].
pub fn target_trades(x: &Ctx, tgt: &[i8], from: usize, to: usize, next_open: bool, why: &str) -> Vec<Trade> {
    let (mut out, mut cur) = (vec![], None::<(i8, usize, f64)>);
    let kind = if next_open { "open" } else { "close" };
    for i in from..=to {
        if i > 0 && tgt[i] == tgt[i - 1] { continue; }
        let want = tgt[i];
        if cur.map_or(0, |c| c.0) == want { continue; }
        if next_open && i + 1 > to { break; }
        let (k, px) = if next_open { (i + 1, x.o[i + 1]) } else { (i, x.c[i]) };
        if let Some((d, e, ep)) = cur.take() { out.push(mk(d as f64, e, ep, kind, k, px, why, false, i)); }
        if want != 0 { cur = Some((want, k, px)); }
    }
    if let Some((d, e, ep)) = cur { out.push(mk(d as f64, e, ep, kind, to, x.c[to], "open", true, to)); }
    out
}

/// DRV = MC/MMC with MC = MMC = 0 on bar `from` (NaN before it and while MMC = 0).
pub fn drv(c: &[f64], from: usize, to: usize, a: f64) -> Vec<f64> {
    let mut out = vec![f64::NAN; c.len()];
    let (mut mc, mut mmc) = (0., 0.);
    for i in from + 1..=to {
        let d = c[i] - c[i - 1];
        mc = a * d + (1. - a) * mc;
        mmc = a * d.abs() + (1. - a) * mmc;
        if mmc > 0. { out[i] = mc / mmc; }
    }
    out
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.r.len() { return None; }
    let schema = params(id)?;
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).map_or(f64::NAN, |d| d.def));
    let (a, cl, warm) = (g("a").clamp(0.0001, 1.), g("c"), g("warm").round().max(0.) as usize);
    let mut tgt = vec![0i8; x.c.len()];
    if id == "rbtie_drv" {
        let v = drv(&x.c, from, to, a);
        for i in from + 1..=to {
            tgt[i] = if i < from + warm { 0 } else if v[i] >= cl { 1 } else if v[i] <= -cl { -1 } else { tgt[i - 1] };
        }
        return Some(target_trades(x, &tgt, from, to, false, "DRV reversal"));
    }
    let (dl, contrary) = (g("d"), g("mode").round() >= 1.);
    let vol: Vec<f64> = x.r.iter().map(|r| r.volume.unwrap_or(f64::NAN)).collect();
    let (mut mv, mut m) = (vol[from], x.c[from]);
    for i in from + 1..=to {
        let (mv0, m0) = (mv, m);
        mv = a * vol[i] + (1. - a) * mv;
        m = a * x.c[i] + (1. - a) * m;
        let vc = (mv - mv0) / mv0 >= cl;
        let (up, dn) = ((m - m0) / m0 >= dl, (m0 - m) / m0 >= dl);
        let mv_dir: i8 = if vc && up && !dn { 1 } else if vc && dn && !up { -1 } else { 0 };
        tgt[i] = if i < from + warm { 0 } else if contrary { if mv_dir != 0 { -mv_dir } else { tgt[i - 1] } }
            else if !vc { 0 } else if mv_dir != 0 { mv_dir } else { tgt[i - 1] };
    }
    Some(target_trades(x, &tgt, from, to, false, if contrary { "contrary reversal" } else { "volume growth tapered off" }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;
    fn bars(c: &[f64], v: Option<&[f64]>) -> Vec<Row> {
        c.iter().enumerate().map(|(i, &c)| Row { ticker: "T".into(), date: format!("2001-01-{:02}", i + 1), open: c, high: c, low: c, close: c, volume: v.map(|v| v[i]) }).collect()
    }
    #[test]
    fn drv_matches_the_workbook_example_and_reverses() {
        // p.6-7: P = 350, 355, 357, 356, 358, A = 0.3 -> DRV(2..5) = 1, 1, 0.5876, 0.7405
        let v = drv(&[350., 355., 357., 356., 358.], 0, 4, 0.3);
        assert!(v[0].is_nan() && (v[1] - 1.).abs() < 1e-12 && (v[2] - 1.).abs() < 1e-12);
        assert!((v[3] - 0.855 / 1.455).abs() < 1e-12 && (v[4] - 1.1985 / 1.6185).abs() < 1e-12);
        let c = [10., 11., 12., 13., 12., 11., 10., 9., 10., 11.];
        let r = bars(&c, None);
        let x = Ctx::new(&r, &r, &[]);
        let p: Params = [("a".to_string(), 0.5), ("c".to_string(), 0.5), ("warm".to_string(), 0.)].into_iter().collect();
        let t = run("rbtie_drv", &x, 0, 9, &p).unwrap();
        // DRV: bar1 1 -> long at 11; bar4 -0.0625/0.9375 -> kept; bar5 -0.53125/0.96875 = -0.55 -> short at 11
        let got: Vec<_> = t.iter().map(|z| (z.dir, z.legs[0].i, z.exit_i, z.open)).collect();
        assert_eq!(got[0], (1., 1, 5, false));
        assert_eq!(got[1].0, -1.);
    }
    #[test]
    fn volume_price_trend_needs_expansion_and_exits_when_it_fades() {
        let c = [10., 10., 11., 12., 12., 12.];
        let v = [100., 100., 200., 300., 100., 100.];
        let r = bars(&c, Some(&v));
        let x = Ctx::new(&r, &r, &[]);
        let p: Params = [("a".to_string(), 0.5), ("c".to_string(), 0.1), ("d".to_string(), 0.01), ("warm".to_string(), 0.)].into_iter().collect();
        let t = run("rbtie_volume_price", &x, 0, 5, &p).unwrap();
        // MV 100,100,150,225,162.5: bar2 +50% & M +5% -> long 11; bar3 +50%, M up -> hold; bar4 -28% -> out at 12
        assert_eq!(t.iter().map(|z| (z.dir, z.legs[0].i, z.exit_i)).collect::<Vec<_>>(), vec![(1., 2, 4)]);
        assert!(run("rbtie_x", &x, 0, 5, &p).is_none());
    }
}

// Instrument admission: Robert M. Barnes (Timing is Everything) 2026-10-31 (titles lane K6).
// Admission only for the module REVIEWED lists (authors/titles-1030/K6w/k6_ref.py (header)).
// Runs on actual indices, ETFs and stocks, US and India (index_etf_or_stock, nifty50 basket, declared application).
pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book == "rbtie" && crate::books::rbtie::reviewed(def.id),
    label: "Robert Barnes optimised these rules on S&P 500 futures (1984-93); runs on indices, ETFs and US/India stocks are a declared application, not the author's test.",
    basis: "Robert M. Barnes, Timing is Everything (INO workbook), DRV p.5-7 and Table 1.7 p.16; Volume-Price p.23-25 and Tables 2.6-2.7 p.30-31 (OCR)",
};
