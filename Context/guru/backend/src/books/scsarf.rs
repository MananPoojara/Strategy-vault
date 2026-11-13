//! Alex Saitta, "Reversal Formations: Predictive Power?", Technical Analysis of Stocks & Commodities V.16:11 (1998) pp.513-517
//! (research share Trading Knowledge I/Stocks & Commodities DVD 11.26 (1982 - 2008) (traders.com)/VOLUMES/V16/C11/084REV.pdf).
//! Book id `scsarf`: the T-bond head-and-shoulders and double top/bottom tests. Lane PG (13 Nov 2026, handover of lane PA2's
//! 'not done' row); independent check authors/pg/w/pg_fut_ref.py. Runs on the Norgate T-bond continuous contract only
//! (futures::CARDS_PG, R4 #293 (1)).
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};
use super::tselhs::Book;

pub const REVIEWED: &[&str] = &["scsarf_hs", "scsarf_dt"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

macro_rules! fut { () => { " Swings (reactionary highs and lows) are found on the daily closes, as the article's test 'used closing data': a close more than 32 ticks (1 point) below the highest close since the last reactionary low makes that highest close a reactionary high, confirmed on that day; the mirror for lows; the first swing is whichever comes first; equal closes keep the earlier day. 'Days' are trading days. Runs only on the Norgate T-bond continuous contract (the article's 'continuous contract T-bond adjusted for the rollover of each contract', 1977-1998): swings and signals on the back-adjusted series, each trade priced in the contract held at entry (R4 #293, #301). Norgate continuous futures, ends 2020-09-11; roll method not stated by the vendor. No costs, as in the article. Entries and exits at closes, so no trade depends on the order inside a day (#240)." } }
const NOTE_HS: &str = concat!("[Adapted: 1 unstated detail, R4 #297; unverified] Head-and-shoulders top: four consecutive reactionary highs H0, SL, H, SR with the reactionary lows L0 (before SL), AL (between SL and H) and AR (between H and SR). H above SL and SR; SL above H0 and AL above L0 (after an uptrend); SR above and AR below the midpoint of SL and AL, and SL above and AL below the midpoint of SR and AR (vertical symmetry); days SL->H between 0.4 and 2.5 times days H->SR (horizontal symmetry). Neckline = the line through AL and AR. From the day SR is confirmed, the first close below the neckline (at that day's neckline value), within as many days after SR as from SL to SR and with no close above H in between, is the entry: short at that close. Height = H minus the neckline at H's day; objective = the neckline at the breakout minus the height (Figure 3: neckline 82-13, height 2-30, objective 85-11). Out at the first close below the objective (profit) or above SR (loss). Bottoms are the mirror (long). Unstated: a breakout that comes while a trade is open, or on its exit day - vault reading: not taken (one contract); the latest completed pattern is the one watched. #168 against Figure 4 (1977-1998): 16 trades, 11 winners (69%), average +$1,578.", fut!());
const NOTE_DT: &str = concat!("[Adapted: 2 unstated details, R4 #297; unverified] Double top: three consecutive reactionary highs H1, H2, H3 and lows L1 (before H1), L2 (between H1 and H2) and L3 (between H2 and H3, the base). H2 above H1, L3 above L2 and L2 above L1 (after an uptrend); |H2 - H3| under 20% of H2 - L3 (vertical symmetry); days H2->L3 between 0.4 and 2.5 times days L3->H3 (horizontal symmetry). Base = the line through L3 parallel to the line through H1 and H2 (Figure 5 caption). Short at the close of the breakout day (close below the base); height = the average of H2 and H3 minus the base at the breakout; objective = base minus height; out at the first close below the objective (profit) or above the base plus half the height (loss; Figure 6: base 108-16, height 2-18, objective 105-30, stop 109-25). Bottoms are the mirror (long). Unstated: (1) 'an immediate decline through the base' - vault reading: the breakout must come in the same down swing that confirms H3, before the next reactionary low is confirmed; (2) a breakout that comes while a trade is open, or on its exit day - vault reading: not taken (one contract); the latest completed pattern is the one watched. #168 against Figure 7 (1977-1998): 11 trades, 6 winners (55%), average +$556.", fut!());

fn card(id: &'static str, name: &'static str, rules: &'static [&'static str], notes: &'static str) -> StrategyDef {
    StrategyDef { id, name, group: "S&C: reversal formations in T-bond futures (Saitta, V.16:11)", session: 0, dir: "Long/Short", status: Status::Live,
        needs_volume: false, needs_vix: false, needs_series: &[], book: "scsarf", rules, notes, claim: None }
}

