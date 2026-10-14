//! Chande & Kroll, "The New Technical Trader" (Wiley 1994). The CMO-driven VIDYA system (ch.8) needs
//! actual futures ticks and its dollar stop. The thrust-oscillator table (ch.6) is a buy-signal event study
//! without an executable exit. Neither is ready on the available price-only cash series.
//! Uses the small engine in chbta2.rs. Source + page per rule: library/chntt/README.md.
use crate::books::chbta2::{getp, none_exit, no_trail, run_sys, Kind, Pos, Sys};
use crate::connors::{sma, Ctx, ParamDef, Params, Status, StrategyDef, Trade};
use crate::books::chbta2::ema;

pub const BOOK: &str = "chntt";
const NYSE: &[&str] = &["NYSE_ADV_DEC.Advances", "NYSE_ADV_DEC.Declines", "NYSE_ADV_DEC.AdvVolume", "NYSE_ADV_DEC.DecVolume"];
const NSE: &[&str] = &["NSE_ADV_DEC.Advances", "NSE_ADV_DEC.Declines", "NSE_ADV_DEC.AdvVolume", "NSE_ADV_DEC.DecVolume"];

pub fn catalogue() -> Vec<StrategyDef> {
    let d = |id, name, group, dir, status, needs_series, rules, notes| StrategyDef { id, name, group, session: 0, dir, status,
        needs_volume: false, needs_vix: false, needs_series, book: BOOK, rules, notes, claim: None };
    use Status::*;
    let nt = |id, name, group, why, rules, notes| d(id, name, group, "Long/Short", NotTestable(why), &[], rules, notes);
    let to_rules: &'static [&'static str] = &[
        "F|\"MT = (AI*AV - DI*DV)\" ... \"TO = (AI*AV - DI*DV)/(AI*AV + DI*DV) * 100\"; \"The multiplication by 100 is optional\" (eq. 6.3, 6.5, p.146); AI/DI = advancing/declining issues, AV/DV = advancing/declining volume (p.144)",
        "E|\"we first calculated TO and then plotted a 21-day simple moving average of the TO, adding a second 13-day exponential moving average of this SMA. A buy signal occurs when the 21-day SMA of TO goes below -0.3 then crosses above its own 13-day EMA. We enter the market on the close the next day.\" (p.157)",
        "X|No exit rule: Table 6.2 reports the gain 3, 6, 12 and 24 months after each buy (p.157). Those observation horizons are not orders; the historical months runner is held."];
    vec![
        d("ch_ntt_vidya_cmo", "CMO-driven VIDYA system", "Ch 8 · How to use this book", "Long/Short", Live, &[], &[
            "F|\"|CMO| = |(2*RSI9 - 100)/100|, A = t*|CMO| (t <= 0.5), VIDYA = A * Close + (1-A) * VIDYA[1]\" (eq. 8.1, p.180); RSI = System Writer's unsmoothed 9-day RSI (p.180)",
            "F|\"we decided to use the value t=0.1 as our constant across all markets and time periods\" (p.181)",
            "E|\"1. The rule to open long positions. If the close of today and yesterday are both above the VIDYA of today, then buy tomorrow on a buy stop order. Place the order 1 tick above today's high.\" (p.181)",
            "E|\"2. The rule to short positions. If the close of today and the close of yesterday are both below the VIDYA of today, then sell tomorrow on a sell stop order. Place the order 1 tick below today's low.\" (p.181-183)",
            "X|\"We won't use any specific exit strategy in testing the model, which means a long entry signal will exit a short trade and vice versa.\" (p.183)",
            "X|\"$1,500 initial stop\" (p.183), in futures contract dollars; held until actual contract inputs and causal fills are verified."],
            "Printed pp.179-189; reviewed order/stop pages pp.180-183 are PDF spreads 101-102. Unsmoothed RSI9 implies |CMO|=|Su-Sd|/(Su+Sd) over 9 changes and t=0.1. One source contract tick is not a generic 0.01 cash-index point. The closing-basis suggestion for stocks does not define a replacement for the dollar stop. Historical runner retained for inspection; no prepared result or source-performance claim."),
        d("ch_ntt_to_buy", "Thrust oscillator buy signal (NYSE breadth)", "Ch 6 · Market thrust", "Long", Live, NYSE, to_rules,
            "Printed pp.143-159, Table 6.2, reviewed PDF spread 89 (pp.156-157). NYSE breadth inputs are required for the buy signal. No source exit or sequential account is supplied; the historical hold-month runner is retained for inspection and held."),
        d("ch_ntt_to_buy_nse", "Thrust oscillator buy signal (NSE breadth)", "Ch 6 · Market thrust", "Long", Live, NSE, to_rules,
            "Historical India mapping retained for inspection. No actual NSE breadth input or missing source exit is invented; held."),
        nt("ch_ntt_vidya_close", "VIDYA close crossover (std-dev VIDYA)", "Ch 3 · VIDYA",
            "VIDYA's volatility index k = sigma(x days)/sigma(reference) is left open (x and the reference are 'for example' choices), and order type/fill is not given.",
            &["E|\"Or, you can go long when prices close above VIDYA, and go short when prices close below VIDYA.\" (p.55)"], "Printed pp.49-72. The fully specified VIDYA system is ch_ntt_vidya_cmo."),
        nt("ch_ntt_vidya_bands", "VIDYA trading bands", "Ch 3 · VIDYA", "Band width and entry/exit are offered as options; no exit rule.",
            &["E|\"You could have gone long the T-Bond when it bounced off the lower band, with a close stop\" (p.55-58)"], "Printed pp.54-58."),
        nt("ch_ntt_linreg", "Linear regression slope / r-squared", "Ch 2 · Linear regression", "Entry is 'when the slope first becomes significantly positive' with no exit ('you may choose to take profits, tighten your stops, or take an antitrend position').",
            &["E|\"You could open a long position when the slope first becomes significantly positive.\" (p.25)"], "Printed pp.19-48 (r2 critical values 0.40/0.20/0.13/0.08 for 10/20/30/50 days)."),
        nt("ch_ntt_qstick_zero", "Qstick zero crossing", "Ch 4 · Qstick", "Order price/time is 'the next day' without an order type, the exit is 'go short ... or close out your long', and the length is 8 days (formula) or 20 days (daily intermediate) with no single choice.",
            &["E|\"When Qstick is negative and then becomes positive, you can go long the next day. When Qstick is positive and becomes negative, you can go short the next day or close out your long position.\" (p.77)"], "Printed pp.73-91."),
        nt("ch_ntt_qstick_ma", "Qstick and its moving average", "Ch 4 · Qstick", "Only a chart reading of crossovers; no order type or exit.",
            &["E|\"A buy signal occurs when the shorter average crosses above the longer one.\" (p.76-77)"], "Printed pp.76-79."),
        nt("ch_ntt_cmo_ma", "CMO and its 10-day SMA", "Ch 5 · CMO", "Offered as 'you could buy ... and sell'; whether a sell exits or reverses and the fill are not given.",
            &["E|\"You could buy when the CMO crossed above its average, and sell when it fell below its moving average.\" (p.114)"], "Printed pp.94-119."),
        nt("ch_ntt_cmo_zero", "Average CMO zero crossing", "Ch 5 · CMO", "'You would buy when the CMO turned positive, and sell when the CMO turned negative' has no order type or fill.",
            &["E|CMO_A = (CMO5 + CMO10 + CMO20)/3; \"buy when the CMO turned positive, and sell when the CMO turned negative\" (p.113)"], "Printed p.113."),
        nt("ch_ntt_stochrsi", "StochRSI 0.2/0.8 signals", "Ch 5 · StochRSI", "Several alternative rules (buy above 0.80 or around 0.2, reverse or close, 0.50 midline, 3-day confirmation) with no single order type or exit.",
            &["E|\"you can buy when it rises above 0.80 and sell when it falls below 0.20. You can then close your position (or reverse) when the values fall below 0.80 or rise above 0.20. Or, you can trade only when the stochRSI falls below 0.50 or rises above 0.50.\" (p.129)"], "Printed pp.124-133."),
        nt("ch_ntt_to_short_term", "Thrust oscillator 5-day SMA vs 5-day EMA", "Ch 6 · Market thrust", "Two alternative triggers ('first turns up ... or crosses above') and no order type; the strategy is only 'suggested' by a chart.",
            &["E|\"A buy signal occurs when the 5-day SMA first turns up after a downswing, or crosses above its own 5-day EMA. A sell signal occurs when the SMA crosses under its trailing EMA.\" (p.156-157)"], "Printed p.157, Figure 6.7."),
        nt("ch_ntt_rotation", "Market rotation by composite momentum", "Ch 8 · Market rotation", "Discretionary ranking of futures markets / stock lists; no entry or exit rule.",
            &["F|\"Composite Momentum: Sum of 5-, 10- and 20-day momentum in dollars\"; trend if r2 > 0.22 (Table 8.5, p.190-191)"], "Printed pp.189-194."),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    Some(match id {
        "ch_ntt_vidya_cmo" => ("ch_vidya", "VIDYA", "CMO-driven system"),
        "ch_ntt_vidya_close" => ("ch_vidya", "VIDYA", "Close crossover · Not testable"),
        "ch_ntt_vidya_bands" => ("ch_vidya", "VIDYA", "Bands · Not testable"),
        "ch_ntt_to_buy" => ("ch_thrust", "Thrust oscillator", "Buy signal · NYSE"),
        "ch_ntt_to_buy_nse" => ("ch_thrust", "Thrust oscillator", "Buy signal · NSE"),
        "ch_ntt_to_short_term" => ("ch_thrust", "Thrust oscillator", "Short-term · Not testable"),
        "ch_ntt_linreg" => ("ch_ntt_ideas", "New Technical Trader indicators", "Linear regression"),
        "ch_ntt_qstick_zero" => ("ch_ntt_ideas", "New Technical Trader indicators", "Qstick zero cross"),
        "ch_ntt_qstick_ma" => ("ch_ntt_ideas", "New Technical Trader indicators", "Qstick MA cross"),
        "ch_ntt_cmo_ma" => ("ch_ntt_ideas", "New Technical Trader indicators", "CMO vs SMA"),
        "ch_ntt_cmo_zero" => ("ch_ntt_ideas", "New Technical Trader indicators", "Average CMO zero"),
        "ch_ntt_stochrsi" => ("ch_ntt_ideas", "New Technical Trader indicators", "StochRSI"),
        "ch_ntt_rotation" => ("ch_ntt_ideas", "New Technical Trader indicators", "Market rotation"),
        _ => return None,
    })
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    Some(match id {
        "ch_ntt_vidya_cmo" => vec![p("cmo_n", "CMO days", 9., 5., 20., 1.), p("t", "Scaling constant t", 0.1, 0.05, 0.5, 0.05),
            p("tick", "Stop-entry offset (1 tick)", 0.01, 0., 5., 0.01), p("stop_pts", "Initial stop, index points (0 = off)", 0., 0., 50., 1.)],
        "ch_ntt_to_buy" | "ch_ntt_to_buy_nse" => vec![p("sma", "TO SMA days", 21., 5., 50., 1.), p("ema", "EMA of the SMA days", 13., 5., 30., 1.),
            p("level", "Oversold level", -0.3, -0.6, 0., 0.05), p("months", "Hold, calendar months (book: 3/6/12/24)", 3., 1., 24., 1.)],
        _ => return None,
    })
}

