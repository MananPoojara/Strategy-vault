//! Lane K48 (5 Nov 2026, decisions #216/#218; authors/titles-1030/K48.tsv): the rest of batch N1 (authors/batch-n/N1.tsv) on the
//! vault's 1-minute file (intraday_ep::day_bars for N-minute bars; the K46 minute walker n1intra::minute for orders inside a bar).
//! Four new cards here, two held cards converted (their catalogue rows stay in books/kftsm.rs and books/vdso.rs, run here):
//! - `fot08_fib_pivots` Lee Leibfarth, 'Fibonacci pivot points', Futures & Options Trader Feb 2008 pp.8-12 (FibPivot v1.0 code p.12).
//! - `sgbic_breakout` Sourabh Gandhi, 'Breakout Intraday Calculator' (Prasad/ALL BOOKS/Intraday-Using-Breakout.pdf, PDF pp.2-5).
//! - `rsrl_rlevels` Rich Saidenberg, R-Levels Trading System (.doc code, @SP.P 5-minute bars, Eastern clock), SPX only.
//! - `vkdcs_daycare` Vilar Kelly's Day Care System (S&P 500), Readme.txt code by L. E. Benade, SPX only (decision #218).
//! - `kftsm_zones` Kaufman TSM CD Ch18 'TSM(S) Zones.txt' (after J. T. Jackson); `vdso_supreme_order` Dasilva, The Supreme Order.
//! Independent reference: authors/titles-1030/K48w/k48_ref.py (written from the same sources).
use crate::books::n1intra::{minute, Ord};
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};
use crate::intraday::{day_trade, Bar1};

