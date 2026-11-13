//! Dennis Meyers, "Making The Titanic Fly", Technical Analysis of Stocks & Commodities V.13:5 (1995) p.189-195, with the
//! sidebar "TradeStation code" (research share Trading Knowledge I/Stocks & Commodities DVD 11.26 (1982 - 2008) (traders.com)/
//! VOLUMES/V13/C05/MAKING.PDF and SIDETRD.PDF). Book id `dmtfly`, under Dennis Meyers. Lane PB (13 Nov 2026, pool B rows 393/394
//! of authors/a1/A1.tsv; R4 #297); independent check authors/pb/w/pb_ref.py.
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};
use super::rbtie::target_trades;

pub const REVIEWED: &[&str] = &["dmtfly_flying"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }
pub const SERIES: &[&str] = &["NYSE_ADV_DEC.Advances", "NYSE_ADV_DEC.Declines", "NYSE_ADV_DEC.AdvVolume", "NYSE_ADV_DEC.DecVolume", "NYSE_NEW_HIGHS", "NYSE_NEW_LOWS"];

const NOTE: &str = "[Adapted: 2 unstated details, R4 #297; verified on the book window] The sidebar's TradeStation 3.01 system, run bar by bar on the close (TradeStation 'buy'/'sell' reverse the position: always in after the first signal). ai/di = NYSE advancing/declining issues, av/dv = their volumes (vault NYSE_ADV_DEC, 1965-2020-02-10), nnh/nnl = NYSE 52-week new highs/lows (Norgate #NYSEHI/#NYSELO, to 2020-09-04); hhv = highest high of the last 252 bars. B1: the close fell more than 9% below hhv (stays set until a buy condition B2/B4/B5 fires). B2: ai >= 2 di today and yesterday and ai >= 5 di on one of them. B4: ai > 1.75 di on at least 3 of the last 4 days. B5: av >= 9 dv today while dv < 9 av on each of the two days before. A new 52-week high (high above the previous bar's hhv) sets S1 and clears S2-S5; S2: a new high in the last 8 bars and nnl > nnh on one of the last 8 days; S3/S4/S5 mirror B2/B4/B5 for declines (S5: dv > 9 av). The S flags persist until a new high or a sell (TradeStation variables). Buy on B1 with B2, B4 or B5 (B3 is deleted in the code), or when the high reaches 2% above the last new high's price (the '2%stop', a level set while long); sell on S1 + S2 + one of S3/S4/S5. Unstated in the code: (1) the 2% stop level before the first long (vault reading: none until the first long; TradeStation's initial value is the first bar's high); (2) a buy and a sell on the same close (vault reading: the sell, the later order, decides the position; no zero-length trade). Meyers tested the DJIA 1/3/1978-1/20/1995 (Figure 8): 28 trades, 64% profitable, total net profit 3,989.72 points; long 14 trades 86%, short 14 trades 43%. The vault's DJI over the same dates gives 29 trades, 62.1%, +3,992 points, longs 12 of 14 (book check passes); the published run uses the longest stretch without NYSE breadth gaps. NYSE breadth: runs on the US indices and ETFs DJI, DIA, SPX, SPY, NDX, QQQ only (as R4 #306); results end 2020-02-10, where the vault's NYSE volume ends.";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef { id: "dmtfly_flying", name: "Meyers flying Titanic: buy breadth thrusts after a 9% drop, sell NYSE new-low divergence after a 52-week high", group: "S&C: Making The Titanic Fly (Meyers, V.13:5)",
        session: 0, dir: "Long/Short", status: Status::Live, needs_volume: false, needs_vix: false, needs_series: SERIES, book: "dmtfly", rules: &[
            "F|input: ad2x(2),ad4x(5),adfx(1.75),adv9x(9),tenpct(0.09),lookbk(252); hhv=highest(h,lookbk); if c < hhv-(tenpct*hhv) then tendrop=1; if h > hhv[1] then djnh=1 (sidebar)",
            "E|B2: (ai >= ad2x*di) and (ai[1] >= ad2x*di[1]) and ((ai >= ad4x*di) or (ai[1] >= ad4x*di[1])); B4: ai[k] > adfx*di[k] on 3 of k=0..3; B5: av >= adv9x*dv and dv[1] < adv9x*av[1] and dv[2] < adv9x*av[2]; buy on close if tendrop and (B2 or B4 or B5), or if h >= djnhstop (= djnhprc*1.02 while long) (sidebar)",
            "X|S1: a DJ 52-week new high occurred; S2: nnl > nnh within 7 days of a new high; S3/S4: the B2/B4 tests on declines; S5: dv > adv9x*av; sell on close if S1 and S2 and (S3 or S4 or S5) (sidebar)"],
        notes: NOTE, claim: crate::connors::claim("28", "64%", "3,989.72 DJIA points total net profit", "n/a", "DJIA daily 1/3/1978-1/20/1995 (Figure 8)") }]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    reviewed(id).then_some(("dmtfly_flying", "Meyers flying Titanic", "Sidebar inputs"))
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    reviewed(id).then(|| vec![p("ad2x", "ad2x: two-day A/D ratio (2)", 2., 1., 5., 0.25), p("ad4x", "ad4x: one-day A/D ratio (5)", 5., 2., 10., 0.5),
        p("adfx", "adfx: 3-of-4-day A/D ratio (1.75)", 1.75, 1., 4., 0.25), p("adv9x", "adv9x: volume ratio (9)", 9., 3., 15., 0.5),
        p("tenpct", "tenpct: drop from the 52-week high (0.09)", 0.09, 0.03, 0.2, 0.01), p("lookbk", "lookbk: bars in the 52-week high (252)", 252., 50., 300., 1.)])
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from > to || to >= x.c.len() { return None; }
    let schema = params(id)?;
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).unwrap().def);
    let s: Vec<&[f64]> = match SERIES.iter().map(|n| x.series(n)).collect::<Option<Vec<_>>>() { Some(v) => v, None => return Some(vec![]) };
    let (ai, di, av, dv, nnh, nnl) = (s[0], s[1], s[2], s[3], s[4], s[5]);
    let (ad2x, ad4x, adfx, adv9x, tenpct) = (g("ad2x"), g("ad4x"), g("adfx"), g("adv9x"), g("tenpct"));
    let lb = g("lookbk").round().max(2.) as usize;
    let (h, c) = (&x.h, &x.c);
    let hhv = |i: usize| h[i + 1 - lb..=i].iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let mut tgt = vec![0i8; to + 1];
    let mut djnh = vec![false; to + 1];
    let (mut pos, mut tendrop, mut djnhx) = (0i8, false, false);
    let (mut djnhprc, mut djnhstop) = (f64::NAN, f64::INFINITY);
    let mut sf = [false; 6]; // s1..s5 (index 1..5), persistent like TradeStation vars
    for i in lb.max(8)..=to {
        let hv = hhv(i);
        if c[i] < hv - tenpct * hv { tendrop = true; }
        if h[i] > hhv(i - 1) { djnh[i] = true; djnhprc = h[i]; for k in 2..=5 { sf[k] = false; } }
        if pos == 1 { djnhstop = djnhprc * 1.02; }
        if djnh[i] { djnhx = true; }
        let (mut b2, mut b4, mut b5) = (false, false, false);
        if tendrop {
            b2 = ai[i] >= ad2x * di[i] && ai[i - 1] >= ad2x * di[i - 1] && (ai[i] >= ad4x * di[i] || ai[i - 1] >= ad4x * di[i - 1]);
            b4 = (0..=3).filter(|&k| ai[i - k] > adfx * di[i - k]).count() >= 3;
            b5 = av[i] >= adv9x * dv[i] && dv[i - 1] < adv9x * av[i - 1] && dv[i - 2] < adv9x * av[i - 2];
        }
        if djnhx {
            sf[1] = true;
            let recent_high = (0..=7).any(|k| djnh[i - k]);
            if recent_high && (0..=7).any(|k| nnl[i - k] > nnh[i - k]) { sf[2] = true; }
            if di[i] >= ad2x * ai[i] && di[i - 1] >= ad2x * ai[i - 1] && (di[i] >= ad4x * ai[i] || di[i - 1] >= ad4x * ai[i - 1]) { sf[3] = true; }
            if (0..=3).filter(|&k| di[i - k] > adfx * ai[i - k]).count() >= 3 { sf[4] = true; }
            if dv[i] > adv9x * av[i] { sf[5] = true; }
        }
        let buy = (tendrop && (b2 || b4 || b5)) || h[i] >= djnhstop;
        if buy { pos = 1; }
        if b2 || b4 || b5 { tendrop = false; }
        if sf[1] && sf[2] && (sf[3] || sf[4] || sf[5]) { pos = -1; djnhx = false; sf = [false; 6]; }
        tgt[i] = pos;
    }
    Some(target_trades(x, &tgt, from, to, false, "Titanic signal"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;
    #[test]
    fn buys_a_breadth_thrust_after_a_drop_and_sells_after_a_new_high_divergence() {
        let n = 39; // bar 39 would buy back on the 2% stop (high 170 >= 1.02 x 166)
        // closes 100 for 10 bars, drop to 85 (more than 9% below the high), then a new high of 130 on bar 30
        let px = |i: usize| if i < 10 { 100. } else if i < 30 { 85. } else { 130. + i as f64 };
        let r: Vec<Row> = (0..n).map(|i| Row { ticker: "DJI".into(), date: format!("2001-{:02}-{:02}", 1 + i / 28, 1 + i % 28), open: px(i), high: px(i) + 1., low: px(i) - 1., close: px(i), volume: None }).collect();
        let mut x = Ctx::new(&r, &r, &[]);
        let p = Params::from([("lookbk".to_string(), 10.)]);
        // B2 on bars 19-20 (ai = 6 x di); S: new high bar 30, nnl > nnh on bar 33, dv > 9 av on bar 35
        let ai: Vec<f64> = (0..n).map(|i| if (19..=20).contains(&i) { 600. } else { 100. }).collect();
        let di = vec![100.; n];
        let av = vec![100.; n];
        let dv: Vec<f64> = (0..n).map(|i| if i == 35 { 1000. } else { 100. }).collect();
        let nnh = vec![50.; n];
        let nnl: Vec<f64> = (0..n).map(|i| if i == 33 { 80. } else { 10. }).collect();
        for (k, v) in SERIES.iter().zip([ai, di, av, dv, nnh, nnl]) { x.extra.insert((*k).into(), v); }
        let t = run("dmtfly_flying", &x, 0, n - 1, &p).unwrap();
        assert_eq!(t.iter().map(|t| (t.dir, t.legs[0].i, t.exit_i, t.open)).collect::<Vec<_>>(), vec![(1., 20, 35, false), (-1., 35, 38, true)]);
        assert!(run("dmtfly_x", &x, 0, 5, &p).is_none());
    }
}

// Instrument admission: Dennis Meyers, Making The Titanic Fly (S&C 1995), lane PB 2026-11-13.
pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book == "dmtfly" && reviewed(def.id),
    label: "Dennis Meyers tested the flying Titanic on the DJIA (1978-1995); it reads NYSE breadth and runs on the US indices and ETFs DJI, DIA, SPX, SPY, NDX and QQQ only (R4 #306), with the two unstated details read as labelled on the card (R4 #297).",
    basis: "Dennis Meyers, Making The Titanic Fly, Stocks & Commodities V.13:5 p.189-195 and the sidebar TradeStation code (VOLUMES/V13/C05/MAKING.PDF, SIDETRD.PDF); NYSE new highs/lows from Norgate (SOURCES norgate-us-indices-20200908-pb); independent check authors/pb/w/pb_ref.py",
};
