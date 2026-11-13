//! Tushar S. Chande, "Adapting Moving Averages To Market Volatility", Technical Analysis of Stocks & Commodities V.10:3 (March
//! 1992) pp.108-114 (research share Trading Knowledge I/Stocks & Commodities DVD 11.26 (1982 - 2008) (traders.com)/VOLUMES/V10/C03/
//! ADAPTIN.pdf). Book id `scchvid`: the RAVI and TurboMACD long trades on weekly VIDYAs, HELD (decision 115). Also holds `pstd`. Lane K16 (titles loop, 2 Nov 2026),
//! to-code row 236; independent check authors/titles-1030/K16w/k16_ref.py.
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};
use super::msws::sma;

pub const REVIEWED: &[&str] = &["scchvid_ravi", "scchvid_tmacd"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

/// Population standard deviation of the last n values (MetaStock Std Dev / Bollinger Bands).
pub fn pstd(v: &[f64], n: usize) -> Vec<f64> {
    let m = sma(v, n);
    (0..v.len()).map(|i| if !m[i].is_finite() { f64::NAN } else { (v[i + 1 - n..=i].iter().map(|x| (x - m[i]).powi(2)).sum::<f64>() / n as f64).sqrt() }).collect()
}

const NOTE: &str = "HELD (lead decision 115). The VIDYA smoothing needs reference standard deviations: the article uses 6 and 4, 'a 10-year average for weekly Standard & Poor's 500 data' in early-1980s S&P index points. Those point values do not carry over to other instruments or to today's S&P level, and any normalisation (for example a trailing average of the instrument's own deviation) would be a value we chose, not the author's (decisions 19, 36), so no results are published. Rules as printed: VIDYA_L = 0.078 x s13/6 x C + (1 - 0.078 x s13/6) x VIDYA_L[-1], VIDYA_S = 0.15 x s10/4 x C + (1 - ...) x VIDYA_S[-1] on weekly closes; RAVI = VIDYA_S - VIDYA_L, long when it turns positive, out when it turns negative; TurboMACD = RAVI minus 'an exponential moving average with a smoothing constant of 0.2' (equation 7 prints 0.2 RAVI + 0.8 RAVI[-1]). Article (weekly S&P 500, January 1980 - July 1991, long trades): RAVI 9 trades, about +207 points with one open trade; TurboMACD 25 trades, 16 profitable (64%), about +218 points.";
const WHY_TMACD: &str = "Held (R4 #297 (5), lane PF 14 Nov 2026): on the S&P futures contract the 1980s reference deviations are literal (R4 #293 (4)), but TurboMACD has three open details: the trigger ('an exponential moving average with a smoothing constant of 0.2' against equation 7's 0.2 RAVI + 0.8 RAVI[-1]), the fill and the VIDYA start; question in authors/verifier-questions.tsv.";
const NOTE_RAVI: &str = "[Adapted: 2 unstated details, R4 #293 (4), #297; lane PF, 14 Nov 2026] Runs only on Norgate continuous S&P 500 futures (&SP, back-adjusted daily bars; a standard deviation of closes is unchanged by the additive roll adjustment), with the article's reference deviations 6 and 4 S&P points used literally (R4 #293 (4): 1980s S&P points are native to the contract). Published only 1982-04-21..1991-07-31, the article's period (futures::USD_WINDOW reason). Weekly bars: the last bar of each Monday-based week. s13 / s10 = population standard deviation of the last 13 / 10 weekly closes. VIDYA_L = 0.078 s13/6 C + (1 - 0.078 s13/6) VIDYA_L[-1], VIDYA_S = 0.15 s10/4 C + (1 - 0.15 s10/4) VIDYA_S[-1], as printed (no cap on the constant). Unstated in the article: (1) VIDYA's start (vault reading: the week's close on the first week with s13, resp. s10); (2) the fill (vault reading: the close of the signal week). Long when RAVI turns positive from negative, out when it turns negative from positive. Article: weekly S&P 500 1/1980-7/1991, 9 long trades, about +207 points (one open); the S&P futures start 1982-04, so the book check is partial.";

pub fn catalogue() -> Vec<StrategyDef> {
    let d = |id, name, rules| StrategyDef { id, name, group: "S&C: Adapting Moving Averages To Market Volatility (Chande, V.10:3)", session: 0, dir: "Long",
        status: if id == "scchvid_ravi" { Status::Live } else { Status::NotTestable(WHY_TMACD) },
        needs_volume: false, needs_vix: false, needs_series: &[], book: "scchvid", rules, notes: if id == "scchvid_ravi" { NOTE_RAVI } else { NOTE }, claim: None };
    vec![
        d("scchvid_ravi", "Chande RAVI: weekly short VIDYA (0.15, 10-week sigma) crosses above the long VIDYA (0.078, 13-week sigma), long only", &[
            "F|\"(4) VIDYA Ld = 0.078 x s13 / 6 x Cd + (1 - 0.078 x s13/6) x VIDYA Ld-1 ... (5) VIDYA Sd = 0.15 x s10/4 x Cd + (1-0.15 x s10/4) x VIDYA Sd-1 ... (6) RAVI d = VIDYA Sd - VIDYA Ld\" (V.10:3, PDF p.3)",
            "E|\"A buy signal is generated when RAVI turns positive from a negative value.\" (PDF p.4)",
            "X|\"Similarly, a sell signal occurs when RAVI turns negative from a positive value.\" (PDF p.4)"]),
        d("scchvid_tmacd", "Chande TurboMACD: weekly RAVI minus its 0.2 exponential trigger turns positive, long only", &[
            "F|\"(7) TMACDd = RAVI d - (0.2 x RAVI d + 0.8 x RAVI d-1) where the second term represents the trigger line, an exponential moving average with a smoothing constant of 0.2.\" (PDF p.4)",
            "E|\"A buy signal is produced when TurboMACD changes from negative to positive values,\" (PDF p.4)",
            "X|\"and a sell signal occurs when it goes from positive to negative values.\" (PDF p.4)"]),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    let v = match id { "scchvid_ravi" => "RAVI", "scchvid_tmacd" => "TurboMACD", _ => return None };
    Some(("scchvid", "Chande VIDYA weekly long trades (S&C)", v))
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> { reviewed(id).then(Vec::new) }

/// Held cards: no run (decision 115).
pub fn run(id: &str, x: &Ctx, from: usize, to: usize, _p: &Params) -> Option<Vec<Trade>> {
    if id != "scchvid_ravi" || from > to || to >= x.r.len() { return None; }
    // week-end bars: the next bar starts a later Monday-based week (the last bar closes a week only on a Friday)
    let wk = |i: usize| super::jbcdt2::days(&x.r[i].date) - super::jbcdt2::weekday(&x.r[i].date);
    let ends: Vec<usize> = (0..=to).filter(|&i| if i < to { wk(i + 1) != wk(i) } else { super::jbcdt2::weekday(&x.r[i].date) == 4 }).collect();
    let wc: Vec<f64> = ends.iter().map(|&i| x.c[i]).collect();
    let (s13, s10) = (pstd(&wc, 13), pstd(&wc, 10));
    let vid = |s: &[f64], k: f64, r: f64| { let mut v = f64::NAN; s.iter().zip(&wc).map(|(sd, c)| { if sd.is_finite() { let a = k * sd / r; v = if v.is_finite() { a * c + (1. - a) * v } else { *c }; } v }).collect::<Vec<f64>>() };
    let (vl, vs) = (vid(&s13, 0.078, 6.), vid(&s10, 0.15, 4.));
    let ravi: Vec<f64> = vl.iter().zip(&vs).map(|(l, s)| s - l).collect();
    let mut tgt = vec![0i8; to + 1];
    let mut pos = 0i8;
    let mut last = 0usize;
    for (k, &i) in ends.iter().enumerate() {
        for j in last..i { tgt[j] = pos; }
        if k > 0 && ravi[k - 1] < 0. && ravi[k] > 0. { pos = 1; }
        if k > 0 && ravi[k - 1] > 0. && ravi[k] < 0. { pos = 0; }
        tgt[i] = pos; last = i + 1;
    }
    for j in last..=to { tgt[j] = pos; }
    Some(super::rbtie::target_trades(x, &tgt, from, to, false, "RAVI sign change"))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn population_sd() { assert!((pstd(&[1., 2., 3., 4.], 4)[3] - 1.118033988749895).abs() < 1e-12); }
    #[test]
    fn ravi_live_tmacd_held() {
        let r = super::super::sctraub::tests::bars(300);
        let x = Ctx::new(&r, &r, &[]);
        assert!(run("scchvid_tmacd", &x, 0, r.len() - 1, &Params::new()).is_none() && REVIEWED.iter().all(|id| family_of(id).is_some()));
        assert!(catalogue().iter().filter(|d| d.status == Status::Live).map(|d| d.id).eq(["scchvid_ravi"]));
        assert!(run("scchvid_ravi", &x, 0, r.len() - 1, &Params::new()).is_some());
    }
}

// Instrument admission: S&C Chande VIDYA 2026-11-02 (titles lane K16).
pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    // lane PF: scchvid_ravi runs only on the S&P futures contract (futures::CARDS_PF), so it is not an index/ETF/stock application
    reviewed: |def: &crate::connors::StrategyDef| def.book == "scchvid" && reviewed(def.id) && crate::futures::card_basis(def).is_none(),
    label: "Tushar Chande tested RAVI and TurboMACD long trades on weekly S&P 500 closes (1980-1991). Runs on indices, ETFs and stocks, US and India, are a declared application.",
    basis: "Technical Analysis of Stocks & Commodities V.10:3 pp.108-114 (Adapting Moving Averages To Market Volatility); independent check authors/titles-1030/K16w/k16_ref.py",
};
