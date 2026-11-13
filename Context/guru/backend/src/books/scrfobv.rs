//! Gilbert Raff, "Time And Indicator Design", Technical Analysis of Stocks & Commodities V.11:2 (February 1993) pp.92-97
//! (research share Trading Knowledge I/Stocks & Commodities DVD 11.26 (1982 - 2008) (traders.com)/VOLUMES/V11/C02/TIMEAND.pdf;
//! the MetaStock signal formulas are images, read with authors/ocr2: authors/pa1/txt/raff_p2.ocr, raff_p7.ocr). Book id
//! `scrfobv`. Lane PA1 (13 Nov 2026, pool A of authors/a1/A1.tsv; R4 #297). Independent check authors/pa1/pa1_ref.py.
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};
use super::msws::ema;
use super::sctraub::week_groups;
use super::tselhs::Book;

pub const REVIEWED: &[&str] = &["scrfobv_3ema"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const NOTE: &str = "[Adapted: 1 unstated detail, R4 #297; unverified] Weekly bars (the article tests 'a weekly chart'; weeks = calendar weeks of the vault's daily bars, volume = the week's total; a day without volume adds nothing). OBV on the weekly bars (MetaStock obv(): + the week's volume on an up close, - on a down close). Three MetaStock exponential averages of OBV: 2, 20 and 50 weeks (Figure 10: mov(obv(),2,E), mov(obv(),20,E), mov(obv(),50,E)). Enter long: 2 > 20 AND 20 > 50; close long: 2 < 20 OR 20 < 50; enter short: 2 < 20 AND 20 < 50; close short: 2 > 20 OR 20 > 50. Orders are filled at the close of the bar after the signal ('traded both long and short positions on the close of the following day of the signal'); as in the MetaStock System Tester an entry needs only its formula to be true while flat. Maximum loss stop 10% of the entry price ('The maximum stop-loss is 10%'). Unstated in the article: (1) how the 10% stop is triggered; vault reading: a stop order 10% beyond the entry fill from the day after the fill, filled at the level or at a worse open. Article (Deutschemark, yen, British pound and T-bond futures, $25,000, $100 commissions): test summaries are images (Figures 11-14, not transcribed). Runs on indices, ETFs and stocks with volume, US and India, are a declared application.";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef { id: "scrfobv_3ema", name: "Raff weekly OBV 2/20/50-week EMAs: long when 2 > 20 > 50, short when 2 < 20 < 50, 10% stop", group: "S&C: Time And Indicator Design (Raff, V.11:2)", session: 0, dir: "Long/Short", status: Status::Live,
        needs_volume: true, needs_vix: false, needs_series: &[], book: "scrfobv", rules: &[
            "E|\"1 If the two-week moving average of OBV is above the 20-week and the 20-week is above the 50-week, buy.\" \"3 If the two-week moving average of OBV is below the 20-week and the 20-week is below the 50-week, short.\" (V.11:2 p.94); Figure 10: when(mov(obv(),2,E),>,mov(obv(),20,E)) AND when(mov(obv(),20,E),>,mov(obv(),50,E))",
            "X|\"2 If the two-week moving average of OBV is below the 20-week or the 20-week is below the 50-week, close long.\" \"4 ... above the 20-week or the 20-week is above the 50-week, close short.\" (p.94-95)",
            "F|\"traded both long and short positions on the close of the following day of the signal ... The maximum stop-loss is 10%.\" (p.93)"],
        notes: NOTE, claim: None }]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    reviewed(id).then_some(("scrfobv_3ema", "Raff weekly OBV averages (S&C)", "2/20/50"))
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    reviewed(id).then(|| vec![p("stop", "Maximum loss stop, % (article 10; 0 = off)", 10., 0., 30., 1.)])
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.r.len() { return None; }
    let stop = p.get("stop").copied().unwrap_or(10.).max(0.) / 100.;
    let w = week_groups(x);
    let vol = |i: usize| x.r[i].volume.filter(|v| *v > 0.).unwrap_or(f64::NAN);
    // a day without volume adds nothing to its week ([Declared reading]: index files have a few volume-less days)
    let mut obv = vec![0.; w.len()];
    let mut acc = 0.;
    for (k, &(s, e)) in w.iter().enumerate() {
        let v: f64 = (s..=e).map(vol).filter(|v| v.is_finite()).sum();
        if k > 0 { let (c, c1) = (x.c[e], x.c[w[k - 1].1]); if c > c1 { acc += v; } else if c < c1 { acc -= v; } }
        obv[k] = acc;
    }
    let (m2, m20, m50) = (ema(&obv, 2), ema(&obv, 20), ema(&obv, 50));
    let mut pos = Book::new();
    let mut order: Option<usize> = None; // the week whose signal fills at this week's close
    for (k, &(s, e)) in w.iter().enumerate() {
        if s > to { break; }
        // stop inside the week (from the day after the entry fill)
        if stop > 0. && pos.side() != 0. {
            let (d, ep, eb) = (pos.side(), pos.entry_px(), pos.entry_bar().unwrap_or(0));
            let lv = ep * (1. - d * stop);
            for i in s.max(eb + 1).max(from)..=e.min(to) {
                if d > 0. && x.l[i] <= lv { pos.exit(i, x.o[i].min(lv), "10% stop"); break; }
                if d < 0. && x.h[i] >= lv { pos.exit(i, x.o[i].max(lv), "10% stop"); break; }
            }
        }
        if let Some(j) = order.take() {
            if e <= to && e >= from {
                let (a, b, c) = (m2[j], m20[j], m50[j]);
                if pos.side() > 0. && (a < b || b < c) { pos.exit(e, x.c[e], "close long"); }
                if pos.side() < 0. && (a > b || b > c) { pos.exit(e, x.c[e], "close short"); }
                if pos.side() == 0. { if a > b && b > c { pos.enter(1., e, x.c[e], "close"); } else if a < b && b < c { pos.enter(-1., e, x.c[e], "close"); } }
            }
        }
        if e >= from && e < to && m50[k].is_finite() && m2[k].is_finite() && m20[k].is_finite() { order = Some(k); }
    }
    Some(pos.done(to, x.c[to]))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn weekly_obv_averages() {
        let r = super::super::sctraub::tests::bars(4000);
        let x = Ctx::new(&r, &r, &[]);
        let t = run("scrfobv_3ema", &x, 0, r.len() - 1, &Params::new()).unwrap();
        assert!(!t.is_empty());
        let w = week_groups(&x);
        for z in &t {
            let e = z.legs[0].i;
            assert!(w.iter().any(|&(_, we)| we == e) && z.legs[0].px == x.c[e]); // entries at a week's last close
            if z.reason == "10% stop" { assert!(z.dir * (z.exit_px / z.legs[0].px - 1.) <= -0.1 + 1e-9); }
        }
    }
}

// Instrument admission: S&C Raff weekly OBV 2026-11-13 (lane PA1, R4 #297).
pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book == "scrfobv" && reviewed(def.id),
    label: "Gilbert Raff tested the weekly OBV averages on currency and T-bond futures; weekly bars of indices, ETFs and stocks with volume, US and India, are a declared application with one labelled reading (R4 #297).",
    basis: "Technical Analysis of Stocks & Commodities V.11:2 pp.92-97 (Time And Indicator Design), Figure 10; R4 #297; independent check authors/pa1/pa1_ref.py",
};
