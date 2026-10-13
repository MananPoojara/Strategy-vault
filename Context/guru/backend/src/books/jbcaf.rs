//! Jacob Bernstein, "Cyclic Analysis in Futures Trading" (Wiley 1988; scan, 299 pdf pages; printed page =
//! pdf page - 6 in the body). Book id `jbcaf`. Source and checks: library/jbcaf/.
//! Most of the book is cycle analysis (time windows, cycle lengths), which is judgement. Two timing tools
//! are stated as stand-alone mechanical rules: the stochastic 25/75 guidelines (Ch.7, p.182-183, formula
//! Fig 7.1 p.177) and the 10-high / 8-low moving-average channel (Ch.9, p.211-214, Fig 9.8).
//! Fills at the signal bar's close (the book is silent on the order; a signal exists once the bar is complete).
use crate::connors::{sma, Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "jbcaf";

pub fn catalogue() -> Vec<StrategyDef> {
    use Status::*;
    let d = |id, name, group, dir, status, rules, notes| StrategyDef {
        id, name, group, session: 0, dir, status, needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK, rules, notes, claim: None,
    };
    vec![
        d("jbcaf_stoch_2575", "Slow stochastic 25/75 re-cross (14 bars, 1/3 smoothing)", "Ch.7 · The Stochastic Approach to Cycle Timing", "Long/Short", Live,
          &["F|\"%K raw = ((Close - Low n) / (High n - Low n)) x 100\"; \"%K = ((%K n-1 x 2) + %K raw) / 3\"; \"%D = ((%D n-1 x 2) + %K) / 3\" (Fig 7.1, p.177); \"1) Use the slow version of stochastics\"; \"2) A 14 period stochastic is best\" (p.186)",
            "F|\"Here are some of my guidelines for the use of stochastics as a timing tool (i.e. not necessarily in conjunction with cycles or seasonals)\" (p.182)",
            "E|\"(4) When stochastic has dropped to under 25% and when both indicators have turned back up over 25%, longs can be established.\" (p.183); \"(3) When %K & %D become oversold falling to 25% or lower ... no buying should yet be done\" (p.183)",
            "E|\"(5) When both stochastic indicators have exceeded or touched 75% thereafter falling under 75%, selling can begin, either of long positions or to establish short positions.\" (p.183)",
            "X|The opposite signal closes the position and reverses it (rule 5: 'either of long positions or to establish short positions')",
            "F|[Read, B] Set-up = %K and %D both <= 25 (>= 75) on one bar; trigger = the first later close with both > 25 (< 75). %K and %D start from the first raw value (seed not given)."],
          "Guidelines (1)-(2) (%K vs %D trend; buying 'on breaks to support', selling 'on rallies to resistance') need support/resistance judgement and are not coded; p.kd_exit=1 (not the default) also closes a long when %K < %D and a short when %K > %D (rule 1/2 reading). Differs from lkcpt_stoch2575 (TradeStation SlowK, %K only, 3-bar SMA smoothing) and jb624_pop (buys above 75). No results printed (Figs 7.6-7.12 are chart examples). Book markets: futures (T-bonds, S&P, soybean meal, cattle, ...); vault = index swap. Fill at the close of the bar that completes the signal (book silent on the order). Long/short stop-and-reverse; p.long_only=1 keeps only the longs."),
        d("jbcaf_mac_10h8l", "10-high / 8-low MA channel, two closes outside", "Ch.9 · Moving-Average Channel and Cycle Timing", "Long/Short", Live,
          &["F|\"10 period M.A. of highs\" and \"8 period M.A. of lows\" (Fig 9.6, p.211); \"The 10/8 channel technique\" (p.217); simple moving average (Fig 9.1, p.207)",
            "E|\"Buy signals are given when two successive closing prices are above the top of the channel; sell signals occur when two successive closings fall under the bottom of the channel.\" (Fig 9.8 caption, p.213)",
            "F|\"the system will so indicate by showing two successive price closings outside the channel (i.e., not touching either extreme of the channel, and not within the channel at all)\" (p.212)",
            "X|[Read, B] Sell signal = close the long and go short, buy signal = cover and go long (the charts show alternating B / S signals; book silent on reversing)"],
          "Differs from jb624_mac2_sar (6-2-4 MAC: whole bars outside the channel, 10-high/8-low); here two CLOSES outside. Further bars outside in the same direction are not new signals. No results printed. Book markets: futures (T-bonds, S&P, soybean meal, cattle, ...); vault = index swap. Fill at the close of the bar that completes the signal (book silent on the order). Long/short stop-and-reverse; p.long_only=1 keeps only the longs."),
        d("jbcaf_cycles", "Cycle lengths, time windows, cycle tops and bottoms", "Ch.1-5, 13-15", "Long/Short",
          NotTestable("cycle lows/highs and their time windows are identified by eye ('the cycle method is still somewhat subject to interpretation', p.261); cycle data are futures (needs futures contracts)"),
          &["E|Enter at the time window of an expected cycle low / high with a timing signal (Ch.6-12)"], "Listed so the pipeline knows these pages were read."),
        d("jbcaf_basic_indicators", "Reversal signals R+/R-, KR+/KR-, 3H+/3L-, HLC/LHC, COH/COL", "Ch.6 · The Basic Indicators and Their Use", "Long/Short",
          NotTestable("defined exactly (p.172-175) but 'used only in the time frame of expected cycle turns' and 'must be used with stop losses or other risk limiting methods' with no size or exit (p.175)"),
          &["E|Upside reversal: L2 < L1 and C2 > C1 (p.174)", "E|3H+: current close higher than the highest of the last three closes, in a cycle low time window (p.175)"], "Listed so the pipeline knows these pages were read."),
        d("jbcaf_mao", "Moving average oscillators (MAO) and oscillator / MA crossovers", "Ch.10 · Moving Average Oscillators and Cycles", "Long/Short",
          NotTestable("no fixed lengths (5/21, 3/11, 13/21, 3/7, exponential versions shown, p.226-229); crossovers 'should ideally be employed during the time window top or bottom for given cycle' and entries at 'resistance and support levels' (p.231)"),
          &["E|\"Buy and sell signals of the oscillator/moving average combination are generated on crossovers\" (p.231)"], "Listed so the pipeline knows these pages were read."),
        d("jbcaf_mah_mal", "MAH / MAL cycle-top and cycle-low timing, support/resistance", "Ch.8-9 · Support, Resistance, Price Projections; MAH and MAL", "Long/Short",
          NotTestable("used with cycle expectations and support/resistance judgement (p.217-223); price projections are judgement (Ch.8)"),
          &["E|\"Cycle lows are usually established as prices begin to make consecutive closes above the MAH value\" (p.220)"], "Listed so the pipeline knows these pages were read."),
        d("jbcaf_open_close5", "5-period open / 5-period close moving averages", "Ch.12 · Openings and Closings", "Long/Short",
          NotTestable("rule 4: 'most effective in conjunction with cycles, within the anticipated time frame of a cycle top or bottom'; rule 5: 'If a cycle is due to top or bottom but if no 5 open/5 close signal appears within the time window, no action is taken, and vice versa' (p.259) - the trade needs the cycle window (judgement)"),
          &["E|\"When the closing MA plot is greater than the opening MA plot, the trend has probably turned higher and the cycle has bottomed\" (p.259)"], "Listed so the pipeline knows these pages were read."),
        d("jbcaf_trendlines", "Cycles, trendlines and timing", "Ch.11", "Long/Short",
          NotTestable("trendlines drawn by eye"), &["E|Trendline breaks within cycle windows (Ch.11)"], "Listed so the pipeline knows these pages were read."),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    Some(match id {
        "jbcaf_stoch_2575" => ("jbcaf_timing", "Bernstein cyclic timing tools", "Stochastic 25/75"),
        "jbcaf_mac_10h8l" => ("jbcaf_timing", "Bernstein cyclic timing tools", "10H/8L channel, 2 closes"),
        "jbcaf_cycles" | "jbcaf_basic_indicators" | "jbcaf_mao" | "jbcaf_mah_mal" | "jbcaf_open_close5" | "jbcaf_trendlines" =>
            ("jbcaf_nt", "Bernstein cyclic analysis (not testable)", "-"),
        _ => return None,
    })
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    Some(match id {
        "jbcaf_stoch_2575" => vec![p("n", "Stochastic period", 14., 5., 40., 1.), p("lo", "Oversold level", 25., 5., 45., 5.), p("hi", "Overbought level", 75., 55., 95., 5.),
                                   p("long_only", "Long only (1) / long-short (0)", 0., 0., 1., 1.), p("kd_exit", "Also exit on %K/%D cross (rule 1-2 reading)", 0., 0., 1., 1.)],
        "jbcaf_mac_10h8l" => vec![p("h", "MA of highs", 10., 3., 30., 1.), p("l", "MA of lows", 8., 3., 30., 1.), p("k", "Successive closes outside", 2., 1., 5., 1.),
                                  p("long_only", "Long only (1) / long-short (0)", 0., 0., 1., 1.)],
        _ => return None,
    })
}

/// Bernstein's slow stochastic (Fig 7.1): raw %K over n bars, %K and %D smoothed (2*prev + new)/3.
pub fn stoch_bern(x: &Ctx, n: usize) -> (Vec<f64>, Vec<f64>) {
    let len = x.c.len();
    let (mut k, mut d) = (vec![f64::NAN; len], vec![f64::NAN; len]);
    let (mut pk, mut pd) = (f64::NAN, f64::NAN);
    for i in 0..len {
        if i + 1 < n { continue; }
        let (mut hh, mut ll) = (f64::MIN, f64::MAX);
        for j in i + 1 - n..=i { hh = hh.max(x.h[j]); ll = ll.min(x.l[j]); }
        let raw = if hh > ll { (x.c[i] - ll) / (hh - ll) * 100. } else { 50. };
        pk = if pk.is_finite() { (pk * 2. + raw) / 3. } else { raw };
        pd = if pd.is_finite() { (pd * 2. + pk) / 3. } else { pk };
        k[i] = pk; d[i] = pd;
    }
    (k, d)
}

fn trade(dir: f64, i: usize, px: f64) -> Trade {
    Trade { dir, legs: vec![Leg { i, px, w: 1. }], exit_i: i, exit_px: px, reason: String::new(), entry_kind: "close", signal_i: i, open: false, exits: vec![] }
}

/// Event machine: `sig[i]` = Some(+1/-1) wanted side (a signal); optional `fl`/`fs` = close a long / short.
/// History before `from` is simulated; only trades entered inside the window are reported.
fn machine(x: &Ctx, from: usize, to: usize, sig: &[Option<i8>], flat: Option<(&[bool], &[bool])>, long_only: bool, why: [&str; 3]) -> Vec<Trade> {
    let mut out = vec![];
    let mut pos: i8 = 0;
    let mut cur: Option<Trade> = None;
    let close = |mut t: Trade, i: usize, w: &str| { t.exit_i = i; t.exit_px = x.c[i]; t.signal_i = i; t.reason = w.into(); t };
    for i in 0..=to {
        if let Some(s) = sig[i] {
            if s != pos {
                if let Some(t) = cur.take() { out.push(close(t, i, if s > 0 { why[0] } else { why[1] })); }
                pos = if s < 0 && long_only { 0 } else { s };
                if pos != 0 && i >= from && i < to { cur = Some(trade(pos as f64, i, x.c[i])); }
            }
            continue;
        }
        if let Some((fl, fs)) = flat {
            if (pos > 0 && fl[i]) || (pos < 0 && fs[i]) {
                if let Some(t) = cur.take() { out.push(close(t, i, why[2])); }
                pos = 0;
            }
        }
    }
    if let Some(mut t) = cur { t.exit_i = to; t.exit_px = x.c[to]; t.signal_i = to; t.reason = "open".into(); t.open = true; out.push(t); }
    out
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    let schema = params(id)?;
    let n = x.c.len();
    if n == 0 || from > to || to >= n { return Some(vec![]); }
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).unwrap().def);
    let u = |k: &str| g(k).round().max(1.) as usize;
    let long_only = g("long_only") >= 0.5;
    Some(match id {
        "jbcaf_stoch_2575" => {
            let (k, d) = stoch_bern(x, u("n"));
            let (lo, hi) = (g("lo"), g("hi"));
            let (mut arm_b, mut arm_s) = (false, false);
            let mut sig = vec![None; n];
            for i in 0..n {
                if !(k[i].is_finite() && d[i].is_finite()) { continue; }
                if arm_b && k[i] > lo && d[i] > lo { sig[i] = Some(1); arm_b = false; }
                else if arm_s && k[i] < hi && d[i] < hi { sig[i] = Some(-1); arm_s = false; }
                if k[i] <= lo && d[i] <= lo { arm_b = true; }
                if k[i] >= hi && d[i] >= hi { arm_s = true; }
            }
            let kd = g("kd_exit") >= 0.5;
            // rule 1/2 option: flat when %K is on the wrong side of %D for the open position
            let flat_l: Vec<bool> = (0..n).map(|i| k[i] < d[i]).collect();
            let flat_s: Vec<bool> = (0..n).map(|i| k[i] > d[i]).collect();
            machine(x, from, to, &sig, kd.then_some((&flat_l[..], &flat_s[..])), long_only,
                    ["both lines back above 25 (after both <= 25)", "both lines back below 75 (after both >= 75)", "%K crossed %D against the position"])
        }
        "jbcaf_mac_10h8l" => {
            let (mh, ml) = (sma(&x.h, u("h")), sma(&x.l, u("l")));
            let kk = u("k");
            let mut sig = vec![None; n];
            let mut last: i8 = 0;
            for i in 0..n {
                if i + 1 < kk { continue; }
                let above = (i + 1 - kk..=i).all(|j| mh[j].is_finite() && x.c[j] > mh[j]);
                let below = (i + 1 - kk..=i).all(|j| ml[j].is_finite() && x.c[j] < ml[j]);
                let s = if above { 1 } else if below { -1 } else { 0 };
                if s != 0 && s != last { sig[i] = Some(s); last = s; }
            }
            machine(x, from, to, &sig, None, long_only, ["two closes above the 10-bar MA of highs", "two closes below the 8-bar MA of lows", ""])
        }
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    #[test]
    fn smoothing_by_hand() {
        // (2*prev + new)/3 with prev 30, raw 60 -> 40 ; %D (2*20 + 40)/3 = 26.67
        let (pk, pd, raw) = (30.0f64, 20.0f64, 60.0f64);
        let k = (pk * 2. + raw) / 3.;
        let d = (pd * 2. + k) / 3.;
        assert!((k - 40.).abs() < 1e-12 && (d - 80. / 3.).abs() < 1e-12);
    }
}
