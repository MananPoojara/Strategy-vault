//! Steve Primo / Connors Group, "Raptor-II Trading System" course (method by Cesar Alvarez): Strategy Guidebook
//! (April 2006, 19 pages), class deck "RAPTOR II" (46 slides), Bonus Research, AmiBroker scan Raptor2_ClassScan.afl,
//! Raptor2_Trades_Variations.xls. Book id "prap2". Library notes and the independent oracle: library/prap2/.
//! A long-only stock method tested on the whole US stock universe 1/3/1995-3/31/2006 (portfolio results only). The
//! vault runs it on one ticker with real daily OHLC + volume: an NSE stock via the basket catalogue (Liquidity Filter
//! benchmark = NIFTY 50 close x volume in place of the S&P 500 x NYSE volume) or an index (instrument swap only).
#![allow(dead_code)]
use crate::connors::*;

pub const BOOK: &str = "prap2";

const RULES: &[&str] = &[
    "F|\"1. Today's closing price of the stock is greater than its 200-day simple moving average (MA).\" (Guidebook p.5)",
    "F|\"2. Today's closing price of the stock is less than its 20-day simple moving average.\"",
    "F|\"3. Today's 2 period RSI is less than 5.\"",
    "F|\"4. The 10 period ADX of the stock is greater than 20\"",
    "F|\"5. The 100-day historical volatility filter is greater than 20%.\" (AFL: 100*StDev(log(C/Ref(C,-1)),100)*sqrt(252); AmiBroker StDev = population)",
    "F|\"6. Liquidity Filter is greater than 3 ... 20-day ma of the Close*Volume of stock is greater than the (65-day ma of the Close*Volume of SP500) times .000003\"",
    "F|\"7. Today's close is 15% or more below the close 5 days ago. OR, today's close is 7.5% or more below the close 2 days ago\" (AFL: ROC(C,5) < -15 OR ROC(C,2) < -15/2)",
    "E|\"8. If all of the above occur, tomorrow you will place a limit buy order 8% below today's close.\" (AFL: filled when L < trigger, at Min(O, trigger))",
    "X|\"9. If you are filled, you will wait for the 2 period RSI to close above 70. When this occurs, you'll exit on the next morning's open.\" (AFL: Sell = RSI(2) > 70 AND BarsSince(Buy) > 0, i.e. from the close after the fill day)",
    "F|\"You will never buy the same stock a second time when you are in it\" (p.9); deck: \"Never, ever add to an already existing stock position\"",
];

