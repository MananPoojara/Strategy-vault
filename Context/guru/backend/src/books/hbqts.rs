//! Howard B. Bandy, *Quantitative Trading Systems: Practical Methods for Design, Testing and Validation* (Blue Owl, 2007).
//! Catalogue status alone does not establish rule, instrument or accounting readiness.
//! The bounded companion-code review is documented in library/bandy/REVIEW-2026-10-13.md.
//! Its exact IDs route through `reviewed`, which preserves formula-array history and
//! source timing with declared indicator initialization and single-symbol research limits.
//! Other legacy translations remain outside that review; their sizing and stop conventions
//! must not be generalized to the reviewed subset or certified from these checks.
use crate::connors::{dmi, rsi, true_range, wilder, Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};
#[path = "hbqts/reviewed.rs"]
pub mod reviewed;

pub const BOOK: &str = "hbqts";
/// Admitted ids (authors/bandy/admission/SOURCE-CONTRACT.md).
pub const REVIEWED: &[&str] = &["hbqts_ma3_20", "hbqts_stop_entry", "hbqts_scale_in", "hbqts_scale_out", "hbqts_parabolic_stop", "hbqts_chandelier", "hbqts_ma6_35", "hbqts_ma45_26", "hbqts_tbond_overnight", "hbqts_donchian", "hbqts_apz", "hbqts_tdom", "hbqts_opex", "hbqts_moon", "hbqts_pattern33", "hbqts_pattern15", "hbqts_pattern111", "hbqts_common_signal", "hbqts_ma8_7", "hbqts_weekly_pir", "hbqts_macd_weekly", "hbqts_dual_bb", "hbqts_noise0", "hbqts_ma8_7_atr_stop", "hbqts_equity_alloc", "hbqts_ma11_50"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

// ------------------------------------------------------------------ AFL functions

/// AMA(array, factor), seeded with the first valid value.
fn ama(v: &[f64], f: f64) -> Vec<f64> {
    let mut out = vec![f64::NAN; v.len()];
    let mut prev = f64::NAN;
    for (i, &x) in v.iter().enumerate() {
        if x.is_nan() { out[i] = prev; continue; }
        prev = if prev.is_nan() { x } else { f * x + (1.0 - f) * prev };
        out[i] = prev;
    }
    out
}
fn ema(v: &[f64], n: usize) -> Vec<f64> { ama(v, 2.0 / (n as f64 + 1.0)) }
fn dema(v: &[f64], n: usize) -> Vec<f64> {
    let e = ema(v, n);
    let ee = ema(&e, n);
    e.iter().zip(&ee).map(|(a, b)| 2.0 * a - b).collect()
}
/// Population standard deviation (AmiBroker StDev).
fn stdev(v: &[f64], n: usize) -> Vec<f64> {
    let mut out = vec![f64::NAN; v.len()];
    for i in n.saturating_sub(1)..v.len() {
        let w = &v[i + 1 - n..=i];
        let m = w.iter().sum::<f64>() / n as f64;
        out[i] = (w.iter().map(|x| (x - m).powi(2)).sum::<f64>() / n as f64).sqrt();
    }
    out
}
/// HHV / LLV; a window holding a missing (warm-up) value is missing, as AmiBroker's Null propagation.
fn hhv(v: &[f64], n: usize) -> Vec<f64> {
    (0..v.len()).map(|i| if i + 1 < n || v[i + 1 - n..=i].iter().any(|x| x.is_nan()) { f64::NAN } else { v[i + 1 - n..=i].iter().cloned().fold(f64::MIN, f64::max) }).collect()
}
fn llv(v: &[f64], n: usize) -> Vec<f64> {
    (0..v.len()).map(|i| if i + 1 < n || v[i + 1 - n..=i].iter().any(|x| x.is_nan()) { f64::NAN } else { v[i + 1 - n..=i].iter().cloned().fold(f64::MAX, f64::min) }).collect()
}
/// MA(C,n) as a plain window sum per bar (no running-sum drift, so exact ties between two averages stay ties).
fn sma(v: &[f64], n: usize) -> Vec<f64> {
    (0..v.len()).map(|i| if i + 1 < n { f64::NAN } else { v[i + 1 - n..=i].iter().sum::<f64>() / n as f64 }).collect()
}
fn refv(v: &[f64], k: usize) -> Vec<f64> { (0..v.len()).map(|i| if i >= k { v[i - k] } else { f64::NAN }).collect() }
fn cross(a: &[f64], b: &[f64]) -> Vec<bool> { (0..a.len()).map(|i| i > 0 && a[i] > b[i] && a[i - 1] <= b[i - 1]).collect() }
fn atr(x: &Ctx, n: usize) -> Vec<f64> { wilder(&true_range(x.r), n) }
/// Bars since the most recent true value (None before the first).
fn bars_since(b: &[bool]) -> Vec<Option<usize>> {
    let mut last = None;
    (0..b.len()).map(|i| { if b[i] { last = Some(i); } last.map(|l| i - l) }).collect()
}
fn hold(buy: &[bool], n: usize) -> Vec<bool> { bars_since(buy).iter().map(|b| b.is_some_and(|k| k >= n)).collect() }
/// AmiBroker Percentile(array, period, rank): value at position floor(rank/100 * (period-1)) of the sorted window.
fn percentile(v: &[f64], n: usize, rank: f64) -> Vec<f64> {
    (0..v.len()).map(|i| {
        if i + 1 < n { return f64::NAN; }
        let mut w: Vec<f64> = v[i + 1 - n..=i].to_vec();
        if w.iter().any(|x| x.is_nan()) { return f64::NAN; }
        w.sort_by(|a, b| a.partial_cmp(b).unwrap());
        w[((rank / 100.0) * (n as f64 - 1.0)).floor() as usize]
    }).collect()
}
fn ymd(x: &Ctx, i: usize) -> (i32, u32, u32) {
    let d = &x.r[i].date;
    (d[0..4].parse().unwrap_or(0), d[5..7].parse().unwrap_or(0), d[8..10].parse().unwrap_or(0))
}
/// Day of week, 0 = Sunday ... 5 = Friday (Sakamoto).
fn dow(y: i32, m: u32, d: u32) -> u32 {
    let t = [0, 3, 2, 5, 0, 3, 5, 1, 4, 6, 2, 4];
    let y = if m < 3 { y - 1 } else { y };
    ((y + y / 4 - y / 100 + y / 400 + t[(m - 1) as usize] + d as i32) % 7) as u32
}
/// Weekly bars (Monday-Sunday calendar weeks) built from daily bars 0..=to. Returns (week index per day, weekly close,
/// weekly high, weekly low, last daily bar of each week).
fn weeks(x: &Ctx, to: usize) -> (Vec<usize>, Vec<f64>, Vec<f64>, Vec<f64>, Vec<usize>) {
    let key = |i: usize| { let (y, m, d) = ymd(x, i); days_from_civil(y, m, d) - dow(y, m, d) as i64 + if dow(y, m, d) == 0 { -7 } else { 0 } };
    let (mut wk, mut c, mut h, mut l, mut last) = (vec![0; to + 1], vec![], vec![], vec![], vec![]);
    let mut prev = i64::MIN;
    for i in 0..=to {
        let k = key(i);
        if k != prev { c.push(x.c[i]); h.push(x.h[i]); l.push(x.l[i]); last.push(i); prev = k; }
        let w = c.len() - 1;
        c[w] = x.c[i]; h[w] = h[w].max(x.h[i]); l[w] = l[w].min(x.l[i]); last[w] = i; wk[i] = w;
    }
    (wk, c, h, l, last)
}
fn days_from_civil(y: i32, m: u32, d: u32) -> i64 {
    let y = if m <= 2 { y - 1 } else { y } as i64;
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let mp = (m as i64 + 9) % 12;
    let doy = (153 * mp + 2) / 5 + d as i64 - 1;
    era * 146097 + yoe * 365 + yoe / 4 - yoe / 100 + doy - 719468
}
/// TimeFrameExpand(weekly, inWeekly) with the default expandLast: the week's value appears on its last daily bar,
/// earlier days of that week carry the previous week's value.
fn expand_last(wv: &[f64], wk: &[usize], last: &[usize]) -> Vec<f64> {
    (0..wk.len()).map(|i| { let w = wk[i]; if last[w] == i { wv[w] } else if w > 0 { wv[w - 1] } else { f64::NAN } }).collect()
}

// ------------------------------------------------------------------ backtester

#[derive(Default)]
struct Sig { buy: Vec<bool>, sell: Vec<bool>, short: Vec<bool>, cover: Vec<bool> }
struct Px<'a> { buy: &'a dyn Fn(usize) -> f64, sell: &'a dyn Fn(usize) -> f64, short: &'a dyn Fn(usize) -> f64, cover: &'a dyn Fn(usize) -> f64 }

fn clamp(x: &Ctx, i: usize, p: f64) -> f64 { p.max(x.l[i]).min(x.h[i]) }
fn trade(dir: f64, ei: usize, epx: f64, xi: usize, xpx: f64, why: &str, kind: &'static str) -> Trade {
    Trade { dir, legs: vec![Leg { i: ei, px: epx, w: 1.0 }], exit_i: xi, exit_px: xpx, reason: why.into(), entry_kind: kind, signal_i: xi, open: false, exits: vec![] }
}

/// AmiBroker-style single-position backtest. `side` 0 = long and short, 1 = long only, 2 = short only.
/// `loss(i)` / `trail(i)` = stop amounts in points taken on the entry bar (None = no stop).
fn ami(x: &Ctx, from: usize, to: usize, s: &Sig, px: &Px, side: u8, loss: Option<&dyn Fn(usize) -> f64>, trail: Option<&dyn Fn(usize) -> f64>) -> Vec<Trade> {
    let get = |v: &Vec<bool>, i: usize| v.get(i).copied().unwrap_or(false);
    let mut out = vec![];
    let mut pos: Option<(f64, usize, f64, f64, f64, f64)> = None; // dir, entry i, entry px, loss amt, trail amt, extreme
    for i in from..=to {
        if let Some((d, ei, ep, la, ta, ext)) = pos {
            // built-in stops (from the bar after entry), max loss first, then trailing
            if i > ei {
                let mut hit: Option<(f64, &str)> = None;
                if la.is_finite() {
                    let st = ep - d * la;
                    if (d > 0.0 && x.o[i] <= st) || (d < 0.0 && x.o[i] >= st) { hit = Some((x.o[i], "max loss stop (gap)")); }
                    else if (d > 0.0 && x.l[i] <= st) || (d < 0.0 && x.h[i] >= st) { hit = Some((st, "max loss stop")); }
                }
                if hit.is_none() && ta.is_finite() {
                    let st = ext - d * ta;
                    if (d > 0.0 && x.o[i] <= st) || (d < 0.0 && x.o[i] >= st) { hit = Some((x.o[i], "trailing stop (gap)")); }
                    else if (d > 0.0 && x.l[i] <= st) || (d < 0.0 && x.h[i] >= st) { hit = Some((st, "trailing stop")); }
                }
                if let Some((p, why)) = hit { out.push(trade(d, ei, ep, i, p, why, "close")); pos = None; }
            }
        }
        if let Some((d, ei, ep, ..)) = pos {
            if d > 0.0 && get(&s.sell, i) { out.push(trade(d, ei, ep, i, clamp(x, i, (px.sell)(i)), "sell signal", "close")); pos = None; }
            else if d < 0.0 && get(&s.cover, i) { out.push(trade(d, ei, ep, i, clamp(x, i, (px.cover)(i)), "cover signal", "close")); pos = None; }
        }
        if pos.is_none() && i < to {
            let (go, p) = if side != 2 && get(&s.buy, i) { (1.0, (px.buy)(i)) } else if side != 1 && get(&s.short, i) { (-1.0, (px.short)(i)) } else { (0.0, 0.0) };
            if go != 0.0 {
                let p = clamp(x, i, p);
                pos = Some((go, i, p, loss.map_or(f64::NAN, |f| f(i)), trail.map_or(f64::NAN, |f| f(i)), p));
            }
        }
        if let Some((d, ei, ep, la, ta, ext)) = pos {
            if i > ei { // trailing extreme: highest high / lowest low from the bar after entry
                pos = Some((d, ei, ep, la, ta, if d > 0.0 { ext.max(x.h[i]) } else { ext.min(x.l[i]) }));
            }
        }
    }
    if let Some((d, ei, ep, ..)) = pos { let mut t = trade(d, ei, ep, to, x.c[to], "open", "close"); t.open = true; out.push(t); }
    out
}
fn at_close<'a>(x: &'a Ctx<'a>) -> impl Fn(usize) -> f64 + 'a { move |i| x.c[i] }

// ------------------------------------------------------------------ catalogue

const STK: &str = "The book ran this on individual US stocks (LPX, AAPL, TXN, AA, ...) 1995-2007 and reports dollars on a $10,000 account or counts of profitable stocks; no index result is printed, so there is nothing to reproduce on SPX/NIFTY. Vault: same AFL rules on the selected index, trades at the close.";
macro_rules! def {
    ($id:expr, $name:expr, $g:expr, $dir:expr, $st:expr, $rules:expr, $notes:expr) => {
        StrategyDef { id: $id, name: $name, group: $g, session: 0, dir: $dir, status: $st, needs_volume: false, needs_vix: false,
            needs_series: &[], book: BOOK, rules: $rules, notes: $notes, claim: None }
    };
}

pub fn catalogue() -> Vec<StrategyDef> {
    use Status::*;
    const G5: &str = "Ch.5-7 AmiBroker examples, entries and exits";
    const G9: &str = "Ch.9-10 Trending and mean-reversion systems";
    const G11: &str = "Ch.11-13 Seasonality, patterns, anticipation";
    const G14: &str = "Ch.14-17 Sectors, rotation, portfolios, filters";
    const G20: &str = "Ch.20-22, App. A Walk forward, Monte Carlo, metrics";
    let mut definitions = vec![
        def!("hbqts_ma3_20", "Example trading system: 3/20 MA crossover (Fig 6.8)", G5, "Long", Live,
            &["E|FMA = MA(C,3); SMA = MA(C,20); Buy = Cross(FMA, SMA) (p.64)", "X|Sell = Cross(SMA, FMA) (p.64)", "F|Settings: Buy/Sell price Close, delay 0, positions Long (p.65-66)"], STK),
        def!("hbqts_ma5_25", "Enter at market: 5/25 MA crossover on the close (Fig 7.1)", G5, "Long", Live,
            &["F|SetTradeDelays(0,0,0,0); BuyPrice = C; SellPrice = C (p.89)", "E|Buy = Cross(MA(C,5), MA(C,25)) (p.89)", "X|Sell = Cross(MA2, MA1) (p.89)"], STK),
        def!("hbqts_limit_entry", "Enter on a limit 1% below the signal close (Fig 7.2)", G5, "Long", Live,
            &["F|BuySig = Cross(MA(C,5), MA(C,25)) (p.90)", "E|Buy = (Ref(BuySig,-1)==1) AND (L < 0.99*Ref(C,-1)); BuyPrice = 0.99*Ref(C,-1) (p.90)", "X|Sell = Cross(MA2, MA1) at the close (p.90)"], STK),
        def!("hbqts_stop_entry", "Enter at stop: 20-day high breakout, 10-day low exit (Fig 7.3)", G5, "Long", Live,
            &["E|Buy = H > Ref(HHV(H,20),-1); BuyPrice = Ref(HHV(H,20),-1) (p.90)", "X|Sell = L < Ref(LLV(L,10),-1); SellPrice = Ref(LLV(L,10),-1) (p.90)"],
            "Stop prices outside the day's range are moved to the range (AmiBroker backtester). Book: no result printed."),
        def!("hbqts_fixed_date", "Enter on a specific date (Fig 7.4)", G5, "Long", NotTestable("debugging harness: forces a buy on 23 Jun 1995 and a sale on 12 Sep 2002 to check code; not a trading rule"),
            &["E|Buy = DateNum()==950623; Sell = DateNum()==1020912 (p.91)"], ""),
        def!("hbqts_random_entry", "Random entry benchmark (Fig 7.5, 22.14, 22.16)", G5, "Long", NotTestable("random entries (AmiBroker Random()) by design - no fixed rule to reproduce"),
            &["E|Buy = Random() < Frequency/252, Sell = BarsSince(Buy) >= HoldBars (p.91-92, 307-309)"], ""),
        def!("hbqts_fixed_interval", "Enter every 6th bar, hold 3 (Fig 7.6)", G5, "Long", Live,
            &["E|Buy = (BarIndex() % 6) == 0 at the close (p.92)", "X|Sell = BarsSince(Buy) >= 3 (p.92)"],
            "BarIndex counts from the first bar of the instrument's loaded history (as in AmiBroker)."),
        def!("hbqts_scale_in", "Scale in on 5-day highs, exit on 20-day low (Fig 7.7)", G5, "Long", Live,
            &["E|Buy (sigScaleIn) = H > Ref(HHV(H,5),-1) at the close; \"The initial position is 25% of available equity. Every time there is a new five day high, add another 25%\" (p.93)",
              "A|Each further 5-day-high bar adds a 25% leg at the close (at most 4 legs = 100%)", "X|Sell = L < Ref(LLV(L,20),-1) at the close (p.93)"],
            "Legs are 25% of the initial allocation (the book's PositionSize = -25 is 25% of current equity; with 4 legs the account is fully invested and further signals are skipped). The book: \"The logic is probably foolish, but the code illustrates the concept\"."),
        def!("hbqts_profit_target", "10% profit target, no other exit (Fig 7.9)", G5, "Long", Live,
            &["E|Buy = Cross(MA(C,5), MA(C,25)) at the close (p.96)", "X|Sell when H > EntryPrice * 1.10 at EntryPrice * 1.10 (p.96)", "F|\"there is no exit other than the profit target\" (p.95-96)"], STK),
        def!("hbqts_scale_out", "Scale out at +10%/+20% with 10% trailing stop (Fig 7.10)", G5, "Long", Live,
            &["E|Buy = Cross(MA(C,5), MA(C,25)) at the close (p.97)", "X|First target: High >= 1.10 x buy price -> sell 50% of the position at that price (p.98)",
              "X|Second target: High >= 1.20 x buy price -> sell the rest at Max(Open, target) (p.98)",
              "X|Trailing stop: Low <= 0.90 x highest high since buy -> sell all at Min(Open, stop); on a bar where it fires the scale-out is cancelled (p.98)"],
            "The book sizes 50% of equity per position (SetPositionSize(50, spsPercentOfEquity)); the vault uses its own allocation."),
        def!("hbqts_parabolic_stop", "AMA crossover entry with parabolic trailing stop (Fig 7.11)", G5, "Long", Live,
            &["E|MAvg = AMA(C, 2/(30+1)); Pass = C >= MA(C,15); Buy = Pass AND Cross(C, MAvg) at the close (p.99-100)",
              "X|Parabolic stop: starts at LLV(Low,1) of the entry bar, IAF 0.02 rising 0.02 per new high to 0.20; exit when Low < yesterday's SAR at the close (TradeAtStop = 0) (p.100-101)"], STK),
        def!("hbqts_chandelier", "Chandelier stop 3 x ATR(10) below the highest high (Fig 7.12)", G5, "Long", Live,
            &["E|Buy = Cross(MA(C,5), MA(C,25)) at the close (p.102)", "X|TradeExitPrice = HighestSince(Buy, H - 3*ATR(10)); Sell = L <= TradeExitPrice, SellPrice = C (p.102)"], STK),
        def!("hbqts_chandelier_1line", "Chandelier stop in one line (Fig 7.13)", G5, "Long", NotTestable("an exit only (ApplyStop trailing 3*ATR(14)); the figure gives no entry rule"),
            &["X|ApplyStop(stopTypeTrailing, stopModePoint, 3*ATR(14), True, True) (p.102)"], ""),
        def!("hbqts_entry_id1", "Entry identification I: 3/16 MA cross, hold 6 days (Fig 7.14)", G5, "Long", Live,
            &["E|Buy = Cross(MA(C,3), MA(C,16)) at the close (p.104)", "X|Sell = BarsSince(Buy) >= 6 (p.104)"], STK),
        def!("hbqts_future_looking", "Entry identification II / future leak (Fig 7.16, 7.18)", G5, "Long", NotTestable("the code looks into the future on purpose (Ref(C,DaysAhead), Ref(C,2)) to illustrate ideal entries and a future leak"),
            &["E|IdealEntry = GainAhead >= 5 AND RiskAhead <= 3 (future data) (p.106); FutureLeak Buy = (Ref(C,2)-C)/C > 0.01 (p.108)"], ""),
        def!("hbqts_ma6_35", "Classic MA crossover 6/35 (Fig 9.1)", G9, "Long", Live,
            &["E|Length1 = 6, Length2 = 35; Buy = Cross(MA(C,6), MA(C,35)) (p.128)", "X|Sell = Cross(MA2, MA1) (p.128)", "F|Settings: prices Close, delays 0, positions Long (p.131-132)"],
            "Book: LPX 1/1995-1/2005, 6/35 lost $3,508 of $10,000 (p.130) - no index result."),
        def!("hbqts_ma45_26", "MA crossover with optimized lengths 45/26 (Fig 9.10)", G9, "Long", Live,
            &["E|Length1 = 45; Length2 = 26 (best LPX combination, p.132); Buy = Cross(MA(C,45), MA(C,26)) - the fast average is longer, the book notes the system \"takes a long position when the fast moving average falls below the slow\" (p.133)", "X|Sell = Cross(MA(C,26), MA(C,45)) (p.132)"], STK),
        def!("hbqts_tbond_overnight", "TBond overnight: buy/short the close in the day's direction, exit next open (Fig 9.12)", G9, "Long/Short", Live,
            &["E|Buy = C > O at the close; Short = C < O at the close (p.134-135)", "X|Sell = Ref(Buy,-1)==1 at the next Open; Cover = Ref(Short,-1)==1 at the next Open (p.135)"],
            "Book: T-bond futures, \"frictional costs now exceed the overnight continuation move\" (p.134); no numbers printed."),
        def!("hbqts_bb_breakout", "Bollinger band breakout 12 / 2.8 (Fig 9.13)", G9, "Long/Short", Live,
            &["E|Buy = Cross(C, BBandTop(C,12,2.8)); Short = Cross(BBandBot(C,12,2.8), C) (p.135)", "X|Sell = Cross(BBT, C); Cover = Cross(C, BBB) (p.135)"],
            "\"marginal for stocks and works for some commodities\" (p.135). Param `side` 0 long+short (default, code defines both), 1 long, 2 short."),
        def!("hbqts_donchian", "Donchian 20/10 breakout with 2 x ATR(20) max-loss stop (Fig 9.14)", G9, "Long/Short", Live,
            &["E|Buy = H > Ref(HHV(H,20),-1) at Ref(HHV(H,20),-1); Short = L < Ref(LLV(L,20),-1) at Ref(LLV(L,20),-1) (p.136)",
              "X|Sell = L < Ref(LLV(L,10),-1) at that level; Cover = H > Ref(HHV(H,10),-1) at that level (p.136)",
              "X|ApplyStop(stopTypeLoss, stopModePoint, 2*ATR(20), 1, False): max loss of 2 x ATR(20) at entry, exit intraday at the stop (p.136)"], STK),
        def!("hbqts_cgo", "Center of Gravity oscillator crossover, 6-day max hold (Fig 10.1)", G9, "Long", Live,
            &["F|CGO(13) on (H+L)/2; CGOSmoothed = DEMA(CGO, 2) (p.137-138)", "E|Buy = Cross(CGO, CGOSmoothed) at the close (p.138)", "X|Sell = Cross(CGOSmoothed, CGO) OR BarsSince(Buy) >= 6 (p.138)"], STK),
        def!("hbqts_stoch_rsi", "Stochastic of the RSI (Fig 10.3)", G9, "Long", Live,
            &["F|r = RSIa(C,10); pir = (r - LLV(r,20)) / (HHV(r,20) - LLV(r,20)); trigger = DEMA(pir,3) (p.140)", "E|Buy = Cross(pir, trigger) at the close (p.140)", "X|Sell = Cross(trigger, pir) (p.140)"], STK),
        def!("hbqts_apz", "Adaptive price zone with ADX filter (Fig 10.6)", G9, "Long", Live,
            &["F|DsEMA = AMA(AMA(C, s), s), s = 2/(sqrt(50)+1); RangeDsEMA likewise of H-L; DnBand = DsEMA - 2 x RangeDsEMA (p.142)",
              "E|Buy = ADX(14) <= 30 AND Low <= DnBand at the close (p.142)", "X|Sell = ADX(14) > 30 (p.142)"],
            "The TASC listing quoted on p.141 (Fig 10.5, 1-bar delays, period 20, width 1.4) is not a separate entry: its fill price setting is not stated. Book result: chart only (p.143)."),
        def!("hbqts_dpo", "Detrended price oscillator crossover (Fig 10.8)", G9, "Long", Live,
            &["F|Oscillator = C - MA(C,30); SmOsc = DEMA(Oscillator,10); Trigger = DEMA(SmOsc,3) (p.144)", "E|Buy = Cross(SmOsc, Trigger) at the close (p.144)", "X|Sell = Cross(Trigger, SmOsc) (p.144)"], STK),
        def!("hbqts_dom", "Seasonality: day of the month (Fig 11.3)", G11, "Long/Short", Live,
            &["E|Cover = Buy = (Day() >= 1 AND Day() <= 5) OR (Day() >= 10 AND Day() <= 16) OR (Day() >= 21 AND Day() <= 31), at the close (p.150-151)",
              "X|Short = Sell = (Day() >= 6 AND Day() <= 9) OR (Day() >= 17 AND Day() <= 20) (p.151)"],
            "Book (513 stocks, 1995-2005 in-sample): long-only profitable in 97%, median annual gain 23%; out-of-sample 2005-2007 83% / 18% (p.151-152). Param `side` 0 long+short (default), 1 long, 2 short."),
        def!("hbqts_tdom", "Seasonality: trading day of the month, Day -3 to Day +2 (Fig 11.8)", G11, "Long", Live,
            &["F|DOI = first trading day of the month; Daynumber = -8 + BarsSince(Ref(DOI,8)) (p.154)", "E|Buy = Daynumber >= -3 AND Daynumber <= 2, at the close (p.154)", "X|Sell = Daynumber >= 3 OR Daynumber <= -4 (p.154)"],
            "The code looks 8 bars ahead to find the next month's first trading day (\"That is OK. We will not be looking ahead when we trade\", p.154): the exchange calendar. Only bars inside the run window are used. Book: 94% of 513 stocks profitable in-sample, median 13%; out-of-sample 86% / 14% (p.154-155)."),
        def!("hbqts_opex", "Seasonality: options expiration Friday (Fig 11.11)", G11, "Long/Short", Live,
            &["F|DOI = Friday with day 15-21 (third Friday); Daynumber = -8 + BarsSince(Ref(DOI,8)) (p.156)",
              "E|Buy = Day -6..-2 or Day 2..4; Short = Day -1..1, at the close (p.156)", "X|Sell = Day -1..1 or Day >= 5; Cover = Day >= 2 (p.156)"],
            "Book: long-only 85% of 513 stocks profitable in-sample, median 10%; out-of-sample 80% / 9% (p.157-158)."),
        def!("hbqts_moon", "Seasonality: phase of the moon (Fig 11.15)", G11, "Long/Short", Live,
            &["F|Moon age as % of the 29.530588853-day cycle from the date's Julian Day (code p.160-161), Pct = int(100 x phase)",
              "E|Cover = Buy = (Pct >= 1 AND Pct <= 59) OR (Pct >= 79 AND Pct <= 99) at the close (p.161)", "X|Short = Sell = Pct >= 60 AND Pct <= 78 (p.161)"],
            "Book: in-sample 96% of stocks profitable long-only, out-of-sample long-only 55% / 2%, long+short 27% / -11% - \"no persistent pattern\" (p.162-164)."),
        def!("hbqts_pattern33", "Three-day close pattern, 33% cutoff: buy 9 strongest, short 9 weakest (Fig 12.1)", G11, "Long/Short", Live,
            &["F|PctC = (C - Ref(C,-1))/C ranked against the last 60 bars: >= Percentile(60, 67) = 3, <= Percentile(60, 33) = 1, else 2; Sequence = last three ranks (p.166)",
              "E|Buy = Sequence in {111,223,323,311,313,213,131,331,113}; Short = Sequence in {132,332,232,231,221,322,333,133,122}, at the close (p.167)",
              "X|Sell = BarsSince(Buy) >= 1; Cover = BarsSince(Short) >= 1 (p.167)"],
            "Percentile position = floor(rank/100 x 59) of the sorted window (AmiBroker's interpolation is not printed). Book: in-sample long-only 90% of stocks profitable; out-of-sample 73% / 8% (p.170-171)."),
        def!("hbqts_pattern15", "Three-day close pattern, 15% cutoff (Fig 12.1)", G11, "Long/Short", Live,
            &["F|Same ranking with Cutoff = 15 (top >= Percentile 85, bottom <= Percentile 15) (p.166, 171)",
              "E|Buy = {111,211,311,121,112,223,131,113,221}; Short = {333,331,332,132,232,133,233,123,313}, at the close (p.167-168)",
              "X|Sell = BarsSince(Buy) >= 1; Cover = BarsSince(Short) >= 1 (p.168)"], "Book: out-of-sample long-only 59% / 2% (p.172-173)."),
        def!("hbqts_pattern111", "Three down-third days in a row (sequence 111), long only (Fig 12.1)", G11, "Long", Live,
            &["E|Buy = Sequence == 111 (33% cutoff), at the close (p.168)", "X|Sell = BarsSince(Buy) >= 1 (p.168)"], ""),
        def!("hbqts_ma1_36", "MA crossover 1/36 (close vs 36-day MA) (Fig 13.2)", G11, "Long", Live,
            &["E|Buy = Cross(MA(C,1), MA(C,36)) (p.176)", "X|Sell = Cross(MA(C,36), MA(C,1)) (p.176)"], STK),
        def!("hbqts_common_signal", "Signals from a common series: 7/3 MA cross, hold 3 (Fig 14.4)", G14, "Long", Live,
            &["E|SetForeign(\"^OEX\"); Buy = Cross(MA(C,7), MA(C,3)) (p.191)", "X|Sell = BarsSince(Buy) >= 3 (p.191)"],
            "The signal series is the S&P 100 (^OEX) in the book; the vault computes it on the traded index itself (SPX / NIFTY 50)."),
        def!("hbqts_sector_zigzag", "Sector ZigZag perfect signals (Fig 14.5, 14.11, 6.13)", G14, "Long/Short", NotTestable("ZigZag uses future prices to place perfect turning points (the book: \"perfect buy and sell signals\")"),
            &["E|zz = Zig(C,10) of FSELX / FSUTX; Buy at zigzag troughs, Sell at peaks (p.191-194)"], ""),
        def!("hbqts_rotation", "Sector ETF rotation by inverted 5-day ROC (Fig 15.1)", G14, "Long", NeedsData("the nine S&P Select Sector SPDR ETFs (XLB, XLE, XLF, XLI, XLK, XLP, XLU, XLV, XLY) daily prices from 1998"),
            &["E|EnableRotationalTrading; hold 2 of the 9 sector ETFs ranked by -ROC(C,5), WorstRankHeld 5, no shorts (p.201)"], ""),
        def!("hbqts_ma8_7", "Simple portfolio example: 8/7 MA crossover (Fig 16.2)", G14, "Long", Live,
            &["E|Buy = Cross(MA(C,8), MA(C,7)) at the close (p.206)", "X|Sell = Cross(MA(C,7), MA(C,8)) (p.206)"],
            "Same rules as Fig 16.10/16.14/22.20 (those add multi-position portfolio sizing / risk sizing only)."),
        def!("hbqts_portfolio_rank", "MA 8/7 portfolio with PositionScore ranking (Fig 16.10, 16.14)", G14, "Long", NeedsData("a multi-issue portfolio (the book's stock watch list) - two simultaneous positions ranked by -ROC(C,5)"),
            &["E|MA 8/7 crossover on every issue, MaxOpenPositions = 2, PositionScore = -ROC(C,5) (p.209-214)"], ""),
        def!("hbqts_fvx", "Intermarket: stochastic RSI of the 5-year note yield (Fig 17.1, 17.3)", G14, "Long", NeedsData("5-year Treasury yield ^FVX daily (Yahoo) - not in the vault"),
            &["E|Buy = Cross(StochRSI(^FVX), DEMA(StochRSI,3)) (Fig 17.1); Fig 17.3 adds a MACD-stochastic permission filter from ^FVX (p.219-224)"], ""),
        def!("hbqts_weekly_pir", "Weekly and daily position-in-range (Fig 17.5)", G14, "Long", Live,
            &["F|PIRDaily = (C - LLV(C,20)) / (HHV(C,20) - LLV(C,20)); PIRWeekly the same over 5 weekly closes, TimeFrameExpand (p.225)",
              "E|Buy = PIRWeekly <= 0.10 AND Ref(PIRDaily,-1) <= 0.15 AND PIRDaily > Ref(PIRDaily,-1), at the close (p.225)", "X|Sell = BarsSince(Buy) >= 3 (p.225)"],
            "\"This is not a good trading system. Its purpose is to illustrate\" (code comment). Weekly bars = calendar weeks; expandLast puts a week's value on its last bar."),
        def!("hbqts_macd_daily", "MACD-slope permission filter on daily data + 8/7 MA (Fig 17.6)", G14, "Long", Live,
            &["F|SmDiff = EMA(EMA(C,5) - EMA(C,37), 7); Permit = SmDiff > Ref(SmDiff,-1) (p.226)", "E|Buy = Cross(MA(C,8), MA(C,7)) AND Permit (p.227)", "X|Sell = Cross(MA(C,7), MA(C,8)) OR NOT Permit (p.227)"],
            "Book: 100 NASDAQ-100 stocks 1995-2005, \"profitable for most stocks with median RAR about 20%\" (p.227)."),
        def!("hbqts_macd_weekly", "MACD-slope permission filter on weekly data + 8/7 MA (Fig 17.7)", G14, "Long", Live,
            &["F|Weekly: SmDiff = EMA(EMA(C,19) - EMA(C,23), 14); Permit = SmDiff > Ref(SmDiff,-1), expanded to daily (p.227-228)", "E|Buy = Cross(MA(C,8), MA(C,7)) AND Permit (p.228)", "X|Sell = Cross(MA(C,7), MA(C,8)) OR NOT Permit (p.228)"], ""),
        def!("hbqts_dual_bb", "Dual time frame Bollinger bands (Fig 17.8)", G14, "Long/Short", Live,
            &["F|BBPctWeekly over BBands(8, 1.1) of weekly closes (expanded); BBPctDaily over BBands(5, 1.1) (p.228-229)",
              "E|Buy = BBPctWeekly >= 0.90 AND BBPctDaily >= 0.90; Short = BBPctWeekly <= 0.10 AND BBPctDaily < 0.10, at the close (p.229)",
              "X|Sell = BBPctDaily <= 0.10; Cover = BBPctDaily >= 0.90 (p.229)"], ""),
        def!("hbqts_diffusion", "Advance-decline diffusion index of the S&P 500 members (Fig 17.11)", G14, "Long", NeedsData("daily advancing/declining counts of the 500 current S&P 500 member stocks (AddToComposite over the watch list); NYSE/NSE-wide breadth is a different population"),
            &["E|ADO = (AdvIss - DecIss)/Count; Buy = Cross(DEMA(ADO,24), DEMA(DEMA(ADO,24),3)); Sell = the opposite cross (p.235)"], ""),
        def!("hbqts_zscore_diffusion", "Z-score diffusion index of energy stocks (Fig 17.13, 17.15)", G14, "Long", NeedsData("the component stocks of XLE and the book's ExampleZScore plug-in composite"),
            &["E|Buy = Cross(DEMA(avg z-score,20), DEMA(...,3)) (p.237-240)"], ""),
        def!("hbqts_cmo", "CMO oscillator crossover with 6-day exit (Fig 20.2)", G20, "Long", Live,
            &["F|CMO(21) = 100 x (SumUp - SumDown)/(SumUp + SumDown); CMOAvg = DEMA(CMO, 96); Trigger = DEMA(CMOAvg, 3) (p.263)",
              "E|Buy = Cross(CMOAvg, Trigger) at the close (p.264)", "X|Sell = Cross(Trigger, CMOAvg) OR BarsSince(Buy) >= 6; then ExRem(Buy,Sell), ExRem(Sell,Buy) (p.264)"],
            "Defaults = the Optimize() defaults printed in the code (21, 96)."),
        def!("hbqts_noise0", "Add-noise test system at zero noise: close vs EMA(4), hold 1 (Fig 22.7)", G20, "Long", Live,
            &["E|MA1 = DEMA(NoisyClose,1) (= the close); MA2 = EMA(NoisyClose,4); Buy = Cross(MA1, MA2) (p.304)", "X|Sell = BarsSince(Buy) >= 1 (p.304)",
              "F|Weight = Optimize(\"Weight\", 0.0, ...): the default adds no noise (p.303)"],
            "Only the zero-noise default is run: noise weights > 0 add random (synthetic) prices, which the vault never uses."),
        def!("hbqts_ma8_7_atr_stop", "8/7 MA with 2 x ATR(20) max-loss stop (Fig 22.21)", G20, "Long", Live,
            &["E|Buy = Cross(MA(C,8), MA(C,7)) (p.312)", "X|Sell = Cross(MA(C,7), MA(C,8)) (p.312)", "X|ApplyStop(stopTypeLoss, stopModePoint, 2.0*ATR(20)) - exit intraday at the stop (p.312)"],
            "The book sizes each trade to risk 2% of equity; the vault uses its own allocation."),
        def!("hbqts_equity_alloc", "Equity allocation example: 8/7 MA, loss and chandelier stops, 100-day exit (Fig 22.22)", G20, "Long", Live,
            &["E|Buy = Cross(MA(C,8), MA(C,7)) (p.313)", "X|Sell = Cross(MA(C,7), MA(C,8)) OR BarsSince(Buy) >= 100 (p.313)",
              "X|ApplyStop(stopTypeLoss, stopModePoint, Min(2*ATR(20), 0.02*C)) (p.313)", "X|ApplyStop(stopTypeTrailing, stopModePoint, 2*ATR(20)) - chandelier from the highest high since entry (p.313)"],
            "Sizing (min of 20% of equity and 2% risk) replaced by the vault allocation."),
        def!("hbqts_ma11_50", "Custom-metric example: 11/50 MA crossover (Fig A.1)", G20, "Long", Live,
            &["E|fast = 11, slow = 50 (Optimize defaults); Buy = Cross(MA(C,11), MA(C,50)) (p.318)", "X|Sell = Cross(MA(C,50), MA(C,11)) (p.318)"], ""),
        def!("hbqts_dema16_8", "Objective-function example: DEMA 16/8 cross, hold 1 (Fig A.7)", G20, "Long", Live,
            &["E|fast = 16, slow = 8 (fixed Optimize defaults); Buy = Cross(DEMA(C,16), DEMA(C,8)) (printed323)", "X|Raw Sell = Cross(DEMA(C,8), DEMA(C,16)) OR BarsSince(raw Buy) >= 1; then Sell = ExRem(Sell,Buy) (printed323)", "F|Declared daily finalized same-close fills, zero delay and normalized long unit exposure; source omits fills and sizing"],
            "Figure A.7 teaches a custom objective-function metric; the complete demonstration trading rule is reviewed separately. Printed324 illustrates actual S&P500 cash-index data; book optimization and equity figures are not reproduced. Actual SPX is primary, NIFTY50 a separately declared cash-index adaptation; stocks/ETFs are not substitutes. First-close DEMA seeds, full prefix, flat selected start and terminal open positions. Unit-return cost deductions include explicitly estimated open liquidation costs; no funded cash account or native AmiBroker binary parity. See docs/hbqts-dema-index-review.md."),
    ];
    for d in &mut definitions {
        if reviewed::IDS.contains(&d.id) && !reviewed::is_dema_index(d.id) {
            d.notes = "Source-reviewed companion AFL research contract: library/bandy/REVIEW-2026-10-13.md. Actual individual stocks only; selected AAPL/RELIANCE ledgers are independent symbol checks, not the book's universe or portfolio. Literal source defaults, explicit seed/Null conventions, no copied performance, sizing, dividends or financing. Open positions remain open. Independent source and actual-ledger review is documented there; deterministic publisher admission remains pending.";
        }
        if d.id=="hbqts_cmo" {
            d.notes="Source Figure20.2, printed pp263-264, uses XLB ETF for walk-forward testing. This reviewed default-rule application requires actual ETFs and checks SPY; it does not reproduce XLB's optimized walk-forward results. No stock or cash-index substitute, pooled sizing or book performance. See library/bandy/REVIEW-2026-10-13.md; independent source and actual-ledger review is documented there; deterministic publisher admission remains pending.";
        }
    }
    definitions
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    if !id.starts_with("hbqts_") { return None; }
    let d = catalogue().into_iter().find(|d| d.id == id)?;
    let reviewed_family = match id {
        "hbqts_ma5_25" | "hbqts_ma1_36" => Some(("hbqts_ma", "Bandy QTS · simple moving average crossover")),
        "hbqts_limit_entry" => Some(("hbqts_limit_entry", "Bandy QTS · one-day limit entry")),
        "hbqts_fixed_interval" => Some(("hbqts_fixed_interval", "Bandy QTS · fixed interval entry")),
        "hbqts_profit_target" => Some(("hbqts_profit_target", "Bandy QTS · profit target only")),
        "hbqts_entry_id1" => Some(("hbqts_entry_id1", "Bandy QTS · moving average entry with timed exit")),
        "hbqts_bb_breakout" => Some(("hbqts_bb_breakout", "Bandy QTS · Bollinger band breakout")),
        "hbqts_cgo" => Some(("hbqts_cgo", "Bandy QTS · center of gravity oscillator")),
        "hbqts_stoch_rsi" => Some(("hbqts_stoch_rsi", "Bandy QTS · stochastic RSI")),
        "hbqts_dpo" => Some(("hbqts_dpo", "Bandy QTS · detrended price oscillator")),
        "hbqts_dom" => Some(("hbqts_dom", "Bandy QTS · calendar day of month")),
        "hbqts_macd_daily" => Some(("hbqts_macd_daily", "Bandy QTS · daily MACD permission filter")),
        "hbqts_cmo" => Some(("hbqts_cmo", "Bandy QTS · CMO oscillator")),
        _ => None,
    };
    if let Some((key, name)) = reviewed_family { return Some((key, name, d.name)); }
    let f: (&'static str, &'static str) = if id.contains("ma") && !id.contains("macd") || id == "hbqts_dema16_8" || id == "hbqts_equity_alloc" {
        ("hbqts_ma", "Bandy QTS · moving average crossovers")
    } else if ["hbqts_dom", "hbqts_tdom", "hbqts_opex", "hbqts_moon"].contains(&id) {
        ("hbqts_season", "Bandy QTS · seasonality")
    } else if id.starts_with("hbqts_pattern") {
        ("hbqts_pattern", "Bandy QTS · three-day close patterns")
    } else if d.group.starts_with("Ch.5") {
        ("hbqts_entries", "Bandy QTS · entries and exits")
    } else if d.group.starts_with("Ch.14") {
        ("hbqts_filters", "Bandy QTS · filters, intermarket, portfolios")
    } else {
        ("hbqts_osc", "Bandy QTS · oscillators and breakouts")
    };
    Some((f.0, f.1, d.name))
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    let ma = |a: f64, b: f64| vec![p("fast", "MA1 length", a, 1., 100., 1.), p("slow", "MA2 length", b, 1., 200., 1.)];
    let side = |d: f64| p("side", "Positions: 0 long+short, 1 long, 2 short", d, 0., 2., 1.);
    Some(match id {
        "hbqts_ma3_20" => ma(3., 20.),
        "hbqts_ma5_25" => ma(5., 25.),
        "hbqts_limit_entry" => { let mut v = ma(5., 25.); v.push(p("pct", "Limit % below the signal close", 1., 0.25, 5., 0.25)); v }
        "hbqts_stop_entry" => vec![p("hi", "Breakout highest-high days", 20., 5., 100., 1.), p("lo", "Exit lowest-low days", 10., 2., 50., 1.)],
        "hbqts_fixed_interval" => vec![p("every", "Entry interval (bars)", 6., 4., 100., 1.), p("hold", "Hold days", 3., 1., 100., 1.)],
        "hbqts_scale_in" => vec![p("hi", "Scale-in highest-high days", 5., 2., 50., 1.), p("lo", "Exit lowest-low days", 20., 5., 100., 1.)],
        "hbqts_profit_target" => { let mut v = ma(5., 25.); v.push(p("tgt", "Profit target %", 10., 1., 50., 1.)); v }
        "hbqts_scale_out" => { let mut v = ma(5., 25.); v.extend([p("t1", "First target %", 10., 2., 50., 1.), p("t2", "Second target %", 20., 4., 100., 1.), p("trail", "Trailing stop %", 10., 2., 30., 1.)]); v }
        "hbqts_parabolic_stop" => vec![p("ama", "AMA length (MALen1)", 30., 1., 31., 1.), p("pass", "Pass MA length (MALen2)", 15., 1., 31., 2.), p("lb", "Initial stop LLV days (LBDays)", 1., 1., 10., 1.),
            p("iaf", "Acceleration factor", 0.02, 0.001, 0.1, 0.001), p("maxaf", "Max acceleration", 0.2, 0.01, 1., 0.01), p("at_stop", "TradeAtStop (0 close, 1 stop price)", 0., 0., 1., 1.)],
        "hbqts_chandelier" => { let mut v = ma(5., 25.); v.extend([p("atr", "ATR range", 10., 1., 30., 1.), p("mult", "ATR multiple", 3., 0.5, 5., 0.25)]); v }
        "hbqts_entry_id1" => { let mut v = ma(3., 16.); v.push(p("hold", "Hold days", 6., 1., 60., 1.)); v }
        "hbqts_ma6_35" => ma(6., 35.),
        "hbqts_ma45_26" => ma(45., 26.),
        "hbqts_ma1_36" => ma(1., 36.),
        "hbqts_ma8_7" => ma(8., 7.),
        "hbqts_ma11_50" => ma(11., 50.),
        "hbqts_tbond_overnight" => vec![side(0.)],
        "hbqts_bb_breakout" => vec![p("lb", "Band lookback", 12., 2., 30., 2.), p("w", "Band width (SD)", 2.8, 1., 3., 0.1), side(0.)],
        "hbqts_donchian" => vec![p("entry", "Entry breakout days", 20., 5., 100., 1.), p("exit", "Exit breakout days", 10., 2., 50., 1.), p("atr", "ATR days", 20., 5., 50., 1.), p("mult", "Stop ATR multiple", 2., 0.5, 5., 0.5), side(0.)],
        "hbqts_cgo" => vec![p("len", "CGO length", 13., 1., 250., 1.), p("sm", "DEMA smoothing", 2., 1., 20., 1.), p("hold", "Max hold days", 6., 1., 10., 1.)],
        "hbqts_stoch_rsi" => vec![p("rsi", "RSI length", 10., 2., 20., 1.), p("lb", "Position-in-range lookback", 20., 4., 40., 2.), p("trig", "Trigger smoother", 3., 2., 10., 1.)],
        "hbqts_apz" => vec![p("period", "Period", 50., 2., 100., 2.), p("band", "Band width", 2., 0.1, 4., 0.1), p("adx_th", "ADX threshold", 30., 1., 50., 1.), p("adx", "ADX length", 14., 2., 40., 2.)],
        "hbqts_dpo" => vec![p("ma", "MA length removed", 30., 2., 200., 1.), p("sm", "Oscillator DEMA", 10., 1., 50., 1.)],
        "hbqts_dom" | "hbqts_moon" | "hbqts_opex" => vec![side(0.)],
        "hbqts_tdom" => vec![p("buy_from", "Buy from day", -3., -8., 0., 1.), p("buy_to", "Buy through day", 2., 0., 8., 1.)],
        "hbqts_pattern33" | "hbqts_pattern15" => vec![p("lb", "Ranking lookback bars", 60., 20., 250., 10.), side(0.)],
        "hbqts_pattern111" => vec![p("lb", "Ranking lookback bars", 60., 20., 250., 10.), p("cutoff", "Cutoff %", 33., 5., 45., 1.)],
        "hbqts_common_signal" => vec![p("fast", "MA1 length", 7., 1., 50., 1.), p("slow", "MA2 length", 3., 1., 50., 1.), p("hold", "Hold days", 3., 1., 20., 1.)],
        "hbqts_weekly_pir" => vec![p("dlb", "Daily lookback", 20., 5., 60., 1.), p("wlb", "Weekly lookback", 5., 2., 20., 1.), p("wl", "Weekly level", 0.10, 0.0, 0.5, 0.05),
            p("dl", "Daily level", 0.15, 0.0, 0.5, 0.05), p("hold", "Hold days", 3., 1., 20., 1.)],
        "hbqts_macd_daily" => vec![p("p", "EMA P", 5., 3., 36., 2.), p("q", "EMA Q", 37., 3., 52., 2.), p("r", "Smoothing R", 7., 3., 15., 1.)],
        "hbqts_macd_weekly" => vec![p("p", "Weekly EMA P", 19., 3., 36., 2.), p("q", "Weekly EMA Q", 23., 3., 52., 2.), p("r", "Weekly smoothing R", 14., 3., 15., 1.)],
        "hbqts_dual_bb" => vec![p("wlb", "Weekly lookback", 8., 2., 10., 2.), p("wbw", "Weekly band width", 1.1, 1., 2.5, 0.1), p("dlb", "Daily lookback", 5., 5., 30., 5.), p("dbw", "Daily band width", 1.1, 1., 2.5, 0.1), side(0.)],
        "hbqts_cmo" => vec![p("pds", "CMO periods", 21., 1., 101., 10.), p("avg", "CMO DEMA smoothing", 96., 1., 101., 5.), p("hold", "Max hold days", 6., 1., 20., 1.)],
        "hbqts_noise0" => vec![p("ema", "EMA length", 4., 1., 10., 1.), p("hold", "Hold days", 1., 1., 5., 1.)],
        "hbqts_ma8_7_atr_stop" => vec![p("fast", "MA1 length", 8., 1., 20., 1.), p("slow", "MA2 length", 7., 1., 20., 1.), p("mult", "Stop ATR(20) multiple", 2., 0.5, 5., 0.5)],
        "hbqts_equity_alloc" => vec![p("fast", "MA1 length", 8., 1., 20., 1.), p("slow", "MA2 length", 7., 1., 20., 1.), p("mult", "ATR multiple", 2., 0.5, 5., 0.5),
            p("risk", "Max risk % of price", 2., 0.5, 10., 0.5), p("hold", "Hold days", 100., 1., 200., 1.)],
        "hbqts_dema16_8" => vec![p("fast", "DEMA first length", 16., 16., 16., 1.), p("slow", "DEMA second length", 8., 8., 8., 1.), p("hold", "Bars since raw Buy", 1., 1., 1., 1.)],
        _ => return None,
    })
}

// ------------------------------------------------------------------ run

/// Moon phase per the book's code (p.160-161), 0..1, new moon = 0.
fn moon_phase(y: i32, m: u32, d: u32) -> f64 {
    let (y, m, d) = (y as f64, m as f64, d as f64);
    let yy = y - ((12.0 - m) / 10.0).floor();
    let mut mm = m + 9.0;
    if mm >= 12.0 { mm -= 12.0; }
    let k1 = (365.25 * (yy + 4712.0)).floor();
    let k2 = (30.6 * mm + 0.5).floor();
    let k3 = ((yy / 100.0 + 49.0).floor() * 0.75).floor() - 38.0;
    let mut jd = k1 + k2 + d + 59.0;
    if jd > 2299160.0 { jd -= k3; }
    let v = (jd - 2451550.1) / 29.530588853 - 0.00407;
    let v = v - v.floor();
    if v < 0.0 { v + 1.0 } else { v }
}

/// Daynumber = -8 + BarsSince(Ref(DOI, 8)) computed from bars 0..=to only.
fn daynumber(doi: &[bool], to: usize) -> Vec<Option<i64>> {
    let n = to + 1;
    let minus8: Vec<bool> = (0..n).map(|i| i + 8 <= to && doi[i + 8]).collect();
    bars_since(&minus8).iter().map(|b| b.map(|k| k as i64 - 8)).collect()
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !id.starts_with("hbqts_") { return None; }
    let schema = params(id)?;
    // This separately reviewed close-entry rule retains a true terminal entry,
    // including a one-observation selection with its genuine prior prefix.
    if reviewed::is_dema_index(id) { return reviewed::run(id,x,from,to,p); }
    if from >= to || to >= x.r.len() { return Some(vec![]); }
    if reviewed::IDS.contains(&id) { return reviewed::run(id, x, from, to, p); }
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).map_or(f64::NAN, |d| d.def));
    let u = |k: &str| g(k).round().max(1.0) as usize;
    let side = if schema.iter().any(|d| d.key == "side") { g("side").round().clamp(0.0, 2.0) as u8 } else { 1 };
    let n = to + 1;
    let c = &x.c[..n];
    let (h, l, o) = (&x.h[..n], &x.l[..n], &x.o[..n]);
    let close = at_close(x);
    let pc = Px { buy: &close, sell: &close, short: &close, cover: &close };
    let mac = |a: usize, b: usize| (sma(c, a), sma(c, b));
    let ma_x = |a: usize, b: usize| { let (m1, m2) = mac(a, b); Sig { buy: cross(&m1, &m2), sell: cross(&m2, &m1), ..Default::default() } };
    let run_long = |s: Sig| ami(x, from, to, &s, &pc, 1, None, None);
    Some(match id {
        "hbqts_ma3_20" | "hbqts_ma5_25" | "hbqts_ma6_35" | "hbqts_ma45_26" | "hbqts_ma1_36" | "hbqts_ma8_7" | "hbqts_ma11_50" => run_long(ma_x(u("fast"), u("slow"))),
        "hbqts_limit_entry" => {
            let (m1, m2) = mac(u("fast"), u("slow"));
            let sig = cross(&m1, &m2);
            let k = 1.0 - g("pct") / 100.0;
            let buy: Vec<bool> = (0..n).map(|i| i > 0 && sig[i - 1] && l[i] < k * c[i - 1]).collect();
            let bp = |i: usize| k * c[i - 1];
            ami(x, from, to, &Sig { buy, sell: cross(&m2, &m1), ..Default::default() }, &Px { buy: &bp, sell: &close, short: &close, cover: &close }, 1, None, None)
        }
        "hbqts_stop_entry" => {
            let (hh, ll) = (refv(&hhv(h, u("hi")), 1), refv(&llv(l, u("lo")), 1));
            let buy: Vec<bool> = (0..n).map(|i| h[i] > hh[i]).collect();
            let sell: Vec<bool> = (0..n).map(|i| l[i] < ll[i]).collect();
            ami(x, from, to, &Sig { buy, sell, ..Default::default() }, &Px { buy: &|i| hh[i], sell: &|i| ll[i], short: &close, cover: &close }, 1, None, None)
        }
        "hbqts_fixed_interval" => {
            let buy: Vec<bool> = (0..n).map(|i| i % u("every") == 0).collect();
            let sell = hold(&buy, u("hold"));
            run_long(Sig { buy, sell, ..Default::default() })
        }
        "hbqts_entry_id1" => { let s = ma_x(u("fast"), u("slow")); let sell = hold(&s.buy, u("hold")); run_long(Sig { sell, ..s }) }
        "hbqts_common_signal" => { let s = ma_x(u("fast"), u("slow")); let sell = hold(&s.buy, u("hold")); run_long(Sig { sell, ..s }) }
        "hbqts_scale_in" => {
            let hh = refv(&hhv(h, u("hi")), 1);
            let ll = refv(&llv(l, u("lo")), 1);
            let mut out = vec![];
            let mut cur: Option<Trade> = None;
            for i in from..=to {
                if let Some(mut t) = cur.take() {
                    if l[i] < ll[i] { t.exit_i = i; t.exit_px = c[i]; t.signal_i = i; t.reason = "20-day low breakout (close)".into(); out.push(t); }
                    else { cur = Some(t); }
                }
                if i < to && h[i] > hh[i] {
                    match cur.as_mut() {
                        None => cur = Some(Trade { dir: 1.0, legs: vec![Leg { i, px: c[i], w: 0.25 }], exit_i: i, exit_px: c[i], reason: String::new(), entry_kind: "close", signal_i: i, open: false, exits: vec![] }),
                        Some(t) if t.legs.len() < 4 && t.legs.last().unwrap().i < i => t.legs.push(Leg { i, px: c[i], w: 0.25 }),
                        _ => {}
                    }
                }
            }
            if let Some(mut t) = cur { t.exit_i = to; t.exit_px = c[to]; t.signal_i = to; t.reason = "open".into(); t.open = true; out.push(t); }
            out
        }
        "hbqts_profit_target" => {
            let s = ma_x(u("fast"), u("slow"));
            let k = 1.0 + g("tgt") / 100.0;
            let mut out = vec![];
            let mut pos: Option<(usize, f64)> = None;
            for i in from..=to {
                match pos {
                    None => if s.buy[i] && i < to { pos = Some((i, c[i])); },
                    Some((ei, ep)) => if h[i] > ep * k { out.push(trade(1.0, ei, ep, i, clamp(x, i, ep * k), "profit target", "close")); pos = None; },
                }
            }
            if let Some((ei, ep)) = pos { let mut t = trade(1.0, ei, ep, to, c[to], "open", "close"); t.open = true; out.push(t); }
            out
        }
        "hbqts_scale_out" => {
            let s = ma_x(u("fast"), u("slow"));
            let (t1, t2, tr) = (1.0 + g("t1") / 100.0, 1.0 + g("t2") / 100.0, 1.0 - g("trail") / 100.0);
            let mut out = vec![];
            let mut st: Option<(usize, f64, f64, u8, Vec<(usize, f64, f64)>)> = None; // entry i, px, high since, exit stage, pieces
            for i in from..=to {
                if let Some((ei, pb, mut hs, mut ex, mut pieces)) = st.take() {
                    hs = hs.max(h[i]);
                    let mut scale = None;
                    if ex == 0 && h[i] >= t1 * pb { ex = 1; scale = Some(t1 * pb); }
                    let mut sp = f64::NAN;
                    if ex == 1 && h[i] >= t2 * pb { ex = 2; sp = o[i].max(t2 * pb); }
                    if l[i] <= tr * hs { ex = 3; sp = o[i].min(tr * hs); scale = None; }
                    if let Some(px) = scale { pieces.push((i, clamp(x, i, px), 0.5)); }
                    if ex >= 2 {
                        let left = 1.0 - pieces.iter().map(|q| q.2).sum::<f64>();
                        pieces.push((i, clamp(x, i, sp), left));
                        let avg = pieces.iter().map(|q| q.1 * q.2).sum::<f64>();
                        let mut t = trade(1.0, ei, pb, i, avg, if ex == 3 { "trailing stop" } else { "second target" }, "close");
                        if pieces.len() > 1 { t.exits = pieces; }
                        out.push(t);
                    } else { st = Some((ei, pb, hs, ex, pieces)); }
                    continue;
                }
                if s.buy[i] && i < to { st = Some((i, c[i], 0.0, 0, vec![])); }
            }
            if let Some((ei, pb, _, _, mut pieces)) = st {
                let left = 1.0 - pieces.iter().map(|q| q.2).sum::<f64>();
                pieces.push((to, c[to], left));
                let avg = pieces.iter().map(|q| q.1 * q.2).sum::<f64>();
                let mut t = trade(1.0, ei, pb, to, avg, "open", "close");
                if pieces.len() > 1 { t.exits = pieces; }
                t.open = true; out.push(t);
            }
            out
        }
        "hbqts_parabolic_stop" => {
            let mavg = ama(c, 2.0 / (u("ama") as f64 + 1.0));
            let pass = sma(c, u("pass"));
            let cr = cross(c, &mavg);
            let buy: Vec<bool> = (0..n).map(|i| c[i] >= pass[i] && cr[i]).collect();
            let lp = llv(l, u("lb"));
            let (iaf, maxaf, at_stop) = (g("iaf"), g("maxaf"), g("at_stop").round() >= 1.0);
            let mut out = vec![];
            let (mut mp, mut af, mut hp, mut psar_prev) = (false, iaf, 0.0, f64::NAN);
            let mut ent = (0usize, 0.0f64);
            for i in from.max(2)..=to {
                let mut psar = f64::NAN;
                if mp && l[i] < psar_prev {
                    let px = if at_stop { clamp(x, i, psar_prev) } else { c[i] };
                    out.push(trade(1.0, ent.0, ent.1, i, px, "parabolic stop", "close"));
                    mp = false;
                }
                if mp {
                    if h[i] > hp { hp = h[i]; af = (af + iaf).min(maxaf); }
                    psar = psar_prev + af * (hp - psar_prev);
                }
                if !mp && buy[i] && i < to {
                    ent = (i, c[i]); psar = lp[i]; hp = h[i]; af = iaf; mp = true;
                }
                psar_prev = psar;
            }
            if mp { let mut t = trade(1.0, ent.0, ent.1, to, c[to], "open", "close"); t.open = true; out.push(t); }
            out
        }
        "hbqts_chandelier" => {
            let s = ma_x(u("fast"), u("slow"));
            let a = atr(x, u("atr"));
            let k = g("mult");
            let mut out = vec![];
            let mut pos: Option<(usize, f64)> = None;
            let mut last_buy: Option<usize> = None;
            let mut best = f64::NAN;
            for i in from..=to {
                if s.buy[i] { last_buy = Some(i); best = f64::NAN; }
                let this = h[i] - k * a[i];
                if last_buy.is_some() { best = if best.is_nan() { this } else { best.max(this) }; }
                if let Some((ei, ep)) = pos {
                    if l[i] <= best { out.push(trade(1.0, ei, ep, i, c[i], "chandelier stop (close)", "close")); pos = None; }
                }
                if pos.is_none() && s.buy[i] && i < to && !(l[i] <= best) { pos = Some((i, c[i])); }
            }
            if let Some((ei, ep)) = pos { let mut t = trade(1.0, ei, ep, to, c[to], "open", "close"); t.open = true; out.push(t); }
            out
        }
        "hbqts_tbond_overnight" => {
            let buy: Vec<bool> = (0..n).map(|i| c[i] > o[i]).collect();
            let short: Vec<bool> = (0..n).map(|i| c[i] < o[i]).collect();
            let sell: Vec<bool> = (0..n).map(|i| i > 0 && buy[i - 1]).collect();
            let cover: Vec<bool> = (0..n).map(|i| i > 0 && short[i - 1]).collect();
            let op = |i: usize| x.o[i];
            ami(x, from, to, &Sig { buy, sell, short, cover }, &Px { buy: &close, sell: &op, short: &close, cover: &op }, side, None, None)
        }
        "hbqts_bb_breakout" => {
            let (lb, w) = (u("lb"), g("w"));
            let (m, sd) = (sma(c, lb), stdev(c, lb));
            let top: Vec<f64> = m.iter().zip(&sd).map(|(a, b)| a + w * b).collect();
            let bot: Vec<f64> = m.iter().zip(&sd).map(|(a, b)| a - w * b).collect();
            ami(x, from, to, &Sig { buy: cross(c, &top), sell: cross(&top, c), short: cross(&bot, c), cover: cross(c, &bot) }, &pc, side, None, None)
        }
        "hbqts_donchian" => {
            let (e, xx) = (u("entry"), u("exit"));
            let (h20, l20, h10, l10) = (refv(&hhv(h, e), 1), refv(&llv(l, e), 1), refv(&hhv(h, xx), 1), refv(&llv(l, xx), 1));
            let a = atr(x, u("atr"));
            let k = g("mult");
            let sig = Sig { buy: (0..n).map(|i| h[i] > h20[i]).collect(), sell: (0..n).map(|i| l[i] < l10[i]).collect(),
                short: (0..n).map(|i| l[i] < l20[i]).collect(), cover: (0..n).map(|i| h[i] > h10[i]).collect() };
            let loss = |i: usize| k * a[i];
            ami(x, from, to, &sig, &Px { buy: &|i| h20[i], sell: &|i| l10[i], short: &|i| l20[i], cover: &|i| h10[i] }, side, Some(&loss), None)
        }
        "hbqts_cgo" => {
            let len = u("len");
            let price: Vec<f64> = (0..n).map(|i| (h[i] + l[i]) / 2.0).collect();
            let mut cgo = vec![0.0; n];
            for i in len..n {
                let (mut num, mut den) = (0.0, 0.0);
                for j in 0..len { num += (1 + j) as f64 * price[i - j]; den += price[i - j]; }
                if den != 0.0 { cgo[i] = 100.0 * (-num / den + (len as f64 + 1.0) / 2.0); }
            }
            let sm = dema(&cgo, u("sm"));
            let buy = cross(&cgo, &sm);
            let hd = hold(&buy, u("hold"));
            let dn = cross(&sm, &cgo);
            run_long(Sig { sell: (0..n).map(|i| dn[i] || hd[i]).collect(), buy, ..Default::default() })
        }
        "hbqts_stoch_rsi" => {
            let r = rsi(c, u("rsi"));
            let lb = u("lb");
            let (hr, lr) = (hhv(&r, lb), llv(&r, lb));
            let pir: Vec<f64> = (0..n).map(|i| (r[i] - lr[i]) / (hr[i] - lr[i])).map(|v| if v.is_finite() { v } else { f64::NAN }).collect();
            let trig = dema(&pir, u("trig"));
            run_long(Sig { buy: cross(&pir, &trig), sell: cross(&trig, &pir), ..Default::default() })
        }
        "hbqts_apz" => {
            let period = g("period");
            let sm = 2.0 / ((if period < 0.0 { 1.0 } else { period.sqrt() }) + 1.0);
            let ds = ama(&ama(c, sm), sm);
            let rng: Vec<f64> = (0..n).map(|i| h[i] - l[i]).collect();
            let rds = ama(&ama(&rng, sm), sm);
            let dn: Vec<f64> = (0..n).map(|i| ds[i] - g("band") * rds[i]).collect();
            let adx = dmi(&x.r[..n], u("adx")).1;
            let th = g("adx_th");
            run_long(Sig { buy: (0..n).map(|i| adx[i] <= th && l[i] <= dn[i]).collect(), sell: (0..n).map(|i| adx[i] > th).collect(), ..Default::default() })
        }
        "hbqts_dpo" => {
            let m = sma(c, u("ma"));
            let osc: Vec<f64> = (0..n).map(|i| c[i] - m[i]).collect();
            let so = dema(&osc, u("sm"));
            let tr = dema(&so, 3);
            run_long(Sig { buy: cross(&so, &tr), sell: cross(&tr, &so), ..Default::default() })
        }
        "hbqts_dom" => {
            let d: Vec<u32> = (0..n).map(|i| ymd(x, i).2).collect();
            let buy: Vec<bool> = d.iter().map(|&d| (1..=5).contains(&d) || (10..=16).contains(&d) || (21..=31).contains(&d)).collect();
            let short: Vec<bool> = d.iter().map(|&d| (6..=9).contains(&d) || (17..=20).contains(&d)).collect();
            ami(x, from, to, &Sig { buy: buy.clone(), sell: short.clone(), short, cover: buy }, &pc, side, None, None)
        }
        "hbqts_tdom" => {
            let doi: Vec<bool> = (0..n).map(|i| i > 0 && ymd(x, i).1 != ymd(x, i - 1).1).collect();
            let dn = daynumber(&doi, to);
            let (a, b) = (g("buy_from").round() as i64, g("buy_to").round() as i64);
            let buy: Vec<bool> = dn.iter().map(|d| d.is_some_and(|d| d >= a && d <= b)).collect();
            let sell: Vec<bool> = dn.iter().map(|d| d.is_some_and(|d| d >= b + 1 || d <= a - 1)).collect();
            run_long(Sig { buy, sell, ..Default::default() })
        }
        "hbqts_opex" => {
            let doi: Vec<bool> = (0..n).map(|i| { let (y, m, d) = ymd(x, i); dow(y, m, d) == 5 && (15..=21).contains(&d) }).collect();
            let dn = daynumber(&doi, to);
            let f = |p: &dyn Fn(i64) -> bool| dn.iter().map(|d| d.is_some_and(|d| p(d))).collect::<Vec<bool>>();
            let sig = Sig { buy: f(&|d| (-6..=-2).contains(&d) || (2..=4).contains(&d)), sell: f(&|d| (-1..=1).contains(&d) || d >= 5),
                short: f(&|d| (-1..=1).contains(&d)), cover: f(&|d| d >= 2) };
            ami(x, from, to, &sig, &pc, side, None, None)
        }
        "hbqts_moon" => {
            let pct: Vec<i64> = (0..n).map(|i| { let (y, m, d) = ymd(x, i); (100.0 * moon_phase(y, m, d)) as i64 }).collect();
            let buy: Vec<bool> = pct.iter().map(|&p| (1..=59).contains(&p) || (79..=99).contains(&p)).collect();
            let short: Vec<bool> = pct.iter().map(|&p| (60..=78).contains(&p)).collect();
            ami(x, from, to, &Sig { buy: buy.clone(), sell: short.clone(), short, cover: buy }, &pc, side, None, None)
        }
        "hbqts_pattern33" | "hbqts_pattern15" | "hbqts_pattern111" => {
            let cutoff = match id { "hbqts_pattern15" => 15.0, "hbqts_pattern111" => g("cutoff"), _ => 33.0 };
            let lb = u("lb");
            let pctc: Vec<f64> = (0..n).map(|i| if i == 0 { f64::NAN } else { (c[i] - c[i - 1]) / c[i] }).collect();
            let (top, bot) = (percentile(&pctc, lb, 100.0 - cutoff), percentile(&pctc, lb, cutoff));
            let pos: Vec<i64> = (0..n).map(|i| if pctc[i] >= top[i] { 3 } else if !(pctc[i] <= bot[i]) { 2 } else { 1 }).collect();
            let valid = |i: usize| i >= 2 && top[i - 2].is_finite();
            let seq: Vec<i64> = (0..n).map(|i| if valid(i) { 100 * pos[i - 2] + 10 * pos[i - 1] + pos[i] } else { 0 }).collect();
            let (bs, ss): (&[i64], &[i64]) = match id {
                "hbqts_pattern33" => (&[111, 223, 323, 311, 313, 213, 131, 331, 113], &[132, 332, 232, 231, 221, 322, 333, 133, 122]),
                "hbqts_pattern15" => (&[111, 211, 311, 121, 112, 223, 131, 113, 221], &[333, 331, 332, 132, 232, 133, 233, 123, 313]),
                _ => (&[111], &[]),
            };
            let buy: Vec<bool> = seq.iter().map(|s| bs.contains(s)).collect();
            let short: Vec<bool> = seq.iter().map(|s| ss.contains(s)).collect();
            let (sell, cover) = (hold(&buy, 1), hold(&short, 1));
            ami(x, from, to, &Sig { buy, sell, short, cover }, &pc, if id == "hbqts_pattern111" { 1 } else { side }, None, None)
        }
        "hbqts_weekly_pir" => {
            let dlb = u("dlb");
            let (hc, lc) = (hhv(c, dlb), llv(c, dlb));
            let pd: Vec<f64> = (0..n).map(|i| (c[i] - lc[i]) / (hc[i] - lc[i])).collect();
            let (wk, wc, _, _, last) = weeks(x, to);
            let wlb = u("wlb");
            let (whc, wlc) = (hhv(&wc, wlb), llv(&wc, wlb));
            let pw: Vec<f64> = (0..wc.len()).map(|w| (wc[w] - wlc[w]) / (whc[w] - wlc[w])).collect();
            let pwx = expand_last(&pw, &wk, &last);
            let (wl, dl) = (g("wl"), g("dl"));
            let buy: Vec<bool> = (0..n).map(|i| i > 0 && pwx[i] <= wl && pd[i - 1] <= dl && pd[i] > pd[i - 1]).collect();
            let sell = hold(&buy, u("hold"));
            run_long(Sig { buy, sell, ..Default::default() })
        }
        "hbqts_macd_daily" | "hbqts_macd_weekly" => {
            let permit: Vec<bool> = if id == "hbqts_macd_daily" {
                let d: Vec<f64> = ema(c, u("p")).iter().zip(ema(c, u("q"))).map(|(a, b)| a - b).collect();
                let s = ema(&d, u("r"));
                (0..n).map(|i| i > 0 && s[i] > s[i - 1]).collect()
            } else {
                let (wk, wc, _, _, last) = weeks(x, to);
                let d: Vec<f64> = ema(&wc, u("p")).iter().zip(ema(&wc, u("q"))).map(|(a, b)| a - b).collect();
                let s = ema(&d, u("r"));
                let pw: Vec<f64> = (0..wc.len()).map(|w| if w > 0 && s[w] > s[w - 1] { 1.0 } else { 0.0 }).collect();
                expand_last(&pw, &wk, &last).iter().map(|v| *v > 0.5).collect()
            };
            let (m1, m2) = mac(8, 7);
            let (bs, ss) = (cross(&m1, &m2), cross(&m2, &m1));
            run_long(Sig { buy: (0..n).map(|i| bs[i] && permit[i]).collect(), sell: (0..n).map(|i| ss[i] || !permit[i]).collect(), ..Default::default() })
        }
        "hbqts_dual_bb" => {
            let (wk, wc, _, _, last) = weeks(x, to);
            let (wlb, wbw, dlb, dbw) = (u("wlb"), g("wbw"), u("dlb"), g("dbw"));
            let pct = |v: &[f64], lb: usize, bw: f64| { let (m, s) = (sma(v, lb), stdev(v, lb)); (0..v.len()).map(|i| (v[i] - (m[i] - bw * s[i])) / (2.0 * bw * s[i])).collect::<Vec<f64>>() };
            let bw = expand_last(&pct(&wc, wlb, wbw), &wk, &last);
            let bd = pct(c, dlb, dbw);
            let sig = Sig { buy: (0..n).map(|i| bw[i] >= 0.9 && bd[i] >= 0.9).collect(), sell: (0..n).map(|i| bd[i] <= 0.1).collect(),
                short: (0..n).map(|i| bw[i] <= 0.1 && bd[i] < 0.1).collect(), cover: (0..n).map(|i| bd[i] >= 0.9).collect() };
            ami(x, from, to, &sig, &pc, side, None, None)
        }
        "hbqts_cmo" => {
            let pds = u("pds");
            let up: Vec<f64> = (0..n).map(|i| if i > 0 && c[i] > c[i - 1] { c[i] - c[i - 1] } else { 0.0 }).collect();
            let dn: Vec<f64> = (0..n).map(|i| if i > 0 && c[i] < c[i - 1] { c[i - 1] - c[i] } else { 0.0 }).collect();
            let sum = |v: &[f64]| (0..n).map(|i| if i + 1 < pds { f64::NAN } else { v[i + 1 - pds..=i].iter().sum::<f64>() }).collect::<Vec<f64>>();
            let (su, sd) = (sum(&up), sum(&dn));
            let cmo: Vec<f64> = (0..n).map(|i| 100.0 * (su[i] - sd[i]) / (su[i] + sd[i])).map(|v| if v.is_finite() { v } else { f64::NAN }).collect();
            let avg = dema(&cmo, u("avg"));
            let trig = dema(&avg, 3);
            let buy = cross(&avg, &trig);
            let (dnx, hd) = (cross(&trig, &avg), hold(&buy, u("hold")));
            run_long(Sig { sell: (0..n).map(|i| dnx[i] || hd[i]).collect(), buy, ..Default::default() })
        }
        "hbqts_noise0" => {
            let (m1, m2) = (dema(c, 1), ema(c, u("ema")));
            let buy = cross(&m1, &m2);
            let sell = hold(&buy, u("hold"));
            run_long(Sig { buy, sell, ..Default::default() })
        }
        "hbqts_dema16_8" => {
            let (f, s) = (dema(c, u("fast")), dema(c, u("slow")));
            let buy = cross(&f, &s);
            let (dn, hd) = (cross(&s, &f), hold(&buy, u("hold")));
            run_long(Sig { sell: (0..n).map(|i| dn[i] || hd[i]).collect(), buy, ..Default::default() })
        }
        "hbqts_ma8_7_atr_stop" => {
            let s = ma_x(u("fast"), u("slow"));
            let a = atr(x, 20);
            let k = g("mult");
            let loss = |i: usize| k * a[i];
            ami(x, from, to, &s, &pc, 1, Some(&loss), None)
        }
        "hbqts_equity_alloc" => {
            let s = ma_x(u("fast"), u("slow"));
            let hd = hold(&s.buy, u("hold"));
            let sig = Sig { sell: (0..n).map(|i| s.sell[i] || hd[i]).collect(), ..s };
            let a = atr(x, 20);
            let (k, r) = (g("mult"), g("risk") / 100.0);
            let loss = |i: usize| if a[i].is_nan() { f64::NAN } else { (k * a[i]).min(c[i] * r) };
            let trail = |i: usize| 2.0 * a[i];
            ami(x, from, to, &sig, &pc, 1, Some(&loss), Some(&trail))
        }
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;
    #[test]
    fn moon_phase_matches_the_books_calibration() {
        // the book calibrates new moon = 0 at the US close of 20 Dec 2006 (phase 0.00844)
        let p = moon_phase(2006, 12, 20);
        assert!(p < 0.02 || p > 0.98, "{p}");
        let full = moon_phase(2007, 1, 4); // full moon 3 Jan 2007 ~ phase 0.5
        assert!((full - 0.5).abs() < 0.06, "{full}");
    }
    #[test]
    fn day_of_week_and_weeks() {
        assert_eq!(dow(2024, 3, 15), 5); // a Friday
        assert_eq!(dow(2007, 1, 1), 1);
    }
    #[test]
    fn tbond_overnight_buys_close_sells_next_open() {
        let mut r: Vec<Row> = (0..10).map(|i| Row { ticker: "T".into(), date: format!("2001-01-{:02}", i + 1), open: 100., high: 102., low: 98., close: 100., volume: None }).collect();
        r[3].close = 101.; r[4].open = 100.5; r[4].close = 100.5;
        let x = Ctx::new(&r, &r, &[]);
        let t = run("hbqts_tbond_overnight", &x, 0, 9, &Params::new()).unwrap();
        assert_eq!(t.len(), 1); assert_eq!(t[0].legs[0].i, 3); assert_eq!(t[0].legs[0].px, 101.); assert_eq!(t[0].exit_i, 4); assert_eq!(t[0].exit_px, 100.5);
    }
    #[test]
    fn catalogue_ids_unique_and_live_have_params() {
        let c = catalogue(); let mut ids: Vec<_> = c.iter().map(|d| d.id).collect(); ids.sort(); ids.dedup();
        assert_eq!(ids.len(), c.len());
        for d in c.iter().filter(|d| d.status == Status::Live) { assert!(params(d.id).is_some(), "{}", d.id); }
    }
}
