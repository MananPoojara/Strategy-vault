//! Tushar Chande, "Beyond Technical Analysis" (Wiley 1997), COMPLETE edition: every fully specified system
//! that books/chande.rs (built from a partial copy) does not already have. chande.rs keeps ch_65sma3cc and the
//! CB-PB family; nothing here repeats them. Source + page per rule: library/chbta2/README.md.
//! Reviewed stop-free price contracts: library/chande/REVIEW-2026-10-13.md.
//! Other historical runners below remain outside that review. Futures dollars, points and ticks require
//! actual contract data; their old generic point parameters do not establish a cash-market adaptation.
use crate::connors::{dmi, sma, Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "chbta2";

// ------------------------------------------------------------------ small shared engine (also used by chntt)

/// How an entry signal formed on bar i's close is executed.
#[derive(Clone, Copy, PartialEq, Debug)]
pub(crate) enum Kind {
    Open,      // market on the next bar's open
    Close,     // on this bar's close
    NextClose, // on the next bar's close
    Stop(f64), // stop order for the next bar only (buy: high >= level, sell: low <= level)
}

pub(crate) struct Pos {
    pub dir: f64,
    pub i: usize,
    pub px: f64,
    pub kind: &'static str,
}

type Sig<'a> = &'a dyn Fn(usize, f64) -> [Option<(f64, Kind)>; 2];
type OnPos<'a, T> = &'a dyn Fn(usize, &Pos) -> T;

pub(crate) struct Sys<'a> {
    /// Entry orders formed on bar i's close given the current direction (0 flat). Same-direction orders are ignored
    /// while in a position; an opposite order reverses (TradeStation default).
    pub entry: Sig<'a>,
    /// Exit on this bar's close.
    pub exit_close: OnPos<'a, Option<String>>,
    /// Exit at the next bar's open (decided on this bar's close).
    pub exit_open: OnPos<'a, Option<String>>,
    /// Protective stop level for the next bar (NaN = none).
    pub trail: OnPos<'a, f64>,
    /// Initial stop distance in price points from the fill (0 = off).
    pub stop_pts: f64,
    /// Book's "one-way" test model: no two consecutive trades in the same direction.
    pub oneway: bool,
}

pub(crate) fn none_exit(_: usize, _: &Pos) -> Option<String> { None }
pub(crate) fn no_trail(_: usize, _: &Pos) -> f64 { f64::NAN }

fn done(p: Pos, i: usize, px: f64, why: &str, sig: usize) -> Trade {
    Trade { dir: p.dir, legs: vec![Leg { i: p.i, px: p.px, w: 1. }], exit_i: i, exit_px: px, reason: why.into(),
        entry_kind: p.kind, signal_i: sig, open: false, exits: vec![] }
}

/// One position at a time, bar by bar. Order inside a bar: next-open exit, next-open entry, protective stop,
/// stop entries, next-close entry, close exit, close signal. Gaps through a stop fill at the open.
pub(crate) fn run_sys(x: &Ctx, from: usize, to: usize, s: &Sys) -> Vec<Trade> {
    let mut out = vec![];
    let (mut pos, mut last): (Option<Pos>, f64) = (None, 0.);
    let mut pend: [Option<(f64, Kind)>; 2] = [None, None];
    let (mut pend_exit, mut trail): (Option<String>, f64) = (None, f64::NAN);
    let init = |p: &Pos| if s.stop_pts > 0. { p.px - p.dir * s.stop_pts } else { f64::NAN };
    for i in from..=to {
        if i > from {
            if let Some(why) = pend_exit.take() {
                if let Some(p) = pos.take() { out.push(done(p, i, x.o[i], &why, i - 1)); }
            }
            for k in 0..2 {
                if let Some((d, Kind::Open)) = pend[k] {
                    pend[k] = None;
                    if pos.as_ref().is_some_and(|p| p.dir == d) || (s.oneway && last == d && pos.is_none()) { continue; }
                    if let Some(p) = pos.take() { out.push(done(p, i, x.o[i], "Opposite signal · next-open reversal", i - 1)); }
                    pos = Some(Pos { dir: d, i, px: x.o[i], kind: "open" });
                    last = d;
                    trail = f64::NAN;
                }
            }
            if let Some(p) = &pos {
                let mut lv = trail;
                let st = init(p);
                if st.is_finite() && !(lv.is_finite() && (lv - st) * p.dir > 0.) { lv = st; }
                if lv.is_finite() && ((p.dir > 0. && x.l[i] <= lv) || (p.dir < 0. && x.h[i] >= lv)) {
                    let px = if p.dir > 0. { x.o[i].min(lv) } else { x.o[i].max(lv) };
                    let why = if lv == st { "Initial stop" } else { "Trailing/exit stop" };
                    out.push(done(pos.take().unwrap(), i, px, why, i));
                }
            }
            // stop entries; if both sides trigger on one bar, the level nearer the open is taken first
            let mut hits: Vec<(f64, f64, f64)> = vec![];
            for k in 0..2 {
                if let Some((d, Kind::Stop(lv))) = pend[k] {
                    pend[k] = None;
                    if (d > 0. && x.h[i] >= lv) || (d < 0. && x.l[i] <= lv) {
                        let px = if d > 0. { x.o[i].max(lv) } else { x.o[i].min(lv) };
                        hits.push(((px - x.o[i]).abs(), d, px));
                    }
                }
            }
            hits.sort_by(|a, b| a.0.total_cmp(&b.0));
            let free = |d: f64, pos: &Option<Pos>| !(pos.as_ref().is_some_and(|p| p.dir == d) || (s.oneway && last == d && pos.is_none()));
            if let Some(&(_, d, px)) = hits.iter().find(|h| free(h.1, &pos)) {
                {
                    if let Some(p) = pos.take() { out.push(done(p, i, px, "Opposite stop entry · reversal", i)); }
                    let p = Pos { dir: d, i, px, kind: "stop" };
                    let st = init(&p);
                    last = d;
                    if st.is_finite() && ((d > 0. && x.l[i] <= st) || (d < 0. && x.h[i] >= st)) {
                        out.push(done(p, i, st, "Initial stop · same bar as entry (conservative)", i));
                    } else { pos = Some(p); }
                }
            }
            for k in 0..2 {
                if let Some((d, Kind::NextClose)) = pend[k] {
                    pend[k] = None;
                    if pos.as_ref().is_some_and(|p| p.dir == d) || (s.oneway && last == d && pos.is_none()) { continue; }
                    if let Some(p) = pos.take() { out.push(done(p, i, x.c[i], "Opposite signal · close reversal", i)); }
                    pos = Some(Pos { dir: d, i, px: x.c[i], kind: "close" });
                    last = d;
                }
            }
        }
        if let Some(why) = pos.as_ref().and_then(|p| (s.exit_close)(i, p)) {
            out.push(done(pos.take().unwrap(), i, x.c[i], &why, i));
        }
        pend = [None, None];
        if i < to {
            let cur = pos.as_ref().map_or(0., |p| p.dir);
            for (k, o) in (s.entry)(i, cur).into_iter().enumerate() {
                let Some((d, kind)) = o else { continue };
                if d == cur { continue; }
                if kind == Kind::Close {
                    if s.oneway && last == d && pos.is_none() { continue; }
                    if let Some(p) = pos.take() { out.push(done(p, i, x.c[i], "Opposite signal · close reversal", i)); }
                    pos = Some(Pos { dir: d, i, px: x.c[i], kind: "close" });
                    last = d;
                } else { pend[k] = Some((d, kind)); }
            }
        }
        trail = pos.as_ref().map_or(f64::NAN, |p| (s.trail)(i, p));
        pend_exit = pos.as_ref().and_then(|p| (s.exit_open)(i, p));
    }
    if let Some(p) = pos {
        let mut t = done(p, to, x.c[to], "open", to);
        t.open = true;
        out.push(t);
    }
    out
}

