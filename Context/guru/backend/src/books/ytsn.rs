//! YT Strategies · Sniper System: the team's AmiBroker AFL "Old Topics/Sniper System/New Optims/SNIPER.afl", long only
//! on daily bars of an index. Coded from the AFL as written, with its own default settings; there is no book PDF.
//!
//! AmiBroker behaviour as in `books::ytsw` (fills at the close unless a live HSL / PB price applies, exit then entry
//! on one bar, ATR(1) Null on the first bar). Specific to this AFL:
//! - its Supertrend: band ATR = mean of the last `period` ATR(1) values; the trend loop starts at bar `period`
//!   (earlier bars are up), and a band only ratchets (no reset on a flip);
//! - a weekly / monthly ST is computed on compressed bars, shifted back one period, expanded with expandFirst and
//!   then shifted FORWARD one daily bar (`Ref(x, 1)`): each day of a period sees the previous completed period,
//!   except the period's last trading day, which already sees its own period (known at that close; the forward
//!   shift reads the next bar's date), and the last bar of the data gets Null;
//! - Null read in the bar loop is AmiBroker's -1e10.
#![allow(dead_code)]
use crate::books::ytsw::{compress, cross_up, trade};
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "yt_sn";
pub const SNIPER: &str = "ytsn_sniper";
/// The older Sniper version, rebuilt from the team's own trade sheets: its AFL is not in the folder. Six team sheets
/// (three "STR Exit" and two "% Swing" workbooks, plus "Nifty 9,1 9,6") differ from SNIPER.afl in two exit rules.
pub const SNIPER_OLD: &str = "ytsn_sniper_old";
pub const IDS: [&str; 2] = [SNIPER, SNIPER_OLD];

const NULL: f64 = -1e10;

pub fn catalogue() -> Vec<StrategyDef> {
    let current = StrategyDef {
        id: SNIPER,
        name: "Sniper (Monthly ST Cross)",
        group: "YT Strategies · Sniper System",
        session: 0,
        dir: "Long",
        status: Status::Live,
        needs_volume: false,
        needs_vix: false,
        needs_series: &[],
        book: BOOK,
        rules: &[
            "F|Monthly Supertrend of the traded index: period 5, factor 2, on (H+L)/2",
            "F|Until a month's last trading day, the daily bars see the previous month's ST",
            "F|Daily Close crosses above the monthly ST line",
            "F|Last month closed below its ST",
            "F|Default: daily cross; options: wait for the month end, or only the monthly Close crossing the ST",
            "E|Buy at the Close",
            "F|No entry on a bar whose High = Close",
            "F|No entry on the last bar of the data",
            "X|Optional exits, each off by default: % swing below the highest Close since entry, hard stop %, profit booking %, a number of bars",
            "X|Optional exit (off by default): the daily ST pair (5, 1) and (5, 2) turning down",
            "X|With every exit off (the AFL's defaults) the position is held to the end of the data",
        ],
        notes: "SNIPER.afl (Sniper System / New Optims), coded as the AFL is written. On the daily entry time frame the entry can never fire (Cross(C, ST) and C < ST on the same bar), as written. ParamList defaults are list indices: entry ST time frame 12 = Monthly, exit ST time frames 10 = Daily, ST average 5 = (High+Low)/2, entry type 0 = Daily Cross. The time-frame list's 'Default' and minute entries are not matched by the AFL's getTimeframe, so on daily data they are the daily chart (option 0 here). The higher-time-frame lines are shifted forward one bar with Ref(x, 1), so on a period's last trading day they already use that period's close (look-ahead of the next bar's date only, as in the AFL). HSL % and PB % default 0 (below their declared minimum 5), % swing 0 and number of days 0 (the counter is tested after it is incremented, so 0 never fires), ST exit off: with the defaults the AFL only exits on the last bar, shown here as an open position. The turnover filter uses open interest and is off by default; penny filter 0. The <parmar/...> includes are not available and do not drive the signals.",
        claim: None,
    };
    let old = StrategyDef {
        id: SNIPER_OLD,
        name: "Sniper (Older Version, Rebuilt)",
        rules: &[
            "F|Entry rules exactly as SNIPER.afl (monthly Supertrend on (H+L)/2, same options and filters); opens on the Nifty 9,1 9,6 sheet's settings: Supertrend 5 with factor 1, entry only at month end",
            "E|Buy at the Close",
            "X|ST exit (older rule, on by default with daily ST1 9/1 and ST2 9/6), until the close has been above ST2: the close crossing below the daily ST1 line, or closing below ST2 (SNIPER.afl needs the close below both)",
            "X|% swing exit (older rule): the close more than the swing % below the highest HIGH since entry (SNIPER.afl uses the highest close)",
            "X|Other exits as SNIPER.afl: hard stop %, profit booking %, a number of bars; all off by default",
        ],
        notes: "Rebuilt from the team's trade sheets, not from an AFL: the older Sniper AFL is not in the team's folder. It is SNIPER.afl with two exit rules changed, each a setting (1 = the older rule, 0 = SNIPER.afl's): until the ST2 flag is set (the close above ST2), the ST exit fires on a new cross of the close below ST1 (it was not below on the previous bar) or on a close below ST2 (SNIPER.afl: below both); setting 1 is a plain ST1-or-ST2 rule, kept for comparison, and the % swing trails from the highest high of the bars after entry, starting from the entry close (SNIPER.afl: the highest close). These two rules were inferred from the 'STR Exit' and 'Month End - % Swing' workbooks, whose trades SNIPER.afl does not reproduce. Its default settings are the 'Nifty 9,1 9,6' sheet's (entry only at month end, entry factor 1, ST exit on with ST1 9/1 and ST2 9/6), so the default run shows the older exits; with SNIPER.afl's defaults every exit is off and both versions give the same single trade.",
        group: "YT Strategies · Sniper System",
        session: 0,
        dir: "Long",
        status: Status::Live,
        needs_volume: false,
        needs_vix: false,
        needs_series: &[],
        book: BOOK,
        claim: None,
    };
    vec![current, old]
}

