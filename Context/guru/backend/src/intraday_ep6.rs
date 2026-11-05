//! EP6 (lane K59, 5 Nov 2026, decisions #240/#243): the shared minute-order resolver for daily-bar rules.
//!
//! A daily rule walks each bar along a price path (its declared convention, e.g. TradeStation's open -> nearer extreme ->
//! other extreme -> close). A day is ORDER-DEPENDENT when the rule's events that day (fills of entry / stop / target levels)
//! differ between the two possible extreme orders. Only on such days, and only for the ids in `IDS` with `p.order` = 1, the
//! day is walked again on the real 1-minute path of the same ticker (intraday_w2b::day: the minute file, the cleaner's
//! excluded days and the rejected NIFTY 50 days; the same loader intraday_ep::day_bars uses). Each minute is one more bar:
//! a gap from the previous price to the minute's open (an order the gap passes fills at the minute's open), then the minute's
//! own open -> nearer extreme -> other extreme -> close. A minute whose events differ between its two extreme orders is
//! AMBIGUOUS (#175: the order inside one minute is unknown); it takes the same convention inside the minute (nearer extreme
//! first) and is counted. A day without minutes keeps the module's declared convention (counted as 'by convention').
//! Days that are not order-dependent are walked exactly as before, so their results do not change.
use crate::connors::{ParamDef, Params, Trade};
use crate::intraday::Bar1;

/// One straight piece of a day's price path, from `a` to `b`. `gap`: the price jumped (no trade in between), so an order
/// the jump passes fills at `b`, not at its level.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Seg { pub a: f64, pub b: f64, pub gap: bool }

impl Seg {
    pub fn up(&self) -> bool { self.b > self.a }
    /// A level the segment reaches (excluding its start, including its end), in the direction it moves.
    pub fn inside(&self, lv: f64) -> bool { lv.is_finite() && if self.up() { lv > self.a && lv <= self.b } else { lv < self.a && lv >= self.b } }
    /// Fill price of an order at `lv` reached on this segment.
    pub fn fill(&self, lv: f64) -> f64 { if self.gap { self.b } else { lv } }
}

/// A bar's path O -> H -> L -> C (`high_first`) or O -> L -> H -> C.
pub fn bar_segs(o: f64, h: f64, l: f64, c: f64, high_first: bool) -> [Seg; 3] {
    let p = if high_first { [o, h, l, c] } else { [o, l, h, c] };
    [Seg { a: p[0], b: p[1], gap: false }, Seg { a: p[1], b: p[2], gap: false }, Seg { a: p[2], b: p[3], gap: false }]
}

/// TradeStation's daily-bar path (lc3::high_first): the extreme nearer the open first, ties the high.
pub fn ts_high_first(o: f64, h: f64, l: f64) -> bool { h - o <= o - l }

/// How one day was decided.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Day {
    /// Not order-dependent (both extreme orders give the same events), or the resolver is off: the declared convention.
    Plain,
    /// Order-dependent, decided on the real 1-minute path; no minute had two events in an unknown order.
    Minutes,
    /// Order-dependent, no minutes that day: the module's declared convention.
    Convention,
    /// Order-dependent, decided on the 1-minute path, but at least one minute reached two levels in an unknown order (#175).
    Ambiguous,
}

impl Day {
    /// Note appended to the reason of a trade an order-dependent day decided (empty for Plain).
    pub fn note(self) -> &'static str {
        match self {
            Day::Plain => "",
            Day::Minutes => " · same-day order: real 1-minute path",
            Day::Convention => " · same-day order: daily-bar convention (no minutes that day)",
            Day::Ambiguous => " · same-day order: 1-minute path, ambiguous minute (nearer extreme first)",
        }
    }
    /// The worse of two marks for one trade (Ambiguous > Convention > Minutes > Plain).
    pub fn worst(self, o: Day) -> Day { self.max(o) }
}

/// Resolve bar `i` of `x` for a rule whose bar walk is `walk(state, path) -> events`. `st` is the state before the
/// intrabar part of the bar (after the open fills); on return it is the state after it. `high_first` = the module's own
/// daily order for this bar (its declared convention). `daily(state, high_first)` is the module's daily intrabar step
/// (for path simulators: `walk` on `bar_segs`). Events must compare equal exactly when the outcomes are the same.
pub fn resolve<S: Clone>(x: &crate::connors::Ctx, i: usize, on: bool, high_first: bool, st: &mut S,
    daily: impl Fn(&mut S, bool) -> Vec<Ev>, walk: impl Fn(&mut S, &[Seg]) -> Vec<Ev>) -> Day {
    resolve_conv(x, i, on, st, |s| { daily(s, high_first); }, &daily, walk)
}

