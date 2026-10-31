//! Markus Heitkoetter, How to Make Money with Trading Systems (Rockwell Trading, 2005, 28-page report).
//! One sample system (Bollinger Band fade, pdf p.9-15); the rest is generic system-design advice.
//! Sources and verification: library/hkmmts/.
use crate::connors::{claim, sma, Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};

const BOOK: &str = "hkmmts";
/// Admitted ids (authors/heitkoetter/admission/SOURCE-CONTRACT.md).
pub const REVIEWED: &[&str] = &["hkmmts_bb_fade", "hkmmts_bb_fade_2day"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const RULES_0: &[&str] = &[
    "F|Bollinger Bands: \"a setting of 34 for the Moving Average and 2.5 for the Standard Deviation\" (p.12) on daily closes. Bands = SMA(34) +/- 2.5 x population standard deviation of the 34 closes (TradeStation BollingerBand). [Book silent on the stdev convention]",
    "E|\"Sell when prices move above the Bollinger Bands and Buy when prices move below the Bollinger Bands.\" (p.10) \"We decide to trade the market intraday, i.e. we will enter and exit a trade on the same day\" (p.9).",
    "E|Daily-bar reading: the bands from yesterday's close are today's levels; short when today's high > upper band, filled at the band or at the open if it opens above; long when today's low < lower band, at the band or the open if it opens below. A day that crosses both bands is skipped (order of touches unknown). [Book silent; necessary for daily data]",
    "X|\"Exit the trade at the close of the same day.\" (p.11)",
];
const RULES_2: &[&str] = &[
    "F|Bollinger Bands SMA(34) +/- 2.5 standard deviations (p.12), as in hkmmts_bb_fade.",
    "E|\"Sell when prices move above the Bollinger Bands and Buy when prices move below the Bollinger Bands.\" (p.10); same daily-bar fill reading as hkmmts_bb_fade.",
    "X|\"If we exit on at the end of the 2nd day after entering the market, we increase the net profit\" (p.14): exit at the close of the 2nd trading day after the entry day. No new entry while a trade is open. [Book silent on overlapping signals]",
];

pub fn catalogue() -> Vec<StrategyDef> {
    let make = |id, name, group, dir, status, rules, notes, claim| StrategyDef {
        id, name, group, session: 0, dir, status, needs_volume: false, needs_vix: false,
        needs_series: &[], book: BOOK, rules, notes, claim,
    };
    vec![
        make("hkmmts_bb_fade", "Bollinger Band fade, exit same-day close", "A Sample Trading System (p.9-13)", "Long/Short", Status::Live, RULES_0,
            "Book: e-mini S&P, last 2 years before 2005, $50/point: net $13,525, avg $149/trade (= 2.98 points), PF 2.20, win 66% in the text but 59.30% in the MA-34 table (p.12); ~91 trades implied (13,525/149). Also DAX, e-mini Nasdaq, EuroStoxx, FTSE (p.13). Run on the cash index instead (instrument swap). Stop-loss variants (p.14) made results worse and are not the system; not coded.",
            claim("~91 (13,525/149)", "66% (text) / 59.3% (table)", "$149 (2.98 pts)", "same day", "e-mini S&P, 2 years to 2005")),
        make("hkmmts_bb_fade_2day", "Bollinger Band fade, exit close of 2nd day", "A Sample Trading System: Improving (p.14-15)", "Long/Short", Status::Live, RULES_2,
            "Book's improved rule (p.14): e-mini S&P net $17,450, PF 3.12, avg $371, max DD $2,775 (table x=2); second test table p.15 shows e-mini S&P $8,100, PF 1.76, avg $153 (the two tables disagree). Run on the cash index instead.",
            claim("~47 (17,450/371)", "n/a", "$371 (7.42 pts)", "2 days", "e-mini S&P, 2 years to 2005")),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    Some(match id {
        "hkmmts_bb_fade" => ("hkmmts_bb", "Heitkoetter Bollinger fade", "Exit same-day close"),
        "hkmmts_bb_fade_2day" => ("hkmmts_bb", "Heitkoetter Bollinger fade", "Exit close of 2nd day"),
        _ => return None,
    })
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    let hold = match id { "hkmmts_bb_fade" => 0., "hkmmts_bb_fade_2day" => 2., _ => return None };
    Some(vec![
        p("ma_n", "Bollinger moving average days", 34., 29., 39., 1.),
        p("sd_k", "Standard deviations", 2.5, 1.5, 3.5, 0.1),
        p("hold", "Exit on close of x-th day after entry (0 = same day)", hold, 0., 6., 1.),
    ])
}

/// Bands at each close: (upper, lower); NaN until n closes exist. Population standard deviation.
pub fn bands(c: &[f64], n: usize, k: f64) -> (Vec<f64>, Vec<f64>) {
    let m = sma(c, n);
    let (mut up, mut lo) = (vec![f64::NAN; c.len()], vec![f64::NAN; c.len()]);
    for i in 0..c.len() {
        if !m[i].is_finite() { continue; }
        let var = c[i + 1 - n..=i].iter().map(|v| (v - m[i]).powi(2)).sum::<f64>() / n as f64;
        up[i] = m[i] + k * var.sqrt();
        lo[i] = m[i] - k * var.sqrt();
    }
    (up, lo)
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    let schema = params(id)?;
    if from > to || to >= x.r.len() { return None; }
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).unwrap().def);
    let (n, k, hold) = (g("ma_n").round().max(2.) as usize, g("sd_k"), g("hold").round().max(0.) as usize);
    let (up, lo) = bands(&x.c, n, k);
    let mut out: Vec<Trade> = vec![];
    let mut i = from.max(1);
    while i <= to {
        let (u, l) = (up[i - 1], lo[i - 1]);
        let (hit_u, hit_l) = (u.is_finite() && x.h[i] > u, l.is_finite() && x.l[i] < l);
        if hit_u == hit_l { i += 1; continue; } // none, or both (order unknown)
        let (dir, px) = if hit_u { (-1., x.o[i].max(u)) } else { (1., x.o[i].min(l)) };
        let e = i + hold;
        let open = e > to;
        let ex = e.min(to);
        out.push(Trade { dir, legs: vec![Leg { i, px, w: 1. }], exit_i: ex, exit_px: x.c[ex],
            reason: if open { "open".into() } else if hold == 0 { "Close of entry day".into() } else { format!("Close of day {hold} after entry") },
            entry_kind: "limit", signal_i: ex, open, exits: vec![] });
        i = e + 1;
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;
    #[test]
    fn fade_fills_at_band_or_open_and_exits_on_close() {
        let mut r: Vec<Row> = (0..60).map(|i| { let c = 100. + if i % 2 == 0 { 1. } else { -1. }; Row { ticker: "T".into(), date: format!("{:05}", i), open: c, high: c + 0.5, low: c - 0.5, close: c, volume: None } }).collect();
        // bands at bar 49: mean 100, pop sd 1 -> upper 102.5
        r[50].open = 101.; r[50].high = 104.; r[50].close = 100.5;
        r[51].open = 106.; r[51].high = 107.; r[51].close = 105.;
        let x = Ctx::new(&r, &r, &[]);
        let ts = run("hkmmts_bb_fade", &x, 40, 51, &Params::new()).unwrap();
        assert_eq!(ts.len(), 2);
        assert_eq!((ts[0].dir, ts[0].legs[0].i, ts[0].exit_i), (-1., 50, 50));
        assert!((ts[0].legs[0].px - 102.5).abs() < 1e-9);
        assert_eq!(ts[0].exit_px, 100.5);
        assert_eq!(ts[1].legs[0].px, 106.); // opened above the band -> fill at open
        let t2 = run("hkmmts_bb_fade_2day", &x, 40, 59, &Params::new()).unwrap();
        assert_eq!((t2[0].legs[0].i, t2[0].exit_i), (50, 52)); // bar 51 signal ignored while in trade
    }
}

// Instrument admission: Heitkoetter 2026-10-25 block, moved from instrument_policy.rs on 31 Oct 2026 (lane ST).
// Admission only for the module REVIEWED lists (authors/heitkoetter/admission/SOURCE-CONTRACT.md); kept in one block to merge.
// Runs on actual indices, ETFs and stocks, US and India (index_etf_or_stock, nifty50 basket, declared application).
pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book=="hkmmts" && crate::books::hkmmts::reviewed(def.id),
    label: "Markus Heitkoetter tested this sample system intraday on e-mini S&P, DAX, e-mini Nasdaq, EuroStoxx and FTSE futures; runs on daily indices, ETFs and stocks, US and India, are a declared application, not the author's test.",
    basis: "Markus Heitkoetter, How to Make Money with Trading Systems (Rockwell Trading, 2005): Bollinger Band (34, 2.5) fade - sell above the upper band, buy below the lower band, exit at the close of the same day (p.9-13) or at the close of the 2nd day after entry (p.14). Reviewed in authors/heitkoetter/admission/SOURCE-CONTRACT.md with independent chronological ledgers; no costs.",
};
