//! Steven Primo / Connors Research Group, "The Alvarez Factor" (TradingMarkets course, workbook dated
//! April 9, 2005; method by Cesar Alvarez). Book id "afactor".
//! Sources: workbook 4_9final.pdf (rules p.13-14), Alvarez Stats.xls (sheet "Variations": the test spec and
//! baseline results 2000-2004), Alvarez Trades.xls, AmiBroker "Alvarez Night Scan.afl" / "Alvarez Night Day
//! and Night.afl" (indicator formulas). Library notes and the independent oracle: library/afactor/.
//! A single-stock day trade: runs on any ticker with real daily OHLC + volume (NSE stocks via the basket
//! catalogue, e.g. `basket:nifty50:RELIANCE`). The book's universe was US stocks 2000-2004 (not in the vault).
use crate::connors::{hv_ddof, sma, Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "afactor";

const RULES: &[&str] = &[
    "F|\"1. Yesterday's closing price must be above 10.\" (workbook p.13; Stats.xls: \"previous close > 10\")",
    "F|\"2. Average Volume for the past 20 days must be at least 100,000 shares\" (p.13); Stats.xls and Night Scan AFL: MA(Volume,20) > 100000, on yesterday's bar",
    "F|\"3. Yesterday's close of the stock must be down.\" (p.13) = yesterday's close < the close the day before",
    "F|\"4. Yesterday's close of the stock must be below it's open.\" (p.13)",
    "F|\"5. Range-Yesterday's high minus its low must be at least 1.1 times greater than its previous 10 days Average True Range\" (p.13); Stats.xls: \"Previous day's High minus previous day's Low is 1.1 times greater than the 10 day ATR 2 days ago\" (strict >, AFL)",
    "F|\"6. Yesterday's 100-day Historical Volatility must be above 20\" (p.13); AFL: StDev(ln(C/C[-1]),100) x 1587.45 (AmiBroker population StDev)",
    "F|\"7. The 2-period smoothed RSI must be less than 60\" (p.14), on yesterday's close (Stats.xls Setup; Night Scan AFL)",
    "E|\"8. ... Today's open must be at least 3% less than the previous days close.\" (p.14); AFL: O < C[-1] and |O - C[-1]| / C[-1] >= .03",
    "E|\"9. The entry is .25%-.75% above the open. Orders should be placed as buy stop market orders.\" Baseline: \"Enter .25% above the open\" (Stats.xls); filled if today's high > open x 1.0025 (AFL C10 uses strict >), at the stop price",
    "X|\"10. Exit on the close.\" (p.14) - same day",
];

pub fn catalogue() -> Vec<StrategyDef> {
    let d = |id, name, status, rules, notes, claim| StrategyDef {
        id, name, group: "The Alvarez Factor · gap-down day trade", session: 0, dir: "Long", status,
        needs_volume: true, needs_vix: false, needs_series: &[], book: BOOK, rules, notes, claim,
    };
    vec![
        d("afactor_baseline", "The Alvarez Factor (baseline)", Status::Live, RULES,
          "Book tested US stocks 2000-01-03 to 2004-12-31 in AmiBroker (Alvarez Trades.xls is an AmiBroker trade export): 8,289 trades, 62.9% winners, +2.22% average, PF 2.42. \
The US stock universe is not in the vault, so the book's numbers cannot be reproduced; run it on an NSE stock (basket tickers, real native daily OHLC + volume) - India = instrument swap only, thresholds kept literal (price > 10 in rupees on the vendor's adjusted prices, 100,000 shares). \
Source conflicts, defaults follow the workbook + the Stats.xls test spec that produced the published results: ATR(10) of the day BEFORE the setup day ('2 days ago'); the AmiBroker Night Scan uses ATR(10) including the setup day (param atr_on_setup = 1). \
The AFL 'Day and Night' exploration enters at open + 0.15% and tests C > 10 / RSI(2) / ATR on TODAY's bar (it reads today's close, a hindsight listing) - not used. TradeStation .ELD files are compressed binaries and could not be read. \
Indicators: Wilder ATR(10) and RSI(2) (AmiBroker), HV with population stdev. One trade per signal day, in and out the same day. On a cash index a 3% gap down with all filters is rare: zero trades is an honest answer.",
          crate::connors::claim("8,289", "62.9%", "2.22%", "same day", "2000-2004, US stocks (AmiBroker), PF 2.42")),
        d("afactor_travel_high", "Alvarez Factor · exit at the day's high (travel range)",
          Status::NotTestable("hindsight: the exit is the day's high, which is unknown until the close (a research statistic, not a tradable rule)"),
          &["E|Baseline entry (rules 1-9)", "X|\"Exit on High of Day\" (workbook p.51-53: 3% lap 6.28%, 4% lap 7.24%, 5% lap 8.17% average)"],
          "Alvarez Trades.xls holds exactly this list: 7,230 trades 2000-2004, average 6.28%, 99.96% > 0. It measures the 'travel range', not a strategy.", None),
        d("afactor_portfolio", "Alvarez Factor · daily portfolio",
          Status::NeedsData("a whole stock universe on one equity curve: the engine backtests one ticker per run, and the book's US stock universe 2000-2004 is not in the vault"),
          &["E|Baseline signals across all stocks each day",
            "F|\"If there are less than 10 triggers for the day, invest 1/10 of portfolio in each stock.\" (Stats.xls, MonthlyHistogram)",
            "F|\"If there are 10 or more triggers, then the return of the portfolio that day is the average return of all stocks that triggered that day.\""],
          "Book: CAGR 309%, 96.7% months profitable, max DD -20.6%, 29-day max draw length (Stats.xls MonthlyBaseline, 2000-2004). Needs a cross-sectional runner (see library/afactor/README.md design note).", None),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    let f = ("afactor", "The Alvarez Factor");
    match id {
        "afactor_baseline" => Some((f.0, f.1, "Baseline · exit on the close")),
        "afactor_travel_high" => Some((f.0, f.1, "Exit at the day's high · hindsight")),
        "afactor_portfolio" => Some((f.0, f.1, "Daily portfolio · needs a universe")),
        _ => None,
    }
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    (id == "afactor_baseline").then(|| vec![
        p("price", "Yesterday's close above (book 10; Stats.xls tests 20, 40)", 10., 5., 40., 5.),
        p("vol", "20-day average volume above (book 100,000; tests 250k/500k/1M)", 100_000., 100_000., 1_000_000., 50_000.),
        p("prev_down", "Yesterday's close down at least X% (book: any down close = 0; tests 1-4%)", 0., 0., 4., 1.),
        p("atr_mult", "Yesterday's range > X x ATR(10) (book 1.1; tests 1.25-2)", 1.1, 1.0, 2.0, 0.05),
        p("atr_on_setup", "ATR(10) incl. the setup day: 0 = book/Stats '2 days ago', 1 = AFL Night Scan", 0., 0., 1., 1.),
        p("hv", "HV(100) above (book 20; tests 30-50)", 20., 20., 50., 10.),
        p("rsi", "RSI(2) below (book 60; tests 40, 20, 10)", 60., 10., 60., 10.),
        p("lap", "Gap down at least % (book 3; tests 2-6)", 3., 2., 6., 1.),
        p("entry", "Buy stop % above the open (book .25; workbook range .25-.75; tests .10-1.00; AFL .15)", 0.25, 0.1, 1.0, 0.05),
    ])
}

/// One trade per qualifying day: buy stop above today's open, sell on today's close.
pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if id != "afactor_baseline" { return None; }
    let schema = params(id)?;
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).unwrap().def);
    let (price, vol_min, pdown, mult, hv_min, rsi_max) = (g("price"), g("vol"), g("prev_down") / 100., g("atr_mult"), g("hv"), g("rsi"));
    let (lap, entry, lag) = (g("lap") / 100., g("entry") / 100., if g("atr_on_setup") >= 0.5 { 0 } else { 1 });
    let vol: Vec<f64> = x.r.iter().map(|r| r.volume.unwrap_or(f64::NAN)).collect();
    let avg_vol = sma(&vol, 20);
    let hv = hv_ddof(&x.c, 100, 0.0); // AmiBroker StDev = population
    let mut out = vec![];
    for i in from.max(3)..=to.min(x.r.len() - 1) {
        let s = i - 1; // setup day = yesterday
        let setup = x.c[s] > price && avg_vol[s] > vol_min && x.c[s] < x.c[s - 1] * (1. - pdown) && x.c[s] < x.o[s]
            && x.h[s] - x.l[s] > mult * x.atr10[s - lag] && hv[s] > hv_min && x.rsi2[s] < rsi_max;
        let gap = x.o[i] < x.c[s] && (x.c[s] - x.o[i]) / x.c[s] >= lap;
        let stop = x.o[i] * (1. + entry);
        if setup && gap && x.h[i] > stop {
            out.push(Trade { dir: 1., legs: vec![Leg { i, px: stop, w: 1. }], exit_i: i, exit_px: x.c[i],
                reason: "Exit on the close".into(), entry_kind: "stop", signal_i: i, open: false, exits: vec![] });
        }
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;
    // 130 alternating bars (HV ~ high, RSI(2) ~ mid), then a wide red down day and a 4% gap down.
    fn rows() -> Vec<Row> {
        let mut r: Vec<Row> = (0..130).map(|i| {
            let c = if i % 2 == 0 { 100. } else { 103. };
            Row { ticker: "T".into(), date: format!("2000-{:03}", i), open: c, high: c + 1., low: c - 1., close: c, volume: Some(200_000.) }
        }).collect();
        r[128] = Row { open: 103., high: 104., low: 94., close: 95., ..r[128].clone() }; // down close, red, range 10
        r[129] = Row { open: 91., high: 93., low: 90., close: 92.5, ..r[129].clone() }; // gap 4.2%
        r
    }
    #[test]
    fn hand_checked_gap_down_day_trade() {
        let r = rows();
        let x = Ctx::new(&r, &r, &[]);
        let t = run("afactor_baseline", &x, 0, 129, &Params::new()).unwrap();
        assert_eq!(t.len(), 1);
        assert_eq!((t[0].legs[0].i, t[0].exit_i), (129, 129));
        assert!((t[0].legs[0].px - 91. * 1.0025).abs() < 1e-9 && t[0].exit_px == 92.5);
        // stop not reached -> no trade; gap 2.9% -> no trade; volume filter
        let mut r2 = r.clone(); r2[129].high = 91.2; r2[129].close = 91.;
        assert!(run("afactor_baseline", &Ctx::new(&r2, &r2, &[]), 0, 129, &Params::new()).unwrap().is_empty());
        let mut r3 = r.clone(); r3[129].open = 95. * 0.971; r3[129].low = 90.;
        assert!(run("afactor_baseline", &Ctx::new(&r3, &r3, &[]), 0, 129, &Params::new()).unwrap().is_empty());
        let p = Params::from([("vol".to_string(), 250_000.)]);
        assert!(run("afactor_baseline", &x, 0, 129, &p).unwrap().is_empty());
        assert!(run("afactor_travel_high", &x, 0, 129, &Params::new()).is_none());
    }
}