pub fn catalogue() -> Vec<StrategyDef> {
    vec![
        card("scsarf_hs", "Saitta head-and-shoulders (T-bond futures): 32-tick swings with symmetry rules; enter on the neckline close, out at the objective or beyond the right shoulder", &[
            "F|\"Reactionary high: A reactionary high is a local maximum price, which is more than 1 point or 32 ticks above the previous reactionary low. Reactionary low: A reactionary low is a local minimum price, which is more than 32 ticks below the previous reactionary high.\" (V.16:11, PDF p.1)",
            "F|\"To qualify as a head-and-shoulders top, four consecutive reactionary highs are required. The last three reactionary highs, respectively, are the pattern's left shoulder (SL), head (H) and right shoulder (SR). The head must be above the left and right shoulders\"; \"the left shoulder must be above the prior reactionary high. In addition, the pattern's left armpit (AL) must be above the previous reactionary low.\" (PDF p.1-2)",
            "F|\"the right shoulder must be above and the right armpit (AR) must be below the midpoint between the left shoulder and left armpit. Likewise, the left shoulder must be above and the left armpit must be below the midpoint between the right shoulder and right armpit\"; \"the number of days from the left shoulder to the head must be no more than 2.5 and no less than 0.4 times the number of days between the head and the right shoulder.\" (PDF p.2)",
            "F|\"This drop from the right shoulder through the neckline must occur in a period that is no longer than the period between the left and right shoulders. In addition, during this period the market must not rise above the head. The price objective of the top is equal to the height of the pattern projected downward from the breakout level.\" (PDF p.2)",
            "E|\"In the case of a head-and-shoulders top, a short position is taken at the settlement price the first day the T-bond closes below the neckline.\" (PDF p.2)",
            "X|\"A profit is taken the first day the market closes below the price objective or a loss is booked the first day it closes above the right shoulder. In the case of a head-and-shoulders bottom, the trading strategy is the inverse.\" (PDF p.2)",
            "F|Figure 4 (1977-1998, one contract, no costs): average profit per trade $1,578; 69% winners; 11 winners, 5 losers; average winner $3,491, average loser $2,631"], NOTE_HS),
        card("scsarf_dt", "Saitta double top/bottom (T-bond futures): 32-tick swings with symmetry rules; enter on the base breakout close, out at the objective or half the height back", &[
            "F|\"To qualify as a double top, three consecutive reactionary highs and reactionary lows are required. The last two reactionary highs (H2 and H3) are the pattern's two peaks. The last low (L3) is what is known as the pattern's base\"; \"its first peak (H2) must be above the first reactionary high (H1). In addition, the base or third reactionary low (L3) must be above the second reactionary low (L2) and the second low must be above the first reactionary low (L1).\" (PDF p.3-4)",
            "F|\"the price difference between the peaks must be less than 20% of the vertical distance from the first peak (H2) to the base (L3)\"; \"the number of days from the first peak (H2) to the base of the pattern (L3) must be no more than 2.5 and no less than 0.4 times the number of days from the base (L3) to the second peak (H3).\" (PDF p.4); base = \"a line drawn through L3, parallel to a line through H1 and H2\" (Figure 5 caption)",
            "F|\"After the second peak has formed, an immediate decline through the base is referred to as a breakout\"; \"The height of the pattern is equal to the vertical distance from the base of the pattern to the average price of the two peaks. The price objective of the top is equal to the height of the pattern projected downward from the base.\" (PDF p.4)",
            "E|\"In the case of a double top, a short position is taken at the settlement price the day the breakout occurs.\" (PDF p.4)",
            "X|\"A profit is taken the first day the market closes below the price objective or a loss is booked the first day the market closes above the midpoint of the height of the pattern. In the case of a double bottom, the trading strategy is the inverse.\" (PDF p.4)",
            "F|Figure 7 (1977-1998): 11 trades, average profit per trade $556, 55% winners (6 / 5), average winner $2,589, average loser $1,881"], NOTE_DT),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    reviewed(id).then_some(("scsarf_hs", "Saitta T-bond reversal formations (S&C)", if id == "scsarf_hs" { "head-and-shoulders" } else { "double top/bottom" }))
}
pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    reviewed(id).then(|| vec![ParamDef { key: "swing", label: "Swing size in points (article 1 point = 32 ticks)", def: 1., min: 0.25, max: 5., step: 0.25 }])
}

