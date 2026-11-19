//! Lee E. Gettess, "Trading in the Trenches" (SS-TDA, Intraday Market Mapping, 1993-95) bound with the "Scorpion S&P"
//! methodology and "Lee Gettess on Day Trading" special report. Book id `lgsc`. Source copy:
//! Books/Lee E. Gettess/Gettes.pdf (54 scanned two-page spreads). Library: library/lgsc/.
//! Nothing here is testable on the book's own terms: the SS-TDA tests print results but the text never states the
//! exit of the base test; the rest is chart reading / intraday swing judgement or leaves the key number to the reader.
//! 2026-10-25 (authors/gettess): the SS-TDA entry is run with the user's standard exits as a labelled variant (lgsc_sstda_std).
use crate::connors::{atr, true_range, Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "lgsc";
/// Admitted ids (authors/gettess/admission/SOURCE-CONTRACT.md).
pub const REVIEWED: &[&str] = &["lgsc_sstda_std", "lgsc_sstda_fpo"]; // + lane R2D (R4 #297/#304)
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }
const STD_NOTE: &str = "[Not from the book] The book states only the SS-TDA entry (p.5), so the user's standard exits are used (labelled variant): exit 0 = at the close after `hold` bars (3 / 5 / 10, default 5), 1 = trailing stop 2 x Wilder ATR(5) of the previous bar behind the best price since entry, 2 = first up close (shorts: first down close). [Declared] Each day while flat both stops work: buy at open + 0.65 x the average true range of the two previous days, sell at open - the same (no rounding is stated for SS-TDA); a stop through the open fills at the open; if both lie inside the bar the path open -> nearer extreme -> other extreme decides (tie: low first); one position at a time, no reversal, exits from the bar after entry, no new entry on an exit bar; positions start inside the selected window. Book: S&P, T-Bond, D-Mark and crude futures; indices, ETFs and stocks, US and India, are a declared application.";