/// |CMO| over n close changes: |Su - Sd| / (Su + Sd) (0 when flat). NaN before n changes.
pub(crate) fn abs_cmo(c: &[f64], n: usize) -> Vec<f64> {
    let mut out = vec![f64::NAN; c.len()];
    for i in n..c.len() {
        let (mut u, mut d) = (0., 0.);
        for k in i + 1 - n..=i {
            let m = c[k] - c[k - 1];
            if m > 0. { u += m } else { d -= m }
        }
        out[i] = if u + d > 0. { (u - d).abs() / (u + d) } else { 0. };
    }
    out
}

/// VIDYA_t = A*C + (1-A)*VIDYA_{t-1}, A = t*|CMO|; seeded with the close of the bar before the first CMO.
pub(crate) fn vidya(c: &[f64], n: usize, t: f64) -> Vec<f64> {
    let cmo = abs_cmo(c, n);
    let mut v = vec![f64::NAN; c.len()];
    if c.len() <= n { return v; }
    v[n - 1] = c[n - 1];
    for i in n..c.len() {
        let a = t * cmo[i];
        v[i] = a * c[i] + (1. - a) * v[i - 1];
    }
    v
}

fn add_months(date: &str, m: usize) -> String {
    let (y, mo): (i64, i64) = (date[0..4].parse().unwrap_or(0), date[5..7].parse().unwrap_or(1));
    let t = y * 12 + (mo - 1) + m as i64;
    format!("{:04}-{:02}-{}", t / 12, t % 12 + 1, &date[8..])
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    let schema = params(id)?;
    if from > to || to >= x.r.len() { return None; }
    let g = |k: &str| getp(&schema, p, k);
    let u = |k: &str| (g(k).round().max(1.)) as usize;
    Some(match id {
        "ch_ntt_vidya_cmo" => {
            let (v, tick) = (vidya(&x.c, u("cmo_n"), g("t").clamp(0., 0.5)), g("tick").max(0.));
            let entry = |i: usize, _: f64| {
                if i == 0 { return [None, None]; }
                let o = if x.c[i] > v[i] && x.c[i - 1] > v[i] { Some((1., Kind::Stop(x.h[i] + tick))) }
                    else if x.c[i] < v[i] && x.c[i - 1] < v[i] { Some((-1., Kind::Stop(x.l[i] - tick))) } else { None };
                [o, None]
            };
            run_sys(x, from, to, &Sys { entry: &entry, exit_close: &none_exit, exit_open: &none_exit, trail: &no_trail,
                stop_pts: g("stop_pts").max(0.), oneway: false })
        }
        "ch_ntt_to_buy" | "ch_ntt_to_buy_nse" => {
            let names = if id == "ch_ntt_to_buy" { NYSE } else { NSE };
            let Some(s) = names.iter().map(|k| x.series(k)).collect::<Option<Vec<&[f64]>>>() else { return Some(vec![]) };
            let to_v: Vec<f64> = (0..x.r.len()).map(|i| {
                let (up, dn) = (s[0][i] * s[2][i], s[1][i] * s[3][i]);
                if up + dn > 0. { (up - dn) / (up + dn) } else { f64::NAN }
            }).collect();
            let sm = sma(&to_v, u("sma"));
            let em = ema(&sm, u("ema"));
            let lvl = g("level");
            let mut sig = vec![false; x.r.len()];
            let mut armed = false;
            for i in 1..x.r.len() {
                if !sm[i].is_finite() { armed = false; continue; }
                if sm[i] < lvl { armed = true; }
                if armed && em[i].is_finite() && em[i - 1].is_finite() && sm[i - 1].is_finite() && sm[i] > em[i] && sm[i - 1] <= em[i - 1] {
                    sig[i] = true;
                    armed = false;
                }
            }
            let m = u("months");
            let entry = |i: usize, _: f64| [sig[i].then_some((1., Kind::NextClose)), None];
            let ex = |i: usize, q: &Pos| (x.r[i].date >= add_months(&x.r[q.i].date, m)).then(|| format!("{m} calendar months after entry · close"));
            run_sys(x, from, to, &Sys { entry: &entry, exit_close: &ex, exit_open: &none_exit, trail: &no_trail, stop_pts: 0., oneway: false })
        }
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn vidya_matches_table_3_7_with_t_half() {
        // Table 3.7 (p.70): June-93 T-bond closes 930201..930226, 9-day |CMO|, VIDYA with t = 0.5 seeded at 930211
        let c = [106.1563, 105.6875, 106.0, 106.5625, 107.0938, 106.7813, 106.6563, 105.8437, 106.5625, 106.7188, 106.5938,
            107.0625, 108.0625, 108.2812, 109.0625, 110.4688, 109.8437, 109.7188, 109.625];
        let cmo = abs_cmo(&c, 9);
        assert!((cmo[9] - 0.140614).abs() < 1e-4);
        assert!((cmo[18] - 0.599983).abs() < 1e-4);
        let v = vidya(&c, 9, 0.5);
        assert_eq!(v[8], 106.5625);
        assert!((v[9] - 106.5735).abs() < 1e-3);
        assert!((v[18] - 109.3646).abs() < 1e-3);
    }
    #[test]
    fn months_roll_over_year_end() {
        assert_eq!(add_months("1990-10-18", 3), "1991-01-18");
        assert_eq!(add_months("1980-04-03", 24), "1982-04-03");
    }
}
