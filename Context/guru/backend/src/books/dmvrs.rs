//! Dennis Meyers, "A/D Volume, New-High, New-Low System", Technical Analysis of Stocks & Commodities V14:7 (1996) p.302-310
//! (research share Trading Knowledge I/Stocks & Commodities DVD 11.26 (1982 - 2008) (traders.com)/VOLUMES/V14/C07/THEADVO.pdf).
//! Book id `dmvrs`, under Dennis Meyers. Lane PF (13 Nov 2026, A1 row 403; R4 #297); independent check authors/pf/w/pf_ref.py.
//! Same engine as dmadr (S&C V15:1) with the advance/decline VOLUME ratio in place of the issues ratio.
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};
use super::dmadr::rs_system;

pub const REVIEWED: &[&str] = &["dmvrs_daily"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }
pub const SERIES: &[&str] = &["NYSE_ADV_DEC.AdvVolume", "NYSE_ADV_DEC.DecVolume", "NYSE_NEW_HIGHS", "NYSE_NEW_LOWS", "DJ_INDUSTRIAL"];

const NOTE: &str = "[Adapted: 1 unstated detail, R4 #297] Run bar by bar on the close; 'buy' and 'sell' reverse the position (always in after the first signal, Figure 4). Advancing/declining volume = vault NYSE_ADV_DEC (1965-03-01..2020-02-10); NH/NL = NYSE 52-week new highs/lows (Norgate #NYSEHI/#NYSELO, SOURCES norgate-us-indices-20200908-pb); DJ = the DJIA close (Norgate $DJI, SOURCES norgate-us-indices-20200908-pf), read on the traded instrument's dates. Unstated in the article: the EMA starting value; vault reading: 0, as a TradeStation variable (forgotten within weeks). The text's 'Vrs is less than the level Vsell' (p.305) is read as the printed rule's 'less than or equal to'. A day without a breadth value keeps the averages unchanged; a zero range makes a strength undefined (no signal that day); if the buy and sell rules both hold on a bar (possible only with Vbuy <= Vsell), the position is kept (the reading the V15:3 trade list shows for this author's systems). The article tests the S&P 500 index; the runs here are on the US indices and ETFs only (NYSE breadth, R4 #306), ending with the breadth data.";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef { id: "dmvrs_daily", name: "Meyers Vrs-NH-NL: advance/decline volume strength vs the Dow, with NYSE new-high/new-low strength, stop and reverse", group: "S&C: A/D Volume, New-High, New-Low System (Meyers, V14:7)",
        session: 0, dir: "Long/Short", status: Status::Live, needs_volume: false, needs_vix: false, needs_series: SERIES, book: "dmvrs", rules: &[
            "F|Vratio = (Adv volume - Dec volume)/(Adv volume + Dec volume); XVratio = XVratio[1] + a (Vratio - XVratio[1]), a = 2/(1+10); Vstr = 100 (XVratio - lowest(XVratio,vdays))/range - 50; DJstr likewise on the Dow Jones average daily close over vdays; Vrs = Vstr - DJstr (p.303-304)",
            "F|XNH = XNH[1] + a (NH - XNH[1]), XNL likewise, a = 2/(1+3); NLstr = 100 (XNL - Lowest(XNL,ldays))/(Highest - Lowest) - 50; NHstr likewise over hdays (p.305)",
            "E|\"Buy rule: If Vrs is greater than or equal to the value Vbuy and NLstr is less than or equal to the value NLbuy and NHstr is greater than or equal to the value NHbuy, then buy the S&P 500 on the close.\" (p.305)",
            "X|\"Sell rule: If Vrs is less than or equal to the value Vsell and NLstr is greater than or equal to the value NLsell and NHstr is less than or equal to the value NHsell, then sell the S&P 500 on the close.\" (p.305); the sell reverses to short (Figure 4)",
            "F|Optimum values (p.307): Vbuy 5, Vsell -35, vdays 10; NLbuy -20, NLsell 30, ldays 40; NHbuy -10, NHsell 20, hdays 10"],
        notes: NOTE, claim: crate::connors::claim("43", "81%", "902.01 S&P 500 points total net profit", "119 bars (winners)", "S&P 500 daily 1/3/1978-3/29/1996 (Figure 3; trade list Figure 4)") }]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    reviewed(id).then_some(("dmvrs_daily", "Meyers Vrs-NH-NL", "Optimum values"))
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    reviewed(id).then(|| vec![p("vbuy", "Vbuy: Vrs buy level", 5., -40., 40., 5.), p("vsell", "Vsell: Vrs sell level", -35., -80., 20., 5.),
        p("vdays", "vdays: Vstr/DJstr lookback", 10., 4., 30., 2.), p("nlbuy", "NLbuy", -20., -50., 50., 10.), p("nlsell", "NLsell", 30., -50., 50., 10.),
        p("ldays", "ldays", 40., 5., 100., 5.), p("nhbuy", "NHbuy", -10., -50., 50., 10.), p("nhsell", "NHsell", 20., -50., 50., 10.), p("hdays", "hdays", 10., 3., 50., 1.)])
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from > to || to >= x.c.len() { return None; }
    let schema = params(id)?;
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).unwrap().def);
    let s: Vec<&[f64]> = match SERIES.iter().map(|n| x.series(n)).collect::<Option<Vec<_>>>() { Some(v) => v, None => return Some(vec![]) };
    let vr: Vec<f64> = (0..x.c.len()).map(|i| (s[0][i] - s[1][i]) / (s[0][i] + s[1][i])).collect();
    let t = [g("vbuy"), g("vsell"), g("vdays"), g("nlbuy"), g("nlsell"), g("ldays"), g("nhbuy"), g("nhsell"), g("hdays")];
    Some(rs_system(x, &vr, s[2], s[3], s[4], t, from, to, "Vrs-NH-NL signal"))
}

pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book == "dmvrs" && reviewed(def.id),
    label: "Dennis Meyers tested the Vrs-NH-NL system on the S&P 500's closes (1978-1996); it reads NYSE breadth and the DJIA close and runs on the US indices and ETFs SPX, SPY, DJI, DIA, NDX and QQQ only (R4 #306), with the unstated detail read as labelled on the card.",
    basis: "Dennis Meyers, A/D Volume, New-High, New-Low System, Stocks & Commodities V14:7 p.302-310 (VOLUMES/V14/C07/THEADVO.pdf); NYSE new highs/lows and the DJIA from Norgate (SOURCES norgate-us-indices-20200908-pb/-pf); independent check authors/pf/w/pf_ref.py",
};
