//! Shiraz A. Lakhi, "Champion Trader" (2001, scanned e-book; book id `slct`; printed page = PDF page). Reading:
//! claude-continuation-20261020/authors/lakhi (admission/ct_ref.py). Live: the CT-formula (Section 3 p.73-102, exits
//! Section 4 p.105-107): trend = 14-day average of (high + low) / 2, up when today's average is above yesterday's; in an
//! uptrend a fresh low pivot sets point A (middle-bar low, the abort line) and point B (third-bar high, the up-signal
//! line); buy when B is hit before A, stop at A, sell when the average turns down. Downtrend: the mirror with high pivots.
//! Not coded: the options / spread-bet vehicles and money management (Section 2, p.108-112).
//! Book: FTSE 100 index; indices, ETFs and US/India stocks here are a declared application.
use crate::connors::{sma, Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};
/// Admitted ids (authors/lakhi/admission/ct_ref.py (header)).
pub const REVIEWED: &[&str] = &["slct_ct_formula"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const NOTE: &str = "[Declared readings] Average = simple 14-day average of (high + low) / 2, not rounded (the book rounds FTSE levels to whole points for hand work, p.75). Up = today's average above yesterday's, down = below; equal is neither. A pivot is fresh when its third bar is today and no set-up is waiting and no trade is open; it needs the average up (low pivot) or down (high pivot) on that day. While a set-up waits, later pivots are ignored (p.84, p.88). Up-signal: a buy stop at B, filled at B or at the open when the day opens above it, taken only if the average rose on the last close before the fill (\"CHECK TO MAKE SURE THE MA IS IN AN UPTREND FIRST\", p.87); otherwise the set-up is dropped. A day that opens at or below A, or touches both A and B without opening beyond either (order unknown on daily bars), drops the set-up with no trade. In a trade: a stop at A from the day after entry, filled at A or a worse open (p.105); the average turning down at a close sells at the next open (p.107). Short side: the mirror. One position at a time. A signal on the last bar is not filled; a position still open on the last bar is marked at its close.";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef { id: "slct_ct_formula", name: "Champion Trader CT-formula: pivot A/B lines in the 14-day average's direction", group: "Lakhi Champion Trader",
        session: 0, dir: "Long/Short", status: Status::Live, needs_volume: false, needs_vix: false, needs_series: &[], book: "slct",
        rules: &["F|\"add the two numbers and divide by 2\"; \"take the last 14 results, and divide the sum by 14\"; \"If today's MA is 'higher' than yesterday's, we define this ... as an 'UPTREND'. If today's MA is 'lower' than yesterday's ... a 'DOWNTREND'\" (p.74-78)",
            "F|\"A 'Low-pivot' occurs simply when a single bar has a 'low', which is below the two bars, which surround it\"; \"POINT A = THE LOWEST POINT IN THE MIDDLE BAR. POINT B = THE HIGHEST POINT IN THE THIRD BAR.\" (p.79-80)",
            "E|\"If the FTSE index hits the USL first then you enter the trade\" (CHECK TO MAKE SURE THE MA IS IN AN UPTREND FIRST, p.87); \"If the FTSE index hits the Abort-line first then you would 'discard' that opportunity\" (p.84, 88)",
            "X|\"if you have entered the market at USL or DSL, then should the market start moving against you, you should exit if it hits the 'Abort-Line', which therefore acts as an immediate 'stop-loss'\" (p.105)",
            "X|\"As long as the MA keeps moving up, I would hold on to the trade\"; short: \"until such time as the MA actually changes direction from down to up. At this point, you would exit\" (p.107)",
            "E|Down signals: high pivot, A = highest point of the middle bar, B = lowest point of the third bar, only when the MA is in a downtrend (p.91-101)"],
        notes: NOTE, claim: None }]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    match id { "slct_ct_formula" => Some(("slct_ct_formula", "Lakhi Champion Trader CT-formula", "14-day average")), _ => None }
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    match id {
        "slct_ct_formula" => Some(vec![p("side", "Sides: 0 = up and down signals (book), 1 = up only, 2 = down only", 0., 0., 2., 1.),
            p("ma", "Trend average days (book 14)", 14., 2., 100., 1.)]),
        _ => None,
    }
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.r.len() { return None; }
    let schema = params(id)?;
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).map_or(f64::NAN, |d| d.def));
    let side = g("side").round() as i64;
    let (o, h, l, c) = (&x.o, &x.h, &x.l, &x.c);
    let mp: Vec<f64> = h.iter().zip(l).map(|(a, b)| (a + b) / 2.).collect();
    let ma = sma(&mp, g("ma").round().max(1.) as usize);
    let trend = |t: usize| if t == 0 { 0. } else if ma[t] > ma[t - 1] { 1. } else if ma[t] < ma[t - 1] { -1. } else { 0. };
    let mk = |d: f64, ei: usize, ep: f64, kind: &'static str, xi: usize, xp: f64, why: &str, si: usize, open: bool| Trade { dir: d,
        legs: vec![Leg { i: ei, px: ep, w: 1. }], exit_i: xi, exit_px: xp, reason: why.into(), entry_kind: kind, signal_i: si, open, exits: vec![] };
    let (mut out, mut setup) = (vec![], None::<(f64, f64, f64)>); // (dir, A, B)
    let mut pos: Option<(f64, usize, f64, &'static str, f64, bool)> = None; // (dir, entry bar, fill, kind, A, sell at this open)
    for t in from.max(2)..=to {
        // intraday: a pending exit at the open, the abort-line stop, or the set-up's lines
        if let Some((d, ei, ep, k, a, sell)) = pos {
            if sell { out.push(mk(d, ei, ep, k, t, o[t], "average changed direction", t - 1, false)); pos = None; }
            else if t > ei && d * (a - if d > 0. { l[t] } else { h[t] }) >= 0. {
                out.push(mk(d, ei, ep, k, t, if d > 0. { o[t].min(a) } else { o[t].max(a) }, "abort line (stop)", t, false)); pos = None;
            }
        } else if let Some((d, a, b)) = setup {
            let (ob, oa) = (d * (o[t] - b) >= 0., d * (a - o[t]) >= 0.);
            let hb = d * ((if d > 0. { h[t] } else { l[t] }) - b) >= 0.;
            let ha = d * (a - if d > 0. { l[t] } else { h[t] }) >= 0.;
            if ob || (hb && !ha && !oa) {
                if trend(t - 1) == d && t < to { pos = Some((d, t, if ob { o[t] } else { b }, if ob { "open" } else { "stop" }, a, false)); }
                setup = None;
            } else if oa || ha { setup = None; }
        }
        // at the close: the trend exit, or a fresh pivot
        if let Some((d, ei, ep, k, a, _)) = pos {
            if trend(t) == -d && t < to { pos = Some((d, ei, ep, k, a, true)); }
        } else if setup.is_none() && t < to {
            let d = trend(t);
            if d > 0. && side != 2 && l[t - 1] < l[t - 2] && l[t - 1] < l[t] { setup = Some((1., l[t - 1], h[t])); }
            if d < 0. && side != 1 && h[t - 1] > h[t - 2] && h[t - 1] > h[t] { setup = Some((-1., h[t - 1], l[t])); }
        }
    }
    if let Some((d, ei, ep, k, _, _)) = pos { out.push(mk(d, ei, ep, k, to, c[to], "open", to, true)); }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;
    fn bar(i: usize, o: f64, h: f64, l: f64, c: f64) -> Row {
        Row { ticker: "T".into(), date: format!("2020-03-{:02}", i + 1), open: o, high: h, low: l, close: c, volume: None }
    }
    #[test]
    fn hand_checked_ct_trades() {
        // 16 rising bars (average rising), a low pivot at bars 15-17 (A = 112, B = 117), bar 18 trades through B -> long at 117;
        // bar 21 falls through A.
        let mut r: Vec<Row> = (0..16).map(|i| { let m = 100. + i as f64; bar(i, m, m + 2., m - 2., m) }).collect();
        r.push(bar(16, 113., 115., 112., 113.)); // middle bar: low 112 below bar 15's 113 = A
        r.push(bar(17, 114., 117., 113., 116.)); // third bar: low 113 > 112, high 117 = B
        r.push(bar(18, 116., 119., 115., 118.)); // high 119 >= 117: filled at 117
        r.push(bar(19, 118., 119., 117.5, 118.));
        r.push(bar(20, 117., 118., 113.5, 114.));
        r.push(bar(21, 110., 111., 90., 95.));
        r.push(bar(22, 96., 97., 94., 95.));
        r.push(bar(23, 95., 96., 93., 94.));
        let x = Ctx::new(&r, &r, &[]);
        let t = run("slct_ct_formula", &x, 0, r.len() - 1, &Params::new()).unwrap();
        assert_eq!((t[0].dir, t[0].legs[0].i, t[0].legs[0].px, t[0].entry_kind), (1., 18, 117., "stop"));
        // bar 21 low 90 < A = 112: the abort-line stop fills at min(open 110, 112) = 110
        assert_eq!((t[0].exit_i, t[0].exit_px, t[0].reason.as_str()), (21, 110., "abort line (stop)"));
        assert!(run("slct_x", &x, 0, r.len() - 1, &Params::new()).is_none());
    }
}
