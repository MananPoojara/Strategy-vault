//! Alexander Elder, "Come Into My Trading Room" (Wiley 2002). Chapter 6 "Trading" (printed pp.123-183)
//! and Chapter 5 Force Index (pp.109-114). Live is a legacy catalogue registration,
//! not source admission. The protected Impulse candidate has its own account
//! executor/review; other incomplete fragments remain held by readiness/policy.
//! Source review: library/elcim/REVIEW-2026-10-15.md. Shared helpers (EMA seeded with an
//! SMA as in Trading for a Living p.123, MACD-Histogram, Force Index, completed-week series, the day-bar order
//! simulator) are also used by books/eltfl.rs.
use crate::connors::{Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "elcim";

// ------------------------------------------------------------------ indicators

/// EMA with K = 2/(N+1), started from the simple average of the first N values (TFL p.123 worksheet steps 3-4).
/// A missing input (NaN) blanks the output and restarts the seeding.
pub fn ema(v: &[f64], n: usize) -> Vec<f64> {
    let mut out = vec![f64::NAN; v.len()];
    let k = 2.0 / (n as f64 + 1.0);
    let (mut run, mut sum, mut prev) = (0usize, 0.0, f64::NAN);
    for i in 0..v.len() {
        if !v[i].is_finite() {
            run = 0;
            sum = 0.0;
            prev = f64::NAN;
            continue;
        }
        run += 1;
        if run < n {
            sum += v[i];
        } else if run == n {
            prev = (sum + v[i]) / n as f64;
            out[i] = prev;
        } else {
            prev = v[i] * k + prev * (1.0 - k);
            out[i] = prev;
        }
    }
    out
}

/// MACD-Histogram = (EMA fast - EMA slow) - EMA(signal) of that difference (TFL p.128-129).
pub fn macd_hist(c: &[f64], fast: usize, slow: usize, sig: usize) -> Vec<f64> {
    let (f, s) = (ema(c, fast), ema(c, slow));
    let line: Vec<f64> = f.iter().zip(&s).map(|(a, b)| a - b).collect();
    let signal = ema(&line, sig);
    line.iter().zip(&signal).map(|(a, b)| a - b).collect()
}

/// Raw Force Index = volume today x (close today - close yesterday); NaN without volume.
pub fn force(x: &Ctx) -> Vec<f64> {
    (0..x.c.len())
        .map(|i| match x.r[i].volume {
            Some(v) if i > 0 && v > 0.0 => v * (x.c[i] - x.c[i - 1]),
            _ => f64::NAN,
        })
        .collect()
}

pub fn rising(v: &[f64], i: usize) -> bool {
    i > 0 && v[i] > v[i - 1]
}
pub fn falling(v: &[f64], i: usize) -> bool {
    i > 0 && v[i] < v[i - 1]
}

/// Smallest price step: one tick (TFL p.241-242 "one tick above the high"). US cash indices/ETFs 0.01, NSE 0.05.
pub fn tick(x: &Ctx) -> f64 {
    let t = x.r.first().map_or("", |r| r.ticker.as_str());
    if ["SPX", "NDX", "DJI", "SPY", "QQQ", "IWM", "MDY", "SMH"].contains(&t) { 0.01 } else { 0.05 }
}

fn day_number(d: &str) -> i64 {
    // days since 1970-01-01 for "YYYY-MM-DD" (civil calendar)
    let y: i64 = d[0..4].parse().unwrap_or(1970);
    let m: i64 = d[5..7].parse().unwrap_or(1);
    let dd: i64 = d[8..10].parse().unwrap_or(1);
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let mp = (m + 9) % 12;
    let doy = (153 * mp + 2) / 5 + dd - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146097 + doe - 719468
}

/// Weekly bars (Monday-Sunday calendar weeks) built from the daily bars. `close[k]` = last daily close of week k;
/// `at[i]` = the last week COMPLETED at the close of day i (day i is the last bar of its week, i.e. the next bar
/// falls in another week, or day i is the last bar of the data), else the previous week. No partial weeks are used.
pub struct Weekly {
    pub close: Vec<f64>,
    pub at: Vec<Option<usize>>,
}
pub fn weekly(x: &Ctx) -> Weekly {
    let wk: Vec<i64> = x.r.iter().map(|r| (day_number(&r.date) + 3).div_euclid(7)).collect();
    let n = wk.len();
    let (mut close, mut at) = (Vec::new(), vec![None; n]);
    for i in 0..n {
        let last = i + 1 == n || wk[i + 1] != wk[i];
        if last {
            close.push(x.c[i]);
            at[i] = Some(close.len() - 1);
        } else {
            at[i] = close.len().checked_sub(1);
        }
    }
    Weekly { close, at }
}
impl Weekly {
    /// +1 if the weekly series rose between the two latest completed weeks, -1 if it fell, 0 otherwise / unknown.
    pub fn slope(&self, s: &[f64], i: usize) -> i8 {
        match self.at[i] {
            Some(k) if k > 0 && s[k] > s[k - 1] => 1,
            Some(k) if k > 0 && s[k] < s[k - 1] => -1,
            _ => 0,
        }
    }
}

// ------------------------------------------------------------------ day-bar simulator

/// How an entry is placed after a setup on the close of bar i.
#[derive(Clone, Copy, PartialEq)]
pub enum Entry {
    /// Buy-stop at high[i] + off (long) / sell-stop at low[i] - off (short), for bar i+1 only; while `keep`
    /// holds it is re-placed at the latest bar's high/low (TFL "trailing buy-stop", CIMTR "good for one day").
    Stop(f64),
    /// Market order at the next bar's open.
    NextOpen,
    /// At the signal bar's close.
    Close,
}
#[derive(Clone, Copy, PartialEq)]
pub enum Protect {
    None,
    /// One tick (`f64`) below the lower low of the entry bar and the bar before it (TFL p.231, p.242). Fixed.
    TwoBar(f64),
    /// SafeZone (CIMTR p.173-178) with coefficient and lookback; ratcheted, never loosened.
    SafeZone(f64, usize),
}
pub struct Rules<'a> {
    pub sides: &'a [f64],
    pub trig: &'a dyn Fn(usize, f64) -> bool,
    pub keep: &'a dyn Fn(usize, f64) -> bool,
    pub exit: &'a dyn Fn(usize, f64) -> bool,
    pub entry: Entry,
    pub exit_next_open: bool,
    pub protect: Protect,
    pub exit_reason: &'a str,
}

