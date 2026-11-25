//! TradingVisions, "Impetus DayTrading System" (tradingvisions.com; research share USB HDD 1/TradeStation Systems & Indicators/Impetus DayTrading System (tradingvisions.com)/
//! Impetus_Demo.6.txt + IDHigh.1/IDLow.2/MaxTradeHigh.3/MinTradeLow.4/Close_at_end_of_day.7.txt + README_NEW.TXT; the folder _TradingVisions.com (All Systems)/_Older Versions holds the same system). Book id `impet`.
//! EasyLanguage, a 5-minute chart with a hidden daily chart (Data2), "Tested with: SPX.XO S&P 500 INDEX". Lane K3 (25 Nov 2026, R4 #398, rows R216/R220/R244).
//! One card: day-range retrace and impetus stop entries between 13:30 and 15:00, one trade a day. Independent check authors/conv-1123/k3w/imp/imp_ref.py.
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};
use crate::intraday::{day_trade, Bar1};

pub const BOOK: &str = "impet";
pub const ID: &str = "impet_demo";

const NOTE: &str = "[Declared adaptation, R4 #340/#400; 5 labelled readings; unverified] The vendor's EasyLanguage demo runs on a 5-minute chart with a daily Data2 and states 'Tested with SPX.XO'; the card runs the printed code on 5-minute bars built from the real SPX 1-minute bars (2008-), S&P points literal (SPX only: the 0.05 / 0.1 / 1-point offsets are index points). Rules as coded: with no position and no trade yet today, between 13:30 and 15:00 (bar stamps): (S1) a retrace short - yesterday's low >= the low before it, yesterday's high < the midpoint of today's range so far and the prior close is >= 1.3% above today's low - sells short at the signalling bar's low - 0.05 on a stop while that bar's low is above the day's low; (L1) the mirror long at the signalling bar's high + 0.05; from 13:45 (L2) buy at the day's high + 0.05 stop once the day high is >= 1.3% above the prior close, (S2) sell short at the day's low - 0.05 stop once the low is >= 1.3% below it. Exits: S1/L1 a stop at max(yesterday's high + 0.1, entry + 0.9% of the prior close) (long: min(yesterday's low - 0.1, entry - 0.9%)); S2/L2 the same with 0.1%; after the position's best price is 0.8% of the prior close beyond the entry, a stop 1 point beyond the entry (LOCKPTS); from 15:00 to 15:15 a stop at the signalling bar's low - 0.05 (long) / high + 0.05 (short); every position closes at the close of the last bar (SetExitOnClose). Readings, numbered (R4 #400/#404; 5 unstated card-specific choices): (1) the source's clock is not stated; its 15:15 and 14:30/14:00 times match the CME S&P pit/Central-time session (08:30-15:15), so source clock = US Eastern - 60 minutes (clock input 60): the window is then 14:30-16:00 Eastern and the 15:00-15:15 exit window falls after the cash index close (no bar); the clock input 0 gives the Eastern reading (measured: 560 trades); (2) a day, and the two days before it, need 380 or more of the 390 minutes, otherwise no trade that day (9 days of 2008-2022 are skipped); (3) the 5-minute bar is resolved on its 1-minute bars: a stop touched at a minute's open fills at the open, otherwise at its level; (4) Data2, the daily series, is built from the same 1-minute bars (the last completed daily bar: VALUE11 = its close, High/Low DATA(2) = its range), not from a separate daily file; (5) the Entries counter that makes it one trade a day is counted at the entry fill. Honest total: 5 card-specific choices. Engine-wide defaults / declared adaptations (labelled, not counted, R4 #404): two orders on opposite sides touched in one minute: the one nearer the minute's open first, flagged ambiguous (0 cases on SPX); orders generated at a bar close work from the next minute; the 5-minute bars are session-aligned and stamped at their end as in the source platform; MaxTradeHigh/MinTradeLow include the entry bar as the vendor's functions do; an opposite entry touched later in the same bar reverses (0 cases). The demo's licence test (CustomerID, date <= 2022-02-22) is not a rule and is not applied. The 'Actual.Performance.JPG' of the vendor is a chart image and is not a printed figure to compare.";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef { id: ID, name: "Impetus DayTrading (retrace and impetus stops, 13:30-15:00, SPX 5-minute)", group: "TradingVisions Impetus", session: 0, dir: "Long/Short",
        status: Status::Live, needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK,
        rules: &[
            "F|Day-range stops: IDHigh/IDLow = the day's running high/low; VALUE11 = Close of Data2 (the prior daily close); 5-minute chart, Data2 daily (README_NEW.TXT), 'Tested with SPX.XO'",
            "E|S1 sell short at RetraceSell - 0.05 stop and L1 buy at RetraceBuy + 0.05 stop (retrace of the day range, 1.3% from the prior close); L2 buy at IDHigh + 0.05 / S2 sell short at IDLow - 0.05 stop from 13:45 (impetus); window 13:30-15:00, one trade a day (Entries counter)",
            "X|SX1/LX1: stop at MaxList(High of Data2 + 0.1, EntryPrice + 0.009 x VALUE11) (mirror for longs); SX2/LX2 with 0.001; SX4/LX4: stop at EntryPrice +/- LockPts after a 0.008 x VALUE11 move; SX3/LX3 15:00-15:15 bar-extreme stops; SetExitOnClose",
        ], notes: NOTE, claim: None }]
}
pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> { (id == ID).then_some(("impet", "Impetus DayTrading", "Retrace and impetus stops")) }
fn pd(key: &'static str, label: &'static str, def: f64, min: f64, max: f64, step: f64) -> ParamDef { ParamDef { key, label, def, min, max, step } }
pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    (id == ID).then(|| vec![pd("clock", "Minutes added to the source clock to get the data (Eastern) clock (60: the source is on Central time; 0: Eastern)", 60., 0., 120., 15.),
        pd("jump", "Day-range move that arms an entry, % of the prior close (code 0.013)", 1.3, 0.5, 3., 0.1),
        pd("lock", "LOCKPTS: points beyond the entry for the lock stop (code 1)", 1., 0.25, 5., 0.25)])
}

