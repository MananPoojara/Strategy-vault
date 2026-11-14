//! YT Strategies · Supertrend (Intraday): the team's AmiBroker AFLs in "Old Topics/Supertrend/ST Fresh Running
//! Buy-Sell Index Intraday" that trade one F&O futures series on intraday bars, long and short, squared off at
//! 15:20. Coded from the AFLs as written, with the AFLs' own default Param values; there is no book PDF and no trade
//! sheet was compared (coded only, 14-11-2026).
//!
//! Coded (one id per AFL; the two files differ only in the entry condition):
//! - "LONG & Short Intraday in Nifty FO by ST(52) with FRESH Entry.afl" (Suraj Mehra, checked by Jimit, 21-02-2022):
//!   buy on the bar where the ST trend turns up (`MTREND[i]==1 AND MTREND[i-1]==-1`), short where it turns down;
//! - "LONG & Short Intraday in Nifty FO by ST(52) with RUNNING Entry.afl" (checked by Saumya, 22-02-2022): buy on any
//!   bar where the ST trend is up and the position is not long (so also on the day's first bar after a square-off),
//!   short on any bar where it is down and the position is not short.
//! Both: the entry is taken only while `TimeNow < SquareOffTime` (1520); an opposite entry closes the open position
//! at the same close (reversal); a long (short) still open on the first bar with `TimeNow >= SquareOffTime` is sold
//! (covered) at that bar's close. All fills are at the bar's close (`BuyPrice = Close` etc.).
//!
//! Data: the 1-minute F&O futures of the team's AmiBroker IEOD database (`crate::intraday`), compressed to the AFL
//! header's "Timeframe: 5 Minute" (the AFL itself has no TimeFrameSet: it runs on the Analysis periodicity, offered as
//! the `tf_min` setting, default 5). Candles are anchored at 09:15 and labelled with their START time, so
//! `TimeNow = Hour()*100+Minute()` of the 15:20 candle is 1520 and the square-off fills at that candle's close (the
//! 15:24 minute); for 5-minute candles this grid equals AmiBroker's own time compression (see `intraday::compress`),
//! for other timeframes it is a convention not checked against an AmiBroker run. The engine's daily `ieodfo` series
//! (the same minutes aggregated to days) carries the trades: entry leg / exit_i are the daily bars of the entry /
//! exit dates, prices are the candle closes, and the entry / exit times are written in each trade's reason.
//!
//! AmiBroker behaviour followed (as in `ytst.rs`):
//! - the ST is the AFL's own loop, which is the team's ST (`ytst::st_lines`): satr = mean of the last `period`
//!   ATR(1) values (ATR(1) Null on the first bar, so the bands are Null until bar `period` and the trend stays 1),
//!   bands ratchet and reset on a flip, the trend flips when the Close crosses the previous bar's band; calculation
//!   point mid point (H+L)/2 (default), close or pivot (H+L+C)/3;
//! - the ST and the position loop run over all loaded candles, continuously across days and sessions (state starts at
//!   the data's first candle, `Mtrend[0]=1`, `Position=0`); a position the data never squares off (a day without a
//!   candle at or after the square-off time) is carried to the next day, as the loop does;
//! - the backtest range: a trade counts when its entry date is inside the selected period; a position still open at
//!   the period's last candle is closed at that candle's close (end of backtest).
//!
//! Not modelled: the AFLs' backtest settings (initial equity 10,00,000, position size, `MAXOPENPOSITIONS` 20,
//! brokerage 0.65 with commission mode 1, `AllowPositionShrinking`) — the engine's own capital / cost settings
//! apply; the commented-out dead-stock / circuit / turnover filters are not used by the AFLs. Contract roll-overs of
//! the continuous futures series are not modelled (an intraday position never crosses an expiry in normal sessions).
#![allow(dead_code)]
use crate::books::ytst::{calc_on, st_lines};
use crate::connors::{Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};
use crate::intraday::{self, Candles, Minutes};

pub const BOOK: &str = "yt_sti";
pub const FRESH: &str = "ytsti_st52_fresh_ls";
pub const RUNNING: &str = "ytsti_st52_running_ls";
pub const IDS: [&str; 2] = [FRESH, RUNNING];

const GROUP: &str = "YT Strategies · Supertrend (Intraday)";

