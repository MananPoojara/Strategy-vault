//! Mark Johnson, "Monte Carlo Resampling of Equity Curves using N-bar Segments" (15 October 2005, Austin Trading
//! Workshop handout, 36 pp.). Section I (PDF p.3-4, printed p.2-3) states the two mechanical systems used in the study.
//! Source: research drive USB HDD 2/tRADING KNO-2/Mark Johnson - Monte Carlo Resampling of Equity Curves Using N-Bar
//! Segments/MC_resampling_Nbars[1].pdf; copy in Books/Mark G Johnson/. Book id "mjmcr". Notes: library/mjmcr/README.md.
use crate::books::mjthir::ema;
use crate::books::rgcts::{stop_engine, Orders};
use crate::connors::{Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "mjmcr";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![
        StrategyDef {
            id: "mjmcr_ema_xo", name: "EMA XO(300,50): always in, reverse at the next open", group: "I. Mechanical trading systems (Figure 1)",
            session: 0, dir: "Long/Short", status: Status::Live, needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK,
            rules: &[
                "F|\"it always has a position in the market, either long or short. It is a 'pure reversal' system. Parameter values are 300 trading days (bars) for the slow EMA and 50 bars for the fast EMA\" (p.2)",
                "X|\"If(Long) and (EMA50 < EMA300) then Exit all long contracts at market tomorrow, and Enter short tomorrow at market\" (Figure 1, rule 2)",
                "E|\"If(Short) and (EMA50 > EMA300) then Exit all short contracts at market tomorrow, and Enter long tomorrow at market\" (Figure 1, rule 3)",
                "F|Size \"N = (0.003 x Total Account Equity) / (BigPointValue x (EMA10 of True Range))\" (rule 1): not modelled (one full position per trade, percentage returns)",
            ],
            notes: "EMA = smoothing 2/(n+1), seeded with the first close of the data [Book silent on seeding; neutral TradeStation/Blox-style choice]. The rules only reverse an existing position; the first position is taken at the first open after the 300-bar priming period, in the direction of EMA50 vs EMA300 on the prior close, because the text says the system 'always has a position' [Book silent on the first entry]. Equal EMAs give no order. Book: Austin_57 futures portfolio, results shown only as Monte Carlo distributions (no per-market trades), so nothing is comparable on one index. India: instrument swap; index shorts are hypothetical.",
            claim: None,
        },
        StrategyDef {
            id: "mjmcr_dcbo", name: "DCBO(300,20): 300-bar breakout entry, 20-bar opposite breakout exit", group: "I. Mechanical trading systems (Figure 2)",
            session: 0, dir: "Long/Short", status: Status::Live, needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK,
            rules: &[
                "F|\"HH300 means the Highest High price over the most recent 300 bars, LL20 means the Lowest Low over the most recent 20 bars\" (p.2)",
                "E|\"If(not Long) enter an order to 'Buy NL contracts tomorrow at price HH300 on a Stop'\" (Figure 2, rule 2)",
                "E|\"If(not Short) enter an order to 'Sell NS contracts tomorrow at price LL300 on a Stop'\" (Figure 2, rule 3)",
                "X|\"If(Short) enter an order to 'Buy all contracts tomorrow at price HH20 on a Stop'\" (Figure 2, rule 4, p.3)",
                "X|\"If(Long) enter an order to 'Sell all contracts tomorrow at price SS20 on a Stop'\" (Figure 2, rule 5, p.3); SS20 read as LL20: the only 20-bar low the text defines, and rule 1 sizes longs on the distance Close - LL20",
                "F|Size NL = 0.0185 x Equity / ((Close - LL20) x BigPointValue), NS with (HH20 - Close) (rule 1): not modelled",
            ],
            notes: "Orders for bar i use HH/LL of the 300 or 20 bars ending at i-1 (the order is placed after that close for 'tomorrow'); a stop fills at the stop price, or at the open if the bar gaps through it. While long, the LL20 exit stop and the LL300 short-entry stop (rule 3: 'not Short') are both live; if one bar reaches both, the long exits at LL20 and goes short at LL300. Shorts mirror this with HH20/HH300. No exit stop works on the entry bar (the exit orders are only placed after the entry day's close). If a flat bar reaches both HH300 and LL300, no trade is taken [Book silent]. First orders after 300 bars of data. Book: Austin_57 futures portfolio, Monte Carlo distributions only; nothing comparable on one index. India: instrument swap; index shorts are hypothetical.",
            claim: None,
        },
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    match id {
        "mjmcr_ema_xo" => Some(("mjmcr_ema_xo", "Johnson EMA XO(300,50)", "paper literal")),
        "mjmcr_dcbo" => Some(("mjmcr_dcbo", "Johnson DCBO(300,20)", "paper literal")),
        _ => None,
    }
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    match id {
        "mjmcr_ema_xo" => Some(vec![p("fast", "Fast EMA (bars)", 50., 5., 200., 5.), p("slow", "Slow EMA (bars)", 300., 50., 500., 10.)]),
        "mjmcr_dcbo" => Some(vec![p("entry", "Entry channel HH/LL (bars)", 300., 20., 500., 10.), p("exit", "Exit channel HH/LL (bars)", 20., 5., 100., 5.)]),
        _ => None,
    }
}

fn open(dir: f64, i: usize, px: f64, sig: usize) -> Trade {
    Trade { dir, legs: vec![Leg { i, px, w: 1. }], exit_i: i, exit_px: px, reason: String::new(), entry_kind: "open", signal_i: sig, open: false, exits: vec![] }
}

fn ema_xo(x: &Ctx, from: usize, to: usize, fast: usize, slow: usize) -> Vec<Trade> {
    let (ef, es) = (ema(&x.c, fast), ema(&x.c, slow));
    let mut out = vec![];
    let mut cur: Option<Trade> = None;
    for i in from.max(slow).max(1)..=to {
        let d = ef[i - 1] - es[i - 1];
        let want = if d > 0. { 1. } else if d < 0. { -1. } else { continue };
        match cur.as_ref() {
            Some(t) if t.dir == want => {}
            Some(_) => {
                let mut t = cur.take().unwrap();
                t.exit_i = i; t.exit_px = x.o[i]; t.signal_i = i - 1;
                t.reason = if want < 0. { "EMA50 < EMA300 · reverse at the open" } else { "EMA50 > EMA300 · reverse at the open" }.into();
                out.push(t);
                cur = Some(open(want, i, x.o[i], i - 1));
            }
            None => cur = Some(open(want, i, x.o[i], i - 1)),
        }
    }
    if let Some(mut t) = cur { t.exit_i = to; t.exit_px = x.c[to]; t.signal_i = to; t.reason = "open".into(); t.open = true; out.push(t); }
    out
}

fn hh(v: &[f64], i: usize, n: usize) -> f64 { v[i + 1 - n..=i].iter().copied().fold(f64::NEG_INFINITY, f64::max) }
fn ll(v: &[f64], i: usize, n: usize) -> f64 { v[i + 1 - n..=i].iter().copied().fold(f64::INFINITY, f64::min) }

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !id.starts_with("mjmcr_") || from >= to || to >= x.r.len() { return None; }
    let g = |k: &str, d: f64| p.get(k).copied().unwrap_or(d).max(1.) as usize;
    match id {
        "mjmcr_ema_xo" => Some(ema_xo(x, from, to, g("fast", 50.), g("slow", 300.))),
        "mjmcr_dcbo" => {
            let (n_in, n_out) = (g("entry", 300.), g("exit", 20.));
            let warm = n_in.max(n_out);
            let orders = |i: usize| Orders { buy: hh(&x.h, i - 1, n_in), sell: ll(&x.l, i - 1, n_in), xlong: ll(&x.l, i - 1, n_out), xshort: hh(&x.h, i - 1, n_out) };
            let mut out = vec![];
            stop_engine(x, from, to, warm, &orders, &mut out);
            for t in out.iter_mut() {
                t.reason = match t.reason.as_str() {
                    "exit long stop" => "LL20 stop (rule 5)".into(),
                    "exit short stop" => "HH20 stop (rule 4)".into(),
                    "reverse: sell stop" => "LL20 stop, then LL300 short entry (rules 5 and 3)".into(),
                    "reverse: buy stop" => "HH20 stop, then HH300 long entry (rules 4 and 2)".into(),
                    r => r.into(),
                };
            }
            Some(out)
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;
    fn rows(n: usize) -> Vec<Row> {
        (0..n).map(|i| Row { ticker: "T".into(), date: format!("{:05}", i), open: 100., high: 101., low: 99., close: 100., volume: None }).collect()
    }
    #[test]
    fn dcbo_enters_on_the_300_bar_high_and_exits_on_the_20_bar_low() {
        let mut r = rows(360);
        // bar 310 trades up through the 300-bar high (101) with a gap open at 100.5: buy stop fills at 101
        r[310].high = 104.; r[310].low = 100.; r[310].close = 103.; r[310].open = 100.5;
        for k in 311..330 { r[k] = Row { ticker: "T".into(), date: format!("{:05}", k), open: 103., high: 104., low: 102.5, close: 103., volume: None }; }
        // bar 330 falls through the 20-bar low (100, bar 310) and the 300-bar low (99): exit at 100, short at 99
        r[330].open = 102.; r[330].high = 102.; r[330].low = 98.; r[330].close = 98.;
        let x = Ctx::new(&r, &r, &[]);
        let ts = run("mjmcr_dcbo", &x, 0, 359, &Params::new()).unwrap();
        assert_eq!(ts[0].dir, 1.); assert_eq!(ts[0].legs[0].i, 310); assert_eq!(ts[0].legs[0].px, 101.);
        assert_eq!(ts[0].exit_i, 330); assert_eq!(ts[0].exit_px, 100.);
        assert_eq!(ts[1].dir, -1.); assert_eq!(ts[1].legs[0].i, 330); assert_eq!(ts[1].legs[0].px, 99.);
    }
    #[test]
    fn ema_xo_starts_after_priming_and_reverses_at_the_open() {
        let mut r = rows(700);
        for (i, row) in r.iter_mut().enumerate() { row.close = 100. + i as f64 * 0.1; row.open = row.close; }
        for k in 450..700 { r[k].close = 145. - (k - 450) as f64 * 0.5; r[k].open = r[k].close; }
        let x = Ctx::new(&r, &r, &[]);
        let ts = run("mjmcr_ema_xo", &x, 0, 699, &Params::new()).unwrap();
        assert_eq!(ts[0].dir, 1.); assert_eq!(ts[0].legs[0].i, 300);
        let (f, s) = (ema(&x.c, 50), ema(&x.c, 300));
        let flip = (300..699).find(|&i| f[i] < s[i]).unwrap();
        assert_eq!(ts[0].exit_i, flip + 1); assert_eq!(ts[1].dir, -1.); assert_eq!(ts[1].legs[0].px, r[flip + 1].open);
    }
}
