//! Peter W. Aan, "The Weekly Rule System, also known as The Mystery System" (PWA Futures, 1993 study; sold through
//! Futures Truth). Book id `aanwr`. Source copy: Books/Peter W. Aan/Weekly Rule System/MS1-MS8.doc (+ "Updated
//! Perfomance till 2010" Futures Truth reports). Page refs = MS file number. Transcription etc.: library/aanwr/.
//!
//! Rules (MS1): always in the market; if short, cover and go long one tick above the highest high of the last n
//! calendar weeks; if long, reverse one tick below the lowest low of the last n calendar weeks. Weeks are Monday-Friday
//! calendar weeks; the n-week high/low is located "at the end of every week" (worksheet), so during week w the stops
//! are the extremes of the n completed weeks before w. Reversal stops fill at the stop, or at the open on a gap.
//! Distinct from cov_tc_4week_reversal (Covel), which uses a rolling 20-trading-day window.
use crate::books::lg3k::reversal;
use crate::connors::{claim, Ctx, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "aanwr";

/// Days since 1970-01-01 of "YYYY-MM-DD".
fn days(d: &str) -> i64 {
    let y: i64 = d[0..4].parse().unwrap_or(1970); let m: i64 = d[5..7].parse().unwrap_or(1); let dd: i64 = d[8..10].parse().unwrap_or(1);
    let (yy, mm) = if m <= 2 { (y - 1, m + 9) } else { (y, m - 3) };
    let era = yy.div_euclid(400); let yoe = yy - era * 400;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + (153 * mm + 2) / 5 + dd - 1;
    era * 146097 + doe - 719468
}
/// Monday-based calendar week number (1970-01-01 was a Thursday).
pub(crate) fn week(d: &str) -> i64 { (days(d) + 3).div_euclid(7) }

pub fn catalogue() -> Vec<StrategyDef> {
    use Status::*;
    vec![StrategyDef { id: "aanwr_weekly_rule", name: "Weekly Rule (Mystery System) · n-week stop and reverse", group: "Weekly Rule System (MS1-MS5)",
        session: 0, dir: "Long/Short", status: Live, needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK,
        rules: &["E|\"If short, cover and go long one tick above the highest high of the last n calendar weeks\" (MS1 rule 1)",
            "E|\"If long, sell and sell short one tick below the lowest low of the most recent n calendar weeks\" (MS1 rule 2)",
            "F|\"Donchian apparently always used 4 weeks\" - the original parameter n = 4 (MS1; table 1 'Original Parameters (4 weeks)')",
            "F|\"A calendar week is Monday through Friday regardless of holidays ... The system is always long or short-never flat\" (MS1)",
            "F|\"At the end of every week you will locate the n week highs or lows ... Your reversal stops will be one tick above and below these levels\" (MS1 worksheet)",
            "X|Exit = the reversal stop (stop and reverse only)"],
        notes: "Book: 18 futures markets, continuous contracts, System Writer Plus, $100 slippage+commission, data to June 1993 (MS5). S&P 500 row, 4/4 weeks: 89 trades, 33% wins, total -$60,425, avg -$678/trade (MS6). Optimised S&P 5/5 weeks: 72 trades, 31%, -$36,125 (MS7). Run on the index (instrument swap); tick = one price increment (param). Separate buy / sell week counts are the book's own third table (MS5/MS8), exposed as params with 4/4 default. First trade: the first breakout either way after the first n complete weeks (MS2 option a, 'wait for a fresh reversal').",
        claim: claim("89", "33%", "-$678", "", "S&P 500 futures to 6/1993, 4 weeks (MS6)") }]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    (id == "aanwr_weekly_rule").then_some(("aanwr_weekly_rule", "Weekly Rule (Mystery System)", "n calendar weeks"))
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    (id == "aanwr_weekly_rule").then(|| vec![
        p("n_buy", "Weeks for the buy stop (book: 4; tested 1-8)", 4., 1., 8., 1.),
        p("n_sell", "Weeks for the sell stop (book: 4; tested 1-8)", 4., 1., 8., 1.),
        p("tick", "One tick (0.05 = NSE / S&P futures increment)", 0.05, 0.0, 1.0, 0.01)])
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    let schema = params(id)?;
    if from > to || to >= x.c.len() { return None; }
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).map_or(f64::NAN, |d| d.def));
    let (nb, ns, tick) = (g("n_buy").max(1.0).round() as usize, g("n_sell").max(1.0).round() as usize, g("tick"));
    // weekly highs/lows of completed weeks, built from bars 0..=to only (no look-ahead past `to`)
    let wk: Vec<i64> = x.r.iter().map(|r| week(&r.date)).collect();
    let mut wh: Vec<f64> = vec![]; let mut wl: Vec<f64> = vec![]; let mut idx = vec![0usize; to + 1];
    for i in 0..=to {
        if i == 0 || wk[i] != wk[i - 1] { wh.push(x.h[i]); wl.push(x.l[i]); } else { let k = wh.len() - 1; wh[k] = wh[k].max(x.h[i]); wl[k] = wl[k].min(x.l[i]); }
        idx[i] = wh.len() - 1;
    }
    let lv = |i: usize| {
        let w = idx[i];
        let b = if w >= nb { wh[w - nb..w].iter().cloned().fold(f64::MIN, f64::max) + tick } else { f64::NAN };
        let s = if w >= ns { wl[w - ns..w].iter().cloned().fold(f64::MAX, f64::min) - tick } else { f64::NAN };
        // the data's first (possibly partial) week is not a complete calendar week
        if w < nb.max(ns) + 1 { (f64::NAN, f64::NAN) } else { (b, s) }
    };
    Some(reversal(x, from, to, "Weekly Rule", lv))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn calendar_weeks() {
        assert_eq!(week("2024-01-01"), week("2024-01-07")); // Mon..Sun same week
        assert_eq!(week("2024-01-08"), week("2024-01-01") + 1);
        assert_eq!(week("1970-01-05") - week("1970-01-04"), 1);
    }
}
