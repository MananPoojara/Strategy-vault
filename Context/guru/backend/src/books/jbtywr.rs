//! Jake Bernstein, "Trade Your Way to Riches Now!" video seminar binder (MBH Commodity Advisors, 1996). Book id `jbtywr`.
//! Sources (one work): research share "USB HDD 2/tRADING KNO-2/Jack Bernstein - Trade Your Way to Riches Now! (Trading
//! Course) (trade-futures.com)/Berstain.pdf" and the seminar's Word figures in Books/Jake Bernstein/*.doc; the
//! "Introduction to Technical Analysis" PDFs are the same binder. Pages = binder section pages (figure numbers given).
//! Reading notes: claude-continuation-20261020/authors/bernstein/agents/tywr/.
use super::jb624::{finish, getp, mark_open, new_trade};
use super::jbhsms::keydate;
use crate::connors::{claim as c, Ctx, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "jbtywr";

macro_rules! def {
    ($id:expr, $name:expr, $group:expr, $dir:expr, $status:expr, $rules:expr, $notes:expr) => {
        StrategyDef { id: $id, name: $name, group: $group, session: 0, dir: $dir, status: $status, needs_volume: false,
            needs_vix: false, needs_series: &[], book: BOOK, rules: $rules, notes: $notes, claim: None }
    };
}

const R_KD: &str = "E|Table header \"ENTRY / EXIT / SL / BUY / CLOSE\": enter on the close of the entry date; a closed market moves the date to the next trading day (table rows, e.g. 12/17/51) (Seasonals Part II, Figs 4-3 to 4-8)";
const R_KDX: &str = "X|Exit on the close of the exit date; \"SL: 5.0%\" = stop loss as a % of the entry price on a closing basis (Figs 4-3, 4-4)";
const N_KD: &str = "Book record: the Dow Jones Industrial Average itself, compared with our DJI series (from 1971). Exit on the first close on or after the exit date; a close below entry x (1 - stop%) before then exits on that close. Other indices, ETFs and stocks (US and India, same dates) are a declared application.";

pub fn catalogue() -> Vec<StrategyDef> {
    use Status::{Live, NeedsData, NotTestable};
    vec![
        StrategyDef { claim: c("44", "77.27%", "25.77 Dow points (2.21%)", "1950-1993", "DJIA, 34 up / 10 down"),
          ..def!("jbtywr_dj_1215", "Dow key date · long Dec 15 to Dec 30, 5% closing stop", "Seasonals Part II · Key Date Seasonals", "Long", Live,
            &["F|Figure 4-3 \"Dow Jones Key Date Seasonal\" (Seasonals Part II p.38): ENTRY 12/15, EXIT 12/30, SL 5.0%, BUY, CLOSE", R_KD, R_KDX], N_KD) },
        StrategyDef { claim: c("49", "79.59%", "", "1945-1993", "DJIA, 39 up / 10 down"),
          ..def!("jbtywr_dj_0406", "Dow key date · long Apr 6 to Apr 16, 5% closing stop", "Seasonals Part II · Key Date Seasonals", "Long", Live,
            &["F|Figure 4-4 \"Dow Jones Key Date Seasonal April 6th-April 16th\" (Seasonals Part II p.39): ENTRY 4/6, EXIT 4/16, SL 5.0%, BUY, CLOSE", R_KD, R_KDX],
            "Close cousin of jbhsms_dj_0407 (Apr 7 to Apr 16, 15% stop, Hot Stock Market Strategies). Book record: the DJIA itself, compared with our DJI series (from 1971). Other indices, ETFs and stocks are a declared application.") },
        StrategyDef { claim: c("30", "70%", "$1,979 (1 crude oil contract)", "", "NYMEX light crude, June and December contracts, 2/7/85-5/2/94, +$59,370"),
          ..def!("jbtywr_crude_outside", "Outside day then close above its high · buy, out on day 60 or a 6% closing stop", "Introduction to Technical Analysis · Outside Day Pattern", "Long", Live,
            &["F|Figure 1-7 \"Outside Day Pattern\" and Figure 1-8 \"Crude Oil Track Record\" (Introduction to Technical Analysis): \"Pattern: L02<L01 H02>H01 C03>H02\" = day 2 makes a lower low and a higher high than day 1 (an outside day) and day 3 closes above day 2's high",
              "E|\"Buy: C03\" = buy on the close of day 3",
              "X|\"Sell: C60  Stop: 6.00 (Percent, Closing)\" = sell on the close of day 60 (day 1 = the first pattern day, so 57 bars after the entry), earlier if a close is 6% below the entry price"],
            "Book test: NYMEX light crude futures (June and December contracts, 1985-1994), with overlapping positions closed at contract expiry. Here: one position at a time, any month, held 57 bars after the entry close; the stop exits on a close below entry x 0.94 [closing-basis stop read as strictly below]. Index, ETF and stock runs (US and India) are a declared application, not the author's test. The Word copy of the figure says the third day \"closes higher than the close of the 2nd day\"; the printed program header (close above the 2nd day's high) is used.") },
        def!("jbtywr_wheat_0901", "December wheat key date · long Sep 1 to Oct 24, 5% stop", "Seasonals Part II · Key Date Seasonals", "Long", NeedsData("December wheat futures contract histories (the seasonal belongs to that market; the vault has no grain futures)"),
            &["F|Figure 4-5 \"Key Date Seasonal in Wheat\": ENTRY 9/1, EXIT 10/24, SL 5%, BUY, CLOSE; 45 trades 1950-94, 75.56%"], ""),
        def!("jbtywr_tbond_0418", "June T-bond key date · short Apr 18 to Apr 26, 1% stop", "Seasonals Part II · Key Date Seasonals", "Short", NeedsData("June Treasury-bond futures contract histories (the vault has no bond futures)"),
            &["F|Figure 4-6 \"Key Date Seasonal in TBond Futures\": ENTRY 4/18, EXIT 4/26, SL 1%, SELL, CLOSE; 16 trades 1978-93, 87.50% (the caption's \"April 10th\" contradicts the table's 4/18)"], ""),
        def!("jbtywr_hogs_1030", "December live hogs key date · long Oct 30 to Nov 27, 6% stop", "Seasonals Part II · Key Date Seasonals", "Long", NeedsData("December live hog futures contract histories (the vault has no livestock futures)"),
            &["F|Figure 4-8 \"Key Date Seasonal Live Hogs\": ENTRY 10/30, EXIT 11/27, SL 6%, BUY, CLOSE; 27 trades 1967-93, 81.48%"], ""),
        def!("jbtywr_lumber", "January lumber key date", "Seasonals Part II · Key Date Seasonals", "Long", NotTestable("Figure 4-1 lists trades (entries about Nov 21, exits about Dec 1) but prints no entry date, exit date or stop header"),
            &["F|Figure 4-1 \"Lumber Key Date Seasonal\" (Seasonals Part II p.36)"], ""),
    ]
}

/// Admission (bernstein-g3, 2026-10-22): reviewed with independent chronological ledgers.
pub const REVIEWED: &[&str] = &["jbtywr_dj_1215", "jbtywr_dj_0406", "jbtywr_crude_outside"];
pub fn reviewed_daily(id: &str) -> bool {
    REVIEWED.contains(&id)
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    Some(match id {
        "jbtywr_dj_1215" => ("jbtywr_kd", "Key-date seasonals (Trade Your Way to Riches Now!)", "Dow Dec 15-Dec 30"),
        "jbtywr_dj_0406" => ("jbtywr_kd", "Key-date seasonals (Trade Your Way to Riches Now!)", "Dow Apr 6-Apr 16"),
        "jbtywr_wheat_0901" | "jbtywr_tbond_0418" | "jbtywr_hogs_1030" | "jbtywr_lumber" => ("jbtywr_kd", "Key-date seasonals (Trade Your Way to Riches Now!)", "Futures (needs data)"),
        "jbtywr_crude_outside" => ("jbtywr_outside", "Outside day pattern (crude oil track record)", "Day-60 exit, 6% closing stop"),
        _ => return None,
    })
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    Some(match id {
        "jbtywr_dj_1215" => vec![p("entry_m", "Entry month", 12., 1., 12., 1.), p("entry_d", "Entry day", 15., 1., 31., 1.), p("exit_m", "Exit month", 12., 1., 12., 1.), p("exit_d", "Exit day", 30., 1., 31., 1.), p("stop_pct", "Closing stop, % below the entry close (100 = none)", 5., 0.5, 100., 0.5)],
        "jbtywr_dj_0406" => vec![p("entry_m", "Entry month", 4., 1., 12., 1.), p("entry_d", "Entry day", 6., 1., 31., 1.), p("exit_m", "Exit month", 4., 1., 12., 1.), p("exit_d", "Exit day", 16., 1., 31., 1.), p("stop_pct", "Closing stop, % below the entry close (100 = none)", 5., 0.5, 100., 0.5)],
        "jbtywr_crude_outside" => vec![p("exit_day", "Exit on the close of pattern day (day 1 = first pattern day)", 60., 4., 120., 1.), p("stop_pct", "Closing stop, % below the entry close", 6., 0.5, 50., 0.5)],
        _ => return None,
    })
}

/// Outside day (day 2 low < day 1 low and high > day 1 high), day 3 close > day 2 high: buy day 3's close; sell on the
/// close of day `exit_day` or on an earlier close below entry x (1 - stop). One position; no entry on an exit bar.
pub fn outside_day(x: &Ctx, from: usize, to: usize, exit_day: usize, stop: f64) -> Vec<Trade> {
    let hold = exit_day.max(4) - 3;
    let (mut out, mut cur): (Vec<Trade>, Option<Trade>) = (Vec::new(), None);
    for i in from.max(2)..=to {
        if let Some(t) = cur.as_ref() {
            if i >= t.legs[0].i + hold {
                out.push(finish(cur.take().unwrap(), i, x.c[i], i, "Day-60 close"));
            } else if x.c[i] < t.legs[0].px * (1.0 - stop) {
                out.push(finish(cur.take().unwrap(), i, x.c[i], i, "Closing stop"));
            }
            continue;
        }
        if i < to && x.l[i - 1] < x.l[i - 2] && x.h[i - 1] > x.h[i - 2] && x.c[i] > x.h[i - 1] {
            cur = Some(new_trade(1.0, i, x.c[i], "close", i));
        }
    }
    mark_open(&mut out, cur, x, to);
    out
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !id.starts_with("jbtywr_") || to >= x.c.len() || from > to {
        return None;
    }
    if params(id).is_none() {
        return if catalogue().iter().any(|s| s.id == id) { Some(vec![]) } else { None };
    }
    let g = |k: &str| getp(id, p, k);
    if id == "jbtywr_crude_outside" {
        return Some(outside_day(x, from, to, g("exit_day").round() as usize, g("stop_pct") / 100.0));
    }
    Some(keydate(x, from, to, g("entry_m"), g("entry_d"), g("exit_m"), g("exit_d"), g("stop_pct")))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;
    fn bar(d: &str, o: f64, h: f64, l: f64, c: f64) -> Row {
        Row { ticker: "T".into(), date: d.into(), open: o, high: h, low: l, close: c, volume: None }
    }
    #[test]
    fn outside_day_entry_and_exits() {
        let mut r = vec![bar("2000-01-03", 10., 11., 9., 10.), bar("2000-01-04", 10., 12., 8., 11.), bar("2000-01-05", 11., 13., 11., 12.5)];
        for k in 0..60 { r.push(bar(&format!("2000-02-{:02}", k % 28 + 1), 12.5, 12.6, 12.4, 12.5)); }
        let x = Ctx::new(&r, &r, &[]);
        let t = outside_day(&x, 0, r.len() - 1, 60, 0.06);
        assert_eq!((t[0].legs[0].i, t[0].exit_i), (2, 59));
        r[10].close = 11.7; // below 12.5 x 0.94 = 11.75
        let x = Ctx::new(&r, &r, &[]);
        assert_eq!(outside_day(&x, 0, r.len() - 1, 60, 0.06)[0].exit_i, 10);
    }
}