/// `resolve` for a module whose daily convention is not a bar-path order (e.g. 'the stop first'): `probe(state, high_first)`
/// is the rule walked on the bar's path in either order (it only decides whether the day is order-dependent), and `conv`
/// is the module's own daily step, used on every day that is not resolved on minutes.
pub fn resolve_conv<S: Clone>(x: &crate::connors::Ctx, i: usize, on: bool, st: &mut S, conv: impl Fn(&mut S),
    probe: impl Fn(&mut S, bool) -> Vec<Ev>, walk: impl Fn(&mut S, &[Seg]) -> Vec<Ev>) -> Day {
    if !on { conv(st); return Day::Plain; }
    let (mut a, mut b) = (st.clone(), st.clone());
    if probe(&mut a, true) == probe(&mut b, false) { conv(st); return Day::Plain; }
    let Some(m) = crate::intraday_w2b::day(x, i).filter(|m| !m.is_empty()) else { conv(st); return Day::Convention };
    let (amb, s) = walk_minutes(&m, x.o[i], st.clone(), &walk);
    *st = s;
    if amb { Day::Ambiguous } else { Day::Minutes }
}

/// One fill of a bar walk: what happened (module code), at which price, and whether the path was moving up (the level is
/// touched from below: a buy stop, a sell limit, a short's stop) or down.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Ev { pub k: u8, pub px: f64, pub up: bool }

/// Walk the day's minutes from the daily open `open`: returns (any ambiguous minute, final state). A minute whose fills
/// differ between its two extreme orders reached two levels inside it. #175: when the minute OPENED THROUGH exactly one of
/// those levels (a level that was not live at the open, such as the stop of an entry filled later in the minute), that
/// level's side came first and the minute is not ambiguous; otherwise it is ambiguous and the nearer extreme goes first.
pub fn walk_minutes<S: Clone>(m: &[Bar1], open: f64, mut s: S, walk: &impl Fn(&mut S, &[Seg]) -> Vec<Ev>) -> (bool, S) {
    let (mut prev, mut amb) = (open, false);
    walk(&mut s, &[]); // a module's bar-open step (if it has one) runs once, before the first minute
    for z in m {
        if z.o != prev { walk(&mut s, &[Seg { a: prev, b: z.o, gap: true }]); }
        let hf = ts_high_first(z.o, z.h, z.l);
        let (mut s1, mut s2) = (s.clone(), s.clone());
        let e1 = walk(&mut s1, &bar_segs(z.o, z.h, z.l, z.c, hf));
        let e2 = walk(&mut s2, &bar_segs(z.o, z.h, z.l, z.c, !hf));
        s = if e1 == e2 { s1 } else {
            let mut lv: Vec<(f64, bool)> = e1.iter().chain(&e2).map(|e| (e.px, e.up)).collect();
            lv.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)));
            lv.dedup();
            let through: Vec<bool> = lv.iter().filter(|&&(p, up)| if up { z.o >= p } else { z.o <= p }).map(|&(_, up)| up).collect();
            if through.len() == 1 { if through[0] == hf { s1 } else { s2 } } else { amb = true; s1 }
        };
        prev = z.c;
    }
    (amb, s)
}

