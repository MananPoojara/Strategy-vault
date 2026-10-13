//! F. H. "Chick" Goslin, Trading Day by Day: Winning the Zero-Sum Game of Futures (2002) + the
//! code files shipped with it (chickgoslin.com: "Code for TS", "Code for Amibroker").
//! Only the three-point "Status" is mechanical; the author says it is "not a straight mechanical
//! system" (p.62) because he anticipates the lines. This module tests the Status exactly as defined,
//! without the anticipation / margin-of-error / "solid trend" overrides (judgement).
//! Sources and verification: library/cgtdd/.
use crate::connors::{Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};

const BOOK: &str = "cgtdd";

pub fn catalogue() -> Vec<StrategyDef> {
    let make = |id, name, group, dir, status, rules, notes| StrategyDef {
        id, name, group, session: 0, dir, status, needs_volume: false, needs_vix: false,
        needs_series: &[], book: BOOK, rules, notes, claim: None,
    };
    vec![
        make("cgtdd_status", "Three-point Status (Bullish long / Neutral flat / Bearish short)", "The Three-Point System (p.60-62)", "Long/Short", Status::Live,
            &["F|Trend line: \"a ten-week ; or 50-day, moving average of the price\" (p.61; \"SMR uses a 49-day moving average\" p.25 = p.trend_n 49). \"+1 point if the line is currently pointing up, zero (0) points if sideways, and minus one (-1) point if down\" (p.61). Pointing up = today's value > yesterday's, down = <, equal = sideways (Amibroker code shipped with the book: long_rising = Long_term > Ref(Long_term,-1)).",
              "F|Short line SL: \"a two-, three-, or four-day moving average of the closing price ... subtracting around a nine-, ten-, or eleven-day moving average\" (p.26); exact form from the TradeStation code shipped with the book: Plot1 = AVERAGE(C,3) - AVERAGE(C,10). Points on direction only: +1 up, 0 sideways, -1 down (p.60-61).",
              "F|Middle line ML: \"a smoothed out (around three weeks, i.e., 15-, 16- or 17-day) moving average of their short-term oscillator\" (p.27); TS code: Plot2 = AVERAGE(AVERAGE(C,3)-AVERAGE(C,10),16). \"+1/2 if up, -1/2 if down ... +1/2 if above zero, -1/2 if below\" (p.61); at exactly zero: \"you can either assign zero points\" (p.61, first option taken).",
              "F|\"when the trend is up, only count the pluses, and when the trend is down, only count the minuses. (In other words, do NOT 'net out' the total points\" (p.60).",
              "E|\"If the trend is up and the point total is plus two (+2) or more, then the trend/momentum 'Status' is 'Bullish' ... long positions are justified.\" (p.61) \"If the trend is down and the point total is minus two (-2) points or more ... 'Bearish' and short positions are justified.\" (p.62)",
              "F|Confirming rule: \"at least one half (1/2) point from the intermediate-term momentum line (ML) in the direction you wish to trade\" (p.62). Default on; p.confirm=0 drops it [labelled option: the book allows an override only when the trend is 'particularly solid', which is judgement].",
              "X|\"anytime the points total is between plus one and a half and minus one and a half points ... Status is Neutral ... 'sidelines' is the indicated position.\" (p.62) Loss exit: \"if you are long or short with the Status and it changes ... from Bullish or Bearish back to Neutral ... simply get out of the trade at the market.\" (p.114)",
              "E|Order: Status is read after the close (p.63, p.109 \"If after the market has closed you decide that a position is fully justified\"); act at the next open (\"Pick up the phone and place the order, at the market\" p.109). Bullish->Bearish reverses at that open. [Book's first try at a limit 'at unchanged' for 15-30 minutes is intraday, not modelled]"],
            "Book trades futures (stock index, currency, metals, grains). Run on the index instead (instrument swap only). No published statistics in the book (chart commentaries only). Not modelled (judgement, p.33, 43, 52, 62): margin-of-error (10/5/2 days), anticipating the lines, the 'solid trend' confirming-rule override, percent-of-margin profit taking. Position: one unit, always the Status position; flat at window start until the first Status change after it."),
        make("cgtdd_margin_profit", "Percent-of-margin profit taking (50/100/150% of margin)", "Third Law: Taking profits (p.111-112)", "Long/Short",
            Status::NotTestable("needs historical exchange initial-margin per contract, and the rule is 'begin to look for a good reason' / 'need a legitimate reason NOT to' (judgement)"),
            &["X|\"When the profit on a trade reaches 50 percent of margin, begin to look for a good reason to take profits\" (p.111)", "X|\"When the profit on a trade reaches 100 percent of margin (or more), you need a legitimate reason NOT to take profits.\" (p.111)", "X|Pyramid out: partial profits around 50%, 100%, 150% of margin, rest until Status is on verge of Neutral (p.111-112)"],
            "Not coded: margin history is missing and the exits are discretionary."),
        make("cgtdd_line_profit", "SL-extreme profit taking", "Third Law: Taking profits (p.112)", "Long/Short",
            Status::NotTestable("'nears or reaches a recent extreme' / 'past few months' highs' is unquantified and 'you can make a case' is judgement"),
            &["X|\"Anytime the SL ... nears or reaches a recent 'extreme' ... then you can make a case to take some profits\" (p.112)", "X|Exception: solid concurrent mode (p.112)"], ""),
        make("cgtdd_stops", "Line-pattern / price-chart / size-of-loss stops", "Third Law: Taking losses (p.114-115)", "Long/Short",
            Status::NotTestable("stop placement is 'somewhat of an art'; size-of-loss stops need margin history and are 'look at the line pattern' judgement"),
            &["X|Price-chart stop at \"the point where the price 'should' not go\" (p.114)", "X|Close-only / late-in-the-day stops (p.114)", "X|50% / 100% of margin loss review (p.115)"], "The objective part (exit on Status back to Neutral) is inside cgtdd_status."),
        make("cgtdd_dips", "Buy dips / sell rallies in a clear trend", "The Price (p.59)", "Long/Short",
            Status::NotTestable("'clear uptrend' is not defined and the book gives no exit for this pattern"),
            &["E|\"When a market is in a clear uptrend, a 'dip' is defined as two days down or the low of the past three days\" (p.59)"], "No exit; never add one."),
        make("cgtdd_chart_patterns", "Flags, breakouts, false breakouts, island reversals, divergences", "The Price / Divergences (p.56-59)", "Long/Short",
            Status::NotTestable("discretionary chart reading: no numbers for flags, support/resistance, islands or divergence highs"),
            &["E|Bull/bear flags, breakouts, false breakouts, island reversals (p.58-59)", "E|Price/SL divergences (p.56)"], ""),
        make("cgtdd_spike", "The Spike rule", "The Spike Rule (p.122-131)", "Long/Short",
            Status::NotTestable("needs a judgement of 'extraordinary' and whether the spike has a news cause"),
            &["F|Trend = standard 50-day moving average using the intraday spike price (p.122)", "E|With-trend, unexplained spikes = continuation; against-trend, news spikes = aberration (p.123)"], "No entry/exit numbers."),
        make("cgtdd_long_term", "Long-term relative-strength method", "Long-Term Trading Method (p.120-121)", "Long/Short",
            Status::NotTestable("needs a futures universe ranked by eye ('eyeballing'), adds 'whenever the SL moves back close to the zero line', and 'all of this requires judgment' (p.121)"),
            &["E|Long the strongest (uptrending) markets, short the weakest, compared at least weekly on their ten-week moving averages (p.120)", "A|Add when the SL moves back close to zero (p.121)"], ""),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    Some(match id {
        "cgtdd_status" => ("cgtdd_status", "Goslin three-point Status", "Status position (SMA 50, 3-10, 16)"),
        "cgtdd_margin_profit" | "cgtdd_line_profit" | "cgtdd_stops" => ("cgtdd_money", "Goslin profit taking and stops", ""),
        "cgtdd_dips" | "cgtdd_chart_patterns" | "cgtdd_spike" | "cgtdd_long_term" => ("cgtdd_judgement", "Goslin chart reading", ""),
        _ => return None,
    })
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    match id {
        "cgtdd_status" => Some(vec![
            p("trend_n", "Trend line SMA days (book 50; SMR 49)", 50., 40., 60., 1.),
            p("sl_fast", "SL fast SMA (TS code 3; book 2-4)", 3., 2., 4., 1.),
            p("sl_slow", "SL slow SMA (TS code 10; book 9-11)", 10., 9., 11., 1.),
            p("ml_n", "ML SMA of SL (TS code 16; book 15-17)", 16., 15., 17., 1.),
            p("confirm", "ML confirming half point required (1 book, 0 off)", 1., 0., 1., 1.),
        ]),
        _ => None,
    }
}

fn sign(d: f64) -> f64 { if d > 0. { 1. } else if d < 0. { -1. } else { 0. } }

/// Plain window mean, summed oldest-to-newest (no running sum, so equal add/drop prices give exactly equal values).
fn wsma(v: &[f64], n: usize) -> Vec<f64> {
    (0..v.len()).map(|i| if i + 1 < n { f64::NAN } else { v[i + 1 - n..=i].iter().fold(0., |a, b| a + b) / n as f64 }).collect()
}

/// Status per bar: +1 Bullish, -1 Bearish, 0 Neutral (missing lines = Neutral).
/// A moving average points up when the value added is above the value dropped (book p.33: "compare the price
/// you are adding to the moving average ... to the price you are taking off"), which equals MA[i] > MA[i-1].
pub fn status(c: &[f64], trend_n: usize, f: usize, s: usize, m: usize, confirm: bool) -> Vec<i8> {
    let (a, b) = (wsma(c, f), wsma(c, s));
    let sl: Vec<f64> = a.iter().zip(&b).map(|(x, y)| x - y).collect();
    let ml = wsma(&sl, m);
    (0..c.len()).map(|i| {
        if i < trend_n || i + 1 < s + m || !ml[i].is_finite() { return 0; }
        let (t, md, sd) = (sign(c[i] - c[i - trend_n]), sign(sl[i] - sl[i - m]), sign(sl[i] - sl[i - 1]));
        if t == 0. { return 0; }
        let ml_pts = if md == t { 0.5 } else { 0. } + if ml[i] * t > 0. { 0.5 } else { 0. };
        let pts = 1. + ml_pts + if sd == t { 1. } else { 0. };
        if pts >= 2. && (!confirm || ml_pts >= 0.5) { t as i8 } else { 0 }
    }).collect()
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if id != "cgtdd_status" { return None; }
    let schema = params(id)?;
    if from > to || to >= x.r.len() { return None; }
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).unwrap().def).round().max(1.) as usize;
    let conf = p.get("confirm").copied().unwrap_or(1.) >= 0.5;
    let st = status(&x.c, g("trend_n"), g("sl_fast"), g("sl_slow"), g("ml_n"), conf);
    let mut out = vec![];
    let mut cur: Option<Trade> = None;
    for i in from..to {
        let want = st[i] as f64;
        let have = cur.as_ref().map_or(0., |t| t.dir);
        if want == have { continue; }
        if let Some(mut t) = cur.take() {
            t.exit_i = i + 1; t.exit_px = x.o[i + 1]; t.signal_i = i;
            t.reason = if want == 0. { "Status back to Neutral · next open".into() } else { "Status reversed · next open".into() };
            out.push(t);
        }
        if want != 0. {
            cur = Some(Trade { dir: want, legs: vec![Leg { i: i + 1, px: x.o[i + 1], w: 1. }], exit_i: i + 1, exit_px: x.o[i + 1],
                reason: String::new(), entry_kind: "open", signal_i: i, open: false, exits: vec![] });
        }
    }
    if let Some(mut t) = cur { t.exit_i = to; t.exit_px = x.c[to]; t.signal_i = to; t.reason = "open".into(); t.open = true; out.push(t); }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn points_follow_the_book() {
        // steady rise: trend up, SL flat (0) -> pts = 1 + ML(0 dir, 0 loc)... use an accelerating rise instead
        let c: Vec<f64> = (0..120).map(|i| 100. + (i as f64).powi(2) * 0.01).collect();
        let s = status(&c, 50, 3, 10, 16, true);
        assert_eq!(s[119], 1); // trend up, SL up, ML up and above zero = +3
        let d: Vec<f64> = c.iter().map(|v| 300. - v).collect();
        assert_eq!(status(&d, 50, 3, 10, 16, true)[119], -1);
        let flat = vec![100.; 120];
        assert_eq!(status(&flat, 50, 3, 10, 16, true)[119], 0); // trend sideways = Neutral
    }
    #[test]
    fn confirming_rule_needs_half_point_from_ml() {
        // trend up + SL up but ML down and below zero: 2 points, none from ML -> Neutral; without the rule -> Bullish
        let mut c: Vec<f64> = (0..100).map(|i| 100. + i as f64 * 0.5).collect();
        for k in 0..12 { c.push(149. - k as f64 * 1.5); }
        c.push(c[c.len() - 1] + 0.3);
        let n = c.len() - 1;
        assert!(c[n] > c[n - 50]);
        let a = status(&c, 50, 3, 10, 16, true);
        let b = status(&c, 50, 3, 10, 16, false);
        assert_eq!(a[n], 0);
        assert_eq!(b[n], 1);
    }
}
