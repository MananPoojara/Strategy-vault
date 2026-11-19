//! D. Aronson, Evidence-Based Technical Analysis (Wiley 2007), Part II case study (ch.8-9, p.389-450).
//! Only rules whose input is the traded index's own close are Live (input series 1 = S&P 500 close).
//! Source: library/arebta/.
use crate::connors::{Claim, Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};
/// Admitted ids (authors/aronson/admission/SOURCE-CONTRACT.md).
pub const REVIEWED: &[&str] = &["arebta_d_djta", "arebta_d_hlr10", "arebta_d_hlr30", "arebta_d_pt10y", "arebta_e_durspd", "arebta_tt_pt10y" /* lane H3b, R4 #339/#343/#346 */, "arebta_tt", "arebta_ti", "arebta_e", "arebta_tt_br", "arebta_ti_br", "arebta_e_br", "arebta_d_br" /* lane K78, R4 #276 (3), #287 */,
    "arebta_tt_br_nse", "arebta_ti_br_nse", "arebta_e_br_nse", "arebta_d_br_nse" /* lane CV6, R4 #340 / #347 / #365 */];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const NOTE: &str = "Case study: 6,402 binary long/short reversal rules on the S&P 500, 1 Nov 1980 - 1 Jul 2005, evaluated on DETRENDED daily returns (mean daily change subtracted, p.391-392); entries and exits \"on the open price of the day following a position-reversal signal\" (p.449). Input series 1 = S&P 500 close; here the traded index's own close (instrument swap for NIFTY). Always in the market once the rule output is defined (the position is taken at the first open after the selected start).";
const OTHER: &str = "umbrella of the other 33 rule inputs of Table 8.2 (p.418); lane U2 (19 Nov 2026): the Dow transports and utilities ($DJT/$DJU), NASDAQ Composite ($COMP), NYSE new highs/lows (#NYSEHI/#NYSELO), the 3-month T-bill (DTB3) and the 10/30-year Treasury yields (DGS10/DGS30) ARE held, so they are no longer the reason; still not held: S&P Industrials, NYSE financials, Value Line, the DJ 20 bonds, AAA and BAA yields; and the volume indicators. The DJIA high/low/close (Table 8.1 raw series 3-5) feed only the accumulation-distribution and money-flow indicators (inputs 12-17, p.410-411), which multiply by NYSE volume; NYSE volume changes units on 2002-06-21 (thousands to shares), so every volume-level input (9-23, 33) is excluded (R4 #287). The six breadth inputs 24-29 are coded as arebta_tt_br, arebta_ti_br, arebta_e_br and arebta_d_br";
/// Lane K78 (R4 #276 (3), #287): Table 8.2 breadth inputs 24-29 from NYSE advances/declines/unchanged and up/down/unchanged volume.
pub const BREADTH: &[&str] = &["NYSE_ADV_DEC.Advances", "NYSE_ADV_DEC.Declines", "NYSE_ADV_DEC.Unchanged", "NYSE_ADV_DEC.AdvVolume", "NYSE_ADV_DEC.DecVolume", "NYSE_ADV_DEC.UnchVolume"];
pub fn breadth(id: &str) -> bool { other(id) || matches!(id, "arebta_tt_br" | "arebta_ti_br" | "arebta_e_br" | "arebta_d_br") || nse(id) }
/// Lane CV6 (R4 #365): the NSE twins read the NSE advances / declines / unchanged and up / down volume (extra_data/india NSE_ADV_DEC).
pub fn nse(id: &str) -> bool { id.ends_with("_br_nse") }
pub const BREADTH_NSE: &[&str] = &["NSE_ADV_DEC.Advances", "NSE_ADV_DEC.Declines", "NSE_ADV_DEC.Unchanged", "NSE_ADV_DEC.AdvVolume", "NSE_ADV_DEC.DecVolume"];
const BR_NOTE: &str = "Lane K78 (R4 #276 (3), #287, #288). Input = NYSE breadth (extra_data NYSE_ADV_DEC, all NYSE issues, 1965-03-01..2020-02-10, as the book's 'all issues traded on the NYSE', p.413); traded = the instrument's own next open (the book traded the S&P 500; other US indices, ETFs and stocks are a declared application). US only (no NYSE breadth for Indian instruments). Runs go to the end of the breadth data (2020-02-10); the book's test period 1 Nov 1980 - 1 Jul 2005 (p.393) is used only for the book check (R4 #288 (1)). Up/down/unchanged volume switch together from thousands to shares on 2002-06-21 (the source lacks 2002-06-20), so the volume RATIO (inputs 27-29) has no unit break; volume levels are not used. [Declared] A day without a breadth print is skipped by the indicator (sums, averages and channels count breadth days only) and the position is unchanged; crossing = previous value on/at one side, today strictly beyond; a flat channel = no value. No detrending (the book scores detrended returns, p.391-392); one unit per trade.";
const INPUTS: &str = "Input (Table 8.2, p.418): 24 CADR, 25 ADR10, 26 ADR30, 27 CUDR (cum. up-down volume ratio), 28 UDR10, 29 UDR30";

