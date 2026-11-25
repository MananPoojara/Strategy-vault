//! Dennis Meyers, "The Advance-Decline, New-High, New-Low Market System", Technical Analysis of Stocks & Commodities V14:2 (1996) p.69-75
//! (research share Trading Knowledge I/Stocks & Commodities DVD 11.26 (1982 - 2008) (traders.com)/VOLUMES/V14/C02/THEADVA.pdf).
//! Book id `dmadw`, under Dennis Meyers. Lane K3 (25 Nov 2026, R4 #398 row R191). Weekly bars; the daily twin is dmadr (V15:1).
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};
use super::dmadr::strength;
use super::rbtie::target_trades;

pub const REVIEWED: &[&str] = &["dmadw_weekly"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }
pub const SERIES: &[&str] = &["NYSE_ADV_DEC.Advances", "NYSE_ADV_DEC.Declines", "NYSE_NEW_HIGHS", "NYSE_NEW_LOWS", "DJ_INDUSTRIAL"];

const NOTE: &str = "[Adapted: 1 data adaptation + 2 unstated details, R4 #400] Weekly bars, orders on the week's last close; the sell reverses to short (the article lists short trades, Figures 1-3). Data adaptation: the article's weekly AI/DI are the issues that advanced/declined over the week (not a sum of daily counts); the vault holds daily NYSE counts only, so the weekly AI-DI is the sum of the week's daily AI-DI (declared, #340). Unstated 1: the weekly NYSE new highs/new lows are the last available daily count of the week. Unstated 2: when the buy and the sell condition are both true on one week the position is kept; flat until the first signal. The text's 'NLstr declining/NHstr rising' is the narrative of the level tests; the printed rules are the levels only (p.71). NYSE breadth 1965-03-01..2020-02-10, DJIA from Norgate.";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef { id: "dmadw_weekly", name: "Meyers weekly ADLrs-NH-NL: advance-decline line strength vs the Dow, with NYSE new-high/new-low strength, stop and reverse", group: "S&C: The Advance-Decline, New-High, New-Low Market System (Meyers, V14:2)",
        session: 0, dir: "Long/Short", status: Status::Live, needs_volume: false, needs_vix: false, needs_series: SERIES, book: "dmadw", rules: &[
            "F|ADL(week) = ADL(last week) + AI - DI; ADLstr = 100 (ADL - lowest ADL over x weeks)/(highest - lowest) - 50; DJstr the same on the Dow over x weeks; ADLrs = ADLstr - DJstr (p.70)",
            "F|NLstr = 100 (NL - lowest NL over l weeks)/(highest - lowest) - 50; NHstr likewise over h weeks (p.70)",
            "E|\"Buy rule: If ADLrs is greater than or equal to Brs and NLstr is less than or equal to NLbx and NHstr is greater than or equal to NHbx, then buy the S&P 500 on the close.\" (p.71)",
            "X|\"Sell rule: If ADLrs is less than or equal to Srs and NLstr is greater than or equal to NLsx and NHstr is less than or equal to NHsx, then sell the S&P 500 on the close.\" (p.71); the sell reverses to short",
            "F|Optimum values (p.72, Figures 1-3): Brs -30, Srs -30, xwks 5, NLbx 10, NLsx 30, lwks 8, NHbx -40, NHsx 40, hwks 4"],
        notes: NOTE, claim: crate::connors::claim("46", "78%", "752.49 S&P 500 points total net profit", "22 weeks (winners)", "S&P 500 weekly 1/6/1978-9/15/1995 (Figure 3)") }]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    reviewed(id).then_some(("dmadw_weekly", "Meyers weekly ADLrs-NH-NL", "Optimum values"))
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    reviewed(id).then(|| vec![p("brs", "Brs: ADLrs buy level", -30., -60., 60., 5.), p("srs", "Srs: ADLrs sell level", -30., -60., 60., 5.),
        p("xwks", "xwks: ADLstr/DJstr window (weeks)", 5., 2., 30., 1.), p("nlbx", "NLbx", 10., -50., 50., 10.), p("nlsx", "NLsx", 30., -50., 50., 10.),
        p("lwks", "lwks", 8., 2., 30., 1.), p("nhbx", "NHbx", -40., -50., 50., 10.), p("nhsx", "NHsx", 40., -50., 50., 10.), p("hwks", "hwks", 4., 2., 30., 1.)])
}

