// Larry Connors — "The TradingMarkets Swing Trading College IX" (2010), sessions 1-14.
// Rules below are implemented from the course slides, with instrument adaptations
// (the course tested SPY / US stocks; this engine also runs selected Indian index data).
// Source notes and passing regression tests do not certify every rule or reproduce
// the author's universe, execution settings or reported performance. Audit evidence:
// library/connors/VERIFICATION-STATUS.md; unresolved differences remain explicit.
//
// Conventions (bar-by-bar, no lookahead):
//   * all conditions are evaluated on the CLOSE of bar i using data <= i
//   * "buy on close" fills at close[i]; "next day's open" fills at open[i+1]
//   * limit entries are placed for bar i+1 and fill at min(open, limit) (long) /
//     max(open, limit) (short) only if the bar trades through the limit
//   * indicators are NaN during warm-up; every comparison with NaN is false,
//     so a rule can never fire on missing data.

use crate::Row;
use std::borrow::Cow;
use std::collections::BTreeMap;

// ---------------------------------------------------------------- indicators

/// Simple moving average. A missing value (NaN) blanks only the windows that contain it: the running sum is
/// rebuilt once the gap leaves the window. Without NaNs the arithmetic is exactly the plain running sum.
pub fn sma(v: &[f64], n: usize) -> Vec<f64> {
    let mut out = vec![f64::NAN; v.len()];
    let (mut sum, mut gaps) = (0.0, 0usize);
    for i in 0..v.len() {
        if v[i].is_nan() { gaps += 1 } else { sum += v[i] }
        if i >= n {
            if v[i - n].is_nan() {
                gaps -= 1;
                if gaps == 0 {
                    sum = v[i + 1 - n..=i].iter().sum();
                }
            } else {
                sum -= v[i - n];
            }
        }
        if i + 1 >= n && gaps == 0 {
            out[i] = sum / n as f64;
        }
    }
    out
}

/// Wilder smoothed moving average (SMMA, alpha = 1/n) seeded with the SMA of the
/// first n valid values. NaN inputs before the first valid value are skipped.
pub fn wilder(v: &[f64], n: usize) -> Vec<f64> {
    let mut out = vec![f64::NAN; v.len()];
    let Some(start) = v.iter().position(|x| !x.is_nan()) else { return out };
    if v.len() < start + n {
        return out;
    }
    let mut prev = v[start..start + n].iter().sum::<f64>() / n as f64;
    out[start + n - 1] = prev;
    for i in start + n..v.len() {
        prev = (prev * (n as f64 - 1.0) + v[i]) / n as f64;
        out[i] = prev;
    }
    out
}

/// Wilder RSI: RS = SMMA(up, n) / SMMA(down, n), RSI = 100 - 100/(1+RS).
pub fn rsi(c: &[f64], n: usize) -> Vec<f64> {
    let mut up = vec![f64::NAN; c.len()];
    let mut dn = vec![f64::NAN; c.len()];
    for i in 1..c.len() {
        let d = c[i] - c[i - 1];
        up[i] = d.max(0.0);
        dn[i] = (-d).max(0.0);
    }
    let (au, ad) = (wilder(&up, n), wilder(&dn, n));
    au.iter()
        .zip(&ad)
        .map(|(&u, &d)| {
            if u.is_nan() || d.is_nan() {
                f64::NAN
            } else if d == 0.0 {
                if u == 0.0 { 50.0 } else { 100.0 }
            } else {
                100.0 - 100.0 / (1.0 + u / d)
            }
        })
        .collect()
}

pub fn true_range(r: &[Row]) -> Vec<f64> {
    (0..r.len())
        .map(|i| {
            let hl = r[i].high - r[i].low;
            if i == 0 {
                hl
            } else {
                let pc = r[i - 1].close;
                hl.max((r[i].high - pc).abs()).max((r[i].low - pc).abs())
            }
        })
        .collect()
}

/// Wilder ATR.
pub fn atr(r: &[Row], n: usize) -> Vec<f64> {
    wilder(&true_range(r), n)
}

/// Wilder directional system. Returns (+DI, ADX).
/// +DM = up move if up > down and up > 0; -DM symmetric.
/// +DI = 100*SMMA(+DM)/ATR, DX = 100*|+DI - -DI|/(+DI + -DI), ADX = SMMA(DX).
pub fn dmi(r: &[Row], n: usize) -> (Vec<f64>, Vec<f64>) {
    let len = r.len();
    let mut pdm = vec![f64::NAN; len];
    let mut mdm = vec![f64::NAN; len];
    let mut tr = vec![f64::NAN; len];
    let trs = true_range(r);
    for i in 1..len {
        let upm = r[i].high - r[i - 1].high;
        let dnm = r[i - 1].low - r[i].low;
        pdm[i] = if upm > dnm && upm > 0.0 { upm } else { 0.0 };
        mdm[i] = if dnm > upm && dnm > 0.0 { dnm } else { 0.0 };
        tr[i] = trs[i];
    }
    let (sp, sm, st) = (wilder(&pdm, n), wilder(&mdm, n), wilder(&tr, n));
    let mut pdi = vec![f64::NAN; len];
    let mut dx = vec![f64::NAN; len];
    for i in 0..len {
        if st[i] > 0.0 {
            let p = 100.0 * sp[i] / st[i];
            let m = 100.0 * sm[i] / st[i];
            pdi[i] = p;
            dx[i] = if p + m > 0.0 { 100.0 * (p - m).abs() / (p + m) } else { 0.0 };
        }
    }
    (pdi, wilder(&dx, n))
}

/// Historical volatility: sample stdev of ln(C/C[-1]) over n days * sqrt(252) * 100.
/// (Session 3: "HV100 divided by the square root of 252 (15.875)" gives the daily sigma.)
pub fn hv(c: &[f64], n: usize) -> Vec<f64> {
    hv_ddof(c, n, 1.0)
}

/// Same with a chosen degrees-of-freedom correction: 1 = sample sd (Swing Trading College), 0 = population sd
/// (TradeStation's StdDev, which reproduces the ETF Sweet Spot book's published trades).
pub fn hv_ddof(c: &[f64], n: usize, ddof: f64) -> Vec<f64> {
    hv_days(c, n, ddof, 252.0)
}

/// Same with a chosen number of trading days per year (the ETF Sweet Spot seminar prints 256).
pub fn hv_days(c: &[f64], n: usize, ddof: f64, days: f64) -> Vec<f64> {
    let mut out = vec![f64::NAN; c.len()];
    let lr: Vec<f64> = (1..c.len()).map(|i| (c[i] / c[i - 1]).ln()).collect();
    for i in n..c.len() {
        let w = &lr[i - n..i]; // returns ending at bar i
        let m = w.iter().sum::<f64>() / n as f64;
        let var = w.iter().map(|x| (x - m).powi(2)).sum::<f64>() / (n as f64 - ddof);
        out[i] = var.sqrt() * days.sqrt() * 100.0;
    }
    out
}

/// Bollinger %b = (C - LBB) / (UBB - LBB), bands = SMA(n) +/- k * population stdev.
pub fn pct_b(c: &[f64], n: usize, k: f64) -> Vec<f64> {
    let mut out = vec![f64::NAN; c.len()];
    for i in n - 1..c.len() {
        let w = &c[i + 1 - n..=i];
        let m = w.iter().sum::<f64>() / n as f64;
        let sd = (w.iter().map(|x| (x - m).powi(2)).sum::<f64>() / n as f64).sqrt();
        if sd > 0.0 {
            out[i] = (c[i] - (m - k * sd)) / (2.0 * k * sd);
        }
    }
    out
}

/// Liquidity Filter (updated 1/19/10): 1,000,000 * A / C where
/// A = SMA20(close*volume) of the instrument, C = SMA65(close*volume) of the benchmark.
/// The course benchmark is the S&P 500; on Indian data we use NIFTY 50.
pub fn liquidity_filter(r: &[Row], bench: &[Row]) -> Vec<f64> {
    let tv = |rows: &[Row]| -> Vec<f64> {
        rows.iter().map(|x| x.volume.map(|v| v * x.close).unwrap_or(f64::NAN)).collect()
    };
    let a = sma(&tv(r), 20);
    let bmap: std::collections::HashMap<&str, f64> = {
        let c = sma(&tv(bench), 65);
        bench.iter().zip(c).map(|(x, v)| (x.date.as_str(), v)).collect()
    };
    r.iter()
        .zip(a)
        .map(|(x, a)| match bmap.get(x.date.as_str()) {
            Some(&c) if c > 0.0 => 1_000_000.0 * a / c,
            _ => f64::NAN,
        })
        .collect()
}

/// Number of consecutive bars (ending at i) for which pred holds.
pub fn streak(n: usize, pred: impl Fn(usize) -> bool) -> Vec<usize> {
    let mut out = vec![0; n];
    for i in 0..n {
        if pred(i) {
            out[i] = if i > 0 { out[i - 1] + 1 } else { 1 };
        }
    }
    out
}

// ---------------------------------------------------------------- context

pub struct Ctx<'a> {
    pub r: &'a [Row],
    pub o: Vec<f64>,
    pub h: Vec<f64>,
    pub l: Vec<f64>,
    pub c: Vec<f64>,
    pub ma200: Vec<f64>,
    pub ma5: Vec<f64>,
    pub rsi2: Vec<f64>,
    pub rsi3: Vec<f64>,
    pub rsi4: Vec<f64>,
    pub hv100: Vec<f64>,
    pub adx10: Vec<f64>,
    pub pdi2: Vec<f64>,
    pub pdi3: Vec<f64>,
    pub atr10: Vec<f64>,
    pub pctb5: Vec<f64>,
    pub lf: Vec<f64>,
    pub has_volume: bool,
    pub down_closes: Vec<usize>,
    pub up_closes: Vec<usize>,
    pub red_bars: Vec<usize>, // close < open
    pub vix: VixAligned,
    /// External series aligned to this ticker's bars (NaN where missing), keyed by name (extra_data/)
    pub extra: BTreeMap<String, Vec<f64>>,
    /// The market benchmark's own consecutive down closes (SPX for US instruments, NIFTY 50 for Indian ones, i.e. the
    /// `bench` rows given to `new`), aligned by date; NaN where the benchmark has no bar on that date.
    pub bench_down: Vec<f64>,
    /// Trusted market of a server-qualified native stock (set only by the server from catalogue evidence; None for
    /// index/ETF series). Used for the instrument's real tick in stock runs: NSE 0.05 INR, US 0.01 USD.
    pub native_market: Option<crate::baskets::Market>,
    /// intraday phase 1: real 1-minute bars of this ticker (None = not loaded; minute-path rules then report a data gap)
    pub minutes: Option<&'static crate::intraday::Minutes>,
    /// options phase 2: real EOD monthly-option chain of this underlying (None = not loaded; option rules report a data gap)
    pub options: Option<&'static crate::options::Chain>,
}

/// India VIX facts, computed on the VIX's own daily series, then aligned to this
/// instrument's bars by date (NaN / 0 / false where VIX has no bar).
#[derive(Default)]
pub struct VixAligned {
    pub first: Option<usize>, // first bar index with VIX data
    pub rsi2: Vec<f64>,
    pub up: Vec<usize>,     // consecutive higher VIX closes
    stretch: Vec<f64>,      // VIX close / its 10-day SMA - 1, on the VIX's own bars
    at: Vec<Option<usize>>, // this instrument's bar -> VIX bar on the same date
    pub gap_over_close: Vec<bool>, // VIX open > yesterday's VIX close
    pub gap_over_high: Vec<bool>,  // VIX open > yesterday's VIX high
}

