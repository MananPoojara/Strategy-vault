//! James A. Hyerczyk, "Pattern, Price & Time: Using Gann Theory in Trading Systems" (2nd ed., 2009; book id `jhppt`; printed
//! page cited). Reading: claude-continuation-20261020/authors/hyerczyk (admission/swing_ref.py). Live: the Gann trend-indicator
//! (swing) chart traded with a stop and reverse (Ch.5 p.69-90, Ch.6 p.91-97): buy on a stop over the last swing top, sell and
//! reverse on a stop under the last swing bottom, the stop moved under each new swing bottom (over each new top when short).
//! Not coded: starting only from an "extreme level" of price and time (judgement, p.95), the later Gann angle / retracement
//! chapters. Book: futures, Forex and stocks; indices, ETFs and US/India stocks are a declared application.
use crate::connors::{Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};
/// Admitted ids (authors/hyerczyk/admission/swing_ref.py (header)).
pub const REVIEWED: &[&str] = &["jhppt_swing_reversal"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const NOTE: &str = "[Declared readings] Swing chart: a higher high = today's high above the last active bar's high, a lower low = today's low below its low; a bar with neither is ignored like an inside bar (p.85). The line turns up after N consecutive higher highs and down after N consecutive lower lows (N = 1 minor, 2 intermediate, 3 main; default 2, the chart the book suggests trading, p.77); once moving, each higher high (lower low) extends it. Top = the up line's highest point when it turns down; bottom = the down line's lowest point when it turns up; the first line direction at the start of the test is not a swing. The chart starts at the first bar of the test. Outside bars follow p.86-87: the 1-bar chart takes both moves in order; the 2- and 3-bar charts count an up move when the line is up, and when the line is down they follow the first move; the order of the high and low is unknown on daily bars, so 'the price closest to the opening occurred first' (p.87), then the close, an exact tie = high first. Before the line has a direction an outside bar counts in its first move's direction. Stops: 'one, two, or three price units' beyond the top or bottom (p.88): one price unit = 0.01 (book default 1 unit; 2 and 3 are options); a stop is hit on touch, filled at the level or at a worse open. Orders use the chart as of yesterday. One fill per day: a new position's stop works from the next day. Before the first trade, if both levels are crossed on one day with no gap, the level nearer the open fills. Not applied: contract sizing.";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef { id: "jhppt_swing_reversal", name: "Gann swing chart: stop and reverse at the last swing top / bottom", group: "Trend indicator (swing chart) trading",
        session: 0, dir: "Long/Short", status: Status::Live, needs_volume: false, needs_vix: false, needs_series: &[], book: "jhppt",
        rules: &["F|\"From a low price each time the market makes a higher-high than the previous bar for two consecutive time periods, an intermediate trend line moves up ... This action makes the low price from two bars back an intermediate bottom.\" (p.78) \"At this point it is no longer necessary to have two consecutive higher-highs to move the trend line up ... until the market posts two consecutive lower-lows.\" (p.92)",
            "F|\"The analyst ignores the inside bar and refers to the last active bar\" (p.86). Outside bar: \"it is safe to assume that the price closest to the opening occurred first and the price closest to the close occurred last.\" (p.87)",
            "E|\"The trader enters the market on the long side when the market crosses the last swing top. She continues to maintain her long position as long as the market continues to make higher tops and higher bottoms, with a protective and reversal stop under the last swing bottom.\" (p.94-95)",
            "X|\"The trade is liquidated, and the position reversed when the last swing bottom is crossed\" (p.97); \"Stops should be placed one, two, or three price units above or below swing tops and bottoms.\" (p.88)"],
        notes: NOTE, claim: None }]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    match id { "jhppt_swing_reversal" => Some(("jhppt_swing_reversal", "Hyerczyk Gann swing stop and reverse", "2-bar / 1 unit")), _ => None }
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    match id {
        "jhppt_swing_reversal" => Some(vec![p("swing", "Swing chart: 1 = minor, 2 = intermediate (default), 3 = main", 2., 1., 3., 1.),
            p("units", "Stop offset in price units beyond the top / bottom (book: 1, 2 or 3; default 1)", 1., 1., 3., 1.),
            p("unit", "One price unit [declared: 0.01]", 0.01, 0.0, 10., 0.01),
            p("side", "Sides: 0 = stop and reverse (book), 1 = long only, 2 = short only", 0., 0., 2., 1.)]),
        _ => None,
    }
}

/// Price nearest the open came first (p.87); then the close came last; exact tie = high first.
fn high_first(o: f64, h: f64, l: f64, c: f64) -> bool {
    if h - o != o - l { return h - o < o - l; }
    if c - l != h - c { return c - l < h - c; }
    true
}

struct Chart { n: u32, dir: i8, peak: f64, trough: f64, cu: u32, cd: u32, rh: f64, rl: f64, top: Option<f64>, bot: Option<f64> }

