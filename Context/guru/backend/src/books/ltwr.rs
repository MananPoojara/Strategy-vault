//! longtermtrading.com (Alan Pryor), "WaveRider" System Manual v1.02a with the website pages (WaveRider Introduction
//! .htm and others). Book id `ltwr`. Source: research drive "USB HDD 1/TradeStation Systems & Indicators/Wave Rider 1.02
//! + Manual for TS (longtermtrading.com)/", copied to Books/longtermtrading.com/WaveRider/. Refs are the manual's
//! sections (TOC: Sec. IV p.18, Sec. V p.21-28 with the ADX table "p.22", Sec. VI p.29, App. D). WRV102AP.ELA is a
//! protected binary and Readme.rar is blocked (no unrar); neither was read.
//! Lane PH (14 Nov 2026, R4 #293 (4), #297, #240/#308/#314 (3)): run on Norgate continuous futures of the manual's 21-commodity
//! portfolio, weekly bars built from the daily back-adjusted bars, fills walked on the week's daily bars. Book check: the vendor's
//! per-market weekly trade lists (MoneyManagement/WR-W-*.xls, 2,975 trades 1970-2002 = the manual's Sec. VI count).
use crate::connors::{claim, sma, wilder, Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "ltwr";

const NOTE: &str = "[Adapted: 2 unstated details, R4 #297; R4 #293 (1), (3), (4)] Runs only on Norgate continuous futures of the manual's 21-commodity portfolio (back-adjusted daily bars, weekly bars = Monday-Sunday calendar weeks of them). At each week's close: ADX(13) of the weekly bars -> the Step 2 row -> buy stop one tick above the highest high of the last Nb weeks, sell stop one tick below the lowest low of the last Ns weeks (the current week included: Sec. V Trade 5), live through the next week. Flat: both stops. Long: the closest of the sell stop-and-reverse, the money stop (entry fill - US$2,100 / the contract's US$ per point on the day, from the actual fill: Trades 6 and 8) and, once the best price since entry is US$5,000 in profit, the profit stop (entry + 50% of that best excursion); short mirrored. A money or profit stop goes flat and the next entry waits for the next week's orders (the manual: TradeStation's same-bar re-entries 'did not occur in the real world'). Fills are walked on the week's daily bars: a stop the open gaps through fills at the open; inside a day the manual's own rule orders the path (Sec. V Trade 4: open > close -> high first, otherwise low first). Unstated in the manual: (1) ADX smoothing - vault reading: TradeStation's ADX (Wilder smoothing of DX), which matches the vendor's own trade lists better than the manual's 'arithmetic average of the previous 13 bar's DXs' (1,503 vs 1,387 of 2,025 entry weeks on 19 markets, 1978-2002); p.adx = 1 is the manual's wording; (2) when the profit stop is updated - vault reading: from the day after the best price, never on the same day; open == close days take the high first. Book check (R4 #168) against the vendor's weekly trade lists (WR-W-*.xls, $150 per trade) from one year after each Norgate series starts to 2002-06-28: all 16 published markets pass; together 1,589 vs 1,633 trades, 37.4% vs 38.3% profitable, average +$387 vs +$398 per trade. Published markets: corn, cocoa, live cattle, lean hogs, soybeans, sugar, soybean meal, 10-year note, T-bond (1978-2020); crude, cotton, gold, copper, natural gas, Swiss franc, silver only to the manual's 2002-06-30 (later more than 5% of trades fill an entry and a stop on one day, R4 #240/#314 (3)); yen, coffee and Eurodollar held (over 5% inside the manual's window too); the Euro (Norgate from 1999) and unleaded gas (Norgate holds RBOB only, from 2005) are not run. Norgate continuous futures, ends 2020-09-11; roll method not stated by the vendor. No costs in the engine (the manual deducts $150 per trade).";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef {
        id: "ltwr_waverider", name: "WaveRider (ADX-adaptive Fibonacci channel stop-and-reverse)",
        group: "Sec. V · WaveRider turning points", session: 0, dir: "Long/Short", status: Status::Live,
        needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK,
        rules: &[
            "F|\"Step 1 - Calculate the 13 Bar ADX\" (Sec. V)",
            "E|\"Step 2 - Calculate the WaveRider Stop and Reverse Points based on the Current Bar's ADX Value\": highest high / lowest low within the last N bars for the buy / sell stop: 0 < ADX <= 13 -> 13 / 8; 13 < ADX <= 26 -> 13 / 8; 26 < ADX <= 39 -> 8 / 5; 39 < ADX <= 52 -> 5 / 3; ADX > 52 -> 2 / 1 (table, Sec. V \"p.22\")",
            "E|\"if long with and ADX of 40 (i.e. within the ADX range of 39 - 52), a sell stop is placed one tick below the lowest low of the 3 bars. If short with an ADX of 40, a buy stop is placed one tick above the highest high of the last 5 bars.\"; \"a reversal system in that a trader continually reverses from long to short and vice-versa\" (Sec. IV.1)",
            "X|\"Step 3 - Calculate the Money Management Stop ... designed to limit your losses to not greater than $2,100 per contract ... calculated by dividing $2,100 by the point value of the underlying commodity\"",
            "X|\"Once $5,000 in open profits is achieved, a trailing stop is calculated based on protecting 50% of all profits. If this stop is closer to the current price than the Buy Stop and Reverse point, it becomes our new Buy Stop exit point\" (Step 4); after a money-management or profit-retention exit \"the trader awaits a new signal to enter in the same or opposite direction\" (Sec. IV.1)",
            "F|Published (Sec. VI): weekly 04/10/1970-6/30/2002, 21 commodities, $150/trade: 2,975 trades, 41.68% profitable, average trade $621.50, PF 1.69, max DD $48,957; monthly 02/26/1971-6/30/2002 ($300/trade): 1,004 trades, 45.82%, average $1,365.06, PF 2.18. Website (WaveRider Introduction.htm): weekly 1/1/1970-12/31/2005: 3,357 trades, 40.81%, average $643.42, PF 1.72",
        ],
        notes: NOTE,
        claim: claim("2,975", "41.68%", "$621.50", "avg 45.7 days", "04/10/1970-6/30/2002, 21 futures, weekly"),
    }]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    Some(match id {
        "ltwr_waverider" => ("ltwr_waverider", "longtermtrading.com WaveRider", "Weekly, book defaults"),
        _ => return None,
    })
}
pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    (id == ID).then(|| vec![ParamDef { key: "adx", label: "ADX smoothing: 0 = TradeStation/Wilder (default), 1 = arithmetic mean of 13 DX (manual Step 1 wording)", def: 0., min: 0., max: 1., step: 1. }])
}