/// The ids whose same-day order is resolved on minutes by default (#243; M6.tsv over 5%). Wave 1: the Larry Williams
/// lc3-family simulators (lwlts, lwdtfo on lwlts::simulate, lwnl, lwicw).
pub const IDS: &[&str] = &[
    "lwlts_mon_gap_t500", "lwlts_mon_gap_t1000", "lwlts_oops_sell_ma9", "lwlts_oops_buy_ma9", "lwlts_oops_sell_thu", "lwlts_oops_buy",
    "lwlts_vb_sp_40_200", "lwlts_talon_sp", "lwlts_hh1", "lwlts_hh5", "lwlts_hh10", "lwlts_hh15", "lwlts_hh20", "lwdtfo_vb_open",
    "lwnl_jul97_td4_buy", "lwnl_jan_rally", "lwnl_jul97_td16_buy_30d", "lwnl_nov97_td15_buy_30d", "lwnl_mar_short", "lwnl_proj_contra",
    "lwnl_jul97_td17_buy", "lwnl_aug97_td2_sell_7d", "lwnl_pullback", "lwnl_aug97_td7_sell_bail", "lwnl_nov97_td17_buy_7d",
    "lwnl_nov97_td6_buy_30d", "lwnl_nov97_td18_buy_7d", "lwicw_oops_tdom_buy", "lwicw_oops_tdom_sell",
    // wave 2: btp2 (books/btp.rs sim; its daily convention is 'the stop first', not a bar-path order: resolve_conv)
    "btp2_wide_range_exhaustion_gap_long", "btp2_momentum_ma_long", "btp2_momentum_ma_short", "btp2_gipsons_long",
    "btp2_8day_hl_reversal_short", "btp2_8day_hl_reversal_long", "btp2_1234_long", "btp2_1234_short"];

/// #243 (c): ids whose convention result fails the pre-set #177 tests against the resolved result on the minute window:
/// only the minute window is published (resolved), the earlier years are withdrawn.
pub const WINDOW_ONLY: &[&str] = &["btp2_8day_hl_reversal_long", "btp2_8day_hl_reversal_short", "btp2_wide_range_exhaustion_gap_long",
    "lwicw_oops_tdom_buy", "lwlts_oops_buy", "lwlts_oops_buy_ma9", "lwlts_vb_sp_40_200", "lwnl_aug97_td7_sell_bail", "lwnl_mar_short",
    "lwnl_proj_contra", "lwnl_pullback"];

/// #243: ids HELD. The five thin ids (M6: over 5% order-dependent, fewer than 10 trades inside the minute window), and EP6 ids
/// whose resolved path has more than 5% ambiguous-minute trades (#175).
pub const HELD: &[(&str, &str)] = &[
    ("btp2_gipsons_short", THIN), ("dl10b_rgs", THIN), ("jr_ledge_tbtb", THIN), ("ws5_gap_window_intraday", THIN), ("ws5_outside_window", THIN),
    ("lwnl_aug97_td2_sell_7d", AMB), ("lwnl_jan_rally", AMB), ("lwnl_jul97_td17_buy", AMB), ("lwnl_jul97_td4_buy", AMB), ("lwnl_nov97_td18_buy_7d", AMB)];
const AMB: &str = "Held (decision #243, lane K59): on SPX's real 1-minute bars one of its 11-12 trades in the minute window (8.3-9.1%) is decided inside a single minute that reaches two of its levels without opening through exactly one of them, above decision #175's 5% limit, so the same-day order cannot be resolved and no results are published.";
const THIN: &str = "Held (decision #243, lane K59): more than 5% of its trades enter at a stop or limit level and reach an exit level the same day, so the daily bar cannot say which came first (#240), and fewer than 10 of its trades fall inside the 1-minute window, so the order cannot be resolved on real minutes. No results are published.";
const WINDOW_NOTE: &str = "Not published on this ticker (decision #243 (c), lane K59): the daily-bar convention changes this rule's result materially against the real 1-minute order, so only the minute-resolved window of the 1-minute tickers (SPY, SPX, NIFTY 50, NIFTYBANK) is published.";

/// The hold or minute-window refusal for (id, ticker), if any (instrument_policy::tradable_check_issue).
pub fn held_note(id: &str, ticker: &str) -> Option<&'static str> {
    if let Some(&(_, n)) = HELD.iter().find(|h| h.0 == id) { return Some(n); }
    (WINDOW_ONLY.contains(&id) && !matches!(ticker, "SPY" | "SPX" | "NIFTY 50" | "NIFTYBANK" | "NIFTY BANK")).then_some(WINDOW_NOTE)
}

/// The resolver is on for this run (an EP6 id and p.order = 1, the default).
pub fn on(id: &str, p: &Params) -> bool { IDS.contains(&id) && p.get("order").copied().unwrap_or(1.0) >= 0.5 }

