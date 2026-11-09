//! Shared core of the ATH engines (standard library only, no crate dependencies, so a verification driver can `#[path]`-include it):
//! the daily bar series as AmiBroker holds it (float32), the reader of the converted `.ad1` files, `ExRem`, the per-symbol
//! `Signals`, the AmiBroker portfolio backtester (`backtest_full`), and the `VariantDef` every ATH engine module exposes.
//!
//! Converted data: `scripts/ath_convert.py` (read only on the team's AmiBroker databases). Verification against the team's trade
//! sheets: `ATH/verify/` (see `ATH/verify/README.md`).
#![allow(dead_code)]
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

// ---------------------------------------------------------------- data

/// One daily bar as AmiBroker stores it (float32 values).
#[derive(Clone, Copy, Debug)]
pub struct Bar {
    pub date: i32,
    pub c: f32,
    pub o: f32,
    pub h: f32,
    pub l: f32,
    pub v: f32,
    pub oi: f32,
    pub aux1: f32,
    pub aux2: f32,
}

pub struct Series {
    pub name: String,
    pub bars: Vec<Bar>,
}

impl Series {
    pub fn close(&self) -> Vec<f32> {
        self.bars.iter().map(|b| b.c).collect()
    }
    pub fn open(&self) -> Vec<f32> {
        self.bars.iter().map(|b| b.o).collect()
    }
    pub fn high(&self) -> Vec<f32> {
        self.bars.iter().map(|b| b.h).collect()
    }
    pub fn low(&self) -> Vec<f32> {
        self.bars.iter().map(|b| b.l).collect()
    }
    pub fn volume(&self) -> Vec<f32> {
        self.bars.iter().map(|b| b.v).collect()
    }
    pub fn openint(&self) -> Vec<f32> {
        self.bars.iter().map(|b| b.oi).collect()
    }
    /// Index of the last bar with `date <= d` (None if the series starts later).
    pub fn at_or_before(&self, d: i32) -> Option<usize> {
        let k = self.bars.partition_point(|b| b.date <= d);
        if k == 0 { None } else { Some(k - 1) }
    }
}

/// File name of a symbol in the converted folder: the symbol percent-encoded (letters, digits and `-_.~` stay) + `.ad1`.
pub fn file_name(name: &str) -> String {
    let mut s = String::new();
    for b in name.bytes() {
        if b.is_ascii_alphanumeric() || b"-_.~".contains(&b) {
            s.push(b as char)
        } else {
            s.push_str(&format!("%{:02X}", b))
        }
    }
    s + ".ad1"
}

pub fn load_series(dir: &Path, name: &str) -> Result<Series, String> {
    let b = fs::read(dir.join(file_name(name))).map_err(|e| format!("{name}: {e}"))?;
    if b.len() < 12 || &b[..8] != b"ATHD1\0\0\0" {
        return Err(format!("{name}: not an ATHD1 file"));
    }
    let n = u32::from_le_bytes([b[8], b[9], b[10], b[11]]) as usize;
    if b.len() != 12 + 36 * n {
        return Err(format!("{name}: bad length"));
    }
    let f = |x: &[u8]| f32::from_le_bytes([x[0], x[1], x[2], x[3]]);
    let bars = (0..n)
        .map(|i| {
            let r = &b[12 + 36 * i..12 + 36 * (i + 1)];
            Bar { date: i32::from_le_bytes([r[0], r[1], r[2], r[3]]), c: f(&r[4..]), o: f(&r[8..]), h: f(&r[12..]), l: f(&r[16..]), v: f(&r[20..]), oi: f(&r[24..]), aux1: f(&r[28..]), aux2: f(&r[32..]) }
        })
        .collect();
    Ok(Series { name: name.to_string(), bars })
}

/// The symbols of a watchlist file (`watchlists/<name>.tls` in a converted folder), in file order.
pub fn read_watchlist(dir: &Path, file: &str) -> Result<Vec<String>, String> {
    let t = fs::read_to_string(dir.join("watchlists").join(file)).map_err(|e| format!("watchlist {file}: {e}"))?;
    Ok(t.lines().map(|l| l.trim().to_string()).filter(|l| !l.is_empty()).collect())
}

/// Load every symbol of `names` that exists in `dir` (symbols without a file or without bars are left out).
pub fn load_many(dir: &Path, names: &[String]) -> Vec<Series> {
    names.iter().filter_map(|n| load_series(dir, n).ok()).filter(|s| !s.bars.is_empty()).collect()
}

// ---------------------------------------------------------------- AmiBroker helpers

/// AmiBroker `Prec(x, 2)` for an equality test: the value in hundredths.
pub fn prec2(x: f32) -> i64 {
    ((x as f64) * 100.0).floor() as i64
}

/// AmiBroker `ExRem(a, b)`: keep the first `a` after a `b`, drop the following `a` until a `b` comes.
pub fn exrem(a: &[bool], b: &[bool]) -> Vec<bool> {
    let mut out = vec![false; a.len()];
    let mut state = false;
    for i in 0..a.len() {
        if a[i] && !state {
            out[i] = true;
            state = true;
        }
        if b[i] {
            state = false;
        }
    }
    out
}

// ---------------------------------------------------------------- signals

/// What AmiBroker's `ApplyStop` does for a symbol: percent stops measured from the entry price. A stop that is hit exits at its
/// level (at the open when the bar gaps through it). Stops start on the bar after the entry unless `activate_immediately`.
#[derive(Clone, Copy, Debug, Default)]
pub struct StopSpec {
    pub loss_pct: f32,
    pub profit_pct: f32,
    /// trailing stop: this percent below the highest high since the entry
    pub trail_pct: f32,
    pub activate_immediately: bool,
}

/// One scale-in order of an open position: on bar `bar` buy `mult` tranches at `price` (outside the open-position limit; the shares are
/// added to the open position at the weighted-average price).
#[derive(Clone, Copy, Debug)]
pub struct ScaleIn {
    pub bar: usize,
    pub price: f32,
    pub mult: f32,
}

/// One partial exit of an open position (`Buy = sigScaleOut` with `SetPositionSize(pct, spsPercentOfPosition)`): on bar `bar` sell `pct` percent
/// of the open shares at `price` (rounded down to whole shares; the rest stays open at the same average price). Added for the profit-booking AFL.
#[derive(Clone, Copy, Debug)]
pub struct ScaleOutOrder {
    pub bar: usize,
    pub price: f32,
    pub pct: f32,
}

/// Signals after `ExRem`, with the price AmiBroker would use on each signal bar.
pub struct Signals {
    pub buy: Vec<bool>,
    pub sell: Vec<bool>,
    pub buy_price: Vec<f32>,
    pub sell_price: Vec<f32>,
    /// `PositionScore` per bar (higher is taken first when signals of one bar compete); None = all equal
    pub score: Option<Vec<f32>>,
    pub stop: Option<StopSpec>,
    /// scale-in orders (AFLs that add tranches to an open position); empty = none
    pub scale: Vec<ScaleIn>,
    /// size of the first buy / one tranche as a fraction of the position size (1.0; 0.2 = one of five tranches)
    pub entry_size: f32,
    /// partial exits of an open position (empty = none)
    pub scale_out: Vec<ScaleOutOrder>,
    /// AllowSameBarExit: a Buy and a Sell on the same bar of a FLAT symbol are one round trip on that bar (entry at `buy_price`, exit at
    /// `sell_price`; used by the intraday AFLs). A sell that closes a position at the start of the bar does not close the position bought
    /// again on the same bar. Default false = a sell on the entry bar is ignored (the long-standing behaviour).
    pub same_bar_exit: bool,
    /// the short side of the AFL (`Short` / `Cover` / `ShortPrice` / `CoverPrice`); None = long only
    pub short_leg: Option<ShortLeg>,
    /// long entries sized as this percent of the previous bar's equity (instead of `pct_of_equity * entry_size`); None = the portfolio's
    pub size_pct: Option<f32>,
    /// `SetOption("MaxOpenLong")` / `("MaxOpenShort")` (any symbol's value counts; 0 = off). When either is set the portfolio-wide
    /// `max_positions` is not applied and the two caps are used instead.
    pub max_long: usize,
    pub max_short: usize,
    /// per-bar size multiple of a long ENTRY on that bar (None = 1.0): a position that the AFL opens with several tranches at once
    /// (`Buy = sigScaleIn` on a flat symbol, sized `po` tranches)
    pub entry_mult: Option<Vec<f32>>,
}

