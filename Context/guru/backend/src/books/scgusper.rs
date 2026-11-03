//! Gordon Gustafson, "Let Me Count The Days: Price Persistency", Technical Analysis of Stocks & Commodities V.20:1 (January 2002)
//! pp.44-47 (research share Trading Knowledge I/Stocks & Commodities DVD 11.26 (1982 - 2008) (traders.com)/VOLUMES/V20/C01/
//! 008PRI.pdf). Book id `scgusper`. Lane K16 (titles loop, 2 Nov 2026), to-code row 269; independent check
//! authors/titles-1030/K16w/k16_ref.py.
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};
use super::tselhs::Book;

pub const REVIEWED: &[&str] = &["scgusper_4_5_8"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const NOTE: &str = "[Declared readings] The printed EasyLanguage: Closeup / Closedn count consecutive higher / lower closes (reset to 0 otherwise). 'if Closedn = 4 then buy ... next bar at open'; 'if Closeup = 5 then exitlong next bar at market'; 'if barssinceentry = 8 then exitlong next bar at market' (8 bars after the entry bar). Long only, one position; a buy signal while long is ignored. 'All entries and exits were made at the open'; no stops ('There is no dollar stop, trailing stop, or volatility stop, and there should be'). The contract count (buynumb) is sizing only and not coded. Article (S&P 500 futures, 21 April 1982 - 29 June 2001, Figure 6): 102 trades, 76.5% profitable, profit factor 3.49. Runs on indices, ETFs and stocks, US and India, are a declared application.";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef { id: "scgusper_4_5_8", name: "Gustafson price persistency: buy after 4 lower closes, out after 5 higher closes or 8 bars, at the open", group: "S&C: Let Me Count The Days (Gustafson, V.20:1)", session: 0, dir: "Long", status: Status::Live,
        needs_volume: false, needs_vix: false, needs_series: &[], book: "scgusper", rules: &[
            "F|\"if c > c[1] then Closeup = Closeup + 1 else Closeup = 0; if c < c[1] then Closedn = Closedn + 1 else Closedn = 0;\" (V.20:1, PDF p.4)",
            "E|\"if Closedn = 4 then buy buynumb contracts next bar at open;\" (PDF p.4)",
            "X|\"if Closeup = 5 then exitlong next bar at market; ... if barssinceentry = 8 then exitlong next bar at market;\" (PDF p.4)"],
        notes: NOTE, claim: None }]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> { reviewed(id).then_some(("scgusper_4_5_8", "Gustafson price persistency (S&C)", "4 / 5 / 8")) }

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    reviewed(id).then(|| vec![p("dn", "Lower closes to buy", 4., 1., 10., 1.), p("up", "Higher closes to exit", 5., 1., 10., 1.), p("hold", "Bars since entry to exit", 8., 1., 30., 1.)])
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.r.len() { return None; }
    let g = |k: &str, d: f64| p.get(k).copied().unwrap_or(d).round().max(1.) as usize;
    let (nd, nu, hold) = (g("dn", 4.), g("up", 5.), g("hold", 8.));
    let (mut up, mut dn) = (0usize, 0usize);
    for t in 1..from { up = if x.c[t] > x.c[t - 1] { up + 1 } else { 0 }; dn = if x.c[t] < x.c[t - 1] { dn + 1 } else { 0 }; }
    let mut pos = Book::new();
    let mut pend: Option<f64> = None;
    for t in from.max(1)..=to {
        if let Some(d) = pend.take() { if d > 0.0 { pos.enter(1.0, t, x.o[t], "open") } else { pos.exit(t, x.o[t], "exit next bar") } }
        up = if x.c[t] > x.c[t - 1] { up + 1 } else { 0 };
        dn = if x.c[t] < x.c[t - 1] { dn + 1 } else { 0 };
        if t == to { break; }
        match pos.entry_bar() {
            None => if dn == nd { pend = Some(1.0) },
            Some(e) => if up == nu || t - e == hold { pend = Some(0.0) },
        }
    }
    Some(pos.done(to, x.c[to]))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn persistency() {
        let r = super::super::sctraub::tests::bars(3000);
        let x = Ctx::new(&r, &r, &[]);
        let t = run("scgusper_4_5_8", &x, 0, r.len() - 1, &Params::new()).unwrap();
        assert!(!t.is_empty() && t.iter().all(|z| z.open || z.exit_i - z.legs[0].i <= 9));
    }
}

// Instrument admission: S&C Gustafson 2026-11-02 (titles lane K16).
pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book == "scgusper" && reviewed(def.id),
    label: "Gordon Gustafson tested the counts on S&P 500 futures (1982-2001). Runs on indices, ETFs and stocks, US and India, are a declared application.",
    basis: "Technical Analysis of Stocks & Commodities V.20:1 pp.44-47 (Let Me Count The Days: Price Persistency), printed EasyLanguage; independent check authors/titles-1030/K16w/k16_ref.py",
};
