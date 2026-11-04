//! Josh Murakami, "Trading System Design: The L/S Model Trading System and Other Trading System Analysis" (c.2002).
//! Book id `jmls`. Source copy: Books/Josh Murakami/L-S Model Trading System.pdf. Transcription: library/jmls/.
//! Every system in the book runs on intraday bars (hourly, 15-minute, 30-minute pit S&P). Lane K39 (4 Nov 2026): three run on
//! N-minute bars built from the vault's real 1-minute bars (intraday_ep); the ATR(35) > 3 filter (S&P points) stays held.
use crate::connors::{ParamDef, Params, Status, StrategyDef, Ctx, Trade};

pub const BOOK: &str = "jmls";
macro_rules! live {
    ($id:expr, $name:expr, $group:expr, $rules:expr, $notes:expr) => {
        StrategyDef { id: $id, name: $name, group: $group, session: 0, dir: "Long/Short", status: Status::Live,
            needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK, rules: $rules, notes: $notes, claim: None }
    };
}

macro_rules! nd {
    ($id:expr, $name:expr, $group:expr, $why:expr, $rules:expr, $notes:expr) => {
        StrategyDef { id: $id, name: $name, group: $group, session: 0, dir: "Long/Short", status: Status::NeedsData($why),
            needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK, rules: $rules, notes: $notes, claim: None }
    };
}