/// Lane H3b: one card per other Table 8.2 input whose series is held and whose rule Table 9.1 (p.445-447) prints.
pub fn other(id: &str) -> bool { OTHER_CARDS.iter().any(|c| c.0 == id) }
/// (id, rule kind = the br card whose rule runs, input number, name, series the card needs)
static OTHER_CARDS: [(&str, &str, usize, &str, &[&str]); 6] = [
    ("arebta_d_djta", "arebta_d_br", 4, "Divergence rule D-8-4-10-60: Dow Jones Transportation Average vs the S&P 500", &["DJ_TRANSPORT"]),
    ("arebta_d_hlr10", "arebta_d_br", 31, "Divergence rule D-8-31-10-30: NYSE new highs-new lows ratio, 10-day MA, vs the S&P 500", &["NYSE_NEW_HIGHS", "NYSE_NEW_LOWS", "NYSE_ADV_DEC.Advances", "NYSE_ADV_DEC.Declines", "NYSE_ADV_DEC.Unchanged"]),
    ("arebta_d_hlr30", "arebta_d_br", 32, "Divergence rule D-8-32-10-15: NYSE new highs-new lows ratio, 30-day MA, vs the S&P 500", &["NYSE_NEW_HIGHS", "NYSE_NEW_LOWS", "NYSE_ADV_DEC.Advances", "NYSE_ADV_DEC.Declines", "NYSE_ADV_DEC.Unchanged"]),
    ("arebta_d_pt10y", "arebta_d_br", 36, "Divergence rule D-7-36-10-30: price of the 10-year Treasury (100 / yield) vs the S&P 500", &["DGS10"]),
    ("arebta_e_durspd", "arebta_e_br", 39, "Extreme-value & transition rule E-11-39-10-60: duration spread (10-year minus 3-month yield)", &["DGS10", "DTB3"]),
    ("arebta_tt_pt10y", "arebta_tt_br", 36, "Traditional trend rule TT-36-5: price of the 10-year Treasury (100 / yield)", &["DGS10"]),
];
fn other_card(id: &str) -> Option<&'static (&'static str, &'static str, usize, &'static str, &'static [&'static str])> { OTHER_CARDS.iter().find(|c| c.0 == id) }
/// Input series of an `other` card on the days (indices into the context) where every needed value and the traded close exist.
fn other_input(id: &str, x: &Ctx, lag: usize) -> Option<(Vec<usize>, Vec<f64>)> {
    let c = other_card(id)?;
    let n = x.c.len();
    let raw: Vec<&[f64]> = c.4.iter().map(|k| x.series(k)).collect::<Option<_>>()?;
    // R4 #372: a FRED H.15 rate print for day t is published on t+1 in the afternoon, so the rate inputs (36, 39) read the previous trading bar's print
    // by default (lag 1); lag 0 is the labelled same-day look-ahead option. Other inputs are end-of-day index / breadth series known at the close.
    let shifted: Vec<Vec<f64>>;
    let cols: Vec<&[f64]> = if matches!(c.2, 36 | 39) && lag > 0 {
        shifted = raw.iter().map(|v| (0..n).map(|i| if i >= lag { v[i - lag] } else { f64::NAN }).collect()).collect();
        shifted.iter().map(|v| v.as_slice()).collect()
    } else { raw };
    let ok = |i: usize| x.c[i].is_finite() && cols.iter().all(|v| v[i].is_finite());
    let mut days: Vec<usize> = (0..n).filter(|&i| ok(i)).collect();
    let ma = |v: &[f64], w: usize| -> Vec<f64> { (0..v.len()).map(|j| if j + 1 < w { f64::NAN } else { v[j + 1 - w..=j].iter().sum::<f64>() / w as f64 }).collect() };
    let s: Vec<f64> = match c.2 {
        4 => days.iter().map(|&i| cols[0][i]).collect(),
        31 | 32 => {
            days.retain(|&i| cols[2][i] + cols[3][i] + cols[4][i] > 0.);
            let hlr: Vec<f64> = days.iter().map(|&i| (cols[0][i] - cols[1][i]) / (cols[2][i] + cols[3][i] + cols[4][i])).collect();
            ma(&hlr, if c.2 == 31 { 10 } else { 30 })
        }
        36 => { days.retain(|&i| cols[0][i] > 0.); days.iter().map(|&i| 100. / cols[0][i]).collect() }
        39 => days.iter().map(|&i| cols[0][i] - cols[1][i]).collect(),
        _ => return None,
    };
    Some((days, s))
}
fn br(d: StrategyDef) -> StrategyDef { StrategyDef { needs_series: BREADTH, ..d } }
const NSE_NOTE: &str = "[R4 #340 / #347 declared adaptation, UNVERIFIED] NSE twin of the NYSE-breadth rule: the same rule, parameters and thresholds on the NSE's own advances / declines / unchanged and up / down volume (extra_data/india NSE_ADV_DEC, 1999-01-01 on; all NSE-listed issues as the book's 'all issues traded on the NYSE'). The NSE publishes no unchanged-issue volume, so the volume ratio (inputs 27-29) is (up - down) / (up + down) where the book divides by up + down + unchanged volume. The divergence rule (D) compares with the NIFTY 50 close on the same date where the NYSE rule uses the SPX close (the NIFTY 50's own close on NIFTY 50). Runs on Indian indices and stocks; US instruments are refused (no NSE series there). The book traded the S&P 500 on NYSE breadth: no NSE result exists. Measured firing rates, NSE vs NYSE: ";
fn twin(d: &StrategyDef, id: &'static str, name: &'static str, rate: &'static str) -> StrategyDef {
    StrategyDef { id, name, group: d.group, session: d.session, dir: d.dir, status: d.status, needs_volume: d.needs_volume, needs_vix: d.needs_vix, book: d.book, rules: d.rules,
        needs_series: BREADTH_NSE, notes: Box::leak(format!("{NSE_NOTE}{rate}").into_boxed_str()), claim: None }
}

