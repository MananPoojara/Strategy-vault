//! Tom Bierovic, "The ABCs of Trading and Technical Analysis" (Online Investor Expo, Las Vegas,
//! 16 Nov 2000, 130-page slide handout). Book id `tbabc`.
//! Source: research drive `USB HDD 2/tRADING KNO-2/Tom Bierovic - ABCs of Trading and Tech Analysis
//! (Online Investor Expo, Las Vegas 2000)/ABCs of Tech Analysis.pdf` (image scan, read page by page).
//! Notes, testability table and verification: library/tbabc/README.md.
//!
//! Page refs are the printed slide-page numbers (PDF page = printed + 1).
//! "One point" / "1 point" = TradeStation's `Point` (one minimum price move); the book's own code
//! (p.87, 97-100) writes the same rules as `+ 1 point Stop`. Tick: 0.01 US, 0.05 NSE (`bwntd::tick_of`).
//!
//! Fill model (own simulator; jkbwts::sim is not used): every order generated at the close of bar t
//! rests through bar t+1. Inside a bar the price path is TradeStation's default assumption
//! (open, then whichever extreme is nearer the open, then the other extreme, then the close);
//! orders fill in the order the path touches them, gaps fill at the open, and when two orders are
//! touched at the same moment exits go first (stops before limits). STAMPEDE and POWERTOOL are
//! literal ports of the printed EasyLanguage: their exit orders exist only from the bar after entry
//! (they are issued inside `If MarketPosition = 1`). Micro-M/W and SIX-SHOOTER (prose rules) place
//! the protective stop and target as soon as the entry fills.
use crate::books::bwntd::tick_of;
/// Admitted ids (authors/bierovic/laneb/SOURCE-CONTRACT.md).
pub const REVIEWED: &[&str] = &["tbabc_micro_m", "tbabc_micro_w", "tbabc_six_long", "tbabc_six_short", "tbabc_stampede_long", "tbabc_powertool"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }
use crate::connors::{true_range, sma, Claim, Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};

