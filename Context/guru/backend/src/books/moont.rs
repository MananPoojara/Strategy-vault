//! MoonTide S&P system (daytradingforecasts.com), TradeStation open code. Research share USB HDD 1/TradeStation Systems &
//! Indicators/MoonTide S&P Sytem for TS (daytradingforecasts.com)/Open Code.rar::Open Code.txt (copy authors/batch-j/J1w/
//! moontide.txt). Book id `moont`. Lane K17 (titles loop, 2 Nov 2026), to-code row 309; independent check
//! authors/titles-1030/K17w/k17_ref.py. Orders run in `rrsim`.
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};
use super::rrsim::{g, o, pd, run_ts, sdef, sma, A, P};

pub const REVIEWED: &[&str] = &["moont_kurt_skew"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const NOTE: &str = "[Declared readings] The printed code: Kurt = Kurtosis(Close,40), Skewv = Skew(Close,15). When Kurt crosses under 0 (below 0 now, at or above 0 one bar earlier): Skewv > 0 -> buy at this bar's close, Skewv < 0 -> sell short at this bar's close (reversing a position). Trailing stop: MP = the position before this close's orders; on the first close of a long (MP = 1, MP[1] <> 1) StopPrice = Low - Average(Range,4) (short: High + Average(Range,4)); every close while long: sell next bar at StopPrice stop, then StopPrice moves a third of the way to the bar's low (short: buy to cover at StopPrice, then a third toward the high). So the first stop is placed at the close after the entry close. Kurtosis and Skew are TradeStation functions the file does not define: read as the population excess kurtosis m4/m2^2 - 3 and the population skewness m3/m2^1.5 of the closes (p.est = 1: the sample formulas, n(n+1)/((n-1)(n-2)(n-3)) Sum z^4 - 3(n-1)^2/((n-2)(n-3)) and n/((n-1)(n-2)) Sum z^3 with the sample SD). Range = High - Low. The file says S&P (the vendor sold an S&P day-trading forecast); daily indices, ETFs and US/India stocks are a declared application; one unit; the vault's standard costs. Kaufman's Kurtosis-Skew program (kfcd) uses the same pair of functions but is a different rule.";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![sdef("moont", "moont_kurt_skew", "MoonTide S&P: 40-day kurtosis crosses under zero, enter at the close in the direction of the 15-day skew; trailing stop a third of the way to each bar's low (high)",
        "MoonTide S&P system (TradeStation code)", "Long/Short", Status::Live, &[
            "F|\"Inputs: KurtLen(40), SkewLen(15);\" \"Kurt = Kurtosis(Close,KurtLen); Skewv = Skew(Close,SkewLen);\"",
            "E|\"If Kurt crosses under 0 then Begin If Skewv > 0 then Buy this bar at close; If Skewv < 0 then Sell Short this bar at close; end;\"",
            "X|\"If MP = 1 and MP[1] <> 1 then StopPrice = Low - Average(Range,4);\" \"If MP = 1 then Begin Sell next bar at StopPrice Stop; StopPrice = StopPrice + (Low - StopPrice)/3; End;\" (short mirrored)"],
        NOTE)]
}
pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    reviewed(id).then_some(("moont_kurt_skew", "MoonTide kurtosis / skew", "Kurtosis 40, Skew 15"))
}
pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    reviewed(id).then(|| vec![pd("est", "Kurtosis / Skew: 0 = population (excess) moments; 1 = sample (spreadsheet) formulas", 0., 0., 1., 1.)])
}

/// Excess kurtosis of the last n closes; NaN before the window or when the window is flat.
pub fn kurtosis(c: &[f64], n: usize, sample: bool) -> Vec<f64> {
    (0..c.len()).map(|i| if i + 1 < n { f64::NAN } else {
        let w = &c[i + 1 - n..=i]; let nf = n as f64; let m = w.iter().sum::<f64>() / nf;
        let m2 = w.iter().map(|x| (x - m) * (x - m)).sum::<f64>();
        if m2 <= 0. { return f64::NAN; }
        if sample {
            let sd = (m2 / (nf - 1.)).sqrt();
            let s4 = w.iter().map(|x| { let z = (x - m) / sd; (z * z) * (z * z) }).sum::<f64>();
            nf * (nf + 1.) / ((nf - 1.) * (nf - 2.) * (nf - 3.)) * s4 - 3. * (nf - 1.) * (nf - 1.) / ((nf - 2.) * (nf - 3.))
        } else {
            let m4 = w.iter().map(|x| { let z = x - m; (z * z) * (z * z) }).sum::<f64>() / nf;
            let v = m2 / nf;
            m4 / (v * v) - 3.
        }
    }).collect()
}
/// Skewness of the last n closes; NaN before the window or when the window is flat.
pub fn skew(c: &[f64], n: usize, sample: bool) -> Vec<f64> {
    (0..c.len()).map(|i| if i + 1 < n { f64::NAN } else {
        let w = &c[i + 1 - n..=i]; let nf = n as f64; let m = w.iter().sum::<f64>() / nf;
        let m2 = w.iter().map(|x| (x - m) * (x - m)).sum::<f64>();
        if m2 <= 0. { return f64::NAN; }
        if sample {
            let sd = (m2 / (nf - 1.)).sqrt();
            nf / ((nf - 1.) * (nf - 2.)) * w.iter().map(|x| { let z = (x - m) / sd; (z * z) * z }).sum::<f64>()
        } else {
            let m3 = w.iter().map(|x| { let z = x - m; (z * z) * z }).sum::<f64>() / nf;
            m3 / (m2 / nf).powf(1.5)
        }
    }).collect()
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.r.len() { return None; }
    let sample = g(p, "est", 0.) >= 0.5;
    let kurt = kurtosis(&x.c, 40, sample);
    let sk = skew(&x.c, 15, sample);
    let rng: Vec<f64> = x.h.iter().zip(&x.l).map(|(h, l)| h - l).collect();
    let ar = sma(&rng, 4);
    let (mut stop, mut mp1) = (0.0f64, 0i8);
    Some(run_ts(x, from, to, |i, st, now, next| {
        if i >= 1 && kurt[i] < 0. && kurt[i - 1] >= 0. {
            if sk[i] > 0. { now.push(o(A::Buy, P::Mkt, "LE")); }
            if sk[i] < 0. { now.push(o(A::Sell, P::Mkt, "SE")); }
        }
        let mp = st.pos;
        if mp == 1 && mp1 != 1 { stop = x.l[i] - ar[i]; }
        if mp == -1 && mp1 != -1 { stop = x.h[i] + ar[i]; }
        if mp == 1 { next.push(o(A::Xl, P::Stop(stop), "LX")); stop += (x.l[i] - stop) / 3.; }
        if mp == -1 { next.push(o(A::Xs, P::Stop(stop), "SX")); stop -= (stop - x.h[i]) / 3.; }
        mp1 = mp;
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn moontide() {
        super::super::nijat::tests::sane(run, REVIEWED);
        let k = kurtosis(&[1., 2., 3., 4.], 4, false);
        assert!((k[3] - (-1.36)).abs() < 1e-12);
        assert!(skew(&[1., 2., 3., 10.], 4, false)[3] > 0.);
    }
}

// Instrument admission: MoonTide S&P system 2026-11-02 (titles lane K17).
pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book == "moont" && reviewed(def.id),
    label: "Vendor S&P system code with no test shown; runs on daily indices, ETFs and stocks, US and India, are a declared application.",
    basis: "MoonTide S&P Sytem for TS (daytradingforecasts.com), Open Code.txt; independent check authors/titles-1030/K17w/k17_ref.py",
};
