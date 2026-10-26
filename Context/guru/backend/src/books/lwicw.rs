//! Larry Williams, "Inner Circle Workshop" manuals and notes (2000). Book id `lwicw`.
//! Sources (read in place, COPY of the four PDFs already in Books/Larry Williams/): "Inner Circle Workshop Notes (2000).pdf"
//! (text, printed pages 1-45), "Inner Circle Workshop Book1 (2000).pdf" (same text typeset, plus tables on Book1 pp.46-55),
//! "Inner Circle Workshop Book2 (2000).pdf" and "Inner Circle Workshop (2000).pdf" (commodity charts, Australian edition of
//! the same text). Transcription and checks: library/lwicw/. Page refs = printed page numbers of the notes / Book1.
//!
//! The S&P printouts are System Writer tests of "S&P 500 IND-9967"; the only report header printed in this work (Book1 p.53,
//! 01/01/83-12/23/98) reads "P.Value $5.000" per 0.01 = $500 per index point (the pre-1997 contract). Dollar stops are
//! converted at that rate (`pt_usd` 500 since 23 Oct 2026; the earlier $250 reading from lwlts stays as an option). Stops are armed on the entry day for open entries (the Monday printout, p.30-31, shows losers of 0 bars and
//! a largest loss of $1,800 = $1,750 stop + $50), from the next day for entries on the close.
//!
//! This file also holds the small simulator shared by the other Larry Williams seminar modules (lwmdc, lwpro): entries at
//! the open, on the close or on a stop; a dollar stop; the bailout (first profitable opening) with an optional delay; the
//! first profitable close; exit on the close N bars after entry; and reversal on an opposite entry.
use crate::connors::{claim, Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "lwicw";

/// Larry Williams block: rules admitted after the 23 Oct 2026 reading and independent ledgers
/// (authors/lwilliams/admission/SOURCE-CONTRACT.md). SPX_POINTS = dollar stops kept as S&P points: actual SPX only.
pub const REVIEWED: &[&str] = &["lwicw_sp_tdw_mon", "lwicw_sp_tdw_tue", "lwicw_sp_tdw_wed", "lwicw_sp_tdw_thu", "lwicw_sp_tdw_fri", "lwicw_sp_tdom", "lwicw_sp_holidays", "lwicw_oops_tdom_buy", "lwicw_oops_tdom_sell", "lwicw_mon_gap_bail"];
pub const SPX_POINTS: &[&str] = &["lwicw_sp_tdw_mon", "lwicw_sp_tdw_tue", "lwicw_sp_tdw_wed", "lwicw_sp_tdw_thu", "lwicw_sp_tdw_fri", "lwicw_sp_tdom", "lwicw_sp_holidays", "lwicw_oops_tdom_buy", "lwicw_oops_tdom_sell", "lwicw_mon_gap_bail"];

const BONDS: &str = "Needs daily T-Bond futures (and Gold futures for the gold filters); the vault has no bond or gold futures series.";

macro_rules! sd {
    ($id:expr, $name:expr, $group:expr, $dir:expr, $status:expr, $rules:expr, $notes:expr, $claim:expr) => {
        StrategyDef { id: $id, name: $name, group: $group, session: 0, dir: $dir, status: $status, needs_volume: false,
            needs_vix: false, needs_series: &[], book: BOOK, rules: $rules, notes: $notes, claim: $claim }
    };
    ($id:expr, $name:expr, $group:expr, $dir:expr, $status:expr, $rules:expr, $notes:expr) => {
        sd!($id, $name, $group, $dir, $status, $rules, $notes, None)
    };
}

const TDW_R: &[&str] = &["E|\"buying on the opening ... for every day of the week\" (S&P 500 Trading Rules, p.38): buy the open of the given weekday",
    "X|\"exiting on the first profitable close with a $3,500 stop\" (p.38): $3,500 = 7 points at the work's $500 a point (Book1 p.53), armed on the entry day",
    "X|Read: first profitable close = the first close after the entry day above the entry price (the author's own code 'Exit Long 1ST PROFT CLOSE ... Exit on Entry Bar: False', MDC manual p.99)"];
const TDW_N: &str = "TDW table p.38 (S&P, test window not printed; the other S&P printouts in these notes run 08/09/82-07/02/99). Each weekday tested on its own; one position at a time.";

pub fn catalogue() -> Vec<StrategyDef> {
    use Status::{Live, NeedsData, NotTestable};
    vec![
        // ---- S&P 500 trading rules (notes p.38-43, Book1 p.49)
        sd!("lwicw_sp_tdw_mon", "S&P buy Monday open, first profitable close", "S&P · Trade day of week", "Long", Live, TDW_R, TDW_N, claim("", "71%", "$230 ($169,810)", "", "S&P (window not printed)")),
        sd!("lwicw_sp_tdw_tue", "S&P buy Tuesday open, first profitable close", "S&P · Trade day of week", "Long", Live, TDW_R, TDW_N, claim("", "65%", "$50 ($39,927)", "", "S&P (window not printed)")),
        sd!("lwicw_sp_tdw_wed", "S&P buy Wednesday open, first profitable close", "S&P · Trade day of week", "Long", Live, TDW_R, TDW_N, claim("", "65%", "-$39 (-$33,842)", "", "S&P (window not printed)")),
        sd!("lwicw_sp_tdw_thu", "S&P buy Thursday open, first profitable close", "S&P · Trade day of week", "Long", Live, TDW_R, TDW_N, claim("", "65%", "$35 ($30,592)", "", "S&P (window not printed)")),
        sd!("lwicw_sp_tdw_fri", "S&P buy Friday open, first profitable close", "S&P · Trade day of week", "Long", Live, TDW_R, TDW_N, claim("", "63%", "-$40 (-$34,665)", "", "S&P (window not printed)")),
        sd!("lwicw_sp_tdom", "S&P buy the open of trade day of month N, bailout", "S&P · Trade day of month", "Long", Live,
            &["E|\"Here you see each TDOM with our bail out and $3,500 stop\" (p.38): buy the open of trading day N of the month (p.tdom; the table covers TDOM 1-23)",
              "X|Bailout: exit on the first opening after the entry day that shows a profit", "X|$3,500 stop = 7 points at $500 a point, armed on the entry day"],
            "TDOM table p.38: e.g. TDOM 1 64% $20, TDOM 10 69% $237, TDOM 13 65% $268, TDOM 23 90% $398 (S&P, window not printed). Default TDOM 1 = the table's first row; every row is tested in verify_lwicw.py.",
            claim("", "64%", "$20 (TDOM 1)", "", "S&P (window not printed)")),
        sd!("lwicw_sp_holidays", "S&P holiday trades", "S&P · Holidays", "Long/Short", Live,
            &["E|New Year's: buy open 3rd TD before; Presidents' Day: buy open 3rd TD before, sell close 2nd TD before; Easter: buy close 2nd TD before, sell open 1st TD after; Memorial Day: buy close 1st TD before; July 4th: buy open 5th TD before; Labor Day: buy open 1st TD after; Thanksgiving: buy close 3rd TD before; Christmas: buy open 5th TD before (p.42)",
              "F|\"1st TD before\" = the last trading day before the holiday (\"For Easter, sell on the close of the day before the holiday\", p.34); Easter = the Good Friday closure",
              "X|\"The exit was to bail out after holding for one day past entry, in other words buy/sell today, hold tomorrow then look to bailout\" (p.42): bailout eligible from the second opening after entry",
              "X|\"The stop is a very small $2,500 from entry\" (p.42) = 5 points at $500 a point; armed on the entry day for open entries",
              "X|Read: an opposite holiday entry while in a trade exits and reverses (Presidents' Day and Easter list both a buy and a sell; System Writer reverses on an opposite entry)"],
            "Printout p.42: S&P 500 IND-9967, 08/09/82-07/02/99. US exchange holidays are computed from the calendar (Presidents' Day = 3rd Monday of February, Memorial Day = last Monday of May, Labor Day = 1st Monday of September, Thanksgiving = 4th Thursday of November, Good Friday from the Easter date, New Year's/July 4th/Christmas on their dates). India: the same US holiday dates are used (instrument swap), counting NIFTY sessions before/after the date.",
            claim("170", "63%", "$639.26 ($108,675)", "1 day", "08/09/82-07/02/99")),
        sd!("lwicw_oops_tdom_buy", "S&P Oops! buy on trade day of month N", "S&P · Oops! by TDOM", "Long", Live,
            &["E|Oops! buy: the open is below the previous day's low, buy on a stop at that low (the author's definition, MDC manual p.103 / LTS p.114)",
              "F|Only on trading day of month N (Book1 p.49 table \"S&P OOPS 1 DAY XIT $2750 STOP\"; buys listed for TDOM 4, 5, 6, 13, 16, 19, 20, 22)",
              "X|$2,750 stop = 5.5 points at $500 a point, armed on the entry day", "X|\"1 day xit\" read as the bailout: notes p.43 give the same TDOM 19 result (\"$45,000 with 83% winners on 18 trades ... with $2,750 stop and bail out\")"],
            "Book1 p.49 table (window not printed). Default TDOM 19 (the row the notes quote); verify_lwicw.py runs every row.",
            claim("18", "83%", "$2,500 ($45,000)", "1 day", "S&P (window not printed)")),
        sd!("lwicw_oops_tdom_sell", "S&P Oops! sell on trade day of month N", "S&P · Oops! by TDOM", "Short", Live,
            &["E|Oops! sell: the open is above the previous day's high, sell on a stop at that high",
              "F|Only on trading day of month N (Book1 p.49 sells: TDOM 3, 5, 6, 17, 22)", "X|$2,750 stop (5.5 points) and bailout, as the buys"],
            "Book1 p.49 table; notes p.43: \"The best TDOM sell days are 3,6,17 netting $56,675 with 77% accuracy\". Default TDOM 3.",
            claim("29", "72%", "$639 ($18,525, TDOM 3)", "1 day", "S&P (window not printed)")),
        sd!("lwicw_mon_gap_bail", "Monday open below Friday's close · bailout, $1,750 stop", "Money management example", "Long", Live,
            &["E|\"a very simple system that buys on the opening every Monday if that opening is less than Fridays close\" (p.30)",
              "X|\"Our exit is standard Bail Out with a $1,750 stop\" (p.30): 3.5 points at $500 a point, armed on the entry day (largest loss $1,800 = stop + $50, losers 0 bars)"],
            "One-contract printout p.31: 172 trades, 69%, $460.47, largest loss $1,800. Market and window are not printed (the dollar sizes fit the S&P; the 100-lot run starts 1/1990 and ends 12/20/96). The same entry with other exits is lwlts_mon_gap_*.",
            claim("172", "69%", "$460.47 ($79,200)", "1 day", "not printed (1990-12/1996?)")),
        // ---- Needs data
        sd!("lwicw_bond_shortterm", "Bond short-term rules (TDW, TDOM, holidays, Gold, price patterns, hidden Oops!)", "Bonds", "Long", NeedsData(BONDS),
            &["E|TDW and TDOM buys on the open, bailout; holiday trades with a $1,400 stop and bailout (p.33-35)", "F|Gold below its close 25 days ago (p.35)",
              "E|Consecutive down closes, gap down closes, Friday set-ups for Monday, hidden Oops! with the projected high/low (H+L+C)/3*2-L/H (p.36-37)"],
            "Printouts: holidays 190 trades 84% $274.74 (01/10/78-07/06/99); patterns 285 trades 87% $254.56 (01/01/90-07/06/99)."),
        sd!("lwicw_bond_best_days", "Bond best buy/sell trade days of the year; Bonds seasonal TDOMs backed by Gold; Bonds Oops! by TDOM", "Bonds", "Long/Short", NeedsData(BONDS),
            &["E|Buy the open of the day shown + 7% of the previous day's range, $1,800 stop or any profitable opening after 3 days (p.44-45)", "E|Book1 p.50-51 tables"], ""),
        sd!("lwicw_sp_bond_filter", "S&P TDW/TDOM with bonds above 14 / 10-15 days ago; first-of-month with bonds; seasonal TDOMs backed by bonds; Oops! revisited", "S&P · Bond filters", "Long/Short", NeedsData(BONDS),
            &["F|\"Bonds in an up-trend, i.e., greater than 14 days ago\" (p.39)", "F|First of month: \"Bonds close is greater than 32 days ago\" (p.43)", "F|Oops! revisited: buy Mon/Tue/Fri if Bonds > 2 days ago; sell if Bonds < 4 days ago (Book1 p.53)"],
            "Every printed result uses the bond filter."),
        sd!("lwicw_sp_patterns", "S&P pattern rules (Monday Oops!, hidden Oops!, consecutive down closes, Monday-only trades, short-term sells)", "S&P · Patterns", "Long/Short",
            NotTestable("The entry rules are listed (p.40-41) and several need bonds, but no exit is stated for them; the combined printout (602 trades, 84%, $542.74, 08/09/82-07/02/99) does not say which exit it used."),
            &["E|e.g. \"take an OOPS! Buy on Monday as long as Friday was not an outside day\" (p.40)"], ""),
        sd!("lwicw_sp_first_of_month", "S&P first trading day of the month, 2-day exit, 90% range stop", "S&P · Additional signals", "Long",
            NotTestable("\"Buy the first trading day of every month ... with a 2 day exit and 90% of the range stop\" (p.43): the entry order (open or close), the meaning of '2 day exit' and the range the 90% refers to are not stated."), &[], ""),
        sd!("lwicw_sp_best_days", "S&P best short-term buy/sell days of the year", "S&P · Additional signals", "Long/Short",
            NotTestable("The table (p.43-44) gives a TDOM per month with results but no entry order or exit."), &[], ""),
        sd!("lwicw_setups", "Set-ups: Williams Valuation Index, Commercials (COT) index, +60 DMI, pinch and paunch, Market Vane sentiment, seasonals, open interest, premiums", "Set-ups", "Long/Short",
            NeedsData("Needs commodity futures, Gold, COT commercial positions, Market Vane sentiment, open interest and spread (nearby/next) data; the readings are also used by judgement ('a cluster of at least 4', p.1)."), &[], ""),
        sd!("lwicw_18day_ma", "18-day moving average two-bar entry", "Entry techniques", "Long/Short",
            NotTestable("The entry is exact (two consecutive non-inside days with lows above / highs below the 18-day average, enter at their highest high / lowest low, p.17) but no exit is given for it; the 10-year table is commodity futures."), &[], ""),
        sd!("lwicw_failure_days", "Fake out day, smash day, naked close, specialists' trap, qualified trend line, Pro-Go, volatility breakout, 3-bar exit, triangles, 8-week box", "Entry / exit techniques", "Long/Short",
            NotTestable("Entry patterns without stated exits, or judgement (\"when the market is primed\", 'set up of some sort', trend lines, boxes); Pro-Go rules 'as provided at the seminar' are not in the manual."), &[], ""),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    let f = |a, b, c| Some((a, b, c));
    match id {
        "lwicw_sp_tdw_mon" => f("lwicw_tdw", "S&P trade day of week", "Monday"),
        "lwicw_sp_tdw_tue" => f("lwicw_tdw", "S&P trade day of week", "Tuesday"),
        "lwicw_sp_tdw_wed" => f("lwicw_tdw", "S&P trade day of week", "Wednesday"),
        "lwicw_sp_tdw_thu" => f("lwicw_tdw", "S&P trade day of week", "Thursday"),
        "lwicw_sp_tdw_fri" => f("lwicw_tdw", "S&P trade day of week", "Friday"),
        "lwicw_sp_tdom" => f("lwicw_tdom", "S&P trade day of month", "Buy TDOM N"),
        "lwicw_sp_holidays" => f("lwicw_holidays", "Holiday trades", "S&P"),
        "lwicw_oops_tdom_buy" => f("lwicw_oops", "Oops! by TDOM", "Buys"),
        "lwicw_oops_tdom_sell" => f("lwicw_oops", "Oops! by TDOM", "Sells"),
        "lwicw_mon_gap_bail" => f("lwicw_mon_gap", "Monday gap-down buy", "Bailout, $1,750 stop"),
        "lwicw_bond_shortterm" | "lwicw_bond_best_days" => f("lwicw_bonds", "Bond short-term trading", "Needs bond data"),
        "lwicw_sp_bond_filter" => f("lwicw_tdom", "S&P trade day of month", "Bond filtered"),
        "lwicw_sp_patterns" | "lwicw_sp_first_of_month" | "lwicw_sp_best_days" => f("lwicw_sp_other", "Other S&P rules", "Not testable"),
        "lwicw_setups" | "lwicw_18day_ma" | "lwicw_failure_days" => f("lwicw_setups", "Set-ups and entry techniques", "Not testable"),
        _ => None,
    }
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    // Larry Williams block (23 Oct 2026): this work's own S&P report header (Book1 p.53, "S&P 500 IND-9967 ... Conv 2
    // P.Value $5.000" per 0.01) is $500 a point, so the book's dollar stops are converted at $500; 250 (the lwlts rate) is an option.
    let pt = || p("pt_usd", "Dollars per index point (book: S&P $500, Book1 p.53 header; 250 = old reading)", 500.0, 50.0, 2500.0, 50.0);
    let stop = |d: f64| p("stop_usd", "Dollar stop (book $)", d, 0.0, 6000.0, 250.0);
    Some(match id {
        "lwicw_sp_tdw_mon" | "lwicw_sp_tdw_tue" | "lwicw_sp_tdw_wed" | "lwicw_sp_tdw_thu" | "lwicw_sp_tdw_fri" => vec![stop(3500.0), pt()],
        "lwicw_sp_tdom" => vec![p("tdom", "Trading day of the month", 1.0, 1.0, 23.0, 1.0), stop(3500.0), pt()],
        "lwicw_sp_holidays" => vec![stop(2500.0), p("bail_delay", "Bars held after entry before the bailout applies (book 1)", 1.0, 0.0, 5.0, 1.0), pt()],
        "lwicw_oops_tdom_buy" => vec![p("tdom", "Trading day of the month", 19.0, 1.0, 23.0, 1.0), stop(2750.0), pt()],
        "lwicw_oops_tdom_sell" => vec![p("tdom", "Trading day of the month", 3.0, 1.0, 23.0, 1.0), stop(2750.0), pt()],
        "lwicw_mon_gap_bail" => vec![stop(1750.0), pt()],
        _ => return None,
    })
}

// ------------------------------------------------------------------ calendar helpers

/// Day of week from "YYYY-MM-DD": 0 = Sunday .. 6 = Saturday.
pub(crate) fn dow(date: &str) -> u32 { crate::books::lwlts::dow(date) }

/// Trading day of the month (1-based) for every bar.
pub(crate) fn tdm(x: &Ctx) -> Vec<usize> {
    let mut out = vec![0; x.r.len()];
    for i in 0..x.r.len() {
        out[i] = if i > 0 && x.r[i - 1].date[..7] == x.r[i].date[..7] { out[i - 1] + 1 } else { 1 };
    }
    out
}

fn ymd(y: i32, m: u32, d: u32) -> String { format!("{y:04}-{m:02}-{d:02}") }

/// n-th (1-based) weekday `wd` (0 = Sunday) of month; n = 0 means the last one.
fn nth_wd(y: i32, m: u32, wd: u32, n: u32) -> String {
    let days = [31, if y % 4 == 0 && (y % 100 != 0 || y % 400 == 0) { 29 } else { 28 }, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31][m as usize - 1];
    let all: Vec<u32> = (1..=days).filter(|&d| dow(&ymd(y, m, d)) == wd).collect();
    let d = if n == 0 { *all.last().unwrap() } else { all[n as usize - 1] };
    ymd(y, m, d)
}

/// Easter Sunday (Gregorian, anonymous algorithm).
fn easter(y: i32) -> (u32, u32) {
    let (a, b, c) = (y % 19, y / 100, y % 100);
    let (d, e) = (b / 4, b % 4);
    let g = (8 * b + 13) / 25;
    let h = (19 * a + b - d - g + 15) % 30;
    let (i, k) = (c / 4, c % 4);
    let l = (32 + 2 * e + 2 * i - h - k) % 7;
    // Larry Williams block (23 Oct 2026): the anonymous algorithm's m has no +90 (that belongs to another variant); with it
    // Easter came a week early in 10 years since 1950 (e.g. 2000, 2011, 2014, 2025).
    let m = (a + 11 * h + 22 * l) / 451;
    let mo = (h + l - 7 * m + 114) / 31;
    let da = (h + l - 7 * m + 114) % 31 + 1;
    (mo as u32, da as u32)
}

/// Good Friday as "YYYY-MM-DD".
fn good_friday(y: i32) -> String {
    let (m, d) = easter(y);
    let (m, d) = if d > 2 { (m, d - 2) } else { (3, 31 + d - 2) };
    ymd(y, m, d)
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub(crate) enum Hol { NewYear, Presidents, Easter, Memorial, July4, Labor, Thanksgiving, Christmas }

/// US exchange holiday reference date (calendar date, not the observed weekday) for year y.
pub(crate) fn holiday(h: Hol, y: i32) -> String {
    match h {
        Hol::NewYear => ymd(y, 1, 1),
        Hol::Presidents => nth_wd(y, 2, 1, 3),
        Hol::Easter => good_friday(y),
        Hol::Memorial => nth_wd(y, 5, 1, 0),
        Hol::July4 => ymd(y, 7, 4),
        Hol::Labor => nth_wd(y, 9, 1, 1),
        Hol::Thanksgiving => nth_wd(y, 11, 4, 4),
        Hol::Christmas => ymd(y, 12, 25),
    }
}

/// Bar index of the k-th trading day before (k >= 1, 1 = last bar dated before the holiday) or after (k <= -1, -1 = first bar
/// dated after the holiday) the holiday date `hd`.
pub(crate) fn td_rel(x: &Ctx, hd: &str, k: i32) -> Option<usize> {
    let first_after = x.r.partition_point(|r| r.date.as_str() <= hd);
    let first_on_or_after = x.r.partition_point(|r| r.date.as_str() < hd);
    if k > 0 {
        // Larry Williams block (23 Oct 2026): the data must reach the holiday, or the last bars would be taken as the days
        // before a holiday that lies beyond the data.
        if first_on_or_after >= x.r.len() { return None; }
        let j = first_on_or_after.checked_sub(k as usize)?;
        Some(j)
    } else {
        let j = first_after + (-k - 1) as usize;
        (j < x.r.len()).then_some(j)
    }
}

// ------------------------------------------------------------------ simulator

#[derive(Clone, Copy, Debug)]
pub(crate) enum Ord { Open, Close, Stop(f64) }

#[derive(Clone, Copy)]
pub(crate) struct Cfg {
    pub(crate) stop: f64,               // protective stop distance in points (NaN or <= 0 = none)
    pub(crate) day0: bool,              // stop armed on the entry bar (open and stop entries)
    pub(crate) bail: Option<usize>,     // bailout on the first profitable opening of a bar > entry bar + k
    pub(crate) prof_close: bool,        // exit on the first profitable close after the entry bar
    pub(crate) hold: Option<usize>,     // exit on the close `hold` bars after the entry bar (0 = day trade)
    pub(crate) reverse: bool,           // an opposite entry exits and reverses
}

impl Cfg {
    pub(crate) fn new(stop: f64) -> Self { Cfg { stop, day0: true, bail: None, prof_close: false, hold: None, reverse: false } }
}

struct Pos { dir: f64, i: usize, px: f64, stop: f64, kind: &'static str }

fn done(p: &Pos, i: usize, px: f64, why: &str) -> Trade {
    Trade { dir: p.dir, legs: vec![Leg { i: p.i, px: p.px, w: 1.0 }], exit_i: i, exit_px: px, reason: why.into(),
        entry_kind: p.kind, signal_i: i, open: false, exits: vec![] }
}

/// Single-position bar simulator. `sig(i)` returns the entry orders for bar i as (dir, order); it may read bar i's open
/// (rules that test the opening) but nothing later in bar i except for `Ord::Close` orders, which are filled at bar i's close.
pub(crate) fn sim(x: &Ctx, from: usize, to: usize, cfg: &Cfg, sig: impl Fn(usize) -> Vec<(f64, Ord)>) -> Vec<Trade> {
    let mut out = vec![];
    let mut pos: Option<Pos> = None;
    let sd = if cfg.stop.is_finite() && cfg.stop > 0.0 { cfg.stop } else { f64::NAN };
    let mk = |dir: f64, i: usize, px: f64, kind: &'static str| Pos { dir, i, px, stop: px - dir * sd, kind };
    for i in from.max(1)..=to {
        let (o, h, l, c) = (x.o[i], x.h[i], x.l[i], x.c[i]);
        // 1. at the open: bailout, then a stop gapped through
        if let Some(p) = &pos {
            let why = if cfg.bail.map_or(false, |k| i > p.i + k) && p.dir * (o - p.px) > 0.0 { Some("bailout: first profitable opening") }
                else if p.stop.is_finite() && p.dir * (o - p.stop) <= 0.0 { Some("stop (gap at the open)") } else { None };
            if let Some(w) = why { out.push(done(p, i, o, w)); pos = None; }
        }
        let orders = sig(i);
        // 2. entries at the open, and stop entries gapped through
        let enter = |pos: &mut Option<Pos>, out: &mut Vec<Trade>, dir: f64, px: f64, kind: &'static str| -> bool {
            match pos.take() {
                None => { *pos = Some(mk(dir, i, px, kind)); true }
                Some(p) if p.dir != dir && cfg.reverse => { out.push(done(&p, i, px, "reverse: opposite entry")); *pos = Some(mk(dir, i, px, kind)); true }
                Some(p) => { *pos = Some(p); false }
            }
        };
        let mut stop_orders = vec![];
        for &(dir, od) in &orders {
            match od {
                Ord::Open => { enter(&mut pos, &mut out, dir, o, "open"); }
                Ord::Stop(lv) if dir * (o - lv) >= 0.0 => { enter(&mut pos, &mut out, dir, o, "open"); }
                Ord::Stop(lv) => stop_orders.push((dir, lv)),
                Ord::Close => {}
            }
        }
        // 3. stop entries inside the bar (at most one), then the protective stop along the bar path O-near-far-C
        let up_first = h - o <= o - l;
        if pos.is_none() || cfg.reverse {
            for (dir, lv) in stop_orders {
                let reach = if dir > 0.0 { h >= lv } else { l <= lv };
                if reach && pos.as_ref().map_or(true, |p| p.dir != dir) {
                    enter(&mut pos, &mut out, dir, lv, "stop");
                    break;
                }
            }
        }
        if let Some(p) = &pos {
            if p.stop.is_finite() && (p.i < i || cfg.day0) {
                // part of the bar that is after the entry
                let hit = if p.i < i || p.kind == "open" {
                    if p.dir > 0.0 { l <= p.stop } else { h >= p.stop }
                } else if p.dir > 0.0 {
                    // long stop entry: filled on the up leg
                    if up_first { l <= p.stop } else { c <= p.stop }
                } else if up_first { c >= p.stop } else { h >= p.stop };
                if hit { out.push(done(p, i, p.stop, "stop")); pos = None; }
            }
        }
        // 4. at the close: time exits, then entries on the close
        if let Some(p) = &pos {
            let why = if cfg.hold.map_or(false, |n| i >= p.i + n) { Some("time exit on the close") }
                else if cfg.prof_close && i > p.i && p.dir * (c - p.px) > 0.0 { Some("first profitable close") } else { None };
            if let Some(w) = why { out.push(done(p, i, c, w)); pos = None; }
        }
        for &(dir, od) in &orders {
            if let Ord::Close = od { enter(&mut pos, &mut out, dir, c, "close"); }
        }
    }
    if let Some(p) = pos {
        let mut t = done(&p, to, x.c[to], "open");
        t.open = true;
        out.push(t);
    }
    out
}

/// Parameter reader shared by the LW seminar modules: the value passed, else the schema default.
pub(crate) fn getter<'a>(schema: &'a [ParamDef], p: &'a Params) -> impl Fn(&str) -> f64 + 'a {
    move |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).map_or(f64::NAN, |d| d.def))
}

pub(crate) fn oops(x: &Ctx, i: usize, dir: f64) -> Option<(f64, Ord)> {
    if i < 1 { return None; }
    if dir > 0.0 && x.o[i] < x.l[i - 1] { Some((1.0, Ord::Stop(x.l[i - 1]))) }
    else if dir < 0.0 && x.o[i] > x.h[i - 1] { Some((-1.0, Ord::Stop(x.h[i - 1]))) } else { None }
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    let schema = params(id)?;
    if from > to || to >= x.r.len() { return None; }
    let g = getter(&schema, p);
    let pts = g("stop_usd") / g("pt_usd");
    let mut cfg = Cfg::new(pts);
    let wd = |i: usize| dow(&x.r[i].date);
    let td = tdm(x);
    let t = match id {
        "lwicw_sp_tdw_mon" | "lwicw_sp_tdw_tue" | "lwicw_sp_tdw_wed" | "lwicw_sp_tdw_thu" | "lwicw_sp_tdw_fri" => {
            let want = match &id[13..] { "mon" => 1, "tue" => 2, "wed" => 3, "thu" => 4, _ => 5 };
            cfg.prof_close = true;
            sim(x, from, to, &cfg, |i| if wd(i) == want { vec![(1.0, Ord::Open)] } else { vec![] })
        }
        "lwicw_sp_tdom" => {
            let n = g("tdom") as usize;
            cfg.bail = Some(0);
            sim(x, from, to, &cfg, |i| if td[i] == n { vec![(1.0, Ord::Open)] } else { vec![] })
        }
        "lwicw_oops_tdom_buy" | "lwicw_oops_tdom_sell" => {
            let n = g("tdom") as usize;
            let dir = if id.ends_with("buy") { 1.0 } else { -1.0 };
            cfg.bail = Some(0);
            sim(x, from, to, &cfg, |i| if td[i] == n { oops(x, i, dir).into_iter().collect() } else { vec![] })
        }
        "lwicw_mon_gap_bail" => {
            cfg.bail = Some(0);
            sim(x, from, to, &cfg, |i| if wd(i) == 1 && x.o[i] < x.c[i - 1] { vec![(1.0, Ord::Open)] } else { vec![] })
        }
        "lwicw_sp_holidays" => {
            cfg.bail = Some(g("bail_delay").max(0.0) as usize);
            cfg.reverse = true;
            use Hol::*;
            // (holiday, trading-day offset, dir, order)
            const RULES: &[(Hol, i32, f64, bool)] = &[(NewYear, 3, 1.0, true), (Presidents, 3, 1.0, true), (Presidents, 2, -1.0, false),
                (Easter, 2, 1.0, false), (Easter, -1, -1.0, true), (Memorial, 1, 1.0, false), (July4, 5, 1.0, true), (Labor, -1, 1.0, true),
                (Thanksgiving, 3, 1.0, false), (Christmas, 5, 1.0, true)];
            let y0: i32 = x.r[0].date[..4].parse().ok()?;
            let y1: i32 = x.r[x.r.len() - 1].date[..4].parse().ok()?;
            let mut ord: Vec<Vec<(f64, Ord)>> = vec![vec![]; x.r.len()];
            for y in y0..=y1 + 1 {
                for &(hh, k, dir, at_open) in RULES {
                    if let Some(j) = td_rel(x, &holiday(hh, y), k) {
                        ord[j].push((dir, if at_open { Ord::Open } else { Ord::Close }));
                    }
                }
            }
            sim(x, from, to, &cfg, |i| ord[i].clone())
        }
        _ => return None,
    };
    Some(t)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;
    fn bar(d: &str, o: f64, h: f64, l: f64, c: f64) -> Row {
        Row { ticker: "T".into(), date: d.into(), open: o, high: h, low: l, close: c, volume: None }
    }
    #[test]
    fn holiday_dates() {
        assert_eq!(holiday(Hol::Easter, 1999), "1999-04-02");
        assert_eq!(holiday(Hol::Easter, 2024), "2024-03-29");
        assert_eq!(holiday(Hol::Thanksgiving, 1998), "1998-11-26");
        assert_eq!(holiday(Hol::Presidents, 1999), "1999-02-15");
        assert_eq!(holiday(Hol::Memorial, 1999), "1999-05-31");
        assert_eq!(holiday(Hol::Labor, 1998), "1998-09-07");
        assert_eq!((holiday(Hol::Easter, 2014), holiday(Hol::Easter, 2000), holiday(Hol::Easter, 2025)), ("2014-04-18".into(), "2000-04-21".into(), "2025-04-18".into()));
    }
    #[test]
    fn tdw_first_profitable_close_and_stop() {
        // Mon 1998-08-24 buy 100; Mon close 99 (entry bar never exits on close); Tue close 101 -> exit 101
        let r = vec![bar("1998-08-21", 100., 101., 99., 100.), bar("1998-08-24", 100., 101., 98.5, 99.),
            bar("1998-08-25", 99., 102., 98.8, 101.), bar("1998-08-26", 101., 102., 100., 101.)];
        let x = Ctx::new(&r, &r, &[]);
        let t = run("lwicw_sp_tdw_mon", &x, 1, 3, &Params::new()).unwrap();
        assert_eq!((t[0].legs[0].px, t[0].exit_i, t[0].exit_px), (100.0, 2, 101.0));
        // a 1-point stop ($500 at the work's $500 a point) is hit on Monday's low; the old $250 rate is still an option
        let p = Params::from([("stop_usd".to_string(), 500.0)]);
        let t = run("lwicw_sp_tdw_mon", &x, 1, 3, &p).unwrap();
        assert_eq!((t[0].exit_i, t[0].exit_px), (1, 99.0));
        let p = Params::from([("stop_usd".to_string(), 250.0), ("pt_usd".to_string(), 250.0)]);
        let t = run("lwicw_sp_tdw_mon", &x, 1, 3, &p).unwrap();
        assert_eq!((t[0].exit_i, t[0].exit_px), (1, 99.0));
    }
}