pub fn catalogue() -> Vec<StrategyDef> {
    use Status::*;
    let d = |id, name, group, status, rules, notes| StrategyDef { id, name, group, session: 0, dir: "Long/Short", status,
        needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK, rules, notes, claim: None };
    vec![
        d("lgsc_sstda", "SS-TDA volatility breakout (65% of 2-day average true range)", "Trading in the Trenches · SS-TDA (p.5-14)",
          NotTestable("no exit stated: the text gives only the entry (open +/- 65% of the 2-day average true range); the printed SystemWriter tests (ATRBO test 5) imply but never state stop-and-reverse - adding it would be our exit, not the book's"),
          &["E|\"a volatility breakout based on 65% of the two day true range added and subtracted from the opening price ... Take the true range for the last two days, add them together, divide by 2, then multiply by 0.65\" (p.5)"],
          "Printed results (ATRBO test 5, 7/1/83-6/30/93): S&P INDEX 708 trades 45%, net $165,700; T-Bonds 665 trades 43%; D-Mark 552 trades 43%; Crude 666 trades 40% (p.6-9)."),
        d("lgsc_sstda_std", "SS-TDA volatility breakout with standard exits", "Trading in the Trenches · SS-TDA (p.5) · standard exits", Live,
          &["E|\"a volatility breakout based on 65% of the two day true range added and subtracted from the opening price ... Take the true range for the last two days, add them together, divide by 2, then multiply by 0.65\" (p.5)",
            "X|[Not from the book] standard exits: close after N bars (3/5/10), 2 x ATR(5) trailing stop, or first up close (shorts: first down close)"],
          STD_NOTE),
        d("lgsc_sstda_fpo", "SS-TDA with first-profitable-open exit", "Trading in the Trenches · SS-TDA (p.10-14)", Live,
          &["E|Same SS-TDA entry (p.5): each day a buy stop at the open + 65% of the average true range of the two previous days, a sell stop at the open - the same",
            "X|\"a trade was taken off when the market opened profitably basis the position ... my 'first profitable open' exit\" (p.10)",
            "X|Unstated in the book: the loss exit; vault reading: the opposite SS-TDA stop reverses the position (the book's SystemWriter ATRBO tests are stop-and-reverse), each stop at most once a day; p.rev = 0 = no loss exit",
            "F|Adapted: 1 unstated detail (R4 #297, #304). Same-day order (both stops inside one day) on the real 1-minute path (EP6, p.order); only the minute window of SPY / SPX / NIFTY 50 / NIFTYBANK is published (#243 (c))"],
          "Printed results (ATRBO test 6): S&P INDEX 976 trades 73%, net $204,225 (p.13); T-Bonds 926 trades 74%; D-Mark 808 trades 71%; Crude 936 trades 68%. [Declared] After a first-profitable-open exit both stops work that same day; a stop the day's path passes fills at its level (on minutes: a minute that gaps past it fills at that minute's open). Book: S&P futures 7/1/83-6/30/93; run on SPY/SPX/NIFTY minute windows as a declared application."),
        d("lgsc_intraday_123", "Intraday Market Mapping (1-2-3 patterns, 3-bar filter)", "Trading in the Trenches · Intraday (p.15-30)",
          NotTestable("rules not fully defined: 1-2-3 swings 'of some degree' on 15- to 30-min charts, judged by eye by the trader (Trading in the Trenches / Scorpion S&P p.17); price bars at that size now exist, the judgement remains"),
          &["F|\"A 1-2-3 ... is formed when the market has made a low of some degree, swung up in the next few bars to make a high, then fallen back down to a low that is higher than the previous low\" (p.17)"], ""),
        d("lgsc_scorpion", "Scorpion S&P methodology (25/175 oscillator, Fibonacci retracements)", "Scorpion S&P (p.1-32 of part 2)",
          NotTestable("the author calls it 'a methodology, rather than a system': swing selection, divergences and stop placement are left to the trader"),
          &["F|\"we create a 25/175 period oscillator from exponential moving averages of the closes, then we also plot a 20 period moving average of the oscillator\" (p.4)",
            "F|\"I have deliberately not given hard and fast parameters for how much should be risked or how quickly to begin taking profits\" (p.29)"], ""),
        d("lgsc_daytrade_report", "Lee Gettess on Day Trading (special report)", "Day Trading report (p.1-12)",
          NotTestable("the displacement percentage is left to the reader ('Exactly what percentage to use is still a question') and no exit/stop is fixed"),
          &["E|Example: 55% of yesterday's range (87 points) added to / subtracted from the June 1997 S&P open (p.10-11)"], ""),
    ]
}
pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> { catalogue().into_iter().find(|d| d.id == id).map(|d| (d.id, d.name, "")) }
pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    if id == "lgsc_sstda_fpo" { // lane R2D (R4 #297/#304)
        let mut v = vec![p("pct", "Percent of the 2-day average true range (book 65)", 65., 10., 150., 5.), p("days", "Days of true range averaged (book 2)", 2., 1., 10., 1.),
            p("rev", "Loss exit [unstated, R4 #297 reading]: 1 = the opposite SS-TDA stop reverses, 0 = none (first profitable open only)", 1., 0., 1., 1.)];
        crate::intraday_ep6::add_param(id, &mut v);
        return Some(v);
    }
    if id != "lgsc_sstda_std" { return None; }
    Some(vec![p("pct", "Percent of the 2-day average true range (book 65)", 65., 10., 150., 5.), p("days", "Days of true range averaged (book 2)", 2., 1., 10., 1.),
        p("exit", "[Not from the book] exit: 0 = close after hold bars, 1 = 2 x ATR(5) trailing stop, 2 = first up close (shorts: first down close)", 0., 0., 2., 1.),
        p("hold", "[Not from the book] bars held for exit 0 (3 / 5 / 10)", 5., 3., 10., 1.),
        p("side", "Sides (0 both, 1 long only, -1 short only)", 0., -1., 1., 1.)])
}