/// A confirmed swing: (kind +1 high / -1 low, bar of the extreme, close there, bar where it was confirmed).
pub type Pivot = (f64, usize, f64, usize);

/// Reactionary highs and lows on the closes (swing > `th` points), in time order.
pub fn pivots(c: &[f64], th: f64) -> Vec<Pivot> {
    let mut out = vec![];
    if c.is_empty() { return out; }
    let (mut hi, mut ihi, mut lo, mut ilo, mut dir) = (c[0], 0, c[0], 0, 0);
    for t in 1..c.len() {
        if dir >= 0 && c[t] > hi { hi = c[t]; ihi = t; }
        if dir <= 0 && c[t] < lo { lo = c[t]; ilo = t; }
        if dir >= 0 && c[t] < hi - th { out.push((1., ihi, hi, t)); dir = -1; lo = c[t]; ilo = t; }
        else if dir <= 0 && c[t] > lo + th { out.push((-1., ilo, lo, t)); dir = 1; hi = c[t]; ihi = t; }
    }
    out
}

/// A completed pattern waiting for its breakout: trade direction, first bar watched (the confirmation of the last swing),
/// the neckline / base (value, bar, slope), the head-to-neckline height (h&s) or the peaks' average (dt), the last bar a h&s
/// breakout may come on, the head a h&s close must not pass, the h&s stop (right shoulder). The latest completed pattern is
/// the one watched; a dt watch ends at the next confirmed swing.
struct Setup { dir: f64, from: usize, line: (f64, usize, f64), height: f64, last: usize, guard: f64, stop: f64, kind_dt: bool }

fn line_at(l: (f64, usize, f64), t: usize) -> f64 { l.0 + l.2 * (t as f64 - l.1 as f64) }

