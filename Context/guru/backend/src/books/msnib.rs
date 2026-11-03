//! Mark Shipman, The Next Big Investment Boom (Kogan Page), ch.5 'Developing an investment strategy': Method 1 (printed
//! p.69-71), Method 2 and the combined rules (p.75-76), the weekly routine (p.86); research share USB HDD 2/tRADING KNO-2/
//! Mark Shipman - The Next Big Investment Boom/Mark Shipman The Next Big Investment Boom.pdf. Book id `msnib`; admitted with
//! `msbml` (same author). Lane K7 (titles loop, 1 Nov 2026); rule text authors/batch-1101/B2w/shipman_nbib.txt; independent
//! check authors/titles-1030/K7w/k7_ref.py (header).
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};
use super::mpts::{period_ends, sma};
use super::msbml::on_weeks;
use super::rbtie::target_trades;

/// Admitted ids (authors/titles-1030/K7w/k7_ref.py (header)).
pub const REVIEWED: &[&str] = &["msnib_ma40w", "msnib_ma40w_brk12"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const NOTE: &str = "[Declared readings] Weekly bar = the last trading day of each Monday-based week (the last data bar counts only if it is a Friday); weekly high = the highest daily high of that week. 40-week MA = simple average of the last 40 weekly closes (p.70); 'trend upwards' = this week's average above last week's. Method 1: invested while the weekly close is above the 40-week MA and the MA is rising, out when either fails. Combined Methods 1 and 2: enter only when, in addition, the weekly close is above the highest weekly high of the previous 12 weeks ('requires the market to post a weekly close above the highest high', p.75; option: the close is the highest weekly close of the last 12 weeks); then stay invested on Method 1 alone. Orders at the next week's first open ('when the markets reopen at the beginning of the following week', p.86); option: the signal week's close. Long or cash. Not coded: Methods 3-4 exits (an 'exponential' rally then the first big reaction; a 'large, sharp reaction') and stage analysis are judgement; Method 2 alone has no exit. Not Pring's 40-week rule (mptae_ma40w: no slope, no breakout). The book's markets are commodities, commodity funds and an S&P 500 example; ETFs, US and India indices and stocks are a declared application.";

pub fn catalogue() -> Vec<StrategyDef> {
    let d = |id, name, rules, notes| StrategyDef { id, name, group: "Shipman Next Big Investment Boom", session: 0, dir: "Long", status: Status::Live,
        needs_volume: false, needs_vix: false, needs_series: &[], book: "msnib", rules, notes, claim: None };
    vec![
        d("msnib_ma40w", "Method 1: invested while the weekly close is above a rising 40-week average", &[
            "E|\"Only invest or stay invested in a market where the current weekly closing price is above the current 40-week MA and the trend of that 40-week MA is upwards.\" (p.70)",
            "X|\"You should never invest or stay invested in a market where the current weekly closing price is below the current 40-week MA. And: You should never invest or stay invested in a market where the trend of the 40-week MA is down.\" (p.70-71)"], NOTE),
        d("msnib_ma40w_brk12", "Methods 1 and 2: enter on a 12-week upside breakout above a rising 40-week average, stay while above the rising average", &[
            "E|\"Entry: Only establish an investment position when the current weekly close is the highest price of the last 12 weeks and it is above the current 40-week MA and the trend of that 40-week MA is upwards.\" (p.75)",
            "X|\"Position running: Stay invested as long as the current weekly close is above the current 40-week MA and the trend of that 40-week MA is upwards.\" (p.76)"], NOTE),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    Some(match id {
        "msnib_ma40w" => ("msnib_ma40w", "Shipman 40-week trend rules", "Method 1: rising 40-week MA"),
        "msnib_ma40w_brk12" => ("msnib_ma40w", "Shipman 40-week trend rules", "Methods 1+2: with a 12-week breakout"),
        _ => return None,
    })
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    if !reviewed(id) { return None; }
    let mut v = vec![
        ParamDef { key: "n", label: "Moving average (weeks, book 40)", def: 40., min: 2., max: 80., step: 1. },
        ParamDef { key: "fill", label: "0 = next week's first open (reading), 1 = signal week's close", def: 0., min: 0., max: 1., step: 1. },
    ];
    if id == "msnib_ma40w_brk12" {
        v.push(ParamDef { key: "brk", label: "Breakout (weeks, book 12)", def: 12., min: 2., max: 52., step: 1. });
        v.push(ParamDef { key: "hl", label: "Breakout: 0 = close above the previous weeks' highest high (reading), 1 = highest weekly close", def: 0., min: 0., max: 1., step: 1. });
    }
    Some(v)
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.r.len() { return None; }
    let g = |k: &str, d: f64| p.get(k).copied().unwrap_or(d);
    let (n, brk, by_close) = (g("n", 40.).round().max(1.) as usize, g("brk", 12.).round().max(1.) as usize, g("hl", 0.) >= 0.5);
    let wk = period_ends(x, false);
    let idx: Vec<usize> = (0..wk.len()).filter(|&i| wk[i]).collect();
    // weekly highs: highest daily high from the bar after the previous week end to this week end
    let whi: Vec<f64> = idx.iter().enumerate().map(|(k, &i)| { let a = if k == 0 { 0 } else { idx[k - 1] + 1 }; x.h[a..=i].iter().copied().fold(f64::NEG_INFINITY, f64::max) }).collect();
    let ma = on_weeks(&wk, &x.c, |v| sma(v, n));
    let combined = id == "msnib_ma40w_brk12";
    let mut tgt = vec![0i8; x.c.len()];
    let (mut cur, mut prev_ma) = (0i8, f64::NAN);
    let mut k = 0;
    for i in 0..x.c.len() {
        if wk[i] {
            let c = x.c[i];
            let ok = c > ma[i] && ma[i] > prev_ma;
            let brk_ok = k >= brk && if by_close { (k + 1 - brk..k).all(|j| c >= x.c[idx[j]]) } else { c > whi[k - brk..k].iter().copied().fold(f64::NEG_INFINITY, f64::max) };
            cur = if !combined || cur == 1 { ok as i8 } else { (ok && brk_ok) as i8 };
            prev_ma = ma[i];
            k += 1;
        }
        tgt[i] = cur;
    }
    Some(target_trades(x, &tgt, from, to, g("fill", 0.) < 0.5, if combined { "40-week MA + 12-week breakout" } else { "40-week MA" }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;
    #[test]
    fn combined_rule_enters_only_on_breakouts_above_a_rising_average() {
        let start = 11323; // 2001-01-01, a Monday, as days after 1970-01-01
        let mut rows = vec![];
        let mut dn = start;
        for k in 0..1200 {
            let v = 100. + 25. * (k as f64 / 170.).sin() + 4. * (k as f64 / 7.).cos();
            rows.push(Row { ticker: "T".into(), date: ymd(dn), open: v, high: v + 1., low: v - 1., close: v, volume: None });
            dn += if k % 5 == 4 { 3 } else { 1 };
        }
        let x = Ctx::new(&rows, &rows, &[]);
        let wk = period_ends(&x, false);
        let a = run("msnib_ma40w", &x, 0, rows.len() - 1, &Params::new()).unwrap();
        let b = run("msnib_ma40w_brk12", &x, 0, rows.len() - 1, &Params::new()).unwrap();
        assert!(!a.is_empty() && !b.is_empty() && b.len() <= a.len());
        assert!(a.iter().chain(&b).all(|t| t.dir > 0. && wk[t.legs[0].i - 1]));
        assert!(run("msnib_x", &x, 0, 5, &Params::new()).is_none());
    }
    fn ymd(z: i64) -> String { // civil-from-days (Howard Hinnant)
        let z = z + 719468; let era = z.div_euclid(146097); let doe = z - era * 146097;
        let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365; let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
        let mp = (5 * doy + 2) / 153; let d = doy - (153 * mp + 2) / 5 + 1; let m = if mp < 10 { mp + 3 } else { mp - 9 };
        format!("{}-{:02}-{:02}", yoe + era * 400 + (m <= 2) as i64, m, d)
    }
}