pub fn catalogue() -> Vec<StrategyDef> {
    let d = |id, name, dir, status, rules, notes, claim| StrategyDef { id, name, group: "Ch.8 S&P 500 rule data-mining case study", session: 0, dir, status, rules, notes,
        needs_volume: false, needs_vix: false, needs_series: &[], book: "arebta", claim };
    let v = vec![
        d("arebta_tt", "Traditional trend rule TT-1-n (channel breakout of the close)", "Long/Short", Status::Live,
          &["F|CBO(n): upper = max of the series over the past n periods, lower = min, \"not including the current period\" (p.397)",
            "E|\"signals long positions when the analyzed series exceeds its maximum value established over the past n-periods ... short positions are signaled when the time series falls below its n-period minimum\" (p.397); otherwise the output is unchanged",
            "E|\"A traditional version of a trend rule produced an output value of +1 (long position in the S&P 500) when the input time series was determined to be in an upward trend ... -1 (short)\" (p.420)",
            "X|Reversal only; trades at the next open (p.449). n tested: 3, 5, 8, 12, 18, 27, 41, 61, 91, 137, 205 (p.420)"],
          NOTE, None),
        d("arebta_ti", "Inverse trend rule TI-1-n", "Long/Short", Status::Live,
          &["E|\"Inverse trend rules simply produced the opposite signals (e.g., short S&P 500 when the analyzed series was determined to be in an uptrend)\" (p.420)",
            "F|Same CBO(n) as TT (p.397), n in {3, 5, 8, 12, 18, 27, 41, 61, 91, 137, 205}", "X|Reversal only; next open (p.449)"],
          NOTE, None),
        d("arebta_e", "Extreme-value & transition rule E-type-1-disp-span", "Long/Short", Status::Live,
          &["F|Indicator = CN(LMA(series, 4), N): 4-day linearly weighted MA (weights 4,3,2,1 / 10, p.401) then channel normalisation CN = (S - min_N)/(max_N - min_N) x 100 over the last N days (p.402, 421)",
            "F|Thresholds 50 +/- displacement (10 or 20); N in {15, 30, 60} (p.429)",
            "E|Events: 1 lower threshold crossed downward, 2 lower crossed upward, 3 upper crossed upward, 4 upper crossed downward (p.422)",
            "E|Type table (long entry/short exit ; short entry/long exit): 1:1;2 2:1;3 3:1;4 4:2;3 5:2;4 6:3;4 7:2;1 8:3;1 9:4;1 10:3;2 11:4;2 12:4;3 (Table 8.3, p.423)",
            "X|Reversal only; next open (p.449). Default E-2-1-20-30 (the series-1 rule printed in Table 9.1)"],
          "Crossing = previous value on/at one side, today strictly beyond (equality convention not stated by the book). CN undefined (flat window) = no event. Same timing/detrending notes as the trend rules.",
          Some(crate::connors::Claim { trades: "-", win: "-", avg: "7.09%/yr mean return on detrended S&P 500 (rule E-2-1-20-30)", hold: "-", period: "11/1/1980 - 7/1/2005, S&P 500 (Table 9.1 p.445; not significant after data-mining correction)" })),
        d("arebta_other_inputs", "TT/TI, E and divergence rules on the other 38 input series", "Long/Short", Status::NeedsData(OTHER),
          &["E|Same TT/TI (p.420), E (p.421-430) rules on inputs 3-40", "E|Divergence D-type-series-disp-span: CN{CN(series, n) - CN(S&P 500, n), 10n} with the 12 threshold types of Table 8.4 (p.433-440)"],
          "Best rule overall E-12-28-10-30 (input 28 = up-down volume ratio 10-day MA, Table 8.2 p.418; coded as arebta_e_br): 10.25%/yr detrended, p = 0.8164 (WRC), not significant (p.441).", None),
        br(d("arebta_tt_br", "Traditional trend rule TT on NYSE breadth (inputs 24-29)", "Long/Short", Status::Live,
          &["F|Breadth inputs (p.413-415): ADR = (advancing - declining) / (advancing + declining + unchanged NYSE issues); NVR = (up volume - down volume) / (up + down + unchanged volume); CADR / CUDR = cumulative sums; ADR10/ADR30, UDR10/UDR30 = 10- and 30-day moving averages (Table 8.2 inputs 24-29, p.418)",
            "E|CBO(input, n): +1 (long the S&P 500) when the input exceeds its maximum of the past n days, not including today; -1 below the n-day minimum (p.397, 420)",
            "X|Reversal only; next open (p.449). Default TT-28-12 = the mirror of TI-28-12, the best printed trend rule on these inputs (Table 9.1 p.445)"],
          BR_NOTE, None)),
        br(d("arebta_ti_br", "Inverse trend rule TI on NYSE breadth (inputs 24-29)", "Long/Short", Status::Live,
          &["F|Same breadth inputs and CBO(n) as arebta_tt_br (p.397, 413-415, 418)",
            "E|\"Inverse trend rules simply produced the opposite signals\" (p.420): short when the input breaks out up, long when it breaks down",
            "X|Reversal only; next open (p.449). Default TI-28-12 (Table 9.1 p.445: 8.49%/yr detrended)"],
          BR_NOTE, Some(crate::connors::Claim { trades: "-", win: "-", avg: "8.49%/yr mean return on detrended S&P 500 (TI-28-12)", hold: "-", period: "11/1/1980 - 7/1/2005, S&P 500 (Table 9.1 p.445; not significant after data-mining correction)" }))),
        br(d("arebta_e_br", "Extreme-value & transition rule E on NYSE breadth (inputs 24-29)", "Long/Short", Status::Live,
          &["F|Indicator = CN(LMA(input, 4), N), the arebta_e operators on a breadth input (p.401-402, 421)",
            "E|Thresholds 50 +/- 10 or 20; N 15/30/60; 12 types of Table 8.3 (p.422-423, 429)",
            "X|Reversal only; next open (p.449). Default E-12-28-10-30, the best of all 6,402 rules (p.441; Table 9.1 p.445: 10.25%/yr detrended)"],
          BR_NOTE, Some(crate::connors::Claim { trades: "-", win: "-", avg: "10.25%/yr mean return on detrended S&P 500 (E-12-28-10-30)", hold: "-", period: "11/1/1980 - 7/1/2005, S&P 500 (Table 9.1 p.445; p = 0.8164, not significant, p.441)" }))),
        br(d("arebta_d_br", "Divergence rule D: NYSE breadth vs the traded index", "Long/Short", Status::Live,
          &["F|Divergence indicator = CN{CN(companion, n) - CN(S&P 500, n), 10n} (double channel normalisation, p.437-438); companion = a breadth input 24-29; the S&P 500 = the SPX close on the same date (default, every US instrument; R4 #288 (2)); p.ref = 1 (labelled option): the traded instrument's own close",
            "E|Upper/lower thresholds 50 +/- 10 or 20; n 15/30/60; the 12 types of Table 8.4 (p.440) = the event pairs of Table 8.3 (p.439)",
            "X|Reversal only; next open (p.449). Default D-10-28-10-30, the best printed divergence rule on these inputs (Table 9.1 p.445: 8.69%/yr detrended)"],
          BR_NOTE, Some(crate::connors::Claim { trades: "-", win: "-", avg: "8.69%/yr mean return on detrended S&P 500 (D-10-28-10-30)", hold: "-", period: "11/1/1980 - 7/1/2005, S&P 500 (Table 9.1 p.445; not significant after data-mining correction)" }))),
    ];
    let mut v = v;
    for (base, id, name, rate) in NSE_TWINS {
        let t = v.iter().find(|d| d.id == base).map(|b| twin(b, id, name, rate));
        v.extend(t);
    }
    for c in OTHER_CARDS.iter() { v.push(other_def(c)); }
    v
}

