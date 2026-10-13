//! Connors Group, "The RSI 25 Explosions System" (April 2004). Book id "rsi25".
//! Five long-only variants of one idea: buy the close when Wilder RSI(4) < 25, exit the close when RSI(4) > 55.
//! Optional 200-day SMA filter, VIX >= 5% above its 10-day SMA filter, and the 25/15 add-on (one extra unit
//! when RSI(4) < 15 while in the trade). All fit the shared `simulate` (close entries, close adds, close exits).
#![allow(unused_imports, dead_code)]
use crate::connors::*;

pub const BOOK: &str = "rsi25";

const BASIC: &str = "r25_rsi25_basic_no_filter";
const OFFICIAL: &str = "r25_rsi25_official_sma200";
const VIX: &str = "r25_rsi25_vix";
const R2515: &str = "r25_rsi25_15";
const R2515_VIX: &str = "r25_rsi25_15_vix";

macro_rules! note { () => { "RSI = Wilder RSI(4) on the close (TradeStation RSI; the book gives no formula). 'Closes below 25' = RSI4 < 25 strict (rule text; p.le=1 switches to <=, the prose's '25 or less'); exit 'closes above 55' = RSI4 > 55 on a later close. \
One position at a time; further signals while long are ignored; a new trade may start the bar after an exit. No stop, no time exit (none in the book). \
[Not from the book] p.min_bars (default 0 = off) blocks entries before that bar of the data, so the RSI seed can settle (e.g. 30); the book's TradeStation runs show no trade before ~bar 210 (MaxBarsBack, fitted, not stated). \
Fills at the signal close (the book's tests); p.exit_open=1 exits at the next open instead (the book's allowed alternative). \
The book tested the NDX, SPX and SOX cash indices in index points, 1 unit, no costs (1991-2003, SOX from 5/94); the vault runs it on indices/ETFs (US check: SPY for SPX, QQQ for NDX, SMH for SOX; India: NIFTY 50, NIFTY BANK, NIFTY IT as the SOX analogue, the midcap indices)." } }
macro_rules! note_trend { () => { " 200-day filter: Close > SMA(Close,200) incl. today, checked on the entry bar only (a close below the SMA does not force an exit)." } }
macro_rules! note_vix { () => { " VIX filter: VIX close >= 1.05 x SMA(VIX close,10) incl. today, entry bar only. US: CBOE ^VIX as the book names it (p-26), the same S&P VIX for NDX and SOX; India: India VIX, so the window starts with its data (2015). The book's numbers fit today's back-filled CBOE ^VIX (12/15 exact counts on the cash indices) and not VXO (9/15)." } }
macro_rules! note_add { () => { " 25/15 add-on: while in the trade, on a close with RSI4 < 15 AND Close > SMA200 (rule 4/5; the VIX filter is NOT needed for the add, p-43) buy one more unit at that close; at most one add per trade (p.max_adds raises it). The add can fire on the entry bar itself when that close already has RSI4 < 15 (both orders fill at that close; it never opens a position on its own): this is the reading that reproduces the book exactly (SPX 105 units / 1,166.24 pts vs book 105 / 1,166.22; SOX 50 / 1,450.21 exact), while 'later bars only' (p.add_entry_bar=0) gives 101 / 1,093.95. \
The book's wording ('while still in the trade... drops further', p-35) suggests a later bar; the entry-bar fill is inferred from the published results and from TradeStation's 'different entry' pyramiding, not stated. \
Both units exit together. The book counts each unit as a trade (TradeStation): here a trade with the add has 2 legs of weight 0.5, so the book's count = number of legs. The claim counts units; the vault's 'trades' stat counts positions (a 2-leg trade = 2 units)." } }

