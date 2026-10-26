//! Yale & Jeffrey A. Hirsch, Stock Trader's Almanac (2008 edition = primary text; 2005 edition for the
//! MACD parameters; 2010 edition for the Best Six Months + Timing + Four-Year Cycle strategy).
//! Only calendar rules with exact start and end trading days are Live. Every window is long from the
//! close before the window to the close of its last day, exactly how the Almanac measures it
//! (close-to-close % changes). Forecasting indicators (January Barometer, December Low, ...) give no
//! entry/exit orders and are NotTestable. Page refs are printed pages of the named edition.
//! Sources, transcription and independent checks: library/hsta/.
use crate::books::apta::ema;
use crate::connors::{claim, Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "hsta";
/// Admitted ids (authors/hirsch/admission/SOURCE-CONTRACT.md).
pub const REVIEWED: &[&str] = &["hsta_best6", "hsta_best6_macd", "hsta_best8_macd", "hsta_best6_4yr", "hsta_santa", "hsta_stpat", "hsta_midyear", "hsta_first_day", "hsta_thanksgiving", "hsta_super8", "hsta_yk_passover", "hsta_election"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

// ------------------------------------------------------------------ calendar helpers

fn ymd(s: &str) -> (i64, u32, u32) {
    (s[0..4].parse().unwrap_or(0), s[5..7].parse().unwrap_or(0), s[8..10].parse().unwrap_or(0))
}

/// Days since 1970-01-01 (H. Hinnant's days_from_civil).
fn days_from_civil(y: i64, m: u32, d: u32) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let mp = (m as i64 + 9) % 12;
    let doy = (153 * mp + 2) / 5 + d as i64 - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146097 + doe - 719468
}

fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719468;
    let era = z.div_euclid(146097);
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (yoe + era * 400 + if m <= 2 { 1 } else { 0 }, m, d)
}

fn iso(z: i64) -> String {
    let (y, m, d) = civil_from_days(z);
    format!("{y:04}-{m:02}-{d:02}")
}

// Hebrew calendar (Reingold & Dershowitz, Calendrical Calculations): R.D. 1 = 0001-01-01 (Gregorian).
const HEBREW_EPOCH_RD: i64 = -1373427;
const RD_1970: i64 = 719163;

fn hebrew_elapsed(y: i64) -> i64 {
    let months = (235 * y - 234).div_euclid(19);
    let parts = 12084 + 13753 * months;
    let day = 29 * months + parts.div_euclid(25920);
    if (3 * (day + 1)).rem_euclid(7) < 3 { day + 1 } else { day }
}

fn hebrew_correction(y: i64) -> i64 {
    let (a, b, c) = (hebrew_elapsed(y - 1), hebrew_elapsed(y), hebrew_elapsed(y + 1));
    if c - b == 356 { 2 } else if b - a == 382 { 1 } else { 0 }
}

/// Rosh Hashanah (1 Tishri) falling in Gregorian year `g`, as days since 1970-01-01.
fn rosh_hashanah(g: i64) -> i64 {
    let hy = g + 3761;
    HEBREW_EPOCH_RD + hebrew_elapsed(hy) + hebrew_correction(hy) - RD_1970
}
fn yom_kippur(g: i64) -> i64 {
    rosh_hashanah(g) + 9
}
/// First day of Passover (15 Nisan) in Gregorian year `g`: always 163 days before that year's Rosh Hashanah.
fn passover(g: i64) -> i64 {
    rosh_hashanah(g) - 163
}
/// US Thanksgiving: fourth Thursday of November.
fn thanksgiving(y: i64) -> i64 {
    let nov1 = days_from_civil(y, 11, 1);
    let wd = (nov1 + 3).rem_euclid(7); // 0 = Monday (1970-01-01 was a Thursday)
    nov1 + (3 - wd).rem_euclid(7) + 21
}

struct Cal<'a> {
    x: &'a Ctx<'a>,
}
impl<'a> Cal<'a> {
    fn n(&self) -> usize {
        self.x.r.len()
    }
    fn d(&self, i: usize) -> (i64, u32, u32) {
        ymd(&self.x.r[i].date)
    }
    fn ym(&self, i: usize) -> (i64, u32) {
        let (y, m, _) = self.d(i);
        (y, m)
    }
    /// First bar of its month (needs the previous bar to know).
    fn first_of_month(&self, i: usize) -> bool {
        i > 0 && self.ym(i) != self.ym(i - 1)
    }
    /// Last bar of its month (the next bar exists and is in a new month; the exchange calendar is known in advance).
    fn last_of_month(&self, i: usize) -> bool {
        i + 1 < self.n() && self.ym(i + 1) != self.ym(i)
    }
    /// Last bar dated on or before `day` (days since 1970).
    fn on_or_before(&self, day: i64) -> Option<usize> {
        let s = iso(day);
        self.x.r.partition_point(|r| r.date.as_str() <= s.as_str()).checked_sub(1)
    }
    /// First bar dated on or after `day`.
    fn on_or_after(&self, day: i64) -> Option<usize> {
        let s = iso(day);
        let k = self.x.r.partition_point(|r| r.date.as_str() < s.as_str());
        (k < self.n()).then_some(k)
    }
}