pub fn catalogue() -> Vec<StrategyDef> {
    use Status::*;
    let d = |id, name, group, dir, status, rules, notes, claim| StrategyDef {
        id, name, group, session: 0, dir, status, needs_volume: false, needs_vix: false,
        needs_series: &[], book: "tbabc", rules, notes, claim,
    };
    vec![
        d("tbabc_micro_m", "Micro-M Top (profit-target exit)", "Two New Reversal Patterns", "Short", Live,
          &["F|\"Bar #1 makes a new 10-bar high\" (p.34) - read: High = highest high of the 10 bars ending with Bar #1",
            "F|\"Bar #2 closes lower than the close of Bar #1\"; \"Bar #3 closes higher than the close of Bar #2\" (p.34)",
            "E|\"On either Bar #4 or Bar #5, sell short at the low of Bar #3 minus one point\" (p.34): sell stop, cancelled after Bar #5",
            "X|\"Set a protective stop at the high of the Micro-M pattern plus one point\" (p.34) - pattern = Bars #1-#3",
            "X|\"Take profits at the entry point minus the amount of the trade's initial risk (profit target)\" (p.34)"],
          "Book shows chart examples only (p.33-35), no statistics. The alternative exit (\"OR lower stop to breakeven at the profit target and trail a stop\") gives no trailing rule and is listed separately as not testable. Stocks/commodities in the book; run on the index (instrument swap). If a newer pattern completes while an older one's order is still resting, the newer one replaces it.",
          None),
        d("tbabc_micro_w", "Micro-W Bottom (profit-target exit)", "Two New Reversal Patterns", "Long", Live,
          &["F|\"Bar #1 makes a new 10-bar low\" (p.36)",
            "F|\"Bar #2 closes higher than the close of Bar #1\"; \"Bar #3 closes lower than the close of Bar #2\" (p.36)",
            "E|\"On either Bar #4 or Bar #5, buy at the high of Bar #3 plus one point\" (p.36): buy stop",
            "X|\"Set a protective stop at the low of the Micro-W pattern minus one point\" (p.36)",
            "X|\"Take profits at the entry point plus the amount of the trade's initial risk (profit target)\" (p.36)"],
          "Chart examples only (p.35-37). Same conventions as the Micro-M.", None),
        d("tbabc_micro_trail", "Micro-M/W with breakeven-and-trail exit", "Two New Reversal Patterns", "Long/Short",
          NotTestable("the second exit (\"lower/raise stop to breakeven at the profit target and trail a stop\", p.34/36) never says how the stop is trailed"),
          &["X|\"... OR lower stop to breakeven at the profit target and trail a stop\" (p.34, 36)"], "No trailing rule in the handout.", None),
        d("tbabc_six_long", "SIX-SHOOTER (Long)", "Two New Reversal Patterns", "Long", Live,
          &["F|\"The stock or commodity makes a new 20-bar low\" (p.38)",
            "F|\"The close is above the open\"; \"The close is above the previous bar's close\" (p.38)",
            "F|\"The close is above the midpoint of the previous 20-bar low\" - the bar that made the previous 20-bar low (chart #40 bar A, p.37); midpoint = (high+low)/2",
            "F|\"The close is in the top 25% of the bar's range\" (p.38)",
            "F|\"The range of the setup bar is greater than the range of the bar that made the previous 20-bar low\" (p.38)",
            "E|\"Buy on the next bar's open\" (p.38)",
            "X|\"Set the initial protective stop one point below the low of the setup bar\" (p.38)",
            "X|\"Raise the stop to breakeven when open profit is equal to or greater than the trade's initial risk on a closing basis\" (p.38)",
            "X|\"Take profits with a limit order at a new 20-bar high\" (p.38): sell limit at the highest high of the 19 bars before the order bar"],
          "Previous 20-bar low = the most recent earlier bar whose low was the lowest of its own 20 bars (chart #40: A right before setup bar B). Chart examples only (p.37-39).",
          None),
        d("tbabc_six_short", "SIX-SHOOTER (Short)", "Two New Reversal Patterns", "Short", Live,
          &["F|\"The stock or commodity makes a new 20-bar high\" (p.40)",
            "F|\"The close is below the open\"; \"The close is below the previous bar's close\" (p.40)",
            "F|\"The close is below the midpoint of the previous 20-bar high\" (p.40)",
            "F|\"The close is in the bottom 25% of the bar's range\" (p.40)",
            "F|\"The range of the setup bar is greater than the range of the bar that made the previous 20-bar high\" (p.40)",
            "E|\"Sell short on the next bar's open\" (p.40)",
            "X|\"Set the initial protective stop one point above the high of the setup bar\" (p.40)",
            "X|\"Lower the stop to breakeven when open profit is equal to or greater than the trade's initial risk on a closing basis\" (p.40)",
            "X|\"Take profits with a limit order at a new 20-bar low\" (p.40)"],
          "Mirror rules are printed in the book (p.40). Chart examples only.", None),
        d("tbabc_stampede_long", "STAMPEDE (Long, printed code)", "STAMPEDE Strategy", "Long", Live,
          &["E|\"Buy at Highest(High, ChanLen) + 1 point Stop\", ChanLen = 30 (code p.87; rules p.82)",
            "X|LXbasic: \"ExitLong at Lowest(Low, ChanLen) - 1 point Stop\" (p.87)",
            "X|LX1: \"ExitLong at EntryPrice - IPS * ATR Stop\", IPS 3, ATR = AvgTrueRange(10) (p.87)",
            "X|LX2: \"If MaxTradeClose >= EntryPrice + BES * ATR Then ExitLong at EntryPrice Stop\", BES 3 (p.88)",
            "X|LX3: \"ExitLong at MaxTradeClose - TS * ATR Stop\", TS 4 (p.88)",
            "X|LX4: \"ExitLong at Close - VS * ATR Stop\", VS 3 (p.88)",
            "X|LX5: \"If MaxTradeClose >= EntryPrice + BPS * ATR Then ExitLong at Lowest(Low, BPB) Stop\", BPS 8, BPB 2 (p.88)"],
          "AvgTrueRange = TradeStation simple average of True Range. MaxTradeClose = highest close since entry, entry bar included (chart #100: \"A is the highest close since entry\"). Book test: SUNW, 1,500 daily bars to 08/31/00, $10,000 of stock per trade, no costs (p.89-90); SUNW is not in our data, so no reproduction. Related but different: stad8_stampede (STAD Vol 8 pyramiding version, other defaults).",
          Some(Claim { trades: "21", win: "71.43%", avg: "$1,151.45 per $10,000 trade", hold: "49 bars winners / 19 losers", period: "SUNW, 1,500 bars to 08/31/2000 (p.90)" })),
        d("tbabc_stampede_short", "STAMPEDE (Short)", "STAMPEDE Strategy", "Short", NotTestable("no code or exit is printed for the short side; borrowing POWERTOOL's short exits would be a rule the handout does not give for STAMPEDE"),
          &["E|\"Sell short at the lowest low of the last 30 bars minus one point\" (p.82)",
            "X|\"Exit at the initial protective stop, breakeven stop, trailing stop, volatility stop, big profit stop, or basic short exit\" (p.82)",
            "X|Formulas mirror the long code; short-side forms as printed in the POWERTOOL short code (p.100): EntryPrice + IPS*ATR; MinTradeClose <= EntryPrice - BES*ATR then EntryPrice; MinTradeClose + TS*ATR; Close + VS*ATR; MinTradeClose <= EntryPrice - BPS*ATR then Highest(High,BPB); basic = Highest(High,ChanLen) + 1 point"],
          "The handout prints code only for the long side; the short exits are the book's own short-side forms (POWERTOOL short code, same author, same inputs). No short-side test printed.",
          None),
        d("tbabc_powertool", "POWERTOOL (Long & Short, printed code)", "POWERTOOL Strategy", "Long/Short", Live,
          &["F|TimingAvg = XAverage(Close,10); TrendAvg = XAverage(Close,50); BullPower = High - TimingAvg; BearPower = Low - TimingAvg; ATR = AvgTrueRange(20) (code p.97)",
            "E|BuySetup = TrendAvg > TrendAvg[1] and Close > TrendAvg and TimingAvg > TrendAvg and BullPower > 0 and (BearPower < 0 or BearPower crosses above 0): \"Buy Next Bar at High + 1 point Stop\" (p.98)",
            "E|SellSetup = TrendAvg < TrendAvg[1] and Close < TrendAvg and TimingAvg < TrendAvg and BearPower < 0 and (BullPower > 0 or BullPower crosses below 0): \"Sell Next Bar at Low - 1 point Stop\" (p.99); an opposite entry reverses the position",
            "X|Long: LX1 EntryPrice - IPS*ATR; LX2 if MaxTradeClose >= EntryPrice + BES*ATR then EntryPrice; LX3 MaxTradeClose - TS*ATR; LX4 Close - VS*ATR; LX5 if MaxTradeClose >= EntryPrice + BPS*ATR then Lowest(Low,BPB); IPS 3, BES 3, TS 4, VS 3, BPS 8, BPB 2 (p.98)",
            "X|Short: SX1 EntryPrice + IPS*ATR; SX2 if MinTradeClose <= EntryPrice - BES*ATR then EntryPrice; SX3 MinTradeClose + TS*ATR; SX6 Close + VS*ATR; SX7 if MinTradeClose <= EntryPrice - BPS*ATR then Highest(High,BPB) (p.99)"],
          "XAverage = TradeStation EMA (2/(n+1), seeded with the first close). Crosses above 0 = now > 0 and previous <= 0. Book tests: PMC-Sierra (PMCS) 11/14/94-10/06/00 and Japanese Yen futures 01/01/94-09/30/00 (p.100-107); neither is in our data.",
          Some(Claim { trades: "19 (PMCS) / 23 (JY)", win: "63.16% / 47.83%", avg: "$2,399.65 per $10,000 stock / $2,787.12 per JY contract", hold: "winners 53 / losers 19 bars (PMCS)", period: "PMCS 11/14/94-10/06/00; JY 01/01/94-09/30/00 (p.102, 107)" })),
        d("tbabc_pnf", "Point & figure signals and exits", "Point & Figure Charts", "Long/Short",
          NotTestable("box size is defined only for stocks priced $20-$100 (\"each X or O represents $1\", p.51); no box rule for an index at 1,000s, and choosing one would be our rule"),
          &["E|Basic buy signal: an X column exceeds the previous X column; basic sell signal: an O column falls below the previous O column (p.53-54)",
            "X|Basic long/short exit (p.54-55); big profit exit: \"When your open profits reach ten boxes or more, take profits on the next reversal\" (p.57)"],
          "Three-box reversal (p.52).", None),
        d("tbabc_ma_cross", "Moving-average crossover chart examples", "Moving Averages", "Long",
          NotTestable("chart captions only (charts #77-78: \"Buy at A when SMA:5 crosses above SMA:12\", \"Sell at B\"); the fill bar/price is not stated and chart #78 (SMA 5/15) does not say whether a sell reverses"),
          &["E|\"Buy at A when SMA:5 crosses above SMA:12\"; \"Sell at B when SMA:5 crosses below SMA:12\"; \"Buy again at C\" (chart #77, p.61)"],
          "Chart #93 (\"Buy at A when the stock closes above the SMA [100] / Sell at B when the stock closes below\") duplicates apta_ma100_dji and is not added.", None),
        d("tbabc_discretionary", "Chart patterns, candlesticks, Fibonacci, indicators", "Bar-chart & candlestick patterns", "Long/Short",
          NotTestable("discretionary: trendlines, flags, triangles, wedges, head-and-shoulders, double tops, candlestick patterns, Fibonacci zones and indicator divergences are drawn/judged by eye and have no exit rules (p.15-74)"),
          &["F|Swing highs/lows, trendlines, gaps, key/closing-price reversals, chart patterns, candlesticks, Bollinger, momentum, RSI, OBV, Fibonacci (p.15-74)"],
          "Teaching material, no mechanical trade rules.", None),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    Some(match id {
        "tbabc_micro_m" => ("tbabc_micro", "Micro-M / Micro-W", "Micro-M top · target"),
        "tbabc_micro_w" => ("tbabc_micro", "Micro-M / Micro-W", "Micro-W bottom · target"),
        "tbabc_micro_trail" => ("tbabc_micro", "Micro-M / Micro-W", "Breakeven + trail (not testable)"),
        "tbabc_six_long" => ("tbabc_six", "SIX-SHOOTER", "Long"),
        "tbabc_six_short" => ("tbabc_six", "SIX-SHOOTER", "Short"),
        "tbabc_stampede_long" => ("tbabc_stampede", "STAMPEDE", "Long (printed code)"),
        "tbabc_stampede_short" => ("tbabc_stampede", "STAMPEDE", "Short (mirror)"),
        "tbabc_powertool" => ("tbabc_powertool", "POWERTOOL", "Long & short"),
        "tbabc_pnf" => ("tbabc_pnf", "Point & figure", "Basic signals"),
        "tbabc_ma_cross" => ("tbabc_ma", "MA crossover examples", "Charts #77-78"),
        "tbabc_discretionary" => ("tbabc_disc", "Discretionary chart reading", "Patterns"),
        _ => return None,
    })
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    let stops = || vec![p("ips", "Initial stop (ATRs)", 3., 2., 4., 1.), p("bes", "Breakeven trigger (ATRs)", 3., 2., 4., 1.),
        p("ts", "Trailing stop (ATRs)", 4., 3., 5., 1.), p("vs", "Volatility stop (ATRs)", 3., 2., 4., 1.),
        p("bps", "Big-profit trigger (ATRs)", 8., 6., 10., 1.), p("bpb", "Big-profit stop: lowest low of N bars", 2., 1., 3., 1.)];
    Some(match id {
        "tbabc_micro_m" | "tbabc_micro_w" => vec![p("n", "Bar #1 new N-bar extreme", 10., 5., 20., 1.)],
        "tbabc_six_long" | "tbabc_six_short" => vec![p("n", "N-bar low/high", 20., 10., 40., 5.), p("q", "Close in top/bottom fraction of range", 0.25, 0.1, 0.5, 0.05)],
        "tbabc_stampede_long" | "tbabc_stampede_short" => {
            let mut v = vec![p("chan", "Channel length", 30., 10., 50., 5.), p("atr", "ATR length", 10., 5., 30., 5.)];
            v.extend(stops());
            v
        }
        "tbabc_powertool" => {
            let mut v = vec![p("timing", "Timing EMA", 10., 5., 15., 2.), p("trend", "Trend EMA", 50., 30., 70., 5.), p("atr", "ATR length", 20., 10., 30., 5.)];
            v.extend(stops());
            v
        }
        _ => return None,
    })
}

// ---------------------------------------------------------------- path-ordered bar simulator

#[derive(Clone, Copy, PartialEq)]
enum K { Stop, Limit, Open }

#[derive(Clone, Copy)]
struct Ord { dir: f64, px: f64, k: K, entry: bool, tag: &'static str, aux: f64 }

#[derive(Clone, Copy)]
struct Pos { dir: f64, px: f64, i: usize, sig: usize, ext: f64, aux: f64, kind: &'static str }

fn ord(dir: f64, px: f64, k: K, entry: bool, tag: &'static str) -> Ord { Ord { dir, px, k, entry, tag, aux: f64::NAN } }

/// TradeStation bar path: open, nearer extreme, other extreme, close.
fn path(o: f64, h: f64, l: f64, c: f64) -> [f64; 4] { if h - o < o - l { [o, h, l, c] } else { [o, l, h, c] } }

/// First moment >= t0 at which the path reaches `lvl` (from below if `up`), with the fill price.
fn touch(p: &[f64; 4], t0: f64, lvl: f64, up: bool) -> Option<(f64, f64)> {
    let hit = |v: f64| if up { v >= lvl } else { v <= lvl };
    let seg = (t0.floor() as usize).min(2);
    let p0 = p[seg] + (p[seg + 1] - p[seg]) * (t0 - seg as f64);
    if hit(p0) { return Some((t0, p0)); } // already through: fills at the current price (gap = open)
    for k in seg..3 {
        if hit(p[k + 1]) { return Some((k as f64 + (lvl - p[k]) / (p[k + 1] - p[k]), lvl)); }
    }
    None
}

fn when(p: &[f64; 4], t0: f64, o: &Ord) -> Option<(f64, f64)> {
    match o.k {
        K::Open => (t0 == 0.).then_some((0., p[0])),
        K::Stop => touch(p, t0, o.px, o.dir > 0.),
        K::Limit => touch(p, t0, o.px, o.dir < 0.),
    }
}

fn close_trade(ps: &Pos, b: usize, px: f64, why: &str) -> Trade {
    Trade { dir: ps.dir, legs: vec![Leg { i: ps.i, px: ps.px, w: 1. }], exit_i: b, exit_px: px, reason: why.into(),
        entry_kind: ps.kind, signal_i: b, open: false, exits: vec![] }
}

/// `gen(t, pos)` = orders resting through bar t+1, built at bar t's close. If `immediate`,
/// `on_entry(pos)` adds the exit orders of a position the moment it fills.
fn sim(x: &Ctx, from: usize, to: usize, immediate: bool, gen: &dyn Fn(usize, Option<&Pos>) -> Vec<Ord>,
       on_entry: &dyn Fn(&Pos) -> Vec<Ord>) -> Vec<Trade> {
    let mut out = vec![];
    let mut pos: Option<Pos> = None;
    for t in from..to {
        let b = t + 1;
        let mut ords = gen(t, pos.as_ref());
        let p = path(x.o[b], x.h[b], x.l[b], x.c[b]);
        let mut now = 0.;
        loop {
            let mut best: Option<(f64, u8, usize, f64)> = None;
            for (j, o) in ords.iter().enumerate() {
                if !o.px.is_finite() && o.k != K::Open { continue; }
                let live = match &pos { None => o.entry, Some(ps) => !o.entry || o.dir != ps.dir };
                if !live { continue; }
                let Some((tm, px)) = when(&p, now, o) else { continue };
                let rank = if o.entry { 2 } else if o.k == K::Stop { 0 } else { 1 };
                if best.map_or(true, |(bt, br, _, _)| tm < bt || (tm == bt && rank < br)) { best = Some((tm, rank, j, px)); }
            }
            let Some((tm, _, j, px)) = best else { break };
            let o = ords.remove(j);
            now = tm;
            if let Some(ps) = pos.take() {
                out.push(close_trade(&ps, b, px, if o.entry { "reversed by opposite entry" } else { o.tag }));
                ords.retain(|q| q.entry);
            }
            if o.entry {
                let kind = if o.k == K::Open { "open" } else { "stop" };
                let ps = Pos { dir: o.dir, px, i: b, sig: t, ext: f64::NAN, aux: o.aux, kind };
                if immediate { ords.extend(on_entry(&ps)); }
                pos = Some(ps);
            }
        }
        if let Some(ps) = pos.as_mut() {
            let c = x.c[b];
            ps.ext = if ps.ext.is_nan() { c } else if ps.dir > 0. { ps.ext.max(c) } else { ps.ext.min(c) };
        }
    }
    if let Some(ps) = pos {
        let mut t = close_trade(&ps, to, x.c[to], "open");
        t.open = true;
        out.push(t);
    }
    out
}

// ---------------------------------------------------------------- indicators

fn hh(v: &[f64], n: usize, t: usize) -> f64 { if n == 0 || t + 1 < n { f64::NAN } else { v[t + 1 - n..=t].iter().copied().fold(f64::MIN, f64::max) } }
fn ll(v: &[f64], n: usize, t: usize) -> f64 { if n == 0 || t + 1 < n { f64::NAN } else { v[t + 1 - n..=t].iter().copied().fold(f64::MAX, f64::min) } }

/// TradeStation XAverage: seeded with the first value, alpha 2/(n+1).
fn xaverage(v: &[f64], n: usize) -> Vec<f64> {
    let a = 2. / (n as f64 + 1.);
    let mut out = Vec::with_capacity(v.len());
    for (i, &x) in v.iter().enumerate() { out.push(if i == 0 { x } else { out[i - 1] + a * (x - out[i - 1]) }); }
    out
}

/// TradeStation AvgTrueRange = simple average of TrueRange.
fn avg_true_range(x: &Ctx, n: usize) -> Vec<f64> { sma(&true_range(x.r), n) }

/// The five ATR exits shared by STAMPEDE and POWERTOOL (+ STAMPEDE's basic channel exit).
fn atr_exits(x: &Ctx, t: usize, ps: &Pos, a: f64, g: &dyn Fn(&str) -> f64, basic: Option<f64>) -> Vec<Ord> {
    let (d, e, px) = (ps.dir, ps.ext, ps.px);
    let (bpb, s) = (g("bpb") as usize, -ps.dir);
    let mut v = vec![];
    if let Some(b) = basic { v.push(ord(s, b, K::Stop, false, if d > 0. { "LXbasic: channel low - 1 pt" } else { "SXbasic: channel high + 1 pt" })); }
    v.push(ord(s, px - d * g("ips") * a, K::Stop, false, if d > 0. { "LX1 initial protective stop" } else { "SX1 initial protective stop" }));
    if d * (e - px) >= g("bes") * a { v.push(ord(s, px, K::Stop, false, if d > 0. { "LX2 breakeven stop" } else { "SX2 breakeven stop" })); }
    v.push(ord(s, e - d * g("ts") * a, K::Stop, false, if d > 0. { "LX3 trailing stop" } else { "SX3 trailing stop" }));
    v.push(ord(s, x.c[t] - d * g("vs") * a, K::Stop, false, if d > 0. { "LX4 volatility stop" } else { "SX6 volatility stop" }));
    if d * (e - px) >= g("bps") * a {
        v.push(ord(s, if d > 0. { ll(&x.l, bpb, t) } else { hh(&x.h, bpb, t) }, K::Stop, false, if d > 0. { "LX5 big profit stop" } else { "SX7 big profit stop" }));
    }
    v
}

/// Most recent bar k < i whose low (high if `hi`) was the extreme of its own n bars.
fn prev_extreme_bar(x: &Ctx, i: usize, n: usize, hi: bool) -> Option<usize> {
    (n - 1..i).rev().find(|&k| if hi { x.h[k] >= hh(&x.h, n, k) } else { x.l[k] <= ll(&x.l, n, k) })
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    let defs = params(id)?;
    if from >= to || to >= x.r.len() { return Some(vec![]); }
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| defs.iter().find(|d| d.key == k).map_or(f64::NAN, |d| d.def));
    let tk = tick_of(x);
    let none = |_: &Pos| vec![];
    Some(match id {
        "tbabc_micro_m" | "tbabc_micro_w" => {
            let d = if id == "tbabc_micro_m" { -1. } else { 1. };
            let n = g("n") as usize;
            // bar #3 at j: bar #1 = j-2 makes the new n-bar extreme, then the two closes.
            let is_pat = |j: usize| j + 1 >= n + 2 && if d < 0. {
                x.h[j - 2] >= hh(&x.h, n, j - 2) && x.c[j - 1] < x.c[j - 2] && x.c[j] > x.c[j - 1]
            } else {
                x.l[j - 2] <= ll(&x.l, n, j - 2) && x.c[j - 1] > x.c[j - 2] && x.c[j] < x.c[j - 1]
            };
            let exits = |ps: &Pos| vec![
                ord(-d, ps.aux, K::Stop, false, "protective stop: pattern extreme + 1 pt"),
                ord(-d, ps.px + (ps.px - ps.aux), K::Limit, false, "profit target: entry + initial risk"),
            ];
            let gen = |t: usize, pos: Option<&Pos>| -> Vec<Ord> {
                if let Some(ps) = pos { return exits(ps); }
                // Bar #4 = t+1 (pattern ends at t) or Bar #5 = t+1 (pattern ended at t-1); newest wins.
                // One shot per pattern: a Bar #5 order is re-issued only if Bar #4 never reached the level.
                let lvl_of = |j: usize| if d < 0. { x.l[j] - tk } else { x.h[j] + tk };
                let unfilled_on_bar4 = |j: usize| if d < 0. { x.l[t] > lvl_of(j) } else { x.h[t] < lvl_of(j) };
                [t, t.wrapping_sub(1)].into_iter().filter(|&j| j < t + 1 && j >= 2)
                    .find(|&j| is_pat(j) && (j == t || unfilled_on_bar4(j))).map(|j| {
                    let stop = if d < 0. { hh(&x.h, 3, j) + tk } else { ll(&x.l, 3, j) - tk };
                    Ord { aux: stop, ..ord(d, lvl_of(j), K::Stop, true, "") }
                }).into_iter().collect()
            };
            sim(x, from, to, true, &gen, &exits)
        }
        "tbabc_six_long" | "tbabc_six_short" => {
            let d = if id == "tbabc_six_long" { 1. } else { -1. };
            let (n, q) = (g("n") as usize, g("q"));
            let setup = |i: usize| -> bool {
                if i < n { return false; }
                let (o, h, l, c) = (x.o[i], x.h[i], x.l[i], x.c[i]);
                let Some(k) = prev_extreme_bar(x, i, n, d < 0.) else { return false };
                let mid = (x.h[k] + x.l[k]) / 2.;
                let rng_ok = h - l > x.h[k] - x.l[k];
                if d > 0. {
                    l <= ll(&x.l, n, i) && c > o && c > x.c[i - 1] && c > mid && c >= l + (1. - q) * (h - l) && rng_ok
                } else {
                    h >= hh(&x.h, n, i) && c < o && c < x.c[i - 1] && c < mid && c <= l + q * (h - l) && rng_ok
                }
            };
            // target = a new n-bar extreme on the order bar = beyond the n-1 bars before it
            let target = |t: usize| if d > 0. { hh(&x.h, n - 1, t) } else { ll(&x.l, n - 1, t) };
            let stop_of = |ps: &Pos| {
                let risk = d * (ps.px - ps.aux);
                if !ps.ext.is_nan() && d * (ps.ext - ps.px) >= risk { (ps.px, "breakeven stop") } else { (ps.aux, "initial protective stop") }
            };
            let exits_at = |t: usize, ps: &Pos| {
                let (s, why) = stop_of(ps);
                vec![ord(-d, s, K::Stop, false, why), ord(-d, target(t), K::Limit, false, "limit at new 20-bar extreme")]
            };
            let gen = |t: usize, pos: Option<&Pos>| -> Vec<Ord> {
                if let Some(ps) = pos { return exits_at(t, ps); }
                if !setup(t) { return vec![]; }
                vec![Ord { aux: if d > 0. { x.l[t] - tk } else { x.h[t] + tk }, ..ord(d, f64::NAN, K::Open, true, "") }]
            };
            let on_entry = |ps: &Pos| exits_at(ps.sig, ps);
            sim(x, from, to, true, &gen, &on_entry)
        }
        "tbabc_stampede_long" | "tbabc_stampede_short" => {
            let d = if id == "tbabc_stampede_long" { 1. } else { -1. };
            let chan = g("chan") as usize;
            let atr = avg_true_range(x, g("atr") as usize);
            let gen = |t: usize, pos: Option<&Pos>| -> Vec<Ord> {
                let a = atr[t];
                if !a.is_finite() || t + 1 < chan { return vec![]; }
                let (entry, basic) = if d > 0. { (hh(&x.h, chan, t) + tk, ll(&x.l, chan, t) - tk) } else { (ll(&x.l, chan, t) - tk, hh(&x.h, chan, t) + tk) };
                let mut v = vec![ord(d, entry, K::Stop, true, "")];
                if let Some(ps) = pos { v.extend(atr_exits(x, t, ps, a, &g, Some(basic))); }
                v
            };
            sim(x, from, to, false, &gen, &none)
        }
        "tbabc_powertool" => {
            let timing = xaverage(&x.c, g("timing") as usize);
            let trend = xaverage(&x.c, g("trend") as usize);
            let atr = avg_true_range(x, g("atr") as usize);
            let bull = |i: usize| x.h[i] - timing[i];
            let bear = |i: usize| x.l[i] - timing[i];
            let gen = |t: usize, pos: Option<&Pos>| -> Vec<Ord> {
                let a = atr[t];
                if t < 1 || !a.is_finite() { return vec![]; }
                let mut v = vec![];
                if trend[t] > trend[t - 1] && x.c[t] > trend[t] && timing[t] > trend[t] && bull(t) > 0.
                    && (bear(t) < 0. || (bear(t) > 0. && bear(t - 1) <= 0.)) { v.push(ord(1., x.h[t] + tk, K::Stop, true, "")); }
                if trend[t] < trend[t - 1] && x.c[t] < trend[t] && timing[t] < trend[t] && bear(t) < 0.
                    && (bull(t) > 0. || (bull(t) < 0. && bull(t - 1) >= 0.)) { v.push(ord(-1., x.l[t] - tk, K::Stop, true, "")); }
                if let Some(ps) = pos { v.extend(atr_exits(x, t, ps, a, &g, None)); }
                v
            };
            sim(x, from, to, false, &gen, &none)
        }
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;
    fn rows(b: &[(f64, f64, f64, f64)]) -> Vec<Row> {
        b.iter().enumerate().map(|(i, &(o, h, l, c))| Row { ticker: "SPX".into(), date: format!("{:04}-01-01", 2000 + i),
            open: o, high: h, low: l, close: c, volume: None }).collect()
    }

    #[test]
    fn path_and_touch_follow_tradestation_order() {
        let p = path(100., 101., 90., 95.); // open nearer the high: O-H-L-C
        assert_eq!(p, [100., 101., 90., 95.]);
        assert_eq!(touch(&p, 0., 99., false).map(|v| v.1), Some(99.));
        assert_eq!(touch(&p, 0., 102., false).map(|v| v.1), Some(100.)); // gap through a sell stop = open
        assert!(touch(&p, 0., 102., true).is_none());
    }

    #[test]
    fn micro_w_buys_on_bar4_and_hits_one_r_target() {
        // 10 flat bars, bar #1 (i=10) new 10-bar low, #2 closes up, #3 closes down, #4 breaks #3 high.
        let mut b = vec![(100., 101., 99., 100.); 10];
        b.push((99., 99.5, 95., 96.)); // #1 low 95
        b.push((96., 98., 95.5, 97.5)); // #2
        b.push((97.5, 98.2, 96., 96.5)); // #3 high 98.2
        b.push((97., 99., 96.8, 98.9)); // #4: buy stop 98.21 fills; stop 94.99, target 98.21 + 3.22 = 101.43
        b.push((99., 102., 98.5, 101.)); // target filled
        b.push((101., 101., 100., 100.5));
        let r = rows(&b);
        let x = Ctx::new(&r, &r, &[]);
        let t = run("tbabc_micro_w", &x, 0, r.len() - 1, &Params::new()).unwrap();
        assert_eq!(t.len(), 1);
        assert_eq!(t[0].legs[0].i, 13);
        assert!((t[0].legs[0].px - 98.21).abs() < 1e-9);
        assert_eq!(t[0].exit_i, 14);
        assert!((t[0].exit_px - (98.21 + (98.21 - 94.99))).abs() < 1e-9);
    }
}

// Instrument admission: Tom Bierovic 2026-10-26 (lane B) block, moved from instrument_policy.rs on 31 Oct 2026 (lane ST).
// Admission only for the module REVIEWED lists (authors/bierovic/laneb/SOURCE-CONTRACT.md); kept in one block to merge.
// Runs on actual indices, ETFs and stocks, US and India (index_etf_or_stock, nifty50 basket, declared application).
pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book=="tbabc" && crate::books::tbabc::reviewed(def.id),
    label: "Bierovic shows these rules on stock and futures charts (STAMPEDE on SUNW, POWERTOOL on PMC-Sierra and yen futures); runs on indices, ETFs and single stocks (US and India) are a declared application, one unit per trade, with TradeStation's bar path for orders on the same bar.",
    basis: "Tom Bierovic, The ABCs of Trading and Technical Analysis (Online Investor Expo 2000), p.33-40, 82-100",
};
