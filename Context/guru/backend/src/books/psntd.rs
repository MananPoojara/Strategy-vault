//! Prashant Shah, "Noiseless Trader" (Definedge slide deck; research share Prasad/ALL BOOKS/Noiseless trader_Definedge (1).pdf,
//! PDF 220-223; text layer and OCR authors/day-1030/R1w/shah.layer.txt, shah.ocr.txt). Book id `psntd`. Lane K34 (4 Nov 2026),
//! D1 K-01 / decision 186: the bullish point & figure system of slide 220, coded WITHOUT its trend filter ("RSI (7,5) and ADX (14)
//! are positive" is a Definedge per-column indicator the deck never defines, decision #28): a labelled subset.
//! Independent check authors/titles-1030/K34w/k34_ref.py.
use crate::connors::{claim, Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "psntd";
pub const REVIEWED: &[&str] = &["psntd_pf_dtb"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const NOTE: &str = "[Declared readings] SUBSET: the deck's trend filter 'RSI (7,5) and ADX (14) are positive' is a Definedge indicator computed on point & figure columns and is not defined anywhere in the deck (slides 160-179 checked), so it is NOT applied; the author's printed results include it. Chart: daily closes, percentage boxes of 0.25% on one logarithmic ladder (level k = 1.0025^k), 3-box reversal, the same construction as the vault's Definedge Triple Top Buy test (bkttb): a rising close fills X boxes up to the highest level at or below it, a falling close fills O boxes down to the lowest level at or above it; a column reverses when the close is 3 or more boxes beyond its extreme. Double Top Buy = the current X column rises above the top of the previous X column (once per column); Double Bottom Sell = the current O column falls below the bottom of the previous O column. Trades at the close of the signal day. 'Maximum 7 box trade' = out at the close when the close is 7 or more boxes below the box of the entry (bearish: above). 'Re-entry: Yes, if 7 box trade' = after a 7-box stop the next Double Top Buy enters again (as after any exit). The chart starts from the first close the vault holds. Bearish system ('Vice versa', p.side = -1): short on a Double Bottom Sell, out on a Double Top Buy or a 7-box rise. The 'or High Pole' exit of slide 222 is not coded. Book: Nifty 500 stocks 1996-2015 (with the filter): bullish average return 6.94% a trade, 55.36% successful; bearish 16.46%, 51.23% (slide 221). NSE stocks are the source market; indices, ETFs and US stocks are a declared application.";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef { id: "psntd_pf_dtb", name: "Point & figure Double Top Buy, out on Double Bottom Sell or a 7-box loss (0.25% x 3, closes; trend filter not applied)", group: "Noiseless Trader (Definedge)",
        session: 0, dir: "Long", status: Status::Live, needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK, rules: &[
            "F|\"Box value: Daily 0.25% x 3 of an instrument\" (slide 220)",
            "E|\"Entry pattern: Double top buy\" (slide 220)",
            "X|\"Exit pattern: Double bottom sell\"; \"Risk control: Maximum 7 box trade\" (slide 220)",
            "F|\"Trend filter: RSI (7,5) and ADX (14) are positive\" (slide 220): NOT applied, undefined in the deck",
            "A|\"Re-entry: Yes, if 7 box trade\"; \"Vice versa for bearish system\" (slide 220)"],
        notes: NOTE, claim: claim("49.40 a stock (bullish)", "55.36%", "+6.94% a trade", "-", "Nifty 500 stocks 1996-2015, with the RSI/ADX column filter") }]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    reviewed(id).then_some(("psntd_pf_dtb", "Double Top Buy P&F (Prashant Shah)", "0.25% x 3, 7-box risk"))
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    reviewed(id).then(|| vec![p("box", "Box size, % (0.25)", 0.25, 0.05, 5., 0.05), p("rev", "Reversal boxes (3)", 3., 1., 5., 1.),
        p("risk", "Maximum boxes against the entry (7; 0 = off)", 7., 0., 30., 1.), p("side", "1 = bullish system (slide 220), -1 = the bearish mirror", 1., -1., 1., 2.)])
}