// ------------------------------------------------------------------ trades

fn mk(dir: f64, x: &Ctx, e: usize, reason: &str) -> Trade {
    Trade { dir, legs: vec![Leg { i: e, px: x.c[e], w: 1.0 }], exit_i: e, exit_px: x.c[e], reason: reason.into(),
        entry_kind: "close", signal_i: e, open: false, exits: vec![] }
}

/// Close-to-close windows -> trades. Entries outside [from, to] are dropped; an exit after `to`
/// (or unknown because the data ends) leaves the trade open, marked at `to`'s close.
fn windows(x: &Ctx, from: usize, to: usize, dir: f64, w: Vec<(usize, Option<usize>)>, reason: &str) -> Vec<Trade> {
    let mut out: Vec<Trade> = vec![];
    for (e, xi) in w {
        if e < from || e > to || out.last().is_some_and(|t| e < t.exit_i) {
            continue;
        }
        if xi.is_some_and(|k| k <= e) {
            continue;
        }
        let mut t = mk(dir, x, e, reason);
        match xi {
            Some(k) if k <= to => {
                t.exit_i = k;
                t.exit_px = x.c[k];
                t.signal_i = k;
            }
            _ => {
                t.exit_i = to;
                t.exit_px = x.c[to];
                t.signal_i = to;
                t.open = true;
                t.reason = "open".into();
            }
        }
        out.push(t);
    }
    out
}

fn macd(c: &[f64], fast: usize, slow: usize, sig: usize) -> (Vec<f64>, Vec<f64>) {
    let (f, s) = (ema(c, fast), ema(c, slow));
    let m: Vec<f64> = f.iter().zip(&s).map(|(a, b)| a - b).collect();
    let g = ema(&m, sig);
    (m, g)
}

// ------------------------------------------------------------------ catalogue

