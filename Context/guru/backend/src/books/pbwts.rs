//! George Pruitt & John R. Hill, "Building Winning Trading Systems with TradeStation" (Wiley 2003, Futures Truth).
//! Book id `pbwts`. Source (read in place, copy pending): research drive "USB HDD 2/tRADING KNO-2/George Pruitt,
//! John R.Hill - Building Winning Trading Systems with TradeStation & Code/...ISBN 0471215694.pdf". Transcription,
//! verification and README: library/pbwts/. Page refs are printed pages.
//!
//! Every Live strategy is the book's EasyLanguage code, line for line (code wins over prose; where the Chapter 6
//! listing and the Appendix B TradeStation 2000i listing differ, Chapter 6 is the default and Appendix B is a
//! labelled option). Orders are TradeStation "next bar" orders built at the close: stops/limits fill at their
//! price when the bar crosses it, or at the open when the bar gaps through. Several orders on one bar are filled
//! in the order the bar reaches them using TradeStation's bar path, which the book itself quotes (p.137-138):
//! "When the Open is closer to the Low than to the High, TradeStation assumes that the Low was reached first"
//! (O->L->H->C), else O->H->L->C; and p.71: "TradeStation gives a higher priority to the order that is closest to
//! the market". An exit order only applies to the position held at the close it was placed on; entries in the
//! direction already held are ignored (no pyramiding); an opposite entry closes and reverses.
//! TradeStation functions: Average = SMA; AvgTrueRange(n) = Average(TrueRange, n); StdDev/StandardDev(...,1) =
//! population standard deviation; BollingerBand(P,n,k) = Average(P,n) + k*StdDev(P,n); XAverage = EMA 2/(n+1)
//! seeded with the first price; RSI/ADX = Wilder. The books tested futures (one contract, $ P&L); here the rules
//! run on an index (instrument swap only, % returns, one unit per trade).
use crate::connors::{claim, dmi, rsi, sma, true_range, Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "pbwts";

// ------------------------------------------------------------------ TradeStation-style order simulator

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Kind { Stop, Limit, Open, Close }

/// One "next bar" order. side +1 = buy (Buy / BuyToCover), -1 = sell (Sell / SellShort).
/// exit = BuyToCover/Sell (liquidate only); `from` = TradeStation "from Entry(name)".
#[derive(Clone, Copy, Debug)]
pub struct Order { pub side: f64, pub kind: Kind, pub px: f64, pub exit: bool, pub name: &'static str, pub from: Option<&'static str> }

pub fn entry(side: f64, kind: Kind, px: f64, name: &'static str) -> Order { Order { side, kind, px, exit: false, name, from: None } }
pub fn exit(side: f64, kind: Kind, px: f64, name: &'static str) -> Order { Order { side, kind, px, exit: true, name, from: None } }

#[derive(Clone, Debug)]
pub struct Pos { pub dir: f64, pub px: f64, pub i: usize, pub name: &'static str, pub id: usize, pub kind: &'static str }

/// Strategy state at a bar's close (what EasyLanguage sees).
#[derive(Default)]
pub struct St {
    pub pos: Option<Pos>,
    pub prev_dir: f64,       // position at the previous bar's close
    pub exits_today: usize,  // ExitsToday(date)
    pub entries_today: usize, // EntriesToday(date)
    pub last_dir: f64,       // MarketPosition(1) in EasyLanguage = direction of the previous (closed) position
}
impl St {
    pub fn mp(&self) -> f64 { self.pos.as_ref().map_or(0.0, |p| p.dir) }
    pub fn entry_px(&self) -> f64 { self.pos.as_ref().map_or(f64::NAN, |p| p.px) }
    pub fn bars_since_entry(&self, i: usize) -> usize { self.pos.as_ref().map_or(0, |p| i - p.i) }
}

/// TradeStation bar path: open, nearer extreme, other extreme, close.
fn path(x: &Ctx, i: usize) -> [f64; 4] {
    let (o, h, l, c) = (x.o[i], x.h[i], x.l[i], x.c[i]);
    if o - l < h - o { [o, l, h, c] } else { [o, h, l, c] }
}

/// When (position along the bar path) and at what price an order fills, if it does.
fn trigger(o: &Order, p: &[f64; 4]) -> Option<(f64, f64)> {
    let px = o.px;
    if o.kind == Kind::Open { return Some((0.0, p[0])); }
    if o.kind == Kind::Close { return Some((3.0, p[3])); }
    if !px.is_finite() { return None; }
    // buy stop / sell limit fill on the way up; sell stop / buy limit on the way down
    let up = (o.kind == Kind::Stop) == (o.side > 0.0);
    if (up && p[0] >= px) || (!up && p[0] <= px) { return Some((0.0, p[0])); }
    for k in 0..3 {
        let (a, b) = (p[k], p[k + 1]);
        let hit = if up { a < px && px <= b } else { a > px && px >= b };
        if hit { return Some((k as f64 + (px - a) / (b - a), px)); }
    }
    None
}

fn kind_str(k: Kind) -> &'static str { match k { Kind::Stop => "stop", Kind::Limit => "limit", Kind::Open => "open", Kind::Close => "close" } }

fn close_trade(out: &mut Vec<Trade>, p: Pos, kind: Kind, i: usize, px: f64, why: String) {
    let sig = if kind == Kind::Close { i } else { i.saturating_sub(1) };
    out.push(Trade { dir: p.dir, legs: vec![Leg { i: p.i, px: p.px, w: 1.0 }], exit_i: i, exit_px: px,
        reason: format!("{} → {}", p.name, why), entry_kind: p.kind, signal_i: sig, open: false, exits: vec![] });
}

/// Run a TradeStation-style strategy over bars from..=to. `gen(i, st)` is the strategy code evaluated at bar i's
/// close; it returns the orders for bar i+1 (it may read bar i+1's open, like EasyLanguage "Open of tomorrow").
/// Flat at `from`; earlier bars only warm the indicators.
pub fn sim(x: &Ctx, from: usize, to: usize, mut gen: impl FnMut(usize, &St) -> Vec<Order>) -> Vec<Trade> {
    let mut st = St::default();
    let mut out = Vec::new();
    let mut next_id = 0usize;
    let mut orders: Vec<(Order, Option<usize>)> = if from >= 1 { gen(from - 1, &st).into_iter().map(|o| (o, None)).collect() } else { vec![] };
    for i in from..=to {
        let before = st.mp();
        st.exits_today = 0;
        st.entries_today = 0;
        let p = path(x, i);
        let mut ev: Vec<(f64, usize, f64)> = orders.iter().enumerate().filter_map(|(k, (o, _))| trigger(o, &p).map(|(t, px)| (t, k, px))).collect();
        // same instant: exits first, then the order nearest the market (TradeStation p.71), then code order
        ev.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap().then((!orders[a.1].0.exit).cmp(&!orders[b.1].0.exit)).then(a.1.cmp(&b.1)));
        for (_, k, px) in ev {
            let (o, tid) = orders[k];
            if o.exit {
                let ok = st.pos.as_ref().is_some_and(|q| q.dir == -o.side && (o.kind == Kind::Close || tid == Some(q.id)) && o.from.map_or(true, |f| f == q.name));
                if ok {
                    let q = st.pos.take().unwrap();
                    st.last_dir = q.dir;
                    close_trade(&mut out, q, o.kind, i, px, format!("{} {}", o.name, kind_str(o.kind)));
                    st.exits_today += 1;
                }
            } else {
                if st.mp() == o.side { continue; }
                if let Some(q) = st.pos.take() {
                    st.last_dir = q.dir;
                    close_trade(&mut out, q, o.kind, i, px, format!("reversed by {} {}", o.name, kind_str(o.kind)));
                    st.exits_today += 1;
                }
                st.pos = Some(Pos { dir: o.side, px, i, name: o.name, id: next_id, kind: kind_str(o.kind) });
                next_id += 1;
                st.entries_today += 1;
            }
        }
        st.prev_dir = before;
        if i < to {
            let tid = st.pos.as_ref().map(|q| q.id);
            orders = gen(i, &st).into_iter().filter(|o| o.px.is_finite() || matches!(o.kind, Kind::Open | Kind::Close))
                .map(|o| (o, if o.exit && o.kind != Kind::Close { tid } else { None })).collect();
        }
    }
    if let Some(q) = st.pos.take() {
        out.push(Trade { dir: q.dir, legs: vec![Leg { i: q.i, px: q.px, w: 1.0 }], exit_i: to, exit_px: x.c[to],
            reason: format!("{} → open", q.name), entry_kind: q.kind, signal_i: to, open: true, exits: vec![] });
    }
    out
}

// ------------------------------------------------------------------ TradeStation functions

pub fn ts_avg_tr(x: &Ctx, n: usize) -> Vec<f64> { sma(&true_range(x.r), n) }
pub fn highest(v: &[f64], n: usize) -> Vec<f64> { roll(v, n, f64::max, f64::NEG_INFINITY) }
pub fn lowest(v: &[f64], n: usize) -> Vec<f64> { roll(v, n, f64::min, f64::INFINITY) }
fn roll(v: &[f64], n: usize, f: fn(f64, f64) -> f64, z: f64) -> Vec<f64> {
    // a window holding a missing value (indicator warm-up) has no value, as TradeStation waits for MaxBarsBack
    (0..v.len()).map(|i| if n == 0 || i + 1 < n || v[i + 1 - n..=i].iter().any(|x| x.is_nan()) { f64::NAN } else { v[i + 1 - n..=i].iter().copied().fold(z, f) }).collect()
}
/// Population standard deviation over the last n values (TradeStation StdDev).
pub fn std_pop(v: &[f64], n: usize) -> Vec<f64> { (0..v.len()).map(|i| std_at(v, i, n)).collect() }
pub fn std_at(v: &[f64], i: usize, n: usize) -> f64 {
    if n == 0 || i + 1 < n { return f64::NAN; }
    let w = &v[i + 1 - n..=i];
    let m = w.iter().sum::<f64>() / n as f64;
    (w.iter().map(|a| (a - m) * (a - m)).sum::<f64>() / n as f64).sqrt()
}
pub fn avg_at(v: &[f64], i: usize, n: usize) -> f64 { if n == 0 || i + 1 < n { f64::NAN } else { v[i + 1 - n..=i].iter().sum::<f64>() / n as f64 } }
pub fn xaverage(v: &[f64], n: usize) -> Vec<f64> {
    let a = 2.0 / (n as f64 + 1.0);
    let mut out = vec![f64::NAN; v.len()];
    for i in 0..v.len() { out[i] = if i == 0 { v[0] } else { out[i - 1] + a * (v[i] - out[i - 1]) }; }
    out
}
fn cross_above(a: &[f64], b: &[f64], i: usize) -> bool { i >= 1 && a[i - 1] < b[i - 1] && a[i] > b[i] }
fn cross_below(a: &[f64], b: &[f64], i: usize) -> bool { i >= 1 && a[i - 1] > b[i - 1] && a[i] < b[i] }
fn month_day(d: &str) -> u32 { d[5..7].parse::<u32>().unwrap_or(0) * 100 + d[8..10].parse::<u32>().unwrap_or(0) }

/// Dynamic Break Out II adaptive look-back (p.128): starts at 20, then every bar
/// lookBackDays = Round(lookBackDays * (1 + (StdDev(C,30) - StdDev(C[1],30)) / StdDev(C,30)), 0), bounded floor..ceiling.
pub fn dbs_lookback(c: &[f64], vol_n: usize, init: f64, floor: f64, ceil: f64) -> Vec<f64> {
    let sd = std_pop(c, vol_n);
    let mut lb = init;
    (0..c.len()).map(|i| {
        if i >= 1 && sd[i].is_finite() && sd[i - 1].is_finite() {
            let delta = if sd[i] != 0.0 { (sd[i] - sd[i - 1]) / sd[i] } else { 0.0 };
            lb = (lb * (1.0 + delta)).round().min(ceil).max(floor);
        }
        lb
    }).collect()
}

// ------------------------------------------------------------------ catalogue

const IDX: &str = "Book: one futures contract per market (17 futures, $75 commission+slippage, results in $). Run on an index (instrument swap only); returns are % per trade, one unit per trade.";
const FUT_TABLE: &str = " The book prints per-market futures tables only (no stock index row), so there is nothing on our indices to reproduce.";

macro_rules! live {
    ($id:expr, $name:expr, $group:expr, $dir:expr, $rules:expr, $notes:expr, $claim:expr) => {
        StrategyDef { id: $id, name: $name, group: $group, session: 0, dir: $dir, status: Status::Live, needs_volume: false,
            needs_vix: false, needs_series: &[], book: BOOK, rules: $rules, notes: $notes, claim: $claim }
    };
}
macro_rules! off {
    ($id:expr, $name:expr, $group:expr, $dir:expr, $status:expr, $rules:expr, $notes:expr) => {
        StrategyDef { id: $id, name: $name, group: $group, session: 0, dir: $dir, status: $status, needs_volume: false,
            needs_vix: false, needs_series: &[], book: BOOK, rules: $rules, notes: $notes, claim: None }
    };
}

pub fn catalogue() -> Vec<StrategyDef> {
    use Status::{NeedsData, NotTestable};
    vec![
        // ---------------- Chapter 6
        live!("pbwts_king_keltner", "King Keltner", "Ch 6 · Trading strategies that work", "Long/Short",
            &["F|movAvgVal = Average((High + Low + Close)/3.0, avgLength 40); upBand = movAvgVal + AvgTrueRange(atrLength 40); dnBand = movAvgVal - AvgTrueRange(40) (p.112 pseudocode, App. B p.312)",
              "E|if(movAvgVal > movAvgVal[1]) then Buy (\"KKBuy\") tomorrow at upBand stop (p.112)",
              "E|if(movAvgVal < movAvgVal[1]) then Sell Short(\"KKSell\") tomorrow at dnBand stop (p.112)",
              "X|liquidPoint = movAvgVal; If MarketPosition = 1 then Sell tomorrow at liquidPoint stop; If MarketPosition = -1 then Buy To Cover tomorrow at liquidPoint stop (p.113)"],
            "The Chapter 6 listing prints Average((High + Low + Close),avgLength) without the /3 (a typo: the bands would sit at 3x the price); the book's own pseudocode (p.112) and its Appendix B listing (p.312: average((High + Low + Close)/3.0,avgLength)) both divide by 3, so /3 is the book's rule. Table 6.1 (p.113): 17 futures 1982-3/19/2002, $75 comm/slip, total $593,302.18, 3,478 trades.",
            claim("3,478 (17 futures)", "22.7%-38.9% per market", "$593,302 total", "", "1982-3/19/2002")),
        live!("pbwts_bollinger_bandit", "Bollinger Bandit", "Ch 6 · Trading strategies that work", "Long/Short",
            &["F|upBand = BollingerBand(Close,bollingerLengths 50,1.25); dnBand = BollingerBand(Close,50,-1.25) (p.116)",
              "F|rocCalc = Close - Close[rocCalcLength-1] (rocCalcLength 30) (p.116)",
              "E|if(MarketPosition <> 1 and rocCalc > 0) then Buy(\"BanditBuy\") tomorrow upBand stop (p.116)",
              "E|if(MarketPosition <> -1 and rocCalc < 0) then SellShort(\"BanditSell\") tomorrow dnBand stop (p.116)",
              "F|if MarketPosition = 0 then liqDays = liqLength (50); if MarketPosition <> 0 then liqDays = MaxList(liqDays - 1, 10) (p.116-117)",
              "X|if(MarketPosition = 1 and Average(Close,liqDays) < upBand) then Sell(\"Long Liq\") tomorrow Average(Close,liqDays) stop (p.117)",
              "X|if(MarketPosition = -1 and Average(Close,liqDays) > dnBand) then BuyToCover(\"Short Liq\") tomorrow Average(Close,liqDays) stop (p.117)",
              "F|Option app_b = 1: the Appendix B TradeStation 2000i listing (p.309): no rocCalc filter, and entries only when ExitsToday(date) = 0"],
            "Default = the Chapter 6 listing (the one the text and Table 6.2 describe). Table 6.2 (p.117): 17 futures 1982-3/19/2002, total $537,821.89, 3,092 trades.",
            claim("3,092 (17 futures)", "26.8%-44.8% per market", "$537,822 total", "", "1982-3/19/2002")),
        live!("pbwts_thermostat", "Thermostat", "Ch 6 · Trading strategies that work", "Long/Short",
            &["F|cmiVal = ChoppyMarketIndex(30) = AbsValue(Close[29] - Close) / (Highest(High,30) - Lowest(Low,30)) * 100 (p.66-67, p.122)",
              "F|keyOfDay = (High + Low + Close)/3; if Close > keyOfDay then sellEasierDay = 1; if Close <= keyOfDay then buyEasierDay = 1 (p.122)",
              "F|buyEasierDay: swingBuyPt = Open of tomorrow + 0.50*AvgTrueRange(10), swingSellPt = Open of tomorrow - 0.75*AvgTrueRange(10); sellEasierDay: swingBuyPt = Open + 0.75*ATR, swingSellPt = Open - 0.50*ATR (p.122)",
              "F|swingBuyPt = MaxList(swingBuyPt, Average(Low,3)); swingSellPt = MinList(swingSellPt, Average(High,3)) (p.122)",
              "E|if cmiVal < 20: if MarketPosition <> 1 then Buy(\"SwingBuy\") next bar at swingBuyPt stop; if MarketPosition <> -1 then SellShort(\"SwingSell\") next bar at swingSellPt stop (p.123)",
              "E|else: Buy(\"TrendBuy\") next bar at BollingerBand(Close,50,2) stop; SellShort(\"TrendSell\") next bar at BollingerBand(Close,50,-2) stop (p.123)",
              "X|else: Sell from Entry(\"TrendBuy\") / BuyToCover from Entry(\"TrendSell\") next bar at Average(Close,50) stop (p.123)",
              "X|else: Sell from Entry(\"SwingBuy\") next bar at EntryPrice - 3*AvgTrueRange(10) stop; BuyToCover from Entry(\"SwingSell\") at EntryPrice + 3*AvgTrueRange(10) stop (p.123)"],
            "Code wins over prose: the prose (p.119-120) says 100% of the ATR and compares with yesterday's key; the program uses 0.75 and today's key. Table 6.3 (p.124): 17 futures 1982-3/19/2002, total $761,073.13, 4,020 trades.",
            claim("4,020 (17 futures)", "28.1%-54.6% per market", "$761,073 total", "", "1982-3/19/2002")),
        live!("pbwts_dbs2", "Dynamic Break Out II", "Ch 6 · Trading strategies that work", "Long/Short",
            &["F|lookBackDays starts at 20; each bar: deltaVolatility = (StandardDev(Close,30,1) - StandardDev(Close[1],30,1)) / StandardDev(Close,30,1); lookBackDays = Round(lookBackDays * (1 + deltaVolatility), 0), MinList(.., ceilingAmt 60), MaxList(.., floorAmt 20) (p.128)",
              "F|upBand = BollingerBand(Close,lookBackDays,+2); dnBand = BollingerBand(Close,lookBackDays,-2); buyPoint = Highest(High,lookBackDays); sellPoint = Lowest(Low,lookBackDays) (p.128)",
              "E|if(Close > upBand) then Buy(\"DBS-2 Buy\") tomorrow at buyPoint stop (p.128)",
              "E|if(Close < dnBand) then SellShort(\"DBS-2 Sell\") tomorrow at sellPoint stop (p.128)",
              "X|if(MarketPosition = 1) then Sell(\"LongLiq\") tomorrow at Average(Close,lookBackDays) stop; if(MarketPosition = -1) then BuyToCover(\"ShortLiq\") tomorrow at Average(Close,lookBackDays) stop (p.129)"],
            "The look-back recursion runs from the first bar with two 30-day StdDev values (bar 31 of the data); bounded and rounded, it does not depend on where a test window starts. Table 6.4 (p.129): 17 futures 1982-3/19/2002, total $452,504.10, 1,820 trades.",
            claim("1,820 (17 futures)", "25.6%-51.0% per market", "$452,504 total", "", "1982-3/19/2002")),
        live!("pbwts_dbs2_fade", "Dynamic Break Out II Fade", "Ch 6 · Trading strategies that work", "Long/Short",
            &["F|Same lookBackDays, bands, buyPoint/sellPoint and liquidation points as DBS II (App. B p.311)",
              "E|if(Close > upBand) then Sell(\"DBS-2 Buy\") tomorrow at buyPoint limit (short at the highest high) (p.311)",
              "E|if(Close < dnBand) then Buy(\"DBS-2 Sell\") tomorrow at sellPoint limit (long at the lowest low) (p.311)",
              "X|if(MarketPosition = 1) then ExitLong(\"LongLiq\") tomorrow at longLiqPoint limit; if(MarketPosition = -1) then ExitShort(\"ShortLiq\") tomorrow at shortLiqPoint limit (p.311)"],
            "\"This version of the DBS buys when the original DBS sold and sells when the original DBS bought\" (p.311). Table 6.5 (p.131): soybeans 6/17/82-4/11/02: 128 trades, 64.06% winners, net -$1,681.25, avg trade -$13.13, PF 0.98. We have no soybean data.",
            claim("128 (soybeans)", "64.06%", "-$13.13", "winners 11 / losers 32 bars", "6/17/82-4/11/02")),
        live!("pbwts_seasonal", "Seasonal Soybean System", "Ch 6 · Trading strategies that work", "Long/Short",
            &["F|monthAndDay = Mod(Date of tomorrow, 10000) (p.132)",
              "E|if(monthAndDay >= goLongStart 301 and monthAndDay <= goLongEnd 701) then buy(\"Seasonal Buy\") tomorrow at Open (p.132)",
              "E|if(monthAndDay >= goShortStart 702 or monthAndDay <= goShortEnd 228) then sellShort(\"Seasonal Sell\") tomorrow at Open (p.133)",
              "X|No other exit: each entry reverses the other (always in the market once started)"],
            "Dates \"were derived from cyclical analysis of historical data on soybeans\" (p.132). Table 6.6 (p.132): soybeans 4/26/96-4/12/02: 11 trades, 36.36% winners, net -$4,362.50, avg trade -$396.59. Feb 29 is in neither window (no order that day).",
            claim("11 (soybeans)", "36.36%", "-$396.59", "winners 145 / losers 120 bars", "4/26/96-4/12/02")),
        off!("pbwts_super_combo", "Super Combo Day Trading", "Ch 6 · Trading strategies that work", "Long/Short",
            NeedsData("5-minute intraday S&P bars (data1) with daily bars (data2): entries after the first 30 minutes, before 14:30 CT, liquidation-reversal before 12:00, 3-bar trailing stop after 14:30, exit on the close"),
            &["F|averageRange = Average(Range,10) of Data2; canTrade if AbsValue(Open-Close of Data2) < 0.85*Average(AbsValue(Open-Close),10) (p.143)",
              "E|buyEasierDay (Close of Data2 <= Close[1]): Buy(\"LBreakOut\") at Open + 0.30*averageRange stop, SellShort at Open - 0.60*averageRange stop; sellEasierDay: 0.60 / 0.30 (p.143-145)",
              "E|Failed breakouts: if intraHigh > High of Data2 + 0.25*averageRange then SellShort(\"SfailedBO\") at High of Data2 - 0.25*averageRange stop; mirror for buys (p.145)",
              "X|Protective stop EntryPrice -/+ MaxList(0.25*averageRange, 3.00 points) (0.15 after a liquidation reversal); break-even after 0.50*averageRange; after 14:30 trail Lowest(Low,3)/Highest(High,3) of 5-min bars; SetExitOnClose (p.145-146)"],
            "Tables 6.7/6.8 (p.147-148): S&P 5-min 2/12/2001-4/18/2002: 155 trades, 38.06%, avg $175.32; 1986-2/28/2002 (Excalibur): 2,129 trades, 42.40%, avg $89. Appendix B (p.318) prints a variant with 0.80 and barCount >= 6."),
        live!("pbwts_ghost_trader", "Ghost Trader", "Ch 6 · Trading strategies that work", "Long/Short",
            &["F|Ghost system (every bar, all signals tracked): if myPosition = 0 and Xaverage(Close[1],9) > Xaverage(High[1],19) and RSI(Close[1],9) crosses below 70 and High >= High[1] then myEntryPrice = MaxList(Open,High[1]), myPosition = 1 (p.150)",
              "F|if myPosition = 1 and Low < Lowest(Low[1],20) then myProfit = MinList(Lowest(Low[1],20),Open) - myEntryPrice, myPosition = 0; shorts mirrored with Xaverage(Low[1],19), RSI crosses above 30, Low <= Low[1] (p.150)",
              "E|if marketPosition = 0 and myProfit < 0 and Xaverage(Close,9) > Xaverage(High,19) and RSI(Close,9) crosses below 70 then buy next bar at High stop (p.151)",
              "E|if marketPosition = 0 and myProfit < 0 and Xaverage(Close,9) < Xaverage(Low,19) and RSI(Close,9) crosses above 30 then sellShort next bar at Low stop (p.151)",
              "X|if marketPosition = 1 then sell next bar at Lowest(Low,20) stop; if marketPosition = -1 then buytocover next bar at Highest(High,20) stop (p.152)",
              "F|Option ghost = 0: the core system without the last-trade-was-a-loser filter (the book's first trade list, p.152)"],
            "crosses below = RSI[1] > 70 and RSI < 70 (the book's definition, p.72 and p.314). myProfit starts at 0, so no real trade before the first (ghost) loser. Book trade lists (p.152) are on a currency future (prices 0.9446 ... 0.7861, $125,000 per point: Euro FX) 12/1/2000-5/1/2002: core 7 closed trades, with the filter 4 closed trades (+4,350, -600, -1,500, +7,262.50). No Euro FX data in the vault.",
            claim("4 closed (Euro FX)", "50%", "+$2,378", "", "12/1/2000-5/1/2002")),
        live!("pbwts_money_manager", "Money Manager", "Ch 6 · Trading strategies that work", "Long/Short",
            &["E|Buy(\"MMBuy\") numContracts shares tomorrow at Highest(High,40) stop; SellShort(\"MMSell\") numContracts shares tomorrow at Lowest(Low,40) stop (p.154)",
              "X|if(MarketPosition = 1) then Sell(\"LongLiq\") next bar at Lowest(Low,20) stop; if(MarketPosition = -1) then BuyToCover(\"ShortLiq\") next bar at Highest(High,20) stop (p.154)",
              "F|numContracts = (initCapital 100000 * rskAmt .02) / (StdDev(Close,30) * BigPointValue), rounded down, at least 1 (p.154): NOT modeled"],
            "The contract-count sizing needs a futures BigPointValue and a dollar account; the vault reports % returns of one unit, so only the entry/exit rules are tested (sizing does not change a trade's % return). No results printed."
            , None),
        // ---------------- Chapters 1-4: example programs (also in Appendix B)
        live!("pbwts_donchian40", "Simple program: 40-day channel stop and reverse", "Ch 1 · Fundamentals", "Long/Short",
            &["E|Buy tomorrow at Highest(High,longLength 40) stop (p.11, p.22, App. B p.315)",
              "E|Sell Short tomorrow at Lowest(Low,shortLength 40) stop (p.22)",
              "X|No other exit: the opposite entry reverses"],
            "The book's 'hello world' strategy (no results printed).", None),
        live!("pbwts_adx_ma", "ADX and moving average strategy", "Ch 4 · Analysis techniques", "Long/Short",
            &["F|adxVal = Adx(adxLength 14); trending if adxVal >= 15 (p.71)",
              "E|if adxVal >= 15: if Average(Close,9) crosses above Average(Close,19) then buy tomorrow at High stop; crosses below then sellshort tomorrow at Low stop (p.71)",
              "X|if adxVal < 15: if MarketPosition = 1 then Sell next bar at Lowest(Low,4) stop; if MarketPosition = -1 then BuyToCover next bar at Highest(High,4) stop (p.71)"],
            "crosses above = A[1] < B[1] and A > B (p.72). ADX = Wilder (TradeStation Adx). No results printed. Appendix B MyAdxSys (p.313) is the same code.", None),
        live!("pbwts_ma_channel", "Embedded functions: 9-day average channel", "Ch 4 · Analysis techniques", "Long/Short",
            &["F|value1 = Highest(Average(Close,9),20); value2 = Lowest(Average(Close,9),20) (p.72)",
              "E|Buy tomorrow at value1 stop; Sellshort tomorrow at value2 stop (p.72)",
              "X|No other exit: the opposite entry reverses"],
            "A buy stop below the market fills at the next open (TradeStation stop rule). No results printed. Appendix B MyMovAvgSys (p.315).", None),
        live!("pbwts_percent_r", "Percent R system (Ch 7 debugging example, corrected)", "Ch 7 · Debugging", "Long/Short",
            &["E|if(percentR(14) crosses above 20) then buy next bar at Highest(High,3) stop (p.158)",
              "E|if(percentR(14) crosses below 80) then sellShort next bar at Lowest(Low,3) stop (p.158)",
              "X|longLiqPoint = EntryPrice - 1.5*Range (the printed '+' is the deliberate bug the chapter fixes: \"changing the '+' to a '-'\", p.160); if MarketPosition = 1 then sell next bar at longLiqPoint stop (p.158)",
              "X|shortLiqPoint = EntryPrice + 1.5*Range; if MarketPosition = -1 then buyToCover next bar at shortLiqPoint stop (p.158)"],
            "Range = High - Low of the current bar (recomputed every bar). PercentR(14) = 100*(Close - Lowest(Low,14))/(Highest(High,14) - Lowest(Low,14)). Table 7.1 prints only debug output (Microsoft), no performance.", None),
        live!("pbwts_dow_oc", "Day of week: open to close", "Ch 8 · Research", "Long or Short",
            &["E|if(DayOfWeek(Date of Tomorrow) = whichDay) then buy (buyOrSell 1) / sellShort (buyOrSell 2) next bar at Open (p.177)",
              "X|SetExitOnClose (p.177)",
              "F|Defaults buyOrSell = 1, whichDay = 1 (Monday) as printed; the book batch-runs buyOrSell 1-2 and whichDay 1-5"],
            "Tables 8.4/8.5 (p.178-179): Dow Jones, NASDAQ, S&P 500 and U.S. Bonds futures, $ per weekday, 1982-3/2000 and 4/2000-4/2002. Compare signs on DJI/NDX/SPX.", None),
        live!("pbwts_dow_oo", "Day of week: open to next open", "Ch 8 · Research", "Long or Short",
            &["E|Same entry as the open-to-close test: buy/sellShort at the open of whichDay (p.177)",
              "X|\"the positions were held overnight and liquidated on the next morning's open\" (p.177)"],
            "The code for this second test is not printed; the prose rule is complete. Table 8.6 (p.180) is a misprint (identical to Table 8.4, captioned 'Liquidate on the Close'); Table 8.7 (p.181) is 4/2000-4/2002.", None),
        // ---------------- not testable here
        off!("pbwts_my_rsi", "MyRsiSystem (double dip RSI)", "Ch 2 · Program structure", "Long/Short",
            NeedsData("futures point value: the stop and target are $1,000 / $2,000 per contract (moneyManStop/PointValue/PriceScale)"),
            &["E|RSI(Close,14) crossing above 60 twice (obCount = 2) -> Sell Short tomorrow at open; crossing below 40 twice (osCount = 2) -> Buy tomorrow at open (p.35)",
              "X|longProtStop = entryprice - (1000/PointValue/PriceScale) stop; longProfitStop = entryprice + (2000/PointValue/PriceScale) limit; shorts mirrored (p.35)"], "No results printed."),
        off!("pbwts_mom_rsi", "Momentum, RSI-based strategy with built-in money management", "Ch 4 · Analysis techniques", "Long/Short",
            NeedsData("futures point value: SetStopLoss(3000) and SetPercentTrailing(3000, 25) are $ per contract"),
            &["E|if Momentum(Close,14) > 0 and RSI(Close,14) crosses below 60 then buy next bar at Lowest(Low,3) limit; Momentum < 0 and RSI crosses above 40 then sellshort next bar at Highest(High,3) limit (p.72-73)",
              "X|SetStopLoss(protStop$ 3000); SetPercentTrailing(trailStopThresh$ 3000, trailStopPrcnt 25) (p.73)"], "App. B MyMomRsi (p.314). No results printed."),
        off!("pbwts_trail_pct", "MyTrailPrcntStop", "App B · Source code", "Long",
            NeedsData("futures point value: trailing threshold $3,000 (maxPositionProf*BigPointValue) and SetStopLoss(3000)"),
            &["E|if marketposition <> 1 then buy tomorrow at Highest(high,10) on stop (p.316)",
              "X|after open profit * BigPointValue > 3000: ExitLong at EntryPrice + maxPositionProf*(1 - 25/100) stop; SetStopLoss(3000) (p.316)"], "No results printed."),
        off!("pbwts_cot", "CotTrader (Commitment of Traders)", "Ch 8 · Research", "Long/Short",
            NeedsData("CFTC Commitment of Traders commercial long/short history aligned to report dates (the CD's COT header files)"),
            &["E|three bars after a COT report date: if bullPos > bearPos buy tomorrow at Open; if bullPos < bearPos sell short tomorrow at Open (p.174-175)"],
            "Table 8.3 (p.175): S&P 500, 68 trades, 64.71% winners, avg $4,375.99."),
        off!("pbwts_time_of_day", "Time of day entry study", "Ch 8 · Research", "Long or Short",
            NeedsData("5-minute intraday S&P bars (entry after barDelay*5 minutes, exit on the close)"),
            &["E|buy/sell next bar at open when barCount = barDelay (5-min bars); SetExitOnClose (p.184-185)"], "Table 8.9 (p.185): 292 trades per run."),
        off!("pbwts_intermarket", "Bond vs S&P intermarket system", "Ch 8 · Research", "Long/Short",
            NeedsData("daily U.S. Treasury bond futures closes (data2)"),
            &["E|if Average(Close of data2,8) crosses below Average(Close of data2,24) then buy(\"SPbuyUSdn\") next bar at High stop; crosses above then sellShort next bar at Low stop (p.192)",
              "X|if MarketPosition = 1 then sell next bar at Lowest(Low,10) stop; if -1 then buyToCover next bar at Highest(High,10) stop (p.193)"],
            "Table 8.11 (p.193): S&P 5/3/1996-4/19/2002, 29 trades, 37.93%, avg $1,294.40."),
        off!("pbwts_double_bottom", "Double bottom finite state machine", "Ch 8 · Research", "Long",
            NotTestable("a ShowMe study that only marks double bottoms; the book gives no order, exit or trade rule"),
            &["F|20-day low pivot, 5-day high pivot, second low pivot within one 10-day ATR, then 3 ATRs up, all within 30 bars (p.189-191)"], "Pattern finder only."),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    Some(match id {
        "pbwts_king_keltner" => ("pbwts_channels", "Pruitt channel breakouts", "King Keltner"),
        "pbwts_bollinger_bandit" => ("pbwts_channels", "Pruitt channel breakouts", "Bollinger Bandit"),
        "pbwts_thermostat" => ("pbwts_channels", "Pruitt channel breakouts", "Thermostat"),
        "pbwts_dbs2" => ("pbwts_dbs", "Dynamic Break Out II", "Breakout"),
        "pbwts_dbs2_fade" => ("pbwts_dbs", "Dynamic Break Out II", "Fade (App. B)"),
        "pbwts_seasonal" => ("pbwts_seasonal", "Seasonal soybean dates", ""),
        "pbwts_super_combo" => ("pbwts_daytrade", "Pruitt day trading", "Super Combo"),
        "pbwts_ghost_trader" => ("pbwts_ghost", "Ghost Trader", ""),
        "pbwts_money_manager" => ("pbwts_donchian", "Donchian channel examples", "Money Manager 40/20"),
        "pbwts_donchian40" => ("pbwts_donchian", "Donchian channel examples", "40-day stop and reverse"),
        "pbwts_adx_ma" => ("pbwts_examples", "EasyLanguage example strategies", "ADX + MA cross"),
        "pbwts_ma_channel" => ("pbwts_examples", "EasyLanguage example strategies", "Average channel"),
        "pbwts_percent_r" => ("pbwts_examples", "EasyLanguage example strategies", "Percent R"),
        "pbwts_my_rsi" => ("pbwts_examples", "EasyLanguage example strategies", "MyRsiSystem"),
        "pbwts_mom_rsi" => ("pbwts_examples", "EasyLanguage example strategies", "Momentum RSI"),
        "pbwts_trail_pct" => ("pbwts_examples", "EasyLanguage example strategies", "Percent trailing stop"),
        "pbwts_dow_oc" => ("pbwts_dow", "Day of week", "Open to close"),
        "pbwts_dow_oo" => ("pbwts_dow", "Day of week", "Open to next open"),
        "pbwts_cot" => ("pbwts_research", "Research tools", "COT"),
        "pbwts_time_of_day" => ("pbwts_daytrade", "Pruitt day trading", "Time of day"),
        "pbwts_intermarket" => ("pbwts_research", "Research tools", "Bonds vs S&P"),
        "pbwts_double_bottom" => ("pbwts_research", "Research tools", "Double bottom"),
        _ => return None,
    })
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    Some(match id {
        "pbwts_king_keltner" => vec![p("avg_len", "avgLength (average of H+L+C/3)", 40., 10., 100., 5.), p("atr_len", "atrLength", 40., 10., 100., 5.)],
        "pbwts_bollinger_bandit" => vec![p("bb_len", "bollingerLengths", 50., 20., 100., 5.), p("num_sd", "Band std devs", 1.25, 0.5, 3., 0.25),
            p("roc_len", "rocCalcLength", 30., 10., 60., 5.), p("liq_len", "liqLength", 50., 20., 100., 5.), p("liq_floor", "Minimum liqDays", 10., 5., 30., 5.),
            p("app_b", "Appendix B 2000i listing (1 = no ROC filter, no entry on an exit day)", 0., 0., 1., 1.)],
        "pbwts_thermostat" => vec![p("bb_len", "bollingerLengths", 50., 20., 100., 5.), p("liq_len", "trendLiqLength", 50., 20., 100., 5.),
            p("num_sd", "numStdDevs", 2., 1., 3., 0.25), p("p1", "swingPrcnt1", 0.5, 0.25, 1.5, 0.05), p("p2", "swingPrcnt2", 0.75, 0.25, 1.5, 0.05),
            p("atr_len", "atrLength", 10., 5., 30., 1.), p("switch", "swingTrendSwitch (CMI)", 20., 10., 40., 5.), p("cmi_len", "ChoppyMarketIndex length", 30., 10., 60., 5.),
            p("prot", "Swing protective stop (ATRs)", 3., 1., 5., 0.5)],
        "pbwts_dbs2" | "pbwts_dbs2_fade" => vec![p("ceiling", "ceilingAmt", 60., 30., 100., 5.), p("floor", "floorAmt", 20., 10., 40., 5.),
            p("bb_trig", "bolBandTrig", 2., 1., 3., 0.25), p("vol_len", "Volatility StdDev length", 30., 10., 60., 5.)],
        "pbwts_seasonal" => vec![p("long_start", "goLongStart (MMDD)", 301., 101., 1231., 1.), p("long_end", "goLongEnd", 701., 101., 1231., 1.),
            p("short_start", "goShortStart", 702., 101., 1231., 1.), p("short_end", "goShortEnd", 228., 101., 1231., 1.)],
        "pbwts_ghost_trader" => vec![p("ghost", "Only trade after a (ghost) loser (0 = core system)", 1., 0., 1., 1.), p("fast", "Xaverage(Close) length", 9., 5., 20., 1.),
            p("slow", "Xaverage(High/Low) length", 19., 10., 40., 1.), p("rsi_len", "RSI length", 9., 5., 20., 1.), p("ob", "RSI overbought", 70., 60., 90., 5.),
            p("os", "RSI oversold", 30., 10., 40., 5.), p("exit_len", "Exit channel", 20., 10., 40., 5.)],
        "pbwts_money_manager" => vec![p("entry_len", "Entry channel", 40., 10., 100., 5.), p("exit_len", "Exit channel", 20., 5., 50., 5.)],
        "pbwts_donchian40" => vec![p("long_len", "longLength", 40., 10., 100., 5.), p("short_len", "shortLength", 40., 10., 100., 5.)],
        "pbwts_adx_ma" => vec![p("adx_len", "adxLength", 14., 7., 30., 1.), p("adx_min", "Trending ADX threshold", 15., 10., 30., 1.),
            p("ma1", "mavLength1", 9., 3., 30., 1.), p("ma2", "mavLength2", 19., 10., 60., 1.), p("exit_len", "Non-trending exit channel", 4., 2., 10., 1.)],
        "pbwts_ma_channel" => vec![p("ma", "movAvgLength1", 9., 3., 30., 1.), p("chan", "channelLength", 20., 5., 60., 5.)],
        "pbwts_percent_r" => vec![p("len", "percentR length", 14., 5., 30., 1.), p("lo", "Buy cross level", 20., 5., 40., 5.), p("hi", "Sell cross level", 80., 60., 95., 5.),
            p("chan", "Entry channel", 3., 1., 10., 1.), p("mult", "Range multiple stop", 1.5, 0.5, 3., 0.25)],
        "pbwts_dow_oc" | "pbwts_dow_oo" => vec![p("buy_or_sell", "buyOrSell (1 buy, 2 sell)", 1., 1., 2., 1.), p("which_day", "whichDay (1 Mon .. 5 Fri)", 1., 1., 5., 1.)],
        _ => return None,
    })
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    let schema = params(id)?;
    if from > to || to >= x.r.len() { return None; }
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).map_or(f64::NAN, |d| d.def));
    let u = |k: &str| { let v = g(k).round(); if v.is_finite() && v >= 1.0 { v as usize } else { 1 } };
    let n = x.r.len();
    let (o, h, l, c) = (&x.o, &x.h, &x.l, &x.c);
    const B: f64 = 1.0;
    const S: f64 = -1.0;
    use Kind::*;
    let t = match id {
        "pbwts_king_keltner" => {
            let typ: Vec<f64> = (0..n).map(|i| (h[i] + l[i] + c[i]) / 3.0).collect();
            let (ma, atr) = (sma(&typ, u("avg_len")), ts_avg_tr(x, u("atr_len")));
            sim(x, from, to, |i, st| {
                let mut v = vec![];
                if i >= 1 && ma[i] > ma[i - 1] { v.push(entry(B, Stop, ma[i] + atr[i], "KKBuy")); }
                if i >= 1 && ma[i] < ma[i - 1] { v.push(entry(S, Stop, ma[i] - atr[i], "KKSell")); }
                if st.mp() == B { v.push(exit(S, Stop, ma[i], "liquidPoint")); }
                if st.mp() == S { v.push(exit(B, Stop, ma[i], "liquidPoint")); }
                v
            })
        }
        "pbwts_bollinger_bandit" => {
            let (len, k, roc_n, liq_len, floor, app_b) = (u("bb_len"), g("num_sd"), u("roc_len"), u("liq_len"), u("liq_floor"), g("app_b") >= 0.5);
            let ma = sma(c, len);
            let sd = std_pop(c, len);
            let mut liq_days = liq_len;
            sim(x, from, to, move |i, st| {
                let (up, dn) = (ma[i] + k * sd[i], ma[i] - k * sd[i]);
                let roc = if i + 1 >= roc_n { c[i] - c[i + 1 - roc_n] } else { f64::NAN };
                let mut v = vec![];
                if app_b {
                    if st.mp() != B && st.exits_today == 0 { v.push(entry(B, Stop, up, "BanditBuy")); }
                    if st.mp() != S && st.exits_today == 0 { v.push(entry(S, Stop, dn, "BanditSell")); }
                } else {
                    if st.mp() != B && roc > 0.0 { v.push(entry(B, Stop, up, "BanditBuy")); }
                    if st.mp() != S && roc < 0.0 { v.push(entry(S, Stop, dn, "BanditSell")); }
                }
                if st.mp() == 0.0 { liq_days = liq_len; } else { liq_days = liq_days.saturating_sub(1).max(floor); }
                let liq = avg_at(c, i, liq_days);
                if st.mp() == B && liq < up { v.push(exit(S, Stop, liq, "Long Liq")); }
                if st.mp() == S && liq > dn { v.push(exit(B, Stop, liq, "Short Liq")); }
                v
            })
        }
        "pbwts_thermostat" => {
            let (len, liq_len, k, p1, p2, sw, cmi_n, prot) = (u("bb_len"), u("liq_len"), g("num_sd"), g("p1"), g("p2"), g("switch"), u("cmi_len"), g("prot"));
            let atr = ts_avg_tr(x, u("atr_len"));
            let (lok_b, lok_s) = (sma(l, 3), sma(h, 3));
            let (ma, sd, liq) = (sma(c, len), std_pop(c, len), sma(c, liq_len));
            let (hh, ll) = (highest(h, cmi_n), lowest(l, cmi_n));
            sim(x, from, to, |i, st| {
                let cmi = if i + 1 >= cmi_n {
                    let den = hh[i] - ll[i];
                    if den != 0.0 { (c[i + 1 - cmi_n] - c[i]).abs() / den * 100.0 } else { 0.0 }
                } else { f64::NAN };
                if !cmi.is_finite() || i + 1 >= n { return vec![]; }
                let key = (h[i] + l[i] + c[i]) / 3.0;
                let buy_easier = c[i] <= key;
                let (mut sb, mut ss) = if buy_easier { (o[i + 1] + p1 * atr[i], o[i + 1] - p2 * atr[i]) } else { (o[i + 1] + p2 * atr[i], o[i + 1] - p1 * atr[i]) };
                sb = sb.max(lok_b[i]);
                ss = ss.min(lok_s[i]);
                let mut v = vec![];
                if cmi < sw {
                    if st.mp() != B { v.push(entry(B, Stop, sb, "SwingBuy")); }
                    if st.mp() != S { v.push(entry(S, Stop, ss, "SwingSell")); }
                } else {
                    let stop = prot * atr[i];
                    v.push(entry(B, Stop, ma[i] + k * sd[i], "TrendBuy"));
                    v.push(entry(S, Stop, ma[i] - k * sd[i], "TrendSell"));
                    let ep = st.entry_px();
                    v.push(Order { from: Some("TrendBuy"), ..exit(S, Stop, liq[i], "TrendBuy liq") });
                    v.push(Order { from: Some("TrendSell"), ..exit(B, Stop, liq[i], "TrendSell liq") });
                    v.push(Order { from: Some("SwingBuy"), ..exit(S, Stop, ep - stop, "SwingBuy prot") });
                    v.push(Order { from: Some("SwingSell"), ..exit(B, Stop, ep + stop, "SwingSell prot") });
                }
                v
            })
        }
        "pbwts_dbs2" | "pbwts_dbs2_fade" => {
            let fade = id == "pbwts_dbs2_fade";
            let lb = dbs_lookback(c, u("vol_len"), 20.0, g("floor"), g("ceiling"));
            let k = g("bb_trig");
            sim(x, from, to, |i, st| {
                let m = lb[i] as usize;
                if m == 0 || i + 1 < m { return vec![]; }
                let (avg, sd) = (avg_at(c, i, m), std_at(c, i, m));
                let (up, dn) = (avg + k * sd, avg - k * sd);
                let bp = h[i + 1 - m..=i].iter().copied().fold(f64::NEG_INFINITY, f64::max);
                let sp = l[i + 1 - m..=i].iter().copied().fold(f64::INFINITY, f64::min);
                let mut v = vec![];
                if fade {
                    if c[i] > up { v.push(entry(S, Limit, bp, "DBS-2 Fade Sell")); }
                    if c[i] < dn { v.push(entry(B, Limit, sp, "DBS-2 Fade Buy")); }
                    if st.mp() == B { v.push(exit(S, Limit, avg, "LongLiq")); }
                    if st.mp() == S { v.push(exit(B, Limit, avg, "ShortLiq")); }
                } else {
                    if c[i] > up { v.push(entry(B, Stop, bp, "DBS-2 Buy")); }
                    if c[i] < dn { v.push(entry(S, Stop, sp, "DBS-2 Sell")); }
                    if st.mp() == B { v.push(exit(S, Stop, avg, "LongLiq")); }
                    if st.mp() == S { v.push(exit(B, Stop, avg, "ShortLiq")); }
                }
                v
            })
        }
        "pbwts_seasonal" => {
            let (ls, le, ss, se) = (g("long_start") as u32, g("long_end") as u32, g("short_start") as u32, g("short_end") as u32);
            sim(x, from, to, |i, _| {
                if i + 1 >= n { return vec![]; }
                let md = month_day(&x.r[i + 1].date);
                let mut v = vec![];
                if md >= ls && md <= le { v.push(entry(B, Open, f64::NAN, "Seasonal Buy")); }
                if md >= ss || md <= se { v.push(entry(S, Open, f64::NAN, "Seasonal Sell")); }
                v
            })
        }
        "pbwts_ghost_trader" => {
            let (fast, slow, rn, ob, os, en) = (u("fast"), u("slow"), u("rsi_len"), g("ob"), g("os"), u("exit_len"));
            let ghost_on = g("ghost") >= 0.5;
            let (xc, xh, xl, r) = (xaverage(c, fast), xaverage(h, slow), xaverage(l, slow), rsi(c, rn));
            let (hh, ll) = (highest(h, en), lowest(l, en));
            let xb = |j: usize| r[j - 1] > ob && r[j] < ob; // RSI crosses below ob on bar j
            let xa = |j: usize| r[j - 1] < os && r[j] > os;
            // ghost system from the first bar (independent of real trades)
            let mut profit = vec![0.0; n];
            let (mut gpos, mut gpx, mut gp) = (0i8, 0.0, 0.0);
            for i in 2..n {
                if gpos == 0 && xc[i - 1] > xh[i - 1] && xb(i - 1) && h[i] >= h[i - 1] { gpx = o[i].max(h[i - 1]); gpos = 1; }
                if gpos == 1 && ll[i - 1].is_finite() && l[i] < ll[i - 1] { gp = ll[i - 1].min(o[i]) - gpx; gpos = 0; }
                if gpos == 0 && xc[i - 1] < xl[i - 1] && xa(i - 1) && l[i] <= l[i - 1] { gpx = o[i].min(l[i - 1]); gpos = -1; }
                if gpos == -1 && hh[i - 1].is_finite() && h[i] > hh[i - 1] { gp = gpx - hh[i - 1].max(o[i]); gpos = 0; }
                profit[i] = gp;
            }
            sim(x, from, to, |i, st| {
                let mut v = vec![];
                let ok = !ghost_on || profit[i] < 0.0;
                if i >= 1 && st.mp() == 0.0 && ok && xc[i] > xh[i] && xb(i) { v.push(entry(B, Stop, h[i], "Ghost buy")); }
                if i >= 1 && st.mp() == 0.0 && ok && xc[i] < xl[i] && xa(i) { v.push(entry(S, Stop, l[i], "Ghost sell")); }
                if st.mp() == B { v.push(exit(S, Stop, ll[i], "Lowest(Low,20)")); }
                if st.mp() == S { v.push(exit(B, Stop, hh[i], "Highest(High,20)")); }
                v
            })
        }
        "pbwts_money_manager" | "pbwts_donchian40" => {
            let mm = id == "pbwts_money_manager";
            let (bh, sl) = if mm { (highest(h, u("entry_len")), lowest(l, u("entry_len"))) } else { (highest(h, u("long_len")), lowest(l, u("short_len"))) };
            let (xh, xl) = if mm { (highest(h, u("exit_len")), lowest(l, u("exit_len"))) } else { (vec![], vec![]) };
            sim(x, from, to, |i, st| {
                let mut v = vec![entry(B, Stop, bh[i], if mm { "MMBuy" } else { "Buy" }), entry(S, Stop, sl[i], if mm { "MMSell" } else { "Sell Short" })];
                if mm && st.mp() == B { v.push(exit(S, Stop, xl[i], "LongLiq")); }
                if mm && st.mp() == S { v.push(exit(B, Stop, xh[i], "ShortLiq")); }
                v
            })
        }
        "pbwts_adx_ma" => {
            let adx = dmi(x.r, u("adx_len")).1;
            let (m1, m2, thr, en) = (sma(c, u("ma1")), sma(c, u("ma2")), g("adx_min"), u("exit_len"));
            let (hh, ll) = (highest(h, en), lowest(l, en));
            sim(x, from, to, |i, st| {
                let mut v = vec![];
                if adx[i] >= thr {
                    if cross_above(&m1, &m2, i) { v.push(entry(B, Stop, h[i], "ADX buy")); }
                    if cross_below(&m1, &m2, i) { v.push(entry(S, Stop, l[i], "ADX sell")); }
                }
                if adx[i] < thr {
                    if st.mp() == B { v.push(exit(S, Stop, ll[i], "Lowest(Low,4)")); }
                    if st.mp() == S { v.push(exit(B, Stop, hh[i], "Highest(High,4)")); }
                }
                v
            })
        }
        "pbwts_ma_channel" => {
            let ma = sma(c, u("ma"));
            let (v1, v2) = (highest(&ma, u("chan")), lowest(&ma, u("chan")));
            sim(x, from, to, |i, _| vec![entry(B, Stop, v1[i], "MA channel buy"), entry(S, Stop, v2[i], "MA channel sell")])
        }
        "pbwts_percent_r" => {
            let len = u("len");
            let (hh, ll) = (highest(h, len), lowest(l, len));
            let pr: Vec<f64> = (0..n).map(|i| if hh[i] > ll[i] { 100.0 * (c[i] - ll[i]) / (hh[i] - ll[i]) } else { f64::NAN }).collect();
            let (lo, hi, m) = (g("lo"), g("hi"), g("mult"));
            let (bh, sl) = (highest(h, u("chan")), lowest(l, u("chan")));
            sim(x, from, to, |i, st| {
                let mut v = vec![];
                if i >= 1 && pr[i - 1] < lo && pr[i] > lo { v.push(entry(B, Stop, bh[i], "%R buy")); }
                if i >= 1 && pr[i - 1] > hi && pr[i] < hi { v.push(entry(S, Stop, sl[i], "%R sell")); }
                let rng = h[i] - l[i];
                if st.mp() == B { v.push(exit(S, Stop, st.entry_px() - m * rng, "longLiqPoint")); }
                if st.mp() == S { v.push(exit(B, Stop, st.entry_px() + m * rng, "shortLiqPoint")); }
                v
            })
        }
        "pbwts_dow_oc" | "pbwts_dow_oo" => {
            let side = if g("buy_or_sell").round() == 2.0 { S } else { B };
            let day = g("which_day").round() as u32;
            let oc = id == "pbwts_dow_oc";
            sim(x, from, to, |i, st| {
                let mut v = vec![];
                if !oc && st.mp() != 0.0 { v.push(exit(-st.mp(), Open, f64::NAN, "next open")); }
                if i + 1 < n && super::lwlts::dow(&x.r[i + 1].date) == day {
                    v.push(entry(side, Open, f64::NAN, if side > 0.0 { "Buy On Open" } else { "Sell On Open" }));
                    if oc { v.push(exit(-side, Close, f64::NAN, "SetExitOnClose")); }
                }
                v
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
        v.iter().enumerate().map(|(i, &(o, h, l, c))| Row { ticker: "T".into(), date: format!("2001-01-{:02}", i + 1), open: o, high: h, low: l, close: c, volume: None }).collect()
    }
    #[test]
    fn stop_fills_at_price_or_gap_open_and_reverses_along_the_bar_path() {
        // bar 1: open nearer the high -> path O,H,L,C: buy stop 105 fills first, then the sell stop 95 reverses
        let r = rows(&[(100., 101., 99., 100.), (104., 106., 94., 95.), (90., 92., 89., 91.)]);
        let x = Ctx::new(&r, &r, &[]);
        let t = sim(&x, 1, 2, |i, st| if i == 0 { vec![entry(1., Kind::Stop, 105., "b"), entry(-1., Kind::Stop, 95., "s")] } else if st.mp() < 0.0 { vec![exit(1., Kind::Stop, 93., "x")] } else { vec![] });
        assert_eq!(t.len(), 2);
        assert_eq!((t[0].dir, t[0].legs[0].px, t[0].exit_px, t[0].exit_i), (1., 105., 95., 1));
        // bar 2: the 93 buy-to-cover is above the open (90) and the high (92): never reached, short stays open
        assert_eq!((t[1].dir, t[1].legs[0].px, t[1].open), (-1., 95., true));
        // gap: a buy stop below the open fills at the open
        let t = sim(&x, 2, 2, |_, _| vec![entry(1., Kind::Stop, 85., "b")]);
        assert_eq!(t[0].legs[0].px, 90.);
    }
    #[test]
    fn exit_orders_do_not_hit_a_position_opened_on_the_same_bar() {
        let r = rows(&[(100., 101., 99., 100.), (100., 106., 90., 95.)]);
        let x = Ctx::new(&r, &r, &[]);
        let t = sim(&x, 1, 1, |_, _| vec![entry(1., Kind::Stop, 105., "b"), exit(-1., Kind::Stop, 92., "x")]);
        assert_eq!(t.len(), 1);
        assert!(t[0].open);
    }
    #[test]
    fn dbs_lookback_rounds_and_is_bounded() {
        let c: Vec<f64> = (0..200).map(|i| 100.0 + ((i * 37) % 11) as f64 * if i > 100 { 3.0 } else { 1.0 }).collect();
        let lb = dbs_lookback(&c, 30, 20.0, 20.0, 60.0);
        assert!(lb.iter().all(|&v| (20.0..=60.0).contains(&v) && v.fract() == 0.0));
    }
    #[test]
    fn every_live_strategy_has_params_and_runs() {
        let r: Vec<Row> = (0..400).map(|i| { let b = 100.0 + (i as f64 / 9.0).sin() * 10.0 + i as f64 * 0.05;
            Row { ticker: "T".into(), date: format!("{:04}-{:02}-{:02}", 2000 + i / 300, 1 + (i / 25) % 12, 1 + i % 25), open: b, high: b + 1.5, low: b - 1.5, close: b + 0.5, volume: None } }).collect();
        let x = Ctx::new(&r, &r, &[]);
        for d in catalogue() {
            let live = d.status == Status::Live;
            assert_eq!(live, params(d.id).is_some(), "{}", d.id);
            assert!(family_of(d.id).is_some(), "{}", d.id);
            if live {
                let t = run(d.id, &x, 60, 399, &Params::new()).unwrap();
                for tr in &t { for k in [tr.legs[0].i, tr.exit_i] { assert!(k >= 60 && k <= 399); }
                    assert!(x.l[tr.legs[0].i] <= tr.legs[0].px && tr.legs[0].px <= x.h[tr.legs[0].i], "{}", d.id); }
            }
        }
    }
}
