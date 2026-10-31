//! Daryl Guppy, "Darvas Trading WorkBook" (guppytraders.com, darvasbook.pdf 21 slides) with its companion
//! "Modern Darvas Trading" slides (mdarvas.pdf, 23 slides). Book id "dgdwb".
//! Source copies: Books/Daryl Guppy/darvasbook.pdf, mdarvas.pdf (every slide read; notes in library/dgdwb/README.md).
//! Own simulator (no jkbwts::sim). One position, long only, weight 1.
use crate::connors::{Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};
/// Admitted ids (authors/guppy/admission/SOURCE-CONTRACT.md).
pub const REVIEWED: &[&str] = &["dgdwb_modern"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const BOX: &str = "F|\"Start with a new high for the selected period / Look for 3 lower highs / Set the top of the D_box / Look for a low followed by 3 higher lows / Set the bottom of the D_Box / Ignore equal highs and lows / Ignore price moves within the box\" (darvasbook slide 4; mdarvas slides 11-13: \"Do not have to be consecutively lower highs\", \"Do not have to be consecutively higher lows\")";
const SPECIAL: &str = "F|Special Darvas: \"Set the top of the box as normal / Set the bottom of the box on three higher lows / Initial low already established\" (darvasbook slide 6): the low search starts on the bar that set the top";

pub fn catalogue() -> Vec<StrategyDef> {
    let d = |id, name, group, status, rules, notes| StrategyDef { id, name, group, session: 0, dir: "Long", status, rules, notes,
        needs_volume: false, needs_vix: false, needs_series: &[], book: "dgdwb", claim: None };
    vec![
        d("dgdwb_classic", "Classic Darvas box (12-month high, intraday stop)", "Classic Darvas", Status::NotTestable("the classic entry also requires a volume increase with the breakout, which the workbook never measures; running it without that condition would change the book's rule"),
          &[BOX, SPECIAL,
            "F|Classic: \"Trade initiated by a new high for the rolling 12 month period / All decisions based on the high or low of the series / Action triggered by the first trade at the trigger price\" (mdarvas slide 10)",
            "E|\"Buy breakouts to new highs\"; \"Trigger is any trade above or below D_Box\" (darvasbook slides 4, 7)",
            "X|\"Sell breakouts below the box\"; \"Exit on intraday trade below the bottom of the box\" (darvasbook slides 4, 13): sell stop at the bottom of the most recent box"],
          CLASSIC_NOTES),
        d("dgdwb_modern", "Modern Darvas box (6-month high, close exit, ghost boxes)", "Modern Darvas", Status::Live,
          &[BOX, SPECIAL,
            "F|Modern: \"Trade initiated by a new high for the rolling 12 or 6 month period / All entry decisions based on the high or close of the series / Action triggered by the first trade at the trigger price or close\" (mdarvas slide 10); the worked example uses the \"Previous 6 month high\" (darvasbook slide 14) and slide 21 says \"Buy new six month high in new up trend\"",
            "E|First trade above the top of the D_Box (p.entry=1: the first close above it, bought at that close)",
            "X|\"All exit decisions based on the close of the series / Exit on the day after the trigger close\" (mdarvas slide 10); \"No sell signal until close below the D_Box ... Sell the next day\" (darvasbook slide 5); \"Modern Darvas ignores lows below the D_Box once the D_Box has been confirmed\" (slide 15)",
            "X|Ghost boxes: \"A 'ghost' box sets a temporary stop loss. It is the same height as the previous valid D_Box\"; \"Measure and project D_Box height\"; \"When prices close above projection target, apply a 'ghost' box\" (mdarvas slides 18-19): each close above top + k x height stacks ghost box k, whose bottom (top + (k-1) x height) is the stop"],
          MODERN_NOTES),
        d("dgdwb_breakout", "Breakout Darvas (second box after a downtrend breakout)", "Breakout Darvas", Status::NotTestable("the downtrend breakout is a trend-line break (chart reading) and 'second D_box' after it is not defined"),
          &["E|\"Trade initiated by the second Darvas box after the initial downtrend breakout / All entry decisions based on the high of the series / Action triggered by the first trade at the trigger price\" (mdarvas slide 10); \"Buy second D_box after breakout from downtrend\" (slide 21)",
            "X|\"All exit decisions based on the close of the series / Exit managed on the day after the trigger close / Stop loss calculation uses 'ghost' boxes\" (slide 10)"], "mdarvas slides 10, 17, 21."),
        d("dgdwb_entry_tactics", "Aggressive and delayed Darvas entries", "Darvas entry tactics", Status::NotTestable("'enter in anticipation of a breakout' and 'pullback' entries have no price rule"),
          &["E|\"Aggressive Darvas: Enter in anticipation of a breakout. Advantage: Tight stop loss\"; \"Delayed entry: ... Closer to stop loss and suitable for delayed identification. Disadvantage: Price might not pullback\" (mdarvas slide 20)"], "mdarvas slide 20."),
    ]
}
const CLASSIC_NOTES: &str = concat!("darvasbook slides 3-13, mdarvas slides 10-14. The classic column of mdarvas slide 10 also lists \"Volume increase with breakout\" without a measure (darvasbook slide 4's buy & sell list omits it); it is not applied. A box whose bottom trades before the breakout is a trend failure and is dropped. ",
    "Read: a bar is a 'new high for the period' when its high is strictly above every high of the bars dated within the previous N calendar months (the first N months of data only warm up). Lower highs = later bars with a high below the top, equal highs ignored; a new period high restarts the box. Low = lowest low since the top bar (inclusive), higher lows = later bars with a low above it, equal lows ignored, a lower low restarts the count. The box is set on the close of the bar that completes both counts and is used from the next bar. Buy stop: \"A buy is triggered by a move above the upper volatility range\" (slide 3), filled at the top or at the open if the bar gaps above it. Boxes keep forming during the trade (\"New Darvas boxes are added as new volatility ranges are defined\", slide 3); the stop is the bottom of the most recent box, from the bar after it is set; the stop is not checked on the entry bar. Long only, one position. Run on the indices (the book trades stocks).");
const MODERN_NOTES: &str = concat!("darvasbook slides 5, 14-19, mdarvas slides 10, 15, 18-19. Defaults: 6 months (p.months=12 is the other value the table allows), entry at the trigger price (p.entry=1 = close), ghost boxes on (p.ghost=0 off). Ghost boxes are only used in a trade and are measured from the most recent valid box; a newly set box restarts the projection. The book does not say whether a new box bottom below a ghost stop lowers the stop: the stop is never lowered [assumption]. darvasbook slide 18 also says \"Determine when a ghost box should be used\"; the mechanical trigger used is mdarvas slide 19's. A box whose bottom is closed below before the breakout is dropped. ",
    "Read: a bar is a 'new high for the period' when its high is strictly above every high of the bars dated within the previous N calendar months (the first N months of data only warm up). Lower highs = later bars with a high below the top, equal highs ignored; a new period high restarts the box. Low = lowest low since the top bar (inclusive), higher lows = later bars with a low above it, equal lows ignored, a lower low restarts the count. The box is set on the close of the bar that completes both counts and is used from the next bar. Exit at the next open after a close below the stop. Long only, one position. Run on the indices (the book trades stocks).");

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    match id {
        "dgdwb_classic" => Some(("dgdwb_darvas", "Darvas box (Guppy workbook)", "Classic · 12-month high, intraday stop")),
        "dgdwb_modern" => Some(("dgdwb_darvas", "Darvas box (Guppy workbook)", "Modern · 6-month high, close exit, ghost boxes")),
        "dgdwb_breakout" => Some(("dgdwb_darvas", "Darvas box (Guppy workbook)", "Breakout · not testable")),
        "dgdwb_entry_tactics" => Some(("dgdwb_darvas", "Darvas box (Guppy workbook)", "Entry tactics · not testable")),
        _ => None,
    }
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    match id {
        "dgdwb_classic" => Some(vec![p("months", "New-high period (calendar months)", 12., 3., 24., 3.),
            p("lower_highs", "Lower highs that set the top", 3., 2., 5., 1.), p("higher_lows", "Higher lows that set the bottom", 3., 2., 5., 1.)]),
        "dgdwb_modern" => Some(vec![p("months", "New-high period (calendar months)", 6., 3., 24., 3.),
            p("lower_highs", "Lower highs that set the top", 3., 2., 5., 1.), p("higher_lows", "Higher lows that set the bottom", 3., 2., 5., 1.),
            p("ghost", "Ghost boxes (1 on, 0 off)", 1., 0., 1., 1.), p("entry", "Entry (0 first trade above top, 1 first close above)", 0., 0., 1., 1.)]),
        _ => None,
    }
}