impl VixAligned {
    pub fn has_bar(&self, i: usize) -> bool {
        self.at.get(i).is_some_and(Option::is_some)
    }
    fn new(r: &[Row], vix: &[Row]) -> Self {
        let n = r.len();
        // VIX bars on days this instrument does not trade (e.g. CBOE rows on US holidays) are dropped, so "yesterday's VIX"
        // is the VIX of the instrument's previous trading day; history before the instrument's first bar is kept for warm-up.
        let days: std::collections::HashSet<&str> = r.iter().map(|x| x.date.as_str()).collect();
        let first = r.first().map_or("", |x| x.date.as_str());
        let vix: Vec<&Row> = vix.iter().filter(|x| x.date.as_str() < first || days.contains(x.date.as_str())).collect();
        let c: Vec<f64> = vix.iter().map(|x| x.close).collect();
        let ma10 = sma(&c, 10);
        let stretch: Vec<f64> = c.iter().zip(&ma10).map(|(c, m)| c / m - 1.0).collect();
        let (vr, vu) = (rsi(&c, 2), streak(c.len(), |i| i > 0 && c[i] > c[i - 1]));
        let pos: std::collections::HashMap<&str, usize> = vix.iter().enumerate().map(|(i, x)| (x.date.as_str(), i)).collect();
        let at: Vec<Option<usize>> = r.iter().map(|x| pos.get(x.date.as_str()).copied()).collect();
        let f = |v: &[f64]| at.iter().map(|k| k.map_or(f64::NAN, |k| v[k])).collect();
        let u = |v: &[usize]| at.iter().map(|k| k.map_or(0, |k| v[k])).collect();
        let b = |g: &dyn Fn(usize) -> bool| at.iter().map(|k| k.map_or(false, |k| k > 0 && g(k))).collect();
        VixAligned {
            first: at.iter().position(Option::is_some).filter(|_| n > 0),
            rsi2: f(&vr),
            up: u(&vu),
            gap_over_close: b(&|k| vix[k].open > vix[k - 1].close),
            gap_over_high: b(&|k| vix[k].open > vix[k - 1].high),
            stretch,
            at,
        }
    }
    /// Consecutive days the VIX closed >= pct% above its 10-day SMA (pct < 0: <= |pct|% below),
    /// counted on the VIX's own bars, then aligned.
    pub fn stretched(&self, pct: f64) -> Vec<usize> {
        let t = pct / 100.0;
        let s = streak(self.stretch.len(), |k| if pct > 0.0 { self.stretch[k] >= t } else { self.stretch[k] <= t });
        self.at.iter().map(|k| k.map_or(0, |k| s[k])).collect()
    }
}

impl<'a> Ctx<'a> {
    pub fn new(r: &'a [Row], bench: &[Row], vix: &[Row]) -> Self {
        let c: Vec<f64> = r.iter().map(|x| x.close).collect();
        let (pdi2, _) = dmi(r, 2);
        let (pdi3, _) = dmi(r, 3);
        let (_, adx10) = dmi(r, 10);
        let n = r.len();
        let down_closes = streak(n, |i| i > 0 && c[i] < c[i - 1]);
        let up_closes = streak(n, |i| i > 0 && c[i] > c[i - 1]);
        let red_bars = streak(n, |i| r[i].close < r[i].open);
        Ctx {
            o: r.iter().map(|x| x.open).collect(),
            h: r.iter().map(|x| x.high).collect(),
            l: r.iter().map(|x| x.low).collect(),
            ma200: sma(&c, 200),
            ma5: sma(&c, 5),
            rsi2: rsi(&c, 2),
            rsi3: rsi(&c, 3),
            rsi4: rsi(&c, 4),
            hv100: hv(&c, 100),
            adx10,
            pdi2,
            pdi3,
            atr10: atr(r, 10),
            pctb5: pct_b(&c, 5, 1.0),
            lf: liquidity_filter(r, bench),
            bench_down: {
                let bd = streak(bench.len(), |k| k > 0 && bench[k].close < bench[k - 1].close);
                let at: std::collections::HashMap<&str, f64> = bench.iter().zip(bd).map(|(b, d)| (b.date.as_str(), d as f64)).collect();
                r.iter().map(|x| at.get(x.date.as_str()).copied().unwrap_or(f64::NAN)).collect()
            },
            has_volume: r.iter().any(|x| x.volume.map_or(false, |v| v > 0.0)),
            down_closes,
            up_closes,
            red_bars,
            vix: VixAligned::new(r, vix),
            extra: BTreeMap::new(),
            native_market: None,
            minutes: None,
            options: None,
            c,
            r,
        }
    }
    /// Real execution tick of a native stock: NSE 0.05 INR, US decimal 0.01 USD (None = not a native stock).
    /// Adaptation: the Connors books used 1/8-dollar stock ticks.
    pub fn stock_tick(&self) -> Option<f64> {
        self.native_market.map(|m| match m { crate::baskets::Market::India => 0.05, crate::baskets::Market::UnitedStates => 0.01 })
    }
    /// One real tick of the traded instrument: a native stock's market tick, else 0.05 on the vault's NSE indices
    /// (tickers start "NIFTY") and 0.01 on US indices/ETFs (decimal quotes). Adaptation: the Connors books used 1/8-dollar ticks.
    pub fn market_tick(&self) -> f64 {
        self.stock_tick().unwrap_or(if self.r.first().map_or(false, |r| r.ticker.starts_with("NIFTY")) { 0.05 } else { 0.01 })
    }
    /// An external series by name, if it has any data for this ticker.
    pub fn series(&self, name: &str) -> Option<&[f64]> {
        self.extra.get(name).filter(|v| v.iter().any(|x| x.is_finite())).map(|v| v.as_slice())
    }
    /// Which of the named series are missing for this ticker.
    pub fn missing(&self, names: &[&'static str]) -> Vec<&'static str> {
        names.iter().copied().filter(|n| self.series(n).is_none()).collect()
    }
    // Indicators at any period: the book's periods come from the cache, others are computed.
    pub fn sma_n(&self, n: usize) -> Cow<'_, [f64]> {
        match n {
            200 => Cow::Borrowed(&self.ma200),
            5 => Cow::Borrowed(&self.ma5),
            _ => Cow::Owned(sma(&self.c, n)),
        }
    }
    pub fn rsi_n(&self, n: usize) -> Cow<'_, [f64]> {
        match n {
            2 => Cow::Borrowed(&self.rsi2),
            3 => Cow::Borrowed(&self.rsi3),
            4 => Cow::Borrowed(&self.rsi4),
            _ => Cow::Owned(rsi(&self.c, n)),
        }
    }
    pub fn pdi_n(&self, n: usize) -> Cow<'_, [f64]> {
        match n {
            2 => Cow::Borrowed(&self.pdi2),
            3 => Cow::Borrowed(&self.pdi3),
            _ => Cow::Owned(dmi(self.r, n).0),
        }
    }
    pub fn pctb_n(&self, n: usize) -> Cow<'_, [f64]> {
        if n == 5 { Cow::Borrowed(&self.pctb5) } else { Cow::Owned(pct_b(&self.c, n, 1.0)) }
    }
    /// Position of the close inside today's range, 0 = low, 1 = high.
    pub fn close_loc(&self, i: usize) -> f64 {
        let rng = self.h[i] - self.l[i];
        if rng > 0.0 { (self.c[i] - self.l[i]) / rng } else { f64::NAN }
    }
    pub fn lowest_close(&self, i: usize, n: usize) -> bool {
        i + 1 >= n && self.c[i + 1 - n..=i].iter().all(|&x| self.c[i] <= x)
    }
    pub fn lowest_low(&self, i: usize, n: usize) -> bool {
        i + 1 >= n && self.l[i + 1 - n..=i].iter().all(|&x| self.l[i] <= x)
    }
    pub fn highest_high(&self, i: usize, n: usize) -> bool {
        i + 1 >= n && self.h[i + 1 - n..=i].iter().all(|&x| self.h[i] >= x)
    }
    pub fn all(&self, i: usize, days: usize, f: impl Fn(usize) -> bool) -> bool {
        i + 1 >= days && (i + 1 - days..=i).all(f)
    }
}

// ---------------------------------------------------------------- engine

#[derive(Clone, Debug, PartialEq)]
pub struct Leg {
    pub i: usize,
    pub px: f64,
    pub w: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Trade {
    pub dir: f64, // +1 long, -1 short
    pub legs: Vec<Leg>,
    pub exit_i: usize,
    pub exit_px: f64,
    pub reason: String, // which exit rule fired (or "open")
    pub entry_kind: &'static str, // "close" or "limit"
    pub signal_i: usize, // bar on whose close the exit rule fired (exit_i, or exit_i - 1 for next-open exits)
    pub open: bool, // still open on the last bar (marked to market)
    /// Scale-out pieces (bar, price, fraction of position), INCLUDING the final one; fractions
    /// sum to 1 and exit_px is their weighted average. Empty = single exit at exit_i/exit_px.
    pub exits: Vec<(usize, f64, f64)>,
}

impl Trade {
    /// Return on the FULL position allocation (unfilled scale-in legs = idle cash).
    pub fn ret(&self, cost: f64) -> f64 {
        self.legs
            .iter()
            .map(|l| l.w * (self.dir * (self.exit_px / l.px - 1.0) - 2.0 * cost))
            .sum()
    }
    /// Mark price on bar k (< exit_i): pieces already closed at their fill, the rest at `close`.
    pub fn mark(&self, k: usize, close: f64) -> f64 {
        let (mut px, mut done) = (0.0, 0.0);
        for &(_, p, f) in self.exits.iter().filter(|e| e.0 <= k) {
            px += f * p;
            done += f;
        }
        px + (1.0 - done) * close
    }
    pub fn deployed(&self) -> f64 {
        self.legs.iter().map(|l| l.w).sum()
    }
    pub fn avg_px(&self) -> f64 {
        self.legs.iter().map(|l| l.w * l.px).sum::<f64>() / self.deployed()
    }
}

pub enum Entry {
    Close,
    Limit(f64), // resting limit for the NEXT bar
}
/// Exit order with the rule that fired (shown in the trade log).
pub enum Exit {
    Close(String),
    NextOpen(String),
}

pub struct Spec<'s> {
    pub dir: f64,
    /// scale-in weights as fractions of a full position; first = initial entry
    pub weights: &'s [f64],
    pub entry: &'s dyn Fn(usize) -> Option<Entry>,
    /// called on bars after the entry bar while legs remain; true = add next leg at close
    pub add: &'s dyn Fn(usize, &Trade) -> bool,
    pub exit: &'s dyn Fn(usize, &Trade) -> Option<Exit>,
}

pub fn no_add(_: usize, _: &Trade) -> bool {
    false
}

/// Single-instrument, one-position-at-a-time simulator over bars [from, to].
pub fn simulate(ctx: &Ctx, s: &Spec, from: usize, to: usize) -> Vec<Trade> {
    let mut trades = Vec::new();
    let mut cur: Option<Trade> = None;
    let mut pending_limit: Option<f64> = None;
    let mut pending_exit = false;
    for i in from..=to {
        if pending_exit {
            let mut t = cur.take().unwrap();
            t.exit_i = i;
            t.exit_px = ctx.o[i];
            trades.push(t);
            pending_exit = false;
        }
        let mut entered_now = false;
        if let Some(lim) = pending_limit.take() {
            let fill = if s.dir > 0.0 {
                (ctx.l[i] <= lim).then(|| ctx.o[i].min(lim))
            } else {
                (ctx.h[i] >= lim).then(|| ctx.o[i].max(lim))
            };
            if let Some(px) = fill {
                cur = Some(Trade { entry_kind: "limit", ..new_trade(s, i, px) });
                entered_now = true; // a limit fill can still exit on this bar's close
            }
        }
        if let Some(t) = cur.as_mut() {
            let on_entry_bar_close = t.legs[0].i == i && !entered_now;
            if !on_entry_bar_close {
                match (s.exit)(i, t) {
                    Some(Exit::Close(why)) => {
                        t.exit_i = i;
                        t.exit_px = ctx.c[i];
                        t.reason = why;
                        t.signal_i = i;
                        trades.push(cur.take().unwrap());
                        continue;
                    }
                    Some(Exit::NextOpen(why)) => {
                        if i < to {
                            pending_exit = true;
                            t.reason = why;
                            t.signal_i = i;
                        }
                        continue;
                    }
                    None => {}
                }
                if t.legs.len() < s.weights.len() && t.legs[0].i < i && (s.add)(i, t) {
                    let w = s.weights[t.legs.len()];
                    t.legs.push(Leg { i, px: ctx.c[i], w });
                }
            }
            continue;
        }
        if i == to {
            break;
        }
        match (s.entry)(i) {
            Some(Entry::Close) => cur = Some(new_trade(s, i, ctx.c[i])),
            Some(Entry::Limit(p)) => pending_limit = Some(p),
            None => {}
        }
    }
    if let Some(mut t) = cur {
        t.exit_i = to;
        t.exit_px = ctx.c[to];
        t.reason = "open".into();
        t.signal_i = to;
        t.open = true;
        trades.push(t);
    }
    trades
}