#[derive(Clone, Copy)]
struct Ord2 { name: u8, dir: f64, up: bool, px: f64 } // name: 1 S1, 2 L1, 3 L2, 4 S2, 11.. exits
const NAMES: [&str; 12] = ["", "S1", "L1", "L2", "S2", "SX1", "SX2", "LX1", "LX2", "LX3/SX3", "LX4/SX4", ""];

fn touched(z: &Bar1, o: &Ord2) -> bool { if o.up { z.h >= o.px } else { z.l <= o.px } }
fn fill_px(z: &Bar1, o: &Ord2) -> f64 { if o.up { z.o.max(o.px) } else { z.o.min(o.px) } }
/// The first order reached in one minute: (order, fill, ambiguous).
fn pick(z: &Bar1, v: &[Ord2]) -> Option<(Ord2, f64, bool)> {
    let hit: Vec<&Ord2> = v.iter().filter(|o| touched(z, o)).collect();
    if hit.is_empty() { return None; }
    let (ups, dns): (Vec<&Ord2>, Vec<&Ord2>) = (hit.iter().copied().filter(|o| o.up).collect(), hit.iter().copied().filter(|o| !o.up).collect());
    let open_through = |o: &Ord2| if o.up { z.o >= o.px } else { z.o <= o.px };
    if !ups.is_empty() && !dns.is_empty() {
        let at_open: Vec<&&Ord2> = hit.iter().filter(|o| open_through(o)).collect();
        if at_open.len() == 1 { return Some((**at_open[0], z.o, false)); }
        let o = *hit.iter().min_by(|a, b| (a.px - z.o).abs().total_cmp(&(b.px - z.o).abs())).unwrap();
        return Some((*o, fill_px(z, o), true));
    }
    let grp = if ups.is_empty() { &dns } else { &ups };
    let first = if ups.is_empty() { grp.iter().max_by(|a, b| a.px.total_cmp(&b.px)) } else { grp.iter().min_by(|a, b| a.px.total_cmp(&b.px)) }.unwrap();
    let passed: Vec<&&Ord2> = grp.iter().filter(|o| open_through(o)).collect();
    if passed.len() > 1 && passed.iter().any(|o| o.name != passed[0].name) { return Some((**passed[0], z.o, true)); }
    Some((**first, fill_px(z, first), false))
}