fn def(id: &'static str, name: &'static str, rules: &'static [&'static str], notes: &'static str) -> StrategyDef {
    StrategyDef { id, name, group: GROUP, session: 0, dir: "Long/Short", status: Status::Live, needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK, rules, notes, claim: None }
}

const DATA_F: &str = "F|Data: 1-minute F&O futures (team AmiBroker IEOD database) compressed to 5-minute candles from 09:15, each labelled with its start time";
const ST_F: &str = "F|ST (the AFL's own loop): period 5, factor 2, on the mid point (H+L)/2; bands = point ± factor × mean of the last 5 ATR(1); the trend flips when the Close crosses the previous candle's band";
const TIME_F: &str = "F|Entries only while the candle's time is before the square-off time 15:20";
const X_SQ: &str = "X|Square off: sell (cover) at the close of the first candle at or after 15:20";
const X_REV: &str = "X|An opposite entry closes the position at the same close and reverses it";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![
        def(FRESH, "Intraday ST(5,2) Long-Short, Fresh Entry, 15:20 Square-off", &[
            DATA_F, ST_F, TIME_F,
            "E|Buy at the candle's close when the ST turns up (up now, down on the previous candle); short when it turns down",
            X_REV, X_SQ,
        ], "LONG & Short Intraday in Nifty FO by ST(52) with FRESH Entry.afl (folder 'ST Fresh Running Buy-Sell Index Intraday'), coded as the AFL is written on the AFL header's 5-minute timeframe. Runs on one F&O futures symbol with intraday data (ieodfo); trades are intraday and each trade's note gives its entry / exit times. The ST runs continuously across days. The AFL's equity, position-size, brokerage and max-position settings are not modelled. Not compared with the team's trade sheets."),
        def(RUNNING, "Intraday ST(5,2) Long-Short, Running Entry, 15:20 Square-off", &[
            DATA_F, ST_F, TIME_F,
            "E|Buy at the candle's close whenever the ST is up and the position is not long (also the day's first candle); short whenever it is down and the position is not short",
            X_REV, X_SQ,
        ], "LONG & Short Intraday in Nifty FO by ST(52) with RUNNING Entry.afl (same folder; identical to the FRESH Entry AFL except that the entry needs only the current ST trend, not a flip), coded as the AFL is written on the 5-minute timeframe. Runs on one F&O futures symbol with intraday data (ieodfo); each trade's note gives its entry / exit times. The AFL's equity, position-size, brokerage and max-position settings are not modelled. Not compared with the team's trade sheets."),
    ]
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    if !IDS.contains(&id) {
        return None;
    }
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    Some(vec![
        p("tf_min", "Timeframe in minutes (the AFL header: 5 Minute)", 5.0, 1.0, 60.0, 1.0),
        p("st_period", "ST period", 5.0, 1.0, 20.0, 1.0),
        p("st_factor", "ST factor", 2.0, 1.0, 20.0, 1.0),
        p("st_point", "ST calculation point (1 mid point (H+L)/2, 2 close, 3 pivot (H+L+C)/3)", 1.0, 1.0, 3.0, 1.0),
        p("sq_time", "Square-off time (HHMM)", 1520.0, 1200.0, 1530.0, 5.0),
    ])
}

/// Settings of one run, read from the params.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Cfg {
    pub fresh: bool,
    pub tf_min: u16,
    pub period: usize,
    pub factor: f64,
    pub point: i64,
    pub sq_time: u16,
}

fn cfg(id: &str, g: &dyn Fn(&str) -> f64) -> Cfg {
    Cfg {
        fresh: id == FRESH,
        tf_min: g("tf_min").round().clamp(1.0, 375.0) as u16,
        period: g("st_period").round().max(1.0) as usize,
        factor: g("st_factor"),
        point: g("st_point").round() as i64,
        sq_time: g("sq_time").round().clamp(0.0, 2359.0) as u16,
    }
}

fn hm(t: u16) -> String {
    format!("{:02}:{:02}", t / 100, t % 100)
}

