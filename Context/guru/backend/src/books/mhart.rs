//! Michael Harris articles. Active Trader, Sept 2001, "Keeping it simple" (3L-R and GAP-2H patterns with EasyLanguage).
//! Traders' magazine Jan/Mar 2005 files on the drive are 1-page excerpts with no rules. Source pages, oracle: library/mhart/.
use crate::connors::{claim, Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "mhart";

pub fn catalogue() -> Vec<StrategyDef> {
    let d = |id, name, group, dir, status, rules, notes, claim| StrategyDef { id, name, group, session: 0, dir, status, needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK, rules, notes, claim };
    vec![
        d("mha_3lr", "3L-R (three lower lows then reversal) · long · 8%/8% · next open", "Active Trader Sep 2001 · Keeping it simple", "Long", Status::Live,
            &["E|\"if l[1] < l[2] and l[2] < l[3] and h[0] > h[3] then buy (\\\"3L-R\\\") next bar on the open\" (Table 2) AT Sep 2001 p.46", "E|\"Buy on the open if: yesterday's low is lower than the low of two days ago; the low of two days ago is less than the low of three days ago; today's high is greater than the high of three days ago.\" p.43", "X|\"input: ptarget(8), stopl(8); profitprice = entryprice*(1+ptarget/100); stopprice = entryprice*(1-stopl/100); if marketposition =1 then begin exitlong at profitprice limit; exitlong at stopprice stop\" (Table 2) p.46"],
            "Book tested Intel (INTC) and Dell (DELL), 100 shares, 8% target/stop (Figs 2-3); the vault has no US single-stock data, so it runs on indices/ETFs or NSE stocks (instrument swap) and the INTC/DELL figures cannot be reproduced. The exit orders are issued only 'if marketposition = 1' at a bar's close, so they work from the bar AFTER the entry bar. Same-bar target+stop: stop first (book silent); opened through a level: fill at the open. The p.43-44 prose 'sample trading rule' uses 7%/7%; the code and the tests use 8% (default).", claim("58","74.14%","n/a","max 3 consecutive losers; 13 straight winners","INTC Jan 1990-Apr 2001, 8%/8%")),
        d("mha_gap2h", "GAP-2H (gap then two higher highs) · long · 7%/7% · next open", "Active Trader Sep 2001 · Keeping it simple", "Long", Status::Live,
            &["E|\"if h[0] > h[1] and h[2] > l[2] and l[0] > l[1] and l[1] > h[2] and h[1] > l[0] then buy (\\\"GPA-2H\\\") next bar on the open\" (Table 3) AT Sep 2001 p.46", "E|\"Buy on the open if: the high of two days ago is higher than the low of two days ago; yesterday's low is higher than the high of two days ago; today's low is greater than yesterday's low; yesterday's high is greater than today's low; today's high is greater than yesterday's high.\" p.46", "X|\"input: ptarget(7), stopl(7)\" limit/stop exits from entry price, issued while marketposition = 1 (Table 3) p.46"],
            "Book tested INTC Jan 1990-Apr 2001 at 7%/7% (\"around 67 percent\", max 2 consecutive losers, Fig 6) and AMZN at 5%/5% (13 trades, Fig 7). No US single-stock data in the vault: runs on indices/ETFs or NSE stocks (instrument swap). Exits work from the bar after entry (code issues them when marketposition = 1). Stop first if both in one bar; open-through fills at the open.", claim("n/a","around 67%","n/a","max 2 consecutive losers","INTC Jan 1990-Apr 2001, 7%/7%")),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    Some(match id {
        "mha_3lr" => ("mha_patterns", "Active Trader 2001 patterns", "3L-R"),
        "mha_gap2h" => ("mha_patterns", "Active Trader 2001 patterns", "GAP-2H"),
        _ => return None,
    })
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    Some(match id {
        "mha_3lr" => vec![p("target","Profit target (% of entry)",8.0,1.0,20.0,0.5), p("stop","Stop-loss (% of entry)",8.0,1.0,20.0,0.5)],
        "mha_gap2h" => vec![p("target","Profit target (% of entry)",7.0,1.0,20.0,0.5), p("stop","Stop-loss (% of entry)",7.0,1.0,20.0,0.5)],
        _ => return None,
    })
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    let schema = params(id)?;
    if !checked(p, &schema) || from > to || to >= x.r.len() { return None; }
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).map_or(0., |d| d.def));
    let logic = match id {
        "mha_3lr" => "L3>L2&L2>L1&H0>H3",
        "mha_gap2h" => "H0>H1&H2>L2&L0>L1&L1>H2&H1>L0",
        _ => return None,
    };
    // exits issued only "if marketposition = 1" at a bar's close -> working from the bar after the entry bar
    Some(simulate(x, from, to, logic, 1., true, false, level_fn(1., g("target"), g("stop"), true)))
}

