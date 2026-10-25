//! Price Headley, "Big Trends Toolkit 2.0 for MetaStock - User's Manual" (Equis / BigTrends, 2008, 75 pp.).
//! Book id `phbttk`. Source copy: Books/Price Headley/BTTK_2 0.pdf (from R:/USB HDD 1/MetaStock/@Systems & Indicators/
//! Price Headleys Big Trends Toolkit 2.0 (bigtrends.com)/Price Headleys Big Trend v.2.0 for Metastock/BTTK_2 0.pdf).
//! The add-on's MetaStock formulas are compiled into Bigtrendv2setup.exe and are not printed, so only rules the manual
//! states completely are Live: the Percent R confirmation system (p.36-38). Percent R = 100 x (Close - lowest Low of
//! 30 bars) / (highest High of 30 bars - lowest Low of 30 bars) (Williams' %R on Headley's 0-100 scale, 80 = near the
//! high). "Crosses over 80" = MetaStock Cross: above 80 today, at or below 80 yesterday.
use crate::connors::{Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};
/// Admitted ids (authors/headley/admission/SOURCE-CONTRACT.md).
pub const REVIEWED: &[&str] = &["phbttk_pctr_long", "phbttk_pctr_short"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

pub const BOOK: &str = "phbttk";
const G: &str = "Big Trends Toolkit 2.0";

pub fn catalogue() -> Vec<StrategyDef> {
    let mk = |id, name, dir, status, rules, notes| StrategyDef {
        id, name, group: G, session: 0, dir, status, needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK, rules, notes, claim: None,
    };
    use Status::*;
    let opt = "the manual adds exit \"(2) on the close 2 weeks before the expiration day (usually close by the first Friday of the expiration month)\" for the option trade";
    vec![
        mk("phbttk_pctr_long", "Percent R (30) bull confirmation system", "Long", Live, &[
            "E|\"A Bullish Setup occurs when the 30-bar Percent R crosses over the 80% threshold. It is 'Bull Confirmed' when the security closes above the high of the Setup bar.\" (p.36) - buy on that close.",
            "F|\"If we never get the confirmation, then I turn off the setup if Percent R comes back to read 79% or lower\" (p.36)",
            "X|\"If we do get confirmation, then the next reading of 79% or lower will set my next closing stop, where an exit will then occur on any close under the low of the bar when Percent R crossed back under 80% to 79% or lower.\" (p.36); \"exit this position (1) when the close is less than the low on the bar Percent R last crossed back under 80 to give a reading of 79.00 or lower\" (p.37). Until the first such reading after entry there is no stop.",
            "F|[Book silent] Signals and fills at the close (MetaStock system test default). A new setup bar (a fresh cross over 80) replaces an unconfirmed one. A 'cross back under 80 to 79 or lower' = %R <= 79 today and > 79 yesterday. One position at a time.",
            "F|Exit \"(2) on the close 2 weeks before the expiration day\" (p.37) belongs to the call-option trade (\"buying a call option two months before expiration\"); a stock/index position has no expiration day, so the stock rule of p.36 is coded. The option trade is phbttk_options (not testable).",
        ], "Book: US stocks (RIMM example), no statistics printed. Run on the indexes (instrument swap)."),
        mk("phbttk_pctr_short", "Percent R (30) bear confirmation system", "Short", Live, &[
            "E|\"A bearish Setup occurs when the 30-bar Percent R crosses under the 20% threshold. It is Bear Confirmed when the security closes below the low of the Setup bar.\" (p.37) - sell short on that close.",
            "F|\"If we never get the confirmation, then I turn off the setup if Percent R comes back to read 21% or higher\" (p.37)",
            "X|\"the next reading of 21% or higher will set my next closing stop, where an exit will then occur on any close over the high of the bar when Percent R crossed back over 20% to 21% or [higher]\" (p.37-38; printed 'or lower', a misprint: 'exit this position (1) when the close is greater than the high on the bar Percent R last crossed back over 20 to give a reading of 21.00 or higher', p.38)",
            "F|[Book silent] As the long side: fills at the close; cross under 20 = below 20 today, at or above 20 yesterday; cross back over 20 to 21 or higher = %R >= 21 today and < 21 yesterday.",
        ], "Book: US stocks, no statistics printed. Run on the indexes (instrument swap)."),
        mk("phbttk_options", "Option trades and the expiration-day exit (all BTTK systems)", "Long/Short", NotTestable("needs option chains; the expiry month is only given as a range (\"out 3-4 months\", \"2nd expiration month\", \"two months before expiration\")"),
            &["X|\"(2) on the close 2 weeks before the expiration day (usually close by the first Friday of the expiration month)\" (p.11, 13, 19, 21, 25, 26, 29-31, 35, 37-38, 46, 48-49)"], opt),
        mk("phbttk_nasdaq_rs", "NASDAQ vs S&P relative strength market timing", "Long/Short", NotTestable("no confirmation window (\"closes in a future day above the NASDAQ Composite high on the day of the bullish RS crossover\") and no exit rule; needs the NASDAQ Composite (the vault has NDX, not COMP)"),
            &["E|\"When the 10-day simple moving average crosses above the 21-day simple moving average of the relative strength (RS) line, this creates the bullish Setup ... It is ONLY confirmed when the NASDAQ Composite CLOSES in a future day above the NASDAQ Composite high on the day of the bullish RS crossover.\" (p.6)"], "Seen (p.5-8)."),
        mk("phbttk_dmi_diff", "DMI Difference system", "Long/Short", NotTestable("the entry is only described as trends 'when the DMI Difference is above its 10-day SMA' with chart examples; no entry trigger is stated"),
            &["F|DMI Difference = (+DMI) - (-DMI), 14 bars; 10-bar SMA of it; +30/-30 extremes (p.9)", "X|\"exit this position (1) if the DMI Difference crosses under its 10day simple moving average, or (2) ...\" (p.11)"], "Seen (p.9-13)."),
        mk("phbttk_accel_bands", "Acceleration Bands 20 / 80 bar systems", "Long/Short", NotTestable("the band formula is not printed in this manual (only 'adjust for a stock's volatility'); the book version with Headley's printed formula is phbt_accel_*"),
            &["E|\"If this bar's close and the prior bar's close are both greater than the upper 20-bar Acceleration Band, then this in considered a buy signal\" (p.16)", "X|80-bar: exit \"(1) if any future bar's close is less than the upper 80-bar Acceleration Band, or (2) any bar closes under the 20-bar exponential moving average (EMA), or (3) ...\" (p.19)"], "Duplicate idea of phbt_accel_daily (Big Trends in Trading), which has the printed formula."),
        mk("phbttk_efficiency", "Efficiency Ratio +30/-30 system", "Long/Short", NotTestable("the Efficiency Ratio period is not given and the setup has no cancellation rule (\"use that bar's high as the key point that must then be Closed above\")"),
            &["E|\"If the Efficiency Ratio crosses over +30, use that bar's high as the key point that must then be Closed above that bar's high to confirm a buy signal - exit if a close is less than the low on the bar when the Efficiency Ratio crosses back under +30.\" (p.23)"], "Seen (p.22-26)."),
        mk("phbttk_force_st", "Headley Force Index 14/28 (short-term trend)", "Long/Short", NotTestable("the Force Index formula/smoothing is not printed and the confirmation has no window (\"must then be Closed above on a future bar\")"),
            &["E|\"if the 14-bar Force Index ... crosses over the 28-bar Force Index, and both readings are greater than 0, use that bar's high as the key point that must then be Closed above on a future bar\" (p.28)", "X|\"(1) if the close is less than the low on the bar when the 14-bar Force Index crosses back under the 28-bar Force Index\" (p.29)"], "Seen (p.27-30)."),
        mk("phbttk_force_lt", "Headley Force Index 60/120 (long-term trend)", "Long/Short", NotTestable("as phbttk_force_st: formula not printed, no confirmation window"),
            &["E|60-bar Force Index crosses over the 120-bar, both > 0; close above that bar's high on a future bar (p.30)", "X|close below the low of the bar when the 60-bar crosses back under the 120-bar (p.31)"], "Seen (p.30-31)."),
        mk("phbttk_mom_div", "Momentum Divergence (MACD% / Close% 40 bars)", "Long/Short", NotTestable("the 0-100 normalisation of MACD and price is not defined in the manual"),
            &["E|\"If the MACD% line is 100% and the Close% line is 100% ... This is a buy signal\" (p.35)", "X|\"(1) when the MACD% < 90 and the Close% < 90\" (p.35)"], "Seen (p.32-35)."),
        mk("phbttk_pctr_retest", "Percent R Retest entries", "Long/Short", NotTestable("the printed rule contradicts itself (bull retest: \"Percent R crosses above 80% and with in five days closes below that day's low\" as the confirmation, then the exit-stop bar's 'high' for a long)"),
            &["E|\"A Bull retest setup occurs when the security becomes confirmed, that is Percent R crosses above 80% and with in five days closes below that day's low. The Retest entry is triggered if %R crosses back above 80% within 30 bars.\" (p.38)"], "Seen (p.38-39)."),
        mk("phbttk_pct_above_ma", "Percentage above 200 / 400-day MA systems", "Long", NotTestable("no cancellation of an unconfirmed setup (\"Buy if the close is greater than the high of the bar when the Percentage above 200-day Simple MA crosses above 100%\" - no window); for an index the close is never 100% above its 200/400-day average in the vault's data"),
            &["E|\"Buy if the close is greater than the high of the bar when the Percentage above 200-day Simple MA crosses above 100%.\" (p.41; 400-day p.43)", "X|\"Exit if the close is below the low of the bar when the Percentage above 200-day Simple MA crosses back under 100%.\" (p.41)"], "Seen (p.40-44)."),
        mk("phbttk_stoch_conf", "Stochastics Confirmation system (21-bar, exponential)", "Long/Short", NotTestable("\"a Stochastics %K setting of 21 periods with exponential smoothing\" - the smoothing length is not given"),
            &["E|\"a bullish setup occurs when the 21-bar Stochastics crosses over the 80% threshold. It is 'Bull Confirmed' when the security closes above the high of the Setup bar ... I turn off the setup if Stochastics comes back to read 79% or lower\" (p.44)"], "Seen (p.44-47)."),
        mk("phbttk_bb80", "80-bar Bollinger Band breakout", "Long/Short", NotTestable("the band width (standard deviations) is not given and the confirmation (\"followed by a close above that bar's high\") has no window"),
            &["E|\"I look for a first close above the upper 80-bar Bollinger Band, followed by a close above that bar's high to confirm the new uptrend.\" (p.47)", "X|\"(1) when the close is less than the low on any return under the 80-bar upper Bollinger band\" (p.48)"], "Seen (p.47-49)."),
        mk("phbttk_other", "Options Shark, NAZRS template, 2x 200-day line", "Long/Short", NotTestable("listed as add-on tools/templates (p.1-2) with no rules in the manual"),
            &["F|BTTK Options Shark exploration, BTTK NAZRS System template, BTTK Line at Cross of 2 Times 200 Day MA (p.1-2)"], "Seen; no rules printed."),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    match id {
        "phbttk_pctr_long" => Some(("phbttk_pctr", "Headley Percent R confirmation", "bull")),
        "phbttk_pctr_short" => Some(("phbttk_pctr", "Headley Percent R confirmation", "bear")),
        _ if id.starts_with("phbttk_") => Some(("phbttk_other", "Big Trends Toolkit 2.0 - other systems (not testable)", "")),
        _ => None,
    }
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    let long = match id { "phbttk_pctr_long" => true, "phbttk_pctr_short" => false, _ => return None };
    Some(vec![
        p("len", "Percent R bars", 30., 5., 60., 1.),
        if long { p("lvl", "Setup: %R crosses over", 80., 50., 95., 1.) } else { p("lvl", "Setup: %R crosses under", 20., 5., 50., 1.) },
        if long { p("off", "Turn-off / stop reading at or below", 79., 45., 94., 1.) } else { p("off", "Turn-off / stop reading at or above", 21., 6., 55., 1.) },
    ])
}

pub fn pct_r(h: &[f64], l: &[f64], c: &[f64], n: usize) -> Vec<f64> {
    (0..c.len()).map(|t| {
        if t + 1 < n { return f64::NAN }
        let hh = h[t + 1 - n..=t].iter().cloned().fold(f64::MIN, f64::max);
        let ll = l[t + 1 - n..=t].iter().cloned().fold(f64::MAX, f64::min);
        if hh > ll { 100. * (c[t] - ll) / (hh - ll) } else { f64::NAN }
    }).collect()
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    let defs = params(id)?;
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| defs.iter().find(|d| d.key == k).map(|d| d.def).unwrap_or(f64::NAN));
    let d = if id == "phbttk_pctr_long" { 1.0 } else { -1.0 };
    let to = to.min(x.c.len().saturating_sub(1));
    let (h, l, c) = (&x.h, &x.l, &x.c);
    let r = pct_r(h, l, c, g("len").max(2.) as usize);
    let (lvl, off) = (g("lvl"), g("off"));
    // in the long frame: setup = cross over lvl, off-event = reading <= off after > off
    let a = |v: f64| d * v;
    let mut out = Vec::new();
    let mut setup: Option<usize> = None;
    let mut pos: Option<(usize, f64)> = None; // entry bar, stop level (NaN = none yet)
    for t in from.max(1)..=to {
        if !r[t].is_finite() || !r[t - 1].is_finite() { continue }
        let cross = a(r[t]) > a(lvl) && a(r[t - 1]) <= a(lvl);
        let back = a(r[t]) <= a(off) && a(r[t - 1]) > a(off);
        if let Some((i, mut stop)) = pos {
            if back { stop = if d > 0. { l[t] } else { h[t] } }
            if stop.is_finite() && a(c[t]) < a(stop) {
                out.push(Trade { dir: d, legs: vec![Leg { i, px: c[i], w: 1.0 }], exit_i: t, exit_px: c[t], reason: "closing stop".into(), entry_kind: "close", signal_i: t, open: false, exits: vec![] });
                pos = None;
            } else { pos = Some((i, stop)); }
            continue;
        }
        if cross { setup = Some(t); continue }
        if let Some(s) = setup {
            if a(c[t]) > a(if d > 0. { h[s] } else { l[s] }) { pos = Some((t, f64::NAN)); setup = None; }
            else if a(r[t]) <= a(off) { setup = None; }
        }
    }
    if let Some((i, _)) = pos {
        out.push(Trade { dir: d, legs: vec![Leg { i, px: c[i], w: 1.0 }], exit_i: to, exit_px: c[to], reason: "open".into(), entry_kind: "close", signal_i: to, open: true, exits: vec![] });
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;
    #[test]
    fn confirm_then_closing_stop() {
        let mut v: Vec<(f64, f64, f64)> = (0..30).map(|i| { let b = 100. - (i % 5) as f64; (b + 1., b - 1., b) }).collect();
        v.push((106., 99., 105.5)); // 30: %R crosses over 80 -> setup, high 106
        v.push((107., 105., 106.5)); // 31: close above 106 -> long at 106.5
        v.push((107., 100., 101.)); // 32: %R back to <= 79 -> stop = low 100
        v.push((102., 99., 99.5)); // 33: close under 100 -> exit at 99.5
        let r: Vec<Row> = v.iter().enumerate().map(|(i, &(h, l, c))| Row { ticker: "T".into(), date: format!("2000-01-{:02}", i % 28 + 1), open: c, high: h, low: l, close: c, volume: None }).collect();
        let x = Ctx::new(&r, &r, &[]);
        let t = run("phbttk_pctr_long", &x, 0, r.len() - 1, &Params::new()).unwrap();
        assert_eq!((t[0].legs[0].i, t[0].legs[0].px, t[0].exit_i, t[0].exit_px), (31, 106.5, 33, 99.5));
    }
}
