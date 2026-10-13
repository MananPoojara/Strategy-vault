//! Nelson Freeburg, "Timing Models and Classic Indicators for Today's Markets" (Formula Research,
//! TradersLibrary seminar handout, 2002, 21 pp.). Source and checks: library/nftm/.
//! Live: the AccuTrack two-asset switching formula (Fig. 22). The Heine bond model, PENTAD and the
//! intermarket T-Bond system need bond / Dow Transports / Utilities / nickel series the vault lacks.
use crate::connors::{Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "nftm";

const HEINE: &str = "weekly Lehman long-term Government bond index, US cash T-Bond yield, 90-day T-Bill yield, Dow Jones Utility Average and CRB futures index; none in the vault";
const PENTAD: &str = "weekly NYSE advance/decline line is available (NYSE_ADV_DEC to 2020-02-10) but Dow Jones Transportation Average, Dow Jones Utility Average and the Lehman / Dow Jones 20 bond index are not in the vault";
const TBOND: &str = "T-Bond and 10-year note futures, LME nickel, Dow Jones Utilities, Fidelity Select Chemicals (FSCHX) and Financial (FIDSX); none in the vault";

pub fn catalogue() -> Vec<StrategyDef> {
    use Status::*;
    let d = |id, name, group, dir, status, needs_series, rules, notes, claim| StrategyDef {
        id, name, group, session: 0, dir, status, needs_volume: false, needs_vix: false, needs_series, book: BOOK, rules, notes, claim,
    };
    vec![
        d("nftm_accutrack", "AccuTrack switching formula (hold this asset while favoured)", "Formula Research: Recent Findings", "Long", Live, &["XMKT.C2"],
          &["F|\"A. Calculate Daily Percent Change of Fund I and Fund II. B. Smooth Each Series with 48-Day EMA. C. Subtract Treated Data for Fund II from Fund I Data. D. Smooth with 12-Day EMA\" (Fig. 22, p.E-17)",
            "E|\"E. If Result Positive Fund I Is Favored\": hold the selected ticker (Fund I) while the result is positive",
            "X|\"If Result Negative Fund II is favored\": switch out of Fund I when the result turns negative (Fund II's side is the same strategy run on the partner, whose result is the exact negative)",
            "F|Partner (Fund II) = column C2 of the ticker's XMKT series (p.xcol picks C2..C5): on IWM C2 = SPX (small-cap vs S&P 500, the book's Vanguard Small-Cap Index vs Index 500 pair, Fig. 20); on NIFTYMIDCAP100 C2 = NIFTY 50"],
          "Book results (Fig. 20, p.E-16): Vanguard Small-Cap Index CAR 11.21%, Index 500 12.99%, switch strategy 15.55%; Small-Cap Value 8.40% / Growth 4.12% / switch 12.87% (test period not printed). Same formula as 'AccuTrack Relative Strength Index' (Building High-Performance Trading Systems Fig. 30). EMA = TradeStation XAverage (alpha 2/(n+1), seeded with the first value); percent change = close/close[1]-1; signals start after 60 valid days. Fill: book silent; default = the signal day's close (Freeburg's own 'Action on Day of Signal' fund convention, Pankin workbook Fig. 33), p.fill=1 = next open. Zero result = no change. Long/cash on this ticker; prices are the vault's index/ETF closes (no dividends on indices).",
          None),
        d("nftm_heine_bond", "Heine Bond Market Timing Model", "Heine Bond Market Timing Model", "Long", NeedsData(HEINE), &[],
          &["F|Bullish: Lehman LT Gov't Bond Index > 24-week SMA; US cash T-Bond yield < 6-week SMA; 90-day T-Bill yield < 6-week SMA; DJ Utility Average > 10-week SMA; CRB futures index < 20-week SMA (p.E-8)",
            "E|\"Buy Bonds When Model Count >= 3 Points\" (p.E-9)", "X|\"Sell Bonds (Switch to Money Market) When Model Count <= 2 Points\" (p.E-9)"],
          "Book (Fig. 10, 1/2/70-4/26/02, DJ Corporate Bond index): exposure 53%, B&H 9.9% CAR (model CAR / trade counts illegible in the scan).", None),
        d("nftm_pentad", "PENTAD Stock Market Timing Model", "PENTAD Stock Market Timing Model", "Long", NeedsData(PENTAD), &[],
          &["F|Bullish: S&P 500 > 65-week WMA; weekly A/D line > 14-week WMA + 0.5%; DJ Transports > 25-week SMA + 0.5%; 27-week SMA of DJ Utilities rises by any amount; Lehman bond index > 38-week WMA + 1% (p.E-13)",
            "F|Bearish: S&P < 65-wk WMA - 3%; A/D < 14-wk WMA - 2%; DJT < 25-wk SMA - 2.5%; 27-wk SMA of DJUA drops 3% from a peak; bond index 2% below its 38-wk WMA (p.E-13; Pankin workbook Fig. 38 sell filter 2%)",
            "F|\"Once a Signal Is Given, Indicator Remains in That Mode until Reversing Signal Is Flashed\" (p.E-14)",
            "E|\"Buy Stocks When Model Count = 5 Points\" (p.E-14)", "X|\"Sell Stocks (Switch to Money Market) When Model Count <= 3\" (p.E-14)"],
          "Book (Fig. 16, 1/2/70-4/26/02): $10,000 -> $792,400 vs S&P B&H 11.6% CAR / $346,200; Pankin workbook Fig. 26 (1970-97): CAR 15.6%, $561,400, max DD 12.3%. The OCR'd bearish bond rule reads '> 38-Wk WMA Plus 1' (scan typo; Fig. 38 gives the 2% sell filter).", None),
        d("nftm_tbond_intermarket", "Intermarket T-Bond Trading System", "Intermarket T-Bond Trading System", "Long/Short", NeedsData(TBOND), &[],
          &["F|Bullish: Nickel < Nickel[20]; DJUA/S&P > DJUA/S&P[32]; FSCHX/S&P < FSCHX/S&P[28]; FIDSX/S&P > FIDSX/S&P[28] (Fig. 28)",
            "E|Long entry: model count >= 3; short entry: model count <= 1 (Fig. 28)", "X|\"$2,000 Initial Stop\" (Fig. 28)"],
          "Book (Figs 25-26, 1986-2002): T-Bond 231 trades 55%, avg $734.82; 10-yr note 211 trades 59%, avg $568.71.", None),
        d("nftm_vl_swing", "Value Line Percent Swing System", "Value Line Percent Swing", "Long", NotTestable("Only table labels 'Buy on N-Percent Rally / Sell on N-Percent Pullback' (Fig. 2): reference point, data frequency and price basis are not stated; needs the Value Line Geometric index (not in the vault)."), &[],
          &["E|Buy on N-percent rally / sell (switch to money market) on N-percent pullback; N = 2/4/6/8% (Fig. 2, since 1963)"], "Fig. 2: 2%/2% = 12.4% a year vs Value Line B&H 3.6%. Not coded.", None),
        d("nftm_results_only", "Wilder Volatility System charts, Hussman fund, fund switching tables, month-end seasonal, SABRE", "Results / charts only", "Long/Short",
          NotTestable("Charts and result tables only (Figs 3-9, 14-15, 20-21, 23, SABRE page); no rules printed. Wilder's Volatility System is already in the vault (wwnc_vol); month-end seasonality is covered by kftsm."), &[],
          &["E|No entry/exit rules printed"], "Not coded.", None),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    Some(match id {
        "nftm_accutrack" => ("nf_accutrack", "Freeburg AccuTrack switching", "Hold while favoured"),
        "nftm_heine_bond" => ("nf_heine", "Heine bond timing model", "2002 · Data missing"),
        "nftm_pentad" => ("nf_pentad", "Freeburg PENTAD", "2002 · Data missing"),
        "nftm_tbond_intermarket" => ("nf_tbond", "Freeburg intermarket T-Bond", "2002 · Data missing"),
        "nftm_vl_swing" => ("nf_vlswing", "Value Line percent swing", "2002 · not testable"),
        "nftm_results_only" => ("nf_results", "Freeburg results-only examples", "2002"),
        _ => return None,
    })
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    match id {
        "nftm_accutrack" => Some(vec![
            p("slow", "EMA of daily % changes (days)", 48., 5., 100., 1.),
            p("fast", "EMA of the difference (days)", 12., 2., 50., 1.),
            p("xcol", "Partner = XMKT column (2-5)", 2., 2., 5., 1.),
            p("fill", "Fill: 0 = signal day's close, 1 = next open", 0., 0., 1., 1.),
        ]),
        _ => None,
    }
}

/// AccuTrack result per bar (NaN until `warm` valid bars). EMA = TradeStation XAverage.
pub fn accutrack(a: &[f64], b: &[f64], slow: usize, fast: usize, warm: usize) -> Vec<f64> {
    let (as_, af) = (2. / (slow as f64 + 1.), 2. / (fast as f64 + 1.));
    let (mut e1, mut e2, mut e3, mut n) = (f64::NAN, f64::NAN, f64::NAN, 0usize);
    let mut out = vec![f64::NAN; a.len()];
    for i in 1..a.len() {
        let (pa, pb) = (a[i] / a[i - 1] - 1., b[i] / b[i - 1] - 1.);
        if !pa.is_finite() || !pb.is_finite() { if n >= warm { out[i] = e3; } continue; }
        if n == 0 { e1 = pa; e2 = pb; e3 = pa - pb; } else {
            e1 += as_ * (pa - e1); e2 += as_ * (pb - e2); e3 += af * ((e1 - e2) - e3);
        }
        n += 1;
        if n >= warm { out[i] = e3; }
    }
    out
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    let schema = params(id)?;
    if from > to || to >= x.r.len() { return None; }
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).unwrap().def);
    let col = g("xcol").round().clamp(2., 5.) as usize;
    let Some(b) = x.series(&format!("XMKT.C{col}")) else { return Some(vec![]) };
    let (slow, fast) = (g("slow").round().max(1.) as usize, g("fast").round().max(1.) as usize);
    let s = accutrack(&x.c, b, slow, fast, slow + fast);
    let next_open = g("fill") > 0.5;
    let mut out = vec![];
    let mut cur: Option<Trade> = None;
    for i in from..=to {
        if !s[i].is_finite() { continue; }
        let (fi, fpx, kind) = if next_open { if i == to { continue } (i + 1, x.o[i + 1], "open") } else { (i, x.c[i], "close") };
        match cur.as_mut() {
            None if s[i] > 0. => cur = Some(Trade { dir: 1., legs: vec![Leg { i: fi, px: fpx, w: 1. }], exit_i: fi, exit_px: fpx, reason: String::new(), entry_kind: kind, signal_i: i, open: false, exits: vec![] }),
            Some(t) if s[i] < 0. && fi > t.legs[0].i => {
                t.exit_i = fi; t.exit_px = fpx; t.signal_i = i; t.reason = "AccuTrack negative: partner favoured".into();
                out.push(cur.take().unwrap());
            }
            _ => {}
        }
    }
    if let Some(mut t) = cur { t.exit_i = to; t.exit_px = x.c[to]; t.signal_i = to; t.reason = "open".into(); t.open = true; out.push(t); }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn accutrack_by_hand_and_antisymmetric() {
        // slow=1 (alpha 1), fast=1: result = today's % change difference
        let a = [100., 110., 99.];
        let b = [100., 100., 100.];
        let s = accutrack(&a, &b, 1, 1, 1);
        assert!(s[0].is_nan());
        assert!((s[1] - 0.10).abs() < 1e-12 && (s[2] + 0.10).abs() < 1e-12);
        let s2 = accutrack(&b, &a, 48, 12, 1);
        let s1 = accutrack(&a, &b, 48, 12, 1);
        assert!((s1[2] + s2[2]).abs() < 1e-15);
        // EMA seeding and update: slow 3 (alpha .5), fast 3 (alpha .5)
        let a = [100., 101., 103.02, 103.02];
        let b = [100., 100., 100., 100.];
        let s = accutrack(&a, &b, 3, 3, 1);
        // pa: .01, .02, 0 ; e1: .01, .015, .0075 ; e3: .01, .0125, .01
        assert!((s[3] - 0.01).abs() < 1e-12);
    }
}
