//! Peter Navarro, "When the Market Moves, Will You Be Ready?" (McGraw-Hill 2004), Ch.8 key point 5, the 3-point-break method
//! (research share Trading Books/Peter Navarro - When the Market Moves, Will You Be Ready/...pdf, book p.78-81 = PDF 95-98, Table 8-2
//! p.79; text and OCR authors/batch-e/E2w/navarro-3pb.txt). Book id `pnwmm`. Lane K10 (titles loop, 1 Nov 2026), to-code row 167;
//! R4 decision 48; independent check authors/titles-1030/K10w/k10_ref.py.
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};
use super::lc3::{bars, Pos};

/// Admitted ids (authors/titles-1030/K10w/k10_ref.py (header)).
pub const REVIEWED: &[&str] = &["pnwmm_3pb"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const NOTE: &str = "[Declared readings] Weekly closes (Monday-based weeks, the close of each week's last trading day; p.weekly = 0 runs it on daily closes, his exercise 8 for swing traders). The first weekly close of the test window is the starting point 0. A later close above the starting point begins an up trend (higher high #1), below it a down trend (lower low #1); in an up trend each close above the last higher high is the next higher high #n, and the trend is broken when a close is below the close of higher high #(n-3), or below the starting point while n is 3 or less ('we continually advance the threshold ... so that it always remains three higher highs away', p.80). Down trends are the mirror. The week that breaks a trend is the new starting point and the next close that differs from it begins the new trend, up or down; a close equal to the last high/low or to the threshold does nothing. Trades at the week's close: long when an up trend reaches its 3rd higher high (the confirmation; the same 'conservative' rule he uses for his short, lead decision 48), sold the week the up trend is broken; short at the 3rd lower low, covered the week the down trend is broken. Options: enter at the 1st higher high / lower low (p.confirm = 1), long only. No stop in the method. Table 8-2 (QQQ weekly from 9/21/2001) is reproduced close for close on the vault's QQQ: short 3/28/2002 $36.06, covered 8/23/2002 $25.15 (+$10,910 per 1,000 shares as printed); his long example buys the starting week 9/21/2001 $28.19, which is not a signal (here: long from the confirmation 10/12/2001 $34.55, out 2/8/2002 $36.17). The book's instrument is QQQ; other ETFs, indices and US/India stocks are a declared application.";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef { id: "pnwmm_3pb", name: "Navarro 3-point-break: long at the 3rd weekly higher high, short at the 3rd lower low, out when the trend breaks", group: "When the Market Moves, Will You Be Ready? (Ch.8)",
        session: 0, dir: "Long/Short", status: Status::Live, needs_volume: false, needs_vix: false, needs_series: &[], book: "pnwmm", rules: &[
            "F|\"we mark 0 in column 3 to set our starting point ... if the price closes higher than our starting point, we will have the beginning of a possible up trend\" (p.80)",
            "F|\"QQQ would have to close below the starting point of $28.19 three higher highs away to break the up trend ... we continually advance the threshold for breaking the trend so that it always remains three higher highs away\" (p.80)",
            "E|\"Note that at this point with three higher highs, the up trend is solidly confirmed.\" (p.80); \"suppose you were conservative and had shorted 1000 shares of QQQ on March 29 (3/29), after the confirmation of the down trend\" (p.81)",
            "X|\"the up trend established by that eighth higher high is finally broken on the week of February 8 (2/8) when QQQ closes below $37.73\" (p.80); \"not until August 23 (8/23) is the down trend finally broken, when the closing price of $25.15 is higher than the twelfth lower low\" (p.81)"],
        notes: NOTE, claim: None }]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    (id == "pnwmm_3pb").then_some(("pnwmm_3pb", "Navarro 3-point-break", "weekly, 3rd high/low"))
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    reviewed(id).then(|| vec![p("weekly", "Bars: 1 = weekly closes (book), 0 = daily closes", 1., 0., 1., 1.),
        p("confirm", "Enter at higher high / lower low number (3 = confirmation; 1 = the trend's beginning)", 3., 1., 3., 1.),
        p("long", "Sides: 0 = long and short (default), 1 = long only", 0., 0., 1., 1.)])
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.r.len() { return None; }
    let schema = params(id)?;
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).unwrap().def);
    let (conf, long_only) = (g("confirm").round().clamp(1., 3.) as usize, g("long") > 0.5);
    let mut pos = Pos::new();
    let (mut sp, mut trend, mut pts) = (f64::NAN, 0i8, Vec::<f64>::new());
    for b in bars(x, to, g("weekly") > 0.5).into_iter().filter(|b| b.1 >= from) {
        let (i, cl) = (b.1, b.5);
        if sp.is_nan() { sp = cl; continue; }
        let d = trend as f64;
        if trend == 0 {
            if cl != sp { trend = if cl > sp { 1 } else { -1 }; pts = vec![cl]; }
        } else {
            let n = pts.len();
            let thr = if n <= 3 { sp } else { pts[n - 4] };
            if d * (cl - thr) < 0. {
                if pos.side() == d { pos.exit(i, cl, "trend broken"); }
                sp = cl; trend = 0; pts.clear();
                continue;
            }
            if d * (cl - pts[n - 1]) > 0. { pts.push(cl); }
        }
        if trend != 0 && pts.len() == conf && pos.side() == 0. && (trend > 0 || !long_only) {
            pos.enter(trend as f64, i, cl, "close");
        }
    }
    Some(pos.done(to, x.c[to]))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;
    #[test]
    fn table_8_2_sequence_on_daily_closes() {
        // the book's weekly closes 9/21/01 - 8/30/02 as daily bars: long 10/12 34.55 -> 2/8 36.17, short 3/29 36.06 -> 8/23 25.15
        let q = [28.19, 28.98, 31.76, 34.55, 33.55, 36.01, 35.60, 37.73, 39.37, 39.28, 39.65, 41.73, 40.11, 39.48, 40.33, 41.67, 40.85, 38.59, 38.83, 38.14, 36.17,
            35.78, 33.65, 35.74, 38.67, 37.23, 36.68, 36.06, 34.37, 33.52, 34.46, 31.04, 29.74, 29.56, 32.93, 31.24, 30.04, 28.30, 27.62, 25.82, 26.10, 26.34,
            24.80, 23.99, 22.67, 22.25, 23.33, 24.70, 25.15, 23.49];
        let r: Vec<Row> = q.iter().enumerate().map(|(i, &c)| Row { ticker: "T".into(), date: format!("{:05}", i), open: c, high: c, low: c, close: c, volume: None }).collect();
        let x = Ctx::new(&r, &r, &[]);
        let t = run("pnwmm_3pb", &x, 0, r.len() - 1, &Params::from([("weekly".to_string(), 0.)])).unwrap();
        let v: Vec<(f64, f64, f64)> = t.iter().map(|z| (z.dir, z.legs[0].px, z.exit_px)).collect();
        assert_eq!(v, vec![(1., 34.55, 36.17), (-1., 36.06, 25.15)]);
        assert!(run("pnwmm_x", &x, 0, 5, &Params::new()).is_none());
    }
}

// Instrument admission: Peter Navarro, When the Market Moves, Will You Be Ready? 2026-11-01 (titles lane K10).
pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book == "pnwmm" && reviewed(def.id),
    label: "Peter Navarro worked the 3-point-break method on QQQ weekly closes (Table 8-2). Runs on other ETFs, indices and stocks, US and India, are a declared application.",
    basis: "Peter Navarro, When the Market Moves, Will You Be Ready? (McGraw-Hill 2004) Ch.8 p.78-81 and Table 8-2 (PDF p.95-98); independent check authors/titles-1030/K10w/k10_ref.py",
};
