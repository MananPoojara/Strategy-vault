//! Martin Zweig, *Martin Zweig's Winning on Wall Street* (updated & revised ed., 1994). Source + audit: library/mzwws/.
//! Live: the two breadth "thrust" momentum indicators (ten-day A/D ratio >= 2, double 9-to-1 up-volume days) held for
//! the book's own three-month test span, and the calendar strategies of Ch 9 (pre-holiday, optimal holiday, summer).
//! Monetary/sentiment models need data the vault does not have; the Four Percent Model and the month-end window
//! are already in the vault (mbsot_zweig4, kftsm_month_end with p.day=1).
use crate::connors::{claim, Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "mzwws";

const NYSE_AD: &[&str] = &["NYSE_ADV_DEC.Advances", "NYSE_ADV_DEC.Declines"];
const NSE_AD: &[&str] = &["NSE_ADV_DEC.Advances", "NSE_ADV_DEC.Declines"];
const NYSE_UV: &[&str] = &["NYSE_ADV_DEC.AdvVolume", "NYSE_ADV_DEC.DecVolume"];
const NSE_UV: &[&str] = &["NSE_ADV_DEC.AdvVolume", "NSE_ADV_DEC.DecVolume"];

const HOLD_NOTE: &str = "Exit: the book says these two indicators 'as designed ... do not give sell signals' (p.93); its own test is 'invested $10,000 in the market averages only at such signals, held for three months, and then sold' (p.85, tables 14-16). \
That printed three-month hold is the default exit (param hold_months; the tables also print 6 and 12 months). Three months = the first bar dated on/after the same calendar day three months later (month-end clamped). \
Buy on the close of the signal day; a new signal needs the indicator to arrive again while flat (signals during a hold are ignored, as the book ignores 'repeats').";

const HOL_NOTE: &str = "Holiday dates are computed from the US rules (Good Friday = Easter-2; Memorial Day = last Monday of May from 1971, May 30 before; July 4 / Christmas observed on Friday when on Saturday and Monday when on Sunday; Labor Day = first Monday of September; Thanksgiving = fourth Thursday of November; New Year = Jan 1). \
Pre-holiday day = last bar dated before the observed holiday, post-holiday day = first bar after it, so the same dates are used on the Indian index (instrument swap; NSE is open on most US holidays). \
Book index: Zweig Unweighted Price Index, Jan 1952 - Jun 1985 (not available): run on DJI/SPX/NIFTY. All trades close-to-close.";

pub fn catalogue() -> Vec<StrategyDef> {
    use Status::*;
    let d = |id, name, group, dir, status, series: &'static [&'static str], rules, notes, claim| StrategyDef {
        id, name, group, session: 0, dir, status, needs_volume: false, needs_vix: false, needs_series: series, book: BOOK, rules, notes, claim,
    };
    const AD_RULES: &[&str] = &[
        "F|\"I like to track the Advance/Decline Ratio (A/D ratio) over a ten-day period. It's very rare for advances to lead declines by a ratio of 2-to-1 over such a span.\" Ratio = advances / declines (NYSE, unchanged ignored), ten-day totals (p.85).",
        "E|Signal when \"the ten-day A/D ratio reached 2-to-1 or more\" (p.85); buy at the close.",
        "X|\"Had you invested $10,000 in the market averages only at such signals, held for three months, and then sold\" (p.85) - exit three calendar months later at the close.",
    ];
    const UV_RULES: &[&str] = &[
        "F|9-to-1 up day: \"daily up volume leads down volume by a ratio of 9-to-1 or more\" (unchanged volume ignored; p.87).",
        "E|\"having two of them within a reasonably short span is very bullish. I call it a 'double 9-to-1' when two such days occur within three months of one another\" (p.88): buy at the close of the second 9-to-1 up day (table 15 dates). Both types (with or without intervening 9-to-1 down days) count (p.88).",
        "X|Table 15/16: % change 3, 6, 12 months later; \"the investor who bought the ZUPI and held for a year after double 9-to-1 signals\" (p.92-93). Default: sell three months later at the close.",
    ];
    vec![
        d("mzwws_ad_2to1", "Ten-day Advance/Decline ratio >= 2-to-1 · NYSE breadth", "Ch 5 · Momentum indicators", "Long", Live, NYSE_AD, AD_RULES,
          HOLD_NOTE, claim("11 signals 1953-1991", "", "+7.5%/qtr S&P (3 mo.)", "3 months", "Table 14: 1/26/54 ... 2/5/91, S&P 500")),
        d("mzwws_ad_2to1_nse", "Ten-day Advance/Decline ratio >= 2-to-1 · NSE breadth (India)", "Ch 5 · Momentum indicators", "Long", Live, NSE_AD, AD_RULES,
          "India mapping: NSE advances/declines (extra_data NSE_ADV_DEC, 1999-) on NIFTY 50. Same exit as the NYSE version.", None),
        d("mzwws_double_91", "Double 9-to-1 up-volume days · NYSE breadth", "Ch 5 · Momentum indicators", "Long", Live, NYSE_UV, UV_RULES,
          HOLD_NOTE, claim("16 signals 1960-1993", "", "+7.0%/qtr Dow (3 mo.)", "3 months", "Table 15: 11/12/62 ... 6/8/88, Dow")),
        d("mzwws_double_91_nse", "Double 9-to-1 up-volume days · NSE breadth (India)", "Ch 5 · Momentum indicators", "Long", Live, NSE_UV, UV_RULES,
          "India mapping: NSE up/down volume (extra_data NSE_ADV_DEC AdvVolume/DecVolume, 1999-) on NIFTY 50.", None),
        d("mzwws_preholiday", "Pre-holiday day (seven holidays)", "Ch 9 · Seasonal indicators", "Long", Live, &[], &[
            "E|\"on the last trading day prior to the holiday, the market had an exceptional tendency to rise\"; holidays: \"Easter, Memorial Day, July 4th, Labor Day, Thanksgiving, Christmas, and New Year's\" (Presidents' Day and Election Day excluded; p.156).",
            "E|\"had you invested $10,000 on only the one day preceding the Easter holiday\" (p.157): buy at the close of the day before the pre-holiday day.",
            "X|Sell at the close of the pre-holiday day (table 27 measures the market's direction on that one day).",
            "A|Option include_post=1: also the day after Thanksgiving and the day after Christmas (table 27 'Total of all 9 days').",
        ], HOL_NOTE, claim("233 (7 holidays) / 299 (9 days)", "86% (193 up/28 down/12 unch.)", "+0.43%/day", "1 day", "Table 27: ZUPI 1/1952-6/1985")),
        d("mzwws_holiday_optimal", "Optimal holiday strategy (long the holiday, short after a subpar holiday)", "Ch 9 · Seasonal indicators", "Long/Short", Live, &[], &[
            "E|Easter, Memorial Day, July 4th, Labor Day, New Year's: long the pre-holiday day; \"if, at the close of the day, the market is unchanged or down ... then one should sell short the market for the day after the holiday\" (p.161).",
            "E|Christmas: long the pre-holiday day and the day after Christmas; \"if the market is flat or down on the day following Christmas, then at the close of that day we would short the market for one day following\" (p.162).",
            "E|Thanksgiving: long Wednesday and Friday; \"If the Zweig Unweighted Price Index rises by less than .50% during the Wednesday and Friday combined, then we'll short the market at the Friday close and hold that short position for one more day\" (p.162).",
            "X|Each position is closed at the close of its last day (table 28 counts days invested).",
        ], HOL_NOTE, claim("347 days / 233 holiday periods", "96% of days (223 up/8 down/2 unch.)", "+0.45%/day, +0.67%/period", "1-3 days", "Table 28: ZUPI 1/1952-6/1985")),
        d("mzwws_summer", "Summer: buy end of May, sell right after Labor Day", "Ch 9 · Seasonal indicators", "Long", Live, &[], &[
            "E|\"For trading strategies involving months of the year, the best would be to buy stocks at the end of May\" (p.168): buy at the close of the last trading day of May.",
            "X|\"hold them throughout June, July, and August, and sell right after the Labor Day holiday\" (p.168): sell at the open of the first trading day after Labor Day.",
        ], "Labor Day = first Monday of September (on NIFTY: first bar dated after that Monday). 'Right after' read as the first opportunity, the next session's open.",
          claim("", "", "+3.5% S&P through August", "~3 months", "p.168-169, S&P 500 1928-2/1993")),
        // ---- already in the vault
        d("mzwws_four_percent", "Four Percent Model (weekly Value Line)", "Ch 5 · Momentum indicators", "Long/Short", NotTestable(
            "duplicate: same rule already live as mbsot_zweig4 (long side; Zweig's table 18 also shorts the sell signals). Book index Value Line Composite is not in the vault."), &[], &[
            "E|\"gives a buy signal when the weekly Value Line Index rallies 4% or more from any weekly close. It then gives a sell signal when the weekly close of the Value Line Composite drops by 4% or more from any weekly peak\" (p.94)"],
            "Table 18/19 (5/6/66-2/22/93, Value Line): 116 trades, buys 58 (48% profitable, +4.7%/trade), sells 58 (45%, +2.4%), $10,000 -> $241,538. Not re-implemented (duplicate).", None),
        d("mzwws_month_end", "Month-end five-day window (Fosback)", "Ch 9 · Seasonal indicators", "Long", NotTestable(
            "duplicate: identical to kftsm_month_end with p.day=1 (buy at the close before the last trading day, sell at the close of trading day 4 of the next month)"), &[], &[
            "E|\"buying no-load funds just prior to the last trading day of the month, and selling them after the fourth trading day of the next month\" (p.170)"],
            "Fosback 1928-4/1975 S&P: $10,000 -> $569,135, .71%/period; Zweig 4/1975-6/1985 S&P: $13,322, .24%/period (p.169-170).", None),
        // ---- data the vault does not have
        d("mzwws_prime_rate", "Prime Rate Indicator", "Ch 4 · Monetary indicators", "Long", NeedsData("US bank prime rate change history (dates and levels, 1954-)"), &[], &[
            "E|\"Any initial cut in the prime rate if the prime's peak was less than 8%\"; \"If the prime's peak is 8% or higher, a buy signal comes on either the second of two cuts or on a full 1% cut in the rate\" (p.45)",
            "X|\"Any initial hike in the prime rate if the prime's low is 8% or greater\"; \"If the prime's low is less than 8%, a sell signal comes on the second of two hikes or on a full 1% jump\" (p.45)"],
            "Tables 1-2 print every signal 1954-1993 (21 buys, 19 sells; S&P 500 buys +17.1%/yr). Codable once a dated prime-rate series exists.", None),
        d("mzwws_fed_indicator", "Fed Indicator (discount rate + reserve requirements)", "Ch 4 · Monetary indicators", "Long", NeedsData("Federal Reserve discount-rate and reserve-requirement change dates"), &[], &[
            "F|Negative points: a hike in either tool = -1 for six months (wipes positives); positive points: an initial cut = +2 (one point lost after six months, the other after a year), further cuts +1 for six months (p.52-54)",
            "E|\"Extremely Bullish = +2 or more points\" (p.55; table 5 lists the extremely bullish periods)"], "", None),
        d("mzwws_installment_debt", "Installment Debt Indicator", "Ch 4 · Monetary indicators", "Long", NeedsData("monthly US consumer installment debt (Fed G.19) year-to-year change, with release lags"), &[], &[
            "E|Bullish/bearish zones of the year-to-year change in consumer installment debt (p.61-64, graph G)"], "", None),
        d("mzwws_monetary_model", "Monetary Model (prime + Fed + installment debt)", "Ch 4 · Monetary indicators", "Long", NeedsData("prime rate, Fed discount/reserve history, installment debt"), &[], &[
            "E|\"When the Monetary Model rises to 6 points, it trips a buy signal. That buy remains in effect until the model falls to 2 points, which then flashes a sell signal.\" (p.73)"],
            "Table 10: ten buys and nine sells since 1954.", None),
        d("mzwws_super_model", "Super Model (Monetary Model + Four Percent Model)", "Ch 6 · Combining monetary and momentum", "Long", NeedsData("monetary data (prime, Fed, installment debt) and the weekly Value Line Composite"), &[], &[
            "F|\"When the Four Percent Model is on a buy signal, award it 2 points ... add the points from the Four Percent Model to the points in the Monetary Model\" (p.105)",
            "E|Buy/sell levels per table 20 worksheet (BUY at 6+, SELL at 2-3 marks; p.106-108)"], "", None),
        d("mzwws_double_barreled", "Double-barreled buy (2-to-1 A/D + super-bullish Fed)", "Ch 10 · Major bull markets", "Long", NeedsData("Fed Indicator history (discount rate / reserve requirements)"), &[], &[
            "E|\"a Super-Advance/Decline Ratio and a Super-Bullish Fed Indicator reading simultaneously. It is sufficient if the two occur within a relatively brief span of one another ... a three-month time frame\" (p.178)",
            "X|Table 33/34 holding spans 1-18 months; no single exit stated"], "Twelve signals 1932-1991 (table 33).", None),
        // ---- not testable
        d("mzwws_sentiment", "Sentiment indicators (fund cash, advisors, odd lots, short interest, put/call ...)", "Ch 8 · Sentiment indicators", "Long", NotTestable(
            "no fixed levels: 'There is no magic level of optimism or pessimism that gives precise signals' (p.128); data also not in the vault"), &[], &[
            "F|Mutual fund cash/assets peaks and troughs, advisory sentiment, etc. (p.125-154)"], "", None),
        d("mzwws_calendar_stats", "Days of the week, months, presidential cycle", "Ch 9 · Seasonal indicators", "Long", NotTestable(
            "statistics without a trading rule (e.g. 'these tendencies might be overwhelmed by more important indicators', p.165; the election cycle 'clearly not enough to dominate strategy', p.173)"), &[], &[
            "F|Friday/Monday effects (p.164-165), monthly tendencies tables 29-30 (p.166-168), election cycle table 31 (p.171-173)"], "", None),
        d("mzwws_tax_selling", "Year-end tax-selling stocks", "Ch 9 · Seasonal indicators", "Long", NotTestable(
            "stock selection with a discretionary sell ('The sell date is quite arbitrary, but if the market acts well in January...', p.174)"), &[], &[
            "E|\"search for stocks that have made new lows sometime around late December ... Buy them and hold them for a few weeks\" (p.174)"], "", None),
        d("mzwws_bear_conditions", "Three crucial conditions for bear markets", "Ch 10 · Major bear markets", "Long", NeedsData("PPI, S&P/Dow P/E ratios and yield-curve history; warnings, no exit rules"), &[], &[
            "F|Extreme deflation (six-month average annualised PPI change <= -10%), P/E >= 18 (S&P) or 20 (Dow), inverted yield curve (p.182-189)"], "Warning conditions, not a trading system.", None),
        d("mzwws_stock_rules", "Stock selection and stop rules", "Ch 11-14 · Stocks, stops, short selling", "Long/Short", NotTestable(
            "individual-stock fundamentals and judgement; not an index rule"), &[], &[
            "F|Earnings, P/E, insider and relative-strength screens; stop-loss guidance for individual stocks (p.195-256)"], "", None),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    Some(match id {
        "mzwws_ad_2to1" => ("mzwws_thrust", "Zweig breadth thrusts", "10-day A/D >= 2 · NYSE"),
        "mzwws_ad_2to1_nse" => ("mzwws_thrust", "Zweig breadth thrusts", "10-day A/D >= 2 · NSE"),
        "mzwws_double_91" => ("mzwws_thrust", "Zweig breadth thrusts", "Double 9-to-1 · NYSE"),
        "mzwws_double_91_nse" => ("mzwws_thrust", "Zweig breadth thrusts", "Double 9-to-1 · NSE"),
        "mzwws_preholiday" => ("mzwws_seasonal", "Zweig seasonal strategies", "Pre-holiday"),
        "mzwws_holiday_optimal" => ("mzwws_seasonal", "Zweig seasonal strategies", "Optimal holiday"),
        "mzwws_summer" => ("mzwws_seasonal", "Zweig seasonal strategies", "Summer"),
        "mzwws_month_end" => ("mzwws_seasonal", "Zweig seasonal strategies", "Month-end (duplicate)"),
        "mzwws_calendar_stats" | "mzwws_tax_selling" => ("mzwws_seasonal", "Zweig seasonal strategies", "Statistics"),
        "mzwws_four_percent" | "mzwws_super_model" | "mzwws_double_barreled" => ("mzwws_models", "Zweig models", "Momentum/monetary"),
        "mzwws_prime_rate" | "mzwws_fed_indicator" | "mzwws_installment_debt" | "mzwws_monetary_model" => ("mzwws_models", "Zweig models", "Monetary"),
        "mzwws_sentiment" | "mzwws_bear_conditions" | "mzwws_stock_rules" => ("mzwws_other", "Zweig other", ""),
        _ => return None,
    })
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    match id {
        "mzwws_ad_2to1" | "mzwws_ad_2to1_nse" => Some(vec![
            p("days", "A/D ratio span (days)", 10.0, 5.0, 20.0, 1.0),
            p("ratio", "Advances/declines ratio threshold", 2.0, 1.5, 3.0, 0.1),
            p("hold_months", "Months held (book tables: 3 or 6)", 3.0, 1.0, 12.0, 1.0),
        ]),
        "mzwws_double_91" | "mzwws_double_91_nse" => Some(vec![
            p("ratio", "Up/down volume ratio for a 9-to-1 day", 9.0, 5.0, 12.0, 0.5),
            p("window_months", "Second 9-to-1 day within (months)", 3.0, 1.0, 6.0, 1.0),
            p("hold_months", "Months held (book tables: 3, 6 or 12)", 3.0, 1.0, 12.0, 1.0),
        ]),
        "mzwws_preholiday" => Some(vec![p("include_post", "Also the days after Thanksgiving and Christmas (table 27 '9 days')", 0.0, 0.0, 1.0, 1.0)]),
        "mzwws_holiday_optimal" => Some(vec![p("thanks_min", "Thanksgiving Wed+Fri gain below which to short (%)", 0.5, 0.0, 2.0, 0.1)]),
        "mzwws_summer" => Some(vec![]),
        _ => None,
    }
}

// ---------------------------------------------------------------- dates
pub fn ymd(d: &str) -> (i64, i64, i64) { (d[0..4].parse().unwrap_or(0), d[5..7].parse().unwrap_or(0), d[8..10].parse().unwrap_or(0)) }
/// days since 1970-01-01 (proleptic Gregorian)
pub fn dnum(y: i64, m: i64, d: i64) -> i64 {
    let y2 = if m <= 2 { y - 1 } else { y };
    let era = y2.div_euclid(400);
    let yoe = y2 - era * 400;
    let mp = (m + 9) % 12;
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146097 + doe - 719468
}
/// 0 = Monday .. 6 = Sunday
pub fn wday(n: i64) -> i64 { (n + 3).rem_euclid(7) }
fn mdays(y: i64, m: i64) -> i64 { dnum(if m == 12 { y + 1 } else { y }, if m == 12 { 1 } else { m + 1 }, 1) - dnum(y, m, 1) }
pub fn add_months(n: i64, k: i64, (y, m, d): (i64, i64, i64)) -> i64 {
    let _ = n;
    let t = y * 12 + (m - 1) + k;
    let (y2, m2) = (t.div_euclid(12), t.rem_euclid(12) + 1);
    dnum(y2, m2, d.min(mdays(y2, m2)))
}
fn easter(y: i64) -> i64 {
    let a = y % 19; let b = y / 100; let c = y % 100; let d = b / 4; let e = b % 4;
    let f = (b + 8) / 25; let g = (b - f + 1) / 3; let h = (19 * a + b - d - g + 15) % 30;
    let i = c / 4; let k = c % 4; let l = (32 + 2 * e + 2 * i - h - k) % 7; let m = (a + 11 * h + 22 * l) / 451;
    let month = (h + l - 7 * m + 114) / 31; let day = (h + l - 7 * m + 114) % 31 + 1;
    dnum(y, month, day)
}
fn observed(n: i64) -> i64 { match wday(n) { 5 => n - 1, 6 => n + 1, _ => n } }
fn nth_wd(y: i64, m: i64, wd: i64, k: i64) -> i64 { let f = dnum(y, m, 1); f + (wd - wday(f)).rem_euclid(7) + 7 * (k - 1) }
fn last_wd(y: i64, m: i64, wd: i64) -> i64 { let l = dnum(y, m, mdays(y, m)); l - (wday(l) - wd).rem_euclid(7) }
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Hol { Easter, Memorial, July4, Labor, Thanksgiving, Christmas, NewYear }
/// Observed US holiday dates (day numbers) of year y. New Year = Jan 1 of y+1 (the holiday that ends year y).
pub fn holidays(y: i64) -> Vec<(Hol, i64)> {
    vec![
        (Hol::Easter, easter(y) - 2),
        (Hol::Memorial, if y >= 1971 { last_wd(y, 5, 0) } else { observed(dnum(y, 5, 30)) }),
        (Hol::July4, observed(dnum(y, 7, 4))),
        (Hol::Labor, nth_wd(y, 9, 0, 1)),
        (Hol::Thanksgiving, nth_wd(y, 11, 3, 4)),
        (Hol::Christmas, observed(dnum(y, 12, 25))),
        (Hol::NewYear, dnum(y + 1, 1, 1)),
    ]
}

fn tr(dir: f64, i: usize, px: f64, kind: &'static str, sig: usize) -> Trade {
    Trade { dir, legs: vec![Leg { i, px, w: 1.0 }], exit_i: i, exit_px: px, reason: String::new(), entry_kind: kind, signal_i: sig, open: false, exits: vec![] }
}
fn close_at(mut t: Trade, i: usize, px: f64, why: String) -> Trade { t.exit_i = i; t.exit_px = px; t.signal_i = i; t.reason = why; t }

/// Hold-N-months simulator over a boolean signal series (signal evaluated on the close).
fn hold_months(x: &Ctx, from: usize, to: usize, sig: &[bool], months: i64, why: &str) -> Vec<Trade> {
    let dn: Vec<i64> = x.r.iter().map(|r| { let (y, m, d) = ymd(&r.date); dnum(y, m, d) }).collect();
    let mut out = vec![];
    let mut cur: Option<(Trade, i64)> = None;
    for i in from..=to {
        if let Some((t, end)) = cur.take() {
            if dn[i] >= end { out.push(close_at(t, i, x.c[i], format!("held {months} months"))); }
            else { cur = Some((t, end)); continue; }
        }
        if sig[i] && cur.is_none() {
            let end = add_months(dn[i], months, ymd(&x.r[i].date));
            let mut t = tr(1.0, i, x.c[i], "close", i); t.reason = why.into();
            cur = Some((t, end));
        }
    }
    if let Some((mut t, _)) = cur { t.exit_i = to; t.exit_px = x.c[to]; t.signal_i = to; t.reason = "open".into(); t.open = true; out.push(t); }
    out
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    let schema = params(id)?;
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).map_or(0.0, |d| d.def));
    let n = x.c.len();
    if n == 0 || to >= n || from > to { return Some(vec![]); }
    let out = match id {
        "mzwws_ad_2to1" | "mzwws_ad_2to1_nse" => {
            let s = if id.ends_with("_nse") { NSE_AD } else { NYSE_AD };
            let (Some(a), Some(d)) = (x.series(s[0]), x.series(s[1])) else { return Some(vec![]) };
            let days = (g("days") as usize).max(1);
            let thr = g("ratio");
            let ratio: Vec<f64> = (0..n).map(|i| {
                if i + 1 < days { return f64::NAN; }
                let (sa, sd) = (a[i + 1 - days..=i].iter().sum::<f64>(), d[i + 1 - days..=i].iter().sum::<f64>());
                if sd > 0.0 { sa / sd } else { f64::NAN } // NaN when any day is missing
            }).collect();
            // signal = the ratio arrives at >= threshold (was below, or unknown, the day before)
            let sig: Vec<bool> = (0..n).map(|i| ratio[i] >= thr && !(i > 0 && ratio[i - 1] >= thr)).collect();
            hold_months(x, from, to, &sig, g("hold_months") as i64, &format!("{days}-day A/D ratio >= {thr}"))
        }
        "mzwws_double_91" | "mzwws_double_91_nse" => {
            let s = if id.ends_with("_nse") { NSE_UV } else { NYSE_UV };
            let (Some(u), Some(dv)) = (x.series(s[0]), x.series(s[1])) else { return Some(vec![]) };
            let thr = g("ratio");
            let w = g("window_months") as i64;
            let dn: Vec<i64> = x.r.iter().map(|r| { let (y, m, d) = ymd(&r.date); dnum(y, m, d) }).collect();
            let nine: Vec<bool> = (0..n).map(|i| u[i].is_finite() && dv[i].is_finite() && dv[i] > 0.0 && u[i] / dv[i] >= thr).collect();
            let mut last: Option<usize> = None;
            let mut sig = vec![false; n];
            for i in 0..n {
                if nine[i] {
                    if let Some(j) = last { if add_months(dn[j], w, ymd(&x.r[j].date)) >= dn[i] { sig[i] = true; } }
                    last = Some(i);
                }
            }
            hold_months(x, from, to, &sig, g("hold_months") as i64, "double 9-to-1 up-volume days")
        }
        "mzwws_preholiday" | "mzwws_holiday_optimal" => holiday(id, x, from, to, g("include_post") >= 0.5, g("thanks_min") / 100.0),
        "mzwws_summer" => {
            let mut out = vec![];
            let mut cur: Option<(Trade, i64)> = None;
            for i in from..=to {
                let (y, m, _) = ymd(&x.r[i].date);
                let dni = { let (y, m, d) = ymd(&x.r[i].date); dnum(y, m, d) };
                if let Some((t, lab)) = cur.take() {
                    if dni > lab { out.push(close_at(t, i, x.o[i], "open of the first session after Labor Day".into())); }
                    else { cur = Some((t, lab)); continue; }
                }
                let last_may = m == 5 && (i + 1 >= n || ymd(&x.r[i + 1].date).1 != 5);
                if last_may && i < n - 1 { let mut t = tr(1.0, i, x.c[i], "close", i); t.reason = "last trading day of May".into(); cur = Some((t, nth_wd(y, 9, 0, 1))); }
            }
            if let Some((mut t, _)) = cur { t.exit_i = to; t.exit_px = x.c[to]; t.signal_i = to; t.reason = "open".into(); t.open = true; out.push(t); }
            out
        }
        _ => return None,
    };
    Some(out.into_iter().filter(|t| t.legs[0].i >= from).collect())
}

