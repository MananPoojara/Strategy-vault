//! Greg Morris, InSync Index trader (G. Morris Corp.; EasyLanguage by Samuel K. Tennis, Vista Research and Trading, for TradeWind
//! Publishing, 1995). Research share USB HDD 1/TradeStation Systems & Indicators/_Misc/Greg Morris - Insync Index for ts/
//! Insync Index G. Morris.rar (read-only extracted copy authors/batch-j/J2w/gm-insync/: S0000.ASC system, I0000.ASC indicator,
//! F0000.ASC vrt.Ease.of.Movement, F0001.ASC vrt.Money.Flow.Index). Book id `gmins`. Lane K17 (titles loop, 2 Nov 2026), to-code
//! row 304, lead decisions 57, 82; independent check authors/titles-1030/K17w/k17_ref.py. Orders run in `rrsim`.
use crate::connors::{rsi, Ctx, ParamDef, Params, Status, StrategyDef, Trade};
use super::nijat::sdev;
use super::rrsim::{highest, lowest, o, run_ts, sdef, sma, xavg, A, P};

pub const REVIEWED: &[&str] = &["gmins_insync"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const NOTE: &str = "[Declared readings] The printed system S0000.ASC with its two printed functions. InSync = 50, +5 for each of ten readings in the sell zone and -5 for each in the buy zone: MFI(14) > 60 / < 40; BollB > 95 / < 5 with BollB = (C + 2SD - SMA20)/(4SD) x 100, SD = StdDev(C,20) (TradeStation's population SD; BollB keeps its last value when SD is 0); CCI(14) > +100 / < -100; FastD(14) > 80 / < 20; FastK(14) > 80 / < 20; RSI(14) > 65 / < 35; PDO: D = C - XAverage(C,18), S = XAverage(D,10): D - S > 0 and S > 0 / D - S < 0 and S < 0; MACD(12,25) = XAverage(C,12) - XAverage(C,25) with A = Average(MACD,9): A > 0 and A < MACD / A < 0 and A > MACD; ROC(10) = (C/C[10] - 1) x 100 with its 10-bar Average, same test; EOM with its 10-bar Average, same test. vrt.Money.Flow.Index as printed: Price = Close, its first value averages the last 14 close changes each times the current bar's volume, then UpAvg = (UpAvg[1] x 13 + up change x volume)/14 (DnAvg the same), MFI = 100 UpAvg/(UpAvg + DnAvg), 0 when both are 0. vrt.Ease.of.Movement = XAverage(((H+L)/2 - ((H+L)/2)[1]) / (MaxList(Volume,1000)/MaxList(Range, 1 point)), 13) x 100000, 1 point = one tick of the instrument (lead decision 57). TradeStation built-ins the files do not print, labelled platform readings: FastK(14) = 100 (C - lowest low 14)/(highest high 14 - lowest low 14) (0 on a flat range); FastD(14) = the 3-bar average of FastK; CCI(14) = (TP - Average(TP,14)) / (0.015 x mean absolute deviation), TP = H+L+C (0 when the deviation is 0); RSI = Wilder's RSI as the vault computes it; XAverage alpha 2/(n+1) seeded with the first value. The volume readings start at the first bar of each stretch with volume (with 14 earlier bars for MFI) and restart after a day without volume; InSync exists once all ten readings do. System: 'if InSync crosses below PreSel (95) then Sell on the close' (TradeStation 1995 syntax: a short entry) and 'crosses above PreBuy (5) then Buy on the close'; each entry reverses the other side (stop and reverse at the close). Needs the instrument's own volume. Morris tested no market in the file; daily indices, ETFs and US/India stocks are a declared application; one unit; the vault's standard costs.";

pub fn catalogue() -> Vec<StrategyDef> {
    let mut d = sdef("gmins", "gmins_insync", "InSync Index (Morris): ten overbought / oversold readings summed around 50; sell short when it falls back under 95, buy when it rises back over 5, at the close",
        "InSync Index trader (G. Morris Corp. / TradeWind 1995, TradeStation code)", "Long/Short", Status::Live, &[
            "F|\"InSync = 50 ;\" \"if (vrt.M.F.I > 60) then InSync = InSync + 5.0 ; if (BollB > 95) ... if (CCI (14) > +100) ... if (FastD (14) > 80) ... if (FastK (14) > 80) ... if (RSI (Close, 14) > 65) ...\" plus the PDO, MACD(12,25), RateOfChange(10) and EOM moving-average tests; the mirror tests subtract 5",
            "F|\"Return = xAverage (((High + Low) * 0.5 - ((High + Low) * 0.5)[1]) / (MaxList (Volume, 1000) / MaxList (Range, 1 point)), Length) ;\" (vrt.Ease.of.Movement); vrt.Money.Flow.Index printed in F0001.ASC",
            "E|\"if (InSync crosses below PreSel) then Sell (\\\"se.InSync\\\") on the close ; if (InSync crosses above PreBuy) then Buy (\\\"le.InSync\\\") on the close ;\" with \"Inputs: PreBuy (5), PreSel (95) ;\"",
            "X|No exit of its own: the opposite entry reverses the position"],
        NOTE);
    d.needs_volume = true;
    vec![d]
}
pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    reviewed(id).then_some(("gmins_insync", "Morris InSync Index", "PreBuy 5, PreSel 95"))
}
pub fn params(id: &str) -> Option<Vec<ParamDef>> { reviewed(id).then(Vec::new) }

