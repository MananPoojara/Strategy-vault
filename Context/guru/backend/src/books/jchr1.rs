//! Jeff Cooper, "Hit and Run Trading: The Short-Term Stock Traders' Bible" (1996; updated edition,
//! Marketplace Books 2004). Book id "jchr1". Source: Books/Jeff Cooper/Hit and Run Trading Updated Version.pdf
//! (scan, read page by page; page refs = printed pages). The author: "Even though most of my entries are
//! predetermined by mechanical setups, my exits are pretty much discretionary" (Appendix p.158), so only the
//! Lizards, whose exit is printed ("sell the position on the close", p.114), are Live. Notes: library/jchr1/.
use crate::connors::{Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "jchr1";

const LIZ_LONG: &[&str] = &[
    "E|\"Today's (day one) open and close must be in the top 25 percent of its daily range.\" (p.113): (open - low) and (close - low) >= 75% of (high - low)",
    "E|\"Today's low must be a 10-trading-day low.\" (p.114): low <= the lowest low of the 10 trading days ending today (ties count)",
    "E|\"Tomorrow buy 1/8 above the day-one high.\" (p.114): buy stop, a gap above it fills at the open (Example 14.2 short fills on the open gap)",
    "X|\"Maximum risk is 1 point and if you are not stopped out, sell the position on the close.\" (p.114): stop = fill - 1 point; stopped if the day's low reaches it [daily bars cannot order fill and stop: conservative], else exit on the close",
];
const LIZ_SHORT: &[&str] = &[
    "E|\"Today's (day one) open and close must be in the bottom 25 percent of its daily range.\" (p.114)",
    "E|\"Today's high must be a 10-trading-day high.\" (p.114)",
    "E|\"Tomorrow sell short 1/8 below the day-one low.\" (p.114): sell stop; a gap below fills at the open (Example 14.2 Amgen)",
    "X|\"Maximum risk is 1 point and if you are not stopped out, cover the short position on the close.\" (p.114)",
];
const LIZ_NOTES: &str = "\"I use this strategy strictly for day trading\" (p.113). No statistics in the book (examples: Computer Associates, Amgen, Atmel, United Airlines 1995-96). \
Book traded US stocks priced about $30-$170 (1995-96); that universe is not in the vault. India = instrument swap: run on NSE stocks (basket tickers) or the indices. \
The 1-point risk and the 1/8 tick are kept literal in the ticker's own price units; on an index (NIFTY ~ thousands of points) a 1-point stop is hit on almost every trade - the honest literal result. \
p.risk_pct is a labelled non-default alternative [Not from the book]. The 2004 postscript (p.17) says to use 10 cents instead of 1/8 for decimal-priced stocks: set p.tick = 0.10. Short-sale uptick rule not modelled.";

const EXIT: &str = "exit is discretionary: the book gives only an initial stop and trailing stops (\"my exits are pretty much discretionary\", p.158)";

pub fn catalogue() -> Vec<StrategyDef> {
    let d = |id, name, group, dir, status, rules, notes| StrategyDef {
        id, name, group, session: 0, dir, status, needs_volume: false, needs_vix: false,
        needs_series: &[], book: BOOK, rules, notes, claim: None,
    };
    use Status::*;
    vec![
        d("jchr1_lizard_long", "Lizards · buys (day trade)", "Ch 14 · Lizards", "Long", Live, LIZ_LONG, LIZ_NOTES),
        d("jchr1_lizard_short", "Lizards · short sales (day trade)", "Ch 14 · Lizards", "Short", Live, LIZ_SHORT, LIZ_NOTES),
        d("jchr1_expansion_breakout", "Expansion Breakouts / Breakdowns", "Ch 5 · Expansion Breakouts", "Long/Short", NotTestable(EXIT),
          &["E|\"Today (day one) must be a two-month calendar high.\" \"Today's range (high minus low) must be equal to or larger than the largest daily range of the previous nine trading days.\" (p.20)",
            "E|\"Tomorrow only, buy 1/8 point above today's high.\" (p.20); shorts reversed",
            "X|\"Initial maximum risk (stop) is 1 point under yesterday's close.\" (p.20) - no profit exit"], ""),
        d("jchr1_123_pullback", "1-2-3-Pullbacks", "Ch 6 · 1-2-3-Pullbacks", "Long/Short", NotTestable(EXIT),
          &["F|14-day ADX greater than 30 (or RS >= 95); +DI > -DI for buys (p.32)",
            "E|1-2-3 correction: three consecutive lower lows or two lower lows and an inside day; day four only, buy 1/8 above the day-three high (p.32)",
            "X|Initial stop \"near the day-three low\"; \"As the position moves in your favor, you should trail your stops\" (p.32)"],
          "Same rules as ats_1234 (Connors, Advanced Trading Strategies, developed with Cooper) - already in the vault there (with a vault time exit)."),
        d("jchr1_stepping_size", "Stepping in Front of Size", "Ch 7 · Stepping in Front of Size", "Long",
          NotTestable("needs Level II bid/size data; profit taking \"is very subjective\" (p.49)"),
          &["F|ADX > 30 and +DI > -DI or RS >= 95; average volume under 200,000 shares (500,000 in the 2004 postscript); stock up on the day (p.48)",
            "E|Two consecutive higher bids with size (5,000+ shares); pay the offer (p.48-49)", "X|Stop 1/8 under the original 5,000-share bid (p.49)"], ""),
        d("jchr1_expansion_pivot", "Expansion Pivots", "Ch 8 · Expansion Pivots", "Long/Short", NotTestable(EXIT),
          &["E|\"Today's range is greater than the daily range of the past nine trading sessions.\" \"Either yesterday or today, the stock is trading at or below the 50-day moving average and explodes higher.\" (p.60)",
            "E|\"Tomorrow, buy 1/8 above the explosion-day high.\" (p.60)", "X|\"Our initial protective stop is 1 point below the explosion day's close.\" (p.60)"],
          "Also 'explodes higher' is not defined beyond the range rule."),
        d("jchr1_180", "180's", "Ch 9 · 180's", "Long/Short", NotTestable(EXIT),
          &["E|Day one closes in the bottom 25% of its range, day two in the top 25% (p.70)",
            "F|Day two close greater than both its 10-day and 50-day moving averages (p.70)",
            "E|\"Day three only, buy 1/8 point above the day-two high.\" (p.70)", "X|\"Our initial protective stop is 1 point under our entry.\" (p.70)",
            "X|\"Even though I like to day-trade with this strategy, I will many times carry the position overnight\" (p.78)"], ""),
        d("jchr1_gilligan", "Gilligan's Island", "Ch 10 · Gilligan's Island", "Long/Short",
          NotTestable("exit undefined: \"Carry the position overnight if it closes strongly\" (p.82) - strong close and the next-day exit are not specified"),
          &["E|Buys: gap open to a new two-month low; close at or in the top 50% of the range and at or above the open; next day only buy 1/8 above today's high (p.82)",
            "X|\"Risk 1 point.\" \"Carry the position overnight if it closes strongly.\" (p.82)"], ""),
        d("jchr1_boomers", "Boomers", "Ch 11 · Boomers", "Long/Short", NotTestable(EXIT),
          &["F|ADX > 30 and +DI > -DI or RS >= 95 (shorts: -DI > +DI \"or the stock must be downtrending strongly\") (p.89)",
            "E|Two consecutive inside days (day two inside day one, day three inside day two); day four only buy 1/8 above the day-three high (p.90)",
            "X|\"Our stop will be 1/8 below the day-three low.\" (p.90)"], ""),
        d("jchr1_slingshots", "Slingshots", "Ch 12 · Slingshots", "Long/Short", NotTestable(EXIT),
          &["E|Day one new two-month high; day two takes out the day-one low by at least 1/8; day two or three only buy 1/8 above the day-one high (p.98)",
            "X|\"Risk two points.\" (p.98)"], ""),
        d("jchr1_whoops", "Whoops", "Ch 13 · Whoops", "Short",
          NotTestable("intraday: re-entry after a stop the same day and 'moves strongly in your favor' need intraday bars and judgement; no profit exit"),
          &["F|Trading under its 10- and 50-day moving averages; previous close below its open (p.105)",
            "E|Today's open at least 1/4 point above yesterday's close; sell short 1/8 below yesterday's close (p.105)",
            "X|Maximum initial risk 1 point; if stopped out re-enter at the entry point (today only); if it moves strongly in your favor move the stop to 1/8 below entry (p.105)"], ""),
        d("jchr1_secondaries", "Secondaries", "Ch 15 · Secondaries", "Long",
          NeedsData("secondary-offering calendar with offering prices (Barron's) - not in the vault"),
          &["F|Close above the 20-day moving average the night the offering is priced (p.120)",
            "E|Buy if the offering opens 1/4 point or more above the offering price (p.120)",
            "X|Stop 1/8 under the offering price; a loser is sold on the close; a winner may be sold on the close or held (p.120)"],
          "Even with the data the winner exit is a choice (\"you have the choice\")."),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    Some(match id {
        "jchr1_lizard_long" => ("jchr1_lizard", "Lizards", "Buys · day trade"),
        "jchr1_lizard_short" => ("jchr1_lizard", "Lizards", "Short sales · day trade"),
        s if s.starts_with("jchr1_") => ("jchr1_other", "Hit and Run Trading · other setups", "Not testable"),
        _ => return None,
    })
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    matches!(id, "jchr1_lizard_long" | "jchr1_lizard_short").then(|| vec![
        p("pct", "Open and close within this % of the range end (book 25)", 25., 10., 40., 5.),
        p("n", "N-trading-day low/high (book 10)", 10., 5., 20., 1.),
        p("tick", "Entry offset in price units (book 1/8; 2004 postscript 0.10)", 0.125, 0.01, 0.25, 0.005),
        p("risk_pts", "Maximum risk in price points (book 1)", 1., 0.25, 5., 0.25),
        p("risk_pct", "[Not from the book] risk as % of entry instead (0 = off, use points)", 0., 0., 3., 0.25),
    ])
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    let dir = match id { "jchr1_lizard_long" => 1., "jchr1_lizard_short" => -1., _ => return None };
    let schema = params(id)?;
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).unwrap().def);
    let (q, n, tick, risk_pts, risk_pct) = (g("pct") / 100., g("n").max(1.) as usize, g("tick"), g("risk_pts"), g("risk_pct"));
    let mut out = vec![];
    if to >= x.r.len() { return Some(out); }
    for t in from.max(n) + 1..=to {
        let s = t - 1; // day one
        let rng = x.h[s] - x.l[s];
        if !(rng > 0.) { continue; }
        let (po, pc) = ((x.o[s] - x.l[s]) / rng, (x.c[s] - x.l[s]) / rng);
        let setup = if dir > 0. {
            po >= 1. - q && pc >= 1. - q && x.l[s] <= x.l[s + 1 - n..=s].iter().copied().fold(f64::INFINITY, f64::min)
        } else {
            po <= q && pc <= q && x.h[s] >= x.h[s + 1 - n..=s].iter().copied().fold(f64::NEG_INFINITY, f64::max)
        };
        if !setup { continue; }
        let level = if dir > 0. { x.h[s] + tick } else { x.l[s] - tick };
        if (dir > 0. && x.h[t] < level) || (dir < 0. && x.l[t] > level) { continue; }
        let px = if dir > 0. { x.o[t].max(level) } else { x.o[t].min(level) };
        let risk = if risk_pct > 0. { px * risk_pct / 100. } else { risk_pts };
        let stop = px - dir * risk;
        let hit = if dir > 0. { x.l[t] <= stop } else { x.h[t] >= stop };
        out.push(Trade { dir, legs: vec![Leg { i: t, px, w: 1. }], exit_i: t, exit_px: if hit { stop } else { x.c[t] },
            reason: if hit { "Maximum risk stop".into() } else { "Exit on the close".into() },
            entry_kind: "stop", signal_i: s, open: false, exits: vec![] });
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;
    #[test]
    fn hand_checked_lizard_long_and_short() {
        let mut r: Vec<Row> = (0..20).map(|i| Row { ticker: "T".into(), date: format!("2000-{:03}", i),
            open: 50., high: 51., low: 49., close: 50., volume: None }).collect();
        r[15] = Row { open: 50.8, high: 51., low: 47., close: 50.9, ..r[15].clone() }; // tail: 10-day low, O and C in top 25%
        r[16] = Row { open: 50.5, high: 52.5, low: 50.4, close: 52., ..r[16].clone() }; // buy 51.125, stop 50.125 not hit
        let x = Ctx::new(&r, &r, &[]);
        let t = run("jchr1_lizard_long", &x, 0, 19, &Params::new()).unwrap();
        assert_eq!(t.len(), 1);
        assert_eq!((t[0].legs[0].i, t[0].legs[0].px, t[0].exit_px), (16, 51.125, 52.));
        assert!(run("jchr1_180", &x, 0, 19, &Params::new()).is_none());
        r[16].low = 50.; // now the 1-point stop at 50.125 is hit
        let t = run("jchr1_lizard_long", &Ctx::new(&r, &r, &[]), 0, 19, &Params::new()).unwrap();
        assert_eq!(t[0].exit_px, 50.125);
        // short: 10-day high with open and close in the bottom 25%, gap below the entry fills at the open
        let mut r: Vec<Row> = (0..20).map(|i| Row { ticker: "T".into(), date: format!("2000-{:03}", i),
            open: 50., high: 51., low: 49., close: 50., volume: None }).collect();
        r[15] = Row { open: 49.2, high: 53., low: 49., close: 49.1, ..r[15].clone() };
        r[16] = Row { open: 48.5, high: 49., low: 47., close: 47.5, ..r[16].clone() };
        let t = run("jchr1_lizard_short", &Ctx::new(&r, &r, &[]), 0, 19, &Params::new()).unwrap();
        assert_eq!((t.len(), t[0].legs[0].px, t[0].exit_px), (1, 48.5, 47.5));
    }
}
