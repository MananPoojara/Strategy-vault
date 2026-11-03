//! Stan Weinstein, "Secrets for Profiting in Bull and Bear Markets" (McGraw-Hill, 1988).
//! Only the no-load mutual-fund switching rule (Ch. 9, printed pp. 310-315) is mechanical:
//! buy when the fund moves above its 30-week MA, switch to cash when it breaks below it.
//! Everything else in the book (stage analysis, resistance breakouts, support-based stops,
//! divergences) needs chart judgement and is catalogued as not testable.
//! Source and checks: library/swsp/.
use crate::connors::{Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "swsp";
/// Admitted ids (authors/lanea/singles/decisions.json).
pub const REVIEWED_LANEA: &[&str] = &["swsp_fund_30w"];
pub fn reviewed_lanea(id: &str) -> bool { REVIEWED_LANEA.contains(&id) }

const JUDGE_STAGE: &str = "Stage 1/2/3/4 classification, 'resistance', 'support' and 'significant' breaks are read off the chart; the book gives no formula for any of them.";
const JUDGE_STOP: &str = "Initial and trailing sell-stops are placed under the prior 'significant' low / support zone / trendline chosen on the chart; no formula is given.";

pub fn catalogue() -> Vec<StrategyDef> {
    let d = |id, name, group, dir, status, rules, notes| StrategyDef {
        id, name, group, session: 0, dir, status, needs_volume: false, needs_vix: false, needs_series: &[],
        book: BOOK, rules, notes, claim: None,
    };
    use Status::*;
    vec![
        d("swsp_fund_30w", "Fund switching on the 30-week MA (investor's way)", "Ch. 9 Odds, Ends, and Profits", "Long", Live,
          &["F|\"Every Friday, plot the closing price ... calculate and keep up a 30-week MA of that data.\" (p. 270, method p. 313)",
            "E|\"when the fund moved above its 30-week MA at $13.50, a low-risk buy signal was flashed.\" (p. 311)",
            "X|\"when Stein Roe broke below its MA, it was time to move out of the aggressive fund into a money market fund\" (p. 311); \"if it closes below your sell-stop point, you'll immediately call the fund and sell it\" (p. 311)",
            "X|\"The simple rules I've just shown you for buying and selling when the 30-week MA is violated works very well for investors.\" (p. 315)"],
          "Weekly close = close of the last trading day of each Monday-Sunday week (the book plots Friday closes); MA = simple 30-week mean of weekly closes. Buy at that weekly close when it is above the MA and the previous weekly close was not; sell at the first weekly close below the MA. Fill = that day's close (the book quotes the signal price as the fill, $13.50). Long/cash only; cash earns nothing in the engine. NOT coded: the initial mental stop 'under the low at point A' (a chart-picked low, p. 311) and the fund-selection relative-strength tips (pp. 316-317). The book applies the rule to no-load stock funds; here the fund is replaced by the index (NIFTY 50 / SPX / DJI). Dividend adjustment footnote (p. 313) not needed for price indices."),
        d("swsp_fund_30d_trader", "Short-term fund trading on the 30-day MA", "Ch. 9 Odds, Ends, and Profits", "Long", NotTestable("Entry needs a chart-read resistance breakout: 'buy the fund only if it both crosses above the short-term MA and breaks out above resistance' (p. 316)."),
          &["F|\"Traders, however, must use a daily chart with a 30-day MA.\" (p. 315)",
            "E|\"buy the fund only if it both crosses above the short-term MA and breaks out above resistance\" (p. 316)",
            "X|\"you should always trade out of the fund if it breaks below the 30-day MA\" (p. 316)"],
          "Exit is mechanical, entry is not (resistance is drawn by eye). Not coded."),
        d("swsp_sector_funds", "Sector-fund switching (Fidelity Select)", "Ch. 9 Odds, Ends, and Profits", "Long", NotTestable("Same 30-week rule but on a universe of 35 sector funds chosen by eye/relative strength (pp. 319-320); no sector-fund data and the selection is judgement."),
          &["E|\"gave a strong buy signal ... when it first moved above its 30-week MA. It broke out above resistance\" (p. 320)"], "Not coded."),
        d("swsp_stage2_breakout", "Stage 2 breakout buy (investor) / continuation buy (trader)", "Ch. 3-5 Buying", "Long", NotTestable(JUDGE_STAGE),
          &["E|\"when a stock initially moves out of its Stage 1 base and enters Stage 2\" (p. 59)",
            "E|\"when the stock drops back close to its MA and consolidates. It then breaks out anew above the top of its resistance zone\" (p. 61)",
            "F|volume, relative strength, 'the less resistance the better', triple confirmation (Ch. 4-5) all read by eye"],
          "Not coded: Stage, base, resistance zone and consolidation have no formula in the book."),
        d("swsp_sell_stops", "Investor / trader sell-stops", "Ch. 6 When to Sell", "Long", NotTestable(JUDGE_STOP),
          &["X|\"place the initial sell-stop under the significant support (trough) prior to the breakout\" (Ch. 6, pp. 178-208)",
            "X|10-15% percentage stops are explicitly rejected (p. 181)"],
          "Not coded."),
        d("swsp_short_sale", "Stage 4 short sale and buy-stops", "Ch. 7 Selling Short", "Short", NotTestable(JUDGE_STAGE),
          &["E|short on a breakdown below support into Stage 4 with the 30-week MA turning down (Ch. 7, pp. 215-267)",
            "X|buy-stop above the prior rally peak / trendline (Ch. 7)"],
          "Not coded."),
        d("swsp_dji_gauge", "DJIA stage analysis on the 30-week MA (market gauge)", "Ch. 8 Long-Term Indicators", "Long/Short", NotTestable("A market-condition gauge, not a trade: 'Once a Stage 3 top formed and the MA stopped rising, it was time to be careful. The final negative signal was a significant break below the 30-week MA' (p. 273); 'significant', 'Stage 3' and 'MA leveling out' are judgement and no instrument is traded."),
          &["F|\"Every Friday, plot the closing price ... keep up a 30-week MA\" (p. 270)",
            "X|\"when the Dow breaks below its 30-week MA and breaks down into Stage 4, it's time to become very defensive\" (p. 270)"],
          "Not coded. The book's dated DJI calls (Aug 1984 above the MA, Oct 1987 break near 2,450) are compared with swsp_fund_30w on DJI in the README."),
        d("swsp_ad_line", "NYSE advance-decline line divergence", "Ch. 8 Long-Term Indicators", "Long/Short", NotTestable("Divergence ('loses upside momentum', 'several months', 'important top formations') is judged on the chart and the book attaches no trade; also needs NYSE advance/decline data."),
          &["F|cumulative NYSE advances minus declines (p. 276)"], "Not coded."),
        d("swsp_momentum_index", "Momentum Index (200-day MA of NYSE net advances)", "Ch. 8 Long-Term Indicators", "Long/Short", NotTestable("Zero-line cross is called a 'favorable longer-term signal' / 'bearish indication' (p. 285) with no instrument, entry or exit; also needs NYSE advance/decline data."),
          &["F|MI = 200-day average of NYSE (advances - declines) (p. 283)", "F|rule 1: crossing of the zero line in either direction is the most important signal (p. 285)"], "Not coded."),
        d("swsp_futures", "Futures chart patterns (head-and-shoulders, 40-day MA)", "Ch. 9 Odds, Ends, and Profits", "Long/Short", NotTestable("Entries on head-and-shoulders necklines and stops at chart points B-E (pp. 331-333) are judgement; also futures data."),
          &["E|\"If you purchased orange juice contracts when the futures moved above the neckline\" (p. 331)"], "Not coded."),
        d("swsp_options", "Options tactics", "Ch. 9 Odds, Ends, and Profits", "Long/Short", NotTestable("needs option chains; tips only (pp. 320-330)"), &["E|buy calls on Stage 2 breakouts (p. 322 ff.)"], "Not coded."),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    let f = ("swsp_fund", "Weinstein fund switching");
    match id {
        "swsp_fund_30w" => Some((f.0, f.1, "30-week MA, investor")),
        "swsp_fund_30d_trader" => Some((f.0, f.1, "30-day MA, trader · not testable")),
        "swsp_sector_funds" => Some((f.0, f.1, "Sector funds · not testable")),
        "swsp_stage2_breakout" | "swsp_sell_stops" | "swsp_short_sale" => Some(("swsp_stage", "Weinstein stage analysis", "not testable")),
        "swsp_dji_gauge" | "swsp_ad_line" | "swsp_momentum_index" => Some(("swsp_market", "Weinstein market indicators", "not testable")),
        "swsp_futures" | "swsp_options" => Some(("swsp_other", "Weinstein other vehicles", "not testable")),
        _ => None,
    }
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    match id {
        "swsp_fund_30w" => Some(vec![ParamDef { key: "weeks", label: "Moving average length (weeks)", def: 30., min: 10., max: 50., step: 5. }]),
        _ => None,
    }
}

/// Exact window mean of the last n weekly closes.
fn sma(v: &[f64], n: usize) -> Vec<f64> {
    (0..v.len()).map(|i| if i + 1 < n { f64::NAN } else { v[i + 1 - n..=i].iter().sum::<f64>() / n as f64 }).collect()
}

/// Days since 1970-01-01 for "YYYY-MM-DD" (proleptic Gregorian).
fn days(date: &str) -> i64 {
    let y: i64 = date[0..4].parse().unwrap_or(1970);
    let m: i64 = date[5..7].parse().unwrap_or(1);
    let d: i64 = date[8..10].parse().unwrap_or(1);
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let mp = (m + 9) % 12;
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146097 + doe - 719468
}
/// Monday-based week number (1970-01-01 was a Thursday).
fn week(date: &str) -> i64 { (days(date) + 3).div_euclid(7) }
fn is_friday(date: &str) -> bool { (days(date) + 3).rem_euclid(7) == 4 }

/// Bar indices that close a week. A bar closes its week when the next bar (exchange calendar) is in a
/// later week; the very last bar of the data closes its week only if it is a Friday.
pub fn week_ends(x: &Ctx) -> Vec<usize> {
    let n = x.r.len();
    (0..n).filter(|&i| if i + 1 < n { week(&x.r[i + 1].date) != week(&x.r[i].date) } else { is_friday(&x.r[i].date) }).collect()
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if id != "swsp_fund_30w" || from >= to || to >= x.r.len() { return None; }
    let n = p.get("weeks").copied().unwrap_or(30.).round().max(2.) as usize;
    let we: Vec<usize> = week_ends(x).into_iter().filter(|&i| i <= to).collect();
    let wc: Vec<f64> = we.iter().map(|&i| x.c[i]).collect();
    let ma = sma(&wc, n);
    let mut out = vec![];
    let mut cur: Option<Trade> = None;
    for k in 1..we.len() {
        let i = we[k];
        if i < from || !ma[k].is_finite() { continue; }
        match cur.as_mut() {
            Some(t) => if wc[k] < ma[k] {
                t.exit_i = i; t.exit_px = x.c[i]; t.signal_i = i; t.reason = "Weekly close below the 30-week MA".into();
                out.push(cur.take().unwrap());
            },
            None => if wc[k] > ma[k] && ma[k - 1].is_finite() && wc[k - 1] <= ma[k - 1] {
                cur = Some(Trade { dir: 1., legs: vec![Leg { i, px: x.c[i], w: 1. }], exit_i: i, exit_px: x.c[i], reason: String::new(),
                    entry_kind: "close", signal_i: i, open: false, exits: vec![] });
            },
        }
    }
    if let Some(mut t) = cur { t.exit_i = to; t.exit_px = x.c[to]; t.signal_i = to; t.reason = "open".into(); t.open = true; out.push(t); }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;
    fn rows(closes: &[f64]) -> Vec<Row> {
        // one bar per weekday starting Monday 2001-01-01
        let mut out = vec![]; let mut d = days("2001-01-01");
        for &c in closes {
            while (d + 3).rem_euclid(7) >= 5 { d += 1; }
            let (y, m, dd) = civil(d);
            out.push(Row { ticker: "T".into(), date: format!("{y:04}-{m:02}-{dd:02}"), open: c, high: c, low: c, close: c, volume: None });
            d += 1;
        }
        out
    }
    fn civil(z: i64) -> (i64, i64, i64) {
        let z = z + 719468; let era = z.div_euclid(146097); let doe = z - era * 146097;
        let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365; let y = yoe + era * 400;
        let doy = doe - (365 * yoe + yoe / 4 - yoe / 100); let mp = (5 * doy + 2) / 153;
        let d = doy - (153 * mp + 2) / 5 + 1; let m = if mp < 10 { mp + 3 } else { mp - 9 };
        (if m <= 2 { y + 1 } else { y }, m, d)
    }
    #[test]
    fn weekday_math() { assert!(is_friday("2010-12-31")); assert_eq!(week("2024-01-01"), week("2024-01-07")); assert_ne!(week("2024-01-07"), week("2024-01-08")); }
    #[test]
    fn buys_on_weekly_cross_and_sells_on_weekly_close_below() {
        // 30 weeks at 100, then 2 weeks at 110 (cross up), then 1 week at 90 (break below)
        let mut c = vec![]; for _ in 0..30 { c.extend([100.; 5]); } for _ in 0..2 { c.extend([110.; 5]); } c.extend([90.; 5]); c.extend([90.; 5]);
        let r = rows(&c); let x = Ctx::new(&r, &r, &[]);
        let t = run("swsp_fund_30w", &x, 0, r.len() - 1, &Params::new()).unwrap();
        assert_eq!(t.len(), 1);
        assert_eq!(t[0].legs[0].i, 30 * 5 + 4); // Friday of week 31
        assert_eq!(t[0].exit_i, 32 * 5 + 4); assert_eq!(t[0].exit_px, 90.); assert!(!t[0].open);
    }
}

// Instrument admission: Stan Weinstein (swsp, lane A) 2026-10-26 (lane B) block, moved from instrument_policy.rs on 31 Oct 2026 (lane ST).
// Admission only for the module REVIEWED lists (authors/lanea/singles/decisions.json); kept in one block to merge.
// Runs on actual indices, ETFs and stocks, US and India (index_etf_or_stock, nifty50 basket, declared application).
pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book=="swsp" && crate::books::swsp::reviewed_lanea(def.id),
    label: "Weinstein's switching rule was written for no-load stock mutual funds; runs on indices, ETFs and single stocks (US and India) are a declared application, one unit per trade.",
    basis: "Stan Weinstein / Secrets for Profiting in Bull and Bear Markets (1988)",
};