/// SafeZone stop levels for each bar (CIMTR Table 6.1 / p.176-178): for longs K[j] = max(J[j-2..=j]) with
/// J[j] = low[j-1] - coef * AvgDownPen(lookback ending j-1); for shorts the mirror with highs (min of 3).
pub fn safezone(x: &Ctx, coef: f64, n: usize) -> (Vec<f64>, Vec<f64>) {
    let len = x.c.len();
    let (mut j_long, mut j_short) = (vec![f64::NAN; len], vec![f64::NAN; len]);
    for j in 1..len {
        let y = j - 1; // yesterday
        if y < n {
            continue;
        }
        let (mut dn, mut dc, mut up, mut uc) = (0.0, 0, 0.0, 0);
        for t in y + 1 - n..=y {
            if x.l[t] < x.l[t - 1] {
                dn += x.l[t - 1] - x.l[t];
                dc += 1;
            }
            if x.h[t] > x.h[t - 1] {
                up += x.h[t] - x.h[t - 1];
                uc += 1;
            }
        }
        if dc > 0 {
            j_long[j] = x.l[y] - coef * dn / dc as f64;
        }
        if uc > 0 {
            j_short[j] = x.h[y] + coef * up / uc as f64;
        }
    }
    let k_long = (0..len).map(|j| if j < 2 { f64::NAN } else { j_long[j - 2..=j].iter().copied().fold(f64::NAN, f64::max) }).collect();
    let k_short = (0..len).map(|j| if j < 2 { f64::NAN } else { j_short[j - 2..=j].iter().copied().fold(f64::NAN, f64::min) }).collect();
    (k_long, k_short)
}

struct Pos {
    t: Trade,
    stop: f64,
}

