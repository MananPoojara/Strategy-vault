//! Martin J. Pring, "Candlesticks Explained" (McGraw-Hill / pring.com 2002; scan, 239 pdf pages; printed page = pdf
//! page - 10). Book id `mpcse`. Reading: claude-continuation-20261020/authors/pring/agents/mpcse/.
//! Live: the three-line break reversal system (Ch.8). Renko and kagi need a brick / reversal size the book leaves to
//! the user; the candle patterns use judgement words and have no exit.
use crate::connors::{Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "mpcse";

/// Admitted after the 2026-10-23 reading and independent ledgers (authors/pring/admission/SOURCE-CONTRACT.md).
pub const REVIEWED: &[&str] = &["mpcse_3lb"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const NOTE_3LB: &str = "Closing prices only. Each new line starts at the previous line's closing level and ends at the new close (Chart 8-2: after the April 8 turnaround the 'low for the move' is the April 4 close). A new line of the same colour needs a close beyond the last line, 'however small' (strict). A reversal needs a close beyond the far end of the first line of the current series, or, once there are three or more lines of that colour, beyond the far end of the third-last line (p.131-133). Signals at the close of the turnaround day (p.135); the fill is that close. [Declared readings] The first line is drawn from the first close of the data file to the first different close; equal closes draw nothing; history before the selected start builds the lines and a run starts flat. Long and short (the book's buyer may be 'covering a short position', p.136); p.short = 0 trades long only. p.lines = 2 or 4 are the other breaks the book names (p.142). The book shows US stocks; indices, ETFs and stocks in the US and India are a declared application.";

pub fn catalogue() -> Vec<StrategyDef> {
    use Status::*;
    let nt = |id, name, group, why, rules| StrategyDef { id, name, group, session: 0, dir: "Long/Short", status: NotTestable(why), needs_volume: false, needs_vix: false,
        needs_series: &[], book: BOOK, rules, notes: "Read in full (authors/pring/agents/mpcse).", claim: None };
    vec![
        StrategyDef { id: "mpcse_3lb", name: "Three-line break: trade the change of line colour", group: "Ch.8 · Three-Line Break", session: 0, dir: "Long/Short", status: Live,
            needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK,
            rules: &[
                "F|\"the three-line break method only deals with closing prices\"; \"The next white line is drawn only when the closing high of this particular session is exceeded ... If the closing high is not exceeded, then nothing is drawn\" (p.129)",
                "F|\"A new series of black lines only begins when the closing price makes a new low for the move ... the closing price must develop under the first white line\"; with \"at least three consecutive lines of the same color ... the low for the move ... is defined as a closing price which occurs below the previous three white lines\" (p.131); up-reversal: \"only when the closing price exceeds the closing price of the three previous black lines\" (p.133)",
                "E|\"Under the three-line break system, buy and sell signals are triggered when the line changes color ... that is, at the close of the turnaround day\" (p.135)",
                "X|Reverse on the next change of colour; \"Some traders prefer to use a two- and others a four-line break. However, three is the most widely used\" (p.142)",
            ],
            notes: NOTE_3LB, claim: None },
        nt("mpcse_renko", "Renko brick colour change", "Ch.9 · Renko",
           "the book gives no brick size: it uses 2-point, $1, 3-point and 6-point bricks on different stocks and says 'the predetermined amounts for the bricks work the same way as the time span on a moving average' (p.159), so the rule is not complete without a size chosen by the user",
           &["E|\"Renko charts create buy and sell signals when a new white or black brick appears\" (p.156)"]),
        nt("mpcse_kagi", "Kagi yang/yin line change", "Ch.10 · Kagi",
           "the reversal amount is left to the user (1 point, 1-2%, 3%, 0.5%, 2.5%, 0.25% in the examples, p.162-180) and the worked example (p.162) and the rule text (p.163) disagree on when the line changes thickness",
           &["E|\"buy when the line goes thick, or changes to a yang, and sell when it changes to a yin\" (p.165)"]),
        nt("mpcse_candles", "Candlestick patterns (engulfing, hammer, stars, harami, windows ...)", "Ch.1-7, 11",
           "patterns need judgement ('strong', 'small', 'after a prolonged advance'); only the long body (3x the previous body, p.11) and the hammer shadow (2x the body, p.18) are measured, and no pattern has an exit ('rarely lasting more than 10 days', p.5)",
           &["E|Interpretive patterns only"]),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    Some(match id {
        "mpcse_3lb" => ("mpcse_3lb", "Pring three-line break", "3 lines"),
        _ if id.starts_with("mpcse_") => ("mpcse_nt", "Pring Japanese charts (not testable)", "-"),
        _ => return None,
    })
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    (id == "mpcse_3lb").then(|| vec![
        ParamDef { key: "lines", label: "Lines in the break (book: 3; 2 and 4 named)", def: 3., min: 2., max: 4., step: 1. },
        ParamDef { key: "short", label: "Also go short (1, default) / long only (0)", def: 1., min: 0., max: 1., step: 1. },
    ])
}

/// Colour of the three-line break chart after each bar (+1 white, -1 black, None before the first line).
pub fn line_colours(c: &[f64], n: usize) -> Vec<Option<i8>> {
    let mut out = vec![None; c.len()];
    if c.is_empty() { return out; }
    // lines of the current series: (start level, end level); colour of the series
    let mut series: Vec<(f64, f64)> = vec![];
    let mut colour: i8 = 0;
    let base = c[0];
    for i in 1..c.len() {
        let x = c[i];
        if colour == 0 {
            if x != base { colour = if x > base { 1 } else { -1 }; series.push((base, x)); }
        } else {
            let last = series.last().unwrap().1;
            let k = series.len().saturating_sub(n);
            // far end of the first line of the series, or of the last n lines once there are n or more
            let far = series[k..].iter().fold(series[k].0, |m, l| if colour > 0 { m.min(l.0) } else { m.max(l.0) });
            if (colour > 0 && x > last) || (colour < 0 && x < last) { series.push((last, x)); }
            else if (colour > 0 && x < far) || (colour < 0 && x > far) { colour = -colour; series = vec![(last, x)]; }
        }
        if colour != 0 { out[i] = Some(colour); }
    }
    out
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if id != "mpcse_3lb" { return None; }
    if from > to || to >= x.r.len() { return None; }
    let n = p.get("lines").copied().unwrap_or(3.).round().clamp(2., 4.) as usize;
    let long_only = p.get("short").copied().unwrap_or(1.) < 0.5;
    let st = line_colours(&x.c, n);
    let (mut out, mut cur, mut last) = (vec![], None::<Trade>, None::<i8>);
    for i in 0..=to {
        let s = st[i];
        let change = matches!((s, last), (Some(a), Some(b)) if a != b);
        if s.is_some() { last = s; }
        if i < from || !change { continue; }
        let sd = s.unwrap();
        if let Some(mut t) = cur.take() {
            t.exit_i = i; t.exit_px = x.c[i]; t.signal_i = i;
            t.reason = (if sd > 0 { "white turnaround line" } else { "black turnaround line" }).into();
            out.push(t);
        }
        if sd > 0 || !long_only {
            cur = Some(Trade { dir: sd as f64, legs: vec![Leg { i, px: x.c[i], w: 1. }], exit_i: i, exit_px: x.c[i], reason: String::new(),
                entry_kind: "close", signal_i: i, open: false, exits: vec![] });
        }
    }
    if let Some(mut t) = cur { t.exit_i = to; t.exit_px = x.c[to]; t.signal_i = to; t.reason = "open".into(); t.open = true; out.push(t); }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn three_line_break_by_hand() {
        // whites 10-11, 11-12, 12-13; 12.5 draws nothing (a break needs a close below 10, the far end of the three lines);
        // 9.9 black (13-9.9); 11.5 nothing (needs > 13); 13.1 white turnaround (9.9-13.1)
        let c = [10., 11., 12., 13., 12.5, 9.9, 11.5, 13.1, 11.0, 9.8];
        let s = line_colours(&c, 3);
        assert_eq!(s, vec![None, Some(1), Some(1), Some(1), Some(1), Some(-1), Some(-1), Some(1), Some(1), Some(-1)]);
        // after the turnaround the low for the move is 9.9 (the previous line's close): 11.0 draws nothing, 9.8 reverses
    }
    #[test]
    fn fewer_than_three_lines_reverse_at_the_first_line() {
        let c = [10., 11., 12., 10.5, 9.9];
        assert_eq!(line_colours(&c, 3), vec![None, Some(1), Some(1), Some(1), Some(-1)]);
    }
}
