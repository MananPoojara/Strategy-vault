//! Donald Dorsey, "Refining the Relative Volatility Index", Technical Analysis of Stocks & Commodities V13 (1995)
//! p.388-391. Research drive: .../Articulos Varios- Club Directores/AAApublicaciones/Encargados/Refining/refinin.zip!REFININ.PDF;
//! copy in Books/Donald Dorsey/. Book id "ddrvi". Notes and verification: library/ddrvi/README.md.
//! The new RVI and inertia follow the article's TradeStation code (Fig.4) line by line; the Fig.5 system
//! "buys when inertia is above 50 and sells when inertia is below 50".
use crate::connors::{claim, Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};

/// Admitted ids (authors/dorsey/admission/SOURCE-CONTRACT.md).
pub const REVIEWED: &[&str] = &["ddrvi_inertia50"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef {
        id: "ddrvi_inertia50", name: "Inertia above/below 50 (new RVI, 20-bar average)", group: "Refining the RVI (S&C V13, 1995)",
        session: 0, dir: "Long", status: Status::Live, needs_volume: false, needs_vix: false, needs_series: &[], book: "ddrvi",
        rules: &[
            "F|\"RVIH is calculated the same as RVI, except that high is substituted for the close ... RVIL ... low is substituted for the close\"; \"New RVI = (RVIH + RVIL)/2\" (p.3)",
            "F|TS code (Fig.4, p.5): VALUE1=@STDDEV(H,10); at CURRENTBAR=13 VALUE10/VALUE11 = sums of VALUE1 over the last 14 bars with H>H[1] / H<H[1]; afterwards VALUE10=(13*VALUE10[1]+VALUE1)/14 if H>H[1] else 13*VALUE10[1]/14 (VALUE11 mirror with H<H[1]); RVIH=100*VALUE10/(VALUE10+VALUE11). RVIL the same on L. @StdDev = population standard deviation (TradeStation). CurrentBar 1 = data bar 15 (the smallest MaxBarsBack the code needs, 14). [Book silent on MaxBarsBack; the seed decays by (13/14)^n]",
            "F|\"IF @CURRENTBAR<33 THEN INERTIA=50 ELSE INERTIA=AVERAGE((RVIH+RVIL)/2,20)\" (Fig.4, p.6)",
            "E|\"a system that buys when inertia is above 50\" (p.6, Fig.5 text). Buy whenever inertia > 50 at the close and no position is open (TradeStation 'Buy' = next bar at the open: p.fill=0; p.fill=1 fills at the signal close). [Book silent on the fill]",
            "X|\"and sells when inertia is below 50\" (p.6): exit the long when inertia < 50 at the close, same fill. Inertia exactly 50 = no action.",
            "X|Default reading of 'sells' = exit to flat (fitted: the book's S&P 500 row, 18/34 wins = 53%, matches long-only 15/29 = 52% on SPX 1983-1994; stop-and-reverse gives 20/58 = 34%). p.mode=1 = stop-and-reverse (short below 50), labelled non-default reading.",
            "F|p.smooth=1: \"OR A SLIGHTLY BETTER FORMULA IS: ... INERTIA=LINEARREGVALUE((RVIH+RVIL)/2,20,10)\" (Fig.4, p.6) = 20-bar least-squares line evaluated 10 bars ago. Non-default (the article's main formula is AVERAGE)."],
        notes: "Book test (Fig.5): 13 futures, 10-20 years of daily data to ~1994, one contract, no adjustment; ~43% winners overall. Run here on the index instead of the futures contract (S&P 500 row compared with SPX). No stop, target or sizing in the article.",
        claim: claim("34 (S&P 500)", "18/34 = 53% (S&P 500); ~43% all markets", "$75,850 total, one contract", "", "10-20 years of daily futures data to ~1994 (Fig.5)"),
    }]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    (id == "ddrvi_inertia50").then_some(("ddrvi_inertia", "Dorsey inertia (refined RVI)", "Above/below 50"))
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    (id == "ddrvi_inertia50").then(|| vec![
        p("sd_n", "Standard deviation length", 10., 5., 30., 1.),
        p("len", "Inertia average length", 20., 5., 50., 1.),
        p("level", "Inertia level", 50., 40., 60., 1.),
        p("mode", "0 = long/flat (book fit), 1 = stop-and-reverse", 0., 0., 1., 1.),
        p("smooth", "0 = AVERAGE (book), 1 = LINEARREGVALUE(.,len,len/2)", 0., 0., 1., 1.),
        p("fill", "0 = next open (TradeStation default), 1 = signal close", 0., 0., 1., 1.),
    ])
}

const MBB: usize = 14; // data index of CurrentBar 1

fn pstd(a: &[f64], i: usize, n: usize) -> f64 {
    if i + 1 < n { return f64::NAN; }
    let w = &a[i + 1 - n..=i];
    let m = w.iter().sum::<f64>() / n as f64;
    (w.iter().map(|v| (v - m).powi(2)).sum::<f64>() / n as f64).sqrt()
}