macro_rules! def {
    ($id:expr, $book:expr, $name:expr, $group:expr, $dir:expr, $rules:expr, $notes:expr) => {
        StrategyDef { id: $id, name: $name, group: $group, session: 0, dir: $dir, status: Status::Live, needs_volume: false,
            needs_vix: false, needs_series: &[], book: $book, rules: $rules, notes: $notes, claim: None }
    };
}
pub const MIN: &str = "A|Minute path [convention]: a stop or limit fills at its level, or at the minute's open when the minute opens through it; inside a minute the price runs open, the extreme nearer the open, the farther extreme, close (a tie: the extreme against the position first; flat: low first); orders live together fill in the order that path reaches them. A trade whose fill depends on that order inside one minute is counted as ambiguous (decision 175: held above 5%).";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![
        def!("fot08_fib_pivots", "fot08", "Fibonacci pivot points: countertrend at R1/S1, breakout at R2/S2 (5-minute)", "Futures & Options Trader 2008", "Long/Short",
            &["F|\"R2 = pivot + ((yesterday's high - yesterday's low) * 0.618); R1 = pivot + (... * 0.382); Pivot = (yesterday's high + yesterday's low + yesterday's close) / 3; S1 = pivot - (... * 0.382); S2 = pivot - (... * 0.618)\" (F&OT Feb 2008 p.9, Table 1)",
              "E|\"if TradeSwitch and time > 930 and time < 1300 and marketposition = 0 then begin if c < R1 then sellshort next bar at R1 limit; if c > S1 then buy next bar at S1 limit; if c > R1 and c < R2 then buy next bar at R2 stop; if c < S1 and c > S2 then sellshort next bar at S2 stop\" (FibPivot code, p.12); TradeSwitch = false once a position is held: one trade a day",
              "X|\"sell from entry (Buy_S1) next bar at Pivot limit; ... at S2 stop; sell from entry (Buy_R2) next bar at Pivot stop\"; shorts mirror (Pivot limit / R2 stop; Pivot stop); \"any open positions are closed at 4 p.m.\" (p.11), setexitonclose",
              "A|Read: \"TARGET: 5-Minute ES\": session-aligned 5-minute bars from the 1-minute file. The article takes yesterday's high/low/close from the 24-hour ES day (midnight-23:59 ET); the vault's series have the regular session only, so the vault's daily bar of the previous day is used [Declared adaptation, decision 216]. NSE: orders from bars stamped after the open to before open+210 minutes (the US 09:30-13:00 window) [Declared reading]. Entry orders live for one bar together, as TradeStation places them: an opposite one filling after the first reverses [Declared reading]. No fixed amounts: SPY, SPX, NIFTY 50, NIFTYBANK as a declared application (the article tested ES).", MIN],
            "The article prints no performance table (two chart examples, Dec 19 and Nov 23, 2007, before the minute history). No costs in the vault's run. Cash-index shorts are hypothetical."),
        def!("sgbic_breakout", "sgbic", "Breakout Intraday Calculator: yesterday's range x 0.45 / 0.75 from today's range (1-minute)", "Breakout Intraday Calculator (Gandhi)", "Long/Short",
            &["F|\"Step 4: Find the range of previous day. Step 5: Multiply the range for previous day with the two fractions (0.45 and 0.75). Mark this result as MF1 and MF2. Step 6: Now find current day's range\" (PDF p.2)",
              "E|\"Step 8: If current day's range is below MF1, then Upside Breakout = Today's low + MF1, Downside Breakout = Today's high - MF1. Step 9: If between MF1 and MF2: Today's low + MF2 / Today's high - MF2. Step 10: If current day's range is above MF2, then DO NOT TRADE. Step 11: buy above Upside Breakout and sell below Downside breakout\" (PDF p.3); \"use it after 20 min of market opening and before 90 min of market closing\" (PDF p.5)",
              "X|\"Step 12: Target should be 0.7%. Step 13: Stop loss should be 0.8% or if the underlying is trading below our buy price or above our sell price for more than 10 min, then square off the position\" (PDF p.3)",
              "A|Read: levels from today's running high/low up to the minute before (the breakout minute itself would widen the range past the band); MF1 <= range <= MF2 is the MF2 band; one trade a day; 'more than 10 min' = 10 consecutive 1-minute closes on the wrong side of the entry, out at that close; flat at the session close [Declared readings]. The article trades single NSE stocks (Reliance, HDFC); NIFTY 50, NIFTYBANK, SPY and SPX are a declared application (percent amounts only).", MIN],
            "The article's two examples (13 Apr 2010, Reliance and HDFC) are single stocks; no performance table. No costs in the vault's run. Cash-index shorts are hypothetical."),
        def!("rsrl_rlevels", "rsrl", "R-Levels: setup/entry levels from yesterday's range, 45-minute late exits (5-minute S&P)", "R-Levels Trading System (Saidenberg)", "Long/Short",
            &["F|\"Use @SP.P 5 minute bars ... Eastern Time: notbef = 1015, notaft = 1529\"; at a new day \"ssetup=htoday[1]+f1*(c[1]-ltoday[1]); senter=((1+f2)/2)*(htoday[1]+c[1])-(f2)*ltoday[1]; benter=((1+f2)/2)*(ltoday[1]+c[1])-(f2)*htoday[1]; bsetup=ltoday[1]-f1*(htoday[1]-c[1]); rfilter=htoday[1]-ltoday[1]>=rangemin\" (f1 .45, f2 0.1, rangemin 1.15, xdiv 3)",
              "E|Every 45 minutes from notbef: \"if ltoday<=bsetup and marketposition<1 then B2=benter-(bsetup-ltoday)/div; if htoday>=ssetup and marketposition>-1 then S2=senter+(htoday-ssetup)/div\"; B2 rounded up, S2 down to 0.10; \"if t>=notbef and t<notaft and startnow>=2 and rfilter and date>entrydate(1)\": \"Sell Short Next Bar S2 stop\" / \"Buy Next Bar B2 stop\" (B1/B3/S1/S3 are never set, so the printed comparisons reduce to these)",
              "X|\"if t>=notaft and t<>sess1endtime\": buy to cover / sell at entryprice +/- reverse stop and at H55[1]+.1 / L55[1]-.1 stop (the last 45-minute block's high/low); always \"MoneyMgmtStop Next Bar entryprice-reverse stop\"; reverse = 4.00 (\"ADJUSTMENT FOR CHANGE TO $250 PER POINT\", from 2 Nov 1997); no exit at the close: a position can stay overnight",
              "A|Read: session-aligned 5-minute bars of SPX from the 1-minute file, Eastern clock as printed; the day's last bar stands for Sess1EndTime (16:00 on the cash index, 16:15 on the pit contract); entrydate(1) = the entry date of the last closed position [Declared readings]. 4.00, 1.15 and 0.10 are S&P index points of the named contract: SPX only, not the futures contract (decision 196(2)).", MIN],
            "The .doc prints code only, no results. No costs in the vault's run. Cash-index shorts are hypothetical."),
        StrategyDef { status: Status::NotTestable("Coded on real SPX 1-minute bars (lane K48, decision #218), but 521 of 3,875 SPX trades (13.4%, 2008-2026; 408 of 2,596, 15.7%, on 2014-2025) depend on the order inside one minute: the 1.05-point fake-through, the trigger stop and the 1.60-point stop are often reached in the same minute. Decision #175 holds a rule above 5%. #177b (scaled SPY, 2014-2025) did not flag it (K48.tsv)."), ..def!("vkdcs_daycare", "vkdcs", "Day Care: yesterday's pivot triggers after a 1.05-point fake-through (5-minute S&P)", "Vilar Kelly's Day Care System", "Long/Short",
            &["F|At Sess1EndTime: \"Pivot8 =(2* (highest(high,Numb)+lowest(low,Numb) + close))/3; SellTrig = (pivot8)-low8; BuyTrig = (Pivot8)-High8; SellFThru = SellTrig + 105 points; BuyFThru = BuyTrig - 105 points\"; \"If Time = 935 then Begin BuyReady = False; SellReady = False; BuySet = False; SellSet = False\" (Readme.txt)",
              "E|\"If L crosses under BuyFThru then BuyReady = True and SellReady = False; If H crosses over SellFThru then SellReady = True and BuyReady = False; If BuyReady = True and C > BuyFThru then BuySet = True and SellSet = False; If SellReady = True and C > SellFThru then SellSet = True and BuySet = False\"; \"If BuySet = True and Time > 945 and Time <= 1530 and TradesToday(date) <= 1 then Buy at BuyTrig Stop\"; sell mirror at SellTrig stop",
              "X|\"ExitLong at EntryPrice - 160 points Stop\"; \"ExitShort at EntryPrice + 160 points Stop\"; \"If MarketPosition = 1 and Time = Sess1EndTime then ExitLong this bar on Close\" (short mirror)",
              "A|Read (decision 218): TradeStation runs 'BuyReady = True and SellReady = False' as ONE assignment, BuyReady = (True and (SellReady = False)); that literal behaviour, what the author's platform executed, is the default, and the evident two-assignment intent is option flags=1. 'points' = 0.01 S&P index points (TradeStation price points), so 105 / 160 points = 1.05 / 1.60 S&P points: SPX only. 5-minute bars (the 09:35 reset is the first 5-minute stamp), Eastern clock; the day's first bar is the 09:35 reset and its last bar Sess1EndTime; crosses = beyond the level now and at or on the other side on the bar before; TradesToday = entries made today; 'C > SellFThru' is kept as printed [Declared readings].", MIN],
            "No published results. No costs in the vault's run. Cash-index shorts are hypothetical.") },
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    Some(match id {
        "fot08_fib_pivots" => ("fot08_fib_pivots", "F&OT Fibonacci pivot points", "5-minute R1/S1 fade, R2/S2 breakout"),
        "sgbic_breakout" => ("sgbic", "Breakout Intraday Calculator", "1-minute range breakout"),
        "rsrl_rlevels" => ("rsrl", "R-Levels (Saidenberg)", "5-minute setup/entry levels"),
        "vkdcs_daycare" => ("vkdcs", "Day Care (Vilar Kelly)", "5-minute pivot fake-through"),
        _ => return None,
    })
}