/// The short side of a symbol's signals: `short` enters a short sale at `short_price`, `cover` buys it back at `cover_price`.
#[derive(Clone, Debug)]
pub struct ShortLeg {
    pub short: Vec<bool>,
    pub cover: Vec<bool>,
    pub short_price: Vec<f32>,
    pub cover_price: Vec<f32>,
    /// short entries sized as this percent of the previous bar's equity; None = the portfolio's percent
    pub size_pct: Option<f32>,
}

impl Signals {
    pub fn new(buy: Vec<bool>, sell: Vec<bool>, buy_price: Vec<f32>, sell_price: Vec<f32>) -> Self {
        Signals { buy, sell, buy_price, sell_price, score: None, stop: None, scale: Vec::new(), entry_size: 1.0, scale_out: Vec::new(), same_bar_exit: false, short_leg: None, size_pct: None, max_long: 0, max_short: 0, entry_mult: None }
    }
}

/// One trade of a symbol: bar indexes and the prices of the signal bars.
#[derive(Clone, Copy, Debug)]
pub struct Entry {
    pub buy_i: usize,
    pub sell_i: usize,
    pub buy_price: f32,
    pub sell_price: f32,
}

/// Pair every Buy with the next Sell (a Buy with no later Sell stays open: `sell_i == usize::MAX`).
pub fn trades_of(sg: &Signals) -> Vec<Entry> {
    let mut out = Vec::new();
    let mut open: Option<usize> = None;
    for i in 0..sg.buy.len() {
        if let Some(b) = open {
            if sg.sell[i] && i > b {
                out.push(Entry { buy_i: b, sell_i: i, buy_price: sg.buy_price[b], sell_price: sg.sell_price[i] });
                open = None;
            }
        }
        if open.is_none() && sg.buy[i] {
            open = Some(i);
        }
    }
    if let Some(b) = open {
        out.push(Entry { buy_i: b, sell_i: usize::MAX, buy_price: sg.buy_price[b], sell_price: 0.0 });
    }
    out
}

// ---------------------------------------------------------------- the portfolio (AmiBroker backtester)

/// Backtest settings of the AFL / Analysis window: initial equity, size as % of the previous bar's equity, open positions,
/// commission in percent of the trade value per side, and the first / last bar date of the range.
#[derive(Clone, Copy, Debug)]
pub struct PortfolioParams {
    pub initial_equity: f64,
    pub pct_of_equity: f64,
    /// a fixed amount per position instead of a % of equity (not verified against a team sheet)
    pub fixed_amount: Option<f64>,
    pub max_positions: usize,
    pub commission_pct: f64,
    pub start: i32,
    pub end: i32,
    /// book profit and cash as AmiBroker does (`ab_profit` / `ab_cost`); on only for the variants in `ath_run::AMIBROKER_PROFIT`
    pub amibroker_profit: bool,
    /// on top of `amibroker_profit`, the rest of AmiBroker's arithmetic (6-decimal equity exports of the Tulsi %swing sheets): the
    /// commission rate as a float32, 2-decimal prices (`price_ab`) in the profit, open positions valued at those prices in the equity,
    /// and the position size from the unrounded equity; on only for the variants in `ath_run::AMIBROKER_FULL`
    pub amibroker_full: bool,
    /// size each entry from the equity at that moment (AmiBroker's default, `UsePrevBarEquityForPosSizing` False): cash plus the open
    /// positions at the current close less the commission to close them, so it falls with every entry of the bar; off = the previous
    /// bar's equity; on only for the variants in `ath_run::CURRENT_EQUITY_SIZING`
    pub current_equity_sizing: bool,
    /// in the full set, size with the stored float32 price (174.39999) instead of its 2-decimal / 7-digit value (174.40); on only for the
    /// variants in `ath_run::SIZE_F32_PRICE` (Fund Issue sheet: HAVELLS 2012-01-30 190,133 shares, GREENPLY 2013-04-09 250,135) -- the
    /// MARICO 2020-12-07 case of `ytath_running_pctswing_after_2_pctswing` needs the 2-decimal price, so it stays per variant
    pub size_f32_price: bool,
}

#[derive(Clone, Debug)]
pub struct TradeOut {
    pub sym: usize,
    pub entry_date: i32,
    pub entry_price: f64,
    pub exit_date: i32,
    pub exit_price: f64,
    pub shares: f64,
    pub profit: f64,
    pub cum_profit: f64,
    /// position value / the previous bar's equity at the entry
    pub weight: f64,
    /// closed by an `ApplyStop`-style stop instead of a sell signal
    pub stopped: bool,
    /// still open at the end of the range (marked at the last close)
    pub open: bool,
    /// a short sale (entry_price = the price shorted, exit_price = the cover price; profit already has the right sign)
    pub short: bool,
}

struct Open {
    sym: usize,
    entry_i: usize,
    shares: f64,
    price: f64,
    weight: f64,
    high_water: f32,
    short: bool,
}

/// Market value of an open position at price `px` (a short sale: the margin set aside plus the unrealised gain).
/// A price as AmiBroker takes it in a trade's profit: its 7-significant-digit decimal (251.899994 -> 251.9).
fn price7(x: f64) -> f64 {
    format!("{:.6e}", x).parse().unwrap_or(x)
}

/// AmiBroker's trade profit (6-decimal export of the Tulsi RSI Ranking sheet, 1,620 trades): shares x (exit - entry) on the 7-digit
/// prices, less the commission on the float32 entry and exit values; the result is a float32 (DCHL 2008-01-03: 791 x 251.9 -> 200,
/// -43,376.34375 where the plain formula gives -43,376.3390).
pub fn ab_profit(sh: f64, entry: f64, exit: f64, comm: f64) -> f64 {
    ab_profit_with(sh, entry, exit, comm, price7)
}

/// A price as AmiBroker takes it in the full set (`amibroker_full`): its 2-decimal value when the stored float32 is a 2-decimal price,
/// else (split-adjusted prices such as 59.266666) its 7-significant-digit decimal. Tulsi 'equity nse 500 c&p dead stock yes' (BACKTEST
/// ALL) and the other 6-decimal exports: 3,086 of 3,094 trade profits exact (7 digits only: 3,049).
fn price_ab(x: f64) -> f64 {
    let r = (x * 100.0).round() / 100.0;
    if (x - r).abs() < 1e-4 { r } else { price7(x) }
}

fn ab_profit_with(sh: f64, entry: f64, exit: f64, comm: f64, price: fn(f64) -> f64) -> f64 {
    ab_profit_raw(sh, entry, exit, comm, price) as f32 as f64
}

/// The profit before AmiBroker stores it as a float32: the full set credits this unrounded value to cash at an exit (Fund Issue sheet:
/// 297 of 305 single-exit days exact, the float32 profit 156); the reported profit stays the float32 value.
fn ab_profit_raw(sh: f64, entry: f64, exit: f64, comm: f64, price: fn(f64) -> f64) -> f64 {
    let ev = (sh as f32 * entry as f32) as f64;
    let xv = (sh as f32 * exit as f32) as f64;
    sh * (price(exit) - price(entry)) - (ev + xv) * comm
}

/// AmiBroker's share count for a % of equity entry in the full set: the unrounded previous-bar equity x % / `price_ab`, floored
/// (MARICO 2020-12-07: 31,771,739.84 x 2 % / 400.40 = 1,586.99999 -> 1,586; the float32 equity 31,771,740 / 400.399994 gives 1,587).
pub fn ab_shares(equity: f64, pct: f64, price: f64) -> f64 {
    (equity * pct / 100.0 / price_ab(price)).floor()
}

/// AmiBroker's cash for an entry: the float32 position value plus its commission.
pub fn ab_cost(sh: f64, px: f64, comm: f64) -> f64 {
    let v = ab_value(sh, px);
    v + v * comm
}

fn ab_value(sh: f64, px: f64) -> f64 {
    (sh as f32 * px as f32) as f64
}

fn commission_value(sh: f64, px: f64, comm: f64, amibroker_full: bool) -> f64 {
    if amibroker_full { ab_value(sh, px) * comm } else { sh * px * comm }
}

