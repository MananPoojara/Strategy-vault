//! John F. Clayburg, "Parallel Functions in Self Adaptive Systems" (Omega World presentation, c. 2000). Book id
//! `clomw`. Source: research drive ".../John Clayburg - Systems & Indicators for TS + Manual (clayburg.com)/
//! OmegaWorldAbstract.doc" (also under "John Clayburg - Parallel Functions in Self Adaptive Systems/"); copy in
//! Books/John Clayburg/. Transcription: library/clomw/.
//!
//! The paper's base system is complete EasyLanguage: it trades the break of the intraday range fixed `Delay` minutes
//! after the session open, with an intraday limit target and stop. Lane D1 (16 Nov 2026, R4 #339/#340): run on the held
//! SPX 1-minute bars (S&P points, SPX only), always UNVERIFIED, four labelled unstated details (see `notes`).
use crate::books::n1intra::{minute, Ord};
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};
use crate::intraday::day_trade;

pub const BOOK: &str = "clomw";

#[allow(dead_code)]
const GAP: &str = "intraday bars of the session (the high/low reached Delay=45 minutes after the open, then the intrabar order of the entry stop, the Tgt=7 limit, the Stp=4 stop and the session close). Daily OHLC cannot tell which of these was hit first, and no intraday history is in the vault";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef { id: "clomw_early_range_breakout", name: "Early-range breakout base system (Omega World)", group: "Base system · Parallel functions paper",
        session: 0, dir: "Long/Short", status: Status::Live, needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK,
        rules: &[
            "F|NuHi/NuLo = the day's running high and low; at T = CalcTime(Sess1StartTime, Delay) set Bpt = NuHi + .1, Spt = NuLo - .1 (code inputs Delay(45), Tgt(7), Stp(4)).",
            "E|If TradesToday(d) = 0: BUY Bpt stop; SELL Spt stop (one trade per day).",
            "X|Long: ExitLong entryprice + Tgt limit; ExitLong entryprice - Stp stop. Short: ExitShort entryprice - Tgt limit; ExitShort entryprice + Stp stop.",
            "X|The paper's parallel function closes an open trade at the session close (T = Sess1EndTime, value C - Bpt).",
        ],
        notes: "Declared adaptation (R4 #339/#340, lane D1): the paper's futures day-trading example is run on the real SPX 1-minute bars (2008-), S&P points literal (Tgt 7, Stp 4, +/-0.1 as printed in the first listing), no costs; always UNVERIFIED. Four details the paper leaves open, each a labelled reading: (1) bar size: not printed (the code reads Sess1StartTime + Delay on any bar), 1-minute bars used; (2) the bar time stamp: TradeStation stamps a bar with its END time, so T = 10:15 is the minute that starts 10:14: the range is the minutes 09:30..10:14 and the stops work from the 10:15 minute (a day missing that minute makes no trade, as the code's T = test never fires); (3) exit orders are placed at the close of the entry minute and work from the next minute (TradeStation next-bar), an entry order is live for a whole minute (both stops live while TradesToday = 0 at the minute's start; the other side reverses if reached in the same minute); (4) the base system has no session exit, so the paper's own parallel function is followed: a position still open at the session's last minute is closed at its close. Minute path: a stop or limit fills at its level or at the minute's open when it opens through it; inside a minute the price runs open, the nearer extreme, the farther extreme, close. The paper prints two input sets: the code Delay(45), Tgt(7), Stp(4) with Bpt = NuHi + .1, and an explanatory copy Delay(30), Tgt(2), Stp(4) without the .1 tick; the first listing is the system. Futures (currency / S&P) day-trading example; no results are printed. The 'parallel function' self-optimisation is a programming technique, not a separate rule set.",
        claim: None }]
}
pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    (id == "clomw_early_range_breakout").then_some(("clomw_orb", "Clayburg early-range breakout", "Omega World base system · data missing"))
}
pub const SPX: &[&str] = &["clomw_early_range_breakout"];
fn pd(key: &'static str, label: &'static str, def: f64, min: f64, max: f64, step: f64) -> ParamDef { ParamDef { key, label, def, min, max, step } }
pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    (id == "clomw_early_range_breakout").then(|| vec![pd("delay", "Delay (minutes after the open; code input 45)", 45., 5., 180., 5.),
        pd("tgt", "Tgt (S&P points; code input 7)", 7., 0.5, 40., 0.5), pd("stp", "Stp (S&P points; code input 4)", 4., 0.5, 40., 0.5)])
}
#[derive(Clone, Default)]
struct S { pos: Option<(f64, f64)>, n: u32, xon: bool, entry_on: bool, used: u8, done: Vec<(f64, f64, f64, &'static str)> }
pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    params(id)?;
    if to >= x.c.len() || from > to { return None; }
    let g = |k: &str| super::jb624::getp(id, p, k);
    let (delay, tgt, stp) = (g("delay").round() as u16, g("tgt"), g("stp"));
    let mut out = vec![];
    for i in from..=to {
        let Some(m) = crate::intraday_w2b::day(x, i) else { continue };
        let open = crate::intraday::session_open(&x.r[i].ticker, &x.r[i].date);
        let tk = open + delay - 1; // the minute that starts one minute before the end-stamp T = Sess1StartTime + Delay
        if !m.iter().any(|q| q.m == tk) { continue; }
        let hi = m.iter().filter(|q| q.m <= tk).map(|q| q.h).fold(f64::NEG_INFINITY, f64::max);
        let lo = m.iter().filter(|q| q.m <= tk).map(|q| q.l).fold(f64::INFINITY, f64::min);
        let (bpt, spt) = (hi + 0.1, lo - 0.1);
        let orders = |s: &S| -> Vec<Ord> {
            let mut v = vec![];
            if let Some((d, ep)) = s.pos { if s.xon { v.push(Ord { up: d > 0.0, lv: ep + d * tgt, tag: 3 }); v.push(Ord { up: d < 0.0, lv: ep - d * stp, tag: 4 }); } }
            if s.n == 0 { if s.used & 1 == 0 { v.push(Ord { up: true, lv: bpt, tag: 1 }); } if s.used & 2 == 0 { v.push(Ord { up: false, lv: spt, tag: 2 }); } }
            v
        };
        let fill = |s: &mut S, t: u8, px: f64| match t {
            1 | 2 => { s.used |= t; let d = if t == 1 { 1.0 } else { -1.0 }; if s.pos.map_or(false, |q| q.0 == d) { return; }
                if let Some((pd_, pe)) = s.pos.take() { s.done.push((pd_, pe, px, "reverse")); } s.pos = Some((d, px)); s.n += 1; }
            3 => { if let Some((d, e)) = s.pos.take() { s.done.push((d, e, px, "target")); } }
            _ => { if let Some((d, e)) = s.pos.take() { s.done.push((d, e, px, "stop")); } }
        };
        let mut st = S::default();
        for q in m.iter().filter(|q| q.m > tk) {
            st.xon = st.pos.is_some();
            st.entry_on = st.n == 0;
            st.used = 0;
            let adv = st.pos.map_or(false, |z| z.0 < 0.0);
            minute(q, &mut st, &orders, &fill, adv);
        }
        // trades of this day: reversal and exit pairs were stored without entry day info; all on day i
        for (d, e, xp, why) in st.done.drain(..) { out.push(day_trade(d, i, e, i, xp, why, "stop", false)); }
        if let Some((d, e)) = st.pos.take() { out.push(day_trade(d, i, e, i, m[m.len() - 1].c, "end of day", "stop", false)); }
    }
    Some(out)
}