impl Chart {
    fn up(&mut self, v: f64) {
        self.cd = 0;
        if self.dir == 1 { self.peak = self.peak.max(v); return; }
        self.cu += 1;
        if self.cu >= self.n { if self.dir == -1 { self.bot = Some(self.trough); } self.dir = 1; self.peak = v; self.cu = 0; }
    }
    fn down(&mut self, v: f64) {
        self.cu = 0;
        if self.dir == -1 { self.trough = self.trough.min(v); return; }
        self.cd += 1;
        if self.cd >= self.n { if self.dir == 1 { self.top = Some(self.peak); } self.dir = -1; self.trough = v; self.cd = 0; }
    }
    fn bar(&mut self, o: f64, h: f64, l: f64, c: f64) {
        let (hh, ll) = (h > self.rh, l < self.rl);
        if !hh && !ll { return; } // inside (or equal) bar: ignored, reference bar kept
        if hh && !ll { self.up(h); } else if ll && !hh { self.down(l); } else {
            let hf = high_first(o, h, l, c);
            if self.n == 1 { if hf { self.up(h); self.down(l); } else { self.down(l); self.up(h); } }
            else if self.dir == 1 || hf { self.up(h); } else { self.down(l); }
        }
        self.rh = h; self.rl = l;
    }
}

fn run_swing(x: &Ctx, from: usize, to: usize, n: u32, k: f64, side: u8) -> Vec<Trade> {
    let (o, h, l, c) = (&x.o, &x.h, &x.l, &x.c);
    let mk = |d: f64, ei: usize, ep: f64, xi: usize, xp: f64, why: &str, open: bool| Trade { dir: d, legs: vec![Leg { i: ei, px: ep, w: 1. }], exit_i: xi, exit_px: xp, reason: why.into(), entry_kind: "stop", signal_i: xi, open, exits: vec![] };
    let mut ch = Chart { n, dir: 0, peak: h[from], trough: l[from], cu: 0, cd: 0, rh: h[from], rl: l[from], top: None, bot: None };
    let (mut out, mut cur) = (vec![], None::<(f64, usize, f64)>);
    for i in from + 1..=to {
        let bl = ch.top.map(|t| t + k);
        let sl = ch.bot.map(|b| b - k);
        if let Some((d, ei, ep)) = cur {
            let lvl = if d > 0. { sl } else { bl };
            if let Some(v) = lvl {
                if (d > 0. && l[i] <= v) || (d < 0. && h[i] >= v) {
                    let px = if (d > 0. && o[i] < v) || (d < 0. && o[i] > v) { o[i] } else { v };
                    let rev = side == 0;
                    out.push(mk(d, ei, ep, i, px, if rev { "reverse" } else { "stop" }, false));
                    cur = if rev { Some((-d, i, px)) } else { None };
                }
            }
        } else {
            let gl = side != 2 && bl.map_or(false, |v| h[i] >= v);
            let gs = side != 1 && sl.map_or(false, |v| l[i] <= v);
            let (bv, sv) = (bl.unwrap_or(f64::NAN), sl.unwrap_or(f64::NAN));
            cur = if gl && o[i] >= bv { Some((1., i, o[i])) } else if gs && o[i] <= sv { Some((-1., i, o[i])) }
                else if gl && gs { if high_first(o[i], h[i], l[i], c[i]) { Some((1., i, bv)) } else { Some((-1., i, sv)) } }
                else if gl { Some((1., i, bv)) } else if gs { Some((-1., i, sv)) } else { None };
        }
        ch.bar(o[i], h[i], l[i], c[i]);
    }
    if let Some((d, ei, ep)) = cur { out.push(mk(d, ei, ep, to, c[to], "open", true)); }
    out
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.r.len() { return None; }
    let schema = params(id)?;
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).map_or(f64::NAN, |d| d.def));
    Some(match id {
        "jhppt_swing_reversal" => run_swing(x, from, to, g("swing").round().clamp(1., 3.) as u32, g("units").max(0.) * g("unit").max(0.), g("side").clamp(0., 2.) as u8),
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn outside_bar_order_and_two_bar_turn() {
        assert!(high_first(10., 10.5, 8., 9.)); // open nearer the high
        assert!(!high_first(10., 12., 9.5, 11.)); // open nearer the low
        let mut ch = Chart { n: 2, dir: 0, peak: 10., trough: 9., cu: 0, cd: 0, rh: 10., rl: 9., top: None, bot: None };
        for (o, h, l, c) in [(9.5, 11., 9.5, 10.8), (10.8, 12., 10., 11.5), (11.5, 11.8, 9.8, 10.), (10., 10.5, 9., 9.2)] { ch.bar(o, h, l, c); }
        assert_eq!((ch.dir, ch.top), (-1, Some(12.)));
    }
}
