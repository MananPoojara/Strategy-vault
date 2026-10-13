//! Jay Kaeppel, Robin Lofton & Brad Zigler, Optionetics "Futures & Commodity Home Study Course"
//! (2007), manual "Home Study Course 2" (Futures and Commodities I), ch.1 "Advanced Market Analysis".
//! Book id `jkofc`. Source: research drive `USB HDD 2/tRADING KNO-2/George A.Fontanills & Jay Kaeppel -
//! Optionetics Futures & Commodity Home Study Course (2007, Videos & Manuals) (optionetics.com)/Home Study Course 2.pdf`.
//! Notes: library/jkofc/README.md.
use crate::connors::{Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};

pub fn catalogue() -> Vec<StrategyDef> {
    let d = |id, name, dir, status, rules, notes| StrategyDef {
        id, name, group: "Ch.1 Advanced Market Analysis", session: 0, dir, status, needs_volume: false, needs_vix: false,
        needs_series: &[], book: "jkofc", rules, notes, claim: None,
    };
    vec![
        d("jkofc_sma4_reverse", "4-day SMA close stop-and-reverse", "Long/Short", Status::Live,
          &["F|Four-day simple moving average of daily closes (p.5, gold example)",
            "E|\"you'll be long whenever the daily closing price is above the moving average and you'll be short whenever the market closes below the moving average\" (p.5)",
            "E|\"Since your signal is dependent on the market's closing price, your orders to buy and sell must be executed on the opening of the following trading day\" (p.5)",
            "X|Opposite close reverses at the next open (Table 1.1: day 7 close below the average, \"On the open of day 8, two February gold contracts are sold\")"],
          "Book presents it as a \"crude trading system\" built from the moving average (p.5); worked example only (February gold, Table 1.1), no statistics. A close equal to the average changes nothing (the book gives no rule for it). Futures in the book; run on the index (instrument swap). Always in the market once the first signal fires."),
        d("jkofc_ma_crossover", "4/12-day moving-average crossover", "Long/Short",
          Status::NotTestable("\"a buy signal flashes whenever the shorter moving average crosses above ... A sale is heralded when [it] falls below\" (p.6): the book does not say whether a sale only exits or also goes short, and fig.1.1 (4 and 12 days) is an illustration"),
          &["E|\"a buy signal flashes whenever the shorter moving average crosses above the trendline of the longer moving average. A sale is heralded when the trendline of the shorter indicator falls below that of the longer-term average\" (p.6)"],
          "Not coded: the meaning of \"sale\" is not stated."),
        d("jkofc_discretionary", "Oscillators, chart patterns and futures-options examples", "Long/Short",
          Status::NotTestable("indicator descriptions (ROC, PPO, stochastics, RSI), chart reading and option-strategy examples on futures with no entry/exit rules; option P&L needs option chains"),
          &["F|Ch.1-4 of Home Study Course 2 and Home Study Course 1 / Quick Start Guide"], "Teaching material."),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    Some(match id {
        "jkofc_sma4_reverse" => ("jkofc_ma", "Moving-average systems", "4-day SMA close reversal"),
        "jkofc_ma_crossover" => ("jkofc_ma", "Moving-average systems", "4/12 crossover (not testable)"),
        "jkofc_discretionary" => ("jkofc_disc", "Teaching material", "Not testable"),
        _ => return None,
    })
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    (id == "jkofc_sma4_reverse").then(|| vec![ParamDef { key: "n", label: "SMA length (days)", def: 4., min: 2., max: 20., step: 1. }])
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if id != "jkofc_sma4_reverse" { return None; }
    let n = p.get("n").copied().unwrap_or(4.).max(1.) as usize;
    // window mean summed afresh each bar (a running sum drifts, and "close = average" must stay exact)
    let ma: Vec<f64> = (0..x.c.len()).map(|i| if i + 1 < n { f64::NAN } else { x.c[i + 1 - n..=i].iter().sum::<f64>() / n as f64 }).collect();
    let mut out = vec![];
    let mut cur: Option<Trade> = None;
    for t in from..to {
        if !ma[t].is_finite() || (x.c[t] - ma[t]).abs() <= 1e-9 * x.c[t].abs() { continue; } // equal: no rule, hold
        let want = if x.c[t] > ma[t] { 1. } else { -1. };
        if cur.as_ref().is_some_and(|c| c.dir == want) { continue; }
        let b = t + 1;
        if let Some(mut old) = cur.take() {
            old.exit_i = b; old.exit_px = x.o[b]; old.signal_i = t; old.reason = "opposite close vs SMA · next open".into();
            out.push(old);
        }
        cur = Some(Trade { dir: want, legs: vec![Leg { i: b, px: x.o[b], w: 1. }], exit_i: b, exit_px: x.o[b], reason: String::new(),
            entry_kind: "open", signal_i: t, open: false, exits: vec![] });
    }
    if let Some(mut t) = cur { t.exit_i = to; t.exit_px = x.c[to]; t.signal_i = to; t.reason = "open".into(); t.open = true; out.push(t); }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;
    #[test]
    fn reproduces_table_1_1_gold_example() {
        // Feb gold closes, days 1-7 (p.4-5); opens set = close for the check.
        let c = [605.20, 610.70, 612.30, 619.90, 625.80, 621.30, 611.80, 611.0];
        let r: Vec<Row> = c.iter().enumerate().map(|(i, &v)| Row { ticker: "GC".into(), date: format!("2000-01-{:02}", i + 1),
            open: v, high: v, low: v, close: v, volume: None }).collect();
        let x = Ctx::new(&r, &r, &[]);
        let t = run("jkofc_sma4_reverse", &x, 0, 7, &Params::new()).unwrap();
        assert_eq!(t[0].dir, 1.); assert_eq!(t[0].legs[0].i, 4); // day 4 close above -> buy day 5 open
        assert_eq!(t[0].exit_i, 7); // day 7 close below -> reverse day 8 open
        assert_eq!(t[1].dir, -1.); assert_eq!(t[1].legs[0].i, 7);
    }
}
