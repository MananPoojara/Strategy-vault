//! David Jennings (The Performance Factory), "Code Snippets & How To" for MetaStock, TradeStation and AIQ (Word file,
//! book id `djcs`; a compilation of mailing-list code by other people, each credited in the file). Reading: lane K2,
//! claude-continuation-20261020/authors/jennings (admission/djcs_ref.py; plain-text export of the .doc).
//! Live: Bob Fulks' percent-from-extreme reversal stops (TradeStation "Other How To" section, inputs BZ 2 / SZ 1) and
//! James Taylor's variation of Bryan Strain's TFS trend-following system (TradeStation section, S&C June 2000), with
//! Strain's original cross logic (commented out in the same code) as an option. Held: Party Girl I Optimized (WealthLab
//! auto-stop semantics not defined in the file). Other rows of the file are handled by the overnight-1029 queue.
//! Source markets: not stated (generic TradeStation code); indices, ETFs and stocks here are a declared application.
use crate::connors::{sma, Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};
/// Admitted ids (authors/jennings/admission/djcs_ref.py (header)).
pub const REVIEWED: &[&str] = &["djcs_pct_reversal", "djcs_tfs"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const NOTE_P: &str = "[Declared readings] Always in after the first fill, stop and reverse. HHigh = the highest high and LLow = the lowest low since the position last changed (both restart at the high and low of the bar where it changed; the code's zero start value is read as the first bar of the test window, where the run starts flat). While long or flat: sell stop for the next bar at HHigh less SZ%; while short or flat: buy stop at LLow plus BZ%. A stop fills at its level or at the open if the day opens beyond it; the reverse position starts at the same price. While flat, a day that reaches both levels is skipped (order inside the day unknown). On the last bar a stop still closes the trade but opens nothing; a position still open is marked at the last close.";
const NOTE_T: &str = "[Declared readings] VolOsc = +volume on an up day (close above open), -volume on a down day, unchanged on an unchanged day (TradeStation variables keep their value); a missing volume blanks it until the next up or down day. Mid = (highest high + lowest low of the last 50 bars, today included) / 2. Taylor variation (default): close above Mid with the 7-day average of VolOsc above 0 buys (covering a short); close below Mid with the average below 0 sells short (closing a long). Strain's original (option 1): only a close crossing Mid acts - crossing above buys if the average is above 0, else only covers a short; crossing below mirrors. TradeStation 'Buy;' = next bar at the open; a signal on the last bar is not filled; a position still open is marked at the last close.";

pub fn catalogue() -> Vec<StrategyDef> {
    let d = |id: &'static str, name: &'static str, vol: bool, rules: &'static [&'static str], notes: &'static str| StrategyDef { id, name,
        group: "Jennings code snippets", session: 0, dir: "Long/Short", status: Status::Live, needs_volume: vol, needs_vix: false, needs_series: &[],
        book: "djcs", rules, notes, claim: None };
    vec![
        d("djcs_pct_reversal", "Percent-from-extreme reversal stops: reverse short 1% under the high since entry, long 2% over the low", false, &[
            "E|Bob Fulks: \"Inputs: BZ(2), SZ(1); ... HHigh = iff(High > HHigh or MP <> MP[1], High, HHigh); LLow = iff(Low < LLow or MP <> MP[1], Low, LLow);\" (TradeStation, 'Highest High' how-to)",
            "X|\"if MP >= 0 then Sell next bar at HHigh * (1 - 0.01 * SZ) stop; if MP <= 0 then Buy next bar at LLow * (1 + 0.01 * BZ) stop;\" - always in, stop and reverse"], NOTE_P),
        d("djcs_tfs", "TFS trend following: close beyond the 50-day mid-channel with the 7-day volume oscillator agreeing, stop and reverse", true, &[
            "F|\"If Close > Open then VolOsc = Volume; If Close < Open then VolOsc = - Volume;\"; mid-channel \"(Highest(High, 50) + Lowest(Low, 50)) / 2\" (TFS, James Taylor, variation of Bryan Strain, S&C June 2000)",
            "E|\"If Close > ((Highest(High, 50) + Lowest(Low, 50)) / 2) AND Average(VolOsc, 7) > 0 then begin Buy; ExitShort; End;\"",
            "X|\"If Close < ((Highest(High, 50) + Lowest(Low, 50)) / 2) AND Average(VolOsc, 7) < 0 then begin Sell; ExitLong; End;\" (\"This variation appears to be more profitable than the original\"; the original cross logic is an option)"], NOTE_T),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    match id {
        "djcs_pct_reversal" => Some(("djcs_pct_reversal", "Fulks percent reversal stops", "2% / 1%")),
        "djcs_tfs" => Some(("djcs_tfs", "Strain / Taylor TFS", "50-bar mid, 7-day volume oscillator")),
        _ => None,
    }
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    match id {
        "djcs_pct_reversal" => Some(vec![p("bz", "BZ: buy stop % above the lowest low (code 2)", 2., 0.1, 20., 0.1), p("sz", "SZ: sell stop % below the highest high (code 1)", 1., 0.1, 20., 0.1)]),
        "djcs_tfs" => Some(vec![p("n", "Channel bars (code 50)", 50., 2., 250., 1.), p("vo", "Volume oscillator average bars (code 7)", 7., 1., 50., 1.),
            p("orig", "Logic: 0 = Taylor variation (active code), 1 = Strain's original cross (commented out in the file)", 0., 0., 1., 1.)]),
        _ => None,
    }
}

