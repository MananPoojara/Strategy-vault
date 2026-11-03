//! Ned Davis Research, *Being Right or Making Money* (2nd ed., 2000; 168 scanned pages). Source + audit: library/ndbrm/.
//! Live: the price-only "S&P 500 percentage reversals" (chart 4-3), the 40-day TRIN indicator (chart 4-4) and the
//! A/D line vs its 25-day moving average (chart 2-30, India mapping). Most other NDR indicators need macro,
//! sentiment, new-high/low or bond data that the vault does not have, or are not fully specified.
use crate::connors::{claim, Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "ndbrm";
/// Admitted ids (authors/ndavis/admission/SOURCE-CONTRACT.md).
pub const REVIEWED: &[&str] = &["ndbrm_spx_reversals", "ndbrm_trin40", "ndbrm_trin40_nse", "ndbrm_ad_line_nse"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const NYSE_TR: &[&str] = &["NYSE_ADV_DEC.Advances", "NYSE_ADV_DEC.Declines", "NYSE_ADV_DEC.AdvVolume", "NYSE_ADV_DEC.DecVolume"];
const NSE_TR: &[&str] = &["NSE_ADV_DEC.Advances", "NSE_ADV_DEC.Declines", "NSE_ADV_DEC.AdvVolume", "NSE_ADV_DEC.DecVolume"];
const NSE_AD: &[&str] = &["NSE_ADV_DEC.Advances", "NSE_ADV_DEC.Declines"];

const TRIN_RULES: &[&str] = &[
    "F|\"Calculated by dividing the ratio of NYSE advances to declines by the ratio of advancing volume to declining volume and then smoothing the result over 40 days\" (p.88); chart scale x100, 'MKDS SMA40' (simple 40-day average).",
    "E|\"The indicator is bullish when the 40-day TRIN rises to 112 or rises above 97.7 and then reverses below that level\" (p.88; chart: 'BUY = Increase to 112 or rise to 97.7 and decline back below that level'). Buy at the close.",
    "X|\"It is bearish when the 40-day TRIN falls to 79.8 or falls below 90.9 and then reverses above that level\" (p.88; chart: 'SELL = Decline to 79.8 or fall to 90.9 and rise back above that level'). Sell at the close; 'S = Switch into Commercial Paper' (long/cash).",
];

pub fn catalogue() -> Vec<StrategyDef> {
    use Status::*;
    let d = |id, name, group, dir, status, series: &'static [&'static str], rules, notes, claim| StrategyDef {
        id, name, group, session: 0, dir, status, needs_volume: false, needs_vix: false, needs_series: series, book: BOOK, rules, notes, claim,
    };
    vec![
        d("ndbrm_spx_reversals", "S&P 500 percentage reversals (+8.4% buy / -7.2% sell, weekly closes)", "Ch 4 · Nine-indicator model (tape)", "Long", Live, &[], &[
            "E|\"The indicator is bullish when the S&P 500 reverses upward by 8.4%\" (p.87); chart 4-3: 'Signals Generated When S&P 500: Rises by 8.4% = Buy' - weekly closes ('Weekly Data 7/19/1968 - 5/19/2000'), rise measured from the lowest weekly close since the last sell.",
            "X|\"and bearish when it reverses downward by 7.2%\" (p.87): 'Falls by 7.2% = Sell' from the highest weekly close since the buy; 'S = Switch into Commercial Paper' (long/cash).",
        ], "Week = Monday-Sunday; the weekly close is the week's last bar (the calendar tells which bar that is). Signals and fills on that close. Before the first signal both the running low and the running high are tracked; whichever reversal comes first sets the mode. \
Chart 4-3 note 'An 8.4% Rise From 4/14/2000 Low of 1356.56 = 1470.51' confirms the rise is from the low weekly close. Book index S&P 500; also runs on NIFTY 50 (instrument swap).",
          claim("", "61% profitable long trades", "Gain/annum 12.1% (buy&hold 8.8%)", "", "Chart 4-3, S&P 500, signal dates 2/19/1969-5/19/2000")),
        d("ndbrm_trin40", "40-day TRIN (Arms Index) · NYSE breadth", "Ch 4 · Nine-indicator model (tape)", "Long", Live, NYSE_TR, TRIN_RULES,
          "Book market: Dow Jones Industrial Average (chart 4-4, daily 2/27/1979-5/19/2000). TRIN from NYSE advances/declines and their volumes (extra_data NYSE_ADV_DEC, 1965-2020). 'Rises above 97.7 and then reverses below' = yesterday >= 97.7 and today < 97.7; 'falls below 90.9 and then reverses above' = yesterday <= 90.9 and today > 90.9.",
          claim("", "86% profitable long trades", "Gain/annum 17.1% (buy&hold 12.7%); GPA on buys 27.1%", "time on buys 48.4%", "Chart 4-4, DJIA, signal dates 3/09/1979-5/19/2000")),
        d("ndbrm_trin40_nse", "40-day TRIN (Arms Index) · NSE breadth (India)", "Ch 4 · Nine-indicator model (tape)", "Long", Live, NSE_TR, TRIN_RULES,
          "India mapping: NSE advances/declines and advancing/declining volume (extra_data NSE_ADV_DEC, 1999-) on NIFTY 50. Thresholds unchanged.", None),
        d("ndbrm_nasdaq_ad", "NASDAQ A/D line vs its 25-day moving average", "Ch 2 · Bad breadth", "Long", NeedsData("NASDAQ daily advances and declines (NASDAQ A/D line), 1979-"), &[], &[
            "E|\"when the Advance/Decline Line has been above its 25-day smoothing, the NASDAQ has boomed ahead at a 42.1% annual rate\" (p.47; text: 'the Advance/Decline Line versus its own 25-day moving average')",
            "X|\"when the Advance/Decline Line has been below its 25-day smoothing, the NASDAQ has lost 2.9% per annum\" (p.47)",
        ], "Chart 2-30: NASDAQ Composite, 'Number of trades: 445'. The vault has NYSE and NSE breadth but not NASDAQ's.",
          claim("445", "", "+42.1%/yr above, -2.9%/yr below", "50% of time above", "Chart 2-30, NASDAQ Composite to 4/28/2000")),
        d("ndbrm_ad_line_nse", "A/D line vs its 25-day moving average · NSE breadth (India)", "Ch 2 · Bad breadth", "Long", Live, NSE_AD, &[
            "F|A/D line = cumulative sum of daily advances minus declines; \"its own 25-day moving average\" (p.47) - simple average (the book's definition of a moving average, p.76).",
            "E|Long while the A/D line is above its 25-day moving average (enter at the close it crosses above).",
            "X|Out while it is below (exit at the close it crosses below); equal = no change.",
        ], "India mapping of chart 2-30 (book: NASDAQ breadth on the NASDAQ Composite): NSE advances/declines on NIFTY 50.", None),
        // ---- need data the vault does not have
        d("ndbrm_new_highs_lows", "Weekly new highs/lows and net new highs (NYSE)", "Ch 4 · Nine-indicator model (tape)", "Long", NeedsData("weekly NYSE new 52-week highs and lows and issues traded"), &[], &[
            "E|\"Weekly new lows divided by issues traded ... bullish below 1.95%, bearish above 7.2%\" (p.85); \"Weekly net new highs ... bullish above 8.8%, bearish below -3.6%\" (p.86); chart 4-1 'New Highs/Issues Traded exceeds 30% for the first time in a year'"],
            "Zone indicators (neutral in between); no stand-alone trade list.", None),
        d("ndbrm_member_short", "NYSE member short ratio", "Ch 4 · Nine-indicator model (sentiment)", "Long", NeedsData("NYSE member short-sale ratio (13/52-week)"), &[], &[
            "E|\"bullish when it falls below 97.75 and reverses back above that level, and bearish when it rises to 102.7 and falls back below that level\" (p.89)"],
            "Chart 4-5: 79% profitable trades, 13.3%/yr (DJIA, 10/14/1977-5/05/2000).", None),
        d("ndbrm_macro_reversals", "Rate, prime, inflation, liquidity and CRB signals", "Ch 2-4 · Monetary/economic indicators", "Long", NeedsData("bond yields, prime rate, CPI, NAPM, M3/industrial production, CRB index histories"), &[], &[
            "E|Bond yield reversals: 'Falls By 8.7% From A Weekly Peak = Buy, Rises By 11.7% From A Weekly Low = Sell' (chart 4-7); prime rate 'Falls by 3.9% = Buy, Rises By 7.5% = Sell' (charts 2-25, 4-8); CPI y/y crossing its six-month smoothing (chart 2-23/3-5); NAPM price index declines 7 / rises 22 points (chart 2-22); liquidity growth above 1 / below -2.6 (chart 2-27); CRB momentum below 96 / above 108.2 (chart 4-9); Aaa yield momentum brackets (chart 3-4)"],
            "All need non-price macro series.", None),
        d("ndbrm_sentiment", "Sentiment indicators (advisors, put/call, DSI, AAII, Rydex, fund cash)", "Ch 2, 7 · Sentiment", "Long", NeedsData("Investors Intelligence, AAII, DSI, CBOE put/call, Rydex and ICI fund data"), &[], &[
            "E|e.g. 'Buy When Bulls/(Bulls+Bears) = 51 / Sell When ... = 81' with monetary conditions (chart 2-32); dynamic brackets 'Extremes can only be known for certain in hindsight' (chart 2-33)"],
            "Several brackets are hindsight-defined (not testable even with data).", None),
        d("ndbrm_nine_model", "Nine-indicator stock market model", "Ch 4 · A model for the stock market", "Long", NeedsData("new highs/lows, member short ratio, bond/earnings yield, composite bond yield, prime rate and CRB histories"), &[], &[
            "E|\"+1 is contributed when the indicator is on a buy signal and a -1 ... on a sell signal\" (three-mode indicators 0); 'Readings above 67% have bullish implications ... below 50% ... bearish' (p.94-95)"], "", None),
        d("ndbrm_bond_model", "Bond market timing model", "Ch 5 · Bond market timing model", "Long", NeedsData("NDR Unweighted Bond Market Composite, CRB/commodity, jobless claims, TED spread, Dow 20 bond and utility indices"), &[], &[
            "E|\"When the model is +2 or higher, we should buy bonds. When the model is -2 or lower, we should sell bonds\" (p.108)"], "Bond market, not an equity index.", None),
        // ---- not testable
        d("ndbrm_ma_slope", "Major indices vs NDR moving averages (slope indicators)", "Ch 3 · Internal indicators", "Long", NotTestable(
            "notation 'SMA 23 - SL 0,0' / 'FMA 45 - SL 1.5,0' is not defined in the book and no signal results are printed (chart 3-6)"), &[], &[
            "F|\"When the moving average rises by a specified amount from a low (say, 5%), it indicates that an uptrend is in place\" (p.77)"], "", None),
        d("ndbrm_thrust", "Deviation-from-trend thrust indicator (Value Line, 2-day/16-day smoothing)", "Ch 3 · Momentum", "Long", NotTestable(
            "'smoothing' type (simple or exponential) is not stated for this indicator, and the Value Line Geometric index is not in the vault"), &[], &[
            "E|'Signals Generated When Indicator: Rises Above 105.5 = Buy, Declines Below 96.5 = Sell; Repeat Signals Within 60 Days Not Shown' (chart 3-7)"],
            "Chart 3-7 prints only median S&P returns 10/22/63/126 days after signals.", None),
        d("ndbrm_techno_ranks", "Techno Ranks price-momentum stock selection", "Ch 6 · Price momentum equity selection", "Long", NotTestable(
            "formula not given ('rates of change over a 26 to 52 week time frame', p.112) and needs historical S&P 500 constituents"), &[], &[
            "E|\"buy the top 5% of the stocks with the strongest momentum in our ranking, and then hold them until they fall below the top 10%\" (p.114)"], "", None),
        d("ndbrm_valuation_zones", "Valuation / macro zone tables (P/E, P/B, yields, debt, GDP)", "Ch 2 · Risk factors", "Long", NotTestable(
            "conditional return tables ('Gain/Annum When ...'), not trading rules; several bands and the regression trendline are fitted in hindsight"), &[], &[
            "F|e.g. chart 2-19 'S&P 500 Gain/Annum When Real Earnings Yield is Above 2 / Between 1 and 2 / 1 and Below'"], "", None),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    Some(match id {
        "ndbrm_spx_reversals" => ("ndbrm_tape", "NDR tape indicators", "S&P 500 % reversals"),
        "ndbrm_trin40" => ("ndbrm_tape", "NDR tape indicators", "40-day TRIN · NYSE"),
        "ndbrm_trin40_nse" => ("ndbrm_tape", "NDR tape indicators", "40-day TRIN · NSE"),
        "ndbrm_nasdaq_ad" => ("ndbrm_tape", "NDR tape indicators", "A/D line vs 25-day MA · NASDAQ"),
        "ndbrm_ad_line_nse" => ("ndbrm_tape", "NDR tape indicators", "A/D line vs 25-day MA · NSE"),
        "ndbrm_new_highs_lows" | "ndbrm_member_short" | "ndbrm_nine_model" => ("ndbrm_model", "NDR nine-indicator model", ""),
        "ndbrm_macro_reversals" | "ndbrm_sentiment" | "ndbrm_valuation_zones" => ("ndbrm_external", "NDR external indicators", ""),
        "ndbrm_bond_model" => ("ndbrm_bond", "NDR bond model", ""),
        "ndbrm_ma_slope" | "ndbrm_thrust" | "ndbrm_techno_ranks" => ("ndbrm_other", "NDR other", ""),
        _ => return None,
    })
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    match id {
        "ndbrm_spx_reversals" => Some(vec![
            p("up", "Buy: rise from the low weekly close (%)", 8.4, 2.0, 20.0, 0.2),
            p("down", "Sell: fall from the high weekly close (%)", 7.2, 2.0, 20.0, 0.2),
        ]),
        "ndbrm_trin40" | "ndbrm_trin40_nse" => Some(vec![
            p("n", "TRIN smoothing (days, simple average)", 40.0, 10.0, 60.0, 5.0),
            p("buy_hi", "Buy when the 40-day TRIN reaches (x100)", 112.0, 100.0, 130.0, 1.0),
            p("buy_x", "Buy when it falls back below (x100)", 97.7, 90.0, 105.0, 0.1),
            p("sell_lo", "Sell when it falls to (x100)", 79.8, 70.0, 90.0, 0.1),
            p("sell_x", "Sell when it rises back above (x100)", 90.9, 85.0, 100.0, 0.1),
        ]),
        "ndbrm_ad_line_nse" => Some(vec![p("n", "A/D line moving average (days)", 25.0, 5.0, 60.0, 1.0)]),
        _ => None,
    }
}

fn ymd(d: &str) -> (i64, i64, i64) { (d[0..4].parse().unwrap_or(0), d[5..7].parse().unwrap_or(0), d[8..10].parse().unwrap_or(0)) }
fn dnum(y: i64, m: i64, d: i64) -> i64 {
    let y2 = if m <= 2 { y - 1 } else { y };
    let era = y2.div_euclid(400);
    let yoe = y2 - era * 400;
    let doy = (153 * ((m + 9) % 12) + 2) / 5 + d - 1;
    era * 146097 + yoe * 365 + yoe / 4 - yoe / 100 + doy - 719468
}

/// Long/cash state machine over per-bar buy/sell flags (evaluated on the close, filled on the close).
fn long_cash(x: &Ctx, from: usize, to: usize, buy: &[bool], sell: &[bool], bwhy: &str, swhy: &str) -> Vec<Trade> {
    let mut out = vec![];
    let mut cur: Option<Trade> = None;
    for i in from..=to {
        if let Some(mut t) = cur.take() {
            if sell[i] { t.exit_i = i; t.exit_px = x.c[i]; t.signal_i = i; t.reason = swhy.into(); out.push(t); } else { cur = Some(t); }
        } else if buy[i] {
            cur = Some(Trade { dir: 1.0, legs: vec![Leg { i, px: x.c[i], w: 1.0 }], exit_i: i, exit_px: x.c[i], reason: bwhy.into(), entry_kind: "close", signal_i: i, open: false, exits: vec![] });
        }
    }
    if let Some(mut t) = cur { t.exit_i = to; t.exit_px = x.c[to]; t.signal_i = to; t.reason = "open".into(); t.open = true; out.push(t); }
    out
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    let schema = params(id)?;
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).unwrap().def);
    let n = x.c.len();
    if n == 0 || to >= n || from > to { return Some(vec![]); }
    let (mut buy, mut sell) = (vec![false; n], vec![false; n]);
    match id {
        "ndbrm_spx_reversals" => {
            let (up, dn) = (g("up") / 100.0, g("down") / 100.0);
            let wk: Vec<i64> = x.r.iter().map(|r| { let (y, m, d) = ymd(&r.date); let z = dnum(y, m, d); z - (z + 3).rem_euclid(7) }).collect();
            // mode: 0 none yet, 1 long (buy signal in force), -1 out (sell in force)
            let (mut mode, mut lo, mut hi) = (0i8, f64::INFINITY, f64::NEG_INFINITY);
            for i in 0..=to {
                let week_end = if i + 1 < n { wk[i + 1] != wk[i] } else { (wk[i] + 4) == { let (y, m, d) = ymd(&x.r[i].date); dnum(y, m, d) } };
                if !week_end { continue; }
                let c = x.c[i];
                lo = lo.min(c); hi = hi.max(c);
                if mode != 1 && c >= lo * (1.0 + up) { mode = 1; buy[i] = true; hi = c; lo = c; }
                else if mode != -1 && c <= hi * (1.0 - dn) { mode = -1; sell[i] = true; lo = c; hi = c; }
            }
            // only enter on a fresh buy at/after `from`
            Some(long_cash(x, from, to, &buy, &sell, "weekly close rose 8.4% from the low", "weekly close fell 7.2% from the high"))
        }
        "ndbrm_trin40" | "ndbrm_trin40_nse" => {
            let s = if id.ends_with("_nse") { NSE_TR } else { NYSE_TR };
            let (Some(a), Some(d), Some(av), Some(dv)) = (x.series(s[0]), x.series(s[1]), x.series(s[2]), x.series(s[3])) else { return Some(vec![]) };
            let nn = (g("n") as usize).max(1);
            let trin: Vec<f64> = (0..n).map(|i| if d[i] > 0.0 && dv[i] > 0.0 && av[i] > 0.0 { (a[i] / d[i]) / (av[i] / dv[i]) * 100.0 } else { f64::NAN }).collect();
            let v: Vec<f64> = (0..n).map(|i| if i + 1 >= nn { trin[i + 1 - nn..=i].iter().sum::<f64>() / nn as f64 } else { f64::NAN }).collect();
            let (bh, bx, sl, sx) = (g("buy_hi"), g("buy_x"), g("sell_lo"), g("sell_x"));
            for i in 1..n {
                if !(v[i].is_finite() && v[i - 1].is_finite()) { continue; }
                buy[i] = v[i] >= bh || (v[i - 1] >= bx && v[i] < bx);
                sell[i] = v[i] <= sl || (v[i - 1] <= sx && v[i] > sx);
            }
            Some(long_cash(x, from, to, &buy, &sell, "40-day TRIN buy", "40-day TRIN sell"))
        }
        "ndbrm_ad_line_nse" => {
            let (Some(a), Some(d)) = (x.series(NSE_AD[0]), x.series(NSE_AD[1])) else { return Some(vec![]) };
            let nn = (g("n") as usize).max(1);
            // cumulative line restarted after any missing day (NaN breaks the line)
            let mut line = vec![f64::NAN; n];
            let mut acc = f64::NAN;
            for i in 0..n {
                let dd = a[i] - d[i];
                acc = if dd.is_finite() { if acc.is_finite() { acc + dd } else { dd } } else { f64::NAN };
                line[i] = acc;
            }
            for i in 0..n {
                if i + 1 < nn { continue; }
                let w = &line[i + 1 - nn..=i];
                if w.iter().any(|z| !z.is_finite()) { continue; }
                let ma = w.iter().sum::<f64>() / nn as f64;
                buy[i] = line[i] > ma;
                sell[i] = line[i] < ma;
            }
            Some(long_cash(x, from, to, &buy, &sell, "A/D line above its 25-day average", "A/D line below its 25-day average"))
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;
    #[test]
    fn weekly_reversal_buys_after_rise_from_low_and_sells_after_fall_from_high() {
        // Fridays in Jan-Mar 2021 (one bar per week): 100, 95, 103 (+8.4% from 95 = 102.98 -> buy), 110, 102 (-7.27% from 110 -> sell)
        let px = [100.0, 95.0, 103.0, 110.0, 102.0, 101.0];
        let days = ["2021-01-01", "2021-01-08", "2021-01-15", "2021-01-22", "2021-01-29", "2021-02-05"];
        let r: Vec<Row> = px.iter().zip(days).map(|(&c, d)| Row { ticker: "T".into(), date: d.into(), open: c, high: c, low: c, close: c, volume: None }).collect();
        let x = Ctx::new(&r, &r, &[]);
        let ts = run("ndbrm_spx_reversals", &x, 0, 5, &Params::new()).unwrap();
        assert_eq!(ts.len(), 1);
        assert_eq!((ts[0].legs[0].i, ts[0].exit_i, ts[0].open), (2, 4, false));
    }
}

// Instrument admission: NDavis 2026-10-24 block, moved from instrument_policy.rs on 31 Oct 2026 (lane ST).
// Admission only for the module REVIEWED lists (authors/ndavis/admission/SOURCE-CONTRACT.md); kept in one block to merge.
// Runs on actual indices, ETFs and stocks, US and India (index_etf_or_stock, nifty50 basket, declared application).
pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| matches!(def.book,"ndbrm") && (crate::books::ndbrm::reviewed(def.id)),
    label: "Ned Davis Research tested these indicators on the S&P 500, the Dow and NASDAQ breadth; runs on indices, ETFs and stocks, US and India, are a declared application, not the author's test. NYSE breadth rules need NYSE breadth data (US, to Feb 2020); NSE breadth rules run on Indian instruments.",
    basis: "Ned Davis, Being Right or Making Money (2nd ed. 2000): S&P 500 8.4%/7.2% weekly reversals and the 40-day TRIN (ch.4 p.87-88), A/D line vs its 25-day average (ch.2 p.47; NSE breadth mapping). Long or cash, signals and fills on the close. Reviewed in authors/ndavis/admission/SOURCE-CONTRACT.md with independent chronological ledgers; no costs.",
};
