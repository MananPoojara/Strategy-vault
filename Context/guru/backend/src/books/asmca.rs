//! Arthur Sklarew, "Techniques of a Professional Commodity Chart Analyst" (Commodity Research Bureau, 1980; research share
//! USB HDD 2/tRADING KNO-2/Artur Sklarew - Techniques of a Professional Commodity Chart Analyst/). Book id `asmca`. One rule:
//! the advanced moving-average channel of daily highs and lows, stop-and-reverse (ch.7, printed p.115-119). Lane K8 (titles
//! loop, 1 Nov 2026); independent check authors/titles-1030/K8w/k8_ref.py (header).
use crate::connors::{sma, Ctx, ParamDef, Params, Status, StrategyDef, Trade};
use super::rbtie::mk;

/// Admitted ids (authors/titles-1030/K8w/k8_ref.py (header)).
pub const REVIEWED: &[&str] = &["asmca_channel"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }
/// A price exactly one tick beyond the line counts ('a single price tick'): compare with a 1e-9 allowance so that rounding
/// in the averages (105.83 - 0.01 = 105.81999999999998) cannot decide a touch.
const EPS: f64 = 1e-9;

const NOTE: &str = "[Declared readings] The book's worked example is the default: 9-day simple averages of the daily highs and of the daily lows, advanced 3 days ('each day's price action must be judged in relation to the 9-day moving average calculated three days ago'); the author ties the length to each market's minor cycle, so other lengths are options, advanced by the book's square-root table (2-4 days -> 2, 5-9 -> 3, 10-16 -> 4, 17-25 -> 5, i.e. the square root rounded up). In a long position only the lower line counts, in a short only the upper line. A reversal needs the price to go one tick beyond the line and also beyond the previous day's range ('valid only if the price has also moved beyond the previous day's range'; 'a single price tick ... should be considered a valid signal'): a sell stop at the lower of the two minus one tick (0.01 US, 0.05 NSE), a buy stop at the higher of the two plus one tick; a gap through the stop fills at the open. The first position is taken the same way on whichever side is hit first (none on a day that hits both). Always in the market after that. The author describes these techniques without recommending any of them ('It is hoped ... that the ideas presented here may prove useful as a foundation for traders to set up their own trend-following methods'). Commodity futures in the book; ETFs, US/India indices and stocks are a declared application. No printed results for this method.";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef { id: "asmca_channel", name: "Advanced moving-average channel: 9-day averages of highs and lows advanced 3 days, stop-and-reverse beyond the line and yesterday's range", group: "Sklarew moving-average channel",
        session: 0, dir: "Long/Short", status: Status::Live, needs_volume: false, needs_vix: false, needs_series: &[], book: "asmca", rules: &[
            "F|\"a twin set of moving averages, one of the daily highs and one of the daily lows, with all moving averages advanced\"; \"advanced the number of days equal to the square root of the time period ... a 9-day moving average channel should be advanced three days\" (p.116-117)",
            "E|\"In an uptrend, only the lower moving average is utilized; in a downtrend, the upper line is the one that is used, giving a reversal signal when penetrated by the price in the course of the day's trading\" (p.116)",
            "X|\"a penetration of a moving average channel line should be considered valid only if the price has also moved beyond the previous day's range in the direction of the penetration\"; \"by a single price tick (the minimum fluctuation)\" (p.119)"],
        notes: NOTE, claim: None }]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    (id == "asmca_channel").then_some(("asmca_channel", "Sklarew advanced MA channel", "9 days advanced 3"))
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    (id == "asmca_channel").then(|| vec![p("n", "Days in the averages of highs and lows (9 = the book's example; advanced by the square root, rounded up)", 9., 2., 25., 1.)])
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.r.len() { return None; }
    let n = p.get("n").copied().unwrap_or(9.).round().max(1.) as usize;
    let adv = (n as f64).sqrt().ceil() as usize;
    let tick = x.market_tick();
    let (mh, ml) = (sma(&x.h, n), sma(&x.l, n));
    let (mut out, mut pos) = (vec![], None::<(f64, usize, f64)>);
    for i in from.max(adv + 1)..=to {
        let (up, lo) = (mh[i - adv], ml[i - adv]);
        if !(up.is_finite() && lo.is_finite()) { continue; }
        let (bs, ss) = (up.max(x.h[i - 1]) + tick, lo.min(x.l[i - 1]) - tick);
        match pos {
            None => {
                let (hb, hs) = (x.h[i] >= bs - EPS, x.l[i] <= ss + EPS);
                if hb && !hs { pos = Some((1., i, x.o[i].max(bs))); } else if hs && !hb { pos = Some((-1., i, x.o[i].min(ss))); }
            }
            Some((d, e, ep)) => {
                let px = if d > 0. && x.l[i] <= ss + EPS { Some(x.o[i].min(ss)) } else if d < 0. && x.h[i] >= bs - EPS { Some(x.o[i].max(bs)) } else { None };
                if let Some(px) = px {
                    out.push(mk(d, e, ep, "stop", i, px, "channel line penetrated", false, i));
                    pos = Some((-d, i, px));
                }
            }
        }
    }
    if let Some((d, e, ep)) = pos { out.push(mk(d, e, ep, "stop", to, x.c[to], "open", true, to)); }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;
    #[test]
    fn advance_is_the_square_root_rounded_up() {
        assert_eq!([4usize, 5, 9, 10, 16, 17].map(|n| (n as f64).sqrt().ceil() as usize), [2, 3, 3, 4, 4, 5]);
    }
    #[test]
    fn reverses_beyond_line_and_previous_range() {
        // flat 100 (highs 101, lows 99), then a rally bar to 104 and a drop bar to 96; n = 2, advance 2
        let hl = [(101., 99.), (101., 99.), (101., 99.), (101., 99.), (104., 100.), (103., 96.)];
        let r: Vec<Row> = hl.iter().enumerate().map(|(i, &(h, l))| Row { ticker: "T".into(), date: format!("2001-01-{:02}", i + 1), open: (h + l) / 2., high: h, low: l, close: (h + l) / 2., volume: None }).collect();
        let x = Ctx::new(&r, &r, &[]);
        let p: Params = [("n".to_string(), 2.)].into_iter().collect();
        let t = run("asmca_channel", &x, 0, r.len() - 1, &p).unwrap();
        // bar 4: buy stop max(101, 101) + 0.01 = 101.01 -> long at 102 (open above the stop); bar 5: sell stop min(99, 100) - 0.01
        assert_eq!(t.iter().map(|z| (z.dir, z.legs[0].i, z.legs[0].px, z.exit_i)).collect::<Vec<_>>(), vec![(1., 4, 102., 5), (-1., 5, 98.99, 5)]);
    }
}

// Instrument admission: Arthur Sklarew (moving-average channel) 2026-11-01 (titles lane K8).
// Admission only for the module REVIEWED lists (authors/titles-1030/K8w/k8_ref.py (header)).
// Runs on actual indices, ETFs and stocks, US and India (index_etf_or_stock, nifty50 basket, declared application).
pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book == "asmca" && crate::books::asmca::reviewed(def.id),
    label: "A commodity-futures charting technique with no printed results; ETFs, US/India indices and stocks are a declared application.",
    basis: "Arthur Sklarew, Techniques of a Professional Commodity Chart Analyst (CRB 1980) ch.7 p.115-119; independent check authors/titles-1030/K8w/k8_ref.py",
};