fn mk(d: f64, ei: usize, ep: f64, xi: usize, xp: f64, why: &str, kind: &'static str, si: usize, open: bool) -> Trade {
    Trade { dir: d, legs: vec![Leg { i: ei, px: ep, w: 1. }], exit_i: xi, exit_px: xp, reason: why.into(), entry_kind: kind, signal_i: si, open, exits: vec![] }
}

fn pct_reversal(x: &Ctx, from: usize, to: usize, bz: f64, sz: f64) -> Vec<Trade> {
    let (o, h, l) = (&x.o, &x.h, &x.l);
    let (mut out, mut pos) = (vec![], None::<(f64, usize, f64)>);
    let (mut hh, mut ll) = (h[from], l[from]);
    for t in from + 1..=to {
        let mp = pos.map_or(0., |p| p.0);
        let (sell, buy) = (hh * (1. - 0.01 * sz), ll * (1. + 0.01 * bz));
        let (s_hit, b_hit) = (mp >= 0. && l[t] <= sell, mp <= 0. && h[t] >= buy);
        let new_dir = match (s_hit, b_hit) { (true, false) => -1., (false, true) => 1., _ => 0. };
        if new_dir != 0. {
            let px = if new_dir < 0. { o[t].min(sell) } else { o[t].max(buy) };
            if let Some((d, ei, ep)) = pos { out.push(mk(d, ei, ep, t, px, "reversal stop", "stop", t, false)); }
            pos = if t < to { Some((new_dir, t, px)) } else { None };
        }
        if new_dir != 0. { hh = h[t]; ll = l[t]; } else { hh = hh.max(h[t]); ll = ll.min(l[t]); }
    }
    if let Some((d, ei, ep)) = pos { out.push(mk(d, ei, ep, to, x.c[to], "open", "stop", to, true)); }
    out
}

fn tfs(x: &Ctx, from: usize, to: usize, n: usize, vo: usize, orig: bool) -> Vec<Trade> {
    let (o, h, l, c) = (&x.o, &x.h, &x.l, &x.c);
    let mut osc = vec![f64::NAN; c.len()];
    let mut last = 0.; // TradeStation Vars start at 0
    for i in 0..c.len() {
        let v = x.r[i].volume.filter(|v| *v > 0.).unwrap_or(f64::NAN);
        if c[i] > o[i] { last = v; } else if c[i] < o[i] { last = -v; }
        osc[i] = last;
    }
    let avg = sma(&osc, vo);
    let mid: Vec<f64> = (0..c.len()).map(|i| if i + 1 < n { f64::NAN } else {
        (h[i + 1 - n..=i].iter().cloned().fold(f64::MIN, f64::max) + l[i + 1 - n..=i].iter().cloned().fold(f64::MAX, f64::min)) / 2. }).collect();
    let (mut out, mut pos) = (vec![], None::<(f64, usize, f64)>);
    for t in from..to {
        let (up, dn) = if orig { (t > 0 && c[t] > mid[t] && c[t - 1] <= mid[t - 1], t > 0 && c[t] < mid[t] && c[t - 1] >= mid[t - 1]) } else { (c[t] > mid[t], c[t] < mid[t]) };
        let (go_l, go_s) = (up && avg[t] > 0., dn && avg[t] < 0.);
        // up without a positive oscillator only covers a short in the original logic (the variation needs both)
        let (x_s, x_l) = if orig { (up, dn) } else { (go_l, go_s) };
        let px = o[t + 1];
        if let Some((d, ei, ep)) = pos {
            if (d > 0. && x_l) || (d < 0. && x_s) {
                let rev = (d > 0. && go_s) || (d < 0. && go_l);
                out.push(mk(d, ei, ep, t + 1, px, if rev { "reverse" } else { "exit" }, "open", t, false)); pos = None;
            }
        }
        if pos.is_none() { if go_l { pos = Some((1., t + 1, px)); } else if go_s { pos = Some((-1., t + 1, px)); } }
    }
    if let Some((d, ei, ep)) = pos { out.push(mk(d, ei, ep, to, c[to], "open", "open", to, true)); }
    out
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.r.len() { return None; }
    let schema = params(id)?;
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).map_or(f64::NAN, |d| d.def));
    let u = |k: &str| g(k).round().max(1.) as usize;
    Some(match id {
        "djcs_pct_reversal" => pct_reversal(x, from, to, g("bz"), g("sz")),
        "djcs_tfs" => tfs(x, from, to, u("n"), u("vo"), g("orig").round() >= 1.),
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn ids_and_params() {
        assert!(reviewed("djcs_pct_reversal") && reviewed("djcs_tfs") && !reviewed("djcs_party_girl"));
        assert_eq!(catalogue().len(), 2);
        assert!(catalogue().iter().find(|d| d.id == "djcs_tfs").unwrap().needs_volume);
    }
}
