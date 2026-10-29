//! William McWilliams, "Market Turn Strategy" (marketturnstrategy.com, 2006, 26 pp.) + its "MTSspreadsheet.xls".
//! Book id `wmmts`. Source copies: Books/William McWilliams/MarketTurnStrategy.pdf and MTSspreadsheet.xls (from
//! R:/Trading Knowledge I/William McWilliams - Market Turn Strategy (marketturnstrategy.com)/).
//! The calculation exists only in the spreadsheet (Sheet1, newest day in row 2): G10 = SUM(G2:G9)*4 with G2..G8 =
//! Close-Low (G9 is empty, so 7 days), J17 = SUM(J2:J16)*2 (J16 empty: 14 days), M31 = SUM(M2:M30) (M30 empty: 28
//! days), the same for High-Low in H/K/N; G19 = G10+J17+M31, H19 = H10+K17+N31, result G21 = G19/H19. It reproduces the
//! manual's printed sums (8.32 / 20.92, 24.49 / 59.89, 0.41, p.12) and the DIA example (p.21-22) to the cent.
use crate::connors::{claim, Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "wmmts";
/// Admitted ids (authors/lanea/newsletters/decisions.json).
pub const REVIEWED_LANEA: &[&str] = &["wmmts_market_turn"];
pub fn reviewed_lanea(id: &str) -> bool { REVIEWED_LANEA.contains(&id) }

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef {
        id: "wmmts_market_turn", name: "Market Turn Strategy (7/14/28-day close-location calculation, .30/.60)", group: "Market Turn Strategy", session: 0, dir: "Long/Short",
        status: Status::Live, needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK,
        rules: &[
            "F|Calculation (MTSspreadsheet.xls, Sheet1 G21): [4 x sum(Close-Low, 7 days) + 2 x sum(Close-Low, 14 days) + sum(Close-Low, 28 days)] / [4 x sum(High-Low, 7) + 2 x sum(High-Low, 14) + sum(High-Low, 28)], daily High/Low/Close of regular trading hours (p.8-11)",
            "E|\"The calculations are done everyday after the market closes. If the number produced is .60 or greater then we will look to get short the next trading day. If it's .30 or lower we will look to get long the next trading day.\" (p.13)",
            "E|Entry price: \"we simply entered into our position all at once at the open\" (Example #3 DIA, p.22; Example #2 SBUX \"entered once at the open\", p.19). The manual leaves averaging in to the trader (p.13); the all-at-once open entry of its examples is the default.",
            "X|\"If we enter into a short position ... we will look to exit when the calculation .35 or lower. If we enter into a long position ... we will look to exit when the calculation reads .55 or higher.\" (p.13); \"wait until the day after the calculation reads .35 or lower ... get out ... the next trading day\" (p.16); \"exit the next trading day, November 11/05 at the open\" (p.22)",
            "X|Stop loss: short \"we would get out with a loss when it reaches .70 or higher\"; long \"we will be stopped out when the calculation reads .20 or lower\" (p.14) - at the next day's open, as the other exits.",
            "F|\"Re-entry Short = if we have been stopped out of a short position ... we can re-enter a short position only when the calculation has first dropped back below .60 ... Re-entry Long = ... only when the calculation has first risen back above .30\" (p.14)",
            "F|[Book silent] One position at a time; an exit and an opposite entry signalled at the same close are both done at the next open. The calculation is compared unrounded (round = 0); round = 1 compares the value rounded to 2 decimals as a spreadsheet cell may display it [non-default reading].",
        ],
        notes: "Book: US stocks/ETFs (GOOG short 1/11/06-1/23/06 +$64.07; SBUX long 12/27/05-2/9/06 +$4.46; DIA long 10/13/05 open 102.03 -> 11/11/05 open 106.56, +$4.53) and futures/forex with regular-hours bars. DIA is not in the vault: the DIA example is compared on the Dow Jones Industrial Average (DJI, = DIA x ~100). Run on the indexes (instrument swap). The manual calls its exits 'not written in stone' (p.13) and adds a discretionary early exit in Example #3; only the printed .55/.35 exits are coded.",
        claim: claim("3 examples", "3/3", "DIA +4.53 (+4.4%)", "DIA 10/13/05-11/11/05", "examples p.15-23 (no back-test)"),
    }]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    (id == "wmmts_market_turn").then_some(("wmmts", "Market Turn Strategy (McWilliams)", "spreadsheet calculation"))
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    (id == "wmmts_market_turn").then(|| vec![
        p("long_in", "Long when calculation <=", 0.30, 0.0, 0.5, 0.01), p("short_in", "Short when calculation >=", 0.60, 0.5, 1.0, 0.01),
        p("long_out", "Exit long when >=", 0.55, 0.3, 1.0, 0.01), p("short_out", "Exit short when <=", 0.35, 0.0, 0.7, 0.01),
        p("long_stop", "Long stop when <=", 0.20, 0.0, 0.4, 0.01), p("short_stop", "Short stop when >=", 0.70, 0.6, 1.0, 0.01),
        p("round", "Compare the value rounded to 2 decimals (1) or exact (0)", 0., 0., 1., 1.),
    ])
}

