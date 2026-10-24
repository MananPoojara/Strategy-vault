//! Steve Wirrick: "Make a Fortune Swing Trading!" (2003; book id `swmfst`), "High Octane Investing" (with Barry Collette and Preston
//! Christensen; `swhoi`) and "High Octane Trading" (2003; `swhot`). Reading: claude-continuation-20261020/authors/wirrick/agents/.
//! Live: the bullish and bearish swing trade set-up of Make a Fortune Swing Trading (p.43, 50-53): a pull-back of at least two days
//! in a row above the 40- or 50-day average, entry when the next day trades above the previous day's high, protective stop below the
//! previous day's low, trailing exit on a close below the 20-day average (p.38, 58). The book trades stocks and their options; the
//! stock-price rule runs on stocks, indices and ETFs as a declared application. Daily bars; stop orders fill at the level or at the
//! open if the bar opens beyond it.
use crate::books::mpts::sma;
use crate::connors::{Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "swmfst";
pub const REVIEWED: &[&str] = &["swmfst_swing"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const SWING_NOTE: &str = "[Declared readings, not from the book] 'Pull back at least two days in a row' = at least two consecutive lower closes (bearish: higher closes) ending on the set-up day; 'remain above the 40- or 50-day moving average' = the set-up day's close above the 50-day simple average (40 is an option); the trigger is the next day only: buy when it trades above the set-up day's high (option: the highest high of the last 2 or 3 days, the book's 'more conservative' trigger), filled at that level or at the open if it opens above; protective stop = the set-up day's low ('the previous day's low', p.51), fixed; trailing exit = a close below the 20-day simple average, at that close (p.38: 'if the stock closes below that moving average'; 18 and 10 days are options, p.46, 89). Stops and the trailing exit start the day after entry (a stop touched on the entry bar is not counted, as the order of the day's prices is unknown). Taking profits at resistance, time stops, the 8-10% warning stop and option selection are judgement and not used. One position at a time; flat at the window start; no costs. Bearish mirror: close below the 50-day average, two higher closes, sell below the set-up day's low, stop above its high, exit on a close above the 20-day average. Book: US stocks traded with options; indices, ETFs and stocks, US and India, are a declared application.";

pub fn catalogue() -> Vec<StrategyDef> {
    use Status::*;
    let nt = |id, name, group, book, why, rules| StrategyDef { id, name, group, session: 0, dir: "Long/Short", status: NotTestable(why), needs_volume: false,
        needs_vix: false, needs_series: &[], book, rules, notes: "Held after the 2026-10-24 reading (authors/wirrick/agents).", claim: None };
    vec![
        StrategyDef { id: "swmfst_swing", name: "Swing trade set-up: two-day pull-back above the 50-day average", group: "Make a Fortune Swing Trading · The Swing Trade Set-up (p.43-58)",
            session: 0, dir: "Long/Short", status: Live, needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK,
            rules: &["F|\"Bullish Setup 1) Wait for the stock to pull back at least two days in a row. 2) Stock must remain above 40- or 50-day moving average\" (p.43)",
                     "E|\"wait for the stock to trade above the high of the previous day. If you want to play it more conservatively wait for the stock to take out the highs of the last 2 or 3 days\" (p.50)",
                     "X|\"You place a protective stop below the swing low or the previous day's low\" (p.51)",
                     "X|\"use the 20-day moving average as a trailing stop, i.e. if the stock closes below that moving average\" (p.38); \"Use a trailing stop in the form of a shorter moving average\" (p.58)",
                     "E|Bearish set-up: a rally within a downtrend, below the 40/50-day average; \"wait for the stock to trade below the previous 2 to 3 day lows\", stop above the swing or the previous day's high (p.53, 72)"],
            notes: SWING_NOTE, claim: None },
        nt("swmfst_patterns", "Cash-in-the-box, ascending triangles and sleeping giants", "Make a Fortune Swing Trading · chart patterns", BOOK,
           "boxes, triangles, support and resistance are drawn on the chart by the trader; 'sleeping giant' is a narrowing range judged by eye; profits are taken at resistance",
           &["E|\"The trigger is the high of the last three or four trading days - take your pick\" (sleeping giant, p.91)"]),
        nt("swhoi_options", "Options strategies (pirate plays, straddles, puts when stocks crash)", "High Octane Investing · Ch.3, 7-12", "swhoi",
           "option strategies need option prices; stock selection (Ch.9) is fundamental and judged", &[]),
        nt("swhoi_bollinger", "Bollinger band squeeze breakouts (Barry Collette)", "High Octane Investing · Ch.5", "swhoi",
           "'when the bands tighten' has no threshold and the exit is not given; written by co-author Barry Collette", &[]),
        nt("swhot_charts", "Trend, moving-average and chart reading", "High Octane Trading · Ch.10-17", "swhot",
           "general chart reading (40-50, 18-21 and 8-10 day averages for the long, intermediate and short trend) and trading psychology; no complete rule beyond the swing set-up of Make a Fortune Swing Trading", &[]),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    Some(match id {
        "swmfst_swing" => ("swmfst_swing", "Wirrick swing trade set-up", "2-day pull-back / 50 / 20"),
        _ if id.starts_with("swmfst_") || id.starts_with("swhoi_") || id.starts_with("swhot_") => ("swirrick_nt", "Wirrick (held)", "-"),
        _ => return None,
    })
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    Some(match id {
        "swmfst_swing" => vec![p("ma", "Trend average (days; book: 40 or 50)", 50., 10., 200., 1.),
            p("pullback", "Pull-back closes in a row (at least)", 2., 1., 5., 1.),
            p("trigger", "Trigger: high/low of the last N days (book: 1, conservative 2-3)", 1., 1., 3., 1.),
            p("exit_ma", "Trailing average (days; book: 20, also 18 or 10)", 20., 5., 50., 1.),
            p("side", "Sides (0 both, 1 long only, -1 short only)", 0., -1., 1., 1.)],
        _ if id.starts_with("swmfst_") || id.starts_with("swhoi_") || id.starts_with("swhot_") => vec![],
        _ => return None,
    })
}

fn stop_hit(x: &Ctx, i: usize, dir: f64, s: f64) -> Option<f64> {
    if dir > 0. { if x.o[i] < s { Some(x.o[i]) } else if x.l[i] < s { Some(s) } else { None } }
    else if x.o[i] > s { Some(x.o[i]) } else if x.h[i] > s { Some(s) } else { None }
}
fn entry_fill(x: &Ctx, i: usize, dir: f64, lv: f64) -> Option<f64> {
    if dir > 0. { if x.o[i] > lv { Some(x.o[i]) } else if x.h[i] > lv { Some(lv) } else { None } }
    else if x.o[i] < lv { Some(x.o[i]) } else if x.l[i] < lv { Some(lv) } else { None }
}

/// Set-up direction on the close of bar i (+1 bullish, -1 bearish, 0 none).
fn setup(x: &Ctx, m: &[f64], i: usize, pb: usize) -> f64 {
    if i < pb || !m[i].is_finite() { return 0.; }
    let down = (0..pb).all(|k| x.c[i - k] < x.c[i - k - 1]);
    let up = (0..pb).all(|k| x.c[i - k] > x.c[i - k - 1]);
    if down && x.c[i] > m[i] { 1. } else if up && x.c[i] < m[i] { -1. } else { 0. }
}

fn run_swing(x: &Ctx, from: usize, to: usize, ma: usize, pb: usize, trig: usize, xma: usize, side: f64) -> Vec<Trade> {
    let (m, e) = (sma(&x.c, ma), sma(&x.c, xma));
    let (mut out, mut cur): (Vec<Trade>, Option<(Trade, f64)>) = (vec![], None);
    let mut i = from;
    while i <= to {
        if let Some((mut t, stop)) = cur.take() {
            let d = t.dir;
            if let Some(px) = stop_hit(x, i, d, stop) {
                t.exit_i = i; t.exit_px = px; t.signal_i = i; t.reason = "protective stop beyond the set-up day".into(); out.push(t);
            } else if e[i].is_finite() && (d * (x.c[i] - e[i])) < 0. {
                t.exit_i = i; t.exit_px = x.c[i]; t.signal_i = i; t.reason = format!("close beyond the {xma}-day average"); out.push(t);
            } else { cur = Some((t, stop)); i += 1; continue; }
        }
        // flat at the close of bar i: a set-up places a stop order for bar i + 1
        let d = setup(x, &m, i, pb);
        let j = i + 1;
        if d == 0. || j > to || (side != 0. && side != d) || i + 1 < trig { i += 1; continue; }
        let lv = if d > 0. { (i + 1 - trig..=i).map(|k| x.h[k]).fold(f64::MIN, f64::max) } else { (i + 1 - trig..=i).map(|k| x.l[k]).fold(f64::MAX, f64::min) };
        if let Some(px) = entry_fill(x, j, d, lv) {
            let stop = if d > 0. { x.l[i] } else { x.h[i] };
            cur = Some((Trade { dir: d, legs: vec![Leg { i: j, px, w: 1. }], exit_i: j, exit_px: px, reason: String::new(), entry_kind: "stop", signal_i: j, open: false, exits: vec![] }, stop));
            i = j + 1;   // exits start the day after entry
            continue;
        }
        i += 1;
    }
    if let Some((mut t, _)) = cur { t.exit_i = to; t.exit_px = x.c[to]; t.signal_i = to; t.reason = "open".into(); t.open = true; out.push(t); }
    out
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.r.len() { return None; }
    let schema = params(id)?;
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).map_or(f64::NAN, |d| d.def));
    let n = |k: &str| g(k).round().max(1.) as usize;
    let side = g("side").round().clamp(-1., 1.);
    Some(run_swing(x, from, to, n("ma"), n("pullback"), n("trigger").min(3), n("exit_ma"), side))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;
    fn rows(v: &[(f64, f64, f64, f64)]) -> Vec<Row> {
        v.iter().enumerate().map(|(i, &(o, h, l, c))| Row { ticker: "T".into(), date: format!("2001-{:02}-{:02}", 1 + i / 28, 1 + i % 28), open: o, high: h, low: l, close: c, volume: None }).collect()
    }
    #[test]
    fn two_lower_closes_above_average_buy_the_break_and_stop_at_setup_low() {
        // ma=3, exit_ma=3: rising to 13, two lower closes (12.5, 12.2) above the 3-day average? use ma=2 for a simple check
        let r = rows(&[(10., 10.5, 9.5, 10.), (11., 11.5, 10.5, 11.), (12., 13.2, 11.8, 13.), (13., 13.1, 12.4, 12.8), (12.8, 12.9, 12.5, 12.7),
                       (12.8, 13.5, 12.6, 13.4), (13.4, 13.6, 12.0, 12.2), (12.2, 12.3, 11.0, 11.2)]);
        let x = Ctx::new(&r, &r, &[]);
        // set-up on bar 4 (closes 13 -> 12.8 -> 12.7, close 12.7 > sma(3)=12.83? no) -> use ma=5 window: sma5 at 4 = 11.9 < 12.7 -> long set-up
        let t = run_swing(&x, 0, 7, 5, 2, 1, 50, 0.);
        assert_eq!((t[0].dir, t[0].legs[0].i, t[0].legs[0].px), (1., 5, 12.9));
        assert_eq!((t[0].exit_i, t[0].exit_px), (6, 12.5)); // stop at the set-up day's low 12.5
    }
}
