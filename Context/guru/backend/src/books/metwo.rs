//! Mark Etzkorn, "Trading with Oscillators: Pinpointing Market Extremes" (Wiley, 1997). Book id `metwo`.
//! Reading: claude-continuation-20261020/authors/etzkorn/agents/. Ch.5 "Simple Tests" (p.62-69) tests three trend-following
//! oscillator signals as stop-and-reverse systems on the close (no stops, nine futures markets 4/2/1984-3/31/1994) and prints
//! the optimised lengths per market (Tables 5.2-5.3). Default = the book's S&P 500 row; the nine-market average row and the
//! standard 12-26-9 MACD are options. The stochastic overbought/oversold tests print only averages over a parameter grid (held).
//! Also book id `mewwot`: "What Works in Online Trading" (McGraw-Hill 2001, edited by Etzkorn), his own article "Clear-cut
//! chart pattern trading" (p.125-132): the spike-reversal day entered on the close and exited after eight days.
use crate::connors::{rsi, Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};
use crate::books::apta::ema;

pub const BOOK: &str = "metwo";
pub const REVIEWED: &[&str] = &["metwo_macd_sar", "metwo_rsi50_sar", "metwo_roc_sar", "mewwot_spike_rev"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const TEST: &str = "F|Test method (p.62-63): \"Trades were executed on the close\"; \"No stops were used\"; stop and reverse; nine futures markets, 10 years of daily continuous data from April 2, 1984, to March 31, 1994";
const NOTE: &str = "Etzkorn calls these 'simple tests ... intended simply to give a rough idea of the behavior of some isolated oscillator signals' (p.68); the printed lengths are in-sample optimisations per futures market. Default = the S&P 500 row (the market closest to the vault's instruments); options give the nine-market average and, for the MACD, the standard 12-26-9. [Declared readings, not from the book] a signal needs a strict cross (equal = no change); the position is taken or reversed at that close; flat until the first signal of the window; indices, ETFs and stocks, US and India, are a declared application (book: futures).";

pub fn catalogue() -> Vec<StrategyDef> {
    use Status::*;
    let d = |id, name, group, rules, notes| StrategyDef { id, name, group, session: 0, dir: "Long/Short", status: Live, needs_volume: false, needs_vix: false,
        needs_series: &[], book: BOOK, rules, notes, claim: None };
    let nt = |id, name, group, why, rules| StrategyDef { id, name, group, session: 0, dir: "Long/Short", status: NotTestable(why), needs_volume: false, needs_vix: false,
        needs_series: &[], book: BOOK, rules, notes: "Held after the 2026-10-24 reading (authors/etzkorn/agents).", claim: None };
    vec![
        d("metwo_macd_sar", "MACD signal-line crossover, stop and reverse (S&P 500 12-24-13)", "Ch.5 · Simple tests (p.66-67, Table 5.2)",
          &["F|\"MACD = EMA1 - EMA2\", \"Signal line = EMA3 of MACD\" (p.41); S&P 500 optimised lengths 12, 24, 13 (Table 5.2, p.67); averages row 17-30-12; standard 12-26-9",
            "E|\"Trades for the MACD were triggered by signal line crossovers: Buy when the MACD crosses above its signal line and sell when it crosses below it.\" (p.66-67)",
            "X|Stop and reverse on the opposite crossover, on the close", TEST],
          NOTE),
        d("metwo_rsi50_sar", "RSI equilibrium (50) crossing, stop and reverse (S&P 500 RSI 64)", "Ch.5 · Simple tests (p.68, Table 5.3)",
          &["F|Wilder RSI (p.33-34): RS = average up close / average down close over the period, then Wilder's smoothing; S&P 500 length 64 (Table 5.3); average 41",
            "E|\"equilibrium line crossings\" (p.68): buy when the RSI crosses above 50, sell when it crosses below 50", "X|Stop and reverse on the opposite crossing, on the close", TEST],
          NOTE),
        d("metwo_roc_sar", "Rate-of-change equilibrium crossing, stop and reverse (S&P 500 ROC 32)", "Ch.5 · Simple tests (p.68, Table 5.3)",
          &["F|\"ROC = Price (today) / price (n days ago)\", equilibrium line 1 (p.27); S&P 500 length 32 (Table 5.3); average 36",
            "E|Buy when the ROC crosses above its equilibrium line, sell when it crosses below (p.68)", "X|Stop and reverse on the opposite crossing, on the close", TEST],
          NOTE),
        StrategyDef { id: "mewwot_spike_rev", name: "Spike-reversal day, entry on the close, out after eight days", group: "What Works in Online Trading · Clear-cut chart pattern trading (p.125-132)",
            session: 0, dir: "Long/Short", status: Live, needs_volume: false, needs_vix: false, needs_series: &[], book: "mewwot",
            rules: &["F|\"Spike-reversal high = Today's high must be n percent higher than yesterday's high and today's close must be below yesterday's close. Spike-reversal low = Today's low must be n percent lower than yesterday's low and today's close must be above yesterday's close.\" (p.130); Fig 5.15 uses \"a 3 percent increase or decrease (from the previous high or low) ... and an accompanying close above or below the close of the previous bar\" (p.130) - default n = 3",
                     "E|\"enter short on the close of spike-reversal high days and enter long on the close of spike-reversal low days\" (p.131)",
                     "X|\"no stops were used and the exit was arbitrary (all trades were exited after eight days)\" (p.132): exit at the close of the eighth trading day after the entry day",
                     "F|The DJIA-stock test used spikes of 0.1-0.4 percent and close-to-close reversals of 0.1-0.4 percent (p.132): options"],
            notes: "Etzkorn's illustration in an anthology he edited; the printed test is an average over a small parameter grid on the 30 Dow stocks (1990-2000). [Declared readings, not from the book] one position at a time (signals during a trade are ignored); fills at the close; indices, ETFs and stocks, US and India, are a declared application (book: Dow stocks).",
            claim: None },
        nt("metwo_stoch_obos", "Stochastic overbought/oversold, initial penetration or retracement", "Ch.5 · Simple tests (p.63-65, Table 5.1)",
           "only averages over a grid of lengths (4-64) and zones (5/95-45/55) are printed; no single setting is given",
           &["E|\"Buy when the indicator became oversold, sell when the indicator became overbought\" (p.62)"]),
        nt("metwo_divergence", "Divergence, failure swings and combined-oscillator methods", "Ch.6-9", "signals read off the chart; no exits or settings",
           &["E|Divergences, Kase PeakOscillator, TD REI, TSI and other oscillators (Ch.6-9)"]),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    Some(match id {
        "metwo_macd_sar" => ("metwo_sar", "Etzkorn oscillator stop-and-reverse tests", "MACD"),
        "metwo_rsi50_sar" => ("metwo_sar", "Etzkorn oscillator stop-and-reverse tests", "RSI 50"),
        "metwo_roc_sar" => ("metwo_sar", "Etzkorn oscillator stop-and-reverse tests", "ROC"),
        "mewwot_spike_rev" => ("mewwot_spike_rev", "Etzkorn spike-reversal day", "3% / 8 days"),
        _ if id.starts_with("metwo_") || id.starts_with("mewwot_") => ("metwo_nt", "Etzkorn (held)", "-"),
        _ => return None,
    })
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    Some(match id {
        "metwo_macd_sar" => vec![p("fast", "Short EMA (S&P 12; average 17; standard 12)", 12., 2., 60., 1.),
            p("slow", "Long EMA (S&P 24; average 30; standard 26)", 24., 3., 120., 1.), p("signal", "Signal EMA (S&P 13; average 12; standard 9)", 13., 2., 60., 1.)],
        "metwo_rsi50_sar" => vec![p("n", "RSI days (S&P 64; average 41)", 64., 4., 100., 1.)],
        "metwo_roc_sar" => vec![p("n", "ROC days (S&P 32; average 36)", 32., 4., 100., 1.)],
        "mewwot_spike_rev" => vec![p("spike", "Spike: high above (low below) yesterday's by % (Fig 5.15: 3; test 0.1-0.4)", 3., 0.1, 10., 0.1),
            p("rev", "Close beyond yesterday's close by % (Fig 5.15: 0; test 0.1-0.4)", 0., 0., 2., 0.1), p("hold", "Trading days held after the entry day (book 8)", 8., 1., 20., 1.)],
        _ if id.starts_with("metwo_") || id.starts_with("mewwot_") => vec![],
        _ => return None,
    })
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !REVIEWED.contains(&id) || from > to || to >= x.r.len() { return None; }
    if id == "mewwot_spike_rev" { return Some(spike_rev(x, from, to, p)); }
    let g = |k: &str, d: f64| p.get(k).copied().unwrap_or(d).round().max(1.) as usize;
    let n = x.c.len();
    // the oscillator minus its equilibrium level: > 0 bullish side, < 0 bearish side
    let osc: Vec<f64> = match id {
        "metwo_macd_sar" => {
            let (f, s) = (ema(&x.c, g("fast", 12.)), ema(&x.c, g("slow", 24.)));
            let macd: Vec<f64> = (0..n).map(|i| f[i] - s[i]).collect();
            let sig = ema(&macd, g("signal", 13.));
            (0..n).map(|i| macd[i] - sig[i]).collect()
        }
        "metwo_rsi50_sar" => rsi(&x.c, g("n", 64.)).iter().map(|v| v - 50.).collect(),
        _ => {
            let k = g("n", 32.);
            (0..n).map(|i| if i >= k && x.c[i - k] > 0. { x.c[i] / x.c[i - k] - 1. } else { f64::NAN }).collect()
        }
    };
    let (mut out, mut cur) = (vec![], None::<Trade>);
    for i in from.max(1)..=to {
        let (a, b) = (osc[i - 1], osc[i]);
        if !(a.is_finite() && b.is_finite()) { continue; }
        let d = if a <= 0. && b > 0. { 1. } else if a >= 0. && b < 0. { -1. } else { continue };
        if cur.as_ref().is_some_and(|t| t.dir == d) { continue; }
        if let Some(mut t) = cur.take() {
            t.exit_i = i; t.exit_px = x.c[i]; t.signal_i = i; t.reason = "opposite crossing: reverse".into(); out.push(t);
        }
        cur = Some(Trade { dir: d, legs: vec![Leg { i, px: x.c[i], w: 1. }], exit_i: i, exit_px: x.c[i], reason: String::new(), entry_kind: "close", signal_i: i, open: false, exits: vec![] });
    }
    if let Some(mut t) = cur { t.exit_i = to; t.exit_px = x.c[to]; t.signal_i = to; t.reason = "open".into(); t.open = true; out.push(t); }
    Some(out)
}

/// Spike-reversal day (What Works in Online Trading p.130-132): entry at the close, exit at the close `hold` bars later.
fn spike_rev(x: &Ctx, from: usize, to: usize, p: &Params) -> Vec<Trade> {
    let sp = p.get("spike").copied().unwrap_or(3.) / 100.;
    let rv = p.get("rev").copied().unwrap_or(0.) / 100.;
    let hold = p.get("hold").copied().unwrap_or(8.).round().max(1.) as usize;
    let (mut out, mut i) = (vec![], from.max(1));
    while i <= to {
        let hi = x.h[i] > x.h[i - 1] * (1. + sp) && x.c[i] < x.c[i - 1] * (1. - rv);
        let lo = x.l[i] < x.l[i - 1] * (1. - sp) && x.c[i] > x.c[i - 1] * (1. + rv);
        if !(hi || lo) { i += 1; continue; }
        let d = if lo { 1. } else { -1. };
        let e = (i + hold).min(to);
        out.push(Trade { dir: d, legs: vec![Leg { i, px: x.c[i], w: 1. }], exit_i: e, exit_px: x.c[e], reason: if i + hold <= to { format!("{hold} days held") } else { "open".into() },
            entry_kind: "close", signal_i: e, open: i + hold > to, exits: vec![] });
        i = e + 1;
    }
    out
}