/// RVIH / RVIL exactly as the Fig.4 TradeStation code (14-bar Wilder-style smoothing seeded with a 14-bar sum).
fn rvi_ts(p: &[f64], sd_n: usize) -> Vec<f64> {
    let n = p.len();
    let sd: Vec<f64> = (0..n).map(|i| pstd(p, i, sd_n)).collect();
    let (mut up, mut dn, mut out) = (vec![0.; n], vec![0.; n], vec![f64::NAN; n]);
    for i in 0..n {
        if i + 1 < MBB { continue; }
        let cb = i + 1 - MBB;
        if cb == 13 {
            for k in 0..14 {
                if p[i - k] > p[i - k - 1] { up[i] += sd[i - k]; }
                if p[i - k] < p[i - k - 1] { dn[i] += sd[i - k]; }
            }
        } else if cb > 13 {
            up[i] = (13. * up[i - 1] + if p[i] > p[i - 1] { sd[i] } else { 0. }) / 14.;
            dn[i] = (13. * dn[i - 1] + if p[i] < p[i - 1] { sd[i] } else { 0. }) / 14.;
            out[i] = 100. * up[i] / (up[i] + dn[i]);
        }
    }
    out
}

/// TradeStation LinearRegValue(price, n, tgt): least-squares line over the last n bars, evaluated tgt bars ago.
fn lin_reg_value(a: &[f64], i: usize, n: usize, tgt: usize) -> f64 {
    let ys = &a[i + 1 - n..=i];
    let mx = (n as f64 - 1.) / 2.;
    let my = ys.iter().sum::<f64>() / n as f64;
    let (mut sxy, mut sxx) = (0., 0.);
    for (k, y) in ys.iter().enumerate() { let dx = k as f64 - mx; sxy += dx * (y - my); sxx += dx * dx; }
    my + sxy / sxx * ((n - 1 - tgt) as f64 - mx)
}

pub fn inertia(x: &Ctx, sd_n: usize, len: usize, smooth: bool) -> Vec<f64> {
    let (a, b) = (rvi_ts(&x.h, sd_n), rvi_ts(&x.l, sd_n));
    let nv: Vec<f64> = a.iter().zip(&b).map(|(u, v)| (u + v) / 2.).collect();
    (0..nv.len()).map(|i| {
        if i + 1 < MBB || i + 1 - MBB < 33 || i + 1 < len { return 50.; }
        if smooth { lin_reg_value(&nv, i, len, len / 2) } else { nv[i + 1 - len..=i].iter().sum::<f64>() / len as f64 }
    }).collect()
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    let schema = params(id)?;
    if from > to || to >= x.r.len() { return None; }
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).unwrap().def);
    let (sd_n, len, level) = (g("sd_n").round().max(2.) as usize, g("len").round().max(2.) as usize, g("level"));
    let (sar, smooth, at_close) = (g("mode") >= 0.5, g("smooth") >= 0.5, g("fill") >= 0.5);
    let ine = inertia(x, sd_n, len, smooth);
    let mut out = vec![];
    let mut cur: Option<Trade> = None;
    for i in from..=to {
        let want = if ine[i] > level { 1. } else if ine[i] < level { if sar { -1. } else { 0. } } else { continue };
        let pos = cur.as_ref().map_or(0., |t| t.dir);
        if want == pos { continue; }
        let (j, px) = if at_close { (i, x.c[i]) } else if i < to { (i + 1, x.o[i + 1]) } else { break };
        if let Some(mut t) = cur.take() {
            t.exit_i = j; t.exit_px = px; t.signal_i = i;
            t.reason = format!("inertia {} {level}", if t.dir > 0. { "below" } else { "above" });
            out.push(t);
        }
        if want != 0. {
            cur = Some(Trade { dir: want, legs: vec![Leg { i: j, px, w: 1. }], exit_i: j, exit_px: px, reason: String::new(),
                entry_kind: if at_close { "close" } else { "open" }, signal_i: i, open: false, exits: vec![] });
        }
    }
    if let Some(mut t) = cur {
        t.exit_i = to; t.exit_px = x.c[to]; t.signal_i = to; t.reason = "open".into(); t.open = true;
        out.push(t);
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;
    #[test]
    fn rvi_seed_and_recursion_match_hand_values() {
        // strictly rising highs: every bar is an 'up' bar, so RVIH = 100 from CurrentBar 14 on
        let p: Vec<f64> = (0..60).map(|i| 100. + i as f64 + (i % 3) as f64 * 0.1).collect();
        let r = rvi_ts(&p, 10);
        assert!(r[MBB - 1 + 13].is_nan());
        assert!((r[MBB - 1 + 14] - 100.).abs() < 1e-9);
    }
    #[test]
    fn long_only_exits_below_50() {
        let rows: Vec<Row> = (0..200).map(|i| {
            let c = if i < 100 { 100. + i as f64 } else { 300. - i as f64 };
            Row { ticker: "T".into(), date: format!("{i:05}"), open: c, high: c + 1., low: c - 1., close: c, volume: None }
        }).collect();
        let x = Ctx::new(&rows, &rows, &[]);
        let ts = run("ddrvi_inertia50", &x, 0, 199, &Params::new()).unwrap();
        assert!(ts.iter().all(|t| t.dir > 0.));
        let ine = inertia(&x, 10, 20, false);
        assert_eq!(ts[0].legs[0].i, (0..200).find(|&i| ine[i] > 50.).unwrap() + 1);
        assert!(ine[ts[0].exit_i - 1] < 50.);
    }
}