fn days(d: &str) -> i64 {
    let (y, m, dd): (i64, i64, i64) = (d[0..4].parse().unwrap_or(1970), d[5..7].parse().unwrap_or(1), d[8..10].parse().unwrap_or(1));
    let y2 = if m <= 2 { y - 1 } else { y };
    let era = y2.div_euclid(400);
    let yoe = y2 - era * 400;
    let doy = (153 * ((m + 9) % 12) + 2) / 5 + dd - 1;
    era * 146097 + yoe * 365 + yoe / 4 - yoe / 100 + doy - 719468
}
fn week(d: &str) -> i64 { (days(d) + 3).div_euclid(7) } // Monday-start weeks

/// Weekly system on daily arrays. Returns the daily target (position held from each week's last bar to the next).
pub fn weekly_targets(x: &Ctx, ai: &[f64], di: &[f64], nh: &[f64], nl: &[f64], dj: &[f64], t: [f64; 9], to: usize) -> Vec<i8> {
    let u = |v: f64| v.round().max(1.0) as usize;
    let (xw, lw, hw) = (u(t[2]), u(t[5]), u(t[8]));
    // week ends = last bar of each week
    let ends: Vec<usize> = (0..=to).filter(|&i| i == to || week(&x.r[i].date) != week(&x.r[i + 1].date)).collect();
    let (mut adl, mut s) = (0.0, 0usize);
    let (mut a, mut n_h, mut n_l, mut d) = (vec![], vec![], vec![], vec![]);
    for &e in &ends {
        let lastfin = |v: &[f64]| (s..=e).rev().map(|k| v[k]).find(|q| q.is_finite()).unwrap_or(f64::NAN);
        adl += (s..=e).map(|k| ai[k] - di[k]).filter(|q| q.is_finite()).sum::<f64>();
        a.push(adl); n_h.push(lastfin(nh)); n_l.push(lastfin(nl)); d.push(lastfin(dj));
        s = e + 1;
    }
    let mut tgt = vec![0i8; to + 1];
    let mut pos = 0i8;
    for (w, &e) in ends.iter().enumerate() {
        let rs = strength(&a, w, xw) - strength(&d, w, xw);
        let (nls, nhs) = (strength(&n_l, w, lw), strength(&n_h, w, hw));
        let buy = rs >= t[0] && nls <= t[3] && nhs >= t[6];
        let sell = rs <= t[1] && nls >= t[4] && nhs <= t[7];
        if buy != sell { pos = if buy { 1 } else { -1 }; } // both at once: the position is kept
        let next = ends.get(w + 1).copied().unwrap_or(to + 1);
        for k in e..next.min(to + 1) { tgt[k] = pos; } // set at the week's last close, held to the next week's last close
    }
    tgt
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from > to || to >= x.c.len() { return None; }
    let schema = params(id)?;
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).unwrap().def);
    let s: Vec<&[f64]> = match SERIES.iter().map(|n| x.series(n)).collect::<Option<Vec<_>>>() { Some(v) => v, None => return Some(vec![]) };
    let t = [g("brs"), g("srs"), g("xwks"), g("nlbx"), g("nlsx"), g("lwks"), g("nhbx"), g("nhsx"), g("hwks")];
    let tgt = weekly_targets(x, s[0], s[1], s[2], s[3], s[4], t, to);
    Some(target_trades(x, &tgt, from, to, false, "ADLrs-NH-NL weekly signal"))
}

pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book == "dmadw" && reviewed(def.id),
    label: "Dennis Meyers tested the weekly ADLrs-NH-NL system on the S&P 500 (1978-1995); it reads NYSE breadth and the DJIA close and runs on the US indices and ETFs SPX, SPY, DJI, DIA, NDX and QQQ only (R4 #306), with one declared data adaptation and two unstated details labelled on the card.",
    basis: "Dennis Meyers, The Advance-Decline, New-High, New-Low Market System, Stocks & Commodities V14:2 p.69-75 (VOLUMES/V14/C02/THEADVA.pdf); NYSE breadth from the vault (NYSE_ADV_DEC), new highs/lows and the DJIA from Norgate",
};
