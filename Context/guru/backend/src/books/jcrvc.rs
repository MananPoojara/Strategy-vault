//! John Craciun, "The Reversal Charts" (markettide.com, 2003-04; Word chapters kagi-1/3/6 and renko-1/3/4/6, research share
//! USB HDD 2/tRADING KNO-2/John Craciun - The Reversal Charts (markettide.com)/). Book id `jcrvc`. Two rules: the Kagi
//! thickness stop-and-reverse and the Renko brick-colour stop-and-reverse, both on daily closes only. Lane K8 (titles loop,
//! 1 Nov 2026); independent check authors/titles-1030/K8w/k8_ref.py (header).
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};
use super::mkits::{hhv, llv};
use super::rbtie::target_trades;

/// Admitted ids (authors/titles-1030/K8w/k8_ref.py (header)).
pub const REVIEWED: &[&str] = &["jcrvc_kagi", "jcrvc_renko"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const KAGI_NOTE: &str = "[Declared readings] Reversal amount 1% of the line's last top or bottom (the author's BA study in Part VI uses a 1% daily Kagi; 2%, 3% and other percentages are options; fixed dollar amounts are not offered because they mean different things on different instruments). The first line starts at the first close in the data and takes its direction from the first close at least one reversal amount away. No position is taken before the line first crosses a prior top (Shoulder) or bottom (Waist). Fills at that day's close, as the author rephrased the rule ('buy or cover at the Close ... Entry at the Open the following day ... is not what Kagi promotes'). Always in the market, long or short ('Exit-and-Reverse'); no stops ('neither stop loss orders, nor trailing stops have been used in our study'). Long only is an option. The author's pattern, pyramiding and lower-time-frame ideas are judgement and not coded. Daily bars; the author's examples are US stocks, so indices, ETFs and India are a declared application. No results printed (one month of BA trades on charts).";
const RENKO_NOTE: &str = "[Declared readings] Brick size = the author's recommended formula (HHV(C,21) + LLV(C,21)) / 2 / 21 (renko-3), taken on the day the last brick printed and kept until the next brick (the formula 'will change daily'; the chart cannot redraw old bricks); option: a percentage of the close instead (2% = the author's IBM study, renko-6, which re-adjusted the size 'every ten dollars'). Bricks on closes: a white brick needs a close at least one brick above the last white brick's top, a reversal needs two bricks ('the price must move double the brick size amount for a brick of different color'); several bricks may print at once and the remainder is ignored. Trades: 'buy or cover on the emergence of a white brick and sell or sell short on the appearance of a black one', at that close (the author's trades are 'overnights from Close to Close'). The first position waits for the first change of colour (the author: 'it's better to wait for a reversal before entering a position'). Long only is an option. Pyramiding, intraday stops and early profit taking are the author's suggestions, not coded. US stocks in the book; indices, ETFs and India are a declared application.";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![
        StrategyDef { id: "jcrvc_kagi", name: "Kagi chart reversal: long when the line turns thick (yang), short when it turns thin (yin), 1% reversal, at the close", group: "Craciun Reversal Charts",
            session: 0, dir: "Long/Short", status: Status::Live, needs_volume: false, needs_vix: false, needs_series: &[], book: "jcrvc", rules: &[
                "F|\"The Kagi charts use only the Close and a pre-set reversal amount\"; a close beyond the line extends it, a close at least the reversal amount the other way starts a new line (kagi-1)",
                "F|\"every time the Close takes out a previous bottom, the line changes from thick (yang) to thin (yin) ... every time the Close takes out a previous top, the line changes from thin (yin) to thick (yang)\" (kagi-1)",
                "E|\"buy or cover at the Close on the day when a yang segment grows out of a yin line\" (kagi-6)",
                "X|\"sell or sell short at the Close on the day when a yin segment grows out of a yang line\"; Exit-and-Reverse, no stops (kagi-6)"],
            notes: KAGI_NOTE, claim: None },
        StrategyDef { id: "jcrvc_renko", name: "Renko brick reversal: long on the first white brick, short on the first black brick, at the close", group: "Craciun Reversal Charts",
            session: 0, dir: "Long/Short", status: Status::Live, needs_volume: false, needs_vix: false, needs_series: &[], book: "jcrvc", rules: &[
                "F|Brick size \"(HHV(C, periods) + LLV(C, periods)) / 2 / periods\", 21 days (renko-3); \"Only the Close values are needed\" (renko-1)",
                "F|A new brick needs a close one brick beyond the last brick; \"the price must move double the brick size amount for a brick of different color\" (renko-1)",
                "E|\"buy or cover on the emergence of a white brick\" (renko-4)",
                "X|\"sell or sell short on the appearance of a black one\" (renko-4)"],
            notes: RENKO_NOTE, claim: None },
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    match id {
        "jcrvc_kagi" => Some(("jcrvc_kagi", "Craciun Kagi reversal", "1% reversal")),
        "jcrvc_renko" => Some(("jcrvc_renko", "Craciun Renko reversal", "21-day brick formula")),
        _ => None,
    }
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    let side = p("side", "1 = long only (default 0: long and short)", 0., 0., 1., 1.);
    match id {
        "jcrvc_kagi" => Some(vec![p("rev", "Reversal amount, % of the last top or bottom (1 = the BA study)", 1., 0.25, 10., 0.25), side]),
        "jcrvc_renko" => Some(vec![p("brick", "Brick as % of the close (0 = the author's 21-day formula; 2 = the IBM study)", 0., 0., 10., 0.5),
            p("len", "Days in the brick formula (21)", 21., 5., 100., 1.), side]),
        _ => None,
    }
}

/// Kagi thickness on closes: +1 yang, -1 yin, 0 before the first crossing of a prior top or bottom.
fn kagi(c: &[f64], r: f64) -> Vec<i8> {
    let mut out = vec![0i8; c.len()];
    let Some(&c0) = c.first() else { return out };
    let (mut d, mut end, mut top, mut bot, mut th) = (0i8, c0, f64::NAN, f64::NAN, 0i8);
    for i in 1..c.len() {
        let v = c[i];
        match d {
            0 => if v >= end * (1. + r) { d = 1; end = v; } else if v <= end * (1. - r) { d = -1; end = v; },
            1 => if v > end { end = v; } else if v <= end * (1. - r) { top = end; d = -1; end = v; },
            _ => if v < end { end = v; } else if v >= end * (1. + r) { bot = end; d = 1; end = v; },
        }
        if th != 1 && end > top { th = 1; } else if th != -1 && end < bot { th = -1; }
        out[i] = th;
    }
    out
}

/// Renko colour on closes (+1 white, -1 black), 0 until the first change of colour.
fn renko(c: &[f64], size: &[f64]) -> Vec<i8> {
    let mut out = vec![0i8; c.len()];
    let ok = |s: f64| s.is_finite() && s > 0.;
    let Some(st) = (0..c.len()).find(|&i| ok(size[i])) else { return out };
    let (mut edge, mut col, mut s, mut rev) = (c[st], 0i8, size[st], false);
    for i in st + 1..c.len() {
        let v = c[i];
        let n = |q: f64| q.floor().max(1.);
        let drawn = if col >= 0 && v >= edge + s { edge += n((v - edge) / s) * s; col = 1; true }
            else if col == -1 && v >= edge + 2. * s { edge = edge + s + n((v - edge - s) / s) * s; col = 1; rev = true; true }
            else if col <= 0 && v <= edge - s { edge -= n((edge - v) / s) * s; col = -1; true }
            else if col == 1 && v <= edge - 2. * s { edge = edge - s - n((edge - s - v) / s) * s; col = -1; rev = true; true }
            else { false };
        if drawn && ok(size[i]) { s = size[i]; }
        out[i] = if rev { col } else { 0 };
    }
    out
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.r.len() { return None; }
    let schema = params(id)?;
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).map_or(f64::NAN, |d| d.def));
    let mut tgt = if id == "jcrvc_kagi" { kagi(&x.c, g("rev") / 100.) } else {
        let (pct, n) = (g("brick"), g("len").round().max(1.) as usize);
        let size: Vec<f64> = if pct > 0. { x.c.iter().map(|c| c * pct / 100.).collect() } else {
            let (hh, ll) = (hhv(&x.c, n), llv(&x.c, n));
            hh.iter().zip(&ll).map(|(h, l)| (h + l) / 2. / n as f64).collect()
        };
        renko(&x.c, &size)
    };
    if g("side") >= 1. { tgt.iter_mut().for_each(|t| *t = (*t).max(0)); }
    Some(target_trades(x, &tgt, from, to, false, if id == "jcrvc_kagi" { "Kagi line changed thickness" } else { "brick of the other colour" }))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn kagi_turns_thick_above_the_last_shoulder() {
        // up to 110, down to 100 (Shoulder 110), up to 105 (Waist 100), down to 99 (Shoulder 105): below the Waist 100 -> yin;
        // up to 106: above the last Shoulder 105 -> yang.
        let c = [100., 110., 100., 105., 99., 106.];
        assert_eq!(kagi(&c, 0.01), vec![0, 0, 0, 0, -1, 1]);
    }
    #[test]
    fn renko_needs_two_bricks_to_reverse() {
        let c = [100., 101., 102.5, 101.5, 100.4, 99.9, 103.];
        let s = vec![1.; c.len()];
        // white 101, white 102 (remainder ignored); 101.5 and 100.4 are above 102 - 2 bricks; 99.9 -> black (first reversal);
        // 103 >= 100 + 2 bricks -> white again
        assert_eq!(renko(&c, &s), vec![0, 0, 0, 0, 0, -1, 1]);
    }
}

// Instrument admission: John Craciun (The Reversal Charts) 2026-11-01 (titles lane K8).
// Admission only for the module REVIEWED lists (authors/titles-1030/K8w/k8_ref.py (header)).
// Runs on actual indices, ETFs and stocks, US and India (index_etf_or_stock, nifty50 basket, declared application).
pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book == "jcrvc" && crate::books::jcrvc::reviewed(def.id),
    label: "Reversal-chart rules shown on US stocks with no printed results; indices, ETFs and US/India stocks are a declared application.",
    basis: "John Craciun, The Reversal Charts (markettide.com, 2003-04): kagi-1, kagi-6 (Kagi), renko-1, renko-3, renko-4, renko-6 (Renko); independent check authors/titles-1030/K8w/k8_ref.py",
};
