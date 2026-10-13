//! Jake Bernstein, "Hot Stock Market Strategies" (2005; data to 2004). Book id `jbhsms`.
//! Source: a 249-page scan, read as page images (printed page = PDF page - 22). Transcription and checks: library/jbhsms/.
//! Testable on our data: the S&P and Dow Jones key-date seasonal trades of ch.7 (each printed with its full trade history).
use super::jb624::{finish, getp, mark_open, new_trade};
use super::jbcdt2::{days, days_ymd};
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "jbhsms";

macro_rules! def {
    ($id:expr, $name:expr, $group:expr, $dir:expr, $status:expr, $rules:expr, $notes:expr) => {
        StrategyDef { id: $id, name: $name, group: $group, session: 0, dir: $dir, status: $status, needs_volume: false,
            needs_vix: false, needs_series: &[], book: BOOK, rules: $rules, notes: $notes, claim: None }
    };
}

const R_ENTRY: &str = "E|\"these trades are always entered on the close of trading and exited on the close of trading on the dates shown\"; \"If a stock is closed on the recommended entry or exit days, then you execute the seasonal trade on the close of trading, the next business day\" (p.107)";
const R_STOP: &str = "X|\"All stop losses for the seasonal stock trades are 'close only.' This means that if you are short, the market must close above the calculated stop loss ... The same stop-loss rule applies for a long position\"; \"The stop loss is a percentage of the price on day of entry\" (p.107); \"Stop %\" = \"the stop loss in percent of the closing price on the date of entry\" (p.108)";
const N_SP: &str = "Book record: S&P 500 futures (\"the same patterns and dates apply to the SPY\", p.108); run on the index (instrument swap). Exit on the first close on or after the exit date; a close below entry x (1 - stop%) before then exits on that close. Stop% 100 = no stop.";
const N_DJ: &str = "Book record: Dow Jones Industrial Average (cash index), entries 1970-2004: compared trade by trade with our DJI series. Exit on the first close on or after the exit date; a close below entry x (1 - stop%) before then exits on that close. Stop% 100 = no stop.";

/// (id, entry m, d, exit m, d, stop %)
const KD: [(&str, f64, f64, f64, f64, f64); 15] = [
    ("jbhsms_sp_0113", 1., 13., 1., 18., 2.0),
    ("jbhsms_sp_0226", 2., 26., 3., 5., 3.5),
    ("jbhsms_sp_0301", 3., 1., 3., 6., 1.5),
    ("jbhsms_sp_0414", 4., 14., 4., 22., 3.0),
    ("jbhsms_sp_0601", 6., 1., 6., 6., 2.0),
    ("jbhsms_sp_0710", 7., 10., 7., 15., 1.5),
    ("jbhsms_sp_0815", 8., 15., 8., 25., 3.5),
    ("jbhsms_sp_1220", 12., 20., 1., 7., 3.0),
    ("jbhsms_dj_0405", 4., 5., 4., 22., 100.0),
    ("jbhsms_dj_1117", 11., 17., 1., 4., 9.0),
    ("jbhsms_dj_1203", 12., 3., 1., 6., 9.0),
    ("jbhsms_dj_0218", 2., 18., 5., 4., 15.0),
    ("jbhsms_dj_0313", 3., 13., 3., 18., 3.0),
    ("jbhsms_dj_0407", 4., 7., 4., 16., 15.0),
    ("jbhsms_dj_0528", 5., 28., 6., 6., 100.0),
];

macro_rules! kd {
    ($id:expr, $name:expr, $fig:expr, $n:expr, $claim:expr) => {
        StrategyDef { claim: $claim, ..def!($id, $name, "Ch.7 · Short-Term Key Date Seasonal Trades", "Long", Status::Live, &[$fig, R_ENTRY, R_STOP], $n) }
    };
}
use crate::connors::claim as c;