fn new_trade(s: &Spec, i: usize, px: f64) -> Trade {
    Trade {
        dir: s.dir,
        legs: vec![Leg { i, px, w: s.weights[0] }],
        exit_i: i,
        exit_px: px,
        reason: String::new(),
        entry_kind: "close",
        signal_i: i,
        open: false, exits: Vec::new()
    }
}

// ---------------------------------------------------------------- guru profile

pub const GURU_ID: &str = "connors";
pub const GURU_NAME: &str = "Larry Connors";
pub const TAGLINE: &str = "Buy fear in uptrends, sell greed in downtrends, scale in, and let the 2-period RSI take you out.";
pub const BOOK: &str = "The TradingMarkets Swing Trading College IX (2010) · 14 sessions";

/// The thesis behind every strategy in the course, in Connors' own framing (sessions 1, 4, 5, 9).
pub const THESIS: &[(&str, &str)] = &[
    ("Trade with the long-term trend", "Buy only when the close is above the 200-day SMA (and short only below it). Test 1a vs 2a: +0.23% vs +0.03% per 5-day hold."),
    ("Buy pullbacks, not breakouts", "Short-term weakness inside a long-term uptrend mean-reverts. A 10-day low of closes lifts the edge to 0.71%; deeper intraday pullbacks (3% and 7% limits) lift it to 1.51% and 3.42%."),
    ("The 2-period RSI is the oscillator", "The lower RSI(2) goes, and the longer it stays low, the stronger the snap-back: \"an elastic band\". Exit into strength: close above the 5-day SMA or RSI(2) above 65-75."),
    ("Scale in (TPS: Time, Price, Scale-in)", "Buy in pieces (1-2-3-4, 2-3-5, 1-2-3-4-5) as the market moves further against you. Averaging into extremes is the edge and also the hardest part psychologically."),
    ("No hard stops", "In nearly every test, stops cut returns and can remove the edge entirely. Control risk with position size or long options, not stops."),
    ("Trade it as a business, systematically", "Quantified rules, a portfolio of strategies (long/short, stocks/ETFs), a capital reserve and a journal. \"Trading is 90% psychological.\""),
];

// ---------------------------------------------------------------- catalogue

#[derive(Clone, Copy, PartialEq)]
pub enum Status {
    Live,
    NeedsData(&'static str),
    NotTestable(&'static str),
}

pub struct Claim {
    pub trades: &'static str,
    pub win: &'static str,
    pub avg: &'static str,
    pub hold: &'static str,
    pub period: &'static str,
}

pub struct StrategyDef {
    pub id: &'static str,
    pub name: &'static str,
    pub group: &'static str,
    pub session: u8,
    pub dir: &'static str,
    pub status: Status,
    pub needs_volume: bool,
    pub needs_vix: bool, // test window starts at the first India VIX bar
    /// External data series the rules read (see extra_data/README.md); missing for a ticker = shown as a data gap
    pub needs_series: &'static [&'static str],
    pub book: &'static str, // which book the rules come from (stc9 = Swing Trading College IX, etfss = ETF Sweet Spot)
    pub rules: &'static [&'static str],
    pub notes: &'static str,
    pub claim: Option<Claim>,
}

const MT: &str = "1/1/95 - 12/31/09, SPY";

macro_rules! claim {
    ($t:expr, $w:expr, $a:expr, $h:expr, $p:expr) => {
        Some(Claim { trades: $t, win: $w, avg: $a, hold: $h, period: $p })
    };
}

/// Book's own published statistics (strings as printed).
pub fn claim(trades: &'static str, win: &'static str, avg: &'static str, hold: &'static str, period: &'static str) -> Option<Claim> {
    Some(Claim { trades, win, avg, hold, period })
}

