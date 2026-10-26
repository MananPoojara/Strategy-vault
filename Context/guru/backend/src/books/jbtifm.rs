//! Jake Bernstein, "Trading the International Futures Markets" (2000). Book id `jbtifm`.
//! Source: research share "USB HDD 2/tRADING KNO-2/Jack Bernstein - Trading the International Future Markets
//! (trade-futures.com)/Trading The International Futures Markets.pdf" (360-page scan; printed page = PDF page - 33 in
//! Appendix III). Reading notes: claude-continuation-20261020/authors/bernstein/agents/tifm/.
//! Testable: the 2000ST range breakout and the 9/18 moving-average crossover, both printed as TradeStation code.
use super::jb624::{finish, getp, mark_open, new_trade, stop_fill};
use crate::connors::{claim as c, sma, true_range, Ctx, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "jbtifm";

macro_rules! def {
    ($id:expr, $name:expr, $group:expr, $dir:expr, $status:expr, $rules:expr, $notes:expr) => {
        StrategyDef { id: $id, name: $name, group: $group, session: 0, dir: $dir, status: $status, needs_volume: false,
            needs_vix: false, needs_series: &[], book: BOOK, rules: $rules, notes: $notes, claim: None }
    };
}

pub fn catalogue() -> Vec<StrategyDef> {
    use Status::{Live, NotTestable};
    vec![
        StrategyDef { claim: c("82", "66%", "$464.60 (1 FTSE-100 contract)", "", "FTSE-100 futures 1984-1999, PF 1.83"),
          ..def!("jbtifm_2000st", "2000ST range breakout · stop and reverse at close ± 0.62 × 3-day average range, 28-day momentum trend", "Ch.10 · Volatility Breakout Systems", "Long/Short", Live,
            &["F|\"ATR.Val = Average (TrueRange, ATR.Bars) * ATR.mult\" with ATR.Bars 3, ATR.Mult 0.62; \"MOM.Val = Momentum (Close, MOM.Bars)\" with MOM.Bars 28; \"if MOM.Val > MOM.Val [MOM.Back] then MOM.Trnd = +01; if MOM.Val < MOM.Val [MOM.Back] then MOM.Trnd = -01\" with MOM.Back 10 (Appendix III, p.298-299)",
              "E|\"if MOM.Trnd > +00 ... le.Price = Close + ATR.val ... if MarketPosition <> +01 then Buy at le.Price stop\"; \"if MOM.Trnd < +00 ... se.Price = Close - ATR.val ... if MarketPosition <> -01 then Sell at se.Price stop\" (p.299; HLC.Mode 0 = Close, the default)",
              "X|Stop and reverse only: the \"jb.Trailing Stop %\" include (MinFloor 1000, Percent 0.90) sits inside a comment block in the printed code (p.298), so it is not active"],
            "Book test: daily FTSE-100 futures 1984-1999 (Fig 10.2, p.192): 82 trades, 66% winners, net $38,097. The orders are placed after each close for the next day; a stop fills at the open if the market opens through it. Index, ETF and stock runs (US and India) are a declared application, not the author's test. Flat at the start of the window; no new position is opened on its last bar.") },
        def!("jbtifm_ma_9_18", "9/18 moving-average crossover · stop and reverse on the close", "Ch.6 · Technical Methods (Appendix III code)", "Long/Short", Live,
            &["F|\"Input: Length1(9),Length2(18)\" (Appendix III, p.307)",
              "E|\"IF CurrentBar > 1 and Average(Close,Length1) crosses over Average(Close,Length2) Then Buy on Close\" (p.307)",
              "X|\"IF CurrentBar > 1 and Average(Close,Length1) crosses below Average(Close,Length2) Then Sell on Close\" (p.307): sell reverses to short; always in the market, no stop"],
            "No results printed. Figure 6.1 shows 9- and 18-month averages on a monthly CAC-40 chart; the printed code has no bar size, so the vault runs it on daily bars [declared application]. A cross = the 9-bar average above the 18-bar average after being at or below it on the previous bar (below: mirror); averages within 1e-9 of price of each other count as equal. Index, ETF and stock runs (US and India) are a declared application. No new position on the window's last bar."),
        def!("jbtifm_jb_gap", "JB GAP system (FTSE-100)", "Ch.10 · JB GAP System", "Long/Short", NotTestable("the exit is not fully printed: \"the number of days to exit (29)\" (p.196) is not defined, the code calls first-profitable-open/close and market-on-close exit functions that are not printed (p.304), and the US$2,950 stop is in FTSE contract dollars with no contract size given"),
            &["E|Long when today opens more than 19 ticks below the lowest low of the last 1 day, buy stop 2 ticks above that low; short when it opens more than 8 ticks above the highest high of the last 20 days, sell stop 2 ticks below it (p.194; code p.304-306)",
              "F|Fig 10.4 (p.195): FTSE-100 1984-1999, 199 trades, 70%, avg $248.04, PF 1.64 (the text says 77% and $985)"], ""),
        def!("jbtifm_dual_orbit", "Dual Orbit system (FTSE-100)", "Ch.10 · Trading Systems", "Long/Short", NotTestable("the printed code calls a trailing-stop and a crossover helper function that are not printed, so the exits cannot be reproduced (Appendix III, p.302-303)"),
            &["F|Fig 10.1: FTSE-100, 91 trades, 62%, avg $432.09"], ""),
        def!("jbtifm_add", "ADD system (FTSE-100)", "Ch.10 · Trading Systems", "Long/Short", NotTestable("the A/D function's mode and the trailing stop are not printed, and the printed code buys when A/D crosses below its 28-bar average, the opposite of the text (Appendix III, p.308)"),
            &["F|Fig 10.3: FTSE-100, 90 trades, 76%"], ""),
    ]
}

/// Admission (bernstein-g3, 2026-10-22): reviewed with independent chronological ledgers.
pub const REVIEWED: &[&str] = &["jbtifm_2000st", "jbtifm_ma_9_18"];
pub fn reviewed_daily(id: &str) -> bool {
    REVIEWED.contains(&id)
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    Some(match id {
        "jbtifm_2000st" => ("jbtifm_2000st", "2000ST range breakout (International Futures)", "Book inputs 3 / 0.62 / 28 / 10"),
        "jbtifm_ma_9_18" => ("jbtifm_ma", "9/18 moving-average crossover (International Futures)", "Stop and reverse on the close"),
        "jbtifm_jb_gap" | "jbtifm_dual_orbit" | "jbtifm_add" => ("jbtifm_other", "International Futures · FTSE systems not reproducible", "Not testable"),
        _ => return None,
    })
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    Some(match id {
        "jbtifm_2000st" => vec![p("atr_bars", "Average true range length (bars)", 3., 1., 20., 1.), p("atr_mult", "Range multiplier", 0.62, 0.1, 3., 0.01),
            p("mom", "Momentum length", 28., 2., 60., 1.), p("mom_back", "Momentum look-back for the trend", 10., 1., 40., 1.)],
        "jbtifm_ma_9_18" => vec![p("fast", "Fast average (bars)", 9., 2., 50., 1.), p("slow", "Slow average (bars)", 18., 3., 200., 1.)],
        _ => return None,
    })
}

/// Close the open trade at `px` on bar i and, unless i is the window's last bar, open the opposite one there.
fn reverse(out: &mut Vec<Trade>, cur: &mut Option<Trade>, i: usize, to: usize, d: f64, px: f64, kind: &'static str, why: &str) {
    if let Some(t) = cur.take() {
        out.push(finish(t, i, px, i, why));
    }
    if i < to {
        *cur = Some(new_trade(d, i, px, kind, i));
    }
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !id.starts_with("jbtifm_") || to >= x.c.len() || from > to {
        return None;
    }
    if params(id).is_none() {
        return if catalogue().iter().any(|s| s.id == id) { Some(vec![]) } else { None };
    }
    let g = |k: &str| getp(id, p, k);
    let u = |k: &str| g(k).round().max(1.0) as usize;
    let (mut out, mut cur): (Vec<Trade>, Option<Trade>) = (Vec::new(), None);
    let pos = |c: &Option<Trade>| c.as_ref().map_or(0.0, |t| t.dir);
    match id {
        "jbtifm_2000st" => {
            let atr: Vec<f64> = sma(&true_range(x.r), u("atr_bars")).iter().map(|v| v * g("atr_mult")).collect();
            let (n, back) = (u("mom"), u("mom_back"));
            let mom: Vec<f64> = (0..x.c.len()).map(|i| if i >= n { x.c[i] - x.c[i - n] } else { f64::NAN }).collect();
            // order placed after bar k's close for bar k + 1
            let order = |k: usize| -> Option<(f64, f64)> {
                if k < back || !mom[k - back].is_finite() || !mom[k].is_finite() || !atr[k].is_finite() {
                    return None;
                }
                if mom[k] > mom[k - back] { Some((1.0, x.c[k] + atr[k])) } else if mom[k] < mom[k - back] { Some((-1.0, x.c[k] - atr[k])) } else { None }
            };
            for i in from + 1..=to {
                if let Some((d, lvl)) = order(i - 1).filter(|(d, _)| pos(&cur) != *d) {
                    if let Some(px) = stop_fill(x, i, d, lvl) {
                        reverse(&mut out, &mut cur, i, to, d, px, "stop", "Opposite breakout (stop and reverse)");
                    }
                }
            }
        }
        _ => {
            let (f, s) = (sma(&x.c, u("fast")), sma(&x.c, u("slow")));
            for i in from.max(1)..=to {
                if !(f[i - 1].is_finite() && s[i - 1].is_finite() && f[i].is_finite() && s[i].is_finite()) {
                    continue;
                }
                // averages within float noise of each other (flat prices) count as equal
                let st = |k: usize| -> i8 { let e = 1e-9 * x.c[k].abs(); if f[k] - s[k] > e { 1 } else if s[k] - f[k] > e { -1 } else { 0 } };
                let (s0, s1) = (st(i - 1), st(i));
                let d = if s1 == 1 && s0 != 1 { 1.0 } else if s1 == -1 && s0 != -1 { -1.0 } else { continue };
                if pos(&cur) != d {
                    reverse(&mut out, &mut cur, i, to, d, x.c[i], "close", "Opposite crossover (stop and reverse)");
                }
            }
        }
    }
    mark_open(&mut out, cur, x, to);
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;
    #[test]
    fn ma_cross_reverses_and_never_opens_on_the_last_bar() {
        let closes = [5., 4., 3., 2., 3., 4., 5., 6., 5., 4., 3.];
        let r: Vec<Row> = closes.iter().enumerate().map(|(i, &c)| Row { ticker: "T".into(), date: format!("2000-01-{:02}", i + 1), open: c, high: c, low: c, close: c, volume: None }).collect();
        let x = Ctx::new(&r, &r, &[]);
        let mut p = Params::new();
        p.insert("fast".into(), 2.);
        p.insert("slow".into(), 3.);
        let t = run("jbtifm_ma_9_18", &x, 0, 10, &p).unwrap();
        // 2-bar avg crosses above the 3-bar avg at bar 5, below at bar 9
        assert_eq!((t[0].dir, t[0].legs[0].i, t[0].exit_i), (1.0, 5, 9));
        assert_eq!((t[1].dir, t[1].legs[0].i, t[1].open), (-1.0, 9, true));
        let t2 = run("jbtifm_ma_9_18", &x, 0, 9, &p).unwrap();
        assert_eq!(t2.len(), 1);
        assert!(!t2[0].open);
    }
}
