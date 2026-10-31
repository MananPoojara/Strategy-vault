//! Alfaranda CTA systems recovered from RAR archives (research drive, read in place):
//! - Magnet (c) 2003: `T:/Magnet System 2003 (alfaranda.com)/Open Code.rar` -> `Open Code.txt`.
//! - Onyx: `T:/Onyx System for TS (alfaranda.com)/Open_Code.rar` -> `Open_Code.txt` (daily branch DATACOMPRESSION=2; intraday branch = NeedsData).
//! Book id "alfrd". Helpers from jcfeed. Notes: library/alfrd/README.md. (Weaver-DT, also alfaranda, is rarsys_weaver_dt.)
use super::jcfeed::{gp, Sim};
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "alfrd";
/// Admitted ids (authors/vendor/laneb/SOURCE-CONTRACT.md).
pub const REVIEWED: &[&str] = &["alfrd_magnet", "alfrd_onyx_daily"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }
const MAG: &str = "alfrd_magnet";
const ONX: &str = "alfrd_onyx_daily";
const ONXI: &str = "alfrd_onyx_intraday";

fn def(id: &'static str, name: &'static str, status: Status, rules: &'static [&'static str], notes: &'static str) -> StrategyDef {
    StrategyDef { id, name, group: "Alfaranda (Open Code.rar)", session: 0, dir: "Long/Short", status, needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK, rules, notes, claim: None }
}