struct Pos { dir: f64, name: u8, ep: f64, hi: f64, lo: f64, amb: bool }

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    params(id)?;
    if to >= x.c.len() || from > to { return None; }
    let g = |k: &str| super::jb624::getp(id, p, k);
    let (off, jump, lock) = (g("clock").round() as u16, g("jump") / 100.0, g("lock"));
    let (lo_t, hi_t, imp_t, x0, x1) = (13 * 60 + 30 + off, 15 * 60 + off, 13 * 60 + 45 + off, 15 * 60 + off, 15 * 60 + 15 + off);
    let daily = |i: usize| -> Option<(f64, f64, f64)> {
        let m = crate::intraday_w2b::day(x, i)?;
        (m.len() >= 380).then(|| (m.iter().map(|z| z.h).fold(f64::MIN, f64::max), m.iter().map(|z| z.l).fold(f64::MAX, f64::min), m[m.len() - 1].c))
    };
    let mut out: Vec<Trade> = vec![];
    for i in from.max(2)..=to {
        let Some(m) = crate::intraday_w2b::day(x, i) else { continue };
        if m.len() < 380 { continue; }
        let (Some((ph, pl, pc)), Some((qh, ql, _))) = (daily(i - 1), daily(i - 2)) else { continue };
        let open = crate::intraday::session_open(&x.r[i].ticker, &x.r[i].date);
        let bars = crate::intraday_ep::nbars(&m, open, 5, crate::intraday_ep::session_close(&x.r[i].ticker));
        let v = pc;
        let (mut idh, mut idl) = (0.0f64, 9999999.0f64);
        let (mut sret, mut lret, mut lim, mut sim, mut entries) = (false, false, false, false, 0u32);
        let (mut rsell, mut rbuy, mut ibuy, mut isell) = (0.0, 0.0, 0.0, 0.0);
        let mut pos: Option<Pos> = None;
        let mut orders: Vec<Ord2> = vec![];
        let mut done: Vec<(Pos, f64, &'static str)> = vec![];
        for (bi, b) in bars.iter().enumerate() {
            for k in b.a..b.b {
                if orders.is_empty() { break; }
                let z = &m[k];
                let eligible: Vec<Ord2> = orders.iter().copied().filter(|o| o.name >= 5 || pos.as_ref().map_or(true, |q| o.dir != q.dir)).collect();
                let Some((od, px, amb)) = pick(z, &eligible) else { continue };
                if od.name >= 5 {
                    if let Some(q) = pos.take() { done.push((q, px, NAMES[od.name as usize])); }
                    orders.retain(|o| o.name < 5);
                } else if let Some(q) = pos.take() {
                    done.push((q, px, "reverse"));
                    pos = Some(Pos { dir: od.dir, name: od.name, ep: px, hi: -999999.0, lo: 999999.0, amb });
                } else { pos = Some(Pos { dir: od.dir, name: od.name, ep: px, hi: -999999.0, lo: 999999.0, amb }); }
                if od.name < 5 { entries += 1; } // the day's one trade: counted at the fill, so a position gone before the bar close still uses it
            }
            idh = idh.max(b.h); idl = idl.min(b.l);
            if let Some(q) = pos.as_mut() { q.hi = q.hi.max(b.h); q.lo = q.lo.min(b.l); }
            orders.clear();
            if bi + 1 == bars.len() {
                if let Some(q) = pos.take() { done.push((q, m[b.b - 1].c, "close")); }
                break;
            }
            let t = b.e;
            if t >= lo_t && t < hi_t && pos.is_none() && entries == 0 {
                if pl >= ql && ph < (idh + idl) * 0.5 && v - idl >= jump * v { sret = true; rsell = b.l; }
                if sret && b.l > idl { orders.push(Ord2 { name: 1, dir: -1.0, up: false, px: rsell - 0.05 }); }
                if ph <= qh && pl > (idh + idl) * 0.5 && idh - v >= jump * v { lret = true; rbuy = b.h; }
                if lret && b.h < idh { orders.push(Ord2 { name: 2, dir: 1.0, up: true, px: rbuy + 0.05 }); }
                if t >= imp_t && idh - v >= jump * v { lim = true; ibuy = idh; }
                if lim { orders.push(Ord2 { name: 3, dir: 1.0, up: true, px: ibuy + 0.05 }); }
                if t >= imp_t && v - idl >= jump * v { sim = true; isell = idl; }
                if sim { orders.push(Ord2 { name: 4, dir: -1.0, up: false, px: isell - 0.05 }); }
            }
            if let Some(q) = &pos {
                let ep = q.ep;
                if q.dir < 0.0 {
                    if q.name == 1 { orders.push(Ord2 { name: 5, dir: 1.0, up: true, px: (ph + 0.1).max(ep + 0.009 * v) }); }
                    if q.name == 4 { orders.push(Ord2 { name: 6, dir: 1.0, up: true, px: (ph + 0.1).max(ep + 0.001 * v) }); }
                    if ep - q.lo >= 0.008 * v { orders.push(Ord2 { name: 10, dir: 1.0, up: true, px: ep - lock }); }
                    if t >= x0 && t <= x1 { orders.push(Ord2 { name: 9, dir: 1.0, up: true, px: b.h + 0.05 }); }
                } else {
                    if q.name == 2 { orders.push(Ord2 { name: 7, dir: -1.0, up: false, px: (pl - 0.1).min(ep - 0.009 * v) }); }
                    if q.name == 3 { orders.push(Ord2 { name: 8, dir: -1.0, up: false, px: (pl - 0.1).min(ep - 0.001 * v) }); }
                    if q.hi - ep >= 0.008 * v { orders.push(Ord2 { name: 10, dir: -1.0, up: false, px: ep + lock }); }
                    if t >= x0 && t <= x1 { orders.push(Ord2 { name: 9, dir: -1.0, up: false, px: b.l - 0.05 }); }
                }
                entries += 1;
            }
        }
        for (q, xp, why) in done {
            let why = if q.amb { format!("{why}, ambiguous minute") } else { why.to_string() };
            out.push(day_trade(q.dir, i, q.ep, i, xp, &why, "stop", false));
        }
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn admitted_on_spx_only_and_gate_row_present() {
        use crate::instrument_policy as ip;
        let d = crate::connors::catalogue().into_iter().find(|d| d.id == ID).unwrap();
        for t in ["SPX", "SPY", "NIFTY 50", "NIFTYBANK"] { assert_eq!(ip::check(&d, t, false).is_none(), t == "SPX", "{t}"); }
        assert!(ip::check(&d, "AAPL", true).is_some());
        let g = crate::gate::get(ID).unwrap();
        assert!(g.verdict == "unverified" && g.reason.contains("R216") && g.ours.contains("0 mismatches"));
    }
    #[test]
    fn a_two_sided_minute_is_ambiguous_and_an_open_through_stop_fills_at_the_open() {
        let z = |o: f64, h: f64, l: f64| Bar1 { m: 0, o, h, l, c: o };
        let v = [Ord2 { name: 2, dir: 1.0, up: true, px: 101.0 }, Ord2 { name: 1, dir: -1.0, up: false, px: 99.0 }];
        assert_eq!(pick(&z(100.0, 101.5, 98.5), &v).map(|r| (r.0.name, r.1, r.2)), Some((2, 101.0, true))); // 101 and 99 are both 1.0 from the open: first listed
        assert_eq!(pick(&z(100.5, 101.5, 100.2), &v).map(|r| (r.0.name, r.1, r.2)), Some((2, 101.0, false)));
        assert_eq!(pick(&z(98.0, 99.0, 97.0), &v).map(|r| (r.0.name, r.1, r.2)), Some((1, 98.0, false))); // opened through the sell stop
        assert!(pick(&z(100.0, 100.5, 99.5), &v).is_none());
    }
}
