//! Omega Research TradeStation EasyLanguage course, Russian translation (author field TELETRADE; research share USB HDD 1/
//! TradeStation/Omega Research Prosuite 2000i/Utilites/ProSuite doc/EasyLanguage.doc; soffice text authors/batch-e/E7w/soff/
//! EasyLanguage.txt, systems extract authors/batch-e/E7w/teletrade_systems_ru.txt). Book id `ttelru`. Three printed systems:
//! practice #10 'KeyReversal System' (with the Home Study Course #28 limit version as an option), 'XBarHighLow' and 'Momentum
//! System', coded as labelled teaching examples (R4 decisions 49, 50, 57). Lane K11 (titles loop, 1 Nov 2026), to-code rows
//! 187-189; independent check authors/titles-1030/K11w/k11_ref.py.
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};
use super::lc3::{hi, lo};
use super::tselhs::{limit_fill, over, under, Book};

/// Admitted ids (authors/titles-1030/K11w/k11_ref.py (header)).
pub const REVIEWED: &[&str] = &["ttelru_keyrev", "ttelru_xbar_hl", "ttelru_momentum"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

macro_rules! common { () => { " In TradeStation 2000i 'Sell' opens a short position (the course defines SELL as entering a short), so an opposite entry reverses; a same-direction entry is ignored (no pyramiding). 'Next bar' orders last one bar: a market order fills at the open, a limit at the limit or at the open when the open is already better. 'Points' = minimum price ticks ('a point is the minimum price fluctuation', the course), 0.01 US and 0.05 NSE (lead decision 57). No bar size is given: daily bars are a declared application. A teaching example from a translated software course with no printed results (lead decision 49); the vault's standard costs (decision 50)." } }

const KR: &str = concat!("[Declared readings] Practice #10 'KeyReversal System' (answer): key reversal up = Low < Low[1] and Close > Close[1] -> Buy this bar on close; key reversal down = High > High[1] and Close < Close[1] -> Sell (short) this bar on close; 'the system does not define exits; the exit is the entry in the opposite direction'. Always in after the first signal. Option p.limit = 1: TradeStation's EasyLanguage Home Study Course Challenge #28 (answer p.211-212, a daily CSCO chart): the same patterns, Buy next bar at a limit = this bar's Close - EntryLimit and Sell Short next bar at a limit = Close + EntryLimit, EntryLimit '5 points' (.05 printed) = 5 ticks; each order lasts one bar. Not the MetaStock Zone key-reversal card (kzmzn_key_rev): that one is long only, adds close below the high, and exits on a high at yesterday's high or a 2% loss.", common!());
const XB: &str = concat!("[Declared readings] 'XBarHighLow' (code printed): Len1 = 8, StopLen = 6. High > Highest(High,8)[1] and Close > Close[1] -> Buy next bar at market; Low < Lowest(Low,8)[1] and Close < Close[1] -> Sell (short) next bar at market. ExitLong at Lowest(Low,6) stop and ExitShort at Highest(High,6) stop, both including the current bar (no [1] printed), placed at each close for the next bar while the position is held (first at the entry bar's close); a stop fills at the stop or at the open on a gap through it. A market order on the next bar's open is taken before that bar's stop.", common!());
const MO: &str = concat!("[Declared readings] 'Momentum System' (code printed): MomLen = 5, Xbars = 8; Value1 = Close - Close[5]. Value1 crosses over 0 -> Buy ('BullSig') next bar at Close - 2 points limit; crosses under 0 -> Sell ('BearSig', short) next bar at Low + 2 points limit (the short limit is printed from the Low, the long from the Close: kept as printed). If BarsSinceEntry = 8 (the entry bar is 0) -> ExitLong ('LiqLong') / ExitShort ('CoverShort') this bar on close. p.tick = 0 removes the 2-tick offsets.", common!());

pub fn catalogue() -> Vec<StrategyDef> {
    let d = |id, name, rules, notes| StrategyDef { id, name, group: "Omega EasyLanguage course (Russian translation) systems", session: 0, dir: "Long/Short", status: Status::Live,
        needs_volume: false, needs_vix: false, needs_series: &[], book: "ttelru", rules, notes, claim: None };
    vec![
        d("ttelru_keyrev", "KeyReversal System: lower low with a higher close buys, higher high with a lower close sells short, at the close", &[
            "E|\"Condition1 = Low < Low [1]; Condition2 = Close > Close [1]; If Condition1 and Condition2 Then Buy this bar on close;\" (practice #10)",
            "E|\"Condition3 = High > High [1]; Condition4 = Close < Close [1]; If Condition3 and Condition4 Then Sell this bar on close;\" (practice #10)",
            "X|The opposite entry is the exit (stop and reverse)."], KR),
        d("ttelru_xbar_hl", "XBarHighLow: 8-bar breakout on an up (down) close at the next open, out on a 6-bar low (high) stop", &[
            "E|\"Condition1 = High > Highest (High, Len1) [1]; Condition2 = Close > Close [1]; ... If Condition1 and Condition2 Then Buy next bar at market; If Condition3 and Condition4 Then Sell next bar at market;\" Input: Len1 (8)",
            "X|\"ExitLong at Lowest (Low, StopLen) Stop; ExitShort at Highest (High, StopLen) Stop;\" Input: StopLen (6)"], XB),
        d("ttelru_momentum", "Momentum System: 5-bar momentum crossing zero, limit entries 2 ticks better, out on the close of the 8th bar", &[
            "E|\"Value1 = Close - Close [MomLen]; If Value1 crosses over 0 Then Buy (\\\"BullSig\\\") Next Bar at Close - 2 points Limit; If Value1 crosses under 0 Then Sell (\\\"BearSig\\\") Next Bar at Low + 2 points Limit;\" Input: MomLen (5)",
            "X|\"If BarsSinceEntry = Xbars Then Begin ExitLong (\\\"LiqLong\\\") This Bar on Close; ExitShort (\\\"CoverShort\\\") This Bar on Close; End;\" Input: Xbars (8)"], MO),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    let f = match id {
        "ttelru_keyrev" => ("Omega course (RU): KeyReversal", "stop and reverse at the close"),
        "ttelru_xbar_hl" => ("Omega course (RU): XBarHighLow", "8-bar breakout, 6-bar stop"),
        "ttelru_momentum" => ("Omega course (RU): Momentum System", "momentum 5, 8-bar exit"),
        _ => return None,
    };
    let id = REVIEWED.iter().find(|r| **r == id)?;
    Some((*id, f.0, f.1))
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    Some(match id {
        "ttelru_keyrev" => vec![p("limit", "Entries: 0 = this bar on close (default), 1 = Home Study #28 next-bar limit 5 ticks better", 0., 0., 1., 1.)],
        "ttelru_xbar_hl" => vec![],
        "ttelru_momentum" => vec![p("tick", "Limit offset in ticks (2; 0 = none)", 2., 0., 10., 1.)],
        _ => return None,
    })
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.r.len() { return None; }
    let schema = params(id)?;
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).map_or(f64::NAN, |d| d.def));
    let (o, h, l, c, n) = (&x.o, &x.h, &x.l, &x.c, x.c.len());
    let tick = x.market_tick();
    let mut pos = Book::new();
    match id {
        "ttelru_keyrev" => {
            let lim = g("limit") > 0.5;
            let mut pend: Option<(f64, f64)> = None;
            for i in from..=to {
                if let Some((d, px)) = pend.take() {
                    if pos.side() != d { if let Some(f) = limit_fill(x, i, d, px) { pos.enter(d, i, f, "limit"); } }
                }
                if i == 0 { continue; }
                let sig = if l[i] < l[i - 1] && c[i] > c[i - 1] { 1. } else if h[i] > h[i - 1] && c[i] < c[i - 1] { -1. } else { 0. };
                if sig == 0. { continue; }
                if !lim { pos.enter(sig, i, c[i], "close"); } else if i < to { pend = Some((sig, c[i] - sig * 5. * tick)); }
            }
        }
        "ttelru_xbar_hl" => {
            let (mut mkt, mut stop): (Option<f64>, f64) = (None, f64::NAN);
            for i in from..=to {
                if let Some(d) = mkt.take() { pos.enter(d, i, o[i], "open"); }
                else if stop.is_finite() {
                    let d = pos.side();
                    if d > 0. && l[i] <= stop { pos.exit(i, o[i].min(stop), "6-bar stop"); }
                    else if d < 0. && h[i] >= stop { pos.exit(i, o[i].max(stop), "6-bar stop"); }
                }
                stop = f64::NAN;
                if i == to || i < 8 { continue; }
                let sig = if h[i] > hi(h, i - 8, i - 1) && c[i] > c[i - 1] { 1. } else if l[i] < lo(l, i - 8, i - 1) && c[i] < c[i - 1] { -1. } else { 0. };
                if sig != 0. && sig != pos.side() { mkt = Some(sig); }
                if i >= 5 { stop = if pos.side() > 0. { lo(l, i - 5, i) } else if pos.side() < 0. { hi(h, i - 5, i) } else { f64::NAN }; }
            }
        }
        "ttelru_momentum" => {
            let off = g("tick").round().max(0.) * tick;
            let v: Vec<f64> = (0..n).map(|i| if i >= 5 { c[i] - c[i - 5] } else { f64::NAN }).collect();
            let z = vec![0.; n];
            let mut pend: Option<(f64, f64)> = None;
            for i in from..=to {
                if let Some((d, px)) = pend.take() {
                    if pos.side() != d { if let Some(f) = limit_fill(x, i, d, px) { pos.enter(d, i, f, "limit"); } }
                }
                if pos.entry_bar() == Some(i.wrapping_sub(8)) && i >= 8 { pos.exit(i, c[i], "8 bars"); }
                if i == to { continue; }
                if over(&v, &z, i) { pend = Some((1., c[i] - off)); } else if under(&v, &z, i) { pend = Some((-1., l[i] + off)); }
            }
        }
        _ => return None,
    }
    Some(pos.done(to, c[to]))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;
    #[test]
    fn russian_course_systems() {
        let r: Vec<Row> = (0..900).map(|i| { let f = i as f64; let c = 100. + 9. * (f / 17.).sin() + 4. * (f / 5.3).cos() + 0.02 * f;
            Row { ticker: "T".into(), date: format!("{:05}", i), open: c - 0.6 * (f / 2.1).sin(), high: c + 1.3 + 0.4 * (f / 3.1).sin(), low: c - 1.1 - 0.7 * (f / 2.3).sin(), close: c, volume: None } }).collect();
        let x = Ctx::new(&r, &r, &[]);
        for id in REVIEWED {
            for prm in [Params::new(), Params::from([("limit".to_string(), 1.)])] {
                let t = run(id, &x, 0, r.len() - 1, &prm).unwrap();
                assert!(t.len() > 5 && t.iter().any(|z| z.dir > 0.) && t.iter().any(|z| z.dir < 0.), "{id}");
                for w in t.windows(2) { assert!(w[1].legs[0].i >= w[0].exit_i, "{id}"); }
            }
            assert!(family_of(id).is_some() && params(id).is_some());
        }
        // the momentum system's timed exit is the close of the 8th bar after the entry bar
        for z in run("ttelru_momentum", &x, 0, r.len() - 1, &Params::new()).unwrap() {
            if z.reason == "8 bars" { assert_eq!(z.exit_i - z.legs[0].i, 8); assert_eq!(z.exit_px, x.c[z.exit_i]); }
        }
        assert!(run("ttelru_x", &x, 0, 5, &Params::new()).is_none());
    }
}

// Instrument admission: Omega EasyLanguage course, Russian translation (TELETRADE) 2026-11-01 (titles lane K11).
pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book == "ttelru" && reviewed(def.id),
    label: "Teaching systems from a Russian translation of Omega Research's EasyLanguage course, with no instrument and no results. Runs on daily indices, ETFs and stocks, US and India, are a declared application.",
    basis: "EasyLanguage.doc (Omega ProSuite 2000i doc folder, TELETRADE), systems XBarHighLow, Momentum System and practice #10 KeyReversal; Home Study Course #28 (p.211-212) for the limit option; independent check authors/titles-1030/K11w/k11_ref.py",
};