pub fn catalogue() -> Vec<StrategyDef> {
    use Status::{NeedsData, NotTestable};
    vec![
        kd!("jbhsms_sp_0113", "S&P key date · long Jan 13 to Jan 18, 2% stop", "F|Figure 7.2 (p.109): \"Seasonal Trade in March S&P Jan. 13 to Jan. 18\" Long, Enter 1/13, Exit 1/18, Stop %: 2.00", N_SP, c("22", "90.91%", "8.24 S&P points", "1983-2004", "March S&P futures; avg loss -17.25")),
        kd!("jbhsms_sp_0226", "S&P key date · long Feb 26 to Mar 5, 3.5% stop", "F|Figure 7.3 (p.111): Long, Enter 2/26, Exit 3/5, Stop %: 3.50 (the text's \"April 6\" exit, p.110, contradicts the table's 3/5 and every listed exit date)", N_SP, c("22", "81.82%", "11.28 S&P points", "1983-2004", "March S&P futures")),
        kd!("jbhsms_sp_0301", "S&P key date · long Mar 1 to Mar 6, 1.5% stop", "F|Figure 7.4 (p.112): Long, Enter 3/1, Exit 3/6, Stop %: 1.50", N_SP, c("22", "77.27%", "9.37 S&P points", "1983-2004", "March S&P futures")),
        kd!("jbhsms_sp_0414", "S&P key date · long Apr 14 to Apr 22, 3% stop", "F|Figure 7.5 (p.113): Long, Enter 4/14, Exit 4/22, Stop %: 3.00", N_SP, c("23", "78.26%", "14.51 S&P points", "1982-2004", "June S&P futures")),
        kd!("jbhsms_sp_0601", "S&P key date · long Jun 1 to Jun 6, 2% stop", "F|Figure 7.6 (p.115): Long, Enter 6/1, Exit 6/6, Stop %: 2.00", N_SP, c("23", "78.26%", "8.80 S&P points", "1982-2004", "June S&P futures")),
        kd!("jbhsms_sp_0710", "S&P key date · long Jul 10 to Jul 15, 1.5% stop", "F|Figure 7.7 (p.116): Long, Enter 7/10, Exit 7/15, Stop %: 1.50", N_SP, c("22", "77.27%", "8.31 S&P points", "1982-2003", "September S&P futures")),
        kd!("jbhsms_sp_0815", "S&P key date · long Aug 15 to Aug 25, 3.5% stop", "F|Figure 7.8 (p.117): Long, Enter 8/15, Exit 8/25, Stop %: 3.50", N_SP, c("22", "81.82%", "9.25 S&P points", "1982-2003", "September S&P futures")),
        kd!("jbhsms_sp_1220", "S&P key date · long Dec 20 to Jan 7, 3% stop", "F|Figure 7.10 (p.119): Long, Enter 12/20, Exit 1/7, Stop %: 3.00", N_SP, c("22", "86.36%", "16.44 S&P points", "1983-2004 (entry years)", "March S&P futures")),
        kd!("jbhsms_dj_0405", "Dow key date · long Apr 5 to Apr 22 (no stop shown)", "F|Figure 7.1 (p.105-106): \"Seasonal Pattern in the Dow Jones Industrial Average from April 5 to April 25, 1970 to 2004\"; the table enters 4/5 and exits 4/22 (\"higher on or about 4/22 than it was on or about 4/5\", p.104); no stop column", "Fig. 7.1 prints no stop, so none is used (stop% 100). The title's \"April 25\" is the window of the chart; the trades exit on 4/22. Dow Jones Industrial Average, compared trade by trade with our DJI series.", c("35", "77.14%", "106.50 Dow points", "1970-2004", "avg profit 3.10%, avg loss -2.01%")),
        kd!("jbhsms_dj_1117", "Dow key date · long Nov 17 to Jan 4, 9% stop", "F|Figure 7.19 (p.129): \"Long DJ Industrial Average, Nov. 17 to Jan. 4\" Enter 11/17, Exit 1/4, Stop %: 9", N_DJ, c("34", "94.12%", "128.30 Dow points", "1970-2003 (entry years)", "")),
        kd!("jbhsms_dj_1203", "Dow key date · long Dec 3 to Jan 6, 9% stop", "F|Figure 7.20 (p.130): \"Long DJ Industrial Average, Dec. 3 to Jan. 6\" Enter 12/3, Exit 1/6, Stop %: 9", N_DJ, c("34", "85.29%", "120.22 Dow points", "1970-2003 (entry years)", "")),
        kd!("jbhsms_dj_0218", "Dow key date · long Feb 18 to May 4, 15% stop", "F|Figure 7.21 (p.131): \"Long DJ Industrial Average, Feb. 18 to May 4\" Enter 2/18, Exit 5/4, Stop %: 15", N_DJ, c("35", "82.86%", "175.34 Dow points", "1970-2004", "")),
        kd!("jbhsms_dj_0313", "Dow key date · long Mar 13 to Mar 18, 3% stop", "F|Figure 7.22 (p.132): \"Long DJ Industrial Average, Mar. 13 to Mar. 18\" Enter 3/13, Exit 3/18, Stop %: 3", N_DJ, c("35", "77.14%", "78.73 Dow points", "1970-2004", "")),
        kd!("jbhsms_dj_0407", "Dow key date · long Apr 7 to Apr 16, 15% stop", "F|Figure 7.23 (p.133): \"Long DJ Industrial Average, Apr. 7 to Apr. 16\" Enter 4/7, Exit 4/16, Stop %: 15", N_DJ, c("35", "77.14%", "69.14 Dow points", "1970-2004", "")),
        kd!("jbhsms_dj_0528", "Dow key date · long May 28 to Jun 6 (stop 100% = none)", "F|Figure 7.24 (p.134): \"Long DJ Industrial Average, May 28 to Jun. 6\" Enter 5/28, Exit 6/6, Stop %: 100", N_DJ, c("35", "82.86%", "60.56 Dow points", "1970-2004", "")),
        def!("jbhsms_sp_1027", "S&P key date · long Oct 27 to Nov 1, 1.5% stop", "Ch.7 · Short-Term Key Date Seasonal Trades", "Long", NotTestable("duplicate of jb30d_sp_seasonal (same dates, same 1.5% close-only stop); Figure 7.9 (p.118): 22 trades 1982-2003, 95.45%, avg 15.33 points"),
            &["F|Figure 7.9 (p.118): Long, Enter 10/27, Exit 11/1, Stop %: 1.50"], ""),
        def!("jbhsms_stocks", "Key-date seasonals in individual stocks (AA, KO, GE, MSFT, NEM, PG, SUNW, TXN)", "Ch.7 · Short-Term Key Date Seasonal Trades", "Long", NeedsData("daily prices of the US stocks AA, KO, GE, MSFT, NEM, PG, SUNW and TXN (1970-2004); the vault's stock store holds NSE baskets only"),
            &["F|Figures 7.11-7.18 (p.121-128): AA 1/22-2/18 stop 15%; KO 3/31-6/6 stop 100; GE 5/25-6/6 stop 6; MSFT 8/31-2/1 stop 100; NEM 9/26-10/27 stop 100; PG 10/26-12/27 stop 100; SUNW 11/2-1/18 stop 100; TXN 12/15-4/19 stop 100", R_ENTRY],
            "Same key-date engine as the index trades; it would run unchanged on these tickers."),
        def!("jbhsms_gap", "Gap trade in stocks (gap size, penetration, % stop, FPO exit)", "Ch.6 · Gap Your Way to Profits", "Long/Short", NeedsData("daily IBM and EBAY prices (the only tests with stated inputs, Tables 6.1-6.3); the inputs are cents per share for those two stocks"),
            &["E|\"The gap method enters a LONG position if a stock has opened below the low of the previous day by more than a given amount and then enters back into the range of the previous day by penetrating back up through the low of the previous day by a given amount\" (p.94)",
              "X|\"The trade exists using a stop loss that is a percentage of the entry price or it takes profit on the first profitable opening\" (p.94)",
              "F|IBM 2000-2004 (Table 6.1, p.91): gap 20 cents above high / 10 cents below low; penetration 35 cents into the high / 50 cents into the low; FPO; 3.9% stop -> 119 trades, 84.0%, $442 (500 shares)",
              "F|IBM (Table 6.2, p.93-94): gap up 30c / down 90c, penetration 35c / 80c, stop 4.75% -> 56 trades, 92.9%, $741; EBAY 11/1998-7/2004 (Table 6.3, p.94-95): gap 48c / 48c, penetration 4c / 98c, stop 3.55% -> 57 trades, 87.7%, $476"],
            "The simpler day-trade version (exit on the close) is the vault's jbcdt_gap_go / jbudt_gap_* / ss_study_oops; on an index the per-share cent inputs have no meaning, so it is not run."),
        def!("jbhsms_fri_mon", "Friday-Monday pattern (stocks / SPY / QQQ / DIA)", "Ch.8 · Day-of-Week Patterns", "Long/Short", NotTestable("the penetration \"given amount\" and the stop are not given (\"the triggers and stops for each of these stocks are different\", p.151); the S&P inputs of other books are jbcdt2_dbo_mon / jbeft_dow_fm97 / jb624_fri_mon"),
            &["F|\"examine the relationship between the opening price and the closing price in S&P futures on Friday (or the last trading day of the week if the market is closed on Friday)\" (p.142)",
              "E|\"place an order to BUY at a given amount ABOVE the HIGH of Friday\" (p.144); sell short below the Friday low \"by a given amount\" (p.144-145)",
              "X|\"The smart stop for this trading method is a dollar-risk stop or a percentage stop of the entry price\"; \"close out the Friday-Monday trade on the first profitable opening (FPO)\" (p.147-148)"], ""),
        def!("jbhsms_mac", "High-low moving average channel (MAC 10/8)", "Ch.9 · Surfing the Stock Channel", "Long/Short", NotTestable("duplicate: the two-bar MAC trend change is jb624_mac2_sar and the buy-at-support use is jb624_mac_sr / jbsmsw_mac_pullback; this chapter adds no exit rule"),
            &["E|\"Use a 10.MA of highs and an 8.MA of lows. Two consecutive price bars entirely above the high of the MAC suggests a change in trend from down to up\" (p.165)"], ""),
        def!("jbhsms_ma200", "200-day moving average results for SPY", "Ch.9 · Surfing the Stock Channel", "Long/Short", NotTestable("Figure 9.7 (p.163) prints results (41 trades, 26.8%, $2,896 on 1,000 shares) but not the entry/exit rule"),
            &["F|\"The 200-Day MA Results for SPY\" (Figure 9.7, p.163)"], ""),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    if let Some(k) = KD.iter().find(|k| k.0 == id) {
        let sp = id.starts_with("jbhsms_sp_");
        let lbl: &'static str = match id {
            "jbhsms_sp_0113" => "S&P Jan 13-Jan 18", "jbhsms_sp_0226" => "S&P Feb 26-Mar 5", "jbhsms_sp_0301" => "S&P Mar 1-Mar 6",
            "jbhsms_sp_0414" => "S&P Apr 14-Apr 22", "jbhsms_sp_0601" => "S&P Jun 1-Jun 6", "jbhsms_sp_0710" => "S&P Jul 10-Jul 15",
            "jbhsms_sp_0815" => "S&P Aug 15-Aug 25", "jbhsms_sp_1220" => "S&P Dec 20-Jan 7", "jbhsms_dj_0405" => "Dow Apr 5-Apr 22",
            "jbhsms_dj_1117" => "Dow Nov 17-Jan 4", "jbhsms_dj_1203" => "Dow Dec 3-Jan 6", "jbhsms_dj_0218" => "Dow Feb 18-May 4",
            "jbhsms_dj_0313" => "Dow Mar 13-Mar 18", "jbhsms_dj_0407" => "Dow Apr 7-Apr 16", _ => "Dow May 28-Jun 6",
        };
        let _ = k;
        return Some(if sp { ("jbhsms_kd_sp", "S&P key-date seasonals (Hot Stock Market Strategies)", lbl) } else { ("jbhsms_kd_dj", "Dow key-date seasonals (Hot Stock Market Strategies)", lbl) });
    }
    Some(match id {
        "jbhsms_sp_1027" => ("jbhsms_kd_sp", "S&P key-date seasonals (Hot Stock Market Strategies)", "S&P Oct 27-Nov 1 · see jb30d_sp_seasonal"),
        "jbhsms_stocks" => ("jbhsms_kd_stocks", "Stock key-date seasonals", "Needs US stock data"),
        "jbhsms_gap" | "jbhsms_fri_mon" | "jbhsms_mac" | "jbhsms_ma200" => ("jbhsms_other", "Hot Stock Market Strategies · other methods", "Not run"),
        _ => return None,
    })
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    let k = KD.iter().find(|k| k.0 == id)?;
    Some(vec![p("entry_m", "Entry month", k.1, 1., 12., 1.), p("entry_d", "Entry day", k.2, 1., 31., 1.), p("exit_m", "Exit month", k.3, 1., 12., 1.),
        p("exit_d", "Exit day", k.4, 1., 31., 1.), p("stop_pct", "Close-only stop, % below the entry close (100 = none)", k.5, 0.5, 100., 0.5)])
}

