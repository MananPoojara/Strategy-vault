//! Active Trader Magazine, May 2001: the Dow / S&P 500 / Nasdaq ratio-and-range study, read from its workbook
//! "May 2001.xls" (research share, Trading Books/Magazine - Active Trader Magazine (activetradermag.com)/May 2001/May 2001.xls;
//! sheets Dow, SP500, Nasdaq, Portf.). The article text is not in the file; the rule is the sheets' own formulas (Mode AA, Trigger AV,
//! entry CP, stop BQ, target CL, exit CS). Book id `atrio`. Lane C4 (17 Nov 2026), R4 #339. Independent check
//! authors/convert-1117/C4/w/atrio_ref.py, which also reproduces the workbook's own 405 cached trades (133 Dow + 141 S&P + 131 Nasdaq).
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};
use super::tselhs::Book;

pub const REVIEWED: &[&str] = &["atrio_ratio_range"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }
const NAN: f64 = f64::NAN;

const NOTE: &str = "[Declared readings] Source = the workbook's formulas only (no article text, no printed prose rule); the card is the formulas as written. Each market trades itself: the Dow sheet Dow, the SP500 sheet the S&P 500, the Nasdaq sheet the Nasdaq 100 (the sheet's 'Nasdaq' level 253.8 on 19 Mar 1991 equals our NDX close 253.85, not the Composite). Ratios on the sheet's closes: DJI: f1 = NDX/DJI, f2 = SPX/DJI; SPX: f1 = NDX/SPX, f2 = SPX/DJI; NDX: f1 = NDX/DJI, f2 = NDX/SPX. H = SMA80(f1)/SMA80(f1) 20 bars earlier, K the same for f2. Mode: DJI Buy if H < 1 and K < 1, Sell if H > 1 and K > 1, else Buy; SPX Buy if H < 1 and K > 1, Sell if H > 1 and K < 1, else Buy; NDX Buy if H > 1 and K > 1, Sell if H < 1 and K < 1, else Buy. Trigger on a close: the mean of (High - Low) over 3 bars is below its 30-bar mean. Entry: at the next open, Buy = long, Sell = short, one position, no re-entry on the exit day. AW = the 3-bar mean range at the signal close. Long: stop entry - 2 AW on the entry bar; later bars max(previous close - 2 AW[t-1], entry - 2 AW(entry)), target entry + 6 AW[t-1]. Short: stop entry + 3 AW(entry), target entry - 3 AW[t-1]. Stop is tested before target; after the entry bar a gap fills at the open if beyond the level. No time exit. The workbook's 8% risk / commitment / $20 commission portfolio sizing is not part of the rule; the vault's standard costs. Card-specific unstated choices (3), each independent: (1) each market trades itself, with the ratio index series chosen as DJI / SPX / NDX (our NDX for the sheet's Nasdaq level, whose 19 Mar 1991 value equals our NDX close); (2) data are aligned by date with the last bar carried over gaps of up to 5 bars; (3) no re-entry on the exit day. Honest total: 3 card-specific choices (audit count 5; its item 3 the vendor opens replaced by our real opens is the labelled data reading (run clamped to real opens, #240) and its item 5 the gap fill at the open beyond a level is an engine default per #404). Engine-wide defaults / declared adaptations (labelled, not counted, R4 #404): a gap beyond a level after the entry bar fills at the open; one position at a time; the vault's standard costs instead of the workbook's sizing and commission; further labelled data readings: vendor opens replaced by our real opens (run clamped to real opens, decision #240 / clamp_open), data aligned by date with the last bar carried over gaps of up to 5 bars. Unverified: the workbook's own result (405 trades, 27.9% winners, 1991-2001 vendor data) is reproduced by the reference on the sheet's own bars, not by our data.";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef { id: "atrio_ratio_range", name: "Dow / S&P 500 / Nasdaq ratio mode, enter at the open after a range contraction, ATR-style stop and target", group: "Active Trader May 2001 workbook", session: 0, dir: "Long/Short", status: Status::Live,
        needs_volume: false, needs_vix: false, needs_series: &[], book: "atrio", rules: &[
            "F|Mode (sheet AA): 80-bar mean of two index ratios against its value 20 bars earlier decides Buy or Sell (see the card text)",
            "E|Trigger (sheet AV): 3-bar mean of High-Low below the 30-bar mean; entry at the next open (sheet CP) long on Buy, short on Sell",
            "X|Stop (sheet BQ) and target (sheet CL) in multiples of the 3-bar mean range; stop first (sheet CS)"],
        notes: NOTE, claim: None }]
}
pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    (id == "atrio_ratio_range").then_some(("atrio_ratio_range", "Index ratio mode + range contraction", "DJI / SPX / NDX"))
}
pub fn params(_id: &str) -> Option<Vec<ParamDef>> { None }

fn mean(v: &[f64]) -> f64 { v.iter().sum::<f64>() / v.len() as f64 }

/// kind: 0 = DJI, 1 = SPX, 2 = NDX. (dj, sp, nd) = closes on the traded bars. Mode +1 Buy / -1 Sell / 0 undefined.
pub fn modes(kind: u8, dj: &[f64], sp: &[f64], nd: &[f64]) -> Vec<i8> {
    let n = dj.len();
    let (a1, b1, a2, b2) = match kind { 0 => (nd, dj, sp, dj), 1 => (nd, sp, sp, dj), _ => (nd, dj, nd, sp) };
    let f1: Vec<f64> = (0..n).map(|i| a1[i] / b1[i]).collect();
    let f2: Vec<f64> = (0..n).map(|i| a2[i] / b2[i]).collect();
    let avg = |f: &[f64], t: usize| mean(&f[t - 79..=t]);
    let mut m = vec![0i8; n];
    for t in 99..n {
        if f1[t - 99..=t].iter().chain(&f2[t - 99..=t]).any(|v| v.is_nan()) { continue; }
        let h = avg(&f1, t) / avg(&f1, t - 20);
        let k = avg(&f2, t) / avg(&f2, t - 20);
        let buy = match kind { 0 => !(h > 1. && k > 1.), 1 => !(h > 1. && k < 1.), _ => !(h < 1. && k < 1.) };
        m[t] = if buy { 1 } else { -1 };
    }
    m
}

