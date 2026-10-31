//! Richard Cluver, How to Profit from Share Market Charting (ShareFinder, 1993): ch.4 Trading Bands and ch.5 summary
//! (printed p.18-21: the 12-month exponential average turn and the 10% trading bands about it) and ch.13 The Stochastic
//! Indicator (printed p.46-47); research share USB HDD 2/tRADING KNO-2/Richard Cluver - How to Profit from Share Market
//! Charting/How to Profit from Share Market Charting by Richard Cluver.pdf (scan, ocr2). Book id `rchsc`. Lane K7 (titles
//! loop, 1 Nov 2026); rule text authors/batch-c/C2w/cluver_rules.txt; independent check authors/titles-1030/K7w/k7_ref.py.
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};
use super::mkits::{hhv, llv};
use super::mpts::{period_ends, sma};
use super::msbml::on_weeks as on_ends;
use super::rbtie::target_trades;

/// Admitted ids (authors/titles-1030/K7w/k7_ref.py (header)).
pub const REVIEWED: &[&str] = &["rchsc_ema12m", "rchsc_band12m", "rchsc_stoch10"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

macro_rules! monthly { () => { "[Declared readings] Month-end bars: the last trading day of each month (the last data bar never counts as a month end); 12-month exponential average with weight 2/(n+1), started at the first month-end close. Signals on the month's last close, filled at the next month's first open (option: that close). Long or cash. The author's test is the Johannesburg Stock Exchange index (not held); US and India indices, ETFs and stocks are a declared application." } }
const NOTE_EMA: &str = concat!(monthly!(), " Buy when the 12-month exponential average turns up (above last month's value), sell when it turns down (below last month's value); an unchanged value changes nothing. Book: JSE Actuaries All Market Index Aug 1982-Jul 1992, seven buys and seven sells, +419.4% (p.20).");
const NOTE_BAND: &str = concat!(monthly!(), " Bands 10% above and below the 12-month exponential average ('The user chooses how wide', p.18; width is an option): buy on a month-end close above the upper band, sell on a month-end close below the lower band, nothing in between. Book: JSE, three transactions to July 1992, +371.65% (p.20).");
const NOTE_STO: &str = "[Declared readings] Daily. %K = 100 x (close - lowest low)/(highest high - lowest low) over 10 days; D = 3-day simple average of %K (the D period is not printed: 'simply a smoothed version of the K line'). Get ready to buy when %K turns up (higher than yesterday after yesterday was at or below the day before) from a value below 20; buy when %K then crosses above D (the same day counts). Get ready to sell when %K turns down from a peak at or above 80 ('between 80 and 90': the upper bound is not applied); sell when %K then crosses below D. A new get-ready signal cancels the opposite one. Long or cash; fills at the next day's open. Book: Rand gold price Aug 1992-Jan 1993, seven transactions, +14.2% (gold not held). US and India indices, ETFs and stocks are a declared application.";

pub fn catalogue() -> Vec<StrategyDef> {
    let d = |id, name, group, rules, notes| StrategyDef { id, name, group, session: 0, dir: "Long", status: Status::Live, needs_volume: false, needs_vix: false,
        needs_series: &[], book: "rchsc", rules, notes, claim: None };
    vec![
        d("rchsc_ema12m", "12-month exponential average turn: invested while it rises", "Cluver moving averages", &[
            "F|\"I chose a 12-month exponential moving average of the Johannesburg Stock Exchange Actuaries All Market Index\" (p.19)",
            "E|\"employ a medium-term exponential moving average, buying when the graph line changes direction from down to up\" (p.21)",
            "X|\"and selling when a rising trend changes direction\" (p.21)"], NOTE_EMA),
        d("rchsc_band12m", "10% trading bands about the 12-month exponential average", "Cluver moving averages", &[
            "F|\"a ten per cent trading band spread about a 12-period exponential moving average ... plotted on month end data\" (p.18)",
            "E|\"The investor buys only above the upper line\" (p.18)",
            "X|\"and he sells only below the bottom line\" (p.18)"], NOTE_BAND),
        d("rchsc_stoch10", "10-day stochastic: %K turns up below 20, buy on its cross above D; sell on the cross below D after a peak above 80", "Cluver stochastic", &[
            "F|\"%K = ((C-L)/(H-L))* 100 ... The D line is simply a smoothed version of the K line\"; \"Applying a 10-day Stochastic indicator\" (p.47)",
            "E|\"watch for the K to turn upwards anywhere below the 20% point which is seen as a get-ready-to-buy signal. The buy is confirmed when it intersects a slightly-later-turning D line\" (p.47)",
            "X|\"a get-ready-to-sell signal is represented by the K line peaking anywhere between 80 and 90 per cent with the sell being confirmed when the K line intersects the slightly-later-turning D line\" (p.47)"], NOTE_STO),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    Some(match id {
        "rchsc_ema12m" => ("rchsc_ema", "Cluver 12-month EMA rules", "EMA turn"),
        "rchsc_band12m" => ("rchsc_ema", "Cluver 12-month EMA rules", "10% bands"),
        "rchsc_stoch10" => ("rchsc_stoch10", "Cluver 10-day stochastic", "K/D 10, 3"),
        _ => return None,
    })
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let fill = ParamDef { key: "fill", label: "0 = next month's first open (reading), 1 = the month-end close", def: 0., min: 0., max: 1., step: 1. };
    let n = ParamDef { key: "n", label: "Exponential average (months, book 12)", def: 12., min: 2., max: 36., step: 1. };
    Some(match id {
        "rchsc_ema12m" => vec![n, fill],
        "rchsc_band12m" => vec![n, ParamDef { key: "band", label: "Band width (% about the average, book 10)", def: 10., min: 1., max: 30., step: 0.5 }, fill],
        "rchsc_stoch10" => vec![ParamDef { key: "k", label: "Stochastic period (days, book 10)", def: 10., min: 3., max: 30., step: 1. },
                                ParamDef { key: "d", label: "D smoothing (days, reading 3)", def: 3., min: 1., max: 10., step: 1. }],
        _ => return None,
    })
}

/// EMA with weight 2/(n+1) started at the first value.
fn ema_first(v: &[f64], n: usize) -> Vec<f64> {
    let a = 2. / (n as f64 + 1.);
    let mut out = vec![f64::NAN; v.len()];
    for i in 0..v.len() { out[i] = if i == 0 { v[0] } else { out[i - 1] + a * (v[i] - out[i - 1]) }; }
    out
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.r.len() { return None; }
    let g = |k: &str, d: f64| p.get(k).copied().unwrap_or(d);
    let (c, len) = (&x.c, x.c.len());
    let mut tgt = vec![0i8; len];
    if id == "rchsc_stoch10" {
        let (n, dn) = (g("k", 10.).round().max(1.) as usize, g("d", 3.).round().max(1.) as usize);
        let (hh, ll) = (hhv(&x.h, n), llv(&x.l, n));
        let k: Vec<f64> = (0..len).map(|i| { let r = hh[i] - ll[i]; if r > 0. { 100. * (c[i] - ll[i]) / r } else { f64::NAN } }).collect();
        let d = sma(&k, dn);
        let (mut cur, mut arm_b, mut arm_s) = (0i8, false, false);
        for i in 2..len {
            if k[i] > k[i - 1] && k[i - 1] <= k[i - 2] && k[i - 1] < 20. { arm_b = true; arm_s = false; }
            if k[i] < k[i - 1] && k[i - 1] >= k[i - 2] && k[i - 1] >= 80. { arm_s = true; arm_b = false; }
            if cur == 0 && arm_b && k[i] > d[i] && k[i - 1] <= d[i - 1] { cur = 1; arm_b = false; }
            else if cur == 1 && arm_s && k[i] < d[i] && k[i - 1] >= d[i - 1] { cur = 0; arm_s = false; }
            tgt[i] = cur;
        }
        return Some(target_trades(x, &tgt, from, to, true, "stochastic K/D"));
    }
    let n = g("n", 12.).round().max(1.) as usize;
    let mo = period_ends(x, true);
    let e = on_ends(&mo, c, |v| ema_first(v, n));
    let band = g("band", 10.) / 100.;
    let (mut cur, mut prev) = (0i8, f64::NAN);
    for i in 0..len {
        if mo[i] {
            cur = if id == "rchsc_ema12m" {
                if e[i] > prev { 1 } else if e[i] < prev { 0 } else { cur }
            } else if c[i] > e[i] * (1. + band) { 1 } else if c[i] < e[i] * (1. - band) { 0 } else { cur };
            prev = e[i];
        }
        tgt[i] = cur;
    }
    Some(target_trades(x, &tgt, from, to, g("fill", 0.) < 0.5, if id == "rchsc_ema12m" { "12-month EMA turn" } else { "12-month EMA bands" }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;
    #[test]
    fn monthly_and_stochastic_rules_trade_on_their_signals() {
        let (mut rows, mut dn) = (vec![], 10959i64); // Monday 2000-01-03, weekdays only
        for k in 0..3000 {
            let v = 100. + 30. * (k as f64 / 300.).sin() + 3. * (k as f64 / 5.).cos();
            rows.push(Row { ticker: "T".into(), date: ymd(dn), open: v, high: v + 1., low: v - 1., close: v, volume: None });
            dn += if k % 5 == 4 { 3 } else { 1 };
        }
        let x = Ctx::new(&rows, &rows, &[]);
        let mo = period_ends(&x, true);
        for id in ["rchsc_ema12m", "rchsc_band12m"] {
            let t = run(id, &x, 0, rows.len() - 1, &Params::new()).unwrap();
            assert!(!t.is_empty() && t.iter().all(|z| z.dir > 0. && mo[z.legs[0].i - 1]), "{id}");
        }
        assert!(run("rchsc_stoch10", &x, 0, rows.len() - 1, &Params::new()).unwrap().len() > 10);
        assert!(run("rchsc_x", &x, 0, 5, &Params::new()).is_none());
    }
    fn ymd(z: i64) -> String {
        let z = z + 719468; let era = z.div_euclid(146097); let doe = z - era * 146097;
        let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365; let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
        let mp = (5 * doy + 2) / 153; let d = doy - (153 * mp + 2) / 5 + 1; let m = if mp < 10 { mp + 3 } else { mp - 9 };
        format!("{}-{:02}-{:02}", yoe + era * 400 + (m <= 2) as i64, m, d)
    }
}

// Instrument admission: Richard Cluver (How to Profit from Share Market Charting) 2026-11-01 (titles lane K7).
// Admission only for the module REVIEWED lists (authors/titles-1030/K7w/k7_ref.py (header)).
// Runs on actual indices, ETFs and stocks, US and India (index_etf_or_stock, nifty50 basket, declared application).
pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book == "rchsc" && reviewed(def.id),
    label: "Richard Cluver tested these rules on the Johannesburg Stock Exchange index and the Rand gold price; US and India indices, ETFs and stocks are a declared application.",
    basis: "Richard Cluver, How to Profit from Share Market Charting (1993), ch.4-5 printed p.18-21, ch.13 p.46-47 (OCR of the scan)",
};
