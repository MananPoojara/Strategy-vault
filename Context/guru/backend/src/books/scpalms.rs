//! Steve Palmquist, "Shorting Moving Average Pullbacks" (MAS), Technical Analysis of Stocks & Commodities V.23:9 (September 2005)
//! pp.42-47 (research share Trading Knowledge I/Stocks & Commodities DVD 11.26 (1982 - 2008) (traders.com)/VOLUMES/V23/C09/185PALM.pdf
//! and the sidebar 185SB.pdf, AIQ EDS code). The short mirror of scpalmap_maps. Book id `scpalms`. Lane K40 (4 Nov 2026, decision
//! #204); independent check authors/titles-1030/K40w/k40_ref.py.
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};
use super::lc3::{hi, lo};
use super::msws::sma;
use super::tselhs::Book;

pub const REVIEWED: &[&str] = &["scpalms_mas"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const NOTE: &str = "[Declared readings] The printed AIQ EDS code: LowerCloses = HiVal([close],5) < LoVal([close],20,20) (the highest of the last 5 closes below the lowest of the 20 closes ending 20 days ago); NotAbovePullbackMA = no close above the 35-day SMA in the last 15 sessions; ClosePullbackMA = SMA35 - close < 1% of the close; setupA = all three. 'Backtest conditions are the setup yesterday and the trigger today': SMAPS1 = setup yesterday and today's low below yesterday's low and yesterday's Stochastic < 80. The article: 'If the stock triggers, a short position is entered at the next day's open. For backtesting purposes, the position was held for three days, then covered at the open': short at the next open after the trigger day, cover at the open three bars after the entry (as scpalmap_maps). The AIQ [Stochastic] settings are not printed (the S&C Traders' Tips for this article used SlowD(14), Stoch(5,3), StochK(14,3), 15/3/3 and a 21-bar %K): labelled reading = the slow %K, a 3-bar SMA of the 14-bar fast %K (options k and slow). The stock filters of MApullbackShort ([Volume ESA] > 2000, i.e. 200,000 shares, and [close] > 15, a fixed-dollar filter on split-adjusted data, decision #166) are not applied, as on scpalmap_maps. Short only, one position, no stop. Article (NASDAQ stocks, AIQ): 1/18/02-10/11/02 82 trades, 63% winners, average winner 4.14%, average loser 2.77%; 6/30/04-8/16/04 28 trades 71%; 9/5/00-12/28/00 21 trades 57% (Figures 3-4); portfolio statistics of a stock universe, not comparable to one instrument. The author tested stocks; runs on indices and ETFs, US and India, are a declared application.";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef { id: "scpalms_mas", name: "Palmquist MAS short: downtrend never above the 35-day SMA bounces within 1%, next day breaks the setup low with Stochastic < 80, short next open, cover at the open 3 bars later", group: "S&C: Shorting Moving Average Pullbacks (Palmquist, V.23:9)", session: 0, dir: "Short", status: Status::Live,
        needs_volume: false, needs_vix: false, needs_series: &[], book: "scpalms", rules: &[
            "F|\"LowerCloses if HiVal([close],5) < LoVal([close],20,20). PullbackMAS is simpleavg([close],35). CloseAbovePullbackMA if [close] > PullbackMAS. NotAbovePullbackMA if CountOf(CloseAbovePullbackMA,15) <1. ClosePullbackMA if ( PullbackMAS -[close] ) < 0.01 * [close].\" (sidebar EDS code)",
            "E|\"MApullbackShortYesterday if valrule(MApullbackShort,1). ShortTrigger if [low] < val([low],1). SMAPS1 if MApullbackShortYesterday and ShortTrigger and StochOK. StochOK if val([Stochastic],1) < 80.\" (sidebar); \"If the stock triggers, a short position is entered at the next day's open.\" (185PALM PDF p.2)",
            "X|\"For backtesting purposes, the position was held for three days, then covered at the open.\" (185PALM PDF p.2)"],
        notes: NOTE, claim: None }]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> { reviewed(id).then_some(("scpalms_mas", "Palmquist MAS short (S&C)", "35-day SMA, 1%, 3-bar hold")) }

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    reviewed(id).then(|| vec![p("k", "Stochastic %K bars (not printed; reading 14)", 14., 2., 50., 1.),
        p("slow", "Stochastic %K smoothing, bars (reading 3; 1 = fast %K)", 3., 1., 10., 1.)])
}

/// Slow %K: an s-bar SMA of the n-bar fast %K = 100 (C - LL) / (HH - LL); a flat range gives 50.
pub fn slow_k(x: &Ctx, n: usize, s: usize) -> Vec<f64> {
    let fast: Vec<f64> = (0..x.c.len()).map(|t| if t + 1 < n { f64::NAN } else {
        let (hh, ll) = (hi(&x.h, t + 1 - n, t), lo(&x.l, t + 1 - n, t));
        if hh > ll { 100. * (x.c[t] - ll) / (hh - ll) } else { 50. } }).collect();
    super::lc3::sma(&fast, s)
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.r.len() { return None; }
    let g = |k: &str, d: f64| p.get(k).copied().unwrap_or(d).round().max(1.) as usize;
    let st = slow_k(x, g("k", 14.), g("slow", 3.));
    let m = sma(&x.c, 35);
    let setup = |t: usize| t >= 39 && m[t - 14].is_finite() && hi(&x.c, t - 4, t) < lo(&x.c, t - 39, t - 20)
        && (t - 14..=t).all(|j| !(x.c[j] > m[j])) && m[t] - x.c[t] < 0.01 * x.c[t];
    let mut pos = Book::new();
    let mut pend = false;
    for t in from..=to {
        if let Some(e) = pos.entry_bar() { if t - e == 3 { pos.exit(t, x.o[t], "3-bar hold"); } }
        if pend { pend = false; if pos.side() == 0. { pos.enter(-1., t, x.o[t], "open"); } }
        if t < to && t >= 1 && pos.side() == 0. && setup(t - 1) && x.l[t] < x.l[t - 1] && st[t - 1] < 80. { pend = true; }
    }
    Some(pos.done(to, x.c[to]))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn mas_short_three_bar_hold() {
        let r = super::super::sctraub::tests::bars(3000);
        let d = |i: usize| 400. - 0.12 * i as f64; // a falling market
        let r: Vec<_> = r.iter().enumerate().map(|(i, b)| crate::Row { open: b.open + d(i), high: b.high + d(i), low: b.low + d(i), close: b.close + d(i), ..b.clone() }).collect();
        let x = Ctx::new(&r, &r, &[]);
        let t = run("scpalms_mas", &x, 0, r.len() - 1, &Params::new()).unwrap();
        assert!(!t.is_empty() && t.iter().all(|z| z.dir < 0. && (z.open || z.exit_i - z.legs[0].i == 3)));
        assert!(t.iter().all(|z| { let i = z.legs[0].i; z.legs[0].px == x.o[i] && x.l[i - 1] < x.l[i - 2] }));
    }
}

// Instrument admission: S&C Palmquist MAS short 2026-11-04 (titles lane K40).
pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book == "scpalms" && reviewed(def.id),
    label: "Steve Palmquist tested the MAS short system on US stocks with AIQ (2000-2005 periods). Runs on indices, ETFs and stocks, US and India, are a declared application.",
    basis: "Technical Analysis of Stocks & Commodities V.23:9 pp.42-47 (Shorting Moving Average Pullbacks), AIQ EDS sidebar; independent check authors/titles-1030/K40w/k40_ref.py",
};
