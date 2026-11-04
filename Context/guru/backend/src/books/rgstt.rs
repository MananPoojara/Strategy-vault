//! Ruggiero Associates, "Stock Traders Toolbox" v1.0 (TradeStation add-on manual, 2001). Book id `rgstt`.
//! The EasyLanguage source ships only inside STT_TOOLS.ELA/ELS (Omega ORCMP archive whose members are
//! additionally encoded), so the systems are known only from the manual's descriptions. Sources: library/rgstt/.
//! Lane K52 (5 Nov 2026, decisions #223 (4), #226): rgs_bigstock coded with readings each tied to a manual page (page numbers of the
//! .doc as rendered by LibreOffice, 9 pages: authors/titles-1030/K52w/rugg/stt.pdf); independent check K52w/k52_ref.py.
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};
use super::msws::{simulate, Exec, Sig};

pub const BOOK: &str = "rgstt";
pub const REVIEWED: &[&str] = &["rgs_bigstock"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const BIG: &str = "[Declared readings, each from a manual page] TheStock = the traded close, TheMarket = the S&P 500 close as Data2 (p.3: 'a stock or mutual fund in data1 with another market index for example, the S&P500 in data2'); CStrength = (TheStock / TheMarket) x 100, RawStr = (CStrength - Lowest(CStrength,Window)) / (Highest(CStrength,Window) - Lowest(CStrength,Window)), STT_CompareStrenRaw = XAverage(RawStr,3) ('Then we will use the STT_CompStrenRaw user function in this system', p.8); Window = 30, the CompareStren default ('Default values, Window=30', p.3; the system's own Period default is not printed, so this is the labelled reading, p.win); buy when it crosses below .45, exit the long when it crosses above .55 (p.3; p.5: 'buys when comparative strength is below .45, and exit the position when it is above .55'), a cross = beyond the level today and not yesterday; one long position, one share (p.5); orders fill at the next open (p.6: the toolbox's systems use 'the next open function', and the buy is 'at the open'). XAverage = TradeStation's, seeded with the first value. Data2 on the traded symbol's dates (decision #223); a date the S&P 500 lacks carries its last close, runs with more than 5% such bars are withheld. Indicators start on the first bar on which both series exist. The manual tested large US stocks (MSFT, T, XOM, AXP, EK, 1/11/93-3/28/01, points); Indian stocks against NIFTY 50 are a labelled benchmark swap; indices and ETFs are a declared application. The S&P 500 and SPY (NIFTY 50) are not run against themselves (decision #225).";

pub fn catalogue() -> Vec<StrategyDef> {
    let mk = |id, name, status, rules: &'static [&'static str], notes| StrategyDef {
        id, name, group: "Stock Traders Toolbox · systems", session: 0, dir: "Long", status, needs_volume: false, needs_vix: false,
        needs_series: &[], book: BOOK, rules, notes, claim: None,
    };
    vec![
        StrategyDef { needs_series: &["BENCH.C"], ..mk("rgs_bigstock", "Inverse comparative strength (STT_BigStockNormal): buy weakness against the S&P 500, sell strength",
            Status::Live,
            &["F|CStrength=(TheStock/TheMarket)*100; RawStr=(CStrength-lowest(CStrength,Window))/(Highest(CStrength,Window)-Lowest(CStrength,Window)); STT_CompareStrenRaw=XAverage(RawStr,3) (manual p.8)",
              "E|\"for large cap stock, buy when this indicator crosses below .45\" (p.3); \"buys when comparative strength is below .45\" (STT_BigStockNormal, p.5)", "X|\"and exit the long position when it crosses above .55\" (p.3); fills at the next open (p.6)"],
            BIG) },
        mk("rgs_efficiency", "Efficiency system (STT_EfficencySystem)",
            Status::NotTestable("BuyLev, ExitLev and Period have no stated values and the crossing/order logic is not described"),
            &["F|Efficiency = Net change / sum(abs(all changes)) over Period (manual)", "E|Type 0: buy weakness (BuyLev<ExitLev); Type 1: buy strength", "X|Exit at ExitLev"], ""),
        mk("rgs_market_timer", "STT_MarketTimer (S&P500 + advancing/declining issues)",
            Status::NotTestable("the formula combining S&P500 with advance/decline issues is not given (only Btrig 1.05, Strig .995)"),
            &["F|Data2 S&P500, Data3 advancing issues, Data4 declining issues; outputs 1 bullish / -1 bearish (Btrig 1.05, Strig .995)"], ""),
        mk("rgs_indicators", "Indicators: PercentUpDays, RSMI, UpDownCount, Confused, volume spikes",
            Status::NotTestable("indicators with suggested reading levels, no entry/exit system"),
            &["F|PercentUpDays above 60% bullish, below 45% bearish; RSMI above .30 bullish, below -.30 bearish; UpDownCount below -5 (3-month window) bear market"], ""),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    id.starts_with("rgs_").then_some(("rgs_toolbox", "Stock Traders Toolbox", "Not testable"))
}
pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    reviewed(id).then(|| vec![ParamDef { key: "win", label: "Window (CompareStren default 30, p.3; the system's Period is not printed)", def: 30., min: 5., max: 120., step: 1. }])
}

