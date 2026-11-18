//! Buff Pelz Dormeier, "Price & Volume, Digging Deeper" (VPCI white paper, Wachovia Securities, c.2007; research share
//! Trading Knowledge III/!PDFs/Price & Volume, Digging Deeper.pdf, 13 pp). Book id `dpvdd`. Lane CV2 (18 Nov 2026).
//! The paper's own SPY test (p.11-13): long when ADX(7) >= 30, MACD(12,26,9) gives a buy and VPCI(7/28) crosses above
//! "VPCI smoothed"; exit when VPCI crosses under it. The Traders' Tips strategy (MACD crossovers, VPCI 5/20) is scdvpci.
use crate::connors::{sma, Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};

pub const REVIEWED: &[&str] = &["dpvdd_vpci_adx_macd"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const NOTE: &str = "[Complete printed test; four labelled readings; UNVERIFIED (R4 #339/#343)] Source: Dormeier, 'Price & Volume, Digging Deeper', p.11-13 (VPCI vs OBV test on the SPDR S&P 500, Feb 1993-2006, 'VPCI 7/28 {7 day short-term trend & 7*4 day long-term trend}', 'ADX equals or is greater than 30', 'MACD (12, 26, 9)'; long positions are exited with VPCI crossunders). Definition printed p.4-6: VWMA(n) = sum(close x volume)/sum(volume); VPC = VWMA(28) - SMA(28); VPR = VWMA(7)/SMA(7); VM = SMA(volume,7)/SMA(volume,28); VPCI = VPC x VPR x VM. Readings: (1) 'VPCI smoothed' = volume-weighted average of VPCI over 8 bars (the paper's smoothed-OBV trigger is an 8-bar average; VPCI smoothed length is not printed); (2) 'MACD buy signal' = MACD line above its 9-bar signal line on the signal bar (not a fresh crossover); MACD uses exponential averages seeded with the first close; (3) ADX(7) = Wilder's built-in ADX (+DM/-DM/TR summed over the first 7 bars then S - S/7 + x; DX = 100|+DI - -DI|/(+DI + -DI); ADX = mean of the first 7 DX then (prev x 6 + DX)/7), Wilder 1978, cited as footnote 11 of the paper; (4) all three entry conditions are read on the bar of the VPCI cross-over, filled at that bar's close; the exit fills at the close of the cross-under bar. The paper's MACD-only and OBV versions are its comparison systems and are not coded. Book figures (Table 2, SPY 1993-2006, no dividends): VPCI 8.11% a year, 35.63% of time invested, 65.15% profitable, profit factor 2.47; Table 3 gives the yearly returns.";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef { id: "dpvdd_vpci_adx_macd", name: "Dormeier VPCI(7/28) cross-over above its smoothed line with ADX(7) >= 30 and MACD buy, exit on cross-under",
        group: "Price & Volume, Digging Deeper (Dormeier) - SPY test", session: 0, dir: "Long", status: Status::Live, needs_volume: true, needs_vix: false, needs_series: &[], book: "dpvdd",
        rules: &["F|\"VPC = VWMA - SMA (long-term); VPR = short VWMA / short SMA; VM = short average volume / long average volume; VPCI = VPC x VPR x VM\" (p.4-6); 'VPCI 7/28' (p.12)",
            "E|\"long positions are taken only when the above conditions are met when accompanied by ... VPCI crossovers\": ADX(7) >= 30, MACD(12,26,9) buy, and VPCI crosses above VPCI smoothed (p.11-12)",
            "X|\"Long positions are exited with ... VPCI crossunders\" (p.12): VPCI crosses under VPCI smoothed"],
        notes: NOTE, claim: None }]
}
pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    reviewed(id).then_some(("dpvdd_vpci_adx_macd", "Dormeier VPCI + ADX + MACD (SPY test)", "7/28, ADX 7 >= 30"))
}
pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    reviewed(id).then(|| vec![p("s", "VPCI short length (book 7)", 7., 2., 50., 1.), p("l", "VPCI long length (book 28)", 28., 5., 200., 1.),
        p("adx_n", "ADX length (book 7)", 7., 2., 50., 1.), p("adx_min", "ADX threshold (book 30)", 30., 0., 60., 1.),
        p("sm", "VPCI smoothed length (reading 8)", 8., 2., 50., 1.)])
}

fn ema0(v: &[f64], n: usize) -> Vec<f64> {
    let k = 2. / (n as f64 + 1.);
    let mut out = Vec::with_capacity(v.len());
    for (i, &x) in v.iter().enumerate() { out.push(if i == 0 { x } else { out[i - 1] + k * (x - out[i - 1]) }); }
    out
}

