//! Darren Kimoto, "Currency Trader Online" (InvesTools 2006 Mega Event seminar manual), "System Development: Backtesting" pp.71-77: the MACD system (21-period EMA, 8-17-9 MACD) with a printed
//! 11-trade backtest (May 2002 - May 2003, quotes near 1.46-1.64, a pound-dollar series). Research share `USB HDD 1/TRADING KNOWLEDGE-2/_InvesTools.com/2006 Mega Event Manuals/07 - Currency Trader Online by Darren Kimoto.pdf`.
//! Book id `kmcd`. Lane K1 (25 Nov 2026, R4 #398 tier 6): run on the Norgate British pound continuous contract (&6B), signals at a close filled at the next close (R4 #374). Reference authors/code-1123/ref/kmcd_ref.py.
use super::tselhs::Book;
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};

const ID: &str = "kmcd_macd_ema21";
const NOTE: &str = "[Declared adaptation, R4 #340 / #398] Kimoto, Currency Trader Online (InvesTools 2006) pp.71-77, 'MACD System': 21-period exponential moving average and an 8-17-9 MACD; go long if the moving average is uptrending and the MACD has a positive cross, go short if downtrending and a negative cross; exit when the MACD crosses back over. The manual backtests it on a pound-dollar daily chart (printed: 11 long trades, signals 29 May 2002 to 13 May 2003, entry the day after the signal, pips per trade). Run on the Norgate British pound continuous contract. Unstated details (4): (1) the signal is read at a close and filled at the NEXT close (R4 #374; the manual enters 'the day after the signal' at a price it does not define); (2) 'uptrending moving average' = the 21-period EMA above its previous value (downtrending: below); (3) the MACD 'cross' is the MACD line crossing its 9-period signal line (the histogram changing sign), the 8-17-9 EMAs started at the first close; (4) the opposite cross is the exit and the book is flat afterwards unless the opposite entry's own trend condition holds on that same bar (then it reverses).";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef { id: ID, name: "Kimoto MACD system: 21-EMA trend with an 8-17-9 MACD signal-line cross, exit on the cross back (British pound futures)", group: "Currency Trader Online (Darren Kimoto)", session: 0, dir: "Long/Short",
        status: Status::Live, needs_volume: false, needs_vix: false, needs_series: &[], book: "kmcd", rules: &[
            "F|\"Chart Setup ~ 21-period exponential moving average ~ 8-17-9 MACD indicator\" (p.73)",
            "E|\"Go long if: Uptrending moving average; Positive cross on the MACD indicator. Go short if: Downtrending moving average; Negative cross on the MACD indicator\" (p.73)",
            "X|\"Exit when MACD indicator crosses back over\" (p.73)"], notes: NOTE, claim: None }]
}
pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> { (id == ID).then_some(("kmcd", "Kimoto MACD system", "21-EMA + 8-17-9 MACD")) }
pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    (id == ID).then(|| vec![p("ma", "Trend EMA (21)", 21., 5., 100., 1.), p("fast", "MACD fast (8)", 8., 2., 30., 1.), p("slow", "MACD slow (17)", 17., 5., 60., 1.), p("sig", "MACD signal (9)", 9., 2., 30., 1.)])
}
fn ema(c: &[f64], n: usize) -> Vec<f64> {
    let a = 2.0 / (n as f64 + 1.0);
    let mut o: Vec<f64> = Vec::with_capacity(c.len());
    for (i, v) in c.iter().enumerate() { o.push(if i == 0 { *v } else { a * v + (1.0 - a) * o[i - 1] }); }
    o
}
pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if id != ID || from >= to || to >= x.r.len() { return None; }
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| params(id).unwrap().iter().find(|d| d.key == k).unwrap().def).round().max(1.0) as usize;
    let c = &x.c;
    let (e, f, s) = (ema(c, g("ma")), ema(c, g("fast")), ema(c, g("slow")));
    let macd: Vec<f64> = f.iter().zip(&s).map(|(a, b)| a - b).collect();
    let sig = ema(&macd, g("sig"));
    let warm = g("slow").max(g("ma")) + g("sig");
    let mut pos = Book::new();
    let mut pend = 0.0f64; // +1 / -1 reverse into, 2 = exit only
    for t in 0..=to {
        if t >= from && pend != 0.0 { if pend == 2.0 { pos.exit(t, c[t], "MACD crossed back"); } else { pos.enter(pend, t, c[t], "close"); } pend = 0.0; }
        if t < warm || t < from || t >= to { continue; }
        let (up, dn) = (macd[t] > sig[t] && macd[t - 1] <= sig[t - 1], macd[t] < sig[t] && macd[t - 1] >= sig[t - 1]);
        let (rising, falling) = (e[t] > e[t - 1], e[t] < e[t - 1]);
        if up { pend = if rising { 1.0 } else if pos.side() < 0.0 { 2.0 } else { 0.0 }; }
        else if dn { pend = if falling { -1.0 } else if pos.side() > 0.0 { 2.0 } else { 0.0 }; }
        if pend == pos.side() { pend = 0.0; }
    }
    Some(pos.done(to, c[to]))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn trades_both_sides_on_the_home_study_bars() {
        let r = super::super::sctraub::tests::bars(1500);
        let x = Ctx::new(&r, &r, &[]);
        let t = run(ID, &x, 0, r.len() - 1, &Params::new()).unwrap();
        assert!(t.len() > 5 && t.iter().any(|z| z.dir > 0.) && t.iter().any(|z| z.dir < 0.));
        assert!(params(ID).is_some() && params("x").is_none() && family_of(ID).is_some());
    }
}
