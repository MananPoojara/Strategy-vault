//! Prime-Line, "Prime9 System - Harmonic Alignment Scalp Strategy" (prime-line.com, 2007; research share USB HDD 2/tRADING KNO-2/Prime Line Prime9 System. Harmonic Aligment Scalp
//! Strategy (prime-line.com)/Harmonic Alignment Scalp Strategy.pdf, a PDF package of 23 one-page slides, all OCR-read). Book id `prime9`. 9-minute chart: buy when price crosses
//! above the 27 SMA (qualifier: the 27 SMA is flat or moving up), sell short when price crosses below it (flat or moving down); exit when price crosses the 27 SMA in the
//! opposite direction. Lane K3 (24 Nov 2026, R4 #397, row I82). Each direction is its own card (one position per card). Independent check authors/conv-1123/k3w/prime9_ref.py.
use crate::connors::{Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "prime9";
pub const IDS: &[&str] = &["prime9_long", "prime9_short"];
pub fn reviewed(id: &str) -> bool { IDS.contains(&id) }

const NOTE: &str = "[Declared adaptation, R4 #340/#380; 9 labelled choices; unverified] The slides trade futures (crude oil, S&P) on a 9-minute chart; this card runs the printed rule on 9-minute bars built from the real 1-minute bars of SPX / NIFTY 50 / NIFTYBANK (SPY as the declared ETF application). Rule: buy when price crosses above the 27-period simple average of the close, qualifier 'the line is flat or moving up'; sell short when price crosses below it, qualifier 'flat or moving down'; exit when price crosses the 27 SMA the other way (Step 3). Unstated choices (one independent choice per number): (1) price = the 9-minute bar close; (2) a cross may start from the line: the previous close on the average counts as the other side; (3) flat or moving up (down) = the 27 SMA not below (above) its value one bar earlier, so a flat line qualifies; (4) the 27 SMA is one continuous series over all sessions; (5) the 9-minute bars are anchored at the session open; (6) no entry is taken on the last 9-minute bar of a session; (7) no initial stop is applied (the slides give no distance: 'ultra-mini risk initial stops'); (8) a position still held at the last 9-minute bar of the session closes at that bar close (no overnight risk); (9) the bar that exits a position does not also open a new one; Honest total: 9 card-specific choices. Engine-wide defaults / declared adaptations (labelled, not counted, R4 #404): the order fills at the next 9-minute bar's open (closed-bar convention); the 9-minute bars are built from the real 1-minute bars (SPX, NIFTY 50, NIFTYBANK; SPY as the declared ETF application); the data window. The optional qualifiers (moving-average envelope, 9 EMA) and the optional trailing stops (9 EMA close, Parabolic SAR, volatility stop) are not applied; the proprietary 'Prime 9' signals shown on the slides' charts are not defined and not coded. Long and short are separate cards.";

pub fn catalogue() -> Vec<StrategyDef> {
    macro_rules! mk { ($id:expr, $name:expr, $dir:expr, $e:expr) => { StrategyDef { id: $id, name: $name, group: "Prime-Line Prime9", session: 0, dir: $dir, status: Status::Live,
        needs_volume: false, needs_vix: false, needs_series: &[], book: "prime9", rules: &[$e,
            "X|\"Exit position when price crosses 27 SMA (Orange line) in the opposite direction\" (Step 3, Strategy Usage); \"No overnight risk\" (advantages slide)",
            "F|9-minute chart; 27 period SMA of the close (Step 2, Insert Indicators; Step 3)"], notes: NOTE, claim: None } } }
    vec![mk!("prime9_long", "Prime9 long: price crosses above the 27 SMA of 9-minute bars while it is flat or rising, exit on the cross back below (indices, 1-minute bars)", "Long",
            "E|\"Buy when price crosses above Orange line. Qualifier: Orange line is flat or moving up\" (Step 3, Buy (Long) Setup)"),
        mk!("prime9_short", "Prime9 short: price crosses below the 27 SMA of 9-minute bars while it is flat or falling, exit on the cross back above (indices, 1-minute bars)", "Short",
            "E|\"Sell Short when price crosses below Orange line. Qualifier: Orange line is flat or moving down\" (Step 3, Sell Short Setup)")]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    reviewed(id).then_some(("prime9", "Prime9 27 SMA cross on 9-minute bars", if id == "prime9_long" { "Long side" } else { "Short side" }))
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    reviewed(id).then(|| vec![ParamDef { key: "sma", label: "Simple moving average length in 9-minute bars (the slides: 27)", def: 27.0, min: 5.0, max: 100.0, step: 1.0 }])
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.r.len() { return None; }
    let s = params(id)?;
    let nn = p.get("sma").copied().unwrap_or(s[0].def).round().max(2.0) as usize;
    let dir = if id == "prime9_long" { 1.0 } else { -1.0 };
    let mut mins = Vec::with_capacity(to + 1);
    let mut bars: Vec<(usize, usize, usize, f64)> = vec![]; // (day, first minute index, last minute index, close) of every 9-minute bar, in order
    for i in 0..=to {
        match crate::intraday_ep::day_bars(x, i, 9) {
            Some((_, _, m, b)) => { for z in &b { bars.push((i, z.a, z.b - 1, z.c)); } mins.push(Some(m)); }
            None => mins.push(None),
        }
    }
    let n = bars.len();
    let mut sma = vec![f64::NAN; n];
    for k in nn - 1..n { sma[k] = bars[k + 1 - nn..=k].iter().map(|b| b.3).sum::<f64>() / nn as f64; }
    let mut out: Vec<Trade> = vec![];
    let mut pos: Option<(usize, f64, usize)> = None; // (entry day, entry price, signal day)
    for k in nn..n {
        let day = bars[k].0;
        let last = k + 1 >= n || bars[k + 1].0 != day;
        let (c, c0) = (bars[k].3, bars[k - 1].3);
        if let Some((ed, px, sg)) = pos {
            let opp = dir * (c - sma[k]) < 0.0 && dir * (c0 - sma[k - 1]) >= 0.0;
            let xp = if last { Some((mins[day].as_ref().unwrap()[bars[k].2].c, "session end")) } else if opp { Some((mins[day].as_ref().unwrap()[bars[k + 1].1].o, "opposite cross")) } else { None };
            if let Some((xp, why)) = xp {
                out.push(Trade { dir, legs: vec![Leg { i: ed, px, w: 1. }], exit_i: day, exit_px: xp, reason: why.into(), entry_kind: "open", signal_i: sg, open: false, exits: vec![] });
                pos = None;
            }
            continue;
        }
        if last || day < from || day > to { continue; }
        let cross = dir * (c - sma[k]) > 0.0 && dir * (c0 - sma[k - 1]) <= 0.0;
        if cross && dir * (sma[k] - sma[k - 1]) >= 0.0 {
            pos = Some((day, mins[day].as_ref().unwrap()[bars[k + 1].1].o, day));
        }
    }
    if let Some((ed, px, sg)) = pos { out.push(Trade { dir, legs: vec![Leg { i: ed, px, w: 1. }], exit_i: to, exit_px: x.c[to], reason: "open".into(), entry_kind: "open", signal_i: sg, open: true, exits: vec![] }); }
    Some(out)
}