pub fn catalogue() -> Vec<StrategyDef> {
    vec![
        def(MAG, "Magnet (range-normalised breakout, stop and reverse at the open)", Status::Live, &[
            "F|\"for value1=slen to llen: sumrgn=sumrgn+truerange[value1]; avergn=sumrgn/value1; mult=1/(avergn*nalf[value1]); rgeh=(h-l[value1])*mult; rgel=(h[value1]-l)*mult\" (sumrgn starts as truerange[1..slen-1]; nalf[v]=power(v,alf)); rgehmax / rgelmax = largest over value1",
            "F|Walk-forward inputs by date: to 1993-12-31 slen 6, llen 21, xoh .95, xol .95, alf .75; 1994 xoh/xol .95; 1995 slen 6, xoh 1.00, xol .95; from 1996 slen 8, llen 21, xoh .65, xol .85, alf .75 (date3..date6)",
            "E|\"If rgehmax >= xoh and rgelmax < xbot Then Buy Next Bar at market\" (xbot 0.9)",
            "E|\"If rgelmax >= xol and rgehmax < xbot Then Sell Short Next Bar at market\"",
            "X|No exit order (Mark 0): an opposite entry reverses the position",
        ],
        "Code inputs are the defaults, including the dated parameter sets (date2 931231 ... date6 971231: a set applies from the day after its date). Data after 1997 uses set 6, exposed as parameters. \
Timeframe and market are not stated in the archive (Open Code.rar, MAGNET.ELA only); the code has no intraday construct and every quantity is range-normalised, so it is run on daily bars [Assumption]. \
Both orders are next-bar market orders; if both fire on one bar they fill in code order at the same open (Buy, then Sell Short) [TradeStation code order]. TrueRange uses the prior close. \
India: instrument swap; index shorts hypothetical."),
        def(ONX, "Onyx, daily mode (gap-reversal close, stop and reverse on the close)", Status::Live, &[
            "F|\"IF DATACOMPRESSION=2 THEN BEGIN MIDP=(H+L)/2\"",
            "E|\"IF OPEN<L[1] AND C>OPEN AND C>MIDP THEN Buy This Bar\"",
            "E|\"IF OPEN>H[1] AND C<OPEN AND C<MIDP THEN Sell Short This Bar\"",
            "X|Daily branch has no exit order: an opposite entry reverses (the LX/SX exits exist only in the intraday branch)",
        ],
        "The code has two branches; DATACOMPRESSION=2 is TradeStation's daily compression, so this daily strategy is complete as written. Fills on the signal bar's close. \
No manual in the folder (ONIX.ELA, ONIX_ts2000i.ELA, Open_Code.rar). India: instrument swap; index shorts hypothetical."),
        def(ONXI, "Onyx, intraday mode (ORDTIME 1600 signals, exit at the open if losing)",
            Status::NeedsData("intraday bars: the DATACOMPRESSION=1 branch needs the day's running high/low and the bar at ORDTIME 16:00 (IDHIGH/IDLOW) and exits on the next day's first bar"), &[
            "E|\"IF T>=ORDTIME AND T[1]<ORDTIME: MIDP=(IDHIGH+IDLOW)/2; IF OPENDAY<PREVLO AND C>OPENDAY AND C>MIDP THEN Buy(\"Buy2\") This Bar\" (mirror for Sell Short)",
            "X|\"IF D>D[1]: IF MP=1 AND OPEN<ENTRYPRICE THEN Sell(\"LX\") This Bar\" (mirror for shorts)",
        ], "Same signal as the daily mode, taken at 16:00 on intraday bars plus a next-session losing-open exit."),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    Some(match id {
        MAG => (MAG, "Magnet (Alfaranda)", "code literal"),
        ONX => ("alfrd_onyx", "Onyx (Alfaranda)", "daily"),
        ONXI => ("alfrd_onyx", "Onyx (Alfaranda)", "intraday"),
        _ => return None,
    })
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    (id == MAG).then(|| vec![p("xbot", "xbot (opposite side must stay below)", 0.9, 0.3, 2., 0.05),
        p("slen6", "slen (set 6, after 1997)", 8., 2., 20., 1.), p("llen6", "llen (set 6)", 21., 5., 50., 1.),
        p("xoh6", "xoh (set 6, buy threshold)", 0.65, 0.3, 1.5, 0.05), p("xol6", "xol (set 6, sell threshold)", 0.85, 0.3, 1.5, 0.05),
        p("alf6", "alf (set 6, length exponent)", 0.75, 0.3, 1.2, 0.05)])
}

fn tr(x: &Ctx, k: usize) -> f64 { x.h[k].max(x.c[k - 1]) - x.l[k].min(x.c[k - 1]) }

fn run_mag(x: &Ctx, from: usize, to: usize, p: &Params) -> Vec<Trade> {
    let xbot = gp(p, "xbot", 0.9);
    // (after date, slen, llen, xoh, xol, alf); the first entry is the initial var values (= set 1)
    let sets = [("", 6., 21., 0.95, 0.95, 0.75), ("1993-12-31", 6., 21., 0.95, 0.95, 0.75), ("1994-12-31", 6., 21., 1.0, 0.95, 0.75),
        ("1995-12-31", 8., 21., 0.65, 0.85, 0.75), ("1996-12-31", 8., 21., 0.65, 0.85, 0.75),
        ("1997-12-31", gp(p, "slen6", 8.), gp(p, "llen6", 21.), gp(p, "xoh6", 0.65), gp(p, "xol6", 0.85), gp(p, "alf6", 0.75))];
    let mut sim = Sim::new();
    for i in from.max(1)..to {
        let d = x.r[i].date.as_str();
        let s = sets.iter().rev().find(|s| d > s.0).unwrap();
        let (slen, llen, xoh, xol, alf) = (s.1.max(2.) as usize, s.2 as usize, s.3, s.4, s.5);
        if llen < slen || i < llen + 1 { continue; }
        let mut sum: f64 = (1..slen).map(|k| tr(x, i - k)).sum();
        let (mut eh, mut el) = (0f64, 0f64);
        for v in slen..=llen {
            sum += tr(x, i - v);
            if sum > 0. {
                let mult = 1. / (sum / v as f64 * (v as f64).powf(alf));
                eh = eh.max((x.h[i] - x.l[i - v]) * mult);
                el = el.max((x.h[i - v] - x.l[i]) * mult);
            }
        }
        let j = i + 1;
        if eh >= xoh && el < xbot { sim.enter(1, j, x.o[j], i, "open", "Buy"); }
        if el >= xol && eh < xbot { sim.enter(-1, j, x.o[j], i, "open", "Sell Short"); }
    }
    sim.finish(x, to)
}

fn run_onyx(x: &Ctx, from: usize, to: usize) -> Vec<Trade> {
    let mut sim = Sim::new();
    for i in from.max(1)..=to {
        let mid = (x.h[i] + x.l[i]) / 2.;
        if x.o[i] < x.l[i - 1] && x.c[i] > x.o[i] && x.c[i] > mid { sim.enter(1, i, x.c[i], i, "close", "Buy"); }
        if x.o[i] > x.h[i - 1] && x.c[i] < x.o[i] && x.c[i] < mid { sim.enter(-1, i, x.c[i], i, "close", "Sell"); }
    }
    sim.finish(x, to)
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if from >= to || to >= x.r.len() { return None; }
    match id { MAG => Some(run_mag(x, from, to, p)), ONX => Some(run_onyx(x, from, to)), _ => None }
}

// Instrument admission: Alfaranda Magnet and Onyx 2026-10-26 (lane B) block, moved from instrument_policy.rs on 31 Oct 2026 (lane ST).
// Admission only for the module REVIEWED lists (authors/vendor/laneb/SOURCE-CONTRACT.md); kept in one block to merge.
// Runs on actual indices, ETFs and stocks, US and India (index_etf_or_stock, nifty50 basket, declared application).
pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book=="alfrd" && crate::books::alfrd::reviewed(def.id),
    label: "Vendor and newsletter code with no per-market test the vault can compare; runs on indices, ETFs and single stocks (US and India) are a declared application, one unit per trade.",
    basis: "Alfaranda Magnet and Onyx TradeStation code",
};
