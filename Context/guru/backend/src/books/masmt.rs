//! Leslie N. Masonson, All About Market Timing (McGraw-Hill 2004). Only the rules the vault does not
//! already hold: the VL4% model is `mbsot_zweig4`/`mzwws_four_percent` and the Hirsch Best Six Months is
//! `hsta_best6`, so neither is re-coded. Page refs are printed pages (PDF page = printed + 28).
//! Every fill is at the close of the signal day (Masonson's TradeStation tests: "All investments were
//! made the same day as the moving average crossover signal was given", p.143). Cash earns nothing and
//! margin costs nothing (the book's T-bill/margin assumptions are not modelled). Sources: library/masmt/.
use crate::books::apta::bars;
use crate::connors::{claim, sma, Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "masmt";

// ------------------------------------------------------------------ dates

fn ymd(s: &str) -> (i64, u32, u32) {
    (s[0..4].parse().unwrap_or(0), s[5..7].parse().unwrap_or(0), s[8..10].parse().unwrap_or(0))
}
fn ym(x: &Ctx, i: usize) -> (i64, u32) {
    let (y, m, _) = ymd(&x.r[i].date);
    (y, m)
}
/// Bars of calendar month (y, m); None unless a later month exists (the month is complete).
fn month_bars(x: &Ctx, y: i64, m: u32) -> Option<std::ops::Range<usize>> {
    let a = x.r.partition_point(|r| r.date.as_str() < format!("{y:04}-{m:02}-01").as_str());
    let (ny, nm) = if m == 12 { (y + 1, 1) } else { (y, m + 1) };
    let b = x.r.partition_point(|r| r.date.as_str() < format!("{ny:04}-{nm:02}-01").as_str());
    (a < b && b < x.r.len()).then_some(a..b)
}
/// First bar dated on or after y-m-d ("or the next closest trading day if on that day the market was closed", p.114).
fn on_or_after(x: &Ctx, y: i64, m: u32, d: u32) -> Option<usize> {
    let k = x.r.partition_point(|r| r.date.as_str() < format!("{y:04}-{m:02}-{d:02}").as_str());
    (k < x.r.len()).then_some(k)
}
/// US presidential cycle: 0 election, 1 post-election, 2 midterm, 3 pre-election year.
fn cyc(y: i64) -> i64 {
    y.rem_euclid(4)
}

// ------------------------------------------------------------------ trades

fn mk(dir: f64, x: &Ctx, i: usize, w: f64) -> Trade {
    Trade { dir, legs: vec![Leg { i, px: x.c[i], w }], exit_i: i, exit_px: x.c[i], reason: String::new(),
        entry_kind: "close", signal_i: i, open: false, exits: vec![] }
}
fn close_at(mut t: Trade, x: &Ctx, i: usize, to: usize, why: &str) -> Trade {
    let k = i.min(to);
    (t.exit_i, t.exit_px, t.signal_i) = (k, x.c[k], k);
    t.open = i > to;
    t.reason = if t.open { "open".into() } else { why.into() };
    t
}

/// Close-to-close windows (entry bar, exit bar or None = not yet known, direction, weight).
/// Entries outside [from, to) are dropped; an exit after `to` leaves the trade open, marked at `to`.
fn windows(x: &Ctx, from: usize, to: usize, w: Vec<(usize, Option<usize>, f64, f64)>, why: &str) -> Vec<Trade> {
    let mut out: Vec<Trade> = vec![];
    for (e, xi, dir, wt) in w {
        if e < from || e >= to || out.last().is_some_and(|t| e < t.exit_i) || xi.is_some_and(|k| k <= e) {
            continue;
        }
        out.push(close_at(mk(dir, x, e, wt), x, xi.unwrap_or(usize::MAX), to, why));
    }
    out
}

/// Position events -> trades. `ev[i]` = Some((dir, why)) when the rule's position becomes `dir`
/// (+1 long, -1 short, 0 cash) at bar i's close; an event equal to the current rule position is a
/// no-op ("if the other strategy issued a buy signal while the current buy signal was in place, no action
/// would be necessary", p.171). Rule state runs from bar 0; a position the rules already hold at
/// `from` is not reported (flat until the next event). `stop` > 0: optional loss stop of `stop`%
/// from the entry (p.135 "limit your loss to a maximum 10% on each trade and if you are stopped out,
/// just sit tight and wait for the next signal"): checked on later bars, gap through = open.
fn run_events(x: &Ctx, from: usize, to: usize, ev: &[Option<(i8, String)>], stop: f64) -> Vec<Trade> {
    let (mut out, mut cur, mut virt) = (vec![], None::<Trade>, 0i8);
    for i in 0..=to {
        if stop > 0.0 {
            if let Some(t) = cur.as_ref().filter(|t| i > t.legs[0].i) {
                let e = t.legs[0].px;
                let hit = if t.dir > 0.0 {
                    let s = e * (1.0 - stop / 100.0);
                    (x.l[i] <= s).then(|| x.o[i].min(s))
                } else {
                    let s = e * (1.0 + stop / 100.0);
                    (x.h[i] >= s).then(|| x.o[i].max(s))
                };
                if let Some(px) = hit {
                    let mut t = cur.take().unwrap();
                    (t.exit_i, t.exit_px, t.signal_i, t.reason) = (i, px, i, format!("{stop}% loss stop"));
                    out.push(t);
                }
            }
        }
        if let Some((d, why)) = &ev[i] {
            if *d != virt {
                if let Some(t) = cur.take() {
                    out.push(close_at(t, x, i, to, why));
                }
                virt = *d;
                if *d != 0 && i >= from && i < to {
                    cur = Some(mk(*d as f64, x, i, 1.0));
                }
            }
        }
    }
    if let Some(t) = cur {
        out.push(close_at(t, x, usize::MAX, to, ""));
    }
    out
}

/// Percentage-reversal model on the closes of bars `idx`: buy when a close is >= `up`% above the lowest
/// close since the last sell (or the start), sell when a close is <= `dn`% below the highest close since
/// the last buy (or the start). Returns (bar, +1 / -1).
pub fn swing_events(c: &[f64], idx: &[usize], up: f64, dn: f64) -> Vec<(usize, i8)> {
    let (u, d) = (1.0 + up / 100.0, 1.0 - dn / 100.0);
    let mut ev = vec![];
    let Some(&i0) = idx.first() else { return ev };
    let (mut st, mut lo, mut hi) = (0i8, c[i0], c[i0]);
    for &i in &idx[1..] {
        let v = c[i];
        if st <= 0 && v >= lo * u {
            ev.push((i, 1));
            (st, hi) = (1, v);
        } else if st >= 0 && v <= hi * d {
            ev.push((i, -1));
            (st, lo) = (-1, v);
        } else {
            lo = lo.min(v);
            hi = hi.max(v);
        }
    }
    ev
}

/// Crossings of `a` over `b` on bars `idx` (values indexed by position in idx): +1 when a rises from
/// <= b to > b, -1 when it falls from >= b to < b.
pub fn cross_events(idx: &[usize], a: &[f64], b: &[f64]) -> Vec<(usize, i8)> {
    (1..idx.len())
        .filter(|&k| a[k].is_finite() && b[k].is_finite() && a[k - 1].is_finite() && b[k - 1].is_finite())
        .filter_map(|k| {
            if a[k] > b[k] && a[k - 1] <= b[k - 1] {
                Some((idx[k], 1))
            } else if a[k] < b[k] && a[k - 1] >= b[k - 1] {
                Some((idx[k], -1))
            } else {
                None
            }
        })
        .collect()
}

fn to_ev(n: usize, evs: &[(usize, i8)], short: bool, up: &str, dn: &str) -> Vec<Option<(i8, String)>> {
    let mut v = vec![None; n];
    for &(i, d) in evs {
        v[i] = Some(if d > 0 { (1, up.to_string()) } else { (if short { -1 } else { 0 }, dn.to_string()) });
    }
    v
}

/// Month-by-month allocation (Vakkur's monthly-return tests, Tables 7-4 / 8-3): weight `w(y, m)` of the
/// month ending at each month-end close, held from the previous month-end close. Runs of whole
/// months at weight 1 are one trade; any other weight is one trade per month (monthly rebalanced);
/// 0 = cash.
fn monthly(x: &Ctx, from: usize, to: usize, w: impl Fn(i64, u32) -> f64, why: &str) -> Vec<Trade> {
    let me = bars(x, 2);
    let mut win: Vec<(usize, Option<usize>, f64, f64)> = vec![];
    let mut run: Option<usize> = None; // start of a weight-1 run
    for k in 1..me.len() {
        let (a, b) = (me[k - 1], me[k]);
        let (y, m) = ym(x, b);
        let wt = w(y, m);
        if wt == 1.0 {
            run.get_or_insert(a);
        } else if let Some(s) = run.take() {
            win.push((s, Some(a), 1.0, 1.0));
        }
        if wt != 1.0 && wt != 0.0 {
            win.push((a, Some(b), 1.0, wt));
        }
    }
    if let Some(s) = run {
        win.push((s, None, 1.0, 1.0));
    }
    win.sort_by_key(|t| t.0);
    windows(x, from, to, win, why)
}

// ------------------------------------------------------------------ catalogue

const NDX6: &[&str] = &[
    "E|\"if the COMPX rises 6 percent from a previous low, then a buy signal is generated on that Friday\" (p.184)",
    "X|\"a 6 percent drop from a previous high generates a sell signal on that Friday. On the sell signal the strategy went short instead of going into cash\" (p.184)",
    "F|Same rules as the VL4%: \"If the VLCI rises 4 percent from its last market low ... based solely on its weekly Friday closing price, that is a buy signal\"; \"If the VLCI declines from its last market top by 4 percent ... based on a weekly Friday closing price, that is a sell signal\" (p.154)",
    "F|Start: \"look for a buy signal when the weekly price goes up by 4 percent from any previous weekly close. Alternatively, a sell signal is generated when the index closes down 4 percent from a prior high\" (p.177): the first signal may be either.",
    "F|[Read] Low / high = lowest / highest weekly close since the last signal (start: the first weekly close). Weekly close = last bar of each Monday-Friday week; fill at that close (\"try to make the investment on Friday\", p.177).",
];

pub fn catalogue() -> Vec<StrategyDef> {
    use Status::*;
    let d = |id, name, group, dir, status, rules, notes, claim| StrategyDef {
        id, name, group, session: 0, dir, status, needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK, rules, notes, claim,
    };
    let dup = |id, name, group, why, rules, notes| d(id, name, group, "Long", NotTestable(why), rules, notes, None);
    vec![
        // ---- Ch 11 percentage-filter models
        d("masmt_six_pct_weekly", "Nasdaq 6% weekly strategy (NC6% / NDX6%)", "Ch 11 · Nasdaq Composite 6 Percent Strategy", "Long/Short", Live, NDX6,
            "Book indices: Nasdaq Composite (not in the vault) and Nasdaq 100 (NDX, in the US file from 1990). NDX6% printed for 11/1990-11/2002 -> compared on NDX. Optimisation variants (p.186): p.buy_pct=5 / p.sell_pct=6 or 8. NIFTY/SPX = instrument swap.",
            claim("55", "47%", "$599,337 net on $100,000 (22.34%/yr; PF 2.13; avg win/avg loss 2.38)", "", "NDX weekly 11/1990-11/2002 (NC6% 2/1971-11/2002: 94 trades, 54.3%, PF 2.37, 18.77%/yr)")),
        d("masmt_six_pct_daily", "Nasdaq 6% daily strategy (Tests 3 and 4)", "Ch 11 · Nasdaq Composite 6 Percent Strategy", "Long/Short", Live,
            &["E|\"I decided to run the same strategy on a daily basis\" (p.186): the 6% rules on daily closes, long on a buy, short on a sell",
              "X|Opposite signal reverses the position (p.184 rules)",
              "F|Book verdict: \"this daily strategy should be avoided at all costs\" (p.187). Optimum quoted: 11% (Nasdaq Composite), 12% (NDX) both sides."],
            "Printed: NDX daily 6% 11/1990-11/2002 lost $88,806 (-17.4%/yr); Nasdaq Composite daily 6% made $15,040. Run on NDX for the comparison.",
            claim("", "", "NDX: loss $88,806 on $100,000", "", "NDX daily 11/1990-11/2002")),
        d("masmt_vl4_20dma", "VL4% daily combined with the 20-dma", "Ch 10 · Combining the VL4% VLAI Daily Strategy with the 20-dma", "Long/Short", Live,
            &["E|\"1. A buy signal occurs when the VLAI rises 4 percentage points from a bottom. 2. A buy signal occurs when the VLAI pierces its 20-dma to the upside, from below\" (p.171)",
              "E|\"3. The earliest buy signal occurring on either strategy would be taken first ... if the other strategy issued a buy signal while the current buy signal was in place, no action would be necessary\" (p.171)",
              "X|\"4. A sell signal occurs when the VLAI declines 4 percentage points from a top. 5. A sell signal occurs when the VLAI drops below its 20-dma to the downside, from above. 6. The earliest sell signal ... would be taken first\" (p.171)",
              "F|[Read] Each sub-model keeps its own state (the 4% swing tracker and the 20-day SMA crossing); the position follows the most recent signal of either. Sells go short: the VL4% TradeStation tests went short (\"This is the same approach used in the previous chapter\", p.184). p.short=0 = cash instead.",
              "F|[Read] '4 percentage points' = 4 percent of the bottom/top close (the VL4% definition, p.154). If both fire opposite ways on one day the 20-dma signal (applied second) wins."],
            "Book index: Value Line Arithmetic Index (not in the vault); run on SPX/DJI/NDX/NIFTY as an instrument swap.",
            claim("207", "", "$521,632 gain on $100,000 (21.67%/yr)", "", "VLAI daily 12/1992-11/2002 (VL4% daily alone 61 trades; 20-dma alone 201 trades, $425,744)")),
        // ---- Ch 9 moving averages
        d("masmt_ma20_sar", "Nasdaq Composite 20-dma strategy", "Ch 9 · 20-dma-Nasdaq Composite Index Test", "Long/Short", Live,
            &["E|\"The second back test of the Nasdaq Composite Index used a 20-dma ... the same assumptions\" (p.144): buy when the daily close pierces the 20-day simple MA from below",
              "X|Sell and go short when the close drops below the 20-dma (25-wma box p.143: \"a sell signal is given and a short position is taken\")",
              "F|\"All investments were made the same day as the moving average crossover signal was given\" (p.143): fills at the signal close. SMA definition p.131.",
              "F|Optional (p.short=0): sell to cash. Optional (p.stop=10): \"limit your loss to a maximum 10% on each trade\" (p.135) - not part of the printed tests, off by default."],
            "Book index: Nasdaq Composite 2/1971-12/2002 (not in the vault; NDX is the closest). Optimisation p.145: 18-dma best ($56.02M, 22.47%/yr). The VLAI 20-dma (p.176, 201 trades) is the same rule.",
            claim("664", "38.25%", "$70,125 per trade ($46,563,046 net on $100,000; PF 1.64)", "98.03% in market", "Nasdaq Composite daily 2/25/1971-12/27/2002")),
        d("masmt_ma25w_sar", "Nasdaq Composite 25-wma strategy", "Ch 9 · 25-Wma-Nasdaq Composite Index Test", "Long/Short", Live,
            &["E|\"Buy signal. When the closing weekly price of the Nasdaq Composite Index pierces the 25-wma from below, a buy signal is given.\" (p.143)",
              "X|\"Sell signal. When the closing weekly price of the Nasdaq Composite Index drops below the 25-wma, a sell signal is given and a short position is taken.\" (p.143)",
              "F|25-week simple MA of weekly closes (last bar of each Monday-Friday week); fill at that close (p.143 \"same day\"). Optional p.short=0 / p.stop=10 as for the 20-dma."],
            "Book index: Nasdaq Composite 2/5/1971-12/27/2002 (NDX closest in the vault).",
            claim("97", "42.27%", "$44,071 per trade ($4,274,870 net on $100,000; PF 2.98; win/loss 4.06)", "93.76% in market", "Nasdaq Composite weekly 2/5/1971-12/27/2002")),
        d("masmt_ma50_200", "50-dma / 200-dma dual crossover", "Ch 9 · How to Use Single and Dual Moving Averages", "Long", Live,
            &["E|\"a buy signal is given when the 50-dma crosses over the 200-dma\" (p.135)",
              "X|\"and a sell signal is given when the 50-dma falls below the 200-dma\" (p.135)",
              "F|\"When you sell, you can either go into a money fund or, if you are an aggressive investor, go short\" (p.135): default = money fund (cash); p.short=1 = short. Fills at the signal close (book silent; the chapter's tests trade the same day, p.143)."],
            "No printed test (Fig 9-1 S&P 500 example: 50-dma crossed below the 200-dma in early November 2001, the last signal through 2002, p.134).", None),
        // ---- Ch 7 calendar
        d("masmt_sept_avoid", "Avoid September entirely (Vakkur)", "Ch 7 · Vakkur's contribution to seasonal investing", "Long", Live,
            &["X|\"Liquidate investments on the last trading day of August\" (p.110)",
              "E|\"and buy back on the first trading day in October. Then, don't sell again until the last trading day of August of the next year\" (p.110). Fills at those days' closes (time of day not given)."],
            "Vakkur: S&P 500 1950-1995 ex dividends, $10,000 -> $624,135 (9.6%/yr, Table 7-4); 1996-2001 $19,411 (11.69%/yr, Table 7-5). Siegel's 'every month except September' (Stocks for the Long Run Fig 18-4) is the same rule.",
            claim("", "", "$10,000 -> $624,135 (9.6%/yr)", "11 months", "S&P 500 1/1950-12/1995")),
        d("masmt_colby_sept_short", "Colby September short", "Ch 7 · Additional testing of September avoidance", "Long/Short", Live,
            &["E|\"instead of going to all cash in September, Colby sold short on the last trading day in August and covered the short on the last trading day in September\" (p.114)",
              "X|Short covered and long again at the close of the last trading day of September: the strategy is the September-avoidance strategy with a short instead of cash (p.114; Conclusions p.115 \"shorting during September and covering and going long in October\")",
              "F|[Read] Long re-entered at the close of the last trading day of September (the cover)."],
            "Colby: DJIA 1900-2000, $100 -> profit $164,048 (B&H ending $22,055). DJIA in the vault from 1970.",
            claim("", "", "$100 -> $164,048 profit", "", "DJIA 1900-2000")),
        d("masmt_colby_oct27_sep5", "Colby Oct 27 long / Sep 5 short", "Ch 7 · Additional testing of September avoidance", "Long/Short", Live,
            &["E|\"bought on October 27 (or the next closest trading day if on that day the market was closed), and sold on the next September 5 (or the next closest trading day ...)\" (p.114)",
              "X|\"The same short strategy was used on September 5 and covered on October 27\" (p.114): the long is sold and the short covered on those days. Fills at the closes."],
            "Colby: DJIA 1900-2000, $100 -> $644,467 (B&H $20,699).",
            claim("", "", "$100 -> $644,467", "", "DJIA 1900-2000")),
        d("masmt_ndr_oct1_may3", "Ned Davis October 1 - May 3", "Ch 7 · Calendar-based investing (background)", "Long", Live,
            &["E|\"entering into the market every October 1\" (p.100)", "X|\"and exiting the following May 3\" (p.100)",
              "F|[Read, B] Non-trading date -> next trading day (the convention the chapter states for Colby's dates, p.114); fills at the close."],
            "Ned Davis Research via Barron's: $10,000, 1950-2000, total gain $585,909 (S&P implied).",
            claim("", "", "$10,000 -> gain $585,909", "~7 months", "1950-2000")),
        d("masmt_harding_bsm", "Harding's adjusted Best Six Months (calendar)", "Ch 7 · Conclusions", "Long", Live,
            &["E|\"Invest in the stock market ... on the last day in October of each year\" (p.115)",
              "X|\"and then sell on the fourth business day in May of the following year\" (p.115)",
              "F|p.106 states Harding's research used \"the next-to-last trading day of October\" for the entry: p.entry_ntl=1 selects it (not the default)."],
            "Near-duplicate of apta_six_best (exit on/before May 4). No printed result for the calendar-only version.", None),
        d("masmt_harding_sts", "Harding Seasonal Timing Strategy (MACD)", "Ch 7 · Sy Harding's Street Smart Report", "Long",
            NotTestable("MACD settings and the signal definition for the STS are not given (\"his newly revised methodology for the MACD-based signals\", p.116); Conclusions p.115 only says 'look for a MACD crossover signal' in a two-month window"),
            &["E|\"buys on the first MACD buy signal that takes place (or is already in place) on or after October 16\" (p.108)",
              "X|\"the first MACD sell signal that takes place (or is already in place) on or after April 20\" (p.108)",
              "F|Conclusions #3: MACD (12,25,9); \"In the April-June time frame look for a MACD crossover signal to the downside (below its '0' line)\" (p.115) - which line crosses what is not stated."],
            "Printed: DJIA 1998-2002 +114.1% (Table 7-3); 1964-2002 15,172%. hsta_best6_macd is the Almanac's MACD version.", None),
        d("masmt_vakkur_may_oct", "Vakkur worst months (May-October)", "Ch 7 · Vakkur Table 7-4", "Long", Live,
            &["E|\"3. Invest during Hirsch's worst months using a May entry and an October exit date\" (p.110)",
              "X|[Read] Exit at the October month-end close (Vakkur's tests are monthly returns: invested from the April month-end close)."],
            "Contrast strategy. S&P 1950-1995: $10,000 -> $58,670 (4.0%/yr).", claim("", "", "$10,000 -> $58,670 (4.0%/yr)", "6 months", "S&P 500 1950-1995")),
        d("masmt_vakkur_switch", "Vakkur switching strategy (100/50/0)", "Ch 7 · Vakkur Table 7-4", "Long", Live,
            &["E|\"Be 100 percent invested in the best six months of November, December, January, March, April, and July\" (p.110)",
              "E|\"Be 50 percent invested and have 50 percent in cash (T-bills) in the next best three months of February, August, and October\" (p.110)",
              "X|\"And be 100 percent in cash for the worst three months of May, June, and September\" (p.110)",
              "F|[Read] Monthly allocation from month-end close to month-end close; 50% months are one trade each (rebalanced monthly). Cash earns 0."],
            "S&P 1950-1995 ex dividends: $10,000 -> $997,620 (10.8%/yr), max annual loss -24.7%.",
            claim("", "", "$10,000 -> $997,620 (10.8%/yr)", "", "S&P 500 1950-1995")),
        d("masmt_vakkur_lev_seasonal", "Vakkur 2:1 leverage in the best months", "Ch 7 · Vakkur Table 7-4", "Long", Live,
            &["E|\"5. Have a 2:1 leverage, where 50 percent is invested with margin (2:1 leverage) for the best six months. As defined in strategy number 4, 100 percent is invested in the next best three months, and 100 percent is in cash for the worst three months\" (p.110)",
              "X|Cash in May, June and September (strategy 4's worst three months). Leverage = leg weight 2 per month (monthly rebalanced); margin interest not modelled (book silent)."],
            "S&P 1950-1995: $10,000 -> $1,839,958 (12.3%/yr), max annual loss -49.6%.",
            claim("", "", "$10,000 -> $1,839,958 (12.3%/yr)", "", "S&P 500 1950-1995")),
        d("masmt_vakkur_lev_all", "Vakkur 2:1 leverage all months", "Ch 7 · Vakkur Table 7-4", "Long", Live,
            &["E|\"6. Invest in all months of the year with 2:1 leverage\" (p.110)", "X|No exit: each month is closed and re-opened at the month-end close (monthly rebalancing of the 2:1 position); margin interest not modelled."],
            "S&P 1950-1995: $10,000 -> $54,903 (3.9%/yr), max annual loss -70.6%.",
            claim("", "", "$10,000 -> $54,903 (3.9%/yr)", "", "S&P 500 1950-1995")),
        // ---- Ch 8 presidential cycle
        d("masmt_pres_best2", "Pre-election and election years only", "Ch 8 · Vakkur's Leveraged Strategies (Table 8-3)", "Long", Live,
            &["E|\"investing only during the best performing two presidential cycle years - the pre-election and election years\" (p.122)",
              "X|\"and remaining in cash for the other two years\" (p.122)",
              "F|Pre-election year = year before a US presidential election (1987, 1991, ...), election year = divisible by 4 (p.120). Invested from the December close of the midterm year to the December close of the election year."],
            "S&P 1950-1995: $10,000 -> $733,605 (10.0%/yr, max annual loss -18.7%). Freeburg DJIA 1886-2001: 5.9%/yr vs 5.2% (p.126). US cycle kept on NIFTY (instrument swap only).",
            claim("", "", "$10,000 -> $733,605 (10.0%/yr)", "2 years", "S&P 500 1950-1995")),
        d("masmt_pres_worst2", "Post-election and midterm years only", "Ch 8 · Table 8-3", "Long", Live,
            &["E|\"investing only in the worst two performing presidential cycle years - the postelection and midterm election years\" (p.122)", "X|Cash in the pre-election and election years"],
            "Contrast. S&P 1950-1995: $10,000 -> $56,297 (3.9%/yr).", claim("", "", "$10,000 -> $56,297 (3.9%/yr)", "2 years", "S&P 500 1950-1995")),
        d("masmt_pres_lev2", "Presidential 2:1 leverage", "Ch 8 · Table 8-3", "Long", Live,
            &["E|\"This strategy used 50 percent margin in the pre-election year, invested funds with no margin in the election year, and was 100 percent in cash for the other two years\" (p.122-124)",
              "X|Cash from the December close of the election year through the midterm year. Pre-election months at leg weight 2 (one trade per month), election year at weight 1. Margin interest not modelled."],
            "S&P 1950-1995: $10,000 -> $1,272,369 (11.4%/yr), max annual loss -46.0%.", claim("", "", "$10,000 -> $1,272,369 (11.4%/yr)", "", "S&P 500 1950-1995")),
        d("masmt_pres_lbm", "Leveraged Best Months (LBM)", "Ch 8 · Table 8-3", "Long", Live,
            &["E|\"the best performing months (November to April) of the year were leveraged 2 to 1 (50 percent margin) in the pre-election and election years, 100 percent was invested during these same months in the postelection and midterm election years\" (p.124)",
              "E|\"and 100 percent was invested during the May-October period during the same two years\" (p.124)",
              "X|\"The only time that there would be a 100 percent cash allocation would be in the May-October postelection and midterm election years.\" (p.124)",
              "F|[Read] The last sentence fixes 'the same two years' = pre-election and election years. Each month belongs to its calendar year's cycle type."],
            "S&P 1950-1995: $10,000 -> $2,183,257 (12.7%/yr), max annual loss -21.2%.", claim("", "", "$10,000 -> $2,183,257 (12.7%/yr)", "", "S&P 500 1950-1995")),
        d("masmt_pres_optimal", "Vakkur Optimal Months", "Ch 8 · Table 8-3", "Long", Live,
            &["E|\"2:1 leverage in pre-election and election years in the following months in those years: January to April, July, November and December\" (Table 8-3 note, p.123)",
              "X|\"100% cash in May and September of all 4 years, and June and August of the postelection and midterm election years\" (p.123)",
              "E|\"and 100% invested in all the other months not mentioned\" (p.123)"],
            "S&P 1950-1995: $10,000 -> $5,189,384 (14.9%/yr); Freeburg 1995-4/2002: 17.4%/yr vs 10.7% (p.126).", claim("", "", "$10,000 -> $5,189,384 (14.9%/yr)", "", "S&P 500 1950-1995")),
        d("masmt_midterm_nov_apr", "Midterm-year November-April", "Ch 8 · Midterm to election year phenomena (Table 8-6)", "Long", Live,
            &["E|\"the performance of the November through April time frame for each midterm election year from 1950 through 1998\" (p.127): long from the October month-end close of each midterm year (1950, 1954, ... ) ",
              "X|to the April month-end close of the following year (Table 8-6 'Nov.-Apr.' % changes)"],
            "Table 8-6 S&P 500: every one of 13 periods 1950-1998 up, mean 18.1%; 1986 18.2%, 1990 23.5%, 1994 9.0%, 1998 21.5%.",
            claim("13", "100%", "+18.1% mean", "6 months", "S&P 500 midterm years 1950-1998")),
        // ---- seen, not re-coded
        dup("masmt_vl4", "Value Line 4% strategy (VL4%)", "Ch 10", "duplicate: mbsot_zweig4 (p.pct=4, p.short=1) / mzwws_four_percent", &["E|\"If the VLCI rises 4 percent from its last market low ... that is a buy signal\" (p.154)"], "Value Line indices not in the vault."),
        dup("masmt_bsm", "Hirsch Best Six Months (Nov 1 - Apr 30)", "Ch 7", "duplicate: hsta_best6 / kftsm_hirsch", &["E|\"invest ... on November 1 of each year, and then sell on April 30 of the following year\" (p.104)"], "Vakkur's Nov-Apr row (Table 7-4: $703,935) is the same rule."),
        dup("masmt_colby_ma", "Colby DJIA moving-average tests (66 / 126 / 200 days, long and short)", "Ch 9 · Colby's DJIA tests", "duplicate: same price/SMA stop-and-reverse as masmt_ma20_sar with p.n=66/126 (and rwmts_sma200_sar for 200)",
            &["E|\"A buy signal was generated when the DJIA price pierced the moving average ... to the upside, and a sell signal ... below ... All sell signals were used to short the market\" (p.136)"],
            "126-dma: 886 trades, 21% profitable, avg win $6886.30 / avg loss -$1238.08, 42 days (DJIA 1900-2001)."),
        dup("masmt_merriman_100", "Merriman 100-dma (long / money market)", "Ch 9", "duplicate: apta_ma100_dji (same 100-day price crossing, long/cash)", &["E|\"signals were initiated when the index prices crossed their respective moving average to the upside (a buy signal) and the downside (a sell signal)\"; \"On all sell signals, the proceeds were invested into a money market account\" (p.138-139)"], "Nasdaq 100 1972-2001 18.9%/yr."),
        dup("masmt_murphy_50", "Murphy Nasdaq 50-dma", "Ch 9", "duplicate: same long/cash price-SMA crossing as apta_ma200_dji / kftsm_hh_ma with a 50-day length", &["E|\"selling whenever the Nasdaq fell under its 50-dma (and re-entering when it rose back above it)\" (p.142)"], "Nasdaq 1972-2002 +13,794%."),
        d("masmt_mcdonald", "McDonald 130-dma (S&P)", "Ch 9", "Long", NotTestable("what is held after a sell (cash or short) is not stated; only the next-day-close fill is"),
            &["F|\"McDonald used the next day's closing price, after the signal was given, as the buy or sell price\" (p.138); 72-, 130-, 132-dma best"], "S&P incl. dividends 1928-1999.", None),
        d("masmt_hays_low_high", "Midterm low to election-year high", "Ch 8 · Table 8-5", "Long", NotTestable("hindsight statistic: the year's low and the following election year's high are only known afterwards"),
            &["F|DJIA mid-year low to election-year high, 1918-1998 (Table 8-5, p.128)"], "", None),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    Some(match id {
        "masmt_six_pct_weekly" => ("masmt_pct", "Masonson percentage filter", "6% weekly"),
        "masmt_six_pct_daily" => ("masmt_pct", "Masonson percentage filter", "6% daily"),
        "masmt_vl4_20dma" => ("masmt_pct", "Masonson percentage filter", "4% + 20-dma"),
        "masmt_vl4" => ("masmt_pct", "Masonson percentage filter", "VL4% (duplicate)"),
        "masmt_ma20_sar" | "masmt_ma25w_sar" | "masmt_ma50_200" | "masmt_colby_ma" | "masmt_merriman_100" | "masmt_murphy_50" | "masmt_mcdonald" => ("masmt_ma", "Masonson moving averages", ""),
        "masmt_pres_best2" | "masmt_pres_worst2" | "masmt_pres_lev2" | "masmt_pres_lbm" | "masmt_pres_optimal" | "masmt_midterm_nov_apr" | "masmt_hays_low_high" => ("masmt_pres", "Presidential cycle (Vakkur / Hays)", ""),
        s if s.starts_with("masmt_") => ("masmt_cal", "Calendar / seasonal (Ch 7)", ""),
        _ => return None,
    })
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    let sh = |def| p("short", "Sell signals go short (1) or to cash (0)", def, 0.0, 1.0, 1.0);
    let st = || p("stop", "Loss stop % from entry (p.135; 0 = off, book tests have none)", 0.0, 0.0, 20.0, 1.0);
    Some(match id {
        "masmt_six_pct_weekly" | "masmt_six_pct_daily" => vec![
            p("buy_pct", "Buy: % rise from the low close", 6.0, 1.0, 15.0, 0.5), p("sell_pct", "Sell: % drop from the high close", 6.0, 1.0, 15.0, 0.5), sh(1.0)],
        "masmt_vl4_20dma" => vec![p("pct", "Swing filter %", 4.0, 1.0, 10.0, 0.5), p("n", "Moving average days", 20.0, 5.0, 100.0, 1.0), sh(1.0)],
        "masmt_ma20_sar" => vec![p("n", "Moving average days", 20.0, 5.0, 250.0, 1.0), sh(1.0), st()],
        "masmt_ma25w_sar" => vec![p("n", "Moving average weeks", 25.0, 5.0, 60.0, 1.0), sh(1.0), st()],
        "masmt_ma50_200" => vec![p("fast", "Fast MA days", 50.0, 5.0, 100.0, 1.0), p("slow", "Slow MA days", 200.0, 50.0, 300.0, 5.0), sh(0.0), st()],
        "masmt_harding_bsm" => vec![p("entry_ntl", "Enter on the next-to-last (1) instead of the last (0) October day", 0.0, 0.0, 1.0, 1.0),
            p("exit_td", "Exit on this trading day of May", 4.0, 1.0, 10.0, 1.0)],
        "masmt_colby_oct27_sep5" => vec![p("long_day", "Long from this October day", 27.0, 1.0, 31.0, 1.0), p("short_day", "Short from this September day", 5.0, 1.0, 30.0, 1.0)],
        "masmt_ndr_oct1_may3" => vec![p("entry_day", "Enter on this October day", 1.0, 1.0, 31.0, 1.0), p("exit_day", "Exit on this May day", 3.0, 1.0, 31.0, 1.0)],
        "masmt_vakkur_switch" => vec![p("half", "Weight in Feb, Aug, Oct", 0.5, 0.0, 1.0, 0.25)],
        "masmt_vakkur_lev_seasonal" | "masmt_vakkur_lev_all" | "masmt_pres_lev2" | "masmt_pres_lbm" | "masmt_pres_optimal" => vec![p("lev", "Leverage in the leveraged months", 2.0, 1.0, 3.0, 0.5)],
        "masmt_sept_avoid" | "masmt_colby_sept_short" | "masmt_vakkur_may_oct" | "masmt_pres_best2" | "masmt_pres_worst2" | "masmt_midterm_nov_apr" => vec![],
        _ => return None,
    })
}

// ------------------------------------------------------------------ run

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    let schema = params(id)?;
    let n = x.r.len();
    if n == 0 || from > to || to >= n {
        return Some(vec![]);
    }
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).map_or(0.0, |d| d.def));
    let u = |k: &str| g(k).round().max(1.0) as usize;
    let short = g("short") >= 0.5;
    let c = &x.c;
    let all: Vec<usize> = (0..n).collect();
    let (y0, _, _) = ymd(&x.r[0].date);
    let (y1, _, _) = ymd(&x.r[n - 1].date);
    let tr = match id {
        "masmt_six_pct_weekly" | "masmt_six_pct_daily" => {
            let idx = if id.ends_with("weekly") { bars(x, 1) } else { all };
            let ev = swing_events(c, &idx, g("buy_pct"), g("sell_pct"));
            run_events(x, from, to, &to_ev(n, &ev, short, "rise from the low close", "drop from the high close"), 0.0)
        }
        "masmt_vl4_20dma" => {
            let pct = g("pct");
            let ma = sma(c, u("n"));
            let mut ev = to_ev(n, &swing_events(c, &all, pct, pct), short, "4% swing buy", "4% swing sell");
            for (i, d) in cross_events(&all, c, &ma) {
                ev[i] = Some(if d > 0 { (1, "close above the 20-dma".into()) } else { (if short { -1 } else { 0 }, "close below the 20-dma".into()) });
            }
            run_events(x, from, to, &ev, 0.0)
        }
        "masmt_ma20_sar" => {
            let ma = sma(c, u("n"));
            run_events(x, from, to, &to_ev(n, &cross_events(&all, c, &ma), short, "close above the MA", "close below the MA"), g("stop"))
        }
        "masmt_ma25w_sar" => {
            let idx = bars(x, 1);
            let wc: Vec<f64> = idx.iter().map(|&i| c[i]).collect();
            let ma = sma(&wc, u("n"));
            run_events(x, from, to, &to_ev(n, &cross_events(&idx, &wc, &ma), short, "weekly close above the 25-wma", "weekly close below the 25-wma"), g("stop"))
        }
        "masmt_ma50_200" => {
            let (f, s) = (sma(c, u("fast")), sma(c, u("slow")));
            run_events(x, from, to, &to_ev(n, &cross_events(&all, &f, &s), short, "50-dma crossed over the 200-dma", "50-dma fell below the 200-dma"), g("stop"))
        }
        "masmt_sept_avoid" | "masmt_colby_sept_short" => {
            let mut w = vec![];
            for y in y0..=y1 {
                let aug = month_bars(x, y, 8).map(|r| r.end - 1);
                let sep = month_bars(x, y, 9).map(|r| r.end - 1);
                let oct1 = month_bars(x, y, 10).map(|r| r.start);
                let re = if id == "masmt_sept_avoid" { oct1 } else { sep };
                // long from the re-entry to next August's last day
                if let Some(e) = re {
                    w.push((e, month_bars(x, y + 1, 8).map(|r| r.end - 1), 1.0, 1.0));
                }
                if id == "masmt_colby_sept_short" {
                    if let Some(a) = aug {
                        w.push((a, sep, -1.0, 1.0));
                    }
                }
            }
            w.sort_by_key(|t| t.0);
            windows(x, from, to, w, if id == "masmt_sept_avoid" { "last trading day of August" } else { "calendar switch" })
        }
        "masmt_colby_oct27_sep5" => {
            let (ld, sd) = (u("long_day") as u32, u("short_day") as u32);
            let mut w = vec![];
            for y in y0..=y1 {
                if let Some(e) = on_or_after(x, y, 10, ld).filter(|&e| ym(x, e) == (y, 10)) {
                    w.push((e, on_or_after(x, y + 1, 9, sd).filter(|&k| ym(x, k) == (y + 1, 9)), 1.0, 1.0));
                }
                if let Some(e) = on_or_after(x, y, 9, sd).filter(|&e| ym(x, e) == (y, 9)) {
                    w.push((e, on_or_after(x, y, 10, ld).filter(|&k| ym(x, k) == (y, 10)), -1.0, 1.0));
                }
            }
            w.sort_by_key(|t| t.0);
            windows(x, from, to, w, "calendar switch")
        }
        "masmt_ndr_oct1_may3" => {
            let (ed, xd) = (u("entry_day") as u32, u("exit_day") as u32);
            let w = (y0..=y1)
                .filter_map(|y| on_or_after(x, y, 10, ed).filter(|&e| ym(x, e) == (y, 10)).map(|e| (e, on_or_after(x, y + 1, 5, xd).filter(|&k| ym(x, k) == (y + 1, 5)), 1.0, 1.0)))
                .collect();
            windows(x, from, to, w, "May 3")
        }
        "masmt_harding_bsm" => {
            let (ntl, td) = (g("entry_ntl") >= 0.5, u("exit_td"));
            let w = (y0..=y1)
                .filter_map(|y| {
                    let oct = month_bars(x, y, 10)?;
                    let e = if ntl { oct.end - 2 } else { oct.end - 1 };
                    let ex = month_bars(x, y + 1, 5).and_then(|r| (r.start + td - 1 < r.end).then_some(r.start + td - 1));
                    Some((e, ex, 1.0, 1.0))
                })
                .collect();
            windows(x, from, to, w, "fourth trading day of May")
        }
        "masmt_vakkur_may_oct" => monthly(x, from, to, |_, m| if (5..=10).contains(&m) { 1.0 } else { 0.0 }, "October month end"),
        "masmt_vakkur_switch" => {
            let h = g("half");
            monthly(x, from, to, |_, m| match m { 11 | 12 | 1 | 3 | 4 | 7 => 1.0, 2 | 8 | 10 => h, _ => 0.0 }, "month end")
        }
        "masmt_vakkur_lev_seasonal" => {
            let l = g("lev");
            monthly(x, from, to, |_, m| match m { 11 | 12 | 1 | 3 | 4 | 7 => l, 2 | 8 | 10 => 1.0, _ => 0.0 }, "month end")
        }
        "masmt_vakkur_lev_all" => {
            let l = g("lev");
            monthly(x, from, to, |_, _| l, "month end")
        }
        "masmt_pres_best2" => monthly(x, from, to, |y, _| if matches!(cyc(y), 3 | 0) { 1.0 } else { 0.0 }, "end of the election year"),
        "masmt_pres_worst2" => monthly(x, from, to, |y, _| if matches!(cyc(y), 1 | 2) { 1.0 } else { 0.0 }, "end of the midterm year"),
        "masmt_pres_lev2" => {
            let l = g("lev");
            monthly(x, from, to, |y, _| match cyc(y) { 3 => l, 0 => 1.0, _ => 0.0 }, "month end")
        }
        "masmt_pres_lbm" => {
            let l = g("lev");
            monthly(x, from, to, |y, m| {
                let best = !(5..=10).contains(&m);
                match (matches!(cyc(y), 3 | 0), best) { (true, true) => l, (true, false) => 1.0, (false, true) => 1.0, (false, false) => 0.0 }
            }, "month end")
        }
        "masmt_pres_optimal" => {
            let l = g("lev");
            monthly(x, from, to, |y, m| {
                let good = matches!(cyc(y), 3 | 0);
                if m == 5 || m == 9 || (!good && (m == 6 || m == 8)) {
                    0.0
                } else if good && matches!(m, 1..=4 | 7 | 11 | 12) {
                    l
                } else {
                    1.0
                }
            }, "month end")
        }
        "masmt_midterm_nov_apr" => {
            let w = (y0..=y1)
                .filter(|&y| cyc(y) == 2)
                .filter_map(|y| Some((month_bars(x, y, 10)?.end - 1, month_bars(x, y + 1, 4).map(|r| r.end - 1), 1.0, 1.0)))
                .collect();
            windows(x, from, to, w, "April month end")
        }
        _ => return None,
    };
    Some(tr)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn six_percent_swing_by_hand() {
        // start 100: 106 >= 100*1.06 -> buy; peak 120; 112.8 = 120*0.94 -> sell; low 110; 117 >= 110*1.06 -> buy
        let c = [100.0, 103.0, 106.0, 120.0, 113.0, 112.8, 110.0, 116.0, 117.0];
        let idx: Vec<usize> = (0..c.len()).collect();
        assert_eq!(swing_events(&c, &idx, 6.0, 6.0), vec![(2, 1), (5, -1), (8, 1)]);
        // a first move down fires a sell first
        assert_eq!(swing_events(&[100.0, 94.0], &[0, 1], 6.0, 6.0), vec![(1, -1)]);
    }

    #[test]
    fn crossings_need_a_prior_bar_on_the_other_side() {
        let idx = [0, 1, 2, 3];
        assert_eq!(cross_events(&idx, &[1.0, 2.0, 2.0, 0.5], &[1.5, 1.5, 1.5, 1.5]), vec![(1, 1), (3, -1)]);
    }
}
