//! Joel Rensink (Risnick), infiniteyield.com, "FirstStrike Plus - Weekly Volatility Breakout Instructions" (2008, 2 pp.).
//! Book id `jrfsp`. Source copy: Books/Joel Risnick/FirstStrike Plus.pdf (from R:/Trading Knowledge I/Joel Risnick -
//! First Strike Plus, First Strike Plus & One Night Stand (infiniteyield.com)/FirstStrike Plus.pdf).
//! Weeks = Monday-Sunday calendar weeks of the daily rows (the week's open = first trading day's open; the prior
//! week's range = its highest high - lowest low). Orders are worked on the daily bars of the week. Same-bar order
//! [book silent]: open -> nearer extreme -> other extreme -> close (high first on a tie), the vault's TradeStation bar
//! path; an order the open is already through fills at the open.
use crate::connors::{Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "jrfsp";

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
    ])
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

struct Pos { dir: f64, i: usize, px: f64, stop: f64, week: usize }

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    let defs = params(id)?;
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| defs.iter().find(|d| d.key == k).map(|d| d.def).unwrap_or(f64::NAN));
    let (em, sm) = (g("entry"), g("stop"));
    let to = to.min(x.c.len().saturating_sub(1));
    let (o, h, l, c) = (&x.o, &x.h, &x.l, &x.c);
    let mut out = Vec::new();
    let mut pos: Option<Pos> = None;
    let close = |q: Pos, i: usize, px: f64, why: &str, out: &mut Vec<Trade>| out.push(Trade {
        dir: q.dir, legs: vec![Leg { i: q.i, px: q.px, w: 1.0 }], exit_i: i, exit_px: px, reason: why.into(),
        entry_kind: "stop", signal_i: i, open: false, exits: vec![] });
    // week state
    let (mut wk, mut wn) = (i64::MIN, 0usize);
    let (mut ch, mut cl) = (f64::NAN, f64::NAN); // current week high-low
    let (mut buy, mut sell, mut wopen, mut rng) = (f64::NAN, f64::NAN, f64::NAN, f64::NAN);
    let (mut buy_live, mut sell_live) = (false, false);
    for t in 0..=to {
        let k = week_key(&x.r[t].date);
        if k != wk {
            wk = k; wn += 1;
            rng = ch - cl; ch = h[t]; cl = l[t];
            wopen = o[t];
            buy = wopen + em * rng; sell = wopen - em * rng;
            buy_live = t >= from && rng.is_finite(); sell_live = buy_live;
            // first profitable weekly open for a position carried from an earlier week
            if let Some(q) = pos.take() {
                if q.week < wn && q.dir * (o[t] - q.px) > 0.0 { close(q, t, o[t], "first profitable weekly open", &mut out); } else { pos = Some(q); }
            }
        } else { ch = ch.max(h[t]); cl = cl.min(l[t]); }
        if t < from { continue; }
        // bar path: open, nearer extreme, other extreme, close
        let hf = h[t] - o[t] <= o[t] - l[t];
        let pts = if hf { [o[t], h[t], l[t], c[t]] } else { [o[t], l[t], h[t], c[t]] };
        let mut px = o[t];
        let mut first = true;
        for &nxt in &pts {
            loop {
                // candidate levels in the direction of travel (at the open: anything the open is through)
                let up = nxt >= px;
                let mut best: Option<(f64, u8)> = None; // (level, kind) kind: 0 buy entry, 1 sell entry, 2 protective stop
                let mut cand = |lv: f64, kind: u8, rising: bool| {
                    if !lv.is_finite() { return; }
                    let hit = if first { if rising { o[t] >= lv } else { o[t] <= lv } } else if rising { up && lv >= px - 1e-12 && lv <= nxt } else { !up && lv <= px + 1e-12 && lv >= nxt };
                    if !hit { return; }
                    let better = match best { None => true, Some((b, _)) => if rising { lv < b } else { lv > b } };
                    if better { best = Some((lv, kind)); }
                };
                let dir = pos.as_ref().map_or(0.0, |q| q.dir);
                if buy_live && dir <= 0.0 { cand(buy, 0, true); }
                if sell_live && dir >= 0.0 { cand(sell, 1, false); }
                if let Some(q) = &pos { cand(q.stop, 2, q.dir < 0.0); }
                let Some((lv, kind)) = best else { break };
                let fill = if first { o[t] } else { lv };
                match kind {
                    2 => { let q = pos.take().unwrap(); close(q, t, fill, "protective stop", &mut out); }
                    _ => {
                        let d = if kind == 0 { 1.0 } else { -1.0 };
                        if kind == 0 { buy_live = false } else { sell_live = false }
                        if let Some(q) = pos.take() { close(q, t, fill, "reversed at this week's entry stop", &mut out); }
                        pos = Some(Pos { dir: d, i: t, px: fill, stop: wopen - d * sm * rng, week: wn });
                    }
                }
                px = fill;
            }
            if !first { px = nxt; }
            first = false;
        }
    }
    if let Some(q) = pos {
        out.push(Trade { dir: q.dir, legs: vec![Leg { i: q.i, px: q.px, w: 1.0 }], exit_i: to, exit_px: c[to], reason: "open".into(), entry_kind: "stop", signal_i: to, open: true, exits: vec![] });
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