// ---- pattern engine (one position per pattern; same code in mhstp/mhpst/mhart so each module stands alone) ----
type Conj = Vec<(u8, usize, u8, usize)>;
/// "L1>H2&H1>L0|C0>C3" = OR of AND-groups of strict inequalities between bar fields (0 = today).
fn parse(logic: &str) -> Vec<Conj> {
    logic.split('|').map(|c| c.split('&').map(|t| {
        let (a, b) = t.split_once('>').expect("pattern term");
        let f = |s: &str| (s.as_bytes()[0], s[1..].parse::<usize>().expect("bar index"));
        let ((x, i), (y, j)) = (f(a.trim()), f(b.trim()));
        (x, i, y, j)
    }).collect()).collect()
}
fn px(x: &Ctx, f: u8, i: usize) -> f64 { match f { b'O' => x.o[i], b'H' => x.h[i], b'L' => x.l[i], _ => x.c[i] } }
fn signal(x: &Ctx, dnf: &[Conj], i: usize) -> bool {
    dnf.iter().any(|c| c.iter().all(|&(a, ai, b, bi)| ai <= i && bi <= i && px(x, a, i - ai) > px(x, b, i - bi)))
}
/// Fixed target/stop around the fill. Same bar: a gap through a level fills at the open; if the bar's range
/// holds both levels the stop is assumed first (book silent; conservative).
fn exit_on(x: &Ctx, i: usize, dir: f64, tp: f64, sp: f64) -> Option<(f64, &'static str)> {
    let (o, h, l) = (x.o[i], x.h[i], x.l[i]);
    if dir > 0. {
        if o <= sp { Some((o, "stop (opened through)")) } else if o >= tp { Some((o, "target (opened through)")) }
        else if l <= sp { Some((sp, "stop")) } else if h >= tp { Some((tp, "target")) } else { None }
    } else if o >= sp { Some((o, "stop (opened through)")) } else if o <= tp { Some((o, "target (opened through)")) }
    else if h >= sp { Some((sp, "stop")) } else if l <= tp { Some((tp, "target")) } else { None }
}
/// at_open: signal on bar i's close, fill at bar i+1's open; else fill at bar i's close.
/// entry_bar_exits: target/stop already working on the open-entry bar (otherwise from the next bar).
fn simulate(x: &Ctx, from: usize, to: usize, logic: &str, dir: f64, at_open: bool, entry_bar_exits: bool,
            levels: impl Fn(f64) -> (f64, f64)) -> Vec<Trade> {
    let dnf = parse(logic);
    let mut out = vec![];
    let mut cur: Option<(Trade, f64, f64, usize)> = None;
    for i in from..=to {
        if let Some((t, tp, sp, active)) = cur.as_mut() {
            if i >= *active {
                if let Some((p, why)) = exit_on(x, i, dir, *tp, *sp) {
                    t.exit_i = i; t.exit_px = p; t.signal_i = i; t.reason = why.into();
                    out.push(cur.take().unwrap().0);
                }
            }
        }
        if cur.is_some() || !signal(x, &dnf, i) { continue; }
        let (k, fill) = if at_open { if i >= to { continue; } (i + 1, x.o[i + 1]) } else { (i, x.c[i]) };
        if !fill.is_finite() { continue; }
        let (tp, sp) = levels(fill);
        let active = if at_open && entry_bar_exits { k } else { k + 1 };
        let t = Trade { dir, legs: vec![Leg { i: k, px: fill, w: 1. }], exit_i: k, exit_px: fill, reason: String::new(),
            entry_kind: if at_open { "open" } else { "close" }, signal_i: i, open: false, exits: vec![] };
        cur = Some((t, tp, sp, active));
    }
    if let Some((mut t, ..)) = cur { t.exit_i = to; t.exit_px = x.c[to]; t.signal_i = to; t.reason = "open".into(); t.open = true; out.push(t); }
    out
}
fn level_fn(dir: f64, target: f64, stop: f64, pct: bool) -> impl Fn(f64) -> (f64, f64) {
    move |e| if pct { (e * (1. + dir * target / 100.), e * (1. - dir * stop / 100.)) } else { (e + dir * target, e - dir * stop) }
}
fn checked(p: &Params, schema: &[ParamDef]) -> bool {
    p.iter().all(|(k, v)| schema.iter().any(|d| d.key == k && v.is_finite() && *v >= d.min && *v <= d.max))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;
    fn rows(bars: &[(f64, f64, f64, f64)]) -> Vec<Row> {
        bars.iter().enumerate().map(|(i, &(o, h, l, c))| Row { ticker: "T".into(), date: format!("{:04}-01-01", 2000 + i), open: o, high: h, low: l, close: c, volume: None }).collect()
    }
    #[test]
    fn parser_and_signal_follow_bar_labels() {
        // bar 0 = today; "C0>C1" true when today's close is above yesterday's
        let r = rows(&[(1., 2., 0.5, 1.), (1., 3., 0.5, 2.)]);
        let x = Ctx::new(&r, &r, &[]);
        assert!(signal(&x, &parse("C0>C1"), 1));
        assert!(!signal(&x, &parse("C1>C0"), 1));
        assert!(!signal(&x, &parse("C0>C2"), 1)); // not enough history
        assert!(signal(&x, &parse("C1>C0|H0>H1"), 1)); // OR group
    }
    #[test]
    fn open_entry_target_stop_and_gap_fills() {
        // signal at bar 1 (C0>C1), fill bar 2 open 100; target 105 / stop 95 (5 points)
        let r = rows(&[(10., 11., 9., 10.), (10., 12., 9., 11.), (100., 104., 96., 100.), (100., 106., 99., 101.)]);
        let x = Ctx::new(&r, &r, &[]);
        let t = simulate(&x, 0, 3, "C0>C1", 1., true, true, level_fn(1., 5., 5., false));
        assert_eq!(t[0].legs[0].i, 2); assert_eq!(t[0].legs[0].px, 100.);
        assert_eq!(t[0].exit_i, 3); assert_eq!(t[0].exit_px, 105.);
        // short, opened through the stop
        let r = rows(&[(10., 11., 9., 10.), (10., 12., 8., 9.), (100., 101., 99., 100.), (110., 112., 108., 111.)]);
        let x = Ctx::new(&r, &r, &[]);
        let t = simulate(&x, 0, 3, "C1>C0", -1., true, true, level_fn(-1., 5., 5., false));
        assert_eq!(t[0].exit_i, 3); assert_eq!(t[0].exit_px, 110.); assert_eq!(t[0].reason, "stop (opened through)");
    }
    #[test]
    fn both_levels_in_one_bar_assume_stop_and_close_entry_waits_a_bar() {
        let r = rows(&[(10., 11., 9., 10.), (10., 12., 9., 100.), (100., 120., 80., 100.)]);
        let x = Ctx::new(&r, &r, &[]);
        let t = simulate(&x, 0, 2, "C0>C1", 1., false, false, level_fn(1., 10., 10., true));
        assert_eq!(t[0].legs[0].px, 100.); assert_eq!(t[0].exit_i, 2); assert_eq!(t[0].exit_px, 90.);
        // entry_bar_exits=false: the open-entry bar's range is ignored, exit on the next bar
        let r = rows(&[(10., 11., 9., 10.), (10., 12., 9., 11.), (100., 120., 80., 100.), (100., 101., 99., 100.)]);
        let x = Ctx::new(&r, &r, &[]);
        let t = simulate(&x, 0, 3, "C0>C1", 1., true, false, level_fn(1., 10., 10., true));
        assert!(t[0].open);
    }
    #[test]
    fn catalogue_ids_unique_and_params_default_to_book() {
        let cat = catalogue();
        let mut ids: Vec<_> = cat.iter().map(|d| d.id).collect(); ids.sort(); ids.dedup();
        assert_eq!(ids.len(), cat.len());
        for d in cat.iter().filter(|d| d.status == Status::Live) {
            assert!(params(d.id).is_some() && family_of(d.id).is_some());
            let r = rows(&[(1., 2., 0.5, 1.); 12]); let x = Ctx::new(&r, &r, &[]);
            assert!(run(d.id, &x, 0, 11, &Params::new()).is_some());
        }
    }
}
