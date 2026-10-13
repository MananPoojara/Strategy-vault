//! Larry Williams, "Inner Circle Seminar" (ADEST, Sydney, June 2006; pages headed "Inner Circle - Sydney 2005"). Book id `lwic6`.
//! Source (read in place; COPY PENDING): research drive "USB HDD 2/tRADING KNO-2/Larry Williams - Inner Circle Seminar 2006
//! (adest.com.au)/InnerCircle.pdf" (104 pp, text read in full; charts unlabelled). Page refs = the manual's page footers.
//! Much of the manual repeats the 2000 workshop (lwicw) and Sure Thing (lwstc); those parts are listed as duplicates.
use crate::connors::{Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "lwic6";

macro_rules! sd {
    ($id:expr, $name:expr, $group:expr, $dir:expr, $status:expr, $rules:expr, $notes:expr) => {
        StrategyDef { id: $id, name: $name, group: $group, session: 0, dir: $dir, status: $status, needs_volume: false,
            needs_vix: false, needs_series: &[], book: BOOK, rules: $rules, notes: $notes, claim: None }
    };
}

pub fn catalogue() -> Vec<StrategyDef> {
    use Status::{Live, NeedsData, NotTestable};
    vec![
        sd!("lwic6_pctr_ma50_long", "%R with the 50-day average trend (long side)", "The power of %R", "Long", Live,
            &["F|%R = (today's close - lowest low) / (highest high - lowest low) over 10 days; \"greater than 75% tops are usually near and when less than 20% bottoms usually form\" (p.91)",
              "F|\"If the average was in an up trend use the %R buys signals to go long and the sell signals to exit but not sell short\"; down trend = \"today's value was less than yesterdays\" of the 50-day moving average (p.91-92)",
              "E|\"for a buy, ... simply wait for the market to rally above the prior days true high ... I use a trailing stop above each day's prior true high and do not count inside days\" (p.92): buy stop at the true high of the day %R is below 20 with the 50-day average rising, moved each day to the prior day's true high (inside days skipped) until filled",
              "X|Sell signal = \"%R to enter the sell zone and then we can place our stop at the true low of the day %R entered the sell zone and trail our stop up each day under the true lows\" (p.92): sell stop at the true low of the first day %R is above 75 while long, then the prior day's true low (inside days skipped)",
              "X|No other stop is given; the pending buy stop is not cancelled (the book gives no cancel rule)"],
            "Book: no test results printed. True high/low = high/low extended to the prior close. The short side is not testable (its exit is \"take profits or tighten stops\", judgement). India: same rules on the index."),
        sd!("lwic6_pctr_ma50_short", "%R with the 50-day average trend (short side)", "The power of %R", "Short",
            NotTestable("Exit is judgement: \"take profits or tighten stops when %R entered the buy zone\" (p.92)."),
            &["E|Down-trending 50-day average, %R in the sell zone, sell stop at the true low of that day trailed under the true lows (p.92)"], ""),
        sd!("lwic6_rnr", "R 'n R system (21-day RSI trend, 4-day %R, Oops!/hidden Oops!/ringed entries)", "R 'n R system", "Long/Short",
            NotTestable("The dollar stop amount and the 4-day %R buy/sell zone levels are not given, and the four exits are listed without saying how they combine."),
            &["F|21-day RSI > 50 only buys, < 50 only sells; 4-day %R for pullbacks (p.93)", "E|Original Oops!, hidden Oops!, ringed high/low formation", "X|$ stop loss; %R going to overbought/sold; trail a stop at prior days high/low; an opposite Oops! signal"], ""),
        sd!("lwic6_bonds", "Bonds: TDW, TDOM (with gold), holidays ($1,400 stop), gold filter", "Inside Circle short term", "Long/Short",
            NeedsData("Needs daily T-Bond futures and Gold futures."),
            &["E|Buy each weekday open, exit first profitable opening (TDW table); TDOM table with gold below 25 days ago; holiday table with a $1,400 stop, 01/10/78-07/06/99 190 trades $52,200 (p.86 ff.)"], "Same family as lwicw_bond_shortterm; different holiday offsets."),
        sd!("lwic6_setups", "WillVal, COT, large traders, ADX +60, pinch/paunch, sentiment, seasonals, OI, premiums", "Set ups", "Long/Short",
            NeedsData("Needs futures, gold/bond/dollar spreads, CFTC COT, sentiment and open interest; the set-ups are clustered by judgement (\"at least 4 ... 3 is the bare minimum\")."),
            &["F|See lwstc_setups (same text, p.5-51)"], ""),
        sd!("lwic6_entries_exits", "Channel breakout, 18-day MA, qualified trend line, ProGo, failure days, specialist trap, triangles, volatility breakout, stochastic exits, 3-bar exit", "Entry / exit techniques", "Long/Short",
            NotTestable("Entries without stated exits, judgement (trend lines, triangles, \"when the market is primed\"), or rules given only at the seminar; the 20/10 and 50/20 channel rules are the Turtle rules already coded as cov_turtle_s1/s2; the Monday gap bailout system (172 trades, p.84-85) is lwicw_mon_gap_bail."),
            &["E|See lwstc_entries (same text, p.52-80)"], ""),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    let f = |a, b, c| Some((a, b, c));
    match id {
        "lwic6_pctr_ma50_long" => f("lwic6_pctr", "%R with the trend", "Long"),
        "lwic6_pctr_ma50_short" => f("lwic6_pctr", "%R with the trend", "Short · not testable"),
        "lwic6_rnr" | "lwic6_entries_exits" => f("lwic6_other", "Other techniques", "Not testable"),
        "lwic6_bonds" | "lwic6_setups" => f("lwic6_other", "Other techniques", "Needs data"),
        _ => None,
    }
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    (id == "lwic6_pctr_ma50_long").then(|| vec![
        p("pctr_n", "%R days (book 10)", 10.0, 4.0, 20.0, 1.0),
        p("ma_n", "Trend moving average days (book 50)", 50.0, 10.0, 200.0, 5.0),
        p("buy_zone", "%R buy zone, below (book 20)", 20.0, 5.0, 40.0, 5.0),
        p("sell_zone", "%R sell zone, above (book 75)", 75.0, 60.0, 95.0, 5.0),
    ])
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if id != "lwic6_pctr_ma50_long" || from > to || to >= x.r.len() { return None; }
    let schema = params(id)?;
    let g = super::lwicw::getter(&schema, p);
    let (n, m, bz, sz) = (g("pctr_n") as usize, g("ma_n") as usize, g("buy_zone"), g("sell_zone"));
    let (o, h, l, c) = (&x.o, &x.h, &x.l, &x.c);
    let pctr = |i: usize| -> f64 {
        if i + 1 < n { return f64::NAN; }
        let (mut hh, mut ll) = (f64::MIN, f64::MAX);
        for k in i + 1 - n..=i { hh = hh.max(h[k]); ll = ll.min(l[k]); }
        if hh > ll { (c[i] - ll) / (hh - ll) * 100.0 } else { f64::NAN }
    };
    // rising m-day simple average <=> close above the close m days earlier
    let trend_up = |i: usize| i >= m && c[i] > c[i - m];
    let th = |i: usize| if i > 0 { h[i].max(c[i - 1]) } else { h[i] };
    let tl = |i: usize| if i > 0 { l[i].min(c[i - 1]) } else { l[i] };
    let inside = |i: usize| i > 0 && h[i] < h[i - 1] && l[i] > l[i - 1];
    let mk = |e: usize, px: f64, j: usize, ex: f64, why: &str, open: bool| Trade { dir: 1.0, legs: vec![Leg { i: e, px, w: 1.0 }],
        exit_i: j, exit_px: ex, reason: why.into(), entry_kind: "stop", signal_i: j, open, exits: vec![] };
    let mut out = vec![];
    let mut buy_stop: Option<f64> = None; // pending entry while flat
    let mut pos: Option<(usize, f64, Option<f64>)> = None; // (entry bar, price, armed exit stop)
    for i in from..=to {
        // fills during bar i: exit stop first (only armed from a prior close), then a pending buy stop
        if let Some((e, px, Some(s))) = pos {
            let fill = if o[i] <= s { Some(o[i]) } else if l[i] <= s { Some(s) } else { None };
            if let Some(f) = fill { out.push(mk(e, px, i, f, "sell signal: trailing stop under the true low", false)); pos = None; }
        } else if pos.is_none() {
            if let Some(b) = buy_stop {
                let fill = if o[i] >= b { Some(o[i]) } else if h[i] >= b { Some(b) } else { None };
                if let Some(f) = fill { pos = Some((i, f, None)); buy_stop = None; }
            }
        }
        // at the close of bar i: arm or trail the stops
        let r = pctr(i);
        match pos {
            Some((e, px, Some(_))) => { if !inside(i) { pos = Some((e, px, Some(tl(i)))); } }
            Some((e, px, None)) => { if r > sz { pos = Some((e, px, Some(tl(i)))); } }
            None => match buy_stop {
                Some(_) => { if !inside(i) { buy_stop = Some(th(i)); } }
                None => { if r < bz && trend_up(i) { buy_stop = Some(th(i)); } }
            },
        }
    }
    if let Some((e, px, _)) = pos { out.push(mk(e, px, to, c[to], "open", true)); }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;
    fn bar(d: &str, o: f64, h: f64, l: f64, c: f64) -> Row {
        Row { ticker: "T".into(), date: d.into(), open: o, high: h, low: l, close: c, volume: None }
    }
    #[test]
    fn pctr_trend_entry_and_exit() {
        // 3-day %R, 2-day trend. Bar 3 closes at the 3-day low with the close above 2 days ago -> buy stop at bar 3's true high
        let r = vec![bar("2000-01-03", 10., 11., 9., 9.5), bar("2000-01-04", 9.5, 12., 9.4, 11.), bar("2000-01-05", 11., 11.5, 9.6, 10.),
            bar("2000-01-06", 10., 10.2, 9.0, 9.6), bar("2000-01-07", 9.7, 10.5, 9.5, 10.4), bar("2000-01-10", 10.5, 12.5, 10.4, 12.4),
            bar("2000-01-11", 12.0, 12.2, 11.0, 11.2)];
        let x = Ctx::new(&r, &r, &[]);
        let p = Params::from([("pctr_n".to_string(), 3.0), ("ma_n".to_string(), 2.0)]);
        let t = run("lwic6_pctr_ma50_long", &x, 2, 6, &p).unwrap();
        // bar 2: %R = (10-9)/(12-9) = 33%; bar 3: 20% (not below 20) with c[3] < c[1] -> no setup at the book's 20
        assert!(t.is_empty());
        let p = Params::from([("pctr_n".to_string(), 3.0), ("ma_n".to_string(), 2.0), ("buy_zone".to_string(), 40.0)]);
        let t = run("lwic6_pctr_ma50_long", &x, 2, 6, &p).unwrap();
        // bar 2: 33% < 40 and c[2]=10 > c[0]=9.5 -> buy stop = true high of bar 2 = 11.5; bar 3 high 10.2 no fill, stop -> 10.2;
        // bar 4 high 10.5 >= 10.2 -> buy 10.2; bar 5 %R 97 > 75 -> sell stop at bar 5 true low 10.4; bar 6 low 11 no fill and
        // is an inside day (stop kept); window ends -> open trade
        assert_eq!((t.len(), t[0].legs[0].i, t[0].legs[0].px, t[0].open), (1, 4, 10.2, true));
    }
}