fn ymd(s: &str) -> (i32, u32, u32) {
    let mut it = s.split('-').map(|t| t.parse::<i32>().unwrap_or(0));
    (it.next().unwrap_or(0), it.next().unwrap_or(1) as u32, it.next().unwrap_or(1) as u32)
}
/// date minus k calendar months, day clamped to the month's length, as YYYY-MM-DD.
fn minus_months(s: &str, k: i32) -> String {
    let (y, m, d) = ymd(s);
    let t = y * 12 + m as i32 - 1 - k;
    let (yy, mm) = (t.div_euclid(12), t.rem_euclid(12) as u32 + 1);
    let leap = (yy % 4 == 0 && yy % 100 != 0) || yy % 400 == 0;
    let dim = [31, if leap { 29 } else { 28 }, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31][mm as usize - 1];
    format!("{:04}-{:02}-{:02}", yy, mm, d.min(dim))
}
/// new_high[i]: high strictly above every high dated on/after (date_i - months) before bar i; false while the data
/// does not yet cover a full period.
fn new_highs(x: &Ctx, months: i32) -> Vec<bool> {
    let n = x.r.len();
    let mut out = vec![false; n];
    let mut lo = 0usize;
    for i in 0..n {
        let cut = minus_months(&x.r[i].date, months);
        if x.r[0].date.as_str() > cut.as_str() { continue; }
        while x.r[lo].date.as_str() < cut.as_str() { lo += 1; }
        out[i] = x.h[lo..i].iter().all(|&v| x.h[i] > v);
    }
    out
}