const ID: &str = "ltwr_waverider";
/// Step 2 table: (ADX upper bound, bars for the buy stop, bars for the sell stop).
const BANDS: [(f64, usize, usize); 5] = [(13., 13, 8), (26., 13, 8), (39., 8, 5), (52., 5, 3), (f64::INFINITY, 2, 1)];

#[derive(Clone, Copy, PartialEq, Debug)]
struct St { pos: i8, ep: f64, ei: usize, mm: f64, best: f64, pp: Option<f64>, can: bool }
#[derive(Clone, Copy, PartialEq, Debug)]
enum Ev { In(i8, f64), Out(i8, usize, f64, f64, u8) } // entry (dir, px); exit (dir, entry bar, entry px, exit px, 0 reverse / 1 money / 2 profit)

fn enter(s: St, dir: i8, px: f64, d: usize, mmpts: f64) -> St { St { pos: dir, ep: px, ei: d, mm: px - dir as f64 * mmpts, best: px, pp: None, can: s.can } }

/// One daily bar walked along `path` (open, then high/low in the given order, close): the live stops fill where the path crosses
/// them, or at the open when it gaps through. Flat: buy stop `bs` / sell stop `ss`. In a position: the closest of the
/// stop-and-reverse, the money stop and the profit stop (ties -> stop-and-reverse, then money stop).
fn day(mut s: St, d: usize, path: [f64; 4], bs: f64, ss: f64, mmpts: f64) -> (St, Vec<Ev>) {
    let (mut ev, mut prev) = (vec![], path[0]);
    for (j, &p) in path.iter().enumerate() {
        loop {
            if s.pos == 0 {
                if !s.can { break; }
                let hit = if j == 0 { if p >= bs { Some((1, p)) } else if p <= ss { Some((-1, p)) } else { None } }
                    else if p > prev && prev < bs && bs <= p { Some((1, bs)) }
                    else if p < prev && prev > ss && ss >= p { Some((-1, ss)) } else { None };
                let Some((dir, px)) = hit else { break };
                s = enter(s, dir, px, d, mmpts);
                ev.push(Ev::In(dir, px));
                prev = px;
                continue;
            }
            let (mut lvl, mut why) = (if s.pos == 1 { ss } else { bs }, 0u8);
            for (v, w) in [(Some(s.mm), 1u8), (s.pp, 2u8)] {
                if let Some(v) = v { if (s.pos == 1 && v > lvl) || (s.pos == -1 && v < lvl) { lvl = v; why = w; } }
            }
            let hit = if j == 0 { ((s.pos == 1 && p <= lvl) || (s.pos == -1 && p >= lvl)).then_some(p) }
                else if s.pos == 1 && p < prev && prev > lvl && lvl >= p { Some(lvl) }
                else if s.pos == -1 && p > prev && prev < lvl && lvl <= p { Some(lvl) } else { None };
            let Some(px) = hit else { break };
            ev.push(Ev::Out(s.pos, s.ei, s.ep, px, why));
            prev = px;
            if why == 0 { let dir = -s.pos; s = enter(s, dir, px, d, mmpts); ev.push(Ev::In(dir, px)); } else { s.pos = 0; s.can = false; break; }
        }
        prev = p;
    }
    (s, ev)
}