const AVG_ENTRY: &str = "ST entry parameters » ST average value (0 close, 1 open, 2 high, 3 low, 4 (O+C)/2, 5 (H+L)/2, 6 (H+L+C)/3, 7 (O+H+L+C)/4)";
const AVG_ST1: &str = "ST1 exit parameters » ST average value (0 close, 1 open, 2 high, 3 low, 4 (O+C)/2, 5 (H+L)/2, 6 (H+L+C)/3, 7 (O+H+L+C)/4)";
const AVG_ST2: &str = "ST2 exit parameters » ST average value (0 close, 1 open, 2 high, 3 low, 4 (O+C)/2, 5 (H+L)/2, 6 (H+L+C)/3, 7 (O+H+L+C)/4)";

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    if id != SNIPER && id != SNIPER_OLD {
        return None;
    }
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    // in SNIPER.afl's Parameters window order and sections ("Section » name"); each ST average value sits with its own
    // Supertrend, as in the AFL
    Some(vec![
        p("penny", "Backtest parameter » Penny stock price (buy only above this close)", 0.0, 0.0, 100.0, 1.0),
        p("turnover", "Backtest parameter » Turnover filter (0 no, 1 yes: the AFL needs open interest above its yearly threshold; an index has no open interest, so no entry)", 0.0, 0.0, 1.0, 1.0),
        p("dead_stock", "Backtest parameter » Dead stock backtest (0 no, 1 yes: no entry on the last bar of the data, where an open trade exits; always on in the AFL)", 1.0, 0.0, 1.0, 1.0),
        p("entry_tf", "ST entry parameters » ST time frame (0 daily: the AFL's Default / minute / Daily choices on daily data, 1 weekly, 2 monthly)", 2.0, 0.0, 2.0, 1.0),
        p("entry_period", "ST entry parameters » Supertrend period", 5.0, 1.0, 100.0, 1.0),
        p("entry_avg", AVG_ENTRY, 5.0, 0.0, 7.0, 1.0),
        p("entry_factor", "ST entry parameters » ST factor", 2.0, 1.0, 50.0, 0.5),
        p("entry_type", "ST entry parameters » Take entry at (0 daily cross, 1 daily cross with month end, 2 only month end)", 0.0, 0.0, 2.0, 1.0),
        p("exit_bars", "Exit parameters » Number of days for exit (0 off)", 0.0, 0.0, 500.0, 1.0),
        p("pct_swing", "Exit parameters » % swing below the highest close since entry (0 off)", 0.0, 0.0, 26.0, 1.0),
        p("exit_live", "Exit parameters » Exit using HSL / PB at (0 close, 1 live: the level, or the open if it gaps past)", 0.0, 0.0, 1.0, 1.0),
        p("hsl_pct", "Exit parameters » HSL % (0 off)", 0.0, 0.0, 30.0, 5.0),
        p("pb_pct", "Exit parameters » PB % (0 off)", 0.0, 0.0, 30.0, 5.0),
        p("exit_buffer", "Exit parameters » Exit buffer % on a gap past the HSL / PB level", 0.0, 0.0, 100.0, 0.01),
        p("st_exit", "ST1 exit parameters » ST toggle: ST1 and ST2 exit (0 off, 1 on)", 0.0, 0.0, 1.0, 1.0),
        p("st1_tf", "ST1 exit parameters » ST time frame (0 daily, 1 weekly, 2 monthly)", 0.0, 0.0, 2.0, 1.0),
        p("st1_period", "ST1 exit parameters » Supertrend period 1", 5.0, 3.0, 9.0, 2.0),
        p("st1_factor", "ST1 exit parameters » ST factor 1", 1.0, 1.0, 6.0, 1.0),
        p("st1_avg", AVG_ST1, 5.0, 0.0, 7.0, 1.0),
        p("st2_tf", "ST2 exit parameters » ST time frame (0 daily, 1 weekly, 2 monthly)", 0.0, 0.0, 2.0, 1.0),
        p("st2_period", "ST2 exit parameters » Supertrend period 2", 5.0, 3.0, 9.0, 2.0),
        p("st2_factor", "ST2 exit parameters » ST factor 2", 2.0, 1.0, 6.0, 1.0),
        p("st2_avg", AVG_ST2, 5.0, 0.0, 7.0, 1.0),
    ]
    .into_iter()
    .chain(if id == SNIPER_OLD {
        vec![
            p("st_exit_rule", "Older version » ST exit rule (0 below ST1 AND ST2, SNIPER.afl; 1 below ST1 OR ST2; 2 a new cross below ST1 OR below ST2, older)", 2.0, 0.0, 2.0, 1.0),
            p("swing_from", "Older version » % swing measured from (1 highest high since entry, older; 0 highest close, SNIPER.afl)", 1.0, 0.0, 1.0, 1.0),
        ]
    } else {
        vec![]
    })
    .map(|mut d| {
        // the older version opens on its own sheet's settings (Sniper System/STR Exit, "Nifty 9,1 9,6": month-end entry
        // on a monthly 5/1 Supertrend, ST exit on with ST1 9/1 and ST2 9/6), so its default run shows the older exits;
        // with SNIPER.afl's defaults every exit is off and both versions give the same single trade
        if id == SNIPER_OLD {
            if let Some(&(_, v)) = OLD_DEFAULTS.iter().find(|(k, _)| *k == d.key) {
                d.def = v;
            }
        }
        d
    })
    .collect())
}

