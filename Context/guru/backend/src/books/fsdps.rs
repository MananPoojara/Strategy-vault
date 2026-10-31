//! Fred G. Schutzman, "Developing Profitable Trading Systems: from Discretionary to Mechanical" (TAG 1997 seminar
//! workbook, INO; book id `fsdps`; PDF pages). Reading: claude-continuation-20261020/authors/schutzman
//! (admission/dps_ref.py; OCR authors/overnight-1029/T2w/txt/1380.ocr.txt). Live: the two printed EasyLanguage systems
//! DJ.TrendFollowingSys (PDF 7, logic by Alex Crokos from Marty Zweig: reverse on a 2% rise from the lowest close or a
//! 6% fall from the highest close) and DJ.CombinationSystem (PDF 15, logic by Dennis Hynes from an Elder article: MACD
//! rising and a close under the 5-day 1-sd Bollinger band buys, MACD falling and a close over the upper band sells).
//! Held: DJ.CounterTrendSys (PDF 10) - TradeStation's SlowD(10) smoothing is not printed and its "1 point" stop offset is
//! a futures point. Not the masmt 6% models: the code measures the fall as highest close / close >= 1.06, not 6% off the
//! high, and makes no trade on the first signal.
//! Book: S&P 500, silver, copper, Canadian dollar futures; indices, ETFs and US/India stocks are a declared application.
use crate::connors::{Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};
use super::pbwts::{avg_at, std_pop};
use super::tilma::ema;
/// Admitted ids (authors/schutzman/admission/dps_ref.py (header)).
pub const REVIEWED: &[&str] = &["fsdps_zweig_2_6", "fsdps_combination"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const NOTE_Z: &str = "[Declared readings] The code runs from the first bar the vault holds (LC = HC = that close). Before the first signal the trend is only set, with no trade (\"0 = no trades yet\"). Up: close / lowest close >= 1.02; down: highest close / close >= 1.06 (the code's ratios, so the fall is about 5.7% off the high). A reversal sells (buys) and goes short (long) at the next day's open (code default openOrCl = \"Open\"; fill 1 = this close), resetting the lowest (highest) close to today's close. Always in once a trade starts; only reversals inside the test window open trades. A signal on the last bar is not filled; a position still open on the last bar is marked at its close.";
const NOTE_C: &str = "[Declared readings] MACD = XAverage(close,12) - XAverage(close,26), each started at the first close the vault holds (TradeStation XAverage). BollingerBand(close,5,-+1) = 5-day average -+ 1 standard deviation (population, divided by 5). \"buy at Market\" / \"sell at Market\" = the next day's open; the opposite signal reverses (\"Exit on a signal in the opposite direction\"); a signal in the direction already held is ignored. No stop (\"MM stop = not in place on this version\"). A signal on the last bar is not filled; a position still open on the last bar is marked at its close.";

pub fn catalogue() -> Vec<StrategyDef> {
    let d = |id: &'static str, name: &'static str, rules: &'static [&'static str], notes: &'static str| StrategyDef { id, name, group: "Schutzman TAG 1997 systems", session: 0,
        dir: "Long/Short", status: Status::Live, needs_volume: false, needs_vix: false, needs_series: &[], book: "fsdps", rules, notes, claim: None };
    vec![
        d("fsdps_zweig_2_6", "Zweig 2% / 6% close reversal (DJ.TrendFollowingSys)", &[
            "F|\"Program initiates trades on a 2% up move or a 6% down move\"; inputs perOffLo(2.00), perOffHi(6.00), openOrCl(\"Open\") (PDF 7)",
            "E|\"else if trend = -1 and (close / LC) >= (1 + perOffLo/100) then begin trend = +1; HC = close; ... buy at market\" (PDF 7)",
            "X|\"else if trend = +1 and (HC / close) >= (1 + perOffHi/100) then begin trend = -1; LC = close; ... sell at market\"; \"if close < LC then LC = close; if close > HC then HC = close\" (PDF 7)"], NOTE_Z),
        d("fsdps_combination", "MACD direction + 5-day Bollinger pullback, stop and reverse (DJ.CombinationSystem)", &[
            "F|\"osc = MACD(close, fastMA, slowMA)\" with fastMA(12), slowMA(26); length(5), sDev(1) (PDF 15)",
            "E|\"if osc > osc[1] and close < BollingerBand(close, length, -sDev) then buy at Market\" (PDF 15)",
            "X|\"if osc < osc[1] and close > BollingerBand(close, length, +sDev) then sell at Market\"; \"Exit on a signal in the opposite direction\" (PDF 15)"], NOTE_C),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    match id {
        "fsdps_zweig_2_6" => Some(("fsdps_zweig_2_6", "Schutzman Zweig 2/6 reversal", "2% / 6%")),
        "fsdps_combination" => Some(("fsdps_combination", "Schutzman MACD + Bollinger combination", "12/26, 5 x 1 sd")),
        _ => None,
    }
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    match id {
        "fsdps_zweig_2_6" => Some(vec![p("lo", "Up move % off the lowest close (book 2)", 2., 0.5, 20., 0.5), p("hi", "Down move % off the highest close (book 6)", 6., 0.5, 20., 0.5),
            p("fill", "Fill: 0 = next day's open (code default), 1 = this close (code option)", 0., 0., 1., 1.)]),
        "fsdps_combination" => Some(vec![p("fast", "MACD fast average (book 12)", 12., 2., 50., 1.), p("slow", "MACD slow average (book 26)", 26., 5., 100., 1.),
            p("len", "Bollinger days (book 5)", 5., 2., 50., 1.), p("sd", "Bollinger standard deviations (book 1)", 1., 0.5, 3., 0.5)]),
        _ => None,
    }
}

