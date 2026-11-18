//! Labunsky, "System for SPX 60 mins" (research share USB HDD 1/MetaStock/@Systems & Indicators/Labunsky System for SP 60 min/System.rar -> System.txt,
//! 0.5 KB, read in full). Book id `lbspx`. Lane CV4 (19 Nov 2026; R4 #339/#340). The text gives one indicator-builder formula "ELAB SPX" (fitted simple
//! averages of C-O) and the tester rules "Enter long: Fml("ELAB SPX") > 0; Enter short: Fml("ELAB SPX") < 0". No results, market bars or exits are
//! printed beyond the title "SPX 60 mins". 60-minute bars are built from the vault's SPX / SPY 1-minute files (intraday_ep::day_bars), one continuous series.
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};

pub const INTRA: &[&str] = &["lbspx_elab60"];
pub fn reviewed(id: &str) -> bool { INTRA.contains(&id) }

const NOTE: &str = "[Declared adaptation, 4 unstated details (R4 #339), UNVERIFIED] The text prints only the formula and the two tester lines. Readings: (1) 'SPX 60 mins' is read as 60-minute bars of the real SPX or SPY 1-minute file, aligned from the session open (9:30-10:30 ... 15:30-16:00, the last bar 30 minutes); (2) positions and the average series carry overnight, one continuous series across days; (3) SPX and SPY (the held minute series of this market) stand in for the index; (4) a formula value of 0 holds the position (the text does not say flat). Engine conventions, not readings: the tester fills at the close of the bar whose formula flips (MetaStock default, no delay), and the averages start with the run's first bar (a comparison with a missing average is false). The 13 average lengths are the printed (fitted) ones, not re-optimised. An opposite 'Enter' reverses; there is no exit but the reversal.";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef { id: "lbspx_elab60", name: "Labunsky ELAB SPX 60-minute: fitted C-O average comparisons, stop and reverse at the bar close", group: "Labunsky: System for SPX 60 mins", session: 0, dir: "Long/Short", status: Status::Live,
        needs_volume: false, needs_vix: false, needs_series: &[], book: "lbspx", rules: &[
            "F|\"If(((Mov(C-O,31,S)>(-1)*Mov(C-O,3,S)) AND (Mov(C-O,3,S)>(-1)*Mov(C-O,74,S))) OR ((Mov(C-O,25,S)>(-1)*Mov(C-O,3,S)) AND (Mov(C-O,7,S)>(-1)*Mov(C-O,46,S))),1, If(((Mov(C-O,29,S)<(-1)*Mov(C-O,23,S)) AND (Mov(C-O,69,S)<(-1)*Mov(C-O,62,S))) OR ((Mov(C-O,62,S)<(-1)*Mov(C-O,12,S)) AND (Mov(C-O,14,S)<(-1)*Mov(C-O,54,S))),-1,0))\" (System.txt)",
            "E|\"Enter long: Fml( \\\"ELAB SPX\\\") > 0   Enter short: Fml( \\\"ELAB SPX\\\") < 0\" (System.txt)",
            "X|No exit other than the opposite Enter (MetaStock system tester: an opposite Enter reverses)"],
        notes: NOTE, claim: None }]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    reviewed(id).then_some(("lbspx_elab60", "Labunsky ELAB SPX 60-minute", "printed lengths"))
}
pub fn params(id: &str) -> Option<Vec<ParamDef>> { let _ = id; None }

fn sma_co(d: &[f64], n: usize) -> Vec<f64> {
    (0..d.len()).map(|k| if k + 1 < n { f64::NAN } else { d[k + 1 - n..=k].iter().sum::<f64>() / n as f64 }).collect()
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, _p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from > to || to >= x.c.len() { return None; }
    if x.minutes.is_none() { return Some(vec![]); }
    let mut bars: Vec<(usize, f64, f64)> = vec![]; // (daily row, close, close - open)
    for i in from..=to {
        if let Some((_, _, _, v)) = crate::intraday_ep::day_bars(x, i, 60) { bars.extend(v.iter().map(|z| (i, z.c, z.c - z.o))); }
    }
    let Some(&(li, lc, _)) = bars.last() else { return Some(vec![]) };
    let d: Vec<f64> = bars.iter().map(|b| b.2).collect();
    let m = |n| sma_co(&d, n);
    let (m3, m7, m12, m14, m23, m25, m29, m31, m46, m54, m62, m69, m74) = (m(3), m(7), m(12), m(14), m(23), m(25), m(29), m(31), m(46), m(54), m(62), m(69), m(74));
    let mut bk = crate::intraday_ep::Book::default();
    for (k, &(i, c, _)) in bars.iter().enumerate() {
        let long = (m31[k] > -m3[k] && m3[k] > -m74[k]) || (m25[k] > -m3[k] && m7[k] > -m46[k]);
        let short = (m29[k] < -m23[k] && m69[k] < -m62[k]) || (m62[k] < -m12[k] && m14[k] < -m54[k]);
        if long { bk.enter(1., i, c, "close", "reverse"); } else if short { bk.enter(-1., i, c, "close", "reverse"); }
    }
    Some(bk.finish(li, lc))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn averages_start_at_n() {
        let s = sma_co(&[1., 2., 3., 4.], 2);
        assert!(s[0].is_nan() && s[1] == 1.5 && s[3] == 3.5);
        let r = super::super::sctraub::tests::bars(30);
        let x = Ctx::new(&r, &r, &[]);
        assert!(run("lbspx_elab60", &x, 0, r.len() - 1, &Params::new()).unwrap().is_empty());
        assert!(run("lbspx_x", &x, 0, 1, &Params::new()).is_none());
    }
}
