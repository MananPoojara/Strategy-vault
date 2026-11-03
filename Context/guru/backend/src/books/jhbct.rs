//! J. D. Hamon, *Breakthroughs in Commodity Technical Analysis* (Windsor Books, 195 scanned pages). Source + audit:
//! library/jhbct/. Almost all of the book is chart geometry (true trend lines, gravity lines, angle lines, congestions,
//! Fibonacci dividers) and is not mechanical. The mechanical rules are in Ch.7 "Improving Your Charting Ability"
//! (p.121-124): the weekly rule on two calendar weeks - the original intraday stop-and-reverse version is already in the
//! vault (aanwr_weekly_rule / kftsm_donchian_4wk with 2 weeks), so only the book's two close-based variations are new here.
use crate::books::aanwr::week;
use crate::connors::{Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "jhbct";
// Admitted 2026-10-24 (authors/hamon/admission/SOURCE-CONTRACT.md): complete close-based weekly rules with independent US/India ledgers.
pub const REVIEWED: &[&str] = &["jhbct_weekly_close", "jhbct_weekly_close4"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const NOTE: &str = "Book market: futures (Sept 82 Corn fig 51, July 82 Coffee fig 52; daily charts with weekly high/low blocks). \
'Two weeks' = the two previous full calendar weeks (Monday-Sunday) before the current week, the book's 'double charting' weekly blocks (p.119-121, 'two weeks, or ten trading days' p.121); \
the window needs data from before its first week. A close exactly at the range boundary is not beyond it. Fills at the close of the signal day (book: 'on close only', 'go in on a close'). Runs on the indices (instrument swap).";

pub fn catalogue() -> Vec<StrategyDef> {
    let d = |id, name, group, dir, status, rules, notes| StrategyDef {
        id, name, group, session: 0, dir, status, needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK, rules, notes, claim: None,
    };
    let nt = |id, name, group, why, rule| StrategyDef { id, name, group, session: 0, dir: "Long/Short", status: Status::NotTestable(why), needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK, rules: rule, notes: "Seen and transcribed; not mechanical.", claim: None };
    vec![
        d("jhbct_weekly_close", "Weekly rule, close only (two-week range, stop and reverse on the close)", "Ch.7 Improving Your Charting Ability", "Long/Short", Status::Live, &[
            "E|\"One strategy is to trade when the high (or low) of two weeks is passed and only get out when the high (or low) of two weeks is passed in the opposite direction. It is stop and reverse, always being in the market. These are the original rules.\" (p.122)",
            "E|\"Using the same two weeks on coffee ... If it is made on close only, it does a little better.\" (p.122) - long on a close above the two-week high, short on a close below the two-week low, reversing at the close.",
            "X|Exit only by the opposite signal (\"only get out when the high (or low) of two weeks is passed in the opposite direction\", p.122), which reverses at that close.",
        ], NOTE),
        d("jhbct_weekly_close4", "Weekly rule entry on a close, exit on a close beyond the last four closes", "Ch.7 Improving Your Charting Ability", "Long/Short", Status::Live, &[
            "E|\"Some people use the weekly rule of closing beyond a certain number of weeks as an entrance method, then exit by other rules. Suppose we decide to go in on a close beyond the last two weeks price range\" (p.122)",
            "X|\"and exit when price closes beyond the last four closes.\" (p.122) - long: close below the lowest of the previous four closes; short: above the highest. Exit at that close; a new entry needs a new close beyond the two-week range (may be the same bar).",
        ], NOTE),
        nt("jhbct_weekly_original", "Weekly rule, original (two-week stop and reverse)", "Ch.7 Improving Your Charting Ability",
            "duplicate: the vault already runs it as aanwr_weekly_rule (p.n_buy=2, p.n_sell=2) and kftsm_donchian_4wk (p.weeks=2)",
            &["E|\"trade when the high (or low) of two weeks is passed ... stop and reverse\" (p.122)"]),
        nt("jhbct_true_trend_lines", "True Trend Lines (TTL)", "Ch.2", "discretionary: lines drawn from congestion 'thrust' measured with dividers and judged congestions (p.30-41)",
            &["E|\"Trade when price goes beyond the previous low, high or congestion\" (p.33)"]),
        nt("jhbct_gravity_lines", "Gravity Lines", "Ch.3", "discretionary: gravity lines drawn by hand from congestion half-heights (p.49-60)",
            &["E|\"Enter only when there is a change of trend, confirmed by a break beyond the previous pivot's low, high, or congestion\" (p.56)"]),
        nt("jhbct_congestion_method", "Hamon's Congestion Method (0-2 lines)", "Ch.4-5", "discretionary: congestion identification is judgement ('Was it or was it not a congestion?', p.84)",
            &["E|\"Enter not more than 20% of the range away from the bottom line or top line\" (p.74)"]),
        nt("jhbct_fast_fib", "Fast Fib Analysis", "Ch.6", "discretionary: divider/Fibonacci target projections (p.89-118)", &["X|Fibonacci price targets"]),
        nt("jhbct_dominant_cycle", "The Quick Dominant Cycle", "Ch.8", "discretionary: swings of 'about the same size' picked with dividers (p.125-129)",
            &["E|\"Enter a trade in the direction opposite from the direction the market has been going each time a cycle time period is reached\" (p.127)"]),
        nt("jhbct_long_range_day", "Long range day indicator", "Ch.9", "not exact: 'days that are about twice the average size', comparison with earlier turning points, 'Do not use this in congestions' (p.137)",
            &["E|\"Wait for a close beyond the last two closes in the direction of the trade. Use this to enter the market after a long range day.\" (p.137)",
              "X|\"Do not exit until another long range day gives a signal.\" (p.137)"]),
        nt("jhbct_dma", "DMA (Discretionary Moving Average) method", "Ch.10", "discretionary by name: market type (congestion / labored / swinging / runaway) is chosen by the trader (p.139-150)",
            &["E|\"buy or sell when the four crosses the nine, but only use the longer term moving average for a directional indicator\" (p.145)"]),
        nt("jhbct_fast_or_slow", "Fast or Slow trading rules (Pork Bellies, Yen, T-Bonds, Wheat, Cotton, Gold)", "Ch.11", "exits use 70/80-degree angle lines whose slope depends on chart scaling the book does not define (p.178-180)",
            &["E|Wheat (second set): \"Change of Trend Rule ... is on a close beyond the last eleven closes. Trade on the close on the twelfth day.\" (p.180)",
              "X|\"Exit when the 70 degree line is crossed.\" (p.180)"]),
        nt("jhbct_median_lines", "Reverse / second-swing median lines", "Ch.12", "discretionary: hand-drawn median lines (p.183-195)", &["E|Median line intercepts"]),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    match id {
        "jhbct_weekly_close" => Some(("jhbct_weekly", "Hamon weekly rule (two weeks)", "Close only, stop and reverse")),
        "jhbct_weekly_close4" => Some(("jhbct_weekly", "Hamon weekly rule (two weeks)", "Close entry, four-close exit")),
        "jhbct_weekly_original" => Some(("jhbct_weekly", "Hamon weekly rule (two weeks)", "Original (duplicate)")),
        s if s.starts_with("jhbct_") => Some(("jhbct_charting", "Hamon chart methods", "Not mechanical")),
        _ => None,
    }
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    match id {
        "jhbct_weekly_close" => Some(vec![p("weeks", "Calendar weeks in the range", 2.0, 1.0, 12.0, 1.0)]),
        "jhbct_weekly_close4" => Some(vec![
            p("weeks", "Calendar weeks in the entry range", 2.0, 1.0, 12.0, 1.0),
            p("exit_closes", "Exit: close beyond the last N closes", 4.0, 1.0, 20.0, 1.0),
        ]),
        _ => None,
    }
}

fn tr(dir: f64, i: usize, px: f64) -> Trade {
    Trade { dir, legs: vec![Leg { i, px, w: 1.0 }], exit_i: i, exit_px: px, reason: String::new(), entry_kind: "close", signal_i: i, open: false, exits: vec![] }
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !matches!(id, "jhbct_weekly_close" | "jhbct_weekly_close4") { return None; }
    let schema = params(id)?;
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).unwrap().def);
    let n = x.c.len();
    if n == 0 || to >= n || from > to { return Some(vec![]); }
    let nw = (g("weeks") as i64).max(1);
    let four = id == "jhbct_weekly_close4";
    let ne = if four { (g("exit_closes") as usize).max(1) } else { 0 };
    let (h, l, c) = (&x.h, &x.l, &x.c);
    let wk: Vec<i64> = x.r.iter().map(|r| week(&r.date)).collect();
    let (mut start, mut curw) = (0usize, 0usize);
    let mut out = vec![];
    let mut cur: Option<Trade> = None;
    for i in 0..=to {
        if wk[i] != wk[curw] { curw = i; }
        while wk[start] < wk[i] - nw { start += 1; }
        // exit rule (four-close variant) first
        if four && i >= ne {
            if let Some(q) = cur.as_ref() {
                let w = &c[i - ne..i];
                let hit = if q.dir > 0.0 { c[i] < w.iter().copied().fold(f64::INFINITY, f64::min) } else { c[i] > w.iter().copied().fold(f64::NEG_INFINITY, f64::max) };
                if hit {
                    let mut q = cur.take().unwrap();
                    q.exit_i = i; q.exit_px = c[i]; q.signal_i = i; q.reason = format!("Close beyond the last {ne} closes"); out.push(q);
                }
            }
        }
        if wk[0] >= wk[i] - nw || start >= curw { continue; }
        let hh = h[start..curw].iter().copied().fold(f64::NEG_INFINITY, f64::max);
        let ll = l[start..curw].iter().copied().fold(f64::INFINITY, f64::min);
        let d = if c[i] > hh { 1.0 } else if c[i] < ll { -1.0 } else { continue };
        if cur.as_ref().map_or(false, |q| q.dir == d) { continue; }
        if let Some(mut q) = cur.take() { q.exit_i = i; q.exit_px = c[i]; q.signal_i = i; q.reason = "Close beyond the opposite two-week extreme: reverse".into(); out.push(q); }
        cur = Some(tr(d, i, c[i]));
    }
    if let Some(mut q) = cur { q.exit_i = to; q.exit_px = c[to]; q.signal_i = to; q.reason = "open".into(); q.open = true; out.push(q); }
    out.retain(|t| t.legs[0].i >= from);
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;
    #[test]
    fn close_above_two_week_high_goes_long_and_four_close_exit() {
        // 2000-01-03 is a Monday; flat 100 bars for three weeks, then a close above the two-week high
        let day = |k: usize| { let d = 3 + k + 2 * (k / 5); (d, if d > 31 { format!("2000-02-{:02}", d - 31) } else { format!("2000-01-{d:02}") }) };
        let mut r: Vec<Row> = (0..25).map(|k| Row { ticker: "T".into(), date: day(k).1, open: 100., high: 101., low: 99., close: 100., volume: None }).collect();
        r[16].close = 101.5; r[16].high = 102.0; // week 4, Tuesday: close > 101 (two-week high)
        r[17].close = 101.6; r[17].high = 102.0;
        r[18].close = 99.5; // below the lowest of the previous four closes (100) -> exit
        let x = Ctx::new(&r, &r, &[]);
        let ts = run("jhbct_weekly_close4", &x, 0, 24, &Params::new()).unwrap();
        assert_eq!((ts[0].dir, ts[0].legs[0].i, ts[0].legs[0].px, ts[0].exit_i, ts[0].exit_px), (1.0, 16, 101.5, 18, 99.5));
    }
}

// Instrument admission: Hamon 2026-10-24 block, moved from instrument_policy.rs on 31 Oct 2026 (lane ST).
// Admission only for the module REVIEWED lists (authors/hamon/admission/SOURCE-CONTRACT.md); kept in one block to merge.
// Runs on actual indices, ETFs and stocks, US and India (index_etf_or_stock, nifty50 basket, declared application).
pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| matches!(def.book,"jhbct") && (crate::books::jhbct::reviewed(def.id)),
    label: "J. D. Hamon showed the weekly rule on futures charts (Sept 82 corn, July 82 coffee); runs on indices, ETFs and stocks, US and India, are a declared application, not the author's test. Cash-index shorts are hypothetical exposure.",
    basis: "J. D. Hamon, Breakthroughs in Commodity Technical Analysis (Windsor Books), Ch.7 Improving Your Charting Ability (p.119-124): the weekly rule on the two previous calendar weeks, close only (stop and reverse), and the close entry with the four-close exit. Reviewed in authors/hamon/admission/SOURCE-CONTRACT.md with independent chronological ledgers. Fills at the close; no costs.",
};