fn pd(key: &'static str, label: &'static str, def: f64, min: f64, max: f64, step: f64) -> ParamDef { ParamDef { key, label, def, min, max, step } }

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    Some(match id {
        "fot08_fib_pivots" => vec![pd("f1", "R1/S1 multiple of yesterday's range", 0.382, 0.05, 2., 0.001), pd("f2", "R2/S2 multiple of yesterday's range", 0.618, 0.05, 3., 0.001)],
        "sgbic_breakout" => vec![pd("mf1", "MF1 multiple of yesterday's range", 0.45, 0.05, 2., 0.05), pd("mf2", "MF2 multiple of yesterday's range", 0.75, 0.05, 3., 0.05),
            pd("target", "Target %", 0.7, 0.1, 10., 0.1), pd("stop", "Stop %", 0.8, 0.1, 10., 0.1), pd("wrong", "Minutes on the wrong side of the entry", 10., 1., 120., 1.)],
        "rsrl_rlevels" => vec![pd("reverse", "Reverse / money-management stop from 2 Nov 1997 (S&P points; 2.00 before, as printed)", 4.0, 0.5, 50., 0.25), pd("rangemin", "Yesterday's range at least (S&P points)", 1.15, 0., 50., 0.05),
            pd("f1", "f1", 0.45, 0., 2., 0.05), pd("f2", "f2", 0.1, 0., 1., 0.05), pd("xdiv", "xdiv", 3., 1., 10., 1.)],
        "vkdcs_daycare" => vec![pd("flags", "Flag lines: 0 = TradeStation literal (one assignment), 1 = two assignments (intent)", 0., 0., 1., 1.),
            pd("thru", "Fake-through beyond the trigger (S&P points; 105 points)", 1.05, 0.05, 20., 0.05), pd("stop", "Stop from entry (S&P points; 160 points)", 1.6, 0.05, 50., 0.05)],
        "kftsm_zones" => vec![pd("lag", "Zones from: 0 = yesterday's daily bar (the text), 1 = the previous session's zones (TradeStation's avg[1] on a daily Data2)", 0., 0., 1., 1.)],
        "vdso_supreme_order" => vec![pd("neutral", "Inside (neutral) bars: 1 = leave stops and the reference bar unchanged (p.7), 0 = advance every bar", 1., 0., 1., 1.)],
        _ => return None,
    })
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if params(id).is_none() || to >= x.c.len() || from > to { return None; }
    let g = |k: &str| super::jb624::getp(id, p, k);
    Some(match id {
        "fot08_fib_pivots" => fib(x, from, to, g("f1"), g("f2")),
        "sgbic_breakout" => gandhi(x, from, to, g("mf1"), g("mf2"), g("target") / 100.0, g("stop") / 100.0, g("wrong").round().max(1.0) as u32),
        "rsrl_rlevels" => rlevels(x, from, to, g("reverse"), g("rangemin"), g("f1"), g("f2"), g("xdiv").max(1.0)),
        "vkdcs_daycare" => daycare(x, from, to, g("flags") >= 0.5, g("thru"), g("stop")),
        "kftsm_zones" => zones(x, from, to, g("lag") >= 0.5),
        _ => supreme(x, from, to, g("neutral") >= 0.5),
    })
}

/// One N-minute bar of the continuous series (daily row, stamp, OHLC, its minutes, first/last bar of its day).
struct B { i: usize, s: u16, h: f64, l: f64, c: f64, m: Vec<Bar1>, first: bool, last: bool }

