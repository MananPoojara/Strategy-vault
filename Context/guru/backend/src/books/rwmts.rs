//! Richard L. Weissman, *Mechanical Trading Systems: Pairing Trader Psychology with Technical Analysis* (Wiley, 2004).
//! Source, page references and book-vs-ours checks: library/rwmts/README.md.
//!
//! Conventions (book p.42-43, CQG code with `[-1]` references): every indicator trigger is evaluated on the CLOSE of
//! bar t-1 and the order is filled at the OPEN of bar t ("set the indicator trigger to the close ... and the entry or
//! exit level to the opening price of the following day"). Exits written as a CQG "Price" field (an MA level, a
//! percentage of the entry price, a channel level) are price orders working during bar t: they fill at the level, or at
//! the open when the bar opens beyond it. When a stop and a profit target could both fill on one bar the trade is
//! assumed stopped out (book p.73-74). Book futures dollar amounts are converted only for the E-mini row the book itself
//! defines as "Cash S&P 500 Index x 50" (Table 3.1 note b): $2,500 = 50 index points. India = the same rules on the
//! Indian index (instrument swap only); the point amounts are then a labelled parameter, not a re-scaled threshold.
use crate::connors::{dmi, rsi, sma, true_range, wilder, Claim, Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "rwmts";
/// Admitted ids (authors/weissman/admission/SOURCE-CONTRACT.md). Held: the three futures-dollar rules (macd_pt,
/// macd_pt_sl, rsi200_usd: $2,500 per E-mini contract), the figure with an unstated ADX filter and the intraday systems.
pub const REVIEWED: &[&str] = &["rwmts_sma200_sar", "rwmts_sma26_sar", "rwmts_sma26_wait3", "rwmts_env_trend", "rwmts_env_fade", "rwmts_macd_sar", "rwmts_dmi_sar", "rwmts_psar", "rwmts_psar_fade", "rwmts_stoch_ext", "rwmts_rsi_ext", "rwmts_mom10_sar", "rwmts_bb_fade", "rwmts_cci_trend", "rwmts_cci_fade", "rwmts_ma2x", "rwmts_ichi2", "rwmts_ma3x", "rwmts_ichi3", "rwmts_macd", "rwmts_dmi", "rwmts_dmi_adx", "rwmts_cb20", "rwmts_bb_break", "rwmts_cb20_stop3", "rwmts_rsi200", "rwmts_bb200", "rwmts_bb_adx", "rwmts_stoch_cci", "rwmts_stoch_cci_time"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

// ------------------------------------------------------------------ indicators

/// CQG exponential average, alpha = 2/(n+1), seeded with the first value.
fn ema(v: &[f64], n: usize) -> Vec<f64> {
    let a = 2.0 / (n as f64 + 1.0);
    let mut out = vec![f64::NAN; v.len()];
    let mut prev = f64::NAN;
    for (i, &x) in v.iter().enumerate() {
        if x.is_nan() { continue; }
        prev = if prev.is_nan() { x } else { prev + a * (x - prev) };
        out[i] = prev;
    }
    out
}
/// Population standard deviation over n bars ending at i.
fn sd(v: &[f64], n: usize) -> Vec<f64> {
    let mut out = vec![f64::NAN; v.len()];
    for i in n.saturating_sub(1)..v.len() {
        let w = &v[i + 1 - n..=i];
        let m = w.iter().sum::<f64>() / n as f64;
        out[i] = (w.iter().map(|x| (x - m).powi(2)).sum::<f64>() / n as f64).sqrt();
    }
    out
}
/// Wilder +DI and -DI (n), returned as (+DI, -DI).
fn di(x: &Ctx, n: usize) -> (Vec<f64>, Vec<f64>) {
    let len = x.r.len();
    let (mut p, mut m, mut tr) = (vec![f64::NAN; len], vec![f64::NAN; len], vec![f64::NAN; len]);
    let trs = true_range(x.r);
    for i in 1..len {
        let up = x.h[i] - x.h[i - 1];
        let dn = x.l[i - 1] - x.l[i];
        p[i] = if up > dn && up > 0.0 { up } else { 0.0 };
        m[i] = if dn > up && dn > 0.0 { dn } else { 0.0 };
        tr[i] = trs[i];
    }
    let (sp, sm, st) = (wilder(&p, n), wilder(&m, n), wilder(&tr, n));
    let f = |s: &[f64]| s.iter().zip(&st).map(|(a, b)| if *b > 0.0 { 100.0 * a / b } else { f64::NAN }).collect::<Vec<f64>>();
    (f(&sp), f(&sm))
}
/// Lambert CCI on the typical price (H+L+C)/3: (TP - SMA) / (0.015 * mean deviation).
fn cci(x: &Ctx, n: usize) -> Vec<f64> {
    let tp: Vec<f64> = (0..x.c.len()).map(|i| (x.h[i] + x.l[i] + x.c[i]) / 3.0).collect();
    let m = sma(&tp, n);
    (0..tp.len()).map(|i| {
        if m[i].is_nan() { return f64::NAN; }
        let md = tp[i + 1 - n..=i].iter().map(|v| (v - m[i]).abs()).sum::<f64>() / n as f64;
        if md > 0.0 { (tp[i] - m[i]) / (0.015 * md) } else { 0.0 }
    }).collect()
}
/// Raw (fast) %K over n bars.
fn fast_k(x: &Ctx, n: usize) -> Vec<f64> {
    (0..x.c.len()).map(|i| {
        if i + 1 < n { return f64::NAN; }
        let hh = x.h[i + 1 - n..=i].iter().cloned().fold(f64::MIN, f64::max);
        let ll = x.l[i + 1 - n..=i].iter().cloned().fold(f64::MAX, f64::min);
        if hh > ll { 100.0 * (x.c[i] - ll) / (hh - ll) } else { 50.0 }
    }).collect()
}
fn xabove(a: &[f64], b: impl Fn(usize) -> f64, i: usize) -> bool { i > 0 && a[i] > b(i) && a[i - 1] <= b(i - 1) }
fn xbelow(a: &[f64], b: impl Fn(usize) -> f64, i: usize) -> bool { i > 0 && a[i] < b(i) && a[i - 1] >= b(i - 1) }
fn hi_n(x: &Ctx, i: usize, n: usize) -> f64 { x.h[i + 1 - n..=i].iter().cloned().fold(f64::MIN, f64::max) }
fn lo_n(x: &Ctx, i: usize, n: usize) -> f64 { x.l[i + 1 - n..=i].iter().cloned().fold(f64::MAX, f64::min) }

// ------------------------------------------------------------------ simulator

#[derive(Clone, Copy)]
struct Pos { dir: f64, ei: usize, epx: f64 }

struct Book { out: Vec<Trade>, pos: Option<Pos> }
impl Book {
    fn new() -> Self { Book { out: vec![], pos: None } }
    fn enter(&mut self, dir: f64, i: usize, px: f64) { self.pos = Some(Pos { dir, ei: i, epx: px }); }
    fn exit(&mut self, i: usize, px: f64, reason: &str, sig: usize) {
        if let Some(p) = self.pos.take() {
            self.out.push(Trade { dir: p.dir, legs: vec![Leg { i: p.ei, px: p.epx, w: 1.0 }], exit_i: i, exit_px: px,
                reason: reason.into(), entry_kind: "open", signal_i: sig, open: false, exits: vec![] });
        }
    }
    fn finish(mut self, x: &Ctx, to: usize) -> Vec<Trade> {
        if self.pos.is_some() { self.exit(to, x.c[to], "open", to); self.out.last_mut().unwrap().open = true; }
        self.out
    }
}

/// Fill of a protective stop / profit target for a position on bar t (open-gap first, then stop before target).
fn level_fill(x: &Ctx, t: usize, dir: f64, stop: Option<f64>, tgt: Option<f64>) -> Option<(f64, &'static str)> {
    let (o, h, l) = (x.o[t], x.h[t], x.l[t]);
    let s_hit = |s: f64| if dir > 0.0 { l <= s } else { h >= s };
    let t_hit = |g: f64| if dir > 0.0 { h >= g } else { l <= g };
    let s_gap = |s: f64| if dir > 0.0 { o <= s } else { o >= s };
    let t_gap = |g: f64| if dir > 0.0 { o >= g } else { o <= g };
    if let Some(s) = stop.filter(|s| s.is_finite()) { if s_gap(s) { return Some((o, "stop (gap at open)")); } }
    if let Some(g) = tgt.filter(|g| g.is_finite()) { if t_gap(g) { return Some((o, "target (gap at open)")); } }
    if let Some(s) = stop.filter(|s| s.is_finite()) { if s_hit(s) { return Some((s, "stop")); } }
    if let Some(g) = tgt.filter(|g| g.is_finite()) { if t_hit(g) { return Some((g, "target")); } }
    None
}

/// Generic CQG-style system. `ent(s)` = entry signal on the close of bar s (+1 long, -1 short, 0 none), filled at the
/// next open; an opposite signal reverses. `cx(s, p)` = condition exit on the close of s, filled at the next open.
/// `lv(t, p)` = (stop, target) price orders working during bar t (including the entry bar).
fn sim(x: &Ctx, from: usize, to: usize, ent: &dyn Fn(usize) -> i8, cx: &dyn Fn(usize, &Pos) -> Option<&'static str>,
       lv: &dyn Fn(usize, &Pos) -> (Option<f64>, Option<f64>)) -> Vec<Trade> {
    let mut b = Book::new();
    for t in from + 1..=to {
        let s = t - 1;
        if let Some(p) = b.pos { if let Some(r) = cx(s, &p) { b.exit(t, x.o[t], r, s); } }
        let e = ent(s);
        if e != 0 {
            let d = e as f64;
            match b.pos {
                Some(p) if p.dir == d => {}
                Some(_) => { b.exit(t, x.o[t], "opposite signal (reverse at open)", s); b.enter(d, t, x.o[t]); }
                None => b.enter(d, t, x.o[t]),
            }
        }
        if let Some(p) = b.pos {
            let (st, tg) = lv(t, &p);
            if let Some((px, why)) = level_fill(x, t, p.dir, st, tg) { b.exit(t, px, why, t); }
        }
    }
    b.finish(x, to)
}
fn no_cx(_: usize, _: &Pos) -> Option<&'static str> { None }
fn no_lv(_: usize, _: &Pos) -> (Option<f64>, Option<f64>) { (None, None) }

/// Donchian channel stop-and-reverse at the prior n-bar high / low (HiLevel(@,n)[-1], LoLevel(@,n)[-1]), touch = fill,
/// gap = open. Optional percentage stop from entry; after it the system is flat until the next channel touch.
fn channel_sim(x: &Ctx, from: usize, to: usize, n: usize, stop_pct: Option<f64>) -> Vec<Trade> {
    let mut b = Book::new();
    for t in from.max(n) + 1..=to {
        let (hi, lo) = (hi_n(x, t - 1, n), lo_n(x, t - 1, n));
        let cur = b.pos.map_or(0.0, |p| p.dir);
        let up = cur <= 0.0 && x.h[t] >= hi;
        let dn = cur >= 0.0 && x.l[t] <= lo;
        // one channel event per bar; flat with both touched: the level nearer the open is assumed first
        let go = if up && dn { if (x.o[t] - hi).abs() <= (x.o[t] - lo).abs() { 1.0 } else { -1.0 } } else if up { 1.0 } else if dn { -1.0 } else { 0.0 };
        if go != 0.0 {
            let px = if go > 0.0 { x.o[t].max(hi) } else { x.o[t].min(lo) };
            if b.pos.is_some() { b.exit(t, px, "channel stop and reverse", t); }
            b.enter(go, t, px);
        }
        if let (Some(p), Some(k)) = (b.pos, stop_pct) {
            if let Some((px, why)) = level_fill(x, t, p.dir, Some(p.epx * (1.0 - p.dir * k)), None) {
                if !(p.ei == t && why.starts_with("stop (gap")) { b.exit(t, px, why, t); }
            }
        }
    }
    b.finish(x, to)
}

/// Wilder parabolic: returns per bar the flip (dir, price) if the SAR was hit on that bar.
fn psar_flips(x: &Ctx, to: usize, af0: f64, afmax: f64) -> Vec<Option<(f64, f64)>> {
    let n = to + 1;
    let mut out = vec![None; n];
    if n < 3 { return out; }
    let mut long = x.c[1] >= x.c[0];
    let mut af = af0;
    let (mut sar, mut ep) = if long { (x.l[0].min(x.l[1]), x.h[1].max(x.h[0])) } else { (x.h[0].max(x.h[1]), x.l[1].min(x.l[0])) };
    for i in 2..n {
        if long {
            if x.l[i] <= sar {
                out[i] = Some((-1.0, x.o[i].min(sar)));
                long = false; sar = ep.max(x.h[i]).max(x.h[i - 1]); ep = x.l[i]; af = af0;
            } else {
                if x.h[i] > ep { ep = x.h[i]; af = (af + af0).min(afmax); }
                sar = (sar + af * (ep - sar)).min(x.l[i]).min(x.l[i - 1]);
            }
        } else if x.h[i] >= sar {
            out[i] = Some((1.0, x.o[i].max(sar)));
            long = true; sar = ep.min(x.l[i]).min(x.l[i - 1]); ep = x.h[i]; af = af0;
        } else {
            if x.l[i] < ep { ep = x.l[i]; af = (af + af0).min(afmax); }
            sar = (sar + af * (ep - sar)).max(x.h[i]).max(x.h[i - 1]);
        }
    }
    out
}

// ------------------------------------------------------------------ catalogue

const NEXT_OPEN: &str = "Signals on the close, orders at the next day's open (p.42-43). Book ran it on futures / spot FX / cash S&P with $100 per round turn; the vault runs it on the selected index (instrument swap only) with its own cost setting.";
const INTRADAY: &str = "intraday bars (2-hour, 60-, 30-, 15- or 5-minute Nasdaq-100 / S&P 500 / T-bond / euro bars, day session) are not in the vault; the book's rules are defined on those bars, a daily run would be a different system";

macro_rules! def {
    ($id:expr, $name:expr, $g:expr, $dir:expr, $st:expr, $rules:expr, $notes:expr, $claim:expr) => {
        StrategyDef { id: $id, name: $name, group: $g, session: 0, dir: $dir, status: $st, needs_volume: false, needs_vix: false,
            needs_series: &[], book: BOOK, rules: $rules, notes: $notes, claim: $claim }
    };
}
fn es(t: &'static str, w: &'static str, p: &'static str, d: &'static str) -> Option<Claim> {
    Some(Claim { trades: t, win: w, avg: p, hold: d, period: "ES row (cash S&P 500 x 50), 12/31/1992-12/31/2002, $100 round turn deducted" })
}

pub fn catalogue() -> Vec<StrategyDef> {
    use Status::*;
    const C1: &str = "Ch.1-2 Indicators as complete systems";
    const C3: &str = "Ch.3 Trend-following systems";
    const C4: &str = "Ch.4 Mean reversion systems";
    const C5: &str = "Ch.5 Short-term systems";
    vec![
        def!("rwmts_sma200_sar", "200-day moving average stop-and-reverse", C1, "Long/Short", Live,
            &["E|\"if the market is in a significant uptrend, prices should not be weak enough to fall below the 200-day moving average\" - long when the close is above the 200-day simple moving average (p.4-5)",
              "X|\"whenever the market becomes weak enough to close below the moving average line, we not only exit all existing long positions but also initiate new short positions\" (stop and reverse, p.5)"],
            NEXT_OPEN, None),
        def!("rwmts_sma26_sar", "26-day moving average as trigger (Fig 2.1)", C1, "Long/Short", Live,
            &["E|Close above the 26-day simple moving average = long, close below = short; stop-and-reverse (Fig 2.1, p.20; p.24 \"the examination of the moving average has focused on stop-and-reverse trading systems\")",
              "X|Stop-and-reverse: the opposite entry signal closes the position at the same open and reverses it"],
            NEXT_OPEN, None),
        def!("rwmts_sma26_wait3", "26-day MA with 3-day whipsaw waiting period (Fig 2.2)", C1, "Long/Short", Live,
            &["E|\"the indicator-driven trigger requires that the market not only settles above the moving average, but that it does so for a consecutive number of time periods\" - 3 consecutive closes above the 26-day SMA = long, 3 below = short (p.19, Fig 2.2)",
              "X|Stop-and-reverse on the opposite signal (p.24)"],
            NEXT_OPEN, None),
        def!("rwmts_env_trend", "2.5% envelope of 21-day MA, exit at the MA (Fig 2.5)", C1, "Long/Short", Live,
            &["E|\"Valid trading signals are generated when the market settles beyond the upper or lower envelopes\" - close above 21-day SMA x 1.025 = long, below x 0.975 = short (p.21, Fig 2.5)",
              "X|\"exit at a 21-day MA\" (Fig 2.5): price order at yesterday's 21-day MA, like the book's own code for the same phrase (Bollinger system p.60: Price field = BMA[-1])"],
            NEXT_OPEN, None),
        def!("rwmts_env_fade", "Fade of the 2.5% envelope of a 21-day MA (Fig 2.6)", C1, "Long/Short", Live,
            &["E|Fade the envelope: close below 21-day SMA x 0.975 = long, close above x 1.025 = short (p.21-23, Fig 2.6)",
              "X|\"profit targets of the MA or 1% of entry\" - exit at yesterday's 21-day MA or at entry +/- 1%, whichever is reached first (p.23, Fig 2.6)",
              "X|\"5% of entry price fail-safe stop loss\" (Fig 2.6)"],
            "Fig 2.6 (spot S&P 500 x 250, 12/31/1997-12/31/2003): 80 trades, 80.00% winners, average duration 4 days, closed net $17,149. Param `targets` 0 = both targets (default, caption reading), 1 = MA only, 2 = 1% only (p.23 lists them as alternatives).",
            Some(Claim { trades: "80", win: "80.00%", avg: "$142 avg profit (x250)", hold: "4 days", period: "Spot S&P 500, 12/31/1997-12/31/2003" })),
        def!("rwmts_macd_sar", "MACD line vs signal line stop-and-reverse (Fig 2.11)", C1, "Long/Short", Live,
            &["F|MACD line = 13-period EMA - 26-period EMA; signal line = 9-period EMA of the MACD line (p.27)",
              "E|\"stop and reverse signals are generated whenever the MACD line closes beyond the MACD's signal line\" (p.27)",
              "X|Stop-and-reverse: the opposite entry signal closes the position at the same open and reverses it"],
            NEXT_OPEN, None),
        def!("rwmts_dmi_sar", "+DI / -DI stop-and-reverse (Fig 2.12)", C1, "Long/Short", Live,
            &["E|\"If the +DI is greater than the -DI, then the market is said to be trending higher; if -DI is greater than the +DI ... bearish ... it is another stop and reverse trend-following system\" (p.28)",
              "F|Period: the book prints none for this figure; it states \"I usually favor using the default parameters designated by the indicator's developer\" (p.15-16) -> Wilder's 14",
              "X|Stop-and-reverse: the opposite entry signal closes the position at the same open and reverses it"],
            NEXT_OPEN, None),
        def!("rwmts_dmi_adx_fig", "DMI crossover with ADX filter (Fig 2.13)", C1, "Long/Short",
            NotTestable("the figure shows the DMI crossover \"with ADX filter\" but the text never says how the filter acts (entries only? exits when ADX < 20?); the coded Ch.3 version is rwmts_dmi_adx"),
            &["E|DMI crossover system with an ADX(9) > 20 trend filter (p.28, Fig 2.13) - filter mechanics not stated"], "", None),
        def!("rwmts_psar", "Wilder parabolic stop-and-reverse (Fig 2.14)", C1, "Long/Short", Live,
            &["E|\"Wilder's parabolic, or stop and reverse (SAR) ... always in the market and whose stop-and-reverse trigger points take on a parabolic shape\" (p.29)",
              "F|Acceleration factor 0.02 rising 0.02 per new extreme to 0.20 (Wilder's defaults; book p.15-16 uses the developer's defaults)",
              "X|Stop-and-reverse: the opposite entry signal closes the position at the same open and reverses it"],
            "Stop-and-reverse at the SAR level during the day (gap = open). Book: spot GBP/USD 2003 (no index result).", None),
        def!("rwmts_psar_fade", "Fade of the parabolic with 2.5 SD fail-safe (Fig 2.15)", C1, "Long/Short", Live,
            &["E|\"fade its stop and reverse signals\": when the parabolic turns long, go short at its stop level, and vice versa (p.29)",
              "F|\"all entries required the market to be trading at less than 2.5 standard deviations from the 20-day moving average\" (p.30)",
              "X|\"fail-safe stop loss 2.5 standard deviations from the 20-day moving average\" (p.30); otherwise the next parabolic flip reverses the faded position"],
            "Fig 2.15 (cash S&P 500 x 250, 12/31/2002-12/31/2003): 24 trades, 70.83% winners, average duration 11 days, closed net $55,220. Bands use yesterday's 20-day SMA and population SD (CQG [-1] convention).",
            Some(Claim { trades: "24", win: "70.83%", avg: "$1,734 avg profit (x250)", hold: "11 days", period: "Cash S&P 500, 2003" })),
        def!("rwmts_stoch_ext", "14-day slow stochastics extremes (Fig 2.17)", C1, "Long/Short", Live,
            &["F|Slow %K = 3-day moving average of Fast %K(14); Slow %D = 3-day moving average of Slow %K (p.32)",
              "E|\"buy signals whenever SD closes below 15 and sell signals when SD closes above 85\" (p.33)",
              "X|\"For profitable exits, we will use SD closes above 30 and below 70\" (p.33)",
              "X|\"fail-safe exit will be designated as 2.5 percent of the asset's value at time of entry\" (p.33)"],
            "Book: spot euro/yen 2001-2003 (no index result).", None),
        def!("rwmts_rsi_ext", "14-day RSI extremes (Fig 2.18)", C1, "Long/Short", Live,
            &["E|\"enters trades whenever the indicator closes beyond either 70 or 30\" - RSI(14) below 30 = long, above 70 = short (p.33-34)",
              "X|\"Exiting with profits occurs whenever RSI closes above 35 or below 65\" (p.34)",
              "X|\"the same 2.5 percent failsafe stop\" (p.34)"],
            "Fig 2.18 (cash S&P 500 x 250, 12/31/1997-12/31/2003): 12 trades, 75.00% winners, average duration 2 days, closed net $25,080.",
            Some(Claim { trades: "12", win: "75.00%", avg: "$2,033 avg profit (x250)", hold: "2 days", period: "Cash S&P 500, 12/31/1997-12/31/2003" })),
        def!("rwmts_mom10_sar", "10-day momentum / ROC zero-line stop-and-reverse (Fig 2.19-2.20)", C1, "Long/Short", Live,
            &["F|Momentum = close - close 10 periods ago (ROC divides instead; same sign, so the same trades - p.35 \"these systems often produce identical results\")",
              "E|\"buy and sell signals triggered by closing beyond the zero level\" (p.35)",
              "X|Stop-and-reverse: the opposite entry signal closes the position at the same open and reverses it"],
            NEXT_OPEN, None),
        def!("rwmts_bb_fade", "Bollinger band fade with 20-day MA exit (Fig 2.21)", C1, "Long/Short", Live,
            &["E|\"entry at upper and lower bands\" (Fig 2.21) - coded like the book's own fade code (p.79): close crosses below the lower 20-day/2 SD band = long, crosses above the upper = short",
              "X|\"exits at 20-day moving average\" - price order at yesterday's 20-day SMA (Fig 2.21)",
              "X|\"or at 2.5% fail-safe stop loss\" (Fig 2.21)"],
            "Fig 2.21 (cash S&P 500 x 250, 2003): 13 trades, 61.54% winners, average duration 6 days, closed net $20,117. Param `cross` 1 = crossing (default), 0 = any close beyond the band.",
            Some(Claim { trades: "13", win: "61.54%", avg: "$1,102 avg profit (x250)", hold: "6 days", period: "Cash S&P 500, 2003" })),
        def!("rwmts_cci_trend", "CCI +/-100 trend system (Fig 2.23)", C1, "Long/Short", Live,
            &["F|CCI(20) on the typical price (p.37: \"usually 20 periods\", mean deviation x .015)",
              "E|\"entry triggers of closing at or beyond +100/-100\" - CCI >= +100 = long, <= -100 = short (Fig 2.23)",
              "X|\"exits at 0\" - long exits when CCI closes at or below 0, short at or above 0 (Fig 2.23)"],
            "Book: March 2004 T-bonds 2003 (no index result). \"0 or better\" read as the zero line.", None),
        def!("rwmts_cci_fade", "CCI +/-200 fade with +/-100 exits (Fig 2.24)", C1, "Long/Short", Live,
            &["E|\"-200 and +200 levels as entry triggers\": CCI(20) <= -200 = long, >= +200 = short (Fig 2.24)",
              "X|\"-100 or +100 as exits\": long exits when CCI >= -100, short when CCI <= +100 (Fig 2.24)",
              "X|\"fail-safe stop exit at 2.5%\" (Fig 2.24)"],
            "Book: AUD/CAD 2001-2003 (no index result).", None),
        // ----- Chapter 3 (CQG code)
        def!("rwmts_ma2x", "Two moving average crossover 9/26 (Table 3.2)", C3, "Long/Short", Live,
            &["E|Long Entry and Short Exit: MA(@,Sim,9)[-1] XABOVE MA(@,Sim,26)[-1] (p.51)",
              "E|Short Entry and Long Exit: MA(@,Sim,9)[-1] XBELOW MA(@,Sim,26)[-1] (p.51)",
              "X|Stop-and-reverse: the opposite entry signal closes the position at the same open and reverses it"],
            NEXT_OPEN, es("117", "35.90%", "$6,023 total", "22 days")),
        def!("rwmts_ichi2", "Ichimoku two moving average crossover (Table 3.3)", C3, "Long/Short", Live,
            &["E|Long Entry and Short Exit: MA(@,Sim,9)[-1] > MA(@,Sim,26)[-1] AND MA(@,Sim,26)[-1] > MA(@,Sim,26)[-2] (p.52)",
              "E|Short Entry and Long Exit: printed as MA(@,Sim,26)[-1] < MA(@,Sim,9)[-1] AND MA(@,Sim,26)[-1] < MA(@,Sim,26)[-2] (p.52); default = the printed code (it reproduces the ES row: 117 vs 118 trades; the prose reading gives 83); the prose (p.24, p.52) says the 9 must close below the 26 - param `short_rule` 0",
              "X|Stop-and-reverse: the opposite entry signal closes the position at the same open and reverses it"],
            NEXT_OPEN, es("118", "23.73%", "-$35,907 total", "22 days")),
        def!("rwmts_ma3x", "Three moving average crossover 9/26/52 (Table 3.4)", C3, "Long/Short", Live,
            &["E|Long Entry: MA9[-1] > MA26[-1] AND MA26[-1] > MA52[-1] (p.53)",
              "X|Long Exit: MA9[-1] < MA26[-1] OR MA26[-1] < MA52[-1] (p.53)",
              "E|Short Entry: MA9[-1] < MA26[-1] AND MA26[-1] < MA52[-1] (p.53)",
              "X|Short Exit: MA9[-1] > MA26[-1] OR MA26[-1] > MA52[-1] (p.53)"],
            NEXT_OPEN, es("84", "28.57%", "-$11,605 total", "21 days")),
        def!("rwmts_ichi3", "Ichimoku three moving average crossover (Table 3.5)", C3, "Long/Short", Live,
            &["E|Long Entry: MA9[-1] > MA26[-1] AND MA26[-1] > MA52[-1] AND MA26[-1] > MA26[-2] AND MA52[-1] > MA52[-2] (p.54)",
              "X|Long Exit: MA9[-1] < MA26[-1] OR MA26[-1] < MA52[-1] (p.54)",
              "E|Short Entry: mirror (all four reversed) (p.54)", "X|Short Exit: MA9[-1] > MA26[-1] OR MA26[-1] > MA52[-1] (p.54)"],
            NEXT_OPEN, es("75", "30.67%", "-$17,869 total", "22 days")),
        def!("rwmts_macd", "MACD 13/26/9 with signal-line sign (Table 3.6)", C3, "Long/Short", Live,
            &["E|Long Entry and Short Exit: MACD(@,13,26)[-1] XABOVE MACDA(@,13,26,9)[-1] AND MACDA(@,13,26,9)[-1] > 0 (p.55)",
              "E|Short Entry and Long Exit: MACD(@,13,26)[-1] XBELOW MACDA(@,13,26,9)[-1] AND MACDA(@,13,26,9)[-1] < 0 (p.55)",
              "X|Stop-and-reverse: the opposite entry signal closes the position at the same open and reverses it"],
            NEXT_OPEN, es("19", "36.84%", "-$4,242 total", "133 days")),
        def!("rwmts_dmi", "DMI +/-20 entries, zero-line exits (Table 3.7)", C3, "Long/Short", Live,
            &["F|DDIF(@,10) = +DI(10) - -DI(10) (p.56; Fig 2.12 shows the difference as one line)",
              "E|Long Entry: DDIF(@,10)[-1] XABOVE 20 (p.56)", "X|Long Exit: DDIF(@,10)[-1] XBELOW 0 (p.57)",
              "E|Short Entry: DDIF(@,10)[-1] XBELOW -20 (p.57)", "X|Short Exit: DDIF(@,10)[-1] XABOVE 0 (p.57)"],
            NEXT_OPEN, es("87", "29.89%", "-$25,911 total", "17 days")),
        def!("rwmts_dmi_adx", "DMI with ADX filter (Table 3.8)", C3, "Long/Short", Live,
            &["E|Long Entry: DDIF(@,10)[-1] XABOVE 20 AND ADX(@,9)[-1] > 20 (p.58)", "X|Long Exit: DDIF(@,10)[-1] XBELOW 0 OR ADX(@,9)[-1] < 20 (p.58)",
              "E|Short Entry: DDIF(@,10)[-1] XBELOW -20 AND ADX(@,9)[-1] > 20 (p.58)", "X|Short Exit: DDIF(@,10)[-1] XABOVE 0 OR ADX(@,9)[-1] < 20 (p.58)"],
            NEXT_OPEN, es("86", "31.4%", "-$18,652 total", "16 days")),
        def!("rwmts_cb20", "20-day channel breakout stop-and-reverse (Table 3.9)", C3, "Long/Short", Live,
            &["E|For Long Entry and Short Exit, set \"Price\" field to: HiLevel(@,20)[-1] (p.59)",
              "E|For Short Entry and Long Exit, set \"Price\" field to: LoLevel(@,20)[-1] (p.59)",
              "F|\"entry and exits at the prior 20-day high or low instead of the traditional greater than or less than\" (p.59)",
              "X|Stop-and-reverse: the opposite entry signal closes the position at the same open and reverses it"],
            "Stop orders at yesterday's 20-bar high/low; touch fills at the level, gaps at the open. Flat bar touching both levels: the level nearer the open is taken (assumption).",
            es("75", "34.67%", "$11,269 total", "35 days")),
        def!("rwmts_bb_break", "Bollinger band breakout, exit at the 20-day MA (Table 3.10)", C3, "Long/Short", Live,
            &["E|Long Entry: Close(@)[-1] > BHI(@,Sim,20,2.00)[-1] (p.60)", "E|Short Entry: Close(@)[-1] < BLO(@,Sim,20,2.00)[-1] (p.60)",
              "X|Long Exit and Short Exit set \"Price\" field to: BMA(@,Sim,20)[-1] (p.60)"],
            NEXT_OPEN, es("97", "36.08%", "-$20,113 total", "13 days")),
        def!("rwmts_cb20_stop3", "20-day channel breakout with 3% stop (Fig 3.4)", C3, "Long/Short", Live,
            &["E|20-day stop-and-reverse channel breakout as rwmts_cb20 (p.65)", "X|\"the same system with a stop-loss filter of 3 percent of asset value at entry\" (p.65, Fig 3.4)"],
            "After the 3% stop the system waits flat for the next channel touch. Book: spot USD/JPY 1993-2002 (no index result). Stop checked from the entry bar on (book p.73-74: ambiguous same-day cases count as losses).", None),
        def!("rwmts_macd_pt", "MACD with $2,500 profit exit (Table 3.12)", C3, "Long/Short", Live,
            &["E|MACD system of Table 3.6 (p.55)", "X|\"MACD with the addition of a profit exit set to $2,500\" (p.66) = 50 index points for the book's ES row (cash S&P 500 x 50)"],
            "After the profit exit the system is flat until the next MACD signal. Points param `pts` default 50 (= $2,500 / $50 per point, Table 3.1 note b).",
            es("41", "68.29%", "$19,829 total", "46 days")),
        def!("rwmts_macd_pt_sl", "MACD with $2,500 profit and stop exit (Table 3.13)", C3, "Long/Short", Live,
            &["E|MACD system of Table 3.6 (p.55)", "X|\"profit exit and a stop loss set to $2,500\" (p.67) = 50 index points each for the ES row"],
            "Points param `pts` default 50 (ES conversion).", es("48", "47.92%", "-$5,753 total", "29 days")),
        // ----- Chapter 4
        def!("rwmts_rsi200", "RSI extremes with 200-day MA filter, 2.5% stop (Table 4.4)", C4, "Long/Short", Live,
            &["E|Long Entry: RSI(@,9)[-1] < 35 AND Close(@)[-1] > MA(@,Sim,200)[-1] (p.75)", "X|Long Exit #1: RSI(@,14)[-1] XABOVE 60 (p.75)",
              "X|Long Exit #2: EntryPrice - (.025 * EntryPrice) (p.75)",
              "E|Short Entry: RSI(@,9)[-1] > 65 AND Close(@)[-1] < MA(@,Sim,200)[-1] (p.75)", "X|Short Exit #1: RSI(@,14)[-1] XBELOW 40 (p.75)",
              "X|Short Exit #2: EntryPrice + (.025 * EntryPrice) (p.75)"],
            NEXT_OPEN, Some(Claim { trades: "50", win: "48.00%", avg: "$8,711 total", hold: "16 days", period: "ES row of the mean-reversion portfolio (Table 4.4), 1993-2002" })),
        def!("rwmts_rsi200_usd", "RSI extremes with 200-day MA filter, $2,500 stop (Table 4.2)", C4, "Long/Short", Live,
            &["E|Entries and RSI exits as rwmts_rsi200 (p.75)", "X|Fixed $2,500 stop for ES (Table 4.1, p.76) = 50 index points: OpenPositionAverageEntryPrice - Dollar2Price(@,2500)"],
            "Points param `pts` default 50 (ES conversion).", es("39", "66.67%", "$10,441 total", "22 days")),
        def!("rwmts_bb200", "Bollinger band fade with 200-day MA filter (Table 4.5)", C4, "Long/Short", Live,
            &["E|Long Entry: Close(@)[-1] XBELOW BLO(@,Sim,20,2)[-1] AND Close(@)[-1] > MA(@,Sim,200)[-1] (p.79)",
              "X|Long Exit #1 Price field: BMA(@,Sim,20)[-1]; #2: EntryPrice - .025*EntryPrice (p.79-80)",
              "E|Short Entry: Close(@)[-1] XABOVE BHI(@,Sim,20,2)[-1] AND Close(@)[-1] < MA(@,Sim,75)[-1] (p.80 - printed 75, title says 200-day)",
              "X|Short Exit #1: BMA(@,Sim,20)[-1]; #2: EntryPrice + .025*EntryPrice (p.80)"],
            "Param `short_ma` default 200 = the section/table title; the printed code says MA(@,Sim,75). The ES row decides: 200 gives 36 trades / 66.67% (book 36 / 66.67%), 75 gives 30 / 70.00%. Set 75 for the printed code.",
            es("36", "66.67%", "$6,192 total", "5 days")),
        def!("rwmts_bb_adx", "Bollinger band fade with ADX filter, 2.5% target and stop (Table 4.6)", C4, "Long/Short", Live,
            &["E|Long Entry: Close(@)[-1] XBELOW BLO(@,Sim,20,2)[-1] AND ADX(@,9)[-1] < 20 (p.81)",
              "E|Short Entry: Close(@)[-1] XABOVE BHI(@,Sim,20,2)[-1] AND ADX(@,9)[-1] < 20 (p.81)",
              "X|\"set both the stop loss and profit exits at 2.5 percent of entry level for the E-mini S&P 500 and Japanese yen crosses. All other instruments will use a 1.25 percent\" (p.81)"],
            "Equity index -> the book's 2.5% (param `pct`; 1.25 is the book's value for the other instruments).",
            es("20", "45.00%", "-$4,336 total", "9 days")),
        def!("rwmts_stoch_cci", "Slow stochastics extremes with CCI filter, 1.5% stop (Table 4.7)", C4, "Long/Short", Live,
            &["E|Long Entry: SSD(@,14,Smo,3,Smo,3)[-1] XBELOW 15 AND CCI(@,10)[-1] < -100 (p.83)", "X|Long Exit #1: SSD[-1] XABOVE 30 (p.83)",
              "X|Long Exit #2: EntryPrice - .015*EntryPrice (p.83)",
              "E|Short Entry: SSD[-1] XABOVE 85 AND CCI(@,10)[-1] > 100 (p.83)", "X|Short Exit #1: SSD[-1] XBELOW 70 (p.83)", "X|Short Exit #2: EntryPrice + .015*EntryPrice (p.83)"],
            "SSD = slow %D: CQG 'Smo' (smoothed, Wilder-type) averages of Fast %K(14), 3 and 3.", es("36", "44.44%", "-$2,048 total", "9 days")),
        def!("rwmts_stoch_cci_time", "Slow stochastics + CCI with 15-day time exit (Table 4.8)", C4, "Long/Short", Live,
            &["E|As rwmts_stoch_cci (p.83)", "X|\"Add This Third Condition to Both Long and Short Exits: BarsSinceEntry(@,0,All,ThisTradeOnly) > 14\" (p.84)"],
            "Time exit read as: on the close of the bar where more than 14 bars have passed since the entry bar, exit at the next open.",
            es("39", "43.59%", "$2,161 total", "7 days")),
        // ----- Chapter 5 (intraday bars)
        def!("rwmts_2h_cb15_8", "Channel breakout 15/8 days with 7.5-day time exit on 2-hour bars (Table 5.4)", C5, "Long/Short", NeedsData(INTRADAY),
            &["E|Entry on 15-day highs/lows, exit on violation of 8-day highs/lows, time exit beyond 7.5 days; 2-hour bars (60 bars = 15 T-bond days) (p.92-93)"], "", None),
        def!("rwmts_2h_rsi400", "RSI extremes with 400-hour MA filter, 2-hour bars (Table 5.5)", C5, "Long/Short", NeedsData(INTRADAY),
            &["E|Chapter 4 RSI-extremes code on 120-minute Nasdaq-100 bars, 400-hour MA (p.94)"], "", None),
        def!("rwmts_2h_stoch_cci", "Slow stochastics + CCI with 30-hour time exit, 2-hour bars (Table 5.6)", C5, "Long/Short", NeedsData(INTRADAY),
            &["E|Chapter 4 stochastics/CCI code on 120-minute Nasdaq-100 bars (p.94-95)"], "", None),
        def!("rwmts_60m_rsi200h", "RSI extremes with 200-hour MA filter, 60-minute bars (Table 5.7)", C5, "Long/Short", NeedsData(INTRADAY),
            &["E|RSI extremes code on 60-minute Nasdaq-100 bars (p.95)"], "", None),
        def!("rwmts_60m_seven", "Seven-period reversal with 1% target and stop, 60-minute bars (Table 5.8)", C5, "Long/Short", NeedsData(INTRADAY),
            &["E|Long Entry: Close[-8] > Close[-7] > ... > Close[-2] AND Close[-2] < Close[-1] (p.96)",
              "X|Long Exit #1: Close[-7] < Close[-6] < ... < Close[-1]; #2 EntryPrice + 1%; #3 EntryPrice - 1% (p.96-97); shorts reversed"], "", None),
        def!("rwmts_60m_rsi_cross", "RSI(14) 25/75 crossover, 3% target / 1% stop, 60-minute bars (Table 5.9)", C5, "Long/Short", NeedsData(INTRADAY),
            &["E|Long Entry: RSI(@,14)[-2] < 25 AND RSI(@,14)[-1] XABOVE 25 (p.97)", "X|EntryPrice + 3% / EntryPrice - 1% (p.97-98)"], "", None),
        def!("rwmts_30m_rsi100h", "RSI extremes with 100-hour MA filter, 30-minute bars (Tables 5.10-5.11)", C5, "Long/Short", NeedsData(INTRADAY),
            &["E|RSI extremes code on 30-minute bars with 2.5% and 1.5% stops (p.98-99)"], "", None),
        def!("rwmts_30m_rsi_cross", "RSI crossover with 1% target and stop, 30-minute bars (Table 5.12)", C5, "Long/Short", NeedsData(INTRADAY),
            &["E|RSI crossover code with exit #1 reduced to 1% (p.100)"], "", None),
        def!("rwmts_15m_5m_rsi", "RSI extremes with 50-hour / 16.67-hour MA, 15- and 5-minute bars (Tables 5.13-5.14)", C5, "Long/Short", NeedsData(INTRADAY),
            &["E|RSI extremes on 15-minute (50-hour MA) and 5-minute (16.67-hour MA) bars, 1% stop ND / 0.5% SP (p.101)"], "", None),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    if !id.starts_with("rwmts_") { return None; }
    let d = catalogue().into_iter().find(|d| d.id == id)?;
    let fam: (&'static str, &'static str) = match id {
        "rwmts_sma200_sar" | "rwmts_sma26_sar" | "rwmts_sma26_wait3" | "rwmts_env_trend" => ("rwmts_ma", "Weissman · moving average triggers"),
        "rwmts_ma2x" | "rwmts_ichi2" | "rwmts_ma3x" | "rwmts_ichi3" => ("rwmts_cross", "Weissman · moving average crossovers"),
        "rwmts_macd_sar" | "rwmts_macd" | "rwmts_macd_pt" | "rwmts_macd_pt_sl" => ("rwmts_macd", "Weissman · MACD"),
        "rwmts_dmi_sar" | "rwmts_dmi_adx_fig" | "rwmts_dmi" | "rwmts_dmi_adx" => ("rwmts_dmi", "Weissman · DMI / ADX"),
        "rwmts_cb20" | "rwmts_cb20_stop3" => ("rwmts_cb", "Weissman · channel breakout"),
        "rwmts_psar" | "rwmts_psar_fade" => ("rwmts_psar", "Weissman · parabolic"),
        "rwmts_env_fade" | "rwmts_bb_fade" | "rwmts_bb200" | "rwmts_bb_adx" | "rwmts_bb_break" => ("rwmts_bands", "Weissman · envelopes and Bollinger bands"),
        "rwmts_rsi_ext" | "rwmts_rsi200" | "rwmts_rsi200_usd" => ("rwmts_rsi", "Weissman · RSI extremes"),
        "rwmts_stoch_ext" | "rwmts_stoch_cci" | "rwmts_stoch_cci_time" => ("rwmts_stoch", "Weissman · stochastics extremes"),
        "rwmts_cci_trend" | "rwmts_cci_fade" | "rwmts_mom10_sar" => ("rwmts_osc", "Weissman · CCI and momentum"),
        _ => ("rwmts_intraday", "Weissman · short-term intraday systems"),
    };
    Some((fam.0, fam.1, d.name))
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    Some(match id {
        "rwmts_sma200_sar" => vec![p("n", "Moving average days", 200., 20., 300., 10.)],
        "rwmts_sma26_sar" => vec![p("n", "Moving average days", 26., 5., 100., 1.)],
        "rwmts_sma26_wait3" => vec![p("n", "Moving average days", 26., 5., 100., 1.), p("wait", "Consecutive closes beyond the MA", 3., 1., 10., 1.)],
        "rwmts_env_trend" => vec![p("n", "Moving average days", 21., 5., 100., 1.), p("env", "Envelope %", 2.5, 0.5, 6., 0.5)],
        "rwmts_env_fade" => vec![p("n", "Moving average days", 21., 5., 100., 1.), p("env", "Envelope %", 2.5, 0.5, 6., 0.5),
            p("tgt", "Profit target % of entry", 1., 0.25, 5., 0.25), p("stop", "Fail-safe stop % of entry", 5., 1., 10., 0.5),
            p("targets", "Targets: 0 MA or %, 1 MA only, 2 % only", 0., 0., 2., 1.)],
        "rwmts_macd_sar" | "rwmts_macd" => vec![p("fast", "Fast EMA", 13., 5., 30., 1.), p("slow", "Slow EMA", 26., 15., 60., 1.), p("sig", "Signal EMA", 9., 3., 20., 1.)],
        "rwmts_macd_pt" | "rwmts_macd_pt_sl" => vec![p("fast", "Fast EMA", 13., 5., 30., 1.), p("slow", "Slow EMA", 26., 15., 60., 1.), p("sig", "Signal EMA", 9., 3., 20., 1.),
            p("pts", "Profit (and stop) in index points ($2,500 / $50 ES)", 50., 10., 400., 10.)],
        "rwmts_dmi_sar" => vec![p("n", "DMI period", 14., 5., 30., 1.)],
        "rwmts_psar" => vec![p("af", "Acceleration step", 0.02, 0.01, 0.05, 0.01), p("afmax", "Maximum acceleration", 0.2, 0.1, 0.5, 0.05)],
        "rwmts_psar_fade" => vec![p("af", "Acceleration step", 0.02, 0.01, 0.05, 0.01), p("afmax", "Maximum acceleration", 0.2, 0.1, 0.5, 0.05),
            p("n", "Band MA days", 20., 10., 50., 1.), p("k", "Standard deviations", 2.5, 1., 4., 0.25)],
        "rwmts_stoch_ext" => vec![p("n", "Stochastic days", 14., 5., 30., 1.), p("lo", "Buy below SD", 15., 5., 30., 1.), p("hi", "Sell above SD", 85., 70., 95., 1.),
            p("xl", "Long exit SD above", 30., 20., 50., 1.), p("xs", "Short exit SD below", 70., 50., 80., 1.), p("stop", "Fail-safe stop %", 2.5, 0.5, 6., 0.5)],
        "rwmts_rsi_ext" => vec![p("n", "RSI days", 14., 5., 30., 1.), p("lo", "Buy below", 30., 10., 40., 1.), p("hi", "Sell above", 70., 60., 90., 1.),
            p("xl", "Long exit above", 35., 30., 60., 1.), p("xs", "Short exit below", 65., 40., 70., 1.), p("stop", "Fail-safe stop %", 2.5, 0.5, 6., 0.5)],
        "rwmts_mom10_sar" => vec![p("n", "Momentum days", 10., 2., 50., 1.)],
        "rwmts_bb_fade" => vec![p("n", "Band days", 20., 10., 50., 1.), p("k", "Standard deviations", 2., 1., 3., 0.25), p("stop", "Fail-safe stop %", 2.5, 0.5, 6., 0.5),
            p("cross", "Entry: 1 crossing, 0 any close beyond", 1., 0., 1., 1.)],
        "rwmts_cci_trend" => vec![p("n", "CCI period", 20., 5., 50., 1.), p("lvl", "Entry level", 100., 50., 200., 10.)],
        "rwmts_cci_fade" => vec![p("n", "CCI period", 20., 5., 50., 1.), p("lvl", "Entry level", 200., 100., 300., 10.), p("xl", "Exit level", 100., 0., 200., 10.),
            p("stop", "Fail-safe stop %", 2.5, 0.5, 6., 0.5)],
        "rwmts_ma2x" => vec![p("fast", "Fast MA", 9., 2., 30., 1.), p("slow", "Slow MA", 26., 10., 60., 1.)],
        "rwmts_ichi2" => vec![p("fast", "Fast MA", 9., 2., 30., 1.), p("slow", "Slow MA", 26., 10., 60., 1.), p("short_rule", "Short rule: 1 printed code (default), 0 prose 9<26", 1., 0., 1., 1.)],
        "rwmts_ma3x" | "rwmts_ichi3" => vec![p("fast", "Fast MA", 9., 2., 30., 1.), p("mid", "Middle MA", 26., 10., 60., 1.), p("slow", "Slow MA", 52., 30., 120., 1.)],
        "rwmts_dmi" => vec![p("n", "DMI period", 10., 5., 30., 1.), p("lvl", "Entry level", 20., 5., 40., 1.)],
        "rwmts_dmi_adx" => vec![p("n", "DMI period", 10., 5., 30., 1.), p("lvl", "Entry level", 20., 5., 40., 1.), p("adx_n", "ADX period", 9., 5., 30., 1.), p("adx", "ADX threshold", 20., 10., 40., 1.)],
        "rwmts_cb20" => vec![p("n", "Channel days", 20., 5., 100., 1.)],
        "rwmts_cb20_stop3" => vec![p("n", "Channel days", 20., 5., 100., 1.), p("stop", "Stop % of entry", 3., 1., 5., 0.5)],
        "rwmts_bb_break" => vec![p("n", "Band days", 20., 10., 50., 1.), p("k", "Standard deviations", 2., 1., 3., 0.25)],
        "rwmts_rsi200" => vec![p("rsi_in", "Entry RSI period", 9., 2., 20., 1.), p("lo", "Long entry RSI below", 35., 10., 50., 1.), p("hi", "Short entry RSI above", 65., 50., 90., 1.),
            p("rsi_out", "Exit RSI period", 14., 2., 30., 1.), p("xl", "Long exit RSI crosses above", 60., 40., 80., 1.), p("xs", "Short exit RSI crosses below", 40., 20., 60., 1.),
            p("ma", "Trend MA days", 200., 50., 300., 10.), p("stop", "Stop % of entry", 2.5, 0.5, 6., 0.5)],
        "rwmts_rsi200_usd" => vec![p("rsi_in", "Entry RSI period", 9., 2., 20., 1.), p("lo", "Long entry RSI below", 35., 10., 50., 1.), p("hi", "Short entry RSI above", 65., 50., 90., 1.),
            p("rsi_out", "Exit RSI period", 14., 2., 30., 1.), p("xl", "Long exit RSI crosses above", 60., 40., 80., 1.), p("xs", "Short exit RSI crosses below", 40., 20., 60., 1.),
            p("ma", "Trend MA days", 200., 50., 300., 10.), p("pts", "Stop in index points ($2,500 / $50 ES)", 50., 10., 400., 10.)],
        "rwmts_bb200" => vec![p("n", "Band days", 20., 10., 50., 1.), p("k", "Standard deviations", 2., 1., 3., 0.25), p("long_ma", "Long filter MA", 200., 50., 300., 5.),
            p("short_ma", "Short filter MA (title 200; code prints 75)", 200., 50., 300., 5.), p("stop", "Stop % of entry", 2.5, 0.5, 6., 0.5)],
        "rwmts_bb_adx" => vec![p("n", "Band days", 20., 10., 50., 1.), p("k", "Standard deviations", 2., 1., 3., 0.25), p("adx_n", "ADX period", 9., 5., 30., 1.),
            p("adx", "ADX below", 20., 10., 40., 1.), p("pct", "Target and stop % of entry", 2.5, 0.5, 5., 0.25)],
        "rwmts_stoch_cci" | "rwmts_stoch_cci_time" => {
            let mut v = vec![p("n", "Stochastic days", 14., 5., 30., 1.), p("lo", "Long: SSD crosses below", 15., 5., 30., 1.), p("hi", "Short: SSD crosses above", 85., 70., 95., 1.),
                p("xl", "Long exit SSD crosses above", 30., 20., 50., 1.), p("xs", "Short exit SSD crosses below", 70., 50., 80., 1.),
                p("cci_n", "CCI period", 10., 5., 30., 1.), p("cci", "CCI beyond +/-", 100., 50., 200., 10.), p("stop", "Stop % of entry", 1.5, 0.5, 5., 0.25)];
            if id == "rwmts_stoch_cci_time" { v.push(p("bars", "Exit when bars since entry exceed", 14., 3., 40., 1.)); }
            v
        }
        _ => return None,
    })
}

// ------------------------------------------------------------------ run

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !id.starts_with("rwmts_") { return None; }
    let schema = params(id)?;
    if from >= to || to >= x.r.len() { return Some(vec![]); }
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).map_or(f64::NAN, |d| d.def));
    let u = |k: &str| g(k).round().max(1.0) as usize;
    let c = &x.c;
    let lvl = |a: &[f64], b: &[f64], s: usize| -> i8 { if a[s] > b[s] { 1 } else if a[s] < b[s] { -1 } else { 0 } };
    let pct_stop = |k: f64| move |_: usize, q: &Pos| (Some(q.epx * (1.0 - q.dir * k / 100.0)), None);
    Some(match id {
        "rwmts_sma200_sar" | "rwmts_sma26_sar" => {
            let m = sma(c, u("n"));
            sim(x, from, to, &|s| lvl(c, &m, s), &no_cx, &no_lv)
        }
        "rwmts_sma26_wait3" => {
            let (m, w) = (sma(c, u("n")), u("wait"));
            sim(x, from, to, &|s| {
                if s + 1 < w { return 0; }
                let r = s + 1 - w..=s;
                if r.clone().all(|i| c[i] > m[i]) { 1 } else if r.clone().all(|i| c[i] < m[i]) { -1 } else { 0 }
            }, &no_cx, &no_lv)
        }
        "rwmts_env_trend" => {
            let (m, e) = (sma(c, u("n")), g("env") / 100.0);
            sim(x, from, to, &|s| if c[s] > m[s] * (1.0 + e) { 1 } else if c[s] < m[s] * (1.0 - e) { -1 } else { 0 },
                &no_cx, &|t, _| (Some(m[t - 1]), None))
        }
        "rwmts_env_fade" => {
            let (m, e, tg, st, mode) = (sma(c, u("n")), g("env") / 100.0, g("tgt") / 100.0, g("stop") / 100.0, g("targets").round() as i32);
            sim(x, from, to, &|s| if c[s] < m[s] * (1.0 - e) { 1 } else if c[s] > m[s] * (1.0 + e) { -1 } else { 0 }, &no_cx, &|t, q| {
                let pctg = q.epx * (1.0 + q.dir * tg);
                let tgt = match mode { 1 => m[t - 1], 2 => pctg, _ => if q.dir > 0.0 { m[t - 1].min(pctg) } else { m[t - 1].max(pctg) } };
                (Some(q.epx * (1.0 - q.dir * st)), Some(tgt))
            })
        }
        "rwmts_macd_sar" => {
            let (f, sl) = (ema(c, u("fast")), ema(c, u("slow")));
            let line: Vec<f64> = f.iter().zip(&sl).map(|(a, b)| a - b).collect();
            let sig = ema(&line, u("sig"));
            sim(x, from, to, &|s| lvl(&line, &sig, s), &no_cx, &no_lv)
        }
        "rwmts_macd" | "rwmts_macd_pt" | "rwmts_macd_pt_sl" => {
            let (f, sl) = (ema(c, u("fast")), ema(c, u("slow")));
            let line: Vec<f64> = f.iter().zip(&sl).map(|(a, b)| a - b).collect();
            let sig = ema(&line, u("sig"));
            let ent = |s: usize| if xabove(&line, |i| sig[i], s) && sig[s] > 0.0 { 1 } else if xbelow(&line, |i| sig[i], s) && sig[s] < 0.0 { -1 } else { 0 };
            let pts = g("pts");
            match id {
                "rwmts_macd" => sim(x, from, to, &ent, &no_cx, &no_lv),
                "rwmts_macd_pt" => sim(x, from, to, &ent, &no_cx, &|_, q| (None, Some(q.epx + q.dir * pts))),
                _ => sim(x, from, to, &ent, &no_cx, &|_, q| (Some(q.epx - q.dir * pts), Some(q.epx + q.dir * pts))),
            }
        }
        "rwmts_dmi_sar" => {
            let (pd, md) = di(x, u("n"));
            sim(x, from, to, &|s| lvl(&pd, &md, s), &no_cx, &no_lv)
        }
        "rwmts_psar" | "rwmts_psar_fade" => {
            let flips = psar_flips(x, to, g("af"), g("afmax"));
            let mut b = Book::new();
            if id == "rwmts_psar" {
                for t in from + 1..=to {
                    if let Some((d, px)) = flips[t] {
                        if b.pos.is_some() { b.exit(t, px, "parabolic stop and reverse", t); }
                        b.enter(d, t, px);
                    }
                }
            } else {
                let (n, k) = (u("n"), g("k"));
                let (m, s) = (sma(c, n), sd(c, n));
                for t in from + 1..=to {
                    if let Some(q) = b.pos { // fail-safe band stop first (conservative)
                        if q.ei < t {
                            let stop = m[t - 1] - q.dir * k * s[t - 1];
                            if let Some((px, why)) = level_fill(x, t, q.dir, Some(stop), None) { b.exit(t, px, why, t); }
                        }
                    }
                    if let Some((d, px)) = flips[t] {
                        if b.pos.is_some() { b.exit(t, px, "parabolic flip (fade reverses)", t); }
                        if (px - m[t - 1]).abs() < k * s[t - 1] { b.enter(-d, t, px); }
                    }
                }
            }
            b.finish(x, to)
        }
        "rwmts_stoch_ext" => {
            let n = u("n");
            let sk = sma(&fast_k(x, n), 3);
            let sdv = sma(&sk, 3);
            let (lo, hi, xl, xs) = (g("lo"), g("hi"), g("xl"), g("xs"));
            sim(x, from, to, &|s| if sdv[s] < lo { 1 } else if sdv[s] > hi { -1 } else { 0 },
                &|s, q| if (q.dir > 0.0 && sdv[s] > xl) || (q.dir < 0.0 && sdv[s] < xs) { Some("SD exit level") } else { None }, &pct_stop(g("stop")))
        }
        "rwmts_rsi_ext" => {
            let r = rsi(c, u("n"));
            let (lo, hi, xl, xs) = (g("lo"), g("hi"), g("xl"), g("xs"));
            sim(x, from, to, &|s| if r[s] < lo { 1 } else if r[s] > hi { -1 } else { 0 },
                &|s, q| if (q.dir > 0.0 && r[s] > xl) || (q.dir < 0.0 && r[s] < xs) { Some("RSI exit level") } else { None }, &pct_stop(g("stop")))
        }
        "rwmts_mom10_sar" => {
            let n = u("n");
            sim(x, from, to, &|s| if s < n { 0 } else if c[s] > c[s - n] { 1 } else if c[s] < c[s - n] { -1 } else { 0 }, &no_cx, &no_lv)
        }
        "rwmts_bb_fade" => {
            let (n, k) = (u("n"), g("k"));
            let (m, s) = (sma(c, n), sd(c, n));
            let up = |i: usize| m[i] + k * s[i];
            let dn = |i: usize| m[i] - k * s[i];
            let cross = g("cross").round() >= 1.0;
            sim(x, from, to, &|i| {
                if cross { if xbelow(c, dn, i) { 1 } else if xabove(c, up, i) { -1 } else { 0 } }
                else if c[i] < dn(i) { 1 } else if c[i] > up(i) { -1 } else { 0 }
            }, &no_cx, &|t, q| (Some(q.epx * (1.0 - q.dir * g("stop") / 100.0)), Some(m[t - 1])))
        }
        "rwmts_cci_trend" => {
            let (v, l) = (cci(x, u("n")), g("lvl"));
            sim(x, from, to, &|s| if v[s] >= l { 1 } else if v[s] <= -l { -1 } else { 0 },
                &|s, q| if (q.dir > 0.0 && v[s] <= 0.0) || (q.dir < 0.0 && v[s] >= 0.0) { Some("CCI back to 0") } else { None }, &no_lv)
        }
        "rwmts_cci_fade" => {
            let (v, l, xl) = (cci(x, u("n")), g("lvl"), g("xl"));
            sim(x, from, to, &|s| if v[s] <= -l { 1 } else if v[s] >= l { -1 } else { 0 },
                &|s, q| if (q.dir > 0.0 && v[s] >= -xl) || (q.dir < 0.0 && v[s] <= xl) { Some("CCI exit level") } else { None }, &pct_stop(g("stop")))
        }
        "rwmts_ma2x" => {
            let (f, sl) = (sma(c, u("fast")), sma(c, u("slow")));
            sim(x, from, to, &|s| if xabove(&f, |i| sl[i], s) { 1 } else if xbelow(&f, |i| sl[i], s) { -1 } else { 0 }, &no_cx, &no_lv)
        }
        "rwmts_ichi2" => {
            let (f, sl) = (sma(c, u("fast")), sma(c, u("slow")));
            let code = g("short_rule").round() >= 1.0;
            sim(x, from, to, &|s| {
                if s == 0 { return 0; }
                if f[s] > sl[s] && sl[s] > sl[s - 1] { 1 }
                else if (if code { sl[s] < f[s] } else { f[s] < sl[s] }) && sl[s] < sl[s - 1] { -1 } else { 0 }
            }, &no_cx, &no_lv)
        }
        "rwmts_ma3x" | "rwmts_ichi3" => {
            let (a, b2, cc) = (sma(c, u("fast")), sma(c, u("mid")), sma(c, u("slow")));
            let ichi = id == "rwmts_ichi3";
            sim(x, from, to, &|s| {
                if s == 0 { return 0; }
                let up = a[s] > b2[s] && b2[s] > cc[s] && (!ichi || (b2[s] > b2[s - 1] && cc[s] > cc[s - 1]));
                let dn = a[s] < b2[s] && b2[s] < cc[s] && (!ichi || (b2[s] < b2[s - 1] && cc[s] < cc[s - 1]));
                if up { 1 } else if dn { -1 } else { 0 }
            }, &|s, q| {
                let out = if q.dir > 0.0 { a[s] < b2[s] || b2[s] < cc[s] } else { a[s] > b2[s] || b2[s] > cc[s] };
                if out { Some("MA alignment broken") } else { None }
            }, &no_lv)
        }
        "rwmts_dmi" | "rwmts_dmi_adx" => {
            let (pd, md) = di(x, u("n"));
            let dd: Vec<f64> = pd.iter().zip(&md).map(|(a, b)| a - b).collect();
            let l = g("lvl");
            let adx = if id == "rwmts_dmi_adx" { Some((dmi(x.r, u("adx_n")).1, g("adx"))) } else { None };
            let ok = |s: usize| adx.as_ref().map_or(true, |(a, th)| a[s] > *th);
            let weak = |s: usize| adx.as_ref().map_or(false, |(a, th)| a[s] < *th);
            sim(x, from, to, &|s| if xabove(&dd, |_| l, s) && ok(s) { 1 } else if xbelow(&dd, |_| -l, s) && ok(s) { -1 } else { 0 },
                &|s, q| {
                    let z = if q.dir > 0.0 { xbelow(&dd, |_| 0.0, s) } else { xabove(&dd, |_| 0.0, s) };
                    if z { Some("DDIF crossed 0") } else if weak(s) { Some("ADX below threshold") } else { None }
                }, &no_lv)
        }
        "rwmts_cb20" => channel_sim(x, from, to, u("n"), None),
        "rwmts_cb20_stop3" => channel_sim(x, from, to, u("n"), Some(g("stop") / 100.0)),
        "rwmts_bb_break" => {
            let (n, k) = (u("n"), g("k"));
            let (m, s) = (sma(c, n), sd(c, n));
            sim(x, from, to, &|i| if c[i] > m[i] + k * s[i] { 1 } else if c[i] < m[i] - k * s[i] { -1 } else { 0 },
                &no_cx, &|t, _| (Some(m[t - 1]), None))
        }
        "rwmts_rsi200" | "rwmts_rsi200_usd" => {
            let (ri, ro, m) = (rsi(c, u("rsi_in")), rsi(c, u("rsi_out")), sma(c, u("ma")));
            let (lo, hi, xl, xs) = (g("lo"), g("hi"), g("xl"), g("xs"));
            let usd = id == "rwmts_rsi200_usd";
            let (st, pts) = (g("stop") / 100.0, g("pts"));
            sim(x, from, to, &|s| if ri[s] < lo && c[s] > m[s] { 1 } else if ri[s] > hi && c[s] < m[s] { -1 } else { 0 },
                &|s, q| if (q.dir > 0.0 && xabove(&ro, |_| xl, s)) || (q.dir < 0.0 && xbelow(&ro, |_| xs, s)) { Some("RSI(14) exit cross") } else { None },
                &|_, q| (Some(if usd { q.epx - q.dir * pts } else { q.epx * (1.0 - q.dir * st) }), None))
        }
        "rwmts_bb200" => {
            let (n, k) = (u("n"), g("k"));
            let (m, s, ml, ms) = (sma(c, n), sd(c, n), sma(c, u("long_ma")), sma(c, u("short_ma")));
            let up = |i: usize| m[i] + k * s[i];
            let dn = |i: usize| m[i] - k * s[i];
            sim(x, from, to, &|i| if xbelow(c, dn, i) && c[i] > ml[i] { 1 } else if xabove(c, up, i) && c[i] < ms[i] { -1 } else { 0 },
                &no_cx, &|t, q| (Some(q.epx * (1.0 - q.dir * g("stop") / 100.0)), Some(m[t - 1])))
        }
        "rwmts_bb_adx" => {
            let (n, k) = (u("n"), g("k"));
            let (m, s, a) = (sma(c, n), sd(c, n), dmi(x.r, u("adx_n")).1);
            let (th, pc) = (g("adx"), g("pct") / 100.0);
            let up = |i: usize| m[i] + k * s[i];
            let dn = |i: usize| m[i] - k * s[i];
            sim(x, from, to, &|i| if xbelow(c, dn, i) && a[i] < th { 1 } else if xabove(c, up, i) && a[i] < th { -1 } else { 0 },
                &no_cx, &|_, q| (Some(q.epx * (1.0 - q.dir * pc)), Some(q.epx * (1.0 + q.dir * pc))))
        }
        "rwmts_stoch_cci" | "rwmts_stoch_cci_time" => {
            let n = u("n");
            let ssd = wilder(&wilder(&fast_k(x, n), 3), 3);
            let v = cci(x, u("cci_n"));
            let (lo, hi, xl, xs, cl) = (g("lo"), g("hi"), g("xl"), g("xs"), g("cci"));
            let bars = if id == "rwmts_stoch_cci_time" { Some(u("bars")) } else { None };
            sim(x, from, to, &|s| if xbelow(&ssd, |_| lo, s) && v[s] < -cl { 1 } else if xabove(&ssd, |_| hi, s) && v[s] > cl { -1 } else { 0 },
                &|s, q| {
                    if (q.dir > 0.0 && xabove(&ssd, |_| xl, s)) || (q.dir < 0.0 && xbelow(&ssd, |_| xs, s)) { Some("SSD exit cross") }
                    else if bars.is_some_and(|b| s >= q.ei && s - q.ei > b) { Some("time exit") } else { None }
                }, &pct_stop(g("stop")))
        }
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;
    fn rows(c: &[f64]) -> Vec<Row> {
        c.iter().enumerate().map(|(i, &v)| Row { ticker: "T".into(), date: format!("{:04}-{:02}-{:02}", 2000 + i / 300, 1 + (i / 25) % 12, 1 + i % 25),
            open: v, high: v + 1., low: v - 1., close: v, volume: None }).collect()
    }
    #[test]
    fn ma_cross_fills_next_open_and_reverses() {
        // 30 bars falling then rising: the 9/26 cross happens on a close, the fill is the next open
        let mut v: Vec<f64> = (0..40).map(|i| 100.0 - i as f64 * 0.5).collect();
        v.extend((0..40).map(|i| 80.0 + i as f64));
        let r = rows(&v); let x = Ctx::new(&r, &r, &[]);
        let f = sma(&x.c, 9); let s = sma(&x.c, 26);
        let k = (27..80).find(|&i| f[i] > s[i] && f[i - 1] <= s[i - 1]).unwrap();
        let t = run("rwmts_ma2x", &x, 0, 79, &Params::new()).unwrap();
        assert_eq!(t[0].legs[0].i, k + 1); assert_eq!(t[0].legs[0].px, x.o[k + 1]); assert_eq!(t[0].dir, 1.0); assert!(t[0].open);
    }
    #[test]
    fn stop_beats_target_on_the_same_bar_and_gaps_fill_at_open() {
        let mut r = rows(&vec![100.; 10]); r[3].high = 110.; r[3].low = 90.; let x = Ctx::new(&r, &r, &[]);
        assert_eq!(level_fill(&x, 3, 1.0, Some(95.), Some(105.)).unwrap().0, 95.);
        r[4].open = 90.; r[4].low = 89.; let x = Ctx::new(&r, &r, &[]);
        assert_eq!(level_fill(&x, 4, 1.0, Some(95.), Some(105.)).unwrap().0, 90.);
    }
    #[test]
    fn channel_breakout_buys_at_the_prior_high() {
        // highs fall and lows rise, so nothing touches the channel until bar 25 jumps through the prior 20-bar high
        let mut r = rows(&vec![100.; 30]);
        for (i, b) in r.iter_mut().enumerate() { b.high = 102. - 0.01 * i as f64; b.low = 98. + 0.01 * i as f64; }
        r[25].high = 106.; r[25].open = 100.5; r[25].close = 105.;
        let x = Ctx::new(&r, &r, &[]);
        let t = run("rwmts_cb20", &x, 21, 29, &Params::new()).unwrap();
        assert_eq!(t.len(), 1); assert_eq!(t[0].legs[0].i, 25); assert!((t[0].legs[0].px - 101.95).abs() < 1e-9); assert_eq!(t[0].dir, 1.0);
    }
    #[test]
    fn catalogue_ids_unique_and_live_have_params() {
        let c = catalogue(); let mut ids: Vec<_> = c.iter().map(|d| d.id).collect(); ids.sort(); ids.dedup();
        assert_eq!(ids.len(), c.len());
        for d in c.iter().filter(|d| d.status == Status::Live) { assert!(params(d.id).is_some(), "{}", d.id); assert!(family_of(d.id).is_some()); }
    }
}