pub fn simulate(x: &Ctx, from: usize, to: usize, r: &Rules) -> Vec<Trade> {
    let (o, h, l, c) = (&x.o, &x.h, &x.l, &x.c);
    let sz = match r.protect {
        Protect::SafeZone(k, n) => Some(safezone(x, k, n)),
        _ => None,
    };
    let mut out = Vec::new();
    let mut pos: Option<Pos> = None;
    let mut pend: Option<(f64, f64)> = None; // (side, stop price) for this bar
    let mut open_entry: Option<(f64, usize)> = None; // (side, signal bar)
    let mut exit_open: Option<usize> = None; // signal bar of a next-open exit
    let close_trade = |p: Pos, i: usize, px: f64, reason: String, sig: usize, out: &mut Vec<Trade>| {
        let mut t = p.t;
        t.exit_i = i;
        t.exit_px = px;
        t.reason = reason;
        t.signal_i = sig;
        out.push(t);
    };
    let new = |d: f64, i: usize, px: f64, kind: &'static str, sig: usize| Pos {
        t: Trade { dir: d, legs: vec![Leg { i, px, w: 1.0 }], exit_i: i, exit_px: px, reason: String::new(), entry_kind: kind, signal_i: sig, open: false, exits: vec![] },
        stop: f64::NAN,
    };
    for j in from..=to {
        // 1. next-open exit decided on the previous close
        if let Some(sig) = exit_open.take() {
            if let Some(p) = pos.take() {
                close_trade(p, j, o[j], format!("{} · next open", r.exit_reason), sig, &mut out);
            }
        }
        // 2. pending entries for this bar
        let mut entered = false;
        if let Some((d, sig)) = open_entry.take() {
            if pos.is_none() {
                pos = Some(new(d, j, o[j], "open", sig));
                entered = true;
            }
        }
        if let Some((d, px)) = pend.take() {
            if pos.is_none() {
                let fill = if d > 0.0 {
                    if o[j] >= px { Some(o[j]) } else if h[j] >= px { Some(px) } else { None }
                } else if o[j] <= px { Some(o[j]) } else if l[j] <= px { Some(px) } else { None };
                match fill {
                    Some(f) => {
                        pos = Some(new(d, j, f, "stop", j.saturating_sub(1)));
                        entered = true;
                    }
                    None => pend = Some((d, px)), // unfilled: may be re-placed at the close
                }
            }
        }
        // 3. protective stop (placed after the entry bar; active from the next bar)
        if let Some(p) = pos.as_ref() {
            if !entered && p.stop.is_finite() {
                let d = p.t.dir;
                let hit = if d > 0.0 { l[j] <= p.stop } else { h[j] >= p.stop };
                if hit {
                    let px = if d > 0.0 { o[j].min(p.stop) } else { o[j].max(p.stop) };
                    let p = pos.take().unwrap();
                    close_trade(p, j, px, "protective stop".into(), j, &mut out);
                }
            }
        }
        // 4. close of bar j: exits, stop updates, new orders
        let mut exited_at_close = false;
        if let Some(p) = pos.as_mut() {
            let d = p.t.dir;
            let e = p.t.legs[0].i;
            match r.protect {
                Protect::TwoBar(tk) if j == e && e > 0 => {
                    p.stop = if d > 0.0 { l[e].min(l[e - 1]) - tk } else { h[e].max(h[e - 1]) + tk };
                }
                Protect::SafeZone(..) if j + 1 < c.len() => {
                    let (kl, ks) = sz.as_ref().unwrap();
                    let lvl = if d > 0.0 { kl[j + 1] } else { ks[j + 1] }; // tomorrow's stop, from today's bar
                    if lvl.is_finite() {
                        p.stop = if !p.stop.is_finite() { lvl } else if d > 0.0 { p.stop.max(lvl) } else { p.stop.min(lvl) };
                    }
                }
                _ => {}
            }
            if (r.exit)(j, d) {
                if r.exit_next_open {
                    if j < to {
                        exit_open = Some(j);
                    }
                } else {
                    let p = pos.take().unwrap();
                    close_trade(p, j, c[j], format!("{} · close", r.exit_reason), j, &mut out);
                    exited_at_close = true;
                }
            }
        }
        if pos.is_none() && !exited_at_close && j < to {
            if let Some((d, _)) = pend.take() {
                if (r.keep)(j, d) {
                    if let Entry::Stop(off) = r.entry {
                        pend = Some((d, if d > 0.0 { h[j] + off } else { l[j] - off }));
                    }
                    continue;
                }
            }
            for &d in r.sides {
                if (r.trig)(j, d) {
                    match r.entry {
                        Entry::Stop(off) => pend = Some((d, if d > 0.0 { h[j] + off } else { l[j] - off })),
                        Entry::NextOpen => open_entry = Some((d, j)),
                        Entry::Close => pos = Some(new(d, j, c[j], "close", j)),
                    }
                    break;
                }
            }
        } else if pos.is_some() && entered {
            pend = None;
        }
    }
    if let Some(p) = pos {
        let mut t = p.t;
        t.exit_i = to;
        t.exit_px = c[to];
        t.signal_i = to;
        t.reason = "open".into();
        t.open = true;
        out.push(t);
    }
    out
}

