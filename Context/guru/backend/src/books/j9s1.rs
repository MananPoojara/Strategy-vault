//! Ian Kotze, "The J9S1 System" (traders-secret-library.com, 1st Place Special Award, June 2009 Challenge; research share Trading Knowledge I/Surefire Trading
//! Challenge Winning Trading Systems/June 2009/The J9S1 System/j9s1.pdf, 7 pp., text layer, read in full). Book id `j9s1`. 10-minute bars, 9-period RSI and 13-period
//! Chande Momentum Oscillator: buy 1 lot when RSI is below 30 and CMO below -50 together; sell 1 lot when RSI is above 70 and CMO above 50; fixed 200-point stop and
//! 350-point target; "keep it until the stop loss or stop profit is taken"; a short and a long may be held together. Lane K1 (24 Nov 2026, Q381 row 317).
//! Independent check authors/conv-1123/K1/j9s1_ref.py. Each direction is its own card (the engine holds one position per card).
use crate::connors::{Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};
use crate::intraday::Bar1;

pub const BOOK: &str = "j9s1";
pub const INTRA: &[&str] = &["j9s1_long", "j9s1_short"];
pub fn reviewed(id: &str) -> bool { INTRA.contains(&id) }

const NOTE: &str = "[Declared adaptation, R4 #340/#380] The system trades the South African ALSI Top 40 (CFD, 10-minute chart); this card runs it on 10-minute bars built from the real 1-minute bars of SPX / NIFTY 50 / NIFTYBANK (SPY via the standard minute route). Labelled readings: (1) the 200-point stop and 350-point target are scaled to percent of the entry price at the ALSI level of June 2009 (about 22,000, an external figure of mine, not printed in the report): 0.91 percent and 1.59 percent; (2) the rule list prints 'RSI must be above 30' for longs and 'below 70' for shorts, but the examples and the summary say RSI below 30 / above 70, which is used; (3) a signal at a closed bar fills at the open of the next 10-minute bar (the next session's first bar after a session's last bar), indicators run as one continuous series across sessions (Wilder RSI, plain CMO); (4) the long and the short side are separate cards with one position each (a new signal in a direction already held is ignored), exits only by the stop or the target, hit on the 1-minute path (stop first if one minute reaches both; a gap fills at the minute's open), held across sessions; a session without cleaned 1-minute bars is skipped.";

