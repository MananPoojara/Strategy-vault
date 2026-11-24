//! Forex Impact (Sharptrade Partners), "Forex Gap Play" on the real 1-minute path (research share Trading Knowledge I/Jason Fielder - Forex Profit Formula System/Gap Trading the Forex/
//! Gap Trading the Forex.pdf, pp.3-19 text, pp.20-33 OCR). Book id `fxgapm`. The rule of `fxgap_weekly` (weekly gap fade of 15 pips or more, 2 lots, stop 150 pips, lot 1 at the previous
//! week's close, lot 2 at the end of the week with a breakeven stop after the retrace), resolved on 1-minute bars because the daily card is withheld (stop, retrace and breakeven share a
//! daily bar, #175/#240). Lane K1 (24 Nov 2026). Independent check authors/conv-1123/K1/fxgapm_ref.py.
use crate::connors::{Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "fxgapm";
pub const INTRA: &[&str] = &["fxgap_m"];
pub fn reviewed(id: &str) -> bool { INTRA.contains(&id) }

const NOTE: &str = "[Declared adaptation, R4 #340/#380] The guide trades the EUR/USD weekend gap (pp.8-19; p.13 claims 'accurate almost 90% of the time', no trade list). Run on 1-minute bars of SPX / NIFTY 50 / NIFTYBANK (SPY via the standard minute route), same rule and scaling as fxgap_weekly. Four labelled readings: (1) 1 pip = 0.01 percent of price: the 15-pip gap = 0.15 percent of the previous close, the 150-pip stop = 1.5 percent of the entry; (2) the entry is the open of the week's first minute, the gap is measured against the previous session's daily close, lot 1 exits at that close (a limit: at the level, or at the minute's open when it opens through it) and lot 2 at the last minute's close of the week's last session; (3) the breakeven stop of lot 2 (at the entry) is live from the minute after the retrace minute; (4) inside one minute the breakeven level (nearer the open) is tested before the 150-pip stop, and a minute that reaches both the stop and the retrace level (levels on both sides of its open, a range of about 1.65 percent) counts the stop first; the share of such minutes is stated on the gate row. Two equal lots, one position; a week with a session lacking cleaned 1-minute bars makes no trade. The guide's claim of about 90 percent is not reproduced (see the gate row).";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef { id: "fxgap_m", name: "Forex Gap Play on the 1-minute path: fade a weekly open gap of 15 pips or more, 2 lots, stop 150 pips, lot 1 at the previous close, lot 2 at the week's end (indices)",
        group: "Forex Impact gap play", session: 0, dir: "Long/Short", status: Status::Live, needs_volume: false, needs_vix: false, needs_series: &[], book: "fxgapm",
        rules: &[
            "E|\"Long Setup: Pair gaps down 15 pips or more ... we are going to buy 2 lots/2 mini-lots\" at the open; \"Short Setup: if the pair gaps up 15 pips or more ... sell 2 lots\" (pp.8-11)",
            "X|\"Place the stop loss 150 pips below the entry\" (short: above); a breakeven stop at the entry price once the pair re-traces to the previous week's close (pp.13-17)",
            "X|\"The first lot will be exited at the previous week's close and the second lot will remain open until the end of the week\" (p.19)"],
        notes: NOTE, claim: None }]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    match id { "fxgap_m" => Some(("fxgapm", "Forex Gap Play (1-minute path)", "gap 0.15 percent, stop 1.5 percent")), _ => None }
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    match id {
        "fxgap_m" => Some(vec![p("gap", "Minimum weekly gap, percent of the previous close (15 pips at 1 pip = 0.01 percent)", 0.15, 0.0, 5.0, 0.05),
            p("stop", "Stop, percent of the entry (150 pips at 1 pip = 0.01 percent)", 1.5, 0.1, 10.0, 0.1)]),
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
    let (gap, stop) = (g("gap") / 100.0, g("stop") / 100.0);
    let wk: Vec<i64> = x.r.iter().map(|r| days(&r.date).map_or(-1, |d| (d + 3).div_euclid(7))).collect();
    let mut out: Vec<Trade> = vec![];
    let mut a = from.max(1);
    while a <= to {
        if wk[a] == wk[a - 1] || wk[a] < 0 { a += 1; continue; }
        let mut e = a; while e < to && wk[e + 1] == wk[a] { e += 1; }
        let wkd: Vec<Option<Vec<crate::intraday::Bar1>>> = (a..=e).map(|d| crate::intraday_w2b::day(x, d)).collect();
        if wkd.iter().all(|d| d.as_ref().is_some_and(|v| !v.is_empty())) {
            let pc = x.c[a - 1];
            let ep = wkd[0].as_ref().unwrap()[0].o;
            let gp = (ep - pc) / pc;
            let dir = if gp <= -gap { 1.0 } else if gp >= gap { -1.0 } else { 0.0 };
            if dir != 0.0 {
                let st = ep * (1.0 - dir * stop);
                let mut ex: [Option<(usize, f64, &'static str)>; 2] = [None, None];
                let mut be_on = false; // lot 2's breakeven stop is live from the minute after the retrace
                let mut retr_pending = false;
                'w: for (k, day) in wkd.iter().enumerate() {
                    let d = a + k;
                    let mins = day.as_ref().unwrap();
                    for (j, b) in mins.iter().enumerate() {
                        if retr_pending { be_on = true; retr_pending = false; }
                        // lot 2's breakeven stop sits between the price and the 150-pip stop: a minute that reaches both crosses the breakeven level first
                        // (nearer the open), so it is tested before the stop; before the retrace only the stop and the retrace level exist (levels on both sides of the open
                        // in one minute needs a range of about 1.65 percent: the stop is then counted first, the declared convention)
                        if ex[1].is_none() && be_on && (if dir > 0.0 { b.l <= ep } else { b.h >= ep }) {
                            ex[1] = Some((d, if dir > 0.0 { b.o.min(ep) } else { b.o.max(ep) }, "breakeven stop"));
                        }
                        let stop_hit = if dir > 0.0 { b.l <= st } else { b.h >= st };
                        let sp = if dir > 0.0 { b.o.min(st) } else { b.o.max(st) };
                        if stop_hit && (ex[0].is_none() || ex[1].is_none()) {
                            for z in ex.iter_mut().filter(|z| z.is_none()) { *z = Some((d, sp, "150-pip stop")); }
                            break 'w;
                        }
                        if ex[0].is_none() && (if dir > 0.0 { b.h >= pc } else { b.l <= pc }) {
                            ex[0] = Some((d, if dir > 0.0 { b.o.max(pc) } else { b.o.min(pc) }, "previous week's close"));
                            retr_pending = true;
                        }
                        if ex[0].is_some() && ex[1].is_some() { break 'w; }
                        if k == wkd.len() - 1 && j == mins.len() - 1 {
                            for z in ex.iter_mut().filter(|z| z.is_none()) { *z = Some((d, b.c, "end of the week")); }
                        }
                    }
                }
                if let (Some(l1), Some(l2)) = (ex[0], ex[1]) {
                    let mut pieces = vec![(l1.0, l1.1, 0.5), (l2.0, l2.1, 0.5)];
                    pieces.sort_by_key(|z| z.0);
                    let fin = if l2.0 >= l1.0 { l2 } else { l1 };
                    out.push(Trade { dir, legs: vec![Leg { i: a, px: ep, w: 1. }], exit_i: fin.0, exit_px: 0.5 * l1.1 + 0.5 * l2.1, reason: fin.2.into(), entry_kind: "open", signal_i: a - 1, open: false, exits: pieces });
                }
            }
        }
        a = e + 1;
    }
    Some(out)
}

pub fn index_only() -> &'static str { "Forex Gap Play (a forex weekly gap rule): run on SPX / NIFTY 50 / NIFTYBANK as a declared adaptation (R4 #340/#380) on real 1-minute bars; UNVERIFIED" }
