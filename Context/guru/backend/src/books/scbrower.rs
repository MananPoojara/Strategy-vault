//! William Brower, "The S&P 500 Seasonal Day Trade", Technical Analysis of Stocks & Commodities V14:8 (1996) p.338-342
//! (research share Trading Knowledge I/Stocks & Commodities DVD 11.26 (1982 - 2008) (traders.com)/VOLUMES/V14/C08/THESP50.pdf).
//! Book id `scbrower`, author William Brower. Lane PF (13 Nov 2026, A1 row 404; R4 #293, #297); independent check authors/pf/w/pf_ref3.py.
//! The system's buy patterns (2, 5, 7) and sell patterns (6, 8), each filtered by the day-of-week-in-month (DoWinMo) of the signal day.
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};
use super::rbtie::mk;

pub const REVIEWED: &[&str] = &["scbrower_long", "scbrower_short"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const NOTE: &str = "[Adapted: 1 unstated detail, R4 #297; declared adaptation: the long and short patterns as two cards] Norgate continuous S&P 500 futures (&SP, back-adjusted daily bars; the article used back-adjusted, forward-rolled Genesis data 4/1982-3/1996). Norgate continuous futures, ends 2020-09-11; roll method not stated by the vendor. Unstated in the article: the unit of 'points'; vault reading: TradeStation points = 0.01 S&P index points, so 30 points = 0.30 (as the vault's McNutt reading, kftsm): read as whole index points, gaps of 30-70 S&P points would almost never occur in 1982-96 and could not give the article's 412 trades. DoWinMo = 10 x (the week-of-month occurrence of the weekday, 1-5) + weekday (Monday 1 .. Friday 5) of the SIGNAL day ('all references to DoWinMo numbers identify the day before the trade'); '>50' = any fifth occurrence. Each trade enters on the next day (at the open, or at the stop/limit level when the day reaches it) and exits at that day's close (MOC); no stops. 'Only one trade is allowed in the same direction at the same time': on a day two buy patterns fire, the market-at-open entry (pattern 2) is the one taken. The article's system holds the long and the short patterns together (a long and a short can run the same day); the vault splits them into a long-side and a short-side card, which the article also reports separately (long $138,440; short $41,050, 68% profitable, $500 per trade). Costs: none (the article charged $100 per trade).";

pub fn catalogue() -> Vec<StrategyDef> {
    let d = |id, name, dir, rules, claim| StrategyDef { id, name, group: "S&C: The S&P 500 Seasonal Day Trade (Brower, V14:8)", session: 0, dir, status: Status::Live,
        needs_volume: false, needs_vix: false, needs_series: &[], book: "scbrower", rules, notes: NOTE, claim };
    vec![
        d("scbrower_long", "Brower S&P seasonal day trade, buy patterns 2, 5 and 7 on their day-of-week-in-month filters, out at the close", "Long", &[
            "E|\"Pattern 2 If tomorrow's open plus 30 points is less than today's close, then buy at market.\" DoWinMo 12, 15, 25, 42 (Figure 2)",
            "E|\"Pattern 5 If tomorrow's open plus 10 points is less than today's low, then buy at today's low stop.\" DoWinMo 12, 15, 21, 33, 35, 42, >50",
            "E|\"Pattern 7 If tomorrow's open minus 40 points is greater than today's close, then buy at today's low limit.\" DoWinMo 22, 33",
            "X|\"Each pattern is a day trade using a simple market-on-close (MOC) exit.\" \"No stops of any sort are used.\" (p.340-341)"],
          crate::connors::claim("n/a", "n/a", "$138,440 net on the long side (with $100 slippage and commission per trade)", "1 day", "S&P 500 futures daily 4/1982-3/1996 (p.341)")),
        d("scbrower_short", "Brower S&P seasonal day trade, sell patterns 6 and 8 on their day-of-week-in-month filters, out at the close", "Short", &[
            "E|\"Pattern 6 If tomorrow's open minus 20 points is greater than today's high, then sell at today's close stop.\" DoWinMo 21, 32, 41, 43, >50 (Figure 2)",
            "E|\"Pattern 8 If tomorrow's open plus 70 points is less than today's close, then sell at today's high limit.\" DoWinMo 14, 33, 34, 42",
            "X|\"Each pattern is a day trade using a simple market-on-close (MOC) exit.\" \"No stops of any sort are used.\" (p.340-341)"],
          crate::connors::claim("about 82", "68%", "$41,050 net, $500 per trade (with $100 slippage and commission)", "1 day", "S&P 500 futures daily 4/1982-3/1996 (p.341)")),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    reviewed(id).then_some(("scbrower_long", "Brower S&P seasonal day trade", if id == "scbrower_long" { "Buy patterns" } else { "Sell patterns" }))
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    reviewed(id).then(|| vec![ParamDef { key: "pt", label: "Value of one 'point' in index points (0.01 = TradeStation points)", def: 0.01, min: 0.01, max: 1., step: 0.01 }])
}

/// DoWinMo of a yyyy-mm-dd date: 10 x occurrence of the weekday in its month + weekday (Mon 1 .. Fri 5); None on a weekend.
pub fn dowinmo(d: &str) -> Option<i64> {
    let w = super::jbcdt2::weekday(d) + 1;
    let day: i64 = d.get(8..10)?.parse().ok()?;
    (w <= 5).then_some(10 * ((day - 1) / 7 + 1) + w)
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from > to || to >= x.c.len() { return None; }
    let pt = p.get("pt").copied().unwrap_or(0.01);
    let (o, h, l, c) = (&x.o, &x.h, &x.l, &x.c);
    let mut out = Vec::new();
    for t in from.max(1)..=to {
        let s = t - 1; // signal day; the DoWinMo filter is on it
        let Some(dw) = dowinmo(&x.r[s].date) else { continue };
        let on = |set: &[i64], fifth: bool| set.contains(&dw) || (fifth && dw > 50);
        let trade = if id == "scbrower_long" {
            if on(&[12, 15, 25, 42], false) && o[t] + 30. * pt < c[s] { Some((o[t], "open", "Pattern 2 (buy at market)")) }
            else if on(&[12, 15, 21, 33, 35, 42], true) && o[t] + 10. * pt < l[s] && h[t] >= l[s] { Some((l[s], "stop", "Pattern 5 (buy stop at the prior low)")) }
            else if on(&[22, 33], false) && o[t] - 40. * pt > c[s] && l[t] <= l[s] { Some((l[s], "limit", "Pattern 7 (buy limit at the prior low)")) }
            else { None }
        } else if on(&[21, 32, 41, 43], true) && o[t] - 20. * pt > h[s] && l[t] <= c[s] { Some((c[s], "stop", "Pattern 6 (sell stop at the prior close)")) }
        else if on(&[14, 33, 34, 42], false) && o[t] + 70. * pt < c[s] && h[t] >= h[s] { Some((h[s], "limit", "Pattern 8 (sell limit at the prior high)")) }
        else { None };
        if let Some((px, kind, why)) = trade {
            out.push(mk(if id == "scbrower_long" { 1. } else { -1. }, t, px, kind, t, c[t], why, false, s));
        }
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn dowinmo_numbers() {
        assert_eq!(dowinmo("1996-03-01"), Some(15)); // first Friday
        assert_eq!(dowinmo("1996-03-11"), Some(21)); // second Monday
        assert_eq!(dowinmo("1996-01-29"), Some(51)); // fifth Monday
        assert_eq!(dowinmo("1996-03-02"), None);     // Saturday
    }
}