/// Defaults of the older version that differ from SNIPER.afl's (the "Nifty 9,1 9,6" sheet's settings).
/// (ST1 factor 1 is already SNIPER.afl's default.)
pub const OLD_DEFAULTS: [(&str, f64); 6] =
    [("entry_type", 2.0), ("entry_factor", 1.0), ("st_exit", 1.0), ("st1_period", 9.0), ("st2_period", 9.0), ("st2_factor", 6.0)];

/// The AFL's Supertrend on bars (o, h, l, c): returns (ST line, close < line); None where the line is Null.
pub(crate) fn sniper_st(o: &[f64], h: &[f64], l: &[f64], c: &[f64], period: usize, factor: f64, avg: i64) -> (Vec<Option<f64>>, Vec<Option<bool>>) {
    let n = c.len();
    let p = period.max(1);
    let tr: Vec<Option<f64>> = (0..n).map(|i| if i == 0 { None } else { Some(h[i].max(c[i - 1]) - l[i].min(c[i - 1])) }).collect();
    let atr: Vec<Option<f64>> = (0..n)
        .map(|i| if i + 1 < p { None } else { tr[i + 1 - p..=i].iter().try_fold(0.0, |s, v| v.map(|v| s + v)).map(|s| s / p as f64) })
        .collect();
    let av = |i: usize| match avg {
        0 => c[i],
        1 => o[i],
        2 => h[i],
        3 => l[i],
        4 => (o[i] + c[i]) / 2.0,
        5 => (h[i] + l[i]) / 2.0,
        6 => (h[i] + l[i] + c[i]) / 3.0,
        7 => (o[i] + h[i] + l[i] + c[i]) / 4.0,
        _ => c[i],
    };
    let mut up: Vec<f64> = (0..n).map(|i| atr[i].map_or(NULL, |a| av(i) + a * factor)).collect();
    let mut dn: Vec<f64> = (0..n).map(|i| atr[i].map_or(NULL, |a| av(i) - a * factor)).collect();
    let mut is_up = vec![true; n];
    for i in p..n {
        is_up[i] = if c[i] > up[i - 1] { true } else if c[i] < dn[i - 1] { false } else { is_up[i - 1] };
        if is_up[i] {
            dn[i] = dn[i].max(dn[i - 1]);
        } else {
            up[i] = up[i].min(up[i - 1]);
        }
    }
    let line: Vec<Option<f64>> = (0..n).map(|i| { let v = if is_up[i] { dn[i] } else { up[i] }; if v == NULL { None } else { Some(v) } }).collect();
    let bear = (0..n).map(|i| line[i].map(|v| c[i] < v)).collect();
    (line, bear)
}

