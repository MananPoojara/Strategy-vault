//! Jack D. Schwager, "A Complete Guide to the Futures Market" (2nd ed., Wiley 2017), Ch.17 "Examples of Original
//! Trading Systems" (printed p.261-277; PDF = printed + 4) with the run-day / wide-ranging-day definitions of Ch.9 (p.118-119).
//! Source copy: Books/Jack D. Schwager/Jack Schwager- Complete Guide to Futures Market-.pdf. README and oracle: library/jscgf/.
use crate::connors::{true_range, Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "jscgf";

const NOTE: &str = "Futures systems run on the daily index (instrument swap; the book's examples are copper, WTI crude and soybean continuous futures, which the vault does not have). Stop-and-reverse, always in after the first signal. Signals are taken at the signal day's close (the daily checklist acts on \"today's close\"; the testing chapter assumes \"order entry on the close\", p.294); p.fill = 1 fills at the next open [Not from the book]. True high = max(high, previous close), true low = min(low, previous close) (p.261 note 1). Each run starts flat at the selected start date; earlier bars only warm the definitions. Full position, one unit.";
const GENERIC_GAP: &str = "Ch.16 (p.235-260) describes generic moving-average and breakout systems and their modifications (confirmation, filters, pyramiding, stops) as illustrations, with no single complete rule set; the plain MA-crossover and N-day breakout versions are already in the vault many times (e.g. cov_tc_100day_reversal, cov_wott_dual_ma).";

pub fn catalogue() -> Vec<StrategyDef> {
    let d = |id, name, status, rules, notes| StrategyDef {
        id, name, group: "Ch 17 · Examples of Original Trading Systems", session: 0, dir: "Long/Short", status,
        needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK, rules, notes, claim: None,
    };
    use Status::*;
    vec![
        d("jscgf_wrd", "Wide-ranging-day system (k 2.0, N 10, N1 0, N2 0)", Live, &[
            "F|\"Wide-ranging day. A day on which the volatility ratio (VR) is greater than k (e.g., k = 2). The VR is equal to today's true range divided by the average true range of the past N-day period (e.g., N = 10).\" p.262 (past N days = the N days before today [Book ambiguous])",
            "F|\"Price trigger range (PTR). The range defined by the highest true high and lowest true low in the interval between N1 days before the most recent wide-ranging day to N2 days after.\" p.262",
            "E|\"1. If short and today's close is above the high of the PTR, liquidate short and go long. 2. If long and today's close is below the low of the PTR, liquidate long and go short. 3. Check whether exactly N2 days have elapsed since the most recent wide-ranging day. If this condition is met, redefine the PTR.\" p.262 (steps in this order)",
            "X|Exit = the opposite signal: \"Buy case. On a close above the high of the PTR, reverse from short to long. Sell case. On a close below the low of the PTR, reverse from long to short.\" p.262",
            "F|Illustrated parameter set N1 = 0, N2 = 0 (Figures 17.1-17.5, p.264); Table 17.1 lists k 1.6/2.0/2.4 x N1, N2 in 0/2/4 p.263",
        ], NOTE),
        d("jscgf_runday_bo", "Run-day breakout system (N1 5, N2 4)", Live, &[
            "F|\"Up run day. 1. The true high of the run day is greater than the maximum true high of the past N days ... 2. The true low on the run day is less than the minimum true low on the subsequent N days.\" (down run day mirrored) p.118; \"Days that fulfill both the up and down run day definitions are not considered run days.\" p.269 note 2",
            "E|\"Buy case. Reverse to long whenever both ... 1. The close is above the maximum true high of the most recent N2 down run days ... 2. The most recent run day is an up run day.\" (sell case mirrored) p.269",
            "X|Exit = the opposite signal (\"Reverse to short whenever ...\", sell case mirrored) p.269",
            "F|\"Check whether the trading day N1 days prior to the current day can be defined as an up or a down run day\" each day before the signal checks p.269; illustrated N1 = 5, N2 = 4 (Figures 17.6-17.9, p.270)",
        ], NOTE),
        d("jscgf_runday_cc", "Run-day consecutive count system (N1 5, N2 3)", Live, &[
            "F|\"Buy count ... starts at zero and increases by one whenever a new up run day is defined. The count is reset to zero whenever there is a down run day ... The buy count is closed when a buy signal is received.\" (sell count mirrored) p.273",
            "E|\"Buy case. Reverse to long whenever the buy count reaches N2.\" \"Sell case. Reverse to short whenever the sell count reaches N2.\" p.273-274",
            "X|Exit = the opposite count signal (\"cover short, go long ... cover long, go short\") p.274",
            "F|Illustrated N1 = 5, N2 = 3 (Figures 17.10-17.14, p.275). Before the first signal both counts run [Book silent: \"Either the buy or sell count is always active, depending on whether the current position is short or long\"].",
        ], NOTE),
        d("jscgf_generic", "Ch.16 generic MA and breakout systems", NotTestable(GENERIC_GAP), &["E|\"close above the 20-day high\" basic breakout system, confirmation and filter variations p.235-260"], GENERIC_GAP),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    Some(match id {
        "jscgf_wrd" => ("jscgf", "Schwager original systems", "Wide-ranging day"),
        "jscgf_runday_bo" => ("jscgf", "Schwager original systems", "Run-day breakout"),
        "jscgf_runday_cc" => ("jscgf", "Schwager original systems", "Run-day consecutive count"),
        "jscgf_generic" => ("jscgf", "Schwager original systems", "Generic Ch.16 · not testable"),
        _ => return None,
    })
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    let fill = p("fill", "Fill: 0 = signal close (book), 1 = next open [Not from the book]", 0.0, 0.0, 1.0, 1.0);
    Some(match id {
        "jscgf_wrd" => vec![p("k", "Volatility ratio above (k)", 2.0, 1.6, 2.4, 0.4), p("n", "ATR days (N)", 10.0, 5.0, 20.0, 5.0),
            p("n1", "Days before the WRD in the PTR (N1)", 0.0, 0.0, 4.0, 2.0), p("n2", "Days after the WRD in the PTR (N2)", 0.0, 0.0, 4.0, 2.0), fill],
        "jscgf_runday_bo" => vec![p("n1", "Run-day definition days (N1)", 5.0, 3.0, 7.0, 2.0), p("n2", "Run days in the breakout extreme (N2)", 4.0, 2.0, 5.0, 1.0), fill],
        "jscgf_runday_cc" => vec![p("n1", "Run-day definition days (N1)", 5.0, 3.0, 7.0, 2.0), p("n2", "Consecutive run days (N2)", 3.0, 1.0, 4.0, 1.0), fill],
        _ => return None,
    })
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

fn true_hl(x: &Ctx) -> (Vec<f64>, Vec<f64>) {
    let th = (0..x.c.len()).map(|i| if i == 0 { x.h[0] } else { x.h[i].max(x.c[i - 1]) }).collect();
    let tl = (0..x.c.len()).map(|i| if i == 0 { x.l[0] } else { x.l[i].min(x.c[i - 1]) }).collect();
    (th, tl)
}

/// Run-day type of bar d given N (1 up, -1 down, 0 none); defined only once d + n <= last bar.
pub(crate) fn run_day(th: &[f64], tl: &[f64], d: usize, n: usize) -> i8 {
    if d < n || d + n >= th.len() { return 0; }
    let max = |v: &[f64]| v.iter().copied().fold(f64::MIN, f64::max);
    let min = |v: &[f64]| v.iter().copied().fold(f64::MAX, f64::min);
    let up = th[d] > max(&th[d - n..d]) && tl[d] < min(&tl[d + 1..=d + n]);
    let dn = tl[d] < min(&tl[d - n..d]) && th[d] > max(&th[d + 1..=d + n]);
    match (up, dn) { (true, false) => 1, (false, true) => -1, _ => 0 }
}

/// Stop-and-reverse on the bars where `sigs` is +1/-1 (fill at that close, or the next open).
fn sar(x: &Ctx, from: usize, to: usize, next_open: bool, sigs: &[i8]) -> Vec<Trade> {
    let mut out = vec![];
    let mut cur: Option<Trade> = None;
    for i in from..=to {
        let s = sigs[i] as f64;
        if s == 0.0 || cur.as_ref().is_some_and(|t| t.dir == s) { continue; }
        let (j, px) = if next_open { if i == to { break; } (i + 1, x.o[i + 1]) } else { (i, x.c[i]) };
        if let Some(mut t) = cur.take() { t.exit_i = j; t.exit_px = px; t.signal_i = i; t.reason = "reverse".into(); out.push(t); }
        cur = Some(Trade { dir: s, legs: vec![Leg { i: j, px, w: 1.0 }], exit_i: j, exit_px: px, reason: String::new(),
            entry_kind: if next_open { "open" } else { "close" }, signal_i: i, open: false, exits: vec![] });
    }
    if let Some(mut t) = cur { t.exit_i = to; t.exit_px = x.c[to]; t.signal_i = to; t.reason = "open".into(); t.open = true; out.push(t); }
    out
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    let schema = params(id)?;
    if validate(id, p).is_some() || from > to || to >= x.r.len() { return None; }
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).unwrap().def);
    let u = |k: &str| g(k) as usize;
    let (th, tl) = true_hl(x);
    // Signals are computed bar by bar in time order with the held direction (needed for the "if short"/"if long" checks),
    // so each system walks forward itself and records +1/-1 on the bars where it reverses.
    let mut sig = vec![0i8; x.c.len()];
    let mut held = 0i8;
    match id {
        "jscgf_wrd" => {
            let (k, n, n1, n2) = (g("k"), u("n"), u("n1"), u("n2"));
            let tr = true_range(x.r);
            let (mut ptr, mut last_w): (Option<(f64, f64)>, Option<usize>) = (None, None);
            for i in 0..=to {
                if i >= from {
                    if let Some((hi, lo)) = ptr {
                        let s = if held <= 0 && x.c[i] > hi { 1 } else if held >= 0 && x.c[i] < lo { -1 } else { 0 };
                        if s != 0 { sig[i] = s; held = s; }
                    }
                }
                if i >= n && tr[i] / (tr[i - n..i].iter().sum::<f64>() / n as f64) > k { last_w = Some(i); }
                if let Some(w) = last_w {
                    if i - w == n2 && w >= n1 {
                        let hi = th[w - n1..=i].iter().copied().fold(f64::MIN, f64::max);
                        let lo = tl[w - n1..=i].iter().copied().fold(f64::MAX, f64::min);
                        ptr = Some((hi, lo));
                    }
                }
            }
        }
        _ => {
            let (n1, n2) = (u("n1"), u("n2"));
            let cc = id == "jscgf_runday_cc";
            let (mut ups, mut dns): (Vec<usize>, Vec<usize>) = (vec![], vec![]);
            let mut last = 0i8;
            let (mut buy, mut sell) = (0usize, 0usize);
            for i in 0..=to {
                let rd = if i >= n1 { run_day(&th[..=i], &tl[..=i], i - n1, n1) } else { 0 };
                if rd == 1 { ups.push(i - n1); } else if rd == -1 { dns.push(i - n1); }
                if rd != 0 { last = rd; }
                if i < from { continue; }
                let s = if cc {
                    if rd == 1 { if held <= 0 { buy += 1; } if held >= 0 { sell = 0; } }
                    if rd == -1 { if held >= 0 { sell += 1; } if held <= 0 { buy = 0; } }
                    if held <= 0 && buy == n2 { 1 } else if held >= 0 && sell == n2 { -1 } else { 0 }
                } else {
                    let ext_hi = (dns.len() >= n2).then(|| dns[dns.len() - n2..].iter().map(|&d| th[d]).fold(f64::MIN, f64::max));
                    let ext_lo = (ups.len() >= n2).then(|| ups[ups.len() - n2..].iter().map(|&d| tl[d]).fold(f64::MAX, f64::min));
                    if held <= 0 && last == 1 && ext_hi.is_some_and(|h| x.c[i] > h) { 1 }
                    else if held >= 0 && last == -1 && ext_lo.is_some_and(|l| x.c[i] < l) { -1 } else { 0 }
                };
                if s != 0 { sig[i] = s; held = s; buy = 0; sell = 0; }
            }
        }
    }
    Some(sar(x, from, to, g("fill") == 1.0, &sig))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;
    fn bar(o: f64, h: f64, l: f64, c: f64) -> Row { Row { ticker: "T".into(), date: String::new(), open: o, high: h, low: l, close: c, volume: None } }
    #[test]
    fn wrd_reverses_on_closes_beyond_the_wide_day() {
        let mut r: Vec<Row> = (0..12).map(|_| bar(100., 101., 99., 100.)).collect();
        r.push(bar(100., 106., 98., 105.)); // 12: TR 8 > 2 x 2 -> PTR 98..106 at its close
        r.push(bar(105., 107., 104., 106.5)); // 13: close 106.5 > 106 -> long at close
        r.push(bar(106., 106., 97., 97.5)); // 14: TR 9.5 vs avg(10)=~2.8 -> new WRD, but signal first: close < 98 -> short
        let x = Ctx::new(&r, &r, &[]);
        let t = run("jscgf_wrd", &x, 0, r.len() - 1, &Params::new()).unwrap();
        assert_eq!(t.len(), 2);
        assert_eq!((t[0].dir, t[0].legs[0].i, t[0].exit_i, t[0].exit_px), (1.0, 13, 14, 97.5));
        assert_eq!(t[1].dir, -1.0);
    }
    #[test]
    fn run_day_definition() {
        let th = [10., 11., 12., 15., 13., 13.5, 14.];
        let tl = [9., 10., 11., 10.5, 11., 11.5, 12.];
        assert_eq!(run_day(&th, &tl, 3, 2), 1); // high above prior 2, low below next 2
        assert_eq!(run_day(&th, &tl, 3, 3), 1);
        assert_eq!(run_day(&th, &tl, 4, 3), 0); // not definable yet
    }
}