// ------------------------------------------------------------------ catalogue

const IMPULSE: &[&str] = &[
    "F|Source pp.158-161: completed weekly EMA26 direction filters daily long/short entries. SMA-seeded daily EMA13 and MACD12-26-9; both slopes must agree. Choose the source-offered EMA weekly filter, not a result-tuned alternative.",
    "E|Source p.161 offers next-open orders after evening signals. Enter one position at the next actual observed open from a flat prior close; no automatic reversal. Only prior calendar weeks are completed at that execution clock, including actual NSE weekend sessions.",
    "X|Source pp.159-161 and Study Guide answer55: exit at the next actual open after either daily Impulse component stops agreeing. A weekly change alone is not an invented daily exit.",
    "X|Mandatory protection pp.171-180: SafeZone10-session penetration mean, coefficient2, protected3-level maximum/minimum and a never-loosen held stop. Source p.179 discretionary turning-point resets are not supplied as an objective rule; this composition fixes the10-session worksheet baseline. Protection is known before open and active on the entry day. A gap fills at the actual open; otherwise a touch fills at the known level.",
    "F|Mandatory account risk pp.220-227 and Study Guide answers77-80: at most2% monthly-reference equity risk per trade including costs; current-month realized losses plus outstanding risk cannot exceed6%. Profits do not replenish the monthly allowance. New-month references/stops recalibrate from actual previous marks.",
    "X|Declared research clock: closing equity below94% of the prior month-end reference halts entries for that month and queues existing-position liquidation at the next actual open. Gaps may exceed planned limits; no loss clipping or funding reset.",
];
const TS_FI: &[&str] = &[
    "F|Screen one: \"I now prefer to use the slope of a weekly exponential moving average ... When the weekly EMA rises, it confirms a bull move and tells us to go long or stand aside. When it falls ... go short or stand aside. I use a 26-week EMA\" (p.132). Weekly bars are completed calendar weeks (book's test procedure, p.126).",
    "F|Screen two, active traders: \"use the two-day EMA of Force Index ... When the weekly trend is up and daily Force Index falls below zero, it flags a buying opportunity ... When the weekly trend is down and the two-day EMA of Force Index rallies above zero, it points to shorting opportunities.\" (p.135)",
    "E|Screen three: \"When the first two screens give you a buy signal (the weekly is up, but the daily is down), place a buy order at the high of the previous day or a tick higher ... Place a buy order, good for one day only.\" Shorts: \"place a sell order at the previous day's low or a tick lower.\" (p.137). Default: at the high/low (ticks = 0); a tick beyond = option. The order is re-placed each day the two screens still agree.",
    "X|\"If you buy in an uptrend when the two-day EMA of Force Index turns negative, sell when it turns positive. If you go short in a downtrend after the two-day EMA of Force Index turns positive, cover when it turns negative.\" (p.136) Executed the next day at the open (p.112: \"sell the day after Force Index turns positive\").",
    "X|Held: global source p.171 requires protective stops. The legacy optional-stop runner does not establish complete2%/6% risk control or daily stop-entry/protection ordering. No-stop is not a valid book default.",
];
const FI2: &[&str] = &[
    "F|\"when the 22-day EMA of price is up and the two-day EMA of Force Index becomes negative, it reveals a short-term splash of bearishness within an uptrend, a buying opportunity.\" (p.110)",
    "F|\"When the 22-day EMA of prices is down, and the two-day EMA of Force Index rallies above zero, it reveals a short-term splash of bullishness within a downtrend, a shorting opportunity.\" (p.112)",
    "E|[Not from the book] The book gives no entry order for this signal; read as a market order at the next open.",
    "X|\"If you're very short-term oriented, sell the day after Force Index turns positive\" / \"cover the day after Force Index turns negative\" (p.112). Executed at the next day's open.",
];

