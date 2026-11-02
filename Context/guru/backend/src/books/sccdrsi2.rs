//! Larry Connors and Ashton Dorkins, "Does The RSI Give You An Edge?", Technical Analysis of Stocks & Commodities V.25:11 (November
//! 2007) pp.48-52 (research share Trading Knowledge I/Stocks & Commodities DVD 11.26 (1982 - 2008) (traders.com)/VOLUMES/V25/C11/
//! 214CONN.pdf). Book id `sccdrsi2`. Lane K16 (titles loop, 2 Nov 2026), to-code row 289; independent check
//! authors/titles-1030/K16w/k16_ref.py.
use crate::connors::{rsi, Ctx, ParamDef, Params, Status, StrategyDef, Trade};
use super::tselhs::Book;

pub const REVIEWED: &[&str] = &["sccdrsi2_below2"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const NOTE: &str = "[Declared readings] The article's study: when the 2-period RSI (Wilder) closes below 2, buy at the next day's open and exit at the close five trading days later (the entry day counts as day 1, so the exit is the close four bars after the entry bar). One position at a time. Option side = -1 tests the short side the article also measured (RSI(2) above 98, short at the next open, cover five days later). The study's universe filters (price above $5, 100-day average volume above 250,000 shares) select stocks and are not applied. Article (all US stocks 1/1/1995-12/31/2006, about 8 million trades): RSI(2) < 2 one-week average +0.88% against +0.25% for all stocks; RSI(2) > 98 -0.17%. The vault's Connors book cards with RSI(2) use other filters and exits. Stocks run as the study intends; indices and ETFs, US and India, are a declared application.";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef { id: "sccdrsi2_below2", name: "Connors & Dorkins RSI(2) below 2: buy the next open, sell at the close five trading days later", group: "S&C: Does The RSI Give You An Edge? (Connors, V.25:11)", session: 0, dir: "Long", status: Status::Live,
        needs_volume: false, needs_vix: false, needs_series: &[], book: "sccdrsi2", rules: &[
            "E|2-period RSI below 2: buy at the open of the next day (V.25:11, PDF p.2, chart example: 'first green arrow')",
            "X|Exit at the close five trading days later ('second green arrow'); one-week average +0.88% vs +0.25% (PDF p.2-3)"],
        notes: NOTE, claim: None }]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> { reviewed(id).then_some(("sccdrsi2_below2", "Connors & Dorkins RSI(2) study (S&C)", "RSI(2) < 2, 5 days")) }

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    reviewed(id).then(|| vec![p("side", "1 = long below 2 (article); -1 = short above 98", 1., -1., 1., 2.), p("days", "Trading days held (entry day = 1)", 5., 1., 20., 1.)])
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.r.len() { return None; }
    let r = rsi(&x.c, 2);
    let short = p.get("side").copied().unwrap_or(1.) < 0.;
    let days = p.get("days").copied().unwrap_or(5.).round().max(1.) as usize;
    let mut pos = Book::new();
    let mut pend = false;
    for t in from..=to {
        if pend { pos.enter(if short { -1.0 } else { 1.0 }, t, x.o[t], "open"); pend = false; }
        if let Some(e) = pos.entry_bar() { if t - e + 1 == days { pos.exit(t, x.c[t], "close day 5"); } }
        if t < to && pos.side() == 0.0 && (if short { r[t] > 98.0 } else { r[t] < 2.0 }) { pend = true; }
    }
    Some(pos.done(to, x.c[to]))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn five_day_hold() {
        let r = super::super::sctraub::tests::bars(3000);
        let x = Ctx::new(&r, &r, &[]);
        assert!(run("sccdrsi2_below2", &x, 0, r.len() - 1, &Params::new()).unwrap().iter().all(|z| z.open || z.exit_i - z.legs[0].i == 4));
    }
}

// Instrument admission: S&C Connors RSI(2) study 2026-11-02 (titles lane K16).
pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book == "sccdrsi2" && reviewed(def.id),
    label: "Connors and Dorkins measured RSI(2) extremes on all US stocks (1995-2006). Runs on indices, ETFs and stocks, US and India, are a declared application.",
    basis: "Technical Analysis of Stocks & Commodities V.25:11 pp.48-52 (Does The RSI Give You An Edge?); independent check authors/titles-1030/K16w/k16_ref.py",
};
