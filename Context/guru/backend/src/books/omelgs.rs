//! Omega Research, "EasyLanguage for TradeStation" (TradeStation 2000i Getting Started; research share USB HDD 1/TradeStation/
//! _Knowleage/Easy Language Getting Started.pdf, byte-identical to ProSuite doc/eng_EL_Getting_Started.pdf; text authors/batch-e/E7w/
//! el_getting_started.txt). Book id `omelgs`. Ch.6 Example 6-4 'Key Reversal LE' with the library 'Key Reversal LX', a labelled
//! teaching example (R4 decisions 49, 50, 57). Ch.7's two-line average strategy (MA2 Entry / MA2 Exit, 9/18 averages, long only, this
//! bar on close) is the same rule as atm00_ma_9_18 and is not coded again. Lane K11 (titles loop, 1 Nov 2026), to-code rows 181-182;
//! independent check authors/titles-1030/K11w/k11_ref.py.
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};
use super::lc3::{hi, lo};
use super::tselhs::{stop_fill, Book};

/// Admitted ids (authors/titles-1030/K11w/k11_ref.py (header)).
pub const REVIEWED: &[&str] = &["omelgs_keyrev5"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const NOTE: &str = "[Declared readings] Example 6-4 'Key Reversal LE' (code printed, p.77): if Low < Lowest(Low,5)[1] (below the lowest low of the 5 bars before) and Close > Close[1] -> Buy next bar at market (the open). Exit: the TradeStation library 'Key Reversal LX', described in words only (p.77-78): it 'looks for the highest High over the last 5 bars and a lower Close than the previous bar. This is exactly the opposite condition to the entry', read as High > Highest(High,5)[1] and Close < Close[1]; then 'the Sell order will be placed on the next bar as a stop order using the Low price of the current bar minus 1 point' and 'only remains active for the designated bar'. The stop is placed only at a close where that condition is true (the entry bar's close included) and fills at the stop, or at the open on a gap below it. '1 point' = one minimum price tick (0.01 US, 0.05 NSE; lead decision 57); p.tick = 0 places the stop at the low itself. Long only, one position (a repeat entry signal while long is ignored), no other stop. Bar size not given: daily bars are a declared application. The book's ch.7 9/18 moving-average strategy and its $250 '$SL' stop example are the same rule as atm00_ma_9_18 (not coded twice; the dollar stop example is not offered). A teaching example from TradeStation's getting-started manual with no results (lead decision 49); the vault's standard costs (decision 50).";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef { id: "omelgs_keyrev5", name: "Key Reversal LE/LX: new 5-bar low with an up close buys at the next open; out on a stop 1 tick under the low of a 5-bar-high down-close bar", group: "EasyLanguage for TradeStation (Getting Started, ch.6)",
        session: 0, dir: "Long", status: Status::Live, needs_volume: false, needs_vix: false, needs_series: &[], book: "omelgs", rules: &[
            "E|\"Input: LowBarsBack( 5 ); if Low < Lowest(Low,LowsBarBack)[1] and Close > Close[1] then Buy(\\\"KeyR\\\") next bar at Market ;\" (Example 6-4, p.77)",
            "X|\"The Key Reversal LX contains a condition that looks for the highest High over the last 5 bars and a lower Close than the previous bar. ... The Sell order will be placed on the next bar as a stop order using the Low price of the current bar minus 1 point\" (p.77-78)"],
        notes: NOTE, claim: None }]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    (id == "omelgs_keyrev5").then_some(("omelgs_keyrev5", "EL Getting Started key reversal", "5-bar, LX stop"))
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    reviewed(id).then(|| vec![ParamDef { key: "tick", label: "LX stop offset under the low, ticks (1; 0 = none)", def: 1., min: 0., max: 10., step: 1. }])
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.r.len() { return None; }
    let (o, h, l, c) = (&x.o, &x.h, &x.l, &x.c);
    let off = p.get("tick").copied().unwrap_or(1.).round().max(0.) * x.market_tick();
    let mut pos = Book::new();
    let (mut buy, mut stop) = (false, f64::NAN);
    for i in from..=to {
        if std::mem::take(&mut buy) { pos.enter(1., i, o[i], "open"); }
        else if stop.is_finite() && pos.side() > 0. { if let Some(f) = stop_fill(x, i, -1., stop) { pos.exit(i, f, "Key Reversal LX stop"); } }
        stop = f64::NAN;
        if i == to || i < 5 { continue; }
        if pos.side() == 0. { buy = l[i] < lo(l, i - 5, i - 1) && c[i] > c[i - 1]; }
        else if h[i] > hi(h, i - 5, i - 1) && c[i] < c[i - 1] { stop = l[i] - off; }
    }
    Some(pos.done(to, c[to]))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;
    #[test]
    fn key_reversal_le_lx() {
        let r: Vec<Row> = (0..900).map(|i| { let f = i as f64; let c = 100. + 9. * (f / 17.).sin() + 4. * (f / 5.3).cos() + 0.02 * f;
            Row { ticker: "T".into(), date: format!("{:05}", i), open: c - 0.6 * (f / 2.1).sin(), high: c + 1.3 + 0.5 * (f / 2.9).cos(), low: c - 1.1 - 0.5 * (f / 3.7).sin(), close: c, volume: None } }).collect();
        let x = Ctx::new(&r, &r, &[]);
        let t = run("omelgs_keyrev5", &x, 0, r.len() - 1, &Params::new()).unwrap();
        assert!(t.len() > 3 && t.iter().all(|z| z.dir > 0.));
        for z in &t { assert_eq!(z.legs[0].px, x.o[z.legs[0].i]); if !z.open { assert!(z.exit_px <= x.l[z.exit_i - 1] - 0.01 + 1e-9); } }
        for w in t.windows(2) { assert!(w[1].legs[0].i > w[0].exit_i); }
        assert!(family_of("omelgs_keyrev5").is_some() && run("omelgs_x", &x, 0, 5, &Params::new()).is_none());
    }
}

// Instrument admission: Omega Research, EasyLanguage for TradeStation (Getting Started) 2026-11-01 (titles lane K11).
pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book == "omelgs" && reviewed(def.id),
    label: "A teaching example from Omega Research's TradeStation 2000i getting-started manual, with no instrument and no results. Runs on daily indices, ETFs and stocks, US and India, are a declared application.",
    basis: "EasyLanguage for TradeStation (Getting Started), ch.6 Example 6-4 and the Key Reversal LX description, PDF p.83 (printed p.77-78); independent check authors/titles-1030/K11w/k11_ref.py",
};
