//! Sylvain Vervoort, "The Quest For Reliable Crossovers", Technical Analysis of Stocks & Commodities V.26:5 (May 2008) pp.20-25
//! (research share Trading Knowledge I/Stocks & Commodities DVD 11.26 (1982 - 2008) (traders.com)/VOLUMES/V26/C05/083VERV.pdf).
//! Book id `scvtema`. Lane C7 (second pass, 17 Nov 2026); independent check authors/convert-1117/C7/ref_vt/c7vt_ref.py.
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};
use super::msws::ema;
use super::rbtie::target_trades;

pub const REVIEWED: &[&str] = &["scvtema_55"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const NOTE: &str = "[Adapted: 3 unstated details, R4 #297 / #339] The article's MetaStock buying formula, with its fixed 55-day average: heikin-ashi open haOpen = (previous (O+H+L+C)/4 + previous haOpen)/2, haC = ((O+H+L+C)/4 + haOpen + Max(H,haOpen) + Min(L,haOpen))/4; TMA1 = Tema(haC,55), TMA2 = Tema(TMA1,55), ZlHa = TMA1 + (TMA1 - TMA2); the same on the typical price Typ() = (H+L+C)/3 gives ZlCl; buy when cross(ZlCl, ZlHa) (the typical-price line crosses above the heikin-ashi line), sell when cross(ZlHa, ZlCl) ('Similar to the buying formula except the last line ... the crossover the other way around'); long only; 'Buying and selling at the closing price of the signaling day'. Tema(x,n) = 3*Mov(x,n,E) - 3*Mov(Mov(x,n,E),n,E) + Mov(Mov(Mov(x,n,E),n,E),n,E), as printed in the article; Mov(x,n,E) is MetaStock's exponential average (alpha 2/(n+1), seeded with the first value, shown from the n-th value). [Unstated details] (1) the first haOpen, which the formula leaves to MetaStock's PREV, is (Open+Close)/2 of the first bar; (2) cross(a,b) is read as a above b today and a at or below b on the previous bar, signals only once all the averages exist; (3) the article's costs of 0.1% on each side are not charged (the engine reports gross trades). [Book result] 211 US stocks from a list of ten-baggers, $1,000 each, March 2003 to 16 May 2007, 0.1% costs each side: profit $413,537 (196%), 37 trades per stock on average; that stock list is not in the vault, so no like-for-like match is claimed. The card runs on any actual index, ETF or stock. UNVERIFIED.";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef { id: "scvtema_55", name: "Vervoort crossover: 55-day zero-lagging TEMA of the typical price crosses the zero-lagging TEMA of the heikin-ashi close", group: "S&C: The Quest For Reliable Crossovers (Vervoort, V.26:5)", session: 0, dir: "Long", status: Status::Live,
        needs_volume: false, needs_vix: false, needs_series: &[], book: "scvtema", rules: &[
            "F|\"TMA1:= Tema(haC,avg); TMA2:= Tema(TMA1,avg); Diff:= TMA1 - TMA2; ZlHa:= TMA1 + Diff; TMA1:= Tema(typ(),avg); TMA2:= Tema(TMA1,avg); Diff:= TMA1 - TMA2; ZlCl:= TMA1 + Diff\" with \"Avg:=55\" (Crossovers sidebar)",
            "E|\"cross(ZlCl,ZlHa) {crossover of the zero-lagging TEMA on the typical price and the zero-lagging TEMA of the heikin-ashi closing price}\" (Buying formula, MetaStock)",
            "X|\"Selling formula (MetaStock): Similar to the buying formula except the last line, which makes the crossover the other way around - that is, cross(ZlHa,ZlCl)\"; \"Buying and selling at the closing price of the signaling day\"; long positions only"],
        notes: NOTE, claim: None }]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    reviewed(id).then_some(("scvtema_55", "Vervoort zero-lag TEMA crossover (S&C)", "55 days"))
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    reviewed(id).then(|| vec![p("avg", "TEMA period (article 55)", 55., 5., 150., 1.)])
}

fn tema(v: &[f64], n: usize) -> Vec<f64> {
    let (e1, e2) = (ema(v, n), ema(&ema(v, n), n));
    let e3 = ema(&e2, n);
    (0..v.len()).map(|i| 3. * e1[i] - 3. * e2[i] + e3[i]).collect()
}
fn zl(v: &[f64], n: usize) -> Vec<f64> {
    let (t1, t2) = (tema(v, n), tema(&tema(v, n), n));
    (0..v.len()).map(|i| t1[i] + (t1[i] - t2[i])).collect()
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from > to || to >= x.c.len() { return None; }
    let n = p.get("avg").copied().unwrap_or(55.).round().max(2.) as usize;
    let len = to + 1;
    let mut hac = vec![f64::NAN; len];
    let mut hao = f64::NAN;
    let mut prev_xc = f64::NAN;
    for i in 0..len {
        let xc = (x.o[i] + x.h[i] + x.l[i] + x.c[i]) / 4.;
        hao = if i == 0 { (x.o[0] + x.c[0]) / 2. } else { (prev_xc + hao) / 2. };
        hac[i] = (xc + hao + x.h[i].max(hao) + x.l[i].min(hao)) / 4.;
        prev_xc = xc;
    }
    let typ: Vec<f64> = (0..len).map(|i| (x.h[i] + x.l[i] + x.c[i]) / 3.).collect();
    let (zha, zcl) = (zl(&hac, n), zl(&typ, n));
    let (mut tgt, mut pos) = (vec![0i8; len], 0i8);
    for i in 1..len {
        if zha[i].is_finite() && zcl[i].is_finite() && zha[i - 1].is_finite() && zcl[i - 1].is_finite() {
            if zcl[i] > zha[i] && zcl[i - 1] <= zha[i - 1] { pos = 1; }
            else if zha[i] > zcl[i] && zha[i - 1] <= zcl[i - 1] { pos = 0; }
        }
        tgt[i] = pos;
    }
    Some(target_trades(x, &tgt, from, to, false, "zero-lag TEMA crossover"))
}

pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book == "scvtema" && reviewed(def.id),
    label: "Sylvain Vervoort tested the crossover on 211 US stocks (March 2003 to May 2007, 0.1% costs each side); runs on actual indices, ETFs and stocks, US and India, are a declared application, not the author's test, with the unstated details read as labelled on the card.",
    basis: "Technical Analysis of Stocks & Commodities V.26:5 pp.20-25 (The Quest For Reliable Crossovers); independent check authors/convert-1117/C7/ref_vt/c7vt_ref.py",
};

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn vtema_has_catalogue_and_params() {
        assert!(family_of("scvtema_55").is_some() && params("scvtema_55").is_some());
        let r = super::super::sctraub::tests::bars(400);
        let x = Ctx::new(&r, &r, &[]);
        assert!(run("scvtema_55", &x, 0, r.len() - 1, &Params::new()).is_some());
    }
}