fn mark(o: &Open, px: f64) -> f64 {
    if o.short { o.shares * (2.0 * o.price - px) } else { o.shares * px }
}

/// AmiBroker's portfolio equity marks open positions at their liquidation value: the entry cost is already in cash, and the
/// estimated commission for closing the position is deducted from its current mark. The full cash-market port uses its 2-decimal
/// price arithmetic for long positions, just as it does when calculating equity for the next bar.
fn liquidation_mark(o: &Open, px: f64, comm: f64, amibroker_full: bool) -> f64 {
    if amibroker_full && !o.short {
        // AmiBroker values a long position as its float32 entry value plus the unrealised profit on 2-decimal prices, less the closing
        // commission on the float32 current value (Fund Issue sheet: the Market Value of the daily log to the paisa, equity before each
        // entry 817 of 823 float32 values; the plain shares x price gave up to Rs 0.9 off)
        let (x, e) = (price_ab(px), price_ab(o.price));
        ab_value(o.shares, o.price) + o.shares * (x - e) - comm * ((o.shares * x) as f32 as f64)
    } else if o.short {
        mark(o, px) - o.shares * px * comm
    } else {
        mark(o, px) * (1.0 - comm)
    }
}

/// Largest whole-share entry affordable under the same cost arithmetic used to debit cash.
/// Computing this with `price * (1 + comm)` while debiting float32 share-value arithmetic can admit or reject a boundary share.
fn affordable_shares(cash: f64, px: f64, comm: f64, amibroker_profit: bool) -> f64 {
    if !cash.is_finite() || !px.is_finite() || cash <= 0.0 || px <= 0.0 {
        return 0.0;
    }
    let cost = |shares: f64| {
        if amibroker_profit {
            ab_cost(shares, px, comm)
        } else {
            shares * px * (1.0 + comm)
        }
    };
    let mut shares = (cash / (px * (1.0 + comm))).floor();
    while shares > 0.0 && cost(shares) > cash {
        shares -= 1.0;
    }
    while cost(shares + 1.0) <= cash {
        shares += 1.0;
    }
    shares
}

/// AmiBroker writes DetailLog numbers with C's `%.7g` (7 significant digits, trailing zeros dropped, exponent form from 1e7).
fn g7(x: f64) -> String {
    if !x.is_finite() { return format!("{x}"); }
    if x == 0.0 { return "0".to_string(); }
    let e = format!("{:.6e}", x);
    let (mant, exp) = e.split_once('e').unwrap();
    let exp: i32 = exp.parse().unwrap();
    if exp < -4 || exp >= 7 {
        let m = if mant.contains('.') { mant.trim_end_matches('0').trim_end_matches('.') } else { mant };
        format!("{m}e{}{:02}", if exp < 0 { '-' } else { '+' }, exp.abs())
    } else {
        let s = format!("{:.*}", (6 - exp).max(0) as usize, x);
        if s.contains('.') { s.trim_end_matches('0').trim_end_matches('.').to_string() } else { s }
    }
}

/// Day summary values: always two decimals.
fn f2(x: f64) -> String { format!("{:.2}", x) }

/// Detail logs show AmiBroker's ranking magnitude. A Null score in this AFL is logged as 1.
fn detail_rank(score: f32) -> f32 {
    if score.is_finite() { score.abs() } else { 1.0 }
}

/// The equity AmiBroker prints on its Enter / Exit Long lines (Detail Log): cash plus the open positions, those held from earlier bars at
/// this bar's Open (the previous close when the symbol has no bar), those bought on this bar at their entry price, each less the commission to
/// close it (ATHC Entry & %Swing Exit with % Hard SL or ATR Hard SL sheet, 2000-03-08: 15,301,940 = cash + positions at the Open x (1 - 0.65 %)).
fn equity_at_open(syms: &[(&Series, &Signals)], open: &[Open], cash: f64, date: i32, comm: f64, amibroker_full: bool) -> f64 {
    let mut eq = cash;
    for pos in open {
        let (ps, _) = syms[pos.sym];
        let px = if ps.bars[pos.entry_i].date == date {
            pos.price
        } else {
            match ps.bars.binary_search_by(|b| b.date.cmp(&date)) {
                Ok(j) => ps.bars[j].o as f64,
                Err(j) => if j > 0 { ps.bars[j - 1].c as f64 } else { pos.price },
            }
        };
        eq += liquidation_mark(pos, px, comm, amibroker_full);
    }
    eq
}

/// Index of the last bar of `s` strictly before `date` (None if there is none).
fn bar_before(s: &Series, date: i32) -> Option<usize> {
    let k = s.bars.partition_point(|b| b.date < date);
    if k == 0 {
        None
    } else {
        Some(k - 1)
    }
}

/// Everything a portfolio run produces: the trades (open ones last, with `open`), the daily equity curve and the counts.
pub struct Backtest {
    pub trades: Vec<TradeOut>,
    /// every bar date of the range on which any symbol has a bar, and liquidation equity (cash + open positions after estimated close costs)
    pub dates: Vec<i32>,
    pub equity: Vec<f64>,
    pub positions: Vec<usize>,
    pub signals: usize,
    pub accepted: usize,
    pub rejected_capacity: usize,
    pub rejected_cash: usize,
    /// AmiBroker-style daily portfolio detail, populated only for an explicit DetailLog export.
    pub detail_log: Option<Vec<DetailLogRow>>,
}

#[derive(Clone, Debug)]
pub struct DetailLogRow {
    /// The source workbook prints the date once per day, then leaves subsequent date cells blank.
    pub date: Option<i32>,
    pub information: String,
}

/// Run the portfolio the way AmiBroker's backtester does (verified on the team's sheets, `ATH/verify/pctswing/RESULT.md`):
/// * per bar: stops, then sell signals, then entries; entries in the order of `Signals::score` (higher first), then `rank[sym]`
///   (lower first; the Analysis window tests the symbols alphabetically and signals of equal score keep that order);
/// * an entry is `pct_of_equity` of the PREVIOUS bar's equity, where equity = cash + open positions at the previous close less
///   the commission to close them; shares are rounded down; the size shrinks to the cash left (commission included);
/// * a candidate that cannot be funded at all (not even one share) ends the entries of that bar;
/// * commission is a percentage of the trade value on both sides.
pub fn backtest_full(syms: &[(&Series, &Signals)], rank: &[usize], pp: &PortfolioParams) -> Backtest {
    backtest_full_with_detail_log(syms, rank, pp, false)
}

