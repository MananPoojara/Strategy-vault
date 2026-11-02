//! Peter Aan, "Relative Strength Index", Technical Analysis of Stocks & Commodities V.7:8 (August 1989) pp.243-245 (research
//! share Trading Knowledge I/Stocks & Commodities DVD 11.26 (1982 - 2008) (traders.com)/VOLUMES/V07/C08/RSI.pdf). Book id
//! `scaanrsi`: the three RSI methods the article tested. Lane K16 (titles loop, 2 Nov 2026), to-code row 238, decision 73;
//! independent check authors/titles-1030/K16w/k16_ref.py.
use crate::connors::{rsi, Ctx, ParamDef, Params, Status, StrategyDef, Trade};
use super::lc3::{hi, lo};
use super::sctraub::next_open;
use super::tselhs::Book;

pub const REVIEWED: &[&str] = &["scaanrsi_cod", "scaanrsi_back", "scaanrsi_bsw"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const NOTE: &str = "[Declared readings] Wilder's 14-day RSI (the article's worked computation; it tested 9, 14 and 19 days, options) with 70/30 levels ('above 70, for instance'). Change of direction: yesterday's RSI above 70 and today's lower than yesterday's = short at the next open (mirror: below 30 and higher = long). Back-through: yesterday above 70 and today below 70 = short at the next open (mirror at 30). Both use the n-day high/low channel trailing stop: for each day the highest high (shorts) or lowest low (longs) of the n days before it, n = 15 (the article's best, on the S&P 500; other n are options), active from the entry day; filled at the stop or at the open when the open is beyond it. After a stop the card is flat until the next signal; an opposite signal reverses at the next open. Buy strength / sell weakness: long when the RSI is above 70, short when below 30, always in once a level is reached, no stops. The author found 'many of the commodities showed only negative results' with the two counter-trend methods; 'Only Standard & Poor's 500-stock index achieved its best results with one of these strategies, the change in direction method and a 15-day high/low channel trailing stop'. Runs on indices, ETFs and stocks, US and India, are a declared application.";

pub fn catalogue() -> Vec<StrategyDef> {
    let d = |id, name, dir, rules| StrategyDef { id, name, group: "S&C: Relative Strength Index (Aan, V.7:8)", session: 0, dir, status: Status::Live,
        needs_volume: false, needs_vix: false, needs_series: &[], book: "scaanrsi", rules, notes: NOTE, claim: None };
    vec![
        d("scaanrsi_cod", "Aan RSI change of direction: fade RSI(14) turning back from above 70 / below 30, 15-day channel trailing stop", "Long/Short", &[
            "E|\"The change of direction method requires (for a short signal) that the RSI trade into overbought territory (above 70, for instance), then turn down for just one day. The short is initiated on the next day's opening\" (V.7:8, PDF p.2)",
            "X|\"with an n-day high/low channel trailing stop. In other words, once a short is initiated, a stop at the highest high of the most recent n days trails behind the market\" (PDF p.2); n = 15: \"the change in direction method and a 15-day high/low channel trailing stop\" (PDF p.2)"]),
        d("scaanrsi_back", "Aan RSI back-through: fade RSI(14) falling back through 70 / rising back through 30, 15-day channel trailing stop", "Long/Short", &[
            "E|\"The 'back-through' parameter method forces the RSI to trade above the overbought parameter (70, for instance) and penetrate 70 from above signaling a sell on tomorrow's opening.\" (PDF p.2)",
            "X|\"Again, an n-day high/low channel trailing stop is used.\" (PDF p.2)"]),
        d("scaanrsi_bsw", "Aan RSI buy strength / sell weakness: long above RSI(14) 70, short below 30, always in", "Long/Short", &[
            "E|\"It buys when the market reaches an overbought indication and sells on an oversold indication. The RSI then signals the start of new trends.\" (PDF p.2)",
            "X|\"No stops are used in this method, and you are constantly in the market.\" (PDF p.2)"]),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    let v = match id { "scaanrsi_cod" => "Change of direction", "scaanrsi_back" => "Back-through", "scaanrsi_bsw" => "Buy strength / sell weakness", _ => return None };
    Some(("scaanrsi", "Aan RSI methods (S&C)", v))
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    let mut v = vec![p("n", "RSI days (article 14; tested 9, 14, 19)", 14., 2., 30., 1.), p("ob", "Overbought level", 70., 50., 95., 1.), p("os", "Oversold level", 30., 5., 50., 1.)];
    if id != "scaanrsi_bsw" { v.push(p("stop", "Channel trailing stop, days (S&P best 15)", 15., 2., 60., 1.)); }
    reviewed(id).then_some(v)
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.r.len() { return None; }
    let g = |k: &str, d: f64| p.get(k).copied().unwrap_or(d);
    let r = rsi(&x.c, g("n", 14.).round().max(1.) as usize);
    let (ob, os) = (g("ob", 70.), g("os", 30.));
    if id == "scaanrsi_bsw" {
        return Some(next_open(x, from, to, "reversed", |t, side| if r[t] > ob { 1.0 } else if r[t] < os { -1.0 } else { side }));
    }
    let n = g("stop", 15.).round().max(1.) as usize;
    let cod = id == "scaanrsi_cod";
    let mut pos = Book::new();
    let mut pend: Option<f64> = None;
    for i in from..=to {
        if let Some(d) = pend.take() { pos.enter(d, i, x.o[i], "open"); }
        if i >= n {
            let s = pos.side();
            if s < 0.0 { let st = hi(&x.h, i - n, i - 1); if x.h[i] >= st { pos.exit(i, x.o[i].max(st), "channel stop"); } }
            else if s > 0.0 { let st = lo(&x.l, i - n, i - 1); if x.l[i] <= st { pos.exit(i, x.o[i].min(st), "channel stop"); } }
        }
        if i == to || i == 0 || !r[i - 1].is_finite() { continue; }
        let (a, b) = (r[i - 1], r[i]);
        let (sh, lg) = if cod { (a > ob && b < a, a < os && b > a) } else { (a > ob && b < ob, a < os && b > os) };
        let want = if sh { -1.0 } else if lg { 1.0 } else { 0.0 };
        if want != 0.0 && want != pos.side() { pend = Some(want); }
    }
    Some(pos.done(to, x.c[to]))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rsi_methods() {
        let r = super::super::sctraub::tests::bars(3000);
        let x = Ctx::new(&r, &r, &[]);
        for id in REVIEWED {
            let t = run(id, &x, 0, r.len() - 1, &Params::new()).unwrap();
            assert!(t.len() > 3 && t.iter().any(|z| z.dir > 0.0) && t.iter().any(|z| z.dir < 0.0), "{id}");
            assert!(family_of(id).is_some() && params(id).is_some());
        }
    }
}

// Instrument admission: S&C Aan RSI 2026-11-02 (titles lane K16).
pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book == "scaanrsi" && reviewed(def.id),
    label: "Peter Aan tested the three RSI methods on futures (results as a chart; only the S&P 500 was profitable, change of direction with a 15-day stop). Runs on indices, ETFs and stocks, US and India, are a declared application.",
    basis: "Technical Analysis of Stocks & Commodities V.7:8 pp.243-245 (Relative Strength Index); independent check authors/titles-1030/K16w/k16_ref.py",
};