pub fn sim(o: &[f64], h: &[f64], l: &[f64], c: &[f64], mode: &[i8], from: usize, to: usize) -> Vec<Trade> {
    let n = c.len();
    let rg: Vec<f64> = (0..n).map(|i| h[i] - l[i]).collect();
    let aw: Vec<f64> = (0..n).map(|t| if t >= 2 { mean(&rg[t - 2..=t]) } else { NAN }).collect();
    let sig: Vec<bool> = (0..n).map(|t| mode[t] != 0 && t >= 29 && aw[t] < mean(&rg[t - 29..=t])).collect();
    let mut book = Book::new();
    let mut pos: Option<(f64, f64, f64)> = None; // (dir, entry, initial risk)
    for t in from + 1..=to {
        let mut fresh = false;
        if pos.is_none() {
            if !sig[t - 1] { continue; }
            let d = if mode[t - 1] > 0 { 1. } else { -1. };
            pos = Some((d, o[t], if d > 0. { 2. } else { 3. } * aw[t - 1]));
            book.enter(d, t, o[t], "open");
            fresh = true;
        }
        let (d, e, risk) = pos.unwrap();
        let a = aw[t - 1];
        let mut xp = None;
        if d > 0. {
            let stop = if fresh { e - risk } else { (c[t - 1] - 2. * a).max(e - risk) };
            let tgt = e + 6. * a;
            if l[t] <= stop { xp = Some((if fresh { stop } else { o[t].min(stop) }, "stop")); }
            else if h[t] >= tgt { xp = Some((if fresh { tgt } else { o[t].max(tgt) }, "target")); }
        } else {
            let stop = e + risk;
            let tgt = e - 3. * a;
            if h[t] >= stop { xp = Some((if fresh { stop } else { o[t].max(stop) }, "stop")); }
            else if l[t] <= tgt { xp = Some((if fresh { tgt } else { o[t].min(tgt) }, "target")); }
        }
        if let Some((px, why)) = xp { book.exit(t, px, why); pos = None; }
    }
    book.done(to, c[to])
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, _p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.r.len() { return None; }
    let t = x.r.first()?.ticker.as_str();
    let kind = match t { "DJI" => 0u8, "SPX" => 1, "NDX" => 2, _ => return None };
    let data = crate::books::rotation::PRELOADED.get()?;
    let get = |sym: &str, own: &str| -> Option<Vec<f64>> {
        if sym == own { return Some(x.c.clone()); }
        Some(crate::second::align(sym, x.r, data.get(sym)?).carried(crate::second::Field::C))
    };
    let (dj, sp, nd) = (get("DJI", t)?, get("SPX", t)?, get("NDX", t)?);
    let m = modes(kind, &dj, &sp, &nd);
    Some(sim(&x.o, &x.h, &x.l, &x.c, &m, from, to))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn long_stop_target_and_short_rules() {
        // flat quiet bars then a range contraction after wide bars: signal at bar 40, long entry at open 41
        let n = 60;
        let mut o = vec![100.; n]; let mut h = vec![105.; n]; let mut l = vec![95.; n]; let mut c = vec![100.; n];
        for t in 37..=40 { h[t] = 100.5; l[t] = 99.5; }
        // bar 41: open 100, low 99.3 > stop (100 - 2*1 = 98): hold; bar 42: low 97 <= max(prev close - 2, 98) = 98, gap-aware fill at min(open, stop)
        l[41] = 99.3; h[41] = 101.; l[42] = 97.; o[42] = 99.; c[41] = 100.;
        let mut mode = vec![0i8; n]; for t in 40..n { mode[t] = 1; }
        let tr = sim(&o, &h, &l, &c, &mode, 30, n - 1);
        assert_eq!(tr[0].legs[0].i, 41);
        assert_eq!(tr[0].exit_i, 42);
        assert!((tr[0].exit_px - 98.).abs() < 1e-12 && tr[0].reason == "stop");
        mode.iter_mut().for_each(|v| if *v > 0 { *v = -1 });
        let tr = sim(&o, &h, &l, &c, &mode, 30, n - 1);
        assert!(tr[0].dir < 0.);
    }
    #[test]
    fn mode_is_defined_only_after_a_hundred_bars() {
        let v: Vec<f64> = (0..130).map(|i| 100. + i as f64).collect();
        let m = modes(0, &v, &v, &v);
        assert!(m[..99].iter().all(|x| *x == 0) && m[99] != 0 && modes(0, &v, &v, &v) == m);
        assert!(run("atrio_x", &Ctx::new(&[], &[], &[]), 0, 1, &Params::new()).is_none());
    }
}

// Instrument admission: Active Trader May 2001 workbook (lane C4, 17 Nov 2026). Only the three indices of the sheets run (others: no result).
pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book == "atrio" && reviewed(def.id),
    label: "The workbook tested the Dow, the S&P 500 and the Nasdaq 100 on vendor data 1991-2001; our run uses our own index bars (real opens only) and any other instrument is refused.",
    basis: "May 2001.xls (research share, Active Trader Magazine May 2001), sheets Dow / SP500 / Nasdaq formulas read in full; independent check authors/convert-1117/C4/w/atrio_ref.py",
};
