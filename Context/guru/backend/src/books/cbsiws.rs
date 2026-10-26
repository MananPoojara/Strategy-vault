//! Charles B. Schaap, "Invest with Success: Big Profits for Small Investors" (StockMarketStore 2008; book id `cbsiws`; scan, PDF page
//! = printed + 23). Reading: claude-continuation-20261020/authors/schaap/agents/. Live: the 50-50 Strategy on weekly bars (ch.8, 15,
//! 16): 50-week exponential average and 20-week RSI, the "double cross" signal, the High Riser entry on "the first bar entirely
//! above the 50 MA after the 50-50 Strategy signal" ("This type of entry can be programmed", p.198), the stop $0.25 under the 50 MA,
//! moved to breakeven and trailed under the rising 50 MA once a month (p.200, 215-218). The $0.25 offset is a US-dollar stock amount,
//! so the rule runs on US instruments only. Low Risers (enter "about three months" after the average turns up), the TOP SCORE stock
//! selection and the position-size rules are judgement or fundamentals and are not modelled.
use crate::connors::{rsi, Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "cbsiws";
/// Admitted ids (authors/schaap/admission/SOURCE-CONTRACT.md).
pub const REVIEWED: &[&str] = &["cbsiws_5050"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const NOTE: &str = "[Declared readings, not from the book] Weekly bars = Monday-to-Sunday calendar weeks (high, low, last close). 50 MA = exponential average of the weekly closes, alpha 2/51, started at the first weekly close; RSI = Wilder RSI(20) of the weekly closes. Signal: after a weekly close below the 50 MA, the first week that closes above the 50 MA with RSI above 50 (price and RSI may cross together or in different weeks). Entry (High Riser): the first week from the signal week on whose whole range is above the 50 MA while the 50 MA is rising (above last week's value), bought at that week's close; a weekly close back below the 50 MA before that cancels the signal. Stop: $0.25 under the entry week's 50 MA; moved to the entry price once a week-end 50 MA is at least entry + $0.25; on the first trading day of each month raised to $0.25 under the last week-end 50 MA (never lowered). The stop is a resting daily order from the next trading day: it fills at the stop, or at the open when the day opens below it. One position at a time; no costs. Low Risers, TOP SCORE screening and position sizing are not modelled. Book: US stocks and ETFs; the $0.25 offset is a US-dollar amount, so the rule runs on US indices, ETFs and stocks only.";

pub fn catalogue() -> Vec<StrategyDef> {
    use Status::*;
    let nt = |id, name, group, why, rules| StrategyDef { id, name, group, session: 0, dir: "Long", status: NotTestable(why), needs_volume: false,
        needs_vix: false, needs_series: &[], book: BOOK, rules, notes: "Held after the 2026-10-25 reading (authors/schaap/agents).", claim: None };
    vec![
        StrategyDef { id: "cbsiws_5050", name: "50-50 Strategy: weekly 50 MA and RSI 20 double cross, High Riser entry, stop $0.25 under the 50 MA",
            group: "Invest with Success · Ch.8, 15, 16 (p.96-100, 193-218)", session: 0, dir: "Long", status: Live, needs_volume: false, needs_vix: false,
            needs_series: &[], book: BOOK,
            rules: &["F|\"You will only use two indicators for the 50-50 Strategy - the 50-week moving average and the Relative Strength Index (RSI) ... Chose exponential ... use an RSI setting of 20\" (p.96)",
                     "E|\"The classic signal of the 50-50 Strategy is the double cross: 1) price crosses above the 50 MA and 2) RSI crosses above the 50 level ... They may cross together, or cross separately\" (p.196)",
                     "E|\"enter High Risers when price drops under the 50 MA and reverses back up above the 50 MA; you enter exactly on the first bar entirely above the 50 MA after the 50-50 Strategy signal is given. This type of entry can be programmed\" (p.198); \"The entry for both occurs when the 50 MA is rising\" (p.198)",
                     "X|\"The stop-loss order is placed under the 50 MA (less $0.25 cents)\" (p.200); breakeven \"when the 50 MA price equals the entry price plus $0.25 cents\" (p.216)",
                     "X|\"The trailing stop is raised up to a higher level under the 50 MA about once a month\" (p.217); \"Assume you moved your trailing stop up every first of the month\" (p.218)"],
            notes: NOTE, claim: None },
        nt("cbsiws_low_riser", "Low Riser entry (about three months after the 50 MA turns up)", "Invest with Success · Ch.15 (p.198-199)",
           "'about three months' after the average turns up is approximate and the entry bar is left to the investor ('enter the Low Riser about three months after the 50-50 Strategy signal. Do what works best for you', p.198)", &[]),
        nt("cbsiws_top_score", "TOP SCORE stock selection and portfolio rules", "Invest with Success · Ch.9-10, 14, 16-17",
           "trend, overhead supply and price are judged on the chart; SCORE reads earnings, sales and other fundamentals; position sizing and portfolio balance are planning rules", &[]),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    Some(match id {
        "cbsiws_5050" => ("cbsiws_5050", "Schaap 50-50 Strategy", "50 EMA / RSI 20 weekly, High Riser"),
        _ if id.starts_with("cbsiws_") => ("cbsiws_nt", "Invest with Success (held)", "-"),
        _ => return None,
    })
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    Some(match id {
        "cbsiws_5050" => vec![p("ma", "Weekly exponential average (book 50)", 50., 10., 100., 1.), p("rsi", "Weekly RSI length (book 20)", 20., 5., 40., 1.),
            p("offset", "Stop offset under the average (dollars; book 0.25)", 0.25, 0., 2., 0.05)],
        _ if id.starts_with("cbsiws_") => vec![],
        _ => return None,
    })
}

fn days(d: &str) -> i64 {
    let (y, m, dd): (i64, i64, i64) = (d[0..4].parse().unwrap_or(1970), d[5..7].parse().unwrap_or(1), d[8..10].parse().unwrap_or(1));
    let y2 = if m <= 2 { y - 1 } else { y };
    let era = y2.div_euclid(400);
    let yoe = y2 - era * 400;
    let doy = (153 * ((m + 9) % 12) + 2) / 5 + dd - 1;
    era * 146097 + yoe * 365 + yoe / 4 - yoe / 100 + doy - 719468
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if id != "cbsiws_5050" { return None; }
    let schema = params(id)?;
    let n = x.r.len();
    if from > to || to >= n { return Some(vec![]); }
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).unwrap().def);
    let (man, rn, off) = (g("ma").round().max(1.) as usize, g("rsi").round().max(2.) as usize, g("offset"));
    // weekly bars (Monday-Sunday): end bar index, high, low, close
    let wid: Vec<i64> = x.r.iter().map(|r| (days(&r.date) + 3).div_euclid(7)).collect();
    let (mut we, mut wh, mut wl, mut wc): (Vec<usize>, Vec<f64>, Vec<f64>, Vec<f64>) = (vec![], vec![], vec![], vec![]);
    let mut wk_of = vec![0usize; n];
    for i in 0..n {
        if i == 0 || wid[i] != wid[i - 1] { we.push(i); wh.push(x.h[i]); wl.push(x.l[i]); wc.push(x.c[i]); }
        let k = we.len() - 1;
        we[k] = i; wh[k] = wh[k].max(x.h[i]); wl[k] = wl[k].min(x.l[i]); wc[k] = x.c[i]; wk_of[i] = k;
    }
    let a = 2. / (man as f64 + 1.);
    let mut ma = vec![f64::NAN; wc.len()];
    for k in 0..wc.len() { ma[k] = if k == 0 { wc[0] } else { ma[k - 1] + a * (wc[k] - ma[k - 1]) }; }
    let r = rsi(&wc, rn);
    let mut out = vec![];
    let (mut armed, mut signal) = (false, false);
    let mut cur: Option<(Trade, f64)> = None; // trade, stop
    for i in from..=to {
        let k = wk_of[i];
        let week_end = we[k] == i;
        // stop handling (from the day after entry)
        if let Some((t, st)) = cur.as_mut() {
            if t.legs[0].i < i {
                let first_of_month = i > 0 && x.r[i].date[0..7] != x.r[i - 1].date[0..7];
                if first_of_month {
                    let last = if k > 0 && we[k - 1] < i { Some(k - 1) } else { None }; // last completed week before today
                    if let Some(kl) = last { *st = st.max(ma[kl] - off); }
                }
                if x.l[i] <= *st {
                    let (mut t, st) = cur.take().unwrap();
                    t.exit_i = i; t.exit_px = x.o[i].min(st); t.signal_i = i; t.reason = "stop under the 50 MA".into(); out.push(t);
                }
            }
        }
        if !week_end { continue; }
        if let Some((t, st)) = cur.as_mut() {
            if ma[k] >= t.legs[0].px + off { *st = st.max(t.legs[0].px); }
            continue;
        }
        if k == 0 { continue; }
        if wc[k] < ma[k] { armed = true; signal = false; continue; }
        if armed && !signal && wc[k] > ma[k] && r[k] > 50. { signal = true; armed = false; }
        if signal && wl[k] > ma[k] && ma[k] > ma[k - 1] && i < to {
            signal = false;
            cur = Some((Trade { dir: 1., legs: vec![Leg { i, px: x.c[i], w: 1. }], exit_i: i, exit_px: x.c[i], reason: String::new(), entry_kind: "close", signal_i: i, open: false, exits: vec![] }, ma[k] - off));
        }
    }
    if let Some((mut t, _)) = cur { t.exit_i = to; t.exit_px = x.c[to]; t.signal_i = to; t.reason = "open".into(); t.open = true; out.push(t); }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn date_numbers() { assert_eq!(days("1970-01-01"), 0); assert_eq!((days("2024-01-01") + 3).rem_euclid(7), 0); }
}
