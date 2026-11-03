//! SVOS EURJPY D1 (Brian Dee, SelectFX, 2009), MetaTrader 4 expert advisor with its VHF indicator, from the Mostafa Belkhayate course
//! CD (research share Trading Knowledge I/Mostafa Belkhayate System (belkhayate.biz)/Course CD/Experts Advisors/SVOS EURJPY D1/
//! SVOS EURJPY D1.mq4 and VHF.mq4). Book id `mtsvos`. Lane K18 (titles loop, 2 Nov 2026), to-code row 326; independent check
//! authors/titles-1030/K18w/k18_ref.py.
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};
use super::sctraub::{ema0, next_open};
use super::rrsim::sma;

pub const REVIEWED: &[&str] = &["mtsvos_d1"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const NOTE: &str = "[Declared readings] MetaTrader 4 expert advisor written for EURJPY daily bars; the vault holds no EURJPY, so it runs on daily indices, ETFs and stocks as a declared application (decisions 88, 114). The EA acts on the first tick of each day on the last closed bar's values, read here as a decision at the close and a fill at the next open. VHF(20) as printed in VHF.mq4: |High of the bar with the highest close - Low of the bar with the lowest close| over 20 bars / sum of |Close - previous Close| over the same 20 bars (ties: the latest bar); trending when VHF >= 0.4. OsMA(10, 25, 5) = MACD (EMA10 - EMA25 of the close, EMAs seeded with the first close) - its 5-bar simple average (MetaTrader's MACD signal); Stochastic(8, 3, 3), simple averages, Low/High: main = 100 x sum of 3 (Close - lowest Low 8) / sum of 3 (highest High 8 - lowest Low 8) (100 when the range is zero), signal = 3-bar average of main. Candles as printed: Body = |Close - Open|; doji Body < (High - Low) / 8.5; morning/evening star and engulfing patterns on the last three closed bars. Exits: shorts close on a bullish engulfing, a morning star, (trending and OsMA > 0) or (not trending and main >= signal); longs mirrored. Entries: a buy when the last bar is up, not an evening star, doji or bearish engulfing, and (trending: OsMA > 0 and rising) or (not trending: main > signal); a sell mirrored. The exits always close the opposite side first, so an opposite entry is a reversal. One position at a time: the EA's extra orders (up to 4 trending, 2 ranging) and its 3x lot boost are not modelled. HELD: the 90-pip stop, 350-pip target and 150-pip trailing stop (pip amounts, decisions 19, 88) and the StdDev(20) > 0.3 filter (a fixed EURJPY price amount, held as in decision 71; option sdmin applies the printed 0.3 price units literally, StdDev = root mean square of the 20 closes around their EMA20). No results are printed. Runs on indices, ETFs and stocks, US and India, are a declared application.";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef { id: "mtsvos_d1", name: "SVOS EURJPY D1 EA: VHF picks OsMA (trending) or Stochastic (ranging) signals on an up / down day, candle-pattern exits", group: "MT4 EA: SVOS EURJPY D1 (Belkhayate course CD)", session: 0, dir: "Long/Short", status: Status::Live,
        needs_volume: false, needs_vix: false, needs_series: &[], book: "mtsvos", rules: &[
            "F|VHF(20) >= 0.4 -> trending (VHF.mq4); OsMA(10, 25, 5); Stochastic(8, 3, 3, SMA, Low/High); candle helpers Body, Doji (/8.5), morning/evening star, engulfing",
            "E|Buy: Close[1] > Open[1], no evening star / doji / bearish engulfing, and (trending and OSMA_1 > 0 and OSMA_1 > OSMA_2) or (not trending and STOCHM_1 > STOCHS_1); sell mirrored",
            "X|CloseAllSellOrders on bullish engulfing, morning star, (trending and OSMA_1 > 0) or (not trending and STOCHM_1 >= STOCHS_1); CloseAllBuyOrders mirrored. 90-pip stop, 350-pip target, 150-pip trail held."],
        notes: NOTE, claim: None }]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> { reviewed(id).then_some(("mtsvos_d1", "SVOS EURJPY D1 EA", "VHF 20 / 0.4")) }

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    reviewed(id).then(|| vec![p("vhf", "VHF threshold (0.4)", 0.4, 0.1, 0.9, 0.05), p("sdmin", "StdDev(20) above x price units (EA 0.3; held, 0 = off)", 0., 0., 100., 0.1)])
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.r.len() { return None; }
    let thr = p.get("vhf").copied().unwrap_or(0.4);
    let sdmin = p.get("sdmin").copied().unwrap_or(0.);
    let (o, h, l, c) = (&x.o, &x.h, &x.l, &x.c);
    let n = c.len();
    let nan = f64::NAN;
    // VHF(20)
    let vhf: Vec<f64> = (0..n).map(|i| {
        if i < 20 { return nan; }
        let (mut hi, mut lo) = (i, i);
        for j in (i - 19..=i).rev() { if c[j] > c[hi] { hi = j; } if c[j] < c[lo] { lo = j; } }
        let mut den = 0.;
        for k in 0..20 { den += (c[i - k] - c[i - k - 1]).abs(); }
        (h[hi] - l[lo]).abs() / den
    }).collect();
    let (e10, e25) = (ema0(c, 10), ema0(c, 25));
    let macd: Vec<f64> = (0..n).map(|i| e10[i] - e25[i]).collect();
    let sig = sma(&macd, 5);
    let osma: Vec<f64> = (0..n).map(|i| macd[i] - sig[i]).collect();
    let (mut ll, mut hh) = (vec![nan; n], vec![nan; n]);
    for i in 7..n { ll[i] = (i - 7..=i).map(|j| l[j]).fold(f64::MAX, f64::min); hh[i] = (i - 7..=i).map(|j| h[j]).fold(f64::MIN, f64::max); }
    let main: Vec<f64> = (0..n).map(|i| {
        if i < 9 { return nan; }
        let (mut a, mut b) = (0., 0.);
        for k in 0..3 { a += c[i - k] - ll[i - k]; b += hh[i - k] - ll[i - k]; }
        if b == 0. { 100. } else { a / b * 100. }
    }).collect();
    let ssig = sma(&main, 3);
    let e20 = ema0(c, 20);
    let sd: Vec<f64> = (0..n).map(|i| if i < 19 { nan } else { ((0..20).map(|k| (c[i - k] - e20[i]).powi(2)).sum::<f64>() / 20.).sqrt() }).collect();
    let body = |i: usize| (o[i] - c[i]).abs();
    Some(next_open(x, from, to, "reversed", |t, side| {
        if t < 2 { return side; }
        let (b1, b2, b3) = (body(t), body(t - 1), body(t - 2));
        let morning = b3 > b2 && b1 > b2 && c[t - 2] < o[t - 2] && c[t] > o[t] && c[t] > o[t - 2].min(c[t - 2]) + b3 * 0.5;
        let evening = b3 > b2 && b1 > b2 && c[t - 2] > o[t - 2] && c[t] < o[t] && c[t] < o[t - 2].max(c[t - 2]) - b3 * 0.5;
        let bull_eng = c[t - 1] < o[t - 1] && c[t] > o[t] && b2 < b1;
        let bear_eng = c[t - 1] > o[t - 1] && c[t] < o[t] && b2 < b1;
        let doji = b1 < (h[t] - l[t]) / 8.5;
        let tr = vhf[t] >= thr;
        let (m1, s1, z1, z2) = (main[t], ssig[t], osma[t], osma[t - 1]);
        let mut s = side;
        if s < 0. && (bull_eng || morning || (tr && z1 > 0.) || (!tr && m1 >= s1)) { s = 0.; }
        if s > 0. && (bear_eng || evening || (tr && z1 < 0.) || (!tr && m1 <= s1)) { s = 0.; }
        let sd_ok = sdmin <= 0. || sd[t] > sdmin;
        let mut act = 0.;
        if !evening && !doji && !bear_eng && c[t] > o[t] && sd_ok && ((z1 > 0. && z1 > z2 && tr) || (m1 > s1 && !tr)) { act = 1.; }
        if !morning && !doji && !bull_eng && c[t] < o[t] && sd_ok && ((z1 < 0. && z1 < z2 && tr) || (m1 < s1 && !tr)) { act = -1.; }
        if act != 0. && s == 0. { s = act; }
        s
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn svos_trades_both_sides() {
        let r = super::super::sctraub::tests::bars(2000);
        let x = Ctx::new(&r, &r, &[]);
        let t = run("mtsvos_d1", &x, 0, r.len() - 1, &Params::new()).unwrap();
        assert!(t.len() > 5 && t.iter().any(|z| z.dir > 0.) && t.iter().any(|z| z.dir < 0.));
        for w in t.windows(2) { assert!(w[1].legs[0].i >= w[0].exit_i); }
        assert!(run("mtsvos_x", &x, 0, 5, &Params::new()).is_none());
    }
}

// Instrument admission: SVOS EURJPY D1 2026-11-02 (titles lane K18).
pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book == "mtsvos" && reviewed(def.id),
    label: "MetaTrader 4 expert advisor hard-coded to EURJPY daily bars, with no published results. Runs on indices, ETFs and stocks, US and India, are a declared application.",
    basis: "Belkhayate course CD, SVOS EURJPY D1.mq4 and VHF.mq4 (printed code); independent check authors/titles-1030/K18w/k18_ref.py",
};
