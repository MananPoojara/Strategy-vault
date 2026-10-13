//! SFO Magazine 2001-2004 (sfomag.com; research drive Trading Knowledge II/@NewsLetters-Mags/SFO Magazine (2001-2004),
//! copy in Books/SFO Magazine/). Book id "sfo": the fully specified systems (entry AND exit) printed in Vol.1 #1 - Vol.3 #5.
//! Page refs = "V<vol>#<issue> p.<PDF page>". Notes, oracle and book checks: library/sfo/README.md.
use crate::books::atm03::{done, finish, new_trade, stdev};
use crate::connors::{sma, Ctx, ParamDef, Params, Status, StrategyDef, Trade};
use Status::*;

pub const BOOK: &str = "sfo";

/// Toepke / MRCI S&P 500 seasonal windows (V2#6 p.64 Figure 4): (direction, entry month, day, exit month, day).
const TOEPKE: [(f64, u32, u32, u32, u32); 6] = [(1., 6, 25, 7, 14), (-1., 7, 16, 7, 23), (-1., 7, 31, 8, 11), (1., 8, 15, 8, 25), (-1., 9, 16, 9, 27), (1., 10, 26, 11, 14)];

pub fn catalogue() -> Vec<StrategyDef> {
    let d = |id, name, group, dir, status, rules, notes, claim| StrategyDef { id, name, group, session: 0, dir, status, needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK, rules, notes, claim };
    let c = crate::connors::claim;
    let sec_rules: &'static [&'static str] = &["F|Table 2 \"Sector Index Seasonality - Selected Percentage Plays\" (V2#10 p.49): Index, Start, End, 11-year average %",
        "E|Long (short for rows marked \"(Bearish)\") at the close of the month before the Start month",
        "X|Exit at the close of the End month (Brown's SMH example: \"short September (Aug close -> Sept close)\", \"long Oct-Apr (Sept close -> Apr close)\")"];
    let sec_note = "Months held = Start..End inclusive; trades at the last close of the month before Start and the last close of End [reading of the example, V2#10 p.50]. Brown also mentions \"a series of stop losses\" without levels (not modelled).";
    vec![
        d("sfo_toepke_sp", "MRCI S&P 500 seasonal key-date trades (Toepke)", "V2#6 p.61-66 · Seasonal trading (Figure 4)", "Long/Short", Live,
          &["F|Figure 4 (p.64), 15-year stats 1988-2002: Buy Sep S&P 6/25-7/14 (13/2, $2,984); Sell Sep S&P 7/16-7/23 (13/2, $5,198); Sell Sep S&P 7/31-8/11 (12/3, $1,898); Buy Sep S&P 8/15-8/25 (13/2, $1,803); Sell Dec S&P 9/16-9/27 (13/2, $3,188); Buy Dec S&P 10/26-11/14 (14/1, $6,227)",
            "E|\"closed higher on about [exit date] than on about [entry date]\" -> enter at the close of the entry date",
            "X|Exit at the close of the exit date; \"MRCI shortens the trade window when an optimized entry or exit date falls on a weekend or holiday\" -> entry on the next trading day, exit on the prior trading day"],
          "p.row = 0 runs all six windows (they do not overlap); 1-6 runs one row. Book: CME S&P 500 futures (Sep/Dec contracts); run on the cash index / ETFs. Author: \"not a black box\".",
          c("90 (6 x 15 yrs)", "87% (78 W / 12 L)", "", "5-19 days", "S&P 500 futures 1988-2002 (MRCI)")),
        d("sfo_sec_sox_jan", "Sector seasonality: semiconductors (SOX) long January (Brown / Almanac)", "V2#10 p.48-52 · Almanac Investing 101 (Table 2)", "Long", Live, sec_rules, sec_note,
          c("11 yrs", "n/a", "+10.9% (SOX Jan-Jan)", "1 month", "Table 2, 11-year average")),
        d("sfo_sec_sox_may", "Sector seasonality: semiconductors (SOX) short May (Brown / Almanac)", "V2#10 p.48-52 · Almanac Investing 101 (Table 2)", "Short", Live, sec_rules, sec_note,
          c("11 yrs", "n/a", "-3.6% (SOX May-May, bearish)", "1 month", "Table 2, 11-year average")),
        d("sfo_sec_sox_sep", "Sector seasonality: semiconductors (SOX) short September (Brown / Almanac)", "V2#10 p.48-52 · Almanac Investing 101 (Table 2)", "Short", Live, sec_rules, sec_note,
          c("11 yrs", "n/a", "-5.8% (SOX Sep-Sep, bearish)", "1 month", "Table 2; SMH 2002: 24.10 -> 19.24")),
        d("sfo_sec_sox_oct_apr", "Sector seasonality: semiconductors (SOX) long October-April (Brown / Almanac)", "V2#10 p.48-52 · Almanac Investing 101 (Table 2)", "Long", Live, sec_rules, sec_note,
          c("11 yrs", "n/a", "+30.3% (SOX Oct-Apr)", "7 months", "Table 2; SMH 2002-03: 19.24 -> 26.39")),
        d("sfo_sec_ndx_nov_jan", "Sector seasonality: Nasdaq 100 long November-January (Brown / Almanac)", "V2#10 p.48-52 · Almanac Investing 101 (Table 2)", "Long", Live, sec_rules, sec_note,
          c("11 yrs", "n/a", "+11.6% (NDX Nov-Jan)", "3 months", "Table 2, 11-year average")),
        d("sfo_sec_rut_jul_oct", "Sector seasonality: Russell 2000 short July-October (Brown / Almanac)", "V2#10 p.48-52 · Almanac Investing 101 (Table 2)", "Short", Live, sec_rules, sec_note,
          c("11 yrs", "n/a", "-4.4% (RUT Jul-Oct, bearish)", "4 months", "Table 2; IWM used for RUT")),
        d("sfo_sec_rut_dec_may", "Sector seasonality: Russell 2000 long December-May (Brown / Almanac)", "V2#10 p.48-52 · Almanac Investing 101 (Table 2)", "Long", Live, sec_rules, sec_note,
          c("11 yrs", "n/a", "+11.9% (RUT Dec-May)", "6 months", "Table 2; IWM used for RUT")),
        d("sfo_bb20_breakout", "20-day Bollinger Band breakout, exit back inside the bands (Steenbarger / Barchart)", "V2#10 p.74 · Psychology and Trading (Table 1)", "Long/Short", Live,
          &["F|Barchart.com 20-day Bollinger Bands, two standard deviations",
            "E|\"buy on close above 20-day Bollinger Band, sell [short] on close below\" (Table 1 note)",
            "X|\"close out the position once the close is back within the bands\" (cover / sell on the next close within the bands)"],
          "Fills at the close. Population SD (Bollinger) [B]. A close from above the upper band straight to below the lower band reverses [B].",
          c("DIA 55, MDY 44, SPY 48, QQQ 43", "DIA 31%, MDY 23%, SPY 33%, QQQ 23%", "DIA -8.90, MDY -13.15, SPY -25.85, QQQ -21.25 points (total)", "", "3/2001-4/2003")),
        d("sfo_needs_data", "SFO seasonal / intraday systems needing data the vault lacks", "V2#6, V2#10, V2#11, V3#3, V3#4", "Long/Short",
          NeedsData("CBOT 10-yr T-note futures (Toepke V2#6 p.64 rows 1-4), NYMEX unleaded gasoline and natural gas futures (Toepke V3#3 p.128), TSX/TSE 300 index (Vialoux V3#4 p.73-75), sector indices XNG, MSH, XTC, BTK, XAL, FPP, XOI, UT, CYC, CMR, DRG, BKX, XAU, XBD, XCI, COMP, PSE (Brown Table 2), 5-minute YM bars (Carter V2#11 p.36-40)"),
          &["E|Toepke: Buy Sep T-note 5/21-8/02 (14/1, $2,458); 5/29-7/16 ($1,640); Dec 8/22-9/15 ($1,329); Dec 8/22-11/12 ($2,110)",
            "E|Toepke energy: Buy July unleaded 3/01-5/09 (14/1, $2,059); June unleaded 3/20-5/14 ($1,885), 4/08-5/09 ($1,456), 4/12-4/29 ($1,095); June nat gas 2/25-4/15 (13/0, $2,862); July nat gas 2/25-4/21 (12/1, $3,108); May nat gas 3/14-4/21 (12/1, $2,638)",
            "E|Vialoux: buy the TSX at the end of November, sell at the end of March (1982-93 6.4% vs 6.1%)",
            "E|Carter mini-Dow: 5-minute SMA(9)/SMA(18) stop-and-reverse, no new entries 12-2 pm ET, 50-point stop, flat at the close"],
          "Same engines as sfo_toepke_sp / sfo_sec_* once the instrument exists.", None),
        d("sfo_not_testable", "Other SFO rules seen but not testable", "SFO 2001-2004", "Long/Short", NotTestable("discretionary selection, missing parameters or no exit (see README)"),
          &["E|Bollinger BB(20,2) + II% non-confirmation (V2#2 p.67-72): no exit stated",
            "E|Carter opening-gap fade (V3#1 p.120-127): selection by \"low pre-market volume\" and a checklist",
            "E|Summa detrended put/call (V2#2 p.10-13): percentile window not stated, no CBOE P/C series; Hendrix gold/XAU ratio (V2#6 p.36-41): lookback unquantified",
            "E|Schlossberg 50% retrace (1-minute, swing undefined); Tower MA21 + stochastic (\"not a finished system\")"],
          "Duplicates skipped: Hirsch best six + MACD = hsta_best6_macd; Rockefeller 200-day = apta_ma200_dji; Donchian 4-week = kdta_donchian4; 5/20 MA = hbiab_ma5_20; Landis 50/200 = sbma_x_50s200s.", None),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    let v: &'static str = catalogue().into_iter().find(|d| d.id == id)?.name;
    Some(match id {
        "sfo_toepke_sp" => ("sfo_seasonal", "SFO · seasonal windows", v),
        x if x.starts_with("sfo_sec_") => ("sfo_sector", "SFO · sector index seasonality (Almanac Table 2)", v),
        "sfo_bb20_breakout" => ("sfo_bands", "SFO · Bollinger breakout", v),
        "sfo_needs_data" | "sfo_not_testable" => ("sfo_other", "SFO · needs data / not testable", v),
        _ => return None,
    })
}