/// STT_CompareStrenRaw(Window) of close `c` against the market `m` (NaN before `s0`).
pub fn compare_raw(c: &[f64], m: &[f64], s0: usize, w: usize) -> Vec<f64> {
    let cs: Vec<f64> = (0..c.len()).map(|i| if i < s0 { f64::NAN } else { c[i] / m[i] * 100. }).collect();
    let (hh, ll) = (super::mkits::hhv(&cs, w), super::mkits::llv(&cs, w));
    let raw: Vec<f64> = (0..c.len()).map(|i| (cs[i] - ll[i]) / (hh[i] - ll[i])).collect();
    super::tslrvs::xavg(&raw, 3)
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from > to || to >= x.r.len() { return None; }
    let Some(m) = x.series("BENCH.C") else { return Some(vec![]) };
    let w = p.get("win").copied().unwrap_or(30.).round().max(2.) as usize;
    let v = compare_raw(&x.c, m, crate::second::run_start(&[m], from), w);
    let n = v.len();
    let el = (0..n).map(|i| i >= 1 && v[i] < 0.45 && v[i - 1] >= 0.45).collect();
    let xl = (0..n).map(|i| i >= 1 && v[i] > 0.55 && v[i - 1] <= 0.55).collect();
    Some(simulate(x, from, to, &Sig { el, xl, es: vec![false; n], xs: vec![false; n] }, Exec { entry_open: true, exit_open: true }))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn compare_strength_is_scaled_over_the_window_and_smoothed() {
        let c = [1., 2., 3., 2., 1.];
        let v = compare_raw(&c, &[1.; 5], 1, 3);
        // CStrength from bar 1: 200, 300, 200, 100 -> RawStr (window 3) NaN, NaN, 0, 0 (bar 3: 200 in 200..300), bar 4: 0
        assert!(v[..3].iter().all(|z| z.is_nan()));
        assert_eq!((v[3], v[4]), (0., 0.));
        let v = compare_raw(&[3., 2., 1., 2., 3.], &[1.; 5], 0, 3);
        assert_eq!((v[2], v[3], v[4]), (0., 0.5, 0.75)); // RawStr 0, 1, 1; XAverage(3) seeded 0, then +0.5 x (1 - 0), +0.5 x (1 - 0.5)
    }
}

// Instrument admission: Ruggiero Stock Traders Toolbox (rgs_bigstock) 2026-11-05 (titles lane K52).
pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book == BOOK && reviewed(def.id),
    label: "Ruggiero tested large US stocks against the S&P 500 (1993-2001); other US stocks, indices and ETFs are a declared application, Indian instruments against NIFTY 50 a labelled benchmark swap.",
    basis: "Ruggiero Associates, Stock Traders Toolbox v1.0 manual (Ruggiero Stock Traders Toolbox.doc, rendered p.3, 5, 6, 8); second series per decision #223; independent check authors/titles-1030/K52w/k52_ref.py.",
};
