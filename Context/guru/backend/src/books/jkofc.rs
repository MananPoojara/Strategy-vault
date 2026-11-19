//! Jay Kaeppel, Robin Lofton & Brad Zigler, Optionetics "Futures & Commodity Home Study Course"
//! (2007), manual "Home Study Course 2" (Futures and Commodities I), ch.1 "Advanced Market Analysis".
//! Book id `jkofc`. Source: research drive `USB HDD 2/tRADING KNO-2/George A.Fontanills & Jay Kaeppel -
//! Optionetics Futures & Commodity Home Study Course (2007, Videos & Manuals) (optionetics.com)/Home Study Course 2.pdf`.
//! Notes: library/jkofc/README.md.
use crate::connors::{Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};
/// Admitted ids (authors/lanea/singles/decisions.json).
pub const REVIEWED_LANEA: &[&str] = &["jkofc_sma4_reverse", "jkofc_ma_crossover"];
pub fn reviewed_lanea(id: &str) -> bool { REVIEWED_LANEA.contains(&id) }

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
        d("jkofc_ma_crossover", "4/12-day moving-average crossover", "Long", Status::Live,
          &["F|Short average 4 days, long average 12 days (Fig.1.1, p.6); simple averages of daily closes [A: the page names no type; the 4-day SMA of p.5 is the only average the chapter defines]",
            "E|\"a buy signal flashes whenever the shorter moving average crosses above the trendline of the longer moving average\" (p.6): buy at the next open after the first close with the 4-day average above the 12-day average",
            "X|\"A sale is heralded when the trendline of the shorter indicator falls below that of the longer-term average\" (p.6): sell at the next open after the first close with the 4-day average below the 12-day average",
            "A|Detail 1 (R4 #343): the page names no average type; simple averages, the only average the chapter defines (4-day SMA, p.5)",
            "A|Detail 2: a cross needs the averages to change sides; equal averages change nothing; the first position opens at the first cross after both averages exist, never from the starting state",
            "A|Option, not the default: p.rev = 1 also goes short on the sale (the book says only \"sale\"; long and flat is the literal reading). Orders fill at the next open [engine convention]; the 4/12 lengths are Fig.1.1's, counted as printed"],
          "Book gives a figure and no statistics (UNVERIFIED). Futures in the book; run on indices, ETFs and stocks (declared application)."),
        d("jkofc_discretionary", "Oscillators, chart patterns and futures-options examples", "Long/Short",
          Status::NotTestable("rules not fully defined: teaching material - indicator descriptions (ROC, PPO, stochastics, RSI), chart reading and strategy examples on futures, with no entry or exit rules (Futures & Commodity Course, Ch.1-4 of Home Study Course 2 and the Quick Start Guide)"),
          &["F|Ch.1-4 of Home Study Course 2 and Home Study Course 1 / Quick Start Guide"], "Teaching material."),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    Some(match id {
        "jkofc_sma4_reverse" => ("jkofc_ma", "Moving-average systems", "4-day SMA close reversal"),
        "jkofc_ma_crossover" => ("jkofc_ma", "Moving-average systems", "4/12-day crossover"),
        "jkofc_discretionary" => ("jkofc_disc", "Teaching material", "Not testable"),
        _ => return None,
    })
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    match id {
        "jkofc_sma4_reverse" => Some(vec![p("n", "SMA length (days)", 4., 2., 20., 1.)]),
        "jkofc_ma_crossover" => Some(vec![p("short", "Short average (days; Fig.1.1: 4)", 4., 2., 30., 1.), p("long", "Long average (days; Fig.1.1: 12)", 12., 3., 60., 1.),
            p("rev", "Sale also goes short (option; default 0 = the sale exits, long and flat)", 0., 0., 1., 1.)]),
        _ => None,
    }
}

fn sma(c: &[f64], n: usize) -> Vec<f64> {
    // window mean summed afresh each bar (a running sum drifts, and "average = average" must stay exact)
    (0..c.len()).map(|i| if i + 1 < n { f64::NAN } else { c[i + 1 - n..=i].iter().sum::<f64>() / n as f64 }).collect()
}

