//! Ultimate Trading Zones (ultimatetradingzones.com course, 2007; research share Trading Knowledge I/Ultimate Trading Zones
//! Complete Course & Videos/UltimateTradingZones3344.pdf, manual pp.13-16 and the worked Example #1, pp.20-26). Book id `utz`.
//! Intraday S&P E-mini rule run on real 1-minute bars: the day is cut into 30-minute zones; after each zone the next zone's
//! short and long entry are the zone's close plus / minus one third of its range; one trade per zone, first level reached.
//! Lane K4 (24 Nov 2026, R4 #397). Independent pandas reference: authors/code-1123/ref/utz_ref.py.
use crate::connors::{Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};
use crate::intraday::{first_touch, fill_px, touched, Level, Touch};

pub const BOOK: &str = "utz";
/// The S&P level of the manual's Example #1 (zone close 1460.25 on 30 Aug 2007): one scaling rule, 1 S&P point = 1/1460 of the price.
const REF: f64 = 1460.0;
const ZONE: u16 = 30;

const NOTE: &str = "[Declared readings] Ultimate Trading Zones manual pp.13-16, Example #1 pp.20-26 (S&P E-mini, 30 Aug 2007). The trading day is cut into 30-minute zones from the day's first minute; no trade in the first zone ('no previous price action'). After each zone: range = high - low of that zone; short entry = close + range/3, long entry = close - range/3 (the printed spreadsheet columns, checked on the manual's two worked rows: H 1463 L 1452.5 C 1460.25 -> 3.50, short 1463.75, long 1456.75; H 1464.5 L 1458.25 C 1463.25 -> 2.08, 1465.33 / 1461.17). The short entry is rounded up and the long entry down to the tick (p.25). In the next zone one trade only, whichever level is reached first (short on the way up, long on the way down), no trade held past the zone: out at the zone's last close. Target 1 point and stop 1 point (risk:reward 1:1, p.16; the manual allows 1-2 points). Four labelled readings: (1) one scaling rule, 1 S&P point = 1/1460 of the entry price, so the NSE runs trade the same percentage; (2) only complete 30-minute zones are traded (11 trade zones on a 9:15-15:30 NSE day, 12 on the US day); (3) the target and stop are tested from the minute after the entry minute, and a minute that touches both uses the level nearer its open; (4) the manual's list of US economic releases to skip is not applied (no matching NSE calendar). Stocks must have no news that day (not applied).";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef { id: "utz_zones", name: "Ultimate Trading Zones: 30-minute zone range trades, one trade per zone", group: "Ultimate Trading Zones (course, 2007)", session: 0,
        dir: "Long/Short", status: Status::Live, needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK,
        rules: &[
            "F|\"Our job ... is to collect the previous 30-minutes High Low and Close prices\"; the day is cut into 30-minute \"trade zones\" and there is \"no trade taken during the first 30 minutes\" (p.13)",
            "F|Spreadsheet (worked rows, p.23-24): range = high - low of the finished zone; short entry = close + range/3, long entry = close - range/3, short rounded up and long down to the tick (p.25)",
            "E|\"we look to get short on the way up and long on the way down ... We only look to take one trade per trade zone, whichever trade is triggered first, Long or Short\" (p.14)",
            "X|\"Our risk reward ratio is 1:1. So for every one point we set as a target we are willing to risk the same amount\" (p.16): S&P E-mini target 1 point, stop 1 point",
            "X|\"No trade is held past the 30-minute period trade zone in which it was entered. Close the position before the next 30-minute trade zone begins\" (p.14)"],
        notes: NOTE, claim: None }]
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    if id != "utz_zones" { return None; }
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    Some(vec![p("path", "1 = real 1-minute bars (the only path: the zones need minute data; days without minutes make no trade)", 1., 1., 1., 1.),
              p("target", "Target in S&P points at the 1460 reference level (printed: 1-2 points for the E-mini)", 1., 0.5, 3., 0.5),
              p("stop", "Stop in S&P points at the 1460 reference level (printed: the same as the target, 1:1)", 1., 0.5, 3., 0.5)])
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    (id == "utz_zones").then_some(("utz_zones", "Ultimate Trading Zones", "30-minute zones, close +/- range/3, 1:1 points"))
}

fn tick_up(v: f64, t: f64) -> f64 { (v / t - 1e-9).ceil() * t }
fn tick_dn(v: f64, t: f64) -> f64 { (v / t + 1e-9).floor() * t }

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if id != "utz_zones" { return None; }
    let g = |k: &str, d: f64| p.get(k).copied().unwrap_or(d);
    let (tgt, stp) = (g("target", 1.), g("stop", 1.));
    let tick = x.market_tick();
    let mut out = Vec::new();
    for j in from..=to.min(x.r.len().saturating_sub(1)) {
        let Some(b) = crate::intraday_w2b::day(x, j) else { continue };
        let Some(first) = b.first() else { continue };
        let m0 = first.m;
        let zone = |k: u16| -> Vec<crate::intraday::Bar1> { b.iter().copied().filter(|z| z.m >= m0 + k * ZONE && z.m < m0 + (k + 1) * ZONE).collect() };
        let last = b.last().map_or(0, |z| z.m);
        let mut k = 0u16;
        loop {
            let zk = zone(k);
            let nxt = zone(k + 1);
            // only complete 30-minute zones are traded
            if zk.is_empty() || nxt.is_empty() || last + 1 < m0 + (k + 2) * ZONE { break; }
            k += 1;
            let hi = zk.iter().map(|z| z.h).fold(f64::MIN, f64::max);
            let lo = zk.iter().map(|z| z.l).fold(f64::MAX, f64::min);
            let c = zk[zk.len() - 1].c;
            let r = (hi - lo) / 3.;
            let lv = [Level { up: true, px: tick_up(c + r, tick) }, Level { up: false, px: tick_dn(c - r, tick) }];
            let (which, bar, px) = match first_touch(&nxt, 0, &lv) {
                Touch::Hit { k, bar, px } => (k, bar, px),
                Touch::Ambiguous { bar } => {
                    let z = &nxt[bar];
                    let k = (0..2).filter(|&k| touched(z, lv[k])).min_by(|&a, &c2| (lv[a].px - z.o).abs().total_cmp(&(lv[c2].px - z.o).abs())).unwrap_or(0);
                    (k, bar, fill_px(z, lv[k]))
                }
                Touch::None => continue,
            };
            let dir = if which == 0 { -1.0 } else { 1.0 }; // the upper level is the short entry
            let (t, s) = (px / REF * tgt, px / REF * stp);
            let ex = [Level { up: dir > 0., px: px + dir * t }, Level { up: dir < 0., px: px - dir * s }]; // target in the trade's direction, stop against it
            let (xp, why) = match crate::intraday_w2b::first_exit(&nxt, bar + 1, &ex) {
                Some((0, _, f, _)) => (f, "target"),
                Some((_, _, f, _)) => (f, "stop"),
                None => (nxt[nxt.len() - 1].c, "zone end close"),
            };
            out.push(Trade { dir, legs: vec![Leg { i: j, px, w: 1.0 }], exit_i: j, exit_px: xp, reason: format!("{why}"), entry_kind: "limit", signal_i: j, open: false, exits: vec![] });
        }
    }
    Some(out)
}