/// Stretches [s, e] of consecutive bars with volume.
fn stretches(v: &[Option<f64>]) -> Vec<(usize, usize)> {
    let mut out = vec![];
    let mut s = None;
    for (i, x) in v.iter().enumerate() {
        match (x.is_some(), s) { (true, None) => s = Some(i), (false, Some(a)) => { out.push((a, i - 1)); s = None; } _ => {} }
    }
    if let Some(a) = s { out.push((a, v.len() - 1)); }
    out
}

/// The InSync index (NaN until all ten readings exist).
pub fn insync(x: &Ctx) -> Vec<f64> {
    let n = x.c.len();
    let (c, h, l) = (&x.c, &x.h, &x.l);
    let vol: Vec<Option<f64>> = x.r.iter().map(|r| r.volume).collect();
    let tick = x.market_tick();
    // MFI and EOM, restarted on each stretch with volume
    let (mut mfi, mut eom) = (vec![f64::NAN; n], vec![f64::NAN; n]);
    for (s, e) in stretches(&vol) {
        let k0 = s.max(14);
        let (mut ua, mut da) = (0.0f64, 0.0f64);
        for k in k0..=e {
            let v = vol[k].unwrap();
            if k == k0 {
                let (mut us, mut ds) = (0.0f64, 0.0f64);
                for j in 0..14 {
                    let up = c[k - j] - c[k - j - 1];
                    let (u, d) = if up >= 0. { (up, 0.) } else { (0., -up) };
                    us += u * v; ds += d * v;
                }
                ua = us / 14.; da = ds / 14.;
            } else {
                let up = c[k] - c[k - 1];
                let (u, d) = if up >= 0. { (up, 0.) } else { (0., -up) };
                ua = (ua * 13. + u * v) / 14.; da = (da * 13. + d * v) / 14.;
            }
            mfi[k] = if ua + da != 0. { 100. * ua / (ua + da) } else { 0. };
        }
        let k1 = s.max(1);
        if k1 > e { continue; }
        let raw: Vec<f64> = (k1..=e).map(|k| ((h[k] + l[k]) * 0.5 - (h[k - 1] + l[k - 1]) * 0.5) / (vol[k].unwrap().max(1000.) / (h[k] - l[k]).max(tick))).collect();
        for (j, v) in xavg(&raw, 13).into_iter().enumerate() { eom[k1 + j] = v * 100000.; }
    }
    let sd = sdev(c, 20, 0.);
    let m20 = sma(c, 20);
    let tp: Vec<f64> = (0..n).map(|i| h[i] + l[i] + c[i]).collect();
    let tpa = sma(&tp, 14);
    let fk: Vec<f64> = (0..n).map(|i| if i < 13 { f64::NAN } else {
        let (hh, ll) = (highest(h, 14, i), lowest(l, 14, i));
        if hh - ll > 0. { (c[i] - ll) / (hh - ll) * 100. } else { 0. }
    }).collect();
    let fd = sma(&fk, 3);
    let r14 = rsi(c, 14);
    let e18 = xavg(c, 18);
    let pd: Vec<f64> = (0..n).map(|i| c[i] - e18[i]).collect();
    let ps = xavg(&pd, 10);
    let (e12, e25) = (xavg(c, 12), xavg(c, 25));
    let macd: Vec<f64> = (0..n).map(|i| e12[i] - e25[i]).collect();
    let ma = sma(&macd, 9);
    let roc: Vec<f64> = (0..n).map(|i| if i < 10 { f64::NAN } else { (c[i] / c[i - 10] - 1.) * 100. }).collect();
    let ra = sma(&roc, 10);
    let ea = sma(&eom, 10);
    let mut bollb = 0.0f64;
    (0..n).map(|i| {
        let v1 = sd[i] * 2.;
        if v1.is_finite() && 2. * v1 != 0. { bollb = (c[i] + v1 - m20[i]) / (2. * v1) * 100.; }
        let cci = if i < 13 { f64::NAN } else {
            let md = tp[i - 13..=i].iter().map(|t| (t - tpa[i]).abs()).sum::<f64>() / 14.;
            if md > 0. { (tp[i] - tpa[i]) / (0.015 * md) } else { 0. }
        };
        let parts = [mfi[i], sd[i], cci, fd[i], fk[i], r14[i], ma[i], ra[i], ea[i], eom[i]];
        if parts.iter().any(|p| !p.is_finite()) { return f64::NAN; }
        let mut s = 50.0f64;
        let pdo_d = pd[i] - ps[i];
        let sell = [mfi[i] > 60., bollb > 95., cci > 100., fd[i] > 80., fk[i] > 80., r14[i] > 65., pdo_d > 0. && ps[i] > 0.,
            ma[i] > 0. && ma[i] < macd[i], ra[i] > 0. && ra[i] < roc[i], ea[i] > 0. && ea[i] < eom[i]];
        let buy = [mfi[i] < 40., bollb < 5., fk[i] < 20., fd[i] < 20., r14[i] < 35., cci < -100., pdo_d < 0. && ps[i] < 0.,
            ma[i] < 0. && ma[i] > macd[i], ra[i] < 0. && ra[i] > roc[i], ea[i] < 0. && ea[i] > eom[i]];
        for b in sell { if b { s += 5.; } }
        for b in buy { if b { s -= 5.; } }
        s
    }).collect()
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, _p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.r.len() { return None; }
    let v = insync(x);
    Some(run_ts(x, from, to, |i, _st, now, _next| {
        if i < 1 { return; }
        if v[i] < 95. && v[i - 1] >= 95. { now.push(o(A::Sell, P::Mkt, "se.InSync")); }
        if v[i] > 5. && v[i - 1] <= 5. { now.push(o(A::Buy, P::Mkt, "le.InSync")); }
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn insync_trader() {
        super::super::nijat::tests::sane(run, REVIEWED);
        let r = super::super::nijat::tests::bars(600);
        let x = Ctx::new(&r, &r, &[]);
        let v = insync(&x);
        assert!(v.iter().filter(|z| z.is_finite()).all(|z| (0. ..=100.).contains(z) && (z % 5.).abs() < 1e-12));
        assert!(stretches(&[None, Some(1.), Some(1.), None, Some(2.)]) == vec![(1, 2), (4, 4)]);
    }
}

// Instrument admission: Morris InSync Index trader 2026-11-02 (titles lane K17).
pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book == "gmins" && reviewed(def.id),
    label: "Morris's InSync trading system code (TradeWind 1995) with no market or test named; runs on daily indices, ETFs and stocks with their own volume, US and India, are a declared application.",
    basis: "Insync Index G. Morris.rar (S0000.ASC, F0000.ASC, F0001.ASC), USB HDD 1/TradeStation Systems & Indicators/_Misc; independent check authors/titles-1030/K17w/k17_ref.py",
};