pub fn calc(h: &[f64], l: &[f64], c: &[f64]) -> Vec<f64> {
    let s = |v: &dyn Fn(usize) -> f64, t: usize, n: usize| (t + 1 - n..=t).map(v).sum::<f64>();
    (0..c.len()).map(|t| {
        if t < 27 { return f64::NAN }
        let cl = |k: usize| c[k] - l[k];
        let hl = |k: usize| h[k] - l[k];
        let num = 4. * s(&cl, t, 7) + 2. * s(&cl, t, 14) + s(&cl, t, 28);
        let den = 4. * s(&hl, t, 7) + 2. * s(&hl, t, 14) + s(&hl, t, 28);
        if den > 0. { num / den } else { f64::NAN }
    }).collect()
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    let defs = params(id)?;
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| defs.iter().find(|d| d.key == k).map(|d| d.def).unwrap_or(f64::NAN));
    let to = to.min(x.c.len().saturating_sub(1));
    let raw = calc(&x.h, &x.l, &x.c);
    let v: Vec<f64> = if g("round") >= 0.5 { raw.iter().map(|z| (z * 100.).round() / 100.).collect() } else { raw };
    let (li, si, lo, so, ls, ss) = (g("long_in"), g("short_in"), g("long_out"), g("short_out"), g("long_stop"), g("short_stop"));
    let o = &x.o;
    let mut out = Vec::new();
    let mut pos: Option<(f64, usize)> = None;
    let (mut long_ok, mut short_ok) = (true, true); // re-entry permission after a stop
    for t in from..to {
        let z = v[t];
        if !z.is_finite() { continue }
        if !long_ok && z > li { long_ok = true }
        if !short_ok && z < si { short_ok = true }
        let e = t + 1;
        if let Some((d, i)) = pos {
            let why = if d > 0. { if z <= ls { Some("stop: calculation <= .20") } else if z >= lo { Some("exit: calculation >= .55") } else { None } }
                else if z >= ss { Some("stop: calculation >= .70") } else if z <= so { Some("exit: calculation <= .35") } else { None };
            if let Some(w) = why {
                if w.starts_with("stop") { if d > 0. { long_ok = false } else { short_ok = false } }
                out.push(Trade { dir: d, legs: vec![Leg { i, px: o[i], w: 1.0 }], exit_i: e, exit_px: o[e], reason: w.into(), entry_kind: "open", signal_i: t, open: false, exits: vec![] });
                pos = None;
            }
        }
        if pos.is_none() {
            if z <= li && long_ok { pos = Some((1., e)) } else if z >= si && short_ok { pos = Some((-1., e)) }
        }
    }
    if let Some((d, i)) = pos {
        if i <= to { out.push(Trade { dir: d, legs: vec![Leg { i, px: o[i], w: 1.0 }], exit_i: to, exit_px: x.c[to], reason: "open".into(), entry_kind: "open", signal_i: to, open: true, exits: vec![] }); }
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn manual_example_p12() {
        // 8-Dec-05 back to 31-Oct-05 (p.11), oldest first
        let d: [(f64, f64, f64); 28] = [(28.49, 27.7, 28.28), (28.4, 28.07, 28.15), (29.04, 28.26, 28.83), (29.9, 28.84, 29.43), (30.58, 30.1, 30.36), (30.97, 30.23, 30.41), (30.5, 29.43, 29.5), (29.82, 29.44, 29.5), (29.98, 28.74, 29.97), (30.32, 29.97, 30.15), (30.5, 30.14, 30.31), (30.68, 29.96, 30.28), (30.58, 30.17, 30.42), (31.25, 30.51, 31.22), (31.62, 30.76, 30.98), (32., 31.21, 31.9), (31.98, 31.38, 31.55), (31.87, 31.47, 31.75), (32., 31.53, 31.7), (31.9, 31.1, 31.13), (31.53, 30.84, 30.84), (31.17, 30.45, 30.45), (31.36, 30.34, 31.14), (32., 31.55, 31.93), (32.06, 31.43, 31.96), (32.46, 31.75, 31.8), (31.99, 31.19, 31.34), (31.58, 30.68, 30.85)];
        let (h, l, c): (Vec<f64>, Vec<f64>, Vec<f64>) = (d.iter().map(|x| x.0).collect(), d.iter().map(|x| x.1).collect(), d.iter().map(|x| x.2).collect());
        let v = calc(&h, &l, &c);
        assert!((v[27] - 24.49 / 59.89).abs() < 1e-3);
        assert_eq!((v[27] * 100.).round(), 41.);
    }

    #[test]
    fn manual_example_3_dia() {
        // MTSspreadsheet.xls Sheet2 "DIA" rows (date, open, high, low, close); the manual's Example #3 (p.21-22):
        // signal 10/12/05, long at the 10/13/05 open 102.03, exit signal 11/10/05, out at the 11/11/05 open 106.56
        let d: &[(&str, f64, f64, f64, f64)] = &[("2005-08-19", 105.87, 106.2, 105.52, 105.73), ("2005-08-22", 105.87, 106.38, 105.17, 105.66), ("2005-08-23", 105.64, 105.73, 104.91, 105.28), ("2005-08-24", 104.99, 105.5, 104.16, 104.18), ("2005-08-25", 104.34, 104.59, 104.2, 104.46), ("2005-08-26", 104.36, 104.39, 103.79, 103.9), ("2005-08-29", 103.5, 104.81, 103.45, 104.64), ("2005-08-30", 104.33, 104.33, 103.45, 104.0), ("2005-08-31", 104.08, 104.98, 103.54, 104.89), ("2005-09-01", 104.78, 105.19, 104.22, 104.6), ("2005-09-02", 104.78, 104.93, 104.3, 104.6), ("2005-09-06", 104.98, 105.93, 104.92, 105.83), ("2005-09-07", 105.81, 106.37, 105.71, 106.23), ("2005-09-08", 105.98, 106.29, 105.67, 105.91), ("2005-09-09", 106.14, 106.93, 106.06, 106.76), ("2005-09-12", 106.7, 107.01, 106.63, 106.81), ("2005-09-13", 106.59, 106.81, 106.01, 106.11), ("2005-09-14", 106.27, 106.41, 105.43, 105.57), ("2005-09-15", 105.81, 105.9, 105.26, 105.66), ("2005-09-16", 105.95, 106.38, 105.65, 106.23), ("2005-09-19", 106.12, 106.15, 105.13, 105.54), ("2005-09-20", 105.68, 105.97, 104.57, 104.72), ("2005-09-21", 104.51, 104.51, 103.62, 103.72), ("2005-09-22", 103.74, 104.4, 103.44, 104.04), ("2005-09-23", 103.92, 104.5, 103.67, 104.1), ("2005-09-26", 104.65, 105.03, 104.02, 104.44), ("2005-09-27", 104.37, 104.9, 104.07, 104.53), ("2005-09-28", 104.8, 105.04, 104.16, 104.7), ("2005-09-29", 104.6, 105.55, 104.1, 105.4), ("2005-09-30", 105.36, 105.79, 105.2, 105.75), ("2005-10-03", 105.74, 106.01, 105.16, 105.28), ("2005-10-04", 105.46, 105.64, 104.17, 104.17), ("2005-10-05", 104.28, 104.35, 103.16, 103.22), ("2005-10-06", 103.34, 103.71, 102.19, 102.9), ("2005-10-07", 103.35, 103.51, 102.82, 102.97), ("2005-10-10", 103.18, 103.24, 102.29, 102.32), ("2005-10-11", 102.69, 103.18, 102.35, 102.53), ("2005-10-12", 102.54, 103.12, 101.88, 102.03), ("2005-10-13", 102.03, 102.44, 101.58, 101.96), ("2005-10-14", 102.49, 102.97, 102.0, 102.83), ("2005-10-17", 103.07, 103.64, 102.94, 103.5), ("2005-10-18", 103.56, 103.76, 102.86, 102.87), ("2005-10-19", 102.61, 104.35, 102.36, 104.31), ("2005-10-20", 104.06, 104.38, 102.49, 102.78), ("2005-10-21", 102.97, 103.05, 101.87, 102.14), ("2005-10-24", 102.54, 103.84, 102.44, 103.75), ("2005-10-25", 103.58, 104.06, 103.07, 103.73), ("2005-10-26", 103.45, 104.26, 103.31, 103.53), ("2005-10-27", 103.32, 103.42, 102.2, 102.34), ("2005-10-28", 102.65, 103.96, 102.46, 103.84), ("2005-10-31", 104.24, 104.79, 103.89, 103.91), ("2005-11-01", 104.25, 104.48, 103.97, 104.14), ("2005-11-02", 103.86, 104.89, 103.76, 104.7), ("2005-11-03", 105.15, 105.59, 104.88, 105.24), ("2005-11-04", 105.37, 105.49, 104.74, 105.19), ("2005-11-07", 105.49, 105.87, 105.17, 105.61), ("2005-11-08", 105.45, 105.74, 105.3, 105.6), ("2005-11-09", 105.42, 106.11, 105.27, 105.6), ("2005-11-10", 105.59, 106.68, 105.33, 106.58), ("2005-11-11", 106.56, 107.11, 106.5, 107.02), ("2005-11-14", 106.96, 107.24, 106.89, 107.2), ("2005-11-15", 107.18, 107.57, 106.77, 107.12), ("2005-11-16", 107.16, 107.35, 106.73, 107.04), ("2005-11-17", 107.14, 107.48, 106.86, 107.38)];
        let r: Vec<crate::Row> = d.iter().map(|&(dt, o, h, l, c)| crate::Row { ticker: "DIA".into(), date: dt.into(), open: o, high: h, low: l, close: c, volume: None }).collect();
        let x = Ctx::new(&r, &r, &[]);
        let t = run("wmmts_market_turn", &x, 0, r.len() - 1, &Params::new()).unwrap();
        let q = &t[0];
        assert_eq!((r[q.legs[0].i].date.as_str(), q.legs[0].px, r[q.exit_i].date.as_str(), q.exit_px, q.dir), ("2005-10-13", 102.03, "2005-11-11", 106.56, 1.0));
    }
}
