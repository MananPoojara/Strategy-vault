//! Sierra Hotel (Mesa Systems, mesa-systems.com), TradeStation code "Open Code.txt" (research share USB HDD 1/TradeStation Systems &
//! Indicators/Mesa Sierra Hotel System for TS (mesa-systems.com)/Open Code.rar -> Open Code.txt). Book id `tssierra`. Lane K18
//! (titles loop, 2 Nov 2026), to-code row 342; independent check authors/titles-1030/K18w/k18_ref.py.
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};
use super::rrsim::{o, run_ts, A, P};

pub const REVIEWED: &[&str] = &["tssierra_hotel"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const NOTE: &str = "[Declared readings] TradeStation code as printed (mark 0, Nt2 43, pctup 2.4, pctdn 2.9, pctjup 1.14, pctjdn 1.2, prc = Close; njp is unused). Smoothed price: j = 43, bott = (j - 1)(j - 2), aa = 3 / j, a0 = (j + 3)(j + 2) / bott, a1 = -2 (4j + 7) / bott, a2 = 10 / bott; xplus = aa x sum over ii = 1..j of (a0 + a1 jj + a2 jj^2) x prc[ii - 1], jj = j - ii + 1. The code's first bar (CurrentBar 1) is read as the first bar with j closes (bar j - 1): there xplus = the close, the swing extremes h_ep = l_ep = xplus and the stops are set. Then each bar: bopos = bopos[1]; if bopos[1] >= 0, a new high in xplus raises h_ep and the sell stop (h_ep x (1 - 2.9%)), and xplus at or below the sell stop with the close below xplus[1] turns bopos to -1 (l_ep = xplus, buy stop = l_ep x 1.024); if bopos[1] <= 0 the mirror with the buy stop. Orders (market, next bar): when not long, a new bopos = +1 buys ('B%LSup'), and xplus > xplus[2] x 1.0114 sets bopos = +1 and buys ('Buy'); when not short, a new bopos = -1 sells short and xplus < xplus[2] x 0.988 sets bopos = -1 and sells short. Always in once started (stop and reverse). mark = 0, so the last-bar exit is off. Fills on the vault's TradeStation daily-bar engine (rrsim). No market is named and no results are printed. Runs on daily indices, ETFs and stocks, US and India, are a declared application.";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef { id: "tssierra_hotel", name: "Sierra Hotel: 43-bar polynomial-smoothed close, 2.4% / 2.9% reversal swings and 1.14% / 1.2% two-bar jumps, stop and reverse", group: "TradeStation code: Sierra Hotel (Mesa Systems)", session: 0, dir: "Long/Short", status: Status::Live,
        needs_volume: false, needs_vix: false, needs_series: &[], book: "tssierra", rules: &[
            "F|xplus = aa * sum((a0 + a1*jj + a2*jj*jj) * prc[ii-1], ii = 1..j), jj = j-ii+1, j = Nt2 = 43; h_ep / l_ep swing extremes of xplus with sellstop = h_ep*(1-pctdn/100), buystop = l_ep*(1+pctup/100)",
            "E|if marketposition<=0: if bopos>0 and bopos[1]<>bopos then Buy(\"B%LSup\") Next Bar at market; if xplus>xplus[2]*oneplus2 then begin bopos=1; ... Buy(\"Buy\") Next Bar at the market; end",
            "E|if marketposition>=0: if bopos<0 and bopos[1]<>bopos then Sell Short(\"S%LSdn\") Next Bar at market; if xplus<xplus[2]*oneminus2 then begin bopos=-1; ... Sell Short(\"Sell\") Next Bar at market; end",
            "X|No stop or target: each entry reverses the position (stop and reverse); mark = 0, no last-bar exit."],
        notes: NOTE, claim: None }]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> { reviewed(id).then_some(("tssierra_hotel", "Sierra Hotel (TradeStation)", "Nt2 43")) }

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    reviewed(id).then(|| vec![p("nt2", "Nt2 (43)", 43., 3., 200., 1.), p("pctup", "pctup (2.4)", 2.4, 0.1, 20., 0.1), p("pctdn", "pctdn (2.9)", 2.9, 0.1, 20., 0.1)])
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.r.len() { return None; }
    let g = |k: &str, d: f64| p.get(k).copied().unwrap_or(d);
    let j = g("nt2", 43.).round().max(3.);
    let (pctup, pctdn, pctjup, pctjdn) = (g("pctup", 2.4), g("pctdn", 2.9), 1.14, 1.2);
    let c = &x.c;
    let n = c.len();
    let ju = j as usize;
    let bott = (j - 1.) * (j - 2.);
    let (aa, a0, a1, a2) = (3. / j, (j + 3.) * (j + 2.) / bott, -2. * (4. * j + 7.) / bott, 10. / bott);
    let (onep, onem, onep2, onem2) = (1. + pctup / 100., 1. - pctdn / 100., 1. + pctjup / 100., 1. - pctjdn / 100.);
    let s0 = ju - 1;
    let mut xp = vec![f64::NAN; n];
    let (mut h_ep, mut l_ep, mut buystop, mut sellstop, mut bopos) = (0., 0., 0., 0., 0i8);
    Some(run_ts(x, from, to, |i, st, _now, next| {
        if i < s0 { return; }
        if i == s0 {
            xp[i] = c[i]; h_ep = xp[i]; l_ep = xp[i]; buystop = l_ep * onep; sellstop = h_ep * onem;
            return;
        }
        let mut sump = 0.;
        for ii in 1..=ju { let jj = j - ii as f64 + 1.; sump += (a0 + a1 * jj + a2 * jj * jj) * c[i + 1 - ii]; }
        xp[i] = aa * sump;
        let b1 = bopos;
        if b1 >= 0 {
            if xp[i] > h_ep { h_ep = xp[i]; sellstop = h_ep * onem; }
            if xp[i] <= sellstop && c[i] < xp[i - 1] { bopos = -1; l_ep = xp[i]; buystop = l_ep * onep; }
        }
        if b1 <= 0 {
            if xp[i] < l_ep { l_ep = xp[i]; buystop = l_ep * onep; }
            if xp[i] >= buystop && c[i] > xp[i - 1] { bopos = 1; h_ep = xp[i]; sellstop = h_ep * onem; }
        }
        let x2 = if i >= 2 { xp[i - 2] } else { f64::NAN };
        if st.pos <= 0 {
            if bopos > 0 && b1 != bopos { next.push(o(A::Buy, P::Mkt, "B%LSup")); }
            if xp[i] > x2 * onep2 { bopos = 1; h_ep = xp[i]; sellstop = h_ep * onem; next.push(o(A::Buy, P::Mkt, "Buy")); }
        }
        if st.pos >= 0 {
            if bopos < 0 && b1 != bopos { next.push(o(A::Sell, P::Mkt, "S%LSdn")); }
            if xp[i] < x2 * onem2 { bopos = -1; l_ep = xp[i]; buystop = l_ep * onep; next.push(o(A::Sell, P::Mkt, "Sell")); }
        }
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn sierra_hotel_reverses() {
        let r = super::super::sctraub::tests::bars(2500);
        let x = Ctx::new(&r, &r, &[]);
        let t = run("tssierra_hotel", &x, 0, r.len() - 1, &Params::new()).unwrap();
        assert!(t.len() > 3 && t.iter().any(|z| z.dir > 0.) && t.iter().any(|z| z.dir < 0.));
        assert!(run("tssierra_x", &x, 0, 5, &Params::new()).is_none());
    }
}

// Instrument admission: Sierra Hotel 2026-11-02 (titles lane K18).
pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book == "tssierra" && reviewed(def.id),
    label: "TradeStation trend system with no named market and no published results. Runs on daily indices, ETFs and stocks, US and India, are a declared application.",
    basis: "Mesa Systems Sierra Hotel Open Code.txt (printed code); independent check authors/titles-1030/K18w/k18_ref.py",
};