/// The AFL's position loop on candles `b` (all loaded candles; `di[k]` = daily index of candle k), trades whose
/// entry daily bar is in from..=to.
pub(crate) fn trades_on(b: &Candles, di: &[usize], x: &Ctx, from: usize, to: usize, c: Cfg) -> Vec<Trade> {
    let n = b.len();
    if n == 0 {
        return Vec::new();
    }
    let calc = calc_on(&b.h, &b.l, &b.c, c.point, false);
    let (tr, _, _) = st_lines(&calc, &b.h, &b.l, &b.c, c.period, c.factor);
    // the last candle of the selected period: positions still open there are closed at its close
    let end = match (0..n).rev().find(|&k| di[k] <= to) {
        Some(k) => k,
        None => return Vec::new(),
    };
    let date = |k: usize| x.r[di[k]].date.as_str();
    let mut out: Vec<Trade> = Vec::new();
    // open position: (dir, entry candle, why)
    let mut pos: Option<(f64, usize, &'static str)> = None;
    let close = |out: &mut Vec<Trade>, (dir, e, why): (f64, usize, &'static str), k: usize, how: &str, open: bool| {
        if di[e] < from || di[e] > to {
            return;
        }
        let when = if b.day[k] == b.day[e] { hm(b.hhmm[k]) } else { format!("{} {}", date(k), hm(b.hhmm[k])) };
        let side = if dir > 0.0 { "Long" } else { "Short" };
        let reason = format!("{side} {} ({why}) → {how} {when}; {}-minute candles, fills at the candle's close", hm(b.hhmm[e]), c.tf_min);
        out.push(Trade { dir, legs: vec![Leg { i: di[e], px: b.c[e], w: 1.0 }], exit_i: di[k], exit_px: b.c[k], reason, entry_kind: "close", signal_i: di[k], open, exits: Vec::new() });
    };
    for k in 1..=end {
        let t = b.hhmm[k];
        let state = pos.map_or(0.0, |p| p.0);
        let flip_up = !c.fresh || tr[k - 1] == -1;
        let flip_dn = !c.fresh || tr[k - 1] == 1;
        let why_up = if c.fresh { "ST turned up" } else { "ST up" };
        let why_dn = if c.fresh { "ST turned down" } else { "ST down" };
        if state != 1.0 && tr[k] == 1 && flip_up && t < c.sq_time {
            if let Some(p) = pos.take() {
                close(&mut out, p, k, "reversed", false);
            }
            pos = Some((1.0, k, why_up));
        } else if state != -1.0 && tr[k] == -1 && flip_dn && t < c.sq_time {
            if let Some(p) = pos.take() {
                close(&mut out, p, k, "reversed", false);
            }
            pos = Some((-1.0, k, why_dn));
        } else if state != 0.0 && t >= c.sq_time {
            if let Some(p) = pos.take() {
                close(&mut out, p, k, "square-off", false);
            }
        }
    }
    if let Some(p) = pos {
        // still open at the period's last candle: closed there (books::close_at_end words it as end of backtest)
        close(&mut out, p, end, "open", true);
    }
    out
}

/// Candles of `m` on the run's timeframe, restricted to the dates the daily carrier has, with their daily indices.
pub(crate) fn candles_for(m: &Minutes, x: &Ctx, tf_min: u16) -> (Candles, Vec<usize>) {
    let map = intraday::day_index(m, x.r);
    let all = intraday::compress(m, tf_min);
    let keep: Vec<usize> = (0..all.len()).filter(|&k| map[all.day[k] as usize].is_some()).collect();
    let pick = |v: &[f64]| keep.iter().map(|&k| v[k]).collect::<Vec<f64>>();
    let b = Candles {
        day: keep.iter().map(|&k| all.day[k]).collect(),
        hhmm: keep.iter().map(|&k| all.hhmm[k]).collect(),
        o: pick(&all.o),
        h: pick(&all.h),
        l: pick(&all.l),
        c: pick(&all.c),
        v: pick(&all.v),
        start: keep.iter().map(|&k| all.start[k]).collect(),
        end: keep.iter().map(|&k| all.end[k]).collect(),
    };
    let di = b.day.iter().map(|&d| map[d as usize].unwrap()).collect();
    (b, di)
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    let schema = params(id)?;
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).map_or(0.0, |d| d.def));
    let n = x.c.len();
    if n == 0 || from > to || to >= n {
        return Some(Vec::new());
    }
    // The daily carrier's rows hold the symbol; admission (instrument_policy / readiness) has already required an
    // ieodfo series with an intraday file, so a missing file here only means no trades.
    let Some(m) = intraday::minutes(&format!("{}{}", intraday::BASKET_PREFIX, x.r[0].ticker)) else {
        return Some(Vec::new());
    };
    let c = cfg(id, &g);
    let (b, di) = candles_for(&m, x, c.tf_min);
    Some(trades_on(&b, &di, x, from, to, c))
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    let d = catalogue().into_iter().find(|d| d.id == id)?;
    Some((d.id, d.name, "Supertrend (Intraday)"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;

    fn day_rows(dates: &[&str]) -> Vec<Row> {
        dates.iter().map(|d| Row { ticker: "TEST".into(), date: (*d).into(), open: 100.0, high: 101.0, low: 99.0, close: 100.0, volume: None }).collect()
    }

    /// Minutes from (day, hhmm, close) with high / low = close ± 0.5.
    fn mins(dates: &[&str], v: &[(u32, u16, f32)]) -> Minutes {
        let mut m = Minutes { symbol: "TEST".into(), dates: dates.iter().map(|d| d.to_string()).collect(), ..Default::default() };
        for &(d, t, c) in v {
            m.day.push(d);
            m.hhmm.push(t);
            m.o.push(c);
            m.h.push(c + 0.5);
            m.l.push(c - 0.5);
            m.c.push(c);
            m.v.push(1.0);
        }
        m
    }

    fn label(minute: u16) -> u16 {
        minute / 60 * 100 + minute % 60
    }

    /// One day of 1-minute bars (09:15 .. 15:29) whose closes follow `f(minute index)`.
    fn day(d: u32, f: impl Fn(usize) -> f32) -> Vec<(u32, u16, f32)> {
        (0..375).map(|k| (d, label(555 + k as u16), f(k))).collect()
    }

    fn cfg1(fresh: bool) -> Cfg {
        Cfg { fresh, tf_min: 1, period: 2, factor: 1.0, point: 1, sq_time: 1520 }
    }

    fn go(m: &Minutes, r: &[Row], c: Cfg, from: usize, to: usize) -> Vec<Trade> {
        let x = Ctx::new(r, r, &[]);
        let (b, di) = candles_for(m, &x, c.tf_min);
        trades_on(&b, &di, &x, from, to, c)
    }

    #[test]
    fn fresh_entry_on_flips_reverses_and_squares_off_at_1520() {
        // flat at 100, falls to 90 from minute 30 (09:45), rises to 120 from minute 120 (11:15)
        let f = |k: usize| if k < 30 { 100.0 } else if k < 120 { 90.0 } else { 120.0 };
        let dates = ["2022-03-01"];
        let m = mins(&dates, &day(0, f));
        let r = day_rows(&dates);
        let t = go(&m, &r, cfg1(true), 0, 0);
        assert_eq!(t.len(), 2, "{t:?}");
        // short on the 09:45 flip down at 90, reversed on the 11:15 flip up at 120; the long squared off at 15:20
        assert_eq!((t[0].dir, t[0].legs[0].px, t[0].exit_px), (-1.0, 90.0, 120.0));
        assert!(t[0].reason.starts_with("Short 09:45 (ST turned down) → reversed 11:15"), "{}", t[0].reason);
        assert_eq!((t[1].dir, t[1].legs[0].px, t[1].exit_px), (1.0, 120.0, 120.0));
        assert!(t[1].reason.contains("Long 11:15 (ST turned up) → square-off 15:20"), "{}", t[1].reason);
        // same-day trades on the daily carrier: entry and exit on daily bar 0, accepted by trade validation
        assert!(t.iter().all(|t| t.legs[0].i == 0 && t.exit_i == 0 && !t.open));
        crate::trade_validation::check(&t, 0, 0, 0.0).unwrap();
    }

    #[test]
    fn running_entry_takes_the_current_trend_each_morning() {
        // day 1 falls at 09:25 and rises at 09:45; day 2 keeps rising. The fresh AFL trades the two day-1 flips only;
        // the running AFL also buys at 09:16 (the trend starts up) and at day 2's first candle, because the trend is
        // still up after the 15:20 square-off
        let f = |k: usize| if k < 10 { 100.0 } else if k < 30 { 90.0 } else { 110.0 + (k as f32) * 0.01 };
        let dates = ["2022-03-01", "2022-03-02"];
        let mut v = day(0, f);
        v.extend(day(1, |k| 115.0 + (k as f32) * 0.01));
        let m = mins(&dates, &v);
        let r = day_rows(&dates);
        let fresh = go(&m, &r, cfg1(true), 0, 1);
        assert_eq!(fresh.iter().map(|t| t.dir).collect::<Vec<_>>(), vec![-1.0, 1.0], "{fresh:?}");
        let running = go(&m, &r, cfg1(false), 0, 1);
        assert_eq!(running.iter().map(|t| t.dir).collect::<Vec<_>>(), vec![1.0, -1.0, 1.0, 1.0], "{running:?}");
        assert!(running[0].reason.starts_with("Long 09:16 (ST up) → reversed 09:25"), "{}", running[0].reason);
        assert_eq!((running[3].legs[0].i, running[3].exit_i), (1, 1));
        assert!(running[3].reason.starts_with("Long 09:15 (ST up) → square-off 15:20"), "{}", running[3].reason);
        crate::trade_validation::check(&running, 0, 1, 0.0).unwrap();
        // the period filter: only day 2's entry when the run starts on day 2
        assert_eq!(go(&m, &r, cfg1(false), 1, 1).len(), 1);
        assert_eq!(go(&m, &r, cfg1(true), 1, 1).len(), 0);
    }

    #[test]
    fn no_entry_at_or_after_the_square_off_time_and_carry_without_a_1520_bar() {
        // the flip comes at 15:22: no entry (TimeNow >= 1520)
        let f = |k: usize| if k < 367 { 100.0 } else { 90.0 };
        let dates = ["2022-03-01", "2022-03-02"];
        let m = mins(&dates, &day(0, f));
        let r = day_rows(&dates[..1]);
        assert!(go(&m, &r, cfg1(true), 0, 0).is_empty());
        // a short session ending 12:44 has no candle at 15:20: the short is carried to the next day's square-off
        let mut v: Vec<(u32, u16, f32)> = (0..60).map(|k| (0, label(675 + k), if k < 20 { 100.0 } else { 90.0 })).collect();
        v.extend(day(1, |_| 90.0));
        let m = mins(&dates, &v);
        let r = day_rows(&dates);
        let t = go(&m, &r, cfg1(true), 0, 1);
        assert_eq!(t.len(), 1);
        assert_eq!((t[0].legs[0].i, t[0].exit_i), (0, 1));
        assert!(t[0].reason.contains("square-off 2022-03-02 15:20"), "{}", t[0].reason);
        // ending the period on day 1 leaves it open at the day's last candle (closed by books::close_at_end)
        let t = go(&m, &r, cfg1(true), 0, 0);
        assert!(t[0].open && t[0].exit_i == 0);
    }

    #[test]
    fn five_minute_candles_square_off_at_the_1520_candle_close() {
        // falls at 10:00; with 5-minute candles the short is covered at the 15:20 candle's close (the 15:24 minute)
        let f = |k: usize| if k < 45 { 100.0 } else if k < 369 { 90.0 } else { 80.0 };
        let dates = ["2022-03-01"];
        let m = mins(&dates, &day(0, f));
        let r = day_rows(&dates);
        let c = Cfg { tf_min: 5, ..cfg1(true) };
        let t = go(&m, &r, c, 0, 0);
        assert_eq!(t.len(), 1, "{t:?}");
        assert!(t[0].reason.starts_with("Short 10:00 (ST turned down) → square-off 15:20; 5-minute candles"), "{}", t[0].reason);
        assert_eq!(t[0].exit_px, 80.0); // 15:24 close (minute 369 = 15:24)
    }

    #[test]
    fn defaults_and_registration() {
        let ps = params(FRESH).unwrap();
        let d = |k: &str| ps.iter().find(|p| p.key == k).unwrap().def;
        assert_eq!((d("tf_min"), d("st_period"), d("st_factor"), d("st_point"), d("sq_time")), (5.0, 5.0, 2.0, 1.0, 1520.0));
        assert!(params("ytst_st_all").is_none());
        assert_eq!(catalogue().len(), IDS.len());
        assert!(catalogue().iter().all(|d| d.book == BOOK && IDS.contains(&d.id)));
        // without intraday data for the ticker the run has no trades (readiness reports why before any run)
        let r = day_rows(&["2022-03-01", "2022-03-02"]);
        let x = Ctx::new(&r, &r, &[]);
        assert!(run(FRESH, &x, 0, 1, &Params::new()).unwrap().is_empty());
        assert!(run("ytst_st_all", &x, 0, 1, &Params::new()).is_none());
    }
}