/// Patterns completed by the swing at index k of `pv` (a top when pv[k] is a high, a bottom when it is a low).
fn setups(pv: &[Pivot], k: usize, dt: bool) -> Option<Setup> {
    let s = pv[k].0; // +1: the last swing is a high -> top (short); -1: bottom (long)
    let v = |j: usize| s * pv[j].2; // values mirrored so that a top's logic serves both
    let b = |j: usize| pv[j].1 as f64;
    if !dt {
        // H0 L0 SL AL H AR SR = k-6 .. k
        if k < 6 { return None; }
        let (h0, l0, sl, al, h, ar, sr) = (k - 6, k - 5, k - 4, k - 3, k - 2, k - 1, k);
        let ok = v(h) > v(sl) && v(h) > v(sr) && v(sl) > v(h0) && v(al) > v(l0)
            && v(sr) > (v(sl) + v(al)) / 2. && v(ar) < (v(sl) + v(al)) / 2.
            && v(sl) > (v(sr) + v(ar)) / 2. && v(al) < (v(sr) + v(ar)) / 2.
            && b(h) - b(sl) <= 2.5 * (b(sr) - b(h)) && b(h) - b(sl) >= 0.4 * (b(sr) - b(h));
        if !ok { return None; }
        let slope = (pv[ar].2 - pv[al].2) / (b(ar) - b(al));
        let line = (pv[al].2, pv[al].1, slope);
        let height = (pv[h].2 - line_at(line, pv[h].1)).abs();
        Some(Setup { dir: -s, from: pv[k].3, line, height, last: pv[sr].1 + (pv[sr].1 - pv[sl].1), guard: pv[h].2, stop: pv[sr].2, kind_dt: false })
    } else {
        // L1 H1 L2 H2 L3 H3 = k-5 .. k
        if k < 5 { return None; }
        let (l1, h1, l2, h2, l3, h3) = (k - 5, k - 4, k - 3, k - 2, k - 1, k);
        let ok = v(h2) > v(h1) && v(l3) > v(l2) && v(l2) > v(l1)
            && (v(h2) - v(h3)).abs() < 0.2 * (v(h2) - v(l3))
            && b(l3) - b(h2) <= 2.5 * (b(h3) - b(l3)) && b(l3) - b(h2) >= 0.4 * (b(h3) - b(l3));
        if !ok { return None; }
        let slope = (pv[h2].2 - pv[h1].2) / (b(h2) - b(h1));
        let line = (pv[l3].2, pv[l3].1, slope);
        Some(Setup { dir: -s, from: pv[k].3, line, height: (pv[h2].2 + pv[h3].2) / 2., last: usize::MAX, guard: f64::NAN, stop: f64::NAN, kind_dt: true })
    }
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from > to || to >= x.c.len() { return None; }
    let dt = id == "scsarf_dt";
    let pv = pivots(&x.c[..=to], p.get("swing").copied().unwrap_or(1.));
    let mut pos = Book::new();
    let (mut obj, mut stp) = (f64::NAN, f64::NAN);
    let mut watch: Option<Setup> = None;
    let mut k = 0; // next swing to examine
    for t in 0..=to {
        while k < pv.len() && pv[k].3 == t {
            // a new swing confirmed today: an opposite swing ends a double-top window; a same-kind swing may start a setup
            if let Some(w) = &watch { if w.kind_dt { watch = None; } }
            if let Some(s) = setups(&pv, k, dt) { watch = Some(s); }
            k += 1;
        }
        if t < from { continue; }
        let side = pos.side();
        if side != 0. {
            if (side < 0. && x.c[t] < obj) || (side > 0. && x.c[t] > obj) { pos.exit(t, x.c[t], "price objective"); }
            else if (side < 0. && x.c[t] > stp) || (side > 0. && x.c[t] < stp) { pos.exit(t, x.c[t], "stop close"); }
            continue; // one contract: patterns completing now are not traded
        }
        let Some(w) = &watch else { continue };
        if t < w.from { continue; }
        if t > w.last || (!w.kind_dt && ((w.dir < 0. && x.c[t] > w.guard) || (w.dir > 0. && x.c[t] < w.guard))) { watch = None; continue; }
        let lv = line_at(w.line, t);
        if (w.dir < 0. && x.c[t] < lv) || (w.dir > 0. && x.c[t] > lv) {
            if t < to {
                if w.kind_dt { let h = (w.height - lv).abs(); obj = lv + w.dir * h; stp = lv - w.dir * h / 2.; }
                else { obj = lv + w.dir * w.height; stp = w.stop; }
                pos.enter(w.dir, t, x.c[t], "close");
            }
            watch = None;
        }
    }
    Some(pos.done(to, x.c[to]))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn swings_alternate_and_patterns_trade_both_ways() {
        let c: Vec<f64> = (0..400).map(|i| 100. + 3. * (i as f64 / 9.).sin() + 1.5 * (i as f64 / 3.1).sin()).collect();
        let pv = pivots(&c, 1.);
        assert!(pv.len() > 20 && pv.windows(2).all(|w| w[0].0 == -w[1].0 && w[0].3 < w[1].3 && w[0].1 < w[1].1));
        for z in &pv { assert!(z.1 < z.3 && (c[z.3] - z.2).abs() > 1.); }
        // a textbook head-and-shoulders top on closes, then a break of the neckline
        let path = [88., 92., 90., 95., 92.5, 99., 92.8, 96., 90.]; // start, H0 L0 SL AL H AR SR, then the break of the neckline
        let mut v = vec![path[0]];
        for w in path.windows(2) { for s in 1..=5 { v.push(w[0] + (w[1] - w[0]) * s as f64 / 5.); } }
        let r: Vec<crate::Row> = v.iter().enumerate().map(|(i, &c)| crate::Row { ticker: "T".into(), date: format!("{:05}", i), open: c, high: c, low: c, close: c, volume: None }).collect();
        let x = Ctx::new(&r, &r, &[]);
        let t = run("scsarf_hs", &x, 0, r.len() - 1, &Params::new()).unwrap();
        assert!(t.len() == 1 && t[0].dir < 0., "{t:?}");
    }
}