pub fn catalogue() -> Vec<StrategyDef> {
    use Status::*;
    let d = |id, name, status, rules, notes, claim| StrategyDef {
        id, name, group: "Raptor-II (2006) · stocks", session: 0, dir: "Long", status, needs_volume: true, needs_vix: false,
        needs_series: &[], book: BOOK, rules, notes, claim,
    };
    vec![
        d("prap2_raptor2", "Raptor-II (RSI(2) < 5 crash buy, 8% limit)", Live, RULES,
          concat!("Meant for the whole US stock universe; the vault runs one ticker (NSE stocks via basket:<list>:<SYMBOL>, or an index). ",
                  "Liquidity Filter = 1,000,000 x MA20(close x volume) / MA65(benchmark close x volume): the book's x .000003 is 'greater than 3'; benchmark NIFTY 50 (index volume) in place of S&P 500 x NYSE volume; tickers without the benchmark (the US server) get no LF and no trades. ",
                  "HV(100) per the course's AFL: population st.dev of 100 ln returns x sqrt(252) x 100. ADX(10) Wilder, RSI(2) Wilder (AmiBroker). ",
                  "The limit is live on the next day only; a later setup places a new order. p.limit = the Variations sheet's 4-10% entries [labelled; default 8 = the rule]. ",
                  "Book results are portfolio-level (10/15/20 slots, highest HV(100) first) on US stocks 1995-2006 and cannot be checked: no US stock data in the vault."),
          claim("thousands (US universe)", "nearly 80%", "15.27% per winning trade", "about 8 days", "1/3/1995-3/31/2006 US stocks: portfolio-10 average compounded annual return 100.4% (deck slide 3); ~16% of limit orders filled (Guidebook p.11)")),
        d("prap2_portfolio", "Raptor-II portfolio (10 / 15 / 20 slots)", NeedsData("the whole US stock universe 1995-2006 on one equity curve with highest-HV(100) tie-break; the engine backtests one ticker per run and holds no US stocks"),
          &["F|\"Divide your account into 10 slots ... If there are more Raptor II Portfolio stock candidates available than open slots allowed, then only place orders on stocks with the highest 100-day Historical Volatility.\" (deck slides 9, 30, 33: 10 / 15 / 20 slots)"],
          "Portfolio layer of prap2_raptor2.", None),
        d("prap2_options", "Raptor-II with call options / spreads", NotTestable("needs option chains (near-month calls, bull-call and bull-put spreads; strike choice partly discretionary: \"you can either guesstimate where you want to buy the options\")"),
          &["E|\"focus on the near month options that are closest to yesterday's closing price ... buy the 50 calls if the stock hits 46 today\" (Guidebook p.14)", "E|Bull-call spread, bull-put spread (deck slides 26-28)"],
          "Options overlay; not testable.", None),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    let f = |v| Some(("prap2", "Raptor-II", v));
    match id {
        "prap2_raptor2" => f("Single stock"),
        "prap2_portfolio" => f("Portfolio · needs a universe"),
        "prap2_options" => f("Options · not testable"),
        _ => None,
    }
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    if id != "prap2_raptor2" {
        return None;
    }
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    Some(vec![
        p("trend", "Close above N-day SMA", 200., 100., 300., 50.),
        p("pull_ma", "Close below N-day SMA", 20., 5., 50., 5.),
        p("rsi_lt", "RSI(2) below", 5., 1., 20., 1.),
        p("adx", "ADX(10) above", 20., 0., 40., 5.),
        p("hv", "HV(100) above", 20., 0., 60., 5.),
        p("lf", "Liquidity Filter above", 3., 0., 40., 1.),
        p("drop5", "Close at least % below the close 5 days ago", 15., 5., 30., 2.5),
        p("drop2", "OR close at least % below the close 2 days ago", 7.5, 2.5, 15., 2.5),
        p("limit", "Buy limit % below the setup close (Variations sheet: 4-10)", 8., 2., 12., 1.),
        p("exit_rsi", "Exit next open after RSI(2) closes above", 70., 50., 90., 5.),
    ])
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if id != "prap2_raptor2" {
        return None;
    }
    let schema = params(id)?;
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).map_or(f64::NAN, |d| d.def));
    let (c, n) = (&x.c, x.c.len());
    let to = to.min(n - 1);
    let trend = x.sma_n(g("trend") as usize);
    let pull = x.sma_n(g("pull_ma") as usize);
    let hv = hv_ddof(c, 100, 0.);
    let r = &x.rsi2;
    let (rlt, adx, hvmin, lf, d5, d2, lim, ex) =
        (g("rsi_lt"), g("adx"), g("hv"), g("lf"), g("drop5"), g("drop2"), 1. - g("limit") / 100., g("exit_rsi"));
    let setup = |i: usize| {
        i >= 5 && c[i] > trend[i] && c[i] < pull[i] && r[i] < rlt && x.adx10[i] > adx && hv[i] > hvmin && x.lf[i] > lf
            && ((c[i] / c[i - 5] - 1.) * 100. < -d5 || (c[i] / c[i - 2] - 1.) * 100. < -d2)
    };
    let (mut out, mut cur, mut order, mut exit): (Vec<Trade>, Option<Trade>, Option<f64>, bool) = (vec![], None, None, false);
    for i in from..=to {
        if exit {
            let mut t = cur.take().unwrap();
            (t.exit_i, t.exit_px) = (i, x.o[i]);
            out.push(t);
            exit = false;
        }
        if let Some(px) = order.take() {
            if x.l[i] < px {
                cur = Some(Trade { dir: 1., legs: vec![Leg { i, px: x.o[i].min(px), w: 1. }], exit_i: i, exit_px: f64::NAN,
                    reason: String::new(), entry_kind: "limit", signal_i: i, open: false, exits: vec![] });
                continue; // the fill day's close does not count for the exit (AFL BarsSince(Buy) > 0)
            }
        }
        if let Some(t) = cur.as_mut() {
            if r[i] > ex && i < to {
                exit = true;
                t.reason = format!("RSI(2) > {}, sold next open", ex);
                t.signal_i = i;
            }
            continue;
        }
        if i < to && setup(i) {
            order = Some(c[i] * lim);
        }
    }
    if let Some(mut t) = cur {
        (t.exit_i, t.exit_px, t.reason, t.signal_i, t.open) = (to, c[to], "open".into(), to, true);
        out.push(t);
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;
    #[test]
    fn crash_setup_limit_fill_and_next_open_exit() {
        // 260 volatile up-trending bars, then a 4-day crash (setup), an 8% gap-down fill, a rally (exit next open).
        let mut px = 50.0;
        let mut r: Vec<Row> = (0..260).map(|i| {
            px *= if i % 3 == 2 { 0.98 } else { 1.03 };
            Row { ticker: "T".into(), date: format!("d{:04}", i), open: px, high: px * 1.01, low: px * 0.99, close: px, volume: Some(1e6) }
        }).collect();
        let mut c = r[259].close;
        for k in 0..4 { c *= 0.94; r.push(Row { date: format!("d{:04}", 260 + k), open: c, high: c, low: c, close: c, ..r[0].clone() }); }
        let s = c; // setup close (bar 263): ~22% below 4 days ago
        r.push(Row { date: "d0264".into(), open: s * 0.95, high: s * 0.96, low: s * 0.90, close: s * 0.91, ..r[0].clone() });
        r.push(Row { date: "d0265".into(), open: s, high: s * 1.2, low: s, close: s * 1.2, ..r[0].clone() });
        r.push(Row { date: "d0266".into(), open: s * 1.15, high: s * 1.2, low: s * 1.1, close: s * 1.1, ..r[0].clone() });
        let x = Ctx::new(&r, &r, &[]);
        let t = run("prap2_raptor2", &x, 0, 266, &Params::new()).unwrap();
        let t = t.iter().find(|t| t.legs[0].i == 264).expect("fill the day after the setup");
        assert!((t.legs[0].px - s * 0.92).abs() < 1e-9, "limit 8% below the setup close (open is above it)");
        assert_eq!((t.exit_i, t.exit_px), (266, s * 1.15));
    }
}
