//! Alex Saitta, "Range Breakout Trading in Treasury Bonds", Technical Analysis of Stocks & Commodities V.13:4 (1995) pp.149-152
//! (research share Trading Knowledge I/Stocks & Commodities DVD 11.26 (1982 - 2008) (traders.com)/VOLUMES/V13/C04/RANGE.PDF).
//! Book id `scasrb`. Lane PA2 (13 Nov 2026); independent check authors/pa2/w/pa2_fut_ref.py. Runs on the Norgate T-bond continuous
//! contract only (futures::CARDS_PA2, R4 #293 (1)).
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};
use super::tselhs::Book;

pub const REVIEWED: &[&str] = &["scasrb_breakout"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const NOTE: &str = "[Adapted: 2 unstated details, R4 #297; unverified] Rule 1 (tight, narrow and lengthy range): the last five days' highest high and lowest low; the 'price ticks where the market did not trade' are counted day by day inside that height (each day's height minus its own range, in ticks; the only count consistent with the article's Figure 1, 22 X's for a 40-tick range, since one union of ranges could never reach 200%), and the range qualifies when that count is under 200% of the height. Rule 2: a close above the range's high (below its low) on a day whose high-low range is more than 125% of the average range of the previous eight days -> long (short) at that close. Rule 3: a long is held until a close below the 8-day simple average of the lows, a short until a close above the 10-day simple average of the highs (both including the day). Unstated in the article: (1) 'The parameters of this trading range remain fixed until a breakout occurs' - vault reading: a qualifying range is kept until the first close outside it (whether or not that day expands enough to trade), then the next qualifying five days set a new one; (2) signals while a position is open - vault reading: ignored (one contract at a time). One tick = 1/32 (R4 #57). Fills at closes only, so no trade depends on the order inside a day (#240). Norgate continuous futures, ends 2020-09-11; roll method not stated by the vendor; signals on the back-adjusted series, trades priced in the contract held at entry (R4 #293, #301).";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef { id: "scasrb_breakout", name: "Saitta T-bond range breakout: close outside a tight five-day range on a 125% range-expansion day; out on the 8-day low / 10-day high averages (T-bond futures)", group: "S&C: range breakout trading in Treasury bonds (Saitta, V.13:4)", session: 0, dir: "Long/Short", status: Status::Live,
        needs_volume: false, needs_vix: false, needs_series: &[], book: "scasrb", rules: &[
            "F|\"During a period of consecutive days, a tight and narrow trading range occurs when the number of price ticks where the market did not trade is less than 200% of the height of the range ... In Figure 1, the number of price ticks where the market did not trade is equal to 22. The height of this trading range is 40 ticks\" (V.13:4, PDF p.2); \"I will define a lengthy price range as five consecutive days.\" (p.2)",
            "F|\"a drastic change of market psychology as occurring when the size of one day's price range expands to more than 125% of the average daily price range of the last eight days\" (p.2)",
            "E|\"A long position was initiated when the market closed above the highest price of a tight, narrow and lengthy trading range and the size of that day's price range expanded to more than 125% of the average daily price range of the last eight days.\" (p.3)",
            "X|\"The long position was then held until the market closed below the eight-day moving average of the lows ... The short position was then held until the market closed above the 10-day moving average of the highs.\" (p.3)",
            "F|\"The parameters of this trading range remain fixed until a breakout occurs.\" (Figure 1 caption)"],
        notes: NOTE, claim: None }]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> { reviewed(id).then_some(("scasrb_breakout", "Saitta T-bond range breakout (S&C)", "5/8")) }
pub fn params(id: &str) -> Option<Vec<ParamDef>> { reviewed(id).then(Vec::new) }

/// The tight-range test of the five bars ending at t (rule 1A/1B).
pub fn tight(x: &Ctx, t: usize, tick: f64) -> Option<(f64, f64)> {
    if t < 4 { return None; }
    let (hh, ll) = (x.h[t - 4..=t].iter().copied().fold(f64::MIN, f64::max), x.l[t - 4..=t].iter().copied().fold(f64::MAX, f64::min));
    let height = ((hh - ll) / tick).round();
    let untraded: f64 = (t - 4..=t).map(|k| height - ((x.h[k] - x.l[k]) / tick).round()).sum();
    (height > 0. && untraded < 2. * height).then_some((hh, ll))
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from > to || to >= x.r.len() { let _ = p; return None; }
    let tick = x.market_tick();
    let n = x.c.len();
    let rng: Vec<f64> = (0..n).map(|i| x.h[i] - x.l[i]).collect();
    let avg = |v: &[f64], t: usize, k: usize| -> f64 { if t + 1 < k { f64::NAN } else { v[t + 1 - k..=t].iter().sum::<f64>() / k as f64 } };
    let mut pos = Book::new();
    let mut box_: Option<(f64, f64)> = None;
    for t in 0..=to {
        // breakout of the fixed range (any close outside ends it)
        let mut signal = 0.;
        if let Some((hh, ll)) = box_ {
            if x.c[t] > hh || x.c[t] < ll {
                let a8 = if t >= 8 { avg(&rng, t - 1, 8) } else { f64::NAN };
                if a8.is_finite() && rng[t] > 1.25 * a8 { signal = if x.c[t] > hh { 1. } else { -1. }; }
                box_ = None;
            }
        }
        if box_.is_none() && signal == 0. { if let Some(b) = tight(x, t, tick) { box_ = Some(b); } }
        if t < from { continue; }
        let s = pos.side();
        if s > 0. && x.c[t] < avg(&x.l, t, 8) { pos.exit(t, x.c[t], "close below the 8-day average of the lows"); }
        else if s < 0. && x.c[t] > avg(&x.h, t, 10) { pos.exit(t, x.c[t], "close above the 10-day average of the highs"); }
        else if s == 0. && signal != 0. && t < to { pos.enter(signal, t, x.c[t], "close"); }
    }
    Some(pos.done(to, x.c[to]))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn tight_range_and_breakouts() {
        let r = super::super::sctraub::tests::bars(2500);
        let x = Ctx::new(&r, &r, &[]);
        assert!((4..r.len()).any(|t| tight(&x, t, 0.01).is_some()));
        let t = run("scasrb_breakout", &x, 0, r.len() - 1, &Params::new()).unwrap();
        for z in &t { let i = z.legs[0].i; assert!(x.h[i] - x.l[i] > 1.25 * (i - 8..i).map(|k| x.h[k] - x.l[k]).sum::<f64>() / 8.); }
    }
}
