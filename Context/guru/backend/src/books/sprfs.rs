//! Stephen A. Pierce, "Rapid Fire Swing Trading: A Simple But Powerful Overlooked Method" (2001-2002).
//! Source copy: Books/Stephen A. Pierce/Pierce, Stephen A - Rapid Fire Swing Trading.pdf (PDF page = printed page).
//! README and oracle: library/sprfs/.
use crate::books::rcmm::{path_entry, ts_path};
use crate::connors::{hv, Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "sprfs";

const NOTE: &str = "Futures rules run on the index (instrument swap). Range = high - low. NR4 = today's range strictly smaller than each of the 3 prior ranges (p.8 \"the smallest of the last four days\"; ties are not 'smallest') [Book silent on ties]. Inside bar (p.8): high <= prior high and low >= prior low, not both equal. Orders for the next session only: buy stop 1 tick above the setup high, sell stop 1 tick below its low (1 tick = 0.05 index points, p.tick). If the bar opens through a stop it fills at the open. Entry bar [Book silent]: TradeStation's bar path (open, the extreme nearer the open, the other extreme, close) decides which stop fills first and whether the protective stop (Rule 3) is reached after the fill on the same bar. Protective stop fills at its level, or at the open on a gap. Exit MOC on the 3rd day counting the entry day as day 1 (examples p.13, 14, 25: entry Aug 14, exit MOC Aug 16). Rule 4's optional profit objective / tightened stop is the trader's choice (\"or exit on the 3-Day Rule\"); Rule 5 applies when \"you have no pre-identified profit exit\", which is the path every worked example follows. Setups are taken only while flat.";
const SAR_GAP: &str = "Discretionary: \"We handle this by using our own discretion, as you should too on doing a 'Stop and Reverse' (SAR) ... you can elect to initially put a SAR on the opposite side\" (p.23). The NASDAQ example then holds \"until either a new NR4/IB signal is generated while the market remained under 50% in the HVR or the HVR rose above 50%\" with no stop stated.";
const POS_GAP: &str = "One example only (British Pound, p.15): \"If you were entering short ... on a position trade and were using a protective stop at the 3-day high, you would have been stopped out on the open of September 21st\". The book does not say whether the 3-day high trails daily, includes the entry day, or replaces the NR4/IB protective stop.";

pub fn catalogue() -> Vec<StrategyDef> {
    let d = |id, name, status, rules: &'static [&'static str], notes| StrategyDef {
        id, name, group: "Rapid Fire Swing Trading · HVR-NR4/IB", session: 0, dir: "Long/Short", status,
        needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK, rules, notes, claim: None,
    };
    use Status::*;
    let core: &'static [&'static str] = &[
        "F|\"(NR4) Narrow Range 4 Bar: A NR4 is a bar with the smallest range of the previous four bars to include the current bar.\" p.8",
        "F|\"(IB) Inside Bar: ... the high is equal to or lower than the prior days high and the low is equal to or higher than the prior days low ... only the high OR the low can be equal\" p.8",
        "E|\"RULE #2 - Once a NR4/IB has been identified, for the next trading session ONLY, place a buy stop order one tick above the high of the NR4/IB bar and a sell stop order one tick below the low\" p.11",
        "X|\"RULE #3 - If the entry order on either side is triggered ... IMMEDIATELY place a protective stop one tick above the high of the NR4/IB bar if short and one tick below the low of the NR4/IB bar if long.\" p.11",
        "X|\"RULE #5 - If the open position is not profitable within three trading days, or you have no pre-identified profit exit, then exit the position MOC (Market on Close) on the 3rd day.\" p.11",
    ];
    vec![
        StrategyDef { claim: crate::connors::claim("12", "75% (9 of 12)", "+$3,844 per S&P contract", "1-3 days", "S&P 500 continuation, Mar-Jul 2001 (p.10)"), ..d("sprfs_nr4ib", "NR4/IB breakout, 3-day MOC exit", Live, core, NOTE) },
        d("sprfs_hvr_nr4ib", "HVR-NR4/IB breakout (HV 10/100 or 6/100 < 50%)", Live, &[
            "F|\"The standard settings used are both a 10/100 and 6/100 HVR with a 50% ratio ... If the located NR4/IB set up has either the 10/100 and/or the 6/100 below 50%, we then have a nice trade set up. At this time we proceed to place the order as described earlier.\" p.16",
            "E|NR4/IB stop orders 1 tick beyond the setup bar, next session only (Rules 2-3, p.11)",
            "X|Protective stop at the opposite extreme; MOC exit on the 3rd day (Rules 3, 5, p.11)",
        ], HVR_NOTE),
        d("sprfs_hvr_nr4", "HVR/NR4 (no inside bar) variation", Live, &[
            "F|\"However, we do enter trades that are just the HVR/NR4 setup. This is a variation of the HVR-NR4/IB setup\" p.24",
            "E|Stop orders 1 tick beyond the NR4 bar, next session only (Rule 2 applied to the NR4 bar, p.11)",
            "X|Protective stop 1 tick beyond the opposite extreme of the NR4 bar; MOC on the 3rd day (Rules 3, 5, p.11)",
        ], HVR_NOTE),
        d("sprfs_sar", "Fake-out reversal (stop and reverse)", NotTestable(SAR_GAP), &["E|\"on the day of entry ONLY if one side of the trade is triggered, you can elect to initially put a SAR on the opposite side\" p.23"], SAR_GAP),
        d("sprfs_position", "Position trade with a 3-day-high protective stop", NotTestable(POS_GAP), &["X|\"using a protective stop at the 3-day high\" p.15"], POS_GAP),
    ]
}
const HVR_NOTE: &str = concat!("Futures rules run on the index (instrument swap). Range = high - low. NR4 = today's range strictly smaller than each of the 3 prior ranges (p.8; ties are not 'smallest') [Book silent on ties]. Inside bar (p.8): high <= prior high and low >= prior low, not both equal. Orders for the next session only, 1 tick (0.05) beyond the setup bar; open through a stop = open fill; on the entry bar TradeStation's bar path (open, nearer extreme, other extreme, close) decides the side and whether the protective stop is reached after the fill [Book silent]. Exit MOC on the 3rd day counting the entry day as day 1 (p.13, 14, 25). Setups taken only while flat. ",
    "HV = annualised sample standard deviation of daily log returns (engine hv()); the book gives no formula for its Dynamic Trader HVR and the ratio does not depend on the annualisation. Setup valid when HV(10)/HV(100) < 0.50 or HV(6)/HV(100) < 0.50 on the setup bar (p.16).");

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    Some(match id {
        "sprfs_nr4ib" => ("sprfs", "Rapid Fire NR4/IB", "NR4/IB"),
        "sprfs_hvr_nr4ib" => ("sprfs", "Rapid Fire NR4/IB", "HVR-NR4/IB"),
        "sprfs_hvr_nr4" => ("sprfs", "Rapid Fire NR4/IB", "HVR/NR4"),
        "sprfs_sar" => ("sprfs", "Rapid Fire NR4/IB", "SAR · not testable"),
        "sprfs_position" => ("sprfs", "Rapid Fire NR4/IB", "Position trade · not testable"),
        _ => return None,
    })
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    let mut v = vec![p("tick", "One tick in index points (NSE 0.05)", 0.05, 0.0, 1.0, 0.05), p("hold", "MOC exit on day (entry day = 1)", 3.0, 1.0, 10.0, 1.0)];
    match id {
        "sprfs_nr4ib" => {}
        "sprfs_hvr_nr4ib" | "sprfs_hvr_nr4" => v.extend([p("hv_a", "Short HV days (1)", 10.0, 3.0, 30.0, 1.0), p("hv_b", "Short HV days (2)", 6.0, 3.0, 30.0, 1.0),
            p("hv_long", "Long HV days", 100.0, 50.0, 250.0, 10.0), p("ratio", "HVR trigger (short/long below)", 0.5, 0.2, 1.0, 0.05)]),
        _ => return None,
    }
    Some(v)
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