pub fn catalogue() -> Vec<StrategyDef> {
    vec![
        live!("jmls_ls_model", "The L/S Model", "The L/S Model (pp.72-77)",
            &["E|\"If the time is greater than 1300 EST and less than 1605 EST and the market is greater than yesterday's close at the close of the hourly bar then go long\" (p.76-77)",
              "E|\"If the time is greater than 1300 EST and less than 1605 EST and the market is less than yesterday's close at the close of the hourly bar then go short\" (p.77)",
              "X|Always in: the opposite signal reverses (\"If there is a reversal in the afternoon and the market has gone against yesterday's close then the system will reverse\", p.78). Code: If time > 1300 and time < sess1endtime - 10 then begin If close>closeD(1) then buy this bar at close; If close<closeD(1) then sell this bar at close; End;"],
            concat!("Natural-hour (clock) bars; trades at the 14:00, 15:00 and 16:00 ET closes against the previous minute day's last close [code > 1300; the prose also lists 1300]. SPY and SPX only: the time filter is a US Eastern clock time with no NSE equivalent in the book. ", "[intraday EP, K39] Run on bars built from real 1-minute bars, one continuous series across days: always in, positions carry overnight. The book used pit S&P futures (9:30-16:15 ET); SPY and SPX are a declared application, cash-index shorts are hypothetical. No costs. ", "Pit S&P futures, hourly bars, 1/96-7/02, $250/point, no inputs (p.74). Optimisation of the time filter on natural hour bars (p.80): 1000 $483,012.5 35% $290.80 avg; 1100 $563,962.5 38% $403.70; 1200 $614,812.5 42% $514.49; 1300 $637,137.5 44% $616.78 (maxDD -$49,100, PF 1.55); 1400 $576,712.5 45% $663.65; 1500 $302,912.5 42% $420.13. Performance report pages are images.")),
        nd!("jmls_ls_atr_filter", "The L/S Model with ATR(35) > 3 volatility filter", "The L/S Model Dissected (pp.78-84)",
            "Needs hourly S&P bars (35-bar average true range of hourly bars and the hourly L/S signals); the vault has daily bars only.",
            &["F|\"The filter requires the average true range of the past x hourly bars to be greater than 3\" (p.82); code: If avgtruerange(35)>3 then begin {system code here} End;",
              "E|L/S Model entries as in jmls_ls_model", "X|Always in, reversing (p.78)"],
            "December 1983-July 2002 pit S&P (p.81-83), equity curve only."),
        live!("jmls_bar_breakout", "Bar Breakout System", "The Bottom Line (pp.22-24)",
            &["E|\"on a 15 minute bar chart, buy at the highest high of the last 25 bars and sell at the lowest low of the last 25 bars\" (p.22)",
              "E|Code: Input: EntryBars(25); If time > Sess1StartTime and time < Sess1EndTime then begin Buy next bar at Highest(High, EntryBars) Stop; Sell next bar at Lowest(Low, EntryBars) Stop; End (p.23-24)",
              "X|Stop and reverse; \"this is without any exit techniques\" (p.24)"],
            concat!("Session-aligned 15-minute bars on all four minute tickers (SPY, SPX, NIFTY 50, NIFTYBANK). Highest(High,25)/Lowest(Low,25) include the bar just closed; no orders from the session's last bar; the next bar's minutes decide the fill (at the stop, or the minute's open when it opened through it); one fill per bar [convention: TradeStation without intrabar order generation]. ", "[intraday EP, K39] Run on bars built from real 1-minute bars (NIFTY 50 days from 2026-08-11 skipped), one continuous series across days: always in, positions carry overnight. The book used pit S&P futures; the indices and SPY are a declared application, cash-index shorts are hypothetical. No costs. ", "Five years: $463,087 net (p.22), historic win rate 42% (p.23), max intraday drawdown $38,550 (p.24); $80,000 per contract FFP version traded up to 130 contracts (p.52).")),
        live!("jmls_ma_cross_30m", "4/21 SMA crossover on 30-minute bars (optimisation example)", "Views on Optimization (pp.37-41)",
            &["E|\"long when the fast moving average is above the slower moving average and short when the fast moving average is below the slow moving average. On 30 min bars the fast simple moving average (ma) is set to 4 and the slow ... The best parameter ends up being 21\" (p.37)",
              "X|Always in (reverses on the opposite cross)"],
            concat!("[reading] The book prints no code: closes of session-aligned 30-minute bars, simple averages over one continuous series, filled at the close of the bar where the order changes (the author's own L/S code fills 'this bar at close'). All four minute tickers. ", "[intraday EP, K39] Run on bars built from real 1-minute bars (NIFTY 50 days from 2026-08-11 skipped), one continuous series across days: always in, positions carry overnight. The book used pit S&P futures; the indices and SPY are a declared application, cash-index shorts are hypothetical. No costs. ", "Optimisation table p.37-38: slow 21 -> $86,600, 182 trades, 39% win, $475.82 avg, maxDD -$39,175, PF 1.37 (test window not stated). Shown as an example of parameter choice, not recommended by the author (p.41).")),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    match id {
        "jmls_ls_model" => Some(("jmls_ls", "The L/S Model", "Core, hourly")),
        "jmls_ls_atr_filter" => Some(("jmls_ls", "The L/S Model", "ATR(35) > 3 filter")),
        "jmls_bar_breakout" => Some(("jmls_bb", "Bar Breakout", "25-bar, 15-minute")),
        "jmls_ma_cross_30m" => Some(("jmls_ma", "SMA crossover", "4/21, 30-minute")),
        _ => None,
    }
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    (id.starts_with("jmls_") && crate::intraday_ep::id(id)).then(Vec::new) // intraday EP block (K39): no inputs to vary here
}

// ---- intraday EP block (lane K39, 4 Nov 2026, decision #202): the three complete systems on N-minute bars built from the
// 1-minute file (intraday_ep::nbars), one continuous series across days (EP2): always in, positions carry overnight.
pub fn run(id: &str, x: &Ctx, from: usize, to: usize, _p: &Params) -> Option<Vec<Trade>> {
    if !id.starts_with("jmls_") || !crate::intraday_ep::id(id) || from > to || to >= x.r.len() { return None; }
    Some(match id {
        "jmls_ls_model" => ls_model(x, from, to),
        "jmls_bar_breakout" => bar_breakout(x, from, to, 25),
        _ => ma_cross(x, from, to, 4, 21),
    })
}

/// The L/S Model (pp.76-77): "If time > 1300 and time < sess1endtime - 10 then begin If close>closeD(1) then buy this bar at
/// close; If close<closeD(1) then sell this bar at close; End;" on natural-hour (clock-aligned) bars; the prose sets the window
/// as "greater than 1300 EST and less than 1605 EST" (the pit session ended 16:15), so the 14:00, 15:00 and 16:00 bars trade
/// (code: > 1300; the prose's list of four points also names 1300). closeD(1) = the previous minute day's last close.
fn ls_model(x: &Ctx, from: usize, to: usize) -> Vec<Trade> {
    let mut bk = crate::intraday_ep::Book::default();
    let (mut prev, mut last) = (f64::NAN, None);
    for i in from..=to {
        let Some(m) = crate::intraday_w2b::day(x, i) else { continue };
        let b = crate::intraday_ep::nbars(&m, 0, 60, crate::intraday_ep::session_close(&x.r[i].ticker));
        for z in &b {
            if z.e > 13 * 60 && z.e < 16 * 60 + 5 && !prev.is_nan() {
                if z.c > prev { bk.enter(1.0, i, z.c, "close", "reverse"); }
                if z.c < prev { bk.enter(-1.0, i, z.c, "close", "reverse"); }
            }
        }
        if let Some(z) = b.last() { prev = z.c; last = Some((i, z.c)); }
    }
    match last { Some((i, c)) => bk.finish(i, c), None => vec![] }
}

/// Bar Breakout (pp.22-24): "If time > Sess1StartTime and time < Sess1EndTime then begin Buy next bar at Highest(High, EntryBars)
/// Stop; Sell next bar at Lowest(Low, EntryBars) Stop; End" on session-aligned 15-minute bars, stop and reverse. The levels
/// include the bar just closed; orders placed on the day's last bar (time = Sess1EndTime) are not placed. The next bar's
/// minutes decide the fill: at the level, or at the minute's open when it opened through it; one fill per bar [convention:
/// TradeStation without intrabar order generation]. Flat at the start, both stops are live; a minute reaching both without
/// opening through exactly one: the level nearer its open first, counted as ambiguous (intraday_w2b::first_exit).
fn bar_breakout(x: &Ctx, from: usize, to: usize, nb: usize) -> Vec<Trade> {
    use crate::intraday::Level;
    let mut bk = crate::intraday_ep::Book::default();
    let mut win: std::collections::VecDeque<(f64, f64)> = Default::default();
    let mut last = None;
    for i in from..=to {
        let Some((_, close, m, b)) = crate::intraday_ep::day_bars(x, i, 15) else { continue };
        let mut lv: Option<(f64, f64)> = None; // (buy stop, sell stop) placed on the previous bar of this day
        for z in &b {
            if let Some((hi, lo)) = lv.take() {
                let d = bk.dir();
                let mut levels = vec![];
                if d <= 0.0 { levels.push(Level { up: true, px: hi }); }
                if d >= 0.0 { levels.push(Level { up: false, px: lo }); }
                // one live level once in a position, so only the very first entry can meet an ambiguous minute
                if let Some((k, _, px, _)) = crate::intraday_w2b::first_exit(&m[z.a..z.b], 0, &levels) {
                    bk.enter(if levels[k].up { 1.0 } else { -1.0 }, i, px, "stop", "reverse");
                }
            }
            win.push_back((z.h, z.l));
            if win.len() > nb { win.pop_front(); }
            if win.len() == nb && z.e < close {
                lv = Some((win.iter().map(|w| w.0).fold(f64::NEG_INFINITY, f64::max), win.iter().map(|w| w.1).fold(f64::INFINITY, f64::min)));
            }
        }
        if let Some(z) = b.last() { last = Some((i, z.c)); }
    }
    match last { Some((i, c)) => bk.finish(i, c), None => vec![] }
}

/// 4/21 simple moving average crossover on 30-minute bars (p.37): "long when the fast moving average is above the slower moving
/// average and short when the fast moving average is below". The book prints no code: [reading] closes of session-aligned
/// 30-minute bars, averages over one continuous series across days, filled at the close of the bar where the order changes
/// (the author's own L/S code fills "this bar at close").
fn ma_cross(x: &Ctx, from: usize, to: usize, f: usize, s: usize) -> Vec<Trade> {
    let mut bk = crate::intraday_ep::Book::default();
    let mut cs: std::collections::VecDeque<f64> = Default::default();
    let mut last = None;
    for i in from..=to {
        let Some((_, _, _, b)) = crate::intraday_ep::day_bars(x, i, 30) else { continue };
        for z in &b {
            cs.push_back(z.c);
            if cs.len() > s { cs.pop_front(); }
            if cs.len() < s { continue; }
            let fa = cs.iter().skip(s - f).sum::<f64>() / f as f64;
            let sa = cs.iter().sum::<f64>() / s as f64;
            if fa > sa { bk.enter(1.0, i, z.c, "close", "reverse"); }
            if fa < sa { bk.enter(-1.0, i, z.c, "close", "reverse"); }
        }
        if let Some(z) = b.last() { last = Some((i, z.c)); }
    }
    match last { Some((i, c)) => bk.finish(i, c), None => vec![] }
}
// ---- end intraday EP block
