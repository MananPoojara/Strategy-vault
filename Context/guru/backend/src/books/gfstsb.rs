//! Gary Ford, Systems Trading for Spread Betting (Harriman House, 2008).
//! Every complete system in the book runs on 60-minute or 6-hour forex bars in NinjaTrader. Lane K42 (4 Nov 2026, decision
//! #211): the three 60-minute systems run on 60-minute bars built from the vault's real 1-minute bars (intraday_ep, EP2: one
//! continuous series across days); the 6-hour systems stay held (pip stops, no 6-hour bars). The daily examples in ch.8 are
//! entry illustrations without exits or lengths. Sources: library/gfstsb/, re-read K42 (printed page = PDF page - 11).
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};

const BOOK: &str = "gfstsb";
macro_rules! ep { () => { "[intraday EP, K42] Session-aligned 60-minute bars built from real 1-minute bars (US 09:30-16:00 ET: six hours and a 30-minute last bar; NSE 09:15-15:30: six hours and a 15-minute last bar; NIFTY 50 days from 2026-08-11 skipped), one continuous series across days: indicators and positions carry overnight. The book ran 24-hour forex sessions (Session begins/ends 00:00, Figs 11.22, 14.23); SPY, SPX, NIFTY 50 and NIFTYBANK are a declared application, cash-index shorts are hypothetical. NinjaTrader settings printed in the backtest windows: CalculateOnBarClose true (orders fill at the next bar's open, p.123, p.174-175), Min. bars required 20 (no order before the 21st bar), Entries per direction 1, Exit on close False; NinjaTrader's EMA (seeded with the first close) and SMA (average of the bars so far until the period is full) [platform functions, not printed]. No costs. " } }
const H6: &str = "needs 6-hour bars (book: GBPUSD and 10 other forex pairs on six-hour bars); the vault has daily bars only";