/// Run a portfolio and optionally retain the daily signal/action/account trace for the ATH DetailLog workbook sheet.
pub fn backtest_full_with_detail_log(syms: &[(&Series, &Signals)], rank: &[usize], pp: &PortfolioParams, want_detail_log: bool) -> Backtest {
    // events: (date, sym, bar index, kind): 0 sell, 1 scale-in, 2 buy
    let mut ev: Vec<(i32, usize, usize, u8)> = Vec::new();
    let mut all_dates: Vec<i32> = Vec::new();
    for (k, (s, sg)) in syms.iter().enumerate() {
        for i in 0..s.bars.len() {
            let d = s.bars[i].date;
            if d < pp.start || d > pp.end {
                continue;
            }
            all_dates.push(d);
            if sg.sell[i] {
                ev.push((d, k, i, 0));
            }
            if sg.buy[i] {
                ev.push((d, k, i, 2));
            }
            if let Some(sl) = &sg.short_leg {
                if sl.cover[i] {
                    ev.push((d, k, i, 4)); // cover, ordered with the sells
                }
                if sl.short[i] {
                    ev.push((d, k, i, 5)); // short sale, ordered with the buys
                }
            }
        }
        for (si, sc) in sg.scale.iter().enumerate() {
            let d = s.bars[sc.bar].date;
            if d >= pp.start && d <= pp.end {
                ev.push((d, k, usize::MAX - si, 1)); // the scale-in is looked up by its index (usize::MAX - si)
            }
        }
        for (si, so) in sg.scale_out.iter().enumerate() {
            let d = s.bars[so.bar].date;
            if d >= pp.start && d <= pp.end {
                ev.push((d, k, usize::MAX - si, 3)); // kind 3 = partial exit, ordered with the sells (kind 0)
            }
        }
    }
    all_dates.sort_unstable();
    all_dates.dedup();
    let score = |k: usize, i: usize| -> f32 {
        if i > usize::MAX / 2 {
            return 0.0;
        }
        match &syms[k].1.score {
            // AmiBroker ranks by the absolute value of PositionScore (100 - ROC < 0 when ROC > 100 %: Tulsi Negative ROC sheet,
            // 2017-10-06 BOMDYEING -70.1 before YAARI +8.3)
            Some(v) if v[i].is_finite() => v[i].abs(),
            // In this AFL's initial ROC window AmiBroker reports a Null PositionScore as rank 1.
            Some(_) => 1.0,
            None => 0.0,
        }
    };
    let ord = |kind: u8| -> u8 { match kind { 3 | 4 => 0, 5 => 2, x => x } };
    ev.sort_by(|a, b| {
        a.0.cmp(&b.0).then(ord(a.3).cmp(&ord(b.3))).then_with(|| {
            if a.3 == 2 || a.3 == 5 {
                score(b.1, b.2).partial_cmp(&score(a.1, a.2)).unwrap_or(std::cmp::Ordering::Equal)
            } else {
                std::cmp::Ordering::Equal
            }
        }).then(rank[a.1].cmp(&rank[b.1]))
    });
    // the full AmiBroker set takes the commission rate as AmiBroker stores it (float32: 0.65 -> 0.64999998 %)
    let comm = if pp.amibroker_full { (pp.commission_pct as f32) as f64 / 100.0 } else { pp.commission_pct / 100.0 };
    // profit of a closed long trade and the cash it returns (entry cost back + profit)
    let close_long = |sh: f64, entry: f64, xp: f64| -> (f64, f64) {
        if pp.amibroker_profit {
            let f: fn(f64) -> f64 = if pp.amibroker_full { price_ab } else { price7 };
            let p = ab_profit_with(sh, entry, xp, comm, f);
            let credit = if pp.amibroker_full { ab_profit_raw(sh, entry, xp, comm, f) } else { p };
            (p, ab_cost(sh, entry, comm) + credit)
        } else {
            (sh * (xp - entry) - comm * (sh * entry + sh * xp), sh * xp * (1.0 - comm))
        }
    };
    // cash paid for a long entry (commission included)
    let open_cost = |sh: f64, px: f64| if pp.amibroker_profit { ab_cost(sh, px, comm) } else { sh * px * (1.0 + comm) };
    let max_long = syms.iter().map(|x| x.1.max_long).max().unwrap_or(0);
    let max_short = syms.iter().map(|x| x.1.max_short).max().unwrap_or(0);
    let mut cash = pp.initial_equity;
    let mut open: Vec<Open> = Vec::new();
    let mut out = Backtest { trades: Vec::new(), dates: all_dates.clone(), equity: Vec::with_capacity(all_dates.len()), positions: Vec::with_capacity(all_dates.len()), signals: 0, accepted: 0, rejected_capacity: 0, rejected_cash: 0, detail_log: want_detail_log.then(Vec::new) };
    let mut cum = 0.0;
    let mut sig_in = vec![false; syms.len()]; // per-symbol signal state (long): a buy signalled and not yet sold
    let mut e = 0;
    for &date in &all_dates {
        let mut day_log: Vec<String> = Vec::new();
        if want_detail_log {
            let mut buys: Vec<(usize, usize)> = Vec::new();
            let mut sells: Vec<usize> = Vec::new();
            for (k, (s, sg)) in syms.iter().enumerate() {
                if let Ok(i) = s.bars.binary_search_by(|b| b.date.cmp(&date)) {
                    if sg.buy[i] { buys.push((k, i)); }
                    // AmiBroker's Detail Log lists a Sell signal of the bar when the symbol's signal state is "in" (a buy was signalled and not
                    // yet sold), also if the portfolio never took that buy (ATHC Entry & %Swing Exit with % Hard SL or ATR Hard SL sheet,
                    // 2010-02-02: JSWENERGY=Sell, RENUKA=Sell), but not a Sell of a symbol that had no buy signal (AMBUJACEM, CYIENT 2000-01-04)
                    if sg.sell[i] && sig_in[k] { sells.push(k); }
                }
            }
            buys.sort_by(|a, b| score(b.0, b.1).partial_cmp(&score(a.0, a.1)).unwrap_or(std::cmp::Ordering::Equal).then(rank[a.0].cmp(&rank[b.0])));
            sells.sort_by_key(|k| rank[*k]);
            let entries = buys.iter().map(|&(k, i)| {
                let value = syms[k].1.score.as_ref().and_then(|v| v.get(i)).copied().map(detail_rank).unwrap_or(1.0);
                format!("{}=Buy({}), ", syms[k].0.name, g7(value as f64))
            }).collect::<Vec<_>>().join("");
            let exits = sells.iter().map(|&k| format!("{}=Sell, ", syms[k].0.name)).collect::<Vec<_>>().join("");
            day_log.push(format!("Entry signals(score):{entries}"));
            day_log.push(format!("Exit signals:{exits}"));
        }
        let mut f = e;
        while f < ev.len() && ev[f].0 == date {
            f += 1;
        }
        // equity at the previous bar: cash + open positions at their previous close, less the commission to close them
        let mut equity_prev = cash;
        for o in &open {
            let (s, _) = syms[o.sym];
            let last = bar_before(s, date).map(|j| s.bars[j].c as f64).unwrap_or(o.price);
            equity_prev += liquidation_mark(o, last, comm, pp.amibroker_full);
        }
        // AmiBroker keeps the account equity as float32 (about 1 rupee at 8.7 million): sizing from the rounded value reproduces its share counts
        // (new_buy_entry_daily sheet: SUDARSCHEM 2009-08-03 8,230 shares; all 3,009 trades and shares equal)
        let equity_raw = equity_prev;
        equity_prev = equity_prev as f32 as f64;
        // AmiBroker keeps the account equity as float32 (about 1 rupee at 8.7 million): sizing from the rounded value reproduces its share counts
        // (new_buy_entry_daily sheet: SUDARSCHEM 2009-08-03 8,230 shares; all 3,009 trades and shares equal)
        equity_prev = equity_prev as f32 as f64;
        // the equity AmiBroker prints on the Exit lines of the bar: cash plus every position (also those sold today) at today's Open, less the closing commission
        let bar_open_equity = if want_detail_log { equity_at_open(syms, &open, cash, date, comm, pp.amibroker_full) } else { 0.0 };
        // stops (ApplyStop): checked on every bar a position's symbol has, before the signals of the bar
        let mut p = 0;
        while p < open.len() {
            let (s, sg) = syms[open[p].sym];
            if open[p].short {
                p += 1;
                continue;
            }
            let hit = match sg.stop {
                Some(st) => s.bars.binary_search_by(|b| b.date.cmp(&date)).ok().and_then(|j| {
                    let o = &mut open[p];
                    let b = &s.bars[j];
                    let active = j > o.entry_i || st.activate_immediately;
                    let mut px: Option<f32> = None;
                    if active && j > o.entry_i {
                        let entry = o.price as f32;
                        if st.loss_pct > 0.0 {
                            let level = entry * (1.0 - st.loss_pct / 100.0);
                            if b.l <= level {
                                px = Some(if b.o <= level { b.o } else { level });
                            }
                        }
                        if px.is_none() && st.trail_pct > 0.0 {
                            let level = o.high_water * (1.0 - st.trail_pct / 100.0);
                            if b.l <= level {
                                px = Some(if b.o <= level { b.o } else { level });
                            }
                        }
                        if px.is_none() && st.profit_pct > 0.0 {
                            let level = entry * (1.0 + st.profit_pct / 100.0);
                            if b.h >= level {
                                px = Some(if b.o >= level { b.o } else { level });
                            }
                        }
                    }
                    if b.h > o.high_water {
                        o.high_water = b.h;
                    }
                    px
                }),
                None => None,
            };
            if let Some(px) = hit {
                let o = open.remove(p);
                let (s, _) = syms[o.sym];
                let xp = px as f64;
                let (profit, back) = close_long(o.shares, o.price, xp);
                cash += back;
                cum += profit;
                if want_detail_log {
                    let entry_rank = syms[o.sym].1.score.as_ref().and_then(|v| v.get(o.entry_i)).copied().map(detail_rank).unwrap_or(1.0);
                    let comm_in = o.shares * o.price * comm;
                    let comm_out = o.shares * xp * comm;
                    let pct = if o.shares * o.price > 0.0 { profit / (o.shares * o.price) * 100.0 } else { 0.0 }; // AmiBroker: profit / entry value, commission not added
                    day_log.push(format!("Exit Long, {}, Price: {}, Shares: {}, Commission: {}, (Total comm.: {}), Profit: {} ({:.2} %), Entry rank:{}, Equity: {}, Fx rate: 1", s.name, g7(xp), o.shares, g7(comm_out), g7(comm_in + comm_out), g7(profit), pct, g7(entry_rank as f64), g7(bar_open_equity)));
                }
                out.trades.push(TradeOut { sym: o.sym, entry_date: s.bars[o.entry_i].date, entry_price: o.price, exit_date: date, exit_price: xp, shares: o.shares, profit, cum_profit: cum, weight: o.weight, stopped: true, open: false, short: false });
            } else {
                p += 1;
            }
        }
        if f > e {
            let mut funds_exhausted = false;
            let mut same_bar: Vec<(usize, usize)> = Vec::new(); // (symbol, bar) of entries that also exit on their own bar
            let mut sold_now: Vec<usize> = Vec::new(); // symbols whose position was closed by a sell of this bar (no same-bar round trip on top)
            let mut entries: Vec<(usize, usize, u8, f64, f64)> = Vec::new(); // (symbol, bar, kind 2 buy / 5 short, price, size multiplier)
            // AmiBroker walks its open-position list (entry order) and applies each position's exit / scale-in / partial exit in
            // that order, so a scale-in only has the cash of the exits listed BEFORE it (team log 03-01-2008: scale-ins of SWARAJENG
            // ... CUB, then the ECLERX exit, then EIHOTEL, GODFRYPHLP). Signals of symbols without a position follow; the buys last.
            let mut bar_ev: Vec<(usize, (i32, usize, usize, u8))> = ev[e..f]
                .iter()
                .map(|&x| {
                    let key = if x.3 == 2 || x.3 == 5 { usize::MAX } else { open.iter().position(|o| o.sym == x.1 && o.short == (x.3 == 4)).unwrap_or(usize::MAX - 1) };
                    (key, x)
                })
                .collect();
            bar_ev.sort_by_key(|x| x.0);
            for &(_, (_, k, i, kind)) in &bar_ev {
                let (s, sg) = syms[k];
                if kind == 3 {
                    // partial exit: sell pct % of the open shares (whole shares, rounded down) at the order's price
                    let so = sg.scale_out[usize::MAX - i];
                    if let Some(p) = open.iter().position(|o| o.sym == k && so.bar > o.entry_i) {
                        let o = &mut open[p];
                        let sold = (o.shares * so.pct as f64 / 100.0).floor();
                        if sold >= 1.0 && sold < o.shares {
                            let xp = so.price as f64;
                            let (profit, back) = close_long(sold, o.price, xp);
                            cash += back;
                            cum += profit;
                            let w = o.weight * sold / o.shares;
                            out.trades.push(TradeOut { sym: k, entry_date: s.bars[o.entry_i].date, entry_price: o.price, exit_date: date, exit_price: xp, shares: sold, profit, cum_profit: cum, weight: w, stopped: false, open: false, short: false });
                            o.shares -= sold;
                            o.weight -= w;
                        }
                    }
                    continue;
                }
                if kind == 1 {
                    // scale-in: more shares of an open position at the order's price, added at the weighted-average price
                    let sc = sg.scale[usize::MAX - i];
                    // AmiBroker (backtestRegular) keeps a per-symbol SIGNAL state, apart from the portfolio: a scale-in while that state is
                    // flat is an entry signal (that order's tranches at its price, competing with the bar's buys); while it is "in" (a buy
                    // was signalled, even one the portfolio rejected) the scale-in only adds to an open position. The state starts flat at
                    // the range start. Team sheet "Fixed Percentage 3-6-9-12 50 Open": 01-01-2008 3MINDIA, CERA ...; 09-01-2008
                    // PUNJLLOYD, GESHIP, NCC (AFL first buy skipped by the circuit filter) are entries, while BGRENERGY / ASIANPAINT
                    // 08-01-2008 (buys of 04/07-01 rejected, 50 open) are ignored scale-ins.
                    if !sig_in[k] {
                        sig_in[k] = true;
                        entries.push((k, sc.bar, 2, sc.price as f64, sc.mult as f64));
                        continue;
                    }
                    if let Some(o) = open.iter_mut().find(|o| o.sym == k && sc.bar > o.entry_i) {
                        let price = sc.price as f64;
                        let wanted = match pp.fixed_amount {
                            Some(a) => (a * sg.entry_size as f64 * sc.mult as f64 / price).floor(),
                            None => (equity_prev * pp.pct_of_equity * sg.entry_size as f64 * sc.mult as f64 / 100.0 / price).floor(),
                        };
            let afford = affordable_shares(cash, price, comm, pp.amibroker_profit);
                        let add = wanted.min(afford);
                        if add >= 1.0 {
                            cash -= open_cost(add, price);
                            o.price = (o.shares * o.price + add * price) / (o.shares + add);
                            o.shares += add;
                            o.weight += add * price / equity_prev;
                        }
                    }
                    continue;
                }
                if kind == 0 {
                    sig_in[k] = false;
                    if let Some(p) = open.iter().position(|o| o.sym == k && !o.short && i > o.entry_i) {
                        let o = open.remove(p);
                        let xp = sg.sell_price[i] as f64;
                        let (profit, back) = close_long(o.shares, o.price, xp);
                        cash += back;
                        cum += profit;
                        sold_now.push(k);
                        if want_detail_log {
                            let entry_rank = sg.score.as_ref().and_then(|v| v.get(o.entry_i)).copied().map(detail_rank).unwrap_or(1.0);
                            let comm_in = commission_value(o.shares, o.price, comm, pp.amibroker_full);
                            let comm_out = commission_value(o.shares, xp, comm, pp.amibroker_full);
                            let pct = if o.shares * o.price > 0.0 { profit / (o.shares * o.price) * 100.0 } else { 0.0 }; // AmiBroker: profit / entry value, commission not added
                            day_log.push(format!("Exit Long, {}, Price: {}, (Avg. exit pr. {}), Shares: {}, Commission: {}, (Total comm.: {}), Profit: {} ({:.2} %), Entry rank:{}, Equity: {}, Fx rate: 1", s.name, g7(xp), g7(xp), o.shares, g7(comm_out), g7(comm_in + comm_out), g7(profit), pct, g7(entry_rank as f64), g7(bar_open_equity)));
                        }
                        out.trades.push(TradeOut { sym: k, entry_date: s.bars[o.entry_i].date, entry_price: o.price, exit_date: date, exit_price: xp, shares: o.shares, profit, cum_profit: cum, weight: o.weight, stopped: false, open: false, short: false });
                    }
                } else if kind == 4 {
                    // cover: buy the short back at CoverPrice (the cash returned is the margin plus the gain)
                    let sl = sg.short_leg.as_ref().unwrap();
                    if let Some(p) = open.iter().position(|o| o.sym == k && o.short && i > o.entry_i) {
                        let o = open.remove(p);
                        let xp = sl.cover_price[i] as f64;
                        let profit = o.shares * (o.price - xp) - comm * (o.shares * o.price + o.shares * xp);
                        cash += o.shares * o.price + o.shares * (o.price - xp) - comm * o.shares * xp;
                        cum += profit;
                        out.trades.push(TradeOut { sym: k, entry_date: s.bars[o.entry_i].date, entry_price: o.price, exit_date: date, exit_price: xp, shares: o.shares, profit, cum_profit: cum, weight: o.weight, stopped: false, open: false, short: true });
                    }
                } else {
                    let em = if kind == 5 { 1.0 } else { sg.entry_mult.as_ref().map_or(1.0, |m| m[i] as f64) };
                    let px = if kind == 5 { sg.short_leg.as_ref().unwrap().short_price[i] } else { sg.buy_price[i] } as f64;
                    if kind == 2 {
                        sig_in[k] = true;
                    }
                    entries.push((k, i, kind, px, em));
                }
            }
            // entries of the bar: buys, short sales and scale-ins of flat symbols, by PositionScore (higher first), then the larger order
            // (more tranches first: the team's AmiBroker log of 01-01-2008 enters 4-, 3-, 2-, then 1-tranche signals), then the symbol order
            let order_size = |&(k, _i, kind, _px, mult): &(usize, usize, u8, f64, f64)| if kind == 5 { 1.0 } else { syms[k].1.entry_size as f64 * mult };
            entries.sort_by(|a, b| {
                score(b.0, b.1).partial_cmp(&score(a.0, a.1)).unwrap_or(std::cmp::Ordering::Equal)
                    .then(order_size(b).partial_cmp(&order_size(a)).unwrap_or(std::cmp::Ordering::Equal))
                    .then(rank[a.0].cmp(&rank[b.0]))
            });
            for &(k, i, kind, price, em) in &entries {
                let (s, sg) = syms[k];
                {
                    let is_short = kind == 5;
                    out.signals += 1;
                    if open.iter().any(|o| o.sym == k) {
                        continue;
                    }
                    let full = if max_long > 0 || max_short > 0 {
                        if is_short {
                            max_short > 0 && open.iter().filter(|o| o.short).count() >= max_short
                        } else {
                            max_long > 0 && open.iter().filter(|o| !o.short).count() >= max_long
                        }
                    } else {
                        open.len() >= pp.max_positions
                    };
                    if full {
                        out.rejected_capacity += 1;
                    } else if funds_exhausted {
                        out.rejected_cash += 1;
                    } else {
                        let leg_pct = if is_short { sg.short_leg.as_ref().unwrap().size_pct } else { sg.size_pct };
                        // the equity the size is taken from: the previous bar's, or (current_equity_sizing) the equity now, after this bar's exits and
                        // entries: positions held from earlier bars at this bar's Open, positions bought on this bar at their entry price, each less the
                        // commission to close it (Supertrend long only sheet: 01-01-2020 MARICO 143 ... 02-01-2020 ACC 33 / ADANIPORTS 130 / ANDHRAPAP 169)
                        let (size_eq, size_raw) = if pp.current_equity_sizing {
                            let mut now = cash;
                            for pos in &open {
                                let (ps, _) = syms[pos.sym];
                                let px = if ps.bars[pos.entry_i].date == date {
                                    pos.price
                                } else {
                                    match ps.bars.binary_search_by(|b| b.date.cmp(&date)) {
                                        Ok(j) => ps.bars[j].o as f64,
                                        Err(j) => if j > 0 { ps.bars[j - 1].c as f64 } else { pos.price },
                                    }
                                };
                                now += liquidation_mark(pos, px, comm, pp.amibroker_full);
                            }
                            (now as f32 as f64, now)
                        } else {
                            (equity_prev, equity_raw)
                        };
                        let wanted = match (pp.fixed_amount, leg_pct) {
                            (Some(a), _) => (a * sg.entry_size as f64 * em / price).floor(),
                            (None, Some(x)) => (size_eq * x as f64 / 100.0 / price).floor(),
                            // AmiBroker floors the float32 quotient (ATGL 2018-11-06: 18,601.9999 -> 18,602 shares)
                            (None, None) if pp.amibroker_full && pp.size_f32_price => (size_raw * (pp.pct_of_equity * sg.entry_size as f64 * em) / 100.0 / (price as f32 as f64)).floor(),
                            (None, None) if pp.amibroker_full => ab_shares(size_raw, pp.pct_of_equity * sg.entry_size as f64 * em, price),
                            (None, None) => (((size_eq * pp.pct_of_equity * sg.entry_size as f64 * em / 100.0 / price) as f32) as f64).floor(),
                        };
                        let afford = affordable_shares(cash, price, comm, pp.amibroker_profit);
                        let shares = wanted.min(afford); // AllowPositionShrinking
                        if shares < 1.0 {
                            funds_exhausted = true;
                            out.rejected_cash += 1;
                            if want_detail_log {
                                day_log.push(format!(" {} not entered because of insufficient funds or wrong position size/value (reqEntryPrice: {}, reqEntryPosSize: {}% of equity (value = {}), reqLotSize: 1)", s.name, g7(price), g7(pp.pct_of_equity * sg.entry_size as f64 * em), g7(equity_prev * pp.pct_of_equity * sg.entry_size as f64 * em / 100.0)));
                            }
                        } else {
                            cash -= open_cost(shares, price);
                            open.push(Open { sym: k, entry_i: i, shares, price, weight: shares * price / equity_prev, high_water: s.bars[i].h, short: is_short });
                            out.accepted += 1;
                            if want_detail_log && !is_short {
                                let fee = commission_value(shares, price, comm, pp.amibroker_full);
                                let entry_eq = equity_at_open(syms, &open, cash, date, comm, pp.amibroker_full);
                                let rank_score = sg.score.as_ref().and_then(|v| v.get(i)).copied().map(detail_rank).unwrap_or(1.0);
                                day_log.push(format!("Enter Long, {}, Price: {}, Shares: {}, Commission: {}, Rank: {}, Equity {}, Margin Loan: 0, Fx rate: 1", s.name, g7(price), shares, g7(fee), g7(rank_score as f64), g7(entry_eq)));
                            }
                            // AllowSameBarExit: a sell on the entry bar of a symbol that was flat at the start of the bar closes the new position at once
                            // (the position keeps its slot and its cash until all entries of the bar are done, then it is closed)
                            if !is_short && sg.same_bar_exit && sg.sell[i] && !sold_now.contains(&k) {
                                same_bar.push((k, i));
                            }
                        }
                    }
                }
            }
            for (k, i) in same_bar {
                if let Some(p) = open.iter().position(|o| o.sym == k && o.entry_i == i) {
                    let o = open.remove(p);
                    let sg = syms[k].1;
                    let xp = sg.sell_price[i] as f64;
                    let (profit, back) = close_long(o.shares, o.price, xp);
                    cash += back;
                    cum += profit;
                    out.trades.push(TradeOut { sym: k, entry_date: date, entry_price: o.price, exit_date: date, exit_price: xp, shares: o.shares, profit, cum_profit: cum, weight: o.weight, stopped: false, open: false, short: false });
                }
            }
            e = f;
        }
        // AmiBroker reports liquidation equity after the bar: cash plus open positions at their close, less their estimated
        // closing commission. This is also the equity basis used before entries on the next bar.
        let mut eq = cash;
        for o in &open {
            let (s, _) = syms[o.sym];
            let k = s.bars.partition_point(|b| b.date <= date);
            eq += liquidation_mark(o, if k > 0 { s.bars[k - 1].c as f64 } else { o.price }, comm, pp.amibroker_full);
        }
        out.equity.push(eq);
        out.positions.push(open.len());
        if let Some(log) = out.detail_log.as_mut() {
            let market_value = eq - cash;
            let holdings = if open.is_empty() { ", ".to_string() } else { open.iter().map(|o| format!(", {} ({}{}),", syms[o.sym].0.name, if o.short { "-" } else { "+" }, o.shares)).collect::<Vec<_>>().join(" ") };
            day_log.push(format!("{} Open Positions: {} Market Value: {}, Equity: {}, Cash: {}, Margin: 0.00, Net Cash Balance: {}, ", open.len(), holdings, f2(market_value), f2(eq), f2(cash), f2(cash)));
            for (i, information) in day_log.drain(..).enumerate() {
                log.push(DetailLogRow { date: if i == 0 { Some(date) } else { None }, information });
            }
        }
    }
    // positions still open at the end of the range (dead stock NO): shown as open trades at the last close
    for o in &open {
        let (s, _) = syms[o.sym];
        let k = s.bars.partition_point(|b| b.date <= pp.end);
        let last = if k > 0 { s.bars[k - 1].c as f64 } else { o.price };
        let dir = if o.short { -1.0 } else { 1.0 };
        let profit = dir * o.shares * (last - o.price) - comm * (o.shares * o.price + o.shares * last);
        out.trades.push(TradeOut { sym: o.sym, entry_date: s.bars[o.entry_i].date, entry_price: o.price, exit_date: s.bars[k.saturating_sub(1)].date, exit_price: last, shares: o.shares, profit, cum_profit: cum, weight: o.weight, stopped: false, open: true, short: o.short });
    }
    out.trades.sort_by_key(|t| (t.open, t.exit_date, t.entry_date));
    out
}

