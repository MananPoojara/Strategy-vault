//! Anonymous excerpt "50 Bar.pdf" (3 pp, trading-software-collection.com repack): "Try a Simple Entry-Exit Strategy"
//! (50-day high/low midpoint) followed by six day-trading entry methods. Book id `fbs50`. Source: research drive
//! "USB HDD 2/tRADING KNO-2/50 Bar Trading Strategy/50 Bar.pdf". Nothing here is a complete mechanical rule.
use crate::connors::{Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "fbs50";

const MID: &str = "rules not fully defined: the text calls the 50-bar midpoint only \"one part of a simple trading strategy\"; 'price' (the close or a price during the bar), the order type and fill and whether 'sell' means exit or reverse to short are not stated (50 Bar Trading Strategy p.1)";
const MIDNOTE: &str = "[R1-1116, R4 #297/#339: adapted, 4 unstated details, UNVERIFIED] The text calls this \"one part of a simple trading strategy\" and prints no market, result or other part. Unstated in the book: (1) 'price': vault reading = the close; (2) the window: the 50 days end today (today's high and low are included; p.incl = 0 reads the 50 days before today); (3) the order and fill: vault reading = at the close of the day on which the close is above (below) the line, no stop or limit, and a close exactly on the line gives no signal; (4) whether 'sell' exits or reverses: vault reading = reverse to short (always in the market after the first signal). Declared application on index, ETF and stock daily closes.";
const DAY: &str = "rules not fully defined: entry ideas with no exit (methods 1-2 need tick counts, 3-4 are chart judgement - ABC correction, 'level of significance' - and 5-6 are commentary; 50 Bar Trading Strategy p.1); none states an exit";

pub fn catalogue() -> Vec<StrategyDef> {
    let d = |id, name, status, rules: &'static [&'static str]| StrategyDef { id, name, group: "50 Bar.pdf", session: 0, dir: "Long/Short", status,
        needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK, rules,
        notes: if id == "fbs50_midpoint" { MIDNOTE } else { "Close to kftsm_moving_channel (moving-channel midpoint) already in the vault; recorded so the source is marked as seen." }, claim: None };
    vec![
        d("fbs50_midpoint", "50-day high/low midpoint line", Status::Live, &[
            "F|p.1: \"Take the value of the highest high during the last 50 days and the value of the lowest low during the last 50 days. Average these two prices daily.\"",
            "E|p.1: \"When price rises above the line, buy.\"",
            "X|p.1: \"When price moves below the line, sell.\"",
        ]),
        d("fbs50_daytrade_entries", "Day-trading entry methods 1-6", Status::NotTestable(DAY), &[
            "E|1. \"the present bar's high is less than the low of two bars ago. It is a good selling entry\" (buy: low above the high two bars ago)",
            "E|2. \"Upticks are greater than Downticks by a factor of say 175% or 150%, perhaps 125%, the close is greater than the open and the close is greater than the previous close\"",
            "E|3. ABC correction; 4. three-bar high/low 'if we have achieved a mathematical level of significance'; 5. news and gaps; 6. sharp trending market",
        ]),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    if id == "fbs50_midpoint" { return Some(("fbs50_midpoint", "50-day high/low midpoint line", "")); }
    id.starts_with("fbs50_").then_some(("fbs50_50bar", "50 Bar Trading Strategy", "Not testable"))
}
pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    (id == "fbs50_midpoint").then(|| vec![ParamDef { key: "n", label: "Window (days)", def: 50., min: 5., max: 250., step: 1. },
        ParamDef { key: "incl", label: "1 = the window includes today (vault reading), 0 = the n days before today", def: 1., min: 0., max: 1., step: 1. }])
}
pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if id != "fbs50_midpoint" || from > to || to >= x.r.len() { return None; }
    for (k, v) in p { let d = params(id)?.into_iter().find(|d| d.key == k)?; if !v.is_finite() || *v < d.min || *v > d.max || v.fract() != 0. { return None; } }
    let n = p.get("n").map_or(50., |v| *v) as usize;
    let incl = p.get("incl").map_or(1., |v| *v) == 1.;
    let (h, l, c) = (&x.h, &x.l, &x.c);
    let mut out: Vec<Trade> = vec![];
    let mut cur: Option<Trade> = None;
    for i in from..=to {
        if i < n { continue; } // n full days before (or ending at) today: the same first bar for both readings
        let w = if incl { (i + 1 - n)..(i + 1) } else { (i - n)..i };
        let hh = h[w.clone()].iter().copied().fold(f64::NEG_INFINITY, f64::max);
        let ll = l[w].iter().copied().fold(f64::INFINITY, f64::min);
        let mid = (hh + ll) / 2.;
        let dir = if c[i] > mid { 1. } else if c[i] < mid { -1. } else { continue };
        if cur.as_ref().map_or(false, |t| t.dir == dir) { continue; }
        if let Some(mut t) = cur.take() { t.exit_i = i; t.exit_px = c[i]; t.signal_i = i; t.reason = "close crossed the 50-day midpoint".into(); out.push(t); }
        if i < to { cur = Some(Trade { dir, legs: vec![Leg { i, px: c[i], w: 1. }], exit_i: i, exit_px: c[i], reason: String::new(), entry_kind: "close", signal_i: i, open: false, exits: vec![] }); }
    }
    if let Some(mut t) = cur.take() { t.exit_i = to; t.exit_px = c[to]; t.signal_i = to; t.reason = "open".into(); t.open = true; out.push(t); }
    Some(out)
}

pub const ADMISSION_R1: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book == BOOK && def.id == "fbs50_midpoint",
    label: "The excerpt names no market; daily bars of indices, ETFs and stocks, US and India, are a declared application. Adapted: 4 unstated details (price, window, fill, sell = reverse), labelled on the card.",
    basis: "50 Bar.pdf p.1 'Try a Simple Entry-Exit Strategy'; R4 #297/#339; independent check authors/relax-339/R1/w/r1_ref.py",
};

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn only_the_midpoint_line_is_live() {
        assert!(super::catalogue().iter().all(|s| (s.id == "fbs50_midpoint") == matches!(s.status, crate::connors::Status::Live) && super::family_of(s.id).is_some()));
        assert!(crate::books::ADMISSIONS.iter().any(|a| (a.reviewed)(&catalogue().into_iter().find(|d| d.id == "fbs50_midpoint").unwrap())));
    }
    #[test]
    fn midpoint_line_reverses_at_the_close_and_ends_open() {
        let r = super::super::sctraub::tests::bars(400);
        let x = Ctx::new(&r, &r, &[]);
        let t = run("fbs50_midpoint", &x, 0, r.len() - 1, &Params::new()).unwrap();
        assert!(t.len() > 3 && t.last().unwrap().open);
        assert!(t.windows(2).all(|w| w[1].legs[0].i == w[0].exit_i && w[1].dir == -w[0].dir));
        assert!(run("fbs50_midpoint", &x, 0, r.len() - 1, &Params::from([("n".to_string(), 49.5)])).is_none());
        assert!(run("fbs50_day", &x, 0, 5, &Params::new()).is_none());
    }
}
