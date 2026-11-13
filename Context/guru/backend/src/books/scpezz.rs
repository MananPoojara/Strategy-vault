//! Lane PE (13 Nov 2026; released by lane PC, authors/pc/PC.tsv 'CODABLE-RELEASED'; R4 #297, #243, #319):
//! Paolo Pezzutti, "Designing Volatility Breakout Systems", Technical Analysis of Stocks & Commodities V.20:10 (2002) pp.50-54
//! (research share 'Trading Knowledge I/Stocks & Commodities DVD 11.26 (1982 - 2008) (traders.com)/VOLUMES/V20/C10/109des.pdf').
//! Book id `scpezz`. Coded: the core application, Figure 2 EasyLanguage (PDF p.3), on the real 1-minute path (EP6) of SPY/SPX
//! only (#243 (c): the minute window is published). Independent reference: authors/pe/w/pe_ref.py (from the article).
use crate::connors::{Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};
use crate::intraday_ep6::{bar_segs, remark, resolve, ts_high_first, Day, Ev, Seg};

pub const ID: &str = "scpezz_vbo";
const LIMIT: &str = "exit limit (previous day's extreme)";
pub fn reviewed(id: &str) -> bool { id == ID }

const NOTE: &str = "[Adapted: 2 unstated details, R4 #297; unverified] TradeStation order semantics of the printed code: each order is generated at a day's close for the next day; a Buy is placed only when not already long, a Sell only when not already short (no pyramiding; a Sell while long reverses it); a stop the open gaps through fills at the open. Unstated detail 1 (the code vs the text): the code buys on the stop even when the open is already above it (filled at the open); the text says 'buy-stop next day ... if the entry point is higher than the open' (option p.entry = 1: no entry when the open is at or beyond the level). Unstated detail 2 (the exit limit on the entry day): the limit at yesterday's high exits on the entry day only when the price rises to it after the fill; a limit at or below the fill price waits for the price to come back up to it (it is not filled at the entry price). Reference check on the article's market (Norgate &SP continuous futures 1996-01-02..2002-01-17, daily TradeStation path, authors/pe/w/pe_ref.py): 242 trades, 86.8% winners, net +$86,625 vs Figure 3's 250 trades, 94% winners, +$84,977.50 (the other reading of detail 2, a filled-at-entry exit, gives 249 trades but 71.9% winners). Published only on the real 1-minute window of SPY (2000-2025): the entry stop and the exit limit (or a reversing stop) often trade the same day (#240), and the daily convention flips the sign there (#243 (c)); the SPX cash-index minute result is withdrawn (#177a: its average trade is more than 3x SPY's). S&P futures points are not used: all levels are prices of the traded series. One unit per trade, no costs.";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef { id: ID, name: "Pezzutti volatility breakout: after a down close above the 13-day average, buy a stop at close + half the range; exit at a limit at the previous day's high (short mirror)",
        group: "S&C: Designing Volatility Breakout Systems (Pezzutti, V.20:10)", session: 0, dir: "Long/Short", status: Status::Live,
        needs_volume: false, needs_vix: false, needs_series: &[], book: "scpezz", claim: None, notes: NOTE,
        rules: &["F|\"Input: p(13),s(13),f(0.5); IF c<c[1] and c>average(c,p) Then Buy c+f*(h-l) Stop; ExitLong at high limit; IF c>c[1] and c<average(c,s) Then Sell c-f*(h-l) Stop; ExitShort at low limit;\" (Figure 2, PDF p.3)",
            "E|\"a Trend indicator: close>average(close, 13) b Setup: close<previous day's close ... c Trigger: buy-stop next day at close+0.5(high - low) if the entry point is higher than the open\" (PDF p.3)",
            "X|\"d Exit limit at previous day's high; exit at the previous day's high retest.\" (PDF p.3); no stop: \"stop-losses have not been applied\" (PDF p.4)",
            "F|Unstated in the article: (1) a gap above the stop (code: fill at the open; text: no entry, option p.entry); (2) an exit limit already below the entry fill on the entry day (vault reading: waits for a rise back to it). Same-day order on the real 1-minute path (EP6, p.order)"] }]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    (id == ID).then_some((ID, "Pezzutti volatility breakout (S&C)", "13-day trend, 0.5 range stop, previous-extreme limit"))
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    if id != ID { return None; }
    let mut v = vec![ParamDef { key: "entry", label: "Open beyond the entry stop [unstated, R4 #297]: 0 = fill at the open (the printed code), 1 = no entry (the text: 'if the entry point is higher than the open')", def: 0., min: 0., max: 1., step: 1. }];
    crate::intraday_ep6::add_param(id, &mut v);
    Some(v)
}