/// The trades of `backtest_full` (kept for the verification drivers).
pub fn backtest(syms: &[(&Series, &Signals)], rank: &[usize], pp: &PortfolioParams) -> Vec<TradeOut> {
    backtest_full(syms, rank, pp).trades
}

/// Rank of every symbol in ascending name order (case-insensitive): the order in which AmiBroker's Analysis window tests them.
pub fn alpha_rank(series: &[Series]) -> Vec<usize> {
    let mut order: Vec<usize> = (0..series.len()).collect();
    order.sort_by(|&a, &b| series[a].name.to_uppercase().cmp(&series[b].name.to_uppercase()));
    let mut rank = vec![0usize; series.len()];
    for (r, &i) in order.iter().enumerate() {
        rank[i] = r;
    }
    rank
}

// ---------------------------------------------------------------- the variants an engine module exposes

/// One parameter of a variant (keys as in `frontend/react/src/services/afl-params-ath.ts`).
#[derive(Clone, Copy, Debug)]
pub struct P {
    pub key: &'static str,
    pub label: &'static str,
    pub def: f64,
    pub min: f64,
    pub max: f64,
    pub step: f64,
}
pub const fn p(key: &'static str, label: &'static str, def: f64, min: f64, max: f64, step: f64) -> P {
    P { key, label, def, min, max, step }
}