const OTHER_NOTE: &str = "Lane H3b (19 Nov 2026, R4 #339/#343/#346). One card for one Table 8.2 input whose series the vault holds and whose rule Table 9.1 (p.445-447) prints; the printed rule ID is the default (Type-Input-Disp-Span for D and E, Input-Span for TT), the other grid values stay options. Declared readings, 3 unstated details and no more: (1) a day on which the input (or any of its parts) has no print is skipped, the sums / averages / channels count days with prints only and the position is unchanged; (2) a crossing is 'previous value on or at one side, today strictly beyond' (the book states no equality convention); (3) the rule trades the instrument's own next open, the S&P 500 only as the divergence reference (SPX close on the same date; the book traded the S&P 500; other US indices, ETFs and stocks are a declared application). Printed, not choices: the input definition, CN, the 4-day LMA of the E rule, the double normalisation 10n of the D rule, the 12 event-type tables, reversal only, next open. No detrending (the book scores detrended returns, p.391-392); one unit per trade; no costs. US only: the series are US series, so Indian instruments get the missing-series message. UNVERIFIED: the book prints return, p-values and confidence limits for the S&P 500 on detrended returns, no trade list. ";
/// Per-card declared substitutions and data coverage (R4 #340, #372).
fn other_label(id: &str) -> &'static str {
    match id {
        "arebta_d_pt10y" | "arebta_tt_pt10y" => "[R4 #340 declared adaptation] FRED DGS10 (10-year Treasury constant maturity, percent) stands in for the book's 10-year Treasury yield (p.417); the price is 100 / yield. [R4 #372] FRED H.15 publishes day t on t+1 in the afternoon, so the rule reads the PREVIOUS bar's print (parameter lag = 1, default); lag = 0 is the same-day reading, a LOOK-AHEAD option shown only for comparison. ",
        "arebta_e_durspd" => "[R4 #340 declared adaptation] FRED DGS10 (10-year constant maturity) and DTB3 (3-month Treasury bill, SECONDARY MARKET RATE on a DISCOUNT basis) stand in for the book's 10-year and 3-month yields (p.417); the spread is DGS10 - DTB3 in percentage points. [R4 #372] H.15 publishes day t on t+1 in the afternoon, so both rates are read one bar late (parameter lag = 1, default); lag = 0 is a LOOK-AHEAD option. ",
        "arebta_d_hlr10" | "arebta_d_hlr30" => "[Coverage] The ratio needs NYSE advancing / declining / unchanged issues (extra_data NYSE_ADV_DEC, 1965-03-01 to 2020-02-10) and the Norgate NYSE 52-week highs / lows (#NYSEHI / #NYSELO, 1932-08-03 to 2020-09-04): the card runs from 1965 and ends 2020-02-10 (auto-end), the book's window ended 2005-07-01. ",
        "arebta_d_djta" => "[Coverage] Norgate $DJT close (1896-10-26 to 2020-09-04, SPX close on the same date from us_data.csv): the card ends 2020-09-04 (auto-end). ",
        _ => "",
    }
}
fn other_def(c: &'static (&'static str, &'static str, usize, &'static str, &'static [&'static str])) -> StrategyDef {
    let (id, kind, inp, name, series) = *c;
    let (rules, def_in, claim): (&'static [&'static str], &str, &str) = match id {
        "arebta_d_djta" => (&["F|Input 4 = Dow Jones Transportation Index close ($DJT, Table 8.2 p.418; raw series)", "F|Divergence indicator = CN{CN(DJTA, n) - CN(S&P 500, n), 10n} (p.437-438)", "E|D-8-4-10-60: type 8 (Table 8.4: long on event 3, short on event 1 of Table 8.3, p.439-440), thresholds 50 +/- 10, n = 60", "X|Reversal only; next open (p.449)"], "D-8-4-10-60", "Table 9.1 p.445 (2nd best of the 100): return 0.102395 a year on detrended S&P 500 returns, SR bootstrap p 0.001, Monte Carlo p 0.0015; siblings D-9-4-10-60 and D-10-4-10-60 on p.446"),
        "arebta_d_hlr10" => (&["F|Input 31 = HLR10 = 10-day MA of HLR = (NYSE new 52-week highs - new lows) / (advancing + declining + unchanged issues) (p.416; Norgate #NYSEHI/#NYSELO and NYSE_ADV_DEC)", "F|Divergence indicator = CN{CN(HLR10, n) - CN(S&P 500, n), 10n} (p.437-438)", "E|D-8-31-10-30: type 8, thresholds 50 +/- 10, n = 30", "X|Reversal only; next open (p.449)"], "D-8-31-10-30", "Table 9.1 p.446: return 0.06831 a year on detrended S&P 500 returns, bootstrap p 0.019, Monte Carlo p 0.023"),
        "arebta_d_hlr30" => (&["F|Input 32 = HLR30 = 30-day MA of HLR (p.416)", "F|Divergence indicator = CN{CN(HLR30, n) - CN(S&P 500, n), 10n} (p.437-438)", "E|D-8-32-10-15: type 8, thresholds 50 +/- 10, n = 15", "X|Reversal only; next open (p.449)"], "D-8-32-10-15", "Table 9.1 p.445: return 0.08048 a year on detrended S&P 500 returns, bootstrap p 0.006, Monte Carlo p 0.0085; siblings D-10-32-20-15, D-9-32-10-15 and TI-32-61 on p.446"),
        "arebta_d_pt10y" => (&["F|Input 36 = price of the 10-year Treasury = 100 / the 10-year yield (p.417: 'a rate of 6.05 percent would be equivalent to a price of 15.38'; FRED DGS10)", "F|Divergence indicator = CN{CN(PT10Y, n) - CN(S&P 500, n), 10n} (p.437-438)", "E|D-7-36-10-30: type 7, thresholds 50 +/- 10, n = 30", "X|Reversal only; next open (p.449)"], "D-7-36-10-30", "Table 9.1 p.445: return 0.079943 a year on detrended S&P 500 returns, bootstrap p 0.0065, Monte Carlo p 0.0055; siblings D-7-36-20-30 and D-7-36-10-60 on p.445"),
        "arebta_e_durspd" => (&["F|Input 39 = duration spread = the 10-year yield minus the 3-month yield (p.417; FRED DGS10, DTB3)", "F|Indicator = CN(LMA(DURSPD, 4), N) (p.401-402, 421)", "E|E-11-39-10-60: event type 11 of Table 8.3 (p.423), thresholds 50 +/- 10, N = 60", "X|Reversal only; next open (p.449)"], "E-11-39-10-60", "Table 9.1 p.445: return 0.092282 a year on detrended S&P 500 returns, bootstrap p 0.001, Monte Carlo p 0.0015"),
        _ => (&["F|Input 36 = price of the 10-year Treasury = 100 / the 10-year yield (p.417)", "E|TT-36-5: CBO(PT10Y, 5): +1 above its 5-day maximum (not including today), -1 below its minimum (p.397, 420)", "X|Reversal only; next open (p.449)"], "TT-36-5", "Table 9.1 p.446: return 0.070709 a year on detrended S&P 500 returns, bootstrap p 0.016, Monte Carlo p 0.0235; sibling TT-36-3 on p.446"),
    };
    let _ = (kind, inp);
    StrategyDef { id, name, group: "Ch.8 S&P 500 rule data-mining case study", session: 0, dir: "Long/Short", status: Status::Live, needs_volume: false, needs_vix: false, needs_series: series, book: "arebta", rules,
        notes: Box::leak(format!("{OTHER_NOTE}{}Default {def_in}. Book: {claim}.", other_label(id)).into_boxed_str()),
        claim: Some(Claim { trades: "-", win: "-", avg: "see the printed Table 9.1 row (detrended S&P 500 returns, not a trade list)", hold: "-", period: "11/1/1980 - 7/1/2005, S&P 500 (Table 9.1 p.445-447; not significant after data-mining correction)" }) }
}
const NSE_TWINS: [(&str, &str, &str, &str); 4] = [
    ("arebta_tt_br", "arebta_tt_br_nse", "Traditional trend rule TT on NSE breadth (inputs 24-29)", "default inputs (input 28, n = 12) over 2014-01-02..2019-08-19, the stretch the NYSE series runs on: NSE 16.9 position reversals a year, long 49.2% of days; NYSE (SPX) 18.5 a year, long 47.1%. Full NSE overlap 1999-11-09..2020-02-10: 16.9 a year, long 48.6%."),
    ("arebta_ti_br", "arebta_ti_br_nse", "Inverse trend rule TI on NSE breadth (inputs 24-29)", "default inputs (input 28, n = 12) over 2014-01-02..2019-08-19, the stretch the NYSE series runs on: NSE 16.9 position reversals a year, long 50.6% of days; NYSE (SPX) 18.5 a year, long 52.8%. Full NSE overlap 1999-11-09..2020-02-10: 16.9 a year, long 51.4%."),
    ("arebta_e_br", "arebta_e_br_nse", "Extreme-value & transition rule E on NSE breadth (inputs 24-29)", "default inputs (E-12-28-10-30) over 2014-01-02..2019-08-19, the stretch the NYSE series runs on: NSE 17.8 position reversals a year, long 56.4% of days; NYSE (SPX) 21.3 a year, long 59.0%. Full NSE overlap 1999-11-09..2020-02-10: 17.7 a year, long 55.6%."),
    ("arebta_d_br", "arebta_d_br_nse", "Divergence rule D: NSE breadth vs the NIFTY 50", "default inputs (D-10-28-10-30) over 2014-01-02..2019-08-19, the stretch the NYSE series runs on: NSE 9.6 position reversals a year, long 62.3% of days; NYSE (SPX) 10.1 a year, long 54.0%. Full NSE overlap 1999-11-09..2020-02-10: 9.1 a year, long 57.3%."),
];

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    Some(match id {
        "arebta_tt" => ("arebta_trend", "Aronson trend rules (series 1)", "TT · traditional"),
        "arebta_ti" => ("arebta_trend", "Aronson trend rules (series 1)", "TI · inverse"),
        "arebta_tt_br" => ("arebta_trend_br", "Aronson trend rules on NYSE breadth", "TT · traditional"),
        "arebta_ti_br" => ("arebta_trend_br", "Aronson trend rules on NYSE breadth", "TI · inverse"),
        "arebta_tt_br_nse" => ("arebta_trend_br_nse", "Aronson trend rules on NSE breadth", "TT · traditional"),
        "arebta_ti_br_nse" => ("arebta_trend_br_nse", "Aronson trend rules on NSE breadth", "TI · inverse"),
        _ => return catalogue().into_iter().find(|d| d.id == id).map(|d| (d.id, d.name, "")),
    })
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    Some(match id {
        "arebta_tt" | "arebta_ti" => vec![p("n", "CBO look-back (book grid 3,5,8,12,18,27,41,61,91,137,205)", 3., 3., 205., 1.)],
        "arebta_e" => vec![p("type", "E-rule type 1-12 (Table 8.3)", 2., 1., 12., 1.), p("disp", "Threshold displacement from 50 (book: 10 or 20)", 20., 10., 20., 10.), p("span", "CN look-back N (book: 15, 30, 60)", 30., 15., 60., 15.)],
        "arebta_tt_br" | "arebta_ti_br" | "arebta_tt_br_nse" | "arebta_ti_br_nse" => vec![p("input", INPUTS, 28., 24., 29., 1.), p("n", "CBO look-back (book grid 3,5,8,12,18,27,41,61,91,137,205)", 12., 3., 205., 1.)],
        "arebta_e_br" | "arebta_e_br_nse" => vec![p("input", INPUTS, 28., 24., 29., 1.), p("type", "E-rule type 1-12 (Table 8.3)", 12., 1., 12., 1.), p("disp", "Threshold displacement from 50 (book: 10 or 20)", 10., 10., 20., 10.), p("span", "CN look-back N (book: 15, 30, 60)", 30., 15., 60., 15.)],
        "arebta_d_br" | "arebta_d_br_nse" => vec![p("input", INPUTS, 28., 24., 29., 1.), p("type", "D-rule type 1-12 (Table 8.4)", 10., 1., 12., 1.), p("disp", "Threshold displacement from 50 (book: 10 or 20)", 10., 10., 20., 10.), p("span", "First CN look-back n (book: 15, 30, 60); the second is 10n", 30., 15., 60., 15.), p("ref", "S&P 500 series: 0 = the SPX close (the book's, default); 1 = the traded instrument's own close (labelled option)", 0., 0., 1., 1.)],
        "arebta_d_djta" | "arebta_d_hlr10" | "arebta_d_hlr30" | "arebta_d_pt10y" => { let (ty, di, sp) = match id { "arebta_d_djta" => (8., 10., 60.), "arebta_d_hlr10" => (8., 10., 30.), "arebta_d_hlr30" => (8., 10., 15.), _ => (7., 10., 30.) };
            vec![p("type", "D-rule type 1-12 (Table 8.4)", ty, 1., 12., 1.), p("disp", "Threshold displacement from 50 (book: 10 or 20)", di, 10., 20., 10.), p("span", "First CN look-back n (book: 15, 30, 60); the second is 10n", sp, 15., 60., 15.), p("ref", "S&P 500 series: 0 = the SPX close (the book's, default); 1 = the traded instrument's own close", 0., 0., 1., 1.), p("lag", "Rate print lag in trading bars: 1 = the previous bar's H.15 print (default, R4 #372); 0 = same-day print = LOOK-AHEAD option (FRED publishes day t on t+1)", 1., 0., 1., 1.)] }
        "arebta_e_durspd" => vec![p("type", "E-rule type 1-12 (Table 8.3)", 11., 1., 12., 1.), p("disp", "Threshold displacement from 50 (book: 10 or 20)", 10., 10., 20., 10.), p("span", "CN look-back N (book: 15, 30, 60)", 60., 15., 60., 15.), p("lag", "Rate print lag in trading bars: 1 = the previous bar's H.15 print (default, R4 #372); 0 = same-day print = LOOK-AHEAD option", 1., 0., 1., 1.)],
        "arebta_tt_pt10y" => vec![p("n", "CBO look-back (book grid 3,5,8,12,18,27,41,61,91,137,205)", 5., 3., 205., 1.), p("lag", "Rate print lag in trading bars: 1 = the previous bar's H.15 print (default, R4 #372); 0 = same-day print = LOOK-AHEAD option", 1., 0., 1., 1.)],
        _ => return None,
    })
}