/// The day's orders: buy stop, sell stop (None = not placed), the long exit limit (yesterday's high), the short exit limit.
#[derive(Clone, Copy)]
struct Lv { buy: Option<f64>, sell: Option<f64>, xl: f64, xs: f64 }

/// pos: direction (0 flat), entry bar, entry price, entry-day mark; done: (dir, entry bar, entry px, exit px, entry mark) closed today.
#[derive(Clone)]
struct St { pos: f64, e: usize, px: f64, m: Day, entered: bool, done: Vec<(f64, usize, f64, f64, Day, &'static str)> }

impl St {
    fn exit(&mut self, xp: f64, why: &'static str) { self.done.push((self.pos, self.e, self.px, xp, self.m, why)); self.pos = 0.; }
    fn enter(&mut self, d: f64, k: usize, px: f64) { if self.pos != 0. { self.exit(px, "reversed by the opposite entry stop"); } self.pos = d; self.e = k; self.px = px; self.m = Day::Plain; self.entered = true; }
}

/// One price path through the day's orders: on each segment the first level reached fills (a gap fills at its end).
fn walk(s: &mut St, segs: &[Seg], lv: Lv, k: usize) -> Vec<Ev> {
    let mut ev = vec![];
    for g in segs {
        let mut cur = g.a;
        loop {
            let seg = Seg { a: cur, b: g.b, gap: g.gap };
            let mut best: Option<(f64, u8)> = None;
            let mut see = |lvl: f64, code: u8, up: bool| if up == seg.up() && seg.inside(lvl) && best.map_or(true, |(b, _)| if up { lvl < b } else { lvl > b }) { best = Some((lvl, code)); };
            if s.pos <= 0. && !s.entered { if let Some(b) = lv.buy { see(b, 0, true); } }
            if s.pos >= 0. && !s.entered { if let Some(b) = lv.sell { see(b, 1, false); } }
            if s.pos > 0. { see(lv.xl, 2, true); }
            if s.pos < 0. { see(lv.xs, 3, false); }
            let Some((lvl, code)) = best else { break };
            let px = seg.fill(lvl);
            match code { 0 => s.enter(1., k, px), 1 => s.enter(-1., k, px), _ => s.exit(px, LIMIT) }
            ev.push(Ev { k: code, px, up: code == 0 || code == 2 });
            cur = if g.gap { g.b } else { lvl };
        }
    }
    ev
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    let schema = params(id)?;
    if from > to || to >= x.c.len() { return None; }
    for k in p.keys() { if !schema.iter().any(|d| d.key == k) { return None; } }
    let Some((from, to)) = crate::intraday_ep6::window(id, x, from, to, p) else { return Some(vec![]) };
    let text = p.get("entry").copied().unwrap_or(0.) >= 0.5;
    let on = crate::intraday_ep6::on(id, p);
    let (o, h, l, c) = (&x.o, &x.h, &x.l, &x.c);
    let mut out = vec![];
    let mut s = St { pos: 0., e: 0, px: 0., m: Day::Plain, entered: false, done: vec![] };
    for k in (from + 1)..=to {
        let t = k - 1;
        if t < 13 { continue; }
        let a13 = c[t - 12..=t].iter().sum::<f64>() / 13.;
        let rg = h[t] - l[t];
        let mut lv = Lv { buy: None, sell: None, xl: h[t], xs: l[t] };
        if c[t] < c[t - 1] && c[t] > a13 && s.pos <= 0. { lv.buy = Some(c[t] + 0.5 * rg); }
        if c[t] > c[t - 1] && c[t] < a13 && s.pos >= 0. { lv.sell = Some(c[t] - 0.5 * rg); }
        if text { if lv.buy.is_some_and(|b| o[k] >= b) { lv.buy = None; } if lv.sell.is_some_and(|b| o[k] <= b) { lv.sell = None; } }
        if s.pos == 0. && lv.buy.is_none() && lv.sell.is_none() { continue; }
        s.entered = false;
        // the open: a position's exit limit, then an entry stop, that the open reaches or passes fills at the open
        if (s.pos > 0. && o[k] >= lv.xl) || (s.pos < 0. && o[k] <= lv.xs) { s.exit(o[k], LIMIT); }
        if s.pos <= 0. { if let Some(b) = lv.buy { if o[k] >= b { s.enter(1., k, o[k]); } } }
        if s.pos >= 0. && !s.entered { if let Some(b) = lv.sell { if o[k] <= b { s.enter(-1., k, o[k]); } } }
        let hf = ts_high_first(o[k], h[k], l[k]);
        let day = resolve(x, k, on, hf, &mut s, |z, hf| walk(z, &bar_segs(o[k], h[k], l[k], c[k], hf), lv, k), |z, g| walk(z, g, lv, k));
        if s.pos != 0. && s.e == k { s.m = day; } else if s.pos != 0. { s.m = s.m.worst(day); }
        for (d, e, px, xp, m, why) in s.done.drain(..) {
            let mut tr = Trade { dir: d, legs: vec![Leg { i: e, px, w: 1. }], exit_i: k, exit_px: xp, reason: why.into(), entry_kind: "stop", signal_i: k, open: false, exits: vec![] };
            remark(&mut tr, m); remark(&mut tr, day);
            out.push(tr);
        }
    }
    if s.pos != 0. { let mut tr = Trade { dir: s.pos, legs: vec![Leg { i: s.e, px: s.px, w: 1. }], exit_i: to, exit_px: c[to], reason: "open".into(), entry_kind: "stop", signal_i: to, open: true, exits: vec![] }; remark(&mut tr, s.m); out.push(tr); }
    Some(out)
}

#[cfg(test)]
mod tests {
    use crate::connors::{Ctx, Params};
    use crate::Row;
    fn r(d: &str, o: f64, h: f64, l: f64, c: f64) -> Row { Row { ticker: "SPY".into(), date: d.into(), open: o, high: h, low: l, close: c, volume: None } }

    /// Rising closes keep the 13-day average below; a down close (102 < 103, above the average) with range 100..104 places a buy
    /// stop at 102 + 2 = 104. The next day enters at 104; the limit there is yesterday's high 104, not above the fill, so it waits
    /// (reading 2); the day after, the limit is the entry day's high 106 and fills.
    #[test]
    fn entry_stop_and_previous_high_limit() {
        let mut v: Vec<Row> = (0..14).map(|i| { let p = 90. + i as f64; r(&format!("2001-01-{:02}", i + 1), p, p + 1., p - 1., p) }).collect();
        v.push(r("2001-01-15", 103., 104., 100., 102.)); // down close above the average: buy stop 104, long limit 104
        v.push(r("2001-01-16", 103., 106., 102.5, 105.)); // entry at 104; the 104 limit is not above the fill: held
        v.push(r("2001-01-17", 105., 107., 104.5, 106.)); // limit = 106 (entry day's high): exit at 106
        let x = Ctx::new(&v, &[], &[]);
        let t = super::run(super::ID, &x, 0, v.len() - 1, &Params::from([("order".to_string(), 0.)])).unwrap();
        assert_eq!(t.len(), 1, "{:?}", t.iter().map(|z| (z.legs[0].i, z.legs[0].px, z.exit_i, z.exit_px)).collect::<Vec<_>>());
        assert_eq!((t[0].legs[0].i, t[0].legs[0].px, t[0].exit_i, t[0].exit_px, t[0].dir), (15, 104., 16, 106., 1.));
        // the text option: an open at or above the stop places no entry (here the open 103 is below 104: same trade)
        let t2 = super::run(super::ID, &x, 0, v.len() - 1, &Params::from([("order".to_string(), 0.), ("entry".to_string(), 1.)])).unwrap();
        assert_eq!(t2.len(), 1);
    }

    #[test]
    fn spy_spx_minute_window_only() {
        for t in ["SPY", "SPX"] { assert!(crate::intraday_ep6::held_note(super::ID, t).is_none(), "{t}"); }
        for t in ["QQQ", "DIA", "NDX", "AAPL"] { assert!(crate::intraday_ep6::held_note(super::ID, t).is_some(), "{t}"); }
    }
}

pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book == "scpezz" && reviewed(def.id),
    label: "Pezzutti tested the system on S&P 500 futures (1996-2002). SPY and SPX are a declared application with prices of the traded series (no futures points), and the same-day order of the entry stop and the exit limit is settled on their real 1-minute bars; only the minute window is published (decision #243 (c)). Other instruments have no minute files and are refused.",
    basis: "Technical Analysis of Stocks & Commodities V.20:10 pp.50-54, Figure 2 (PDF p.3); R4 #297, #243; independent check authors/pe/w/pe_ref.py",
};