#[derive(Clone, Copy)]
struct Cand { top: f64, lh: usize, low: f64, hl: usize }

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    let schema = params(id)?;
    let n = x.r.len();
    if from > to || to >= n { return None; }
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).unwrap().def);
    for (k, v) in p { if !schema.iter().any(|d| d.key == k) || v.fract() != 0. { return None; } }
    let modern = id == "dgdwb_modern";
    let (months, nlh, nhl) = (g("months") as i32, g("lower_highs") as usize, g("higher_lows") as usize);
    let (ghost, close_entry) = (modern && g("ghost") == 1., modern && g("entry") == 1.);
    let nh = new_highs(x, months);
    let mut cand: Option<Cand> = None;
    let mut boxx: Option<(f64, f64)> = None; // most recent valid box (top, bottom), usable from the next bar
    let mut out = vec![];
    let mut pos: Option<(Trade, f64, usize, (f64, f64))> = None; // trade, stop, entry bar, ghost reference box
    let mut pending_exit: Option<usize> = None; // modern: close below stop on this bar -> exit next open
    for i in 0..=to {
        // ---- orders on bar i (decided by the state at the close of i-1)
        if let Some(sig) = pending_exit.take() {
            let (mut t, _, _, _) = pos.take().unwrap();
            t.exit_i = i; t.exit_px = x.o[i]; t.signal_i = sig; t.reason = "close below the D_Box stop · next open".into();
            out.push(t);
        }
        if let Some((t, stop, ei, _)) = pos.as_mut() {
            if !modern && i > *ei && x.l[i] < *stop {
                let px = x.o[i].min(*stop);
                let mut t = t.clone(); t.exit_i = i; t.exit_px = px; t.signal_i = i; t.reason = "intraday trade below the D_Box bottom".into();
                out.push(t); pos = None; boxx = None;
            }
        } else if let Some((top, bot)) = boxx {
            let fill = if close_entry { (x.c[i] > top).then_some(x.c[i]) } else { (x.h[i] > top).then(|| x.o[i].max(top)) };
            if let Some(px) = fill {
                if i < from { boxx = None; } else {
                    let t = Trade { dir: 1., legs: vec![Leg { i, px, w: 1. }], exit_i: i, exit_px: px, reason: String::new(),
                        entry_kind: if close_entry { "close" } else { "stop" }, signal_i: i, open: false, exits: vec![] };
                    pos = Some((t, bot, i, (top, bot)));
                }
            }
        }
        // ---- box bookkeeping at the close of bar i
        if nh[i] {
            cand = Some(Cand { top: x.h[i], lh: 0, low: x.l[i], hl: 0 });
        } else if let Some(c) = cand.as_mut() {
            if x.h[i] < c.top { c.lh += 1; }
            if x.l[i] < c.low { c.low = x.l[i]; c.hl = 0; } else if x.l[i] > c.low { c.hl += 1; }
        }
        if let Some(c) = cand { if c.lh >= nlh && c.hl >= nhl {
            boxx = Some((c.top, c.low)); cand = None;
            if let Some((_, stop, _, gref)) = pos.as_mut() { *stop = stop.max(c.low); *gref = (c.top, c.low); }
        } }
        if pos.is_none() {
            // trend failure before a breakout drops the box (classic: any trade below; modern: a close below)
            if let Some((_, bot)) = boxx { if (!modern && x.l[i] < bot) || (modern && x.c[i] < bot) { boxx = None; } }
        } else if modern {
            let (_, stop, _, gref) = pos.as_mut().unwrap();
            if ghost {
                let (gt, gb) = *gref; let h = gt - gb;
                if h > 0. { let mut k = 1.; while x.c[i] > gt + k * h { *stop = stop.max(gt + (k - 1.) * h); k += 1.; } }
            }
            if x.c[i] < *stop && i < to { pending_exit = Some(i); boxx = None; }
        }
    }
    if let Some((mut t, _, _, _)) = pos { t.exit_i = to; t.exit_px = x.c[to]; t.signal_i = to; t.reason = "open".into(); t.open = true; out.push(t); }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;
    fn row(i: usize, h: f64, l: f64) -> Row {
        let d = 1 + i as u32; let (m, dd) = (1 + (d - 1) / 28, 1 + (d - 1) % 28);
        Row { ticker: "T".into(), date: format!("2001-{:02}-{:02}", m, dd), open: (h + l) / 2., high: h, low: l, close: (h + l) / 2., volume: None }
    }
    #[test]
    fn classic_box_breakout_and_intraday_stop() {
        // 2000 warm-up (flat at 10), then 2001: new high 20 (bar 0 of 2001), 3 lower highs, low 15 then 3 higher lows, breakout, stop.
        let mut r: Vec<Row> = (0..300).map(|i| { let mut x = row(0, 10., 9.); x.date = format!("2000-{:02}-{:02}", 1 + i / 28, 1 + i % 28); x }).collect();
        let seq = [(20., 18.), (19., 16.), (19.5, 15.), (18., 16.), (18.5, 16.5), (19., 17.), (21., 19.), (22., 20.), (21., 14.)];
        for (k, &(h, l)) in seq.iter().enumerate() { r.push(row(k, h, l)); }
        let x = Ctx::new(&r, &r, &[]);
        let ts = run("dgdwb_classic", &x, 0, r.len() - 1, &Params::new()).unwrap();
        assert_eq!(ts.len(), 1);
        let t = &ts[0];
        assert_eq!(t.legs[0].i, 306); assert_eq!(t.legs[0].px, 20.); // buy stop at the 20 top
        assert_eq!(t.exit_i, 308); assert_eq!(t.exit_px, 15.); // sell stop at the 15 bottom
    }
    #[test]
    fn minus_months_clamps() { assert_eq!(minus_months("2004-03-31", 1), "2004-02-29"); assert_eq!(minus_months("2005-01-15", 12), "2004-01-15"); }
}

// Instrument admission: Guppy 2026-10-25 block, moved from instrument_policy.rs on 31 Oct 2026 (lane ST).
// Admission only for the module REVIEWED lists (authors/guppy/admission/SOURCE-CONTRACT.md); kept in one block to merge.
// Runs on actual indices, ETFs and stocks, US and India (index_etf_or_stock, nifty50 basket, declared application).
pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book=="dgdwb" && crate::books::dgdwb::reviewed(def.id),
    label: "Daryl Guppy's Darvas workbook applies the Modern Darvas box to shares (ASX/US stocks); runs on indices, ETFs and single stocks (US and India) are a declared application.",
    basis: "Darvas Trading WorkBook (darvasbook.pdf slides 3-6, 14-19) + Modern Darvas Trading (mdarvas.pdf slides 10-19): 6-month new high, 3 lower highs set the top, a low and 3 higher lows set the bottom, buy the first trade above the box, exit the day after a close below the D_Box, ghost boxes for the stop. Index/ETF/stock runs are a declared application.",
};
