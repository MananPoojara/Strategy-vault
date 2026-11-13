//! Dennis Meyers, "The T-Bond Futures & Stock Market Breadth System", Technical Analysis of Stocks & Commodities V15:11 (1997)
//! p.488-496 (research share Trading Knowledge I/Stocks & Commodities DVD 11.26 (1982 - 2008) (traders.com)/VOLUMES/V15/C11/THETBON.pdf).
//! Book id `dmtbb`, under Dennis Meyers. Lane PF (13 Nov 2026, A1 row 439; R4 #293, #297); independent check authors/pf/w/pf_ref.py.
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};
use super::dmadr::ema_first;
use super::rbtie::target_trades;

pub const REVIEWED: &[&str] = &["dmtbb_bond"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }
pub const SERIES: &[&str] = &["NYSE_ADV_DEC.Advances", "NYSE_ADV_DEC.Declines", "NYSE_ADV_DEC.AdvVolume", "NYSE_ADV_DEC.DecVolume", crate::futures::TBOND_SERIES];

const NOTE: &str = "[Adapted: 2 unstated details, R4 #297] Run bar by bar on the close; a buy covers the short and a sell closes the long (Figure 5 alternates Buy/Sell). ai/di/av/dv = NYSE advancing/declining issues and volume (vault NYSE_ADV_DEC, 1965-03-01..2020-02-10); tbond = the CBOT T-bond continuous future, back-adjusted close (Norgate &ZB_CCB, 1978-03-10..2020-09-11, roll method not stated by the vendor; R4 #293 (2): indicators on the back-adjusted series), read on the traded instrument's dates. Unstated in the article: (1) the T-bond contract series and roll (vault reading: Norgate's back-adjusted continuous contract); (2) the EMA starting value (vault reading: the first value, as TradeStation's XAverage). Readings shared with the live Turbo card (dmtad_turbo, same author and wording): 'lowest spx price found while in last sell' = the lowest close from the sell bar to today, and before the first opposite trade the confirmation is skipped; the second sell rule's 'davr[1] >= sellvol' is read as sellvolr. A day without a T-bond value keeps the average unchanged. The article tests the S&P 500 index; the runs here are on the US indices and ETFs only (NYSE breadth, R4 #306), ending with the breadth data.";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef { id: "dmtbb_bond", name: "Meyers T-bond & breadth: T-bond direction plus NYSE advance/decline thrusts, with a % confirmation from the last extreme, stop and reverse", group: "S&C: The T-Bond Futures & Stock Market Breadth System (Meyers, V15:11)",
        session: 0, dir: "Long/Short", status: Status::Live, needs_volume: false, needs_vix: false, needs_series: SERIES, book: "dmtbb", rules: &[
            "F|adr = ai/di; advr = av/dv; dar = di/ai; davr = dv/av; tbondx = tbondx[1] + a (tbond - tbondx[1]), a = 2/(1+2); tbDrB = tbondx - tbondx[Uplag]; tbDrS = tbondx - tbondx[Dnlag] (p.489-490)",
            "E|\"If tbDrB is positive for nb days in a row and spx is greater than the lowest spx price found while in last sell by at least pctud percent and adr is greater than or equal to buyr for xbuy days out of nbuy days and adr is greater than or equal to one for all of the nbuy days then buy the S&P 500 on the close. Or If tbDrB is positive for nb days in a row and spx is greater than the lowest spx price ... by at least pctud percent and advr is greater than or equal to buyvolr then buy\" (p.490)",
            "X|\"If tbDrS is negative for ns days in a row and spx is less than the highest spx price found while in last buy by at least pctud percent and dar is greater than or equal to the number sellr for xsell days out of nsell days then sell the S&P 500 on the close. Or If tbDrS is negative for ns days in a row and spx is less than the highest spx price ... and [davr >= sellvolr and davr[1] >= sellvolr2 Or davr[1] >= sellvol and davr >= sellvolr2] then sell\" (p.490-491)",
            "F|Final optimum values (p.493): Uplag 5, nb 2, Dnlag 21, ns 11; buyr 1.2, xbuy 2, nbuy 4; buyvolr 3.5; sellr 1.8, xsell 3, nsell 4; sellvolr 11, sellvolr2 2; pctud 1.75"],
        notes: NOTE, claim: None }]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    reviewed(id).then_some(("dmtbb_bond", "Meyers T-bond & breadth", "Final optimum values"))
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    reviewed(id).then(|| vec![p("uplag", "Uplag days", 5., 1., 40., 1.), p("nb", "nb: days tbDrB > 0", 2., 1., 20., 1.), p("dnlag", "Dnlag days", 21., 1., 60., 1.),
        p("ns", "ns: days tbDrS < 0", 11., 1., 30., 1.), p("buyr", "buyr", 1.2, 1., 3., 0.1), p("xbuy", "xbuy", 2., 1., 5., 1.), p("nbuy", "nbuy", 4., 1., 6., 1.),
        p("buyvolr", "buyvolr", 3.5, 1., 10., 0.5), p("sellr", "sellr", 1.8, 1., 3., 0.1), p("xsell", "xsell", 3., 1., 5., 1.), p("nsell", "nsell", 4., 1., 6., 1.),
        p("sellvolr", "sellvolr", 11., 2., 15., 0.5), p("sellvolr2", "sellvolr2", 2., 1., 5., 0.1), p("pctud", "pctud %", 1.75, 0., 5., 0.25)])
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from > to || to >= x.c.len() { return None; }
    let schema = params(id)?;
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).unwrap().def);
    let u = |k: &str| g(k).round().max(1.0) as usize;
    let s: Vec<&[f64]> = match SERIES.iter().map(|n| x.series(n)).collect::<Option<Vec<_>>>() { Some(v) => v, None => return Some(vec![]) };
    let (ai, di, av, dv) = (s[0], s[1], s[2], s[3]);
    let tbx = ema_first(&s[4][..=to], 2.0 / 3.0);
    let (uplag, nb, dnlag, ns, xbuy, nbuy, xsell, nsell) = (u("uplag"), u("nb"), u("dnlag"), u("ns"), u("xbuy"), u("nbuy"), u("xsell"), u("nsell"));
    let pct = g("pctud") / 100.0;
    let dr = |k: usize, lag: usize| if k >= lag { tbx[k] - tbx[k - lag] } else { f64::NAN };
    let in_row = |i: usize, n: usize, lag: usize, sign: f64| i + 1 >= n && (i + 1 - n..=i).all(|k| sign * dr(k, lag) > 0.0);
    let last = |i: usize, n: usize| (i + 1 >= n).then(|| i + 1 - n..=i);
    let c = &x.c;
    let mut tgt = vec![0i8; to + 1];
    let (mut pos, mut ext) = (0i8, f64::NAN); // ext: lowest close while short / highest close while long
    for i in 1..=to {
        if pos == 1 { ext = ext.max(c[i]); } else if pos == -1 { ext = ext.min(c[i]); }
        let buy = pos != 1 && in_row(i, nb, uplag, 1.0) && (pos == 0 || c[i] >= ext * (1.0 + pct)) && {
            let r1 = last(i, nbuy).is_some_and(|w| { let a: Vec<f64> = w.map(|k| ai[k] / di[k]).collect(); a.iter().filter(|&&v| v >= g("buyr")).count() >= xbuy && a.iter().all(|&v| v >= 1.0) });
            r1 || av[i] / dv[i] >= g("buyvolr")
        };
        let sell = !buy && pos != -1 && in_row(i, ns, dnlag, -1.0) && (pos == 0 || c[i] <= ext * (1.0 - pct)) && {
            let r1 = last(i, nsell).is_some_and(|w| w.filter(|&k| di[k] / ai[k] >= g("sellr")).count() >= xsell);
            let (d0, d1) = (dv[i] / av[i], dv[i - 1] / av[i - 1]);
            r1 || (d0 >= g("sellvolr") && d1 >= g("sellvolr2")) || (d1 >= g("sellvolr") && d0 >= g("sellvolr2"))
        };
        if buy || sell { pos = if buy { 1 } else { -1 }; ext = c[i]; }
        tgt[i] = pos;
    }
    Some(target_trades(x, &tgt, from, to, false, "T-bond & breadth signal"))
}

pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book == "dmtbb" && reviewed(def.id),
    label: "Dennis Meyers tested the T-bond & breadth system on the S&P 500's closes (1978-1997); it reads NYSE breadth and the CBOT T-bond future and runs on the US indices and ETFs SPX, SPY, DJI, DIA, NDX and QQQ only (R4 #306), with two unstated details read as labelled on the card.",
    basis: "Dennis Meyers, The T-Bond Futures & Stock Market Breadth System, Stocks & Commodities V15:11 p.488-496 (VOLUMES/V15/C11/THETBON.pdf); T-bond from Norgate &ZB_CCB (SOURCES norgate-continuous-futures-20200908); independent check authors/pf/w/pf_ref.py",
};
