//! Forex Impact / Joel Rensink "First Strike Plus & One Night Stand" folder (infiniteyield.com), the original FirstStrike weekly OCO. Book id `fxfs`.
//! Rule (Q381 row 321): Monday's open +/- 50 pips as buy / sell stops (one cancels the other), a 60-pip stop from the fill, flat until the Friday exit.
//! Lane K1 (24 Nov 2026). Independent check authors/conv-1123/K1/fxfs_ref.py. Orders and stops run on the real 1-minute path.
use crate::connors::{Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "fxfs";
pub const INTRA: &[&str] = &["fxfs_weekly"];
pub fn reviewed(id: &str) -> bool { INTRA.contains(&id) }

const NOTE: &str = "[Declared adaptation, R4 #340/#380] The system trades GBP/USD (Monday 01:00 EST open, Friday 16:00 EST exit; file figures 1977-2007: 1,275 trades, 45% winners, 41,100 pips, not reproducible on indices). This card runs the rule on 1-minute bars of SPX / NIFTY 50 / NIFTYBANK. Four labelled readings: (1) 1 pip = 0.01 percent of the reference price, so 50 pips = 0.50 percent and the 60-pip stop = 0.60 percent; (2) the Monday open is the first minute's open of the week's first session (a holiday Monday starts the week on the next session); (3) the Friday exit is the close of the last minute of the week's last session; (4) a minute that reaches both entry levels fills the level nearer its open, and a stop inside the entry minute counts (pessimistic). A week with any session lacking 1-minute bars makes no trade.";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef { id: "fxfs_weekly", name: "FirstStrike: weekly open +/- 50 pips buy/sell stop (OCO), 60-pip stop, exit at the Friday close (indices, 1-minute path)",
        group: "Forex Impact FirstStrike", session: 0, dir: "Long/Short", status: Status::Live, needs_volume: false, needs_vix: false, needs_series: &[], book: "fxfs",
        rules: &[
            "E|Monday open +/- 50 pips: buy stop above, sell stop below; when one fills the other is cancelled (one trade a week)",
            "X|60-pip stop from the fill; no profit target",
            "X|Flat at the Friday close (the week's last session)"],
        notes: NOTE, claim: None }]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    match id { "fxfs_weekly" => Some(("fxfs", "FirstStrike weekly OCO", "entry 0.5 percent, stop 0.6 percent")), _ => None }
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    match id {
        "fxfs_weekly" => Some(vec![p("entry", "Entry distance from the Monday open, percent (50 pips at 1 pip = 0.01 percent)", 0.5, 0.05, 5.0, 0.05),
            p("stop", "Stop from the fill, percent (60 pips at 1 pip = 0.01 percent)", 0.6, 0.05, 10.0, 0.05)]),
        _ => None,
    }
}

fn days(d: &str) -> Option<i64> {
    let v: Vec<i64> = d.split('-').map(|s| s.parse().ok()).collect::<Option<_>>()?;
    if v.len() != 3 { return None; }
    let (mut y, m, dd) = (v[0], v[1], v[2]);
    if m <= 2 { y -= 1; }
    let era = y.div_euclid(400); let yoe = y - era * 400;
    let doy = (153 * (if m > 2 { m - 3 } else { m + 9 }) + 2) / 5 + dd - 1;
    Some(era * 146097 + yoe * 365 + yoe / 4 - yoe / 100 + doy - 719468)
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.r.len() { return None; }
    let s = params(id)?;
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| s.iter().find(|d| d.key == k).unwrap().def);
    let (ent, stp) = (g("entry") / 100.0, g("stop") / 100.0);
    let wk: Vec<i64> = x.r.iter().map(|r| days(&r.date).map_or(-1, |d| (d + 3).div_euclid(7))).collect();
    let mut out: Vec<Trade> = vec![];
    let mut a = from.max(1);
    while a <= to {
        if wk[a] == wk[a - 1] || wk[a] < 0 { a += 1; continue; }
        let mut e = a; while e < to && wk[e + 1] == wk[a] { e += 1; }
        let wkd: Vec<Option<Vec<crate::intraday::Bar1>>> = (a..=e).map(|d| crate::intraday_w2b::day(x, d)).collect();
        if wkd.iter().all(|d| d.as_ref().is_some_and(|v| !v.is_empty())) {
            let base = wkd[0].as_ref().unwrap()[0].o;
            let (up, dn) = (base * (1.0 + ent), base * (1.0 - ent));
            let mut pos: Option<(f64, f64, f64, usize)> = None; // dir, fill, stop, entry day
            let mut res: Option<(usize, f64, &'static str)> = None;
            'w: for (k, day) in wkd.iter().enumerate() {
                let d = a + k;
                for (j, b) in day.as_ref().unwrap().iter().enumerate() {
                    if pos.is_none() {
                        let (tu, td) = (b.h >= up, b.l <= dn);
                        let dir = if tu && td { if (up - b.o).abs() <= (b.o - dn).abs() { 1.0 } else { -1.0 } } else if tu { 1.0 } else if td { -1.0 } else { 0.0 };
                        if dir != 0.0 {
                            let fill = if dir > 0.0 { b.o.max(up) } else { b.o.min(dn) };
                            pos = Some((dir, fill, fill * (1.0 - dir * stp), d));
                        }
                    }
                    if let Some((dir, _, st, _)) = pos {
                        if (dir > 0.0 && b.l <= st) || (dir < 0.0 && b.h >= st) {
                            res = Some((d, if dir > 0.0 { b.o.min(st) } else { b.o.max(st) }, "60-pip stop")); break 'w;
                        }
                    }
                    if k == wkd.len() - 1 && j == day.as_ref().unwrap().len() - 1 { res = Some((d, b.c, "Friday close")); }
                }
            }
            if let (Some((dir, fill, _, ed)), Some((xi, xp, why))) = (pos, res) {
                out.push(Trade { dir, legs: vec![Leg { i: ed, px: fill, w: 1. }], exit_i: xi, exit_px: xp, reason: why.into(), entry_kind: "stop", signal_i: a, open: false, exits: vec![] });
            }
        }
        a = e + 1;
    }
    Some(out)
}

pub fn index_only() -> &'static str { "FirstStrike (a GBP/USD weekly breakout): run on the indices SPX / NIFTY 50 / NIFTYBANK as a declared adaptation (R4 #340/#380) on real 1-minute bars; UNVERIFIED" }
