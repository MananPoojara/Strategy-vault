//! Lane R2E (13 Nov 2026, R4 #297, #305 (3), #311): Jason Nguyen, "Impact Trading System" (tradingimpact.com e-book,
//! part 1; research share 'USB HDD 2/tRADING KNO-2/Jason Nguyen - Impact Trading System (tradingimpact.com)/impact forex
//! trading system part 1.pdf', rules PDF p.5-19). Book id `jnits`. A forex trend-pullback system shown on hourly charts; run
//! here on daily bars of the four 1-minute tickers, the entry day and every held day walked on the real 1-minute path where the
//! daily bar cannot order the entry stop, the protective stop and the two targets (EP6, #243 (c): minute window only).
//! Independent reference: authors/titles-1030/R2Ew/r2e_nguyen.py (written from the pages and the readings, not from this file).
use crate::connors::{sma, Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};
use crate::intraday_ep6::{bar_segs, resolve, ts_high_first, Day, Ev, Seg};
use super::atm03::cci_of;

pub const BOOK: &str = "jnits";
pub const ID: &str = "jnits_impact";
pub fn reviewed(id: &str) -> bool { id == ID }

const NOTE: &str = "[R2E, R4 #297/#311: adapted: 2 unstated details] Unstated in the book: how long the entry stop works; vault reading: it is placed after each close that meets the set-up (trend + CCI8 beyond -100/+100) at that bar's high + 1 tick (low - 1 tick for shorts) and works for the next bar only. Unstated in the book: where the pullback starts for the stop; vault reading: the stop is 1 tick below the lowest low (above the highest high) from the first bar of the current CCI8 leg below 0 (above 0) through the set-up bar. Stated: SMA8 > SMA20 > SMA40 with SMA20 and SMA40 'flat or rising' = not falling (R4 #311 (a)); CCI(8) = (TP - SMA8(TP)) / (0.015 x mean deviation), TP = (H+L+C)/3; 2 lots: lot 1 target = entry + the risk (1R), lot 2 target = entry + 2R, the stop for both (Exit Strategy 1, default); p.exit = 2 = the book's Exit Strategy 2 (lot 2 leaves at the close of the first day the SMA5 is falling; R4 #311 (b)). 1 pip = 1 tick of the traded instrument (US 0.01, NSE 0.05; #57). Fills: stops and targets at their level, or at the open when the open is already through them. Same-day order (#243, #305 (3)): when the daily bar cannot say which of the entry, the stop and the targets came first, the day is walked on the real 1-minute bars; published on the minute window only. Daily bars of an hourly forex system are a declared application; the book prints chart examples only (no results), so the card is UNVERIFIED.";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef { id: ID, name: "Nguyen Impact: SMA 8/20/40 trend, CCI(8) pullback, stop entry over the last bar, 1R/2R targets", group: "Impact Trading System (PDF p.5-19)",
        session: 0, dir: "Long/Short", status: Status::Live, needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK,
        rules: &["F|\"SMA8 above SMA20, SMA20 above SMA40. Both SMA20 and SMA40 are either flat or rising\" (Figure 1); short: below / 'flat or falling' (Figure 9)",
            "F|\"We then look for price to retrace/pullback. This retracement must make CCI8 to fall below -100 level\" (short: rise above +100)",
            "E|\"We then place a buy order at 1 pip above the high of the last candle/bar. When the buy order is triggered, we're in the trade. For best results, we should enter with 2 lots.\" (short: 1 pip below the low)",
            "X|\"We place stop loss 1 pip below the lowest price of the retracement/pullback.\" (short: above the highest price)",
            "X|Exit Strategy 1: \"target objective for the first lot such that our potential profit is equal to our potential loss\"; \"for the second lot ... two times our potential loss\"",
            "A|Exit Strategy 2 (p.exit = 2): first lot as above; \"we let it run as long as SMA5 is still pointing upward. We only exit the second lot when the SMA5 turns from upward to downward\""],
        notes: NOTE, claim: None }]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> { reviewed(id).then_some((ID, "Nguyen Impact trend pullback", "1R/2R")) }

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    reviewed(id).then(|| vec![p("exit", "Exit strategy: 1 = 1R/2R targets (book default), 2 = lot 2 leaves when the SMA5 turns down", 1., 1., 2., 1.),
        p("tick", "Tick size (0 = instrument: 0.05 NSE, 0.01 US)", 0., 0., 1., 0.01),
        p("order", "Same-day order of the levels: 1 = real 1-minute path (#243, #305 (3)), 0 = the daily-bar path (open, nearer extreme, other extreme, close)", 1., 0., 1., 1.)])
}

