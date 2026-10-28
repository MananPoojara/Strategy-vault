//! YT Strategies · Inverted Pin Bar: the team's AmiBroker AFL in "Old Topics/PinBar (Inverted) short strategy", a
//! single-symbol daily short. Coded from the AFL as written, with its own default settings; there is no book PDF.
//! AmiBroker conventions as in `ytsw.rs` / `ytwh.rs`.
#![allow(dead_code)]
use crate::books::ytsw::true_range;
use crate::books::ytwh::{bound, collect, llv, Ev};
use crate::connors::{wilder, Ctx, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "yt_pin";
pub const PIN_SHORT: &str = "ytpin_inverted_pin_bar_short";
pub const IDS: [&str; 1] = [PIN_SHORT];

fn def(id: &'static str, name: &'static str, dir: &'static str, rules: &'static [&'static str], notes: &'static str) -> StrategyDef {
    StrategyDef { id, name, group: "YT Strategies · Inverted Pin Bar", session: 0, dir, status: Status::Live, needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK, rules, notes, claim: None }
}

pub fn catalogue() -> Vec<StrategyDef> {
    vec![def(PIN_SHORT, "Inverted Pin Bar Short", "Short", &[
        "F|Inverted pin bar: the top of the body (open on a red bar, close otherwise) is below high - 80% of the range",
        "E|Short at the close on a pin bar when yesterday's close is more than 5% above the lowest low of the last 20 bars (including today), today's true range is at least 1.5 x ATR(5), and the low is not the close; not on the first bar",
        "X|Stop: after a bar whose high exceeds the previous bar's high (1 such bar by default) the stop moves to that high; a lower high keeps it; an equal high resets it to 999999 (no stop). The stop is updated on every bar, flat or short",
        "X|Cover when the high exceeds the previous bar's stop: at the stop, or at the open + 0.25% when the open is above it. No target, no time exit",
    ], "pinbar short strategy.afl, coded as the AFL is written. Inside the loop ATR(5) is Null (-1e10) for its first bars, so the range test passes there (AmiBroker loop behaviour). Cover fills are kept within the bar's range (price bound checking). Dead-stock, penny and turnover options are stock-template settings left at their defaults (off). The AFL names no universe; the team's workbook is an optimisation only.")]
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    if id != PIN_SHORT {
        return None;
    }
    Some(vec![
        p("pin_range", "Range for pin bar % (upper wick share)", 80.0, 10.0, 100.0, 1.0),
        p("lookback", "Lookback period for the lowest low", 20.0, 1.0, 500.0, 1.0),
        p("pct_rise", "Percentage rise above the lowest low", 5.0, 1.0, 50.0, 1.0),
        p("atr_period", "ATR period", 5.0, 1.0, 100.0, 1.0),
        p("atr_mult", "ATR multiplier", 1.5, 1.0, 5.0, 0.5),
        p("hdays", "Higher-high days for the stop", 1.0, 0.0, 10.0, 1.0),
    ])
}

const BIG: f64 = 999_999.0;

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    let schema = params(id)?;
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).map_or(0.0, |d| d.def));
    let gu = |k: &str| g(k).round().max(1.0) as usize;
    let (o, h, l, c) = (&x.o, &x.h, &x.l, &x.c);
    let n = c.len();
    if n == 0 || from > to || to >= n {
        return Some(Vec::new());
    }
    let lo = llv(l, gu("lookback"));
    let rise = g("pct_rise");
    let pr = g("pin_range");
    let tr: Vec<f64> = true_range(h, l, c).into_iter().map(|v| v.unwrap_or(f64::NAN)).collect();
    let atr = wilder(&tr, gu("atr_period"));
    let mult = g("atr_mult");
    let hdays = g("hdays").round() as i64;
    let pin = |i: usize| {
        let con = h[i] - (h[i] - l[i]) * pr / 100.0;
        if o[i] > c[i] { o[i] < con } else { c[i] < con }
    };
    let mut evs: Vec<Ev> = Vec::new();
    let mut pos = false;
    let mut sl = vec![BIG; n];
    let mut cnt = vec![0i64; n];
    for i in 1..n {
        let atr_x = if atr[i].is_nan() { -1e10 } else { atr[i] * mult };
        let filter = l[i] != c[i] && c[i] > 0.0;
        if !pos && pin(i) && c[i - 1] > lo[i] * (1.0 + rise / 100.0) && tr[i] >= atr_x && filter {
            sl[i] = BIG;
            pos = true;
            evs.push(Ev { dir: -1.0, e: i, epx: c[i], exit: None });
        } else if pos && h[i] > sl[i - 1] {
            let px = if o[i] > sl[i - 1] { o[i] + o[i] * 0.0025 } else { sl[i - 1] };
            if let Some(v) = evs.last_mut() {
                v.exit = Some((i, bound(x, i, px), i, "Higher-high stop"));
            }
            sl[i] = BIG;
            cnt[i] = 0;
            pos = false;
        } else if h[i] > h[i - 1] {
            cnt[i] = cnt[i - 1] + 1;
            sl[i] = if cnt[i] >= hdays { h[i] } else { sl[i - 1] };
        } else if h[i] < h[i - 1] {
            sl[i] = sl[i - 1];
            cnt[i] = 0;
        }
        // equal highs: the stop stays at its initial 999999 and the count at 0
    }
    Some(collect(x, evs, from, to))
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    let name = catalogue().into_iter().find(|d| d.id == id)?.name;
    Some((match id {
        PIN_SHORT => "ytpin_inverted_pin_bar_short",
        _ => return None,
    }, name, "PinBar (Inverted) short strategy"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::books::ytwh::testbars::*;

    /// Narrow rising bars from 100 to 110, then a wide inverted pin bar.
    fn setup() -> Vec<(f64, f64, f64, f64)> {
        let mut v: Vec<(f64, f64, f64, f64)> = (0..11).map(|k| { let c = 100.0 + k as f64; (c - 0.3, c + 0.5, c - 0.5, c + 0.1) }).collect();
        // bar 11: high 118, low 109, open 109.8, close 110.2: body top 110.2 < 118 - 0.8 x 9 = 110.8; TR 8 >= 1.5 x ATR
        v.push((109.8, 118.0, 109.0, 110.2));
        v
    }

    #[test]
    fn shorts_pin_and_covers_on_higher_high_stop() {
        let mut v = setup();
        v.push((110.0, 112.0, 109.0, 110.5)); // bar 12: lower high (112 < 118): stop stays 999999
        v.push((110.5, 113.0, 110.0, 112.0)); // bar 13: higher high -> stop 113
        v.push((112.0, 114.0, 111.0, 113.5)); // bar 14: high 114 > 113 -> cover at 113
        let t = go(run, PIN_SHORT, &bars(&v), &[]);
        assert_eq!(t.len(), 1);
        assert_eq!((t[0].0, t[0].1, t[0].2, t[0].3), (11, 110.2, 14, 113.0));
        let r = bars(&v);
        let x = Ctx::new(&r, &r, &[]);
        assert_eq!(run(PIN_SHORT, &x, 0, 13, &Params::new()).unwrap()[0].dir, -1.0);
    }

    #[test]
    fn gap_over_stop_covers_at_open_plus_quarter_pct() {
        let mut v = setup();
        v.push((110.5, 113.0, 110.0, 112.0)); // bar 12: higher high? 113 < 118 -> lower high, stop stays
        v.push((112.0, 114.0, 111.0, 113.5)); // bar 13: higher high -> stop 114
        v.push((115.0, 116.0, 114.5, 115.5)); // bar 14: opens 115 > 114 -> cover at 115 x 1.0025
        let t = go(run, PIN_SHORT, &bars(&v), &[]);
        assert_eq!(t[0].2, 14);
        assert!((t[0].3 - 115.0 * 1.0025).abs() < 1e-9);
    }

    #[test]
    fn needs_prior_rise_and_open_position_marked() {
        // without the 5% rise from the 20-bar low (flat bars) there is no short
        let mut v: Vec<(f64, f64, f64, f64)> = (0..11).map(|_| (100.0, 100.5, 99.5, 100.1)).collect();
        v.push((99.8, 108.0, 99.0, 100.2));
        assert!(go(run, PIN_SHORT, &bars(&v), &[]).is_empty());
        // a short still open at the end is marked at the last close
        let t = go(run, PIN_SHORT, &bars(&setup()), &[]);
        assert_eq!((t[0].0, t[0].2, t[0].4), (11, 11, true));
    }

    #[test]
    fn every_strategy_runs() {
        smoke(run, &IDS);
    }
}