fn sector(id: &str) -> Option<(f64, f64, f64)> {
    Some(match id {
        "sfo_sec_sox_jan" => (1., 1., 1.),
        "sfo_sec_sox_may" => (-1., 5., 5.),
        "sfo_sec_sox_sep" => (-1., 9., 9.),
        "sfo_sec_sox_oct_apr" => (1., 10., 4.),
        "sfo_sec_ndx_nov_jan" => (1., 11., 1.),
        "sfo_sec_rut_jul_oct" => (-1., 7., 10.),
        "sfo_sec_rut_dec_may" => (1., 12., 5.),
        _ => return None,
    })
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    if let Some((_, s, e)) = sector(id) { return Some(vec![p("start", "Start month (held from the prior month's last close)", s, 1., 12., 1.), p("end", "End month (sold at its last close)", e, 1., 12., 1.)]); }
    Some(match id {
        "sfo_toepke_sp" => vec![p("row", "Window (0 = all six, 1-6 = one row of Figure 4)", 0., 0., 6., 1.)],
        "sfo_bb20_breakout" => vec![p("n", "Band length", 20., 5., 100., 1.), p("k", "Standard deviations", 2., 0.5, 4., 0.1)],
        "sfo_needs_data" | "sfo_not_testable" => vec![],
        _ => return None,
    })
}