/// An ST on the chosen time frame as the daily bars see it: (line, bear signal, period close).
fn st_on(x: &Ctx, tf: i64, period: usize, factor: f64, avg: i64) -> (Vec<Option<f64>>, Vec<Option<bool>>, Vec<Option<f64>>) {
    let n = x.c.len();
    if tf != 1 && tf != 2 {
        let (line, bear) = sniper_st(&x.o, &x.h, &x.l, &x.c, period, factor, avg);
        return (line, bear, x.c.iter().map(|&v| Some(v)).collect());
    }
    let b = compress(x, if tf == 2 { 'M' } else { 'W' });
    let m = b.c.len();
    // open of each period = open of its first daily bar
    let mut o = vec![0.0; m];
    for i in (0..n).rev() {
        o[b.group[i]] = x.o[i];
    }
    let (line, bear) = sniper_st(&o, &b.h, &b.l, &b.c, period, factor, avg);
    // Ref(x, -1) on the periods, expandFirst, then Ref(x, 1) on the days: day i sees period group[i + 1] - 1
    let see = |i: usize| -> Option<usize> { if i + 1 >= n { None } else { b.group[i + 1].checked_sub(1) } };
    (
        (0..n).map(|i| see(i).and_then(|g| line[g])).collect(),
        (0..n).map(|i| see(i).and_then(|g| bear[g])).collect(),
        (0..n).map(|i| see(i).map(|g| b.c[g])).collect(),
    )
}

/// AFL Cross(a, b) with both possibly Null: no cross before both have values, Null as -1e10 after that.
fn cross_opt(a: &[Option<f64>], b: &[Option<f64>]) -> Vec<bool> {
    let first = (0..a.len()).find(|&i| a[i].is_some() && b[i].is_some()).unwrap_or(a.len());
    let v = |z: Option<f64>| z.unwrap_or(NULL);
    (0..a.len()).map(|i| i > first && v(a[i]) > v(b[i]) && v(a[i - 1]) <= v(b[i - 1])).collect()
}