/// Highest high / lowest low of the n bars ending at i (inclusive). NaN before n bars exist.
pub(crate) fn hh(x: &Ctx, i: usize, n: usize) -> f64 {
    if n == 0 || i + 1 < n { return f64::NAN; }
    x.h[i + 1 - n..=i].iter().copied().fold(f64::NEG_INFINITY, f64::max)
}
pub(crate) fn ll(x: &Ctx, i: usize, n: usize) -> f64 {
    if n == 0 || i + 1 < n { return f64::NAN; }
    x.l[i + 1 - n..=i].iter().copied().fold(f64::INFINITY, f64::min)
}
/// Same window ending at the previous bar ("of the last n days" before today).
pub(crate) fn hh1(x: &Ctx, i: usize, n: usize) -> f64 { if i == 0 { f64::NAN } else { hh(x, i - 1, n) } }
pub(crate) fn ll1(x: &Ctx, i: usize, n: usize) -> f64 { if i == 0 { f64::NAN } else { ll(x, i - 1, n) } }

/// Standard EMA, alpha = 2/(n+1), seeded with the first finite value.
pub(crate) fn ema(v: &[f64], n: usize) -> Vec<f64> {
    let a = 2. / (n as f64 + 1.);
    let mut out = vec![f64::NAN; v.len()];
    let mut prev = f64::NAN;
    for (i, &x) in v.iter().enumerate() {
        if !x.is_finite() { prev = f64::NAN; continue; }
        prev = if prev.is_finite() { prev + a * (x - prev) } else { x };
        out[i] = prev;
    }
    out
}

pub(crate) fn getp(schema: &[ParamDef], p: &Params, k: &str) -> f64 {
    p.get(k).copied().filter(|v| v.is_finite()).unwrap_or_else(|| schema.iter().find(|d| d.key == k).map_or(0., |d| d.def))
}

// ------------------------------------------------------------------ catalogue

const STOP_GAP: &str = "Book tests use a per-contract dollar initial stop on futures continuous contracts; converting it needs the contract's point value and futures data. Exposed as stop_pts (index points), off by default.";
const ND_INTERMARKET: &str = "Needs daily futures for two or three different markets (gold, T-bond, soybeans, copper, deutsche mark, corn, crude oil, Eurodollar, yen, live hogs). The vault has equity indices only; running bond rules on an equity index would be an asset-class swap.";
const ND_TICKS: &str = "The barrier is a number of futures ticks (20 or 10 ticks of each contract). An index has no contract tick; needs futures contract tick size and data.";
const ND_SIZE: &str = "Position size = $10,000 / (5-day high-low range in contract dollars), max 10 contracts. Needs contract point values; the vault reports one-unit percentage returns.";

