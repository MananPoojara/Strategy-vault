//! Lane R2C (13 Nov 2026; authors/titles-1030/R2C.tsv; R4 #297 "up to two unstated details"; reader row authors/b1/BOOKS-GAP.tsv):
//! James Dicks, "Forex Trading Secrets: Trading Strategies for the Forex Market" (McGraw-Hill, 2009; research share 'Trading
//! Knowledge I/James Dicks - Forex Trading Secrets. Trading Strategies for the Forex Market/...pdf'; printed pages cited).
//! Book id `jdfts`. Coded: ch.15 'Basic Moving-Average Crosses' (EMA 5/13, p.263) as a daily declared application (R4 #67, #114).
//! Held (judgement): the 50 EMA + RSI + MACD 'all elements in alignment' entry (p.262-263, early signals read by eye), the gap
//! fade (waits for 'a consolidation pattern or a clear reversal signal', p.264), the daily high-low basket ('The entry will be
//! mostly discretionary', p.266). R4 #305 (1): the same calculation as the live wwwts_ema5_13, so the card is marked
//! "Duplicate of wwwts_ema5_13" (NotTestable, no ADMISSION) and counted once; run() is kept for the record and its test.
//! Independent reference: authors/titles-1030/R2Cw/r2c_ref.py (written from the pages, not from this file).
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};
use super::msws::ema;
use super::tselhs::Book;

pub const REVIEWED: &[&str] = &["jdfts_ema5_13"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const DUP: &str = "Duplicate of wwwts_ema5_13 (R4 #305 (1)): James Dicks' EMA 5/13 cross (p.263) is the same calculation as the live card wwwts_ema5_13 (EMA 5 crossing EMA 13, long or short at the next open, reversing on the opposite cross); counted once, not run as a separate strategy";
const NOTE: &str = "Duplicate of wwwts_ema5_13 (R4 #305 (1)); not run twice. Readings kept for the record: [Adapted: 2 unstated details, R4 #297; unverified] EMA = exponential average of closes (alpha 2/(n+1), started at the first close of the data). A cross up = EMA 5 above EMA 13 at this close and at or below it at the previous close (cross down mirrored); the trade is opened at the next bar's open, as printed. Unstated in the book: (1) the bar size - the book uses it 'for scalping in smaller time frames, such as 1 or 5 minutes'; vault reading: daily bars of indices, ETFs and stocks, US and India, as a declared application of a forex rule (R4 #67, #114). (2) the exit - the book gives only the two entries ('I open a long position ... I open a short position at the open of the next candle') and says the trade 'has to be tightly monitored'; vault reading: each opposite cross closes the trade and opens the other side at the same next open (always in after the first cross). The same EMA 5/13 calculation is on the WWW Trading System card (wwwts_ema5_13, which also exits on an equal close); this card is James Dicks's own printed rule. No performance figures are printed.";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef { id: "jdfts_ema5_13", name: "Dicks: EMA 5 crosses EMA 13, long or short at the next open, reversing on the opposite cross", group: "Basic Moving-Average Crosses (ch.15, p.263)",
        session: 0, dir: "Long/Short", status: Status::NotTestable(DUP), needs_volume: false, needs_vix: false, needs_series: &[], book: "jdfts",
        rules: &["E|\"I just plot a 5-period and a 13-period EMA on a chart and wait for a cross of both to happen. When the 5 EMA crosses upward above the 13 EMA, I open a long position at the open of the next candle; when the 5 EMA crosses downward below the 13 EMA, I open a short position at the open of the next candle.\" (p.263)",
            "X|No exit is printed: \"This is a very short-term strategy that has to be tightly monitored because changes on those small time frames occur very fast.\" (p.263); vault reading: the opposite cross reverses",
            "F|\"The 5/13 EMA cross is a very simple strategy that can be used for scalping in smaller time frames, such as 1 or 5 minutes.\" (p.263)"],
        notes: NOTE, claim: None }]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    (id == "jdfts_ema5_13").then_some(("jdfts_ema5_13", "Dicks EMA 5/13 cross", "Duplicate of wwwts_ema5_13"))
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    reviewed(id).then(|| vec![p("fast", "Fast EMA, bars (book 5)", 5., 1., 50., 1.), p("slow", "Slow EMA, bars (book 13)", 13., 2., 100., 1.)])
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from > to || to >= x.r.len() { return None; }
    let g = |k: &str, d: f64| p.get(k).copied().unwrap_or(d).round().max(1.) as usize;
    let (f, s) = (ema(&x.c, g("fast", 5.)), ema(&x.c, g("slow", 13.)));
    let mut pos = Book::new();
    let mut next = 0.;
    for t in from..=to {
        if next != 0. { pos.enter(next, t, x.o[t], "open"); next = 0.; }
        if t == 0 || t == to { continue; }
        if f[t] > s[t] && f[t - 1] <= s[t - 1] { next = 1.; } else if f[t] < s[t] && f[t - 1] >= s[t - 1] { next = -1.; }
    }
    Some(pos.done(to, x.c[to]))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn dicks_reverses_at_the_open_after_each_cross() {
        let r = super::super::sctraub::tests::bars(2500);
        let x = Ctx::new(&r, &r, &[]);
        let (f, s) = (ema(&x.c, 5), ema(&x.c, 13));
        let t = run("jdfts_ema5_13", &x, 0, r.len() - 1, &Params::new()).unwrap();
        assert!(t.len() > 10 && t.iter().any(|z| z.dir > 0.) && t.iter().any(|z| z.dir < 0.));
        for z in &t { let i = z.legs[0].i; assert_eq!(z.legs[0].px, x.o[i]); assert!(z.dir * (f[i - 1] - s[i - 1]) > 0. && z.dir * (f[i - 2] - s[i - 2]) <= 0.); }
        for w in t.windows(2) { assert_eq!(w[1].legs[0].i, w[0].exit_i); assert_eq!(w[1].dir, -w[0].dir); }
    }
}
// No ADMISSION: duplicate of wwwts_ema5_13 (R4 #305 (1)).