/// Index of the last bar dated before day number `h` and the first bar dated after it.
fn around(dn: &[i64], h: i64) -> (Option<usize>, Option<usize>) {
    let k = dn.partition_point(|&d| d < h);
    let pre = if k > 0 { Some(k - 1) } else { None };
    let post_k = dn.partition_point(|&d| d <= h);
    (pre, if post_k < dn.len() { Some(post_k) } else { None })
}

fn holiday(id: &str, x: &Ctx, from: usize, to: usize, include_post: bool, thanks_min: f64) -> Vec<Trade> {
    // the exchange calendar is known in advance: holiday neighbours come from the full bar list, trades are cut at `to`
    let n = x.r.len();
    let dn: Vec<i64> = x.r.iter().map(|r| { let (y, m, d) = ymd(&r.date); dnum(y, m, d) }).collect();
    let c = &x.c;
    let (y0, y1) = (ymd(&x.r[0].date).0 - 1, ymd(&x.r[n - 1].date).0);
    let mut out = vec![];
    let lg = |a: usize, b: usize, why: String| close_at(tr(1.0, a, c[a], "close", a), b, c[b], why);
    let sh = |a: usize, b: usize, why: String| close_at(tr(-1.0, a, c[a], "close", a), b, c[b], why);
    for y in y0..=y1 {
        for (hol, hd) in holidays(y) {
            let (Some(pre), Some(q)) = around(&dn, hd) else { continue };
            if pre == 0 || pre - 1 < from || pre - 1 > to { continue; }
            let name = format!("{hol:?}");
            if id == "mzwws_preholiday" {
                out.push(lg(pre - 1, pre, format!("{name}: pre-holiday day")));
                if include_post && matches!(hol, Hol::Thanksgiving | Hol::Christmas) { out.push(lg(q - 1, q, format!("{name}: day after"))); }
                continue;
            }
            match hol {
                Hol::Christmas => {
                    out.push(lg(pre - 1, q, format!("{name}: pre-holiday day and the day after")));
                    if c[q] <= c[pre] && q + 1 < n { out.push(sh(q, q + 1, format!("{name}: day after flat/down -> short one day"))); }
                }
                Hol::Thanksgiving => {
                    out.push(lg(pre - 1, q, format!("{name}: Wednesday and Friday")));
                    if c[q] / c[pre - 1] - 1.0 < thanks_min && q + 1 < n { out.push(sh(q, q + 1, format!("{name}: Wed+Fri gain < {:.2}% -> short one day", thanks_min * 100.0))); }
                }
                _ => {
                    out.push(lg(pre - 1, pre, format!("{name}: pre-holiday day")));
                    if c[pre] <= c[pre - 1] { out.push(sh(pre, q, format!("{name}: pre-holiday flat/down -> short the day after"))); }
                }
            }
        }
    }
    // entries after `to` are dropped; a position still held at `to` is marked open there
    out.retain(|t| t.legs[0].i <= to && !(t.legs[0].i == to && t.exit_i > to));
    for t in out.iter_mut() { if t.exit_i > to { t.exit_i = to; t.exit_px = c[to]; t.signal_i = to; t.reason = "open".into(); t.open = true; } }
    out.sort_by_key(|t| (t.legs[0].i, t.exit_i));
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn calendar_rules() {
        assert_eq!(wday(dnum(2024, 11, 28)), 3); // Thanksgiving 2024 is a Thursday
        assert_eq!(nth_wd(2024, 11, 3, 4), dnum(2024, 11, 28));
        assert_eq!(easter(2024), dnum(2024, 3, 31));
        assert_eq!(easter(1985), dnum(1985, 4, 7));
        assert_eq!(last_wd(2024, 5, 0), dnum(2024, 5, 27));
        assert_eq!(observed(dnum(2020, 7, 4)), dnum(2020, 7, 3)); // Saturday -> Friday
        assert_eq!(add_months(0, 3, (2001, 11, 30)), dnum(2002, 2, 28));
    }
}
