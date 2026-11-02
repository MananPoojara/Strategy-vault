//! Tushar S. Chande, "Adapting Moving Averages To Market Volatility", Technical Analysis of Stocks & Commodities V.10:3 (March
//! 1992) pp.108-114 (research share Trading Knowledge I/Stocks & Commodities DVD 11.26 (1982 - 2008) (traders.com)/VOLUMES/V10/C03/
//! ADAPTIN.pdf). Book id `scchvid`: the RAVI and TurboMACD long trades on weekly VIDYAs. Lane K16 (titles loop, 2 Nov 2026),
//! to-code row 236; independent check authors/titles-1030/K16w/k16_ref.py.
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};
use super::msws::sma;
use super::sctraub::period_ends;
use super::tselhs::Book;

pub const REVIEWED: &[&str] = &["scchvid_ravi", "scchvid_tmacd"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

/// Population standard deviation of the last n values (MetaStock Std Dev / Bollinger Bands).
pub fn pstd(v: &[f64], n: usize) -> Vec<f64> {
    let m = sma(v, n);
    (0..v.len()).map(|i| if !m[i].is_finite() { f64::NAN } else { (v[i + 1 - n..=i].iter().map(|x| (x - m[i]).powi(2)).sum::<f64>() / n as f64).sqrt() }).collect()
}

const NOTE: &str = "[Declared readings] Weekly closes = each calendar week's last close. VIDYA_L = 0.078 x s13/ref13 x C + (1 - 0.078 x s13/ref13) x VIDYA_L[-1] and VIDYA_S = 0.15 x s10/ref10 x C + (1 - ...) x VIDYA_S[-1] (equations 4-5), s = the standard deviation of the last 13 (10) weekly closes (Lotus @STD, the population form). The article's reference deviations 6 and 4 'represent a 10-year average for weekly Standard & Poor's 500 data' (1980s S&P points); as a declared reading the reference on every instrument is the average of the same deviation over the trailing 520 weeks (10 years) including the current week, so signals start ten years into the data. The smoothing factor is capped at 1 ('as long as the factor (1-t) is positive'); each VIDYA starts at the close of its first week with a reference. RAVI = VIDYA_S - VIDYA_L: buy when it turns positive from a negative value, sell (exit) when it turns negative. TurboMACD = RAVI - trigger, the trigger being 'an exponential moving average with a smoothing constant of 0.2' of RAVI (the default); option lit = 1 uses the printed equation 7 literally, trigger = 0.2 RAVI + 0.8 RAVI[-1]. Long only (the article tests long trades); fills at the signal week's close. Article (weekly S&P 500, January 1980 - July 1991, long trades): RAVI 9 trades, about +207 points with one open trade; TurboMACD 25 trades, 16 profitable (64%), about +218 points. Runs on indices, ETFs and stocks, US and India, are a declared application.";

pub fn catalogue() -> Vec<StrategyDef> {
    let d = |id, name, rules| StrategyDef { id, name, group: "S&C: Adapting Moving Averages To Market Volatility (Chande, V.10:3)", session: 0, dir: "Long", status: Status::Live,
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

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    let mut v = vec![p("refw", "Reference window, weeks (10 years)", 520., 52., 1040., 52.)];
    if id == "scchvid_tmacd" { v.push(p("lit", "1 = trigger as printed (0.2 RAVI + 0.8 RAVI[-1])", 0., 0., 1., 1.)); }
    reviewed(id).then_some(v)
}

fn vidya(c: &[f64], k: f64, n: usize, refw: usize) -> Vec<f64> {
    let s = pstd(c, n);
    let mut out = vec![f64::NAN; c.len()];
    let mut prev = f64::NAN;
    for i in 0..c.len() {
        if i + 1 < refw || !s[i + 1 - refw].is_finite() { continue; }
        let r = s[i + 1 - refw..=i].iter().sum::<f64>() / refw as f64;
        let a = if r > 0.0 { (k * s[i] / r).min(1.0) } else { 0.0 };
        prev = if prev.is_finite() { a * c[i] + (1.0 - a) * prev } else { c[i] };
        out[i] = prev;
    }
    out
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.r.len() { return None; }
    let refw = p.get("refw").copied().unwrap_or(520.).round().max(1.) as usize;
    let we = period_ends(x, false);
    let w: Vec<f64> = we.iter().map(|&i| x.c[i]).collect();
    let (vl, vs) = (vidya(&w, 0.078, 13, refw), vidya(&w, 0.15, 10, refw));
    let ravi: Vec<f64> = (0..w.len()).map(|k| vs[k] - vl[k]).collect();
    let sig: Vec<f64> = if id == "scchvid_ravi" { ravi.clone() } else if p.get("lit").copied().unwrap_or(0.) > 0.5 {
        (0..w.len()).map(|k| if k == 0 { f64::NAN } else { ravi[k] - (0.2 * ravi[k] + 0.8 * ravi[k - 1]) }).collect()
    } else {
        let mut e = f64::NAN;
        ravi.iter().map(|&r| { if r.is_finite() { e = if e.is_finite() { 0.2 * r + 0.8 * e } else { r }; r - e } else { f64::NAN } }).collect()
    };
    let mut pos = Book::new();
    for (k, &i) in we.iter().enumerate() {
        if i < from || i > to || k == 0 || !sig[k - 1].is_finite() { continue; }
        if pos.side() == 0.0 && sig[k] > 0.0 && sig[k - 1] < 0.0 { pos.enter(1.0, i, x.c[i], "close"); }
        else if pos.side() > 0.0 && sig[k] < 0.0 && sig[k - 1] > 0.0 { pos.exit(i, x.c[i], "turned negative"); }
    }
    Some(pos.done(to, x.c[to]))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn population_sd() { assert!((pstd(&[1., 2., 3., 4.], 4)[3] - 1.118033988749895).abs() < 1e-12); }
    #[test]
    fn vidya_weekly() {
        let r = super::super::sctraub::tests::bars(6000);
        let x = Ctx::new(&r, &r, &[]);
        let mut p = Params::new();
        p.insert("refw".into(), 104.0);
        for id in REVIEWED {
            let t = run(id, &x, 0, r.len() - 1, &p).unwrap();
            assert!(!t.is_empty() && t.iter().all(|z| z.dir > 0.0), "{id}");
        }
    }
}

// Instrument admission: S&C Chande VIDYA 2026-11-02 (titles lane K16).
pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book == "scchvid" && reviewed(def.id),
    label: "Tushar Chande tested RAVI and TurboMACD long trades on weekly S&P 500 closes (1980-1991). Runs on indices, ETFs and stocks, US and India, are a declared application.",
    basis: "Technical Analysis of Stocks & Commodities V.10:3 pp.108-114 (Adapting Moving Averages To Market Volatility); independent check authors/titles-1030/K16w/k16_ref.py",
};