/// Wilder ADX (see NOTE reading 3); NaN until 2n bars.
fn adx(h: &[f64], l: &[f64], c: &[f64], n: usize) -> Vec<f64> {
    let len = c.len();
    let mut out = vec![f64::NAN; len];
    if len < 2 * n + 1 { return out; }
    let (mut pdm, mut mdm, mut tr) = (vec![0.; len], vec![0.; len], vec![0.; len]);
    for i in 1..len {
        let (up, dn) = (h[i] - h[i - 1], l[i - 1] - l[i]);
        pdm[i] = if up > dn && up > 0. { up } else { 0. };
        mdm[i] = if dn > up && dn > 0. { dn } else { 0. };
        tr[i] = (h[i] - l[i]).max((h[i] - c[i - 1]).abs()).max((l[i] - c[i - 1]).abs());
    }
    let nf = n as f64;
    let (mut sp, mut sm, mut st) = (pdm[1..=n].iter().sum::<f64>(), mdm[1..=n].iter().sum::<f64>(), tr[1..=n].iter().sum::<f64>());
    let mut dx = vec![f64::NAN; len];
    for i in n..len {
        if i > n { sp = sp - sp / nf + pdm[i]; sm = sm - sm / nf + mdm[i]; st = st - st / nf + tr[i]; }
        let (pi, mi) = (100. * sp / st, 100. * sm / st);
        dx[i] = if pi + mi == 0. { 0. } else { 100. * (pi - mi).abs() / (pi + mi) };
    }
    let mut a = dx[n..2 * n].iter().sum::<f64>() / nf;
    out[2 * n - 1] = a;
    for i in 2 * n..len { a = (a * (nf - 1.) + dx[i]) / nf; out[i] = a; }
    out
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.r.len() { return None; }
    let g = |k: &str, d: f64| p.get(k).copied().unwrap_or(d);
    let (s, l, an, sm) = (g("s", 7.).round().max(1.) as usize, g("l", 28.).round().max(1.) as usize, g("adx_n", 7.).round().max(2.) as usize, g("sm", 8.).round().max(1.) as usize);
    let amin = g("adx_min", 30.);
    let n = x.c.len();
    let v: Vec<f64> = x.r.iter().map(|r| r.volume.filter(|v| *v > 0.).unwrap_or(f64::NAN)).collect();
    let pv: Vec<f64> = (0..n).map(|i| x.c[i] * v[i]).collect();
    let (pvs, vs, pvl, vl) = (sma(&pv, s), sma(&v, s), sma(&pv, l), sma(&v, l));
    let (cs, cl) = (sma(&x.c, s), sma(&x.c, l));
    let vp: Vec<f64> = (0..n).map(|i| (pvl[i] / vl[i] - cl[i]) * (pvs[i] / vs[i] / cs[i]) * (vs[i] / vl[i])).collect();
    let vpv: Vec<f64> = (0..n).map(|i| vp[i] * v[i]).collect();
    let (a, b) = (sma(&vpv, sm), sma(&v, sm));
    let sg: Vec<f64> = (0..n).map(|i| a[i] / b[i]).collect(); // sum(VPCI x V)/sum(V) over sm bars
    let m: Vec<f64> = { let (e12, e26) = (ema0(&x.c, 12), ema0(&x.c, 26)); (0..n).map(|i| e12[i] - e26[i]).collect() };
    let ms = ema0(&m, 9);
    let ad = adx(&x.h, &x.l, &x.c, an);
    let mut out: Vec<Trade> = vec![];
    let mut cur: Option<usize> = None;
    for i in from.max(1)..=to {
        let (up, dn) = (vp[i] > sg[i] && vp[i - 1] <= sg[i - 1], vp[i] < sg[i] && vp[i - 1] >= sg[i - 1]);
        if let Some(e) = cur {
            if dn { out.push(Trade { dir: 1., legs: vec![Leg { i: e, px: x.c[e], w: 1. }], exit_i: i, exit_px: x.c[i], reason: "VPCI crosses under VPCI smoothed".into(), entry_kind: "close", signal_i: i, open: false, exits: vec![] }); cur = None; }
        } else if up && ad[i] >= amin && m[i] > ms[i] { cur = Some(i); }
    }
    if let Some(e) = cur { out.push(Trade { dir: 1., legs: vec![Leg { i: e, px: x.c[e], w: 1. }], exit_i: to, exit_px: x.c[to], reason: "open".into(), entry_kind: "close", signal_i: to, open: true, exits: vec![] }); }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn vpci_adx_macd() {
        let r = super::super::sctraub::tests::bars(3000);
        let x = Ctx::new(&r, &r, &[]);
        let t = run("dpvdd_vpci_adx_macd", &x, 0, r.len() - 1, &Params::new()).unwrap();
        assert!(t.iter().all(|z| z.dir > 0. && z.legs[0].px == x.c[z.legs[0].i] && z.exit_i > z.legs[0].i));
        let ad = adx(&x.h, &x.l, &x.c, 7);
        assert!(t.iter().all(|z| ad[z.legs[0].i] >= 30.));
    }
}

// Instrument admission: Dormeier VPCI paper (SPY test) 2026-11-18 (lane CV2).
pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book == "dpvdd" && reviewed(def.id),
    label: "Dormeier tested the rule on the SPDR S&P 500 (SPY); other ETFs, indices with volume and stocks, US and India, are a declared application.",
    basis: "Dormeier, Price & Volume, Digging Deeper, p.11-13 (research share Trading Knowledge III/!PDFs); independent check authors/convert-1118/CV2/ref/dpvdd_ref.py",
};
