//! Mark Deaton, "Swing Trading Money Machine" (Renegade Trader, 2007 course manual; book id `mdstm`; PDF pages).
//! Reading: claude-continuation-20261020/authors/deaton (admission/stmm_ref.py). Live: the signal and trigger swings
//! (PDF 6-9, 22): a low, a lower low and a higher low make a swing, completed when a later bar trades above the first
//! bar's high; a trigger is the next completed swing whose lower low is higher than the signal's; buy as price exceeds
//! the trigger's first-bar high, stop under its lower low, out on the 3-5 bar count (PDF 3, 14-15; 161.8% / 261.8%
//! Fibonacci targets as options, PDF 18-19). Short: the mirror (PDF 11, 22). Not coded: the trailing stop to 'swing
//! retracement levels' with exits only when 'severely violated' (judgement, PDF 9), the Elliott wave count, intraday
//! fine-tuning. Book: QQQ, XLE, XLF, SPY, OEX daily and intraday; indices, ETFs and US/India stocks here are a declared
//! application.
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};
use super::vrsdd::manage;
/// Admitted ids (authors/deaton/admission/stmm_ref.py (header)).
pub const REVIEWED: &[&str] = &["mdstm_signal_trigger"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const NOTE: &str = "[Declared readings] Daily bars. A swing low is a bar whose low is below the low of the bar before it (the 'low, then a lower low') followed by a bar with a higher low; the bar before the lower low is the swing's first bar. The swing is completed by the first bar that trades above the first bar's high, which may be the higher-low bar itself ('it happened on the 3rd candlestick', PDF 8); a low below the swing's lower low before that cancels it. Only the most recent swing is followed ('make sure your using the most recent low, and lower low', PDF 8): a newer swing low replaces one not yet completed. Each completed swing is compared with the previous completed swing: when its lower low is higher, it is a trigger (a trigger can serve as the next signal, PDF 15). Entry: on the bar that completes the trigger, at the trigger's first-bar high, or at the open when it opens above. Stop: the trigger's lower low ('just below' read as at it). Exit (default): at the close of the 5th bar after the entry bar, the end of the book's '3-5 candlestick count' (option 3 or 4; the book picks the bar by judgement). Option: a target at 161.8% or 261.8% of the distance from the signal's lower low to the trigger's middle-bar high, added to that high (PDF 19). Stops and targets are checked from the bar after entry, stop first, each filled at its level or a gapped open. Short: swing highs, the mirror (PDF 11, 22). One position at a time; swings keep being tracked during a trade but triggers then are not traded; if a long and a short trigger complete on the same bar, the order is unknown and neither is taken.";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef { id: "mdstm_signal_trigger", name: "Signal and trigger swings: buy above the trigger's first high, stop under its lower low, out after 5 bars",
        group: "Deaton Swing Trading Money Machine", session: 0, dir: "Long/Short", status: Status::Live, needs_volume: false, needs_vix: false, needs_series: &[], book: "mdstm",
        rules: &["F|\"The signal comes when we get a low, then a lower low, then a higher low. The signal is completed when the following candlestick exceeds the high from the first candlestick in the signal.\" (PDF 8)",
            "E|\"The trigger is just like the signal swing except that the LOWER LOW in the pattern is higher than the lower low from the signal ... Once that happens you enter as soon as the following candlesticks exceed the high from the first low.\" (PDF 8)",
            "X|\"Our initial stop is set just below the lower low of the trigger signal\" (PDF 9); short: \"Our stop is just above the middle trigger candlestick\" (PDF 22)",
            "X|\"one of my simplest methods for determining an exit is to do a 3-5 candlestick count from your entry and look to exit\" (PDF 3); \"one of the simplest forms of exit is a 3-5 bar exit. If you are trading a daily chart this would be a 3-5 day exit\" (PDF 14)",
            "X|Option: \"getting the difference between the signal patterns low and the trigger patterns high and multiplying that by 1.618 then adding the result to the high. Same with 2.618\" (PDF 19)"],
        notes: NOTE, claim: None }]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    (id == "mdstm_signal_trigger").then_some(("mdstm_signal_trigger", "Deaton signal and trigger swings", "5-bar exit"))
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    (id == "mdstm_signal_trigger").then(|| vec![p("side", "Sides: 0 = long and short (book), 1 = long only, 2 = short only", 0., 0., 2., 1.),
        p("bars", "Exit at the close of bar N after entry (book 3-5)", 5., 3., 5., 1.),
        p("fib", "Fibonacci target: 0 = none, 1 = 161.8%, 2 = 261.8% (PDF 19)", 0., 0., 2., 1.)])
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.r.len() { return None; }
    let schema = params(id)?;
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).map_or(f64::NAN, |d| d.def));
    let (side, n) = (g("side").round() as i64, g("bars").round().max(1.) as usize);
    let fib = match g("fib").round() as i64 { 1 => 1.618, 2 => 2.618, _ => 0. };
    let (o, h, l) = (&x.o, &x.h, &x.l);
    // pending swing (middle bar, completion level); previous completed swing's extreme
    let (mut pl, mut ps, mut prev_l, mut prev_h) = (None::<(usize, f64)>, None::<(usize, f64)>, None::<f64>, None::<f64>);
    let (mut out, mut busy) = (vec![], None::<usize>);
    for k in 2..=to {
        let (mut tl, mut ts) = (None, None);
        if l[k - 1] < l[k - 2] && l[k] > l[k - 1] { pl = Some((k - 1, h[k - 2])); }
        if let Some((b, lv)) = pl {
            if l[k] < l[b] { pl = None; }
            else if h[k] > lv {
                pl = None;
                if let Some(s) = prev_l.filter(|s| l[b] > *s) { tl = Some((b, lv, s)); }
                prev_l = Some(l[b]);
            }
        }
        if h[k - 1] > h[k - 2] && h[k] < h[k - 1] { ps = Some((k - 1, l[k - 2])); }
        if let Some((b, lv)) = ps {
            if h[k] > h[b] { ps = None; }
            else if l[k] < lv {
                ps = None;
                if let Some(s) = prev_h.filter(|s| h[b] < *s) { ts = Some((b, lv, s)); }
                prev_h = Some(h[b]);
            }
        }
        if k < from || busy.is_some_and(|z| k <= z) || (tl.is_some() && ts.is_some()) { continue; }
        let (d, (b, lv, s)) = match (tl, ts) { (Some(z), None) if side != 2 => (1., z), (None, Some(z)) if side != 1 => (-1., z), _ => continue };
        let (ep, kind) = if d * (o[k] - lv) > 0. { (o[k], "open") } else { (lv, "stop") };
        let (stop, mid) = if d > 0. { (l[b], h[b]) } else { (h[b], l[b]) };
        let tgt = if fib > 0. { mid + d * fib * (d * (mid - s)) } else { f64::NAN };
        let tr = manage(x, d, k, ep, kind, stop, &[(1., tgt)], false, k + n, to);
        busy = Some(if tr.open { to } else { tr.exit_i });
        out.push(tr);
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;
    fn bar(i: usize, o: f64, h: f64, l: f64, c: f64) -> Row {
        Row { ticker: "T".into(), date: format!("2020-05-{:02}", i + 1), open: o, high: h, low: l, close: c, volume: None }
    }
    #[test]
    fn hand_checked_signal_trigger_trade() {
        let r = vec![bar(0, 10., 10.5, 9.8, 10.), bar(1, 10., 10.2, 9.5, 9.6), // first bar 1 (high 10.2), lower low bar 2
            bar(2, 9.6, 9.9, 9.2, 9.8), bar(3, 9.8, 10.0, 9.4, 9.9), // bar 3 higher low: swing low 9.2 (first bar = bar 1, high 10.2)
            bar(4, 9.9, 10.3, 9.7, 10.2), // completes the signal (10.3 > 10.2), signal low 9.2
            bar(5, 10.2, 10.25, 9.6, 9.7), bar(6, 9.7, 9.9, 9.5, 9.8), // lower low 9.5 at bar 6 (first bar 5, high 10.25)
            bar(7, 9.8, 10.1, 9.6, 10.), // higher low: trigger swing low 9.5 > 9.2
            bar(8, 10., 10.4, 9.9, 10.3), // completes the trigger: long at 10.25, stop 9.5
            bar(9, 10.3, 10.5, 10.1, 10.4), bar(10, 10.4, 10.6, 10.2, 10.5), bar(11, 10.5, 10.7, 10.3, 10.6),
            bar(12, 10.6, 10.8, 10.4, 10.7), bar(13, 10.7, 10.9, 10.5, 10.8), // 5th bar after entry: out at 10.8
            bar(14, 10.8, 10.9, 10.6, 10.7)];
        let x = Ctx::new(&r, &r, &[]);
        let t = run("mdstm_signal_trigger", &x, 0, r.len() - 1, &Params::new()).unwrap();
        let got: Vec<_> = t.iter().map(|z| (z.dir, z.legs[0].i, (z.legs[0].px * 100.).round(), z.entry_kind, z.exit_i, (z.exit_px * 100.).round(), z.reason.clone())).collect();
        assert_eq!(got, vec![(1., 8, 1025., "stop", 13, 1080., "time exit".to_string())]);
        // 161.8%: 9.9 (trigger middle-bar high) + 1.618 x (9.9 - 9.2) = 11.03 is not reached
        let mut f = Params::new(); f.insert("fib".into(), 1.);
        assert_eq!(run("mdstm_signal_trigger", &x, 0, r.len() - 1, &f).unwrap()[0].exit_i, 13);
        assert!(run("mdstm_x", &x, 0, r.len() - 1, &Params::new()).is_none());
    }
}
