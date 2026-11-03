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
const WHY: &str = "Held (decision 115): the article's reference deviations (6 and 4) are S&P 500 index points of the 1980s; a substitute would be our own value.";

pub fn catalogue() -> Vec<StrategyDef> {
    let d = |id, name, rules| StrategyDef { id, name, group: "S&C: Adapting Moving Averages To Market Volatility (Chande, V.10:3)", session: 0, dir: "Long", status: Status::NotTestable(WHY),
        needs_volume: false, needs_vix: false, needs_series: &[], book: "scchvid", rules, notes: NOTE, claim: None };
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
pub fn run(_id: &str, _x: &Ctx, _from: usize, _to: usize, _p: &Params) -> Option<Vec<Trade>> { None }

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn population_sd() { assert!((pstd(&[1., 2., 3., 4.], 4)[3] - 1.118033988749895).abs() < 1e-12); }
    #[test]
    fn vidya_cards_are_held() {
        let r = super::super::sctraub::tests::bars(300);
        let x = Ctx::new(&r, &r, &[]);
        for id in REVIEWED { assert!(run(id, &x, 0, r.len() - 1, &Params::new()).is_none() && family_of(id).is_some()); }
        assert!(catalogue().iter().all(|d| d.status != Status::Live));
    }
}

// Instrument admission: S&C Chande VIDYA 2026-11-02 (titles lane K16).
pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book == "scchvid" && reviewed(def.id),
    label: "Tushar Chande tested RAVI and TurboMACD long trades on weekly S&P 500 closes (1980-1991). Runs on indices, ETFs and stocks, US and India, are a declared application.",
    basis: "Technical Analysis of Stocks & Commodities V.10:3 pp.108-114 (Adapting Moving Averages To Market Volatility); independent check authors/titles-1030/K16w/k16_ref.py",
};
