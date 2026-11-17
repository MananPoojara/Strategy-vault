//! Ed Downs, "Looking At Momentum With TRIX" + sidebar "TRIX momentum in MetaStock", Technical Analysis of Stocks & Commodities
//! V.10:12 (Dec 1992) pp.537-540 (research share Trading Knowledge I/Stocks & Commodities DVD 11.26 (1982 - 2008) (traders.com)/
//! VOLUMES/V10/C12/LOOKING.PDF and SIDETRI.pdf). Book id `scdtrix`. Lane C5 (second pass, 17 Nov 2026); independent check
//! authors/convert-1117/C5/ref/c5_ref.py.
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};
use super::msws::sma;
use super::sctraub::{ema0, next_open};

pub const REVIEWED: &[&str] = &["scdtrix_x", "scdtrix_m"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const NOTE: &str = "[Declared readings] TRIX = the one-day percent change of the triple EMA of the close, as the article prints it for MetaStock Professional (E1 = EMA(close,p), E2 = EMA(E1,p), E3 = EMA(E2,p), TRIX = (E3 today - E3 yesterday) / E3 yesterday x 100); every EMA has alpha 2/(n+1) and starts at the first bar. The period is 14, the article's example (Figures 1-3, 'a 14-period TRIX'); the sidebar optimises it over 5-21 in steps of 4, and 5 to 21 are options. Both systems are the sidebar's MetaStock code and are always in the market after the first signal (the sidebar prints only 'Enter long' and 'Enter short', so each entry closes the opposite position). 'Classic' (scdtrix_x): long when yesterday's TRIX was below its 3-day exponential average and today's is at or above it; short when yesterday's was above and today's is at or below. 'Momentum' (scdtrix_m): m = TRIX today - TRIX yesterday; long when m < 0 and m is above its 3-day simple average of m; short when m > 0 and m is below it (the article's 'arbitrary signal level' is zero in the code). Orders are decided at a close and filled at the next open. Signals start 3 x period + 3 bars after the first bar so the triple average has settled (vault warm-up convention). The article gives no results table, only chart arrows on Alcoa, Apple, AIG and Avnet; the article's own text says the signals work better on some stocks than others and optimises the period per stock, which the vault does not. Runs on indices, ETFs and stocks, US and India, are a declared application.";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![
        StrategyDef { id: "scdtrix_x", name: "Downs TRIX(14) crosses its 3-day average: stop and reverse at the next open", group: "S&C: Momentum with TRIX (Downs, V.10:12)", session: 0, dir: "Long/Short", status: Status::Live,
            needs_volume: false, needs_vix: false, needs_series: &[], book: "scdtrix", rules: &[
                "F|\"E1 = EMA(d,p) ... E2 = EMA(E1,p) E3 = EMA(E2,p) TRIX = ((E3(today)-E3(yesterday))/E3(yesterday)))(100)\" (V.10:12 p.537)",
                "E|\"Trades are placed when the TRIX indicator changes direction, which is normally detected using a moving average crossover system. We buy when TRIX goes above its moving average to the upside and sell when it goes below the moving average.\" Sidebar: \"Enter long: when(ref(trix(OPT1),-1),<,mov(trix(OPT1),3,E)) AND when(trix(OPT1),>=,mov(trix(OPT1),3,E)); Enter short: when(ref(trix(OPT1),-1),>,mov(trix(OPT1),3,E)) AND when(trix(OPT1),<=,mov(trix(OPT1),3,E))\"",
                "X|The sidebar code has only \"Enter long\" and \"Enter short\": each entry closes the opposite position (stop and reverse). \"We buy when TRIX goes above its moving average to the upside and sell when it goes below the moving average.\"",
                "F|\"Classic TRIX trading system using a 14-period TRIX and three-period moving average.\" (Figure 2 caption)"],
            notes: NOTE, claim: None },
        StrategyDef { id: "scdtrix_m", name: "Downs TRIX momentum(14): reverse on peak reversals of TRIX's day-to-day change, at the next open", group: "S&C: Momentum with TRIX (Downs, V.10:12)", session: 0, dir: "Long/Short", status: Status::Live,
            needs_volume: false, needs_vix: false, needs_series: &[], book: "scdtrix", rules: &[
                "F|\"TRIX momentum = TRIXtoday - TRIXyesterday\" (p.538)",
                "E|\"a system test in which trades are taken on peak reversals of TRIX momentum above and below an arbitrary signal level.\" Sidebar: \"Enter long: when(trix(OPT1)-ref(trix(OPT1),-1),<,0) AND when(mov(trix(OPT1)-ref(trix(OPT1),-1),3,S),<,trix(OPT1)-ref(trix(OPT1),-1)); Enter short: when(trix(OPT1)-ref(trix(OPT1),-1),>,0) AND when(mov(trix(OPT1)-ref(trix(OPT1),-1),3,S),>,trix(OPT1)-ref(trix(OPT1),-1))\"",
                "X|The sidebar code has only \"Enter long\" and \"Enter short\": the short entry (TRIX momentum above zero and below its 3-day average) closes the long, and the long entry closes the short (stop and reverse).\"",
                "F|\"Figure 3: Momentum of 14-day TRIX\" (14-period example)"],
            notes: NOTE, claim: None },
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    reviewed(id).then_some(("scdtrix_x", "Downs TRIX (S&C)", "14-period TRIX"))
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    reviewed(id).then(|| vec![p("n", "TRIX period, days (example 14; sidebar optimises 5-21)", 14., 2., 40., 1.)])
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.r.len() { return None; }
    let n = p.get("n").copied().unwrap_or(14.).round().max(2.) as usize;
    let e3 = ema0(&ema0(&ema0(&x.c, n), n), n);
    let trix: Vec<f64> = (0..e3.len()).map(|i| if i == 0 { f64::NAN } else { (e3[i] - e3[i - 1]) / e3[i - 1] * 100.0 }).collect();
    let warm = 3 * n + 3;
    if id == "scdtrix_x" {
        let m = ema0(&trix, 3);
        Some(next_open(x, from, to, "reversed", |t, side| {
            if t < warm || !(trix[t - 1].is_finite() && m[t - 1].is_finite()) { return side; }
            if trix[t - 1] < m[t - 1] && trix[t] >= m[t] { 1.0 } else if trix[t - 1] > m[t - 1] && trix[t] <= m[t] { -1.0 } else { side }
        }))
    } else {
        let mo: Vec<f64> = (0..trix.len()).map(|i| if i == 0 { f64::NAN } else { trix[i] - trix[i - 1] }).collect();
        let s = sma(&mo, 3);
        Some(next_open(x, from, to, "reversed", |t, side| {
            if t < warm || !(mo[t].is_finite() && s[t].is_finite()) { return side; }
            if mo[t] < 0.0 && s[t] < mo[t] { 1.0 } else if mo[t] > 0.0 && s[t] > mo[t] { -1.0 } else { side }
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn trix_systems_reverse() {
        let r = super::super::sctraub::tests::bars(1500);
        let x = Ctx::new(&r, &r, &[]);
        for id in REVIEWED {
            let t = run(id, &x, 0, r.len() - 1, &Params::new()).unwrap();
            assert!(t.len() > 10, "{id}");
            for w in t.windows(2) { assert_eq!(w[0].exit_i, w[1].legs[0].i); }
        }
    }
}

// Instrument admission: S&C Downs TRIX 2026-11-17 (lane C5 second pass).
pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book == "scdtrix" && reviewed(def.id),
    label: "Ed Downs showed the TRIX systems as chart arrows on four US stocks (Alcoa, Apple, AIG, Avnet) and optimised the period per stock. Runs on indices, ETFs and stocks, US and India, with the 14-day example period are a declared application.",
    basis: "Technical Analysis of Stocks & Commodities V.10:12 pp.537-540 (Looking At Momentum With TRIX; sidebar TRIX momentum in MetaStock); independent check authors/convert-1117/C5/ref/c5_ref.py",
};
