//! Mark Vakkur, "Seasonality And The S&P 500", Technical Analysis of Stocks & Commodities V.14:6 (June 1996) pp.241-243 (research
//! share Trading Knowledge I/Stocks & Commodities DVD 11.26 (1982 - 2008) (traders.com)/VOLUMES/V14/C06/SEASONA.pdf). Book id
//! `scvks`: the avoid-September and November-April switches as their own cards (lead decision 80). Lane K13 (titles loop,
//! 2 Nov 2026), to-code row 222; independent check authors/titles-1030/K13w/k13_ref.py.
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};
use super::sctraub::{month_of, period_ends};
use super::tselhs::Book;

pub const REVIEWED: &[&str] = &["scvks_ex_sep", "scvks_nov_apr"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const NOTE: &str = "[Declared readings] Vakkur's tests are month-to-month changes of the S&P 500 monthly close ('I was only concerned with the percentage change in the S&P 500 for each given month from close to close'), so each switch is made at a month-end close (the last trading day of the month): avoid September = sell at the August close, buy back at the September close; November-April = buy at the October close, sell at the April close. Cash earns nothing here (the article credits T-bills); no dividends. Article (S&P 500, January 1950 - December 1995, $10,000): buy and hold $372,388; avoid September $624,135; November-April $703,935; May-October $58,670. Lead decision 80: their own cards. The vault also shows this article's figures second-hand in Appel's book (masmt_sept_avoid, read there as buying back on the first trading day of October; masmt_vakkur_may_oct; masmt_vakkur_switch). Runs on indices, ETFs and stocks, US and India, are a declared application.";

pub fn catalogue() -> Vec<StrategyDef> {
    let d = |id, name, rules| StrategyDef { id, name, group: "S&C: Seasonality And The S&P 500 (Vakkur, V.14:6)", session: 0, dir: "Long", status: Status::Live,
        needs_volume: false, needs_vix: false, needs_series: &[], book: "scvks", rules, notes: NOTE, claim: None };
    vec![
        d("scvks_ex_sep", "Vakkur avoid September: in the market every month but September (out at the August close, back at the September close)", &[
            "E|\"If the investor were to create the most simple of systems in which he exited the S&P 500 every September and then reentered in October\" (V.14:6 p.242)",
            "X|\"switch out of stocks every September into a money market mutual fund\" (p.242); monthly closes"]),
        d("scvks_nov_apr", "Vakkur November-April: in from the October close to the April close, cash May-October", &[
            "E|\"suppose the investor was only invested in November through April, then switched into a money market mutual fund for the other six months of the year\" (p.242)",
            "X|Out at the April month-end close; back in at the October month-end close (monthly close-to-close changes)"]),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    let v = match id { "scvks_ex_sep" => "Avoid September", "scvks_nov_apr" => "November-April", _ => return None };
    Some(("scvks", "Vakkur S&P 500 seasonality (S&C)", v))
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> { reviewed(id).then(Vec::new) }

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, _p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.r.len() { return None; }
    let mut pos = Book::new();
    for i in period_ends(x, true) {
        if i < from || i > to { continue; }
        let m = month_of(&x.r[i].date);
        let want = if id == "scvks_ex_sep" { m != 8 } else { matches!(m, 10 | 11 | 12 | 1 | 2 | 3) };
        if want { pos.enter(1.0, i, x.c[i], "close"); } else { pos.exit(i, x.c[i], "season out"); }
    }
    Some(pos.done(to, x.c[to]))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn seasonal_switches() {
        let r = super::super::sctraub::tests::bars(3000);
        let x = Ctx::new(&r, &r, &[]);
        for id in REVIEWED {
            let t = run(id, &x, 0, r.len() - 1, &Params::new()).unwrap();
            assert!(t.len() > 5, "{id}");
            for z in t.iter().filter(|z| !z.open) { assert_eq!(month_of(&r[z.exit_i].date), if *id == "scvks_ex_sep" { 8 } else { 4 }); }
        }
    }
}

// Instrument admission: S&C Vakkur seasonality 2026-11-02 (titles lane K13).
pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book == "scvks" && reviewed(def.id),
    label: "Mark Vakkur tested the switches on S&P 500 monthly closes (1950-1995). Runs on indices, ETFs and stocks, US and India, are a declared application.",
    basis: "Technical Analysis of Stocks & Commodities V.14:6 pp.241-243 (Seasonality And The S&P 500); independent check authors/titles-1030/K13w/k13_ref.py",
};
