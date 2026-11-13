//! Lane R2C (13 Nov 2026; authors/titles-1030/R2C.tsv; R4 #297 "up to two unstated details"; reader row authors/o3/O3.tsv):
//! Philip Gotthelf, "Precious Metals Trading" (Wiley, 2005; research share 'USB HDD 2/tRADING KNO-2/Philip Gotthelf - Precious
//! Metals Trading/0471711519.pdf'; printed pages cited). Book id `pgpmt`.
//! Coded: the 30-day moving average trend rule of ch.5 'Price Action' (printed p.107-114), stop and reverse at the close.
//! Held: the seasonal-month patterns (read off charts), the 31-day average used to smooth gold for seasonal study (p.~118, a
//! research tool, no trade rule), metals fundamentals and options chapters.
//! Independent reference: authors/titles-1030/R2Cw/r2c_ref.py (written from the pages, not from this file).
use crate::connors::{sma, Ctx, ParamDef, Params, Status, StrategyDef, Trade};
use super::tselhs::Book;

pub const REVIEWED: &[&str] = &["pgpmt_ma30"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const NOTE: &str = "[Adapted: 2 unstated details, R4 #297; unverified] Simple average of the last 30 daily closes. A close above the average is long, a close below it is short; each crossing reverses the position at that close (the book's silver example goes long on the breakout above the 30-day average and short on the bust below it, with 'a few interim stop-outs as prices briefly touched the moving average line', p.114). Unstated in the book: (1) what counts as a crossing and where it is filled - the book follows the line on futures charts; vault reading: the daily close crossing the average, filled at that close (a close exactly on the average changes nothing). (2) the size of the 'penetration factor' the book recommends to confirm a crossing ('simply an adjustment made to the moving average to confirm the validity of any crossing', p.112) but never sizes ('statistically derived by back-testing'); vault reading: no penetration (0%), the plain average the book's own example follows ('All we were following was a simple 30-day moving average', p.114). Option pen = a penetration in percent of the average (a close must be that far beyond the average to reverse). The book shows the rule on silver and gold futures charts (eSignal) and prints no trade results; it runs here on indices, ETFs (including GLD; SLV is not held) and stocks, US and India, as a declared application.";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef { id: "pgpmt_ma30", name: "Gotthelf: close above / below the 30-day moving average, stop and reverse", group: "Price Action (ch.5, p.107-114)",
        session: 0, dir: "Long/Short", status: Status::Live, needs_volume: false, needs_vix: false, needs_series: &[], book: "pgpmt",
        rules: &["E|\"In early November, the price broke out above the 30-day moving average ... Once silver broke out, it steadily progressed ahead of the 30-day moving average\" (p.113); \"Thereafter, there is an assumption that you would have sold short for the decline\" (p.114)",
            "X|\"All we were following was a simple 30-day moving average coupled with widely known charting rules. Undoubtedly, there were a few interim stop-outs as prices briefly touched the moving average line.\" (p.114)",
            "F|\"A penetration factor is simply an adjustment made to the moving average to confirm the validity of any crossing by the price. In some cases, the factor is statistically derived by back-testing\" (p.112); \"This is why some traders use the penetration factor referenced earlier.\" (p.114)"],
        notes: NOTE, claim: None }]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    (id == "pgpmt_ma30").then_some(("pgpmt_ma30", "Gotthelf 30-day moving average", "30"))
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    reviewed(id).then(|| vec![p("n", "Moving average, days (book 30)", 30., 2., 200., 1.), p("pen", "Penetration, % of the average (book: not sized; vault 0)", 0., 0., 5., 0.1)])
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.r.len() { return None; }
    let n = p.get("n").copied().unwrap_or(30.).round().max(1.) as usize;
    let pen = p.get("pen").copied().unwrap_or(0.).max(0.) / 100.;
    let ma = sma(&x.c, n);
    let mut pos = Book::new();
    for t in from..=to {
        if !ma[t].is_finite() { continue; }
        if x.c[t] > ma[t] * (1. + pen) { pos.enter(1., t, x.c[t], "close"); }
        else if x.c[t] < ma[t] * (1. - pen) { pos.enter(-1., t, x.c[t], "close"); }
    }
    Some(pos.done(to, x.c[to]))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn ma30_reverses_at_the_crossing_close() {
        let r = super::super::sctraub::tests::bars(2500);
        let x = Ctx::new(&r, &r, &[]);
        let ma = sma(&x.c, 30);
        let t = run("pgpmt_ma30", &x, 0, r.len() - 1, &Params::new()).unwrap();
        assert!(t.len() > 5 && t.iter().any(|z| z.dir > 0.) && t.iter().any(|z| z.dir < 0.));
        for z in &t { let e = z.legs[0].i; assert!(z.dir * (x.c[e] - ma[e]) > 0.); if !z.open { assert!(z.dir * (x.c[z.exit_i] - ma[z.exit_i]) < 0.); } }
        for w in t.windows(2) { assert_eq!(w[1].legs[0].i, w[0].exit_i); assert_eq!(w[1].dir, -w[0].dir); }
        let mut p = Params::new(); p.insert("pen".into(), 1.);
        assert!(run("pgpmt_ma30", &x, 0, r.len() - 1, &p).unwrap().len() < t.len());
    }
}

// Instrument admission: Gotthelf 30-day average daily application 2026-11-13 (lane R2C, R4 #297).
pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book == "pgpmt" && reviewed(def.id),
    label: "Philip Gotthelf shows the 30-day moving average on silver and gold futures charts; daily bars of indices, ETFs and stocks, US and India, are a declared application, with two unstated details read as labelled on the card (R4 #297).",
    basis: "Precious Metals Trading (Wiley 2005) ch.5 p.107-114; R4 #297; independent check authors/titles-1030/R2Cw/r2c_ref.py",
};