pub fn catalogue() -> Vec<StrategyDef> {
    use Status::*;
    let d = |id, name, group, session, dir, status, rules, notes, claim| StrategyDef {
        id, name, group, session, dir, status, needs_volume: false, needs_vix: false, needs_series: &[], book: "stc9", rules, notes, claim,
    };
    let vx = |def: StrategyDef| StrategyDef { needs_vix: true, ..def };
    const VIXNOTE: &str = "Course used the CBOE VIX on SPY (1995-2009). US runs use the CBOE VIX; India runs use India VIX (NSE), daily bars built from 1-minute data (padded bars excluded).";
    let mut v = vec![
        // ---- Session 1: the 4 keys + RSI2 + R3
        d("f1a", "Test 1a · Above 200-day MA", "Foundation (4 Keys)", 1, "Long", Live,
          &["F|Close > 200-day SMA", "E|No other signal: buy on the close whenever the condition holds and no trade is open", "X|Sell on the close 5 trading days after entry"],
          "Baseline for the 4 keys.", claim!("3,944,383", "51%", "0.23%", "5 days", "US stocks")),
        d("f3a", "Test 3a · Pullback to 10-day low close", "Foundation (4 Keys)", 1, "Long", Live,
          &["F|Close > 200-day SMA", "E|Close is the lowest close of the last 10 days", "E|Buy on the close", "X|Sell on the close 5 trading days after entry"],
          "", claim!("510,866", "55%", "0.71%", "5 days", "US stocks")),
        d("f4a", "Test 4a · 10-day low + 3% limit", "Foundation (4 Keys)", 1, "Long", Live,
          &["F|Close > 200-day SMA", "E|Today's low is the lowest low of the last 10 days", "E|Next day: buy with a limit 3% below today's close", "X|Sell on the close 5 trading days after entry"],
          "", claim!("105,771", "58%", "1.51%", "5 days", "US stocks")),
        d("f4c", "Test 4c · 10-day low + 7% limit", "Foundation (4 Keys)", 1, "Long", Live,
          &["F|Close > 200-day SMA", "E|Today's low is the lowest low of the last 10 days", "E|Next day: buy with a limit 7% below today's close", "X|Sell on the close 5 trading days after entry"],
          "A 7% intraday drop is rare on a broad index; expect very few trades.", claim!("27,723", "62%", "3.42%", "5 days", "US stocks")),
        d("f4d", "Test 4d · 7% limit, exit > 5MA", "Foundation (4 Keys)", 1, "Long", Live,
          &["F|Close > 200-day SMA", "E|Today's low is the lowest low of the last 10 days", "E|Next day: buy with a limit 7% below today's close", "X|Sell on the close when Close > 5-day SMA"],
          "", claim!("27,723", "72%", "4.42%", "-", "US stocks")),
        d("rsi2_5d", "Test 5d · RSI(2) < 2", "Foundation (4 Keys)", 1, "Long", Live,
          &["F|Close > 200-day SMA", "E|RSI(2) < 2", "E|Buy on the close", "X|Sell on the close when Close > 5-day SMA"],
          "", claim!("73,413", "70%", "1.25%", "-", "US stocks")),
        d("r3", "R3 Strategy", "Index Timing", 1, "Long", Live,
          &["F|Index close > 200-day SMA", "E|RSI(2) has fallen 3 days in a row", "E|RSI(2) on the first falling day < 60", "E|RSI(2) today < 10", "E|Buy unit 1 on the close", "A|Buy a 2nd unit on the close when RSI(2) has fallen 5 days in a row", "X|Sell on the close when RSI(2) > 75 or Close > 5-day SMA"],
          "Full position = 2 equal units; returns are on the full 2-unit allocation. The later book version (High Probability ETF Trading, 2009) differs: the first drop starts from a reading below 60 and the exit is RSI(2) > 70. We test the course rules.", claim!("69", "86.96%", "1.37%", "6.03 bars", "S&P 500, PF 8.7")),
        // ---- Session 2: TPS for stocks
        StrategyDef { needs_volume: true, ..d("tps_stocks", "TPS for Stocks", "Equity Long", 2, "Long", Live,
          &["F|Close > 200-day SMA", "F|HV(100) > 30", "F|ADX(10) > 30", "F|Liquidity Filter > 3", "E|Closes down 3 days in a row, including today", "E|RSI(2) < 1", "E|Buy 2 units on the close", "A|Close ≥ 0.5% below the last buy: buy 3 units", "A|Close another ≥ 0.5% lower: buy 5 units", "X|Sell on the close when RSI(2) > 70"],
          "Weights 2/3/5 of a 10-unit position. Stock filters (HV>30, ADX>30) rarely pass on a broad index; zero trades is an honest answer.",
          claim!("2,395", "84.13%", "3.64%", "5.7 days", "1/1/2001 - 12/31/2009, US stocks")) },
        // ---- Session 3
        StrategyDef { needs_volume: true, ..d("x555", "5x5x5 Version 2.0", "Equity Long", 3, "Long", Live,
          &["F|Close > 200-day SMA", "F|Liquidity Filter > 3", "F|HV(100) > 30", "F|ADX(10) > 30", "E|Closes down 2 days in a row", "E|Close ≥ 5% below its 5-day SMA", "E|Next day: buy with a limit at yesterday's close × (1 − max(5%, HV(100)/√252 %))", "X|After a close above the 5-day SMA, sell at the next day's open", "X|After a close ≥ 5% in profit, sell at the next day's open"],
          "", None) },
        StrategyDef { needs_volume: true, ..d("h10", "STC Short Selling (H10)", "Equity Short", 3, "Short", Live,
          &["F|Liquidity Filter > 20", "F|HV(100) > 40", "F|ADX(10) > 40", "F|Close > 200-day SMA", "E|RSI(2) > 90", "E|Today's high is the highest high of the last 10 days", "E|Next day: sell short with a limit at Close + 0.4 × ATR(10)", "X|Once RSI(2) < 40, cover at the next day's open"],
          "Max 10 slots in the course; single instrument here.", claim!("-", "≈70%", "-", "-", "since 1995, US stocks")) },
        // ---- Session 4: TPS ETF
        d("tps_long", "TPS 2-3-5 Long", "ETF (TPS)", 4, "Long", Live,
          &["F|Close ≥ 1.5% above the 200-day SMA (optional in the course; ON here)", "E|RSI(2) < 30", "E|Buy 20% of a full position on the close", "A|Close below the first entry price: buy 30%", "A|Close below the second entry price: buy 50%", "X|Sell on the close when RSI(2) > 70", "X|Stop: sell on the close if Close ≥ 1.5% below the 200-day SMA"],
          "Course version (2-3-5). The book version (High Probability ETF Trading, 2009) uses RSI(2) < 25 two days in a row, 10/20/30/40 sizing and a RSI(2) > 70 exit.", claim!("911", "85.8%", "-", "-", "SP-500 1960-2009 scale-in study")),
        d("tps_short", "TPS 2-3-5 Short", "ETF (TPS)", 4, "Short", Live,
          &["F|Close ≥ 1.5% below the 200-day SMA", "E|RSI(2) > 75 two days in a row", "E|Short 20% of a full position on the close", "A|Close above the first entry price: short 30%", "A|Close above the second entry price: short 50%", "X|Cover on the close when RSI(2) < 25", "X|Stop: cover on the close if Close ≥ 1.5% above the 200-day SMA"],
          "", None),
        // ---- Session 7: leveraged ETF 1-2-3-4-5
        StrategyDef { needs_volume: true, ..d("s12345", "1-2-3-4-5 Scale-In", "Leveraged ETF", 7, "Long", Live,
          &["F|21-day average volume > 250,000 and 21-day lowest volume ≥ 50,000 (no 200-day filter)", "E|RSI(2) < 5", "E|Buy 1/15 of a full position on the close", "A|Each close below the last buy: buy 2/15, then 3/15, 4/15, 5/15", "X|Sell on the close when RSI(2) > 50"],
          "Built for 2x/3x ETFs; run here on the unleveraged index.", None) },
        // ---- Session 7: market timing — EOM
        d("eom_co", "EOM · Close < Open", "Market Timing · End of Month", 7, "Long", Live,
          &["F|Close > 200-day SMA", "F|Day of month ≥ 25", "E|2 or more bars in a row closing below their open", "E|RSI(2) ≤ 65", "E|Buy on the close", "X|Sell on the close when RSI(2) > 65"],
          "", claim!("73", "80.8%", "0.65%", "4.2 days", MT)),
        d("eom_rsi", "EOM · RSI", "Market Timing · End of Month", 7, "Long", Live,
          &["F|Close > 200-day SMA", "F|Day of month ≥ 25", "E|RSI(2) < 25", "E|Buy on the close", "X|Sell on the close when RSI(2) > 65"],
          "", claim!("69", "81.2%", "0.72%", "4.3 days", MT)),
        d("eom_cc", "EOM · Close < Close", "Market Timing · End of Month", 7, "Long", Live,
          &["F|Close > 200-day SMA", "F|Day of month ≥ 25", "E|2 or more lower closes in a row", "E|RSI(2) ≤ 65", "E|Buy on the close", "X|Sell on the close when RSI(2) > 65"],
          "", claim!("75", "81.3%", "0.68%", "4.2 days", MT)),
        // ---- RSI
        d("rsi2_4", "RSI(2) < 4", "Market Timing · RSI", 7, "Long", Live,
          &["F|Close > 200-day SMA", "E|RSI(2) < 4", "E|Buy on the close", "X|Sell on the close when RSI(2) > 65"],
          "", claim!("38", "92.1%", "1.43%", "4.4 days", MT)),
        d("rsi4_25", "RSI(4) < 25", "Market Timing · RSI", 7, "Long", Live,
          &["F|Close > 200-day SMA", "E|RSI(4) < 25", "E|RSI(2) ≤ 65", "E|Buy on the close", "X|Sell on the close when RSI(2) > 65"],
          "", claim!("86", "77.9%", "0.98%", "4.9 days", MT)),
        d("rsi_div8", "RSI Divergence 8-day", "Market Timing · RSI", 7, "Long", Live,
          &["F|Close > 200-day SMA", "E|Close < the close 8 days ago", "E|RSI(2) > RSI(2) 8 days ago", "E|RSI(2) < 15", "E|Buy on the close", "X|Sell on the close when RSI(2) > 65"],
          "", claim!("10", "90%", "1.10%", "4.4 days", MT)),
        d("rsi_div2", "RSI Divergence 2-day", "Market Timing · RSI", 7, "Long", Live,
          &["F|Close > 200-day SMA", "E|Close < the close 2 days ago", "E|RSI(2) > RSI(2) 2 days ago", "E|RSI(2) < 30", "E|Buy on the close", "X|Sell on the close when RSI(2) > 65"],
          "", claim!("38", "73.7%", "0.61%", "4.7 days", MT)),
        d("rsi_1", "RSI-1 · 3-day RSI(2) sum < 50", "Market Timing · RSI", 7, "Long", Live,
          &["F|Close > 200-day SMA", "E|RSI(2) summed over the last 3 days < 50", "E|Buy on the close", "X|Sell on the close when RSI(2) > 65"],
          "", claim!("62", "85.5%", "1.04%", "4.4 days", MT)),
        d("rsi_2", "RSI-2 · 2-day RSI(2) sum < 40", "Market Timing · RSI", 7, "Long", Live,
          &["F|Close > 200-day SMA", "E|RSI(2) summed over the last 2 days < 40", "E|Buy on the close", "X|Sell on the close when RSI(2) > 65"],
          "", claim!("139", "79.9%", "0.67%", "4.6 days", MT)),
        d("rsi_3", "RSI-3 · 2-day RSI(3) sum < 45", "Market Timing · RSI", 7, "Long", Live,
          &["F|Close > 200-day SMA", "E|RSI(3) summed over the last 2 days < 45", "E|Buy on the close", "X|Sell on the close when RSI(2) > 65"],
          "", claim!("88", "81.8%", "0.97%", "4.6 days", MT)),
        // ---- Price, %b
        d("price", "Price · 3 red bars, close in bottom 10%", "Market Timing · Price & %b", 7, "Long", Live,
          &["F|Close > 200-day SMA", "E|Close < Open for 3 or more days in a row", "E|Close in the bottom 10% of today's range", "E|RSI(2) ≤ 65", "E|Buy on the close", "X|Sell on the close when RSI(2) > 65"],
          "", claim!("47", "89.4%", "0.94%", "4.5 days", MT)),
        d("pctb_1", "%b < 0 two days + RSI", "Market Timing · Price & %b", 7, "Long", Live,
          &["F|Close > 200-day SMA", "E|5-period %b (1 std) < 0 for 2 or more days", "E|RSI(2) < 10", "E|Buy on the close", "X|Sell on the close when RSI(2) > 65"],
          "", claim!("87", "83.9%", "0.94%", "4.7 days", MT)),
        d("pctb_2", "%b < 0.2 three days + bottom 25%", "Market Timing · Price & %b", 7, "Long", Live,
          &["F|Close > 200-day SMA", "E|5-period %b (1 std) < 0.2 for 3 or more days", "E|Close in the bottom 25% of today's range", "E|RSI(2) ≤ 65", "E|Buy on the close", "X|Sell on the close when RSI(2) > 65"],
          "", claim!("56", "85.7%", "1.15%", "4.8 days", MT)),
        // ---- DMI
        d("dmi_1", "DMI(1) · +DI(3) < 5", "Market Timing · DMI", 7, "Long", Live,
          &["F|Close > 200-day SMA", "E|+DI(3) < 5", "E|RSI(2) ≤ 65", "E|Buy on the close", "X|Sell on the close when RSI(2) > 65"],
          "", claim!("91", "79.1%", "0.75%", "4.5 days", MT)),
        d("dmi_2", "DMI(2) · +DI(2) < 3", "Market Timing · DMI", 7, "Long", Live,
          &["F|Close > 200-day SMA", "E|+DI(2) < 3", "E|RSI(2) ≤ 65", "E|Buy on the close", "X|Sell on the close when RSI(2) > 65"],
          "", claim!("128", "81.3%", "0.78%", "4.5 days", MT)),
        d("dmi_3", "DMI(3) · +DI(2) < 10, 2 days", "Market Timing · DMI", 7, "Long", Live,
          &["F|Close > 200-day SMA", "E|+DI(2) < 10 for 2 days in a row", "E|RSI(2) ≤ 65", "E|Buy on the close", "X|Sell on the close when RSI(2) > 65"],
          "", claim!("181", "80.1%", "0.60%", "4.3 days", MT)),
        d("dmi_4", "DMI(4) · +DI(2) < 10, 3 days", "Market Timing · DMI", 7, "Long", Live,
          &["F|Close > 200-day SMA", "E|+DI(2) < 10 for 3 days in a row", "E|RSI(2) ≤ 65", "E|Buy on the close", "X|Sell on the close when RSI(2) > 65"],
          "", claim!("117", "79.5%", "0.68%", "4.3 days", MT)),
        d("dmi_5", "DMI(5) · +DI(2) < 15, 4 days", "Market Timing · DMI", 7, "Long", Live,
          &["F|Close > 200-day SMA", "E|+DI(2) < 15 for 4 days in a row", "E|RSI(2) ≤ 65", "E|Buy on the close", "X|Sell on the close when RSI(2) > 65"],
          "", claim!("107", "80.4%", "0.72%", "4.2 days", MT)),
        // ---- Shorts
        d("short_4up", "Short · 4 Up Days", "Market Timing · Short", 7, "Short", Live,
          &["F|Close < 200-day SMA", "E|4 or more higher closes in a row", "E|Close ≥ 5-day SMA", "E|Short on the close", "X|Cover on the close when Close < 5-day SMA"],
          "", claim!("31", "71%", "1.11%", "4.2 days", MT)),
        d("short_strong", "Short · Strong 2-day", "Market Timing · Short", 7, "Short", Live,
          &["F|Close < 200-day SMA", "E|2-day return > 3%", "E|RSI(2) > 80", "E|Close ≥ 5-day SMA", "E|Short on the close", "X|Cover on the close when Close < 5-day SMA"],
          "", claim!("49", "77.6%", "1.30%", "4.6 days", MT)),
    ];
    // ---- VIX (India VIX stands in for the CBOE VIX)
    let g = "Market Timing · VIX";
    v.extend([
        vx(d("vix200", "VIX 200 · stretched 5% for 3 days", g, 7, "Long", Live,
          &["F|Close > 200-day SMA", "E|VIX close ≥ 5% above its 10-day SMA for 3 or more days in a row", "E|RSI(2) < 10 on the trigger day", "E|Buy on the close", "X|Sell on the close when RSI(2) > 65"],
          VIXNOTE, claim!("34", "85.3%", "1.10%", "4.6 days", MT))),
        vx(d("vix2", "VIX (2) · VIX RSI(2) > 90 + gap", g, 7, "Long", Live,
          &["F|Close > 200-day SMA", "E|RSI(2) of the VIX > 90", "E|VIX opens above yesterday's VIX close", "E|RSI(2) < 30", "E|Buy on the close", "X|Sell on the close when RSI(2) > 65"],
          VIXNOTE, claim!("105", "81.0%", "0.88%", "4.5 days", MT))),
        vx(d("vix4", "VIX (4) · VIX up 5 days", g, 7, "Long", Live,
          &["F|Close > 200-day SMA", "E|VIX closes higher 5 days in a row", "E|RSI(2) < 30 on the trigger day", "E|Buy on the close", "X|Sell on the close when RSI(2) > 65"],
          VIXNOTE, claim!("21", "81%", "0.95%", "4.4 days", MT))),
        vx(d("vix7", "VIX (7) · up 4 days + gap over high", g, 7, "Long", Live,
          &["F|Close > 200-day SMA", "E|VIX closes higher 4 days in a row", "E|VIX opens above yesterday's VIX high", "E|RSI(2) < 30", "E|Buy on the close", "X|Sell on the close when RSI(2) > 65"],
          VIXNOTE, claim!("26", "80.8%", "1.13%", "4.2 days", MT))),
        vx(d("vix8", "VIX (8) · stretched 10% for 2 days", g, 7, "Long", Live,
          &["F|Close > 200-day SMA", "E|VIX close ≥ 10% above its 10-day SMA for 2 or more days in a row", "E|RSI(2) < 30", "E|Buy on the close", "X|Sell on the close when RSI(2) > 65"],
          VIXNOTE, claim!("38", "71.1%", "0.60%", "4.6 days", MT))),
        vx(d("vix53_short", "Short · VIX 5.3", "Market Timing · Short", 7, "Short", Live,
          &["F|Close < 200-day SMA", "E|VIX close ≥ 5% below its 10-day SMA for 3 or more days in a row", "E|RSI(2) > 80 on the trigger day", "E|Close ≥ 5-day SMA", "E|Short on the close", "X|Cover on the close when Close < 5-day SMA"],
          VIXNOTE, claim!("35", "68.6%", "0.86%", "4.5 days", MT))),
        vx(d("vix101_short", "Short · VIX 10.1", "Market Timing · Short", 7, "Short", Live,
          &["F|Close < 200-day SMA", "E|VIX close ≥ 10% below its 10-day SMA for 1 or more days", "E|RSI(2) > 80 on the trigger day", "E|Close ≥ 5-day SMA", "E|Short on the close", "X|Cover on the close when Close < 5-day SMA"],
          VIXNOTE, claim!("29", "62.1%", "0.94%", "4.9 days", MT))),
    ]);
    // ---- The ETF Sweet Spot Trading System (Connors Research, 2004), Part III rules p.10-11
    let ss = |id, name, dir, rules, claim| StrategyDef { book: "etfss", ..d(id, name, "ETF Mean Reversion", 0, dir, Live, rules, SSNOTE, claim) };
    const SSNOTE: &str = "The book counts each unit as a separate trade and reports points per unit; here a position is 2 equal units and returns are on the full position. HV = 40-day historical volatility: population standard deviation of daily log returns × √256: the book does not define it; the Tortoise Capital ETF seminar slides of the same system print HV(40) = STD(ln(close/yesterday's close),40)*100*sqrt(256) (ETFseminar1.pdf p.163). √252 is available as a labelled option. Book tests: SPY, QQQ, IWM, SMH, MDY, 1/3/00 - 6/16/04, close-to-close, no costs.";
    const SSL: &[&str] = &["F|The ETF closes above its 200-day SMA", "E|Today's high is below the 10-day SMA", "E|Today's close is at least 1% below its 10-day SMA for each 15% of its 40-day historical volatility (HV)", "E|Buy at the close", "A|Buy an additional unit if the stretch repeats itself while you are in the trade", "X|Exit at today's close when yesterday's close is above its 10-day SMA"];
    const SSS: &[&str] = &["F|The ETF closes below its 200-day SMA", "E|Today's low is above the 10-day SMA", "E|Today's close is at least 1% above its 10-day SMA for each 15% of its 40-day historical volatility (HV)", "E|Enter short at the close", "A|Short an additional unit if the stretch repeats itself while you are in the trade", "X|Exit at today's close when yesterday's close is below its 10-day SMA"];
    const SSB: &[&str] = &["F|Long side: close above the 200-day SMA; short side: close below it", "E|Long: today's high below the 10-day SMA and the close at least max(1%, HV(40)/15) below it; short: the mirror", "E|Enter at the close", "A|Add one more unit if the stretch repeats while in the trade", "X|Exit at today's close when yesterday's close crossed back over the 10-day SMA"];
    // Conservative variation (printed p.32): "77.5% of the second signals that were generated by Rule #5 were profitable. For those
    // of you who may want to be conservative, you can limit your trades only to the 2nd signal." Admitted 2026-10-21 (newrules-misc).
    const SS2NOTE: &str = "Printed p.32 (SPY results summary): the book's conservative variation trades only the Rule #5 signal: one unit entered on the close where the stretch repeats while the first-signal position (not traded) would still be open, and closed by that position's Rule #6 exit. The book reports only '77.5% of the second signals ... were profitable' (SPY, no further figures), so no claim is shown. [Not from the book] Only one repeat is counted per position, as in Rule #5 (p.15: 'when the conditions of Rules #3 and #4 are met a second time'). The book advises ETFs only (p.12); index and stock runs are a declared application (user rule 21 Oct 2026), not the author's test.";
    const SS2L: &[&str] = &["F|(p10) The ETF closes above its 200-day SMA; today's high is below the 10-day SMA; the close is at least 1% below the 10-day SMA for each 15% of HV(40): the 1st signal is NOT traded", "E|(p15, p32) \"you can limit your trades only to the 2nd signal\": buy one unit at the close when Rules #2-#4 are met a second time while the 1st signal's position would still be open", "X|(p10) Exit at today's close when yesterday's close is above its 10-day SMA (the 1st signal's Rule #6 exit)"];
    const SS2S: &[&str] = &["F|(p10-11) The ETF closes below its 200-day SMA; today's low is above the 10-day SMA; the close is at least 1% above the 10-day SMA for each 15% of HV(40): the 1st signal is NOT traded", "E|(p15, p32) Short one unit at the close when Rules #2-#4 are met a second time while the 1st signal's position would still be open", "X|(p11) Exit at today's close when yesterday's close is below its 10-day SMA"];
    v.extend([
        ss("ss_long", "ETF Sweet Spot · Long", "Long", SSL, claim!("36 units", "80.56%", "1.30 pts", "6 / 13 bars (win / loss)", "1/3/00 - 6/16/04, SPY")),
        ss("ss_short", "ETF Sweet Spot · Short", "Short", SSS, claim!("52 units", "67.31%", "1.02 pts", "5 / 12 bars (win / loss)", "1/3/00 - 6/16/04, SPY")),
        ss("ss_both", "ETF Sweet Spot · Long & Short", "Long/Short", SSB, claim!("88 units", "72.73%", "1.14 pts", "5 / 12 bars (win / loss)", "1/3/00 - 6/16/04, SPY")),
        StrategyDef { notes: SS2NOTE, ..ss("ss_long_2nd", "ETF Sweet Spot · Long, 2nd signal only", "Long", SS2L, None) },
        StrategyDef { notes: SS2NOTE, ..ss("ss_short_2nd", "ETF Sweet Spot · Short, 2nd signal only", "Short", SS2S, None) },
    ]);
    // Strategies whose inputs are not in the data — listed, never faked.
    // TRIN Market Timing (Session 7 p.95-113): the market's own Arms index (TRIN series: NSE on India, NYSE on the US)
    let na: &[&str] = &[];
    for (id, name, rules, claim) in [
        ("trin4", "TRIN 4 Days > 1.0", TRIN4, claim!("55", "78.2%", "438.2 total points", "4.4 days", MT)),
        ("trin4_12", "TRIN 4 Days > 1.2", TRIN4_12, claim!("14", "85.7%", "164.6 total points", "4.3 days", MT)),
        ("trin3_10", "TRIN 3 Days > 1.0", TRIN3_10, claim!("102", "72.5%", "513.8 total points", "4.5 days", MT)),
        ("trin3_11", "TRIN 3 Days > 1.1", TRIN3_11, claim!("62", "77.4%", "437.4 total points", "4.4 days", MT)),
        ("trin2_11", "TRIN 2 Days > 1.1", TRIN2_11, claim!("132", "71.2%", "-", "4.7 days", MT)),
    ] {
        v.push(StrategyDef { id, name, group: "Market Timing · TRIN", session: 7, dir: "Long", status: Live, needs_volume: false, needs_vix: false, needs_series: &["TRIN"], book: "stc9", rules,
            notes: "TRIN = the exchange's Arms index (advancing ÷ declining issues) ÷ (advancing ÷ declining volume), daily close: NSE breadth for Indian indices, NYSE for US ones (NYSE data ends 2020-02-10). Buy on the close of the trigger day [Book silent: as every Session 7 rule].", claim });
    }
    v.push(StrategyDef { id: "lev_calls", name: "Leveraged ETF Call Options", group: "Options", session: 8, dir: "Long", status: NotTestable("needs option chains"), needs_volume: false, needs_vix: false, needs_series: &[], book: "stc9", rules: na,
        notes: "On the next-to-last 1-2-3-4-5 scale-in buy ½ position of the 2nd ITM front-month call, the other ½ on the final scale-in; exit RSI(2) > 50.", claim: None });
    v.push(StrategyDef { id: "busted", name: "Busted Options Program", group: "Options", session: 8, dir: "Short", status: NotTestable("needs earnings + option chains"), needs_volume: false, needs_vix: false, needs_series: &[], book: "stc9", rules: na,
        notes: "SPX < 200 SMA; stock misses earnings and opens ≥ 15% lower; sell 1st/2nd OTM next-month call (premium ≥ $1); hold to expiry, stop at 2× premium.", claim: None });
 
    v.extend(crate::books::catalogue());
    v
}

