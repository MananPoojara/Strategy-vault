//! Jake Bernstein, "High Probability Patterns and Rule Based Trading" (seminar slides, Traders' Library forum,
//! Chicago, 24 April 2010). Book id `jbhpp`. Source: Books/Jake Bernstein/Jake Bernstein - High Probability Patterns
//! and Rule Based Trading.pdf (113 slides; page = PDF page). Reading notes: claude-continuation-20261020/authors/bernstein/agents/hpp/.
//! Its four timing triggers (MAC + Williams A/D 57, 28-bar momentum divergence, 8-bar open/close, MOM/MA 28) are coded
//! from other Bernstein books (jbudt_mac_ad57, jb624_pmf, jb30d_oc8, jbsmsw_mom_ma); the only new complete rule is the
//! Dow key-date seasonal shown as the worked example of a seasonaltrader.com search.
use super::jb624::getp;
use super::jbhsms::keydate;
use crate::connors::{claim as c, Ctx, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "jbhpp";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef { id: "jbhpp_dj_1026", name: "Dow key date · long Oct 26 to Jan 10, 9% stop", group: "Seasonals · Key Date Trades", session: 0, dir: "Long",
        status: Status::Live, needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK,
        rules: &["F|Slide p.101: \"DJ Industrial Avg $DJIA L Entry 10/26 Exit 1/10 Stop % 9\"; the trade list beside it enters 10/26 (next trading day when closed) and exits 1/10 of the next year",
            "E|Key-date seasonals are entered on the close of the entry date (the slide's table: Price In = that day's close)",
            "X|Exit on the close of the exit date, or earlier on a close 9% below the entry price (\"Stop %\" column)"],
        notes: "Book record (p.101): 93.0% winners, P/L ratio 41.2, average profit 186.66 Dow points, average loss -60.00, entries from 1950. Exit on the first close on or after Jan 10; a close below entry x 0.91 before then exits on that close. Compared on our DJI series (from 1971). Other indices, ETFs and stocks (US and India, same dates) are a declared application.",
        claim: c("", "93.0%", "186.66 Dow points", "1950-2008 (entry years)", "DJIA; P/L ratio 41.2, average loss -60.00") }]
}

pub fn reviewed_daily(id: &str) -> bool {
    id == "jbhpp_dj_1026"
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    (id == "jbhpp_dj_1026").then_some(("jbhpp_kd", "Dow key-date seasonal (High Probability Patterns seminar)", "Oct 26-Jan 10, 9% stop"))
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    (id == "jbhpp_dj_1026").then(|| vec![p("entry_m", "Entry month", 10., 1., 12., 1.), p("entry_d", "Entry day", 26., 1., 31., 1.), p("exit_m", "Exit month", 1., 1., 12., 1.),
        p("exit_d", "Exit day", 10., 1., 31., 1.), p("stop_pct", "Closing stop, % below the entry close (100 = none)", 9., 0.5, 100., 0.5)])
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if id != "jbhpp_dj_1026" || to >= x.c.len() || from > to {
        return None;
    }
    let g = |k: &str| getp(id, p, k);
    Some(keydate(x, from, to, g("entry_m"), g("entry_d"), g("exit_m"), g("exit_d"), g("stop_pct")))
}