#[derive(Clone)]
struct Pos { dir: f64, i: usize, px: f64, stop: f64, t1: f64, t2: f64, lots: [bool; 2], exits: Vec<(usize, f64, f64)>, day: Day, why: &'static str }

#[derive(Clone)]
struct St { i: usize, o: f64, targets2: bool, pend: Option<(f64, f64, f64)>, pos: Option<Pos>, done: Vec<Trade> }

fn finish(p: Pos, why: &'static str) -> Trade {
    let last = p.exits.iter().map(|e| e.0).max().unwrap_or(p.i);
    let px = p.exits.iter().map(|e| e.1 * e.2).sum::<f64>() / p.exits.iter().map(|e| e.2).sum::<f64>();
    let mut t = Trade { dir: p.dir, legs: vec![Leg { i: p.i, px: p.px, w: 1. }], exit_i: last, exit_px: px, reason: why.into(), entry_kind: "stop",
        signal_i: last, open: false, exits: p.exits };
    crate::intraday_ep6::mark(&mut t, p.day);
    t
}

impl St {
    /// Levels live now: (code, level, reached moving up). 0 entry, 1 stop, 2 lot-1 target, 3 lot-2 target.
    fn levels(&self) -> Vec<(u8, f64, bool)> {
        match (&self.pos, self.pend) {
            (Some(p), _) => {
                let mut v = vec![(1u8, p.stop, p.dir < 0.)];
                if p.lots[0] { v.push((2, p.t1, p.dir > 0.)); }
                if p.lots[1] && self.targets2 { v.push((3, p.t2, p.dir > 0.)); }
                v
            }
            (None, Some((d, lv, _))) => vec![(0, lv, d > 0.)],
            _ => vec![],
        }
    }
    fn hit(&mut self, k: u8, px: f64) {
        let i = self.i;
        if k == 0 {
            let (d, _, stop) = self.pend.take().unwrap();
            let r = (px - stop) * d;
            self.pos = Some(Pos { dir: d, i, px, stop, t1: px + d * r, t2: px + 2. * d * r, lots: [true, true], exits: vec![], day: Day::Plain, why: "" });
            return;
        }
        let Some(p) = self.pos.as_mut() else { return };
        match k {
            1 => { for l in 0..2 { if p.lots[l] { p.lots[l] = false; p.exits.push((i, px, 0.5)); } } p.why = "stop"; }
            2 => { p.lots[0] = false; p.exits.push((i, px, 0.5)); p.why = "1R target"; }
            _ => { p.lots[1] = false; p.exits.push((i, px, 0.5)); p.why = "2R target"; }
        }
        if !p.lots[0] && !p.lots[1] { let p = self.pos.take().unwrap(); let w = p.why; self.done.push(finish(p, w)); }
    }
    /// The bar walk: `segs` empty = the bar's open (levels the open is already through fill there).
    fn walk(&mut self, segs: &[Seg]) -> Vec<Ev> {
        let mut ev = vec![];
        if segs.is_empty() {
            loop {
                let o = self.o;
                let Some(&(k, _, up)) = self.levels().iter().find(|&&(_, lv, up)| if up { o >= lv } else { o <= lv }) else { break };
                self.hit(k, o); ev.push(Ev { k, px: o, up });
            }
            return ev;
        }
        for g in segs {
            let mut g = *g;
            loop {
                let cand = self.levels().into_iter().filter(|&(_, lv, up)| up == g.up() && g.inside(lv))
                    .min_by(|a, b| (a.1 - g.a).abs().total_cmp(&(b.1 - g.a).abs()));
                let Some((k, lv, up)) = cand else { break };
                let px = g.fill(lv);
                self.hit(k, px); ev.push(Ev { k, px, up });
                g.a = lv;
            }
        }
        ev
    }
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from > to || to >= x.r.len() { return None; }
    let schema = params(id)?;
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).map_or(0., |d| d.def));
    let Some((from, to)) = crate::intraday_ep6::window(id, x, from, to, p) else { return Some(vec![]) }; // #243 (c)
    let on = crate::intraday_ep6::on(id, p);
    let (o, h, l, c) = (&x.o, &x.h, &x.l, &x.c);
    let tick = if g("tick") > 0. { g("tick") } else { x.market_tick() };
    let (s5, s8, s20, s40) = (sma(c, 5), sma(c, 8), sma(c, 20), sma(c, 40));
    let cci = cci_of(h, l, c, 8);
    let r8 = |v: f64| (v * 1e8).round() / 1e8;
    let mut st = St { i: 0, o: 0., targets2: g("exit").round() != 2., pend: None, pos: None, done: vec![] };
    for i in from..=to {
        (st.i, st.o) = (i, o[i]);
        let n0 = st.done.len();
        let day = resolve(x, i, on, ts_high_first(o[i], h[i], l[i]), &mut st,
            |s, hf| { let mut e = s.walk(&[]); e.extend(s.walk(&bar_segs(o[i], h[i], l[i], c[i], hf))); e },
            |s, segs| s.walk(segs));
        if let Some(p) = st.pos.as_mut() { p.day = p.day.worst(day); }
        for t in st.done[n0..].iter_mut() { crate::intraday_ep6::remark(t, day); }
        st.pend = None; // the entry stop works for one bar only
        // Exit Strategy 2: lot 2 leaves at the close of a day the SMA5 is falling (rising for shorts)
        if !st.targets2 && i >= 1 && s5[i].is_finite() && s5[i - 1].is_finite() {
            if let Some(p) = st.pos.as_mut().filter(|p| p.lots[1] && (s5[i] - s5[i - 1]) * p.dir < 0.) {
                p.lots[1] = false; p.exits.push((i, c[i], 0.5)); p.why = "SMA5 turned";
                if !p.lots[0] { let p = st.pos.take().unwrap(); st.done.push(finish(p, "SMA5 turned")); }
            }
        }
        if st.pos.is_some() || i >= to || i < 1 || ![s8[i], s20[i], s40[i], s20[i - 1], s40[i - 1], cci[i]].iter().all(|v| v.is_finite()) { continue; }
        let up = s8[i] > s20[i] && s20[i] > s40[i] && s20[i] >= s20[i - 1] && s40[i] >= s40[i - 1] && cci[i] < -100.;
        let dn = s8[i] < s20[i] && s20[i] < s40[i] && s20[i] <= s20[i - 1] && s40[i] <= s40[i - 1] && cci[i] > 100.;
        if !(up || dn) { continue; }
        let d = if up { 1. } else { -1. };
        let mut k = i;
        while k > 0 && cci[k - 1].is_finite() && cci[k - 1] * d < 0. { k -= 1; }
        let ext = if up { l[k..=i].iter().copied().fold(f64::INFINITY, f64::min) } else { h[k..=i].iter().copied().fold(f64::NEG_INFINITY, f64::max) };
        st.pend = Some((d, r8(if up { h[i] + tick } else { l[i] - tick }), r8(ext - d * tick)));
    }
    let mut out = st.done;
    if let Some(mut p) = st.pos {
        let rest: f64 = p.lots.iter().filter(|&&b| b).count() as f64 * 0.5;
        p.exits.push((to, c[to], rest));
        let mut t = finish(p, "open");
        t.open = true;
        out.push(t);
    }
    out.sort_by_key(|t| t.legs[0].i);
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn impact_two_lots_split_and_levels_follow_the_risk() {
        let r = super::super::sctraub::tests::bars(3000);
        let x = Ctx::new(&r, &r, &[]);
        let t = run(ID, &x, 0, r.len() - 1, &Params::new()).unwrap();
        assert!(!t.is_empty());
        for z in t.iter().filter(|z| !z.open) {
            assert!((z.exits.iter().map(|e| e.2).sum::<f64>() - 1.).abs() < 1e-12);
            assert!(z.exits.iter().all(|e| e.0 >= z.legs[0].i));
        }
        let t2 = run(ID, &x, 0, r.len() - 1, &Params::from([("exit".into(), 2.)])).unwrap();
        assert!(!t2.is_empty());
        assert!(crate::intraday_ep6::IDS.contains(&ID) && crate::intraday_ep6::WINDOW_ONLY.contains(&ID));
    }
}

// Instrument admission: Nguyen Impact, lane R2E (R4 #311): daily application, minute window only (#243 (c)).
pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book == BOOK && reviewed(def.id),
    label: "Jason Nguyen's hourly forex system on daily bars, published only on the minute windows of the 1-minute tickers (the daily bar cannot order its entry stop, stop and two targets). Adapted: 2 unstated details, labelled on the card.",
    basis: "Impact Trading System part 1 (tradingimpact.com), PDF p.5-19; R4 #297, #305 (3), #311; independent check authors/titles-1030/R2Ew/r2e_nguyen.py",
};