/// Strategy family and variant label. Rule sets that the course presents as versions of
/// one idea (e.g. the 4 Keys tests 1a..5d) are shown as ONE strategy with variants.
pub fn family_of(id: &str) -> (&'static str, &'static str, &'static str) {
    match id {
        "f1a" => ("keys4", "The 4 Keys of Swing Trading", "1a · Above 200-day MA, 5-day hold"),
        "f3a" => ("keys4", "The 4 Keys of Swing Trading", "3a · + close at a 10-day low"),
        "f4a" => ("keys4", "The 4 Keys of Swing Trading", "4a · + 10-day low, 3% limit entry"),
        "f4c" => ("keys4", "The 4 Keys of Swing Trading", "4c · + 7% limit entry"),
        "f4d" => ("keys4", "The 4 Keys of Swing Trading", "4d · 7% limit, exit above 5-day SMA"),
        "rsi2_5d" => ("keys4", "The 4 Keys of Swing Trading", "5d · RSI(2) < 2, exit above 5-day SMA"),
        "tps_long" => ("tps", "TPS 2-3-5 (Time · Price · Scale-in)", "Long"),
        "tps_short" => ("tps", "TPS 2-3-5 (Time · Price · Scale-in)", "Short"),
        "eom_co" => ("eom", "End of Month", "Close < Open"),
        "eom_rsi" => ("eom", "End of Month", "RSI(2) < 25"),
        "eom_cc" => ("eom", "End of Month", "Close < Close"),
        "rsi2_4" => ("rsi", "RSI Market Timing", "RSI(2) < 4"),
        "rsi4_25" => ("rsi", "RSI Market Timing", "RSI(4) < 25"),
        "rsi_div8" => ("rsi", "RSI Market Timing", "Divergence 8-day"),
        "rsi_div2" => ("rsi", "RSI Market Timing", "Divergence 2-day"),
        "rsi_1" => ("rsi", "RSI Market Timing", "RSI-1 · 3-day sum < 50"),
        "rsi_2" => ("rsi", "RSI Market Timing", "RSI-2 · 2-day sum < 40"),
        "rsi_3" => ("rsi", "RSI Market Timing", "RSI-3 · RSI(3) 2-day sum < 45"),
        "vix200" => ("vix", "VIX Market Timing", "VIX 200 · stretched 5%, 3 days"),
        "vix2" => ("vix", "VIX Market Timing", "VIX (2) · VIX RSI(2) > 90 + gap"),
        "vix4" => ("vix", "VIX Market Timing", "VIX (4) · VIX up 5 days"),
        "vix7" => ("vix", "VIX Market Timing", "VIX (7) · up 4 days + gap"),
        "vix8" => ("vix", "VIX Market Timing", "VIX (8) · stretched 10%, 2 days"),
        "pctb_1" => ("pctb", "%b Market Timing", "%b < 0 two days + RSI"),
        "pctb_2" => ("pctb", "%b Market Timing", "%b < 0.2 three days + bottom 25%"),
        "dmi_1" => ("dmi", "DMI Market Timing", "DMI(1) · +DI(3) < 5"),
        "dmi_2" => ("dmi", "DMI Market Timing", "DMI(2) · +DI(2) < 3"),
        "dmi_3" => ("dmi", "DMI Market Timing", "DMI(3) · +DI(2) < 10, 2 days"),
        "dmi_4" => ("dmi", "DMI Market Timing", "DMI(4) · +DI(2) < 10, 3 days"),
        "dmi_5" => ("dmi", "DMI Market Timing", "DMI(5) · +DI(2) < 15, 4 days"),
        "short_4up" => ("mtshort", "Market Timing Shorts", "4 Up Days"),
        "short_strong" => ("mtshort", "Market Timing Shorts", "Strong 2-day"),
        "vix53_short" => ("mtshort", "Market Timing Shorts", "VIX 5.3"),
        "vix101_short" => ("mtshort", "Market Timing Shorts", "VIX 10.1"),
        "trin4" | "trin4_12" | "trin3_10" | "trin3_11" | "trin2_11" => ("trin", "TRIN Market Timing", ""),
        "r3" => ("r3", "R3 Strategy", ""),
        "tps_stocks" => ("tps_stocks", "TPS for Stocks", ""),
        "x555" => ("x555", "5x5x5 Version 2.0", ""),
        "ss_long" => ("sweetspot", "ETF Sweet Spot", "Long"),
        "ss_short" => ("sweetspot", "ETF Sweet Spot", "Short"),
        "ss_both" => ("sweetspot", "ETF Sweet Spot", "Long & Short"),
        "ss_long_2nd" => ("sweetspot", "ETF Sweet Spot", "Long · 2nd signal only"),
        "ss_short_2nd" => ("sweetspot", "ETF Sweet Spot", "Short · 2nd signal only"),
        "h10" => ("h10", "STC Short Selling (H10)", ""),
        "s12345" => ("s12345", "1-2-3-4-5 Scale-In", ""),
        "price" => ("price", "Price Strategy", ""),
        "lev_calls" => ("lev_calls", "Leveraged ETF Call Options", ""),
        "busted" => ("busted", "Busted Options Program", ""),
        _ => crate::books::family_of(id).unwrap_or(("other", "Other", "")),
    }
}

