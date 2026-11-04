//! Alexandra Guevara, "RSI Volatility Spread", TradeStation Labs report (2010; research share USB HDD 1/2011/_TradeStation/
//! RSI Volatility Spread/: 'TradeStation Labs.mht' and the strategy code 'AG.ATR RSI Strategy.txt', 25 lines). Book id `tslrvs`.
//! One strategy: the RSI of a 20-bar exponential average of the bar range, of the traded symbol (Data1) and of the S&P 500 (Data2),
//! their ratio against its average; long at the next open on lower-than-average relative volatility, out at the next open on higher.
//! Lane K52 (5 Nov 2026, decisions #223, #225, #226; EP4 second.rs); independent check authors/titles-1030/K52w/k52_ref.py.
use crate::connors::{rsi, Ctx, ParamDef, Params, Status, StrategyDef, Trade};
use super::msws::{simulate, Exec, Sig};

pub const REVIEWED: &[&str] = &["tslrvs_spread"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const NOTE: &str = "[Declared readings] The printed TradeStation strategy is the rule (code over prose): ATR = XAverage(Range, ATRLen) with Range = High - Low (TradeStation's Range function; the report's prose calls it 'average true range'); RangeMkt = High Data2 - Low Data2; ATRMkt = XAverage(RangeMkt, ATRLen); RSIATR = RSI(ATR, RSILen); RSIATRMkt = RSI(ATRMkt, RSILen); RSISpread = RSIATR / RSIATRMkt; AvgSpread = XAverage(RSISpread, RSILen); SpreadDiff = RSISpread - AvgSpread; AvgSpreadDiff = XAverage(SpreadDiff, AvgLen); inputs 20 / 120 / 50 (strategy inputs table). Buy next bar at market (the next open) when AvgSpreadDiff < 0 and RSIATRMkt > RSIATR; Sell (exit the long) next bar at market when AvgSpreadDiff > 0 and RSIATR > RSIATRMkt. Long only, one position. XAverage = TradeStation's, seeded with the first value; RSI = TradeStation RSI (Wilder averages seeded with the first RSILen changes). Data2 (decision #223): the S&P 500 on the traded symbol's dates; a date the S&P 500 lacks shows its previous bar's high and low (TradeStation Data2), runs with more than 5% such bars are withheld. Indicators start on the first bar on which both series exist. A division by zero (RSIATRMkt = 0) leaves no value from there, as TradeStation stops a strategy on that error. The author tested EWA (iShares MSCI Australia) with $SPX.X as Data2 over 19 years and lists IXC, IXJ, EWU, EWD, EWL, PBJ, PWC and XLP; other ETFs, US indices and US stocks are a declared application, and Indian instruments with NIFTY 50 as Data2 are a labelled benchmark swap. The S&P 500 itself and SPY are not run (compared with itself, decision #225). Cash-index check (decisions #177/#178): the rule run with the S&P 500 and with SPY as Data2 (2010-2020 and 1993-2026, 10 symbols), and on NDX vs QQQ and DJI vs DIA, is not flagged in both windows, so the S&P 500 and NIFTY 50 cash-index highs and lows are used, with this note.";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef { id: "tslrvs_spread", name: "RSI Volatility Spread: buy lower-than-average volatility against the S&P 500, sell higher", group: "TradeStation Labs: RSI Volatility Spread (Guevara)",
        session: 0, dir: "Long", status: Status::Live, needs_volume: false, needs_vix: false, needs_series: &["BENCH.H", "BENCH.L"], book: "tslrvs", rules: &[
            "F|\"ATR = XAverage(Range, ATRLen); RangeMkt = High Data2 - Low Data2; ATRMkt = XAverage(RangeMkt, ATRLen); RSIATR = RSI(ATR, RSILen); RSIATRMkt = RSI(ATRMkt, RSILen); RSISpread = RSIATR / RSIATRMkt; AvgSpread = XAverage(RSISpread, RSILen); SpreadDiff = RSISpread - AvgSpread; AvgSpreadDiff = XAverage(SpreadDiff, AvgLen);\" (Inputs: ATRLen(20), AvgLen(120), RSILen(50); \"//S&P 500 as Data2\")",
            "E|\"If AvgSpreadDiff < 0 and RSIATRMkt > RSIATR then Buy (\"Lo Volty Buy\") next bar at market;\"",
            "X|\"If AvgSpreadDiff > 0 and RSIATR > RSIATRMkt then Sell (\"High Volty Sell\") next bar at market;\""],
        notes: NOTE, claim: None }]
}
pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    reviewed(id).then_some(("tslrvs_spread", "TradeStation Labs RSI Volatility Spread", "ATR 20, RSI 50, average 120, S&P 500 Data2"))
}
pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    reviewed(id).then(|| vec![p("atr", "ATRLen (printed 20)", 20., 2., 100., 1.), p("avg", "AvgLen (printed 120)", 120., 2., 400., 1.), p("rsi", "RSILen (printed 50)", 50., 2., 200., 1.)])
}