/// Which converted database and watchlist a variant runs on.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Data {
    /// `C&P DATABASE UPDATED TILL 06-09-2021_WITH T.O` (cash prices, folder `ath-cp-2021-09-06`)
    Cash { watchlist: &'static str },
    /// `NSE FO + ATH UPDATED TILL_26-10-2021` (F&O stocks, folder `ath-foath-2021-10-26`)
    Fo { watchlist: &'static str },
    /// `C&P DATABASE UPDATED TILL 31-03-2022` (folder `ath-cp-2022-03-31`): the database the team's 2022 sheets were made on
    Cash22 { watchlist: &'static str },
    /// `NSE FO + ATH UPDATED TILL_31-03-2022` (folder `ath-foath-2022-03-31`)
    Fo22 { watchlist: &'static str },
}

/// Index series an AFL reads with `Foreign()`, by name (NSENIFTY, NSE500, NSENIFTY1, VIX, ...), from the cash database.
pub struct Env {
    pub index: BTreeMap<String, Series>,
}
impl Env {
    pub fn empty() -> Env {
        Env { index: BTreeMap::new() }
    }
    pub fn get(&self, name: &str) -> Option<&Series> {
        self.index.get(name)
    }
}

/// Parameter reader handed to a variant: `g("exitpct")`; every schema key has its default applied.
pub type G<'a> = &'a dyn Fn(&str) -> f64;

/// A sub-variant the Rust engine runs.
#[derive(Clone, Copy)]
pub struct VariantDef {
    pub id: &'static str,
    pub data: Data,
    /// the AFL's own parameters (the common portfolio parameters equity, sizing, pct, alloc, max_positions are added by the runner)
    pub schema: fn() -> Vec<P>,
    /// signals of the whole universe (one `Signals` per series, same order): per-symbol AFLs map their function over the series;
    /// AFLs with a cross-sectional step (ranking by turnover rank, ...) compute it here
    pub signals: fn(&[Series], &Env, G) -> Vec<Signals>,
}

/// Defaults of the common portfolio parameters every variant gets.
pub fn common_schema() -> Vec<P> {
    vec![
        p("equity", "Equity", 1_000_000.0, 1000.0, 1.0e10, 1.0),
        p("sizing", "FIXED EQUITY / % OF Equity (1 % of equity, compounding; 2 fixed amount)", 1.0, 1.0, 2.0, 1.0),
        p("pct", "POS. SIZE NO OF SHARE / %OF EQUITY (0 = 100 / MAXIMUM POSITION)", 0.0, 0.0, 100.0, 0.01),
        p("alloc", "EQUITY ALLOCATION (fixed amount per position, sizing 2)", 100_000.0, 1.0, 1.0e9, 1.0),
        p("max_positions", "MAXIMUM POSITION", 20.0, 1.0, 500.0, 1.0),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ath_amibroker_liquidation_mark_deducts_the_closing_commission() {
        let shares = 491.0;
        let px = 407.239990234375;
        let comm = (0.65_f32 as f64) / 100.0;
        let pos = Open { sym: 0, entry_i: 0, shares, price: px, weight: 0.02, high_water: 0.0, short: false };
        // marked at the entry price: the float32 entry value, less the commission on the float32 current value
        let entry_value = (shares as f32 * px as f32) as f64;
        let got = liquidation_mark(&pos, px, comm, true);
        assert!((got - (entry_value - comm * ((shares * 407.24) as f32 as f64))).abs() < 1e-8);
    }

    #[test]
    fn ath_amibroker_affordability_uses_the_cash_debit_arithmetic() {
        let px = 19.729999542236328;
        let comm = (0.65_f32 as f64) / 100.0;
        let cost_54093 = ab_cost(54_093.0, px, comm);
        assert_eq!(affordable_shares(cost_54093, px, comm, true), 54_093.0);
        assert_eq!(affordable_shares(cost_54093 - 0.01, px, comm, true), 54_092.0);
    }

    #[test]
    fn ath_detail_log_numbers_are_written_like_amibroker() {
        assert_eq!(g7(407.239990234375), "407.24");
        assert_eq!(g7(9997400.599127533), "9997401");
        assert_eq!(g7(67344780.5), "6.734478e+07");
        assert_eq!(g7(1299.7063351395539), "1299.706");
        assert_eq!(g7(-59043.05859375), "-59043.06");
        assert_eq!(g7(93.33682), "93.33682");
        assert_eq!(g7(0.0), "0");
        assert_eq!(f2(596016.6852330323), "596016.69");
    }

    #[test]
    fn ath_amibroker_detail_rank_uses_absolute_score_and_null_fallback() {
        assert_eq!(detail_rank(-16.231361_f32), 16.231361_f32);
        assert_eq!(detail_rank(f32::NAN), 1.0);
    }

    #[test]
    fn ath_amibroker_detail_log_jan_2000_entry_and_liquidation_values() {
        let prices = [407.239990234375_f32, 97.5_f32, 12.100000381469727_f32];
        let names = ["DRREDDY", "BIRLACABLE", "OSWALAGRO 310801"];
        let scores = [93.33682_f32, 87.47836_f32, f32::NAN];
        let mut series = Vec::new();
        let mut signals = Vec::new();
        for i in 0..3 {
            series.push(Series {
                name: names[i].to_string(),
                bars: vec![
                    Bar { date: 20000103, c: prices[i], o: prices[i], h: prices[i], l: prices[i], v: 1.0, oi: 0.0, aux1: 0.0, aux2: 0.0 },
                    Bar { date: 20000104, c: prices[i], o: prices[i], h: prices[i], l: prices[i], v: 1.0, oi: 0.0, aux1: 0.0, aux2: 0.0 },
                ],
            });
            let mut sg = Signals::new(vec![false, true], vec![false, false], vec![prices[i], prices[i]], vec![prices[i], prices[i]]);
            sg.score = Some(vec![f32::NAN, scores[i]]);
            signals.push(sg);
        }
        let symbols: Vec<_> = series.iter().zip(signals.iter()).collect();
        let pp = PortfolioParams {
            initial_equity: 10_000_000.0, pct_of_equity: 2.0, fixed_amount: None, max_positions: 50,
            commission_pct: 0.65, start: 20000103, end: 20000104, amibroker_profit: true, amibroker_full: true, current_equity_sizing: false, size_f32_price: false,
        };
        let run = backtest_full_with_detail_log(&symbols, &[0, 1, 2], &pp, true);
        let log = run.detail_log.unwrap();
        let start = log.iter().position(|r| r.date == Some(20000104)).unwrap();
        let day: Vec<_> = log.iter().skip(start).map(|r| r.information.as_str()).collect();
        assert!(day.iter().any(|x| x.contains("OSWALAGRO 310801=Buy(1)")));
        assert!(day.iter().any(|x| x.contains("DRREDDY, Price:") && x.contains("Shares: 491")), "{day:?}");
        assert!(day.iter().any(|x| x.contains("BIRLACABLE, Price:") && x.contains("Shares: 2051")), "{day:?}");
        assert!(day.iter().any(|x| x.contains("OSWALAGRO 310801, Price:") && x.contains("Shares: 16528")), "{day:?}");
        let summary = day.iter().find(|x| x.contains("Open Positions:")).unwrap();
        let market = summary.split("Market Value: ").nth(1).unwrap().split(',').next().unwrap().parse::<f64>().unwrap();
        let equity = summary.split("Equity: ").nth(1).unwrap().split(',').next().unwrap().parse::<f64>().unwrap();
        let cash = summary.split("Cash: ").nth(1).unwrap().split(',').next().unwrap().parse::<f64>().unwrap();
        assert!((market - 596_016.69).abs() < 0.01, "market value {market}");
        assert!((equity - 9_992_201.09).abs() < 0.01, "equity {equity}");
        assert!((cash - 9_396_184.40).abs() < 0.01, "cash {cash}");
    }

    #[test]
    fn ath_amibroker_profit_matches_the_6_decimal_export() {
        // Tulsi 'With RSI Ranking - New AFL' sheet, DCHL 220113 2008-01-03: X profit -43376.343750 (plain formula -43376.3390)
        let (ep, xp) = (251.9_f32 as f64, 200.0_f32 as f64);
        assert_eq!(ab_profit(791.0, ep, xp, 0.0065), -43376.34375);
        assert!((791.0 * (xp - ep) - 0.0065 * (791.0 * ep + 791.0 * xp) + 43376.339).abs() < 1e-3);
        // X entry value 199252.890625 = float32(791 x 251.9)
        assert_eq!(ab_cost(791.0, ep, 0.0), 199252.890625);
        assert_eq!(price7(65.400002_f32 as f64), 65.4);
    }

    #[test]
    fn ath_amibroker_full_set_sizes_from_the_unrounded_equity() {
        // Tulsi BACKTEST ALL equity export: MARICO 2020-12-07, X equity prev bar 31,771,740 (float32), AmiBroker 1,586 shares
        assert_eq!(ab_shares(31_771_739.84, 2.0, 400.4_f32 as f64), 1586.0);
        assert_eq!((((31_771_740.0 * 2.0 / 100.0 / (400.4_f32 as f64)) as f32) as f64).floor(), 1587.0);
        assert_eq!(price_ab(77.87999_f32 as f64), 77.88);
        assert_eq!(price_ab(59.266666_f32 as f64), 59.26667);
    }

    #[test]
    fn ath_amibroker_full_set_sizes_with_the_two_decimal_price() {
        // Tulsi 'Fund Issue' sheet (ATHC Entry & %Swing Exit with % Hard SL or ATR Hard SL), JCHAC 2010-02-02: equity 244,969,198.05 x 5 % / 174.40
        // = 70,231.995 -> AmiBroker 70,231 shares; the float32 price 174.39999 gave 70,232
        assert_eq!(ab_shares(244_969_198.0546, 5.0, 174.4_f32 as f64), 70231.0);
        assert_eq!((((244_969_200.0_f64 * 5.0 / 100.0 / (174.4_f32 as f64)) as f32) as f64).floor(), 70232.0);
    }

    #[test]
    fn ath_detail_log_lists_every_sell_prints_score_1_and_the_equity_at_the_open() {
        // A is bought on 2000-01-03 and sold on 2000-01-04; B has a buy the portfolio rejects (1 position max) and a Sell on 2000-01-04; C has a Sell
        // and never a buy; no PositionScore anywhere
        let mk = |name: &str, c: [f32; 2], o: [f32; 2]| Series {
            name: name.to_string(),
            bars: (0..2).map(|i| Bar { date: 20000103 + i as i32, c: c[i], o: o[i], h: c[i].max(o[i]), l: c[i].min(o[i]), v: 1.0, oi: 0.0, aux1: 0.0, aux2: 0.0 }).collect(),
        };
        let a = mk("A", [100.0, 110.0], [100.0, 105.0]);
        let b = mk("B", [50.0, 50.0], [50.0, 50.0]);
        let sa = Signals::new(vec![true, false], vec![false, true], vec![100.0, 110.0], vec![100.0, 110.0]);
        let sb = Signals::new(vec![true, false], vec![false, true], vec![50.0, 50.0], vec![50.0, 50.0]);
        let c = mk("C", [20.0, 20.0], [20.0, 20.0]);
        let sc = Signals::new(vec![false, false], vec![false, true], vec![20.0, 20.0], vec![20.0, 20.0]);
        let symbols = vec![(&a, &sa), (&b, &sb), (&c, &sc)];
        let pp = PortfolioParams {
            initial_equity: 1_000_000.0, pct_of_equity: 10.0, fixed_amount: None, max_positions: 1,
            commission_pct: 0.65, start: 20000103, end: 20000104, amibroker_profit: false, amibroker_full: false, current_equity_sizing: false, size_f32_price: false,
        };
        let run = backtest_full_with_detail_log(&symbols, &[0, 1, 2], &pp, true);
        let log = run.detail_log.unwrap();
        let day2 = log.iter().position(|r| r.date == Some(20000104)).unwrap();
        let day1: Vec<_> = log.iter().take(day2).map(|r| r.information.as_str()).collect();
        let day2: Vec<_> = log.iter().skip(day2).map(|r| r.information.as_str()).collect();
        assert!(day1.iter().any(|x| x.contains("A=Buy(1), B=Buy(1), ")), "{day1:?}");
        assert!(day1.iter().any(|x| x.starts_with("Enter Long, A,") && x.contains("Rank: 1,")), "{day1:?}");
        assert!(day2.iter().any(|x| *x == "Exit signals:A=Sell, B=Sell, "), "{day2:?}");
        // cash 1,000,000 - 1,000 x 100 x 1.0065 = 899,350 plus A at the Open 105 x 1,000 x (1 - 0.0065) = 104,317.5
        assert!(day2.iter().any(|x| x.starts_with("Exit Long, A,") && x.contains("Entry rank:1,") && x.contains("Equity: 1003668,")), "{day2:?}");
    }

    #[test]
    fn ath_amibroker_full_set_values_a_position_as_entry_value_plus_unrealised_profit() {
        // Fund Issue sheet 2000-01-03: SYNDIBANK 34,129 @ 14.65, REGENCERAM 22,675 @ 22.05, CENTBOP 19,607 @ 25.5, all marked at the entry
        // close: Market Value 1,490,202.37 (the plain shares x price x (1 - commission) gives 1,490,202.41)
        let comm = (0.65_f32 as f64) / 100.0;
        let mut mv = 0.0;
        for (sh, px) in [(34129.0, 14.65_f32), (22675.0, 22.05_f32), (19607.0, 25.5_f32)] {
            let o = Open { sym: 0, entry_i: 0, shares: sh, price: px as f64, weight: 0.05, high_water: px, short: false };
            mv += liquidation_mark(&o, px as f64, comm, true);
        }
        assert!((mv - 1_490_202.37).abs() < 0.006, "market value {mv}");
    }

    #[test]
    fn ath_amibroker_full_set_credits_the_unrounded_profit_but_reports_the_float32_one() {
        // Fund Issue sheet 2007-08-02 style: profit 11,111,527.0 is a float32 (step 1); the cash got the unrounded value
        let (sh, e, x, c) = (43396.0, 100.0_f32 as f64, 327.81_f32 as f64, (0.65_f32 as f64) / 100.0);
        let raw = ab_profit_raw(sh, e, x, c, price_ab);
        assert_eq!(ab_profit_with(sh, e, x, c, price_ab), raw as f32 as f64);
        assert!(raw != raw as f32 as f64, "{raw}");
    }

    #[test]
    fn ath_amibroker_float32_price_sizing_gives_the_fund_issue_share_counts() {
        // HAVELLS 2012-01-30: equity 337,448,042.1 x 5 % / 88.74 -> 190,132 with the 2-decimal price, AmiBroker 190,133 (stored float32 price)
        let eq = 337_448_042.1_f64;
        assert_eq!(ab_shares(eq, 5.0, 88.74_f32 as f64), 190132.0);
        assert_eq!((eq * 5.0 / 100.0 / (88.74_f32 as f64)).floor(), 190133.0);
    }
}
