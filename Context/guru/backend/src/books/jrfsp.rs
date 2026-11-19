//! Joel Rensink (Risnick), infiniteyield.com, "FirstStrike Plus - Weekly Volatility Breakout Instructions" (2008, 2 pp.).
//! Book id `jrfsp`. Source copy: Books/Joel Risnick/FirstStrike Plus.pdf (from R:/Trading Knowledge I/Joel Risnick -
//! First Strike Plus, First Strike Plus & One Night Stand (infiniteyield.com)/FirstStrike Plus.pdf).
//! Weeks = Monday-Sunday calendar weeks of the daily rows (the week's open = first trading day's open; the prior
//! week's range = its highest high - lowest low). Orders are worked on the daily bars of the week. Same-bar order
//! [book silent]: open -> nearer extreme -> other extreme -> close (high first on a tie), the vault's TradeStation bar
//! path; an order the open is already through fills at the open.
use crate::connors::{Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "jrfsp";
/// Admitted ids (authors/lanea/newsletters/decisions.json).
pub const REVIEWED_LANEA: &[&str] = &["jrfsp_weekly_breakout"];
pub fn reviewed_lanea(id: &str) -> bool { REVIEWED_LANEA.contains(&id) }

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef {
        id: "jrfsp_weekly_breakout", name: "FirstStrike Plus weekly volatility breakout (first profitable open)", group: "FirstStrike Plus", session: 0, dir: "Long/Short",
        status: Status::Live, needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK,
        rules: &[
            "F|\"Before the market's open on Monday morning ... you must know what the total range of the previous week was, (high - low = range) and multiply that figure by .30 to obtain the values which will determine our entry points for the next week\" (step 1, p.1)",
            "E|\"After Monday's morning open be ready to buy on a stop at the opening price + the quantity from step (1). Or, be ready to sell at the price on a stop of the opening price - the quantity from step (1).\" (step 2, p.1)",
            "X|\"Multiply [the previous week's range] x .10 ... If you get long you will place a position stop below the week's open a total of [that amount] ... (For a short trade, reverse these instructions)\" (steps 3-4, p.1). The stop stays at the entry week's level until the trade ends.",
            "X|\"wait for the open of the next weekly bar ... If exiting at the open of the next week would result in a profit, exit the market. If not, continue holding until a succeeding week's opening price is profitable or you get stopped out for a loss.\" (step 4, p.1)",
            "X|\"Keep monitoring and placing your current buy and sell prices for each week. Your new buy or sell orders for the week ... may be closer than a protective stop loss order for an existing position ... You can save money by exiting and reversing position at the closer price.\" (steps 5-6, p.1): a carried position is closed and reversed at this week's opposite entry stop when price reaches it before the protective stop.",
            "F|[Book silent] One position at a time: this week's same-direction entry stop is not used while that position is held. Each weekly entry order fills at most once per week (a filled order is used up); both sides can fill in one week (\"at times both the week's long trade and short trade were taken, with one or both stopped out for a loss\", p.2).",
            "F|[Book silent] Same-day order of fills: open -> nearer extreme -> other extreme -> close; gaps fill at the open. The book rounds pip amounts up to whole pips (FX); index levels are used unrounded.",
        ],
        notes: "Written for spot FX on weekly bars (book example EUR/USD week of Oct 6-10 2008; the equity curve is GBP/USD, 6 years, 30%, $10/pip, no numbers printed). Unit-free rule, run on the index (instrument swap). Homework variants 60%, 75%, 100% (p.1) are the entry multiplier parameter. No published statistics, so no book-vs-ours comparison is possible.",
        claim: None,
    }]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    (id == "jrfsp_weekly_breakout").then_some(("jrfsp", "FirstStrike Plus (Rensink)", "30% / 10% weekly range"))
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    (id == "jrfsp_weekly_breakout").then(|| vec![
        p("entry", "Entry distance from the week's open, x prior week's range", 0.30, 0.10, 1.00, 0.05),
        p("stop", "Stop distance beyond the week's open, x prior week's range", 0.10, 0.0, 0.50, 0.05),
    ]).map(|mut v| { crate::intraday_ep6::add_param(id, &mut v); v }) // EP6 (#243, lane K63)
}

/// Calendar week number with Monday as the first day.
fn week_key(d: &str) -> i64 {
    let y: i64 = d[0..4].parse().unwrap_or(1970);
    let m: i64 = d[5..7].parse().unwrap_or(1);
    let dd: i64 = d[8..10].parse().unwrap_or(1);
    let (y2, m2) = if m <= 2 { (y - 1, m + 12) } else { (y, m) };
    let days = 365 * y2 + y2 / 4 - y2 / 100 + y2 / 400 + (153 * (m2 - 3) + 2) / 5 + dd - 719469;
    (days + 3).div_euclid(7)
}

