//! Lawrence Chan, "Daytrading The E-Mini" (data reduction), Technical Analysis of Stocks & Commodities V.21:8 (August 2003)
//! pp.18-26 (research share Trading Knowledge I/Stocks & Commodities DVD 11.26 (1982 - 2008) (traders.com)/VOLUMES/V21/C08/
//! 165CHAN.pdf). Book id `scchem`. Lane PA1 (13 Nov 2026, pool A of authors/a1/A1.tsv; R4 #297). 5-minute bars built from the
//! vault's 1-minute files (intraday_ep::day_bars), one continuous series across days. Independent check authors/pa1/pa1_min_ref.py.
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};
use crate::intraday::day_trade;

pub const INTRA: &[&str] = &["scchem_stoch"];
pub fn reviewed(id: &str) -> bool { INTRA.contains(&id) }

const NOTE: &str = "[Adapted: 2 unstated details, R4 #297; unverified] 5-minute bars of the regular session with the noisy data removed (the article's 'data reduction'): the article cuts the E-mini's first and last 15 minutes (9:30-16:15 ET -> 9:45-16:00 ET). The vault's US minute files hold the cash session 9:30-16:00, so US runs keep 9:45-16:00 (the article's clock window); NSE runs (9:15-15:30 IST) drop the session's first and last 15 minutes (declared adaptation). Indicators run on that one reduced 5-minute series, carried across days: S = 10-bar simple average of the stochastic SlowD(45, 5); SMA = 100-bar simple average of the close. Long when S crosses above 50 and the close is above SMA; short when S crosses below 50 and the close is below SMA; an opposite signal reverses; every position is closed at the last reduced bar's close of the day ('This system exits at the end of each day, and no stops are used'); no entry on that last bar. Unstated in the article: (1) the SlowD construction (NeoTicker); vault reading: %K = (close - lowest low of 45 bars) / (highest high - lowest low) x 100, SlowK = its 5-bar simple average, SlowD = SlowK's 5-bar simple average; (2) the fill of a signal; vault reading: the signal bar's close. Article (S&P e-mini, 5-minute, 2002, $5 round turn, no slippage): full session 232 positions, 52% profitable, +$10,215, profit factor 1.33; reduced 9:45-16:00 219 positions, 52%, +$13,180, profit factor 1.50 (p.22-23). SPY/SPX/NIFTY 50/NIFTYBANK minutes are a declared application (SPY minutes cover 2002).";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef { id: "scchem_stoch", name: "Chan e-mini data reduction: 5-minute SlowD(45,5) crossing 50 with the 100-bar average, first 15 minutes cut, out at the close", group: "S&C: Daytrading The E-Mini (Chan, V.21:8)", session: 0, dir: "Long/Short", status: Status::Live,
        needs_volume: false, needs_vix: false, needs_series: &[], book: "scchem", rules: &[
            "F|\"S = 10-period simple moving average of stochastic SlowD (45-period and five-period smoothing); SMA = 100-period simple moving average of the close\" (V.21:8 p.20)",
            "E|\"Long entry signal = S cross above 50 and close > SMA; Short entry signal = S cross below 50 and close < SMA\" (p.20); \"I removed the first and last 15 minutes of trading data from the chart\" (p.22)",
            "X|\"This system exits at the end of each day, and no stops are used.\" (p.22)"],
        notes: NOTE, claim: None }]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    reviewed(id).then_some(("scchem_stoch", "Chan e-mini stochastic day trade (S&C)", "5-minute"))
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    reviewed(id).then(|| vec![p("head", "Minutes cut after the open (article 15; 0 = full session)", 15., 0., 60., 5.),
        p("k", "Stochastic period (article 45)", 45., 5., 100., 1.), p("ma", "Close average, bars (article 100)", 100., 10., 300., 10.)])
}

/// Simple moving average that skips nothing: NaN until n finite values in a row exist.
fn sma(v: &[f64], n: usize) -> Vec<f64> {
    (0..v.len()).map(|i| if i + 1 < n { f64::NAN } else { let w = &v[i + 1 - n..=i]; if w.iter().all(|z| z.is_finite()) { w.iter().sum::<f64>() / n as f64 } else { f64::NAN } }).collect()
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from > to || to >= x.c.len() { return None; }
    if x.minutes.is_none() { return Some(vec![]); }
    let g = |k: &str, d: f64| p.get(k).copied().unwrap_or(d);
    let head = g("head", 15.).round().max(0.) as u16;
    let (kn, man) = (g("k", 45.).round().max(2.) as usize, g("ma", 100.).round().max(2.) as usize);
    // the reduced series from a few days before the window (indicator history), (day index, bar close, high, low, last of day)
    let start = from.saturating_sub(10);
    let mut bars: Vec<(usize, f64, f64, f64)> = vec![];
    let mut last_of_day: Vec<bool> = vec![];
    for i in start..=to {
        let Some((open, close, _, v)) = crate::intraday_ep::day_bars(x, i, 5) else { continue };
        let tail = if x.r[i].ticker.starts_with("NIFTY") { 15 } else { 0 };
        let keep: Vec<_> = v.iter().filter(|b| b.s >= open + head && b.e <= close - if head == 0 { 0 } else { tail }).collect();
        for (j, b) in keep.iter().enumerate() { bars.push((i, b.c, b.h, b.l)); last_of_day.push(j + 1 == keep.len()); }
    }
    let n = bars.len();
    let fk: Vec<f64> = (0..n).map(|t| {
        if t + 1 < kn { return f64::NAN; }
        let (mut hh, mut ll) = (f64::NEG_INFINITY, f64::INFINITY);
        for b in &bars[t + 1 - kn..=t] { hh = hh.max(b.2); ll = ll.min(b.3); }
        if hh > ll { (bars[t].1 - ll) / (hh - ll) * 100. } else { f64::NAN }
    }).collect();
    let sd = sma(&sma(&fk, 5), 5);
    let s = sma(&sd, 10);
    let closes: Vec<f64> = bars.iter().map(|b| b.1).collect();
    let ma = sma(&closes, man);
    let mut out = vec![];
    let mut pos: Option<(f64, usize, f64)> = None; // dir, day index, entry px
    for t in 1..n {
        let (i, c, ..) = bars[t];
        if i < from { continue; }
        if !last_of_day[t] && s[t].is_finite() && s[t - 1].is_finite() && ma[t].is_finite() {
            let want = if s[t] > 50. && s[t - 1] <= 50. && c > ma[t] { 1. } else if s[t] < 50. && s[t - 1] >= 50. && c < ma[t] { -1. } else { 0. };
            if want != 0. && pos.map_or(true, |q| q.0 != want) {
                if let Some((d, ei, ep)) = pos.take() { out.push(day_trade(d, ei, ep, i, c, "reversed", "close", false)); }
                pos = Some((want, i, c));
            }
        }
        if last_of_day[t] { if let Some((d, ei, ep)) = pos.take() { out.push(day_trade(d, ei, ep, i, c, "end of day", "close", false)); } }
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn stochastic_day_trades_close_each_day() {
        let r = super::super::sctraub::tests::bars(30);
        let x = Ctx::new(&r, &r, &[]);
        assert!(run("scchem_stoch", &x, 0, r.len() - 1, &Params::new()).unwrap().is_empty()); // no minutes: no trades
        let f = sma(&[1., 2., 3., f64::NAN, 5.], 2);
        assert!(f[0].is_nan() && f[1] == 1.5 && f[2] == 2.5 && f[3].is_nan() && f[4].is_nan());
    }
}
// Admission: the intraday block (intraday_ep::PA1, the four minute tickers), like the other minute modules.