#[test]
fn every_testable_strategy_has_entry_and_exit_rules() {
    for d in catalogue().iter().filter(|d| d.status == Status::Live) {
        for tag in ["E|", "X|"] {
            assert!(d.rules.iter().any(|r| r.starts_with(tag)), "{} has no {} rule", d.id, tag);
        }
        assert!(d.rules.iter().all(|r| ["F|", "E|", "A|", "X|"].iter().any(|t| r.starts_with(t))), "{} has an untagged rule", d.id);
    }
}

#[test]
fn every_strategy_has_a_family() {
    for d in catalogue() {
        assert_ne!(family_of(d.id).0, "other", "{} has no family", d.id);
    }
}

// ---------------------------------------------------------------- strategy rules

/// Strategy parameters by key; a missing key means the book's value (see `params`).
pub type Params = BTreeMap<String, f64>;

/// One tunable number in a strategy's rules: book default + a sensible search range.
pub struct ParamDef {
    pub key: &'static str,
    pub label: &'static str,
    pub def: f64,
    pub min: f64,
    pub max: f64,
    pub step: f64,
}

const TRIN_X: &str = "X|Exit when the RSI 2-period of the index closes above 65";
const TRIN4: &[&str] = &["F|The index closes above its 200-day moving average", "F|TRIN closes above 1.0 four days in a row", "E|RSI 2-period of the index is below 50; buy on the close", TRIN_X];
const TRIN4_12: &[&str] = &["F|The index closes above its 200-day moving average", "F|TRIN closes above 1.2 four days in a row", "E|RSI 2-period of the index is below 50 on trigger day; buy on the close", TRIN_X];
const TRIN3_10: &[&str] = &["F|The index closes above its 200-day moving average", "F|TRIN closes above 1.0 three days in a row", "E|RSI 2-period of the index is below 50 on trigger day; buy on the close", TRIN_X];
const TRIN3_11: &[&str] = &["F|The index closes above its 200-day moving average", "F|TRIN closes above 1.1 three days in a row", "E|RSI 2-period of the index is below 50 on trigger day; buy on the close", TRIN_X];
const TRIN2_11: &[&str] = &["F|The index closes above its 200-day moving average", "F|TRIN closes above 1.1 two days in a row", "E|RSI 2-period of the index is below 50 on trigger day; buy on the close", TRIN_X];

/// ETF Sweet Spot rules admitted after the 2026-10-21 source review (connors/newrules-misc/SOURCE-CONTRACT.md).
pub const ETFSS_REVIEWED: &[&str] = &["ss_long_2nd", "ss_short_2nd"];
const MT_IDS: &[&str] = &[
    "eom_co", "eom_rsi", "eom_cc", "rsi2_4", "rsi4_25", "rsi_div8", "rsi_div2", "rsi_1", "rsi_2", "rsi_3", "price",
    "pctb_1", "pctb_2", "dmi_1", "dmi_2", "dmi_3", "dmi_4", "dmi_5", "vix200", "vix2", "vix4", "vix7", "vix8",
    "trin4", "trin4_12", "trin3_10", "trin3_11", "trin2_11",
];

/// The numbers each strategy's rules actually use. Defaults are the course values.
pub fn params(id: &str) -> Vec<ParamDef> {
    if let Some(v) = crate::books::params(id) {
        return v;
    }
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    let trend = || p("trend", "Trend filter: SMA length", 200.0, 100.0, 300.0, 25.0);
    let rsi_n = || p("rsi_n", "RSI period", 2.0, 2.0, 5.0, 1.0);
    let hold = || p("hold", "Exit after N bars", 5.0, 2.0, 10.0, 1.0);
    let ma = || p("ma", "Short SMA length", 5.0, 3.0, 10.0, 1.0);
    let lt = |def, min, max, step| p("rsi_lt", "Entry: RSI below", def, min, max, step);
    let gt = |def, min, max, step| p("rsi_gt", "Entry: RSI above", def, min, max, step);
    let ex = |def, min, max, step| p("exit_rsi", "Exit: RSI crosses", def, min, max, step);
    let low = |label| p("low_n", label, 10.0, 5.0, 20.0, 1.0);
    let limit = |def, min, max| p("limit", "Limit: % below close", def, min, max, 0.5);
    let stock = || [p("hv", "HV(100) above", 30.0, 10.0, 50.0, 5.0), p("adx", "ADX(10) above", 30.0, 10.0, 50.0, 5.0), p("lf", "Liquidity Filter above", 3.0, 1.0, 10.0, 1.0)];
    let mut v = match id {
        "f1a" => vec![trend(), hold()],
        "f3a" => vec![trend(), low("Lowest close of N days"), hold()],
        "f4a" => vec![trend(), low("Lowest low of N days"), limit(3.0, 1.0, 8.0), hold()],
        "f4c" => vec![trend(), low("Lowest low of N days"), limit(7.0, 3.0, 10.0), hold()],
        "f4d" => vec![trend(), low("Lowest low of N days"), limit(7.0, 3.0, 10.0), ma()],
        "rsi2_5d" => vec![trend(), rsi_n(), lt(2.0, 1.0, 10.0, 1.0), ma()],
        "r3" => vec![trend(), rsi_n(), p("fall", "RSI falling N days", 3.0, 2.0, 5.0, 1.0), p("first_lt", "First falling day RSI below", 60.0, 40.0, 80.0, 5.0),
            lt(10.0, 5.0, 20.0, 1.0), p("add_fall", "Add unit 2 after N falling days", 5.0, 4.0, 7.0, 1.0), ex(75.0, 60.0, 90.0, 5.0), ma()],
        "tps_stocks" => {
            let mut v = vec![trend(), p("down", "Down closes in a row", 3.0, 1.0, 5.0, 1.0), rsi_n(), lt(1.0, 1.0, 10.0, 1.0),
                p("add_pct", "Add when % below last buy", 0.5, 0.0, 2.0, 0.25), ex(70.0, 50.0, 90.0, 5.0)];
            v.extend(stock());
            v
        }
        "x555" => {
            let mut v = vec![trend(), p("down", "Down closes in a row", 2.0, 1.0, 4.0, 1.0), ma(), p("stretch", "Close % below short SMA", 5.0, 2.0, 10.0, 1.0),
                p("limit_min", "Min limit % below close", 5.0, 2.0, 8.0, 1.0), p("profit", "Exit at % profit", 5.0, 2.0, 10.0, 1.0)];
            v.extend(stock());
            v
        }
        "h10" => vec![trend(), p("lf", "Liquidity Filter above", 20.0, 5.0, 40.0, 5.0), rsi_n(), gt(90.0, 80.0, 98.0, 2.0), p("hv", "HV(100) above", 40.0, 20.0, 60.0, 5.0),
            p("adx", "ADX(10) above", 40.0, 20.0, 60.0, 5.0), p("hh_n", "Highest high of N days", 10.0, 5.0, 20.0, 1.0), p("atr_k", "Limit: × ATR(10) above close", 0.4, 0.0, 1.0, 0.1), ex(40.0, 20.0, 60.0, 5.0)],
        "tps_long" => vec![trend(), p("band", "Close % above trend SMA", 1.5, 0.0, 3.0, 0.5), rsi_n(), lt(30.0, 10.0, 40.0, 5.0), ex(70.0, 55.0, 85.0, 5.0), p("stop", "Stop: % below trend SMA", 1.5, 0.5, 5.0, 0.5)],
        "tps_short" => vec![trend(), p("band", "Close % below trend SMA", 1.5, 0.0, 3.0, 0.5), rsi_n(), gt(75.0, 60.0, 90.0, 5.0), ex(25.0, 10.0, 40.0, 5.0), p("stop", "Stop: % above trend SMA", 1.5, 0.5, 5.0, 0.5)],
        "s12345" => vec![rsi_n(), lt(5.0, 2.0, 15.0, 1.0), ex(50.0, 40.0, 80.0, 5.0)],
        "vix53_short" | "vix101_short" => {
            let (s, d) = if id == "vix53_short" { (5.0, 3.0) } else { (10.0, 1.0) };
            vec![trend(), p("stretch", "VIX % below its 10-day SMA", s, 2.0, 15.0, 1.0), p("days", "…for N days in a row", d, 1.0, 5.0, 1.0), rsi_n(), gt(80.0, 60.0, 95.0, 5.0), ma()]
        }
        "short_4up" => vec![trend(), p("up", "Up closes in a row", 4.0, 2.0, 7.0, 1.0), ma()],
        "ss_long" | "ss_short" | "ss_both" | "ss_long_2nd" | "ss_short_2nd" => vec![trend(), p("ma", "Mean SMA length (setup, stretch, exit)", 10.0, 5.0, 20.0, 1.0), p("hv_n", "Volatility lookback (HV days)", 40.0, 20.0, 60.0, 5.0),
            p("hv_div", "HV per 1% of stretch", 15.0, 10.0, 25.0, 1.0), p("min_stretch", "Minimum stretch %", 1.0, 0.0, 2.0, 0.25),
            // the user's 30 Oct approval names ss_long, ss_short and ss_both: the two 2nd-signal cards stay on 252 until extended
            p("hv_days", "Trading days per year in HV (256 = seminar p.163; 252 = common convention)", if id.ends_with("_2nd") { 252.0 } else { 256.0 }, 252.0, 256.0, 4.0)],
        "short_strong" => vec![trend(), p("rise", "2-day rise above %", 3.0, 1.0, 6.0, 0.5), rsi_n(), gt(80.0, 60.0, 95.0, 5.0), ma()],
        _ if MT_IDS.contains(&id) => vec![trend(), rsi_n(), ex(65.0, 50.0, 85.0, 5.0)],
        _ => vec![],
    };
    let days = |def| p("days", "…for N days in a row", def, 1.0, 5.0, 1.0);
    v.extend(match id {
        "eom_co" | "eom_cc" => vec![p("dom", "Day of month from", 25.0, 15.0, 28.0, 1.0), p("bars", if id == "eom_co" { "Red bars in a row" } else { "Lower closes in a row" }, 2.0, 1.0, 4.0, 1.0)],
        "eom_rsi" => vec![p("dom", "Day of month from", 25.0, 15.0, 28.0, 1.0), lt(25.0, 10.0, 40.0, 5.0)],
        "rsi2_4" => vec![lt(4.0, 1.0, 15.0, 1.0)],
        "trin4" | "trin4_12" | "trin3_10" | "trin3_11" | "trin2_11" => {
            let (lvl, n) = match id { "trin4" => (1.0, 4.0), "trin4_12" => (1.2, 4.0), "trin3_10" => (1.0, 3.0), "trin3_11" => (1.1, 3.0), _ => (1.1, 2.0) };
            vec![p("trin_lvl", "TRIN closes above", lvl, 0.8, 1.6, 0.05), days(n), lt(50.0, 20.0, 70.0, 5.0)]
        }
        "rsi4_25" => vec![p("slow_n", "Second RSI period", 4.0, 3.0, 6.0, 1.0), p("slow_lt", "Second RSI below", 25.0, 10.0, 40.0, 5.0)],
        "rsi_div8" => vec![p("look", "Divergence lookback", 8.0, 3.0, 15.0, 1.0), lt(15.0, 5.0, 30.0, 5.0)],
        "rsi_div2" => vec![p("look", "Divergence lookback", 2.0, 2.0, 8.0, 1.0), lt(30.0, 10.0, 40.0, 5.0)],
        "rsi_1" => vec![p("days", "Sum RSI over N days", 3.0, 2.0, 5.0, 1.0), p("sum_lt", "RSI sum below", 50.0, 20.0, 100.0, 10.0)],
        "rsi_2" => vec![p("days", "Sum RSI over N days", 2.0, 2.0, 5.0, 1.0), p("sum_lt", "RSI sum below", 40.0, 20.0, 100.0, 10.0)],
        "rsi_3" => vec![p("sum_n", "Summed RSI period", 3.0, 2.0, 5.0, 1.0), p("days", "Sum RSI over N days", 2.0, 2.0, 5.0, 1.0), p("sum_lt", "RSI sum below", 45.0, 20.0, 100.0, 5.0)],
        "price" => vec![p("bars", "Red bars in a row", 3.0, 1.0, 5.0, 1.0), p("loc", "Close in bottom % of range", 10.0, 5.0, 30.0, 5.0)],
        "pctb_1" => vec![p("pctb_n", "%b period", 5.0, 3.0, 10.0, 1.0), days(2.0), p("pctb_lt", "%b below", 0.0, -0.2, 0.2, 0.1), lt(10.0, 5.0, 30.0, 5.0)],
        "pctb_2" => vec![p("pctb_n", "%b period", 5.0, 3.0, 10.0, 1.0), days(3.0), p("pctb_lt", "%b below", 0.2, 0.0, 0.5, 0.1), p("loc", "Close in bottom % of range", 25.0, 10.0, 50.0, 5.0)],
        "dmi_1" | "dmi_2" | "dmi_3" | "dmi_4" | "dmi_5" => {
            let (n, d, t) = match id { "dmi_1" => (3.0, 1.0, 5.0), "dmi_2" => (2.0, 1.0, 3.0), "dmi_3" => (2.0, 2.0, 10.0), "dmi_4" => (2.0, 3.0, 10.0), _ => (2.0, 4.0, 15.0) };
            vec![p("di_n", "+DI period", n, 2.0, 10.0, 1.0), days(d), p("di_lt", "+DI below", t, 1.0, 25.0, 1.0)]
        }
        "vix200" | "vix8" => {
            let (s, d, r) = if id == "vix200" { (5.0, 3.0, 10.0) } else { (10.0, 2.0, 30.0) };
            vec![p("stretch", "VIX % above its 10-day SMA", s, 2.0, 15.0, 1.0), days(d), lt(r, 5.0, 40.0, 5.0)]
        }
        "vix2" => vec![p("vix_rsi", "VIX RSI(2) above", 90.0, 70.0, 98.0, 2.0), lt(30.0, 10.0, 40.0, 5.0)],
        "vix4" | "vix7" => vec![p("up", "VIX up closes in a row", if id == "vix4" { 5.0 } else { 4.0 }, 2.0, 7.0, 1.0), lt(30.0, 10.0, 40.0, 5.0)],
        _ => vec![],
    });
    v
}