const T83: [(u8, u8); 12] = [(1, 2), (1, 3), (1, 4), (2, 3), (2, 4), (3, 4), (2, 1), (3, 1), (4, 1), (3, 2), (4, 2), (4, 3)];

/// CN(s, n) = (s - min) / (max - min) x 100 over the last n values including today (p.402); NaN if any is missing or flat.
fn cn(s: &[f64], n: usize) -> Vec<f64> {
    (0..s.len()).map(|k| {
        if k + 1 < n { return f64::NAN; }
        let w = &s[k + 1 - n..=k];
        if w.iter().any(|v| !v.is_finite()) { return f64::NAN; }
        let (mn, mx) = (w.iter().copied().fold(f64::MAX, f64::min), w.iter().copied().fold(f64::MIN, f64::max));
        if mx > mn { (s[k] - mn) / (mx - mn) * 100. } else { f64::NAN }
    }).collect()
}

/// Lane K78: Table 8.2 input 24-29 on the breadth days (index list into the context) of this context.
fn breadth_input(x: &Ctx, input: usize, nse: bool) -> Option<(Vec<usize>, Vec<f64>)> {
    let vol = input >= 27;
    let names: &[&str] = if nse { BREADTH_NSE } else { BREADTH };
    let col = |k: usize| x.series(names[k + if vol { 3 } else { 0 }]);
    let zero = vec![0.; x.c.len()];
    let (a, d) = (col(0)?, col(1)?);
    let u: &[f64] = if nse && vol { &zero } else { col(2)? }; // NSE: no unchanged-issue volume (twin note)
    let days: Vec<usize> = (0..x.c.len()).filter(|&i| a[i].is_finite() && d[i].is_finite() && u[i].is_finite() && a[i] + d[i] + u[i] > 0. && x.c[i].is_finite()).collect();
    let ratio: Vec<f64> = days.iter().map(|&i| (a[i] - d[i]) / (a[i] + d[i] + u[i])).collect();
    let s = match (input - 24) % 3 {
        0 => ratio.iter().scan(0., |acc, r| { *acc += r; Some(*acc) }).collect(),
        k => { let n = if k == 1 { 10 } else { 30 }; (0..ratio.len()).map(|j| if j + 1 < n { f64::NAN } else { ratio[j + 1 - n..=j].iter().sum::<f64>() / n as f64 }).collect() }
    };
    Some((days, s))
}