pub fn catalogue() -> Vec<StrategyDef> {
    use Status::*;
    let d = |id, name, group, dir, status, rules, notes, claim| StrategyDef {
        id, name, group, session: 0, dir, status, needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK, rules, notes, claim,
    };
    vec![
        // ---- Best Six Months family
        d("hsta_best6", "Best Six Months switching strategy (Nov 1 - Apr 30)", "STA 2008 p.48 · Best Six Months", "Long", Live,
            &["E|\"Investing in the Dow Jones Industrial Average between November 1st and April 30th each year\" (p.48): buy at the close of the last trading day of October",
              "X|\"and then switching into fixed income for the other six months\" (p.48): sell at the close of the last trading day of April",
              "F|[Read] The p.48 table's '% Change Nov 1-Apr 30' is the October month-end close to the April month-end close (checked: 2006 row 12080.73 -> 13062.91 = +8.1%)."],
            "Book index: DJIA 1950-2007 (our DJI starts 1970; also runs on SPX/NDX/NIFTY as an instrument swap). Cash return of the out-of-market months is not modelled. Near-duplicate: kftsm_hirsch (Kaufman's description: buys the first November day) and apta_six_best (Appel: exits on/before May 4); this is Hirsch's own month-end definition.",
            claim("57", "45 up / 12 down", "+7.9% avg (May-Oct +0.5%)", "6 months", "DJIA 1950-2007 ($10,000 -> $588,413)")),
        d("hsta_best6_macd", "Best Six Months + MACD timing (Dow / S&P)", "STA 2008 p.50; STA 2005 p.79 · MACD-Timing Triples Best Six Months", "Long", Live,
            &["E|\"starting at the beginning of October we employ an 8-17-9 MACD trigger to indicate and confirm a buy\" (2005 p.79); \"Starting October 1 we look to catch the market's first hint of an uptrend\" (2008 p.50)",
              "X|\"Starting at the beginning of April, we employ a 12-25-9 MACD trigger to indicate and confirm a sell\" (2005 p.79); \"beginning April 1 we prepare to exit these seasonal positions as soon as the market falters\" (2008 p.50)",
              "F|[Fitted reading, C] MACD = EMA(fast) - EMA(slow) of daily closes, trigger = EMA(9) of MACD. Buy at the close of the first day on/after the first October trading day with MACD(8,17) above its trigger; sell at the close of the first day on/after the first April trading day with MACD(12,25) below its trigger. Reproduces the p.50 DJIA rows 2002-2008 to 0.1% (both columns) and 14 of 17 NASDAQ buy dates (p.58, run on NDX)."],
            "Book: DJIA; the same signals are applied to the S&P (2005 p.79: DIA/SPY sold on the April trigger). Signal dates are not printed for the Dow; pre-2001 rows of the p.50 table do not reproduce with this (or any tested) reading (see README). EMA seeded with the simple average of the first n values.",
            claim("57", "50 up / 7 down", "+10.0% avg (worst-six -1.3%)", "~6.4 months", "DJIA 1950-2007 ($10,000 -> $1,782,490)")),
        d("hsta_best8_macd", "NASDAQ Best Eight Months + MACD timing", "STA 2008 p.58 · Get More Out of NASDAQ's Best Eight Months", "Long", Live,
            &["E|\"Using the same MACD timing indicators on the NASDAQ as is done for the Dow (page 50)\" (p.58): MACD(8,17,9) buy from October 1",
              "X|Best Eight Months = Nov 1-June 30 (p.58); \"Start Looking for NASDAQ MACD Sell Signal\" in June (weekly page, p.57/2005 p.58): MACD(12,25,9) sell from the first June trading day",
              "F|[Fitted reading, C] same MACD state rule as hsta_best6_macd; p.58 prints every signal date 1971-2007."],
            "Book index: NASDAQ Composite (not in the vault) -> run on NDX (1990-) as an instrument swap. Signal dates and closes printed on p.58 are compared date by date in the README.",
            claim("36", "32 up / 4 down", "", "~8 months", "NASDAQ Composite 1971-2007 ($10,000 -> $792,291)")),
        d("hsta_best6_4yr", "Best Six Months + MACD + Four-Year Cycle", "STA 2010 p.60 · Best Six Months + Timing + Four-Year Cycle", "Long", Live,
            &["F|\"FOUR TRADES EVERY FOUR YEARS\": Post-election year Sell/Buy, Midterm Sell/Buy, Pre-election Hold/Hold, Election Hold/Hold (2010 p.60 table)",
              "E|\"Buy ... during the post-election and midterm years and then hold from the midterm MACD seasonal buy signal sometime after October 1\" (2010 p.60): MACD(8,17,9) buy from October 1 of post-election and midterm years",
              "X|\"and hold until the post-election MACD seasonal sell signal sometime after April 1, approximately 2.5 years\" (2010 p.60): MACD(12,25,9) sell from April 1, only in post-election and midterm years",
              "F|Election year = year divisible by 4 (US presidential elections); post-election = +1, midterm = +2, pre-election = +3."],
            "Book index: DJIA 1949-2009. Uses the same fitted MACD state reading as hsta_best6_macd.",
            claim("30", "26 up / 4 down", "+9.9% avg (worst-six -1.5%)", "", "DJIA 1949-2009 ($10,000 -> $1,357,181)")),
        // ---- short calendar windows
        d("hsta_santa", "Santa Claus Rally", "STA 2008 p.112 · If Santa Claus Should Fail to Call", "Long", Live,
            &["E|\"a short, sweet, respectable rally within the last five days of the year and the first two in January\" (p.112): buy at the close of the 6th-to-last trading day of December",
              "X|Sell at the close of the second trading day of January (the table's 'Rally % Change' = trading days 5..1 before year end + January days 1-2)",
              "F|Santa's failure to show \"tends to precede bear markets\" (p.112) is a forecast, not an order: not coded."],
            "Book index: S&P 500 1969-2007, every year's daily % printed on p.112 (compared year by year in the README).",
            claim("38", "", "+1.5% avg", "7 days", "S&P 500 1969-2006 (1.5% since 1950)")),
        d("hsta_stpat", "Day before St. Patrick's Day", "STA 2008 p.32 · Profit on Day Before St. Patrick's Day", "Long", Live,
            &["E|\"gains the day before Saint Patrick's Day have proved best ... average gain of 0.30% on the S&P\"; \"going long the day before may prove fortunate\" (p.32): buy at the close two trading days before St. Patrick's Day",
              "X|Sell at the close of the trading day before St. Patrick's Day ('% Change 1 Day Prior' column)",
              "F|\"When St. Patrick's Day falls on Saturday or Sunday, the following trading day is used\" (p.32 footnote)"],
            "Book index: S&P 500 1953-2007 with every year printed on p.32.",
            claim("55", "", "+0.30% avg", "1 day", "S&P 500 1953-2007")),
        d("hsta_midyear", "NASDAQ 12-day mid-year rally (Christmas in July)", "STA 2008 p.60 · Mid-Year Rally: Christmas in July", "Long", Live,
            &["E|\"NASDAQ averaging a 2.9% gain since 1987 during the 12-day period from June's third-to-last trading day through July's ninth trading day\"; \"a short, tradable mid-year rally\" (p.60): buy at the close of June's 4th-to-last trading day (table column 'June 4th Last Trading Day')",
              "X|Sell at the close of July's 9th trading day (column 'July 9th Trading Day')"],
            "Book index: NASDAQ Composite 1985-2006 (not in the vault) -> NDX (1990-) as an instrument swap.",
            claim("22", "", "+2.9% avg (1987-2006)", "12 days", "NASDAQ Composite 1987-2006")),
        d("hsta_first_day", "First trading day of the month", "STA 2008 p.62 · First-Trading-Day-Of-The-Month Phenomenon", "Long", Live,
            &["E|\"the Dow Jones Industrial Average has gained more points on the first trading days of all months than all other days combined\" (p.62): buy at the close of the last trading day of each month",
              "X|Sell at the close of the first trading day of the next month"],
            "Book index: DJIA 9/1997-5/2007: 117 first days, +4565.51 points (avg +39.02) vs +948.21 on 2313 other days.",
            claim("117", "", "+39.02 Dow points avg", "1 day", "DJIA Sep 1997 - May 2007")),
        d("hsta_thanksgiving", "Thanksgiving market (Wednesday before + Friday after)", "STA 2008 p.102 · Trading the Thanksgiving Market", "Long", Live,
            &["E|\"For 35 years the combination of the Wednesday before Thanksgiving and the Friday after had a great track record\" (p.102): buy at the close of the Tuesday before Thanksgiving (table 'Total Gain' = Wednesday + Friday points)",
              "X|Sell at the close of the Friday after Thanksgiving (default). \"The best strategy appears to be coming into the week long and exiting into strength Friday or Monday\" (p.102): p.exit_monday=1 sells at the close of the following Monday instead",
              "F|Thanksgiving = 4th Thursday of November (US)."],
            "Book index: DJIA 1952-2006 (points per year printed p.102). 'Exiting into strength' is discretionary; the two exits are the book's two measured windows. On NIFTY the US holiday dates are applied as calendar dates (instrument swap).",
            claim("55", "", "Wed-Fri +460.99 Dow pts 1988-2006 (7 losses of 19)", "3 days", "DJIA 1952-2006")),
        d("hsta_super8", "Super-8 days each month", "STA 2008 p.86 · Market Gains More on Super-8 Days", "Long", Live,
            &["E|\"bullish bias from the last three trading days of the previous month through the first two days of the current month\" (p.86): buy at the close of the 4th-to-last trading day of each month",
              "X|Sell at the close of the 2nd trading day of the next month",
              "E|\"and now shows significant bullishness during the middle three trading days, 9 to 11\" (p.86): buy at the close of the 8th trading day",
              "X|Sell at the close of the 11th trading day",
              "F|\"Super 8 Days = Last 3 + First 2 + Middle 3\" (p.86 table footnote)"],
            "Book index: DJIA 1999-Apr 2007 monthly Super-8 % changes printed on p.86 (100 months: net +68.31% vs -23.49% rest of month).",
            claim("100 months", "", "+0.68% per month (Super-8)", "8 days/month", "DJIA Jan 1999 - Apr 2007")),
        d("hsta_yk_passover", "Buy Yom Kippur, sell Passover", "STA 2008 p.88 · Sell Rosh Hashanah, Buy Yom Kippur, Sell Passover", "Long", Live,
            &["E|\"buying instead of selling Yom Kippur\"; \"being long Yom Kippur to Passover has produced more than twice as many advances, averaging gains of 7.1%\" (p.88): buy at the Dow close on Yom Kippur",
              "X|Sell at the close on (the first day of) Passover the next spring",
              "F|\"If on Saturday or Sunday prior close is used\" (p.88 footnote): the last trading day on or before each date",
              "F|Hebrew dates computed with the standard arithmetic Hebrew calendar (Reingold-Dershowitz); matches all 108 dates of the p.88 table except Passover 1990 (book Apr 11 = 2nd day; 15 Nisan = Apr 10)."],
            "Book index: DJIA 1971-2007 (every year printed p.88). Selling Rosh Hashanah = being out (no short is stated).",
            claim("36", "25 up / 11 down", "+7.1% avg", "~6 months", "DJIA 1971-2007")),
        d("hsta_election", "Last seven months of election years", "STA 2008 p.52 · Only One Loss Last 7 Months of Election Years", "Long", Live,
            &["F|Election years (year divisible by 4): \"a very positive picture for the last seven or eight months of election years\" (p.52)",
              "E|Last 7 months: buy at the close of the last trading day of May (p.months=8: of April) of an election year",
              "X|Sell at the close of the last trading day of December"],
            "Book index: S&P 500 1952-2004 (each election year printed p.52). The book presents a statistic for both spans; 7 months is the page's headline, 8 months a labelled option.",
            claim("14", "13 up / 1 down", "+7.2% avg (last 7 months)", "7 months", "S&P 500 election years 1952-2004")),
        // ---- NotTestable / NeedsData
        d("hsta_first5", "January's First Five Days early warning", "STA 2008 p.14", "Long/Short", NotTestable("forecast indicator: predicts the full-year direction; the book gives no entry or exit order"),
            &["F|\"The last 36 up First Five Days were followed by full-year gains 31 times\" (p.14)"], "Statistic only.", None),
        d("hsta_jan_barometer", "January Barometer", "STA 2008 p.16", "Long/Short", NotTestable("forecast indicator ('as the S&P goes in January, so goes the year'); the book gives no entry or exit order"),
            &["F|\"our January Barometer states that as the S&P goes in January, so goes the year\" (p.16)"], "Statistic only; see also Down Januarys p.42.", None),
        d("hsta_dec_low", "December Low Indicator", "STA 2008 p.40", "Long/Short", NotTestable("warning sign only ('watch out'); no entry or exit order"),
            &["F|\"When the Dow closes below its December closing low in the first quarter, it is frequently an excellent warning sign\" (p.40)"], "Statistic only.", None),
        d("hsta_dfdm", "Down Friday / Down Monday warning", "STA 2008 p.100", "Short", NotTestable("\"Use DF/DM as a warning to examine market conditions carefully\" (p.100): no order"),
            &["F|A down Friday followed by a down Monday \"is often an important market inflection point\" (p.100)"], "Statistic only.", None),
        d("hsta_holidays", "Three days before and after holidays", "STA 2008 p.84", "Long", NotTestable("per-holiday, per-day average tables; \"each holiday moves to its own rhythm\" (p.84); no single entry/exit rule is stated"),
            &["F|Average % change 3 days before and after eight market holidays, 1980-April 2007 (p.84)"], "Statistic only.", None),
        d("hsta_weekdays", "Don't sell stocks on Friday (day-of-week)", "STA 2008 p.68, p.141-144", "Long", NotTestable("day-of-week point tables; no entry/exit rule"),
            &["F|\"Monday and Tuesday have been the most consistently bullish days of the week\" (p.68)"], "Statistic only.", None),
        d("hsta_rally_seasons", "A rally for all seasons", "STA 2008 p.70", "Long", NotTestable("measured from the season's lowest close to the next quarter's highest close: known only in hindsight"),
            &["F|\"lowest close in the Dow Jones Industrials in May or June to the highest close in July, August, or September\" (p.70)"], "Hindsight statistic.", None),
        d("hsta_first_month_q", "First month of quarters most bullish", "STA 2008 p.74", "Long", NotTestable("monthly averages by quarter; no trading rule"),
            &["F|\"The opening month of the first three quarters produces the greatest gains\" (p.74)"], "Statistic only.", None),
        d("hsta_triple_witch", "Triple-witching week patterns", "STA 2008 p.76", "Long/Short", NotTestable("weekly point tables; no entry/exit rule"),
            &["F|\"DOWN WEEKS TEND TO FOLLOW DOWN TWWs\" (p.76)"], "Statistic only.", None),
        d("hsta_jan_effect", "January Effect starts mid-December (small vs large caps)", "STA 2008 p.104-106", "Long", NeedsData("Russell 2000 and Russell 1000 daily data (and the book states averages, not orders)"),
            &["F|Russell 2000 vs Russell 1000 half-month returns from 12/15 (p.104)"], "Statistic; would need small/large-cap index data.", None),
        d("hsta_free_lunch", "Free lunch: NYSE stocks at new lows on December 15", "STA 2008 p.110", "Long", NotTestable("stock selection is discretionary (\"we sift through the chaff\"; 'stocks down the most are selected') and needs the NYSE new-low universe"),
            &["E|\"NYSE stocks selling at their lows on December 15 will usually outperform the market by February 15\" (p.110)"], "Needs a stock universe with 52-week lows; selection not mechanical.", None),
        d("hsta_sector", "Sector seasonality percentage plays", "STA 2008 p.114-116", "Long", NotTestable("start/finish given only as the beginning/middle/end third of a month (B/M/E); exact points are sent to newsletter subscribers"),
            &["E|e.g. \"XAU Gold & Silver July E September E\" (p.114)"], "Would also need the sector indices (XNG, BTK, XAU, SOX, ...).", None),
        d("hsta_presidential", "Presidential election / stock market cycle statistics", "STA 2008 p.130; 2010 p.130", "Long", NotTestable("cycle statistics only; the cycle's only trading rule is hsta_best6_4yr"),
            &["F|Gains by year of the four-year cycle since 1833 (p.130)"], "Statistic only.", None),
        d("hsta_decennial", "Decennial cycle", "STA 2008 p.129", "Long", NotTestable("year-of-decade statistics; no trading rule"),
            &["F|Average gains by year of the decade (p.129)"], "Statistic only.", None),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    Some(match id {
        "hsta_best6" => ("hsta_best6", "Best Six Months", "Calendar Nov-Apr"),
        "hsta_best6_macd" => ("hsta_best6", "Best Six Months", "MACD timing"),
        "hsta_best8_macd" => ("hsta_best6", "Best Six Months", "NASDAQ Best Eight Months + MACD"),
        "hsta_best6_4yr" => ("hsta_best6", "Best Six Months", "MACD + four-year cycle"),
        "hsta_santa" | "hsta_stpat" | "hsta_midyear" | "hsta_thanksgiving" | "hsta_yk_passover" => ("hsta_holiday", "Holiday and seasonal windows", match id {
            "hsta_santa" => "Santa Claus rally",
            "hsta_stpat" => "Day before St. Patrick's",
            "hsta_midyear" => "NASDAQ mid-year rally",
            "hsta_thanksgiving" => "Thanksgiving",
            _ => "Yom Kippur to Passover",
        }),
        "hsta_first_day" => ("hsta_month", "Turn of the month", "First trading day"),
        "hsta_super8" => ("hsta_month", "Turn of the month", "Super-8 days"),
        "hsta_election" => ("hsta_cycle", "Presidential cycle", "Election-year last months"),
        "hsta_presidential" | "hsta_decennial" => ("hsta_cycle", "Presidential cycle", "Statistics"),
        "hsta_first5" | "hsta_jan_barometer" | "hsta_dec_low" | "hsta_dfdm" => ("hsta_forecast", "Almanac forecasting indicators", "Indicator"),
        "hsta_holidays" | "hsta_weekdays" | "hsta_rally_seasons" | "hsta_first_month_q" | "hsta_triple_witch" | "hsta_jan_effect" | "hsta_free_lunch" | "hsta_sector" => ("hsta_stats", "Almanac statistics", "Statistic"),
        _ => return None,
    })
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    let macd = |sell_month: f64| {
        vec![
            p("buy_fast", "Buy MACD fast EMA", 8., 5., 15., 1.),
            p("buy_slow", "Buy MACD slow EMA", 17., 13., 30., 1.),
            p("buy_signal", "Buy MACD trigger EMA", 9., 5., 13., 1.),
            p("sell_fast", "Sell MACD fast EMA", 12., 5., 15., 1.),
            p("sell_slow", "Sell MACD slow EMA", 25., 13., 30., 1.),
            p("sell_signal", "Sell MACD trigger EMA", 9., 5., 13., 1.),
            p("buy_month", "Start looking for the buy in month", 10., 9., 11., 1.),
            p("sell_month", "Start looking for the sell in month", sell_month, 3., 7., 1.),
        ]
    };
    Some(match id {
        "hsta_best6" => vec![p("entry_month", "Buy at the close of the last day of month", 10., 9., 11., 1.), p("exit_month", "Sell at the close of the last day of month", 4., 3., 6., 1.)],
        "hsta_best6_macd" | "hsta_best6_4yr" => macd(4.),
        "hsta_best8_macd" => macd(6.),
        "hsta_santa" => vec![p("days_before", "Last trading days of December", 5., 1., 10., 1.), p("days_after", "First trading days of January", 2., 1., 5., 1.)],
        "hsta_midyear" => vec![p("june_day", "Buy at the close of June's Nth-to-last day", 4., 2., 8., 1.), p("july_day", "Sell at the close of July's Nth trading day", 9., 3., 15., 1.)],
        "hsta_thanksgiving" => vec![p("exit_monday", "Exit Monday after (1) instead of Friday (0)", 0., 0., 1., 1.)],
        "hsta_super8" => vec![
            p("last_n", "Last trading days of the previous month", 3., 1., 5., 1.),
            p("first_n", "First trading days of the month", 2., 1., 5., 1.),
            p("mid_from", "Middle window first day", 9., 6., 12., 1.),
            p("mid_to", "Middle window last day", 11., 8., 15., 1.),
        ],
        "hsta_election" => vec![p("months", "Last N months of the election year (7 or 8)", 7., 7., 8., 1.)],
        "hsta_first_day" | "hsta_stpat" | "hsta_yk_passover" => vec![],
        _ => return None,
    })
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    let schema = params(id)?;
    let n = x.r.len();
    if n == 0 || from > to || to >= n {
        return Some(vec![]);
    }
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).map_or(0.0, |d| d.def));
    let u = |k: &str| g(k).round().max(0.0) as usize;
    let cal = Cal { x };
    let (y0, _, _) = ymd(&x.r[0].date);
    let (y1, _, _) = ymd(&x.r[n - 1].date);
    let mut w: Vec<(usize, Option<usize>)> = vec![];
    let tr = match id {
        "hsta_best6" => {
            let (em, xm) = (u("entry_month") as u32, u("exit_month") as u32);
            for i in 0..n {
                if cal.ym(i).1 == em && cal.last_of_month(i) {
                    w.push((i, (i + 1..n).find(|&j| cal.ym(j).1 == xm && cal.last_of_month(j))));
                }
            }
            windows(x, from, to, 1.0, w, "last day of exit month · close")
        }
        "hsta_best6_macd" | "hsta_best8_macd" | "hsta_best6_4yr" => {
            let (bm, bs) = macd(&x.c, u("buy_fast"), u("buy_slow"), u("buy_signal"));
            let (sm, ss) = macd(&x.c, u("sell_fast"), u("sell_slow"), u("sell_signal"));
            let (bmo, smo) = (u("buy_month") as u32, u("sell_month") as u32);
            let cycle = id == "hsta_best6_4yr";
            // post-election (y%4==1) and midterm (y%4==2) years only
            let ok = |y: i64| !cycle || matches!(y.rem_euclid(4), 1 | 2);
            let (mut out, mut cur, mut armed): (Vec<Trade>, Option<Trade>, bool) = (vec![], None, false);
            for i in from..=to {
                let (y, m) = cal.ym(i);
                match cur.as_mut() {
                    None => {
                        if m == bmo && ok(y) {
                            armed = true;
                        }
                        if armed && bm[i] > bs[i] {
                            cur = Some(mk(1.0, x, i, ""));
                            armed = false;
                        }
                    }
                    Some(t) => {
                        if m == smo && ok(y) && i > t.legs[0].i {
                            armed = true;
                        }
                        if armed && sm[i] < ss[i] {
                            t.exit_i = i;
                            t.exit_px = x.c[i];
                            t.signal_i = i;
                            t.reason = "MACD sell trigger · close".into();
                            out.push(cur.take().unwrap());
                            armed = false;
                        }
                    }
                }
            }
            if let Some(mut t) = cur {
                t.exit_i = to;
                t.exit_px = x.c[to];
                t.signal_i = to;
                t.open = true;
                t.reason = "open".into();
                out.push(t);
            }
            out
        }
        "hsta_santa" => {
            let (b, a) = (u("days_before"), u("days_after"));
            for i in 0..n {
                if i + 1 < n && cal.d(i + 1).0 != cal.d(i).0 && i >= b && a >= 1 {
                    let xi = i + a;
                    let ok = xi < n && cal.d(xi).0 == cal.d(i + 1).0;
                    w.push((i - b, if ok { Some(xi) } else { None }));
                }
            }
            windows(x, from, to, 1.0, w, "January trading day · close")
        }
        "hsta_stpat" => {
            for y in y0..=y1 {
                if let Some(s) = cal.on_or_after(days_from_civil(y, 3, 17)) {
                    if s >= 2 && cal.d(s).0 == y && cal.d(s - 2).0 == y {
                        w.push((s - 2, Some(s - 1)));
                    }
                }
            }
            windows(x, from, to, 1.0, w, "day before St. Patrick's · close")
        }
        "hsta_midyear" => {
            let (jd, ld) = (u("june_day").max(1), u("july_day").max(1));
            for y in y0..=y1 {
                let (Some(lj), Some(fj)) = (cal.on_or_before(days_from_civil(y, 6, 30)), cal.on_or_after(days_from_civil(y, 7, 1))) else { continue };
                if cal.ym(lj) != (y, 6) || cal.ym(fj) != (y, 7) || lj + 1 < jd {
                    continue;
                }
                let e = lj + 1 - jd;
                let xi = fj + ld - 1;
                let ok = xi < n && cal.ym(xi) == (y, 7);
                if cal.ym(e) == (y, 6) {
                    w.push((e, if ok { Some(xi) } else { None }));
                }
            }
            windows(x, from, to, 1.0, w, "July trading day · close")
        }
        "hsta_first_day" => {
            for i in 0..n {
                if cal.last_of_month(i) {
                    w.push((i, Some(i + 1)));
                }
            }
            windows(x, from, to, 1.0, w, "first trading day · close")
        }
        "hsta_thanksgiving" => {
            let monday = g("exit_monday") >= 0.5;
            for y in y0..=y1 {
                let t = thanksgiving(y);
                let (Some(e), xi) = (cal.on_or_before(t - 2), cal.on_or_after(t + if monday { 4 } else { 1 })) else { continue };
                // the Tuesday bar must belong to Thanksgiving week (not a stale bar)
                if days_from_civil(cal.d(e).0, cal.d(e).1, cal.d(e).2) < t - 7 {
                    continue;
                }
                w.push((e, xi));
            }
            windows(x, from, to, 1.0, w, if monday { "Monday after Thanksgiving · close" } else { "Friday after Thanksgiving · close" })
        }
        "hsta_super8" => {
            let (ln, fnn, mf, mt) = (u("last_n"), u("first_n").max(1), u("mid_from").max(1), u("mid_to").max(1));
            for f in 1..n {
                if !cal.first_of_month(f) {
                    continue;
                }
                let ym = cal.ym(f);
                let in_month = |k: usize| k < n && cal.ym(k) == ym;
                // last ln days of the previous month + first fnn days: entry close of bar f-1-ln
                if f >= ln + 1 {
                    let xi = f + fnn - 1;
                    w.push((f - 1 - ln, if in_month(xi) { Some(xi) } else { None }));
                }
                // middle days mf..=mt: entry close of day mf-1 (bar f+mf-2), exit close of day mt (bar f+mt-1)
                if mt >= mf && mf >= 2 && in_month(f + mf - 2) {
                    let xi = f + mt - 1;
                    w.push((f + mf - 2, if in_month(xi) { Some(xi) } else { None }));
                }
            }
            windows(x, from, to, 1.0, w, "Super-8 window end · close")
        }
        "hsta_yk_passover" => {
            for y in y0..=y1 {
                let yk = yom_kippur(y);
                let (Some(e), xi) = (cal.on_or_before(yk), cal.on_or_before(passover(y + 1))) else { continue };
                if days_from_civil(cal.d(e).0, cal.d(e).1, cal.d(e).2) < yk - 7 {
                    continue;
                }
                // the exit bar is only known once a bar after Passover exists (else: still open)
                let known = cal.on_or_after(passover(y + 1) + 1).is_some();
                w.push((e, if known { xi } else { None }));
            }
            windows(x, from, to, 1.0, w, "Passover · close")
        }
        "hsta_election" => {
            let em = 12 - u("months").clamp(7, 8) as u32;
            for i in 0..n {
                let (y, m) = cal.ym(i);
                if y.rem_euclid(4) == 0 && m == em && cal.last_of_month(i) {
                    w.push((i, (i + 1..n).find(|&j| cal.ym(j) == (y, 12) && cal.last_of_month(j))));
                }
            }
            windows(x, from, to, 1.0, w, "last trading day of December · close")
        }
        _ => return None,
    };
    Some(tr)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;

    #[test]
    fn hebrew_dates_match_the_almanac_table() {
        // 2008 ed. p.88: Rosh Hashanah, Yom Kippur, next-year Passover
        let book = [(1971, "1971-09-20", "1971-09-29", "1972-03-30"), (1973, "1973-09-27", "1973-10-06", "1974-04-07"),
            (1986, "1986-10-04", "1986-10-13", "1987-04-14"), (2005, "2005-10-04", "2005-10-13", "2006-04-13"),
            (2006, "2006-09-23", "2006-10-02", "2007-04-03")];
        for (y, rh, yk, ps) in book {
            assert_eq!(iso(rosh_hashanah(y)), rh);
            assert_eq!(iso(yom_kippur(y)), yk);
            assert_eq!(iso(passover(y + 1)), ps);
        }
        assert_eq!(iso(thanksgiving(2006)), "2006-11-23");
        assert_eq!(iso(thanksgiving(2008)), "2008-11-27");
    }

    fn rows(dates: &[&str]) -> Vec<Row> {
        dates.iter().enumerate().map(|(k, d)| Row { ticker: "T".into(), date: d.to_string(), open: 100. + k as f64, high: 101. + k as f64, low: 99. + k as f64, close: 100. + k as f64, volume: None }).collect()
    }

    #[test]
    fn santa_window_is_last_five_december_days_plus_two_january_days() {
        let d = ["2001-12-20", "2001-12-21", "2001-12-24", "2001-12-26", "2001-12-27", "2001-12-28", "2001-12-31", "2002-01-02", "2002-01-03", "2002-01-04"];
        let r = rows(&d);
        let x = Ctx::new(&r, &r, &[]);
        let t = run("hsta_santa", &x, 0, 9, &Params::new()).unwrap();
        assert_eq!(t.len(), 1);
        assert_eq!((t[0].legs[0].i, t[0].exit_i), (1, 8)); // close of 12-21 (6th-to-last) -> close of Jan 3 (2nd day)
    }
}
