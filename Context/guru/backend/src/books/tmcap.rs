//! TradingMarkets / Connors Research, "Capital Appreciation Program" (TMCAP) slides, Sept 2005 (research share USB HDD 1/@TradingMarkets.com &
//! TradersLibrary.com/Capital Appreciation Program/tmcap.pdf, text layer; rules p.2-3 'TMCAP METHOD RULES' and 'TMCAP PORTFOLIO'). Book id `tmcap`.
//! Rules A-F as printed: A above the 200-day SMA; B the stock's 100-day HV at least 2 x the S&P 500 Cash Index's; C 10-period ADX 35 or more;
//! D "Yesterday's close must be the fifth consecutive day that the stock closed below its 2-period SMA"; E enter on the next day's open, "(A second
//! entry occurs if the stock closes under the 2-period SMA the next day)"; F exit the full position when the 2-period RSI closes above 75.
//! Lane K1 (23 Nov 2026, row 422 of titles-1030/to-code.tsv). Independent check authors/conv-1123/K1/tmcap_ref.py.
use crate::connors::{Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "tmcap";
pub const REVIEWED: &[&str] = &["tmcap_main"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const NOTE: &str = "[Declared adaptation, R4 #340] The slides run US stocks against the S&P 500 Cash Index (July 2005: 21 trades, 18 winners; the results are for the program's whole stock list and 10-slot portfolio, with 1 cent a share commission and T-bill interest on cash, so no figure is compared). On NSE stocks the benchmark is the NIFTY 50. A stock rule: it runs on actual stocks only (an index has no 'index HV twice the S&P 500'). The 10-slot account and 'highest historical volatility first' tie-break (portfolio page) need a stock universe, so a single-stock run takes every signal. [Declared readings, each an unstated choice] (1) 'the fifth consecutive day' is read as exactly the fifth close below the 2-day SMA, so the setup fires once per run of closes (not on the sixth or later); (2) rules A-C (above the 200-day SMA, HV 2x, ADX 35) are read on that same fifth day; (3) the second entry's 'next day' is the day after the setup day: if that close is also under the 2-day SMA a second unit is bought at the following open; (4) the second entry happens once only, at most two units; (5) an RSI exit on the day after the setup day takes precedence over the second entry; (6) a position is two equal units (half each); (7) the return is on the full two-unit allocation and an unfilled second unit is idle cash. Honest total: 7 card-specific choices. Engine-wide defaults / declared adaptations (labelled, not counted, R4 #404): strict comparisons following the printed word ('above' and 'below' on closes, close strictly above the 200-day SMA, strictly below the 2-day SMA, 'closes above 75' as RSI(2) strictly above 75); the whole position sold at the next open after the RSI(2) exit close (an option sells at that close); the vault HV(100) = sample standard deviation of 100 daily log returns for stock and benchmark alike; Wilder's RSI(2) and Wilder's DMI definition for ADX(10); simple averages of closes (built-ins); NIFTY 50 as the benchmark for NSE stocks (declared adaptation #340), a single-stock run takes every signal (no 10-slot account or highest-HV tie-break), an open trade marked at the window's last close, the data window.";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef { id: "tmcap_main", name: "TMCAP: above the 200-day, 100-day HV twice the market's, ADX(10) 35+, fifth close under the 2-day average, buy next open, out when RSI(2) > 75",
        group: "TradingMarkets Capital Appreciation Program", session: 0, dir: "Long", status: Status::Live, needs_volume: false, needs_vix: false, needs_series: &[], book: "tmcap",
        rules: &[
            "F|\"A. Stock must be above it's 200-day SMA.\" \"B. The stock's 100-day HV must be at least 2 x greater than the 100-day HV of the S&P 500 Cash Index.\" \"C. Stock's 10-period ADX must be 35 or greater.\" (p.2)",
            "E|\"D. Yesterday's close must be the fifth consecutive day that the stock closed below it's 2-period SMA.\" \"E. Enter on the stock's next day opening.\" (p.2)",
            "A|\"(A second entry occurs if the stock closes under the 2-period SMA the next day.)\" (p.2); each unit occupies one of 10 slots (p.3)",
            "X|\"F. Exit full position when the 2-period RSI closes above 75.\" (p.2)"],
        notes: NOTE, claim: None }]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    match id { "tmcap_main" => Some(("tmcap_main", "TMCAP", "HV 2x, ADX 35, 5 closes under SMA(2), RSI(2) 75")), _ => None }
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    match id {
        "tmcap_main" => Some(vec![p("trend", "Trend average, days (slide 200)", 200., 10., 300., 10.), p("hv_x", "Stock HV(100) at least N x the market's (slide 2)", 2., 1., 5., 0.5),
            p("adx", "ADX(10) at least (slide 35)", 35., 0., 60., 5.), p("below", "Closes under the 2-day average in a row, exactly (slide 5)", 5., 1., 10., 1.),
            p("add", "Second unit after one more close under it: 1 = yes (slide), 0 = no", 1., 0., 1., 1.), p("exit_rsi", "Exit: RSI(2) closes above (slide 75)", 75., 50., 95., 5.),
            p("exit_fill", "Exit fill: 0 = next open (reading), 1 = that close", 0., 0., 1., 1.)]),
        _ => None,
    }
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.r.len() { return None; }
    let s = params(id)?;
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| s.iter().find(|d| d.key == k).unwrap().def);
    let (o, c, n) = (&x.o, &x.c, x.c.len());
    let (tr, s2, rs) = (x.sma_n(g("trend").round() as usize), x.sma_n(2), x.rsi_n(2));
    let (hvx, adx, need, rx) = (g("hv_x"), g("adx"), g("below").round() as usize, g("exit_rsi"));
    let (add, at_close) = (g("add") > 0.5, g("exit_fill") > 0.5);
    let mut run_len = vec![0usize; n];
    for i in 0..n { if c[i] < s2[i] { run_len[i] = if i > 0 { run_len[i - 1] } else { 0 } + 1; } }
    let entry = |i: usize| c[i] > tr[i] && x.hv100[i] >= hvx * x.bench_hv100[i] && x.adx10[i] >= adx && run_len[i] == need;
    let (mut out, mut cur, mut sig, mut pend_in, mut pend_add) = (vec![], None::<Trade>, 0usize, false, false);
    for i in from..=to {
        if pend_in {
            pend_in = false;
            cur = Some(Trade { dir: 1., legs: vec![Leg { i, px: o[i], w: 0.5 }], exit_i: i, exit_px: o[i], reason: String::new(), entry_kind: "open", signal_i: i, open: false, exits: vec![] });
        }
        if pend_add {
            pend_add = false;
            if let Some(t) = cur.as_mut() { t.legs.push(Leg { i, px: o[i], w: 0.5 }); }
        }
        if let Some(t) = cur.as_mut() {
            if rs[i] > rx {
                let (xi, xp) = if at_close || i == to { (i, c[i]) } else { (i + 1, o[i + 1]) };
                if !(i == to && !at_close) {
                    t.exit_i = xi; t.exit_px = xp; t.signal_i = i; t.reason = if at_close { format!("RSI(2) > {rx}, sold at the close") } else { format!("RSI(2) > {rx}, sold next open") };
                    out.push(cur.take().unwrap());
                }
            } else if add && t.legs.len() == 1 && i == sig + 1 && c[i] < s2[i] && i < to { pend_add = true; }
            continue;
        }
        if i < to && entry(i) { pend_in = true; sig = i; }
    }
    if let Some(mut t) = cur { t.exit_i = to; t.exit_px = c[to]; t.reason = "open".into(); t.signal_i = to; t.open = true; out.push(t); }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;
    #[test]
    fn rejects_other_ids_and_short_windows() {
        let r: Vec<Row> = (0..5).map(|i| Row { ticker: "T".into(), date: format!("2001-01-{:02}", i + 1), open: 1., high: 1., low: 1., close: 1., volume: None }).collect();
        let x = Ctx::new(&r, &r, &[]);
        let p = Params::default();
        assert!(run("tmcap_x", &x, 0, 4, &p).is_none());
        assert_eq!(run("tmcap_main", &x, 0, 4, &p).unwrap().len(), 0);
    }
}
