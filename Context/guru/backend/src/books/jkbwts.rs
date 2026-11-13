//! Joe Krutsinger, "Building Winning Trading Systems" seminar manual (Dec 1997; material from his
//! "Trading Systems: Secrets of the Masters", Irwin 1997, + the Nov 1997 Omega Research "JK RainMaker" article).
//! Book id `jkbwts`. Source copy: Books/Joe Krutsinger/Building Winning Trading Systems/Manual.pdf (82 scanned pages;
//! page refs below are the printed page numbers). Transcription, testability table, oracle: library/jkbwts/.
//!
//! Every Live strategy is a literal port of the printed TradeStation 4 code / Quick System sheet. Evaluation on daily
//! bars (shared by the other Krutsinger modules jk36k / jkbk, which import `sim` from here):
//! * code runs at each bar's close; orders it places act on the NEXT bar only;
//! * market orders fill at the open; a stop/limit already through the open fills at the open, otherwise at its price;
//! * intrabar path: open -> the extreme nearer the open -> the other extreme -> close (tie: low first). Orders fill in
//!   the order the path reaches them; a money-management stop is armed from the entry fill (also on the entry bar);
//! * Buy/Sell reverse an opposite position, never pyramid; exit orders act only on the position held when issued;
//! * "Close all trades at end of day session" = exit at that bar's close.
//! Dollar money-management stops become index points: stop_usd / pt_usd, pt_usd = the book contract's $ per point
//! (S&P 500 and NYSE Composite futures $500 in 1997). Same number of points on NIFTY (instrument swap only).
//! Systems built for currency, bond, crude-oil or gold futures are NeedsData: the vault has no such futures data.
use crate::connors::{claim, Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "jkbwts";
/// Admitted ids (authors/krutsinger/admission/SOURCE-CONTRACT.md).
pub const REVIEWED: &[&str] = &["jkbwts_dinapoli5", "jkbwts_rainmaker", "jkbwts_buymon1400" /* intraday wave 2b: daily bars decide every trade */];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

// ------------------------------------------------------------------ shared daily simulator

#[derive(Clone, Copy, PartialEq, Debug)]
pub(crate) enum Ty { Mkt, Stop, Limit }

/// An order for the next bar. entry: `dir` = side to enter. exit: `dir` = side of the position to close.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Ord { pub entry: bool, pub dir: i8, pub ty: Ty, pub px: f64, pub tag: &'static str, pid: u64 }
pub(crate) fn entry(dir: i8, ty: Ty, px: f64, tag: &'static str) -> Ord { Ord { entry: true, dir, ty, px, tag, pid: 0 } }
pub(crate) fn exit(dir: i8, ty: Ty, px: f64, tag: &'static str) -> Ord { Ord { entry: false, dir, ty, px, tag, pid: 0 } }

#[derive(Clone, Debug)]
pub(crate) struct Pos { pub dir: i8, pub i: usize, pub px: f64, id: u64, kind: &'static str }

/// TradeStation DayOfWeek (0 = Sunday .. 5 = Friday, 6 = Saturday) of "YYYY-MM-DD".
pub(crate) fn dow(d: &str) -> i64 {
    let y: i64 = d[0..4].parse().unwrap_or(1970); let m: i64 = d[5..7].parse().unwrap_or(1); let dd: i64 = d[8..10].parse().unwrap_or(1);
    let (yy, mm) = if m <= 2 { (y - 1, m + 9) } else { (y, m - 3) };
    let era = yy.div_euclid(400); let yoe = yy - era * 400;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + (153 * mm + 2) / 5 + dd - 1;
    (era * 146097 + doe - 719468 + 4).rem_euclid(7) // 1970-01-01 was a Thursday (4)
}
/// TradeStation Average: exact window sum each bar (equality tests stay exact).
pub(crate) fn avg(v: &[f64], n: usize) -> Vec<f64> {
    let mut out = vec![f64::NAN; v.len()];
    if n == 0 { return out; }
    for i in n - 1..v.len() { out[i] = v[i + 1 - n..=i].iter().sum::<f64>() / n as f64; }
    out
}

#[derive(Clone)]
struct St<'a> { x: &'a Ctx<'a>, pos: Option<Pos>, out: Vec<Trade>, stop: f64, ids: u64, entries_on_bar: usize, od: crate::intraday_ep6::Day } // od: EP6 (#243, lane K63) mark of the open position
impl St<'_> {
    fn close(&mut self, i: usize, px: f64, reason: &str, sig: usize) {
        if let Some(p) = self.pos.take() {
            let mut t = Trade { dir: p.dir as f64, legs: vec![Leg { i: p.i, px: p.px, w: 1.0 }], exit_i: i, exit_px: px,
                reason: reason.into(), entry_kind: p.kind, signal_i: sig, open: false, exits: vec![] };
            crate::intraday_ep6::mark(&mut t, self.od);
            self.out.push(t);
        }
    }
    fn open(&mut self, i: usize, dir: i8, px: f64, kind: &'static str) {
        self.ids += 1; self.entries_on_bar += 1;
        self.pos = Some(Pos { dir, i, px, id: self.ids, kind });
        self.od = crate::intraday_ep6::Day::Plain;
    }
    /// Fill order `o` at `px` on bar i. Returns false if it does not apply (no matching position / same-side entry).
    fn fill(&mut self, i: usize, o: &Ord, px: f64) -> bool {
        let kind = match o.ty { Ty::Mkt => "open", Ty::Stop => "stop", Ty::Limit => "limit" };
        if o.entry {
            match &self.pos { Some(p) if p.dir == o.dir => return false, Some(_) => self.close(i, px, &format!("Reversed by {}", o.tag), i), None => {} }
            self.open(i, o.dir, px, kind);
            true
        } else {
            match &self.pos { Some(p) if p.dir == o.dir && p.id == o.pid => {} _ => return false }
            self.close(i, px, o.tag, if o.ty == Ty::Mkt { i.saturating_sub(1) } else { i });
            true
        }
    }
    fn prot(&self) -> Option<(i8, f64, u64)> {
        let p = self.pos.as_ref()?;
        (self.stop > 0.0).then(|| (p.dir, p.px - p.dir as f64 * self.stop, p.id))
    }
    fn bar(&mut self, i: usize, mut ords: Vec<Ord>) {
        let (o, h, l) = (self.x.o[i], self.x.h[i], self.x.l[i]);
        self.entries_on_bar = 0;
        // 1. market orders at the open: exits first, then entries
        ords.sort_by_key(|q| (q.ty != Ty::Mkt, q.entry));
        let mut live = vec![];
        for q in ords {
            if q.ty == Ty::Mkt { self.fill(i, &q, o); } else { live.push(q); }
        }
        self.walk(i, o, h, l, &mut live);
    }
    /// EP6 (#243, lane K63): the same order walk on explicit segments. No segments = the open (orders already through it
    /// fill at the open); on a segment the nearest level on the way fills at its level, on a gap segment at the gap's end.
    fn walk_segs(&mut self, i: usize, live: &mut Vec<Ord>, segs: &[crate::intraday_ep6::Seg]) -> Vec<crate::intraday_ep6::Ev> {
        let mut ev = vec![];
        let open = [crate::intraday_ep6::Seg { a: self.x.o[i], b: self.x.o[i], gap: false }];
        let first = segs.is_empty();
        for g in if first { &open[..] } else { segs } {
            let (mut at, to) = (g.a, g.b);
            loop {
                let up = to > at;
                let mut best: Option<(f64, f64, usize, bool)> = None; // (level, fill, index or MAX for the MM stop, rising)
                let mut consider = |px: f64, k: usize, buy_side: bool, is_stop: bool| {
                    let trig_up = buy_side == is_stop;
                    let hit = if first { ((trig_up && px <= at) || (!trig_up && px >= at)).then_some(at) }
                        else if trig_up && up && px > at && px <= to { Some(px) }
                        else if !trig_up && !up && px < at && px >= to { Some(px) } else { None };
                    if let Some(f) = hit {
                        if best.map_or(true, |(b, _, _, _)| (f - at).abs() < (b - at).abs()) { best = Some((f, if g.gap && !first { to } else { f }, k, trig_up)); }
                    }
                };
                for (k, q) in live.iter().enumerate() {
                    // only orders that apply to the position now (an exit of the position held at the prior close, an entry
                    // not on the side already held): the others neither fill nor take a turn
                    let applies = if q.entry { self.pos.as_ref().map_or(true, |p| p.dir != q.dir) } else { self.pos.as_ref().is_some_and(|p| p.dir == q.dir && p.id == q.pid) };
                    if !applies { continue; }
                    let buy_side = if q.entry { q.dir > 0 } else { q.dir < 0 };
                    consider(q.px, k, buy_side, q.ty == Ty::Stop);
                }
                if let Some((d, px, _)) = self.prot() { consider(px, usize::MAX, d < 0, true); }
                let Some((lv, px, k, rising)) = best else { break };
                ev.push(crate::intraday_ep6::Ev { k: if k == usize::MAX { u8::MAX } else { k as u8 }, px, up: rising });
                if k == usize::MAX { self.close(i, px, "Money management stop", i); }
                else { let q = live.remove(k); self.fill(i, &q, px); }
                at = lv;
            }
        }
        ev
    }
    /// 2. walk the path of one bar (or one minute: intraday wave 2b); at the open itself a stop/limit already through fills at the open
    fn walk(&mut self, i: usize, o: f64, h: f64, l: f64, live: &mut Vec<Ord>) {
        let path = if o - l <= h - o { [o, l, h] } else { [o, h, l] };
        let mut at = o;
        let mut first = true;
        for &to in &path[1..] {
            loop {
                let up = to > at;
                // candidates: (price, index in live or usize::MAX for the MM stop)
                let mut best: Option<(f64, usize)> = None;
                let mut consider = |px: f64, k: usize, buy_side: bool, is_stop: bool| {
                    // buy stop / sell limit trigger going up; sell stop / buy limit going down
                    let trig_up = buy_side == is_stop;
                    let hit = if first && ((trig_up && px <= at) || (!trig_up && px >= at)) { Some(at) }
                        else if trig_up && up && px > at && px <= to { Some(px) }
                        else if !trig_up && !up && px < at && px >= to { Some(px) } else { None };
                    if let Some(f) = hit {
                        let better = match best { None => true, Some((b, _)) => (f - at).abs() < (b - at).abs() };  // nearest to the current price fills first, so an order already through the open beats a later intrabar one
                        if better { best = Some((f, k)); }
                    }
                };
                for (k, q) in live.iter().enumerate() {
                    let buy_side = if q.entry { q.dir > 0 } else { q.dir < 0 };
                    consider(q.px, k, buy_side, q.ty == Ty::Stop);
                }
                if let Some((d, px, _)) = self.prot() { consider(px, usize::MAX, d < 0, true); }
                let was_first = first;
                first = false;
                let Some((px, k)) = best else { break };
                // lane F1 (futures contexts only): every order already through the open fills at the open, as the convention
                // above states; elsewhere the walk is unchanged (after one fill at the open the rest wait for the path)
                if was_first && px == at && crate::futures::ctx_market(self.x).is_some() { first = true; }
                if k == usize::MAX { self.close(i, px, "Money management stop", i); }
                else { let q = live.remove(k); self.fill(i, &q, px); }
                at = px;
            }
            at = to;
        }
    }
}

/// Runs `rule(i, pos)` at every close from `from` to `to`-1 (orders for bar i+1). `stop` = MM stop in points (0 = none),
/// `eod` = exit any position at the close of every bar. Positions still open at `to` are marked to market.
pub(crate) fn sim(x: &Ctx, from: usize, to: usize, stop: f64, eod: bool,
                  mut rule: impl FnMut(usize, Option<&Pos>, usize) -> Vec<Ord>) -> Vec<Trade> {
    sim_path(x, from, to, stop, eod, false, rule)
}

// ---- intraday wave 2b block: the same order model walked minute by minute
/// `minutes` = walk each day's real 1-minute bars (in time order, each minute along the same open -> nearer extreme ->
/// other extreme path) instead of the daily bar; a day without minute data fills no order. Only stop/limit orders and the
/// money-management stop (market orders are not used by the minute-path rules).
pub(crate) fn sim_path(x: &Ctx, from: usize, to: usize, stop: f64, eod: bool, minutes: bool,
                  rule: impl FnMut(usize, Option<&Pos>, usize) -> Vec<Ord>) -> Vec<Trade> {
    sim_ep6(x, from, to, stop, eod, minutes, false, rule)
}

/// `sim_path` with EP6 (#243, lane K63): with `on` (daily path), a bar whose fills differ between its two extreme orders is
/// walked on its real 1-minute path where minutes exist (intraday_ep6::resolve); the market orders at the open come first.
#[allow(clippy::too_many_arguments)]
pub(crate) fn sim_ep6(x: &Ctx, from: usize, to: usize, stop: f64, eod: bool, minutes: bool, on: bool,
                  mut rule: impl FnMut(usize, Option<&Pos>, usize) -> Vec<Ord>) -> Vec<Trade> {
    use crate::intraday_ep6::{remark, resolve_conv, Seg};
    let mut st = St { x, pos: None, out: vec![], stop, ids: 0, entries_on_bar: 0, od: crate::intraday_ep6::Day::Plain };
    let mut pending: Vec<Ord> = vec![];
    for i in from..=to {
        if i > from && on && !minutes {
            let (o, h, l, c) = (x.o[i], x.h[i], x.l[i], x.c[i]);
            st.entries_on_bar = 0;
            // 1. market orders at the open: exits first, then entries (as `bar`)
            let mut ords = std::mem::take(&mut pending);
            ords.sort_by_key(|q| (q.ty != Ty::Mkt, q.entry));
            let mut live = vec![];
            for q in ords { if q.ty == Ty::Mkt { st.fill(i, &q, o); } else { live.push(q); } }
            // 2. the path: the module's daily path (open, nearer extreme (tie: low), other extreme), or the minutes
            let keep = std::mem::take(&mut st.out);
            let mut s2 = (st.clone(), live);
            let daily = |s: &mut (St, Vec<Ord>), hf: bool| -> Vec<crate::intraday_ep6::Ev> {
                let segs = if hf { [Seg { a: o, b: h, gap: false }, Seg { a: h, b: l, gap: false }] } else { [Seg { a: o, b: l, gap: false }, Seg { a: l, b: h, gap: false }] };
                let mut e = s.0.walk_segs(i, &mut s.1, &[]);
                e.extend(s.0.walk_segs(i, &mut s.1, &segs));
                e
            };
            let walk = |s: &mut (St, Vec<Ord>), g: &[Seg]| s.0.walk_segs(i, &mut s.1, g);
            let _ = c;
            // the daily convention itself is the module's own walk (unchanged); `daily` only probes the two orders
            let day = resolve_conv(x, i, true, &mut s2, |s: &mut (St, Vec<Ord>)| s.0.walk(i, o, h, l, &mut s.1), &daily, &walk);
            st = s2.0;
            let today = std::mem::replace(&mut st.out, keep);
            st.out.extend(today.into_iter().map(|mut t| { remark(&mut t, day); t }));
            if st.pos.is_some() { st.od = st.od.worst(day); }
        } else if i > from && minutes {
            st.entries_on_bar = 0;
            let mut live: Vec<Ord> = std::mem::take(&mut pending).into_iter().filter(|q| q.ty != Ty::Mkt).collect();
            for m in crate::intraday_w2b::day(x, i).unwrap_or_default() { st.walk(i, m.o, m.h, m.l, &mut live); }
        } else if i > from { st.bar(i, std::mem::take(&mut pending)); }
        // ---- end intraday wave 2b block ----
        if eod && st.pos.is_some() { st.close(i, x.c[i], "End of day", i); }
        if i < to {
            let pid = st.pos.as_ref().map_or(0, |p| p.id);
            // lane F1 (futures contexts only): like TradeStation without pyramiding, an entry on the side already held at the
            // close is not sent for the next bar (elsewhere unchanged)
            let fut = crate::futures::ctx_market(x).is_some();
            pending = rule(i, st.pos.as_ref(), st.entries_on_bar).into_iter()
                .filter(|q| q.entry || st.pos.as_ref().is_some_and(|p| p.dir == q.dir))
                .filter(|q| !(fut && q.entry && st.pos.as_ref().is_some_and(|p| p.dir == q.dir)))
                .map(|mut q| { q.pid = pid; q }).collect();
        }
    }
    if let Some(p) = st.pos.take() {
        let mut t = Trade { dir: p.dir as f64, legs: vec![Leg { i: p.i, px: p.px, w: 1.0 }], exit_i: to, exit_px: x.c[to],
            reason: "open".into(), entry_kind: p.kind, signal_i: to, open: true, exits: vec![] };
        crate::intraday_ep6::mark(&mut t, st.od);
        st.out.push(t);
    }
    st.out
}

// ------------------------------------------------------------------ catalogue

const FUT: &str = "Built and tested on a futures market the vault has no data for (see rules); the vault holds only stock indices, ETFs and NSE stocks. Not run on an index: that would be a market substitution, not an instrument swap.";

pub fn catalogue() -> Vec<StrategyDef> {
    use Status::*;
    let d = |id, name, group, dir, status, rules, notes, claim| StrategyDef { id, name, group, session: 0, dir, status,
        needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK, rules, notes, claim };
    vec![
        d("jkbwts_dinapoli5", "Dinapoli5 (3/7/25 MA limit entries)", "S&P 500 systems", "Long/Short", Live,
          &["F|Inputs Length1 3, Length2 7, Length3 25 (p.5 code)",
            "E|If (Average(C,3)[3] < Average(C,7)[5] OR Average(C,7)[5] < Average(C,25)[5]) AND (Average(C,3) > Average(C,7) AND Average(C,7) > Average(C,25)) then Buy at Average(C,3)[3] limit (next bar)",
            "E|Mirror: (Average(C,3)[3] > Average(C,7)[5] OR Average(C,7)[5] > Average(C,25)[5]) AND Average(C,3) < Average(C,7) AND Average(C,7) < Average(C,25) then Sell at Average(C,3)[3] limit",
            "X|Stop and reverse only: the BarsSinceEntry > Length4(4) exits are inside { } comment braces in the printed code, so not used"],
          "p.4-5. Book: S&P 500 INDEX 67/99 continuous futures, daily, 01/02/90-12/26/97, 1 contract, $55 commission: 67 trades, 49% profitable, net $120,240, avg trade $1,794.63. Run on the cash index (SPX/SPY/NIFTY) instead of the futures (instrument swap).",
          claim("67", "49%", "$1,794.63", "", "S&P 500 futures 1/1990-12/1997 (p.4)")),
        d("jkbwts_gettess30", "Gettess30 (2 down closes, open below close)", "S&P 500 systems", "Long", Live,
          &["E|If MRO(Close >= Close[1], LEConsec 2, 1) = -1 (no up or unchanged close in the last 2 bars) and Open of tomorrow < Close then Buy at Market (p.13)",
            "X|If MarketPosition = 1 and Open of tomorrow > EntryPrice then ExitLong at Market",
            "X|Money Management stop $4,000 (Quick System sheet) = 8 S&P points at $500/point, armed from the entry fill"],
          "p.12-13. Book: S&P 500 INDEX 67/99, daily, 01/05/87-12/26/97: 193 trades, 76% profitable, net $87,460, avg $453.16 ($55 commission). 'Open of tomorrow' is known when the order fills at that open, so the condition is evaluated there.",
          claim("193", "76%", "$453.16", "", "S&P 500 futures 1/1987-12/1997 (p.12)")),
        d("jkbwts_buymon1400", "Buy Monday-1400 XMOC", "S&P 500 systems", "Long", Live,
          &["E|If DayOfWeek(Date) = 5 (Friday) then Buy at Market (next bar's open: Monday, or Tuesday after a Monday holiday) (p.19)",
            "X|Money Management stop $1,400 = 2.8 S&P points at $500/point, from the entry",
            "X|Close all trades at end of day session (exit at the entry day's close)"],
          "p.18-19. Book: S&P 500 INDEX 67/99, daily, 01/05/87-12/26/97: 549 trades, 52% profitable, net $147,480, avg $268.63 ($55 commission). Same idea as jk36k_jk1_buy_monday ($1,250 stop) and jkbk_buy_monday_ts83 ($1,000 stop).",
          claim("549", "52%", "$268.63", "same day", "S&P 500 futures 1/1987-12/1997 (p.18)")),
        d("jkbwts_saidenberg30", "Saidenberg30 (range breakout day trade)", "S&P 500 systems", "Long/Short", Live,
          &["E|If TradesToday(Date) < 2: Buy at H + (Range*0.66) stop; Sell at L - (Range*0.66) stop (next bar) (p.27 code)",
            "X|Daily S&P 500 bars: exit market on close, $1,000 money management stop = 2 S&P points at $500/point (code comment p.27)",
            "F|TradesToday counts entries on the bar just closed: after a 2-trade day no orders are placed for the next day",
            "X|[intraday wave 2b] Default p.path=1: the day is walked on real SPX 1-minute bars in time order, so the minutes show which stop came first (each order fills at most once a day, at its level or at the minute's open when it opened through it; the second entry reverses the first; the $1,000 = 2.00-point stop works from each fill). Inside one minute the module's TradeStation path is used (open, nearer extreme, other extreme) [convention]. SPX only (the book's fixed S&P points; no conversion is invented), 2014-01..2026-08 (SPX-only minute rules start where SPX daily opens are real). p.path=0 = the old daily bar path [Not from the book]."],
          "p.26-27. Book: S&P 500 INDEX 67/99, daily, 01/02/85-12/26/97: 1,107 trades, 48% profitable, net $122,080, avg $110.28 ($50 commission per the code comment). The code comment also gives a US Bonds version ($500 stop) - NeedsData (no bond futures).",
          claim("1107", "48%", "$110.28", "same day", "S&P 500 futures 1/1985-12/1997 (p.26)")),
        d("jkbwts_rainmaker", "JK RainMaker Index", "S&P 500 systems", "Long", Live,
          &["E|JK1: if DayOfWeek(Date) = 1 (Monday) and C > Average(C,3) then Buy tomorrow at Open of tomorrow + (Range/9) stop (p.37; article p.38-39: 'buy tomorrow at the open, plus the range of today divided by nine on a stop')",
            "E|JK2: if DayOfWeek(Date) = 5 (Friday) then Buy tomorrow at Market",
            "X|JK3: if DayOfWeek(Date) = 3 (Wednesday) and C > Average(C,3) then ExitLong tomorrow at Open of tomorrow - (Range/9) stop",
            "X|JK4: if DayOfWeek(Date) = 4 (Thursday) then ExitLong tomorrow at Market",
            "X|Bonus exit: if BarsSinceEntry >= 5 then ExitLong tomorrow at Market",
            "X|No stops (article p.40 / code comment p.37); the comment's 'suggested $5,000 MM stop' is the non-default stop_usd option"],
          "p.36-42. Article (Omega Research Nov 1997): S&P 500 futures Sept 1990-Sept 1997, no stops, no commissions: net $299,430, 62% winning, avg trade $865.40, 346 trades (p.41), max DD -$30,500. Sept 1990-Sept 1996: 297 trades, avg $647.93 (p.41). Manual printout p.36: S&P 500 INDEX 67/99 04/21/82-12/26/97: 787 trades, 59%, avg $413.33 (stop/commission not stated). JK1-JK4 inputs are 1/0 switches (all 1 in the book).",
          claim("346", "62%", "$865.40", "", "S&P 500 futures 9/1990-9/1997, no stop/commission (p.39-41)")),
        // ---------------- futures-only systems: no data
        d("jkbwts_conner19", "Conner 19", "Currency / bond / commodity futures", "Long/Short", Live,
          &["E|If MRO(Close <= Close[1], 4, 1) = -1 then Buy tomorrow at Lowest(H,4) stop; if MRO(Close >= Close[1], 4, 1) = -1 then Sell tomorrow at Highest(L,4) stop (p.3)",
            "X|If BarsSinceEntry > 0: ExitLong at entry-bar Low - 2 points stop / ExitShort at entry-bar High + 2 points stop; $1,000 MM stop (LX1/SX1 commented out)"],
          concat!("p.2-3. Book: Japanese Yen CME daily 01/05/87-12/26/97: 95 trades, 32%, avg $786.97 (LX1/SX1 coded but not used; $1,000 MM stop, code note p.3). Runs on the Japanese Yen continuous futures contract only (R4 #293): signals, indicators and point P&L on the back-adjusted series, % per trade on the unadjusted price of the contract held, $ amounts at the contract's $ per point (futures::MARKETS). ", "Norgate continuous futures, ends 2020-09-11; roll method not stated by the vendor."),
          claim("95", "32%", "$786.97", "", "Japanese Yen 1/1987-12/1997 (p.2)")),
        d("jkbwts_ehrlich17", "Ehrlich17", "Currency / bond / commodity futures", "Long/Short", Live,
          &["E|If Low < Lowest(Low,13)[3] then buy tomorrow at Highest(High,3)[1] stop; if High > Highest(High,13)[3] then sell tomorrow at Lowest(Low,3)[1] stop (p.7)",
            "X|$2,500 money management stop, $2,500 trailing stop"],
          concat!("p.6-7. Book: NY light crude oil daily 03/30/83-12/26/97: 49 trades, 63%, avg $690.51; code comment: $2,500 money management stop, $2,500 trailing stop, $55 commission deducted (p.7). Trailing stop reading (labelled): TS4 $ risk trailing = the best high (low) since entry, entry bar included, less (plus) $2,500, set at each close for the next bar. Runs on the Crude Oil continuous futures contract only (R4 #293): signals, indicators and point P&L on the back-adjusted series, % per trade on the unadjusted price of the contract held, $ amounts at the contract's $ per point (futures::MARKETS). ", "Norgate continuous futures, ends 2020-09-11; roll method not stated by the vendor."),
          claim("49", "63%", "$690.51", "", "Crude oil 3/1983-12/1997 (p.6)")),
        d("jkbwts_fox31", "Fox31", "Currency / bond / commodity futures", "Long/Short", Live,
          &["E|If CurrentBar > 1 and Close > Highest(High,240)[1] then Buy on Close; if Close < Lowest(Low,240)[1] then Sell on Close (p.9)", "X|Stop and reverse; no stops"],
          concat!("p.8-9. Book: Japanese Yen CME daily 01/05/87-12/26/97: 26 trades, 65%, avg $1,611.83; Quick System sheet: no stops (p.9). Runs on the Japanese Yen continuous futures contract only (R4 #293): signals, indicators and point P&L on the back-adjusted series, % per trade on the unadjusted price of the contract held, $ amounts at the contract's $ per point (futures::MARKETS). ", "Norgate continuous futures, ends 2020-09-11; roll method not stated by the vendor."),
          claim("26", "65%", "$1,611.83", "", "Japanese Yen 1/1987-12/1997 (p.8)")),
        d("jkbwts_pathfinder", "Pathfinder Currency", "Currency / bond / commodity futures", "Long/Short", NeedsData("Japanese Yen futures plus an unnamed data2 series"),
          &["E|Average(C,6) > Average(C,6)[1] and Average(C,9) > Average(C,18)[1] and Average(C,3) of data2 > Average(C,25) of data2: Buy tomorrow at market (mirror for Sell) (p.11)",
            "X|ExitLong tomorrow at market if Average(C,9) < Average(C,18) (mirror for shorts)"],
          "The printed code reads 'data2' but the book does not say which market data2 is. Also Japanese Yen futures.", claim("119", "34%", "$869.79", "", "Japanese Yen 1/1987-12/1997 (p.10)")),
        d("jkbwts_kase30", "Kase30", "Currency / bond / commodity futures", "Long/Short", Live,
          &["E|Average(C,4) crosses over Average(C,6): Buy Highest(High,2) + 1 point stop; crosses below: Sell Lowest(Low,2) - 1 point stop (p.15)", "X|$1,000 money management stop"],
          concat!("p.14-15. Book: British Pound daily 01/02/85-12/26/97: 170 trades, 35%, avg $599.93; code comment: $1,000 money management stop, $55 commission (p.15). Runs on the British Pound continuous futures contract only (R4 #293): signals, indicators and point P&L on the back-adjusted series, % per trade on the unadjusted price of the contract held, $ amounts at the contract's $ per point (futures::MARKETS). ", "Norgate continuous futures, ends 2020-09-11; roll method not stated by the vendor."),
          claim("170", "35%", "$599.93", "", "British Pound 1/1985-12/1997 (p.14)")),
        d("jkbwts_one_night_stand", "One Night Stand", "Currency / bond / commodity futures", "Long/Short", Live,
          &["E|Average(C,10) > Average(C,40) and DayOfWeek = 4 (Thursday): Buy next bar at Highest(H,4) stop; Average(C,10) < Average(C,40) and Thursday: Sell next bar at Lowest(L,8) stop (p.17; TS 8.3 code in joe_krut.pdf)",
            "X|If MarketPosition = 1 ExitLong (next bar) at Market; = -1 ExitShort at Market; $1,000 money management stop"],
          concat!("p.16-17. Book: Japanese Yen CME daily 01/05/87-12/26/97: 150 trades, 57%, avg $200.83; sheet: $1,000 money management stop (p.17). Krutsinger: designed for the currencies like the British Pound and the Japanese Yen (joe_krut.pdf); run on the tested Yen contract. Runs on the Japanese Yen continuous futures contract only (R4 #293): signals, indicators and point P&L on the back-adjusted series, % per trade on the unadjusted price of the contract held, $ amounts at the contract's $ per point (futures::MARKETS). ", "Norgate continuous futures, ends 2020-09-11; roll method not stated by the vendor."),
          claim("150", "57%", "$200.83", "1 day", "Japanese Yen 1/1987-12/1997 (p.16)")),
        d("jkbwts_ez_bonder", "EZ Bonder", "Currency / bond / commodity futures", "Long", Live,
          &["E|Buy next bar at C + Range * 1 stop (p.21)", "X|If MarketPosition = 1 ExitLong next bar at C - Range * 1.8 stop; $1,000 MM stop"],
          concat!("p.20-21. Book: Day T-Bonds daily 01/05/87-12/26/97: 189 trades, 37%, avg $255.35; sheet: $1,000 money management stop (the $500 $-risk trailing stop is not enabled, p.21). Norgate does not say which session its T-bond series is (labelled). Runs on the US T-Bond continuous futures contract only (R4 #293): signals, indicators and point P&L on the back-adjusted series, % per trade on the unadjusted price of the contract held, $ amounts at the contract's $ per point (futures::MARKETS). ", "Norgate continuous futures, ends 2020-09-11; roll method not stated by the vendor."),
          claim("189", "37%", "$255.35", "", "Day T-Bonds 1/1987-12/1997 (p.20)")),
        d("jkbwts_neeley31", "Neeley31", "Currency / bond / commodity futures", "Long/Short", Live,
          &["E|RateOfChange(C,10) > 0: Buy next bar at H stop; < 0: Sell next bar at L stop (p.23)", "X|$3,000 MM stop"],
          concat!("p.22-23. Book: Japanese Yen CME weekly 01/09/87-12/26/97: 28 trades, 39%, avg $3,753.04; sheet: $3,000 money management stop (p.23). Weekly bars are built from the daily contract (Monday-Sunday weeks); fills are dated on the week's last trading day (labelled). Runs on the Japanese Yen continuous futures contract only (R4 #293): signals, indicators and point P&L on the back-adjusted series, % per trade on the unadjusted price of the contract held, $ amounts at the contract's $ per point (futures::MARKETS). ", "Norgate continuous futures, ends 2020-09-11; roll method not stated by the vendor."),
          claim("28", "39%", "$3,753.04", "", "Japanese Yen weekly 1/1987-12/1997 (p.22)")),
        d("jkbwts_detrend", "DeTrend (Jeff Roy DeTrend Oscillator)", "Currency / bond / commodity futures", "Long/Short", Live,
          &["E|DeTrend = Close - Average(Close,37); DeTrend > 0.01 and LastTrade = -1: Buy market; DeTrend < -0.01 and LastTrade = 1: Sell market (p.25)",
            "A|Flat and LastTrade = 1: buy at LongPositionHigh stop; flat and LastTrade = -1: sell at ShortPositionLow stop",
            "X|No exit in the code: the opposite market entry reverses the position (stop and reverse)"],
          concat!("p.24-25. Book: Swiss Franc daily 01/02/90-12/26/97: 60 trades, 58%, avg $1,128.13. The code has no exit: after the first fill the system is always in the market, so the re-entry stops (LongPositionHigh / ShortPositionLow) act only at the start (the initial LastTrade = 1 puts a buy stop at 0, i.e. long at the first open). Runs on the Swiss Franc continuous futures contract only (R4 #293): signals, indicators and point P&L on the back-adjusted series, % per trade on the unadjusted price of the contract held, $ amounts at the contract's $ per point (futures::MARKETS). ", "Norgate continuous futures, ends 2020-09-11; roll method not stated by the vendor."),
          claim("60", "58%", "$1,128.13", "", "Swiss Franc 1/1990-12/1997 (p.24)")),
        d("jkbwts_saidenberg30_bonds", "Saidenberg30 (US Bonds)", "Currency / bond / commodity futures", "Long/Short", NeedsData("US Treasury bond futures daily data"),
          &["E|Same code as jkbwts_saidenberg30 on daily US Bond bars", "X|$500 money management stop (p.27 comment)"], FUT, None),
        d("jkbwts_stuckey19", "Stuckey19", "Currency / bond / commodity futures", "Long/Short", Live,
          &["E|Close > Highest(High,40)[1]: Buy on Close; Close < Lowest(Low,40)[1]: Sell on Close (p.29)", "X|Stop and reverse; no stops"],
          concat!("p.28-29. Book: Japanese Yen CME daily 01/05/87-12/26/97: 27 trades, 63%, avg $3,169.07; sheet: no stops (p.29). Runs on the Japanese Yen continuous futures contract only (R4 #293): signals, indicators and point P&L on the back-adjusted series, % per trade on the unadjusted price of the contract held, $ amounts at the contract's $ per point (futures::MARKETS). ", "Norgate continuous futures, ends 2020-09-11; roll method not stated by the vendor."),
          claim("27", "63%", "$3,169.07", "", "Japanese Yen 1/1987-12/1997 (p.28)")),
        d("jkbwts_wagner31", "Wagner31", "Currency / bond / commodity futures", "Long/Short", Live,
          &["E|C < C[1] and C[1] < C[2] and C[3] < C[4] and Open of tomorrow < C: Buy next bar at C + 1 point stop; mirror for Sell (p.31)", "X|Profit target $3,000"],
          concat!("p.30-31. Book: COMEX gold weekly 04/08/88-12/26/97: 19 trades, 68%, avg $843.95; sheet: $3,000 profit target, no stop (p.31). Weekly bars built from the daily contract; the target is live from the week after the entry week and fills are dated on the week's last trading day (labelled readings). Runs on the Gold continuous futures contract only (R4 #293): signals, indicators and point P&L on the back-adjusted series, % per trade on the unadjusted price of the contract held, $ amounts at the contract's $ per point (futures::MARKETS). ", "Norgate continuous futures, ends 2020-09-11; roll method not stated by the vendor."),
          claim("19", "68%", "$843.95", "", "COMEX gold weekly 4/1988-12/1997 (p.30)")),
        d("jkbwts_bwilliams19", "B Williams19", "Currency / bond / commodity futures", "Long/Short", Live,
          &["E|Range/MaxList(1,V) > Average(Range/MaxList(1,V),20): Buy next bar at H stop; < : Sell next bar at L stop (p.33)", "X|Stop and reverse (the $4,000 MM stop is disabled on the sheet)"],
          concat!("p.32-33. Book: Japanese Yen CME daily 01/05/87-12/26/97: 475 trades, 39%, avg $25.74; sheet: the $4,000 money management stop is not enabled (p.33). Volume = the Norgate continuous series volume (the book read the CME contract file; labelled reading). Runs on the Japanese Yen continuous futures contract only (R4 #293): signals, indicators and point P&L on the back-adjusted series, % per trade on the unadjusted price of the contract held, $ amounts at the contract's $ per point (futures::MARKETS). ", "Norgate continuous futures, ends 2020-09-11; roll method not stated by the vendor."),
          claim("475", "39%", "$25.74", "", "Japanese Yen 1/1987-12/1997 (p.32)")),
        d("jkbwts_williams19", "Williams 19", "Currency / bond / commodity futures", "Long/Short", Live,
          &["E|PH = ((H+L+C)/3*2)-L, PL = ((H+L+C)/3*2)-H; Buy next bar at Highest(PH,16)+1 point stop; Sell next bar at Lowest(PL,16)-1 point stop (p.35)", "X|Stop and reverse; no stops"],
          concat!("p.34-35. Book: Japanese Yen CME daily 01/05/87-12/26/97: 87 trades, 39%, avg $842.99; sheet: no stops (p.35). Runs on the Japanese Yen continuous futures contract only (R4 #293): signals, indicators and point P&L on the back-adjusted series, % per trade on the unadjusted price of the contract held, $ amounts at the contract's $ per point (futures::MARKETS). ", "Norgate continuous futures, ends 2020-09-11; roll method not stated by the vendor."),
          claim("87", "39%", "$842.99", "", "Japanese Yen 1/1987-12/1997 (p.34)")),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    Some(match id {
        "jkbwts_buymon1400" => ("jk_buy_monday", "Buy Monday (Krutsinger)", "BWTS 1997, $1,400 stop"),
        "jkbwts_saidenberg30" | "jkbwts_saidenberg30_bonds" => ("jkbwts_saidenberg", "Saidenberg30", if id.ends_with("bonds") { "US Bonds" } else { "S&P 500" }),
        _ => return catalogue().into_iter().find(|d| d.id == id).map(|d| (d.id, d.name, "")),
    })
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    let pt = p("pt_usd", "$ per index point of the book's futures contract", 500., 1., 1000., 50.);
    Some(match id {
        "jkbwts_dinapoli5" => vec![p("len1", "Length1 (fast average)", 3., 2., 10., 1.), p("len2", "Length2", 7., 4., 20., 1.), p("len3", "Length3", 25., 10., 60., 5.)],
        "jkbwts_gettess30" => vec![p("consec", "LEConsec (down closes)", 2., 1., 6., 1.), p("stop_usd", "Money management stop $ (0 = none)", 4000., 0., 8000., 250.), pt],
        "jkbwts_buymon1400" => vec![p("stop_usd", "Money management stop $ (0 = none)", 1400., 0., 5000., 100.), pt],
        "jkbwts_saidenberg30" => vec![p("k", "Range multiple for the stops", 0.66, 0.1, 2.0, 0.02), p("max_trades", "TradesToday limit", 2., 1., 5., 1.), p("stop_usd", "Money management stop $ (0 = none)", 1000., 0., 5000., 100.), pt,
            p("path", "1 = real 1-minute bars (default; SPX only; days without minute data make no trade), 0 = [Not from the book] daily bar path", 1., 0., 1., 1.)], // intraday wave 2b
        "jkbwts_rainmaker" => vec![p("div", "Range divisor", 9., 2., 20., 1.), p("ma", "Average length", 3., 2., 10., 1.), p("bse", "Bonus exit after N bars", 5., 2., 15., 1.),
            p("jk1", "JK1 entry on (1/0)", 1., 0., 1., 1.), p("jk2", "JK2 entry on (1/0)", 1., 0., 1., 1.), p("jk3", "JK3 exit on (1/0)", 1., 0., 1., 1.), p("jk4", "JK4 exit on (1/0)", 1., 0., 1., 1.),
            p("stop_usd", "Money management stop $ (0 = none, book; 5000 = code comment's suggestion)", 0., 0., 10000., 500.), pt],
        // lane F1 (R4 #293): the futures systems, on their named continuous contract
        "jkbwts_fox31" => vec![p("len", "Length (Highest High / Lowest Low)", 240., 20., 400., 10.)],
        "jkbwts_stuckey19" => vec![p("len", "Length (Highest High / Lowest Low)", 40., 10., 200., 5.)],
        "jkbwts_ez_bonder" => vec![p("k1", "Length1 (range multiple for the buy stop)", 1., 0.1, 3., 0.1), p("k2", "Length2 (range multiple for the exit stop)", 1.8, 0.1, 4., 0.1),
            p("stop_usd", "Money management stop $ per contract (0 = none)", 1000., 0., 5000., 250.)],
        "jkbwts_kase30" => vec![p("len", "Length (averages Length*2 / Length*3, Highest/Lowest of Length)", 2., 1., 10., 1.), p("ticks", "Entry offset in ticks ('1 point', #57)", 1., 0., 5., 1.),
            p("stop_usd", "Money management stop $ per contract (0 = none)", 1000., 0., 5000., 250.)],
        "jkbwts_one_night_stand" => vec![p("stop_usd", "Money management stop $ per contract (0 = none)", 1000., 0., 5000., 250.)],
        "jkbwts_williams19" => vec![p("len", "Length (Highest PH / Lowest PL)", 16., 4., 60., 1.), p("ticks", "Entry offset in ticks ('1 point', #57)", 1., 0., 5., 1.)],
        "jkbwts_bwilliams19" => vec![p("len", "Length (average of Range / Volume)", 20., 5., 60., 1.)],
        "jkbwts_ehrlich17" => vec![p("len", "Lowest Low / Highest High length (13, shifted 3 bars)", 13., 5., 40., 1.), p("stop_usd", "Money management stop $ per contract (0 = none)", 2500., 0., 10000., 250.),
            p("trail_usd", "$ risk trailing stop per contract (0 = none)", 2500., 0., 10000., 250.)],
        "jkbwts_neeley31" => vec![p("len", "RateOfChange length (weeks)", 10., 2., 52., 1.), p("stop_usd", "Money management stop $ per contract (0 = none)", 3000., 0., 10000., 250.)],
        "jkbwts_wagner31" => vec![p("ticks", "Entry offset in ticks ('1 point', #57)", 1., 0., 5., 1.), p("target_usd", "Profit target $ per contract", 3000., 500., 10000., 250.)],
        "jkbwts_detrend" => vec![p("period", "Period (DeTrend = Close - Average(Close, Period))", 37., 5., 100., 1.), p("thresh", "Thresh (in the contract's quote)", 0.01, 0., 0.05, 0.001)],
        "jkbwts_conner19" => vec![p("consec", "LEConsec / SEConsec", 4., 2., 8., 1.), p("ticks", "Exit offset in ticks ('2 points', #57)", 2., 0., 5., 1.),
            p("stop_usd", "Money management stop $ per contract (0 = none)", 1000., 0., 5000., 250.)],
        _ => return None,
    })
}

/// Lane F1: a weekly-bar system on the daily context. Monday-Sunday weeks (the week's last trading day closes it; open =
/// its first open, high/low = its extremes); only weeks that start on/after `from` and end by `to` are traded. The rule runs
/// on the weekly bars; each fill is dated on the last trading day of its week (the weekly bar does not say which day), and
/// a trade still open is marked at `to`.
fn weekly_run(x: &Ctx, from: usize, to: usize, f: impl Fn(&Ctx, usize, usize) -> Vec<Trade>) -> Vec<Trade> {
    let wk = |i: usize| (crate::day_number(&x.r[i].date) + 3).div_euclid(7);
    let (mut rows, mut start, mut end): (Vec<crate::Row>, Vec<usize>, Vec<usize>) = (vec![], vec![], vec![]);
    let mut i = 0;
    while i <= to {
        let mut j = i;
        while j + 1 <= to && wk(j + 1) == wk(i) { j += 1; }
        let (h, l) = (x.h[i..=j].iter().cloned().fold(f64::MIN, f64::max), x.l[i..=j].iter().cloned().fold(f64::MAX, f64::min));
        rows.push(crate::Row { ticker: x.r[j].ticker.clone(), date: x.r[j].date.clone(), open: x.o[i], high: h, low: l, close: x.c[j], volume: None });
        start.push(i); end.push(j);
        i = j + 1;
    }
    let Some(wf) = start.iter().position(|&s| s >= from) else { return vec![] };
    let wt = end.len() - 1;
    if wf >= wt { return vec![]; }
    let wx = Ctx::new(&rows, &[], &[]);
    f(&wx, wf, wt).into_iter().map(|mut t| {
        for l in &mut t.legs { l.i = end[l.i]; }
        for e in &mut t.exits { e.0 = end[e.0]; }
        t.signal_i = end[t.signal_i];
        if t.open { t.exit_i = to; t.exit_px = x.c[to]; t.signal_i = to; } else { t.exit_i = end[t.exit_i]; }
        t
    }).collect()
}

/// Lane F1: "Buy/Sell on Close" stop-and-reverse (Fox31, Stuckey19). Signals at the close of bar i < `to` fill at that close.
fn close_sar(x: &Ctx, from: usize, to: usize, long: impl Fn(usize) -> bool, short: impl Fn(usize) -> bool, tag: (&str, &str)) -> Vec<Trade> {
    let mut out = vec![];
    let mut pos: Option<(f64, usize)> = None;
    for i in from..to {
        let d = if long(i) { 1. } else if short(i) { -1. } else { continue };
        if pos.is_some_and(|p| p.0 == d) { continue; }
        if let Some((pd, e)) = pos.take() {
            out.push(Trade { dir: pd, legs: vec![Leg { i: e, px: x.c[e], w: 1.0 }], exit_i: i, exit_px: x.c[i], reason: format!("Reversed by {}", if d > 0. { tag.0 } else { tag.1 }),
                entry_kind: "close", signal_i: i, open: false, exits: vec![] });
        }
        pos = Some((d, i));
    }
    if let Some((pd, e)) = pos {
        out.push(Trade { dir: pd, legs: vec![Leg { i: e, px: x.c[e], w: 1.0 }], exit_i: to, exit_px: x.c[to], reason: "open".into(), entry_kind: "close", signal_i: to, open: true, exits: vec![] });
    }
    out
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    let schema = params(id)?;
    if from > to || to >= x.c.len() { return None; }
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).map_or(f64::NAN, |d| d.def));
    let u = |k: &str| g(k).max(1.0).round() as usize;
    let (o, h, l, c) = (&x.o, &x.h, &x.l, &x.c);
    let stop = if g("stop_usd") > 0.0 { g("stop_usd") / g("pt_usd") } else { 0.0 };
    let wd = |i: usize| dow(&x.r[i].date);
    // lane F1: the futures systems trade only their Norgate continuous contract (readiness refuses every other ticker)
    if crate::futures::CARDS.iter().any(|c| c.0 == id) && crate::futures::ctx_market(x).is_none() { return Some(vec![]); }
    Some(match id {
        "jkbwts_dinapoli5" => {
            let (a, b, z) = (avg(c, u("len1")), avg(c, u("len2")), avg(c, u("len3")));
            sim(x, from, to, 0.0, false, |i, _, _| {
                if i < 5 || !z[i - 5].is_finite() || !a[i - 3].is_finite() { return vec![]; }
                let mut v = vec![];
                if (a[i - 3] < b[i - 5] || b[i - 5] < z[i - 5]) && a[i] > b[i] && b[i] > z[i] { v.push(entry(1, Ty::Limit, a[i - 3], "Dinapoli5 buy limit")); }
                if (a[i - 3] > b[i - 5] || b[i - 5] > z[i - 5]) && a[i] < b[i] && b[i] < z[i] { v.push(entry(-1, Ty::Limit, a[i - 3], "Dinapoli5 sell limit")); }
                v
            })
        }
        "jkbwts_gettess30" => {
            let n = u("consec");
            sim(x, from, to, stop, false, |i, pos, _| {
                let mut v = vec![];
                if let Some(p) = pos { if o[i + 1] > p.px { v.push(exit(1, Ty::Mkt, 0.0, "Open of tomorrow > EntryPrice")); } }
                if i >= n && (0..n).all(|b| c[i - b] < c[i - b - 1]) && o[i + 1] < c[i] { v.push(entry(1, Ty::Mkt, 0.0, "Gettess30 buy")); }
                v
            })
        }
        "jkbwts_buymon1400" => buy_monday(x, from, to, stop),
        "jkbwts_saidenberg30" => {
            let (k, mt) = (g("k"), u("max_trades"));
            sim_path(x, from, to, stop, true, g("path") >= 0.5, |i, _, today| { // intraday wave 2b: path 1 = 1-minute bars
                if today >= mt { return vec![]; }
                let r = h[i] - l[i];
                vec![entry(1, Ty::Stop, h[i] + r * k, "Saidenberg buy stop"), entry(-1, Ty::Stop, l[i] - r * k, "Saidenberg sell stop")]
            })
        }
        "jkbwts_rainmaker" => {
            let (div, ma, bse) = (g("div"), avg(c, u("ma")), u("bse"));
            let on = |k: &str| g(k) >= 0.5;
            sim(x, from, to, stop, false, |i, pos, _| {
                let r = h[i] - l[i];
                let mut v = vec![];
                if on("jk1") && wd(i) == 1 && c[i] > ma[i] { v.push(entry(1, Ty::Stop, o[i + 1] + r / div, "JK1LE")); }
                if on("jk2") && wd(i) == 5 { v.push(entry(1, Ty::Mkt, 0.0, "JK2LE")); }
                if on("jk3") && wd(i) == 3 && c[i] > ma[i] { v.push(exit(1, Ty::Stop, o[i + 1] - r / div, "JK3LX")); }
                if on("jk4") && wd(i) == 4 { v.push(exit(1, Ty::Mkt, 0.0, "JK4LX")); }
                if pos.is_some_and(|p| i - p.i >= bse) { v.push(exit(1, Ty::Mkt, 0.0, "BSELX")); }
                v
            })
        }
        // ---- lane F1 (R4 #293): futures systems on the named Norgate continuous contract (back-adjusted bars)
        "jkbwts_fox31" | "jkbwts_stuckey19" => {
            let n = u("len");
            let hh = |i: usize| h[i - n..i].iter().cloned().fold(f64::MIN, f64::max);
            let ll = |i: usize| l[i - n..i].iter().cloned().fold(f64::MAX, f64::min);
            close_sar(x, from, to, |i| i >= n && c[i] > hh(i), |i| i >= n && c[i] < ll(i), ("Buy on Close", "Sell on Close"))
        }
        "jkbwts_ez_bonder" => {
            let (k1, k2) = (g("k1"), g("k2"));
            let stop = crate::futures::usd_points(x, from, g("stop_usd"))?;
            sim(x, from, to, stop, false, |i, pos, _| {
                let r = h[i] - l[i];
                let mut v = vec![entry(1, Ty::Stop, c[i] + r * k1, "EZ Bonder buy stop")];
                if pos.is_some_and(|p| p.dir == 1) { v.push(exit(1, Ty::Stop, c[i] - r * k2, "EZ Bonder exit stop")); }
                v
            })
        }
        "jkbwts_kase30" => {
            let n = u("len");
            let (a, b) = (avg(c, 2 * n), avg(c, 3 * n));
            let off = g("ticks").round() * x.market_tick();
            let stop = crate::futures::usd_points(x, from, g("stop_usd"))?;
            sim(x, from, to, stop, false, |i, _, _| {
                if i < 1 || !b[i - 1].is_finite() || i + 1 < n { return vec![]; }
                let mut v = vec![];
                if a[i] > b[i] && a[i - 1] <= b[i - 1] { v.push(entry(1, Ty::Stop, h[i + 1 - n..=i].iter().cloned().fold(f64::MIN, f64::max) + off, "Kase30 buy stop")); }
                if a[i] < b[i] && a[i - 1] >= b[i - 1] { v.push(entry(-1, Ty::Stop, l[i + 1 - n..=i].iter().cloned().fold(f64::MAX, f64::min) - off, "Kase30 sell stop")); }
                v
            })
        }
        "jkbwts_one_night_stand" => {
            let (a, b) = (avg(c, 10), avg(c, 40));
            let stop = crate::futures::usd_points(x, from, g("stop_usd"))?;
            sim(x, from, to, stop, false, |i, pos, _| {
                let mut v = vec![];
                if let Some(p) = pos { v.push(exit(p.dir, Ty::Mkt, 0.0, "Exit at market")); }
                if b[i].is_finite() && wd(i) == 4 {
                    if a[i] > b[i] { v.push(entry(1, Ty::Stop, h[i - 3..=i].iter().cloned().fold(f64::MIN, f64::max), "Thursday buy stop")); }
                    if a[i] < b[i] { v.push(entry(-1, Ty::Stop, l[i - 7..=i].iter().cloned().fold(f64::MAX, f64::min), "Thursday sell stop")); }
                }
                v
            })
        }
        "jkbwts_williams19" => {
            let n = u("len");
            let off = g("ticks").round() * x.market_tick();
            let ph: Vec<f64> = (0..c.len()).map(|i| (h[i] + l[i] + c[i]) / 3. * 2. - l[i]).collect();
            let pl: Vec<f64> = (0..c.len()).map(|i| (h[i] + l[i] + c[i]) / 3. * 2. - h[i]).collect();
            sim(x, from, to, 0.0, false, |i, _, _| {
                if i + 1 < n { return vec![]; }
                vec![entry(1, Ty::Stop, ph[i + 1 - n..=i].iter().cloned().fold(f64::MIN, f64::max) + off, "Williams19 buy stop"),
                     entry(-1, Ty::Stop, pl[i + 1 - n..=i].iter().cloned().fold(f64::MAX, f64::min) - off, "Williams19 sell stop")]
            })
        }
        "jkbwts_bwilliams19" => {
            let n = u("len");
            let rv: Vec<f64> = (0..c.len()).map(|i| (h[i] - l[i]) / x.r[i].volume.unwrap_or(0.).max(1.)).collect();
            let m = avg(&rv, n);
            sim(x, from, to, 0.0, false, |i, _, _| {
                if !m[i].is_finite() { return vec![]; }
                if rv[i] > m[i] { vec![entry(1, Ty::Stop, h[i], "B Williams19 buy stop")] } else if rv[i] < m[i] { vec![entry(-1, Ty::Stop, l[i], "B Williams19 sell stop")] } else { vec![] }
            })
        }
        "jkbwts_ehrlich17" => {
            let n = u("len");
            let stop = crate::futures::usd_points(x, from, g("stop_usd"))?;
            let trail = crate::futures::usd_points(x, from, g("trail_usd"))?;
            sim(x, from, to, stop, false, |i, pos, _| {
                let mut v = vec![];
                if i >= n + 3 {
                    if l[i] < l[i - 3 - (n - 1)..=i - 3].iter().cloned().fold(f64::MAX, f64::min) { v.push(entry(1, Ty::Stop, h[i - 3..i].iter().cloned().fold(f64::MIN, f64::max), "Ehrlich17 buy stop")); }
                    if h[i] > h[i - 3 - (n - 1)..=i - 3].iter().cloned().fold(f64::MIN, f64::max) { v.push(entry(-1, Ty::Stop, l[i - 3..i].iter().cloned().fold(f64::MAX, f64::min), "Ehrlich17 sell stop")); }
                }
                // TS4 Format dialog $ risk trailing stop: the best bar extreme since entry (entry bar included) less the amount, for the next bar
                if let Some(p) = pos.filter(|_| trail > 0.) {
                    v.push(if p.dir == 1 { exit(1, Ty::Stop, h[p.i..=i].iter().cloned().fold(f64::MIN, f64::max) - trail, "$ risk trailing stop") }
                           else { exit(-1, Ty::Stop, l[p.i..=i].iter().cloned().fold(f64::MAX, f64::min) + trail, "$ risk trailing stop") });
                }
                v
            })
        }
        "jkbwts_neeley31" => {
            let n = u("len");
            let stop_usd = g("stop_usd");
            weekly_run(x, from, to, |w, a, b| {
                let Some(stop) = crate::futures::usd_points(w, a, stop_usd) else { return vec![] };
                sim(w, a, b, stop, false, |i, _, _| {
                    if i < n { return vec![]; }
                    // RateOfChange(C,10) > 0 <=> C > C[10] on a positive price (the sign of the 10-week point change)
                    let d = w.c[i] - w.c[i - n];
                    if d > 0. { vec![entry(1, Ty::Stop, w.h[i], "Neeley31 buy stop")] } else if d < 0. { vec![entry(-1, Ty::Stop, w.l[i], "Neeley31 sell stop")] } else { vec![] }
                })
            })
        }
        "jkbwts_wagner31" => {
            let (ticks, target_usd) = (g("ticks").round(), g("target_usd"));
            weekly_run(x, from, to, |w, a, b| {
                let Some(tg) = crate::futures::usd_points(w, a, target_usd) else { return vec![] };
                let off = ticks * w.market_tick();
                let c = &w.c;
                sim(w, a, b, 0.0, false, |i, pos, _| {
                    let mut v = vec![];
                    // Profit target $3,000 (Format dialog): a limit from the entry price, live from the week after the entry week
                    if let Some(p) = pos { v.push(exit(p.dir, Ty::Limit, p.px + p.dir as f64 * tg, "Profit target")); }
                    if i >= 4 {
                        if c[i] < c[i - 1] && c[i - 1] < c[i - 2] && c[i - 3] < c[i - 4] && w.o[i + 1] < c[i] { v.push(entry(1, Ty::Stop, c[i] + off, "Wagner31 buy stop")); }
                        if c[i] > c[i - 1] && c[i - 1] > c[i - 2] && c[i - 3] > c[i - 4] && w.o[i + 1] > c[i] { v.push(entry(-1, Ty::Stop, c[i] - off, "Wagner31 sell stop")); }
                    }
                    v
                })
            })
        }
        "jkbwts_detrend" => {
            let (per, th) = (u("period"), g("thresh"));
            let m = avg(c, per);
            // the printed code's variables, updated at each close from the position after that bar's fills
            let (mut last, mut lph, mut spl) = (1i8, 0.0f64, 999999.0f64);
            sim(x, from, to, 0.0, false, move |i, pos, _| {
                if !m[i].is_finite() { return vec![]; }
                let dt = c[i] - m[i];
                let mp = pos.map_or(0, |p| p.dir);
                let mut v = vec![];
                if dt > th && last == -1 { v.push(entry(1, Ty::Mkt, 0.0, "DeTrend buy")); }
                if dt < -th && last == 1 { v.push(entry(-1, Ty::Mkt, 0.0, "DeTrend sell")); }
                if mp == 0 && last == 1 { v.push(entry(1, Ty::Stop, lph, "Re-enter long at LongPositionHigh")); }
                if mp == 0 && last == -1 { v.push(entry(-1, Ty::Stop, spl, "Re-enter short at ShortPositionLow")); }
                if mp > 0 { last = 1; if h[i] > lph { lph = h[i]; } spl = 999999.0; }
                if mp < 0 { last = -1; if l[i] < spl { spl = l[i]; } lph = 0.0; }
                v
            })
        }
        "jkbwts_conner19" => {
            let n = u("consec");
            let off = g("ticks").round() * x.market_tick();
            let stop = crate::futures::usd_points(x, from, g("stop_usd"))?;
            sim(x, from, to, stop, false, |i, pos, _| {
                let mut v = vec![];
                if i >= n {
                    if (i + 1 - n..=i).all(|k| !(c[k] <= c[k - 1])) { v.push(entry(1, Ty::Stop, h[i + 1 - n..=i].iter().cloned().fold(f64::MAX, f64::min), "LE1")); }
                    if (i + 1 - n..=i).all(|k| !(c[k] >= c[k - 1])) { v.push(entry(-1, Ty::Stop, l[i + 1 - n..=i].iter().cloned().fold(f64::MIN, f64::max), "SE1")); }
                }
                if let Some(p) = pos.filter(|p| i > p.i) {
                    v.push(if p.dir == 1 { exit(1, Ty::Stop, l[p.i] - off, "LX2 entry-bar low - 2 points") } else { exit(-1, Ty::Stop, h[p.i] + off, "SX2 entry-bar high + 2 points") });
                }
                v
            })
        }
        _ => return None,
    })
}

/// Friday close -> buy the next bar at market, MM stop `stop` points, exit at that bar's close.
pub(crate) fn buy_monday(x: &Ctx, from: usize, to: usize, stop: f64) -> Vec<Trade> {
    sim(x, from, to, stop, true, |i, _, _| if dow(&x.r[i].date) == 5 { vec![entry(1, Ty::Mkt, 0.0, "Buy Monday")] } else { vec![] })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;
    fn rows(v: &[(&str, f64, f64, f64, f64)]) -> Vec<Row> {
        v.iter().map(|&(d, o, h, l, c)| Row { ticker: "T".into(), date: d.into(), open: o, high: h, low: l, close: c, volume: None }).collect()
    }
    /// lane F1 (R4 #293): the futures systems on a futures ticker (back-adjusted rows; points, ticks and $ stops of the contract)
    fn fut(v: &[(&str, f64, f64, f64, f64)], t: &str) -> Vec<Row> { rows(v).into_iter().map(|mut r| { r.ticker = t.into(); r }).collect() }
    #[test]
    fn futures_close_reversal_and_stop_orders() {
        let mut v: Vec<(String, f64, f64, f64, f64)> = (0..45).map(|k| (format!("1990-01-{:02}", k % 28 + 1), 1., 1.01, 0.99, 1.)).collect();
        v[41] = (v[41].0.clone(), 1., 1.03, 1., 1.02); // close above the 40-bar high -> long at 1.02
        v[43] = (v[43].0.clone(), 1., 1., 0.95, 0.96); // close below the 40-bar low -> reverse short at 0.96
        let r = fut(&v.iter().map(|(d, o, h, l, c)| (d.as_str(), *o, *h, *l, *c)).collect::<Vec<_>>(), "Japanese Yen (Norgate &6J)");
        let x = Ctx::new(&r, &[], &[]);
        let t = run("jkbwts_stuckey19", &x, 0, 44, &Params::new()).unwrap();
        assert_eq!((t.len(), t[0].legs[0].i, t[0].exit_i, t[0].exit_px, t[1].dir), (2, 41, 43, 0.96, -1.));
        assert_eq!(x.market_tick(), 0.0001);
        // Kase30 on British Pound: $1,000 = 0.016 points at $62,500 per 1.00; buy stop = Highest(High,2) + one 0.0002 tick
        let b = fut(&[("1990-01-02", 1.5, 1.5, 1.5, 1.5)], "British Pound (Norgate &6B)");
        let xb = Ctx::new(&b, &[], &[]);
        assert!((crate::futures::usd_points(&xb, 0, 1000.).unwrap() - 0.016).abs() < 1e-12 && xb.market_tick() == 0.0002);
        // Williams 19: PH = (H+L+C)/3*2 - L; buy stop at Highest(PH,16) + 1 tick on the next bar
        let w: Vec<(String, f64, f64, f64, f64)> = (0..18).map(|k| (format!("1990-02-{:02}", k + 1), 1., if k == 17 { 1.2 } else { 1.01 }, if k >= 16 { 0.995 } else { 0.99 }, 1.)).collect();
        let rw = fut(&w.iter().map(|(d, o, h, l, c)| (d.as_str(), *o, *h, *l, *c)).collect::<Vec<_>>(), "Japanese Yen (Norgate &6J)");
        let xw = Ctx::new(&rw, &[], &[]);
        let tw = run("jkbwts_williams19", &xw, 0, 17, &Params::new()).unwrap();
        let ph = (1.01 + 0.99 + 1.) / 3. * 2. - 0.99;
        assert_eq!(tw.len(), 1); assert!((tw[0].legs[0].px - (ph + 0.0001)).abs() < 1e-12 && tw[0].legs[0].i == 17 && tw[0].dir == 1.);
        assert!(crate::futures::home("jkbwts_williams19", "SPY").is_some() && crate::futures::home("jkbwts_williams19", "Japanese Yen (Norgate &6J)").is_none());
    }
    #[test]
    fn dow_matches_tradestation() {
        assert_eq!(dow("1997-12-26"), 5); assert_eq!(dow("2024-01-01"), 1); assert_eq!(dow("2000-03-01"), 3);
    }
    #[test]
    fn buy_monday_stop_and_close() {
        // Fri 2024-01-05, Mon 01-08 (stop hit: low 97 < 100-2.8), Fri 01-12, Mon 01-15 (exit at close)
        let r = rows(&[("2024-01-05", 100., 101., 99., 100.), ("2024-01-08", 100., 101., 97., 99.), ("2024-01-09", 99., 99., 99., 99.),
            ("2024-01-12", 99., 99., 99., 99.), ("2024-01-15", 99., 103., 98., 102.)]);
        let x = Ctx::new(&r, &r, &[]);
        let t = buy_monday(&x, 0, 4, 2.8);
        assert_eq!(t.len(), 2);
        assert_eq!((t[0].legs[0].i, t[0].legs[0].px, t[0].exit_i), (1, 100., 1)); assert!((t[0].exit_px - 97.2).abs() < 1e-9);
        assert_eq!((t[1].legs[0].i, t[1].exit_px), (4, 102.));
    }
    #[test]
    fn path_low_first_hits_sell_then_buy_stop() {
        // order for bar 1: buy stop 105, sell stop 95; open 99 nearer low -> low 94 first (short at 95), then high 106 (reverse long at 105)
        let r = rows(&[("2024-01-01", 100., 101., 99., 100.), ("2024-01-02", 99., 106., 94., 100.)]);
        let x = Ctx::new(&r, &r, &[]);
        let t = sim(&x, 0, 1, 0.0, true, |_, _, _| vec![entry(1, Ty::Stop, 105., "b"), entry(-1, Ty::Stop, 95., "s")]);
        assert_eq!(t.len(), 2);
        assert_eq!((t[0].dir, t[0].legs[0].px, t[0].exit_px), (-1., 95., 105.));
        assert_eq!((t[1].dir, t[1].legs[0].px, t[1].exit_px), (1., 105., 100.));
    }
}

// Instrument admission: Krutsinger 2026-10-25 block, moved from instrument_policy.rs on 31 Oct 2026 (lane ST).
// Admission only for the module REVIEWED lists (authors/krutsinger/admission/SOURCE-CONTRACT.md); kept in one block to merge.
// Runs on actual indices, ETFs and stocks, US and India (index_etf_or_stock, nifty50 basket, declared application).
pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| (def.book=="jkbwts" && crate::books::jkbwts::reviewed(def.id)) || (def.book=="jkbk" && crate::books::jkbk::reviewed(def.id)) || (def.book=="upg18" && crate::books::upg18::reviewed(def.id)) || (def.book=="jk36k" && def.id=="jk36k_jk1_buy_monday") /* intraday wave 2b */,
    label: "Joe Krutsinger built and reported these systems on S&P 500 futures (Dinapoli5, RainMaker, JK-Call Buyer), stock-index futures (SAFE) and QQQ / any market (Prince, English Channel, Parrot); runs on other indices, ETFs and single stocks (US and India) are a declared application.",
    basis: "Building Winning Trading Systems (1997): Dinapoli5 p.5, JK RainMaker p.37; joe_krut.pdf: SAFE worksheet p.22, JK-Call Buyer p.34-35 (Active Trader Sept 2000); vendor code: English Channel (EnglishChannel.pdf, English_Channel.txt), Prince 2002 (Open Code.txt), Parrot (Parrot.txt). Index/ETF/stock runs are a declared application.",
};
