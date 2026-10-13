//! Jake Bernstein & Elliott Bernstein, "Stock Market Strategies That Work" (McGraw-Hill, 2002). Book id `jbsmsw`.
//! Source copy: Books/Jake Bernstein/Stock Market Strategies That Work.pdf (printed page = PDF page - 20).
//! Transcription and checks: library/jbsmsw/. Helpers come from `jb624`.
use super::jb624::{finish, getp, limit_fill, mac, mark_open, new_trade};
use crate::connors::{sma, Ctx, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "jbsmsw";

macro_rules! def {
    ($id:expr, $name:expr, $group:expr, $dir:expr, $status:expr, $rules:expr, $notes:expr) => {
        StrategyDef { id: $id, name: $name, group: $group, session: 0, dir: $dir, status: $status, needs_volume: false,
            needs_vix: false, needs_series: &[], book: BOOK, rules: $rules, notes: $notes, claim: None }
    };
}

pub fn catalogue() -> Vec<StrategyDef> {
    use Status::{Live, NotTestable};
    vec![
        def!("jbsmsw_mom_ma", "MOM/MA 28 · buy on the cross up, out on the cross down", "Ch.10 · Implementing Market Strategies", "Long", Live,
            &["F|\"One of our favorite indicators is the 28-period momentum moving average (MOM/MA). This indicator combines a 28-period momentum with a 28-period moving average derivative of that momentum\" (p.129)",
              "E|\"the MOM/MA method signals a buy when momentum crosses above its MA\" (p.130): buy on the close of that day",
              "X|\"and a sell when momentum crosses below its MA\" (p.130); \"Their trade ends only when momentum once again falls below its moving average\" (p.133): sell on the close"],
            "Momentum = close - close 28 bars ago; simple 28-bar MA of it. Long only (a stock book: the sell signal ends the holding). The authors' own CHINA trades used judgement on top (early profit target, delayed entry, p.130-134); those overrides are not coded. Meant for stocks: run on NSE basket stocks or the indices."),
        def!("jbsmsw_mac_pullback", "MAC pullback buy at the channel low, out on the opposite MAC signal", "Ch.8 · Support and Resistance: the MAC", "Long/Short", Live,
            &["F|\"a 10-day moving average of the daily highs and an 8-day moving average of the daily lows\" (p.95)",
              "F|\"The trend: An uptrend develops once there have been two or more successive price bars completely above the top of the MAC. A downtrend develops when there have been two or more successive bars below the bottom of the MAC\" (p.97)",
              "E|\"In an uptrend, the trader will place an order to buy at the MAC low. In a downtrend, the trader will place an order to sell short at the MAC high\" (p.97): resting limit at yesterday's MA",
              "X|\"For more aggressive traders, a sell signal will get you out of your long position or a buy signal will get you out of your short position\" (p.97); \"you can exit if there are two consecutive bars outside the MAC\" (p.112): exit on the close of the opposite trigger",
              "X|\"For less risk, use a 10 percent closing basis stop of the price at which you entered the stock\" (p.97): p.stop_pct=10 (non-default; 0 = off)"],
            "Book: risk is either the opposite signal or a 10% closing stop; the opposite signal is the only one that also ends winning trades, so it is the default exit. The trailing stop below the last three bars starts only after \"a reasonable profit\" (no size given, p.112) and is not coded; channel surfing (exit at the MAC high) is jb624_mac_sr."),
        def!("jbsmsw_add", "Advance/Decline derivative (ADD)", "Ch.6 · Technical Approach", "Long/Short", NotTestable("the length of the moving average of the A/D oscillator is not printed; authors: \"objective but not entirely systematic\" and it needs a stop added"),
            &["E|\"If I calculate a moving average of the A/D oscillator ... use the derivative value and the A/D value for generating signals\" (p.78)"], ""),
        def!("jbsmsw_3610_parabolic", "3-6-10 moving averages with parabolic", "Ch.9 · Two Real-Time Examples", "Long", NotTestable("a narrated trade with judgement (bought before the parabolic confirmed, held on 'confidence')"),
            &["E|\"The buy signal came at the end of the trading on the 9th, when the 3- and 6-day moving average lines crossed above the 10-day moving average line\" (p.124)"], ""),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    Some(match id {
        "jbsmsw_mom_ma" => ("jbsmsw_mom_ma", "MOM/MA 28 (stocks)", "Long only, cross down exit"),
        "jbsmsw_mac_pullback" => ("jbsmsw_mac", "MAC support/resistance (stocks)", "Buy MAL / sell MAH, opposite-signal exit"),
        "jbsmsw_add" => ("jbsmsw_add", "A/D derivative", "Not testable"),
        "jbsmsw_3610_parabolic" => ("jbsmsw_3610", "3-6-10 MA + parabolic", "Not testable"),
        _ => return None,
    })
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    Some(match id {
        "jbsmsw_mom_ma" => vec![p("mom", "Momentum length", 28., 5., 60., 1.), p("ma", "MA of momentum (bars)", 28., 5., 60., 1.)],
        "jbsmsw_mac_pullback" => vec![p("mah", "MA of highs (bars)", 10., 5., 20., 1.), p("mal", "MA of lows (bars)", 8., 4., 20., 1.), p("bars", "Successive complete bars for a trend", 2., 1., 4., 1.),
            p("stop_pct", "Closing-basis stop, % from entry (0 = off; book alternative 10)", 0., 0., 20., 1.)],
        _ => return None,
    })
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !id.starts_with("jbsmsw_") || params(id).is_none() || to >= x.c.len() || from > to {
        return None;
    }
    let g = |k: &str| getp(id, p, k);
    let u = |k: &str| g(k).round().max(1.0) as usize;
    let mut out = Vec::new();
    let mut cur: Option<Trade> = None;
    match id {
        "jbsmsw_mom_ma" => {
            let k = u("mom");
            let mom: Vec<f64> = (0..x.c.len()).map(|i| if i >= k { x.c[i] - x.c[i - k] } else { f64::NAN }).collect();
            let ma = sma(&mom, u("ma"));
            let st = |i: usize| -> i8 { let e = 1e-9 * x.c[i].abs(); if mom[i] - ma[i] > e { 1 } else if ma[i] - mom[i] > e { -1 } else { 0 } };
            for i in from.max(1)..=to {
                if !ma[i - 1].is_finite() {
                    continue;
                }
                let (s0, s1) = (st(i - 1), st(i));
                if cur.is_some() {
                    if s1 == -1 && s0 != -1 {
                        out.push(finish(cur.take().unwrap(), i, x.c[i], i, "Momentum crossed below its MA"));
                    }
                    continue;
                }
                if i < to && s1 == 1 && s0 != 1 {
                    cur = Some(new_trade(1.0, i, x.c[i], "close", i));
                }
            }
        }
        "jbsmsw_mac_pullback" => {
            let m = mac(x, u("mah"), u("mal"), u("bars"));
            let stop = g("stop_pct") / 100.0;
            for i in from.max(1)..=to {
                if let Some(t) = cur.as_ref() {
                    let d = t.dir;
                    if m.trig[i] as f64 == -d {
                        out.push(finish(cur.take().unwrap(), i, x.c[i], i, "Opposite MAC signal"));
                    } else if stop > 0.0 && (x.c[i] - t.legs[0].px) * d <= -stop * t.legs[0].px {
                        out.push(finish(cur.take().unwrap(), i, x.c[i], i, "Closing-basis stop"));
                    }
                    continue;
                }
                let d = m.trend[i - 1];
                if d == 0 {
                    continue;
                }
                let lvl = if d > 0 { m.mal[i - 1] } else { m.mah[i - 1] };
                if let Some(px) = limit_fill(x, i, d as f64, lvl) {
                    let t = new_trade(d as f64, i, px, "limit", i - 1);
                    // the entry bar's close can already be the opposite trigger or the stop
                    if m.trig[i] == -d {
                        out.push(finish(t, i, x.c[i], i, "Opposite MAC signal"));
                    } else if stop > 0.0 && (x.c[i] - px) * d as f64 <= -stop * px {
                        out.push(finish(t, i, x.c[i], i, "Closing-basis stop"));
                    } else {
                        cur = Some(t);
                    }
                }
            }
        }
        _ => return Some(vec![]),
    }
    mark_open(&mut out, cur, x, to);
    Some(out)
}
