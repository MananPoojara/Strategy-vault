//! Rudy Stefenel, "Anchored Momentum", Technical Analysis of Stocks & Commodities V.16:2 (February 1998) pp.89-98 (research share
//! Trading Knowledge I/Stocks & Commodities DVD 11.26 (1982 - 2008) (traders.com)/VOLUMES/V16/C02/014ANCH.pdf). Book id `scstam`.
//! Lane K16 (titles loop, 2 Nov 2026), to-code row 245, decision 73; independent check authors/titles-1030/K16w/k16_ref.py.
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};
use super::msws::sma;
use super::sctraub::{days, ema0};
use super::tselhs::Book;

pub const REVIEWED: &[&str] = &["scstam_hyst"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const NOTE: &str = "[Declared readings] The printed SuperCharts QuickEditor rules: AM = 100 x (XAverage(Close, EmaPer) / Average(Close, 2 x MomPer + 1) - 1) (0 if the average is not positive). Enter long when AM > Hyst/2; exit long when AM < -Hyst/2 AND DateToJulian(Date) - DateToJulian(EntryDate) > 29, i.e. more than 29 calendar days after the entry date (the fund's short-term trading fee window). The author's optimised values MomPer 10, EmaPer 7, Hyst 1.1 (Fidelity Select Biotechnology, 1 March 1993 - 1 September 1997) are the default (decision 73); Hyst 0 is the article's simple zero-cross system, an option. Orders fill at the next bar's open (QuickEditor market orders); XAverage seeded with the first close; long only, no commissions in the article. Article Figure 8 (Fidelity Select Biotechnology, out of sample 9/1/88-3/1/93, 1,481 shares): 16 trades, 69% profitable, total net profit $27,522.90, profit factor 8.35; in sample 70% profitable. Mutual funds are not held; runs on indices, ETFs and stocks, US and India, are a declared application.";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef { id: "scstam_hyst", name: "Stefenel anchored momentum: EMA(7) vs centred SMA(21) above +0.55% buys, below -0.55% sells after 29 days", group: "S&C: Anchored Momentum (Stefenel, V.16:2)", session: 0, dir: "Long", status: Status::Live,
        needs_volume: false, needs_vix: false, needs_series: &[], book: "scstam", rules: &[
            "E|\"Enter-long: IFF(Average(Close,(2*MomPer)+1)>0, 100*((XAverage(Close,EmaPer)/Average(Close,(2*MomPer)+1))-1),0)>Hyst/2\" (V.16:2, PDF p.7-8)",
            "X|\"Exit-long: IFF(...) <-Hyst/2 AND(DateToJulian(Date)-DateToJulian(EntryDate) ) >29\" (PDF p.8)",
            "F|\"net profit are MomPer = 10, EmaPer = 7 and Hyst = 1.1\" (PDF p.6)"],
        notes: NOTE, claim: None }]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    reviewed(id).then_some(("scstam_hyst", "Stefenel anchored momentum (S&C)", "10 / 7 / 1.1"))
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    reviewed(id).then(|| vec![p("mom", "MomPer (SMA = 2 x MomPer + 1)", 10., 2., 40., 1.), p("ema", "EmaPer", 7., 2., 40., 1.), p("hyst", "Hyst, % (0 = zero cross)", 1.1, 0., 5., 0.1),
        p("hold", "Minimum calendar days before an exit (> 29)", 29., 0., 90., 1.)])
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.r.len() { return None; }
    let g = |k: &str, d: f64| p.get(k).copied().unwrap_or(d);
    let (s, e) = (sma(&x.c, 2 * g("mom", 10.).round().max(1.) as usize + 1), ema0(&x.c, g("ema", 7.).round().max(1.) as usize));
    let (h, hold) = (g("hyst", 1.1) / 2.0, g("hold", 29.).round() as i64);
    let am: Vec<f64> = (0..x.c.len()).map(|i| if !s[i].is_finite() { f64::NAN } else if s[i] > 0.0 { 100.0 * (e[i] / s[i] - 1.0) } else { 0.0 }).collect();
    let mut pos = Book::new();
    let mut pend: Option<f64> = None;
    for t in from..=to {
        if let Some(d) = pend.take() { if d > 0.0 { pos.enter(1.0, t, x.o[t], "open") } else { pos.exit(t, x.o[t], "AM below -Hyst/2") } }
        if t == to { break; }
        match pos.entry_bar() {
            None => if am[t] > h { pend = Some(1.0) },
            Some(eb) => if am[t] < -h && days(&x.r[t].date) - days(&x.r[eb].date) > hold { pend = Some(0.0) },
        }
    }
    Some(pos.done(to, x.c[to]))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn anchored_momentum_holds_30_days() {
        let r = super::super::sctraub::tests::bars(3000);
        let x = Ctx::new(&r, &r, &[]);
        let t = run("scstam_hyst", &x, 0, r.len() - 1, &Params::new()).unwrap();
        assert!(t.len() > 3);
        for z in t.iter().filter(|z| !z.open) { assert!(days(&r[z.exit_i].date) - days(&r[z.legs[0].i].date) > 29); }
    }
}

// Instrument admission: S&C Stefenel anchored momentum 2026-11-02 (titles lane K16).
pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book == "scstam" && reviewed(def.id),
    label: "Rudy Stefenel tested anchored momentum on Fidelity Select Biotechnology fund prices (1988-1997). Runs on indices, ETFs and stocks, US and India, are a declared application.",
    basis: "Technical Analysis of Stocks & Commodities V.16:2 pp.89-98 (Anchored Momentum), QuickEditor rules; independent check authors/titles-1030/K16w/k16_ref.py",
};
