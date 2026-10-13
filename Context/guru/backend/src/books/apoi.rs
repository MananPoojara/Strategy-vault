//! Gerald Appel, Opportunity Investing (FT Press / Pearson, 2007).
//! Mostly portfolio and fund-selection material. Live: the presidential-cycle strategies (Ch 8).
//! The high-yield 1.25/0.50 model, the 28% new-highs signal and the fund rotation models need data
//! the vault lacks (NeedsData). Models already in the vault from apta are not repeated (README).
//! Page refs are printed pages (from the PDF text layer). Sources and checks: library/apoi/.
use super::apta::run_long;
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "apoi";

pub fn catalogue() -> Vec<StrategyDef> {
    let d = |id, name, group, status, rules, notes| StrategyDef {
        id, name, group, session: 0, dir: "Long", status, needs_volume: false, needs_vix: false, needs_series: &[],
        book: BOOK, rules, notes, claim: None,
    };
    use Status::*;
    let nt = |id, name, group, why| d(id, name, group, NotTestable(why), &[], "");
    vec![
        d("apoi_pres_s1", "Presidential election cycle, Strategy 1 (October 1 of year two to the end of election year)", "Ch 8 · Winning with the presidential election cycle", Live,
            &["E|\"buying the Standard & Poor's 500 Index on October 1 of the second year of presidential terms\" (p.212): buy at the close of the first trading day on or after October 1 of a midterm year (year divisible by 4, plus 2).",
              "X|\"and holding positions through December 31 of election year (Strategy for Investor 1). The holding period for stocks would be 27 months. Stocks are sold at the close of election year\" (p.212): sell at the close of the last trading day of the election year.",
              "F|\"Investor 1 remaining in cash until the next entry, 21 months later.\" (p.212)"],
            "Book (from M. Nickles, Pepperdine Graziadio Business Report 2004): S&P 500 1952-2004, profitable every period, $1,000 -> $72,701 (table p.213, per period %). US election calendar applied to every index (India = instrument swap only)."),
        d("apoi_pres_s2", "Presidential election cycle, Strategy 2 (inauguration to September 30 of year two)", "Ch 8 · Winning with the presidential election cycle", Live,
            &["E|\"A second investor buys stocks at the time Investor 1 closes out his positions (inaugural day)\" (p.212): buy at the close of the last trading day of the election year.",
              "X|\"and holds positions through September 30 of the second year of the presidential term (Strategy 2)\" (p.212): sell at the close of the last trading day on or before September 30 of the midterm year."],
            "Book: profitable in 7 of 13 periods, $1,000 -> $643 (p.212-213)."),
        d("apoi_hy_125_050", "1.25/0.50 timing model for high-yield bond funds", "Ch 5 · Securing junk bond yields", NeedsData("needs weekly total-return prices (NAV with distributions) of high-yield bond mutual funds; the vault holds equity indices only"),
            &["F|\"Trades are made on the first trading day of each week, based on the price levels of the last day of the preceding week.\" (Rule 1, p.139)",
              "E|\"You buy when the total return level of the fund(s) you are tracking rises by 1.25% from the lowest low that has existed since the most recent sell signal.\" (Rule 2, p.139)",
              "X|\"You sell when the total return level ... declines by 0.50% from the most recent high since the onset of the most recent buy signal. There are no other buy/sell rules in this model.\" (p.139-140)"],
            "Book's performance tables for individual high-yield funds are on p.146-148."),
        d("apoi_nh28", "28% major-term new highs buy signal", "Ch 8 · Market breadth", NeedsData("needs weekly NYSE new highs and the weekly number of NYSE issues traded (Barron's); not in the vault"),
            &["E|\"At the end of each week, check out the number of issues on the New York Stock Exchange that advanced to new highs in price during that week. Divide this number into the number of issues traded ... If the number of new highs is 0.28 (28%) of the total number of issues traded ... a major-term buy signal\" (p.225)",
              "X|\"that remains in effect for the next 52 weeks!\" (p.225)"],
            "Book: S&P 500 1943-2004, 12 signals, 11 profitable (91.7%), average +17.25% (p.225-226)."),
        d("apoi_sector_rotation", "Seven Sector Portfolio rebalanced every six months", "Ch 2 · Rebalancing the portfolio", NeedsData("needs total-return histories of the seven sector mutual funds (financials, utilities, REITs, energy, ...); not in the vault"),
            &["E|\"each of the top two sectors start the new 6-month period with 28.6% of your total portfolio assets, each of the next three sectors ... 14.3% ... and each of the last two sectors in performance start the new 6-month period with no assets\" (p.50-51)",
              "X|\"Hold positions for 6 months. At the end of six months, again rebalance\" (p.51)"], ""),
        d("apoi_fund_leaders", "Top-decile mutual fund rotation (quarterly ranking)", "Ch 3 · Selecting mutual funds", NeedsData("needs a mutual-fund universe with quarterly performance; not in the vault"),
            &["E|\"I define a mutual fund as a market leader if its performance over the most-recent 3 months has been more favorable than 90% of all mutual funds\" (p.75)",
              "X|\"Hold the leaders until they stop leading. Change to new, emerging leadership.\" (p.75)"], ""),
        nt("apoi_mood", "Public psychology, Fed action and seasonal mood indicators", "Ch 1 · Market mood indicators", "\"Mood indicators do not provide precise entry and exit points\" (p.20)"),
        nt("apoi_pe_levels", "Price/earnings levels (buy below 9.3, caution above 20.2)", "Ch 7 · Opportunity Investing Strategy 1", "no exit (\"Become increasingly cautious\", p.186) and needs S&P 500 earnings history"),
        nt("apoi_yield_spread", "Baa bond yield minus earnings yield zones", "Ch 7 · Opportunity Investing Strategy 2", "statistics by zone without a trading rule; needs Moody's Baa and forward/trailing EPS (p.188-197)"),
        nt("apoi_earnings_growth", "Buy when earnings news is bad, sell when best", "Ch 7 · Earnings", "zone statistics only (Figure 7-5), no rule (p.200)"),
        nt("apoi_breadth_warnings", "A-D line confirmation, 10-day 60% breadth, split-market warning, bottom-finding NH/NL levels", "Ch 8 · Market breadth", "\"should probably not be taken as sell signals\" (p.224); bottom-finding is gradual and judgement (\"Prepare to enter\", p.222)"),
        nt("apoi_four_year_cycle", "Four-year cycle and phases", "Ch 8 · Market cycles", "cycle lows judged on charts (p.206-211)"),
        nt("apoi_reit_channel", "REIT major channel", "Ch 9 · REITs", "hand-drawn channel (p.243-245)"),
        nt("apoi_income_funds", "Bond ladders, closed-end discounts, overseas and commodity funds, inflation hedges", "Ch 4, 6, 10-12", "portfolio construction without timing rules"),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    Some(match id {
        "apoi_pres_s1" => ("apoi_pres", "Presidential election cycle", "Strategy 1"),
        "apoi_pres_s2" => ("apoi_pres", "Presidential election cycle", "Strategy 2"),
        s if s.starts_with("apoi_") => ("apoi_other", "Other Opportunity Investing material", ""),
        _ => return None,
    })
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    Some(match id {
        "apoi_pres_s1" | "apoi_pres_s2" => vec![],
        _ => return None,
    })
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, _p: &Params) -> Option<Vec<Trade>> {
    params(id)?;
    if from > to || to >= x.r.len() {
        return None;
    }
    let n = x.r.len();
    let ym = |i: usize| -> (i64, i64) { (x.r[i].date[0..4].parse().unwrap_or(0), x.r[i].date[5..7].parse().unwrap_or(0)) };
    let daily: Vec<usize> = (0..n).collect();
    // first trading day on/after Oct 1 of a midterm year; last trading day of an election year;
    // last trading day on/before Sep 30 of a midterm year
    let oct1 = |i: usize| { let (y, m) = ym(i); y % 4 == 2 && m == 10 && i > 0 && ym(i - 1).1 == 9 };
    let year_end = |i: usize| { let (y, _) = ym(i); y % 4 == 0 && i + 1 < n && ym(i + 1).0 != y };
    let sep30 = |i: usize| { let (y, m) = ym(i); y % 4 == 2 && m == 9 && i + 1 < n && ym(i + 1).1 != 9 };
    Some(match id {
        "apoi_pres_s1" => run_long(x, &daily, from, to, |_, i, h| {
            if !h && oct1(i) { Some("October 1 of the midterm year".into()) } else if h && year_end(i) { Some("end of the election year".into()) } else { None }
        }),
        _ => run_long(x, &daily, from, to, |_, i, h| {
            if !h && year_end(i) { Some("end of the election year (inauguration)".into()) } else if h && sep30(i) { Some("September 30 of the midterm year".into()) } else { None }
        }),
    })
}