pub fn run(id: &str, ctx: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    let x = ctx;
    let c = &x.c;
    let schema = params(id);
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).unwrap_or_else(|| panic!("{id} has no param {k}")).def);
    let u = |k: &str| g(k).round().max(1.0) as usize;
    let one: &[f64] = &[1.0];
    let long = |entry: &dyn Fn(usize) -> Option<Entry>, exit: &dyn Fn(usize, &Trade) -> Option<Exit>| {
        simulate(x, &Spec { dir: 1.0, weights: one, entry, add: &no_add, exit }, from, to)
    };
    let short = |entry: &dyn Fn(usize) -> Option<Entry>, exit: &dyn Fn(usize, &Trade) -> Option<Exit>| {
        simulate(x, &Spec { dir: -1.0, weights: one, entry, add: &no_add, exit }, from, to)
    };
    let at_close = |b: bool| b.then_some(Entry::Close);
    let dom = |i: usize| x.r[i].date[8..10].parse::<u32>().unwrap_or(0);
    let pct = |k: &str| g(k) / 100.0;
    let trend = || x.sma_n(u("trend"));
    let rsi = || x.rsi_n(u("rsi_n"));
    let hold = |n: usize| move |i: usize, t: &Trade| (i >= t.legs[0].i + n).then(|| Exit::Close(format!("{n} bars held")));

    let trades = match id {
        "f1a" | "f3a" => {
            let (tr, n) = (trend(), u("hold"));
            let low = |i| id == "f1a" || x.lowest_close(i, u("low_n"));
            long(&|i| at_close(c[i] > tr[i] && low(i)), &hold(n))
        }
        "f4a" | "f4c" | "f4d" => {
            let (tr, lim, low) = (trend(), pct("limit"), u("low_n"));
            let entry = |i: usize| (c[i] > tr[i] && x.lowest_low(i, low)).then(|| Entry::Limit(c[i] * (1.0 - lim)));
            if id == "f4d" {
                let (ma, m) = (x.sma_n(u("ma")), u("ma"));
                long(&entry, &|i, _| (c[i] > ma[i]).then(|| Exit::Close(format!("Close > {m}-day SMA"))))
            } else {
                long(&entry, &hold(u("hold")))
            }
        }
        "rsi2_5d" => {
            let (tr, rs, lt, ma, m) = (trend(), rsi(), g("rsi_lt"), x.sma_n(u("ma")), u("ma"));
            long(&|i| at_close(c[i] > tr[i] && rs[i] < lt), &|i, _| (c[i] > ma[i]).then(|| Exit::Close(format!("Close > {m}-day SMA"))))
        }
        "r3" => {
            let (tr, rs, n, ma, m) = (trend(), rsi(), u("rsi_n"), x.sma_n(u("ma")), u("ma"));
            let (fall, first_lt, lt, add, ex) = (u("fall"), g("first_lt"), g("rsi_lt"), u("add_fall"), g("exit_rsi"));
            let falling = streak(c.len(), |i| i > 0 && rs[i] < rs[i - 1]);
            simulate(x, &Spec {
                dir: 1.0,
                weights: &[0.5, 0.5],
                entry: &|i| at_close(c[i] > tr[i] && falling[i] >= fall && rs[i + 1 - fall] < first_lt && rs[i] < lt),
                add: &|i, _| falling[i] >= add,
                exit: &|i, _| if rs[i] > ex { Some(Exit::Close(format!("RSI({n}) > {ex}"))) } else { (c[i] > ma[i]).then(|| Exit::Close(format!("Close > {m}-day SMA"))) },
            }, from, to)
        }
        "tps_stocks" => {
            let (tr, rs, n, ex, lt, add) = (trend(), rsi(), u("rsi_n"), g("exit_rsi"), g("rsi_lt"), 1.0 - pct("add_pct"));
            let (hv, adx, lf, down) = (g("hv"), g("adx"), g("lf"), u("down"));
            simulate(x, &Spec {
                dir: 1.0,
                weights: &[0.2, 0.3, 0.5],
                entry: &|i| at_close(c[i] > tr[i] && x.hv100[i] > hv && x.adx10[i] > adx && x.lf[i] > lf && x.down_closes[i] >= down && rs[i] < lt),
                add: &|i, t| c[i] <= t.legs.last().unwrap().px * add,
                exit: &|i, _| (rs[i] > ex).then(|| Exit::Close(format!("RSI({n}) > {ex}"))),
            }, from, to)
        }
        "x555" => {
            let (tr, ma, m) = (trend(), x.sma_n(u("ma")), u("ma"));
            let (hv, adx, lf, down, st, lmin, prof) = (g("hv"), g("adx"), g("lf"), u("down"), 1.0 - pct("stretch"), g("limit_min"), 1.0 + pct("profit"));
            let pr = g("profit");
            long(
                &|i| (c[i] > tr[i] && x.lf[i] > lf && x.hv100[i] > hv && x.adx10[i] > adx && x.down_closes[i] >= down && c[i] <= ma[i] * st)
                    .then(|| Entry::Limit(c[i] * (1.0 - lmin.max(x.hv100[i] / 252f64.sqrt()) / 100.0))),
                &|i, t| if c[i] > ma[i] { Some(Exit::NextOpen(format!("Close > {m}-day SMA, sold next open"))) } else { (c[i] >= t.avg_px() * prof).then(|| Exit::NextOpen(format!("Close ≥ {pr}% profit, sold next open"))) },
            )
        }
        "h10" => {
            let (tr, rs, n, ex, gt) = (trend(), rsi(), u("rsi_n"), g("exit_rsi"), g("rsi_gt"));
            let (lf, hv, adx, hh, k) = (g("lf"), g("hv"), g("adx"), u("hh_n"), g("atr_k"));
            short(
                &|i| (x.lf[i] > lf && rs[i] > gt && x.hv100[i] > hv && x.adx10[i] > adx && c[i] > tr[i] && x.highest_high(i, hh)).then(|| Entry::Limit(c[i] + k * x.atr10[i])),
                &|i, _| (rs[i] < ex).then(|| Exit::NextOpen(format!("RSI({n}) < {ex}, covered next open"))),
            )
        }
        "tps_long" | "tps_short" => {
            let (tr, rs, n, ex, t) = (trend(), rsi(), u("rsi_n"), g("exit_rsi"), u("trend"));
            let (band, stop, sp) = (pct("band"), pct("stop"), g("stop"));
            if id == "tps_long" {
                let lt = g("rsi_lt");
                simulate(x, &Spec {
                    dir: 1.0,
                    weights: &[0.2, 0.3, 0.5],
                    entry: &|i| at_close(c[i] >= tr[i] * (1.0 + band) && rs[i] < lt),
                    add: &|i, t| c[i] < t.legs.last().unwrap().px,
                    exit: &|i, _| if rs[i] > ex { Some(Exit::Close(format!("RSI({n}) > {ex}"))) } else { (c[i] <= tr[i] * (1.0 - stop)).then(|| Exit::Close(format!("Close ≥ {sp}% below {t}-day SMA"))) },
                }, from, to)
            } else {
                let gt = g("rsi_gt");
                simulate(x, &Spec {
                    dir: -1.0,
                    weights: &[0.2, 0.3, 0.5],
                    entry: &|i| at_close(i > 0 && c[i] <= tr[i] * (1.0 - band) && rs[i] > gt && rs[i - 1] > gt),
                    add: &|i, t| c[i] > t.legs.last().unwrap().px,
                    exit: &|i, _| if rs[i] < ex { Some(Exit::Close(format!("RSI({n}) < {ex}"))) } else { (c[i] >= tr[i] * (1.0 + stop)).then(|| Exit::Close(format!("Close ≥ {sp}% above {t}-day SMA"))) },
                }, from, to)
            }
        }
        "s12345" => {
            let (rs, n, lt, ex) = (rsi(), u("rsi_n"), g("rsi_lt"), g("exit_rsi"));
            let vol: Vec<f64> = x.r.iter().map(|r| r.volume.unwrap_or(f64::NAN)).collect();
            let avg21 = sma(&vol, 21);
            let liquid = |i: usize| avg21[i] > 250_000.0 && i >= 20 && vol[i - 20..=i].iter().all(|&v| v >= 50_000.0);
            simulate(x, &Spec {
                dir: 1.0,
                weights: &[1.0 / 15.0, 2.0 / 15.0, 3.0 / 15.0, 4.0 / 15.0, 5.0 / 15.0],
                entry: &|i| at_close(liquid(i) && rs[i] < lt),
                add: &|i, t| c[i] < t.legs.last().unwrap().px,
                exit: &|i, _| (rs[i] > ex).then(|| Exit::Close(format!("RSI({n}) > {ex}"))),
            }, from, to)
        }
        // Market-timing family: condition on the close, buy the close, exit RSI > 65.
        // "RSI(2) <= 65" entry caps track the exit level so a trade never exits on its entry bar.
        _ if MT_IDS.contains(&id) => {
            let (tr, rs_v, n, ex) = (trend(), rsi(), u("rsi_n"), g("exit_rsi"));
            let rs: &[f64] = &rs_v;
            let cap = |i: usize| rs[i] <= ex;
            let all = |i: usize, d: usize, f: &dyn Fn(usize) -> bool| x.all(i, d, f);
            let cond: Box<dyn Fn(usize) -> bool + '_> = match id {
                "eom_co" | "eom_cc" => {
                    let (d, b) = (g("dom") as u32, u("bars"));
                    let bars = if id == "eom_co" { &x.red_bars } else { &x.down_closes };
                    Box::new(move |i| dom(i) >= d && bars[i] >= b && cap(i))
                }
                "eom_rsi" => {
                    let (d, lt) = (g("dom") as u32, g("rsi_lt"));
                    Box::new(move |i| dom(i) >= d && rs[i] < lt)
                }
                "rsi2_4" => {
                    let lt = g("rsi_lt");
                    Box::new(move |i| rs[i] < lt)
                }
                "rsi4_25" => {
                    let (r4, lt) = (x.rsi_n(u("slow_n")), g("slow_lt"));
                    Box::new(move |i| r4[i] < lt && cap(i))
                }
                "rsi_div8" | "rsi_div2" => {
                    let (k, lt) = (u("look"), g("rsi_lt"));
                    Box::new(move |i| i >= k && c[i] < c[i - k] && rs[i] > rs[i - k] && rs[i] < lt)
                }
                "rsi_1" | "rsi_2" | "rsi_3" => {
                    let r = if id == "rsi_3" { x.rsi_n(u("sum_n")) } else { rsi() };
                    let (d, lt) = (u("days"), g("sum_lt"));
                    Box::new(move |i| i + 1 >= d && (0..d).map(|k| r[i - k]).sum::<f64>() < lt)
                }
                "price" => {
                    let (b, loc) = (u("bars"), pct("loc"));
                    Box::new(move |i| x.red_bars[i] >= b && x.close_loc(i) <= loc && cap(i))
                }
                "pctb_1" | "pctb_2" => {
                    let (pb, d, t) = (x.pctb_n(u("pctb_n")), u("days"), g("pctb_lt"));
                    if id == "pctb_1" {
                        let lt = g("rsi_lt");
                        Box::new(move |i| all(i, d, &|k| pb[k] < t) && rs[i] < lt)
                    } else {
                        let loc = pct("loc");
                        Box::new(move |i| all(i, d, &|k| pb[k] < t) && x.close_loc(i) <= loc && cap(i))
                    }
                }
                "dmi_1" | "dmi_2" | "dmi_3" | "dmi_4" | "dmi_5" => {
                    let (di, d, t) = (x.pdi_n(u("di_n")), u("days"), g("di_lt"));
                    Box::new(move |i| all(i, d, &|k| di[k] < t) && cap(i))
                }
                "vix200" | "vix8" => {
                    let (s, d, lt) = (x.vix.stretched(g("stretch")), u("days"), g("rsi_lt"));
                    Box::new(move |i| s[i] >= d && rs[i] < lt)
                }
                "trin4" | "trin4_12" | "trin3_10" | "trin3_11" | "trin2_11" => {
                    let (lvl, d, lt) = (g("trin_lvl"), u("days"), g("rsi_lt"));
                    let t: &[f64] = x.series("TRIN").unwrap_or(&[]);
                    Box::new(move |i| i + 1 >= d && (i + 1 - d..=i).all(|k| t.get(k).is_some_and(|v| *v > lvl)) && rs[i] < lt)
                }
                "vix2" => {
                    let (v, lt) = (g("vix_rsi"), g("rsi_lt"));
                    Box::new(move |i| x.vix.rsi2[i] > v && x.vix.gap_over_close[i] && rs[i] < lt)
                }
                _ => {
                    let (up, lt, gap) = (u("up"), g("rsi_lt"), id == "vix7");
                    Box::new(move |i| x.vix.up[i] >= up && (!gap || x.vix.gap_over_high[i]) && rs[i] < lt)
                }
            };
            long(&|i| at_close(c[i] > tr[i] && cond(i)), &|i, _| (rs[i] > ex).then(|| Exit::Close(format!("RSI({n}) > {ex}"))))
        }
        "ss_long" | "ss_short" | "ss_both" | "ss_long_2nd" | "ss_short_2nd" => {
            let (tr, ma, m, hvv) = (trend(), x.sma_n(u("ma")), u("ma"), hv_days(c, u("hv_n"), 0.0, g("hv_days")));
            let (div, floor) = (g("hv_div"), g("min_stretch"));
            // stretch = at least `floor`% beyond the SMA, or 1% for every `div`% of HV if that is larger (book: max(1%, HV/15))
            let st = |i: usize| (hvv[i] / div).max(floor) / 100.0;
            let buy = |i: usize| c[i] > tr[i] && x.h[i] < ma[i] && c[i] <= ma[i] * (1.0 - st(i));
            let sell = |i: usize| c[i] < tr[i] && x.l[i] > ma[i] && c[i] >= ma[i] * (1.0 + st(i));
            let side = |dir: f64| {
                let sig = |i: usize| if dir > 0.0 { buy(i) } else { sell(i) };
                simulate(x, &Spec {
                    dir,
                    weights: &[0.5, 0.5],
                    entry: &|i| at_close(sig(i)),
                    add: &|i, _| sig(i),
                    exit: &|i, _| {
                        let crossed = i > 0 && if dir > 0.0 { c[i - 1] > ma[i - 1] } else { c[i - 1] < ma[i - 1] };
                        crossed.then(|| Exit::Close(format!("Yesterday's close {} {m}-day SMA", if dir > 0.0 { ">" } else { "<" })))
                    },
                }, from, to)
            };
            match id {
                "ss_long" => side(1.0),
                "ss_short" => side(-1.0),
                // 2nd signal only: the Rule #5 unit of each (untraded) first-signal position, closed by its Rule #6 exit
                "ss_long_2nd" | "ss_short_2nd" => side(if id == "ss_long_2nd" { 1.0 } else { -1.0 })
                    .into_iter()
                    .filter(|t| t.legs.len() == 2)
                    .map(|t| Trade { legs: vec![Leg { w: 1.0, ..t.legs[1] }], ..t })
                    .collect(),
                _ => {
                    // one position per ETF: merge both sides and skip a signal that starts while the other side is open
                    // ponytail: skips the overlapping trade instead of re-simulating; rule 1 (200-day filter) makes overlaps rare
                    let mut all = [side(1.0), side(-1.0)].concat();
                    all.sort_by_key(|t| t.legs[0].i);
                    let mut busy = None;
                    all.into_iter().filter(|t| { let free = busy.map_or(true, |b| t.legs[0].i > b); if free { busy = Some(t.exit_i); } free }).collect()
                }
            }
        }
        "vix53_short" | "vix101_short" | "short_4up" | "short_strong" => {
            let (tr, ma, m) = (trend(), x.sma_n(u("ma")), u("ma"));
            let setup: Box<dyn Fn(usize) -> bool + '_> = match id {
                "short_4up" => {
                    let up = u("up");
                    Box::new(move |i| x.up_closes[i] >= up)
                }
                "short_strong" => {
                    let (rs, rise, gt) = (rsi(), pct("rise"), g("rsi_gt"));
                    Box::new(move |i| i >= 2 && c[i] / c[i - 2] - 1.0 > rise && rs[i] > gt)
                }
                _ => {
                    let (rs, s, d, gt) = (rsi(), x.vix.stretched(-g("stretch")), u("days"), g("rsi_gt"));
                    Box::new(move |i| s[i] >= d && rs[i] > gt)
                }
            };
            short(&|i| at_close(c[i] < tr[i] && setup(i) && c[i] >= ma[i]), &|i, _| (c[i] < ma[i]).then(|| Exit::Close(format!("Close < {m}-day SMA"))))
        }
        _ => return crate::books::run(id, ctx, from, to, p),
    };
    Some(trades)
}