fn mk(d: f64, ei: usize, ep: f64, kind: &'static str, xi: usize, xp: f64, why: &str, si: usize, open: bool) -> Trade {
    Trade { dir: d, legs: vec![Leg { i: ei, px: ep, w: 1. }], exit_i: xi, exit_px: xp, reason: why.into(), entry_kind: kind, signal_i: si, open, exits: vec![] }
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.r.len() { return None; }
    let schema = params(id)?;
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).map_or(f64::NAN, |d| d.def));
    let c = &x.c;
    // signals: (bar, +1 buy / -1 sell); filled at the next open unless `close`
    let mut sig: Vec<(usize, f64)> = vec![];
    let close = id == "fsdps_zweig_2_6" && g("fill").round() >= 1.;
    match id {
        "fsdps_zweig_2_6" => {
            let (lo, hi) = (1. + g("lo") / 100., 1. + g("hi") / 100.);
            let (mut lc, mut hc, mut tr) = (c[0], c[0], 0);
            for t in 0..to {
                if tr == 0 {
                    if c[t] / lc >= lo { tr = 1; }
                    if hc / c[t] >= hi { tr = -1; }
                } else if tr == 1 && hc / c[t] >= hi { tr = -1; lc = c[t]; sig.push((t, -1.)); }
                else if tr == -1 && c[t] / lc >= lo { tr = 1; hc = c[t]; sig.push((t, 1.)); }
                if c[t] < lc { lc = c[t]; }
                if c[t] > hc { hc = c[t]; }
            }
        }
        "fsdps_combination" => {
            let u = |k: &str| g(k).round().max(1.) as usize;
            let (f, s) = (ema(c, u("fast")), ema(c, u("slow")));
            let n = u("len");
            let sd = std_pop(c, n);
            for t in 1..to {
                let (o, o1) = (f[t] - s[t], f[t - 1] - s[t - 1]);
                let (m, w) = (avg_at(c, t, n), g("sd") * sd[t]);
                if o > o1 && c[t] < m - w { sig.push((t, 1.)); }
                else if o < o1 && c[t] > m + w { sig.push((t, -1.)); }
            }
        }
        _ => return None,
    }
    let kind = if close { "close" } else { "open" };
    let (mut out, mut cur) = (vec![], None::<(f64, usize, f64)>);
    for (t, d) in sig {
        if t < from || cur.is_some_and(|z| z.0 == d) { continue; }
        let (fi, fp) = if close { (t, c[t]) } else { (t + 1, x.o[t + 1]) };
        if let Some((cd, ei, ep)) = cur { out.push(mk(cd, ei, ep, kind, fi, fp, "opposite signal", t, false)); }
        cur = Some((d, fi, fp));
    }
    if let Some((d, ei, ep)) = cur { out.push(mk(d, ei, ep, kind, to, c[to], "open", to, true)); }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;
    #[test]
    fn hand_checked_zweig_reversals() {
        // 100 -> 103 sets the trend up (no trade); peak 110; 103.7 (110/103.7 = 1.0608) sells short at the next open;
        // the low resets to 103.7, falls to 100; 102.5 (> 1.02 x 100) buys at the next open.
        let cl = [100., 103., 110., 104., 103.7, 101., 100., 101., 102.5, 102.];
        let r: Vec<Row> = cl.iter().enumerate().map(|(i, &c)| Row { ticker: "T".into(), date: format!("2020-04-{:02}", i + 1), open: c + 0.5, high: c + 1., low: c - 1., close: c, volume: None }).collect();
        let x = Ctx::new(&r, &r, &[]);
        let t = run("fsdps_zweig_2_6", &x, 0, r.len() - 1, &Params::new()).unwrap();
        let got: Vec<_> = t.iter().map(|z| (z.dir, z.legs[0].i, z.legs[0].px, z.exit_i, z.exit_px, z.open)).collect();
        assert_eq!(got, vec![(-1., 5, 101.5, 9, 102.5, false), (1., 9, 102.5, 9, 102., true)]);
        assert!(run("fsdps_x", &x, 0, r.len() - 1, &Params::new()).is_none());
    }
}

// Instrument admission: Fred G. Schutzman 2026-10-29 (lane K1) block, moved from instrument_policy.rs on 31 Oct 2026 (lane ST).
// Admission only for the module REVIEWED lists (authors/schutzman/admission/dps_ref.py (header)); kept in one block to merge.
// Runs on actual indices, ETFs and stocks, US and India (index_etf_or_stock, nifty50 basket, declared application).
pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book=="fsdps" && crate::books::fsdps::reviewed(def.id),
    label: "Fred Schutzman tests these EasyLanguage systems on S&P 500, silver, copper and Canadian dollar futures; runs on indices, ETFs and US/India stocks are a declared application, not the author's test.",
    basis: "Fred G. Schutzman, Developing Profitable Trading Systems (TAG 1997 workbook), PDF 7 and 15",
};
