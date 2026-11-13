//! Charles K. Langford, "Three Common Tools, One Protocol", Technical Analysis of Stocks & Commodities V.26:9 (2008) pp.48-49, 52
//! (research share Trading Knowledge I/Stocks & Commodities DVD 11.26 (1982 - 2008) (traders.com)/VOLUMES/V26/C09/166LANG.pdf).
//! Book id `sclang`: the EMA(7)/EMA(27) trend with MACD(7,27,7) waves. Lane PA2 (13 Nov 2026, pool A of authors/a1/A1.tsv,
//! R4 #297); independent check authors/pa2/w/pa2_ref.py (written from the article, not from this file).
//! This module also holds the lane's instrument admission entry (PA2_BOOKS).
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};
use super::sctraub::ema0;
use super::scdkdmi::{at_close, gt};

pub const REVIEWED: &[&str] = &["sclang_protocol"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const NOTE: &str = "[Adapted: 1 unstated detail, R4 #297; unverified] Trend = the side of the last EMA(7) / EMA(27) cross of the close: 'I initially open a long position when the EMA(7) crosses above the EMA(27)' and in a bearish trend 'do the opposite: just take short positions'. Inside the trend the MACD (EMA7 - EMA27 of the close, signal line = EMA(7) of the MACD) times the waves: in an uptrend sell (go flat) when the signal line crosses below the MACD line and buy back when the MACD crosses above it; in a downtrend cover when the MACD crosses above the signal and short again when it crosses below. An EMA cross always sets the new trend's position (long on the cross up, short on the cross down) whatever the MACD. Nothing is traded before the first EMA cross of the data (crosses count from bar 27 on, once the seeded EMAs have left the first close); a window that starts inside a trend waits for that trend's next MACD signal. Unstated in the article: (1) the fill - vault reading: signals and fills at the close of the signal day (the article quotes closing prices for its signals: 'The price at the close was 525'0'). EMAs: alpha 2/(n+1), seeded with the first close (common charting default). The RSI(14) divergence is a warning to stop adding positions, not a protocol rule, so it is not coded. Option short = 0 trades the uptrend half only (long / flat). The article's example is December 2007 CBOT wheat (no trade table); runs on indices, ETFs and stocks, US and India, are a declared application.";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef { id: "sclang_protocol", name: "Langford protocol: EMA(7)/EMA(27) cross sets the trend, MACD(7,27,7) crosses trade its waves, at the close", group: "S&C: three common tools, one protocol (Langford, V.26:9)", session: 0, dir: "Long/Short", status: Status::Live,
        needs_volume: false, needs_vix: false, needs_series: &[], book: "sclang", rules: &[
            "F|\"The daily bar chart shows two EMAs (seven and 27 periods); the MACD (7, 27, 7) ... and the 14-period RSI.\" (V.26:9, PDF p.1)",
            "E|\"I apply the protocol by taking an initial position when the EMA(7) and the EMA(27) intersect and then buy and sell following the MACD. During a bullish trend, I initially open a long position when the EMA(7) crosses above the EMA(27).\" (PDF p.2)",
            "X|\"You would sell when the signal line or EMA(7) of the MACD crosses below the MACD line (beginning of a counterwave) and buy back when the MACD gives a signal to buy (beginning of a wave). Then you would sell again at the next short signal and buy back at the following long signal. Through the MACD you would take only long positions during an uptrend, until the EMA(7) and EMA(27) intersect for a correction or a downtrend.\" (PDF p.2)",
            "E|\"In case of a bearish trend, you would do the opposite: just take short positions.\" (PDF p.3)",
            "F|\"on May 25, 2007, the EMA(7) crossed above the EMA(27). The price at the close was 525'0 cents.\" (PDF p.1): signals at the close"],
        notes: NOTE, claim: None }]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> { reviewed(id).then_some(("sclang_protocol", "Langford EMA/MACD protocol (S&C)", "7/27/7")) }
pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    reviewed(id).then(|| vec![
        ParamDef { key: "fast", label: "Fast EMA days (article 7)", def: 7., min: 2., max: 50., step: 1. },
        ParamDef { key: "slow", label: "Slow EMA days (article 27)", def: 27., min: 3., max: 200., step: 1. },
        ParamDef { key: "sig", label: "MACD signal EMA days (article 7)", def: 7., min: 1., max: 50., step: 1. },
        ParamDef { key: "short", label: "1 = short the downtrend waves (article); 0 = long only", def: 1., min: 0., max: 1., step: 1. }])
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from > to || to >= x.r.len() { return None; }
    let g = |k: &str, d: f64| p.get(k).copied().unwrap_or(d).round().max(1.) as usize;
    let (f, s) = (ema0(&x.c, g("fast", 7.)), ema0(&x.c, g("slow", 27.)));
    let m: Vec<f64> = f.iter().zip(&s).map(|(a, b)| a - b).collect();
    let sg = ema0(&m, g("sig", 7.));
    let shorts = p.get("short").copied().unwrap_or(1.) > 0.5;
    // trend: +1 / -1 after the first EMA cross (computed over the whole history so a window start inherits it)
    // (crosses count from bar `slow` on: before it the seeded EMAs still sit on the first close)
    let mut trend = vec![0.0; x.c.len()];
    for t in g("slow", 27.).max(1)..x.c.len() {
        trend[t] = if gt(f[t], s[t]) && !gt(f[t - 1], s[t - 1]) && trend[t - 1] <= 0. { 1. }
            else if gt(s[t], f[t]) && !gt(s[t - 1], f[t - 1]) && trend[t - 1] >= 0. { -1. } else { trend[t - 1] };
    }
    Some(at_close(x, from, to, "MACD wave exit", |t, side| {
        if t == 0 || trend[t] == 0. { return side; }
        let dn = if shorts { -1. } else { 0. };
        if trend[t] != trend[t - 1] { return if trend[t] > 0. { 1. } else { dn }; }
        let up_x = gt(m[t], sg[t]) && !gt(m[t - 1], sg[t - 1]);
        let dn_x = gt(sg[t], m[t]) && !gt(sg[t - 1], m[t - 1]);
        match trend[t] { x if x > 0. => if up_x { 1. } else if dn_x { 0. } else { side },
            x if x < 0. => if dn_x { dn } else if up_x { 0. } else { side },
            _ => side }
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn trend_side_and_waves() {
        let r = super::super::sctraub::tests::bars(2500);
        let x = Ctx::new(&r, &r, &[]);
        let t = run("sclang_protocol", &x, 0, r.len() - 1, &Params::new()).unwrap();
        assert!(t.len() > 10 && t.iter().any(|z| z.dir > 0.) && t.iter().any(|z| z.dir < 0.));
        // every long is entered while EMA7 >= EMA27 and every short while EMA7 <= EMA27
        let (f, s) = (ema0(&x.c, 7), ema0(&x.c, 27));
        for z in &t { let i = z.legs[0].i; assert!(if z.dir > 0. { f[i] >= s[i] } else { f[i] <= s[i] }); assert_eq!(z.legs[0].px, x.c[i]); }
        let mut p = Params::new(); p.insert("short".into(), 0.);
        assert!(run("sclang_protocol", &x, 0, r.len() - 1, &p).unwrap().iter().all(|z| z.dir > 0.));
    }
}

// Instrument admission: lane PA2 S&C articles (pool A), 13 Nov 2026 (one entry for the lane's plain declared-application books).
pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| PA2_BOOKS.contains(&def.book) && pa2_reviewed(def.book, def.id),
    label: "Stocks & Commodities articles (1982-2008) read in lane PA2, illustrated by their authors on futures, single stocks or the S&P 500; runs on indices, ETFs and stocks, US and India, are a declared application, not the authors' tests (any unstated detail is a labelled reading on the card, R4 #297).",
    basis: "Lane PA2 (pool A of authors/a1/A1.tsv, R4 #297): each card cites its article pages; independent check authors/pa2/w/pa2_ref.py",
};
/// Books admitted by the entry above.
pub const PA2_BOOKS: &[&str] = &["sclang", "sckjb", "scmart", "scehfish", "scuhnn", "scsntf", "scbstfs", "scsrcc", "scmbel"];
fn pa2_reviewed(book: &str, id: &str) -> bool {
    match book { "sclang" => reviewed(id), "sckjb" => super::sckjb::reviewed(id), "scmart" => super::scmart::reviewed(id), "scehfish" => super::scehfish::reviewed(id), "scuhnn" => super::scuhnn::reviewed(id), "scsntf" => super::scsntf::reviewed(id), "scbstfs" => super::scbstfs::reviewed(id), "scsrcc" => super::scsrcc::reviewed(id), "scmbel" => super::scmbel::reviewed(id), _ => false }
}
