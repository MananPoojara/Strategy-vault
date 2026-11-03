//! Thomas Bulkowski, "Improving The Win-Loss Ratio With The RSI", Technical Analysis of Stocks & Commodities V.16:3 (March
//! 1998) pp.111-121 (research share Trading Knowledge I/Stocks & Commodities DVD 11.26 (1982 - 2008) (traders.com)/VOLUMES/V16/
//! C03/017WIN.pdf). Book id `scbulrsi`. Lane K16 (titles loop, 2 Nov 2026), to-code row 246; independent check
//! authors/titles-1030/K16w/k16_ref.py.
use crate::connors::{rsi, Ctx, ParamDef, Params, Status, StrategyDef, Trade};
use super::tselhs::Book;

pub const REVIEWED: &[&str] = &["scbulrsi_16", "scbulrsi_delay"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const NOTE: &str = "[Declared readings] Wilder's RSI with the author's optimised stock values: 16-day lookback, buy level 30, sell level 70 ('The best buy level turned out to be 30, the best sell level was 70 and the best lookback was 16 days'). Long only, one position. Fills at the close of the trading day after the signal: 'Paper trades were made using closing prices the day after the signal' (PDF p.4), as Figure 3 shows ('bought at the close the day after the buy signal ... sold on May 5, 1997 (the next trading day after the sell signal) at the close'); option fill = 1 fills at the signal day's close instead. Plain rule: buy when the RSI is below 30, sell when it is above 70. Delayed rule: buy when the RSI moves back above 30 (yesterday below, today above), sell when it falls back below 70. The author uses 16 days for stocks and 9 for other analysis; 14 (Wilder's default) and 28 ('also tested well') are options. Article (242 stocks, about two years, paper-traded seven months 11/96-6/97): plain 59 trades, 93% profitable (55/59), average gain 17%, average hold 82 days; delayed: average gain 15%, hold 81 days. The author tested stocks only; runs on indices and ETFs, US and India, are a declared application, stocks run as the method intends.";

pub fn catalogue() -> Vec<StrategyDef> {
    let d = |id, name, rules| StrategyDef { id, name, group: "S&C: Improving The Win-Loss Ratio With The RSI (Bulkowski, V.16:3)", session: 0, dir: "Long", status: Status::Live,
        needs_volume: false, needs_vix: false, needs_series: &[], book: "scbulrsi", rules, notes: NOTE, claim: None };
    vec![
        d("scbulrsi_16", "Bulkowski RSI(16): buy below 30, sell above 70 (long only, next day's close)", &[
            "E|\"Buy when RSI < 30, sell when RSI > 70; 16-day lookback\" (V.16:3, PDF p.3 table '30/70/16 values')",
            "X|Sell (go flat) when the RSI(16) is above 70 (same table), at the next day's close.", "F|\"Paper trades were made using closing prices the day after the signal.\" (PDF p.4; Figure 3 caption)", "F|\"The best buy level turned out to be 30, the best sell level was 70 and the best lookback was 16 days\" (PDF p.4)"]),
        d("scbulrsi_delay", "Bulkowski delayed RSI(16): buy when it moves back above 30, sell when it falls back below 70", &[
            "E|\"Delayed buying and selling: Buy when RSI moves above 30, sell when RSI falls below 70; 16-day lookback\" (PDF p.3 table)", "X|Sell when the RSI falls back below 70 (yesterday above, today below), at the next day's close (\"closing prices the day after the signal\", PDF p.4)."]),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    let v = match id { "scbulrsi_16" => "Below 30 / above 70", "scbulrsi_delay" => "Delayed (back through the levels)", _ => return None };
    Some(("scbulrsi", "Bulkowski RSI 30/70/16 (S&C)", v))
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    reviewed(id).then(|| vec![p("n", "RSI lookback, days (stocks 16; 14, 28 options)", 16., 2., 40., 1.), p("buy", "Buy level", 30., 5., 70., 1.), p("sell", "Sell level", 70., 30., 95., 1.),
        p("fill", "Fill: 0 = close of the day after the signal (article); 1 = the signal day's close", 0., 0., 1., 1.)])
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.r.len() { return None; }
    let g = |k: &str, d: f64| p.get(k).copied().unwrap_or(d);
    let r = rsi(&x.c, g("n", 16.).round().max(1.) as usize);
    let (bl, sl, del, now) = (g("buy", 30.), g("sell", 70.), id == "scbulrsi_delay", g("fill", 0.) > 0.5);
    let mut pos = Book::new();
    let mut pend = false; // an order for the next close: buy when flat, sell when long
    for t in from.max(1)..=to {
        if std::mem::take(&mut pend) { if pos.side() == 0.0 { pos.enter(1.0, t, x.c[t], "close"); } else { pos.exit(t, x.c[t], "RSI sell level"); } }
        let (a, b) = (r[t - 1], r[t]);
        if !b.is_finite() { continue; }
        let sig = if pos.side() == 0.0 { if del { a < bl && b > bl } else { b < bl } } else if del { a > sl && b < sl } else { b > sl };
        if !sig { continue; }
        if !now { pend = t < to; } else if pos.side() == 0.0 { pos.enter(1.0, t, x.c[t], "close"); } else { pos.exit(t, x.c[t], "RSI sell level"); }
    }
    Some(pos.done(to, x.c[to]))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rsi_levels() {
        let r = super::super::sctraub::tests::bars(3000);
        let x = Ctx::new(&r, &r, &[]);
        for id in REVIEWED {
            let t = run(id, &x, 0, r.len() - 1, &Params::new()).unwrap();
            assert!(!t.is_empty() && t.iter().all(|z| z.dir > 0.0 && z.legs[0].px == x.c[z.legs[0].i]), "{id}");
            // article fills: the close of the day after the signal, so the RSI condition held on the bar before the entry
            let r16 = rsi(&x.c, 16);
            assert!(t.iter().all(|z| { let i = z.legs[0].i; if *id == "scbulrsi_16" { r16[i - 1] < 30. } else { r16[i - 2] < 30. && r16[i - 1] > 30. } }), "{id}");
            let mut p = Params::new(); p.insert("fill".into(), 1.);
            assert!(run(id, &x, 0, r.len() - 1, &p).unwrap().iter().all(|z| { let i = z.legs[0].i; if *id == "scbulrsi_16" { r16[i] < 30. } else { r16[i - 1] < 30. && r16[i] > 30. } }), "{id} fill 1");
        }
    }
}

// Instrument admission: S&C Bulkowski RSI 2026-11-02 (titles lane K16).
pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book == "scbulrsi" && reviewed(def.id),
    label: "Thomas Bulkowski tested the RSI levels on 242 US stocks (about two years). Runs on indices and ETFs, US and India, are a declared application.",
    basis: "Technical Analysis of Stocks & Commodities V.16:3 pp.111-121 (Improving The Win-Loss Ratio With The RSI); independent check authors/titles-1030/K16w/k16_ref.py",
};