/// TradeStation XAverage: seeded with the first value (here the first finite one), then a 2/(n+1) exponential average.
pub fn xavg(v: &[f64], n: usize) -> Vec<f64> {
    let a = 2. / (n as f64 + 1.);
    let mut out = vec![f64::NAN; v.len()];
    let Some(s) = v.iter().position(|x| x.is_finite()) else { return out };
    out[s] = v[s];
    for i in s + 1..v.len() { out[i] = out[i - 1] + a * (v[i] - out[i - 1]); }
    out
}

/// (AvgSpreadDiff, RSIATR, RSIATRMkt) on every bar; NaN before the first bar where both series exist (from `s0`).
pub fn lines(h: &[f64], l: &[f64], mh: &[f64], ml: &[f64], s0: usize, atr_n: usize, avg_n: usize, rsi_n: usize) -> (Vec<f64>, Vec<f64>, Vec<f64>) {
    // computed on the bars from s0 only (connors::rsi would read a NaN change before s0 as a zero change), then padded
    let n = h.len().max(s0);
    let rng = |a: &[f64], b: &[f64]| (s0..n).map(|i| a[i] - b[i]).collect::<Vec<f64>>();
    let (ra, rm) = (rsi(&xavg(&rng(h, l), atr_n), rsi_n), rsi(&xavg(&rng(mh, ml), atr_n), rsi_n));
    let spread: Vec<f64> = ra.iter().zip(&rm).map(|(a, m)| if *m == 0. { f64::NAN } else { a / m }).collect();
    let avg = xavg(&spread, rsi_n);
    let diff: Vec<f64> = spread.iter().zip(&avg).map(|(s, a)| s - a).collect();
    let pad = |v: Vec<f64>| { let mut o = vec![f64::NAN; s0]; o.extend(v); o };
    (pad(xavg(&diff, avg_n)), pad(ra), pad(rm))
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from > to || to >= x.r.len() { return None; }
    let (Some(mh), Some(ml)) = (x.series("BENCH.H"), x.series("BENCH.L")) else { return Some(vec![]) };
    let g = |k: &str, d: f64| p.get(k).copied().unwrap_or(d).round().max(1.) as usize;
    let s0 = crate::second::run_start(&[mh, ml], from);
    let (asd, ra, rm) = lines(&x.h, &x.l, mh, ml, s0, g("atr", 20.), g("avg", 120.), g("rsi", 50.));
    let el = (0..x.c.len()).map(|i| asd[i] < 0. && rm[i] > ra[i]).collect();
    let xl = (0..x.c.len()).map(|i| asd[i] > 0. && ra[i] > rm[i]).collect();
    let n = x.c.len();
    Some(simulate(x, from, to, &Sig { el, xl, es: vec![false; n], xs: vec![false; n] }, Exec { entry_open: true, exit_open: true }))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn xaverage_seeds_on_the_first_value_and_lines_start_with_data2() {
        let v = [f64::NAN, 2., 4., 4.];
        let e = xavg(&v, 3); // alpha 0.5
        assert!(e[0].is_nan() && e[1] == 2. && e[2] == 3. && e[3] == 3.5);
        let n = 400;
        let f = |k: usize, a: f64| (0..n).map(|i| 1. + a * (((i * k) % 17) as f64)).collect::<Vec<f64>>();
        let (h, l, mh, ml) = (f(3, 0.2), vec![0.; n], f(5, 0.1), vec![0.; n]);
        let (asd, ra, rm) = lines(&h, &l, &mh, &ml, 30, 20, 120, 50);
        assert!(asd[..80].iter().all(|v| v.is_nan()) && asd[80].is_finite()); // ATR from bar 30, first RSI value 50 changes later
        assert!(ra[79].is_nan() && ra[80].is_finite() && rm[80].is_finite());
        // RSI is scale-free: the market's range times 10 gives the same lines
        let (asd10, _, _) = lines(&h, &l, &mh.iter().map(|v| v * 10.).collect::<Vec<_>>(), &ml, 30, 20, 120, 50);
        assert!(asd.iter().zip(&asd10).skip(80).all(|(a, b)| (a - b).abs() < 1e-9));
    }
}

// Instrument admission: TradeStation Labs RSI Volatility Spread 2026-11-05 (titles lane K52).
pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book == "tslrvs" && crate::books::tslrvs::reviewed(def.id),
    label: "Alexandra Guevara (TradeStation Labs) tested EWA and other ETFs against the S&P 500; other ETFs, US indices and stocks are a declared application, Indian instruments with NIFTY 50 as Data2 a labelled benchmark swap.",
    basis: "TradeStation Labs report 'RSI Volatility Spread' (2010) and its strategy code AG.ATR RSI Strategy.txt (research share USB HDD 1/2011/_TradeStation/RSI Volatility Spread); second series per decision #223; independent check authors/titles-1030/K52w/k52_ref.py.",
};
