//! Larry Williams, "Forecast 2010" (annual forecast report, 55 pp, text). Book id `lwfc10`. Larry Williams block, 23 Oct 2026.
//! Source copy: Books/Larry Williams/Forecast_2010_by_Larry_Williams.pdf (research drive "Trading Books/Larry Williams -
//! Forecast for 2010/"). Reading: claude-continuation-20261020/authors/lwilliams/agents/lwfc10/. Page refs = printed pages.
//! The report is mostly a road map; its "Mechanical S&P Trades for 2010" section (p.11-16) holds three complete rules.
//! Simulator: books::lwicw::sim (open entries, dollar stop armed on the entry bar, bailout = first profitable opening).
use crate::books::lwicw::{sim, Cfg, Ord};
use crate::connors::{claim, Ctx, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "lwfc10";

/// Rules admitted after the 23 Oct 2026 reading and independent ledgers (authors/lwilliams/admission/SOURCE-CONTRACT.md).
pub const REVIEWED: &[&str] = &["lwfc10_tdlm", "lwfc10_tdoy", "lwfc10_midterm"];
/// $3,500 S&P-futures stop kept as 14 S&P points ($250 a point, the contract of the 2009 report): actual SPX only.
pub const SPX_POINTS: &[&str] = &["lwfc10_tdlm", "lwfc10_tdoy"];
/// US mid-term election years: US instruments only.
pub const US_ONLY: &[&str] = &["lwfc10_midterm"];

const NOTE: &str = "Book: S&P 500 futures, test 1983-2009 (\"Mechanical S&P Trades for 2010\", p.11). The $3,500 stop is one S&P futures contract; at $250 a point (the contract size in 2009; the report prints no point value) it is 14 S&P points.";

pub fn catalogue() -> Vec<StrategyDef> {
    let sd = |id, name, dir, rules, notes, c| StrategyDef { id, name, group: "Mechanical S&P trades for 2010", session: 0, dir, status: Status::Live,
        needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK, rules, notes, claim: c };
    vec![
        sd("lwfc10_tdlm", "Presidential clout: buy after the best trading-days-left-in-month set-ups", "Long",
            &["F|\"The best set up days to buy the next day have been TDLM 2, 10, 12, 19, 22, and 23\" (p.11-12); the Genesis code tests \"Next Bar Trading Days Left In Month\" = N, so the ENTRY day has N trading days left [Read: code wins; the last trading day of a month counts 1, as the list's 23 requires]",
              "F|\"Today's 200 day moving average of closing prices is higher than it was 3 days ago\" (p.12)",
              "F|\"The day that just closed is not an Outside Day with a higher close\" (p.12); outside day = higher high and lower low than the day before",
              "F|Genesis code: \"And Next Bar Day Is ('Friday') = False\" (p.12): no Friday entries [code only]",
              "E|Read: buy on the next day's opening (the order type is not printed)",
              "X|\"The exit is a $3,500 stop or an exit on the First Profitable Open (FPO)\" (p.12): 14 S&P points from the entry, armed on the entry day"],
            NOTE, claim("484", "84.9%", "$537 ($259,783)", "2.58 bars", "S&P 1983-2009")),
        sd("lwfc10_tdoy", "Presidential clout: best trading days of the year", "Long",
            &["F|\"IF MovingAvg (Close , 200) > MovingAvg (Close , 200).3 And Trading DayOfYear = 20 Or = 63 Or = 99 Or = 212 Or = 219 Or = 232 Or = 235 Or = 254\" (p.13) [Read: the trend filter applies to every listed day, as the prose says; trading day 1 = the first session of the year]",
              "E|\"Buy Tomorrow on the Opening\" (p.13)",
              "X|\"using the same stop and FPO exit as discussed above\" (p.13): $3,500 = 14 S&P points, first profitable opening"],
            NOTE, claim("120", "99%", "", "", "S&P 1982-2009")),
        sd("lwfc10_midterm", "Mid-term election trade: buy the next-to-last trading day of October", "Long",
            &["F|Mid-term election years (1982, 1986 ... 2006 in the p.15 table): years two after a presidential election; p.years = 1 runs every year (the p.16 table)",
              "E|\"it has been best to buy on the opening of the next to last trading day in October\" (p.15)",
              "X|\"and wait at least one day for the FPO\" (p.15): first profitable opening from the second opening after entry; no stop is stated"],
            "Book: S&P 500 futures; p.15 mid-term years 1982-2006: 7 trades, all winners, $14,198; p.16 every year 1982-2009: 28 trades, 25 winners, $20,140. No stop in the book, so none is added.",
            claim("7", "100%", "$2,028 ($14,198)", "", "S&P mid-term years 1982-2006")),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    let f = |v| Some(("lwfc10", "Mechanical S&P trades for 2010", v));
    match id { "lwfc10_tdlm" => f("TDLM set-ups"), "lwfc10_tdoy" => f("Trading day of year"), "lwfc10_midterm" => f("Mid-term election"), _ => None }
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    let stop = || p("stop_usd", "Dollar stop (book $3,500)", 3500.0, 0.0, 10000.0, 250.0);
    let pt = || p("pt_usd", "Dollars per S&P point ($250 in 2009)", 250.0, 50.0, 500.0, 50.0);
    match id {
        "lwfc10_tdlm" => Some(vec![stop(), pt(), p("no_fri", "Skip Friday entries (code listing: 1)", 1.0, 0.0, 1.0, 1.0)]),
        "lwfc10_tdoy" => Some(vec![stop(), pt()]),
        "lwfc10_midterm" => Some(vec![p("years", "0 = mid-term election years only (book); 1 = every year (p.16 table)", 0.0, 0.0, 1.0, 1.0)]),
        _ => None,
    }
}

fn sma(c: &[f64], n: usize, i: usize) -> f64 { if i + 1 < n { f64::NAN } else { c[i + 1 - n..=i].iter().sum::<f64>() / n as f64 } }

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    let schema = params(id)?;
    if from > to || to >= x.r.len() { return None; }
    let g = crate::books::lwicw::getter(&schema, p);
    let (h, l, c) = (&x.h, &x.l, &x.c);
    let n = x.r.len();
    let ym = |i: usize| &x.r[i].date[..7];
    // trading days left in the month, the last trading day = 1 (needs the next bar to know the month ended)
    let mut left = vec![0usize; n];
    for i in (0..n).rev() { left[i] = if i + 1 < n && ym(i + 1) == ym(i) { left[i + 1] + 1 } else { 1 }; }
    let mut tdoy = vec![0usize; n];
    for i in 0..n { tdoy[i] = if i > 0 && x.r[i - 1].date[..4] == x.r[i].date[..4] { tdoy[i - 1] + 1 } else { 1 }; }
    let up200 = |t: usize| t >= 202 && sma(c, 200, t) > sma(c, 200, t - 3);
    let wd = |i: usize| crate::books::lwlts::dow(&x.r[i].date);
    Some(match id {
        "lwfc10_tdlm" | "lwfc10_tdoy" => {
            let mut cfg = Cfg::new(g("stop_usd") / g("pt_usd"));
            cfg.bail = Some(0);
            let no_fri = g("no_fri") == 1.0;
            sim(x, from, to, &cfg, |i| {
                if i < 1 { return vec![]; }
                let t = i - 1;
                let ok = if id == "lwfc10_tdlm" {
                    // the last bar of the data cannot know its month has ended; the next bar's month decides `left`
                    i + 1 < n && [2, 10, 12, 19, 22, 23].contains(&left[i]) && up200(t) && !(t >= 1 && h[t] > h[t - 1] && l[t] < l[t - 1] && c[t] > c[t - 1])
                        && !(no_fri && wd(i) == 5)
                } else { [20, 63, 99, 212, 219, 232, 235, 254].contains(&tdoy[t]) && up200(t) };
                if ok { vec![(1.0, Ord::Open)] } else { vec![] }
            })
        }
        "lwfc10_midterm" => {
            let mut cfg = Cfg::new(f64::NAN);
            cfg.bail = Some(1);
            let every = g("years") == 1.0;
            sim(x, from, to, &cfg, |i| {
                let y: i32 = x.r[i].date[..4].parse().unwrap_or(0);
                let ok = &x.r[i].date[5..7] == "10" && left[i] == 2 && i + 1 < n && (every || y.rem_euclid(4) == 2);
                if ok { vec![(1.0, Ord::Open)] } else { vec![] }
            })
        }
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;
    #[test]
    fn midterm_buys_the_next_to_last_october_open_and_waits_a_day() {
        let mk = |d: &str, o: f64, c: f64| Row { ticker: "T".into(), date: d.into(), open: o, high: o.max(c) + 1., low: o.min(c) - 1., close: c, volume: None };
        let r = vec![mk("2006-10-27", 100., 100.), mk("2006-10-30", 100., 99.), mk("2006-10-31", 99.5, 101.), mk("2006-11-01", 101., 102.), mk("2006-11-02", 102., 102.)];
        let x = Ctx::new(&r, &r, &[]);
        let t = run("lwfc10_midterm", &x, 0, 4, &Params::new()).unwrap();
        // entry 10-30 open 100; 10-31 opens 99.5 (not profitable, and too early); 11-01 opens 101 > 100: exit
        assert_eq!(t.len(), 1);
        assert_eq!((t[0].legs[0].i, t[0].exit_i, t[0].exit_px), (1, 3, 101.));
    }
}
