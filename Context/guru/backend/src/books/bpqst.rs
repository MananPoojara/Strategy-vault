//! Bill & Greg Poulos, "Quantum Swing Trader" (Profits Run, Inc., 2006; 123 pp.). Book id `bpqst`.
//! Source copy: Books/Bill Poulos/Quantum Swing Trader.pdf (from R:/USB HDD 2/tRADING KNO-2/Bill & Greg Poulos -
//! Quantum Swing Trader (quantumswingtrader.com)/Quantum Swing Trader.pdf), scanned; trading rules pp.71-97, search
//! criteria pp.98-103 read in full.
//! Setups = the book's own Search Criteria segments (pp.100-103), which the book says find "a stock that meets the
//! setup conditions described previously ... for the next day's order entry" (p.98): "going up" = SMA today > SMA
//! yesterday. Slow stochastics 8,3,3 %K = 3-bar simple average of the 8-bar fast %K. Same-bar order [book silent]:
//! open -> nearer extreme -> other extreme -> close (high first on a tie); an order the open is through fills at the open.
use crate::connors::{sma, Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "bpqst";
/// Admitted ids (authors/lanea/newsletters/decisions.json).
pub const REVIEWED_LANEA: &[&str] = &["bpqst_uptrend", "bpqst_downtrend"];
pub fn reviewed_lanea(id: &str) -> bool { REVIEWED_LANEA.contains(&id) }

const C0: &str = "F|[Book silent] One position at a time; setups are not taken while a position is open. Same-day order of fills: open -> nearer extreme -> other extreme -> close; gaps fill at the open. The breakeven stop and both trailing stops work from the moment the target fills; trailing levels use the bars up to the previous close. Each third's stop is the higher (long) / lower (short) of the breakeven stop and its trailing stop (both orders work on the same shares).";
const C1: &str = "F|Stocks of $10-$200 with 50-day average volume >= 1,000,000 shares (Master List, p.99) - run on the index (instrument swap); the price/volume screen does not apply to an index.";

pub fn catalogue() -> Vec<StrategyDef> {
    let mk = |id, name, dir, rules| StrategyDef {
        id, name, group: "Quantum Swing Trader", session: 0, dir, status: Status::Live, needs_volume: false, needs_vix: false,
        needs_series: &[], book: BOOK, rules,
        notes: "Book: US stocks (eSignal/Advanced GET charts 2004-2006), no back-test statistics printed, so no book-vs-ours comparison. Run on the index (instrument swap). Book prose setups (p.72-73, 85-86) add \"the market trades above the moving average ... and then trades down to\" it and \"no subsequent daily bar high trades below the moving average\"; the book's own search segments (p.100-103) define the setup and do not contain them (no lookback is given for either), and the order is only for the following day, so 'subsequent' never binds before the fill. 'Ignore inside days' (p.75, Fig 60): an inside day is not a setup day and the pending order (setup day's high + 1/2%) stays working the next day.",
        claim: None,
    };
    vec![
        mk("bpqst_uptrend", "Quantum Swing Trader - uptrend rules (long, reversal short)", "Long/Short", UP),
        mk("bpqst_downtrend", "Quantum Swing Trader - downtrend rules (short, reversal long)", "Short/Long", DN),
    ]
}

const UP: &[&str] = &[
            "F|Setup (any one segment, at today's close): Segment 1/2/3: \"The Simple 200 [50 / 20] day Moving Average of today > The Simple 200 [50 / 20] day Moving Average of yesterday AND The High of today > [that average] AND The Low of today < [that average] AND The Slow Stochastics (8,3,3) %K <= 40%\" (p.100-101). Segment 4: \"SMA20 today > SMA20 yesterday AND SMA50 today > SMA50 yesterday AND The Low of today > SMA50 AND The High of today < SMA20 AND %K <= 40%\" (p.101)",
            "F|\"Ignore inside days. An inside day is one whose High is lower than the previous day High and whose Low is higher than the previous day's Low.\" (p.75, Fig 60: \"Buy stop here / Not here\")",
            "E|\"place an order for the following day ... Buy at the setup day's High + 1/2% of the High Stop\" (p.74)",
            "X|\"Sell at the setup day Low - 1/2% of the setup day Low Stop\" (initial stop, p.76)",
            "X|\"Reverse and go short at the initial stop point only if the stop point is within 10 days of the current day\" (p.77): \"Sell short at the setup day Low - 1/2% of the setup day Low Stop\" (p.78); read: the stop fills no more than 10 trading days after the setup day. Reversal stop: \"Buy to cover at the High of the day before Reversal Day + 1/2% of the High of the day before Reversal Day Stop\" (p.78), then \"the same profit target exits as described in the Trading Rules - Downtrend section\" (p.78). A reversed trade is not reversed again; \"Do not reverse at the breakeven point\" (p.81).",
            "X|\"Exit 1/3 of the position at a 3% profit ... Sell at Entry Price x 1.03 Limit\" and \"change the initial stop to breakeven for the remaining position ... Sell at Entry Price Stop\" (p.81)",
            "X|\"Trailing Stop 1 ... Exit 1/3 of the position at the Lowest Low of the last two days (LL2) ... Sell at LL2 - 1/2% of the LL2 Stop\" (p.82)",
            "X|\"Trailing Stop 2 ... Exit 1/3 of the position at the last Swing Low. A Swing Low is a daily bar with at least two bars before and after whose Highs are higher than the Swing Low Bar High and whose Lows are higher than the Swing Low Bar Low ... Sell at Swing Low - 1/2% of the Swing Low Stop\" (p.83)",
    C0, C1,
];
const DN: &[&str] = &[
            "F|Setup (any one segment, at today's close): Segment 1/2/3: \"The Simple 200 [50 / 20] day Moving Average of today < [that of] yesterday AND The High of today > [that average] AND The Low of today < [that average] AND The Slow Stochastics (8,3,3) %K >= 60%\" (p.102-103). Segment 4: \"SMA20 today < SMA20 yesterday AND SMA50 today < SMA50 yesterday AND The Low of today > SMA20 AND The High of today < SMA50 AND %K >= 60%\" (p.103). (Fig 67/68 captions say \">= 70%\"; the rule tables say 60%.)",
            "F|\"Ignore Inside Days - An Inside Day is one whose High is lower than the previous day High and whose Low is higher than the previous day Low.\" (p.88)",
            "E|\"Sell Short at the setup day's Low - 1/2% of the Low Stop\" (order for the following day, p.87)",
            "X|Initial stop: \"Buy to cover at the setup day High + 1/2% of the setup day High Stop\" (mirror of p.76; p.89-90)",
            "X|Reversal (p.90-91): within 10 days, \"Buy at the setup day High + 1/2% of the setup day High Stop\"; reversal stop \"Sell at the Low of the day before Reversal Day - 1/2% of the Low of the day before Reversal Day Stop\", then the uptrend profit-target exits (p.91).",
            "X|Exits (mirror, p.92-96): 1/3 at Entry Price x 0.97 limit, then the stop to breakeven (entry price); 1/3 at the Highest High of the last two days + 1/2%; 1/3 at the last Swing High + 1/2% (Swing High = a bar with at least two bars before and after whose Highs and Lows are lower).",
    C0, C1,
];

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    match id {
        "bpqst_uptrend" => Some(("bpqst", "Quantum Swing Trader (Poulos)", "uptrend (long)")),
        "bpqst_downtrend" => Some(("bpqst", "Quantum Swing Trader (Poulos)", "downtrend (short)")),
        _ => None,
    }
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    let k = match id { "bpqst_uptrend" => 40., "bpqst_downtrend" => 60., _ => return None };
    Some(vec![
        p("ma_s", "Short-term SMA", 20., 5., 50., 1.), p("ma_m", "Intermediate SMA", 50., 20., 100., 1.), p("ma_l", "Long-term SMA", 200., 100., 300., 10.),
        p("k_len", "Stochastic %K length", 8., 3., 21., 1.), p("k_slow", "Stochastic %K slowing", 3., 1., 5., 1.),
        p("k_lvl", if k < 50. { "Setup when %K <=" } else { "Setup when %K >=" }, k, 0., 100., 5.),
        p("buf", "Order buffer, % of the bar's price", 0.5, 0., 2., 0.1), p("target", "Profit target %", 3., 1., 10., 0.5),
        p("rev_days", "Reverse only within N days of the setup day", 10., 0., 30., 1.),
    ])
}