fn crossover(x: &Ctx, from: usize, to: usize, p: &Params) -> Vec<Trade> {
    let g = |k: &str, d: f64| p.get(k).copied().unwrap_or(d);
    let (ns, nl, rev) = (g("short", 4.).max(1.) as usize, g("long", 12.).max(2.) as usize, g("rev", 0.) >= 0.5);
    let (ms, ml) = (sma(&x.c, ns), sma(&x.c, nl));
    let mut out = vec![];
    let mut cur: Option<Trade> = None;
    let mut prev = 0.; // side of the last bar on which the averages differed
    for t in 0..to {
        let (a, b) = (ms[t], ml[t]);
        if !a.is_finite() || !b.is_finite() || (a - b).abs() <= 1e-9 * b.abs() { continue; } // equal: nothing changes
        let s = if a > b { 1. } else { -1. };
        let crossed = prev != 0. && s != prev;
        prev = s;
        if !crossed || t < from { continue; }
        let f = t + 1;
        if let Some(mut old) = cur.take() {
            old.exit_i = f; old.exit_px = x.o[f]; old.signal_i = t; old.reason = "average cross · next open".into();
            out.push(old);
        }
        if s > 0. || rev {
            cur = Some(Trade { dir: s, legs: vec![Leg { i: f, px: x.o[f], w: 1. }], exit_i: f, exit_px: x.o[f], reason: String::new(),
                entry_kind: "open", signal_i: t, open: false, exits: vec![] });
        }
    }
    if let Some(mut t) = cur { t.exit_i = to; t.exit_px = x.c[to]; t.signal_i = to; t.reason = "open".into(); t.open = true; out.push(t); }
    out
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if id == "jkofc_ma_crossover" { return Some(crossover(x, from, to, p)); }
    if id != "jkofc_sma4_reverse" { return None; }
    let n = p.get("n").copied().unwrap_or(4.).max(1.) as usize;
    let ma = sma(&x.c, n);
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
    #[test]
    fn crossover_buys_on_cross_and_reverses() {
        // closes fall, rise (4 crosses above 12 at the rise), fall again (4 crosses below)
        let c: Vec<f64> = (0..14).map(|i| 100. - i as f64).chain((0..10).map(|i| 87. + 3. * i as f64)).chain((0..12).map(|i| 114. - 4. * i as f64)).collect();
        let r: Vec<Row> = c.iter().enumerate().map(|(i, &v)| Row { ticker: "X".into(), date: format!("2000-02-{:02}", i + 1),
            open: v, high: v, low: v, close: v, volume: None }).collect();
        let x = Ctx::new(&r, &r, &[]);
        let mut sr = Params::new(); sr.insert("rev".into(), 1.);
        let s = run("jkofc_ma_crossover", &x, 0, r.len() - 1, &sr).unwrap();
        assert!(s.len() >= 2 && s[0].dir == 1. && s[1].dir == -1. && s[0].exit_i == s[1].legs[0].i);
        let l = run("jkofc_ma_crossover", &x, 0, r.len() - 1, &Params::new()).unwrap();
        assert!(l.iter().all(|z| z.dir == 1.) && l.len() == 1);
    }
    /// Reference feed: `JK_DUMP=out JK_CSV=us_data.csv JK_TICKER=SPX [JK_ID=..] cargo test --release jk_dump -- --ignored`
    #[test]
    #[ignore]
    fn jk_dump() {
        let (Ok(out), Ok(csv), Ok(tk)) = (std::env::var("JK_DUMP"), std::env::var("JK_CSV"), std::env::var("JK_TICKER")) else { return };
        let iso = |d: &str| format!("{}-{}-{}", &d[6..10], &d[3..5], &d[0..2]);
        let rows: Vec<Row> = std::fs::read_to_string(csv).unwrap().lines().skip(1).filter_map(|l| { let mut q = false; let l: String = l.chars().filter_map(|c| if c == '"' { q = !q; None } else if q && c == ',' { None } else { Some(c) }).collect(); // quoted thousands "22,331.40"
            let f: Vec<&str> = l.split(',').collect();
            if f[0] != tk { return None; }
            Some(Row { ticker: tk.clone(), date: iso(f[1]), open: f[2].parse().ok()?, high: f[3].parse().ok()?, low: f[4].parse().ok()?, close: f[5].parse().ok()?, volume: None }) }).collect();
        let x = Ctx::new(&rows, &rows, &[]);
        let mut pp = Params::new();
        if let Ok(v) = std::env::var("JK_REV") { pp.insert("rev".into(), v.parse().unwrap()); }
        let t = run("jkofc_ma_crossover", &x, 0, rows.len() - 1, &pp).unwrap();
        let mut o = String::new();
        for z in &t { o += &format!("{}\t{}\t{}\t{}\t{}\t{}\n", z.dir, rows[z.legs[0].i].date, z.legs[0].px, rows[z.exit_i].date, z.exit_px, z.open); }
        std::fs::write(out, o).unwrap();
    }
}

// Instrument admission: Jay Kaeppel, Robin Lofton & Brad Zigler (jkofc, lane A) 2026-10-26 (lane B) block, moved from instrument_policy.rs on 31 Oct 2026 (lane ST).
// Admission only for the module REVIEWED lists (authors/lanea/singles/decisions.json); kept in one block to merge.
// Runs on actual indices, ETFs and stocks, US and India (index_etf_or_stock, nifty50 basket, declared application).
pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book=="jkofc" && crate::books::jkofc::reviewed_lanea(def.id),
    label: "Kaeppel's 4-day average reversal was written for futures (February gold); Runs on indices, ETFs and single stocks (US and India) are a declared application, one unit per trade.",
    basis: "Jay Kaeppel, Robin Lofton & Brad Zigler / Optionetics Futures & Commodity Home Study Course 2 (2007)",
};