/// The labelled switch, appended to every EP6 id's parameters.
pub fn add_param(id: &str, v: &mut Vec<ParamDef>) {
    if IDS.contains(&id) {
        v.push(ParamDef { key: "order", label: "Same-day order of the stop/limit levels: 1 = real 1-minute path where minutes exist (#243), 0 = the daily-bar convention (book check)", def: 1.0, min: 0.0, max: 1.0, step: 1.0 });
    }
}

/// #243 (c): the run window clamped to the ticker's minute file for WINDOW_ONLY ids with the resolver on (None: the window
/// and the minute file do not overlap); otherwise the window unchanged.
pub fn window(id: &str, x: &crate::connors::Ctx, from: usize, to: usize, p: &Params) -> Option<(usize, usize)> {
    if !(WINDOW_ONLY.contains(&id) && on(id, p)) { return Some((from, to)); }
    // no minutes loaded (a unit-test context, or a ticker without minute files, which held_note already refuses): unchanged
    let Some(m) = x.minutes else { return Some((from, to)) };
    let bad = |r: &crate::Row| r.ticker == "NIFTY 50" && r.date.as_str() >= crate::intraday_w2b::NIFTY_BAD_FROM; // rejected minutes
    let (a, b) = (x.r.partition_point(|r| r.date.as_str() < m.first()), x.r.partition_point(|r| r.date.as_str() <= m.last() && !bad(r)));
    let (f, t) = (from.max(a), to.min(b.checked_sub(1)?));
    (f <= t).then_some((f, t))
}

/// Append the day's mark to a trade's reason.
pub fn mark(t: &mut Trade, d: Day) { if d != Day::Plain { t.reason.push_str(d.note()); } }

/// Re-mark a trade closed on a bar decided as `d`: the worse of its existing mark (from its position) and `d`.
pub fn remark(t: &mut Trade, d: Day) {
    if d == Day::Plain { return; }
    let old = [Day::Ambiguous, Day::Convention, Day::Minutes].into_iter().find(|k| t.reason.ends_with(k.note())).unwrap_or(Day::Plain);
    if old != Day::Plain { let n = t.reason.len() - old.note().len(); t.reason.truncate(n); }
    mark(t, old.worst(d));
}

