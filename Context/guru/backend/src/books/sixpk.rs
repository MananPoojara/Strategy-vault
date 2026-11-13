//! thedailytrade.com (Jon Andersen), "THE Six-Pack Trading System" (Six_Pack.pdf, 3 pp, plus "SEB Improved version.txt").
//! Book id `sixpk`. Source: research drive "USB HDD 2/tRADING KNO-2/The Six-Pack Trading System (thedailytrade.com)/".
//! Six oscillator systems for the E-mini, each gated by "the 34 sma [21 sma] confirms the direction of the signal".
//! The text never says what "confirms the direction" means (price above/below the SMA, or the SMA rising/falling),
//! the filter cannot be dropped, and no code or example resolves it. T3Osc also uses the undefined MetaStock custom formulas
//! Fml("T3") and Fml("T5").
//! Lane R2B (13 Nov 2026, R4 #297: up to two unstated details, each a labelled reading): SEB Classic, StoRSI and CMO3 are coded
//! with (1) "confirms the direction" read as the close above the SMA for buys / below it for shorts (option sma_mode = 1: the
//! SMA rising / falling) and (2) the covers filled at the next open like the entries (option cover = 1: at the signal close).
//! MasterBlaster (FibCMO precedence + the filter + cover fill = 3), AWEsome (fixed E-mini point triggers) and T3Osc (undefined
//! formulas) stay held. Independent check: authors/titles-1030/R2Bw/r2b_ref.py.
use crate::connors::{rsi, sma, Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "sixpk";
pub const REVIEWED: &[&str] = &["sixpk_seb_classic", "sixpk_storsi", "sixpk_cmo3"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }
const READ: &str = "F|Adapted: 2 unstated details (lane R2B, 13 Nov 2026, R4 #297). Unstated in the source: (1) what 'the SMA confirms the direction of the signal' means; vault reading: buys only when the signal close is above the SMA, short sales only when it is below (labelled option sma_mode = 1: the SMA higher / lower than the day before); (2) when a cover fills ('Cover the long position if there is a close above ...'); vault reading: at the next open, like the entries ('Buy the opening after ...'); labelled option cover = 1: at that close. A cover signal is also the opposite entry signal, so when the filter allows it the position reverses at the same open. Flat at the start of the window; no order for the bar after the window; the open position at the window end is marked at the last close. No stop (p.3: 'Stops ... can and should be used', none given). The source prints no results.";

const AMB: &str = "ambiguous filter: \"Initiate trades only if the 34 sma [21 sma] confirms the direction of the signal (the trend is your friend)\" does not say whether the close must be above/below the SMA or the SMA must be rising/falling, and the filter cannot be dropped; no code or example in the source resolves it";
const T3: &str = "undefined indicator: T3osc = Mov(Fml(\"T3\") - Ref(Fml(\"T5\"),-1),3,S) uses MetaStock custom formulas T3 and T5 that the source never defines; the 34-SMA filter is also ambiguous";
const NOTE: &str = "Written for the E-mini S&P (\"Mechanical Trading the E-Mini's\"); the coded cards (SEB Classic, StoRSI, CMO3; lane R2B, R4 #297) run on indices, ETFs and stocks as a declared application with the readings listed in their rules; the others stay held. p.3 'Additional Comments': \"Stops and additional exit criteria can and should be used\" (no stop given) and \"Any momentum oscillators can be substituted\" are generic advice, recorded but not rules.";

pub fn catalogue() -> Vec<StrategyDef> {
    let d = |id, name, status, rules| StrategyDef { id, name, group: "Six-Pack pp.1-3", session: 0, dir: "Long/Short", status,
        needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK, rules, notes: NOTE, claim: None };
    vec![
        d("sixpk_seb_classic", "1 · SEB Classic (Standard Error Bands)", Status::Live, &[
            "F|\"Initiate trades only if the 34 sma confirms the direction of the signal (the trend is your friend).\"",
            "E|\"Buy the opening after price closes below the lower standard error band.\" / \"Sell short the opening after price closes above the upper standard error band.\"",
            "X|\"Cover the long position if there is a close above the upper standard error band.\" / \"Cover the short position if there is a close below the lower standard error band.\"",
            "F|Formulae p.3: \"SEB = lin. reg:8; 2 std. err.; 3sma\": Jon Andersen's Standard Error Bands as MetaStock builds them: middle = 3-day simple average of the 8-day linear regression value (least-squares line through the last 8 closes, its value on the last bar); bands = middle +/- 2 x the 3-day simple average of the 8-day standard error of the estimate (sqrt(sum of squared residuals / (8 - 2)))",
            READ,
        ]),
        d("sixpk_storsi", "2 · StoRSI (Stochastic RSI)", Status::Live, &[
            "F|\"Initiate trades only if the 21 sma confirms the direction of the signal\"",
            "E|\"Buy the opening after StoRSI closes below the trigger of '10'.\" / \"Sell short the opening after StoRSI closes above the trigger of '90'.\"",
            "X|\"Cover the long position if there is a close above the trigger of '90'.\" / short: \"close below the trigger of '10'\"",
            "F|\"StoRSI = Mov( ( RSI(8) - LLV( RSI(8) ,8 ) ) / ( ( HHV( RSI(8) ,8) ) - LLV(RSI(8),8) ),3,E) *100\"; RSI = Wilder's (first average = mean of the first 8 changes); Mov(..,3,E) = exponential average, alpha 2/(3+1), started from the mean of its first 3 values; a day whose 8-day RSI range is zero has no value and the average waits for the next value",
            READ,
        ]),
        d("sixpk_masterblaster", "3 · MasterBlaster (FibCMO & SEB)", Status::NotTestable(AMB), &[
            "F|\"Initiate trades only if the 34 sma confirms the direction of the signal\"",
            "E|\"Buy the opening after the FibCMO closes below the trigger of '-34'.\" / \"Sell short the opening after FibCMO closes above the trigger of '+34'.\"",
            "X|\"Cover the long position if there is a close above the upper standard error band.\" / short: \"close below the lower standard error band\"",
            "F|\"FibCMO = CMO3+CMO5+CMO8/3\" (operator precedence also unresolved: (CMO3+CMO5+CMO8)/3 or CMO3+CMO5+CMO8/3)",
        ]),
        d("sixpk_awesome", "4 · AWEsome (MovAvg Osc)", Status::NotTestable(AMB), &[
            "F|\"Initiate trades only if the 21 sma confirms the direction of the signal\"; AWE = Mov(Mov(C,3,S)-Mov(C,13,S),3,S)",
            "E|\"Buy the opening after the AWE closes below the trigger of '-.4'.\" / \"Sell short the opening after price closes above the trigger of '+.4'.\" (text says 'price', formula context says AWE)",
            "X|\"Cover the long position if there is a close above '+.4'.\" / \"Cover the short position if there is a close below '-.4'.\" (absolute E-mini points; no index scaling given)",
        ]),
        d("sixpk_t3osc", "5 · T3Osc (T3 Oscillator)", Status::NotTestable(T3), &[
            "F|\"Initiate trades only if the 34 sma confirms the direction of the signal\"",
            "E|\"Buy the opening after the T3Osc closes below the trigger of '-1.5'.\" / short above '+1.5'",
            "X|\"Cover the long position if T3Osc close is above '+1.5'.\" / short below '-1.5'",
            "F|\"T3osc = Mov(Fml(\"T3\")- Ref(Fml(\"T5\"),-1),3,S)\"",
        ]),
        d("sixpk_cmo3", "6 · CMO3 (Chande Momentum Osc)", Status::Live, &[
            "F|\"Initiate trades only if the 34 sma confirms the direction of the signal\"; \"CMO3 = CMO set to 3 periods\"",
            "E|\"Buy the opening after the CMO3 closes below the trigger of '-21'.\" / \"Sell short the opening after CMO3 closes above the trigger of '+21'.\"",
            "X|\"Cover the long position if the CMO3 closes above the '+21'.\" / \"Cover the short position if the CMO3 closes below '-21'.\"",
            "F|CMO(3) = 100 x (sum of up close changes - sum of down close changes) / (their total) over the last 3 changes (Chande); no value when the 3 closes are unchanged",
            READ,
        ]),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    match id {
        "sixpk_seb_classic" => Some(("sixpk_seb_classic", "Six-Pack SEB Classic", "34-SMA filter")),
        "sixpk_storsi" => Some(("sixpk_storsi", "Six-Pack StoRSI 10/90", "21-SMA filter")),
        "sixpk_cmo3" => Some(("sixpk_cmo3", "Six-Pack CMO3 +/-21", "34-SMA filter")),
        _ => id.starts_with("sixpk_").then_some(("sixpk_six_pack", "Six-Pack Trading System", "Not testable")),
    }
}
pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    let (f, trig) = match id { "sixpk_seb_classic" => (34., 0.), "sixpk_storsi" => (21., 10.), "sixpk_cmo3" => (34., 21.), _ => return None };
    let mut v = vec![p("sma", "Trend filter SMA, days (book)", f, 2., 200., 1.),
        p("sma_mode", "'SMA confirms the direction': 0 = close above (buys) / below (shorts) the SMA; 1 = SMA rising / falling", 0., 0., 1., 1.),
        p("cover", "Cover fill: 0 = next open (like the entries); 1 = the signal close", 0., 0., 1., 1.)];
    if trig > 0. { v.push(p("trig", if id == "sixpk_storsi" { "Lower trigger (upper = 100 - trigger; book 10)" } else { "Trigger, +/- (book 21)" }, trig, 1., 49., 1.)); }
    Some(v)
}

/// Least-squares line through v[i+1-n..=i] (x = 1..n): (value at x = n, standard error of the estimate).
fn lr_ste(v: &[f64], i: usize, n: usize) -> (f64, f64) {
    let w = &v[i + 1 - n..=i];
    let nf = n as f64;
    let (sx, sy) = ((1..=n).map(|k| k as f64).sum::<f64>(), w.iter().sum::<f64>());
    let sxx: f64 = (1..=n).map(|k| (k * k) as f64).sum();
    let sxy: f64 = w.iter().enumerate().map(|(k, y)| (k + 1) as f64 * y).sum();
    let b = (nf * sxy - sx * sy) / (nf * sxx - sx * sx);
    let a = (sy - b * sx) / nf;
    let sse: f64 = w.iter().enumerate().map(|(k, y)| { let e = y - (a + b * (k + 1) as f64); e * e }).sum();
    (a + b * nf, (sse / (nf - 2.)).sqrt())
}
/// Exponential average, alpha 2/(n+1), started from the mean of its first n finite values; a NaN keeps the last value but is
/// reported as NaN.
fn ema_skip(v: &[f64], n: usize) -> Vec<f64> {
    let k = 2.0 / (n as f64 + 1.0);
    let (mut out, mut e, mut seen, mut sum) = (vec![f64::NAN; v.len()], f64::NAN, 0usize, 0.0);
    for (i, &x) in v.iter().enumerate() {
        if !x.is_finite() { continue; }
        if seen < n { seen += 1; sum += x; if seen == n { e = sum / n as f64; } } else { e += k * (x - e); }
        out[i] = e;
    }
    out
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    let schema = params(id)?;
    if from >= to || to >= x.c.len() { return None; }
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).map_or(0., |d| d.def));
    let (c, n) = (&x.c, x.c.len());
    let fl = (g("sma").round() as usize).max(2);
    let ma = sma(c, fl);
    let slope_mode = g("sma_mode") >= 0.5;
    let cover_close = g("cover") >= 0.5;
    // lo[i] / hi[i]: the close-of-day buy (cover-short) and sell-short (cover-long) conditions
    let (lo, hi): (Vec<bool>, Vec<bool>) = match id {
        "sixpk_seb_classic" => {
            let (mut lr, mut se) = (vec![f64::NAN; n], vec![f64::NAN; n]);
            for i in 7..n { let (a, b) = lr_ste(c, i, 8); lr[i] = a; se[i] = b; }
            let (m, s) = (sma(&lr, 3), sma(&se, 3));
            (0..n).map(|i| (c[i] < m[i] - 2. * s[i], c[i] > m[i] + 2. * s[i])).unzip()
        }
        "sixpk_storsi" => {
            let r = rsi(c, 8);
            let raw: Vec<f64> = (0..n).map(|i| {
                if i < 7 || r[i + 1 - 8..=i].iter().any(|v| v.is_nan()) { return f64::NAN; }
                let (lo, hi) = r[i + 1 - 8..=i].iter().fold((f64::INFINITY, f64::NEG_INFINITY), |(a, b), &v| (a.min(v), b.max(v)));
                if hi > lo { (r[i] - lo) / (hi - lo) } else { f64::NAN }
            }).collect();
            let sr: Vec<f64> = ema_skip(&raw, 3).iter().map(|v| v * 100.).collect();
            let t = g("trig");
            (0..n).map(|i| (sr[i] < t, sr[i] > 100. - t)).unzip()
        }
        "sixpk_cmo3" => {
            let t = g("trig");
            (0..n).map(|i| {
                if i < 3 { return (false, false); }
                let (mut u, mut d) = (0., 0.);
                for k in i - 2..=i { let z = c[k] - c[k - 1]; if z > 0. { u += z } else { d -= z } }
                if u + d > 0. { let v = 100. * (u - d) / (u + d); (v < -t, v > t) } else { (false, false) }
            }).unzip()
        }
        _ => return None,
    };
    let ok = |i: usize, d: f64| -> bool {
        if !ma[i].is_finite() { return false; }
        // SMA[i] - SMA[i-1] = (C[i] - C[i-n]) / n: its sign from the two closes (exact, no rounding of two averages)
        if slope_mode { i >= fl && ma[i - 1].is_finite() && d * (c[i] - c[i - fl]) > 0. } else { d * (c[i] - ma[i]) > 0. }
    };
    let mk = |d: f64, i: usize, px: f64, s: usize| Trade { dir: d, legs: vec![Leg { i, px, w: 1. }], exit_i: i, exit_px: px, reason: String::new(), entry_kind: "open", signal_i: s, open: false, exits: vec![] };
    let (mut out, mut cur, mut pend): (Vec<Trade>, Option<Trade>, Option<(bool, f64, usize)>) = (vec![], None, None);
    for i in from..=to {
        if let Some((close_it, d, s)) = pend.take() {
            if close_it { if let Some(mut t) = cur.take() { t.exit_i = i; t.exit_px = x.o[i]; t.signal_i = s; t.reason = "cover: next open".into(); out.push(t); } }
            if d != 0. { cur = Some(mk(d, i, x.o[i], s)); }
        }
        if i == to { break; }
        let side = cur.as_ref().map_or(0., |t| t.dir);
        let cover = (side > 0. && hi[i]) || (side < 0. && lo[i]);
        if cover && cover_close {
            let mut t = cur.take().unwrap(); t.exit_i = i; t.exit_px = c[i]; t.signal_i = i; t.reason = "cover: signal close".into(); out.push(t);
        }
        let flat_next = cur.is_none() || cover;
        let want = if flat_next && lo[i] && ok(i, 1.) && side <= 0. { 1. } else if flat_next && hi[i] && ok(i, -1.) && side >= 0. { -1. } else { 0. };
        if (cover && !cover_close) || want != 0. { pend = Some((cover && !cover_close, want, i)); }
    }
    if let Some(mut t) = cur { t.exit_i = to; t.exit_px = c[to]; t.signal_i = to; t.reason = "open".into(); t.open = true; out.push(t); }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn three_coded_three_held() {
        let c = super::catalogue();
        assert_eq!(c.len(), 6);
        for s in &c {
            assert_eq!(matches!(s.status, crate::connors::Status::Live), reviewed(s.id), "{}", s.id);
            assert!(family_of(s.id).is_some() && params(s.id).is_some() == reviewed(s.id));
        }
    }
    #[test]
    fn regression_line_and_standard_error() {
        // exact line: zero error; value at the last x
        let v: Vec<f64> = (0..8).map(|k| 3. + 2. * k as f64).collect();
        let (a, e) = lr_ste(&v, 7, 8);
        assert!((a - 17.).abs() < 1e-12 && e.abs() < 1e-9);
        // y = 0,0,0,0,0,0,0,8: b = Sxy/Sxx = 28/42 = 2/3, fitted value at x = 8: mean 1 + b*(8 - 4.5) = 10/3
        let w = [0., 0., 0., 0., 0., 0., 0., 8.];
        let (a, e) = lr_ste(&w, 7, 8);
        assert!((a - 10. / 3.).abs() < 1e-9, "{a}");
        assert!(e > 0.);
    }
    #[test]
    fn entries_next_open_filtered_covers_reverse() {
        let r: Vec<crate::Row> = (0..600).map(|i| { let t = i as f64; let c = 100. + 0.05 * t + 6. * (t / 9.).sin() + 2. * (t / 2.3).cos();
            crate::Row { ticker: "T".into(), date: format!("{:05}", i), open: c - 0.3 * (t / 1.7).sin(), high: c + 1., low: c - 1., close: c, volume: None } }).collect();
        let x = Ctx::new(&r, &r, &[]);
        let ma = sma(&x.c, 34);
        for id in REVIEWED {
            let t = run(id, &x, 0, 599, &Params::new()).unwrap();
            assert!(t.len() > 4, "{id}");
            for z in &t {
                let s = z.legs[0].i - 1; // signal close
                assert_eq!(z.legs[0].px, x.o[z.legs[0].i]);
                if *id != "sixpk_storsi" { assert!(z.dir * (x.c[s] - ma[s]) > 0., "{id}"); }
            }
            for w in t.windows(2) { assert!(w[1].legs[0].i >= w[0].exit_i); }
        }
    }
}

// Lane R2B (13 Nov 2026, R4 #297): SEB Classic, StoRSI and CMO3 with their two labelled readings.
pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book == BOOK && reviewed(def.id),
    label: "Jon Andersen wrote these Six-Pack rules for the E-mini S&P with no test results; runs on indices, ETFs and stocks, US and India, are a declared application with the filter and cover fill read as stated on the card.",
    basis: "THE Six-Pack Trading System (thedailytrade.com, Six_Pack.pdf p.1-3); R4 #297; independent check authors/titles-1030/R2Bw/r2b_ref.py",
};
