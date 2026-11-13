//! Alex Saitta, "A Price- And Volume-Based System", Technical Analysis of Stocks & Commodities V14:3 (1996) p.103-105
//! (research share Trading Knowledge I/Stocks & Commodities DVD 11.26 (1982 - 2008) (traders.com)/VOLUMES/V14/C03/APRICEA.pdf).
//! Book id `scsaitta`, author Alex Saitta. Lane PF (13 Nov 2026, A1 row 397; R4 #293, #297); independent check authors/pf/w/pf_ref2.py.
//! The T-bond 20-day moving average system with the volume condition on entries; p.vol = 0 is the article's initial always-in system.
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};
use super::rbtie::target_trades;

pub const REVIEWED: &[&str] = &["scsaitta_volma"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const NOTE: &str = "[Adapted: 2 unstated details, R4 #297] Norgate continuous T-bond futures (&ZB), back-adjusted daily bars; volume = Norgate's continuous-series volume, i.e. the contract held in the series, not the total of all months (labelled reading, as R4 #301 (3)); Norgate continuous futures, ends 2020-09-11; roll method not stated by the vendor (the article used a back-adjusted continuous contract too, 'adjusted for the quarterly rollover'). Unstated in the article: (1) fills: vault reading, every entry and exit at the close of the signal day (the rules are close conditions: 'crossed and then closed above', 'closed below'); (2) 'the average daily volume of the last 10 days': vault reading, the 10 days before the signal day (today's volume is compared with it). A 'cross' needs the previous close at or below (above) the previous average; a long closed by a close below the average reverses to short the same day when that day's volume qualifies. One contract, no costs (as the article). p.vol = 0: the article's initial system (always in, no volume condition, Figures 1-2).";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef { id: "scsaitta_volma", name: "Saitta T-bond 20-day moving average with a volume condition: enter on a close through the average on above-average volume, exit on the close back through it", group: "S&C: A Price- And Volume-Based System (Saitta, V14:3)",
        session: 0, dir: "Long/Short", status: Status::Live, needs_volume: true, needs_vix: false, needs_series: &[], book: "scsaitta", rules: &[
            "E|\"A long position was initiated when the market crossed and then closed above the 20-day moving average and that day's trading volume was greater than the average daily volume of the last 10 days.\" (p.104)",
            "X|\"The long position was liquidated when the market closed below the average.\" (p.104)",
            "E|\"A short position was initiated when the market closed below the 20-day moving average and that day's volume was greater than the average volume of the last 10 days.\" (p.105) \"Positions are initiated only when the T-bond closes through the moving average and that day's volume exceeds the identified threshold.\"",
            "X|\"The short position was covered when the market closed above the moving average.\" (p.105)",
            "F|Tested on a continuous T-bond contract adjusted for the quarterly rollover, 1988-1995, one contract, no commission or slippage (p.103-104)"],
        notes: NOTE, claim: crate::connors::claim("126 (derived: $35,472 / $281)", "33% (derived: 42 of 126; 84 losers printed)", "$35,472 total ($281 per trade); 84 losing trades (printed)", "n/a", "T-bond futures daily 1988-1995 (p.104, Figures 4-5); the initial always-in system: $12,504, $60 per trade, 151 losers") }]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    reviewed(id).then_some(("scsaitta_volma", "Saitta price and volume system", "20-day average, 10-day volume"))
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    reviewed(id).then(|| vec![p("n", "Moving average days", 20., 5., 60., 1.), p("vn", "Average volume days", 10., 3., 30., 1.),
        p("vol", "Volume condition (1) / initial always-in system (0)", 1., 0., 1., 1.)])
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from > to || to >= x.c.len() { return None; }
    let schema = params(id)?;
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).unwrap().def);
    let (n, vn, use_vol) = (g("n").round().max(1.) as usize, g("vn").round().max(1.) as usize, g("vol") >= 0.5);
    let c = &x.c;
    let sma = |i: usize| if i + 1 >= n { c[i + 1 - n..=i].iter().sum::<f64>() / n as f64 } else { f64::NAN };
    let vol = |i: usize| x.r[i].volume.unwrap_or(f64::NAN);
    let vol_ok = |i: usize| !use_vol || (i >= vn && vol(i) > (i - vn..i).map(vol).sum::<f64>() / vn as f64);
    let mut tgt = vec![0i8; to + 1];
    let mut pos = 0i8;
    for i in 1..=to {
        let (m, m1) = (sma(i), sma(i - 1));
        if pos == 1 && c[i] < m { pos = 0; }
        if pos == -1 && c[i] > m { pos = 0; }
        if pos == 0 && vol_ok(i) {
            if c[i] > m && c[i - 1] <= m1 { pos = 1; } else if c[i] < m && c[i - 1] >= m1 { pos = -1; }
        }
        tgt[i] = pos;
    }
    Some(target_trades(x, &tgt, from, to, false, "Saitta signal"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;
    #[test]
    fn volume_gates_the_entry_not_the_exit() {
        // Unit-test fixture only (no market data): flat at 100 for 25 bars, a rise through the average on bar 25 with low volume
        // (no entry), back below, then a second cross on bar 30 with high volume (entry), and a close below the average on bar 34.
        let px = |i: usize| match i { 25 => 101., 26..=29 => 99., 30..=33 => 102., _ if i >= 34 => 95., _ => 100. };
        let vol = |i: usize| if i == 30 { 500. } else { 100. };
        let r: Vec<Row> = (0..40).map(|i| Row { ticker: "T".into(), date: format!("2000-{:02}-{:02}", 1 + i / 28, 1 + i % 28), open: px(i), high: px(i), low: px(i), close: px(i), volume: Some(vol(i)) }).collect();
        let x = Ctx::new(&r, &r, &[]);
        let t = run("scsaitta_volma", &x, 0, 39, &Params::new()).unwrap();
        assert_eq!(t.iter().map(|t| (t.dir, t.legs[0].i, t.exit_i)).collect::<Vec<_>>(), vec![(1.0, 30, 34)]);
        let mut p = Params::new(); p.insert("vol".into(), 0.);
        let t0 = run("scsaitta_volma", &x, 0, 39, &p).unwrap();
        assert_eq!(t0[0].legs[0].i, 25); // without the volume condition the first cross enters
    }
}
