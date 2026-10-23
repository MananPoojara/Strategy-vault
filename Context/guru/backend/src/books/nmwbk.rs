//! Jack D. Schwager, "The New Market Wizards" (1992), Gil Blake interview "The Master of Consistency"
//! (printed p.91-93). Source copy: Books/Jack D. Schwager/Jack Schwager - The New Market Wizards Conversations with
//! America's Top Traders.pdf. Reading (all interviews): claude-continuation-20261020/authors/schwager/agents/nmw/.
use crate::connors::{Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "nmwbk";

/// Rules admitted after the 2026-10-23 full reading (authors/schwager/admission/SOURCE-CONTRACT.md) and independent
/// chronological Python ledgers (0 mismatches, US and India). Indices, ETFs and stocks are a declared application.
pub const REVIEWED: &[&str] = &["nmwbk_penny"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const NOTE: &str = "Gil Blake traded a Fidelity municipal bond fund's daily NAV, switching to the cash fund on down days (1980-84); he credits the result to \"some sort of smoothing process operating in the NAV numbers\" and agrees that in the bond market itself switching on every daily change \"is a disastrous strategy\" (p.92). Indices, ETFs and stocks are a declared application of the printed rule, not his test; no fund NAVs, interest on cash or the reported 20% a year are claimed. One penny = any change in the close; an unchanged close keeps the position [declared reading]. Long or flat (\"during those periods we were in cash\"). Switches happen at the close of the signal day, as fund switches price at that day's NAV; p.fill = 1 fills at the next open [Not from the book]. Each run starts flat at the selected start date.";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef {
        id: "nmwbk_penny", name: "Gil Blake \"one penny\" fund-switching rule (long or cash)", group: "Gil Blake · The Master of Consistency",
        session: 0, dir: "Long", status: Status::Live, needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK,
        rules: &[
            "E|\"Why couldn't I just switch out of the fund into cash when it started to go down and then switch back into the fund when it started to go back up?\" p.91",
            "E|\"it was the simplest approach that proved the best. We called it the 'one penny' rule ... approximately an 83 percent probability that any uptick or downtick day would be followed by a day with a price move in the same direction\" p.92",
            "X|Switch to cash on a down day: \"If it took only a one-day change in the direction of the NAV value in order for you to get a signal ... it only worked out to about twenty or thirty times per year\" p.93; \"during those periods we were in cash\" p.92",
        ],
        notes: NOTE, claim: None,
    }]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    (id == "nmwbk_penny").then_some(("nmwbk", "New Market Wizards rules", "Gil Blake one-penny switch"))
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    (id == "nmwbk_penny").then(|| vec![ParamDef { key: "fill", label: "Fill: 0 = signal close (book), 1 = next open [Not from the book]", def: 0.0, min: 0.0, max: 1.0, step: 1.0 }])
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    params(id)?;
    if from > to || to >= x.r.len() || p.iter().any(|(k, v)| k != "fill" || !(*v == 0.0 || *v == 1.0)) { return None; }
    let next_open = p.get("fill") == Some(&1.0);
    let mut out = vec![];
    let mut cur: Option<Trade> = None;
    for i in from.max(1)..=to {
        let up = x.c[i] > x.c[i - 1];
        let down = x.c[i] < x.c[i - 1];
        if !(if cur.is_some() { down } else { up }) { continue; }
        let (j, px) = if next_open { if i == to { break; } (i + 1, x.o[i + 1]) } else { (i, x.c[i]) };
        match cur.take() {
            Some(mut t) => { t.exit_i = j; t.exit_px = px; t.signal_i = i; t.reason = "down day".into(); out.push(t); }
            None => cur = Some(Trade { dir: 1.0, legs: vec![Leg { i: j, px, w: 1.0 }], exit_i: j, exit_px: px, reason: String::new(),
                entry_kind: if next_open { "open" } else { "close" }, signal_i: i, open: false, exits: vec![] }),
        }
    }
    if let Some(mut t) = cur { t.exit_i = to; t.exit_px = x.c[to]; t.signal_i = to; t.reason = "open".into(); t.open = true; out.push(t); }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;
    #[test]
    fn long_after_an_up_day_cash_after_a_down_day() {
        let r: Vec<Row> = [10.0, 10.0, 10.1, 10.2, 10.2, 10.1, 10.0, 10.3].iter().enumerate()
            .map(|(i, &c)| Row { ticker: "T".into(), date: format!("2001-01-{:02}", i + 1), open: c, high: c, low: c, close: c, volume: None }).collect();
        let x = Ctx::new(&r, &r, &[]);
        let t = run("nmwbk_penny", &x, 0, r.len() - 1, &Params::new()).unwrap();
        assert_eq!((t[0].legs[0].i, t[0].exit_i), (2, 5)); // flat close at 4 keeps the position
        assert_eq!((t[1].legs[0].i, t[1].open), (7, true));
    }
}