// ---------------------------------------------------------------- tests

#[cfg(test)]
mod tests {
    use super::*;

    fn row(d: usize, o: f64, h: f64, l: f64, c: f64) -> Row {
        Row { ticker: "T".into(), date: format!("2020-01-{:02}", d % 28 + 1), open: o, high: h, low: l, close: c, volume: Some(1e6) }
    }

    #[test]
    fn rsi_matches_wilder_hand_calc() {
        // closes 10, 11, 10, 12 with n=2:
        // changes +1, -1, +2 ; seed avg up = (1+0)/2 = .5, dn = (0+1)/2 = .5 -> RSI 50 at i=2
        // i=3: up = (.5*1 + 2)/2 = 1.25, dn = (.5*1 + 0)/2 = .25 -> RSI = 100 - 100/6
        let r = rsi(&[10.0, 11.0, 10.0, 12.0], 2);
        assert!(r[1].is_nan());
        assert!((r[2] - 50.0).abs() < 1e-9);
        assert!((r[3] - (100.0 - 100.0 / 6.0)).abs() < 1e-9);
    }

    #[test]
    fn sweet_spot_hv_uses_256_days_and_keeps_252_as_option() {
        let c: Vec<f64> = (0..60).map(|i| 100.0 + (i as f64 * 0.7).sin() * 3.0).collect();
        let (a, b) = (hv_days(&c, 40, 0.0, 256.0)[50], hv_days(&c, 40, 0.0, 252.0)[50]);
        assert!((a / b - (256.0f64 / 252.0).sqrt()).abs() < 1e-12);
        assert_eq!(hv_ddof(&c, 40, 0.0)[50], b); // every other caller stays on 252, bit for bit
        let def = |id: &str| params(id).iter().find(|d| d.key == "hv_days").unwrap().def;
        for id in ["ss_long", "ss_short", "ss_both"] { assert_eq!(def(id), 256.0); }
        for id in ["ss_long_2nd", "ss_short_2nd"] { assert_eq!(def(id), 252.0); }
    }

    #[test]
    fn hv_of_constant_growth_is_zero_and_pctb_bounds() {
        let c: Vec<f64> = (0..120).map(|i| 100.0 * 1.01f64.powi(i)).collect();
        assert!(hv(&c, 100)[110].abs() < 1e-6);
        let b = pct_b(&[1.0, 2.0, 3.0, 4.0, 5.0], 5, 1.0);
        // mean 3, pop sd sqrt(2): %b = (5 - (3 - 1.414)) / 2.828
        assert!((b[4] - (5.0 - (3.0 - 2f64.sqrt())) / (2.0 * 2f64.sqrt())).abs() < 1e-9);
    }

    #[test]
    fn engine_scale_in_and_limit_fill() {
        // 3-leg TPS-style long: enter 100, add 95, add 90, exit 110
        let px = [100.0, 95.0, 90.0, 110.0];
        let rows: Vec<Row> = px.iter().enumerate().map(|(i, &p)| row(i, p, p, p, p)).collect();
        let ctx = Ctx::new(&rows, &rows, &[]);
        let t = simulate(&ctx, &Spec {
            dir: 1.0,
            weights: &[0.2, 0.3, 0.5],
            entry: &|i| (i == 0).then_some(Entry::Close),
            add: &|i, t| ctx.c[i] < t.legs.last().unwrap().px,
            exit: &|i, _| (ctx.c[i] > 105.0).then(|| Exit::Close("test".into())),
        }, 0, 3);
        assert_eq!(t.len(), 1);
        assert_eq!(t[0].legs.len(), 3);
        let want = 0.2 * (110.0 / 100.0 - 1.0) + 0.3 * (110.0 / 95.0 - 1.0) + 0.5 * (110.0 / 90.0 - 1.0);
        assert!((t[0].ret(0.0) - want).abs() < 1e-12);

        // limit 3% below 100 -> 97; next bar opens 96 (gap through) -> fill at the open 96
        let rows = vec![row(0, 100.0, 100.0, 100.0, 100.0), row(1, 96.0, 99.0, 95.0, 98.0), row(2, 98.0, 99.0, 97.0, 99.0)];
        let ctx = Ctx::new(&rows, &rows, &[]);
        let t = simulate(&ctx, &Spec {
            dir: 1.0,
            weights: &[1.0],
            entry: &|i| (i == 0).then(|| Entry::Limit(97.0)),
            add: &no_add,
            exit: &|_, _| None,
        }, 0, 2);
        assert_eq!(t[0].legs[0].px, 96.0);
        assert!(t[0].open);
    }
}

#[cfg(test)]
mod sma_gap_tests {
    use super::sma;
    #[test]
    fn sma_recovers_after_a_gap() {
        let v = [1.0, 2.0, f64::NAN, 4.0, 5.0, 6.0, 7.0];
        let m = sma(&v, 2);
        assert!(m[2].is_nan() && m[3].is_nan());
        assert_eq!((m[1], m[4], m[5], m[6]), (1.5, 4.5, 5.5, 6.5));
        let plain = [3.0, 1.0, 4.0, 1.0, 5.0];
        assert_eq!(sma(&plain, 3)[2..], [8.0 / 3.0, 2.0, 10.0 / 3.0]);
    }
}
