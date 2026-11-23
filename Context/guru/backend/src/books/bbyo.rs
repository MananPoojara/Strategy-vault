//! Thomas Busby, "Winning the DayTrading Game" (Wiley 2005), ch.9 "Getting Down to Brass Tacks", index-fund rule (printed p.120-121; copy read in full from
//! `Trading Books/Thomas Busby -  Winning the DayTrading Game/Thomas Busby -  Winning the DayTrading Game.pdf`). Book id `bbyo`. Lane K4 (Q383, 23 Nov 2026).
//! Daily NSE indices (the book trades the S&P 500 and Nasdaq 100 index funds; declared R4 #340 adaptation). Independent check authors/code-1123/ref/bbyo_ref.py.
use super::rrsim::{g, o, pd, run_ts, sdef, A, P};
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};

const ID: &str = "bbyo_yearly_open";
const NOTE: &str = "[Declared adaptation, R4 #340] Busby's index-fund rule (ch.9): 'I begin each year by recording the opening price of the S&P 500 and Nasdaq 100 futures. I calculate a 2.5 percent deviation up and down from the opens. If either of these levels is achieved, I trade the index funds either long or short ... If the market goes against me ... I exit the position when the market goes 2.5 percent above [below] the yearly open. I do not reverse the position and re-enter the market. I wait until the beginning of the next quarter and begin the deviation process again. If the market is going in my favor, I evaluate at the beginning of the second quarter. I record the open of the second quarter and again calculate a 2.5 percent deviation above and below the open. If the market moves 2.5 percent against me, I get out.' The book trades index funds; the vault runs it on each of daily NIFTY 50 and NIFTYBANK separately (the book's 'which index is strongest' choice between two indices is not coded). [Declared readings] (1) the reference is the open of the first bar of the calendar year for the first quarter and the open of the first bar of each later calendar quarter (the book's 'open of the second quarter' extended to every quarter); in the first quarter of the data the first bar's open; (2) a 2.5% level touched during a bar is a stop order at that price (a gap through it fills at the open), the entry stop and the exit stop being the same two levels; (3) after an exit the rule waits for the next quarter (no re-entry the same quarter) and an open position is carried into the next quarter with the new bands. The book gives no result for the rule. One position.";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![sdef("bbyo", ID, "Busby yearly-open deviation: long / short at 2.5% above / below the quarter open, out at 2.5% against it, no re-entry until the next quarter", "Winning the DayTrading Game (ch.9 index rule)", "Long/Short", Status::Live, &[
        "F|\"I begin each year by recording the opening price of the S&P 500 and Nasdaq 100 Futures. I calculate a 2.5 percent deviation up and down from the opens.\" (second quarter: \"I record the open of the second quarter and again calculate a 2.5 percent deviation above and below the open\")",
        "E|\"If either of these levels is achieved, I trade the index funds either long or short, as indicated.\" (2.5% above the open = long, 2.5% below = short)",
        "X|\"If the market goes against me ... I exit the position when the market goes 2.5 percent above the yearly open. I do not reverse the position and re-enter the market. I wait until the beginning of the next quarter and begin the deviation process again.\"; \"If the market moves 2.5 percent against me, I get out.\"",
        ], NOTE)]
}
pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> { (id == ID).then_some((ID, "Busby yearly-open deviation", "2.5% bands")) }
pub fn params(id: &str) -> Option<Vec<ParamDef>> { (id == ID).then(|| vec![pd("dev", "deviation from the open (percent)", 2.5, 0.5, 10., 0.5)]) }
fn quarter(d: &str) -> i32 { let (y, m): (i32, i32) = (d[0..4].parse().unwrap_or(0), d[5..7].parse().unwrap_or(1)); y * 4 + (m - 1) / 3 }
pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if id != ID || from >= to || to >= x.r.len() { return None; }
    let dev = g(p, "dev", 2.5) / 100.0;
    let (open, n) = (&x.o, x.c.len());
    let (mut r, mut locked, mut prev) = (open[0], false, 0i8);
    Some(run_ts(x, from, to, |i, st, _now, next| {
        if st.pos == 0 && prev != 0 { locked = true; }
        prev = st.pos;
        if i + 1 >= n { return; }
        if quarter(&x.r[i + 1].date) != quarter(&x.r[i].date) { r = open[i + 1]; locked = false; }
        let (up, dn) = (r * (1.0 + dev), r * (1.0 - dev));
        match st.pos {
            0 if !locked => { next.push(o(A::Buy, P::Stop(up), "long at +2.5% of the open")); next.push(o(A::Sell, P::Stop(dn), "short at -2.5% of the open")); }
            1 => next.push(o(A::Xl, P::Stop(dn), "exit long at -2.5% of the open")),
            -1 => next.push(o(A::Xs, P::Stop(up), "exit short at +2.5% of the open")),
            _ => {}
        }
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn quarters_and_run() {
        assert_eq!(quarter("2026-03-31") + 1, quarter("2026-04-01"));
        assert_eq!(quarter("2025-12-31") + 1, quarter("2026-01-02"));
        let r = super::super::sctraub::tests::bars(1500);
        let x = Ctx::new(&r, &r, &[]);
        let t = run(ID, &x, 0, r.len() - 1, &Params::new()).unwrap();
        assert!(!t.is_empty());
        assert!(run("bbyo_x", &x, 0, 5, &Params::new()).is_none());
    }
}

// Instrument admission: Busby yearly-open rule 2026-11-23 (lane K4).
pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book == "bbyo" && def.id == "bbyo_yearly_open",
    label: "A rule for US index funds with no result shown; runs on daily NIFTY 50 and NIFTYBANK only (the two indices the independent reference covers; a declared application, R4 #340), not the author's test.",
    basis: "Busby, Winning the DayTrading Game ch.9 (index-fund rule); independent check authors/code-1123/ref/bbyo_ref.py",
};