struct Pos { dir: f64, i: usize, px: f64, setup: usize, stop: f64, reversed: bool, hit: bool, left: [bool; 3], exits: Vec<(usize, f64, f64)> }

fn finish(q: Pos, why: &str) -> Trade {
    let last = q.exits.iter().map(|e| e.0).max().unwrap_or(q.i);
    let px = q.exits.iter().map(|e| e.1 * e.2).sum::<f64>();
    let single = q.exits.len() == 1;
    Trade { dir: q.dir, legs: vec![Leg { i: q.i, px: q.px, w: 1.0 }], exit_i: last, exit_px: px, reason: why.into(), entry_kind: "stop",
        signal_i: last, open: false, exits: if single { vec![] } else { q.exits } }
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    let defs = params(id)?;
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| defs.iter().find(|d| d.key == k).map(|d| d.def).unwrap_or(f64::NAN));
    let side = if id == "bpqst_uptrend" { 1.0 } else { -1.0 };
    let to = to.min(x.c.len().saturating_sub(1));
    let (o, h, l, c) = (&x.o, &x.h, &x.l, &x.c);
    let n = c.len();
    let (s, m, lg) = (sma(c, g("ma_s") as usize), sma(c, g("ma_m") as usize), sma(c, g("ma_l") as usize));
    let kl = g("k_len").max(1.) as usize;
    let fast: Vec<f64> = (0..n).map(|t| {
        if t + 1 < kl { return f64::NAN }
        let hh = h[t + 1 - kl..=t].iter().cloned().fold(f64::MIN, f64::max);
        let ll = l[t + 1 - kl..=t].iter().cloned().fold(f64::MAX, f64::min);
        if hh > ll { 100. * (c[t] - ll) / (hh - ll) } else { f64::NAN }
    }).collect();
    let k = sma(&fast, g("k_slow").max(1.) as usize);
    let (klvl, buf, tgt, rev) = (g("k_lvl"), g("buf") / 100., g("target") / 100., g("rev_days"));
    let inside = |t: usize| t > 0 && h[t] < h[t - 1] && l[t] > l[t - 1];
    let setup = |t: usize| -> bool {
        if t == 0 || inside(t) { return false }
        let kk = if side > 0. { k[t] <= klvl } else { k[t] >= klvl };
        let dir = |v: &Vec<f64>| side * (v[t] - v[t - 1]) > 0.;
        let straddle = |v: &Vec<f64>| h[t] > v[t] && l[t] < v[t];
        let seg123 = [&lg, &m, &s].iter().any(|v| dir(v) && straddle(v));
        let seg4 = dir(&s) && dir(&m) && if side > 0. { l[t] > m[t] && h[t] < s[t] } else { l[t] > s[t] && h[t] < m[t] };
        kk && (seg123 || seg4)
    };
    // last confirmed swing low / high known at the close of bar t (pivot j <= t - 2)
    let piv = |lowside: bool| -> Vec<f64> {
        let mut v = vec![f64::NAN; n];
        let mut cur = f64::NAN;
        for t in 0..n {
            if t >= 4 {
                let j = t - 2;
                let ok = [j - 2, j - 1, j + 1, j + 2].iter().all(|&q| if lowside { h[q] > h[j] && l[q] > l[j] } else { h[q] < h[j] && l[q] < l[j] });
                if ok { cur = if lowside { l[j] } else { h[j] } }
            }
            v[t] = cur;
        }
        v
    };
    let (swl, swh) = (piv(true), piv(false));
    let mut out = Vec::new();
    let mut pos: Option<Pos> = None;
    let mut pend: Option<usize> = None; // setup bar of the working entry order
    for t in from.max(2)..=to {
        // working levels for this bar: (level, rising?, kind) kind 0 entry, 1 initial/reversal stop, 2 target, 3/4 third stops
        let hf = h[t] - o[t] <= o[t] - l[t];
        let pts = if hf { [h[t], l[t], c[t]] } else { [l[t], h[t], c[t]] };
        let mut px = o[t];
        let mut first = true;
        let mut seg = 0usize;
        loop {
            let nxt = if first { o[t] } else { pts[seg] };
            let up = nxt >= px;
            let mut best: Option<(f64, u8)> = None;
            {
                let mut cand = |lv: f64, rising: bool, kind: u8| {
                    if !lv.is_finite() { return }
                    let hit = if first { if rising { o[t] >= lv } else { o[t] <= lv } }
                        else if rising { up && lv >= px && lv <= nxt } else { !up && lv <= px && lv >= nxt };
                    if !hit { return }
                    let better = match best { None => true, Some((b, _)) => if rising { lv < b } else { lv > b } };
                    if better { best = Some((lv, kind)) }
                };
                match &pos {
                    None => if let Some(su) = pend {
                        let lv = if side > 0. { h[su] * (1. + buf) } else { l[su] * (1. - buf) };
                        cand(lv, side > 0., 0);
                    },
                    Some(q) => {
                        let d = q.dir;
                        if !q.hit {
                            cand(q.stop, d < 0., 1);
                            cand(q.px * (1. + d * tgt), d > 0., 2);
                        } else {
                            let (tr1, tr2) = if d > 0. { (l[t - 1].min(l[t - 2]) * (1. - buf), swl[t - 1] * (1. - buf)) } else { (h[t - 1].max(h[t - 2]) * (1. + buf), swh[t - 1] * (1. + buf)) };
                            let eff = |tr: f64| if !tr.is_finite() { q.px } else if d > 0. { tr.max(q.px) } else { tr.min(q.px) };
                            if q.left[1] { cand(eff(tr1), d < 0., 3) }
                            if q.left[2] { cand(eff(tr2), d < 0., 4) }
                        }
                    }
                }
            }
            if let Some((lv, kind)) = best {
                let fill = if first { o[t] } else { lv };
                match kind {
                    0 => {
                        let su = pend.take().unwrap();
                        let st = if side > 0. { l[su] * (1. - buf) } else { h[su] * (1. + buf) };
                        pos = Some(Pos { dir: side, i: t, px: fill, setup: su, stop: st, reversed: false, hit: false, left: [true; 3], exits: vec![] });
                    }
                    1 => {
                        let mut q = pos.take().unwrap();
                        q.exits.push((t, fill, 1.0));
                        let can_rev = !q.reversed && (t - q.setup) as f64 <= rev;
                        let d = q.dir;
                        out.push(finish(q, if can_rev { "initial stop - reversed" } else { "stop" }));
                        if can_rev {
                            let st = if d > 0. { h[t - 1] * (1. + buf) } else { l[t - 1] * (1. - buf) };
                            pos = Some(Pos { dir: -d, i: t, px: fill, setup: t, stop: st, reversed: true, hit: false, left: [true; 3], exits: vec![] });
                        }
                    }
                    2 => { let q = pos.as_mut().unwrap(); q.exits.push((t, fill, 1. / 3.)); q.left[0] = false; q.hit = true; }
                    _ => {
                        let q = pos.as_mut().unwrap();
                        let which = (kind - 2) as usize; // a level shared by both thirds fills twice, back to back
                        q.exits.push((t, fill, 1. / 3.)); q.left[which] = false;
                        if !q.left[1] && !q.left[2] {
                            let q = pos.take().unwrap();
                            out.push(finish(q, if kind == 3 { "trailing stop 1 (last third)" } else { "trailing stop 2 (last third)" }));
                        }
                    }
                }
                px = fill;
                continue;
            }
            if first { first = false; continue; }
            px = nxt;
            seg += 1;
            if seg == 3 { break }
        }
        // orders for tomorrow, set at today's close
        if pos.is_some() { pend = None; }
        else if inside(t) { /* ignore inside days: a working order stays */ }
        else if setup(t) { pend = Some(t); }
        else { pend = None; }
    }
    if let Some(mut q) = pos {
        let left = q.left.iter().filter(|&&b| b).count() as f64 / 3.;
        q.exits.push((to, c[to], left));
        let mut tr = finish(q, "open");
        tr.exit_i = to; tr.signal_i = to; tr.open = true;
        out.push(tr);
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;
    #[test]
    fn target_then_breakeven() {
        // flat bars, then a setup is forced via a tiny average window; check the order mechanics on hand-made bars
        let mut r: Vec<Row> = (0..30).map(|i| { let v = 100. + i as f64 * 0.1; Row { ticker: "T".into(), date: format!("2000-01-{:02}", i % 28 + 1), open: v, high: v + 1., low: v - 1., close: v - 1., volume: None } }).collect();
        // bar 30: straddles the rising 3-bar average with %K low (close near the low)
        r.push(Row { ticker: "T".into(), date: "2000-02-01".into(), open: 102.9, high: 103.9, low: 101.0, close: 102.75, volume: None });
        // bar 31: buy stop at 103.9 x 1.005 = 104.4195; runs to 108 (target 107.552): 1/3 exits, stop to breakeven
        r.push(Row { ticker: "T".into(), date: "2000-02-02".into(), open: 103.0, high: 108.0, low: 102.9, close: 107.0, volume: None });
        // bar 32: falls through breakeven -> remaining 2/3 exit at the entry price
        r.push(Row { ticker: "T".into(), date: "2000-02-03".into(), open: 106.0, high: 106.0, low: 100.0, close: 101.0, volume: None });
        let x = Ctx::new(&r, &r, &[]);
        let mut p = Params::new();
        for (k, v) in [("ma_s", 3.), ("ma_m", 3.), ("ma_l", 3.)] { p.insert(k.into(), v); }
        let t = run("bpqst_uptrend", &x, 0, r.len() - 1, &p).unwrap();
        let q = &t[0];
        assert_eq!(q.legs[0].i, 31);
        assert!((q.legs[0].px - 104.4195).abs() < 1e-9);
        assert_eq!(q.exits.len(), 3);
        assert!((q.exits[0].1 - 104.4195 * 1.03).abs() < 1e-9);
        assert!((q.exits[1].1 - 104.4195).abs() < 1e-9 && q.exits[1].0 == 32);
    }
}
