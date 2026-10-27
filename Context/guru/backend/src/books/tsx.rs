//! "TS Express, Journal for Informed Users of TradeStation" (Inside Edge Systems, ed. William Brower), 1994-1997.
//! Book id `tsx`. Source: research drive "Trading Knowledge II/@NewsLetters-Mags/TS Express, Journal for Informed Users
//! of Tradestation/" (12 files; 6 are scanned PDFs that open, 6 are not valid PDFs: TS Jul-Agost 95, TS July-Ag 94,
//! TS May-Jun 95, TS May-Jun 97, TS Nov-Dec 95, TS Sept-Oct 95). Copies of the used issues in Book-to-rust/Books/TS Express/.
//! The journal is mostly programming tips; one complete system (DMA Test) is Live.
use crate::connors::{sma, Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "tsx";
/// Admitted ids (authors/lanea/newsletters/decisions.json).
pub const REVIEWED_LANEA: &[&str] = &["tsx_dma"];
pub fn reviewed_lanea(id: &str) -> bool { REVIEWED_LANEA.contains(&id) }

pub fn catalogue() -> Vec<StrategyDef> {
    use Status::*;
    let d = |id, name, group, dir, status, rules, notes, claim| StrategyDef { id, name, group, session: 0, dir, status, needs_volume: false, needs_vix: false,
        needs_series: &[], book: BOOK, rules, notes, claim };
    vec![
        d("tsx_dma", "Displaced moving average (20, displaced 20) stop-and-reverse", "Sep/Oct 1997 · The displaced moving average (W. Brower)", "Long/Short", Live,
          &["F|\"Input : DMALen(20), Displace(20); ... DMA = Average( C, DMALen )[Displace];\" (TS Express Sep/Oct 1997 p.5, sidebar 'System: DMA Test')",
            "E|\"If C>DMA then Buy(\"DMA LE\") market; If C< DMA then Sell(\"DMA SE\") market;\"",
            "X|Stop-and-reverse on the opposite signal (no other exit in the code)"],
          "'market' orders fill at the next bar's open (TradeStation). Book: 17 futures, 10 years of back-adjusted continuous contracts, $150 per trade; displacement 0/10/20/30 tested, 20 recommended.", None),
        d("tsx_other_side", "\"The Other Side\" swing-point cluster breakout (W. Brower)", "Jan/Feb 1997 · Break on through to the other side", "Long/Short",
          NotTestable("the exits are a dollar money-management stop 'from $1,000 to $2,000' and a dollar profit target 'from $9,000 to $35,000' per futures contract, given only as optimisation ranges with no single value"),
          &["E|\"Long entries are allowed if the closing price crosses over any Swinghigh level by 1.4 times the average Truerange of 20 bars\" (Jan/Feb 1997 p.3; code p.4)",
            "X|\"For an exit strategy, use money management and profit target stops\"; code header \"Stops: MM from $1000 to $2,000, Profit Target = from $9,000 to $35,000\""],
          "", None),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    Some(match id { "tsx_dma" => ("tsx_dma", "Displaced MA (TS Express)", ""), "tsx_other_side" => ("tsx_other_side", "The Other Side (TS Express)", ""), _ => return None })
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    match id {
        "tsx_dma" => Some(vec![p("len", "DMALen", 20., 2., 100., 1.), p("disp", "Displace (bars)", 20., 0., 50., 1.)]),
        _ => None,
    }
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    let schema = params(id)?;
    if from > to || to >= x.r.len() { return None; }
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).map_or(0., |d| d.def));
    let (len, disp) = ((g("len").round() as usize).max(1), g("disp").max(0.).round() as usize);
    let ma = sma(&x.c, len);
    let mut out = vec![];
    let mut cur: Option<Trade> = None;
    for t in from.max(disp)..to {
        let dma = ma[t - disp];
        if !dma.is_finite() { continue; }
        let d = if x.c[t] > dma { 1. } else if x.c[t] < dma { -1. } else { continue };
        if cur.as_ref().is_some_and(|c| c.dir == d) { continue; }
        let (i, px) = (t + 1, x.o[t + 1]);
        if let Some(mut tr) = cur.take() { tr.exit_i = i; tr.exit_px = px; tr.signal_i = t; tr.reason = "opposite DMA signal · next open".into(); out.push(tr); }
        cur = Some(Trade { dir: d, legs: vec![Leg { i, px, w: 1. }], exit_i: i, exit_px: px, reason: String::new(), entry_kind: "open", signal_i: t, open: false, exits: vec![] });
    }
    if let Some(mut t) = cur { t.exit_i = to; t.exit_px = x.c[to]; t.signal_i = to; t.reason = "open".into(); t.open = true; out.push(t); }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;
    #[test]
    fn dma_uses_the_average_from_disp_bars_ago() {
        let cl: Vec<f64> = (0..60).map(|i| if i < 45 { 100. } else { 110. }).collect();
        let r: Vec<Row> = cl.iter().enumerate().map(|(i, &c)| Row { ticker: "T".into(), date: format!("2000-01-{:02}", 1 + i % 28), open: c + 1., high: c + 2., low: c - 2., close: c, volume: None }).collect();
        let x = Ctx::new(&r, &r, &[]);
        let ts = run("tsx_dma", &x, 0, 59, &Params::new()).unwrap();
        // flat series: no signal until close 110 > DMA (100) on bar 45 -> buy at bar 46 open
        assert_eq!(ts.len(), 1); assert_eq!(ts[0].legs[0].i, 46); assert_eq!(ts[0].legs[0].px, 111.); assert!(ts[0].open);
    }
}
