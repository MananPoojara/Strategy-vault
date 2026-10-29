//! Ashwani Gujral, "How to Make Money Trading with Charts" (Vision Books, 2nd ed.; book id `ggcharts`; PDF pages of the text layer).
//! Reading: claude-continuation-20261020/authors/gujral (agents/ggcharts) and authors/lanec/admission3/lanec3_ref.py. Admitted: the 23-30 EMA band
//! (Ch.9 p.220, a method the book credits to George Kleinman) and the MACD histogram turn for trading markets (Ch.10 summary, p.260).
//! Not added (duplicates already live): close vs the 20/50-day EMA = apsmts_pen (p.n, p.kind=2), 20/50 EMA cross = fgtofd_ema20_50,
//! MACD 12/26/9 signal cross = hbiab_macd_example. Railway-Line Theory, pullbacks, patterns: judgement. Book: Indian stocks.
use crate::books::lc3::{ema, Pos};
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};

pub const REVIEWED: &[&str] = &["ggcharts_band", "ggcharts_mhist"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const NOTE: &str = "[Declared readings] EMA = exponential average of closes, smoothing 2/(n+1), started from the simple average of the first n closes. Signals on the daily close, filled at that close (the book gives no order type). The book gives the two opposite signals only, so a position is held until the opposite signal reverses it (always in after the first signal). No costs.";

fn d(id: &'static str, name: &'static str, group: &'static str, rules: &'static [&'static str], notes: &'static str) -> StrategyDef {
    StrategyDef { id, name, group, session: 0, dir: "Long/Short", status: Status::Live, needs_volume: false, needs_vix: false, needs_series: &[], book: "ggcharts", rules, notes, claim: None }
}

pub fn catalogue() -> Vec<StrategyDef> {
    vec![
        d("ggcharts_band", "23-30 EMA band: close beyond the band, then a higher (lower) close the next day", "Ch.9 Moving Averages · Use of EMA Band (p.220)",
          &["E|\"On a daily chart of a stock, the 23-30 EMA are plotted and a buy (sell) signal is generated only when the price closes above (below) this band and closes the next day higher (lower) than the previous day.\" (p.220)",
            "X|Opposite signal (stop and reverse) - the book states only the two signals",
            "F|Read: 'above the band' = close above both the 23- and the 30-day EMA; the confirming close is the very next day; the signal is taken at that day's close"],
          NOTE),
        d("ggcharts_mhist", "MACD histogram turn: up from below zero (long), down from above zero (short)", "Ch.10 Oscillators · MACD summary (p.260)",
          &["E|\"If you use MACD in a trading market: Go long when the MACD Histogram turns up from below zero. Exit when there is a signal to go short.\" (p.260)",
            "X|\"Go short when MACD Histogram turns down above zero. Exit the position when there is a signal to go long.\" (p.260)",
            "F|MACD = 12-day EMA - 26-day EMA; signal = 9-day EMA of MACD (p.221); histogram = MACD - signal. Read: 'turns up from below zero' = the histogram was below zero and at or below the bar before, and today it is higher; 'turns down above zero' mirrored"],
          "The book places this rule in a 'trading market' (ADX falling, below 30, p.260); that regime call is not applied here. [Declared readings] EMA = exponential average of closes, smoothing 2/(n+1), started from the simple average of the first n values. Signals at the daily close, filled at that close. Always in after the first signal. No costs."),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    let f = match id { "ggcharts_band" => "23-30 EMA band", "ggcharts_mhist" => "MACD histogram turn", _ => return None };
    let id = REVIEWED.iter().copied().find(|r| *r == id)?;
    Some((id, "Gujral Trading with Charts", f))
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> { reviewed(id).then(Vec::new) }

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, _p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.r.len() { return None; }
    let c = &x.c;
    let mut pos = Pos::new();
    if id == "ggcharts_band" {
        let (a, b) = (ema(c, 23), ema(c, 30));
        for i in from.max(1)..=to {
            let j = i - 1;
            if !(a[j].is_finite() && b[j].is_finite()) { continue; }
            if c[j] > a[j].max(b[j]) && c[i] > c[j] { pos.enter(1., i, c[i], "close"); }
            else if c[j] < a[j].min(b[j]) && c[i] < c[j] { pos.enter(-1., i, c[i], "close"); }
        }
    } else {
        let (f, s) = (ema(c, 12), ema(c, 26));
        let m: Vec<f64> = (0..c.len()).map(|i| f[i] - s[i]).collect();
        let sg = ema(&m, 9);
        let hst: Vec<f64> = (0..c.len()).map(|i| m[i] - sg[i]).collect();
        for i in from.max(2)..=to {
            let (h0, h1, h2) = (hst[i], hst[i - 1], hst[i - 2]);
            if !(h0.is_finite() && h1.is_finite() && h2.is_finite()) { continue; }
            if h1 < 0. && h1 <= h2 && h0 > h1 { pos.enter(1., i, c[i], "close"); }
            else if h1 > 0. && h1 >= h2 && h0 < h1 { pos.enter(-1., i, c[i], "close"); }
        }
    }
    Some(pos.done(to, c[to]))
}