fn tr_(d: f64, ei: usize, epx: f64, xi: usize, xpx: f64, why: &str, open: bool) -> Trade {
    Trade { dir: d, legs: vec![Leg { i: ei, px: epx, w: 1.0 }], exit_i: xi, exit_px: xpx, reason: why.into(), entry_kind: "stop", signal_i: xi, open, exits: vec![] }
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    let schema = params(id)?;
    if from > to || to >= x.c.len() { return None; }
    for k in p.keys() { if !schema.iter().any(|d| d.key == k) { return None; } }
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).unwrap().def);
    if id == "lgsc_sstda_fpo" {
        let Some((from, to)) = crate::intraday_ep6::window(id, x, from, to, p) else { return Some(vec![]) }; // #243 (c)
        return Some(fpo(x, from, to, g("pct") / 100., g("days").round().max(1.) as usize, g("rev") >= 0.5, crate::intraday_ep6::on(id, p)));
    }
    let (pct, n, mode, hold, side) = (g("pct") / 100., g("days").round().max(1.) as usize, g("exit").round() as i64, g("hold").round().max(1.) as usize, g("side").round());
    let (o, h, l, c) = (&x.o, &x.h, &x.l, &x.c);
    let tr = true_range(x.r);
    let a5 = atr(x.r, 5);
    let mut out = vec![];
    // position: dir, entry bar, entry px, best price, trailing level
    let mut pos: Option<(f64, usize, f64, f64, f64)> = None;
    for k in from..=to {
        if let Some((d, e, px, mut best, mut trail)) = pos {
            if k > e {
                if mode == 1 {
                    if k > e + 1 { best = if d > 0. { best.max(h[k - 1]) } else { best.min(l[k - 1]) }; }
                    let lv = best - d * 2. * a5[k - 1];
                    if lv.is_finite() { trail = if trail.is_finite() { if d > 0. { trail.max(lv) } else { trail.min(lv) } } else { lv }; }
                    if trail.is_finite() {
                        let gap = if d > 0. { o[k] <= trail } else { o[k] >= trail };
                        let hit = if d > 0. { l[k] <= trail } else { h[k] >= trail };
                        if gap || hit { out.push(tr_(d, e, px, k, if gap { o[k] } else { trail }, "2 x ATR(5) trailing stop", false)); pos = None; continue; }
                    }
                } else if mode == 0 && k >= e + hold {
                    out.push(tr_(d, e, px, k, c[k], &format!("{hold} bars held"), false)); pos = None; continue;
                } else if mode == 2 && (if d > 0. { c[k] > c[k - 1] } else { c[k] < c[k - 1] }) {
                    out.push(tr_(d, e, px, k, c[k], if d > 0. { "first up close" } else { "first down close" }, false)); pos = None; continue;
                }
            }
            pos = Some((d, e, px, best, trail));
            continue;
        }
        if k < n { continue; }
        let f = tr[k - n..k].iter().sum::<f64>() / n as f64 * pct;
        let (b, s) = (o[k] + f, o[k] - f);
        let bh = (side == 0. || side == 1.) && h[k] >= b;
        let sh = (side == 0. || side == -1.) && l[k] <= s;
        let d = if bh && sh { if o[k] - l[k] <= h[k] - o[k] { -1. } else { 1. } } else if bh { 1. } else if sh { -1. } else { 0. };
        if d == 0. { continue; }
        let px = if d > 0. { b.max(o[k]) } else { s.min(o[k]) };
        pos = Some((d, k, px, px, f64::NAN));
    }
    if let Some((d, e, px, _, _)) = pos { out.push(tr_(d, e, px, to, c[to], "open", true)); }
    Some(out)
}

// ---- lane R2D (13 Nov 2026, R4 #297/#304): SS-TDA with the first-profitable-open exit; the opposite stop reverses (reading).
#[derive(Clone)]
struct Fs { pos: Option<(f64, usize, f64, crate::intraday_ep6::Day)>, done: [bool; 2], lv: [f64; 2], k: usize, rev: bool, out: Vec<(Trade, crate::intraday_ep6::Day)> }