fn series(x: &Ctx, from: usize, to: usize, n: u16) -> Vec<B> {
    let mut out = vec![];
    for i in from..=to {
        let Some((_, _, m, v)) = crate::intraday_ep::day_bars(x, i, n) else { continue };
        let k = v.len();
        out.extend(v.into_iter().enumerate().map(|(j, z)| B { i, s: z.e, h: z.h, l: z.l, c: z.c, m: m[z.a..z.b].to_vec(), first: j == 0, last: j + 1 == k }));
    }
    out
}

/// A position: direction, entry price, entry tag, entry day row, entry kind.
#[derive(Clone, Copy)]
struct P { d: f64, ep: f64, tag: u8, ei: usize, kind: &'static str }
/// Walker state: the position, trades closed in the current minute(s), orders placed for this bar, entry orders already
/// filled in it (bit mask), the current day row, entries today, and two free levels (vdso's stops).
#[derive(Clone, Default)]
struct St { pos: Option<P>, done: Vec<(P, usize, f64, &'static str)>, live: Vec<Ord>, used: u32, i: usize, n: u32, a: f64, b: f64 }
impl St {
    fn dir(&self) -> f64 { self.pos.map_or(0.0, |p| p.d) }
    /// Entry: ignored in the held direction; an opposite position is closed at the same price ("reverse").
    fn enter(&mut self, d: f64, px: f64, tag: u8, kind: &'static str) {
        if self.dir() == d { return; }
        if let Some(p) = self.pos.take() { self.done.push((p, self.i, px, "reverse")); }
        self.pos = Some(P { d, ep: px, tag, ei: self.i, kind });
        self.n += 1;
    }
    fn exit(&mut self, px: f64, why: &'static str) { if let Some(p) = self.pos.take() { self.done.push((p, self.i, px, why)); } }
    fn flush(&mut self, out: &mut Vec<Trade>) { for (p, xi, xp, why) in self.done.drain(..) { out.push(day_trade(p.d, p.ei, p.ep, xi, xp, why, p.kind, false)); } }
    fn walk(&mut self, q: &Bar1, orders: &dyn Fn(&St) -> Vec<Ord>, fill: &dyn Fn(&mut St, u8, f64)) { let adv = self.dir() < 0.0; minute(q, self, orders, fill, adv); }
    fn finish(mut self, out: &mut Vec<Trade>, i: usize, px: f64) {
        self.flush(out);
        if let Some(p) = self.pos.take() { out.push(day_trade(p.d, p.ei, p.ep, i, px, "open", p.kind, true)); }
    }
}
fn ord(up: bool, lv: f64, tag: u8) -> Ord { Ord { up, lv, tag } }

/// Leibfarth's Fibonacci pivots: entry tags 1 SellShort_R1 (limit), 2 Buy_S1 (limit), 3 Buy_R2 (stop), 4 SellShort_S2 (stop);
/// exit tags 10 Pivot limit, 11 S2/R2 stop, 12 Pivot stop. Entry orders live while flat; exits only for the held entry.
fn fib(x: &Ctx, from: usize, to: usize, f1: f64, f2: f64) -> Vec<Trade> {
    let mut out = vec![];
    let orders = |s: &St| -> Vec<Ord> { s.live.iter().copied().filter(|o| if o.tag < 10 { s.used & (1 << o.tag) == 0 && s.dir() != if o.tag == 2 || o.tag == 3 { 1.0 } else { -1.0 } } else { s.pos.is_some() }).collect() };
    let fill = |s: &mut St, t: u8, px: f64| match t {
        1 | 4 => { s.used |= 1 << t; s.enter(-1.0, px, t, if t == 1 { "limit" } else { "stop" }) }
        2 | 3 => { s.used |= 1 << t; s.enter(1.0, px, t, if t == 2 { "limit" } else { "stop" }) }
        10 => s.exit(px, "pivot target"),
        11 => s.exit(px, "stop"),
        _ => s.exit(px, "pivot stop"),
    };
    for i in from.max(1)..=to {
        let Some((open, _, m, b)) = crate::intraday_ep::day_bars(x, i, 5) else { continue };
        let y = &x.r[i - 1];
        let (pv, rg) = ((y.high + y.low + y.close) / 3.0, y.high - y.low);
        let (r1, s1, r2, s2) = (pv + rg * f1, pv - rg * f1, pv + rg * f2, pv - rg * f2);
        let (mut st, mut sw, mut next) = (St { i, ..St::default() }, true, vec![]);
        for (k, z) in b.iter().enumerate() {
            st.live = std::mem::take(&mut next);
            st.used = 0;
            for q in &m[z.a..z.b] { st.walk(q, &orders, &fill); }
            if k + 1 == b.len() { st.exit(z.c, "end of day"); break; }
            if sw && z.e > open && z.e < open + 210 && st.pos.is_none() {
                if z.c < r1 { next.push(ord(true, r1, 1)); }
                if z.c > s1 { next.push(ord(false, s1, 2)); }
                if z.c > r1 && z.c < r2 { next.push(ord(true, r2, 3)); }
                if z.c < s1 && z.c > s2 { next.push(ord(false, s2, 4)); }
            }
            if let Some(p) = st.pos {
                sw = false;
                next.extend(match p.tag { 2 => vec![ord(true, pv, 10), ord(false, s2, 11)], 1 => vec![ord(false, pv, 10), ord(true, r2, 11)],
                    3 => vec![ord(false, pv, 12)], _ => vec![ord(true, pv, 12)] });
            }
        }
        st.flush(&mut out);
    }
    out
}

/// Gandhi's breakout calculator on 1-minute bars: entry tags 1 buy stop, 2 sell stop; exits 10 target, 11 stop.
fn gandhi(x: &Ctx, from: usize, to: usize, k1: f64, k2: f64, tgt: f64, stp: f64, wrong: u32) -> Vec<Trade> {
    let mut out = vec![];
    let orders = |s: &St| -> Vec<Ord> {
        match s.pos {
            None if s.used == 0 => s.live.clone(),
            None => vec![],
            Some(p) if p.d > 0.0 => vec![ord(true, p.ep * (1.0 + tgt), 10), ord(false, p.ep * (1.0 - stp), 11)],
            Some(p) => vec![ord(false, p.ep * (1.0 - tgt), 10), ord(true, p.ep * (1.0 + stp), 11)],
        }
    };
    let fill = |s: &mut St, t: u8, px: f64| match t {
        1 => { s.used = 1; s.enter(1.0, px, 1, "stop") }
        2 => { s.used = 1; s.enter(-1.0, px, 2, "stop") }
        10 => s.exit(px, "target"),
        _ => s.exit(px, "stop"),
    };
    for i in from.max(1)..=to {
        let Some(m) = crate::intraday_w2b::day(x, i) else { continue };
        let t = &x.r[i].ticker;
        let (open, close) = (crate::intraday::session_open(t, &x.r[i].date), crate::intraday_ep::session_close(t));
        let rg = x.r[i - 1].high - x.r[i - 1].low;
        let (mf1, mf2) = (k1 * rg, k2 * rg);
        let (mut st, mut hh, mut ll, mut cnt) = (St { i, ..St::default() }, f64::NAN, f64::NAN, 0u32);
        for (j, q) in m.iter().enumerate() {
            st.live.clear();
            if j > 0 && st.used == 0 && q.m >= open + 20 && q.m < close - 90 {
                let r = hh - ll;
                let f = if r < mf1 { mf1 } else if r <= mf2 { mf2 } else { f64::NAN };
                if f.is_finite() { st.live = vec![ord(true, ll + f, 1), ord(false, hh - f, 2)]; }
            }
            st.walk(q, &orders, &fill);
            if j == 0 { hh = q.h; ll = q.l; } else { hh = hh.max(q.h); ll = ll.min(q.l); }
            if let Some(p) = st.pos {
                cnt = if (p.d > 0.0 && q.c < p.ep) || (p.d < 0.0 && q.c > p.ep) { cnt + 1 } else { 0 };
                if cnt >= wrong { st.exit(q.c, "wrong side of the entry"); }
            }
            if j + 1 == m.len() { st.exit(q.c, "end of day"); }
        }
        st.flush(&mut out);
    }
    out
}

/// Saidenberg's R-Levels on 5-minute SPX bars (one series; positions may stay overnight). Entry tags 1 sell short S2 stop,
/// 2 buy B2 stop; exit tags (short / long): 11 / 12 reverse stop, 13 / 14 late exit, 15 / 16 money-management stop.
fn rlevels(x: &Ctx, from: usize, to: usize, rev: f64, rmin: f64, f1: f64, f2: f64, div: f64) -> Vec<Trade> {
    let b = series(x, from, to, 5);
    let mut out = vec![];
    let orders = |s: &St| -> Vec<Ord> { s.live.iter().copied().filter(|o| match o.tag {
        1 => s.used & 2 == 0 && s.dir() != -1.0, 2 => s.used & 4 == 0 && s.dir() != 1.0,
        11 | 13 | 15 => s.dir() < 0.0, _ => s.dir() > 0.0 }).collect() };
    let fill = |s: &mut St, t: u8, px: f64| match t {
        1 => { s.used |= 2; s.enter(-1.0, px, 1, "stop") }
        2 => { s.used |= 4; s.enter(1.0, px, 2, "stop") }
        11 | 12 => s.exit(px, "reverse stop"),
        13 | 14 => s.exit(px, "late exit"),
        _ => s.exit(px, "money-management stop"),
    };
    let (nb, na) = (615u16, 929u16);
    let (mut st, mut next) = (St::default(), vec![]);
    let (mut ht, mut lt, mut start, mut rf) = (9999.0f64, 0.0f64, 0u32, false);
    let (mut ss, mut bs, mut se, mut be, mut s2, mut b2) = (0.0, 0.0, 0.0, 0.0, 0.0f64, 99999.0f64);
    let (mut h0, mut l0, mut h1, mut l1) = (0.0f64, 99999.0f64, 0.0f64, 99999.0f64);
    let mut last_entry: Option<usize> = None; // entry row of the last closed position (entrydate(1))
    for k in 0..b.len() {
        let z = &b[k];
        st.i = z.i;
        st.live = std::mem::take(&mut next);
        st.used = 0;
        for q in &z.m { st.walk(q, &orders, &fill); }
        if let Some(d) = st.done.last() { last_entry = Some(d.0.ei); }
        st.flush(&mut out);
        if k > 0 && z.first {
            start += 1;
            let (hp, lp, cp) = (ht, lt, b[k - 1].c);
            ss = hp + f1 * (cp - lp);
            se = ((1.0 + f2) / 2.0) * (hp + cp) - f2 * lp;
            be = ((1.0 + f2) / 2.0) * (lp + cp) - f2 * hp;
            bs = lp - f1 * (hp - cp);
            ht = z.h; lt = z.l;
            rf = hp - lp >= rmin;
            s2 = 0.0; b2 = 99999.0;
        }
        if z.h > ht { ht = z.h; }
        if z.l < lt { lt = z.l; }
        if z.h > h0 { h0 = z.h; }
        if z.l < l0 { l0 = z.l; }
        let t = z.s;
        let mp = st.dir();
        // "if d<971102 then reverse=2.00 else reverse=4.00" and the 0.05 / 0.10 rounding and late offsets (the contract's tick)
        let (rev, q, lo) = if x.r[z.i].date.as_str() < "1997-11-02" { (2.0, 20.0, 0.05) } else { (rev, 10.0, 0.1) };
        if z.last || t == nb || (t > nb && (t - nb) % 45 == 0) {
            h1 = h0; l1 = l0; h0 = 0.0; l0 = 99999.0;
            if lt <= bs && mp < 1.0 { b2 = be - (bs - lt) / div; }
            if ht >= ss && mp > -1.0 { s2 = se + (ht - ss) / div; }
        }
        if b2 < 99999.0 && b2 != (b2 * q).trunc() / q { b2 = (b2 * q + 0.99999).trunc() / q; }
        if s2 > 0.0 { s2 = (s2 * q).trunc() / q; }
        if t >= nb && t < na && start >= 2 && rf && last_entry.map_or(true, |e| e < z.i) {
            if s2 > 0.0 && mp > -1.0 { next.push(ord(false, s2, 1)); }
            if b2 < 99999.0 && mp < 1.0 { next.push(ord(true, b2, 2)); }
        }
        let ep = st.pos.map_or(0.0, |p| p.ep);
        if t >= na && !z.last {
            if mp < 0.0 { next.push(ord(true, ep + rev, 11)); }
            if mp > 0.0 { next.push(ord(false, ep - rev, 12)); }
            next.push(ord(true, h1 + lo, 13));
            next.push(ord(false, l1 - lo, 14));
        }
        if mp > 0.0 { next.push(ord(false, ep - rev, 16)); }
        if mp < 0.0 { next.push(ord(true, ep + rev, 15)); }
    }
    match b.last() { Some(z) => st.finish(&mut out, z.i, z.c), None => {} }
    out
}

/// Vilar Kelly's Day Care on 5-minute SPX bars. Entry tags 1 buy stop at BuyTrig, 2 sell short stop at SellTrig; exit tags
/// 10 long stop, 11 short stop; MOC at the day's last bar.
fn daycare(x: &Ctx, from: usize, to: usize, two: bool, thru: f64, stp: f64) -> Vec<Trade> {
    let b = series(x, from, to, 5);
    let mut out = vec![];
    let orders = |s: &St| -> Vec<Ord> { s.live.iter().copied().filter(|o| match o.tag {
        1 => s.used & 2 == 0 && s.dir() != 1.0, 2 => s.used & 4 == 0 && s.dir() != -1.0, 10 => s.dir() > 0.0, _ => s.dir() < 0.0 }).collect() };
    let fill = |s: &mut St, t: u8, px: f64| match t {
        1 => { s.used |= 2; s.enter(1.0, px, 1, "stop") }
        2 => { s.used |= 4; s.enter(-1.0, px, 2, "stop") }
        _ => s.exit(px, "stop"),
    };
    let (mut st, mut next) = (St::default(), vec![]);
    let (mut bt, mut sl, mut bf, mut sf) = (f64::NAN, f64::NAN, f64::NAN, f64::NAN);
    let (mut br, mut sr, mut bset, mut sset) = (false, false, false, false);
    let (mut dh, mut dl) = (f64::NAN, f64::NAN);
    for k in 0..b.len() {
        let z = &b[k];
        st.i = z.i;
        if z.first { st.n = 0; }
        st.live = std::mem::take(&mut next);
        st.used = 0;
        for q in &z.m { st.walk(q, &orders, &fill); }
        st.flush(&mut out);
        if z.first { dh = z.h; dl = z.l; } else { dh = dh.max(z.h); dl = dl.min(z.l); }
        let (pbf, psf) = (bf, sf);
        if z.last {
            let p8 = 2.0 * (dh + dl + z.c) / 3.0;
            sl = p8 - dl; bt = p8 - dh; sf = sl + thru; bf = bt - thru;
        }
        if z.first { br = false; sr = false; bset = false; sset = false; }
        if k > 0 {
            if z.l < bf && b[k - 1].l >= pbf { if two { br = true; sr = false; } else { br = !sr; } }
            if z.h > sf && b[k - 1].h <= psf { if two { sr = true; br = false; } else { sr = !br; } }
        }
        if br && z.c > bf { if two { bset = true; sset = false; } else { bset = !sset; } }
        if sr && z.c > sf { if two { sset = true; bset = false; } else { sset = !bset; } }
        let t = z.s;
        if bset && t > 585 && t <= 930 && st.n <= 1 { next.push(ord(true, bt, 1)); }
        if sset && t > 585 && t <= 930 && st.n <= 1 { next.push(ord(false, sl, 2)); }
        match st.pos { Some(p) if p.d > 0.0 => next.push(ord(false, p.ep - stp, 10)), Some(p) => next.push(ord(true, p.ep + stp, 11)), None => {} }
        if z.last { st.exit(z.c, "end of day"); st.flush(&mut out); next.clear(); }
    }
    out
}

/// Kaufman / Jackson zones on 10-minute bars (one series; positions carry over days). Exit tags 10 low2 / high2 stop,
/// 11 midpoint limit.
fn zones(x: &Ctx, from: usize, to: usize, lag: bool) -> Vec<Trade> {
    let b = series(x, from, to, 10);
    let mut out = vec![];
    let orders = |s: &St| -> Vec<Ord> { if s.pos.is_some() { s.live.clone() } else { vec![] } };
    let fill = |s: &mut St, t: u8, px: f64| s.exit(px, if t == 10 { "stop" } else { "midpoint target" });
    let (mut st, mut next) = (St::default(), vec![]);
    let (mut zz, mut prev): (Option<[f64; 5]>, Option<[f64; 5]>) = (None, None);
    for k in 0..b.len() {
        let z = &b[k];
        st.i = z.i;
        st.live = std::mem::take(&mut next);
        for q in &z.m { st.walk(q, &orders, &fill); }
        if z.first && z.i >= 1 {
            let y = &x.r[z.i - 1];
            let avg = (y.high + y.low + y.close) / 3.0;
            let (h1, l1) = (2.0 * avg - y.low, 2.0 * avg - y.high);
            let cur = [avg + y.high - y.low, h1, l1, avg + y.low - y.high, (h1 + l1) / 2.0];
            zz = if lag { prev } else { Some(cur) };
            prev = Some(cur);
        }
        let Some([h2, h1, l1, l2, mid]) = zz else { continue };
        if k > 0 {
            if z.l <= l1 && b[k - 1].l > l1 { st.enter(1.0, z.c, 0, "close"); }
            else if z.h >= h1 && b[k - 1].h < h1 { st.enter(-1.0, z.c, 0, "close"); }
        }
        st.flush(&mut out);
        match st.pos { Some(p) if p.d > 0.0 => next = vec![ord(false, l2, 10), ord(true, mid, 11)], Some(_) => next = vec![ord(true, h2, 10), ord(false, mid, 11)], None => {} }
    }
    match b.last() { Some(z) => st.finish(&mut out, z.i, z.c), None => {} }
    out
}

/// The Supreme Order on daily bars, sequenced by the day's minutes (a day inside the minute history without minutes runs on
/// its daily bar as one path). Always in after the first fill: tag 1 buy stop, tag 2 sell stop, each a reversal.
fn supreme(x: &Ctx, from: usize, to: usize, neutral: bool) -> Vec<Trade> {
    let mut out = vec![];
    let days: Vec<usize> = (from..=to).filter(|&i| crate::intraday_w2b::day(x, i).is_some()).collect();
    let (Some(&a), Some(&e)) = (days.first(), days.last()) else { return out };
    let tick = if x.r[a].ticker.starts_with("NIFTY") { 0.05 } else { 0.01 };
    let mut st = St::default();
    let mut rf: Option<(f64, f64)> = None; // the reference (immediate past) bar's high and low
    for i in a..=e {
        let r = &x.r[i];
        let path = crate::intraday_w2b::day(x, i).unwrap_or_else(|| vec![Bar1 { m: 0, o: r.open, h: r.high, l: r.low, c: r.close }]);
        let dh = path.iter().map(|q| q.h).fold(f64::NEG_INFINITY, f64::max);
        let dl = path.iter().map(|q| q.l).fold(f64::INFINITY, f64::min);
        st.i = i;
        if let Some((rh, rl)) = rf {
            let orders = |s: &St| -> Vec<Ord> { match s.pos { None => vec![ord(true, rh + tick, 1), ord(false, rl - tick, 2)],
                Some(p) if p.d > 0.0 => vec![ord(false, s.a, 2)], Some(_) => vec![ord(true, s.b, 1)] } };
            let fill = |s: &mut St, t: u8, px: f64| if t == 1 { s.enter(1.0, px, 1, "stop"); s.a = rl - tick; } else { s.enter(-1.0, px, 2, "stop"); s.b = rh + tick; };
            for q in &path { st.walk(q, &orders, &fill); }
            st.flush(&mut out);
        }
        if !(neutral && rf.map_or(false, |(rh, rl)| dh <= rh && dl >= rl)) {
            match st.pos { Some(p) if p.d > 0.0 => st.a = st.a.max(dl - tick), Some(_) => st.b = st.b.min(dh + tick), None => {} }
            rf = Some((dh, dl));
        }
    }
    let c = crate::intraday_w2b::day(x, e).map_or(x.r[e].close, |m| m[m.len() - 1].c);
    st.finish(&mut out, e, c);
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    fn q(m: u16, o: f64, h: f64, l: f64, c: f64) -> Bar1 { Bar1 { m, o, h, l, c } }
    #[test]
    fn state_reverses_ignores_same_side_and_flushes() {
        let mut s = St { i: 3, ..St::default() };
        s.enter(1.0, 100.0, 2, "limit");
        s.enter(1.0, 101.0, 3, "stop");
        s.i = 4;
        s.enter(-1.0, 99.0, 1, "limit");
        s.exit(98.0, "pivot target");
        let mut out = vec![];
        s.flush(&mut out);
        assert_eq!(out.iter().map(|t| (t.dir, t.legs[0].i, t.legs[0].px, t.exit_i, t.exit_px, t.reason.as_str(), t.entry_kind)).collect::<Vec<_>>(),
            vec![(1.0, 3, 100.0, 4, 99.0, "reverse", "limit"), (-1.0, 4, 99.0, 4, 98.0, "pivot target", "limit")]);
        assert_eq!(s.n, 2);
    }
    #[test]
    fn supreme_order_stop_flips_twice_inside_one_minute_path() {
        // long with the sell stop at 9.99 (reference bar 10..11, tick 0.01): a minute 10.5 -> 9.5 -> 11.5 -> 11 reverses short at
        // 9.99 and back long at 11.01 (the new buy stop one tick above the reference high)
        let (rh, rl, tick) = (11.0, 10.0, 0.01);
        let mut s = St { a: rl - tick, ..St::default() };
        s.enter(1.0, 10.8, 1, "stop");
        let orders = |s: &St| -> Vec<Ord> { match s.pos { None => vec![], Some(p) if p.d > 0.0 => vec![ord(false, s.a, 2)], Some(_) => vec![ord(true, s.b, 1)] } };
        let fill = |s: &mut St, t: u8, px: f64| if t == 1 { s.enter(1.0, px, 1, "stop"); s.a = rl - tick; } else { s.enter(-1.0, px, 2, "stop"); s.b = rh + tick; };
        s.walk(&q(600, 10.5, 11.5, 9.5, 11.0), &orders, &fill);
        let v: Vec<(f64, f64)> = s.done.iter().map(|d| (d.0.d, (d.2 * 100.0).round() / 100.0)).collect();
        assert_eq!(v, vec![(1.0, 9.99), (-1.0, 11.01)]);
        assert_eq!(s.dir(), 1.0);
    }
    /// The reference comparison (run by hand): VAULT_US=<us_data.csv> VAULT_IDX=<idx_data.csv> VAULT_M1_DIR=<data/intraday>
    /// K48_OUT=<csv> cargo test --release k48_dump -- --ignored; then authors/titles-1030/K48w/cmp_dump.py.
    #[test]
    #[ignore]
    fn k48_dump() {
        use std::io::Write;
        let mut f = std::io::BufWriter::new(std::fs::File::create(std::env::var("K48_OUT").unwrap()).unwrap());
        for (file, t) in [("VAULT_US", "SPY"), ("VAULT_US", "SPX"), ("VAULT_IDX", "NIFTY 50"), ("VAULT_IDX", "NIFTYBANK")] {
            let rows = crate::load_csv(&std::env::var(file).unwrap()).unwrap().remove(t).unwrap();
            let mut x = Ctx::new(&rows, &rows, &[]);
            x.minutes = Some(Box::leak(Box::new(crate::intraday::Minutes::load(&std::env::var("VAULT_M1_DIR").unwrap(), t).unwrap())));
            let from = rows.partition_point(|r| r.date.as_str() < x.minutes.unwrap().first());
            let opts: &[(&str, &[(&str, f64)])] = &[("fot08_fib_pivots", &[]), ("sgbic_breakout", &[]), ("kftsm_zones", &[]), ("kftsm_zones", &[("lag", 1.)]),
                ("vdso_supreme_order", &[]), ("vdso_supreme_order", &[("neutral", 0.)]), ("rsrl_rlevels", &[]), ("vkdcs_daycare", &[]), ("vkdcs_daycare", &[("flags", 1.)])];
            for (id, kv) in opts {
                if ["rsrl_rlevels", "vkdcs_daycare"].contains(id) && t != "SPX" { continue; }
                let mut p = Params::new();
                for (k, v) in kv.iter() { p.insert(k.to_string(), *v); }
                let tag = kv.iter().map(|(k, v)| format!("{k}={v}")).collect::<Vec<_>>().join(";");
                for z in crate::books::run(id, &x, from, rows.len() - 1, &p).unwrap() {
                    writeln!(f, "{id}{}{tag},{t},{},{},{},{},{},\"{}\",{}", if tag.is_empty() { "" } else { "|" }, z.dir, rows[z.legs[0].i].date, z.legs[0].px, rows[z.exit_i].date, z.exit_px, z.reason, z.open).unwrap();
                }
            }
        }
    }
}