/// Wilder's 13-bar ADX of weekly bars (h, l, c): +DM/-DM/true range smoothed 1/13 per bar, DX, then the ADX as Wilder's smoothing
/// of DX (TradeStation) or the arithmetic mean of the last 13 DX (`mean`).
fn adx13(h: &[f64], l: &[f64], c: &[f64], mean: bool) -> Vec<f64> {
    let n = c.len();
    let (mut tr, mut pdm, mut ndm) = (vec![f64::NAN; n], vec![f64::NAN; n], vec![f64::NAN; n]);
    for i in 1..n {
        tr[i] = (h[i] - l[i]).max((h[i] - c[i - 1]).abs()).max((l[i] - c[i - 1]).abs());
        let (up, dn) = (h[i] - h[i - 1], l[i - 1] - l[i]);
        pdm[i] = if up > dn && up > 0. { up } else { 0. };
        ndm[i] = if dn > up && dn > 0. { dn } else { 0. };
    }
    let (a, p, m) = (wilder(&tr, 13), wilder(&pdm, 13), wilder(&ndm, 13));
    let dx: Vec<f64> = (0..n).map(|i| if a[i] > 0. { let (dp, dm) = (100. * p[i] / a[i], 100. * m[i] / a[i]); if dp + dm > 0. { 100. * (dp - dm).abs() / (dp + dm) } else { 0. } } else { f64::NAN }).collect();
    if mean { sma(&dx, 13) } else { wilder(&dx, 13) }
}

