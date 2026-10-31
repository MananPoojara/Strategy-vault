//! B. Krishnakumar, "Backtesting for Dummies" (DECNOCH 2019, Definedge conference deck; research share Prasad/ALL BOOKS/DECNOCH
//! 2019 PPT.pdf, PDF p.220-238: rule p.225-226, TradePoint settings and group table p.229, FINPIPE trade list p.232-233; text and
//! OCR authors/batch-c/C6w/decnoch19-ttb.txt). Book id `bkttb`. Lane K10 (titles loop, 1 Nov 2026), to-code row 166;
//! independent check authors/titles-1030/K10w/k10_ref.py.
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};
use super::lc3::Pos;

/// Admitted ids (authors/titles-1030/K10w/k10_ref.py (header)).
pub const REVIEWED: &[&str] = &["bkttb_ttb_dbs"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const NOTE: &str = "[Declared readings] Point & figure chart of daily closes with percentage boxes of 0.25% and a 3-box reversal (TradePoint settings p.229: Frequency DAILY, Box Size 0.25, Rev. Box 3, Price Close, Type Percent). Box levels form one logarithmic ladder, level k = 1.0025^k; a rising close fills X boxes up to the highest level at or below it, a falling close fills O boxes down to the lowest level at or above it; an X column turns into an O column when the close is 3 or more boxes below its top (the O column starts one box below the top), and back the same way. The chart starts from the first close in the data; the first column starts on the first move of one box or more. Triple Top Buy = the classic triple top: the two previous X columns topped at the same box and the current X column rises above it (Definedge's exact definition is not printed; this reading is the default because it comes close to the author's group table, see below; p.ttb = 0 is the looser reading 'above the tops of both previous X columns', which gives about 25 times as many trades). Double Bottom Sell = an O column falls below the bottom of the previous O column. Long only: buy at the close of the day a Triple Top Buy triggers (when flat), sell at the close of the day of the next Double Bottom Sell. The author's FINPIPE (Finolex Industries) is not in the vault's data; his group table prints GODREJCP 5 trades, 60% successful, 27.64% total and ASTERDM 3 trades, none successful, -5.2% (1 Jan 1990 - 18 Sep 2019). On the vault's data to 18 Sep 2019: GODREJCP 6 closed trades, 2 winners, +10.78% in total, ASTERDM 4 closed, none winning, -11.95%, and both have a Triple Top Buy on 11 Sep 2019, which is why the scanner listed them (the looser reading: 134 and 12 trades). NSE stocks are the source market; indices, ETFs and US stocks are a declared application.";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef { id: "bkttb_ttb_dbs", name: "Point & figure Triple Top Buy, out on the next Double Bottom Sell (0.25% boxes, 3-box reversal, closes)", group: "Backtesting for Dummies (DECNOCH 2019)",
        session: 0, dir: "Long", status: Status::Live, needs_volume: false, needs_vix: false, needs_series: &[], book: "bkttb", rules: &[
            "F|TradePoint P&F Back Testing (p.229): \"Frequency: DAILY ... Box Size 0.25 Rev. Box 3 Price Close Type Percent\"",
            "E|\"Lets consider a simple Long-only scanner where buy is triggered on a triple top buy (TTB) signal.\" (p.225)",
            "X|\"Exit is a subsequent Double Bottom Sell.\" (p.225)"],
        notes: NOTE, claim: None }]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    (id == "bkttb_ttb_dbs").then_some(("bkttb_ttb_dbs", "Triple Top Buy P&F (Krishnakumar)", "0.25% x 3"))
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    reviewed(id).then(|| vec![p("box", "Box size, % (0.25)", 0.25, 0.05, 5., 0.05), p("rev", "Reversal boxes (3)", 3., 1., 5., 1.),
        p("ttb", "Triple top: 1 = the two previous X tops equal and exceeded (classic, default), 0 = above both previous X tops", 1., 0., 1., 1.)])
}

