//! Larry Connors / The Connors Group, "The TradingMarkets R3/R4 Momentum Method" (Guidebook, February 2007,
//! updated 2/28/07, 30 pages; with Steve Primo's class slide deck "R3/R4 Method" = R3_R4_BASIC.pdf, 68 slides).
//! Book id "r3r4". Library notes and the independent oracle: library/r3r4/.
//! A long-only stock swing method. Each version runs on one ticker with real daily OHLC + volume (NSE stocks via
//! the basket catalogue, e.g. `basket:nifty50:RELIANCE`; the Liquidity Filter's benchmark is NIFTY 50 close x
//! volume in place of the S&P 500 x NYSE volume). The book's universe (6,000+ US stocks 1995-2006) is not in the vault.
use crate::connors::{claim, sma, Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "r3r4";

/// One version of the method and its book parameters.
struct Ver {
    id: &'static str,
    trend: f64,
    drops: f64,
    window: f64,
    lf: f64,
    lf_max: f64, // 0 = no upper bound
    limit: f64,
    price: bool, // $1 < close < $15
    scale: bool,
}

const VERS: &[Ver] = &[
    Ver { id: "r3r4_base", trend: 200., drops: 3., window: 4., lf: 3., lf_max: 0., limit: 5., price: false, scale: false },
    Ver { id: "r3r4_base_scale", trend: 200., drops: 3., window: 4., lf: 3., lf_max: 0., limit: 5., price: false, scale: true },
    Ver { id: "r3r4_bigcap", trend: 200., drops: 4., window: 5., lf: 20., lf_max: 0., limit: 5., price: false, scale: false },
    Ver { id: "r3r4_bigcap_scale", trend: 200., drops: 4., window: 5., lf: 20., lf_max: 0., limit: 5., price: false, scale: true },
    Ver { id: "r3r4_bigcap_deck", trend: 300., drops: 3., window: 4., lf: 20., lf_max: 0., limit: 3., price: false, scale: false },
    Ver { id: "r3r4_bigcap_deck_scale", trend: 300., drops: 3., window: 4., lf: 20., lf_max: 0., limit: 3., price: false, scale: true },
    Ver { id: "r3r4_midcap", trend: 300., drops: 3., window: 4., lf: 10., lf_max: 30., limit: 3., price: false, scale: false },
    Ver { id: "r3r4_midcap_scale", trend: 300., drops: 3., window: 4., lf: 10., lf_max: 30., limit: 3., price: false, scale: true },
    Ver { id: "r3r4_lowprice", trend: 200., drops: 3., window: 4., lf: 3., lf_max: 0., limit: 5., price: true, scale: false },
    Ver { id: "r3r4_lowprice_scale", trend: 200., drops: 3., window: 4., lf: 3., lf_max: 0., limit: 5., price: true, scale: true },
];

const BASE: &[&str] = &[
    "F|\"1. The Liquidity Filter reading should be greater than 3\" (Guide p.6; Appendix A p.28: 20-day MA of Close*Volume of the stock > 65-day MA of Close*Volume of the S&P 500 (NYSE volume) x .000003)",
    "F|\"2. The stock should close above it's 200-day simple moving average (MA).\"",
    "F|\"3. The 2-period RSI has dropped 3 out of the last 4 days.\"",
    "F|\"4. The 2-period RSI is less than 10.\"",
    "F|\"5. The 10-period ADX is greater than 20.\"",
    "F|\"6. The 100-day historical volatility (HV) is greater than 30.\"",
    "E|\"7. Enter the next day if the stock reaches 5% or more below the previous day's close.\" (deck: \"Place Buy Limit Order 5% below close\")",
    "X|\"If the close of the stock is above the 5-day moving average, exit at the next open.\"",
];
const BIG: &[&str] = &[
    "F|\"1. The Liquidity Filter reading should be greater than 20\" (Guide p.7; Appendix B p.29: x .000020)",
    "F|\"2. The stock should close above it's 200-day simple moving average (MA).\"",
    "F|\"3. The 2-period RSI has dropped 4 out of the last 5 days.\"",
    "F|\"4. The 2-period RSI is less than 10.\"",
    "F|\"5. The 10-period ADX is greater than 20.\"",
    "F|\"6. The 100-day historical volatility (HV) is greater than 30.\"",
    "E|\"7. Enter the next day if the stock reaches 5% or more below the previous day's close.\"",
    "X|\"If the close of the stock is above the 5-day moving average, exit at the next open.\"",
];
const BIG_DECK: &[&str] = &[
    "F|\"1. Stock closes above its 300-period SMA\" (deck, R3/R4 \"BIG CAP\" SETUP RULES)",
    "F|\"2. The 100-day Historical Volatility is greater than 30\"",
    "F|\"3. The 10-period ADX is greater than 20\"",
    "F|\"4. The Liquidity Filter is greater than 20\"",
    "F|\"5. The 2-period RSI has dropped 3 out of the last 4 days\"",
    "F|\"6. The 2-period RSI close is less than 10\"",
    "E|\"Entry = Place Buy Limit Order 3% below close.\"",
    "X|\"Exit = if stock's close is greater than its 5-day MA, exit on the opening of next day.\"",
];
const MID: &[&str] = &[
    "F|\"1. Stock closes above its 300-period SMA\" (deck, R3/R4 \"MID CAP\" SETUP RULES; not in the Guide)",
    "F|\"2. The 100-day Historical Volatility is greater than 30\"",
    "F|\"3. The 10-period ADX is greater than 20\"",
    "F|\"4. The Liquidity Filter is greater than 10 and less than 30\"",
    "F|\"5. The 2-period RSI has dropped 3 out of the last 4 days\"",
    "F|\"6. The 2-period RSI close is less than 10\"",
    "E|\"Entry = Place Buy Limit Order 3% below close.\"",
    "X|\"Exit = if stock's close is greater than its 5-day MA, exit on the opening of next day.\"",
];
const LOW: &[&str] = &[
    "F|\"1. The Liquidity Filter reading should be greater than 3\" (Guide p.8)",
    "F|\"2. The stock should close above it's 200-day simple moving average (MA).\"",
    "F|\"3. The closing price of the stock should be greater than $1 and less than $15.\"",
    "F|\"4. The 2-period RSI is less than 10.\" \"5. The 10-period ADX is greater than 20.\" \"6. The 100-day historical volatility (HV) is greater than 30.\"",
    "F|\"9. The 2-period RSI has dropped 3 out of the last 4 days.\"",
    "E|\"7. Enter the next day if the stock reaches 5% or more below the previous day's close.\"",
    "X|\"If the close of the stock is above the 5-day moving average, exit at the next open.\"",
];
const SCALE: &str = "A|\"Scale-In Rule: If at any time during an open position there is another TradingMarkets R3/R4 Momentum Method setup, then enter at the open the next day. Only scale in once into any open position.\" (Guide p.9) Setup = rules 1-6 on a close while the position is open (the fill day's close included; p.add_next_day = 1 skips it [Not from the book]); the add is a market order at the next open; an exit signal on the same close wins";

const NOTE: &str = "Indicators as in the Swing Trading College (stc9) engine: Wilder RSI(2), Wilder ADX(10), HV(100) = sample stdev of ln returns x sqrt(252) x 100, Liquidity Filter = 1,000,000 x MA20(close x volume) / MA65(benchmark close x volume), so 'greater than 3' = the appendix's .000003. \
India mapping: run on an NSE stock (basket tickers); the benchmark is NIFTY 50 close x index volume (index volume starts 2000) in place of the S&P 500 x NYSE volume; the $1/$15 price band is kept literal in rupees. \
'Dropped 3 out of the last 4 days' = RSI(2) closed below the prior day's RSI(2) on at least 3 of the last 4 days, today included (4 of 4 also qualifies) [Read: the book gives no tie-break]. \
Entry: buy limit at the setup close x (1 - 5%), live the next day only, filled at min(open, limit). Exit: first close above the 5-day SMA (the fill day's close included), sold at the next open. One position per ticker. \
Book test: 6,000+ US stocks, open-to-open, 1/1/1995-12/31/2006, 1 cent commissions, 3-month T-bill on cash, 10 or 20 slots, highest HV(100) first when signals exceed slots - portfolio results only (see r3r4_portfolio). On a broad index HV(100) > 30 is rare: zero trades is an honest answer.";

pub fn catalogue() -> Vec<StrategyDef> {
    // Aggressive versions = the same rules + the scale-in line (built once).
    const ALL: [&[&str]; 5] = [BASE, BIG, BIG_DECK, MID, LOW];
    static S: std::sync::OnceLock<Vec<Vec<&'static str>>> = std::sync::OnceLock::new();
    let s: &'static Vec<Vec<&'static str>> = S.get_or_init(|| ALL.iter().map(|r| [*r, &[SCALE][..]].concat()).collect());
    let scaled = |k: usize| -> &'static [&'static str] { &s[k][..] };
    let d = |id, name, rules, c| StrategyDef {
        id, name, group: "R3/R4 Momentum Method · stocks", session: 0, dir: "Long", status: Status::Live,
        needs_volume: true, needs_vix: false, needs_series: &[], book: BOOK, rules, notes: NOTE, claim: c,
    };
    let base = || claim("thousands of set-ups", "nearly 75%", "-", "-", "Guide p.4 (US stocks 1995-2006): portfolio 10 slots CAGR 258.68%, max DD (daily) -43.34%, 88% winning months, Sharpe 5.42 (Table 1 p.10)");
    vec![
        d("r3r4_base", "R3/R4 Base Rules", BASE, base()),
        d("r3r4_base_scale", "R3/R4 Base Rules · Aggressive (scale-in)", scaled(0),
          claim("-", "-", "-", "-", "Guide p.5: aggressive version 410.42% compounded annual return, profitable 88% of months, Sharpe 6.64 (US stocks 1995-2006, 10 slots)")),
        d("r3r4_bigcap", "R3/R4 Big Cap Rules (Guide)", BIG, None),
        d("r3r4_bigcap_scale", "R3/R4 Big Cap Rules (Guide) · Aggressive", scaled(1), None),
        d("r3r4_bigcap_deck", "R3/R4 Big Cap Rules (class deck)", BIG_DECK, None),
        d("r3r4_bigcap_deck_scale", "R3/R4 Big Cap Rules (class deck) · Aggressive", scaled(2), None),
        d("r3r4_midcap", "R3/R4 Mid Cap Rules (class deck)", MID, None),
        d("r3r4_midcap_scale", "R3/R4 Mid Cap Rules (class deck) · Aggressive", scaled(3), None),
        d("r3r4_lowprice", "R3/R4 Low Priced Rules", LOW, None),
        d("r3r4_lowprice_scale", "R3/R4 Low Priced Rules · Aggressive", scaled(4), None),
        StrategyDef {
            notes: "Portfolio-level results only (Tables 1-12, Guide p.10-21; deck slides 21-68): they need every US stock 1995-2006.",
            status: Status::NeedsData("a US stock universe (6,000+ NYSE/NASDAQ/AMEX stocks 1995-2006) on one equity curve: the engine backtests one ticker per run"),
            ..d("r3r4_portfolio", "R3/R4 portfolio (10 / 20 slots)", &[
                "F|\"8. Buy a maximum of 10 positions (10% per position).\" (Guide p.6); 20-slot tests in Tables 3-4, 7-8, 11-12",
                "F|\"If you have too many set-ups for slots available, only select the stocks which have the highest HV's.\" (Guide p.9)",
            ], None)
        },
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    let f = |v| Some(("r3r4", "R3/R4 Momentum Method", v));
    match id {
        "r3r4_base" => f("Base · 10 slots"),
        "r3r4_base_scale" => f("Base · aggressive scale-in"),
        "r3r4_bigcap" => f("Big Cap (Guide: 4 of 5 drops, 5% limit)"),
        "r3r4_bigcap_scale" => f("Big Cap (Guide) · aggressive"),
        "r3r4_bigcap_deck" => f("Big Cap (deck: 300 SMA, 3% limit)"),
        "r3r4_bigcap_deck_scale" => f("Big Cap (deck) · aggressive"),
        "r3r4_midcap" => f("Mid Cap (deck)"),
        "r3r4_midcap_scale" => f("Mid Cap (deck) · aggressive"),
        "r3r4_lowprice" => f("Low Priced ($1-$15)"),
        "r3r4_lowprice_scale" => f("Low Priced · aggressive"),
        "r3r4_portfolio" => f("Portfolio · needs a universe"),
        _ => None,
    }
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let v = VERS.iter().find(|v| v.id == id)?;
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    let mut out = vec![
        p("lf", "Liquidity Filter above", v.lf, 1., 40., 1.),
        p("trend", "Close above N-day SMA", v.trend, 100., 300., 50.),
        p("drops", "RSI(2) dropped on at least N days...", v.drops, 2., 5., 1.),
        p("window", "...of the last N days", v.window, 3., 6., 1.),
        p("rsi_lt", "RSI(2) below", 10., 2., 30., 1.),
        p("adx", "ADX(10) above", 20., 10., 40., 5.),
        p("hv", "HV(100) above", 30., 10., 60., 5.),
        p("limit", "Buy limit % below the setup close", v.limit, 1., 10., 1.),
        p("exit_ma", "Exit after a close above N-day SMA", 5., 3., 10., 1.),
    ];
    if v.lf_max > 0. {
        out.push(p("lf_max", "Liquidity Filter below", v.lf_max, 15., 60., 5.));
    }
    if v.scale {
        out.push(p("add_next_day", "Scale-in setups counted only from the day after the fill (0 = book 'at any time during an open position', fill-day close included) [Not from the book]", 0., 0., 1., 1.));
    }
    if v.price {
        out.push(p("price_min", "Close above ($ in the book; rupees as printed)", 1., 1., 10., 1.));
        out.push(p("price_max", "Close below ($ in the book; rupees as printed)", 15., 5., 50., 5.));
    }
    Some(out)
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    let v = VERS.iter().find(|v| v.id == id)?;
    let schema = params(id)?;
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).map_or(f64::NAN, |d| d.def));
    let (c, n) = (&x.c, x.c.len());
    let trend = sma(c, g("trend") as usize);
    let ma = sma(c, g("exit_ma") as usize);
    let (need, win) = (g("drops") as usize, g("window") as usize);
    let (lf, lf_max) = (g("lf"), if v.lf_max > 0. { g("lf_max") } else { f64::INFINITY });
    let (pmin, pmax) = if v.price { (g("price_min"), g("price_max")) } else { (f64::NEG_INFINITY, f64::INFINITY) };
    let (rlt, adx, hv, lim) = (g("rsi_lt"), g("adx"), g("hv"), 1. - g("limit") / 100.);
    let rs = &x.rsi2;
    let setup = |i: usize| {
        i >= win
            && (i + 1 - win..=i).filter(|&k| rs[k] < rs[k - 1]).count() >= need
            && x.lf[i] > lf && x.lf[i] < lf_max && c[i] > trend[i] && rs[i] < rlt && x.adx10[i] > adx && x.hv100[i] > hv
            && c[i] > pmin && c[i] < pmax
    };
    let w0 = if v.scale { 0.5 } else { 1. };
    let (mut out, mut cur): (Vec<Trade>, Option<Trade>) = (vec![], None);
    let (mut lim_px, mut add, mut exit) = (None::<f64>, false, false);
    for i in from..=to.min(n - 1) {
        if exit {
            let mut t = cur.take().unwrap();
            (t.exit_i, t.exit_px) = (i, x.o[i]);
            out.push(t);
            exit = false;
        }
        if add {
            cur.as_mut().unwrap().legs.push(Leg { i, px: x.o[i], w: 0.5 });
            add = false;
        }
        if let Some(px) = lim_px.take() {
            if x.l[i] <= px {
                cur = Some(Trade { dir: 1., legs: vec![Leg { i, px: x.o[i].min(px), w: w0 }], exit_i: i, exit_px: f64::NAN,
                    reason: String::new(), entry_kind: "limit", signal_i: i, open: false, exits: vec![] });
            }
        }
        if let Some(t) = cur.as_mut() {
            if c[i] > ma[i] {
                if i < to {
                    exit = true;
                    t.reason = format!("Close > {}-day SMA, sold next open", g("exit_ma"));
                    t.signal_i = i;
                }
            } else if v.scale && t.legs.len() == 1 && i < to && (g("add_next_day") < 0.5 || t.legs[0].i < i) && setup(i) {
                add = true;
            }
            continue;
        }
        if i < to && setup(i) {
            lim_px = Some(c[i] * lim);
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
    // 320 bars of a strong zig-zag uptrend (HV, ADX high), then 4 down days -> setup, a 5% dip fill, then a rally.
    fn rows() -> Vec<Row> {
        let mut px = 50.0;
        let mut r: Vec<Row> = (0..320).map(|i| {
            px *= if i % 2 == 0 { 1.04 } else { 0.99 };
            Row { ticker: "T".into(), date: format!("d{:04}", i), open: px, high: px * 1.01, low: px * 0.99, close: px, volume: Some(1e6) }
        }).collect();
        let mut c = r[319].close;
        for k in 0..4 { c *= 0.97; r.push(Row { date: format!("d{:04}", 320 + k), open: c, high: c * 1.005, low: c * 0.995, close: c, ..r[0].clone() }); }
        let s = c; // setup close (bar 323)
        r.push(Row { date: "d0324".into(), open: s * 0.99, high: s, low: s * 0.94, close: s * 0.95, ..r[0].clone() }); // limit s*0.95 fills
        r.push(Row { date: "d0325".into(), open: s * 1.2, high: s * 1.3, low: s * 1.2, close: s * 1.3, ..r[0].clone() }); // close > MA5
        r.push(Row { date: "d0326".into(), open: s * 1.25, high: s * 1.3, low: s * 1.2, close: s * 1.25, ..r[0].clone() }); // exit at open
        r
    }
    #[test]
    fn hand_checked_limit_entry_and_ma_exit() {
        let r = rows();
        let x = Ctx::new(&r, &r, &[]); // benchmark = itself -> LF ~ 1e6
        let s = r[323].close;
        let t = run("r3r4_base", &x, 0, 326, &Params::new()).unwrap();
        let t = t.iter().find(|t| t.legs[0].i == 324).expect("trade from the d0323 setup");
        assert!((t.legs[0].px - s * 0.95).abs() < 1e-9, "limit at 5% below the setup close");
        assert_eq!((t.exit_i, t.exit_px), (326, s * 1.25));
        // limit 10% below: not reached -> no trade from that setup
        let p = Params::from([("limit".to_string(), 10.)]);
        assert!(run("r3r4_base", &x, 0, 326, &p).unwrap().iter().all(|t| t.legs[0].i != 324));
        assert!(run("r3r4_portfolio", &x, 0, 326, &Params::new()).is_none());
    }
}
