//! Dennis Meyers, "A New Utility Average Stock Market System", Technical Analysis of Stocks & Commodities V15:3 (1997) p.101-111
//! (research share Trading Knowledge I/Stocks & Commodities DVD 11.26 (1982 - 2008) (traders.com)/VOLUMES/V15/C03/ANEWUTI.pdf).
//! Book id `dmlut`, under Dennis Meyers. Lane PF (13 Nov 2026, A1 row 437; R4 #297); independent check authors/pf/w/pf_ref.py.
//! Not the same article as dmutil (V13:8, weekly UD% / UtRs): this is the later DAILY log model with separate buy and sell lags.
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};
use super::dmadr::ema_first;
use super::rbtie::target_trades;

pub const REVIEWED: &[&str] = &["dmlut_daily"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }
pub const SERIES: &[&str] = &["DJ_UTILITY"];

const NOTE: &str = "[Adapted: 3 readings, R4 #297; the third is fixed by the article's own trade list] Run bar by bar on the close; 'Buy SP on the close' / 'Sell SP on the close' reverse the position (always in after the first signal; Figures 2 and 6 alternate Buy/Sell). Util = 100 LOG(DJ Utility close) from Norgate $DJU (SOURCES norgate-us-indices-20200908-pb, ends 2020-09-04); SP = the S&P 500 index close = the traded SPX close, so the card runs on SPX only. Unstated in the article: (1) the EMA starting value; vault reading: the first value, as TradeStation's XAverage (a 0 start would put a false jump into the 65/100-day log changes); (2) the printed 'Utav = Ut[1] + a(Util - Utav[1])' is read as the exponential moving average the text names, Utav = Utav[1] + a(Util - Utav[1]). 'Positive for Bcons days in a row' with Bcons = 0 is always true (the article: 'Setting Bcons = 0 means that the utility direction parameter is not used'). Read from the trade list: when the buy and the sell rule both hold on a bar, the position is kept (Figure 6 stays short from the 4/30/74 sell to the 9/12/74 buy although both rules hold daily from mid-August 1974; taking the buy first would flip every day). A day without a utility close keeps the averages unchanged.";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef { id: "dmlut_daily", name: "Meyers daily log utility system: Dow Utilities strength vs the S&P 500 over separate buy and sell lags, stop and reverse", group: "S&C: A New Utility Average Stock Market System (Meyers, V15:3)",
        session: 0, dir: "Long/Short", status: Status::Live, needs_volume: false, needs_vix: false, needs_series: SERIES, book: "dmlut", rules: &[
            "F|Util = 100 (LOG(Dow Jones Utility closing price)); UtSpR = 100 (LOG(DJ Util/S&P500)); Utav, UtSpRav = their 3-day exponential moving averages (a = 0.5) (p.104-105)",
            "F|UtDrB = Utav - Utav[Uplag]; UtRsB = UtSpRav - UtSpRav[Uplag]; UtDrS = Utav - Utav[DnLag]; UtRsS = UtSpRav - UtSpRav[Dnlag] (p.105)",
            "E|\"If UtRsB is greater than or equal to the value Brs and UtDrB is positive for Bcons days in a row or UtRsB is greater than or equal to the value BrsOnly Then Buy SP on the close.\" (p.105-106)",
            "X|\"If UtRsS is less than or equal to the value Srs and UtDrS is negative for Scons days in a row or UtRsS is less than or equal to the value SrsOnly Then Sell SP on the close.\" (p.106); the sell reverses to short (Figure 1: 13 long and 13 short trades)",
            "F|Optimum values (Figures 1-6): Brs 3, BrsOnly 3, Bcons 0, UpLag 65 days, Srs -6, SrsOnly -24, Scons 52, DnLag 100"],
        notes: NOTE, claim: crate::connors::claim("35", "91%", "1,340.64 S&P 500 points total net profit", "333 bars (winners)", "S&P 500 daily 1/2/1953-11/29/1996 (Figure 5; trade list Figure 6)") }]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    reviewed(id).then_some(("dmlut_daily", "Meyers daily log utility", "Optimum values"))
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    reviewed(id).then(|| vec![p("brs", "Brs", 3., -10., 20., 0.5), p("brsonly", "BrsOnly", 3., -10., 30., 0.5), p("bcons", "Bcons: days UtDrB > 0", 0., 0., 60., 1.),
        p("uplag", "UpLag days", 65., 5., 150., 5.), p("srs", "Srs", -6., -30., 10., 0.5), p("srsonly", "SrsOnly", -24., -40., 0., 1.),
        p("scons", "Scons: days UtDrS < 0", 52., 0., 80., 1.), p("dnlag", "DnLag days", 100., 5., 150., 5.)])
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from > to || to >= x.c.len() { return None; }
    let schema = params(id)?;
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).unwrap().def);
    let Some(du) = x.series("DJ_UTILITY") else { return Some(vec![]) };
    let u = |k: &str| g(k).round().max(0.0) as usize;
    let (bcons, uplag, scons, dnlag) = (u("bcons"), u("uplag").max(1), u("scons"), u("dnlag").max(1));
    let util: Vec<f64> = du[..=to].iter().map(|v| 100.0 * v.ln()).collect();
    let usp: Vec<f64> = (0..=to).map(|i| 100.0 * (du[i] / x.c[i]).ln()).collect();
    let (utav, usav) = (ema_first(&util, 0.5), ema_first(&usp, 0.5));
    let d = |v: &[f64], i: usize, lag: usize| if i >= lag { v[i] - v[i - lag] } else { f64::NAN };
    // "positive for n days in a row": the n latest bars (n = 0: always true)
    let run_of = |n: usize, i: usize, lag: usize, sign: f64| n == 0 || (i + 1 >= n && (i + 1 - n..=i).all(|k| sign * d(&utav, k, lag) > 0.0));
    let mut tgt = vec![0i8; to + 1];
    let mut pos = 0i8;
    for i in 0..=to {
        let (rsb, rss) = (d(&usav, i, uplag), d(&usav, i, dnlag));
        let buy = (rsb >= g("brs") && run_of(bcons, i, uplag, 1.0)) || rsb >= g("brsonly");
        let sell = (rss <= g("srs") && run_of(scons, i, dnlag, -1.0)) || rss <= g("srsonly");
        if buy != sell { pos = if buy { 1 } else { -1 }; } // both at once: the position is kept (Figure 6, Aug-Sep 1974)
        tgt[i] = pos;
    }
    Some(target_trades(x, &tgt, from, to, false, "Log utility signal"))
}

pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book == "dmlut" && reviewed(def.id),
    label: "Dennis Meyers switches the S&P 500 on the Dow Jones Utility Average's daily log strength against the S&P 500 index; the ratio uses the S&P 500 index close, so the card runs on SPX only, with the two unstated details read as labelled on the card.",
    basis: "Dennis Meyers, A New Utility Average Stock Market System, Stocks & Commodities V15:3 p.101-111 (VOLUMES/V15/C03/ANEWUTI.pdf); Dow Jones Utility Average from Norgate (SOURCES norgate-us-indices-20200908-pb); independent check authors/pf/w/pf_ref.py",
};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;
    #[test]
    fn utility_strength_buys_and_sells() {
        // Unit-test fixture only (no market data): utilities outrun the index for 120 bars, then lag for 200.
        let n = 330;
        let r: Vec<Row> = (0..n).map(|i| Row { ticker: "SPX".into(), date: format!("{}-{:02}-{:02}", 2000 + i / 300, 1 + (i % 300) / 25, 1 + i % 25), open: 100., high: 100., low: 100., close: 100., volume: None }).collect();
        let mut x = Ctx::new(&r, &r, &[]);
        let du: Vec<f64> = (0..n).map(|i| if i < 120 { 100. * (1.001f64).powi(i as i32) } else { 100. * (1.001f64).powi(120) * (0.999f64).powi(i as i32 - 120) }).collect();
        x.extra.insert("DJ_UTILITY".into(), du);
        let t = run("dmlut_daily", &x, 0, n - 1, &Params::new()).unwrap();
        assert_eq!(t.iter().map(|t| t.dir).collect::<Vec<_>>(), vec![1.0, -1.0]);
        // buy on the first bar with a 65-bar change (bar 65: +6.5 >= 3); sell once the 100-bar change is <= -6 with 52 falling days in a row
        assert_eq!(t[0].legs[0].i, 65);
        assert!((200..=240).contains(&t[1].legs[0].i), "{}", t[1].legs[0].i);
    }
}