/// Key-date trade: buy the close of the first bar on or after the entry date; sell the close of the first bar on or
/// after the exit date (next year when the exit month/day precedes the entry's); close-only stop below entry x (1 - s%).
pub fn keydate(x: &Ctx, from: usize, to: usize, em: f64, ed: f64, xm: f64, xd: f64, stop_pct: f64) -> Vec<Trade> {
    let (em, ed, xm, xd) = (em.round() as i64, ed.round() as i64, xm.round() as i64, xd.round() as i64);
    let wraps = (xm, xd) < (em, ed);
    let stop = if stop_pct >= 100.0 { f64::NAN } else { stop_pct / 100.0 };
    let (mut out, mut cur): (Vec<Trade>, Option<Trade>) = (Vec::new(), None);
    let mut exit_k = i64::MAX;
    for i in from.max(1)..=to {
        let di = days(&x.r[i].date);
        if let Some(t) = cur.as_ref() {
            let e = t.legs[0].px;
            if di >= exit_k {
                out.push(finish(cur.take().unwrap(), i, x.c[i], i, "Exit date (close)"));
            } else if x.c[i] < e * (1.0 - stop) {
                out.push(finish(cur.take().unwrap(), i, x.c[i], i, "Close-only stop"));
            }
            continue;
        }
        let y: i64 = x.r[i].date[0..4].parse().unwrap_or(1970);
        let ent = days_ymd(y, em, ed);
        let ext = days_ymd(if wraps { y + 1 } else { y }, xm, xd);
        if i < to && di >= ent && days(&x.r[i - 1].date) < ent && di < ext {
            exit_k = ext;
            cur = Some(new_trade(1.0, i, x.c[i], "close", i));
        }
    }
    mark_open(&mut out, cur, x, to);
    out
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !id.starts_with("jbhsms_") || to >= x.c.len() || from > to {
        return None;
    }
    if params(id).is_none() {
        return if catalogue().iter().any(|s| s.id == id) { Some(vec![]) } else { None };
    }
    let g = |k: &str| getp(id, p, k);
    Some(keydate(x, from, to, g("entry_m"), g("entry_d"), g("exit_m"), g("exit_d"), g("stop_pct")))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;
    fn row(d: &str, c: f64) -> Row {
        Row { ticker: "T".into(), date: d.into(), open: c, high: c, low: c, close: c, volume: None }
    }
    #[test]
    fn year_wrap_and_weekend_entry() {
        // Dec 20 2003 is a Saturday -> enter Mon Dec 22; exit Jan 7 2004 (Wed)
        let r = vec![row("2003-12-19", 100.), row("2003-12-22", 101.), row("2003-12-23", 100.), row("2004-01-06", 103.), row("2004-01-07", 104.), row("2004-01-08", 99.)];
        let x = Ctx::new(&r, &r, &[]);
        let ts = keydate(&x, 0, 5, 12., 20., 1., 7., 3.0);
        assert_eq!(ts.len(), 1);
        assert_eq!((ts[0].legs[0].i, ts[0].exit_i), (1, 4));
        // close-only stop: 3% below 101 = 97.97
        let r2 = vec![row("2003-12-19", 100.), row("2003-12-22", 101.), row("2003-12-23", 97.9), row("2004-01-07", 104.)];
        let x2 = Ctx::new(&r2, &r2, &[]);
        let t2 = keydate(&x2, 0, 3, 12., 20., 1., 7., 3.0);
        assert_eq!((t2[0].exit_i, t2[0].reason.as_str()), (2, "Close-only stop"));
    }
}
