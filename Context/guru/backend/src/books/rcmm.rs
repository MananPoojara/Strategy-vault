//! Raymond Chong, "Market Millions: The Logical Trading System" v2.0 (market-millions.com).
//! Source copy: Books/Raymond Chong/Raymond Chong - Market Millions. The Logical Trading System.pdf (PDF page = printed page).
//! A Turtle variant: only what differs from `covel` (cov_turtle_s1/s2) is coded here: the 1/2 ATR entry-day stop,
//! the pyramid matrix built from the ORIGINAL entry price, Option 1 after a stopped pyramid unit, the 14-day ATR,
//! and the switch to the 10-day (20-day) range stop only once pyramiding is over. README and oracle: library/rcmm/.
use crate::connors::{true_range, Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "rcmm";

const NOTE: &str = "Futures/forex/stock rules run on the index (instrument swap). ATR = 14-day ATR of the signal day (the close before the entry day), frozen for the trade as in every matrix of the book (p.43, 49, 59, 70). Default = simple 14-day average of true range (the book's definition, p.33: \"This range is then averaged over a given period, usually 14 days\"); p.atr_wilder=1 = Wilder ATR(14), the StockCharts/NetDania/Barchart value the book tells you to read (p.34) [Book silent on the formula]. Breakout = 1 tick beyond the 20-day high/low of the 20 bars ending on the signal day (p.25: at the end of the day include today's bar), 1 tick = 0.05 (\"one point above\", p.49). Matrix (p.43-44): unit k price = X +/- (k-1) x 1/2 ATR from the ORDER price X (the crude example fills unit 1 at the 45.30 open but keeps X = 45.21, p.70); stop orders fill at the level, or at the open when the market opens through it. Daily-bar order [Book silent]: TradeStation's documented bar path - open, then the extreme nearer the open (tie: the low), the other extreme, the close (\"When the open is closer to the low than to the high, TradeStation assumes that the low was reached first\"); breakout, pyramid units and every stop (1/2 ATR entry stop for units filled that day, the shared stop otherwise) trigger in that order along the path, so a 1/2 ATR entry stop counts only when the market comes back to it after the fill; if both breakout levels lie on the path the first one reached is taken; no re-entry on an exit bar. Unit size = risk% x fill / (2 ATR) of equity (p.36-38: absolute risk / dollar value of 2 ATR), default 1% (the p.41-42 worked example). A pyramid unit stopped by its entry stop is not re-entered and no later unit is added (Option 1, p.53); the 10-day range stop takes over when it exceeds the 2 ATR stop once pyramiding is over (4th unit filled, or adds ended by Option 1; p.46, 72). Not coded: same-day intraday re-entry after an entry-stop exit (p.92/96 stop rule 1: needs the intraday path; the next-day re-entry is the normal breakout rule), Option 2 pyramid re-entry (p.53, 63), the optional 20-day validation filter with the 55-day failsafe (p.89-91), portfolio unit limits and strongest/weakest selection (p.92, 96: need several markets).";
const VALIDATE_GAP: &str = "Optional filter (p.89-91, p.96 entry rules 2-3): take a 20-day breakout only if the previous 20-day breakout \"would have resulted in a loss, that being that the trade was stopped out by hitting the 2 ATR stop\", otherwise use the 55-day breakout. The book does not say which earlier breakouts count while a previous (hypothetical) trade is still running, nor whether a 1/2 ATR entry-stop loss counts as the \"2 ATR stop\" loss; coding it would invent that rule.";
const PORTFOLIO_GAP: &str = "Needs several markets traded together: 4 units per market, 6 closely correlated, 10 weakly correlated, 12 per direction (p.92), and \"trade the strongest commodity within the group\" (p.96). The vault runs one index at a time.";
const SAR_FAQ_GAP: &str = "FAQ variation 1 (p.98): \"apply the 2ATR stop as a trailing stop off the highest high\". The book does not say whether it replaces the matrix stops from the entry day or only after the 4th unit, nor which ATR (frozen or current) trails. Variation 2 (3-day range stop) is p.exit = 3 on rcmm_20.";

pub fn catalogue() -> Vec<StrategyDef> {
    let d = |id, name, status, rules, notes| StrategyDef {
        id, name, group: "Market Millions · Turtle variant", session: 0, dir: "Long/Short", status,
        needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK, rules, notes, claim: None,
    };
    use Status::*;
    vec![
        d("rcmm_20", "Market Millions 20-day breakout (1/2 ATR entry stop, 4-unit matrix, 10-day stop)", Live, &[
            "E|\"Signals are generated anytime the current price 'breaks' the highest high or lowest low of that period\" (20 days) p.25; \"place a buy stop order at a point above the highest high, and a sell stop order at a point below the lowest low\" p.25, \"one point above the 20 day highest high\" p.49",
            "X|\"Entry stops are only in effect the day the trade goes on ... For long positions, entry stops are placed 1/2 ATR below the entry price\" (short: above) p.40",
            "X|\"Trailing stops are set at the close of the entry day, and are calculated at 2 ATR ... placed 2 ATR below the entry price\" p.40",
            "A|\"additional units are added at 1/2 ATR increments\" p.42; Table 2 p.43: unit k at X + (k-1)/2 ATR, entry stop one 1/2 ATR below, trailing stop X + (k-1)/2 ATR - 2 ATR, \"X represents the original entry price\"; 4 units",
            "X|\"we'll apply a trailing stop of $19.36 [$20.36 - 2 x 0.50] to unit number 1 AND 2. So both units will now have the same stop\" p.42-43",
            "X|\"As the trade goes our way and reaches the 4th unit, we will start using the 10 day stop when it exceeds the 4th unit 2ATR stop\" p.46; \"Each day we verify the lowest low for the past 10 days ... and see if we need to adjust our stop\" p.47",
            "F|Stopped pyramid unit: \"Option 1: is to simply trade the units you're already in, without entering more positions\" p.53",
            "F|Unit = \"Absolute dollar risk divided by the dollar value of a 2 ATR move\" p.38; ATR \"averaged over a given period, usually 14 days\" p.33",
        ], NOTE),
        d("rcmm_55", "Market Millions 55-day breakout (20-day range stop)", Live, &[
            "E|\"The only difference is that instead of using the 20 day breakout as an entry, you use a 55 day breakout to enter trades.\" p.91",
            "X|\"One important note is that the 55 day entry uses a 20 day range trailing stop as opposed to the 10 day trailing stop\" p.91; \"Use a 10 day range stop for a 20 bar breakout. And a 20 day stop for a 55 bar breakout.\" p.96",
            "X|Same 1/2 ATR entry-day stop, 2 ATR trailing stop and 4-unit 1/2 ATR matrix as the 20-day system (\"follows the same ideas presented so far\" p.91)",
        ], NOTE),
        d("rcmm_20_validated", "20-day breakout validated by a losing previous breakout (55-day failsafe)", NotTestable(VALIDATE_GAP), &[
            "F|\"if that previous breakout would have resulted in a loss, that being that the trade was stopped out by hitting the 2 ATR stop, then this 'validates' the current signal ... If it resulted in a winning trade, then the 20 day entry failed the validation test and should be ignored.\" p.89",
            "E|\"Always enter on a 55 bar breakout, particularly when the 20 bar breakout is not validated.\" p.96",
        ], VALIDATE_GAP),
        d("rcmm_portfolio", "Unit limits and strongest/weakest market selection", NeedsData(PORTFOLIO_GAP), &[
            "F|\"You should only hold up to 4 units in any market ... 6 units of any markets that are closely co-related ... 10 units of markets that have a weak co-relation ... a maximum of 12 units in any single direction.\" p.92",
        ], PORTFOLIO_GAP),
        d("rcmm_faq_trail", "FAQ: 2 ATR trailing stop off the highest high", NotTestable(SAR_FAQ_GAP), &[
            "X|\"You can apply the 2ATR stop as a trailing stop off the highest high (on a long) or off the lowest low (on a short).\" p.98",
        ], SAR_FAQ_GAP),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    Some(match id {
        "rcmm_20" => ("rcmm", "Market Millions", "20-day breakout"),
        "rcmm_55" => ("rcmm", "Market Millions", "55-day breakout"),
        "rcmm_20_validated" => ("rcmm", "Market Millions", "Validated 20-day · not testable"),
        "rcmm_portfolio" => ("rcmm", "Market Millions", "Portfolio limits · data missing"),
        "rcmm_faq_trail" => ("rcmm", "Market Millions", "FAQ trailing stop · not testable"),
        _ => return None,
    })
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    let (e, x) = match id { "rcmm_20" => (20.0, 10.0), "rcmm_55" => (55.0, 20.0), _ => return None };
    Some(vec![
        p("entry", "Entry breakout days", e, 10.0, 100.0, 5.0),
        p("exit", "Range-stop days (after pyramiding; FAQ variation: 3)", x, 3.0, 50.0, 1.0),
        p("atr_n", "ATR days", 14.0, 5.0, 40.0, 1.0),
        p("entry_stop", "Entry-day stop (ATR)", 0.5, 0.25, 2.0, 0.25),
        p("trail", "Trailing stop (ATR)", 2.0, 0.5, 4.0, 0.5),
        p("add", "Add a unit every (ATR)", 0.5, 0.25, 2.0, 0.25),
        p("units", "Maximum units", 4.0, 1.0, 4.0, 1.0),
        p("risk", "Unit size: % of equity risked per 2 ATR", 1.0, 0.5, 5.0, 0.5),
        p("tick", "One tick / point in index points (NSE 0.05)", 0.05, 0.0, 1.0, 0.05),
        p("atr_wilder", "ATR: 0 = simple average (p.33), 1 = Wilder ATR(14) of the charting services (p.34) [Not from the book's formula]", 0.0, 0.0, 1.0, 1.0),
    ])
}

pub fn validate(id: &str, p: &Params) -> Option<String> {
    let schema = params(id)?;
    for (k, &v) in p {
        let Some(d) = schema.iter().find(|d| d.key == k) else { return Some(format!("unknown parameter {k}")) };
        let int = d.def.fract() == 0.0 && d.step.fract() == 0.0;
        if !v.is_finite() || v < d.min || v > d.max || (int && v.fract() != 0.0) { return Some(format!("{k} must be in {}..{}", d.min, d.max)); }
    }
    None
}

fn hh(x: &Ctx, i: usize, n: usize) -> f64 { x.h[i - n..i].iter().copied().fold(f64::MIN, f64::max) }
fn ll(x: &Ctx, i: usize, n: usize) -> f64 { x.l[i - n..i].iter().copied().fold(f64::MAX, f64::min) }

/// Simple n-day average of true range (ending on bar i) or Wilder ATR.
fn atr_series(x: &Ctx, n: usize, wilder: bool) -> Vec<f64> {
    if wilder { return crate::connors::atr(x.r, n); }
    crate::connors::sma(&true_range(x.r), n)
}

/// TradeStation's daily-bar path: open, then the extreme nearer the open (tie: low), the other extreme, the close
/// ("When the open is closer to the low than to the high, TradeStation assumes that the low was reached first").
pub(crate) fn ts_path(x: &Ctx, i: usize) -> [f64; 4] {
    let (o, h, l, c) = (x.o[i], x.h[i], x.l[i], x.c[i]);
    if o - l <= h - o { [o, l, h, c] } else { [o, h, l, c] }
}

/// First stop-entry trigger reached along the path: (dir, fill, segment). Open through a trigger = open fill.
pub(crate) fn path_entry(pts: &[f64; 4], up: f64, dn: f64) -> Option<(f64, f64, usize)> {
    if pts[0] >= up { return Some((1.0, pts[0], 0)); }
    if pts[0] <= dn { return Some((-1.0, pts[0], 0)); }
    (0..3).find_map(|s| {
        let (a, b) = (pts[s], pts[s + 1]);
        if b > a && b >= up { Some((1.0, up, s)) } else if b < a && b <= dn { Some((-1.0, dn, s)) } else { None }
    })
}

struct Cfg { entry: usize, exit: usize, es: f64, trail: f64, add: f64, units: usize, risk: f64, tick: f64 }

struct Pos {
    dir: f64, a: f64, x0: f64,
    legs: Vec<Leg>,
    out: Vec<Option<(usize, f64, &'static str)>>,
    stop: f64, why: &'static str, // shared stop of units held at the open (set at a close) and its name
    adding: bool,                  // pyramiding still active
}

impl Pos {
    fn live(&self) -> usize { self.out.iter().filter(|o| o.is_none()).count() }
    /// Matrix price of unit k (0-based) from the original order price X (Table 2, p.43).
    fn price(&self, k: usize, add: f64) -> f64 { self.x0 + self.dir * k as f64 * add * self.a }
    fn fill(&mut self, c: &Cfg, i: usize, px: f64) {
        self.legs.push(Leg { i, px, w: c.risk / 100.0 * px / (2.0 * self.a) });
        self.out.push(None);
    }
    /// Open beyond the shared stop = every held unit out at the open; open beyond unit prices = those units at the open.
    fn open_gap(&mut self, c: &Cfg, i: usize, o: f64) {
        if self.dir * (o - self.stop) <= 0.0 {
            for x in self.out.iter_mut().filter(|x| x.is_none()) { *x = Some((i, o, self.why)); }
            return;
        }
        while self.adding && self.legs.len() < c.units && self.dir * (o - self.price(self.legs.len(), c.add)) >= 0.0 { self.fill(c, i, o); }
    }
    /// Walk the rest of bar i's path from `start` on segment s: units fill at their matrix price on favourable legs,
    /// stops (1/2 ATR entry stop for units filled today, the shared stop otherwise) fill at their level on adverse legs.
    fn walk(&mut self, c: &Cfg, i: usize, pts: &[f64; 4], s: usize, start: f64) {
        let (d, mut a) = (self.dir, start);
        for &b in &pts[s + 1..] {
            if self.live() == 0 { return; }
            if d * (b - a) > 0.0 {
                while self.adding && self.legs.len() < c.units {
                    let p = self.price(self.legs.len(), c.add);
                    if d * (b - p) < 0.0 { break; }
                    self.fill(c, i, if d * (p - a) >= 0.0 { p } else { a });
                }
            } else if d * (b - a) < 0.0 {
                for k in 0..self.legs.len() {
                    if self.out[k].is_some() { continue; }
                    let today = self.legs[k].i == i;
                    let lvl = if today { self.price(k, c.add) - d * c.es * self.a } else { self.stop };
                    if d * (b - lvl) <= 0.0 {
                        self.out[k] = Some((i, if d * (a - lvl) <= 0.0 { a } else { lvl }, if today { "1/2 ATR entry stop" } else { self.why }));
                        if today && k > 0 { self.adding = false; } // Option 1 (p.53): no re-entry, no further units
                    }
                }
            }
            a = b;
        }
    }
    /// At the close: all surviving units take the trailing stop of the highest surviving unit; once pyramiding is
    /// over, the n-day range stop replaces it when it is beyond (p.46-47).
    fn close_update(&mut self, x: &Ctx, c: &Cfg, i: usize) {
        if self.legs.len() == c.units { self.adding = false; }
        let top = (0..self.legs.len()).filter(|&k| self.out[k].is_none()).max().unwrap();
        (self.stop, self.why) = (self.price(top, c.add) - self.dir * c.trail * self.a, "2 ATR stop");
        if !self.adding && i + 1 >= c.exit {
            let r = if self.dir > 0.0 { ll(x, i + 1, c.exit) } else { hh(x, i + 1, c.exit) };
            if self.dir * (r - self.stop) > 0.0 { (self.stop, self.why) = (r, "range stop"); }
        }
    }
    fn trade(self, open: bool) -> Trade {
        let outs: Vec<(usize, f64, &str)> = self.out.into_iter().map(Option::unwrap).collect();
        let q: Vec<f64> = self.legs.iter().map(|l| l.w / l.px).collect();
        let qs: f64 = q.iter().sum();
        let exit_i = outs.iter().map(|o| o.0).max().unwrap();
        let pieces: Vec<(usize, f64, f64)> = outs.iter().zip(&q).map(|(o, q)| (o.0, o.1, q / qs)).collect();
        let exit_px = pieces.iter().map(|e| e.1 * e.2).sum();
        let exits = if outs.iter().all(|o| o.0 == exit_i) { vec![] } else { pieces };
        let mut why: Vec<&str> = outs.iter().map(|o| o.2).collect();
        why.dedup();
        Trade { dir: self.dir, legs: self.legs, exit_i, exit_px, reason: if open { "open".into() } else { why.join(" + ") },
            entry_kind: "stop", signal_i: exit_i, open, exits }
    }
}

fn mm(x: &Ctx, a: &[f64], c: &Cfg, from: usize, to: usize) -> Vec<Trade> {
    let mut out = vec![];
    let mut cur: Option<Pos> = None;
    for i in from.max(c.entry.max(c.exit))..=to {
        let pts = ts_path(x, i);
        if let Some(mut p) = cur.take() {
            p.open_gap(c, i, pts[0]);
            p.walk(c, i, &pts, 0, pts[0]);
            if p.live() == 0 { out.push(p.trade(false)); continue; } // no re-entry on an exit bar
            p.close_update(x, c, i);
            cur = Some(p);
            continue;
        }
        let av = a[i - 1];
        if !av.is_finite() || av <= 0.0 { continue; }
        let (up, dn) = (hh(x, i, c.entry) + c.tick, ll(x, i, c.entry) - c.tick);
        let Some((dir, fill, s)) = path_entry(&pts, up, dn) else { continue };
        let mut p = Pos { dir, a: av, x0: if dir > 0.0 { up } else { dn }, legs: vec![], out: vec![], stop: f64::NAN, why: "", adding: true };
        p.fill(c, i, fill);
        p.open_gap(c, i, pts[0]);
        p.walk(c, i, &pts, s, fill);
        if p.live() == 0 { out.push(p.trade(false)); continue; }
        p.close_update(x, c, i);
        cur = Some(p);
    }
    if let Some(mut p) = cur {
        for o in p.out.iter_mut().filter(|o| o.is_none()) { *o = Some((to, x.c[to], "open")); }
        out.push(p.trade(true));
    }
    out
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    let schema = params(id)?;
    if validate(id, p).is_some() || from > to || to >= x.r.len() { return None; }
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).unwrap().def);
    let u = |k: &str| g(k) as usize;
    let c = Cfg { entry: u("entry"), exit: u("exit"), es: g("entry_stop"), trail: g("trail"), add: g("add"), units: u("units"), risk: g("risk"), tick: g("tick") };
    let a = atr_series(x, u("atr_n"), g("atr_wilder") == 1.0);
    Some(mm(x, &a, &c, from, to))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;
    fn bar(o: f64, h: f64, l: f64, c: f64) -> Row { Row { ticker: "T".into(), date: String::new(), open: o, high: h, low: l, close: c, volume: None } }

    /// Book Table 1 (p.43): X = 20.11, ATR 0.50 -> units 20.11/20.36/20.61/20.86, trailing stop of 4 units = 19.86.
    #[test]
    fn matrix_matches_table_1_and_entry_stop_option_1() {
        // 30 flat bars of range 0.5 -> 14-day ATR 0.50; X = 20.10 + 0.01
        let mut r: Vec<Row> = (0..30).map(|_| bar(19.8, 20.10, 19.60, 19.8)).collect();
        r.push(bar(20.08, 20.20, 20.00, 20.15)); // unit 1 at 20.11
        r.push(bar(20.20, 20.40, 20.15, 20.38)); // unit 2 at 20.36 (entry stop 20.11 not hit)
        r.push(bar(20.40, 20.65, 20.38, 20.60)); // unit 3 at 20.61
        r.push(bar(20.62, 20.90, 20.62, 20.88)); // unit 4 at 20.86 -> all stops 19.86
        for _ in 0..5 { r.push(bar(21.0, 21.2, 20.9, 21.0)); }
        r.push(bar(21.0, 21.0, 19.5, 19.6)); // 10-day low (19.60) still below 19.86 -> 2 ATR stop at 19.86
        let x = Ctx::new(&r, &r, &[]);
        let mut pr = Params::new(); pr.insert("tick".into(), 0.01);
        let t = run("rcmm_20", &x, 0, r.len() - 1, &pr).unwrap();
        assert_eq!(t.len(), 1);
        let px: Vec<f64> = t[0].legs.iter().map(|l| (l.px * 100.0).round() / 100.0).collect();
        assert_eq!(px, vec![20.11, 20.36, 20.61, 20.86]);
        assert!((t[0].exit_px - 19.86).abs() < 1e-9, "{}", t[0].exit_px);
        assert_eq!(t[0].exit_i, r.len() - 1);
        // unit 3 entry stop (= unit 2 price 20.36) hit on its own entry day: unit 3 out, no unit 4 (Option 1)
        let mut r2 = r.clone(); r2[32] = bar(20.55, 20.65, 20.30, 20.50); // high first: unit 3 fills at 20.61, then its 20.36 entry stop
        let x = Ctx::new(&r2, &r2, &[]);
        let t = run("rcmm_20", &x, 0, r2.len() - 1, &pr).unwrap();
        assert_eq!(t[0].legs.len(), 3);
        assert!((t[0].exits[2].1 - 20.36).abs() < 1e-9);
        // units 1-2 then switch to the 10-day range stop (19.60 > 2 ATR stop 19.36) once adds ended
        assert!((t[0].exits[0].1 - 19.60).abs() < 1e-9 && t[0].reason.contains("range stop"), "{:?}", t[0].exits);
    }
}