pub fn catalogue() -> Vec<StrategyDef> {
    let make = |id, name, group, dir, status, rules, notes| StrategyDef {
        id, name, group, session: 0, dir, status, needs_volume: false, needs_vix: false,
        needs_series: &[], book: BOOK, rules, notes, claim: None,
    };
    vec![
        make("gfstsb_ema_cross", "EMA 9/45 crossover, stop and reverse", "Ch.11 Moving Average Example (p.97-120)", "Long/Short", Status::Live,
            &["E|\"Long entry occurs if the EMA(Close[0], Fast Bar Period) crosses above the EMA(Close[0], Slow Bar Period).\" (p.98)",
              "E|\"Short entry occurs if the EMA(Close[0], Fast Bar Period) crosses below the EMA(Close[0], Slow Bar Period).\" (p.98)",
              "X|\"We will not use a stop or profit target ... always has an open position\"; an opposite signal closes and reverses (p.97-98).",
              "F|Defaults FastEMA 9, SlowEMA 45 (Figure 11.4 parameter page, p.100; \"I have left the values set to the default values\" p.118). Calculate on bar close = fill at the next bar's open (Fig. 11.3 p.100; p.174-175 explains next-bar fills).",
              "F|\"I have configured the strategy to run on a 60 minute bar.\" (p.118); EURJPY, ~8 years of 1-minute data (p.115)."],
            concat!("Signals: NinjaTrader CrossAbove/CrossBelow (look-back 1) of EMA(Close,9) and EMA(Close,45) at the bar close, filled at the next bar's open; always in. ", ep!(), "Results only as a screenshot (Figure 11.23) for EURJPY 60-minute bars 1 Jan 2000 - 9 Apr 2008 (Fig 11.22); optimisation (ch.13, Table 13.4) picked 9/20 on hourly EURJPY (optimised, not the default). No index result to compare.")),
        make("gfstsb_channel", "SMA(High/Low,200) channel cross with EMA 5/10 exit", "Ch.14 Putting It All Together (p.149-190)", "Long/Short", Status::Live,
            &["E|\"Open Long: If Close[0] crosses above SMA(High, 200)\"; \"Open Short: If Close[0] crosses below SMA(Low, 200)\" (Table 14.4)",
              "X|\"Close Long: If EMA(FastEMA[0]) crosses below EMA(SlowEMA[0])\"; \"Close Long: If Close[0] < SMA(High, 200)\" (Table 14.4)",
              "X|\"Close Short: If EMA(FastEMA[0]) crosses above EMA(SlowEMA[0])\"; \"Close Short: If Close[0] > SMA(Low, 200)\" (Table 14.4)",
              "F|Defaults FastEMA 5, SlowEMA 10 (Table 14.3, p.155); channel SMA 200 (Table 14.4, p.174). Optimised (190, 2, 20) is a later fitted set (Table 14.13), not the default.",
              "F|60-minute bars, CalculateOnBarClose = true: fills at the open of the next bar (p.174-175; 60-minute bars p.181)."],
            concat!("Order of the six conditions as NinjaTrader evaluates the sets on one bar close: long -> an open-short signal reverses, else an EMA cross down or a close below SMA(High) exits; short -> an open-long signal reverses, else an EMA cross up or a close above SMA(Low) exits; flat -> open long / open short. Fills at the next bar's open. Defaults 200/5/10 are Table 14.3's stated creation defaults (the Fig. 14.23 backtest window shows 200/4/9; option fast/slow). ", ep!(), "GBPUSD 60-minute bars 1 Jan 2000 - 1 Jan 2007: net 18,320, PF 1.09, 30.43% profitable (Table 14.5); forex only, no index result to compare.")),
        make("gfstsb_inside_bar", "Inside bar breakout (trailing stop)", "Appendix A (p.221-227)", "Long/Short", Status::NeedsData(H6),
            &["E|Two inside six-hour bars: High[2] < High[3], Low[2] > Low[3], High[1] < High[2], Low[1] > Low[2]; long if Close[0] > High[3], short if Close[0] < Low[3] (Table 19.1)",
              "X|\"Exit the position on a trailing stop.\" 100 (pips) in Table 19.2 (p.227)"],
            "GBPUSD 1/1/2004-1/1/2007: net -3,560, PF 0.76, 25.71% profitable (Table 19.2). Stop is in forex points; no index equivalent is given."),
        make("gfstsb_inside_bar_sma", "Inside bar with SMA reversion (parts 1 and 2)", "Appendix A (p.228-240)", "Long/Short", Status::NeedsData(H6),
            &["E|Two inside bars and Close[1] < SMA[0] -> long; Close[1] > SMA[0] -> short (Table 19.3); part 2 adds Close[0] > Close[1] / < Close[1] (Table 19.5)",
              "X|Close long when High[0] >= SMA[0]; close short when Low[0] <= SMA[0] (Tables 19.3, 19.5)",
              "X|Trailing stop 200 (part 1, Table 19.4) / static stop-loss 100 (part 2, Table 19.6); SMA 30"],
            "Six-hour forex bars; stop sizes in forex points."),
        make("gfstsb_perfect_order", "Moving-average perfect order", "Appendix A (p.241-256)", "Long/Short", Status::Live,
            &["E|Long: SMA1 > SMA2 > SMA3 > SMA4 > SMA5; short: the reverse (Table 19.11); SMA 10, 20, 50, 100, 200 (Table 19.12)",
              "X|Close long when SMA1 crosses below SMA2; close short when SMA1 crosses above SMA2 (Table 19.11)"],
            concat!("Open when the perfect order holds at a bar close (not only when it forms): flat -> long or short; long -> a full bear order reverses, else SMA1 crossing below SMA2 exits; short -> mirror. Fills at the next bar's open. ", "60-minute bars: the optimisation tables' bar period (Tables 19.13, 19.16). ", ep!(), "GBPUSD 2006: net 3,020, PF 1.11, 35.64% profitable (Table 19.12); forex only, no index result to compare.")),
        make("gfstsb_daily_ema_example", "Daily EMA crossover illustration", "Ch.8 Trade Entry (p.70-71)", "Long/Short",
            Status::NotTestable("EMA lengths are not stated (Figure 8.1 legend is unreadable at the printed resolution); borrowing 9/45 from the hourly ch.11 system would be another system's parameters"),
            &["E|\"When the fast moving average ... crosses up through the slow moving average ... a buy signal is generated. Conversely ... a sell signal\" (p.71)", "X|\"the position is reversed when the signals are generated\" (p.71)"], ""),
        make("gfstsb_macd_example", "MACD entry illustration", "Ch.8 Trade Entry (p.72-73)", "Long/Short",
            Status::NotTestable("entry example only: no exit is given, and 'above its 9 day EMA or the centreline' leaves the trigger open"),
            &["E|\"A bullish signal occurs when MACD ... moves above its 9 day EMA or the centreline\" (12, 26, 9) (p.72)"], "No exit; never add one."),
        make("gfstsb_inside_day_example", "Inside day entry illustration", "Ch.8 Trade Entry (p.73)", "Long",
            Status::NotTestable("'just above the close' is not quantified and there is no profit exit"),
            &["E|\"enter a long order just above the close price of the inside day, with a stop-loss just below the open price of the inside day\" (p.73)"], ""),
        make("gfstsb_exit_menu", "Exit and filter toolbox (ch.9-10)", "Ch.9-10 Exits / Filters (p.75-96)", "Long/Short",
            Status::NotTestable("menu of generic exits and filters (end of day, ATR, Donchian, pivots, Fibonacci, trailing stops, ADX/DMI) with no complete system or numbers"),
            &["X|End of day, time of day, static stops, X-bar high/low, ATR, moving average, Donchian, pivot, Fibonacci, trailing stop, profit target, direction reversal (ch.9)"], ""),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    Some(match id {
        "gfstsb_ema_cross" | "gfstsb_daily_ema_example" => ("gfstsb_ma", "Ford moving-average crossover", ""),
        "gfstsb_channel" => ("gfstsb_channel", "Ford channel crossover", ""),
        "gfstsb_inside_bar" | "gfstsb_inside_bar_sma" | "gfstsb_inside_day_example" => ("gfstsb_inside", "Ford inside bar", ""),
        "gfstsb_perfect_order" => ("gfstsb_po", "Ford perfect order", ""),
        "gfstsb_macd_example" | "gfstsb_exit_menu" => ("gfstsb_misc", "Ford examples and toolbox", ""),
        _ => return None,
    })
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    Some(match id {
        "gfstsb_ema_cross" => vec![p("fast", "FastEMA (Fig 11.4 default)", 9., 2., 50., 1.), p("slow", "SlowEMA (Fig 11.4 default)", 45., 5., 200., 1.)],
        "gfstsb_channel" => vec![p("chan", "ChannelPeriod (SMA of highs / lows)", 200., 20., 400., 10.),
            p("fast", "FastEMA (Table 14.3 default 5; the Fig 14.23 run shows 4)", 5., 2., 30., 1.), p("slow", "SlowEMA (Table 14.3 default 10; the Fig 14.23 run shows 9)", 10., 3., 60., 1.)],
        "gfstsb_perfect_order" => vec![p("s1", "SMA1", 10., 2., 50., 1.), p("s2", "SMA2", 20., 3., 100., 1.), p("s3", "SMA3", 50., 5., 200., 1.),
            p("s4", "SMA4", 100., 10., 300., 5.), p("s5", "SMA5", 200., 20., 500., 10.)],
        _ => return None,
    })
}

// ---- intraday EP block (lane K42, 4 Nov 2026, decision #211): the three 60-minute NinjaTrader systems on 60-minute bars
// built from the 1-minute file (intraday_ep::day_bars, session-aligned), one continuous series across days (EP2).
pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    let schema = params(id)?;
    if !crate::intraday_ep::id(id) || from > to || to >= x.r.len() { return None; }
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).map_or(1.0, |d| d.def)).max(1.0).round() as usize;
    // (daily row, open, high, low, close) of every 60-minute bar, in time order
    let mut b: Vec<(usize, f64, f64, f64, f64)> = vec![];
    for i in from..=to {
        if let Some((_, _, _, v)) = crate::intraday_ep::day_bars(x, i, 60) { b.extend(v.iter().map(|z| (i, z.o, z.h, z.l, z.c))); }
    }
    let Some(&(li, _, _, _, lc)) = b.last() else { return Some(vec![]) };
    let c: Vec<f64> = b.iter().map(|z| z.4).collect();
    // signal per bar close: +1 open long, -1 open short, 2 close (flat), 0 nothing; decided with the position held at that close
    let mut bk = crate::intraday_ep::Book::default();
    let up = |a: &[f64], v: &[f64], k: usize| a[k - 1] <= v[k - 1] && a[k] > v[k]; // NinjaTrader CrossAbove, look-back 1
    let sig: Box<dyn Fn(usize, f64) -> (f64, &'static str)> = match id {
        "gfstsb_ema_cross" => {
            let (f, s) = (nt_ema(&c, g("fast")), nt_ema(&c, g("slow")));
            Box::new(move |k, _| if up(&f, &s, k) { (1.0, "reverse") } else if up(&s, &f, k) { (-1.0, "reverse") } else { (0.0, "") })
        }
        "gfstsb_channel" => {
            let n = g("chan");
            let (hs, ls) = (nt_sma(&b.iter().map(|z| z.2).collect::<Vec<_>>(), n), nt_sma(&b.iter().map(|z| z.3).collect::<Vec<_>>(), n));
            let (f, s) = (nt_ema(&c, g("fast")), nt_ema(&c, g("slow")));
            let c = c.clone();
            Box::new(move |k, d| {
                let (ol, os) = (up(&c, &hs, k), up(&ls, &c, k));
                if d > 0.0 { if os { (-1.0, "reverse") } else if up(&s, &f, k) { (2.0, "ema cross") } else if c[k] < hs[k] { (2.0, "channel") } else { (0.0, "") } }
                else if d < 0.0 { if ol { (1.0, "reverse") } else if up(&f, &s, k) { (2.0, "ema cross") } else if c[k] > ls[k] { (2.0, "channel") } else { (0.0, "") } }
                else if ol { (1.0, "") } else if os { (-1.0, "") } else { (0.0, "") }
            })
        }
        _ => {
            let m: Vec<Vec<f64>> = ["s1", "s2", "s3", "s4", "s5"].iter().map(|k| nt_sma(&c, g(k))).collect();
            Box::new(move |k, d| {
                let bull = (0..4).all(|j| m[j][k] > m[j + 1][k]);
                let bear = (0..4).all(|j| m[j][k] < m[j + 1][k]);
                if d > 0.0 { if bear { (-1.0, "reverse") } else if up(&m[1], &m[0], k) { (2.0, "sma cross") } else { (0.0, "") } }
                else if d < 0.0 { if bull { (1.0, "reverse") } else if up(&m[0], &m[1], k) { (2.0, "sma cross") } else { (0.0, "") } }
                else if bull { (1.0, "") } else if bear { (-1.0, "") } else { (0.0, "") }
            })
        }
    };
    let mut pend = (0.0, "");
    for k in 0..b.len() {
        let (i, o) = (b[k].0, b[k].1);
        match pend.0 { 2.0 => bk.exit(i, o, pend.1), 0.0 => {}, d => bk.enter(d, i, o, "open", pend.1) }
        pend = if k >= MIN_BARS { sig(k, bk.dir()) } else { (0.0, "") };
    }
    Some(bk.finish(li, lc))
}