fn rng(x: &Ctx, j: usize) -> f64 { x.h[j] - x.l[j] }
pub(crate) fn nr4(x: &Ctx, j: usize) -> bool { j >= 3 && (1..=3).all(|k| rng(x, j) < rng(x, j - k)) }
pub(crate) fn inside(x: &Ctx, j: usize) -> bool {
    j >= 1 && x.h[j] <= x.h[j - 1] && x.l[j] >= x.l[j - 1] && !(x.h[j] == x.h[j - 1] && x.l[j] == x.l[j - 1])
}

/// Next-session stop entries around setup bar s, protective stop at the other extreme, MOC on day `hold`.
fn nr4_trades(x: &Ctx, from: usize, to: usize, tick: f64, hold: usize, setup: &dyn Fn(usize) -> bool) -> Vec<Trade> {
    let mut out = vec![];
    let mut i = from.max(1);
    while i <= to {
        let s = i - 1;
        if !setup(s) { i += 1; continue; }
        let (up, dn) = (x.h[s] + tick, x.l[s] - tick);
        let pts = ts_path(x, i);
        let Some((dir, fill, seg)) = path_entry(&pts, up, dn) else { i += 1; continue };
        let stop = if dir > 0.0 { dn } else { up };
        let mut t = Trade { dir, legs: vec![Leg { i, px: fill, w: 1.0 }], exit_i: i, exit_px: fill, reason: String::new(),
            entry_kind: "stop", signal_i: s, open: false, exits: vec![] };
        // entry bar: the protective stop counts only if the path comes back to it after the fill
        let after = pts[seg + 1..].iter().copied().fold(dir * f64::INFINITY, |m, v| if dir > 0.0 { m.min(v) } else { m.max(v) });
        if dir * (after - stop) <= 0.0 {
            t.exit_px = stop; t.reason = "protective stop".into(); t.signal_i = i;
            out.push(t); i += 1; continue;
        }
        let mut j = i;
        loop {
            let adverse = if dir > 0.0 { x.l[j] } else { x.h[j] };
            if j == i { if j + 1 == i + hold { t.exit_px = x.c[j]; t.reason = format!("MOC day {hold}"); break; } if j == to { t.exit_px = x.c[j]; t.reason = "open".into(); t.open = true; break; } j += 1; continue; }
            if dir * (x.o[j] - stop) <= 0.0 { t.exit_px = x.o[j]; t.reason = "protective stop (gap)".into(); }
            else if dir * (adverse - stop) <= 0.0 { t.exit_px = stop; t.reason = "protective stop".into(); }
            else if j + 1 == i + hold { t.exit_px = x.c[j]; t.reason = format!("MOC day {hold}"); }
            else if j == to { t.exit_px = x.c[j]; t.reason = "open".into(); t.open = true; }
            else { j += 1; continue; }
            break;
        }
        t.exit_i = j; t.signal_i = j;
        out.push(t);
        i = j + 1;
    }
    out
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    let schema = params(id)?;
    if validate(id, p).is_some() || from > to || to >= x.r.len() { return None; }
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).unwrap().def);
    let (tick, hold) = (g("tick"), g("hold") as usize);
    Some(match id {
        "sprfs_nr4ib" => nr4_trades(x, from, to, tick, hold, &|s| nr4(x, s) && inside(x, s)),
        _ => {
            let (a, b, l) = (hv(&x.c, g("hv_a") as usize), hv(&x.c, g("hv_b") as usize), hv(&x.c, g("hv_long") as usize));
            let r = g("ratio");
            let hvr = |s: usize| l[s] > 0.0 && (a[s] / l[s] < r || b[s] / l[s] < r);
            let ib = id == "sprfs_hvr_nr4ib";
            nr4_trades(x, from, to, tick, hold, &|s| nr4(x, s) && (!ib || inside(x, s)) && hvr(s))
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;
    fn bar(o: f64, h: f64, l: f64, c: f64) -> Row { Row { ticker: "T".into(), date: String::new(), open: o, high: h, low: l, close: c, volume: None } }
    #[test]
    fn lumber_style_short_exits_moc_on_day_three() {
        let mut r = vec![bar(100., 110., 90., 100.), bar(100., 108., 92., 100.), bar(100., 107., 93., 100.)];
        r.push(bar(100., 105., 95., 100.)); // NR4 + inside
        r.push(bar(96., 97., 90., 91.)); // sell stop 94.95 gapped -> fills at the 96 open? no: open 96 > 94.95, low 90 -> 94.95
        r.push(bar(91., 93., 89., 90.));
        r.push(bar(90., 92., 88., 89.)); // day 3 MOC 89
        r.push(bar(89., 120., 80., 100.));
        let x = Ctx::new(&r, &r, &[]);
        let t = run("sprfs_nr4ib", &x, 0, r.len() - 1, &Params::new()).unwrap();
        assert_eq!(t.len(), 1);
        assert_eq!((t[0].dir, t[0].legs[0].i, t[0].exit_i), (-1.0, 4, 6));
        assert!((t[0].legs[0].px - 94.95).abs() < 1e-9 && t[0].exit_px == 89.0);
        // both sides on the entry bar: open equidistant -> low first (TradeStation path): short, then stopped at the high side
        r[4] = bar(100., 106., 94., 100.);
        let x = Ctx::new(&r, &r, &[]);
        let t = run("sprfs_nr4ib", &x, 0, r.len() - 1, &Params::new()).unwrap();
        assert_eq!(t[0].exit_i, 4);
        assert_eq!(t[0].dir, -1.0);
        assert!((t[0].ret(0.0) + 10.1 / 94.95).abs() < 1e-9);
    }
}