/// LongEntry before the position loop.
pub(crate) fn long_entry(x: &Ctx, p: &dyn Fn(&str) -> f64) -> Vec<bool> {
    let n = x.c.len();
    let gi = |k: &str| p(k).round() as i64;
    let gu = |k: &str| p(k).round().max(1.0) as usize;
    let (line, bear, close_m) = st_on(x, gi("entry_tf"), gu("entry_period"), p("entry_factor"), gi("entry_avg"));
    let cr = cross_up(&x.c, &line);
    let monthly: Vec<bool> = (0..n).map(|i| cr[i] && bear[i] == Some(true)).collect();
    let month = |i: usize| x.r[i].date.get(5..7).and_then(|s| s.parse::<i64>().ok()).unwrap_or(0);
    let final_sig: Vec<bool> = match gi("entry_type") {
        1 => {
            let mut flag = false;
            (0..n)
                .map(|i| {
                    if monthly[i] {
                        flag = true;
                    }
                    let eom = i + 1 < n && month(i) != month(i + 1);
                    if eom && flag {
                        flag = false;
                        true
                    } else {
                        false
                    }
                })
                .collect()
        }
        2 => cross_opt(&close_m, &line),
        _ => monthly,
    };
    // LongEntry = DeadstockBuySig AND Turnover AND NOT UC AND Close > PennyFlt AND the entry signal. Turnover on reads
    // OpenInt > Volsig: an index series has no open interest (0 in AmiBroker), so the filter allows no entry.
    let (penny, dead, turnover) = (p("penny"), p("dead_stock").round() == 1.0, p("turnover").round() == 1.0);
    (0..n).map(|i| (!dead || i + 1 != n) && !turnover && x.h[i] != x.c[i] && x.c[i] > penny && final_sig[i]).collect()
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    let schema = params(id)?;
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).map_or(0.0, |d| d.def));
    let n = x.c.len();
    if n == 0 || from > to || to >= n {
        return Some(Vec::new());
    }
    let gi = |k: &str| g(k).round() as i64;
    let gu = |k: &str| g(k).round().max(1.0) as usize;
    let entry = long_entry(x, &g);
    let (l1, _, c1) = st_on(x, gi("st1_tf"), gu("st1_period"), g("st1_factor"), gi("st1_avg"));
    let (l2, _, c2) = st_on(x, gi("st2_tf"), gu("st2_period"), g("st2_factor"), gi("st2_avg"));
    let v = |z: Option<f64>| z.unwrap_or(NULL);
    let (hsl_pct, pb_pct, live, buf) = (g("hsl_pct"), g("pb_pct"), gi("exit_live") == 1, g("exit_buffer"));
    let (exit_bars, swing, st_on_exit) = (gi("exit_bars"), g("pct_swing"), gi("st_exit") == 1);
    // the older version's two exit rules (SNIPER.afl itself: AND rule, highest close)
    let rule = if id == SNIPER_OLD { gi("st_exit_rule") } else { 0 };
    let swing_high = id == SNIPER_OLD && gi("swing_from") == 1;
    let (o, h, l, c) = (&x.o, &x.h, &x.l, &x.c);
    let mut out = Vec::new();
    // position: entry bar, HSL, PB, bar counter, ST exit flags, trailing high close
    let mut pos: Option<usize> = None;
    let (mut hsl, mut pb): (Option<f64>, Option<f64>) = (None, None);
    let (mut counter, mut st_exit, mut flag52, mut trail) = (0i64, false, false, 0.0f64);
    for i in from.max(1)..=to {
        if let Some(e) = pos {
            let sl_hit = hsl_pct > 0.0 && (if live { l[i] } else { c[i] }) < v(hsl);
            let pb_hit = pb_pct > 0.0 && (if live { h[i] } else { c[i] }) > v(pb);
            counter += 1;
            let (below1, below2) = (v(c1[i]) < v(l1[i]), v(c2[i]) < v(l2[i]));
            // rule 2: the close crossing below ST1 on this bar (it was not below on the previous bar)
            let cross1 = below1 && !(v(c1[i - 1]) < v(l1[i - 1]));
            let fire = match rule { 1 => below1 || below2, 2 => cross1 || below2, _ => below1 && below2 };
            if !flag52 && fire {
                st_exit = true;
            }
            if v(c2[i]) > v(l2[i]) && !flag52 {
                flag52 = true;
            }
            if v(c2[i]) < v(l2[i]) && flag52 {
                st_exit = true;
            }
            let top = if swing_high { h[i] } else { c[i] };
            if top > trail {
                trail = top;
            }
            let swing_hit = swing > 0.0 && c[i] < trail - trail * 0.01 * swing;
            let bars_hit = counter == exit_bars;
            let last = i + 1 == n;
            if pb_hit || sl_hit || last || bars_hit || (st_exit && st_on_exit) || swing_hit {
                let mut px = c[i];
                if pb_hit && live {
                    let lv = v(pb);
                    px = if o[i] > lv { l[i].max(o[i] * (1.0 - buf * 0.01)) } else { lv };
                }
                if sl_hit && live {
                    let lv = v(hsl);
                    px = if o[i] < lv { l[i].max(o[i] * (1.0 - buf * 0.01)) } else { lv };
                }
                let reason = if sl_hit { "Hard stop" } else if pb_hit { "Profit booking" } else if st_exit && st_on_exit { "ST exit" } else if swing_hit { "% swing exit" } else if bars_hit { "Bars exit" } else { "" };
                if reason.is_empty() {
                    // only the AFL's last-bar exit: the position is still open at the end of the data
                    out.push(trade(1.0, e, c[e], i, c[i], "open", true));
                } else {
                    out.push(trade(1.0, e, c[e], i, px, reason, false));
                }
                pos = None;
                hsl = None;
                pb = None;
                trail = 0.0;
                st_exit = false;
                flag52 = false;
            }
        }
        if pos.is_none() && entry[i] {
            pos = Some(i);
            hsl = if hsl_pct > 0.0 { Some(c[i] * (1.0 - hsl_pct * 0.01)) } else { None };
            pb = if pb_pct > 0.0 { Some(c[i] * (1.0 + pb_pct * 0.01)) } else { None };
            counter = 0;
            st_exit = false;
            trail = c[i]; // both rules start from the entry close; the older one then takes each later bar's high
        }
    }
    if let Some(e) = pos {
        out.push(trade(1.0, e, c[e], to, c[to], "open", true));
    }
    Some(out)
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    match id {
        SNIPER => Some((SNIPER, "Sniper (Monthly ST Cross)", "Sniper System")),
        SNIPER_OLD => Some((SNIPER_OLD, "Sniper (Older Version, Rebuilt)", "Sniper System")),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;

    fn weekdays(start: (i64, i64, i64), count: usize) -> Vec<String> {
        let (mut y, mut m, mut d) = start;
        let mut out = Vec::new();
        while out.len() < count {
            let s = format!("{y:04}-{m:02}-{d:02}");
            let wd = crate::books::ytmr::day_number(&s).map_or(-1, |k| (k + 4).rem_euclid(7));
            if wd != 0 && wd != 6 {
                out.push(s);
            }
            let dim = match m { 2 => if y % 4 == 0 { 29 } else { 28 }, 4 | 6 | 9 | 11 => 30, _ => 31 };
            d += 1;
            if d > dim {
                d = 1;
                m += 1;
                if m > 12 {
                    m = 1;
                    y += 1;
                }
            }
        }
        out
    }
    fn rows(dates: &[String], closes: &[f64]) -> Vec<Row> {
        dates.iter().zip(closes).map(|(d, &c)| Row { ticker: "T".into(), date: d.clone(), open: c, high: c * 1.01, low: c * 0.99, close: c, volume: None }).collect()
    }
    fn params_of(kv: &[(&str, f64)]) -> Params {
        kv.iter().map(|(k, v)| (k.to_string(), *v)).collect()
    }

    #[test]
    fn st_loop_starts_at_period_and_ratchets() {
        let c = [10.0, 10.0, 10.0, 10.0, 12.0, 13.0, 9.0, 8.0];
        let h: Vec<f64> = c.iter().map(|v| v + 0.5).collect();
        let l: Vec<f64> = c.iter().map(|v| v - 0.5).collect();
        let (line, bear) = sniper_st(&c, &h, &l, &c, 2, 1.0, 0);
        // ATR Null on bars 0-1; the lower band only rises in the up trend; the close of 9 breaks it at bar 6, then
        // the upper band only falls
        assert_eq!(line, vec![None, None, Some(9.0), Some(9.0), Some(10.25), Some(11.0), Some(12.0), Some(11.0)]);
        assert_eq!(bear, vec![None, None, Some(false), Some(false), Some(false), Some(false), Some(true), Some(true)]);
    }

    #[test]
    fn monthly_line_seen_from_previous_month_until_month_end() {
        let d = weekdays((2024, 1, 1), 70);
        let closes: Vec<f64> = (0..70).map(|k| 100.0 + k as f64).collect();
        let r = rows(&d, &closes);
        let x = Ctx::new(&r, &r, &[]);
        let (_, _, cm) = st_on(&x, 2, 1, 1.0, 5);
        let jan_end = d.iter().position(|s| s == "2024-01-31").unwrap();
        // January days (before its last) see no completed month; Jan 31 sees January's close; Feb days see it too
        assert_eq!(cm[0], None);
        assert_eq!(cm[jan_end - 1], None);
        assert_eq!(cm[jan_end], Some(closes[jan_end]));
        assert_eq!(cm[jan_end + 1], Some(closes[jan_end]));
        assert_eq!(cm[69], None); // the last bar of the data
    }

    /// Weekly entry ST: 150 falling bars, a 10% jump on a Wednesday (through the previous week's ST), 100 rising, 12 falling.
    fn swing_series() -> Vec<f64> {
        let mut c = vec![200.0];
        for k in 1..262 {
            let f = if k < 150 { 0.997 } else if k == 150 { 1.10 } else if k < 250 { 1.006 } else { 0.985 };
            c.push(c[k - 1] * f);
        }
        c
    }

    #[test]
    fn daily_st_entry_never_fires_as_written() {
        // on the daily time frame Cross(C, ST) and C < ST cannot hold on one bar
        let c = swing_series();
        let r = rows(&weekdays((2020, 1, 1), c.len()), &c);
        let x = Ctx::new(&r, &r, &[]);
        assert!(run(SNIPER, &x, 0, c.len() - 1, &params_of(&[("entry_tf", 0.0)])).unwrap().is_empty());
    }

    #[test]
    fn weekly_cross_entry_and_swing_exit() {
        let c = swing_series();
        let r = rows(&weekdays((2020, 1, 1), c.len()), &c);
        let x = Ctx::new(&r, &r, &[]);
        let t = run(SNIPER, &x, 0, c.len() - 1, &params_of(&[("entry_tf", 1.0), ("pct_swing", 5.0)])).unwrap();
        assert!(!t.is_empty());
        let e = t[0].legs[0].i;
        assert_eq!(e, 150);
        let (line, bear, _) = st_on(&x, 1, 5, 2.0, 5);
        assert!(c[e] > line[e].unwrap() && c[e - 1] <= line[e - 1].unwrap_or(NULL) && bear[e] == Some(true));
        // exit: first close more than 5% below the highest close since entry
        let mut top = c[e];
        let want = (e + 1..c.len()).find(|&i| { top = top.max(c[i]); c[i] < top * 0.95 }).unwrap();
        assert_eq!((t[0].exit_i, t[0].reason.as_str(), t[0].exit_px), (want, "% swing exit", c[want]));
    }

    #[test]
    fn turnover_filter_blocks_entries_and_settings_follow_the_afl_sections() {
        let c = swing_series();
        let r = rows(&weekdays((2020, 1, 1), c.len()), &c);
        let x = Ctx::new(&r, &r, &[]);
        let on = |kv: &[(&str, f64)]| run(SNIPER, &x, 0, c.len() - 1, &params_of(kv)).unwrap();
        assert!(!on(&[("entry_tf", 1.0)]).is_empty());
        assert!(on(&[("entry_tf", 1.0), ("turnover", 1.0)]).is_empty()); // OpenInt is 0 on an index
        let ps = params(SNIPER).unwrap();
        let section = |k: &str| ps.iter().find(|p| p.key == k).unwrap().label.split(" » ").next().unwrap().to_string();
        assert_eq!((section("entry_avg"), section("st1_avg"), section("st2_avg")), ("ST entry parameters".into(), "ST1 exit parameters".into(), "ST2 exit parameters".into()));
        assert_eq!(&ps.iter().map(|p| p.key).collect::<Vec<_>>()[..3], ["penny", "turnover", "dead_stock"]);
    }

    #[test]
    fn hard_stop_live_fills_at_gap_open() {
        let c = swing_series();
        let d = weekdays((2020, 1, 1), c.len());
        let mut r = rows(&d, &c);
        let p = params_of(&[("entry_tf", 1.0)]);
        let e = { let x = Ctx::new(&r, &r, &[]); run(SNIPER, &x, 0, c.len() - 1, &p).unwrap()[0].legs[0].i };
        // the next bar opens 10% below the entry close and its low is lower still: a live 5% stop fills at the open
        let ce = c[e];
        r[e + 1].open = ce * 0.90;
        r[e + 1].low = ce * 0.89;
        let x = Ctx::new(&r, &r, &[]);
        let t = run(SNIPER, &x, 0, c.len() - 1, &params_of(&[("entry_tf", 1.0), ("hsl_pct", 5.0), ("exit_live", 1.0)])).unwrap();
        assert_eq!(t[0].legs[0].i, e);
        assert_eq!((t[0].exit_i, t[0].reason.as_str()), (e + 1, "Hard stop"));
        assert!((t[0].exit_px - ce * 0.90).abs() < 1e-9);
        // stop tested on the close instead: the close is above the stop, no exit on that bar
        let t = run(SNIPER, &x, 0, c.len() - 1, &params_of(&[("entry_tf", 1.0), ("hsl_pct", 5.0)])).unwrap();
        assert_ne!(t[0].exit_i, e + 1);
    }

    #[test]
    fn defaults_hold_to_the_end() {
        let closes: Vec<f64> = (0..500).map(|k| 100.0 + 25.0 * ((k as f64) / 40.0).sin() + k as f64 * 0.05).collect();
        let d = weekdays((2015, 1, 1), 500);
        let r = rows(&d, &closes);
        let x = Ctx::new(&r, &r, &[]);
        let t = run(SNIPER, &x, 0, 499, &Params::new()).unwrap();
        // no exits on by default: at most one trade, open at the end
        assert!(t.len() <= 1);
        assert!(t.iter().all(|t| t.open && t.exit_i == 499));
    }

    /// `kv` plus SNIPER.afl's values for the keys whose defaults the older version changes (unless `kv` sets them).
    fn afl_defaults<'a>(kv: &[(&'a str, f64)]) -> Vec<(&'a str, f64)> {
        let afl = params(SNIPER).unwrap();
        let mut out: Vec<(&str, f64)> = OLD_DEFAULTS
            .iter()
            .filter(|(k, _)| !kv.iter().any(|(q, _)| q == k))
            .map(|(k, _)| (*k, afl.iter().find(|d| d.key == *k).unwrap().def))
            .collect();
        out.extend_from_slice(kv);
        out
    }

    #[test]
    fn older_version_defaults_are_its_sheet_settings() {
        let (cur, old) = (params(SNIPER).unwrap(), params(SNIPER_OLD).unwrap());
        let def = |ps: &Vec<ParamDef>, k: &str| ps.iter().find(|d| d.key == k).unwrap().def;
        for (k, v) in OLD_DEFAULTS {
            assert_eq!(def(&old, k), v, "{k}");
            assert_ne!(def(&cur, k), v, "{k} already SNIPER.afl's default");
        }
        assert_eq!(def(&cur, "st_exit"), 0.0); // SNIPER.afl itself is unchanged
    }

    #[test]
    fn older_version_with_both_rules_off_is_sniper_afl() {
        let c = swing_series();
        let r = rows(&weekdays((2020, 1, 1), c.len()), &c);
        let x = Ctx::new(&r, &r, &[]);
        for kv in [vec![("entry_tf", 1.0), ("pct_swing", 5.0)], vec![("entry_tf", 1.0), ("st_exit", 1.0), ("st1_factor", 1.0), ("st2_factor", 3.0)]] {
            let cur = run(SNIPER, &x, 0, c.len() - 1, &params_of(&kv)).unwrap();
            let mut off = afl_defaults(&kv);
            off.extend([("st_exit_rule", 0.0), ("swing_from", 0.0)]);
            let old = run(SNIPER_OLD, &x, 0, c.len() - 1, &params_of(&off)).unwrap();
            let key = |t: &Vec<Trade>| t.iter().map(|t| (t.legs[0].i, t.exit_i, t.exit_px.to_bits())).collect::<Vec<_>>();
            assert_eq!(key(&cur), key(&old), "{kv:?}");
        }
    }

    #[test]
    fn older_rules_exit_no_later_than_sniper_afl() {
        // highs 1% above the close: a trail from the highest high is higher, so its swing exit comes no later; the older
        // ST rule fires whenever the close is below ST2 (and on a new ST1 break), so it comes no later than the AND rule
        let c = swing_series();
        let r = rows(&weekdays((2020, 1, 1), c.len()), &c);
        let x = Ctx::new(&r, &r, &[]);
        for kv in [vec![("entry_tf", 1.0), ("pct_swing", 5.0)], vec![("entry_tf", 1.0), ("st_exit", 1.0), ("st1_factor", 1.0), ("st2_factor", 3.0)]] {
            let cur = run(SNIPER, &x, 0, c.len() - 1, &params_of(&kv)).unwrap();
            let old = run(SNIPER_OLD, &x, 0, c.len() - 1, &params_of(&afl_defaults(&kv))).unwrap();
            assert_eq!(cur[0].legs[0].i, old[0].legs[0].i);
            assert!(old[0].exit_i <= cur[0].exit_i, "{kv:?}");
        }
        let swing = |id| run(id, &x, 0, c.len() - 1, &params_of(&afl_defaults(&[("entry_tf", 1.0), ("pct_swing", 5.0)]))).unwrap()[0].exit_i;
        assert!(swing(SNIPER_OLD) < swing(SNIPER)); // 1% highs make the older trail fire earlier here
    }

    #[test]
    fn every_strategy_runs() {
        let closes: Vec<f64> = (0..600).map(|k| 100.0 + 25.0 * ((k as f64) / 40.0).sin() + k as f64 * 0.05).collect();
        let d = weekdays((2015, 1, 1), 600);
        let r = rows(&d, &closes);
        let x = Ctx::new(&r, &r, &[]);
        for id in IDS {
            assert!(family_of(id).is_some());
            for kv in [
                vec![],
                vec![("entry_type", 1.0), ("pct_swing", 5.0)],
                vec![("entry_type", 2.0), ("st_exit", 1.0)],
                vec![("entry_tf", 1.0), ("hsl_pct", 5.0), ("pb_pct", 10.0), ("exit_live", 1.0), ("exit_bars", 20.0)],
                vec![("entry_tf", 0.0), ("st1_tf", 1.0), ("st2_tf", 2.0), ("st_exit", 1.0)],
            ] {
                let t = run(id, &x, 100, 599, &params_of(&kv)).unwrap();
                assert!(t.iter().all(|t| t.exit_i >= t.legs[0].i && t.legs[0].i >= 100), "{id} {kv:?}");
                assert!(t.iter().filter(|t| t.open).all(|t| t.exit_i == 599));
            }
        }
        assert_eq!(catalogue().len(), IDS.len());
    }
}