pub fn catalogue() -> Vec<StrategyDef> {
    macro_rules! mk { ($id:expr, $name:expr, $dir:expr, $e:expr) => { StrategyDef { id: $id, name: $name, group: "Kotze J9S1", session: 0, dir: $dir, status: Status::Live,
        needs_volume: false, needs_vix: false, needs_series: &[], book: "j9s1", rules: &[$e,
            "X|\"Stop loss must be fixed, in the examples I used 200 points\"; \"350 points profit stop/target limit\" (p.4); \"keep it until the stop loss or stop profit is taken\" (p.5)",
            "A|\"Lot size: 1 at a time but you may end up holding up to 3 at a given time\"; 10 min chart, 9 period RSI and 13 period Chande Momentum Oscillator (p.2)"], notes: NOTE, claim: None } } }
    vec![mk!("j9s1_long", "J9S1 long: RSI(9) below 30 and CMO(13) below -50 on 10-minute bars, stop 0.91 percent, target 1.59 percent (indices, 1-minute path)", "Long",
            "E|\"Buy 1 contract when the RSI is below 30 and the Chande is below -50 at the same time\" (p.5)"),
        mk!("j9s1_short", "J9S1 short: RSI(9) above 70 and CMO(13) above 50 on 10-minute bars, stop 0.91 percent, target 1.59 percent (indices, 1-minute path)", "Short",
            "E|\"Sell one if the RSI is above 70 and the Chande is above 50 at the same time\" (p.5)")]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    reviewed(id).then_some(("j9s1", "J9S1 RSI(9) + CMO(13) on 10-minute bars", if id == "j9s1_long" { "Long side" } else { "Short side" }))
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    reviewed(id).then(|| vec![p("stop", "Stop, percent of the entry (200 points at the ALSI level of about 22,000)", 0.91, 0.1, 10.0, 0.01),
        p("target", "Target, percent of the entry (350 points at the ALSI level of about 22,000)", 1.59, 0.1, 20.0, 0.01)])
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.r.len() { return None; }
    let s = params(id)?;
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| s.iter().find(|d| d.key == k).unwrap().def);
    let (stp, tgt) = (g("stop") / 100.0, g("target") / 100.0);
    let dir = if id == "j9s1_long" { 1.0 } else { -1.0 };
    // per-session minutes and 10-minute bars (days without cleaned minutes: None)
    let mut mins: Vec<Option<Vec<Bar1>>> = Vec::with_capacity(to + 1);
    let mut bars: Vec<(usize, usize, usize, f64)> = vec![]; // (day, first minute index, last minute index, close) of every 10-minute bar, in order
    for i in 0..=to {
        match crate::intraday_ep::day_bars(x, i, 10) {
            Some((_, _, m, b)) => { for z in &b { bars.push((i, z.a, z.b - 1, z.c)); } mins.push(Some(m)); }
            None => mins.push(None),
        }
    }
    let n = bars.len();
    let (mut rsi, mut cmo) = (vec![f64::NAN; n], vec![f64::NAN; n]);
    let (mut ag, mut al) = (0.0, 0.0);
    for k in 1..n {
        let ch = bars[k].3 - bars[k - 1].3;
        let (u, d) = (ch.max(0.0), (-ch).max(0.0));
        if k <= 9 { ag += u / 9.0; al += d / 9.0; } else { ag = (ag * 8.0 + u) / 9.0; al = (al * 8.0 + d) / 9.0; }
        if k >= 9 { rsi[k] = if al == 0.0 { if ag == 0.0 { 50.0 } else { 100.0 } } else { 100.0 - 100.0 / (1.0 + ag / al) }; }
        if k >= 13 {
            let (mut su, mut sd) = (0.0, 0.0);
            for j in k - 12..=k { let c = bars[j].3 - bars[j - 1].3; su += c.max(0.0); sd += (-c).max(0.0); }
            cmo[k] = if su + sd == 0.0 { 0.0 } else { 100.0 * (su - sd) / (su + sd) };
        }
    }
    let mut out: Vec<Trade> = vec![];
    let mut busy: (usize, usize) = (0, 0); // (day, minute index) of the last exit; entries must be later
    for k in 0..n.saturating_sub(1) {
        let sig = if dir > 0.0 { rsi[k] < 30.0 && cmo[k] < -50.0 } else { rsi[k] > 70.0 && cmo[k] > 50.0 };
        if !sig { continue; }
        let (ed, ea, _, _) = bars[k + 1];
        if ed < from || ed > to || (ed, ea) <= busy { continue; }
        let px = mins[ed].as_ref().unwrap()[ea].o;
        let (st, tg) = (px * (1.0 - dir * stp), px * (1.0 + dir * tgt));
        let mut res: Option<(usize, usize, f64, &'static str)> = None;
        'w: for d in ed..=to {
            let Some(m) = &mins[d] else { continue };
            for (j, b) in m.iter().enumerate().skip(if d == ed { ea } else { 0 }) {
                let (sh, th) = if dir > 0.0 { (b.l <= st, b.h >= tg) } else { (b.h >= st, b.l <= tg) };
                if sh { res = Some((d, j, if dir > 0.0 { b.o.min(st) } else { b.o.max(st) }, "stop")); break 'w; }
                if th { res = Some((d, j, if dir > 0.0 { b.o.max(tg) } else { b.o.min(tg) }, "target")); break 'w; }
            }
        }
        let sg = bars[k].0;
        match res {
            Some((d, j, xp, why)) => { busy = (d, j); out.push(Trade { dir, legs: vec![Leg { i: ed, px, w: 1. }], exit_i: d, exit_px: xp, reason: why.into(), entry_kind: "open", signal_i: sg, open: false, exits: vec![] }); }
            None => { out.push(Trade { dir, legs: vec![Leg { i: ed, px, w: 1. }], exit_i: to, exit_px: x.c[to], reason: "open".into(), entry_kind: "open", signal_i: sg, open: true, exits: vec![] }); break; }
        }
    }
    Some(out)
}