pub fn catalogue() -> Vec<StrategyDef> {
    let d = |id, name, group, dir, status, rules, notes| StrategyDef { id, name, group, session: 0, dir, status,
        needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK, rules, notes, claim: None };
    use Status::*;
    vec![
        d("ch_bta2_ma_stack", "Multiple short SMAs versus 65-day SMA", "Ch 2 · Number of rules", "Long/Short", Live, &[
            "E|Buy on the close when both the 3-day and 4-day simple averages are above the 65-day average; short with the symmetric inequalities (printed p.20).",
            "X|Opposing full agreement exits and reverses; mixed agreement holds the current position (pp.20-21, discussion of delayed entries and exits).",
            "F|The printed 3-, 4-, 5-, 6-, 7-day variant adds short averages to the same condition (p.20).",
            "X|The example explicitly uses no initial stop (pp.20-21)."],
            "Reviewed price-return research contract, printed pp.20-21 (PDF pp.29-30). Defaults first=3, count=2, slow=65 are printed; count=5 is the printed 3-through-7 variant. Other lookbacks are configurable research variants. Flat at the test start, one position per actual symbol, close orders include the final observed close; the terminal position is marked open. No futures dollar costs or contract sizing are reproduced."),
        d("ch_bta2_65sma_ravi", "65sma-3cc with RAVI filter", "Ch 4 · 65sma-3cc system", "Long/Short", Live, &[
            "F|\"The RAVI is the absolute percentage difference between the 7-day and 65-day simple moving averages of the daily close\" (p.94); RAVI = Absolute value (100 x (7-SMA - 65-SMA)/65-SMA) (p.41)",
            "E|\"The filtered model could generate a buy signal only if RAVI was greater than one and there were three consecutive closes above the 65sma\" (p.96); sells symmetric (p.96)",
            "E|Entry \"at the market on the open of the next trading day after the close of the day the signal is received\" (p.80); entry signals are also the opposite exit (always in the market, p.97)",
            "F|Barrier 1.0 (Fig 4.17, p.96); 1.5 (Fig 4.18, p.97) and 0.5 (Table 4.4, p.98) are labelled options",
            "X|Tests use a \"one-way\" model: no back-to-back long or short trades (p.80, p.99)",
            "X|Reviewed version is the stop-free, always-invested filtered model in Figures 4.17-4.18, pp.96-97, using the base reversal execution from p.80. Table 4.4's separate $1,000-stop experiment is held."],
            "Reviewed printed pp.80,94-97,99. Defaults SMA=65, confirm=3, fast=7, RAVI>1.0. Flat at test start; actual selected cash/ETF/stock prices are a declared research application. No dollar stop, contract tick, rollover, portfolio sizing or book performance is reproduced. A legacy nonzero stop_pts request is rejected; changing the RAVI threshold remains a variant of this family."),
        d("ch_bta2_65sma_ravi_x14", "65sma-3cc + RAVI 0.5 + 14-day exit", "Ch 4 · 65sma-3cc system", "Long/Short", Live, &[
            "E|65sma-3cc entries (three consecutive closes above/below the 65-day SMA, next open) filtered by RAVI > 0.5 (p.100)",
            "X|\"if long, we would exit a trade tomorrow on the open if today's close is lower than the lowest low of the last 14 days\" (p.100); shorts symmetric (highest high)",
            "X|One-way model: after an exit wait for the opposite signal (p.89, p.99)",
            "X|Table 4.6 also uses a $1,000 initial stop [stop_pts, off by default]"],
            "Printed pp.99-101, Table 4.6. 'Last 14 days' excludes today (a close cannot be below today's own low). Table 4.6 S&P-500: profit $2,490, 460 days in market (1,290 without exit). With ravi=0 this is BTA ch.6 case 5 (14-day high-low exit)."),
        d("ch_bta2_tat", "Trend-Anti-trend (T-AT)", "Ch 4 · Trend-Anti-trend", "Long/Short", Live, &[
            "E|\"If high > highest(H,25)[1] and ADX(18) < average(ADX(18),18) then sell tomorrow on the open\" (p.120)",
            "E|\"If low < lowest(L,25)[1] and ADX(18) < average(ADX(18),18) then buy tomorrow on the open\" (p.120)",
            "E|\"If MARKETPOSITION = 1 and low < lowest(L,25)[1] and ADX(18) > average(ADX(18),18) then sell tomorrow on the open\" (p.120)",
            "E|\"If MARKETPOSITION = -1 and high > highest(H,25)[1] and ADX(18) > average(ADX(18),18) then buy tomorrow on the open\" (p.120)",
            "X|\"we will test this system without specific exits, so that the entries also serve as the exit for the opposite position\" (p.116)",
            "X|Table 4.14 uses a $5,000 initial stop [stop_pts; S&P = 10 points]"],
            "Printed pp.116-123. TradeStation semantics: a buy order while long is ignored, a sell order while long reverses. From flat only the anti-trend orders can fire (the trend orders require a position). If a bar makes both a new 25-day high and low from flat, no order (ambiguous). ADX = Wilder (engine dmi). Table 4.14 S&P-500: 141 trades, PF 1.34, $5,000 stop."),
        d("ch_bta2_ma1011_trend", "S&P 10/11-day SMA crossover (trend)", "Ch 4 · S&P bottom-fishing", "Long/Short", Live, &[
            "E|\"the standard simple moving average crossover system with 10-day and 11-day simple moving averages\" (p.133): buy when the 10-day SMA crosses above the 11-day SMA, sell when it crosses below, on the close",
            "X|Opposite crossover reverses; \"relatively loose $2,000 initial stop\" (p.133) = 4 S&P points ($500/point, p.134)"],
            "Printed p.133. Book (S&P futures 4/21/82-7/10/95): lost $181,005 on 530 trades, 34% winners. Order type taken from the anti-trend rules on the same page (\"on the close\")."),
        d("ch_bta2_ma1011_anti", "S&P 10/11-day SMA crossover (anti-trend)", "Ch 4 · S&P bottom-fishing", "Long/Short", Live, &[
            "E|\"1. Buy if the 10-day SMA crosses below the 11-day SMA on the close.\" (p.133)",
            "E|\"2. Sell if the 10-day SMA crosses above the 11-day SMA on the close.\" (p.133)",
            "X|Opposite signal reverses; $2,000 initial stop = 4 S&P points ($500/point, p.134)"],
            "Printed p.133. Book (S&P futures 4/21/82-7/10/95): netted $55,920 on 531 trades, 48% (254) winners, max DD $32,735."),
        d("ch_bta2_bottom_fish", "S&P bottom-fishing pattern", "Ch 4 · S&P bottom-fishing", "Long", Live, &[
            "F|\"1. A 20-day low has formed within the last 5 days.\" (p.134)",
            "F|\"2. Today's high-low range > X; X = 4 for conservative trades; X = 1 for aggressive trades (each point is ... one full S&P index point = $500)\" (p.134)",
            "F|\"3. Today's closing-opening range > Y; Y = 3 for conservative trades; Y = 0 for aggressive trades.\" (p.134)",
            "E|\"4. If rules 1, 2, and 3 are true, then buy tomorrow on the close.\" (p.134)",
            "X|\"5. Exit on the close of the twentieth day in the trade.\" (p.134)",
            "X|\"6. Initial money management stop = $2,000 per contract.\" = 4 S&P points (p.134)"],
            "Printed pp.132-140, Table 4.20 (S&P futures with rollover): X=4,Y=3 9/82-2/88 18 trades 44% won; 2/88-7/95 46 trades 39%; X=1,Y=0 9/82-2/88 57 trades 45%; 2/88-7/95 93 trades 35%. Reading: the 20-day lowest low lies on today or one of the 4 previous bars; 20th day = 20 bars after the entry bar. On NIFTY the S&P point thresholds are used literally (instrument swap only)."),
        d("ch_bta2_bottom_fish_generic", "Generic bottom-fishing pattern", "Ch 4 · S&P bottom-fishing", "Long", Live, &[
            "F|Same pattern with \"X= 0.1 and Y = 0\" (p.137)",
            "E|\"The entry is switched to above the high of the signal day, instead of buying at the next day's close\" (p.137) = buy stop next day",
            "X|\"A trend-following exit, at the lowest low of the last 20 days\" (p.137) = sell stop",
            "X|$2,000 initial stop (17 futures markets) [stop_pts, off by default; S&P = 4]"],
            "Printed pp.137-139, Table 4.21. S&P-500 row: profit $127,925, 111 trades, 30% won, PF 1.64. Buy stop = signal high + tick (0.01 index point); the 20-day low includes the current bar."),
        d("ch_bta2_xopp", "Extraordinary opportunity (3% band)", "Ch 4 · Extraordinary opportunities", "Long/Short", Live, &[
            "F|\"Use a 50-day SMA and plot a 3-percent trading band around it. Then a 7-day SMA must cross outside the upper or lower band\" (p.141)",
            "E|\"The long entry rule requires that the 7-day SMA be beyond 1.03 times the 50-day SMA in order to purchase just above the highest high of the last 20 days. The opposite conditions are needed for the short trades.\" (p.142)",
            "X|\"The exit was on the close of the twentieth day\" (p.142)",
            "X|$3,000 stop [stop_pts, off by default]"],
            "Printed pp.140-144, Table 4.22. S&P-500: $28,040, 22 winners of 58 trades (38%), PF 1.27. Buy stop = 20-day high (incl. signal day) + tick; short: 7-SMA < 0.97 x 50-SMA, sell stop 20-day low - tick. Opposite stop entry reverses."),
        d("ch_bta2_xopp_trail", "Extraordinary opportunity (1% band, 20-day trailing stop)", "Ch 4 · Extraordinary opportunities", "Long/Short", Live, &[
            "E|Same entries with a 1 percent band (Table 4.23 title, p.144)",
            "X|\"trailing 20-day stop\" (Table 4.23, p.144) = exit at the 20-day low (long) / high (short) on a stop",
            "X|$3,000 stop [stop_pts, off by default]"],
            "Printed p.143-144, Table 4.23. S&P-500: -$25,955, 25 winners of 105 trades (23%), PF 0.73. No time exit in this variant (the trailing stop replaces it)."),
        d("ch_bta2_cbo_trail5", "20-bar breakout on close, 5-day trailing exit", "Ch 5 · Channel breakout variations", "Long/Short", Live, &[
            "E|\"1. If today's close is higher than the highest high of the last 20 days, then buy on the close. 2. If today's close is lower than the lowest low of the last 20 days, then sell on the close.\" (p.149)",
            "X|\"3. Exit the long trade at the lowest low of the last 5 days on a stop. 4. Exit the short trade at the highest high of the last 5 days on a stop.\" (p.149)",
            "X|$1,500 initial stop [stop_pts, off by default]"],
            "Printed pp.149-152, Table 5.2 (BP, coffee, cotton, crude, DM, ED, gold, JY, bond, wheat: avg 220 trades, 36% won, PF 1.19). No index in the book's table. 20-day window excludes today; 5-day stop window includes the bar the stop is set on."),
        d("ch_bta2_cbo_volexit", "20-bar breakout on close, volatility exit", "Ch 5 · Channel breakout variations", "Long/Short", Live, &[
            "E|Rules 1 and 3: buy/sell on the close beyond the 20-day high/low (p.152)",
            "X|\"2. Exit the long trade at the highest 20-day high minus three times the 10-day SMA of the daily trading range on a stop.\" (p.152)",
            "X|\"4. Exit the short trade at the lowest 20-day low plus three times the 10-day SMA of the daily trading range on a stop.\" (p.152)",
            "X|$1,500 initial stop [stop_pts, off by default]"],
            "Printed pp.152-155, Table 5.3 (avg 191 trades, 37% won, PF 1.28). Daily trading range = high - low (p.152)."),
        d("ch_bta2_cbo_inside", "20-day breakout with inside volatility barrier", "Ch 5 · Channel breakout variations", "Long/Short", Live, &[
            "E|\"the upper inside barrier is the highest 20-day high minus the volatility ... We will go long on the close if it is above the upper inside barrier, and sell on the close if the market closes below the lower inside barrier\" (p.159)",
            "F|\"Volatility is defined here as the 10-day SMA of the daily trading range\" (p.159)",
            "X|\"The volatility exit is at three times the 10-day SMA of the daily trading range\" (p.159)",
            "X|$1,500 initial stop [stop_pts, off by default]"],
            "Printed pp.159-161, Table 5.7 (avg 294 trades, 35% won, PF 1.22). Same 20-day window as the plain breakout (excluding today). If the barriers cross and the close is beyond both, no order."),
        d("ch_bta2_cbo_ref", "20-day breakout on close, no exits (reference)", "Ch 5 · Channel breakout variations", "Long/Short", Live, &[
            "E|\"a 20-day channel breakout system without any exits as the reference system\" (p.162): buy on a close above the 20-day high, sell on a close below the 20-day low",
            "X|\"long entries are short exits and vice versa\" (p.162); $1,500 initial stop [stop_pts, off by default]"],
            "Printed pp.161-165, Table 5.8 (average profit $51,638 per market, avg 118 trades)."),
        d("ch_bta2_adx30", "ADX > 30 crossover system", "Ch 5 · Two ADX variations", "Long/Short", Live, &[
            "E|\"entered long or short when the 14-day ADX value crossed over 30. A crossover occurs when yesterday's value is less than 30 and today's value is more than 30\" (p.165)",
            "E|\"If the 3-day SMA of the close is greater than the 20-day exponential moving average, then the system buys on the next days open. Conversely ... sells on the next day's open.\" (p.165)",
            "X|\"The exit signal was a trailing stop at the highest high or lowest low of the past 20 days.\" (p.165)",
            "X|$1,500 initial stop [stop_pts, off by default]"],
            "Printed pp.165-168, Table 5.11 (avg 47 trades, 43% won, PF 2.19; no index in table). EMA alpha 2/(n+1). Trailing stop window includes the bar it is set on."),
        d("ch_bta2_adx_rising", "Rising ADX system", "Ch 5 · Two ADX variations", "Long/Short", Live, &[
            "E|\"it requires only that the ADX be rising. The ADX was assumed to be rising if it was higher than its value 28 days ago. Otherwise, the rules of the two variations are identical.\" (p.165)",
            "E|3-day SMA vs 20-day EMA sets the direction; entry next day's open (p.165)",
            "X|Trailing stop at the 20-day lowest low / highest high (p.165); $1,500 initial stop [stop_pts, off]"],
            "Printed pp.165-168, Table 5.12 (avg 159 trades, 37% won, PF 1.65). Rising is a state (every day it holds can trigger while flat), consistent with 3x more trades than the crossover version."),
        d("ch_bta2_pullback", "Pullback system (5-day extreme vs 50-day SMA)", "Ch 5 · Pullback system", "Long/Short", Live, &[
            "E|\"the trend is defined as up when the market makes a new 5-day low but the low remains above the 50-day SMA ... the new 5-day high remains below the 50-day SMA\" (p.169)",
            "E|\"The entry was on the open after a new 5-day low above the 50-day SMA and vice versa.\" (p.169)",
            "X|\"The exit was on the trailing 20-day high or low.\" (p.169); $1,500 initial stop [stop_pts, off]"],
            "Printed pp.168-173, Table 5.13 (avg 129 trades, 29% won, PF 1.65). New 5-day low = low below the lowest low of the previous 5 bars."),
        d("ch_bta2_long_bomb", "The Long Bomb", "Ch 5 · Long bomb", "Long", Live, &[
            "E|\"Condition1 = c > h[2]; Condition2 = c[1] > h[3]; Condition3 = c[2] > h[4]; If Condition1 and Condition2 and Condition3 then buy ... at high + 1 point stop\" (p.174)",
            "X|\"ExitLong at Lowest(low,dx)[1] - 1 point stop;\" dx = 50 (p.174)",
            "X|Sizing NumC = intportion(10000/((C - lowest(low,5))*BigPointValue)) and a $1,500 stop (p.176) [not modelled: one unit; stop_pts off]"],
            "Printed pp.173-177, Table 5.15 (10 futures markets 1/82-7/95, ~30 trades each). 1 point = 0.01 index point (TradeStation minimum move of the index data). Sizing changes dollar P&L only."),
        d("ch_bta2_vol2x", "Volatility system (2x range stops)", "Ch 8 · Data scrambling", "Long/Short", Live, &[
            "E|\"The buy stop for tomorrow is today's high plus two times today's high-low range. Similarly, the sell stop is today's low minus two times today's high-low range.\" (p.237)",
            "X|\"exiting on the close of the twentieth day in the trade\" (p.237)",
            "X|$3,000 initial stop [stop_pts, off by default]"],
            "Printed pp.236-239 (Swiss franc 6/89-6/95; Table 8.4 synthetic data). An opposite stop entry reverses (TradeStation default). Also used in ch.7 with a $2,500 stop and 10-day exit."),
        d("ch_bta2_65sma_stop", "65sma-3cc with $1,000 initial stop", "Ch 4 · 65sma-3cc system", "Long/Short", NeedsData(STOP_GAP), &[
            "E|65sma-3cc one-way model (p.89)", "X|\"Adding a $1,000 stop\" (Table 4.3, p.90); $250-$1,750 range p.89; ch.6 $1,500/$5,000 stops on DM"],
            "Printed pp.88-93. Multi-market futures dollar stop. For the S&P alone run ch_bta2_65sma_ravi with ravi=0, stop_pts=2."),
        d("ch_bta2_goldbond_ma", "Gold-bond dual MA intermarket", "Ch 4 · Gold-bond intermarket", "Long/Short", NeedsData(ND_INTERMARKET), &[
            "E|\"1. If the 10-day SMA of gold crosses above the 50-day SMA, then sell the T-bond futures tomorrow on the open. 2. Conversely, if the 10-day SMA gold crosses under the 50-day SMA, then buy the T-bond futures tomorrow on the open.\" (p.126)",
            "X|$5,000 initial stop (p.127)"], "Table 4.15: 122 trades, 48% won, $38,675. Same rule on soybeans/copper/DM -> bond (Table 4.16), corn/ED -> crude (4.18), yen -> DM and corn -> hogs (4.19)."),
        d("ch_bta2_goldbond_adx", "Gold-bond ADX-filtered intermarket", "Ch 4 · Gold-bond intermarket", "Long/Short", NeedsData(ND_INTERMARKET), &[
            "E|\"1. If the 14-day ADX is greater than its value 14 days ago, and if the 3-day SMA is below the 20-day SMA of the daily gold closes, then buy the bond futures on tomorrows open. 2. ... above ... then sell the bond futures on tomorrow's open.\" (p.127)",
            "X|$5,000 initial stop (p.127)"], "Table 4.15: 152 trades, 52% won, $92,488."),
        d("ch_bta2_intermarket_trio", "Gold-soybeans-bond (three markets)", "Ch 4 · Gold-bond intermarket", "Long/Short", NeedsData(ND_INTERMARKET), &[
            "E|\"if the 10-day SMA of the daily close was below the 50-day SMA for both gold and soybeans, then that would trigger a buy signal for bonds\" (p.129)", "X|$5,000 stop"],
            "Table 4.17: 77 trades 47% (gold-soy-bond); 27 trades 56% (gold-copper-bond)."),
        d("ch_bta2_cbo_20tick", "20-bar breakout with 20-tick barrier", "Ch 5 · Channel breakout variations", "Long/Short", NeedsData(ND_TICKS), &[
            "E|Close must exceed the 20-day high/low by 20 ticks instead of 1 tick (p.155)", "X|Volatility exit (Table 5.4) or no exit (Table 5.5); $1,500 stop"],
            "Tables 5.4-5.5. Ch.7 uses the same breakout with a 10-tick barrier (CHBOC)."),
        d("ch_bta2_pullback_10c", "Pullback system, up to ten contracts", "Ch 5 · Pullback system", "Long/Short", NeedsData(ND_SIZE), &[
            "E|Pullback entries (p.172)", "A|\"trade up to a maximum of ten contracts using the 5-day high-low range as the measure of volatility\" for $10,000 risk (p.172)"],
            "Table 5.14. Same signals as ch_bta2_pullback; only sizing differs."),
        d("ch_bta2_adx_burst", "ADX burst trend-seeking system", "Ch 4 · ADX burst", "Long/Short", NotTestable(
            "Entry order type is not given and \"a simple 20-day exit strategy\" is not defined (20th-day close or 20-day channel); the book shows only a chart for the burst trigger."), &[
            "E|Daily change of the 18-day ADX > 1 (\"ADX burst\"); direction by 3-day SMA vs 12-day SMA (p.113)", "X|\"a simple 20-day exit strategy\" with $5,000 stop (p.113)"],
            "Printed pp.111-116, Tables 4.12-4.13."),
        d("ch_bta2_65sma_volstop", "65sma-3cc with volatility-based stop", "Ch 4 · 65sma-3cc system", "Long/Short", NotTestable(
            "The book tests multiples 0.25-3.0 of the 15-day SMA of true range on bond/hogs/sugar charts and names no single multiple or the bar the stop is measured on."), &[
            "X|\"an initial money management stop as a multiple of the 15-day SMA of the daily true range\" (p.91)"], "Printed pp.91-93, Figures 4.13-4.15 only."),
        d("ch_bta2_bottom_fish_case2", "Bottom-fishing, case 2 exit", "Ch 4 · S&P bottom-fishing", "Long", NotTestable(
            "\"a trailing stop on the 5-day low after a $1,000 profit\" does not say whether the profit is measured on the close or intraday, nor whether the 5-day low includes today."), &[
            "X|\"use a trailing stop on the 5-day low after a $1,000 profit on the trade (case 2)\" (p.137)"], "Printed p.137: X=4,Y=3, 2/12/88-7/10/95: $59,025, 44 trades, 45% won."),
        d("ch_bta2_cbo_dmi50", "20-day channel breakout with DMI > 50 filter", "Ch 6 · Equity curve", "Long/Short", NotTestable(
            "\"the 14-day DMI be greater than ... 50\" does not say which DMI line (DX, +DI, -DI or ADX); entries/exits of the CHBO system in this test are not restated."), &[
            "F|\"The filter merely stipulates that the 14-day DMI be greater than an arbitrarily chosen level such as 50.\" (p.201)"], "Printed pp.200-204, Table 6.1 (DM only)."),
        d("ch_bta2_65sma_two_exits", "65sma-3cc with 10-day and volatility exits", "Ch 6 · Equity curve", "Long/Short", NotTestable(
            "Case 4 combines a 10-day high/low exit with \"the volatility-based exit discussed in chapter 5\" (defined from a 20-day channel the 65sma system does not have); precedence and reentry are not given."), &[
            "X|\"one an exit at the lowest low or highest high of 10 days, and the other the volatility-based exit discussed in chapter 5\" (p.187)"], "Printed pp.187-190, DM equity curves only."),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    Some(match id {
        "ch_bta2_ma_stack" => ("ch_ma_stack", "Multiple short SMAs versus long SMA", "All-average agreement"),
        "ch_bta2_65sma_ravi" => ("ch_reversal", "65sma-3cc reversal", "RAVI filter"),
        "ch_bta2_65sma_ravi_x14" => ("ch_reversal", "65sma-3cc reversal", "RAVI 0.5 + 14-day exit"),
        "ch_bta2_65sma_stop" => ("ch_reversal", "65sma-3cc reversal", "$1,000 stop · Data missing"),
        "ch_bta2_65sma_volstop" => ("ch_reversal", "65sma-3cc reversal", "Volatility stop · Not testable"),
        "ch_bta2_65sma_two_exits" => ("ch_reversal", "65sma-3cc reversal", "Two exits · Not testable"),
        "ch_bta2_tat" => ("ch_tat", "Trend-Anti-trend", "TradeStation code"),
        "ch_bta2_ma1011_trend" => ("ch_sp1011", "S&P 10/11-day SMA crossover", "Trend"),
        "ch_bta2_ma1011_anti" => ("ch_sp1011", "S&P 10/11-day SMA crossover", "Anti-trend"),
        "ch_bta2_bottom_fish" => ("ch_bottom", "Bottom-fishing pattern", "S&P, close entry"),
        "ch_bta2_bottom_fish_generic" => ("ch_bottom", "Bottom-fishing pattern", "Generic, stop entry"),
        "ch_bta2_bottom_fish_case2" => ("ch_bottom", "Bottom-fishing pattern", "Case 2 · Not testable"),
        "ch_bta2_xopp" => ("ch_xopp", "Extraordinary opportunity", "3% band, 20th-day exit"),
        "ch_bta2_xopp_trail" => ("ch_xopp", "Extraordinary opportunity", "1% band, 20-day trail"),
        "ch_bta2_cbo_trail5" => ("ch_cbo", "20-day channel breakout on close", "5-day trailing exit"),
        "ch_bta2_cbo_volexit" => ("ch_cbo", "20-day channel breakout on close", "Volatility exit"),
        "ch_bta2_cbo_inside" => ("ch_cbo", "20-day channel breakout on close", "Inside volatility barrier"),
        "ch_bta2_cbo_ref" => ("ch_cbo", "20-day channel breakout on close", "No exits (reference)"),
        "ch_bta2_cbo_20tick" => ("ch_cbo", "20-day channel breakout on close", "Tick barrier · Data missing"),
        "ch_bta2_cbo_dmi50" => ("ch_cbo", "20-day channel breakout on close", "DMI filter · Not testable"),
        "ch_bta2_adx30" => ("ch_adx", "ADX systems", "ADX crosses 30"),
        "ch_bta2_adx_rising" => ("ch_adx", "ADX systems", "ADX rising (28 days)"),
        "ch_bta2_adx_burst" => ("ch_adx", "ADX systems", "ADX burst · Not testable"),
        "ch_bta2_pullback" => ("ch_pullback", "Pullback system", "One unit"),
        "ch_bta2_pullback_10c" => ("ch_pullback", "Pullback system", "Ten contracts · Data missing"),
        "ch_bta2_long_bomb" => ("ch_long_bomb", "The Long Bomb", "TradeStation code"),
        "ch_bta2_vol2x" => ("ch_vol2x", "Volatility breakout (2x range)", "20th-day exit"),
        "ch_bta2_goldbond_ma" => ("ch_intermarket", "Gold-bond intermarket", "Dual MA · Data missing"),
        "ch_bta2_goldbond_adx" => ("ch_intermarket", "Gold-bond intermarket", "ADX filter · Data missing"),
        "ch_bta2_intermarket_trio" => ("ch_intermarket", "Gold-bond intermarket", "Three markets · Data missing"),
        _ => return None,
    })
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    let stop = |def| p("stop_pts", "Initial stop, index points (0 = off)", def, 0., 50., 1.);
    let tick = || p("tick", "Stop-entry offset (1 point/tick)", 0.01, 0., 5., 0.01);
    let sma_n = || p("sma", "Trend SMA closes", 65., 45., 85., 10.);
    let cc = || p("confirm", "Consecutive closes", 3., 2., 5., 1.);
    let ravi = |d| p("ravi", "RAVI barrier (%)", d, 0., 3., 0.5);
    Some(match id {
        "ch_bta2_ma_stack" => vec![p("first", "First short SMA length", 3., 2., 10., 1.),
            p("count", "Number of consecutive short SMAs", 2., 2., 5., 1.), p("slow", "Long SMA length", 65., 10., 200., 1.)],
        "ch_bta2_65sma_ravi" => vec![sma_n(), cc(), p("fast", "RAVI short SMA", 7., 5., 10., 1.), ravi(1.0)],
        "ch_bta2_65sma_ravi_x14" => vec![sma_n(), cc(), p("fast", "RAVI short SMA", 7., 5., 10., 1.), ravi(0.5),
            p("exit_n", "Exit: close beyond N-day extreme", 14., 5., 30., 1.), stop(0.)],
        "ch_bta2_tat" => vec![p("n", "Breakout days", 25., 10., 40., 5.), p("adx", "ADX length", 18., 10., 30., 2.),
            p("adx_avg", "ADX average length", 18., 10., 30., 2.), stop(0.)],
        "ch_bta2_ma1011_trend" | "ch_bta2_ma1011_anti" => vec![p("fast", "Short SMA", 10., 5., 20., 1.), p("slow", "Long SMA", 11., 6., 30., 1.), stop(4.)],
        "ch_bta2_bottom_fish" => vec![p("x", "Range > X points", 4., 0., 10., 0.5), p("y", "Close - open > Y points", 3., 0., 10., 0.5),
            p("low_n", "N-day low", 20., 10., 40., 5.), p("within", "Low formed within days", 5., 1., 10., 1.), p("hold", "Exit on close of day N", 20., 5., 40., 5.), stop(4.)],
        "ch_bta2_bottom_fish_generic" => vec![p("x", "Range > X points", 0.1, 0., 10., 0.1), p("y", "Close - open > Y points", 0., 0., 10., 0.5),
            p("low_n", "N-day low", 20., 10., 40., 5.), p("within", "Low formed within days", 5., 1., 10., 1.), p("exit_n", "Exit at N-day low", 20., 5., 40., 5.), tick(), stop(0.)],
        "ch_bta2_xopp" => vec![p("band", "Band around 50-day SMA (%)", 3., 0.5, 5., 0.5), p("slow", "Slow SMA", 50., 20., 100., 10.), p("fast", "Fast SMA", 7., 3., 15., 1.),
            p("chan", "Entry channel days", 20., 10., 40., 5.), p("hold", "Exit on close of day N", 20., 5., 40., 5.), tick(), stop(0.)],
        "ch_bta2_xopp_trail" => vec![p("band", "Band around 50-day SMA (%)", 1., 0.5, 5., 0.5), p("slow", "Slow SMA", 50., 20., 100., 10.), p("fast", "Fast SMA", 7., 3., 15., 1.),
            p("chan", "Entry channel days", 20., 10., 40., 5.), p("trail_n", "Trailing stop days", 20., 5., 40., 5.), tick(), stop(0.)],
        "ch_bta2_cbo_trail5" => vec![p("chan", "Breakout days", 20., 10., 40., 5.), p("trail_n", "Trailing stop days", 5., 2., 20., 1.), stop(0.)],
        "ch_bta2_cbo_volexit" => vec![p("chan", "Breakout days", 20., 10., 40., 5.), p("vol_n", "Range SMA days", 10., 5., 20., 1.), p("mult", "Range multiple", 3., 1., 5., 0.5), stop(0.)],
        "ch_bta2_cbo_inside" => vec![p("chan", "Breakout days", 20., 10., 40., 5.), p("vol_n", "Range SMA days", 10., 5., 20., 1.), p("mult", "Exit range multiple", 3., 1., 5., 0.5), stop(0.)],
        "ch_bta2_cbo_ref" => vec![p("chan", "Breakout days", 20., 10., 40., 5.), stop(0.)],
        "ch_bta2_adx30" => vec![p("adx", "ADX length", 14., 7., 28., 1.), p("level", "ADX level", 30., 15., 45., 5.), p("fast", "Close SMA", 3., 2., 10., 1.),
            p("ema", "Close EMA", 20., 10., 50., 5.), p("trail_n", "Trailing stop days", 20., 5., 40., 5.), stop(0.)],
        "ch_bta2_adx_rising" => vec![p("adx", "ADX length", 14., 7., 28., 1.), p("lookback", "Rising vs N days ago", 28., 5., 40., 1.), p("fast", "Close SMA", 3., 2., 10., 1.),
            p("ema", "Close EMA", 20., 10., 50., 5.), p("trail_n", "Trailing stop days", 20., 5., 40., 5.), stop(0.)],
        "ch_bta2_pullback" => vec![p("n", "New N-day extreme", 5., 2., 20., 1.), p("sma", "Trend SMA", 50., 20., 100., 10.), p("trail_n", "Trailing stop days", 20., 5., 40., 5.), stop(0.)],
        "ch_bta2_long_bomb" => vec![p("dx", "Exit at N-day low", 50., 10., 100., 5.), tick(), stop(0.)],
        "ch_bta2_vol2x" => vec![p("mult", "Range multiple", 2., 0.5, 4., 0.5), p("hold", "Exit on close of day N", 20., 5., 40., 5.), stop(0.)],
        _ => return None,
    })
}

/// Only these exact contracts have been checked by the independent source-based oracle.
/// Legacy cash-point stops cannot be enabled through a parameter left in an old request.
pub fn validate_reviewed(id: &str, p: &Params) -> Option<String> {
    if !matches!(id, "ch_bta2_ma_stack" | "ch_bta2_65sma_ravi") { return None; }
    if p.get("stop_pts").is_some_and(|v| !v.is_finite() || *v != 0.) {
        return Some("The reviewed Chande contract has no initial stop. Futures dollar stops cannot be supplied as cash/index points; actual contract data and a separate review are required.".into());
    }
    let schema = params(id).unwrap();
    for d in &schema {
        if let Some(v) = p.get(d.key) {
            if !v.is_finite() || *v < d.min || *v > d.max || (d.key != "ravi" && v.fract() != 0.) {
                return Some(format!("Invalid reviewed Chande parameter {}: use a finite value in [{}, {}]{}.", d.key, d.min, d.max, if d.key == "ravi" { "" } else { " and an integer lookback" }));
            }
        }
    }
    if id == "ch_bta2_ma_stack" && getp(&schema, p, "first") + getp(&schema, p, "count") - 1. >= getp(&schema, p, "slow") {
        return Some("Each short SMA must be shorter than the long SMA.".into());
    }
    None
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    let schema = params(id)?;
    if from > to || to >= x.r.len() || validate_reviewed(id, p).is_some() { return None; }
    let g = |k: &str| getp(&schema, p, k);
    let u = |k: &str| (g(k).round().max(1.)) as usize;
    let nan = f64::NAN;
    let stop_pts = g("stop_pts").max(0.);
    let tick = if schema.iter().any(|d| d.key == "tick") { g("tick").max(0.) } else { 0. };
    let one = |o: Option<(f64, Kind)>| [o, None];
    let n = x.r.len();
    macro_rules! sys {
        ($e:expr, $c:expr, $o:expr, $t:expr, $w:expr) => {
            run_sys(x, from, to, &Sys { entry: $e, exit_close: $c, exit_open: $o, trail: $t, stop_pts, oneway: $w })
        };
    }
    let hold_exit = |h: usize| move |i: usize, q: &Pos| (i >= q.i + h).then(|| format!("Close of day {h} in the trade"));
    let rng: Vec<f64> = (0..n).map(|i| x.h[i] - x.l[i]).collect();
    Some(match id {
        "ch_bta2_ma_stack" => {
            let slow = sma(&x.c, u("slow"));
            let short: Vec<_> = (u("first")..u("first") + u("count")).map(|k| sma(&x.c, k)).collect();
            let mut out = vec![];
            let mut pos: Option<Pos> = None;
            // These are on-close orders, so the final bar can also fill. A mixed
            // set of averages holds the position; it is not an exit condition.
            for i in from..=to {
                let dir = if short.iter().all(|v| v[i] > slow[i]) { 1. }
                    else if short.iter().all(|v| v[i] < slow[i]) { -1. } else { 0. };
                if dir == 0. || pos.as_ref().is_some_and(|q| q.dir == dir) { continue; }
                if let Some(q) = pos.take() { out.push(done(q, i, x.c[i], "Opposing SMA agreement · close reversal", i)); }
                pos = Some(Pos { dir, i, px: x.c[i], kind: "close" });
            }
            if let Some(q) = pos {
                let mut t = done(q, to, x.c[to], "open", to);
                t.open = true;
                out.push(t);
            }
            out
        }
        "ch_bta2_65sma_ravi" | "ch_bta2_65sma_ravi_x14" => {
            let (ma, fast) = (sma(&x.c, u("sma")), sma(&x.c, u("fast")));
            let (k, bar) = (u("confirm"), g("ravi"));
            let mut cnt = vec![0i64; n]; // signed streak of closes strictly above (+) / below (-) the SMA
            for i in 0..n {
                let d = if !ma[i].is_finite() { 0 } else if x.c[i] > ma[i] { 1 } else if x.c[i] < ma[i] { -1 } else { 0 };
                cnt[i] = if d == 0 { 0 } else if i > 0 && cnt[i - 1].signum() == d { cnt[i - 1] + d } else { d };
            }
            let entry = |i: usize, _: f64| {
                let ravi = (100. * (fast[i] - ma[i]) / ma[i]).abs();
                one((cnt[i].unsigned_abs() as usize >= k && ravi > bar).then(|| (cnt[i].signum() as f64, Kind::Open)))
            };
            if id == "ch_bta2_65sma_ravi" { sys!(&entry, &none_exit, &none_exit, &no_trail, true) } else {
                let m = u("exit_n");
                let ex = |i: usize, q: &Pos| {
                    let hit = if q.dir > 0. { x.c[i] < ll1(x, i, m) } else { x.c[i] > hh1(x, i, m) };
                    hit.then(|| format!("Close beyond the {m}-day extreme · next open"))
                };
                sys!(&entry, &none_exit, &ex, &no_trail, true)
            }
        }
        "ch_bta2_tat" => {
            let nb = u("n");
            let adx = dmi(x.r, u("adx")).1;
            let avg = sma(&adx, u("adx_avg"));
            let entry = |i: usize, cur: f64| {
                let (nh, nl) = (x.h[i] > hh1(x, i, nb), x.l[i] < ll1(x, i, nb));
                let (weak, strong) = (adx[i] < avg[i], adx[i] > avg[i]);
                one(if cur == 0. {
                    if nh && nl { None } else if nh && weak { Some((-1., Kind::Open)) } else if nl && weak { Some((1., Kind::Open)) } else { None }
                } else if cur > 0. {
                    ((nh && weak) || (nl && strong)).then_some((-1., Kind::Open))
                } else {
                    ((nl && weak) || (nh && strong)).then_some((1., Kind::Open))
                })
            };
            sys!(&entry, &none_exit, &none_exit, &no_trail, false)
        }
        "ch_bta2_ma1011_trend" | "ch_bta2_ma1011_anti" => {
            let (f, s) = (sma(&x.c, u("fast")), sma(&x.c, u("slow")));
            let anti = id.ends_with("anti");
            let entry = |i: usize, _: f64| {
                if i == 0 { return [None, None]; }
                let up = f[i] > s[i] && f[i - 1] <= s[i - 1];
                let dn = f[i] < s[i] && f[i - 1] >= s[i - 1];
                let d = if up { 1. } else if dn { -1. } else { return [None, None] };
                one(Some((if anti { -d } else { d }, Kind::Close)))
            };
            sys!(&entry, &none_exit, &none_exit, &no_trail, false)
        }
        "ch_bta2_bottom_fish" | "ch_bta2_bottom_fish_generic" => {
            let (xx, yy, ln, w) = (g("x"), g("y"), u("low_n"), u("within"));
            let generic = id.ends_with("generic");
            let entry = |i: usize, _: f64| {
                if i + 1 < ln { return [None, None]; }
                let lo = ll(x, i, ln);
                let age = (0..ln).find(|&a| x.l[i - a] == lo).unwrap_or(ln);
                let ok = age < w && x.h[i] - x.l[i] > xx && x.c[i] - x.o[i] > yy;
                one(ok.then(|| (1., if generic { Kind::Stop(x.h[i] + tick) } else { Kind::NextClose })))
            };
            if generic {
                let m = u("exit_n");
                let tr = |i: usize, _: &Pos| ll(x, i, m);
                sys!(&entry, &none_exit, &none_exit, &tr, false)
            } else {
                let he = hold_exit(u("hold"));
                sys!(&entry, &he, &none_exit, &no_trail, false)
            }
        }
        "ch_bta2_xopp" | "ch_bta2_xopp_trail" => {
            let (s, f, b, ch) = (sma(&x.c, u("slow")), sma(&x.c, u("fast")), g("band") / 100., u("chan"));
            let entry = |i: usize, _: f64| {
                if f[i] > (1. + b) * s[i] { one(Some((1., Kind::Stop(hh(x, i, ch) + tick)))) }
                else if f[i] < (1. - b) * s[i] { one(Some((-1., Kind::Stop(ll(x, i, ch) - tick)))) }
                else { [None, None] }
            };
            if id == "ch_bta2_xopp" {
                let he = hold_exit(u("hold"));
                sys!(&entry, &he, &none_exit, &no_trail, false)
            } else {
                let m = u("trail_n");
                let tr = |i: usize, q: &Pos| if q.dir > 0. { ll(x, i, m) } else { hh(x, i, m) };
                sys!(&entry, &none_exit, &none_exit, &tr, false)
            }
        }
        "ch_bta2_cbo_trail5" | "ch_bta2_cbo_volexit" | "ch_bta2_cbo_ref" | "ch_bta2_cbo_inside" => {
            let ch = u("chan");
            let has = |k: &str| schema.iter().any(|d| d.key == k);
            let vol = if has("vol_n") { sma(&rng, u("vol_n")) } else { vec![nan; n] };
            let inside = id == "ch_bta2_cbo_inside";
            let entry = |i: usize, _: f64| {
                let (up, dn) = if inside { (hh1(x, i, ch) - vol[i], ll1(x, i, ch) + vol[i]) } else { (hh1(x, i, ch), ll1(x, i, ch)) };
                let (l, s) = (x.c[i] > up, x.c[i] < dn);
                one(if l && !s { Some((1., Kind::Close)) } else if s && !l { Some((-1., Kind::Close)) } else { None })
            };
            let (tn, mult) = (if has("trail_n") { u("trail_n") } else { 0 }, if has("mult") { g("mult") } else { 0. });
            let tr = |i: usize, q: &Pos| {
                if tn > 0 { return if q.dir > 0. { ll(x, i, tn) } else { hh(x, i, tn) }; }
                if mult > 0. { return if q.dir > 0. { hh(x, i, ch) - mult * vol[i] } else { ll(x, i, ch) + mult * vol[i] }; }
                nan
            };
            sys!(&entry, &none_exit, &none_exit, &tr, false)
        }
        "ch_bta2_adx30" | "ch_bta2_adx_rising" => {
            let adx = dmi(x.r, u("adx")).1;
            let (f, e, m) = (sma(&x.c, u("fast")), ema(&x.c, u("ema")), u("trail_n"));
            let rising = id == "ch_bta2_adx_rising";
            let (lvl, lb) = (if rising { 0. } else { g("level") }, if rising { u("lookback") } else { 0 });
            let entry = |i: usize, _: f64| {
                if i < lb.max(1) { return [None, None]; }
                let trig = if rising { adx[i] > adx[i - lb] } else { adx[i - 1] < lvl && adx[i] > lvl };
                if !trig { return [None, None]; }
                one(if f[i] > e[i] { Some((1., Kind::Open)) } else if f[i] < e[i] { Some((-1., Kind::Open)) } else { None })
            };
            let tr = |i: usize, q: &Pos| if q.dir > 0. { ll(x, i, m) } else { hh(x, i, m) };
            sys!(&entry, &none_exit, &none_exit, &tr, false)
        }
        "ch_bta2_pullback" => {
            let (k, ma, m) = (u("n"), sma(&x.c, u("sma")), u("trail_n"));
            let entry = |i: usize, _: f64| {
                let l = x.l[i] < ll1(x, i, k) && x.l[i] > ma[i];
                let s = x.h[i] > hh1(x, i, k) && x.h[i] < ma[i];
                one(if l && !s { Some((1., Kind::Open)) } else if s && !l { Some((-1., Kind::Open)) } else { None })
            };
            let tr = |i: usize, q: &Pos| if q.dir > 0. { ll(x, i, m) } else { hh(x, i, m) };
            sys!(&entry, &none_exit, &none_exit, &tr, false)
        }
        "ch_bta2_long_bomb" => {
            let dx = u("dx");
            let entry = |i: usize, _: f64| {
                let ok = i >= 4 && x.c[i] > x.h[i - 2] && x.c[i - 1] > x.h[i - 3] && x.c[i - 2] > x.h[i - 4];
                one(ok.then(|| (1., Kind::Stop(x.h[i] + tick))))
            };
            let tr = |i: usize, _: &Pos| ll1(x, i, dx) - tick;
            sys!(&entry, &none_exit, &none_exit, &tr, false)
        }
        "ch_bta2_vol2x" => {
            let mult = g("mult");
            let entry = |i: usize, _: f64| {
                let r = x.h[i] - x.l[i];
                [Some((1., Kind::Stop(x.h[i] + mult * r))), Some((-1., Kind::Stop(x.l[i] - mult * r)))]
            };
            let he = hold_exit(u("hold"));
            sys!(&entry, &he, &none_exit, &no_trail, false)
        }
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;
    fn rows(c: &[f64]) -> Vec<Row> {
        c.iter().enumerate().map(|(i, &v)| Row { ticker: "T".into(), date: format!("2000-01-{:02}", i % 28 + 1), open: v, high: v + 1., low: v - 1., close: v, volume: None }).collect()
    }
    #[test]
    fn reviewed_stack_holds_mixed_agreement_and_reverses_at_final_close() {
        let mut c = vec![100.; 12];
        c.extend([140., 100., 80., 70.]);
        let r = rows(&c);
        let x = Ctx::new(&r, &r, &[]);
        let p = Params::from([("first".into(), 2.), ("count".into(), 2.), ("slow".into(), 10.)]);
        let t = run("ch_bta2_ma_stack", &x, 0, 15, &p).unwrap();
        assert_eq!(t.len(), 2);
        assert_eq!((t[0].legs[0].i, t[0].exit_i, t[0].exit_px), (12, 15, 70.));
        assert_eq!((t[1].dir, t[1].legs[0].i, t[1].open), (-1., 15, true));
        assert_eq!(t[1].entry_kind, "close");
        assert!(run("ch_bta2_ma_stack", &x, 0, 8, &p).unwrap().is_empty());
    }
    #[test]
    fn reviewed_equality_is_neutral_and_cash_stops_are_rejected() {
        let r = rows(&vec![100.; 90]);
        let x = Ctx::new(&r, &r, &[]);
        for id in ["ch_bta2_ma_stack", "ch_bta2_65sma_ravi"] {
            assert!(run(id, &x, 0, 89, &Params::new()).unwrap().is_empty());
            assert!(run(id, &x, 0, 89, &Params::from([("stop_pts".into(), 2.)])).is_none());
            assert!(run(id, &x, 0, 89, &Params::from([("stop_pts".into(), f64::NAN)])).is_none());
        }
        assert!(validate_reviewed("ch_bta2_ma_stack", &Params::from([("count".into(), 2.5)])).is_some());
    }
    #[test]
    fn reviewed_ravi_fills_only_the_next_observed_open_and_does_not_use_prestart_orders() {
        let mut c = vec![100.; 74];
        c[70] = 110.; c[71] = 120.; c[72] = 130.; c[73] = 140.;
        let mut r = rows(&c);
        r[73].open = 135.; r[73].low = 134.; r[73].high = 141.;
        let x = Ctx::new(&r, &r, &[]);
        assert!(run("ch_bta2_65sma_ravi", &x, 0, 72, &Params::new()).unwrap().is_empty());
        let t = run("ch_bta2_65sma_ravi", &x, 0, 73, &Params::new()).unwrap();
        assert_eq!(t.len(), 1);
        assert_eq!((t[0].legs[0].i, t[0].legs[0].px, t[0].open), (73, 135., true));
        assert_eq!(t[0].entry_kind, "open");
        assert!(run("ch_bta2_65sma_ravi", &x, 73, 73, &Params::new()).unwrap().is_empty());
    }
    #[test]
    fn reviewed_ravi_barrier_is_strict_at_equality_and_on_each_side() {
        let mut c = vec![100.; 74];
        c[70] = 102.; c[71] = 103.; c[72] = 104.; c[73] = 105.;
        let r = rows(&c); let x = Ctx::new(&r, &r, &[]);
        let fast=sma(&x.c,7); let slow=sma(&x.c,65);
        let level=(100.*(fast[72]-slow[72])/slow[72]).abs();
        for (barrier, count) in [(level,0), (level+1e-9,0), (level-1e-9,1)] {
            let p=Params::from([("ravi".into(),barrier)]);
            assert_eq!(run("ch_bta2_65sma_ravi",&x,0,73,&p).unwrap().len(),count);
        }
    }
    #[test]
    fn long_bomb_buys_above_high_and_exits_below_prior_low() {
        // rising closes trigger the pattern on bar 4; buy stop high+0.01 fills on bar 5 at the open (gap) or level
        let mut c: Vec<f64> = vec![100.; 60];
        for i in 52..57 { c[i] = 100. + 3. * (i - 51) as f64; }
        for v in c.iter_mut().skip(57) { *v = 90.; }
        let r = rows(&c);
        let x = Ctx::new(&r, &r, &[]);
        let t = run("ch_bta2_long_bomb", &x, 0, 59, &Params::new()).unwrap();
        assert_eq!(t[0].legs[0].i, 55); // bar 54 is the first with c > h[2], c[1] > h[3], c[2] > h[4]
        assert!((t[0].legs[0].px - (c[54] + 1. + 0.01)).abs() < 1e-9 || t[0].legs[0].px == c[55]);
        assert_eq!(t[0].exit_i, 57); // 90 gaps below lowest low of prior 50 bars (99) - 0.01 -> open fill
        assert_eq!(t[0].exit_px, 90.);
    }
    #[test]
    fn ma1011_anti_buys_the_down_cross_on_the_close_and_stops_at_4_points() {
        let mut c = vec![100.; 30];
        c[20] = 90.; // 10-SMA drops below 11-SMA on bar 20
        for v in c.iter_mut().skip(21) { *v = 80.; }
        let r = rows(&c);
        let x = Ctx::new(&r, &r, &[]);
        let t = run("ch_bta2_ma1011_anti", &x, 0, 29, &Params::new()).unwrap();
        assert_eq!(t[0].dir, 1.);
        assert_eq!((t[0].legs[0].i, t[0].legs[0].px), (20, 90.));
        assert_eq!((t[0].exit_i, t[0].exit_px, t[0].reason.as_str()), (21, 80., "Initial stop")); // gap through 86
    }
}