#[derive(Clone, Copy)]
struct Pos { dir: f64, i: usize, px: f64, stop: f64, week: usize }

/// The week's working orders and the position during one bar (EP6, lane K63): this week's entry levels (`live` = not yet
/// used), the protective stop's week values, the trades closed on the bar and the position's same-day-order mark.
#[derive(Clone)]
struct St { pos: Option<Pos>, buy_live: bool, sell_live: bool, out: Vec<Trade>, od: crate::intraday_ep6::Day }

/// The week's levels: entry stops `buy` / `sell`, the week's open and range, the stop multiple, the week number.
#[derive(Clone, Copy)]
struct Wk { buy: f64, sell: f64, wopen: f64, rng: f64, sm: f64, wn: usize }

impl St {
    fn close(&mut self, q: Pos, i: usize, px: f64, why: &str) {
        let mut t = Trade { dir: q.dir, legs: vec![Leg { i: q.i, px: q.px, w: 1.0 }], exit_i: i, exit_px: px, reason: why.into(),
            entry_kind: "stop", signal_i: i, open: false, exits: vec![] };
        crate::intraday_ep6::mark(&mut t, self.od);
        self.out.push(t);
    }
    /// One leg of bar t's path from `*px` to `nxt` (`first`: the open, levels the open is through; `gap`: a jump, levels
    /// inside it fill at `nxt`): levels in the direction of travel, the nearest first. Returns the fills (EP6 events).
    fn step(&mut self, x: &Ctx, t: usize, w: &Wk, first: bool, px: &mut f64, nxt: f64, gap: bool) -> Vec<crate::intraday_ep6::Ev> {
        let o = x.o[t];
        let mut ev = vec![];
        loop {
            // candidate levels in the direction of travel (at the open: anything the open is through)
            let up = nxt >= *px;
            let cur = *px;
            let mut best: Option<(f64, u8, bool)> = None; // (level, kind, rising) kind: 0 buy entry, 1 sell entry, 2 protective stop
            let mut cand = |lv: f64, kind: u8, rising: bool| {
                if !lv.is_finite() { return; }
                let hit = if first { if rising { o >= lv } else { o <= lv } } else if rising { up && lv >= cur - 1e-12 && lv <= nxt } else { !up && lv <= cur + 1e-12 && lv >= nxt };
                if !hit { return; }
                let better = match best { None => true, Some((b, _, _)) => if rising { lv < b } else { lv > b } };
                if better { best = Some((lv, kind, rising)); }
            };
            let dir = self.pos.as_ref().map_or(0.0, |q| q.dir);
            if self.buy_live && dir <= 0.0 { cand(w.buy, 0, true); }
            if self.sell_live && dir >= 0.0 { cand(w.sell, 1, false); }
            if let Some(q) = &self.pos { cand(q.stop, 2, q.dir < 0.0); }
            let Some((lv, kind, rising)) = best else { break };
            let fill = if first { o } else if gap { nxt } else { lv };
            ev.push(crate::intraday_ep6::Ev { k: kind, px: fill, up: rising });
            match kind {
                2 => { let q = self.pos.take().unwrap(); self.close(q, t, fill, "protective stop"); }
                _ => {
                    let d = if kind == 0 { 1.0 } else { -1.0 };
                    if kind == 0 { self.buy_live = false } else { self.sell_live = false }
                    if let Some(q) = self.pos.take() { self.close(q, t, fill, "reversed at this week's entry stop"); }
                    self.pos = Some(Pos { dir: d, i: t, px: fill, stop: w.wopen - d * w.sm * w.rng, week: w.wn });
                    self.od = crate::intraday_ep6::Day::Plain;
                }
            }
            *px = if first || !gap { fill } else { lv };
        }
        ev
    }
    /// Bar t on the daily path: open, nearer extreme, other extreme, close (the module's convention).
    fn bar(&mut self, x: &Ctx, t: usize, w: &Wk) {
        let (o, h, l, c) = (x.o[t], x.h[t], x.l[t], x.c[t]);
        let hf = h - o <= o - l;
        let pts = if hf { [o, h, l, c] } else { [o, l, h, c] };
        let mut px = o;
        self.step(x, t, w, true, &mut px, o, false);
        for &nxt in &pts[1..] { self.step(x, t, w, false, &mut px, nxt, false); px = nxt; }
    }
    /// EP6 walk: no segments = the open step; otherwise each segment from its start.
    fn walk(&mut self, x: &Ctx, t: usize, w: &Wk, segs: &[crate::intraday_ep6::Seg]) -> Vec<crate::intraday_ep6::Ev> {
        let o = x.o[t];
        if segs.is_empty() { let mut px = o; return self.step(x, t, w, true, &mut px, o, false); }
        let mut ev = vec![];
        for g in segs { let mut px = g.a; ev.extend(self.step(x, t, w, false, &mut px, g.b, g.gap)); }
        ev
    }
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    let defs = params(id)?;
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| defs.iter().find(|d| d.key == k).map(|d| d.def).unwrap_or(f64::NAN));
    let (em, sm) = (g("entry"), g("stop"));
    let to = to.min(x.c.len().saturating_sub(1));
    let (o, h, l, c) = (&x.o, &x.h, &x.l, &x.c);
    let mut out = Vec::new();
    let Some((from, to)) = crate::intraday_ep6::window(id, x, from, to, p) else { return Some(vec![]) }; // EP6 #243 (c)
    let on = crate::intraday_ep6::on(id, p); // EP6 (#243, lane K63)
    let mut st = St { pos: None, buy_live: false, sell_live: false, out: vec![], od: crate::intraday_ep6::Day::Plain };
    // week state
    let (mut wk, mut wn) = (i64::MIN, 0usize);
    let (mut ch, mut cl) = (f64::NAN, f64::NAN); // current week high-low
    let (mut buy, mut sell, mut wopen, mut rng) = (f64::NAN, f64::NAN, f64::NAN, f64::NAN);
    for t in 0..=to {
        let k = week_key(&x.r[t].date);
        if k != wk {
            wk = k; wn += 1;
            rng = ch - cl; ch = h[t]; cl = l[t];
            wopen = o[t];
            buy = wopen + em * rng; sell = wopen - em * rng;
            st.buy_live = t >= from && rng.is_finite(); st.sell_live = st.buy_live;
            // first profitable weekly open for a position carried from an earlier week
            if let Some(q) = st.pos.take() {
                if q.week < wn && q.dir * (o[t] - q.px) > 0.0 { st.close(q, t, o[t], "first profitable weekly open"); } else { st.pos = Some(q); }
            }
        } else { ch = ch.max(h[t]); cl = cl.min(l[t]); }
        out.append(&mut st.out);
        if t < from { continue; }
        let w = Wk { buy, sell, wopen, rng, sm, wn };
        if on {
            use crate::intraday_ep6::{bar_segs, remark, resolve_conv, Seg};
            let day = resolve_conv(x, t, true, &mut st, |s: &mut St| s.bar(x, t, &w),
                |s: &mut St, hf: bool| { let mut e = s.walk(x, t, &w, &[]); e.extend(s.walk(x, t, &w, &bar_segs(o[t], h[t], l[t], c[t], hf))); e },
                |s: &mut St, g: &[Seg]| s.walk(x, t, &w, g));
            for mut tr in st.out.drain(..) { remark(&mut tr, day); out.push(tr); }
            if st.pos.is_some() { st.od = st.od.worst(day); }
        } else {
            st.bar(x, t, &w);
            out.append(&mut st.out);
        }
    }
    if let Some(q) = st.pos {
        let mut t = Trade { dir: q.dir, legs: vec![Leg { i: q.i, px: q.px, w: 1.0 }], exit_i: to, exit_px: c[to], reason: "open".into(), entry_kind: "stop", signal_i: to, open: true, exits: vec![] };
        crate::intraday_ep6::mark(&mut t, st.od);
        out.push(t);
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;
    fn row(d: &str, o: f64, h: f64, l: f64, c: f64) -> Row { Row { ticker: "T".into(), date: d.into(), open: o, high: h, low: l, close: c, volume: None } }
    #[test]
    fn breakout_then_first_profitable_open() {
        // prior week range 110-90 = 20 -> entry 6, stop 2. Week 2 opens 100: buy stop 106, stop 98.
        let r = vec![
            row("2008-10-06", 100., 110., 95., 100.), row("2008-10-10", 100., 105., 90., 100.),
            row("2008-10-13", 100., 101., 99., 100.5), // no fill
            row("2008-10-14", 101., 107., 100.5, 106.5), // buy at 106 (open nearer the low: low first, then high)
            row("2008-10-20", 108., 109., 107., 108.), // next week opens 108 > 106: exit at the open
        ];
        let x = Ctx::new(&r, &r, &[]);
        let t = run("jrfsp_weekly_breakout", &x, 0, r.len() - 1, &Params::new()).unwrap();
        assert_eq!(t.len(), 1);
        assert_eq!((t[0].dir, t[0].legs[0].i, t[0].legs[0].px, t[0].exit_i, t[0].exit_px), (1.0, 3, 106.0, 4, 108.0));
    }
}
