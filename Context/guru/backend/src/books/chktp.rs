//! Lane K82 (13 Nov 2026; authors/titles-1030/K82.tsv; reader row authors/o2/O2.tsv): Chuck Hughes, "The Key to Profits in Tough
//! Times" (Legacy Publishing 2010, a bound set of his reports; research share 'Trading Books/Chuck Hughes - The Key to Profits in
//! Tough Times/Chuck Hughes - The Key to Profits in Tough Times.pdf', 108 PDF pages, text layer). Book id `chktp`.
//! Pages are PDF pages (the first report's printed page = PDF page - 1).
//! Coded: the Major Trend System (month-end close vs its 20-month EMA, long above / short below, PDF p.5-7) and the 50/100-day EMA
//! system with the Major Trend as the short filter (PDF p.8, p.52-53). Held: the Wealth Building Formula (stockholders' equity
//! growth, fundamentals) and the option strategies.
//! Independent reference: authors/titles-1030/K82w/k82_ref.py (written from the pages, not from this file).
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};
use super::msws::ema;
use super::sctraub::period_ends;
use super::tselhs::Book;

pub const REVIEWED: &[&str] = &["chktp_major_trend", "chktp_ema50_100"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const SRC: &str = "Hughes, The Key to Profits in Tough Times (Legacy Publishing 2010); PDF pages cited.";
const MT_NOTE: &str = "[Declared readings] The '1-Month Price' is the close of the month's last trading day; the 20-month EMA is the usual exponential average (alpha 2/21) of those month-end closes, seeded with the first month-end close of the data and shown from the 20th. The book does not say when the order is filled; the trade is taken at that month-end close. Always in: long while the month-end close is above the EMA, short while it is below (an equal close keeps the position); the first trade of a test is the side at its first month-end. Book figure (S&P 500 Index, '$10,000 ... grew to $29,311 over the past 10 years', 1999-2009, chart only, no trade count, PDF p.6-7): not a #168 check (no count or win rate printed). The stock results (e.g. AAPL +9,338%, 100% winners, PDF p.12-21) cannot come from this rule on stock prices alone and are not compared. Runs on indices, ETFs and stocks, US and India, are a declared application (the book applies it to stocks, ETFs and the S&P 500).";
const EMA_NOTE: &str = "[Declared readings] EMA = exponential average of daily closes (alpha 2/(n+1)), seeded with the first close of the data. Signals are the crosses: the 50-day EMA crossing above the 100-day EMA buys (and covers a short), crossing below sells the long; a short is opened on that sell cross only when the Major Trend System is down at the time (the last month-end close, this month's if today is the month's last trading day, below its 20-month EMA): 'I normally will not short a stock using the 50/100-Day EMA trend following system unless the major price trend is also down' (PDF p.8). A short is held until the next buy cross. Orders are filled at the signal day's close (the book gives no fill). Book figure: 'five sell signals all of which turned out to be losing trades' on the S&P 500 between mid-2003 and January 2008 (chart, PDF p.8); no table. Runs on indices, ETFs and stocks, US and India, are a declared application.";

pub fn catalogue() -> Vec<StrategyDef> {
    let d = |id, name, group, dir, status, rules, notes| StrategyDef { id, name, group, session: 0, dir, status, needs_volume: false, needs_vix: false,
        needs_series: &[], book: "chktp", rules, notes, claim: None };
    vec![
        d("chktp_major_trend", "Hughes Major Trend System: month-end close above / below its 20-month EMA, long or short", "Major Trend System (PDF p.5-7)", "Long/Short", Status::Live,
          &["E|\"My Major Trend System is a simple trend following system that buys when a stock's one month price is above its 20-Month Exponential Moving Average and sells short when a stock's one month price drops below its 20-Month Exponential Moving Average.\" (PDF p.6)",
            "F|\"the 1-Month Price of a stock in relation to its 20-Month Exponential Moving Average (EMA) is an excellent way to identify the major or long term price trend\"; 'Short' signal: \"1-Month Price is Below 20-Month EMA\" (PDF p.5)",
            "X|Stop and reverse on the opposite signal (the system 'takes both long and short trades', PDF p.6)"],
          MT_NOTE),
        d("chktp_ema50_100", "Hughes 50/100-day EMA system: buy the 50/100 cross up, short the cross down only when the Major Trend is down", "50/100-Day EMA System (PDF p.8, p.52-53)", "Long/Short", Status::Live,
          &["E|\"A price up trend exists when a stock's 50-Day Exponential Moving Average (EMA) line is above the 100-Day Exponential Moving Average line and the stock should be bought. A price down trend exists when the 50-Day EMA is below the 100-Day EMA and a stock should be sold.\" (PDF p.52)",
            "F|\"I normally will not short a stock using the 50/100-Day EMA trend following system unless the major price trend is also down. This helps prevent 'whipsaw' trades\" (PDF p.8)",
            "X|Long: the 50-day EMA crossing below the 100-day EMA ('a sell signal', PDF p.53); short: the next cross up"],
          EMA_NOTE),
        d("chktp_wealth_building", "Wealth Building Formula: stocks growing stockholders' equity 10% a year, bought on a 50/100 EMA buy", "Wealth Building Formula (PDF p.45-61)", "Long",
          Status::NeedsData("Held: the stock selection needs each company's stockholders' equity (intrinsic value) growth rate, 10% a year or more (PDF p.45, 50, 61); the vault holds no fundamentals. The price part is the 50/100-day EMA rule (chktp_ema50_100)."),
          &["E|\"Stocks of companies that are growing their stockholder's equity at a 10% annual rate or higher and are in a price uptrend with the 50-Day EMA above the 100-Day EMA\" (PDF p.61)"],
          SRC),
        d("chktp_options", "Option strategies: call purchases on Wealth Building stocks, limited-risk shorts (puts, inverse ETFs, buy-writes, bear spreads)", "Option strategies (PDF p.8, 24-44, 61-108)", "Long/Short",
          Status::NotTestable("Held: no strike, expiry or exit rule is given; the call and put examples (RIMM 70-strike, HPQ 35-strike, PDF p.61-66) are single trades chosen by the author, and the four 'limited risk' short types (PDF p.8) are listed without rules; option prices are not held for these dates."),
          &[], SRC),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    match id {
        "chktp_major_trend" => Some(("chktp_major_trend", "Hughes Major Trend System", "20 months")),
        "chktp_ema50_100" => Some(("chktp_ema50_100", "Hughes 50/100-day EMA system", "50/100 days")),
        "chktp_wealth_building" | "chktp_options" => catalogue().into_iter().find(|d| d.id == id).map(|d| (d.id, d.name, "")),
        _ => None,
    }
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    match id {
        "chktp_major_trend" => Some(vec![p("months", "EMA of month-end closes, months (book 20)", 20., 2., 60., 1.)]),
        "chktp_ema50_100" => Some(vec![p("fast", "Fast EMA, days (book 50)", 50., 2., 200., 1.), p("slow", "Slow EMA, days (book 100)", 100., 3., 400., 1.),
            p("months", "Major Trend filter: EMA of month-end closes, months (book 20)", 20., 2., 60., 1.)]),
        _ => None,
    }
}

/// Major Trend state on every bar: +1 / -1 from the latest month-end close (at or before the bar) vs its `m`-month EMA; 0 before.
fn major_trend(x: &Ctx, m: usize) -> (Vec<f64>, Vec<bool>) {
    let ends = period_ends(x, true);
    let e = ema(&ends.iter().map(|&i| x.c[i]).collect::<Vec<_>>(), m);
    let (mut st, mut is_end) = (vec![0.0; x.c.len()], vec![false; x.c.len()]);
    let (mut k, mut cur) = (0usize, 0.0);
    for i in 0..x.c.len() {
        if k < ends.len() && ends[k] == i {
            is_end[i] = true;
            if e[k].is_finite() { cur = if x.c[i] > e[k] { 1.0 } else if x.c[i] < e[k] { -1.0 } else { cur }; }
            k += 1;
        }
        st[i] = cur;
    }
    (st, is_end)
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.r.len() { return None; }
    let g = |k: &str, d: f64| p.get(k).copied().unwrap_or(d).round().max(1.) as usize;
    let (mt, is_end) = major_trend(x, g("months", 20.));
    let mut pos = Book::new();
    if id == "chktp_major_trend" {
        for t in from..=to {
            if is_end[t] && mt[t] != 0.0 && mt[t] != pos.side() { pos.enter(mt[t], t, x.c[t], "close"); }
        }
    } else {
        let (a, b) = (ema(&x.c, g("fast", 50.)), ema(&x.c, g("slow", 100.)));
        for t in from.max(1)..=to {
            if !(a[t - 1].is_finite() && b[t - 1].is_finite()) { continue; }
            if a[t] > b[t] && a[t - 1] <= b[t - 1] { pos.enter(1.0, t, x.c[t], "close"); }
            else if a[t] < b[t] && a[t - 1] >= b[t - 1] {
                if mt[t] < 0.0 { pos.enter(-1.0, t, x.c[t], "close"); } else if pos.side() > 0.0 { pos.exit(t, x.c[t], "50/100 sell cross"); }
            }
        }
    }
    Some(pos.done(to, x.c[to]))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn major_trend_trades_only_at_month_ends_and_ema_shorts_need_the_filter() {
        let r = super::super::sctraub::tests::bars(3000);
        let x = Ctx::new(&r, &r, &[]);
        let (mt, is_end) = major_trend(&x, 20);
        let t = run("chktp_major_trend", &x, 0, r.len() - 1, &Params::new()).unwrap();
        assert!(t.len() > 3);
        for w in t.windows(2) { assert_eq!(w[0].exit_i, w[1].legs[0].i); assert_eq!(w[0].dir, -w[1].dir); }
        for z in &t { assert!(is_end[z.legs[0].i]); assert_eq!(mt[z.legs[0].i], z.dir); }
        let e = run("chktp_ema50_100", &x, 0, r.len() - 1, &Params::new()).unwrap();
        assert!(e.iter().any(|z| z.dir > 0.));
        for z in e.iter().filter(|z| z.dir < 0.) { assert!(mt[z.legs[0].i] < 0.); }
    }
}

// Instrument admission: Hughes Major Trend and 50/100 EMA systems 2026-11-13 (lane K82).
pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book == "chktp" && reviewed(def.id),
    label: "Chuck Hughes applies both systems to stocks, ETFs and the S&P 500 Index (results as charts and end values only). Runs on indices, ETFs and stocks, US and India, are a declared application.",
    basis: "The Key to Profits in Tough Times (2010) PDF p.5-8, 52-53; independent check authors/titles-1030/K82w/k82_ref.py",
};