/// Lane K78: TT/TI, E and D rules on a breadth input, computed on the breadth days (D: those with an SPX close too); the
/// state is carried over the other days.
fn output_br(id: &str, x: &Ctx, g: &dyn Fn(&str) -> f64) -> Vec<f64> {
    let twin_nse = nse(id);
    let card = id;
    let id = other_card(id).map_or(id.trim_end_matches("_nse"), |c| c.1); // lane H3b: the other-input cards run their br rule
    let mut out = vec![0.; x.c.len()];
    let inp = if other(card) { other_input(card, x, if matches!(other_card(card).map(|c| c.2), Some(36 | 39)) { g("lag") as usize } else { 0 }) } else { breadth_input(x, (g("input") as usize).clamp(24, 29), twin_nse) };
    let Some((mut days, mut s)) = inp else { return out };
    // R4 #288 (2): the divergence's S&P 500 = the SPX close on the same date (FOLLOWER_CLOSE, exact join); p.ref = 1: own close
    // the NSE twin: the NIFTY 50 close on the same date (BENCH.C; on NIFTY 50 itself its own close), as the NYSE rule uses the SPX close
    let own_nifty: Vec<f64>;
    let spx = if id == "arebta_d_br" && g("ref") < 0.5 {
        if twin_nse {
            match x.series("BENCH.C") { Some(v) => Some(v), None if x.r.first().is_some_and(|r| r.ticker == "NIFTY 50") => { own_nifty = x.c.clone(); Some(&own_nifty[..]) } None => return out }
        } else { match x.series("FOLLOWER_CLOSE") { Some(v) => Some(v), None => return out } }
    } else { None };
    if let Some(v) = spx {
        let keep: Vec<usize> = (0..days.len()).filter(|&k| v[days[k]].is_finite()).collect();
        s = keep.iter().map(|&k| s[k]).collect();
        days = keep.iter().map(|&k| days[k]).collect();
    }
    let m = days.len();
    let mut sig = vec![0.; m];
    let mut st = 0.;
    if id == "arebta_tt_br" || id == "arebta_ti_br" {
        let n = g("n").max(1.) as usize;
        let inv = if id == "arebta_ti_br" { -1. } else { 1. };
        for k in 0..m {
            if k >= n && s[k - n..=k].iter().all(|v| v.is_finite()) {
                let w = &s[k - n..k];
                if s[k] > w.iter().copied().fold(f64::MIN, f64::max) { st = inv; } else if s[k] < w.iter().copied().fold(f64::MAX, f64::min) { st = -inv; }
            }
            sig[k] = st;
        }
    } else {
        let (ty, disp, span) = (g("type") as usize, g("disp"), g("span") as usize);
        let ind = if id == "arebta_e_br" {
            let lma: Vec<f64> = (0..m).map(|k| if k < 3 { f64::NAN } else { (4. * s[k] + 3. * s[k - 1] + 2. * s[k - 2] + s[k - 3]) / 10. }).collect();
            cn(&lma, span)
        } else {
            let px: Vec<f64> = days.iter().map(|&i| spx.map_or(x.c[i], |v| v[i])).collect();
            let (a, b) = (cn(&s, span), cn(&px, span));
            cn(&a.iter().zip(&b).map(|(p, q)| p - q).collect::<Vec<_>>(), 10 * span)
        };
        let (le, se) = T83[ty.clamp(1, 12) - 1];
        let (lo, up) = (50. - disp, 50. + disp);
        let ev = |e: u8, a: f64, b: f64| match e { 1 => a >= lo && b < lo, 2 => a <= lo && b > lo, 3 => a <= up && b > up, _ => a >= up && b < up };
        for k in 1..m {
            let (a, b) = (ind[k - 1], ind[k]);
            if a.is_finite() && b.is_finite() {
                if ev(le, a, b) { st = 1.; } else if ev(se, a, b) { st = -1.; }
            }
            sig[k] = st;
        }
    }
    let mut cur = 0.;
    let mut k = 0;
    for i in 0..out.len() {
        if k < m && days[k] == i { cur = sig[k]; k += 1; }
        out[i] = cur;
    }
    out
}

