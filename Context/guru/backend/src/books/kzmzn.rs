//! "Metastock Zone" (a web collection of MetaStock list posts and Traders' Tips, in the K.A.L.O.S.Z archive folder: research share
//! USB HDD 1/MetaStock/@Systems & Indicators/K.A.L.O.S.Z Archive of Metastock Formulas/Metastock Zone.pdf). Book id `kzmzn`. Four
//! System Tester systems: Key Reversals (Scorpio / Equis), StochRSI (Baude, after Chande), StoRSI (Karnish) and Onno's Binary Wave.
//! Lane K8 (titles loop, 1 Nov 2026); independent check authors/titles-1030/K8w/k8_ref.py (header).
use crate::connors::{rsi, Ctx, ParamDef, Params, Status, StrategyDef, Trade};
use super::mkits::{ema, hhv, llv};
use super::rbtie::{mk, target_trades};

/// Admitted ids (authors/titles-1030/K8w/k8_ref.py (header)).
pub const REVIEWED: &[&str] = &["kzmzn_key_rev", "kzmzn_stochrsi", "kzmzn_storsi", "kzmzn_binary_wave"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

macro_rules! common { () => { " Fills at the signal day's close (the MetaStock System Tester default, no delay). MetaStock built-ins as documented: RSI = Wilder's, Mo(n) = 100 x C / C n bars ago, Mov(..,E) = exponential average. A forum/vendor sample with no printed results; ETFs, US/India indices and stocks are a declared application." } }

const KEY: &str = concat!("[Declared readings] Massimiliano Scorpio's key reversal as coded by Equis (S&C Dec 2001 Traders' Tips): buy at the close of a day with a lower low, a higher close and a close below its high; close the long at the close of the first later day whose high is at or above the previous high; Maximum Loss stop 2% below the entry price, hit inside the day (fill at the stop, or the open on a gap). Exits are tested from the day after entry; no new entry on an exit day. Long only.", common!());
const SRSI: &str = concat!("[Declared readings] StochRSI(n, s) = 100 x Sum(RSI(n) - LLV(RSI(n), n), s) / Sum(HHV(RSI(n), n) - LLV(RSI(n), n), s) for (14,3), (70,15), (252,54). Buy at the close when StochRSI-14 crosses from below 20 to 20 or above, and either StochRSI-70 or StochRSI-252 is at least 70 (rule #1), or StochRSI-70 is at least 50 and its largest one-day rise in the last 21 days is more than 3 points (rule #2, as coded: HHV(ROC(..,1,Points),21) > 3); the English rules require the StochRSI-14 cross in both (the MSW code's operator grouping drops it from rule #2). Sell: the poster's example 'CLOSE > Ref(CLOSE,-3)' under 'Sell Rule: Whatever you decide upon', labelled the author's example exit, tested from the day after entry. Long only.", common!());
const STO: &str = concat!("[Declared readings] StoRSI = Mov((RSI(8) - LLV(RSI(8),8)) / (HHV(RSI(8),8) - LLV(RSI(8),8)), 3, E) x 100, the exponential average starting from the simple average of its first 3 values. Enter long on Cross(17, StoRSI) (falls below 17), enter short on Cross(StoRSI, 83); each entry reverses the other side (no close rules printed). Karnish's suggested 13-day average trend filter is advice, not coded; he says the 'skeleton system' will not trade distant history well.", common!());
const BIN: &str = concat!("[Declared readings] The binary wave from the list reply: +1 after RSI(14) < 30 or Mo(12) < 90, -1 after RSI(14) > 70 or Mo(12) > 110, otherwise it keeps its value (the nested If favours the buy when both fire); the position follows the wave (stop-and-reverse).", common!());

pub fn catalogue() -> Vec<StrategyDef> {
    let d = |id, name, dir, rules, notes| StrategyDef { id, name, group: "Metastock Zone systems", session: 0, dir, status: Status::Live,
        needs_volume: false, needs_vix: false, needs_series: &[], book: "kzmzn", rules, notes, claim: None };
    vec![
        d("kzmzn_key_rev", "Key Reversals (Scorpio): lower low, higher close, close below the high; out when the high reaches yesterday's, 2% max loss", "Long", &[
            "E|\"Enter Long: L < Ref(L,-1) AND C>Ref(C,-1) AND C < H\"", "X|\"Close Long: H >=Ref(H,-1)\"; \"STOPS Maximum Loss: LONG ONLY 2.0 Percent\""], KEY),
        d("kzmzn_stochrsi", "StochRSI system: StochRSI-14 crosses up through 20 with StochRSI-70/252 strength; example exit close above the close 3 days back", "Long", &[
            "F|\"StochRSI-14 (Sum(RSI(14)-LLV(RSI(14),14),3) / Sum(HHV(RSI(14),14) - LLV(RSI(14),14),3))*100\"; likewise 70/15 and 252/54",
            "E|\"BUY Rule #1: EITHER STOCHRSI-70 OR STOCHRSI-252 >= 70.0 AND STOCHRSI-14 CROSSES FROM BELOW 20.0 TO EQUAL OR ABOVE 20.0\"; Rule #2: STOCHRSI-70 >= 50 and has risen by at least 3 points ... during any one of the past 21 trading days",
            "X|\"Sell Rule: Whatever you decide upon. CLOSE > Ref(CLOSE,-3)\""], SRSI),
        d("kzmzn_storsi", "StoRSI (Karnish): long when StoRSI falls below 17, short when it rises above 83, always in", "Long/Short", &[
            "E|\"Enter Long: Cross(17,(Mov((RSI(8)-LLV(RSI(8),8)) / (HHV(RSI(8),8)- (LLV(RSI(8),8))),3,E)*100))\"",
            "X|\"Enter Short: Cross((Mov((RSI(8)-LLV(RSI(8),8)) / (HHV(RSI(8),8)- (LLV(RSI(8),8))),3,E)*100),83)\""], STO),
        d("kzmzn_binary_wave", "Onno's Binary Wave: long after RSI(14) < 30 or Mo(12) < 90, short after RSI(14) > 70 or Mo(12) > 110", "Long/Short", &[
            "E|\"ENTER LONG: RSI(14)<30 OR Mo(12)<90\"", "X|\"ENTER SHORT: RSI(14)>70 OR Mo(12)>110\"; \"ValueWhen(1,signal<>0,signal)\""], BIN),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    match id {
        "kzmzn_key_rev" => Some(("kzmzn_key_rev", "Metastock Zone key reversals", "2% max loss")),
        "kzmzn_stochrsi" => Some(("kzmzn_stochrsi", "Metastock Zone StochRSI", "14/70/252")),
        "kzmzn_storsi" => Some(("kzmzn_storsi", "Metastock Zone StoRSI", "17/83")),
        "kzmzn_binary_wave" => Some(("kzmzn_binary_wave", "Metastock Zone binary wave", "RSI 14, Mo 12")),
        _ => None,
    }
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    match id {
        "kzmzn_key_rev" => Some(vec![p("stop", "Maximum loss, % (2)", 2., 0.5, 10., 0.5)]),
        "kzmzn_storsi" => Some(vec![p("lo", "Buy below (17)", 17., 1., 50., 1.), p("hi", "Sell above (83)", 83., 50., 99., 1.)]),
        "kzmzn_stochrsi" | "kzmzn_binary_wave" => Some(vec![]),
        _ => None,
    }
}

fn sum_n(v: &[f64], n: usize) -> Vec<f64> {
    (0..v.len()).map(|i| if i + 1 >= n && v[i + 1 - n..=i].iter().all(|x| x.is_finite()) { v[i + 1 - n..=i].iter().sum() } else { f64::NAN }).collect()
}

fn stoch_rsi(c: &[f64], n: usize, s: usize) -> Vec<f64> {
    let r = rsi(c, n);
    let (hh, ll) = (hhv(&r, n), llv(&r, n));
    let num: Vec<f64> = (0..c.len()).map(|i| r[i] - ll[i]).collect();
    let den: Vec<f64> = (0..c.len()).map(|i| hh[i] - ll[i]).collect();
    let (sn, sd) = (sum_n(&num, s), sum_n(&den, s));
    sn.iter().zip(&sd).map(|(a, b)| if a.is_finite() && b.is_finite() && *b != 0. { 100. * a / b } else { f64::NAN }).collect()
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.r.len() { return None; }
    let schema = params(id)?;
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).map_or(f64::NAN, |d| d.def));
    let (o, h, l, c, n_) = (&x.o, &x.h, &x.l, &x.c, x.c.len());
    let mut w = vec![0i8; n_];
    match id {
        "kzmzn_storsi" => {
            let r = rsi(c, 8);
            let (hh, ll) = (hhv(&r, 8), llv(&r, 8));
            let raw: Vec<f64> = (0..n_).map(|i| if hh[i].is_finite() && hh[i] != ll[i] { (r[i] - ll[i]) / (hh[i] - ll[i]) } else { f64::NAN }).collect();
            let k: Vec<f64> = ema(&raw, 3).iter().map(|v| v * 100.).collect();
            let (lo, hi) = (g("lo"), g("hi"));
            for i in 1..n_ { w[i] = if k[i] < lo && k[i - 1] >= lo { 1 } else if k[i] > hi && k[i - 1] <= hi { -1 } else { w[i - 1] }; }
            return Some(target_trades(x, &w, from, to, false, "StoRSI crossed"));
        }
        "kzmzn_binary_wave" => {
            let r = rsi(c, 14);
            for i in 1..n_ {
                let mo = if i >= 12 { 100. * c[i] / c[i - 12] } else { f64::NAN };
                w[i] = if r[i] < 30. || mo < 90. { 1 } else if r[i] > 70. || mo > 110. { -1 } else { w[i - 1] };
            }
            return Some(target_trades(x, &w, from, to, false, "binary wave flipped"));
        }
        _ => {}
    }
    let (s14, s70, s252) = if id == "kzmzn_stochrsi" { (stoch_rsi(c, 14, 3), stoch_rsi(c, 70, 15), stoch_rsi(c, 252, 54)) } else { (vec![], vec![], vec![]) };
    let mx = if id == "kzmzn_stochrsi" { let rise: Vec<f64> = (0..n_).map(|i| if i > 0 { s70[i] - s70[i - 1] } else { f64::NAN }).collect(); hhv(&rise, 21) } else { vec![] };
    let (mut out, mut pos) = (vec![], None::<(usize, f64)>);
    for i in from..=to {
        if let Some((e, ep)) = pos {
            if i > e {
                if id == "kzmzn_key_rev" {
                    let st = ep * (1. - g("stop") / 100.);
                    if l[i] <= st { out.push(mk(1., e, ep, "close", i, o[i].min(st), "maximum loss", false, i)); pos = None; continue; }
                    if h[i] >= h[i - 1] { out.push(mk(1., e, ep, "close", i, c[i], "high at or above yesterday's", false, i)); pos = None; continue; }
                } else if i >= 3 && c[i] > c[i - 3] { out.push(mk(1., e, ep, "close", i, c[i], "close above the close 3 days back", false, i)); pos = None; continue; }
            }
        }
        if pos.is_none() && i < to && i >= 1 {
            let entry = if id == "kzmzn_key_rev" { l[i] < l[i - 1] && c[i] > c[i - 1] && c[i] < h[i] }
                else { s14[i] >= 20. && s14[i - 1] < 20. && (s70[i] >= 70. || s252[i] >= 70. || (s70[i] >= 50. && mx[i] > 3.)) };
            if entry { pos = Some((i, c[i])); }
        }
    }
    if let Some((e, ep)) = pos { out.push(mk(1., e, ep, "close", to, c[to], "open", true, to)); }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;
    #[test]
    fn key_reversal_entry_and_exit() {
        // bar 1: lower low, higher close, close below high -> buy 10.5; bar 2: high 10.8 < 11 -> hold; bar 3: high 11.2 >= 10.8 -> out at 11
        let b = [(10., 11., 9.5, 10.), (10., 10.9, 9., 10.5), (10.5, 10.8, 10.35, 10.6), (10.6, 11.2, 10.4, 11.)];
        let r: Vec<Row> = b.iter().enumerate().map(|(i, &(o, h, l, c))| Row { ticker: "T".into(), date: format!("2001-01-{:02}", i + 1), open: o, high: h, low: l, close: c, volume: None }).collect();
        let x = Ctx::new(&r, &r, &[]);
        let t = run("kzmzn_key_rev", &x, 0, 3, &Params::new()).unwrap();
        assert_eq!(t.iter().map(|z| (z.legs[0].i, z.legs[0].px, z.exit_i, z.exit_px)).collect::<Vec<_>>(), vec![(1, 10.5, 3, 11.)]);
    }
}

// Instrument admission: Metastock Zone systems (K.A.L.O.S.Z archive folder) 2026-11-01 (titles lane K8).
// Admission only for the module REVIEWED lists (authors/titles-1030/K8w/k8_ref.py (header)).
// Runs on actual indices, ETFs and stocks, US and India (index_etf_or_stock, nifty50 basket, declared application).
pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book == "kzmzn" && crate::books::kzmzn::reviewed(def.id),
    label: "MetaStock list and Traders' Tips systems with no printed results; ETFs, US/India indices and stocks are a declared application.",
    basis: "Metastock Zone.pdf (K.A.L.O.S.Z archive folder) PDF p.665-666, 772-774, 1101-1105; independent check authors/titles-1030/K8w/k8_ref.py",
};