#[cfg(test)]
mod tests {
    use super::*;
    fn b(o: f64, h: f64, l: f64, c: f64) -> Bar1 { Bar1 { m: 0, o, h, l, c } }
    // a tiny walker: long buy-stop entry at `e`, then a sell stop at `s`; events = (code, price)
    #[derive(Clone)]
    struct St { e: f64, s: f64, long: Option<f64>, out: Vec<(f64, f64)> }
    fn walk(st: &mut St, segs: &[Seg]) -> Vec<Ev> {
        let mut ev = vec![];
        for g in segs {
            let mut g = *g;
            loop {
                let hit = match st.long { None if g.up() && g.inside(st.e) => Some((0u8, st.e)), Some(_) if !g.up() && g.inside(st.s) => Some((1, st.s)), _ => None };
                let Some((k, lv)) = hit else { break };
                let px = g.fill(lv);
                if k == 0 { st.long = Some(px); st.e = f64::NAN; } else { st.out.push((st.long.take().unwrap(), px)); }
                ev.push(Ev { k, px, up: g.up() });
                g.a = lv;
            }
        }
        ev
    }
    #[test]
    fn minute_order_decides_and_ambiguity_is_counted() {
        let st = St { e: 101.0, s: 99.0, long: None, out: vec![] };
        // daily bar 100 / 102 / 98 / 100: TS path (open nearer the low... h-o = 2 = o-l: high first) enters and stops.
        let minutes = [b(100.0, 100.5, 98.0, 99.5), b(99.5, 102.0, 99.4, 101.5)]; // the low came first: entered, not stopped
        let (amb, s) = walk_minutes(&minutes, 100.0, st.clone(), &walk);
        assert!(!amb && s.long == Some(101.0) && s.out.is_empty());
        // one minute reaches both levels: ambiguous; inside it the nearer extreme (the high) first, so entered then stopped
        let (amb, s) = walk_minutes(&[b(100.0, 102.0, 98.0, 99.5)], 100.0, st.clone(), &walk);
        assert!(amb && s.out == vec![(101.0, 99.0)]);
        // a gap through the entry level fills at the minute's open
        let (amb, s) = walk_minutes(&[b(100.0, 100.2, 99.9, 100.0), b(101.5, 101.8, 101.2, 101.6)], 100.0, st, &walk);
        assert!(!amb && s.long == Some(101.5));
    }
    #[test]
    fn switch_and_window_defaults() {
        let mut p = Params::new();
        assert!(on("lwlts_hh1", &p) && !on("lwlts_tdm6_sp", &p));
        p.insert("order".into(), 0.0);
        assert!(!on("lwlts_hh1", &p));
        let mut v = vec![];
        add_param("lwlts_tdm6_sp", &mut v);
        assert!(v.is_empty());
        add_param("lwlts_hh1", &mut v);
        assert_eq!(v[0].key, "order");
        assert!(Day::Ambiguous.worst(Day::Minutes) == Day::Ambiguous && Day::Plain.note().is_empty());
    }
    #[test]
    fn held_ids_publish_nothing() {
        let all = crate::connors::catalogue();
        for (id, _) in HELD {
            let d = all.iter().find(|d| d.id == *id).unwrap_or_else(|| panic!("{id}"));
            for t in ["SPY", "SPX", "QQQ", "NIFTY 50"] {
                assert!(crate::instrument_policy::check(d, t, false).is_some_and(|i| i.note.starts_with("Held (decision #243")), "{id} {t}");
            }
        }
    }
    fn minutes(name: &str, days: &[(&str, &[&str])]) -> &'static crate::intraday::Minutes {
        let dir = std::env::temp_dir().join(format!("ep6{name}{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let (mut body, mut idx) = (String::from("Date,Time,Open,High,Low,Close\n"), String::from("Date,offset,bytes\n"));
        for (d, lines) in days {
            let start = body.len();
            for l in *lines { body.push_str(&format!("{d},{l}\n")); }
            idx.push_str(&format!("{d},{start},{}\n", body.len() - start));
        }
        std::fs::write(dir.join("T.m1.csv"), &body).unwrap();
        std::fs::write(dir.join("T.m1idx.csv"), &idx).unwrap();
        Box::leak(Box::new(crate::intraday::Minutes::load(dir.to_str().unwrap(), "T").unwrap()))
    }
    fn row(d: &str, o: f64, h: f64, l: f64, c: f64) -> crate::Row { crate::Row { ticker: "T".into(), date: d.into(), open: o, high: h, low: l, close: c, volume: None } }
    #[test]
    fn lwlts_oops_entry_day_decided_on_minutes() {
        // Monday opens 95 under Friday's low 99: buy stop 99, $2,000 = 8-point stop at 91 armed on the entry day. The daily bar
        // (h-o = o-l: high first) enters and stops; the minutes put the low (90) before the fill, so the trade is not stopped.
        let r = vec![row("2020-01-02", 100., 101., 99., 100.), row("2020-01-03", 100., 101., 99., 100.), row("2020-01-06", 95., 100., 90., 96.)];
        let mut x = crate::connors::Ctx::new(&r, &r, &[]);
        let conv = crate::books::lwlts::run("lwlts_oops_buy", &x, 0, 2, &[("order".to_string(), 0.0)].into_iter().collect()).unwrap();
        assert!(conv.len() == 1 && conv[0].exit_px == 91.0 && !conv[0].reason.contains("same-day"), "{conv:?}");
        let none = crate::books::lwlts::run("lwlts_oops_buy", &x, 0, 2, &Params::new()).unwrap();
        assert!(none.len() == 1 && none[0].exit_px == 91.0 && none[0].reason.ends_with(Day::Convention.note()), "{none:?}");
        x.minutes = Some(minutes("oops", &[("2020-01-06", &["09:30,95,95.5,90,91.5", "09:31,91.5,100,91.5,99.5", "09:32,99.5,99.6,95.5,96"])]));
        let t = crate::books::lwlts::run("lwlts_oops_buy", &x, 0, 2, &Params::new()).unwrap();
        assert!(t.len() == 1 && t[0].open && t[0].legs[0].px == 99.0 && t[0].reason.ends_with(Day::Minutes.note()), "{t:?}");
        // a card that is not an EP6 id is untouched: lwlts_mon_gap_close has no same-day order question here either way
        assert!(crate::books::lwlts::params("lwlts_mon_gap_close").unwrap().iter().all(|d| d.key != "order"));
    }
}