/// Rule output +1/-1 (0 = not yet defined) at each close.
fn output(id: &str, x: &Ctx, g: &dyn Fn(&str) -> f64) -> Vec<f64> {
    if breadth(id) { return output_br(id, x, g); }
    let c = &x.c;
    let mut out = vec![0.; c.len()];
    let mut s = 0.;
    if id == "arebta_e" {
        let (ty, disp, span) = (g("type") as usize, g("disp"), g("span") as usize);
        let (le, se) = T83[ty.clamp(1, 12) - 1];
        let (lo, up) = (50. - disp, 50. + disp);
        let lma: Vec<f64> = (0..c.len()).map(|i| if i < 3 { f64::NAN } else { (4. * c[i] + 3. * c[i - 1] + 2. * c[i - 2] + c[i - 3]) / 10. }).collect();
        let cn: Vec<f64> = (0..c.len()).map(|i| {
            if i + 1 < span + 3 { return f64::NAN; }
            let w = &lma[i + 1 - span..=i];
            let (mn, mx) = (w.iter().copied().fold(f64::MAX, f64::min), w.iter().copied().fold(f64::MIN, f64::max));
            if mx > mn { (lma[i] - mn) / (mx - mn) * 100. } else { f64::NAN }
        }).collect();
        let ev = |e: u8, a: f64, b: f64| match e { 1 => a >= lo && b < lo, 2 => a <= lo && b > lo, 3 => a <= up && b > up, _ => a >= up && b < up };
        for i in 1..c.len() {
            let (a, b) = (cn[i - 1], cn[i]);
            if a.is_finite() && b.is_finite() {
                if ev(le, a, b) { s = 1.; } else if ev(se, a, b) { s = -1.; }
            }
            out[i] = s;
        }
    } else {
        let n = g("n").max(1.) as usize;
        let inv = if id == "arebta_ti" { -1. } else { 1. };
        for i in n..c.len() {
            let w = &c[i - n..i];
            if c[i] > w.iter().copied().fold(f64::MIN, f64::max) { s = inv; } else if c[i] < w.iter().copied().fold(f64::MAX, f64::min) { s = -inv; }
            out[i] = s;
        }
    }
    out
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    let schema = params(id)?;
    if from > to || to >= x.c.len() { return None; }
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).unwrap().def);
    let sig = output(id, x, &g);
    let (mut out, mut cur): (Vec<Trade>, Option<Trade>) = (vec![], None);
    for i in from..to {
        let want = sig[i];
        if want == 0. || cur.as_ref().is_some_and(|t| t.dir == want) { continue; }
        let px = x.o[i + 1];
        if let Some(mut t) = cur.take() { t.exit_i = i + 1; t.exit_px = px; t.signal_i = i; t.reason = "Rule output reversed · next open".into(); out.push(t); }
        cur = Some(Trade { dir: want, legs: vec![Leg { i: i + 1, px, w: 1. }], exit_i: i + 1, exit_px: px, reason: String::new(), entry_kind: "open", signal_i: i, open: false, exits: vec![] });
    }
    if let Some(mut t) = cur { t.exit_i = to; t.exit_px = x.c[to]; t.signal_i = to; t.reason = "open".into(); t.open = true; out.push(t); }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;
    #[test]
    fn tt3_reverses_on_next_open() {
        let cl = [10., 11., 12., 13., 12.5, 12., 9., 9.5];
        let r: Vec<Row> = cl.iter().enumerate().map(|(i, &c)| Row { ticker: "T".into(), date: format!("{:04}-01-01", 2000 + i), open: c + 0.1, high: c + 1., low: c - 1., close: c, volume: None }).collect();
        let x = Ctx::new(&r, &r, &[]);
        let t = run("arebta_tt", &x, 0, 7, &Params::new()).unwrap();
        // bar 3: 13 > max(10,11,12) -> long at open of bar 4; bar 6: 9 < min(13,12.5,12) -> short at open of bar 7
        assert_eq!((t[0].dir, t[0].legs[0].i, t[0].exit_i), (1., 4, 7));
        assert_eq!((t[1].dir, t[1].open), (-1., true));
        let ti = run("arebta_ti", &x, 0, 7, &Params::new()).unwrap();
        assert_eq!(ti[0].dir, -1.);
    }

    /// Lane H3b: the 10-year price input is 100 / yield (p.417), so TT-36-n is the channel breakout of that price; the duration
    /// spread is yield10 - yield3m; HLR = (new highs - new lows) / (adv + dec + unch).
    #[test]
    fn other_inputs_follow_the_printed_definitions() {
        let pr = [10., 11., 12., 13., 12.5, 12., 9., 9.5];
        let r: Vec<Row> = pr.iter().enumerate().map(|(i, &c)| Row { ticker: "T".into(), date: format!("{:04}-01-01", 2000 + i), open: c + 0.1, high: c + 1., low: c - 1., close: c, volume: None }).collect();
        let mut x = Ctx::new(&r, &r, &[]);
        x.extra.insert("DGS10".into(), pr.iter().map(|p| 100. / p).collect());
        let t = run("arebta_tt_pt10y", &x, 0, 7, &prm(&[("n", 3.), ("lag", 0.)])).unwrap();
        assert_eq!((t[0].dir, t[0].legs[0].i, t[0].exit_i, t[1].dir), (1., 4, 7, -1.)); // as tt3_reverses_on_next_open on the price 100 / yield
        x.extra.insert("DTB3".into(), vec![1.; 8]);
        let (days, s) = other_input("arebta_e_durspd", &x, 0).unwrap();
        assert_eq!(days.len(), 8);
        assert!((s[0] - (10. - 1.)).abs() < 1e-12 && (s[3] - (100. / 13. - 1.)).abs() < 1e-12);
        // R4 #372: the default reads the previous bar's rate print (lag 1): the first day has none, day k reads bar k-1
        let (d1, s1) = other_input("arebta_e_durspd", &x, 1).unwrap();
        assert_eq!((d1.len(), d1[0]), (7, 1));
        assert!((s1[0] - (100. / pr[0] - 1.)).abs() < 1e-12 && (s1[3] - (100. / pr[3] - 1.)).abs() < 1e-12);
        let t1 = run("arebta_tt_pt10y", &x, 0, 7, &prm(&[("n", 3.)])).unwrap(); // default lag 1: the price series is one bar late, so the long starts one bar later
        assert_eq!((t1[0].dir, t1[0].legs[0].i), (1., 5));
        for (k, v) in [("NYSE_NEW_HIGHS", 30.), ("NYSE_NEW_LOWS", 10.), ("NYSE_ADV_DEC.Advances", 1000.), ("NYSE_ADV_DEC.Declines", 800.), ("NYSE_ADV_DEC.Unchanged", 200.)] { x.extra.insert(k.into(), vec![v; 8]); }
        let (_, h) = other_input("arebta_d_hlr10", &x, 0).unwrap();
        assert!(h.iter().all(|v| v.is_nan()), "10-day MA needs 10 days"); // only 8 bars
        let (_, h) = other_input("arebta_d_hlr10", &x, 0).unwrap();
        assert_eq!(h.len(), 8);
        // the divergence cards need the SPX close: none loaded -> no trade
        assert!(run("arebta_d_djta", &x, 0, 7, &Params::new()).unwrap().is_empty());
    }
    /// Lane K78: a context with NYSE breadth columns; `ratio` sets advances/declines so ADR = ratio (unchanged = 0).
    fn br_ctx(start: &str, ratio: &[f64], close: &[f64]) -> (Vec<Row>, Vec<Vec<f64>>) {
        let y: i32 = start[..4].parse().unwrap();
        let r: Vec<Row> = close.iter().enumerate().map(|(i, &c)| Row { ticker: "SPY".into(), date: format!("{:04}-{:02}-{:02}", y + (i / 336) as i32, 1 + (i % 336) / 28, 1 + i % 28), open: c + 0.5, high: c + 1., low: c - 1., close: c, volume: None }).collect();
        let cols = vec![ratio.iter().map(|q| if q.is_finite() { 1000. * (1. + q) } else { f64::NAN }).collect(), ratio.iter().map(|q| 1000. * (1. - q)).collect(), vec![0.; ratio.len()],
                        ratio.iter().map(|q| 1000. * (1. + q / 2.)).collect(), ratio.iter().map(|q| 1000. * (1. - q / 2.)).collect(), vec![0.; ratio.len()]];
        (r, cols)
    }
    fn with_br<'a>(r: &'a [Row], cols: &[Vec<f64>]) -> Ctx<'a> {
        let mut x = Ctx::new(r, r, &[]);
        for (k, c) in BREADTH.iter().zip(cols) { x.extra.insert(k.to_string(), c.clone()); }
        x
    }
    fn prm(v: &[(&str, f64)]) -> Params { v.iter().map(|(k, x)| (k.to_string(), *x)).collect() }

    #[test]
    fn nse_twin_reads_nse_breadth_and_matches_the_nyse_rule_on_the_same_counts() { // lane CV6
        let q = [0.1, 0.1, 0.1, 0.1, -0.1, f64::NAN, -0.1, -0.1, 0.1, 0.1, 0.1, 0.1];
        let (r, cols) = br_ctx("1995", &q, &[100.; 12]);
        let mut x = Ctx::new(&r, &r, &[]);
        for (k, c) in BREADTH_NSE.iter().zip(&cols) { x.extra.insert(k.to_string(), c.clone()); } // advances, declines, unchanged, up volume, down volume
        let p = prm(&[("input", 24.), ("n", 3.)]);
        let nyse = run("arebta_tt_br", &with_br(&r, &cols), 0, 11, &p).unwrap();
        let nse_t = run("arebta_tt_br_nse", &x, 0, 11, &p).unwrap();
        assert_eq!(nse_t.iter().map(|z| (z.dir, z.legs[0].i)).collect::<Vec<_>>(), nyse.iter().map(|z| (z.dir, z.legs[0].i)).collect::<Vec<_>>());
        assert!(run("arebta_tt_br_nse", &with_br(&r, &cols), 0, 11, &p).unwrap().is_empty(), "the twin ignores NYSE series");
        assert!(run("arebta_tt_br", &x, 0, 11, &p).unwrap().is_empty(), "the original ignores NSE series");
        // volume ratio without unchanged volume: (up - down) / (up + down) carries the same signs here
        assert_eq!(run("arebta_tt_br_nse", &x, 0, 11, &prm(&[("input", 27.), ("n", 3.)])).unwrap().len(), 3);
    }

    #[test]
    fn tt_ti_br_cadr_breakout_skips_missing_breadth_days() {
        // CADR (input 24) rises 3 days, falls 3 days (day 5 has no breadth print: skipped, the state is unchanged), rises again
        let q = [0.1, 0.1, 0.1, 0.1, -0.1, f64::NAN, -0.1, -0.1, 0.1, 0.1, 0.1, 0.1];
        let (r, cols) = br_ctx("1995", &q, &[100.; 12]);
        let x = with_br(&r, &cols);
        let t = run("arebta_tt_br", &x, 0, 11, &prm(&[("input", 24.), ("n", 3.)])).unwrap();
        // breadth days 0..4,6..: CADR .1 .2 .3 .4 .3 | .2 .1 .2 .3 .4 .5; day 3 > max(0..2) long (open 4); day 6: .2 < min(.4,.3,... ) -> the 3 breadth days before are 2,3,4 (.3,.4,.3): short (open 7); day 9 .3 > max(.2,.1,.2): long (open 10)
        assert_eq!(t.iter().map(|z| (z.dir, z.legs[0].i)).collect::<Vec<_>>(), vec![(1., 4), (-1., 7), (1., 10)]);
        assert_eq!(run("arebta_ti_br", &x, 0, 11, &prm(&[("input", 24.), ("n", 3.)])).unwrap()[0].dir, -1.);
        // the volume ratio (input 27) carries the same signs here
        assert_eq!(run("arebta_tt_br", &x, 0, 11, &prm(&[("input", 27.), ("n", 3.)])).unwrap().len(), 3);
        // no 2005 cut (R4 #288 (1)): the same breadth in 2006 trades the same way
        let (r2, c2) = br_ctx("2006", &q, &[100.; 12]);
        assert_eq!(run("arebta_tt_br", &with_br(&r2, &c2), 0, 11, &prm(&[("input", 24.), ("n", 3.)])).unwrap().len(), 3);
        // no breadth series (India) -> no trades
        assert!(run("arebta_tt_br", &Ctx::new(&r, &r, &[]), 0, 11, &Params::new()).unwrap().is_empty());
    }

    #[test]
    fn e_br_type2_long_on_lower_down_cross_short_on_upper_up_cross() {
        // CADR up 25 days, down 25, up 25: CN(LMA(CADR,4),15) falls through 30 (event 1: long) then rises through 70 (event 3: short)
        let q: Vec<f64> = (0..75).map(|i| if (25..50).contains(&i) { -0.1 } else { 0.1 }).collect();
        let (r, cols) = br_ctx("1990", &q, &[100.; 75]);
        let x = with_br(&r, &cols);
        let t = run("arebta_e_br", &x, 0, 74, &prm(&[("input", 24.), ("type", 2.), ("disp", 20.), ("span", 15.)])).unwrap();
        assert_eq!(t.iter().map(|z| z.dir).collect::<Vec<_>>(), vec![1., -1.]);
        assert!((26..35).contains(&t[0].legs[0].i) && (51..60).contains(&t[1].legs[0].i), "{:?}", t.iter().map(|z| z.legs[0].i).collect::<Vec<_>>());
    }

    #[test]
    fn d_br_double_channel_normalisation() {
        // a companion (CADR) that lags the price by 3 days diverges from it; nothing can fire before n + 10n = 165 breadth days
        let px: Vec<f64> = (0..400).map(|i| 100. + 10. * ((i as f64) / 9.).sin()).collect();
        let lag: Vec<f64> = (0..400).map(|i| if i < 4 { 0.01 } else { (px[i - 3] - px[i - 4]) / 100. }).collect();
        let (r, cols) = br_ctx("1990", &lag, &px);
        let mut x = with_br(&r, &cols);
        // default reads the SPX close (R4 #288 (2)): none loaded -> no trades; p.ref = 1 uses the own close
        assert!(run("arebta_d_br", &x, 0, 399, &prm(&[("input", 24.), ("span", 15.), ("type", 1.)])).unwrap().is_empty());
        let own = run("arebta_d_br", &x, 0, 399, &prm(&[("input", 24.), ("span", 15.), ("type", 1.), ("disp", 10.), ("ref", 1.)])).unwrap();
        x.extra.insert("FOLLOWER_CLOSE".into(), px.clone());
        let t = run("arebta_d_br", &x, 0, 399, &prm(&[("input", 24.), ("span", 15.), ("type", 1.), ("disp", 10.)])).unwrap();
        assert_eq!(own.iter().map(|z| (z.dir, z.legs[0].i)).collect::<Vec<_>>(), t.iter().map(|z| (z.dir, z.legs[0].i)).collect::<Vec<_>>());
        assert!(t.len() >= 2 && t.windows(2).all(|w| w[0].dir == -w[1].dir) && t[0].legs[0].i > 164);
        // Table 8.3/8.4: type 7 swaps type 1's events, so it reverses at the same bars in the opposite direction
        let u = run("arebta_d_br", &x, 0, 399, &prm(&[("input", 24.), ("span", 15.), ("type", 7.), ("disp", 10.)])).unwrap();
        assert_eq!(t.iter().map(|z| (-z.dir, z.legs[0].i)).collect::<Vec<_>>(), u.iter().map(|z| (z.dir, z.legs[0].i)).collect::<Vec<_>>());
    }
}

// Instrument admission: David Aronson 2026-10-25 block, moved from instrument_policy.rs on 31 Oct 2026 (lane ST).
// Admission only for the module REVIEWED lists (authors/aronson/admission/SOURCE-CONTRACT.md); kept in one block to merge.
// Runs on actual indices, ETFs and stocks, US and India (index_etf_or_stock, nifty50 basket, declared application).
pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book=="arebta" && crate::books::arebta::reviewed(def.id),
    label: "Aronson data-mined these rules on the S&P 500 (1980-2005, detrended returns); runs on other indices, ETFs and single stocks (US and India) on their own closes are a declared application, one unit per trade, next-open fills, no detrending.",
    basis: "David Aronson, Evidence-Based Technical Analysis (Wiley 2007), ch.8, p.397-449",
};
