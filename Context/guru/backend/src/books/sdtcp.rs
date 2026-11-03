//! Suri Duddella, "Trade Chart Patterns Like The Pros" (surinotes.com, 2007; research share Trading Knowledge I/Suri Duddella -
//! Trade Chart Patterns Like The Pros (Included TS Code) (surinotes.com)/). Book id `sdtcp`. Three rules with their own entry,
//! stop and target or time exit: n-Bar Rallies/Declines (p.27-30), NR7ID with previous-bar breakout (p.31-34) and the Donchian
//! 4-week channel with an ATR target (p.123-126). Lane K8 (titles loop, 1 Nov 2026); independent check
//! authors/titles-1030/K8w/k8_ref.py (header). The book's other patterns are held (see the notes).
use crate::connors::{atr, Ctx, ParamDef, Params, Status, StrategyDef, Trade};
use super::rbtie::mk;

/// Admitted ids (authors/titles-1030/K8w/k8_ref.py (header)).
pub const REVIEWED: &[&str] = &["sdtcp_nbar", "sdtcp_nr7id", "sdtcp_donchian"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

macro_rules! common { () => { " One position at a time; exits are tested from the bar after the entry bar (daily bars cannot show the order of moves inside the entry bar); when a stop and a target are both inside one bar, the stop is taken. Stop entries fill at the level or at the open when the bar gaps through it; the same for stop exits and targets. One tick = 0.01 on US prices, 0.05 on NSE. The book's examples are Russell 2000 e-mini tick charts and daily/weekly US stocks ('any time-frame'): daily ETFs, US/India indices and stocks are a declared application. No printed results. Held from this book: Market Structure lows/highs ('It needs other indicators ... to confirm'), three-bar groups (targets at a 'major swing high'), matching highs/lows (no tolerance), WR7OD ('current trend' undefined), pivots and opening-range breakouts (intraday), Fibonacci/harmonic/geometric/channel/zigzag patterns (hand-drawn swings), bands (interpretation)." } }
const NBAR: &str = concat!("[Declared readings] A new-low bar = a low below the lowest low of the 21 bars before it ('at least a 21-bar new high and new-low'); the setup = a streak of at least 3 such bars in a row ('at least 3 new highs or 3 new lows successively'), the whole streak counted. Buy stop one tick above the high of the last new-low bar, valid on the next bar only ('After an exhaustion move, enter a long one tick ... above the last falling bar's high'). Stop one tick below the lowest low of the streak; target = entry + 62% of the streak's range (highest high to lowest low of its bars; 'a target 62% to 100%'; 100% is an option). Short mirror on new-high streaks; long only is an option; when a bar ends both streaks the long setup is used.", common!());
const NR7: &str = concat!("[Declared readings] NR7 = a daily range narrower than each of the 6 days before; inside day = high below the previous high and low above the previous low (p.31). 'enter a long trade on the next day if the price trades 10 cents above the previous days high or enter a short if the price trades 10 cents below the previous days' low' (the GOOG daily version, p.33): 0.10 above or below, the next day only; a day that trades both levels is skipped (the order is unknown on daily bars). 'Trade stops are placed on the other end of NR7ID against the trade'; 'should be closed within 1-3 bars (days)': out at the close of the 3rd day counting the entry day (1 and 2 are options). The ORB version (10-bar open-to-high/low distances) is not coded.", common!());
const DON: &str = concat!("[Declared readings] 4 weeks = 20 trading days. Long on a stop at the highest high of the 20 bars before today ('Enter long ... when price exceeds the highest high of a 4-week range'), short at the 20-bar lowest low; a day that crosses both is skipped. Target = 2 x Wilder ATR(14) from the entry price (the book: '1.5 to 2 Average True Ranges'; the ATR period is not printed; 1.5 is an option), using the ATR known at the previous close. Stop = the channel's mid-point when the trade is entered ('Protect trades at mid-channel level'; the 10-day low/high alternative is not used). After an exit the next breakout re-enters.", common!());

pub fn catalogue() -> Vec<StrategyDef> {
    let d = |id, name, rules, notes| StrategyDef { id, name, group: "Duddella Trade Chart Patterns", session: 0, dir: "Long/Short", status: Status::Live,
        needs_volume: false, needs_vix: false, needs_series: &[], book: "sdtcp", rules, notes, claim: None };
    vec![
        d("sdtcp_nbar", "n-Bar Decline / Rally: after 3+ successive 21-bar new lows, buy a tick above the last bar's high; 62% target, stop under the run", &[
            "F|\"Prices must make at least a 21-bar new high and new-low, and at least 3 new highs or 3 new lows successively\" (p.27)",
            "E|\"After an exhaustion move, enter a long one tick or 5 cents above the last falling bar's high, or enter a short one tick below the last rising bar's low\" (p.27)",
            "X|\"Place a stop order one tick below the low of the falling bar in a long setup\"; \"Place a target 62% to 100% of the entire range of n-bar Rally/Declines\" (p.27)"], NBAR),
        d("sdtcp_nr7id", "NR7 inside day: next day 10 cents beyond its high or low, stop at the other end, out within 3 days", &[
            "F|\"NR7 is defined as the daily range which is narrower than the prior 6-Day range\"; inside day \"completely encompassed by the previous days' range\" (p.31)",
            "E|\"enter a long trade on the next day if the price trades 10 cents above the previous days high or enter a short if the price trades 10 cents below the previous days' low\" (p.33)",
            "X|\"Trade stops are placed on the other end of NR7ID against the trade ... should be closed within 1-3 bars (days)\" (p.33)"], NR7),
        d("sdtcp_donchian", "Donchian 4-week breakout with a 2-ATR target and a mid-channel stop", &[
            "E|\"Enter long or cover short when price exceeds the highest high of a 4-week range. Enter a short when price falls below the lowest-low of a 4-week range\" (p.123)",
            "X|\"Exit trades when prices reach 1.5 to 2 Average True Ranges from the entry\"; \"Protect trades at mid-channel level\" (p.123)"], DON),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    match id {
        "sdtcp_nbar" => Some(("sdtcp_nbar", "Duddella n-Bar Rally/Decline", "21-bar, 3 bars, 62%")),
        "sdtcp_nr7id" => Some(("sdtcp_nr7id", "Duddella NR7ID breakout", "10 cents, 3 days")),
        "sdtcp_donchian" => Some(("sdtcp_donchian", "Duddella Donchian 4-week", "20 days, 2 ATR")),
        _ => None,
    }
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    match id {
        "sdtcp_nbar" => Some(vec![p("min", "New-low (new-high) bars in a row, at least (3)", 3., 2., 10., 1.),
            p("pct", "Target, % of the run's range (62; up to 100)", 62., 20., 100., 1.), p("side", "1 = long only (default 0: long and short)", 0., 0., 1., 1.)]),
        "sdtcp_nr7id" => Some(vec![p("off", "Entry offset beyond the NR7ID bar (0.10 = '10 cents')", 0.10, 0., 5., 0.05),
            p("days", "Close the trade at the close of this day, entry day = 1 (3; 1-3)", 3., 1., 3., 1.)]),
        "sdtcp_donchian" => Some(vec![p("mult", "Target in ATR(14) from the entry (2; 1.5 option)", 2., 1., 4., 0.5)]),
        _ => None,
    }
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.r.len() { return None; }
    let schema = params(id)?;
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).map_or(f64::NAN, |d| d.def));
    let (o, h, l, c) = (&x.o, &x.h, &x.l, &x.c);
    let tick = x.market_tick();
    let a = atr(x.r, 14);
    let strict = id == "sdtcp_donchian";
    // pending entry: (side 1/-1/0 = both, (long level, short level), (long stop, short stop), target distance)
    let mut pend: Option<(i8, (f64, f64), (f64, f64), Option<f64>)> = None;
    // position: (dir, entry bar, entry price, stop, target, last bar)
    let mut pos: Option<(f64, usize, f64, f64, Option<f64>, Option<usize>)> = None;
    let mut out = vec![];
    let new_low = |k: usize| k >= 21 && l[k] < l[k - 21..k].iter().copied().fold(f64::INFINITY, f64::min);
    let new_high = |k: usize| k >= 21 && h[k] > h[k - 21..k].iter().copied().fold(f64::NEG_INFINITY, f64::max);
    for i in from..=to {
        if let (None, Some((d, lvl, stp, rng))) = (pos, pend) {
            let (mut hl, mut hs) = if strict { (h[i] > lvl.0, l[i] < lvl.1) } else { (h[i] >= lvl.0, l[i] <= lvl.1) };
            if d == 0 && hl && hs { hl = false; hs = false; }
            if d >= 0 && hl { let ep = o[i].max(lvl.0); pos = Some((1., i, ep, stp.0, rng.map(|r| ep + r), None)); }
            else if d <= 0 && hs { let ep = o[i].min(lvl.1); pos = Some((-1., i, ep, stp.1, rng.map(|r| ep - r), None)); }
            if let Some(q) = pos.as_mut() {
                if id == "sdtcp_nr7id" { q.5 = Some(i + g("days").round().max(1.) as usize - 1); }
                if id == "sdtcp_donchian" { let m = g("mult") * a[i - 1]; q.4 = Some(if q.0 > 0. { q.2 + m } else { q.2 - m }); }
            }
        }
        pend = None;
        if let Some((d, e, ep, stp, tg, _)) = pos {
            if i > e {
                let px = if d > 0. && l[i] <= stp { Some(o[i].min(stp)) } else if d < 0. && h[i] >= stp { Some(o[i].max(stp)) }
                    else if let Some(t) = tg.filter(|&t| (d > 0. && h[i] >= t) || (d < 0. && l[i] <= t)) { Some(if d > 0. { o[i].max(t) } else { o[i].min(t) }) }
                    else { None };
                if let Some(px) = px { out.push(mk(d, e, ep, "stop", i, px, "stop or target", false, i)); pos = None; }
            }
        }
        if let Some((d, e, ep, _, _, Some(last))) = pos {
            if i >= last { out.push(mk(d, e, ep, "stop", i, c[i], "time exit", false, i)); pos = None; }
        }
        if pos.is_none() && i < to {
            match id {
                "sdtcp_nbar" => {
                    let (mn, pct) = (g("min").round().max(1.) as usize, g("pct") / 100.);
                    let sides: &[i8] = if g("side") >= 1. { &[1] } else { &[1, -1] };
                    for &d in sides {
                        let f = |k: usize| if d > 0 { new_low(k) } else { new_high(k) };
                        let mut k = i as i64;
                        while k >= 0 && f(k as usize) { k -= 1; }
                        let run = (k + 1) as usize..=i;
                        if i as i64 - k >= mn as i64 {
                            let hi = h[run.clone()].iter().copied().fold(f64::NEG_INFINITY, f64::max);
                            let lo = l[run].iter().copied().fold(f64::INFINITY, f64::min);
                            pend = Some((d, (h[i] + tick, l[i] - tick), (lo - tick, hi + tick), Some(pct * (hi - lo))));
                            break;
                        }
                    }
                }
                "sdtcp_nr7id" => {
                    if i >= 7 && (i - 6..i).all(|j| h[i] - l[i] < h[j] - l[j]) && h[i - 1] > h[i] && l[i - 1] < l[i] {
                        let off = g("off");
                        pend = Some((0, (h[i] + off, l[i] - off), (l[i], h[i]), None));
                    }
                }
                _ => {
                    if i >= 19 {
                        let hh = h[i - 19..=i].iter().copied().fold(f64::NEG_INFINITY, f64::max);
                        let ll = l[i - 19..=i].iter().copied().fold(f64::INFINITY, f64::min);
                        pend = Some((0, (hh, ll), ((hh + ll) / 2., (hh + ll) / 2.), None));
                    }
                }
            }
        }
    }
    if let Some((d, e, ep, ..)) = pos { out.push(mk(d, e, ep, "stop", to, c[to], "open", true, to)); }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;
    fn rows(b: &[(f64, f64, f64, f64)]) -> Vec<Row> {
        b.iter().enumerate().map(|(i, &(o, h, l, c))| Row { ticker: "T".into(), date: format!("2001-{:02}-{:02}", 1 + i / 28, 1 + i % 28), open: o, high: h, low: l, close: c, volume: None }).collect()
    }
    #[test]
    fn nr7id_breakout_stop_and_time_exit() {
        let mut b = vec![(100., 101., 99., 100.); 6];
        b.extend([(100., 101.5, 98.5, 100.), (100., 100.6, 99.6, 100.2), (100.3, 101., 100., 100.8), (100.8, 101.2, 100.4, 101.), (101., 101.5, 100.8, 101.3)]);
        let r = rows(&b);
        let x = Ctx::new(&r, &r, &[]);
        let t = run("sdtcp_nr7id", &x, 0, r.len() - 1, &Params::new()).unwrap();
        // bar 7 is NR7 and inside bar 6; bar 8 trades 100.7 = 100.6 + 0.10: long at 100.7, out at bar 10's close (3rd day)
        assert_eq!(t.iter().map(|z| (z.dir, z.legs[0].i, z.legs[0].px, z.exit_i, z.exit_px)).collect::<Vec<_>>(), vec![(1., 8, 100.69999999999999, 10, 101.3)]);
    }
}

// Instrument admission: Suri Duddella (Trade Chart Patterns Like The Pros) 2026-11-01 (titles lane K8).
// Admission only for the module REVIEWED lists (authors/titles-1030/K8w/k8_ref.py (header)).
// Runs on actual indices, ETFs and stocks, US and India (index_etf_or_stock, nifty50 basket, declared application).
pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book == "sdtcp" && crate::books::sdtcp::reviewed(def.id),
    label: "Pattern setups shown on e-mini tick charts and US stock charts with no results; daily ETFs, US/India indices and stocks are a declared application.",
    basis: "Suri Duddella, Trade Chart Patterns Like The Pros (2007) p.27-34, 123-126; independent check authors/titles-1030/K8w/k8_ref.py",
};
