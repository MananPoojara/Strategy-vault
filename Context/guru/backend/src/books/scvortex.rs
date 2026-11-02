//! Etienne Botes and Douglas Siepman, "The Vortex Indicator", Technical Analysis of Stocks & Commodities, January 2010, and that
//! issue's Traders' Tips (research share Trading Knowledge I/Stocks & Commodities 2010 -2011 (traders.com)/SC2010-01.pdf, OCR of
//! PDF p.4-7 and p.28-33). Book id `scvortex`. Lane K16 (titles loop, 2 Nov 2026), to-code row 298; independent check
//! authors/titles-1030/K16w/k16_ref.py.
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};
use super::sctraub::next_open;
use super::tselhs::Book;

pub const REVIEWED: &[&str] = &["scvortex_14"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const NOTE: &str = "[Declared readings] VM+ = |High - previous Low|, VM- = |Low - previous High|, TR = true range; VI+ = Sum(VM+, 14) / Sum(TR, 14), VI- = Sum(VM-, 14) / Sum(TR, 14) (period 14, the Traders' Tips default). Entry as the article trades it: on the bar where VI+ crosses above VI- a buy stop is placed at that bar's high, good while VI+ stays above VI- and cancelled if it turns back; mirrored, a sell stop at the crossing bar's low; a position is stopped and reversed by the opposite stop (fill at the level or the open beyond it). Option open = 1: reverse at the next open after each cross (the Traders' Tips always-in system). The trailing stop the article recommends has no value and is not applied. Article: compared with Wilder's DMI system on 38 futures (1978-2009) and 101 Nasdaq stocks (1992-2009). Runs on indices, ETFs and stocks, US and India, are a declared application.";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef { id: "scvortex_14", name: "Vortex indicator (14): VI+ crossing VI- arms a stop at the crossing bar's high (low), stop and reverse", group: "S&C: The Vortex Indicator (Botes & Siepman, Jan 2010)", session: 0, dir: "Long/Short", status: Status::Live,
        needs_volume: false, needs_vix: false, needs_series: &[], book: "scvortex", rules: &[
            "F|VI+ = Sum(|High - Low[1]|, 14) / Sum(TR, 14), VI- = Sum(|Low - High[1]|, 14) / Sum(TR, 14) (S&C January 2010, Traders' Tips, PDF p.28-33)",
            "E|When VI+ crosses above VI-, buy on a stop at the high of the crossing bar while VI+ stays above VI- (short: sell stop at the crossing bar's low) (article, OCR PDF p.4-7)",
            "X|The opposite stop closes and reverses the position (always in once filled)."],
        notes: NOTE, claim: None }]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> { reviewed(id).then_some(("scvortex_14", "Vortex indicator (S&C)", "14 periods")) }

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    reviewed(id).then(|| vec![p("n", "Period (14)", 14., 2., 60., 1.), p("open", "1 = reverse at the next open after a cross", 0., 0., 1., 1.)])
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.r.len() { return None; }
    let n = p.get("n").copied().unwrap_or(14.).round().max(1.) as usize;
    let len = x.c.len();
    let (mut vp, mut vm) = (vec![f64::NAN; len], vec![f64::NAN; len]);
    for i in n..len {
        let (mut a, mut b, mut tr) = (0.0, 0.0, 0.0);
        for j in i + 1 - n..=i {
            a += (x.h[j] - x.l[j - 1]).abs();
            b += (x.l[j] - x.h[j - 1]).abs();
            tr += (x.h[j] - x.l[j]).max((x.h[j] - x.c[j - 1]).abs()).max((x.l[j] - x.c[j - 1]).abs());
        }
        if tr > 0.0 { vp[i] = a / tr; vm[i] = b / tr; }
    }
    let cross = |t: usize| -> f64 {
        if t == 0 || !vp[t - 1].is_finite() || !vp[t].is_finite() { return 0.0; }
        if vp[t] > vm[t] && vp[t - 1] <= vm[t - 1] { 1.0 } else if vp[t] < vm[t] && vp[t - 1] >= vm[t - 1] { -1.0 } else { 0.0 }
    };
    if p.get("open").copied().unwrap_or(0.) > 0.5 {
        return Some(next_open(x, from, to, "reversed", |t, side| { let c = cross(t); if c != 0.0 { c } else { side } }));
    }
    let mut pos = Book::new();
    let mut ord: Option<(f64, f64)> = None;
    for t in from..=to {
        if let Some((d, lv)) = ord {
            let hit = if d > 0.0 { x.h[t] >= lv } else { x.l[t] <= lv };
            if hit && pos.side() != d { pos.enter(d, t, if d > 0.0 { x.o[t].max(lv) } else { x.o[t].min(lv) }, "stop"); ord = None; }
        }
        let c = cross(t);
        if c != 0.0 { ord = (pos.side() != c).then(|| (c, if c > 0.0 { x.h[t] } else { x.l[t] })); }
        else if let Some((d, _)) = ord { if !(if d > 0.0 { vp[t] > vm[t] } else { vp[t] < vm[t] }) { ord = None; } }
    }
    Some(pos.done(to, x.c[to]))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn vortex_stops() {
        let r = super::super::sctraub::tests::bars(2000);
        let x = Ctx::new(&r, &r, &[]);
        let t = run("scvortex_14", &x, 0, r.len() - 1, &Params::new()).unwrap();
        assert!(t.len() > 3);
        for w in t.windows(2) { assert_eq!(w[0].exit_i, w[1].legs[0].i); }
    }
}

// Instrument admission: S&C Vortex 2026-11-02 (titles lane K16).
pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book == "scvortex" && reviewed(def.id),
    label: "Botes and Siepman compared the Vortex indicator with Wilder's DMI on futures and Nasdaq stocks (1978-2009). Runs on indices, ETFs and stocks, US and India, are a declared application.",
    basis: "Technical Analysis of Stocks & Commodities, January 2010 (The Vortex Indicator) and its Traders' Tips; independent check authors/titles-1030/K16w/k16_ref.py",
};