pub fn catalogue() -> Vec<StrategyDef> {
    let d = |id, name, group, dir, status, vol, rules, notes| StrategyDef {
        id, name, group, session: 0, dir, status, needs_volume: vol, needs_vix: false, needs_series: &[], book: BOOK, rules, notes, claim: None,
    };
    use Status::*;
    vec![
        d("elcim_impulse", "Impulse System (weekly trend + 13-day EMA & MACD-Histogram)", "Ch 6 · The Impulse System", "Long/Short", Live, false, IMPULSE,
          "Provisional protected daily Impulse composition on actual individual stocks; source pp.158-161,171-180,220-227. Source-offered next-open choice plus SafeZone10/2 and mandatory2%/6% account control. Normalized fractional cash-only exposure; shorts reserve principal, with no real borrow/lot or monetary capital claim. Flat selected start and actual recursive warmup; terminal holdings pay entry costs only. Gross uses identical accepted quantities/fills. Supplied adjustment basis and continuous stops remain qualified. This is not the old stop-free fragment or Elder's discretionary selection/chart results. Independent acceptance is required; library/elcim/REVIEW-2026-10-15.md."),
        d("elcim_ts_fi", "Triple Screen (2002 update) · 26-week EMA + 2-day Force Index", "Ch 6 · Triple Screen Update", "Long/Short", Live, true, TS_FI,
          "pp.128-138. Book prints no test results. Needs volume (Force Index): NIFTY 50 volume starts 2000. Buy stop fills at the open if the bar gaps through it, else at the stop. Profit-taking at channel lines and the 'weekly EMA turns flat' exit for long-term traders are not exact rules (channel coefficient is fitted to contain ~95% of prices) and are not modelled; this is the short-term exit the book states for the Force Index entry."),
        d("elcim_fi2", "Force Index 2-day EMA with 22-day EMA trend", "Ch 5 · Force Index", "Long/Short", Live, true, FI2,
          "pp.109-112. Book prints no test results. Entry order is not given by the book: next-open market order is a labelled necessary assumption. 'Becomes negative' = the 2-day EMA crosses from >= 0 to < 0 on that close. Spike/divergence exits and channel targets are discretionary and not modelled."),
        d("elcim_ts_conservative", "Triple Screen · conservative (daily MACD-Histogram / Stochastic)", "Ch 6 · Triple Screen Update", "Long/Short", NotTestable("no exact exit: profits are taken 'after the weekly EMA turns flat' or at a channel line whose width is fitted by eye; Stochastic parameters/reference lines not given"),
          false, &["E|\"When the weekly trend is up, look for daily MACD-Histogram to fall below zero and tick up, or for Stochastic to fall to its lower reference line, giving a buy signal.\" (p.135)", "X|\"Take your profits after the weekly EMA turns flat\" / \"sell when prices hit the upper channel line\" (p.136)"], "pp.135-136."),
        d("elcim_safezone", "SafeZone stop", "Ch 6 · Exiting Trades", "Long/Short", NotTestable("a stop only: the book gives no entry ('Once your system identifies an uptrend...'); available as a labelled stop option of elcim_ts_fi"),
          false, &["X|\"Multiply the Average Downside Penetration by a coefficient, starting with two ... Subtract the result from yesterday's low, and place your stop there.\" (p.174); Excel rules p.176-178, Table 6.1."], "pp.173-180. The Excel rules are coded (safezone()) and checked against Table 6.1 (IBM) in this module's tests."),
        d("elcim_chandelier", "Chandelier Exit (Chuck LeBeau)", "Ch 6 · Exiting Trades", "Long/Short", NotTestable("an exit only, presented from Chuck LeBeau; no entry rule"),
          false, &["X|\"Hhv(hi, 22) - 3 * ATR(22)\" (p.181)"], "pp.180-183."),
        d("elcim_thermometer", "Market Thermometer", "Ch 6 · Market Thermometer", "Long/Short", NotTestable("timing filter and profit target only; no entry rule of its own"),
          false, &["F|\"Temperature = the greater of either (High today - High yesterday) or (Low yesterday - Low today)\"; 22-day EMA (p.162-163)", "X|\"Exit positions when Market Thermometer rises to triple the height of its moving average.\" (p.163)"], "pp.162-165."),
        d("elcim_daytrade", "Day-trading with Triple Screen (25-/5-minute charts)", "Ch 6 · Day-Trading", "Long/Short", NotTestable("intraday and discretionary: entries 'in the vicinity of a moving average', exits 'in the vicinity of the channel line'"),
          false, &["E|\"If the 25-minute trend is up, buy pullbacks to the EMA on a five-minute chart\" (p.149)"], "pp.138-157."),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    Some(match id {
        "elcim_impulse" => ("elcim_impulse", "Elder Impulse System", "Weekly trend + daily EMA/MACD-H"),
        "elcim_ts_fi" => ("el_triple_screen", "Elder Triple Screen", "2002 update · Force Index"),
        "elcim_ts_conservative" => ("el_triple_screen", "Elder Triple Screen", "2002 update · conservative (not testable)"),
        "elcim_fi2" => ("el_force_index", "Elder Force Index", "22-day EMA trend (2002)"),
        "elcim_safezone" | "elcim_chandelier" | "elcim_thermometer" => ("elcim_tools", "Elder exits & tools", "Not testable alone"),
        "elcim_daytrade" => ("elcim_daytrade", "Elder day-trading", "Intraday"),
        _ => return None,
    })
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    match id {
        "elcim_impulse" => Some(vec![
            p("ema", "Daily EMA length (fixed source choice)", 13., 13., 13., 1.),
            p("fast", "MACD fast EMA (fixed)", 12., 12., 12., 1.),
            p("slow", "MACD slow EMA (fixed)", 26., 26., 26., 1.),
            p("sig", "MACD signal EMA (fixed)", 9., 9., 9., 1.),
            p("wema", "Weekly EMA length (fixed)", 26., 26., 26., 1.),
            p("wfilter", "Weekly filter: EMA26 slope (fixed0; alternatives held)", 0., 0., 0., 1.),
            p("next_open", "Next actual open (fixed1; same-close held)", 1., 1., 1., 1.),
        ]),
        "elcim_ts_fi" => Some(vec![
            p("wema", "Weekly EMA length", 26., 10., 52., 1.),
            p("fi", "Force Index EMA length", 2., 1., 13., 1.),
            p("ticks", "Ticks beyond the previous high/low (book: at it, or a tick beyond)", 0., 0., 1., 1.),
            p("safezone", "Legacy held stop option: 0 = off, 1 = on; source protection is mandatory", 0., 0., 1., 1.),
            p("sz_coef", "SafeZone coefficient", 2., 2., 3., 0.5),
            p("sz_len", "SafeZone lookback days", 10., 10., 20., 5.),
        ]),
        "elcim_fi2" => Some(vec![
            p("ema", "Trend EMA length", 22., 10., 40., 1.),
            p("fi", "Force Index EMA length", 2., 1., 13., 1.),
        ]),
        _ => None,
    }
}

pub fn validate(id: &str, p: &Params) -> Option<String> {
    if id==super::elcim_account::ID {return super::elcim_account::validate(p);}
    validate_with(&params(id)?, p)
}
pub fn validate_with(schema: &[ParamDef], p: &Params) -> Option<String> {
    for (k, &v) in p {
        let Some(d) = schema.iter().find(|d| d.key == k) else { return Some(format!("Unknown parameter: {k}")) };
        let steps = (v - d.min) / d.step;
        if !v.is_finite() || v < d.min || v > d.max || (steps - steps.round()).abs() > 1e-9 {
            return Some(format!("{k} must be from {} to {} in steps of {}", d.min, d.max, d.step));
        }
    }
    None
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    // Cost-aware API execution is dispatched before this generic book interface.
    // Direct zero-cost callers must receive the same protected account decisions.
    if id==super::elcim_account::ID {return super::elcim_account::run(x,from,to,0.,p).ok().map(|a|a.trades);}
    let schema = params(id)?;
    if validate(id, p).is_some() || from > to || to >= x.c.len() {
        return None;
    }
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).unwrap().def);
    let u = |k: &str| g(k) as usize;
    let both: &[f64] = &[1.0, -1.0];
    let never = |_: usize, _: f64| false;
    match id {
        "elcim_ts_fi" => {
            let wk = weekly(x);
            let we = ema(&wk.close, u("wema"));
            let fi = ema(&force(x), u("fi"));
            let setup = |i: usize, d: f64| wk.slope(&we, i) == d as i8 && (if d > 0.0 { fi[i] < 0.0 } else { fi[i] > 0.0 });
            let exit = |i: usize, d: f64| if d > 0.0 { fi[i] > 0.0 } else { fi[i] < 0.0 };
            let protect = if u("safezone") == 1 { Protect::SafeZone(g("sz_coef"), u("sz_len")) } else { Protect::None };
            Some(simulate(x, from, to, &Rules { sides: both, trig: &setup, keep: &setup, exit: &exit,
                entry: Entry::Stop(g("ticks") * tick(x)), exit_next_open: true, protect, exit_reason: "2-day Force Index crossed zero" }))
        }
        "elcim_fi2" => {
            let e = ema(&x.c, u("ema"));
            let fi = ema(&force(x), u("fi"));
            let trig = |i: usize, d: f64| i > 0 && if d > 0.0 { rising(&e, i) && fi[i] < 0.0 && fi[i - 1] >= 0.0 } else { falling(&e, i) && fi[i] > 0.0 && fi[i - 1] <= 0.0 };
            let exit = |i: usize, d: f64| if d > 0.0 { fi[i] > 0.0 } else { fi[i] < 0.0 };
            Some(simulate(x, from, to, &Rules { sides: both, trig: &trig, keep: &never, exit: &exit,
                entry: Entry::NextOpen, exit_next_open: true, protect: Protect::None, exit_reason: "2-day Force Index crossed zero" }))
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;
    fn row(d: &str, h: f64, l: f64, c: f64) -> Row {
        Row { ticker: "IBM".into(), date: d.into(), open: c, high: h, low: l, close: c, volume: Some(1.0) }
    }
    #[test]
    fn safezone_matches_book_table_6_1() {
        // CIMTR Table 6.1 (IBM 2001), columns J (stop) and K (protected stop), coefficient 2, 10-day lookback.
        let t = [("04-19", 115.90, 110.30, 114.47), ("04-20", 116.40, 113.75, 114.83), ("04-23", 114.05, 111.68, 112.00),
            ("04-24", 114.75, 112.28, 112.67), ("04-25", 114.85, 111.99, 114.85), ("04-26", 116.70, 113.68, 113.74),
            ("04-27", 116.90, 114.55, 116.20), ("04-30", 118.05, 114.72, 115.14), ("05-01", 118.65, 114.90, 118.51),
            ("05-02", 118.95, 113.74, 115.40), ("05-03", 115.10, 112.35, 113.70), ("05-04", 115.86, 111.20, 115.86),
            ("05-07", 117.25, 115.00, 115.90), ("05-08", 117.75, 115.50, 117.70), ("05-09", 118.18, 115.30, 116.98),
            ("05-10", 118.90, 115.20, 115.20), ("05-11", 114.15, 110.96, 111.81), ("05-14", 113.18, 111.00, 112.56),
            ("05-15", 114.15, 112.50, 113.58), ("05-16", 115.80, 112.20, 115.80)];
        let r: Vec<Row> = t.iter().map(|&(d, h, l, c)| row(&format!("2001-{d}"), h, l, c)).collect();
        let x = Ctx::new(&r, &r, &[]);
        let (kl, ks) = safezone(&x, 2.0, 10);
        let r2 = |v: f64| (v * 100.0).round() / 100.0;
        // book rows 16..22 = our bars 13..19 (protected stops)
        assert_eq!(r2(kl[13]), 113.01);
        assert_eq!(r2(kl[14]), 113.51);
        assert_eq!(r2(kl[15]), 113.51);
        assert_eq!(r2(kl[16]), 113.60);
        assert_eq!(r2(kl[17]), 113.60);
        assert_eq!(r2(kl[19]), 109.75);
        // short side (column R): rows 16-19 -> 116.45, 117.28, 118.82, 119.27
        assert_eq!(r2(ks[13]), 116.45);
        assert_eq!(r2(ks[14]), 117.28);
        assert_eq!(r2(ks[15]), 118.82);
        assert_eq!(r2(ks[16]), 119.27);
    }
    #[test]
    fn ema_is_sma_seeded_and_weeks_complete_on_their_last_bar() {
        let v = ema(&[1., 2., 3., 4.], 3);
        assert!(v[1].is_nan());
        assert_eq!(v[2], 2.0);
        assert_eq!(v[3], 3.0); // 4*0.5 + 2*0.5
        // Thu, Fri, Mon: Friday completes week 0, Monday is in week 1 (not complete unless last bar)
        let r = vec![row("2024-01-04", 1., 1., 10.), row("2024-01-05", 1., 1., 11.), row("2024-01-08", 1., 1., 12.), row("2024-01-09", 1., 1., 13.)];
        let x = Ctx::new(&r, &r, &[]);
        let w = weekly(&x);
        assert_eq!(w.close, vec![11., 13.]);
        assert_eq!(w.at, vec![None, Some(0), Some(0), Some(1)]);
    }
}