/// Close-only P&F per bar: +1 Double Top Buy, -1 Double Bottom Sell, 0 none; plus the box of each close (up: floor, dn: ceil).
pub fn pf_dtb(c: &[f64], pct: f64, rev: i64) -> (Vec<i8>, Vec<i64>, Vec<i64>) {
    let s = (1. + pct / 100.).ln();
    let up = |p: f64| (p.ln() / s + 1e-9).floor() as i64;
    let dn = |p: f64| (p.ln() / s - 1e-9).ceil() as i64;
    let (bu, bd): (Vec<i64>, Vec<i64>) = (c.iter().map(|&p| up(p)).collect(), c.iter().map(|&p| dn(p)).collect());
    let mut sig = vec![0i8; c.len()];
    if c.is_empty() { return (sig, bu, bd); }
    let (mut col, mut top, mut bot) = (0i8, bu[0], bu[0]);
    let (mut px, mut po): (Option<i64>, Option<i64>) = (None, None); // previous X top, previous O bottom
    let mut fired = false;
    for i in 1..c.len() {
        let (u, d) = (bu[i], bd[i]);
        match col {
            0 => if u > top { col = 1; top = u; } else if d < bot { col = -1; bot = d; },
            1 => if u > top { top = u; if !fired && px.is_some_and(|x| top > x) { fired = true; sig[i] = 1; } }
                 else if d <= top - rev { px = Some(top); col = -1; bot = d; fired = false; if po.is_some_and(|b| bot < b) { fired = true; sig[i] = -1; } },
            _ => if d < bot { bot = d; if !fired && po.is_some_and(|b| bot < b) { fired = true; sig[i] = -1; } }
                 else if u >= bot + rev { po = Some(bot); col = 1; top = u; fired = false; if px.is_some_and(|x| top > x) { fired = true; sig[i] = 1; } },
        }
    }
    (sig, bu, bd)
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.r.len() { return None; }
    let schema = params(id)?;
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).unwrap().def);
    let (sig, bu, bd) = pf_dtb(&x.c[..=to], g("box"), g("rev").round().max(1.) as i64);
    let (risk, side) = (g("risk").round().max(0.) as i64, if g("side") < 0. { -1i8 } else { 1 });
    let c = &x.c;
    let mut out = vec![];
    let mut cur: Option<(usize, i64)> = None; // entry bar, entry box
    let close = |out: &mut Vec<Trade>, ei: usize, i: usize, why: &str, open: bool| out.push(Trade { dir: side as f64, legs: vec![Leg { i: ei, px: c[ei], w: 1. }], exit_i: i, exit_px: c[i], reason: why.into(), entry_kind: "close", signal_i: i, open, exits: vec![] });
    for i in from..=to {
        if let Some((ei, eb)) = cur {
            let stopped = risk > 0 && if side > 0 { bd[i] <= eb - risk } else { bu[i] >= eb + risk };
            if sig[i] == -side { close(&mut out, ei, i, if side > 0 { "Double Bottom Sell" } else { "Double Top Buy" }, false); cur = None; }
            else if stopped { close(&mut out, ei, i, "7-box risk control", false); cur = None; }
        }
        if cur.is_none() && sig[i] == side && i < to { cur = Some((i, if side > 0 { bu[i] } else { bd[i] })); }
    }
    if let Some((ei, _)) = cur { close(&mut out, ei, to, "open", true); }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;
    #[test]
    fn double_top_buy_double_bottom_sell_and_seven_box_stop() {
        let b = 1.0025f64;
        let base = (100f64.ln() / b.ln()).floor() as i32;
        // boxes: start 0; X to 10, O to 6, X to 11 (DTB above 10), O to 4 (DBS below 6); then X to 14 (DTB above 11), O to 7 (7 boxes under 14)
        let ks = [0, 10, 6, 11, 4, 14, 7];
        let c: Vec<f64> = ks.iter().map(|&k| b.powi(base + k)).collect();
        assert_eq!(pf_dtb(&c, 0.25, 3).0, vec![0, 0, 0, 1, -1, 1, 0]);
        let r: Vec<Row> = c.iter().enumerate().map(|(i, &v)| Row { ticker: "T".into(), date: format!("{:05}", i), open: v, high: v, low: v, close: v, volume: None }).collect();
        let x = Ctx::new(&r, &r, &[]);
        let t = run("psntd_pf_dtb", &x, 0, 6, &Params::new()).unwrap();
        assert_eq!(t.iter().map(|z| (z.legs[0].i, z.exit_i, z.reason.as_str())).collect::<Vec<_>>(), vec![(3, 4, "Double Bottom Sell"), (5, 6, "7-box risk control")]);
        assert!(run("psntd_x", &x, 0, 5, &Params::new()).is_none());
    }
}

// Instrument admission: Prashant Shah, Noiseless Trader 2026-11-04 (titles lane K34, D1 K-01).
pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book == BOOK && reviewed(def.id),
    label: "Prashant Shah's slide shows this system on Nifty 500 stocks with a Definedge column trend filter that is not applied here; runs on indices, ETFs and stocks, US and India, are a declared application of the rest of the rule.",
    basis: "Prashant Shah, Noiseless Trader (Definedge deck), PDF 220-223; independent check authors/titles-1030/K34w/k34_ref.py",
};