fn ym(d: &str) -> (i64, i64) { (d[0..4].parse().unwrap_or(0), d[5..7].parse().unwrap_or(0)) }

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    let schema = params(id)?;
    let n = x.r.len();
    if !p.keys().all(|k| schema.iter().any(|d| d.key == k)) || from > to || to >= n { return None; }
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).map_or(0., |d| d.def));
    let c = &x.c;
    let date = |i: usize| x.r[i].date.as_str();
    if let Some((dir, _, _)) = sector(id) {
        let (s, e) = (g("start").round() as i64, g("end").round() as i64);
        let pre = if s == 1 { 12 } else { s - 1 };
        let ends: Vec<usize> = (0..n).filter(|&i| i + 1 < n && ym(date(i + 1)) != ym(date(i))).collect();
        let (mut out, mut cur) = (vec![], None::<Trade>);
        for &i in ends.iter().filter(|&&i| i <= to) {
            let m = ym(date(i)).1;
            if let Some(t) = cur.take() {
                if m == e { out.push(done(t, i, c[i], i, "close of the End month")); } else { cur = Some(t); }
            }
            if cur.is_none() && m == pre && i >= from && i < to { cur = Some(new_trade(dir, i, c[i], "close")); }
        }
        finish(&mut out, cur, x, to);
        return Some(out);
    }
    Some(match id {
        "sfo_toepke_sp" => {
            let row = g("row").round() as usize;
            let (y0, y1) = (ym(date(from)).0, ym(date(to)).0);
            let mut out = vec![];
            for y in y0..=y1 {
                for (k, &(dir, m1, d1, m2, d2)) in TOEPKE.iter().enumerate() {
                    if row != 0 && row != k + 1 { continue; }
                    let (a, b) = (format!("{y:04}-{m1:02}-{d1:02}"), format!("{y:04}-{m2:02}-{d2:02}"));
                    let Some(ei) = (from..=to).find(|&i| date(i) >= a.as_str()) else { continue };
                    if date(ei) > b.as_str() { continue; }
                    let t = new_trade(dir, ei, c[ei], "close");
                    // last session on or before the exit date; if the data ends before it, the trade is still open
                    let xi = (ei..n).take_while(|&i| date(i) <= b.as_str()).last().unwrap_or(ei);
                    let passed = date(xi) == b.as_str() || (xi + 1 < n && date(xi + 1) > b.as_str());
                    if passed && xi <= to && xi > ei { out.push(done(t, xi, c[xi], xi, "close of the exit date")); }
                    else if ei < to && !(passed && xi == ei) { finish(&mut out, Some(t), x, to); }
                }
            }
            out.sort_by_key(|t| t.legs[0].i);
            out
        }
        "sfo_bb20_breakout" => {
            let nn = g("n").max(2.) as usize;
            let (m, sd, k) = (sma(c, nn), stdev(c, nn), g("k"));
            let (mut out, mut cur) = (vec![], None::<Trade>);
            for i in from..=to {
                if m[i].is_nan() { continue; }
                let (ub, lb) = (m[i] + k * sd[i], m[i] - k * sd[i]);
                let want: i8 = if c[i] > ub { 1 } else if c[i] < lb { -1 } else { 0 };
                let held = cur.as_ref().map_or(0, |t| t.dir as i8);
                if want == held || (held == 0 && want == 0) { continue; }
                if let Some(t) = cur.take() { out.push(done(t, i, c[i], i, if want == 0 { "close back within the bands" } else { "close beyond the opposite band" })); }
                if want != 0 { cur = Some(new_trade(want as f64, i, c[i], "close")); }
            }
            finish(&mut out, cur, x, to);
            out
        }
        "sfo_needs_data" | "sfo_not_testable" => vec![],
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;
    #[test]
    fn toepke_shifts_weekend_dates_inward() {
        // 2001-06-25 is a Monday; 2001-07-14 a Saturday -> exit Friday 2001-07-13
        let ds = ["2001-06-22", "2001-06-25", "2001-06-26", "2001-07-12", "2001-07-13", "2001-07-16"];
        let r: Vec<Row> = ds.iter().enumerate().map(|(i, d)| Row { ticker: "T".into(), date: d.to_string(), open: 100., high: 101., low: 99., close: 100. + i as f64, volume: None }).collect();
        let x = Ctx::new(&r, &r, &[]);
        let mut p = Params::new();
        p.insert("row".into(), 1.);
        let t = run("sfo_toepke_sp", &x, 0, 5, &p).unwrap();
        assert_eq!((t[0].legs[0].i, t[0].exit_i, t[0].exit_px), (1, 4, 104.));
    }
}