/// NinjaTrader "Min. bars required" (20 in every printed backtest window, Figs 11.22 and 14.23): OnBarUpdate is not called before.
const MIN_BARS: usize = 20;
/// NinjaTrader EMA: the first value is the first input, then Value = Input x 2/(1+P) + (1 - 2/(1+P)) x Value[1].
fn nt_ema(v: &[f64], n: usize) -> Vec<f64> {
    let k = 2.0 / (1.0 + n as f64);
    let mut out: Vec<f64> = Vec::with_capacity(v.len());
    for (j, &x) in v.iter().enumerate() { out.push(if j == 0 { x } else { x * k + (1.0 - k) * out[j - 1] }); }
    out
}
/// NinjaTrader SMA: the average of the last min(P, bars so far) inputs.
fn nt_sma(v: &[f64], n: usize) -> Vec<f64> {
    (0..v.len()).map(|j| { let w = &v[(j + 1).saturating_sub(n)..=j]; w.iter().sum::<f64>() / w.len() as f64 }).collect()
}
// ---- end intraday EP block

#[cfg(test)]
mod tests {
    #[test]
    fn only_the_60_minute_systems_are_live() {
        for d in super::catalogue() { assert_eq!(d.status == crate::connors::Status::Live, crate::intraday_ep::id(d.id), "{}", d.id); }
    }
    #[test]
    fn ninjatrader_ema_sma_and_cross_logic_on_hand_built_bars() {
        assert_eq!(super::nt_ema(&[10., 13., 7.], 2), vec![10., 12., 12. * (1. - 2. / 3.) + 7. * (2. / 3.)]);
        assert_eq!(super::nt_sma(&[1., 3., 5., 7.], 3), vec![1., 2., 3., 5.]);
    }
}