pub fn catalogue() -> Vec<StrategyDef> {
    let d = |id, name, group, needs_vix, rules, notes, claim| StrategyDef {
        id,
        name,
        group,
        session: 0,
        dir: "Long",
        status: Status::Live,
        needs_volume: false,
        needs_vix,
        needs_series: &[],
        book: BOOK,
        rules,
        notes,
        claim,
    };
    vec![
        d(BASIC, "RSI 25 Explosions (no trend filter)", "Part I · RSI 25 Explosions System (Without Trend Filter)", false,
          &[
              "E|The 4-day RSI of the market closes below 25.",
              "E|Buy at the close.",
              "X|Exit at the close when the 4-day RSI of the market closes above 55.",
          ],
          note!(),
          claim("127", "78.74%", "29.50 pts", "4 / 12 bars (win / loss)", "NDX 1/2/91-12/31/03, index points, PF 3.78 (SPX 121 / 76.03% / 8.85; SOX 85 / 74.12% / 11.92)")),
        d(OFFICIAL, "RSI 25 Explosions (Official, 200-day SMA)", "Part I · Official Rules", false,
          &[
              "F|The market closes above its 200-day moving average.",
              "E|The 4-day RSI of the market closes below 25.",
              "E|Buy at the close.",
              "X|Exit at the close when the 4-day RSI of the market closes above 55.",
          ],
          concat!(note!(), note_trend!()),
          claim("75", "88.00%", "30.97 pts", "4 / 10 bars (win / loss)", "NDX 1/2/91-12/31/03, index points, PF 13.07 (SPX 80 / 82.50% / 9.70; SOX 41 / 85.37% / 20.60)")),
        d(VIX, "RSI 25 Explosions with the VIX", "Part II · Maximizing RSI Explosions System With The VIX", true,
          &[
              "F|The market closes above its 200-day moving average.",
              "E|The 4-day RSI of the market closes below 25.",
              "F|The VIX closes at least 5% above its 10-day moving average.",
              "E|Buy at the close.",
              "X|Exit at the close when the 4-day RSI of the market closes above 55.",
          ],
          concat!(note!(), note_trend!(), note_vix!()),
          claim("49", "85.71%", "33.97 pts", "4 / 8 bars (win / loss)", "NDX 1/2/91-12/31/03, index points, PF 14.85 (SPX 63 / 82.54% / 10.49; SOX 22 / 86.36% / 28.41)")),
        d(R2515, "RSI 25/15 (aggressive add-on)", "Part III · For Aggressive Traders Only: The RSI 25/15 System", false,
          &[
              "F|The market closes above its 200-day moving average.",
              "E|The 4-day RSI of the market closes below 25.",
              "E|Buy your position at the close.",
              "A|The market closes above its 200-day moving average.",
              "A|While still in the trade, if the 4-day RSI of the market closes below 15, buy another unit.",
              "X|Exit at the entire position at close when the 4-day RSI of the market closes above 55 or the next morning.",
          ],
          concat!(note!(), note_trend!(), note_add!()),
          claim("96 units (vault trades = positions)", "86.46%", "34.50 pts", "4 / 10 bars (win / loss)", "NDX 1991-2003, index points, PF 14.45 (SPX 105 / 83.81% / 11.11; SOX 50 / 86.00% / 29.00)")),
        d(R2515_VIX, "RSI 25/15 with the VIX", "Part III · Adding The VIX to the RSI 25/15", true,
          &[
              "F|The market closes above its 200-day moving average.",
              "E|The 4-day RSI of the market closes below 25.",
              "F|The VIX closes at least 5% above its 10-day moving average.",
              "E|Buy your position at the close.",
              "A|The market closes above its 200-day moving average, but the VIX does not have to be 5% above its 10-day moving average.",
              "A|While still in the trade, if the 4-day RSI of the market closes below 15, buy another unit.",
              "X|Exit at the entire position at close when the 4-day RSI of the market closes above 55 or the next morning.",
          ],
          concat!(note!(), note_trend!(), note_vix!(), note_add!()),
          claim("63 units (vault trades = positions)", "85.71%", "42.58 pts", "4 / 8 bars (win / loss)", "NDX 1991-2003, index points, PF 20.73 (SPX 85 / 84.71% / 12.06; SOX 29 / 86.21% / 35.82)")),
    ]
}

