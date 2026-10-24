//! Lars N. Kestner, "A Comparison of Popular Trading Systems", 2nd ed. (Commodity Traders' Consumer Report, 1999).
//! Book id `lkcpt`. Source: research drive "Trading Knowledge I/Lars Kestner - A Comparison of Popular Trading
//! Systems (2nd Ed.)/copts.pdf" (244 pp.). Every system is implemented from Kestner's own EasyLanguage in the
//! Appendix (pp.225-244), which the book says generated its results; where the prose differs the code wins and the
//! difference is listed in library/lkcpt/README.md. These are Kestner's codings ("my personal interpretation of each
//! author's idea", p.9) of systems by Barnes, Bernstein, Chande & Kroll, DeMark, Jensen, Krutsinger and Wilder.
//!
//! Conventions (p.10): signals at the day's close, orders for the next day; market orders at the next open; stop and
//! limit orders fill when the next bar trades through the price (at the open if it gaps through). Several stop/limit
//! orders on one bar are filled in price-path order (open -> nearer extreme -> other extreme -> close). An entry
//! order is ignored while already in that direction (no pyramiding); an opposite entry reverses; exit orders apply
//! only to the position held when they were placed. "Exit #1" (pp.10, 244, used where the author gave no exit):
//! exit at the next open once the close is more than 3 x StdDev(Close - Close[1], 30) (population, TradeStation) away
//! from the entry price. No commissions. The book used variable futures contract counts (p.9); here one unit.
//! Duplicates not re-coded: the control systems 10/40 MA, 20-day momentum and 14-day RSI 35/65 are the same rules as
//! lkqts_ma1040 / lkqts_mom20 / lkqts_rsi14 (Kestner, Quantitative Trading Strategies).
use crate::connors::{rsi, sma, true_range, Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "lkcpt";
/// Admitted ids (authors/kestner/admission/SOURCE-CONTRACT.md).
pub const REVIEWED: &[&str] = &["lkcpt_cb4020", "lkcpt_stoch2575", "lkcpt_box10", "lkcpt_box40", "lkcpt_drv", "lkcpt_osc", "lkcpt_osc_adx", "lkcpt_spop", "lkcpt_spop_rev", "lkcpt_ctod", "lkcpt_dema", "lkcpt_pctf", "lkcpt_vidya", "lkcpt_qstick8", "lkcpt_qstick20", "lkcpt_cmo", "lkcpt_tdline1", "lkcpt_tdline3", "lkcpt_rsimild", "lkcpt_reimild", "lkcpt_demarker", "lkcpt_obos", "lkcpt_kurt03", "lkcpt_kurt10", "lkcpt_fsrs", "lkcpt_123", "lkcpt_ons", "lkcpt_tts3", "lkcpt_tts10", "lkcpt_qp", "lkcpt_gap", "lkcpt_fib", "lkcpt_jesse", "lkcpt_wvol", "lkcpt_wdmi"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

// ------------------------------------------------------------------ EasyLanguage helpers

fn xavg(v: &[f64], n: f64) -> Vec<f64> {
    let a = 2.0 / (n + 1.0);
    let mut out = vec![f64::NAN; v.len()];
    let mut p = f64::NAN;
    for i in 0..v.len() {
        if !v[i].is_finite() {
            continue;
        }
        p = if p.is_nan() { v[i] } else { a * v[i] + (1.0 - a) * p };
        out[i] = p;
    }
    out
}
/// TradeStation StdDev: population standard deviation of the last n values.
fn stdev(v: &[f64], n: usize) -> Vec<f64> {
    let mut out = vec![f64::NAN; v.len()];
    for i in n.saturating_sub(1)..v.len() {
        let w = &v[i + 1 - n..=i];
        if w.iter().any(|x| !x.is_finite()) {
            continue;
        }
        let m = w.iter().sum::<f64>() / n as f64;
        out[i] = (w.iter().map(|x| (x - m).powi(2)).sum::<f64>() / n as f64).sqrt();
    }
    out
}
/// TradeStation WAverage: linear weights n..1, newest heaviest.
fn wavg(v: &[f64], n: usize) -> Vec<f64> {
    let den = (n * (n + 1) / 2) as f64;
    (0..v.len()).map(|i| if i + 1 < n { f64::NAN } else { (0..n).map(|k| (n - k) as f64 * v[i - k]).sum::<f64>() / den }).collect()
}
fn hh(v: &[f64], i: usize, n: usize) -> f64 {
    if i + 1 < n { return f64::NAN }
    v[i + 1 - n..=i].iter().cloned().fold(f64::NEG_INFINITY, f64::max)
}
fn ll(v: &[f64], i: usize, n: usize) -> f64 {
    if i + 1 < n { return f64::NAN }
    v[i + 1 - n..=i].iter().cloned().fold(f64::INFINITY, f64::min)
}
fn chg(c: &[f64]) -> Vec<f64> {
    (0..c.len()).map(|i| if i == 0 { f64::NAN } else { c[i] - c[i - 1] }).collect()
}
/// FastK = 100 (C - LL)/(HH - LL); SlowK = 3-bar SMA of FastK (= FastD); SlowD = 3-bar SMA of SlowK.
fn stoch(x: &Ctx, n: usize) -> (Vec<f64>, Vec<f64>, Vec<f64>) {
    let k: Vec<f64> = (0..x.c.len()).map(|i| {
        let (h, l) = (hh(&x.h, i, n), ll(&x.l, i, n));
        if h > l { 100.0 * (x.c[i] - l) / (h - l) } else { f64::NAN }
    }).collect();
    let sk = sma(&k, 3);
    let sd = sma(&sk, 3);
    (k, sk, sd)
}
/// Wilder +DI, -DI and ADX (TradeStation DMIPlus/DMIMinus/ADX).
fn dmi3(x: &Ctx, n: usize) -> (Vec<f64>, Vec<f64>, Vec<f64>) {
    let len = x.c.len();
    let tr = true_range(x.r);
    let (mut p, mut m, mut t) = (vec![f64::NAN; len], vec![f64::NAN; len], vec![f64::NAN; len]);
    for i in 1..len {
        let up = x.h[i] - x.h[i - 1];
        let dn = x.l[i - 1] - x.l[i];
        p[i] = if up > dn && up > 0.0 { up } else { 0.0 };
        m[i] = if dn > up && dn > 0.0 { dn } else { 0.0 };
        t[i] = tr[i];
    }
    let (sp, sm, st) = (crate::connors::wilder(&p, n), crate::connors::wilder(&m, n), crate::connors::wilder(&t, n));
    let mut pdi = vec![f64::NAN; len];
    let mut mdi = vec![f64::NAN; len];
    let mut dx = vec![f64::NAN; len];
    for i in 0..len {
        if st[i] > 0.0 {
            pdi[i] = 100.0 * sp[i] / st[i];
            mdi[i] = 100.0 * sm[i] / st[i];
            let s = pdi[i] + mdi[i];
            dx[i] = if s > 0.0 { 100.0 * (pdi[i] - mdi[i]).abs() / s } else { 0.0 };
        }
    }
    let adx = crate::connors::wilder(&dx, n);
    (pdi, mdi, adx)
}
fn xover(a: &[f64], b: &[f64], i: usize) -> bool {
    i > 0 && a[i] > b[i] && a[i - 1] <= b[i - 1]
}
fn xunder(a: &[f64], b: &[f64], i: usize) -> bool {
    i > 0 && a[i] < b[i] && a[i - 1] >= b[i - 1]
}
fn dow(d: &str) -> u32 {
    // 0 = Sunday ... 6 = Saturday (Sakamoto)
    let (y, m, dd): (i32, i32, i32) = (d[0..4].parse().unwrap_or(2000), d[5..7].parse().unwrap_or(1), d[8..10].parse().unwrap_or(1));
    let t = [0, 3, 2, 5, 0, 3, 5, 1, 4, 6, 2, 4];
    let y = if m < 3 { y - 1 } else { y };
    ((y + y / 4 - y / 100 + y / 400 + t[(m - 1) as usize] + dd).rem_euclid(7)) as u32
}

// ------------------------------------------------------------------ order engine

#[derive(Clone, Copy, PartialEq, Debug)]
enum A { Buy, Sell, XL, XS }
#[derive(Clone, Copy, Debug)]
enum T { Mkt, Stp(f64), Lmt(f64) }
#[derive(Clone, Copy, Debug)]
struct O { a: A, t: T, why: &'static str }
fn o(a: A, t: T, why: &'static str) -> O { O { a, t, why } }

pub struct St {
    pub pos: i8,
    pub ep: f64,
    pub ei: usize,
}

struct Eng<'a> {
    x: &'a Ctx<'a>,
    st: St,
    cur: Option<Trade>,
    out: Vec<Trade>,
}
impl<'a> Eng<'a> {
    fn close_pos(&mut self, k: usize, px: f64, sig: usize, why: &str) {
        if let Some(mut t) = self.cur.take() {
            t.exit_i = k;
            t.exit_px = px;
            t.signal_i = sig;
            t.reason = why.into();
            self.out.push(t);
        }
        self.st.pos = 0;
    }
    fn open_pos(&mut self, d: i8, k: usize, px: f64, kind: &'static str, sig: usize) {
        self.cur = Some(Trade { dir: d as f64, legs: vec![Leg { i: k, px, w: 1.0 }], exit_i: k, exit_px: px, reason: String::new(),
            entry_kind: kind, signal_i: sig, open: false, exits: Vec::new() });
        self.st = St { pos: d, ep: px, ei: k };
    }
    fn act(&mut self, a: A, k: usize, px: f64, kind: &'static str, why: &str) {
        match a {
            A::XL if self.st.pos > 0 => self.close_pos(k, px, k - 1, why),
            A::XS if self.st.pos < 0 => self.close_pos(k, px, k - 1, why),
            A::Buy if self.st.pos <= 0 => {
                if self.st.pos < 0 { self.close_pos(k, px, k - 1, why) }
                self.open_pos(1, k, px, kind, k - 1);
            }
            A::Sell if self.st.pos >= 0 => {
                if self.st.pos > 0 { self.close_pos(k, px, k - 1, why) }
                self.open_pos(-1, k, px, kind, k - 1);
            }
            _ => {}
        }
    }
    /// Fill the orders placed on bar k-1 against bar k.
    fn fill(&mut self, k: usize, orders: &[O]) {
        let x = self.x;
        let (op, h, l, c) = (x.o[k], x.h[k], x.l[k], x.c[k]);
        // market orders at the open: exits, then the LAST entry order (a later Buy/Sell in the code replaces an earlier one)
        for od in orders.iter().filter(|q| matches!(q.t, T::Mkt) && matches!(q.a, A::XL | A::XS)) {
            self.act(od.a, k, op, "open", od.why);
        }
        if let Some(od) = orders.iter().filter(|q| matches!(q.t, T::Mkt) && matches!(q.a, A::Buy | A::Sell)).last() {
            self.act(od.a, k, op, "open", od.why);
        }
        let path = if (h - op) < (op - l) { [op, h, l, c] } else { [op, l, h, c] };
        // first time the path reaches px moving up (up=true) or down
        let when = |px: f64, up: bool| -> Option<(f64, f64)> {
            if (up && op >= px) || (!up && op <= px) {
                return Some((0.0, op));
            }
            for s in 0..3 {
                let (a, b) = (path[s], path[s + 1]);
                let hit = if up { a < px && b >= px } else { a > px && b <= px };
                if hit {
                    return Some((s as f64 + (px - a) / (b - a), px));
                }
            }
            None
        };
        let mut hits: Vec<(f64, f64, O)> = orders.iter().filter_map(|od| {
            let (px, up) = match (od.a, od.t) {
                (A::Buy | A::XS, T::Stp(p)) => (p, true),
                (A::Sell | A::XL, T::Stp(p)) => (p, false),
                (A::XL | A::Sell, T::Lmt(p)) => (p, true),
                (A::XS | A::Buy, T::Lmt(p)) => (p, false),
                _ => return None,
            };
            if !px.is_finite() { return None }
            when(px, up).map(|(t, f)| (t, f, *od))
        }).collect();
        hits.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
        let pos0 = self.st.pos;
        for (_, px, od) in hits {
            // exits only for the position held at the open of this bar
            if matches!(od.a, A::XL) && pos0 <= 0 { continue }
            if matches!(od.a, A::XS) && pos0 >= 0 { continue }
            let kind = if matches!(od.t, T::Stp(_)) { "stop" } else { "limit" };
            self.act(od.a, k, px, kind, od.why);
        }
    }
}

/// Runs a system. `bar(i, state, orders)` is called on every close from `from`; it pushes orders for the next bar and
/// may return an exit to fill at this close ("This Bar on Close").
fn el(x: &Ctx, from: usize, to: usize, mut bar: impl FnMut(usize, &St, &mut Vec<O>) -> Option<(A, &'static str)>) -> Vec<Trade> {
    let mut e = Eng { x, st: St { pos: 0, ep: f64::NAN, ei: 0 }, cur: None, out: Vec::new() };
    let mut pending: Vec<O> = Vec::new();
    for i in from..=to {
        if i > from && !pending.is_empty() {
            let p = std::mem::take(&mut pending);
            e.fill(i, &p);
        }
        let mut orders = Vec::new();
        let cx = bar(i, &e.st, &mut orders);
        if let Some((a, why)) = cx {
            if (a == A::XL && e.st.pos > 0) || (a == A::XS && e.st.pos < 0) {
                let px = x.c[i];
                e.close_pos(i, px, i, why);
            }
        }
        if i < to {
            // orders that cannot apply to the position held after this close are dropped (TradeStation ignores them)
            let pos = e.st.pos;
            pending = orders.into_iter().filter(|od| match od.a {
                A::Buy => pos <= 0,
                A::Sell => pos >= 0,
                A::XL => pos > 0,
                A::XS => pos < 0,
            }).collect();
        }
    }
    if let Some(mut t) = e.cur.take() {
        t.exit_i = to;
        t.exit_px = x.c[to];
        t.reason = "open".into();
        t.signal_i = to;
        t.open = true;
        e.out.push(t);
    }
    e.out
}

/// Exit #1 (p.244): exit next bar at market when the close is beyond entry +/- 3 x StdDev(C - C[1], 30).
fn exit1(st: &St, i: usize, c: &[f64], sd30: &[f64], ord: &mut Vec<O>) {
    let v = 3.0 * sd30[i];
    if st.pos != 0 && (c[i] > st.ep + v || c[i] < st.ep - v) {
        ord.push(o(if st.pos > 0 { A::XL } else { A::XS }, T::Mkt, "Exit #1: 3 sigma(30) from entry"));
    }
}

// ------------------------------------------------------------------ catalogue

struct D {
    id: &'static str,
    name: &'static str,
    author: &'static str,
    pages: &'static str,
    rules: &'static [&'static str],
    sp: &'static str, // book SP futures row: trades / win% / bars win / bars loss (1/1/84-12/31/98)
    notes: &'static str,
}

const DEFS: &[D] = &[
    D { id: "lkcpt_cb4020", name: "40/20 Donchian channel breakout (stops)", author: "Control systems (Donchian)", pages: "pp.20-24, code p.225",
        rules: &["E|Buy next bar at Highest(High, 40) stop; Sell next bar at Lowest(Low, 40) stop", "X|ExitLong next bar at Lowest(Low, 20) stop; ExitShort next bar at Highest(High, 20) stop"],
        sp: "92 / 34% / 49 / 18", notes: "Differs from lkqts_cb4020 (Quantitative Trading Strategies): stop orders on highs/lows instead of closes." },
    D { id: "lkcpt_stoch2575", name: "14-day Slow %K 25/75 crossover", author: "Control systems", pages: "pp.40-44, code p.226",
        rules: &["E|IF SlowK(14) > 25 AND SlowK(14)[1] < 25: Buy next bar at market", "E|IF SlowK(14) < 75 AND SlowK(14)[1] > 75: Sell next bar at market", "X|Always in: the opposite entry reverses"],
        sp: "146 / 62% / 18 / 38", notes: "SlowK = 3-bar SMA of FastK (TradeStation). 25/75 thresholds differ from lkqts_stoch14 (20/80)." },
    D { id: "lkcpt_box10", name: "Box size breakout contra method (10 days)", author: "Robert Barnes - Trading in Choppy Markets", pages: "pp.45-49, code pp.226-227",
        rules: &["E|IF Low = Lowest(Low, 10): Buy next bar at market; IF High = Highest(High, 10): Sell next bar at market", "X|ChRange = (Close - Lowest(Low,10))/(Highest(High,10) - Lowest(Low,10)); ChRange < 0.6: ExitShort this bar on close; ChRange > 0.4: ExitLong this bar on close"],
        sp: "430 / 68% / 3 / 9", notes: "Prose says shorts enter 'on tomorrow's close'; the code (used here) enters at the next open." },
    D { id: "lkcpt_box40", name: "Box size breakout contra method (40 days)", author: "Robert Barnes - Trading in Choppy Markets", pages: "pp.50-54, code pp.226-227",
        rules: &["E|Box contra method with Channel = 40", "X|Same 60% range exits on the close"], sp: "112 / 68% / 9 / 37", notes: "" },
    D { id: "lkcpt_drv", name: "DRV profit taker", author: "Robert Barnes - Trading in Choppy Markets", pages: "pp.55-59, code p.227",
        rules: &["F|DRV = XAverage(Close - Close[1], 10)/XAverage(|Close - Close[1]|, 10)", "E|DRV > -0.4 and DRV[1] < -0.4: Buy next bar at market", "E|DRV < 0.4 and DRV[1] > 0.4: Sell next bar at market", "X|Always in: opposite entry reverses"],
        sp: "162 / 67% / 17 / 37", notes: "" },
    D { id: "lkcpt_osc", name: "Oscillator method (10-40)", author: "Robert Barnes - Trading in Choppy Markets", pages: "pp.60-64, code p.227",
        rules: &["F|Osc = Average(Close,10) - Average(Close,40)", "E|Osc > Osc[1]: Buy next bar at market; Osc < Osc[1]: Sell next bar at market", "X|Always in"],
        sp: "555 / 38% / 10 / 5", notes: "" },
    D { id: "lkcpt_osc_adx", name: "Oscillator method with ADX filter", author: "Robert Barnes - Trading in Choppy Markets", pages: "pp.65-69, code pp.227-228",
        rules: &["E|Osc turns up and ADX(14) < 30: Buy next bar at market; Osc turns down and ADX(14) < 30: Sell", "X|Osc > Osc[1]: ExitShort next bar at market; Osc < Osc[1]: ExitLong next bar at market"],
        sp: "444 / 40% / 11 / 5", notes: "" },
    D { id: "lkcpt_spop", name: "Stochastic pop", author: "Jake Bernstein - Short-Term Futures Trading", pages: "pp.70-74, code p.228",
        rules: &["E|FastK(14) > 75 and FastK[1] < 75 and FastK > SlowK: Buy next bar at market", "E|FastK(14) < 25 and FastK[1] > 25 and FastK < SlowK: Sell next bar at market", "X|FastK < FastD: ExitLong next bar at market; FastK > FastD: ExitShort next bar at market"],
        sp: "495 / 38% / 3 / 2", notes: "Differs from jb624_pop (5-bar slowed %K, close fills)." },
    D { id: "lkcpt_spop_rev", name: "Stochastic pop (revised by Kestner)", author: "Jake Bernstein / Lars Kestner", pages: "pp.75-79, code pp.228-229",
        rules: &["E|SlowK(14) > 75 and SlowK[1] < 75 and SlowK > SlowD: Buy next bar at market", "E|SlowK < 25 and SlowK[1] > 25 and SlowK < SlowD: Sell next bar at market", "X|SlowK < SlowD: ExitLong next bar at market; SlowK > SlowD: ExitShort next bar at market"],
        sp: "281 / 37% / 7 / 3", notes: "Prose adds 'Slow %K must have been below 75 since last long entry'; not in the code (code used)." },
    D { id: "lkcpt_ctod", name: "Critical time of day (week), revised by Kestner", author: "Jake Bernstein / Lars Kestner", pages: "pp.80-84, code p.229",
        rules: &["F|On a Monday that follows a Friday bar: BuyPoint = Highest(High,2), SellPoint = Lowest(Low,2) (Friday + Monday); on a Monday after no Friday bar: no points", "E|Monday-Thursday closes: Buy next bar at BuyPoint stop; Sell next bar at SellPoint stop", "X|Friday: ExitLong / ExitShort this bar on close"],
        sp: "949 / 47% / 2 / 2", notes: "Prose: 'If a holiday falls on either Friday or Monday, no signals are generated that week'; the code only blocks a Friday holiday (after a Monday holiday it keeps the previous points) - code used." },
    D { id: "lkcpt_dema", name: "Dual exponential moving average (8/18/9)", author: "Jake Bernstein - Short-Term Futures Trading", pages: "pp.85-89, code p.230",
        rules: &["F|Difference = XAverage(Close,8) - XAverage(Close,18)", "E|Difference > XAverage(Difference, 9): Buy next bar at market; < : Sell next bar at market", "X|Always in"],
        sp: "418 / 31% / 16 / 6", notes: "" },
    D { id: "lkcpt_pctf", name: "%F with 3-day moving average", author: "Tushar Chande & Stanley Kroll - The New Technical Trader", pages: "pp.90-94, code p.230",
        rules: &["F|%F = 100 (Close - LinearRegValue(Close, 5, 0))/Close", "E|%F > Average(%F, 3): Buy next bar at market; < : Sell next bar at market", "X|Always in"],
        sp: "1753 / 48% / 2 / 2", notes: "" },
    D { id: "lkcpt_vidya", name: "0.2 / 0.1 VIDYA crossover", author: "Tushar Chande & Stanley Kroll - The New Technical Trader", pages: "pp.100-104, code pp.230-231",
        rules: &["F|K = StdDev(Close,20)/Average(StdDev(Close,20),20); IF 0.2 K > 1 THEN K = 1/0.1 (as coded)", "F|VIDYA(a) = a K Close + (1 - a K) VIDYA[1], seeded on bar 1", "E|VIDYA(0.2) > VIDYA(0.1): Buy next bar at market; < : Sell", "X|Always in"],
        sp: "196 / 25% / 44 / 11", notes: "Seeded with the first close of the data (XAverage on bar 1)." },
    D { id: "lkcpt_qstick8", name: "QStick 8 with 8-day average", author: "Tushar Chande & Stanley Kroll - The New Technical Trader", pages: "pp.105-109, code p.231",
        rules: &["F|QStick = Average(Close - Open, 8); QAvg = Average(QStick, 8)", "E|QStick > QAvg: Buy next bar at market; < : Sell", "X|Always in"], sp: "729 / 37% / 7 / 4", notes: "" },
    D { id: "lkcpt_qstick20", name: "QStick 20 with 20-day average", author: "Tushar Chande & Stanley Kroll - The New Technical Trader", pages: "pp.110-114, code p.231",
        rules: &["F|QStick = Average(Close - Open, 20); QAvg = Average(QStick, 20)", "E|QStick > QAvg: Buy; < : Sell (next bar at market)", "X|Always in"], sp: "457 / 34% / 14 / 5", notes: "" },
    D { id: "lkcpt_cmo", name: "Chande momentum oscillator with 10-day average", author: "Tushar Chande & Stanley Kroll - The New Technical Trader", pages: "pp.115-119, code pp.231-232",
        rules: &["F|CMO = 100 (Up - Down)/(Up + Down) over the last 10 close-to-close changes", "E|CMO > Average(CMO,10): Buy next bar at market; < : Sell", "X|Always in"], sp: "672 / 37% / 8 / 4", notes: "" },
    D { id: "lkcpt_tdline1", name: "TD Line penetration, strength 1", author: "Thomas DeMark - The New Science of Technical Analysis", pages: "pp.120-124, code pp.232-234",
        rules: &["F|Last / second swing highs and lows of strength TDStr within 50 bars (SwingHigh/SwingLow); TD line = TLValue through them at the current bar", "E|Buy next bar at the TD line of highs (stop) when it slopes down, today's high is below it and a qualifier holds; Sell next bar at the TD line of lows when it slopes up and today's low is above it", "X|Target (limit) and stop at TD line +/- the extreme move beyond the line (Price projector), set when the setup appears"],
        sp: "201 / 50% / 13 / 8", notes: "Code as printed: SecondHighBar = SwingHigh(2, ...) (a price, then capped at 50) and TDQual2 = TRUE make qualifiers always true; both kept literally. Swing: strictly higher/lower than TDStr bars on each side." },
    D { id: "lkcpt_tdline3", name: "TD Line penetration, strength 3", author: "Thomas DeMark - The New Science of Technical Analysis", pages: "pp.125-129, code pp.232-234",
        rules: &["F|As strength 1 with TDStr = 3", "E|TD line stop entries", "X|Price-projector target and stop"], sp: "102 / 53% / 18 / 13", notes: "" },
    D { id: "lkcpt_rsimild", name: "RSI mild overbought/oversold with target", author: "Thomas DeMark - The New Science of Technical Analysis", pages: "pp.130-134, code pp.234-235",
        rules: &["E|RSI(14) crosses above 40 and Highest(RSI, 6)[1] > 40: Buy next bar at market", "E|RSI(14) crosses below 60 and Lowest(RSI, 6)[1] < 60: Sell next bar at market", "X|Exit #1: close beyond entry +/- 3 x StdDev(C - C[1], 30) -> exit next bar at market"],
        sp: "163 / 61% / 13 / 11", notes: "" },
    D { id: "lkcpt_reimild", name: "REI mild overbought/oversold, MA filter, with target", author: "Thomas DeMark - The New Science of Technical Analysis", pages: "pp.135-139, code pp.235-236",
        rules: &["F|REI(8) with DeMark's high/low qualifiers as coded (REIHigh = High - High[2], REILow = Low - Low[2], sums over 8)", "E|REI crosses above -60, Highest(REI,6)[1] > -60 and Close > Average(Close,20): Buy next bar at market", "E|REI crosses below +60, Lowest(REI,6)[1] < 60 and Close < Average(Close,20): Sell", "X|Exit #1"],
        sp: "41 / 41% / 17 / 14", notes: "LowQualifier compares Low with Close[5]/Close[6] as printed." },
    D { id: "lkcpt_demarker", name: "DeMarker mild overbought/oversold, MA filter", author: "Thomas DeMark - The New Science of Technical Analysis", pages: "pp.140-144, code p.236",
        rules: &["F|DeMarkHigh = High - High[1] when High > High[1] (else keeps its last value, as coded); DeMarkLow likewise; DeMarker = 100 Sum13(DeMarkHigh)/(Sum13(DeMarkHigh) + Sum13(DeMarkLow))", "E|DeMarker crosses above 30, Highest(DeMarker,6)[1] > 30, Close > Average(Close,20): Buy next bar at market", "E|DeMarker crosses below 70, Lowest(DeMarker,6)[1] < 70, Close < Average(Close,30): Sell", "X|Exit #1"],
        sp: "18 / 72% / 13 / 10", notes: "" },
    D { id: "lkcpt_obos", name: "Overbought/oversold moving average system", author: "Thomas DeMark - The New Science of Technical Analysis", pages: "pp.145-149, code p.237",
        rules: &["F|HighLast: in the last 4 bars a Low equalled the 13-bar highest Low; LowLast: a High equalled the 13-bar lowest High", "E|Close > Average(High,3) and LowLast: Buy next bar at market; Close < Average(Low,3) and HighLast: Sell", "X|Exit #1"],
        sp: "211 / 73% / 7 / 26", notes: "" },
    D { id: "lkcpt_kurt03", name: "Kurtosis (alpha 0.03)", author: "Ulf Jensen - How I Tripled My Money in the Futures Market", pages: "pp.150-154, code pp.237-238",
        rules: &["F|Acceleration = Momentum(C,3) - Momentum(C,3)[1]; Kurtosis = 0.03 Acceleration + 0.97 Kurtosis[1]", "E|Kurtosis > 0: Buy next bar at market; < 0: Sell", "X|Always in"], sp: "1048 / 37% / 6 / 2", notes: "" },
    D { id: "lkcpt_kurt10", name: "Kurtosis (alpha 0.10)", author: "Ulf Jensen - How I Tripled My Money in the Futures Market", pages: "pp.155-159, code pp.237-238",
        rules: &["F|Kurtosis = 0.10 Acceleration + 0.90 Kurtosis[1]", "E|Kurtosis > 0: Buy; < 0: Sell (next bar at market)", "X|Always in"], sp: "1089 / 39% / 5 / 2", notes: "" },
    D { id: "lkcpt_fsrs", name: "FSRS", author: "Ulf Jensen - How I Tripled My Money in the Futures Market", pages: "pp.160-164, code p.238",
        rules: &["F|FSRS = 10000 WAverage(Kurtosis(0.03), 6) + RSI(Close, 9)", "E|FSRS > WAverage(FSRS, 6): Buy next bar at market; < : Sell", "X|Always in"], sp: "755 / 44% / 6 / 4", notes: "" },
    D { id: "lkcpt_123", name: "1-2-3 reversal system with profit target", author: "Ulf Jensen - How I Tripled My Money in the Futures Market", pages: "pp.165-169, code pp.238-239",
        rules: &["E|C > C[1], C[1] < C[2], C[2] < C[3], SlowK(9) > SlowK(9)[1], SlowD(9) < 50: Buy next bar at market", "E|Mirror: Sell next bar at market", "X|Exit #1"], sp: "285 / 56% / 10 / 10", notes: "" },
    D { id: "lkcpt_ons", name: "One Night Stand", author: "Joe Krutsinger - The Trading Systems Toolkit", pages: "pp.170-174, code p.239",
        rules: &["E|Thursday and Average(C,10) > Average(C,40): Buy next bar at Highest(High,4) stop", "E|Thursday and not: Sell next bar at Lowest(Low,8) stop", "X|ExitLong / ExitShort next bar at market (Monday's open)"],
        sp: "227 / 50% / 1 / 1", notes: "Differs from jkbwts_one_night_stand ($1,000 stop, needs JY futures)." },
    D { id: "lkcpt_tts3", name: "Joe's Texas Two Step (3 days)", author: "Joe Krutsinger - The Trading Systems Toolkit", pages: "pp.175-179, code p.239",
        rules: &["E|RSI(14) > Average(RSI(14), 3): Buy next bar at market; < : Sell", "X|Always in"], sp: "1407 / 41% / 4 / 2", notes: "" },
    D { id: "lkcpt_tts10", name: "Joe's Texas Two Step (10 days)", author: "Joe Krutsinger - The Trading Systems Toolkit", pages: "pp.180-184, code p.239",
        rules: &["E|RSI(14) > Average(RSI(14), 10): Buy next bar at market; < : Sell", "X|Always in"], sp: "716 / 31% / 10 / 3", notes: "" },
    D { id: "lkcpt_qp", name: "Joe's Quarter Pounder", author: "Joe Krutsinger - The Trading Systems Toolkit", pages: "pp.185-189, code p.240",
        rules: &["E|C[3] > O[3], C[2] > O[2], C[2] > C[3]: Buy next bar at High[2] stop", "E|C[3] < O[3], C[2] < O[2], C[2] < C[3]: Sell next bar at Low[2] stop", "X|Exit #1"], sp: "431 / 47% / 7 / 7", notes: "" },
    D { id: "lkcpt_gap", name: "Joe's Gap", author: "Joe Krutsinger - The Trading Systems Toolkit", pages: "pp.190-194, code p.240",
        rules: &["E|Low > High[1]: Buy next bar at market; High < Low[1]: Sell next bar at market", "X|BarsSinceEntry > 2: exit this bar on close"], sp: "228 / 46% / 3 / 3", notes: "" },
    D { id: "lkcpt_fib", name: "Joe's Fib Catcher", author: "Joe Krutsinger - The Trading Systems Toolkit", pages: "pp.195-199, code p.241",
        rules: &["E|Low = Lowest(Low,14): Buy next bar at Highest(High,4) stop", "E|High = Highest(High,14): Sell next bar at Lowest(Low,4) stop", "X|BarsSinceEntry > 2: exit this bar on close"], sp: "51 / 57% / 3 / 3", notes: "" },
    D { id: "lkcpt_jesse", name: "Joe's Jesse Livermore", author: "Joe Krutsinger - The Trading Systems Toolkit", pages: "pp.200-204, code p.241",
        rules: &["E|Outside day: Buy next bar at Highest(High,2) stop and Sell next bar at Lowest(Low,2) stop", "X|ExitLong next bar at Lowest(Low,8) stop; ExitShort at Highest(High,8) stop", "X|BarsSinceEntry > 11: exit this bar on close"], sp: "279 / 34% / 11 / 5", notes: "" },
    D { id: "lkcpt_wvol", name: "Volatility system (Kestner's coding)", author: "Welles Wilder - New Concepts in Technical Trading Systems", pages: "pp.205-209, code p.242",
        rules: &["F|VolIndex = (6 VolIndex[1] + TrueRange)/7 (seeded at 0)", "E|First bar of the test: Close > Average(Close,10): Buy next bar at market, else Sell", "X|Long and Close < HighestClose - 3 VolIndex: Sell next bar at market (LowestClose = Close); short and Close > LowestClose + 3 VolIndex: Buy (HighestClose = Close)"],
        sp: "114 / 35% / 58 / 19", notes: "Differs from wwnc_vol (Wilder's close fills and SIC). HighestClose/LowestClose start at 0 and update only while in that position, as coded." },
    D { id: "lkcpt_wdmi", name: "Directional movement (Kestner's coding)", author: "Welles Wilder - New Concepts in Technical Trading Systems", pages: "pp.210-214, code pp.242-243",
        rules: &["E|DMIPlus(14) crosses above DMIMinus(14) and ADXR(15) > 25: Buy next bar at market; crosses below and ADXR(15) > 25: Sell", "X|DMIPlus < DMIMinus: ExitLong next bar at market; DMIPlus > DMIMinus: ExitShort next bar at market"],
        sp: "104 / 22% / 16 / 5", notes: "ADXR(15) = (ADX(15) + ADX(15) 15 bars ago)/2. Differs from wwnc_dmi (extreme-point rule, close fills)." },
];

pub fn catalogue() -> Vec<StrategyDef> {
    let mut v: Vec<StrategyDef> = DEFS.iter().map(|d| {
        let notes: &'static str = Box::leak(format!("{} Book: SP futures (S&P 500), 1/1/1984-12/31/1998, trades / win% / avg bars win / avg bars loss = {}. Run here on one index (instrument swap; India: NIFTY 50 / NIFTY BANK). {}", d.pages, d.sp, d.notes).into_boxed_str());
        let mut parts = d.sp.split(" / ");
        let tr = parts.next().unwrap_or("-");
        let win = parts.next().unwrap_or("-");
        let hold: &'static str = Box::leak(format!("{} / {} bars (win / loss)", parts.next().unwrap_or("-"), parts.next().unwrap_or("-")).into_boxed_str());
        StrategyDef { id: d.id, name: d.name, group: d.author, session: 0, dir: "Long/Short", status: Status::Live, needs_volume: false,
            needs_vix: false, needs_series: &[], book: BOOK, rules: d.rules, notes,
            claim: crate::connors::claim(tr, win, "-", hold, "SP futures 1984-1998") }
    }).collect();
    v.push(StrategyDef { id: "lkcpt_pctf20", name: "%F with moving average crossover (20/10)", group: "Tushar Chande & Stanley Kroll - The New Technical Trader",
        session: 0, dir: "Long/Short", status: Status::NotTestable("the parameters are contradictory: contents say '20 day %F with 10 day MA', the results page says '10 days used for forecast, 20 day average of %F', and the printed code hard-codes a 5-bar regression"),
        needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK, rules: &["E|%F > its N-day average: buy; below: sell (pp.95-99)"], notes: "SP row: 815 trades, 35% win.", claim: None });
    v
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    let d = DEFS.iter().find(|d| d.id == id).map(|d| (d.author, d.name)).or_else(|| (id == "lkcpt_pctf20").then_some(("Tushar Chande & Stanley Kroll - The New Technical Trader", "%F with moving average crossover (20/10)")))?;
    let fam = match d.0 {
        a if a.starts_with("Control") => ("lkcpt_control", "COPTS control systems"),
        a if a.starts_with("Robert Barnes") => ("lkcpt_barnes", "Barnes systems (Kestner coding)"),
        a if a.starts_with("Jake Bernstein") => ("lkcpt_bernstein", "Bernstein systems (Kestner coding)"),
        a if a.starts_with("Tushar") => ("lkcpt_chande", "Chande & Kroll systems (Kestner coding)"),
        a if a.starts_with("Thomas DeMark") => ("lkcpt_demark", "DeMark systems (Kestner coding)"),
        a if a.starts_with("Ulf Jensen") => ("lkcpt_jensen", "Jensen systems (Kestner coding)"),
        a if a.starts_with("Joe Krutsinger") => ("lkcpt_krut", "Krutsinger systems (Kestner coding)"),
        _ => ("lkcpt_wilder", "Wilder systems (Kestner coding)"),
    };
    Some((fam.0, fam.1, d.1))
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    Some(match id {
        "lkcpt_cb4020" => vec![p("entry", "Entry channel (highs/lows)", 40., 10., 100., 5.), p("exit", "Exit channel", 20., 5., 50., 5.)],
        "lkcpt_stoch2575" => vec![p("n", "Stochastic length", 14., 5., 30., 1.), p("thres", "Threshold (buy above, sell below 100 - it)", 25., 10., 40., 5.)],
        "lkcpt_box10" => vec![p("ch", "Box (channel) days", 10., 5., 60., 5.)],
        "lkcpt_box40" => vec![p("ch", "Box (channel) days", 40., 5., 60., 5.)],
        "lkcpt_drv" => vec![p("n", "DRV smoothing", 10., 3., 30., 1.), p("lvl", "Cross level", 0.4, 0.1, 0.8, 0.1)],
        "lkcpt_osc" => vec![p("s", "Short MA", 10., 3., 30., 1.), p("l", "Long MA", 40., 20., 100., 5.)],
        "lkcpt_osc_adx" => vec![p("s", "Short MA", 10., 3., 30., 1.), p("l", "Long MA", 40., 20., 100., 5.), p("adx", "ADX below", 30., 15., 50., 5.)],
        "lkcpt_spop" | "lkcpt_spop_rev" => vec![p("n", "Stochastic length", 14., 5., 30., 1.), p("thres", "Pop threshold (75/25)", 25., 10., 40., 5.)],
        "lkcpt_ctod" => vec![],
        "lkcpt_dema" => vec![p("s", "Short EMA", 8., 3., 20., 1.), p("l", "Long EMA", 18., 10., 50., 1.), p("d", "Trigger EMA", 9., 3., 20., 1.)],
        "lkcpt_pctf" => vec![p("reg", "Regression bars", 5., 3., 30., 1.), p("ma", "Trigger average", 3., 2., 20., 1.)],
        "lkcpt_vidya" => vec![p("a_s", "Short alpha", 0.2, 0.05, 0.5, 0.05), p("a_l", "Long alpha", 0.1, 0.02, 0.3, 0.02)],
        "lkcpt_qstick8" => vec![p("q", "QStick length", 8., 3., 30., 1.), p("a", "Average length", 8., 3., 30., 1.)],
        "lkcpt_qstick20" => vec![p("q", "QStick length", 20., 3., 30., 1.), p("a", "Average length", 20., 3., 30., 1.)],
        "lkcpt_cmo" => vec![p("n", "CMO length", 10., 5., 30., 1.), p("a", "Average length", 10., 3., 30., 1.)],
        "lkcpt_tdline1" => vec![p("str", "TD line strength", 1., 1., 5., 1.)],
        "lkcpt_tdline3" => vec![p("str", "TD line strength", 3., 1., 5., 1.)],
        "lkcpt_rsimild" => vec![p("n", "RSI length", 14., 5., 30., 1.), p("days", "Mild-extreme days", 5., 2., 10., 1.)],
        "lkcpt_reimild" | "lkcpt_demarker" => vec![p("days", "Mild-extreme days", 5., 2., 10., 1.)],
        "lkcpt_obos" => vec![],
        "lkcpt_kurt03" => vec![p("alpha", "Kurtosis alpha", 0.03, 0.01, 0.3, 0.01), p("mo", "Momentum length", 3., 1., 10., 1.)],
        "lkcpt_kurt10" => vec![p("alpha", "Kurtosis alpha", 0.10, 0.01, 0.3, 0.01), p("mo", "Momentum length", 3., 1., 10., 1.)],
        "lkcpt_fsrs" | "lkcpt_123" | "lkcpt_ons" | "lkcpt_qp" | "lkcpt_gap" => vec![],
        "lkcpt_tts3" => vec![p("rsi", "RSI length", 14., 5., 30., 1.), p("avg", "RSI average", 3., 2., 20., 1.)],
        "lkcpt_tts10" => vec![p("rsi", "RSI length", 14., 5., 30., 1.), p("avg", "RSI average", 10., 2., 20., 1.)],
        "lkcpt_fib" => vec![p("set", "Extreme days", 14., 5., 30., 1.), p("opp", "Opposite trigger days", 4., 2., 10., 1.)],
        "lkcpt_jesse" => vec![],
        "lkcpt_wvol" => vec![p("mult", "ATR multiple", 3., 1., 5., 0.25)],
        "lkcpt_wdmi" => vec![p("n", "DMI length", 14., 5., 30., 1.), p("adxr", "ADXR above", 25., 10., 40., 5.)],
        _ => return None,
    })
}

// ------------------------------------------------------------------ TD line helpers (TradeStation SwingHigh / SwingLow)

/// Bars-ago of the `inst`-th most recent swing (high if hi) of strength `s` among the last `len` bars ending at i:
/// a pivot k bars ago with s bars on each side strictly lower (higher) and s <= k. Returns (price, bars ago).
fn swing(v: &[f64], i: usize, inst: usize, s: usize, len: usize, hi: bool) -> Option<(f64, usize)> {
    let mut found = 0;
    for k in s..len {
        if k + s > i {
            break;
        }
        let j = i - k;
        let p = v[j];
        let ok = (1..=s).all(|q| if hi { p > v[j - q] && p > v[j + q] } else { p < v[j - q] && p < v[j + q] });
        if ok {
            found += 1;
            if found == inst {
                return Some((p, k));
            }
        }
    }
    None
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !id.starts_with("lkcpt_") || id == "lkcpt_pctf20" {
        return None;
    }
    let defs = params(id)?;
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| defs.iter().find(|d| d.key == k).map_or(f64::NAN, |d| d.def));
    let u = |k: &str| g(k).round().max(1.0) as usize;
    let (c, h, l, op) = (&x.c, &x.h, &x.l, &x.o);
    let n = c.len();
    if from >= to || to >= n {
        return Some(vec![]);
    }
    let sd30 = stdev(&chg(c), 30);
    let mkt = |a, w| o(a, T::Mkt, w);
    // always-in systems driven by a level comparison
    let sar_lvl = |up: Vec<bool>, dn: Vec<bool>, lw: &'static str, sw: &'static str| -> Vec<Trade> {
        el(x, from, to, |i, _, ord| {
            if up[i] { ord.push(mkt(A::Buy, lw)) }
            if dn[i] { ord.push(mkt(A::Sell, sw)) }
            None
        })
    };
    let t = match id {
        "lkcpt_cb4020" => {
            let (en, ex) = (u("entry"), u("exit"));
            el(x, from, to, |i, _, ord| {
                ord.push(o(A::Buy, T::Stp(hh(h, i, en)), "Highest(High,40) stop"));
                ord.push(o(A::Sell, T::Stp(ll(l, i, en)), "Lowest(Low,40) stop"));
                ord.push(o(A::XL, T::Stp(ll(l, i, ex)), "Lowest(Low,20) stop"));
                ord.push(o(A::XS, T::Stp(hh(h, i, ex)), "Highest(High,20) stop"));
                None
            })
        }
        "lkcpt_stoch2575" => {
            let (_, sk, _) = stoch(x, u("n"));
            let th = g("thres");
            let up: Vec<bool> = (0..n).map(|i| i > 0 && sk[i] > th && sk[i - 1] < th).collect();
            let dn: Vec<bool> = (0..n).map(|i| i > 0 && sk[i] < 100.0 - th && sk[i - 1] > 100.0 - th).collect();
            sar_lvl(up, dn, "SlowK crossed above 25", "SlowK crossed below 75")
        }
        "lkcpt_box10" | "lkcpt_box40" => {
            let ch = u("ch");
            el(x, from, to, |i, st, ord| {
                if l[i] == ll(l, i, ch) { ord.push(mkt(A::Buy, "10-day low (contra buy)")) }
                if h[i] == hh(h, i, ch) { ord.push(mkt(A::Sell, "10-day high (contra sell)")) }
                let (hi, lo) = (hh(h, i, ch), ll(l, i, ch));
                let r = (c[i] - lo) / (hi - lo);
                if st.pos < 0 && r < 0.6 { return Some((A::XS, "close in the lower 60% of the box")) }
                if st.pos > 0 && r > 0.4 { return Some((A::XL, "close in the upper 60% of the box")) }
                None
            })
        }
        "lkcpt_drv" => {
            let d = chg(c);
            let ad: Vec<f64> = d.iter().map(|v| v.abs()).collect();
            let (a, b) = (xavg(&d, g("n")), xavg(&ad, g("n")));
            let r: Vec<f64> = (0..n).map(|i| a[i] / b[i]).collect();
            let lv = g("lvl");
            let up: Vec<bool> = (0..n).map(|i| i > 0 && r[i] > -lv && r[i - 1] < -lv).collect();
            let dn: Vec<bool> = (0..n).map(|i| i > 0 && r[i] < lv && r[i - 1] > lv).collect();
            sar_lvl(up, dn, "DRV crossed above -0.4", "DRV crossed below 0.4")
        }
        "lkcpt_osc" | "lkcpt_osc_adx" => {
            let (a, b) = (sma(c, u("s")), sma(c, u("l")));
            let os: Vec<f64> = (0..n).map(|i| a[i] - b[i]).collect();
            // a change smaller than 1e-9 x price is floating-point noise from equal decimal prices: treated as unchanged
            let eps = |i: usize| 1e-9 * c[i];
            let up: Vec<bool> = (0..n).map(|i| i > 0 && os[i] > os[i - 1] + eps(i)).collect();
            let dn: Vec<bool> = (0..n).map(|i| i > 0 && os[i] < os[i - 1] - eps(i)).collect();
            if id == "lkcpt_osc" {
                sar_lvl(up.clone(), dn.clone(), "oscillator turned up", "oscillator turned down")
            } else {
                let (_, _, adx) = dmi3(x, 14);
                let lim = g("adx");
                el(x, from, to, |i, _, ord| {
                    if i == 0 { return None }
                    if dn[i] && adx[i] < lim { ord.push(mkt(A::Sell, "oscillator down, ADX < 30")) }
                    if up[i] && adx[i] < lim { ord.push(mkt(A::Buy, "oscillator up, ADX < 30")) }
                    if up[i] { ord.push(mkt(A::XS, "oscillator turned up")) }
                    if dn[i] { ord.push(mkt(A::XL, "oscillator turned down")) }
                    None
                })
            }
        }
        "lkcpt_spop" | "lkcpt_spop_rev" => {
            let (fk, sk, sdd) = stoch(x, u("n"));
            let th = g("thres");
            let (a, b) = if id == "lkcpt_spop" { (fk, sk) } else { (sk, sdd) };
            el(x, from, to, |i, _, ord| {
                if i == 0 { return None }
                if a[i] > 100.0 - th && a[i - 1] < 100.0 - th && a[i] > b[i] { ord.push(mkt(A::Buy, "%K popped above 75")) }
                if a[i] < th && a[i - 1] > th && a[i] < b[i] { ord.push(mkt(A::Sell, "%K popped below 25")) }
                if a[i] < b[i] { ord.push(mkt(A::XL, "%K below %D")) }
                if a[i] > b[i] { ord.push(mkt(A::XS, "%K above %D")) }
                None
            })
        }
        "lkcpt_ctod" => {
            let wd: Vec<u32> = x.r.iter().map(|r| dow(&r.date)).collect();
            let (mut bp, mut sp) = (99999.0f64, -99999.0f64);
            // state from the start of the data (VAR values persist bar to bar)
            let mut pts = vec![(99999.0f64, -99999.0f64); n];
            for i in 0..n {
                let yd = if i > 0 { wd[i - 1] } else { 0 };
                if wd[i] == 1 && yd == 5 { bp = hh(h, i, 2); sp = ll(l, i, 2); }
                if wd[i] == 1 && yd != 5 { bp = 99999.0; sp = -99999.0; }
                pts[i] = (bp, sp);
            }
            el(x, from, to, |i, st, ord| {
                if wd[i] != 5 {
                    ord.push(o(A::Buy, T::Stp(pts[i].0), "above the Friday-Monday high"));
                    ord.push(o(A::Sell, T::Stp(pts[i].1), "below the Friday-Monday low"));
                    None
                } else if st.pos != 0 {
                    Some((if st.pos > 0 { A::XL } else { A::XS }, "Friday close"))
                } else {
                    None
                }
            })
        }
        "lkcpt_dema" => {
            let (a, b) = (xavg(c, g("s")), xavg(c, g("l")));
            let d: Vec<f64> = (0..n).map(|i| a[i] - b[i]).collect();
            let t = xavg(&d, g("d"));
            sar_lvl((0..n).map(|i| d[i] > t[i]).collect(), (0..n).map(|i| d[i] < t[i]).collect(), "EMA difference above trigger", "EMA difference below trigger")
        }
        "lkcpt_pctf" => {
            let rb = u("reg");
            let f: Vec<f64> = (0..n).map(|i| {
                if i + 1 < rb { return f64::NAN }
                // least-squares line through the last rb closes, value at the current bar
                let m = rb as f64;
                let xs: Vec<f64> = (0..rb).map(|k| k as f64).collect();
                let ys: Vec<f64> = (0..rb).map(|k| c[i + 1 - rb + k]).collect();
                let (sx, sy) = (xs.iter().sum::<f64>(), ys.iter().sum::<f64>());
                let sxy: f64 = xs.iter().zip(&ys).map(|(a, b)| a * b).sum();
                let sxx: f64 = xs.iter().map(|a| a * a).sum();
                let slope = (m * sxy - sx * sy) / (m * sxx - sx * sx);
                let icpt = (sy - slope * sx) / m;
                100.0 * (c[i] - (icpt + slope * (m - 1.0))) / c[i]
            }).collect();
            let a = sma(&f, u("ma"));
            sar_lvl((0..n).map(|i| f[i] > a[i]).collect(), (0..n).map(|i| f[i] < a[i]).collect(), "%F above its average", "%F below its average")
        }
        "lkcpt_vidya" => {
            let (asf, alf) = (g("a_s"), g("a_l"));
            let sdc = stdev(c, 20);
            let asd = sma(&sdc, 20);
            let (mut vs, mut vl) = (vec![f64::NAN; n], vec![f64::NAN; n]);
            for i in 0..n {
                let mut k = sdc[i] / asd[i];
                if asf * k > 1.0 { k = 1.0 / alf }
                if i == 0 { vs[0] = c[0]; vl[0] = c[0]; continue }
                if !k.is_finite() { vs[i] = vs[i - 1]; vl[i] = vl[i - 1]; continue }
                vs[i] = asf * k * c[i] + (1.0 - asf * k) * vs[i - 1];
                vl[i] = alf * k * c[i] + (1.0 - alf * k) * vl[i - 1];
            }
            sar_lvl((0..n).map(|i| vs[i] > vl[i]).collect(), (0..n).map(|i| vs[i] < vl[i]).collect(), "fast VIDYA above slow", "fast VIDYA below slow")
        }
        "lkcpt_qstick8" | "lkcpt_qstick20" => {
            let co: Vec<f64> = (0..n).map(|i| c[i] - op[i]).collect();
            let q = sma(&co, u("q"));
            let qa = sma(&q, u("a"));
            sar_lvl((0..n).map(|i| q[i] > qa[i]).collect(), (0..n).map(|i| q[i] < qa[i]).collect(), "QStick above its average", "QStick below its average")
        }
        "lkcpt_cmo" => {
            let m = u("n");
            let cmo: Vec<f64> = (0..n).map(|i| {
                if i < m { return f64::NAN }
                let (mut up, mut dn) = (0.0, 0.0);
                for k in 0..m {
                    let d = c[i - k] - c[i - k - 1];
                    if d > 0.0 { up += d } else { dn += d }
                }
                if up - dn == 0.0 { f64::NAN } else { 100.0 * (up + dn) / (up - dn) }
            }).collect();
            let a = sma(&cmo, u("a"));
            sar_lvl((0..n).map(|i| cmo[i] > a[i]).collect(), (0..n).map(|i| cmo[i] < a[i]).collect(), "CMO above its average", "CMO below its average")
        }
        "lkcpt_tdline1" | "lkcpt_tdline3" => {
            let s = u("str");
            let (mut btg, mut bls, mut stg, mut sls) = (0.0f64, 0.0f64, 0.0f64, 0.0f64);
            let tl = |p1: f64, b1: f64, p2: f64, b2: f64| if b2 == b1 { f64::NAN } else { p2 + (0.0 - b2) * (p2 - p1) / (b2 - b1) };
            el(x, from, to, |i, st, ord| {
                if i < 60 { return None }
                let lo1 = swing(l, i, 1, s, 50, false);
                let lo2 = swing(l, i, 2, s, 50, false);
                let hi1 = swing(h, i, 1, s, 50, true);
                let hi2 = swing(h, i, 2, s, 50, true);
                // TradeStation returns -1 when no swing is found
                let (last_low, mut llb) = lo1.map_or((-1.0, -1.0), |(p, b)| (p, b as f64));
                let (second_low, mut slb) = lo2.map_or((-1.0, -1.0), |(p, b)| (p, b as f64));
                let (last_high, mut lhb) = hi1.map_or((-1.0, -1.0), |(p, b)| (p, b as f64));
                let second_high = hi2.map_or(-1.0, |(p, _)| p);
                let mut shb = second_high; // printed code: SecondHighBar = SwingHigh(2, ...) - a price
                if llb > 50.0 { llb = 50.0 }
                if slb > 50.0 { slb = 50.0 }
                if lhb > 50.0 { lhb = 50.0 }
                if shb > 50.0 { shb = 50.0 }
                let low_line = tl(second_low, slb, last_low, llb);
                let high_line = tl(second_high, shb, last_high, lhb);
                let buy = high_line.is_finite() && high_line != 0.0 && last_high < second_high && h[i] < high_line;
                let sell = low_line.is_finite() && low_line != 0.0 && last_low > second_low && l[i] > low_line;
                if buy { ord.push(o(A::Buy, T::Stp(high_line), "TD line of highs penetrated")) }
                if sell { ord.push(o(A::Sell, T::Stp(low_line), "TD line of lows penetrated")) }
                if buy && st.pos != 1 {
                    let w = (shb.max(1.0)) as usize;
                    let (mut lo, mut lob) = (f64::INFINITY, 0usize);
                    for k in 0..w.min(i + 1) { if l[i - k] < lo { lo = l[i - k]; lob = k } }
                    let lob = (lob as f64).min(50.0);
                    let slope = (last_high - second_high) / (lhb - shb);
                    let old = high_line + slope * lob;
                    let diff = (old - lo).max(0.0);
                    btg = high_line + diff;
                    bls = high_line - diff;
                }
                if sell && st.pos != -1 {
                    let w = (slb.max(1.0)) as usize;
                    let (mut hi, mut hib) = (f64::NEG_INFINITY, 0usize);
                    for k in 0..w.min(i + 1) { if h[i - k] > hi { hi = h[i - k]; hib = k } }
                    let hib = (hib as f64).min(50.0);
                    let slope = (last_low - second_low) / (llb - slb);
                    let old = low_line + slope * hib;
                    let diff = (hi - old).max(0.0);
                    stg = low_line - diff;
                    sls = low_line + diff;
                }
                ord.push(o(A::XL, T::Lmt(btg), "TD price projector target"));
                ord.push(o(A::XL, T::Stp(bls), "TD stop loss"));
                ord.push(o(A::XS, T::Lmt(stg), "TD price projector target"));
                ord.push(o(A::XS, T::Stp(sls), "TD stop loss"));
                None
            })
        }
        "lkcpt_rsimild" => {
            let r = rsi(c, u("n"));
            let dl = u("days");
            el(x, from, to, |i, st, ord| {
                exit1(st, i, c, &sd30, ord);
                if i < dl + 2 { return None }
                let lv = |a: f64| i > 0 && r[i] > a && r[i - 1] <= a;
                let lw = |a: f64| i > 0 && r[i] < a && r[i - 1] >= a;
                if lv(40.0) && hh(&r, i - 1, dl + 1) > 40.0 { ord.push(mkt(A::Buy, "RSI mildly oversold, back above 40")) }
                if lw(60.0) && ll(&r, i - 1, dl + 1) < 60.0 { ord.push(mkt(A::Sell, "RSI mildly overbought, back below 60")) }
                None
            })
        }
        "lkcpt_reimild" => {
            let dl = u("days");
            let mut rei = vec![f64::NAN; n];
            let (mut rh, mut rl, mut rt) = (vec![0.0; n], vec![0.0; n], vec![0.0; n]);
            for i in 8..n {
                let hq = if h[i - 2] < c[i - 7] && h[i - 2] < c[i - 8] && h[i] < l[i - 5] && h[i] < l[i - 6] { 0.0 } else { 1.0 };
                let lq = if l[i - 2] > c[i - 7] && l[i - 2] > c[i - 8] && l[i] > c[i - 5] && l[i] > c[i - 6] { 0.0 } else { 1.0 };
                rh[i] = h[i] - h[i - 2];
                rl[i] = l[i] - l[i - 2];
                rt[i] = hq * rh[i] + lq * rl[i];
                if i >= 15 {
                    let s: f64 = (i - 7..=i).map(|k| rt[k]).sum();
                    let mut d: f64 = (i - 7..=i).map(|k| rh[k].abs() + rl[k].abs()).sum();
                    if d == 0.0 { d = 1.0 }
                    rei[i] = 100.0 * s / d;
                }
            }
            let ma = sma(c, 20);
            el(x, from, to, |i, st, ord| {
                exit1(st, i, c, &sd30, ord);
                if i < 25 { return None }
                if rei[i] > -60.0 && rei[i - 1] <= -60.0 && hh(&rei, i - 1, dl + 1) > -60.0 && c[i] > ma[i] { ord.push(mkt(A::Buy, "REI mildly oversold, above 20-day MA")) }
                if rei[i] < 60.0 && rei[i - 1] >= 60.0 && ll(&rei, i - 1, dl + 1) < 60.0 && c[i] < ma[i] { ord.push(mkt(A::Sell, "REI mildly overbought, below 20-day MA")) }
                None
            })
        }
        "lkcpt_demarker" => {
            let dl = u("days");
            let (mut dh, mut dlo) = (vec![0.0; n], vec![0.0; n]);
            for i in 1..n {
                dh[i] = if h[i] > h[i - 1] { h[i] - h[i - 1] } else { dh[i - 1] };
                dlo[i] = if l[i] < l[i - 1] { l[i - 1] - l[i] } else { dlo[i - 1] };
            }
            let dm: Vec<f64> = (0..n).map(|i| {
                if i < 13 { return f64::NAN }
                let (a, b): (f64, f64) = ((i - 12..=i).map(|k| dh[k]).sum(), (i - 12..=i).map(|k| dlo[k]).sum());
                if a + b != 0.0 { 100.0 * a / (a + b) } else { 0.0 }
            }).collect();
            let (m20, m30) = (sma(c, 20), sma(c, 30));
            el(x, from, to, |i, st, ord| {
                exit1(st, i, c, &sd30, ord);
                if i < 25 { return None }
                if dm[i] > 30.0 && dm[i - 1] <= 30.0 && hh(&dm, i - 1, dl + 1) > 30.0 && c[i] > m20[i] { ord.push(mkt(A::Buy, "DeMarker mildly oversold, above 20-day MA")) }
                if dm[i] < 70.0 && dm[i - 1] >= 70.0 && ll(&dm, i - 1, dl + 1) < 70.0 && c[i] < m30[i] { ord.push(mkt(A::Sell, "DeMarker mildly overbought, below 30-day MA")) }
                None
            })
        }
        "lkcpt_obos" => {
            let (ah, al) = (sma(h, 3), sma(l, 3));
            el(x, from, to, |i, st, ord| {
                exit1(st, i, c, &sd30, ord);
                if i < 20 { return None }
                let hl = (0..4).any(|k| l[i - k] == hh(l, i - k, 13));
                let lh = (0..4).any(|k| h[i - k] == ll(h, i - k, 13));
                if c[i] > ah[i] && lh { ord.push(mkt(A::Buy, "close above 3-day MA of highs after a 13-day low")) }
                if c[i] < al[i] && hl { ord.push(mkt(A::Sell, "close below 3-day MA of lows after a 13-day high")) }
                None
            })
        }
        "lkcpt_kurt03" | "lkcpt_kurt10" | "lkcpt_fsrs" => {
            let (al, mo) = if id == "lkcpt_fsrs" { (0.03, 3) } else { (g("alpha"), u("mo")) };
            let mut k = vec![0.0; n];
            for i in 1..n {
                let acc = if i > mo { (c[i] - c[i - mo]) - (c[i - 1] - c[i - 1 - mo]) } else { 0.0 };
                k[i] = al * acc + (1.0 - al) * k[i - 1];
            }
            if id == "lkcpt_fsrs" {
                let w = wavg(&k, 6);
                let r9 = rsi(c, 9);
                let f: Vec<f64> = (0..n).map(|i| 10000.0 * w[i] + r9[i]).collect();
                let fw = wavg(&f, 6);
                sar_lvl((0..n).map(|i| f[i] > fw[i]).collect(), (0..n).map(|i| f[i] < fw[i]).collect(), "FSRS above its weighted average", "FSRS below its weighted average")
            } else {
                sar_lvl((0..n).map(|i| i > mo && k[i] > 0.0).collect(), (0..n).map(|i| i > mo && k[i] < 0.0).collect(), "kurtosis positive", "kurtosis negative")
            }
        }
        "lkcpt_123" => {
            let (_, sk, sdd) = stoch(x, 9);
            el(x, from, to, |i, st, ord| {
                exit1(st, i, c, &sd30, ord);
                if i < 4 { return None }
                if c[i] > c[i - 1] && c[i - 1] < c[i - 2] && c[i - 2] < c[i - 3] && sk[i] > sk[i - 1] && sdd[i] < 50.0 { ord.push(mkt(A::Buy, "1-2-3 bottom")) }
                if c[i] < c[i - 1] && c[i - 1] > c[i - 2] && c[i - 2] > c[i - 3] && sk[i] < sk[i - 1] && sdd[i] > 50.0 { ord.push(mkt(A::Sell, "1-2-3 top")) }
                None
            })
        }
        "lkcpt_ons" => {
            let (a, b) = (sma(c, 10), sma(c, 40));
            el(x, from, to, |i, st, ord| {
                if dow(&x.r[i].date) == 4 {
                    if a[i] > b[i] { ord.push(o(A::Buy, T::Stp(hh(h, i, 4)), "Thursday: 4-day high stop")) }
                    else { ord.push(o(A::Sell, T::Stp(ll(l, i, 8)), "Thursday: 8-day low stop")) }
                }
                if st.pos > 0 { ord.push(mkt(A::XL, "next open")) }
                if st.pos < 0 { ord.push(mkt(A::XS, "next open")) }
                None
            })
        }
        "lkcpt_tts3" | "lkcpt_tts10" => {
            let r = rsi(c, u("rsi"));
            let a = sma(&r, u("avg"));
            sar_lvl((0..n).map(|i| r[i] > a[i]).collect(), (0..n).map(|i| r[i] < a[i]).collect(), "RSI above its average", "RSI below its average")
        }
        "lkcpt_qp" => el(x, from, to, |i, st, ord| {
            exit1(st, i, c, &sd30, ord);
            if i < 3 { return None }
            if c[i - 3] > op[i - 3] && c[i - 2] > op[i - 2] && c[i - 2] > c[i - 3] { ord.push(o(A::Buy, T::Stp(h[i - 2]), "High[2] stop")) }
            if c[i - 3] < op[i - 3] && c[i - 2] < op[i - 2] && c[i - 2] < c[i - 3] { ord.push(o(A::Sell, T::Stp(l[i - 2]), "Low[2] stop")) }
            None
        }),
        "lkcpt_gap" | "lkcpt_fib" | "lkcpt_jesse" => {
            let (set, opp) = if id == "lkcpt_fib" { (u("set"), u("opp")) } else { (14, 4) };
            el(x, from, to, |i, st, ord| {
                if i < 15 { return None }
                match id {
                    "lkcpt_gap" => {
                        if l[i] > h[i - 1] { ord.push(mkt(A::Buy, "gap up")) }
                        if h[i] < l[i - 1] { ord.push(mkt(A::Sell, "gap down")) }
                    }
                    "lkcpt_fib" => {
                        if l[i] == ll(l, i, set) { ord.push(o(A::Buy, T::Stp(hh(h, i, opp)), "14-day low: 4-day high stop")) }
                        if h[i] == hh(h, i, set) { ord.push(o(A::Sell, T::Stp(ll(l, i, opp)), "14-day high: 4-day low stop")) }
                    }
                    _ => {
                        if h[i] > h[i - 1] && l[i] < l[i - 1] {
                            ord.push(o(A::Buy, T::Stp(hh(h, i, 2)), "outside day: 2-day high stop"));
                            ord.push(o(A::Sell, T::Stp(ll(l, i, 2)), "outside day: 2-day low stop"));
                        }
                        ord.push(o(A::XL, T::Stp(ll(l, i, 8)), "8-day low stop"));
                        ord.push(o(A::XS, T::Stp(hh(h, i, 8)), "8-day high stop"));
                    }
                }
                let lim = if id == "lkcpt_jesse" { 11 } else { 2 };
                if st.pos != 0 && i - st.ei > lim {
                    return Some((if st.pos > 0 { A::XL } else { A::XS }, "bars since entry exceeded"));
                }
                None
            })
        }
        "lkcpt_wvol" => {
            let tr = true_range(x.r);
            let mut vi = vec![0.0; n];
            for i in 0..n { vi[i] = (6.0 * if i > 0 { vi[i - 1] } else { 0.0 } + tr[i]) / 7.0 }
            let m10 = sma(c, 10);
            let mult = g("mult");
            let (mut hc, mut lc) = (0.0f64, 0.0f64);
            el(x, from, to, |i, st, ord| {
                if i == from {
                    ord.push(mkt(if c[i] > m10[i] { A::Buy } else { A::Sell }, "first bar: close vs 10-day average"));
                }
                if st.pos == 1 && c[i] > hc { hc = c[i] }
                if st.pos == -1 && c[i] < lc { lc = c[i] }
                if st.pos == 1 && c[i] < hc - mult * vi[i] { lc = c[i]; ord.push(mkt(A::Sell, "close 3 ATR below the highest close")) }
                if st.pos == -1 && c[i] > lc + mult * vi[i] { hc = c[i]; ord.push(mkt(A::Buy, "close 3 ATR above the lowest close")) }
                None
            })
        }
        "lkcpt_wdmi" => {
            let m = u("n");
            let (pd, md, _) = dmi3(x, m);
            let (_, _, a15) = dmi3(x, 15);
            let adxr: Vec<f64> = (0..n).map(|i| if i >= 15 { (a15[i] + a15[i - 15]) / 2.0 } else { f64::NAN }).collect();
            let lim = g("adxr");
            el(x, from, to, |i, _, ord| {
                if xover(&pd, &md, i) && adxr[i] > lim { ord.push(mkt(A::Buy, "+DI crossed above -DI, ADXR > 25")) }
                if xunder(&pd, &md, i) && adxr[i] > lim { ord.push(mkt(A::Sell, "+DI crossed below -DI, ADXR > 25")) }
                if pd[i] < md[i] { ord.push(mkt(A::XL, "+DI below -DI")) }
                if pd[i] > md[i] { ord.push(mkt(A::XS, "+DI above -DI")) }
                None
            })
        }
        _ => return None,
    };
    Some(t)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;
    fn rows(v: &[(f64, f64, f64, f64)]) -> Vec<Row> {
        v.iter().enumerate().map(|(i, &(o, h, l, c))| Row { ticker: "T".into(), date: format!("2001-{:02}-{:02}", 1 + i / 28, 1 + i % 28), open: o, high: h, low: l, close: c, volume: None }).collect()
    }
    #[test]
    fn stop_entry_fills_at_the_stop_or_the_gap_open() {
        // 45 narrowing bars around 100, then bar 45 trades to 103 from an open of 100: the buy stop at the 40-day high
        // (bar 5's high, 100.95) fills at the stop price, not at the bar's high
        let mut v: Vec<(f64, f64, f64, f64)> = (0..45).map(|i| (100.0, 101.0 - 0.01 * i as f64, 99.0 + 0.01 * i as f64, 100.0)).collect();
        v.push((100.0, 103.0, 99.5, 102.0));
        v.push((104.0, 105.0, 103.5, 104.5)); // gaps above the new 40-day high: no new entry (already long)
        let r = rows(&v);
        let x = Ctx::new(&r, &r, &[]);
        let t = run("lkcpt_cb4020", &x, 40, r.len() - 1, &Params::new()).unwrap();
        assert_eq!(t.len(), 1);
        assert_eq!(t[0].dir, 1.0);
        assert_eq!(t[0].legs[0].i, 45);
        assert!((t[0].legs[0].px - 100.95).abs() < 1e-9);
        assert!(t[0].open);
    }
    #[test]
    fn day_of_week() {
        assert_eq!(dow("2024-01-01"), 1); // Monday
        assert_eq!(dow("1998-12-31"), 4); // Thursday
    }
}