/// Days since 1970-01-01 (civil calendar).
fn days(s: &str) -> i64 {
    let (y, m, d): (i64, i64, i64) = (s[..4].parse().unwrap_or(0), s[5..7].parse().unwrap_or(1), s[8..10].parse().unwrap_or(1));
    let y = if m <= 2 { y - 1 } else { y };
    let (era, yoe) = (y.div_euclid(400), y.rem_euclid(400));
    era * 146097 + yoe * 365 + yoe / 4 - yoe / 100 + (153 * ((m + 9) % 12) + 2) / 5 + d - 1 - 719468
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if id != ID || from > to || to >= x.r.len() { return None; }
    // futures contracts only: elsewhere no US$ per point exists, so no day is traded (fail closed, #293 (3))
    let tick = crate::futures::ctx_market(x).map_or(0., |m| m.tick);
    let mean = p.get("adx").copied().unwrap_or(0.) >= 0.5;
    // Monday-Sunday weeks of the bars up to `to`
    let mut wk: Vec<Vec<usize>> = vec![];
    for i in 0..=to {
        let k = (days(&x.r[i].date) + 3).div_euclid(7);
        match wk.last_mut() { Some(w) if (days(&x.r[w[0]].date) + 3).div_euclid(7) == k => w.push(i), _ => wk.push(vec![i]) }
    }
    let h: Vec<f64> = wk.iter().map(|w| w.iter().map(|&i| x.h[i]).fold(f64::MIN, f64::max)).collect();
    let l: Vec<f64> = wk.iter().map(|w| w.iter().map(|&i| x.l[i]).fold(f64::MAX, f64::min)).collect();
    let c: Vec<f64> = wk.iter().map(|w| x.c[*w.last().unwrap()]).collect();
    let adx = adx13(&h, &l, &c, mean);
    let mk = |dir: i8, ei: usize, ep: f64, xi: usize, xp: f64, why: &str, open: bool| Trade { dir: dir as f64, legs: vec![Leg { i: ei, px: ep, w: 1. }], exit_i: xi, exit_px: xp,
        reason: why.into(), entry_kind: "stop", signal_i: xi, open, exits: vec![] };
    const WHY: [&str; 3] = ["stop and reverse", "money stop $2,100", "profit stop (50% of open profit above $5,000)"];
    let mut s = St { pos: 0, ep: 0., ei: 0, mm: 0., best: 0., pp: None, can: true };
    let mut out = vec![];
    for k in 0..wk.len().saturating_sub(1) {
        let a = adx[k];
        if !a.is_finite() { continue; }
        let &(_, nb, ns) = BANDS.iter().find(|b| a <= b.0)?;
        if k + 1 < nb || k + 1 < ns { continue; }
        let bs = h[k + 1 - nb..=k].iter().fold(f64::MIN, |a, &b| a.max(b)) + tick;
        let ss = l[k + 1 - ns..=k].iter().fold(f64::MAX, |a, &b| a.min(b)) - tick;
        s.can = s.pos == 0; // after a money or profit stop the next entry waits for the next week's orders
        for &d in &wk[k + 1] {
            if d < from { continue; }
            let (Some(mmpts), Some(pf)) = (crate::futures::usd_points(x, d, 2100.), crate::futures::usd_points(x, d, 5000.)) else { continue };
            let (o, c) = (x.o[d], x.c[d]);
            // the manual's intrabar rule (Sec. V Trade 4): open > close -> the high came first, else the low (open == close: high first, reading)
            let path = if o >= c { [o, x.h[d], x.l[d], c] } else { [o, x.l[d], x.h[d], c] };
            let (ns_, ev) = day(s, d, path, bs, ss, mmpts);
            for e in ev { if let Ev::Out(dir, ei, ep, px, why) = e { out.push(mk(dir, ei, ep, d, px, WHY[why as usize], false)); } }
            s = ns_;
            if s.pos != 0 {
                s.best = if s.pos == 1 { s.best.max(x.h[d]) } else { s.best.min(x.l[d]) };
                if (s.best - s.ep).abs() >= pf { s.pp = Some(s.ep + s.pos as f64 * 0.5 * (s.best - s.ep).abs()); } // live from the next day
            }
        }
    }
    if s.pos != 0 { out.push(mk(s.pos, s.ei, s.ep, to, x.c[to], "open", true)); }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn catalogue_is_live_and_owned() {
        for s in &catalogue() {
            assert!(s.id.starts_with("ltwr_") && s.book == BOOK && family_of(s.id).is_some());
            assert!(matches!(s.status, Status::Live));
        }
    }
    #[test]
    fn day_walks_the_path_in_the_given_order() {
        // flat, buy stop 10.5 / sell stop 9.5, money stop 0.5 point. High first (10 -> 11 -> 9 -> 9.2): long at 10.5, then the money
        // stop 10.0 (closer than the 9.5 stop-and-reverse) goes flat; no new entry the same week
        let s = St { pos: 0, ep: 0., ei: 0, mm: 0., best: 0., pp: None, can: true };
        let (a, ev) = day(s, 7, [10., 11., 9., 9.2], 10.5, 9.5, 0.5);
        assert_eq!(ev, vec![Ev::In(1, 10.5), Ev::Out(1, 7, 10.5, 10., 1)]);
        assert!(a.pos == 0 && !a.can);
        // low first: short at 9.5, its money stop 10.0 fills on the way to the high
        let (_, ev2) = day(s, 7, [10., 9., 11., 9.2], 10.5, 9.5, 0.5);
        assert_eq!(ev2, vec![Ev::In(-1, 9.5), Ev::Out(-1, 7, 9.5, 10., 1)]);
        // money stop beyond the stop-and-reverse: the reverse fills (long 10.5 -> short 9.5), the short's money stop 11.5 is not reached
        let (c, ev3) = day(s, 7, [10., 11., 9., 9.2], 10.5, 9.5, 2.);
        assert_eq!(ev3, vec![Ev::In(1, 10.5), Ev::Out(1, 7, 10.5, 9.5, 0), Ev::In(-1, 9.5)]);
        assert!(c.pos == -1 && (c.mm - 11.5).abs() < 1e-12);
        // a gap through the buy stop fills at the open
        let (_, ev4) = day(s, 7, [12., 12.5, 11.8, 12.], 10.5, 9.5, 2.);
        assert_eq!(ev4, vec![Ev::In(1, 12.)]);
    }
    #[test]
    fn adx_table_rows() {
        let row = |a: f64| BANDS.iter().find(|b| a <= b.0).map(|b| (b.1, b.2)).unwrap();
        assert_eq!((row(11.9), row(17.5), row(27.4), row(42.7), row(56.1)), ((13, 8), (13, 8), (8, 5), (5, 3), (2, 1))); // Sec. V Trades 8, 7, 2, 1
    }
}