/// (200-day filter, VIX filter, 25/15 add-on) per strategy.
fn flags(id: &str) -> Option<(bool, bool, bool)> {
    Some(match id {
        BASIC => (false, false, false),
        OFFICIAL => (true, false, false),
        VIX => (true, true, false),
        R2515 => (true, false, true),
        R2515_VIX => (true, true, true),
        _ => return None,
    })
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    let (trend, vix, add) = flags(id)?;
    let mut v = vec![
        p("rsi_n", "RSI period", 4.0, 2.0, 14.0, 1.0),
        p("entry", "Buy when RSI closes below", 25.0, 5.0, 40.0, 5.0),
        p("exit", "Exit when RSI closes above", 55.0, 40.0, 80.0, 5.0),
        p("le", "Entry test: 0 = below (<), 1 = or equal (<=)", 0.0, 0.0, 1.0, 1.0),
        p("exit_open", "Exit fill: 0 = signal close, 1 = next open", 0.0, 0.0, 1.0, 1.0),
        p("min_bars", "[Not from the book] No entry before this bar of the data (RSI warm-up)", 0.0, 0.0, 250.0, 10.0),
    ];
    if trend {
        v.push(p("trend", "Trend filter: SMA period", 200.0, 50.0, 300.0, 10.0));
    }
    if vix {
        v.push(p("vix_pct", "VIX % above its 10-day SMA", 5.0, 0.0, 15.0, 1.0));
    }
    if add {
        v.push(p("add", "Add a unit when RSI closes below", 15.0, 5.0, 25.0, 5.0));
        v.push(p("max_adds", "Max add-on units per trade", 1.0, 1.0, 5.0, 1.0));
        v.push(p("add_entry_bar", "Add-on may fire on the entry bar: 1 = yes (book-matching), 0 = later bars only", 1.0, 0.0, 1.0, 1.0));
    }
    Some(v)
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    let (trend, vix, add) = flags(id)?;
    let schema = params(id)?;
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).map_or(0.0, |d| d.def));
    let u = |k: &str| g(k).round().max(1.0) as usize;
    let rn = u("rsi_n").max(2);
    let rs = x.rsi_n(rn);
    let ma = x.sma_n(if trend { u("trend") } else { 1 });
    let st = if vix { x.vix.stretched(g("vix_pct")) } else { Vec::new() };
    let (lo, hi, add_lt, le, nxt) = (g("entry"), g("exit"), g("add"), g("le") >= 0.5, g("exit_open") >= 0.5);
    let below = |v: f64, t: f64| if le { v <= t } else { v < t };
    let up = |i: usize| !trend || x.c[i] > ma[i];
    let warm = g("min_bars").max(0.0) as usize;
    let entry = |i: usize| (i >= warm && below(rs[i], lo) && up(i) && (!vix || st[i] >= 1)).then_some(Entry::Close);
    let add_fn = |i: usize, _: &Trade| add && below(rs[i], add_lt) && up(i);
    let exit = |i: usize, _: &Trade| {
        (rs[i] > hi).then(|| {
            let why = format!("RSI({rn}) {:.1} > {hi}", rs[i]);
            if nxt { Exit::NextOpen(why) } else { Exit::Close(why) }
        })
    };
    let units = 1 + if add { u("max_adds") } else { 0 };
    let weights = vec![1.0 / units as f64; units];
    let mut trades = simulate(x, &Spec { dir: 1.0, weights: &weights, entry: &entry, add: &add_fn, exit: &exit }, from, to);
    // The add-on order is also live on the entry bar (both orders fire on that close when RSI4 < 15), which the shared
    // simulator does not allow; insert that leg and keep the unit cap. Later adds never depend on the leg count, so
    // this equals a full re-simulation.
    if add && g("add_entry_bar") >= 0.5 {
        for t in trades.iter_mut() {
            let i = t.legs[0].i;
            if add_fn(i, &*t) {
                t.legs.insert(1, Leg { i, px: x.c[i], w: t.legs[0].w });
                t.legs.truncate(units);
            }
        }
    }
    Some(trades)
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    let (f, n) = ("r25", "RSI 25 Explosions (Connors Group 2004)");
    Some(match id {
        BASIC => (f, n, "No trend filter"),
        OFFICIAL => (f, n, "Official (200-day SMA)"),
        VIX => (f, n, "200-day SMA + VIX"),
        R2515 => (f, n, "25/15 add-on"),
        R2515_VIX => (f, n, "25/15 add-on + VIX"),
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;

    fn rows(c: &[f64]) -> Vec<Row> {
        c.iter()
            .enumerate()
            .map(|(i, &c)| Row { ticker: "T".into(), date: format!("{:04}-01-01", 1000 + i), open: c, high: c, low: c, close: c, volume: None })
            .collect()
    }

    /// Uptrend, then a 4-bar slide (RSI4 < 25 -> buy that close; deeper -> 25/15 add), then a jump (RSI4 > 55 -> exit).
    #[test]
    fn entry_add_exit_by_hand() {
        let mut c: Vec<f64> = (0..230).map(|i| 100.0 + i as f64).collect();
        c.extend([328.0, 326.0, 323.0, 319.0, 314.0, 340.0]);
        let r = rows(&c);
        let x = Ctx::new(&r, &r, &[]);
        let last = c.len() - 1;
        let first = (1..=last).find(|&i| x.rsi4[i] < 25.0).unwrap();
        assert!(first >= 230 && x.c[first] > x.ma200[first]);
        let t = run(BASIC, &x, 0, last, &Params::new()).unwrap();
        assert_eq!(t.len(), 1);
        assert_eq!((t[0].legs[0].i, t[0].exit_i, t[0].open), (first, last, false));
        assert!(x.rsi4[last] > 55.0);
        // min_bars (not from the book) pushes the entry past the warm-up bar
        let t = run(BASIC, &x, 0, last, &Params::from([("min_bars".to_string(), (first + 1) as f64)])).unwrap();
        assert!(t.iter().all(|t| t.legs[0].i > first));
        // 25/15: one add, on the entry bar itself if that close already has RSI4 < 15 (book-matching), else on the
        // first later close with RSI4 < 15 above the 200-day SMA
        assert!(x.rsi4[first] < 15.0, "entry bar is deep enough to test the entry-bar add");
        let later = (first + 1..last).find(|&i| x.rsi4[i] < 15.0 && x.c[i] > x.ma200[i]).expect("slide deep enough");
        let legs = |p: &Params| run(R2515, &x, 0, last, p).unwrap()[0].legs.iter().map(|l| l.i).collect::<Vec<_>>();
        assert_eq!(legs(&Params::new()), vec![first, first]);
        assert_eq!(legs(&Params::from([("add_entry_bar".to_string(), 0.0)])), vec![first, later]);
        let t2 = run(R2515, &x, 0, last, &Params::new()).unwrap();
        assert_eq!(t2[0].exit_i, last);
    }
}