fn fpo(x: &Ctx, from: usize, to: usize, pct: f64, n: usize, rev: bool, on: bool) -> Vec<Trade> {
    use crate::intraday_ep6::{bar_segs, mark, resolve, ts_high_first, Day, Ev, Seg};
    let (o, h, l, c) = (&x.o, &x.h, &x.l, &x.c);
    let tr = true_range(x.r);
    let walk = |s: &mut Fs, segs: &[Seg]| -> Vec<Ev> {
        let mut ev = vec![];
        for g in segs {
            let (d, j) = if g.up() { (1.0, 0usize) } else { (-1.0, 1usize) };
            if g.a == g.b || s.done[j] || !g.inside(s.lv[j]) { continue; }
            if let Some(q) = s.pos { if q.0 == d || !s.rev { continue; } }
            let px = g.fill(s.lv[j]);
            if let Some(q) = s.pos.take() { s.out.push((tr_(q.0, q.1, q.2, s.k, px, "opposite SS-TDA stop (reverse)", false), q.3)); }
            s.pos = Some((d, s.k, px, Day::Plain));
            s.done[j] = true;
            ev.push(Ev { k: j as u8, px, up: d > 0.0 });
        }
        ev
    };
    let mut out = vec![];
    let mut st = Fs { pos: None, done: [false; 2], lv: [f64::NAN; 2], k: 0, rev, out: vec![] };
    for k in from.max(n)..=to {
        if let Some(q) = st.pos.filter(|q| q.1 < k && (o[k] - q.2) * q.0 > 0.0) {
            let mut t = tr_(q.0, q.1, q.2, k, o[k], "first profitable open", false);
            mark(&mut t, q.3);
            out.push(t);
            st.pos = None;
        }
        let f = tr[k - n..k].iter().sum::<f64>() / n as f64 * pct;
        st.lv = [o[k] + f, o[k] - f];
        st.done = [false; 2];
        st.k = k;
        let hf = ts_high_first(o[k], h[k], l[k]);
        let day = resolve(x, k, on, hf, &mut st, |s, hf| walk(s, &bar_segs(o[k], h[k], l[k], c[k], hf)), &walk);
        for (mut t, m) in st.out.drain(..) { mark(&mut t, m.worst(day)); out.push(t); }
        if let Some(q) = st.pos.as_mut() { if q.1 == k || day != Day::Plain { q.3 = q.3.worst(day); } }
    }
    if let Some(q) = st.pos { let mut t = tr_(q.0, q.1, q.2, to, c[to], "open", true); mark(&mut t, q.3); out.push(t); }
    out
}

#[cfg(test)]
mod r2d_tests {
    use super::*;
    use crate::Row;
    fn row(d: &str, o: f64, h: f64, l: f64, c: f64) -> Row { Row { ticker: "T".into(), date: d.into(), open: o, high: h, low: l, close: c, volume: None } }
    #[test]
    fn sstda_fpo_reverses_at_the_opposite_stop_and_exits_on_the_first_profitable_open() {
        let r = vec![row("2009-01-05", 100., 101., 99., 100.), row("2009-01-06", 100., 101., 99., 100.),
            row("2009-01-07", 100., 102., 99.5, 101.5), // buy stop 100 + 0.65 x 2 = 101.3 fills
            row("2009-01-08", 101., 101.2, 99., 99.5), // not profitable at the open; sell stop 101 - 0.65 x 2.25 reverses
            row("2009-01-09", 99., 99.1, 98.9, 99.)]; // short profitable at the open: out at 99
        let x = Ctx::new(&r, &r, &[]);
        let ts = run("lgsc_sstda_fpo", &x, 0, 4, &Params::new()).unwrap();
        let s = 101. - 0.65 * 2.25;
        assert_eq!(ts.len(), 2);
        assert!((ts[0].dir, ts[0].legs[0].i, ts[0].exit_i) == (1., 2, 3) && (ts[0].legs[0].px - 101.3).abs() < 1e-9 && (ts[0].exit_px - s).abs() < 1e-9);
        assert!((ts[1].dir, ts[1].legs[0].i, ts[1].exit_i, ts[1].exit_px) == (-1., 3, 4, 99.) && (ts[1].legs[0].px - s).abs() < 1e-9);
        assert!(reviewed("lgsc_sstda_fpo") && crate::intraday_ep6::WINDOW_ONLY.contains(&"lgsc_sstda_fpo") && crate::intraday_ep6::held_note("lgsc_sstda_fpo", "SPX").is_some());
    }
}
