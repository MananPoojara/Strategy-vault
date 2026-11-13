//! Bryan Strain, "How To Get In With The Trend And Out At The End", Technical Analysis of Stocks & Commodities V.18:6 (2000)
//! pp.36-41 (research share Trading Knowledge I/Stocks & Commodities DVD 11.26 (1982 - 2008) (traders.com)/VOLUMES/V18/C06/
//! 051HOW.pdf). Book id `scbstfs`: the trend-following system (TFS). Lane PA2 (13 Nov 2026); independent check
//! authors/pa2/w/pa2_ref.py. Admission: sclang.rs (PA2_BOOKS).
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};
use super::msws::sma;
use super::scdkdmi::at_close;

pub const REVIEWED: &[&str] = &["scbstfs_tfs"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const NOTE: &str = "[Adapted: 2 unstated details, R4 #297; unverified] Tether line = (HHV(H,50) + LLV(L,50)) / 2 (the 50-day high and low including today, as the MetaStock formula); volume oscillator = MOV(IF(C>O,+V,IF(C<O,-V,0)),7,S); MBO = MOV(C,25,S) - MOV(C,200,S). Buy when the close crosses above the tether line (today's close above it, yesterday's not) while the volume oscillator is positive and today's MBO is above yesterday's ('This indicator must be positive when the close crosses above the tether line'); sell when the close crosses below the tether line ('the tether line becomes your stop'). Long only ('not meant for ... short selling'). Unstated in the article: (1) the fill - vault reading: the close of the signal day ('designed for end-of-day trading'); (2) the fundamentals check before each entry ('make sure the company has fundamentals that favor it') is not defined - vault reading: left out. The article trades S&P 500 stocks and prints five trades on one chart (17.55, -1.0625, -0.5625, -0.0625, 3.75 points) without naming the stock or dates, so it stays UNVERIFIED. Same strategy as jcsis_tres (Cava's Spanish reprint, long/short) and djcs_tfs (Taylor's declared variation); this card is Strain's original article. Reads the open, high and low, so index runs start where real opens start. Needs volume; indices, ETFs and stocks, US and India, with volume are a declared application.";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef { id: "scbstfs_tfs", name: "Strain trend-following system: close crosses above the 50-day tether line with a positive 7-day volume oscillator and a rising 25/200 MBO; out below the tether line", group: "S&C: get in with the trend and out at the end (Strain, V.18:6)", session: 0, dir: "Long", status: Status::Live,
        needs_volume: true, needs_vix: false, needs_series: &[], book: "scbstfs", rules: &[
            "F|\"Identify the highest high value of the past 50 periods. Identify the lowest low value of the past 50 periods. Add the two values. Divide the result by 2. The MetaStock formula is: (HHV(H,50)+LLV(L,50))/2\" (V.18:6, PDF p.2)",
            "F|\"If today's close > today's open, the volume is positive. If today's close < today's open, volume is negative. If today's close = today's open, volume is neutral. You then take the seven-day simple moving average of the results.\" MOV(IF(C>O,+V,IF(C<O,-V,0)),7,S); \"This indicator must be positive when the close crosses above the tether line.\" (PDF p.3)",
            "F|\"It is calculated by subtracting the 200-day moving average from the 25-day moving average ... Mov(C,25,S)-Mov(C,200,S) ... The rule is that today's MBO must be greater than yesterday's.\" (PDF p.3)",
            "E|\"When the close crosses above the tether line, a buy signal is triggered.\" (PDF p.2)",
            "X|\"As soon as the price crosses below the tether line, exit your long positions.\" (PDF p.3)"],
        notes: NOTE, claim: None }]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> { reviewed(id).then_some(("scbstfs_tfs", "Strain tether-line TFS (S&C)", "50")) }
pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    reviewed(id).then(|| vec![ParamDef { key: "n", label: "Tether line days (article 50)", def: 50., min: 5., max: 200., step: 1. }])
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from > to || to >= x.r.len() { return None; }
    let n = p.get("n").copied().unwrap_or(50.).round().max(1.) as usize;
    let len = x.c.len();
    let teth: Vec<f64> = (0..len).map(|i| if i + 1 < n { f64::NAN } else {
        let (h, l) = (&x.h[i + 1 - n..=i], &x.l[i + 1 - n..=i]);
        (h.iter().copied().fold(f64::MIN, f64::max) + l.iter().copied().fold(f64::MAX, f64::min)) / 2. }).collect();
    let sv: Vec<f64> = (0..len).map(|i| { let v = x.r[i].volume.unwrap_or(f64::NAN); if x.c[i] > x.o[i] { v } else if x.c[i] < x.o[i] { -v } else { 0. } }).collect();
    let vo = sma(&sv, 7);
    let (m25, m200) = (sma(&x.c, 25), sma(&x.c, 200));
    let mbo: Vec<f64> = m25.iter().zip(&m200).map(|(a, b)| a - b).collect();
    Some(at_close(x, from, to, "close crossed below the tether line", |t, side| {
        if t == 0 || !teth[t - 1].is_finite() { return side; }
        let (up, dn) = (x.c[t] > teth[t] && x.c[t - 1] <= teth[t - 1], x.c[t] < teth[t] && x.c[t - 1] >= teth[t - 1]);
        if side > 0. { return if dn { 0. } else { side }; }
        if up && vo[t] > 0. && mbo[t].is_finite() && mbo[t - 1].is_finite() && mbo[t] > mbo[t - 1] { 1. } else { side }
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn tether_entries_need_volume_and_rising_mbo() {
        let r = super::super::sctraub::tests::bars(2500);
        let x = Ctx::new(&r, &r, &[]);
        let t = run("scbstfs_tfs", &x, 0, r.len() - 1, &Params::new()).unwrap();
        assert!(t.len() > 3 && t.iter().all(|z| z.dir > 0. && z.legs[0].i >= 200));
        let (m25, m200) = (sma(&x.c, 25), sma(&x.c, 200));
        for z in &t { let i = z.legs[0].i; assert!(m25[i] - m200[i] > m25[i - 1] - m200[i - 1]); }
    }
}
