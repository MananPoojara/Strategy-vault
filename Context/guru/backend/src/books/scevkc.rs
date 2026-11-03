//! Stuart Evens, "Keltner Channel", Technical Analysis of Stocks & Commodities V.17:12 (December 1999) pp.533-538 (research share
//! Trading Knowledge I/Stocks & Commodities DVD 11.26 (1982 - 2008) (traders.com)/VOLUMES/V17/C12/095KELT.pdf). Book id `scevkc`:
//! the four Keltner channel methods A-D. Lane K13 (titles loop, 2 Nov 2026), to-code row 205; independent check
//! authors/titles-1030/K13w/k13_ref.py.
use crate::connors::{rsi, Ctx, ParamDef, Params, Status, StrategyDef, Trade};
use super::msws::sma;
use super::tselhs::Book;

pub const REVIEWED: &[&str] = &["scevkc_a", "scevkc_b", "scevkc_c", "scevkc_d"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const NOTE: &str = "[Declared readings] Keltner's channel as printed: middle = 10-day SMA of (H+L+C)/3, band = 10-day SMA of (H-L), upper/lower = middle +/- band; RSI(14) is Wilder's. As in the article's MetaStock tests: 'Trades were entered at the open on the day following the close that first gave the signal' and 'closed at the closing price on the day that first gave the exit signal'. Methods A and B are stop and reverse (the opposite signal closes at that close and the new side opens at the next open); C and D can be flat. No stops. Article (23 stocks, 4 January 1993 - 21 September 1999, points): Keltner A 7 of 23 profitable, -609.5; B 13 of 23, -61.8; C 5 of 23, -645.5; D 8 of 23, +1.0 (buy and hold +865.9). Other vault cards using Keltner bands are different rules and sources. Runs on indices, ETFs and stocks, US and India, are a declared application.";

pub fn catalogue() -> Vec<StrategyDef> {
    let d = |id, name, rules| StrategyDef { id, name, group: "S&C: Keltner Channel (Evens, V.17:12)", session: 0, dir: "Long/Short", status: Status::Live,
        needs_volume: false, needs_vix: false, needs_series: &[], book: "scevkc", rules, notes: NOTE, claim: None };
    vec![
        d("scevkc_a", "Keltner method A: long on a close above the upper channel, short on a close below the lower (stop and reverse)", &[
            "F|\"AvPrice=(C+H+L)/3 Middle MA= 10-day SMA(AvPrice) ... Band MA = 10-day SMA(H-L) Upper band = Middle MA+ Band MA Lower band = Middle MA - Band MA\" (V.17:12 p.535)",
            "E|\"Method A Long: Close above upper band Short: Close below lower band\" (p.536)", "X|Stop and reverse: \"Methods A and B are stop and reverse and always in the market\" (p.536)"]),
        d("scevkc_b", "Keltner method B: long on a close below the lower channel, short on a close above the upper (stop and reverse)", &[
            "E|\"Method B Long: Close below lower band Short: Close above upper band\" (p.536)", "X|Stop and reverse (p.536)"]),
        d("scevkc_c", "Keltner method C: Keltner breakout confirmed by RSI(14) on the same side of 50; out on the opposite band or RSI", &[
            "E|\"Method C Long: Close above upper band and RSI(14) > 50 ... Short: Close under lower band and RSI(14) < 50\" (p.536)",
            "X|\"Exit long: Close below lower band or RSI(14) < 50 ... Exit short: Close above upper band or RSI(14) > 50\" (p.536)"]),
        d("scevkc_d", "Keltner method D: fade the channel with RSI(14) on the other side of 50; out on the opposite band or RSI", &[
            "E|\"Method D Long: Close below lower band and RSI(14) >50 ... Short: Close above upper band and RSI(14) <50\" (p.536)",
            "X|\"Exit long: Close above upper band or RSI(14) < 50 ... Exit short: Close below lower band or RSI(14) > 50\" (p.536)"]),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    let v = match id { "scevkc_a" => "Method A", "scevkc_b" => "Method B", "scevkc_c" => "Method C (RSI)", "scevkc_d" => "Method D (RSI)", _ => return None };
    Some(("scevkc", "Evens Keltner channel methods (S&C)", v))
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> { reviewed(id).then(Vec::new) }

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, _p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.r.len() { return None; }
    let n = x.c.len();
    let mid = sma(&(0..n).map(|i| (x.h[i] + x.l[i] + x.c[i]) / 3.0).collect::<Vec<_>>(), 10);
    let band = sma(&(0..n).map(|i| x.h[i] - x.l[i]).collect::<Vec<_>>(), 10);
    let r = rsi(&x.c, 14);
    let m = id.as_bytes()[id.len() - 1];
    let mut pos = Book::new();
    let mut pend: Option<f64> = None;
    for t in from..=to {
        if let Some(d) = pend.take() { pos.enter(d, t, x.o[t], "open"); }
        let (up, lo) = (mid[t] + band[t], mid[t] - band[t]);
        if !(up.is_finite() && r[t].is_finite()) { continue; }
        let (c, q) = (x.c[t], r[t]);
        let (le, se, xl, xs) = match m {
            b'a' => (c > up, c < lo, c < lo, c > up),
            b'b' => (c < lo, c > up, c > up, c < lo),
            b'c' => (c > up && q > 50.0, c < lo && q < 50.0, c < lo || q < 50.0, c > up || q > 50.0),
            _ => (c < lo && q > 50.0, c > up && q < 50.0, c > up || q < 50.0, c < lo || q > 50.0),
        };
        if pos.side() > 0.0 && xl { pos.exit(t, c, "exit signal"); } else if pos.side() < 0.0 && xs { pos.exit(t, c, "exit signal"); }
        if pos.side() == 0.0 && t < to { if le { pend = Some(1.0); } else if se { pend = Some(-1.0); } }
    }
    Some(pos.done(to, x.c[to]))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn keltner_methods() {
        let r = super::super::sctraub::tests::bars(3000);
        let x = Ctx::new(&r, &r, &[]);
        for id in REVIEWED {
            let t = run(id, &x, 0, r.len() - 1, &Params::new()).unwrap();
            assert!(!t.is_empty() && t.iter().all(|z| z.legs[0].px == x.o[z.legs[0].i]), "{id}");
            assert!(family_of(id).is_some());
        }
    }
}

// Instrument admission: S&C Evens Keltner 2026-11-02 (titles lane K13).
pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book == "scevkc" && reviewed(def.id),
    label: "Stuart Evens tested the four methods on 23 US stocks (1993-1999). Runs on indices, ETFs and other stocks, US and India, are a declared application.",
    basis: "Technical Analysis of Stocks & Commodities V.17:12 pp.533-538 (Keltner Channel); independent check authors/titles-1030/K13w/k13_ref.py",
};