/// Close-only P&F signals per bar: +1 Triple Top Buy, -1 Double Bottom Sell, 0 none.
pub fn pf_signals(c: &[f64], pct: f64, rev: i64, equal: bool) -> Vec<i8> {
    let s = (1. + pct / 100.).ln();
    let up = |p: f64| (p.ln() / s + 1e-9).floor() as i64;
    let dn = |p: f64| (p.ln() / s - 1e-9).ceil() as i64;
    let mut sig = vec![0i8; c.len()];
    if c.is_empty() { return sig; }
    let (mut col, mut top, mut bot) = (0i8, up(c[0]), up(c[0]));
    let (mut xs, mut os): (Vec<i64>, Vec<i64>) = (vec![], vec![]);
    let mut fired = false;
    let ttb = |top: i64, xs: &[i64]| xs.len() >= 2 && { let (a, b) = (xs[xs.len() - 1], xs[xs.len() - 2]); top > a.max(b) && (!equal || a == b) };
    for i in 1..c.len() {
        let (u, d) = (up(c[i]), dn(c[i]));
        match col {
            0 => if u > top { col = 1; top = u; } else if d < bot { col = -1; bot = d; },
            1 => if u > top { top = u; if !fired && ttb(top, &xs) { fired = true; sig[i] = 1; } }
                 else if d <= top - rev { xs.push(top); col = -1; top -= 1; bot = d; fired = false; if os.last().is_some_and(|&b| bot < b) { fired = true; sig[i] = -1; } },
            _ => if d < bot { bot = d; if !fired && os.last().is_some_and(|&b| bot < b) { fired = true; sig[i] = -1; } }
                 else if u >= bot + rev { os.push(bot); col = 1; bot += 1; top = u; fired = false; if ttb(top, &xs) { fired = true; sig[i] = 1; } },
        }
    }
    sig
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.r.len() { return None; }
    let schema = params(id)?;
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).unwrap().def);
    let sig = pf_signals(&x.c[..=to], g("box"), g("rev").round().max(1.) as i64, g("ttb") > 0.5);
    let mut pos = Pos::new();
    for i in from..=to {
        if sig[i] > 0 && pos.side() == 0. { pos.enter(1., i, x.c[i], "close"); }
        else if sig[i] < 0 && pos.side() > 0. { pos.exit(i, x.c[i], "Double Bottom Sell"); }
    }
    Some(pos.done(to, x.c[to]))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;
    #[test]
    fn triple_top_and_double_bottom() {
        let b = 1.0025f64;
        // boxes: start 0; X to 10, O to 6, X to 10, O to 5 (DBS), X to 11 (TTB: above 10 and 10), O to 3 (DBS)
        let ks = [0, 10, 6, 10, 5, 11, 3];
        let base = (100f64.ln() / b.ln()).floor() as i32;
        let c: Vec<f64> = ks.iter().map(|&k| b.powi(base + k)).collect();
        let s = pf_signals(&c, 0.25, 3, false);
        assert_eq!(s, vec![0, 0, 0, 0, -1, 1, -1]);
        assert_eq!(pf_signals(&c, 0.25, 3, true)[5], 1);
        assert_eq!(pf_signals(&[c[0], c[1], c[2], c[3] / b, c[4], c[5]], 0.25, 3, true)[5], 0); // unequal previous tops: no classic TTB
        let r: Vec<Row> = c.iter().enumerate().map(|(i, &v)| Row { ticker: "T".into(), date: format!("{:05}", i), open: v, high: v, low: v, close: v, volume: None }).collect();
        let x = Ctx::new(&r, &r, &[]);
        let t = run("bkttb_ttb_dbs", &x, 0, 6, &Params::new()).unwrap();
        assert_eq!(t.iter().map(|z| (z.legs[0].i, z.exit_i, z.open)).collect::<Vec<_>>(), vec![(5, 6, false)]);
        assert!(run("bkttb_x", &x, 0, 5, &Params::new()).is_none());
    }
}

// Instrument admission: B. Krishnakumar, Backtesting for Dummies (DECNOCH 2019) 2026-11-01 (titles lane K10).
pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book == "bkttb" && reviewed(def.id),
    label: "B. Krishnakumar ran the Triple Top Buy scan on NSE stocks in Definedge TradePoint. Runs on indices, ETFs and stocks, US and India, are a declared application.",
    basis: "B. Krishnakumar, Backtesting for Dummies, DECNOCH 2019 deck PDF p.225-233 (DECNOCH 2019 PPT.pdf); independent check authors/titles-1030/K10w/k10_ref.py",
};
