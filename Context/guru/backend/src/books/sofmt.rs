//! Jack D. Schwager, "Schwager on Futures: Managed Trading - Myths & Truths" (Wiley 1996), Ch.5 appendix
//! "The BARRA/MLM Index" (printed p.82-84; PDF 55-56). Source copy: Books/Jack D. Schwager/Schwager on Futures -
//! Managed Trading by Jack D. Schwager.pdf. Reading: claude-continuation-20261020/authors/schwager/agents/sofmt/.
use crate::connors::{Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "sofmt";

/// Rules admitted after the 2026-10-23 full reading (authors/schwager/admission/SOURCE-CONTRACT.md) and independent
/// chronological Python ledgers (0 mismatches, US and India). Indices, ETFs and stocks are a declared application.
pub const REVIEWED: &[&str] = &["sofmt_mlm"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const NOTE: &str = "The book's index applies this rule to 25 equal-weighted US futures markets and adds T-bill interest; here it runs on one index, ETF or stock at a time (declared application), without interest, one unit, prices in points. The market index is a chain of percentage changes, so comparing it with its own 12-month average is the same as comparing the price with the average of its 12 sampled prices. Moving-average type and the equal case are not stated: a simple average of the last 12 monthly values including the current one is used and an equal value keeps the position [Book silent: declared reading]. Next-to-last day = the trading day before the month's last trading day in the price file. Fills at that close (the position for the following month is set \"as of the close of the current month\", p.83 note 11); p.fill = 1 fills at the next open [Not from the book]. Each run starts flat at the selected start date and takes the indicated side at the first monthly reading in the window; earlier months only warm the average.";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef {
        id: "sofmt_mlm", name: "BARRA/MLM trend index rule (12-month average, monthly)", group: "Ch 5 appendix · The BARRA/MLM Index",
        session: 0, dir: "Long/Short", status: Status::Live, needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK,
        rules: &[
            "F|\"This index is calculated each month using the closing price on the next-to-last day of the month.\" p.83",
            "E|\"If the current index value is greater than the 12-month moving average, a long position is indicated for the following month; if it is less than the moving average, a short position is indicated.\" p.83",
            "X|\"This system is always in the market, with the position being reversed whenever the rule just described indicates a position opposite to the one currently held.\" p.83",
            "F|\"monthly price changes are defined as changes from the close on the next-to-last day of one month to the close on the next-to-last day of the next month ... it allows the position for the following month to be determined as of the close of the current month\" p.83 note 11",
        ],
        notes: NOTE, claim: None,
    }]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    (id == "sofmt_mlm").then_some(("sofmt", "Schwager managed-futures index", "BARRA/MLM 12-month average"))
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    (id == "sofmt_mlm").then(|| vec![
        ParamDef { key: "months", label: "Moving-average months", def: 12.0, min: 3.0, max: 24.0, step: 1.0 },
        ParamDef { key: "fill", label: "Fill: 0 = signal close (book), 1 = next open [Not from the book]", def: 0.0, min: 0.0, max: 1.0, step: 1.0 },
    ])
}

fn month(d: &str) -> &str { d.get(..7).unwrap_or(d) }

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    let schema = params(id)?;
    if from > to || to >= x.r.len() { return None; }
    for (k, v) in p {
        let d = schema.iter().find(|d| d.key == k)?;
        if !v.is_finite() || *v < d.min || *v > d.max || v.fract() != 0.0 { return None; }
    }
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).unwrap().def);
    let (m, next_open) = (g("months") as usize, g("fill") == 1.0);
    let n = x.r.len();
    let mut samples: Vec<f64> = vec![];
    let mut out = vec![];
    let mut cur: Option<Trade> = None;
    for i in 0..=to {
        // next-to-last trading day of its month: the next bar is in the same month, the one after is not
        if i + 2 >= n || month(&x.r[i + 1].date) != month(&x.r[i].date) || month(&x.r[i + 2].date) == month(&x.r[i].date) { continue; }
        samples.push(x.c[i]);
        if i < from || samples.len() < m { continue; }
        let ma = samples[samples.len() - m..].iter().sum::<f64>() / m as f64;
        let s = if x.c[i] > ma { 1.0 } else if x.c[i] < ma { -1.0 } else { 0.0 };
        if s == 0.0 || cur.as_ref().is_some_and(|t| t.dir == s) { continue; }
        let (j, px) = if next_open { if i == to { break; } (i + 1, x.o[i + 1]) } else { (i, x.c[i]) };
        if let Some(mut t) = cur.take() { t.exit_i = j; t.exit_px = px; t.signal_i = i; t.reason = "reverse".into(); out.push(t); }
        cur = Some(Trade { dir: s, legs: vec![Leg { i: j, px, w: 1.0 }], exit_i: j, exit_px: px, reason: String::new(),
            entry_kind: if next_open { "open" } else { "close" }, signal_i: i, open: false, exits: vec![] });
    }
    if let Some(mut t) = cur { t.exit_i = to; t.exit_px = x.c[to]; t.signal_i = to; t.reason = "open".into(); t.open = true; out.push(t); }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;
    #[test]
    fn reverses_on_the_next_to_last_day_against_the_12_month_average() {
        // 3 bars per month: the middle bar is the next-to-last day. Rising 12 months, then a drop.
        let mut r = vec![];
        for mo in 1..=16u32 {
            let px = if mo <= 13 { 100.0 + mo as f64 } else { 80.0 };
            for d in 1..=3u32 { r.push(Row { ticker: "T".into(), date: format!("{}-{:02}-{:02}", 2000 + (mo - 1) / 12, (mo - 1) % 12 + 1, d), open: px, high: px, low: px, close: px, volume: None }); }
        }
        let x = Ctx::new(&r, &r, &[]);
        let t = run("sofmt_mlm", &x, 0, r.len() - 1, &Params::new()).unwrap();
        // month 12 (index 34): 112 > avg(101..112) -> long; month 14 (index 40): 80 < avg -> short
        assert_eq!((t[0].dir, t[0].legs[0].i, t[0].exit_i), (1.0, 34, 40));
        assert_eq!((t[1].dir, t[1].open), (-1.0, true));
    }
}
