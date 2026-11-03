//! RINA Systems (Leo Zamansky & David Stendahl), "Dynamic Zone User's Guide" v2.5 (1996-98) and the source article
//! L. Zamansky & D. Stendahl, "Dynamic Zones", Technical Analysis of Stocks & Commodities V15:7 (1997) pp.306-310.
//! Book id `rinadz`. Source (read in place, copy pending): research drive "USB HDD 1/TradeStation Systems &
//! Indicators/Rina Dynamic Zone Indicator (rinasystems.com)/Manual.pdf" (pp.9-23) and "USB HDD 1/TradeStation/_Rina
//! (rinafinancial.com)/_Manuals & Articles/Rina Systems Misc Articles/DYNAMIC.pdf" (the article: zone algorithm).
//! Dynamic zone (article p.306-307): over the last N indicator values, the buy zone is V with P{X <= V} = Pbuy and the
//! sell zone V with P{X >= V} = Psell, read off the empirical frequency: "multiply 0.1 times 80, which equals 8, and
//! then look for the price that represents the point that the frequency equals 8, adding the last three frequencies".
//! So the sell zone is the k-th largest of the N values and the buy zone the k-th smallest, k = P x N; when P x N is
//! not a whole number the count is accumulated until it reaches P x N (k = ceil; `k_round` = 1 uses floor, labelled).
//! The DZBuy/DZSell functions themselves are protected in the product, so this reading is class C.
//! TradeStation "Buy/Sell/ExitLong at market" on a daily system fills at the next bar's open; "crosses above" =
//! today above and yesterday at or below.
use crate::connors::{claim, rsi, sma, Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "rinadz";
/// Admitted ids (authors/stendahl/admission/SOURCE-CONTRACT.md).
pub const REVIEWED: &[&str] = &["rinadz_rsi9", "rinadz_rsi9_fixed", "rinadz_rsi14s", "rinadz_pctr", "rinadz_xmacd"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const ZONE: &str = "F|Dynamic zones (S&C V15:7 p.306-307): \"For dynamic zone buy: P{X <= V}=P1. For dynamic zone sell: P{X >= V}=P2\" over the lookback of N bars; \"If we multiply 0.1 times 80, which equals 8, and then look for the price that represents the point that the frequency equals 8, adding the last three frequencies (4, 3 and 1) will give us 8 ... x=19 will be the zone with the probability of 0.1\" (k-th largest / smallest of the last N values, k = P x N rounded up)";
const NOTE: &str = "Article/manual systems for the S&P 500 (cash index, and S&P futures in the manual); run on any index. Orders 'at market' fill at the next open. One unit, percentage returns. The DZBuy/DZSell code is protected; the zone is computed from the article's own algorithm and worked example.";

pub fn catalogue() -> Vec<StrategyDef> {
    let d = |id, name, dir, status, rules, claim| StrategyDef { id, name, group: "RINA Dynamic Zones (S&C V15:7; DZ User's Guide)", session: 0, dir, status,
        needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK, rules, notes: NOTE, claim };
    vec![
        d("rinadz_rsi9", "DZ RSI(9) stop-and-reverse (70 bars, 10%/10%)", "Long/Short", Status::Live, &[
            ZONE,
            "E|\"Input: Ind(RSI(C,9)), N(70), StartPrB(0.10), StartPrS(0.10); ... IF CurrentBar > 1 and Ind crosses above BuyZone then Buy at market; IF CurrentBar > 1 and Ind crosses below SellZone then Sell at market;\" (DZ User's Guide p.11)",
            "X|Always in the market: a Buy reverses a short, a Sell reverses a long (\"Our system will use a lookback period of 70 days with a probability of 10% for both the buy and sell zones\", article p.307)",
            "F|Published (S&P 500 cash index 1/5/70-11/27/96, article p.307 / guide p.12): net $202,235, 68.18% profitable, 176 trades, avg trade $1,149, PF 1.72",
        ], claim("176", "68.18%", "$1,149", "-", "1/5/70-11/27/96, S&P 500 cash index")),
        d("rinadz_rsi9_fixed", "Fixed-zone RSI(9) 30/70 stop-and-reverse (article's comparison)", "Long/Short", Status::Live, &[
            "E|\"The system buys the market as the RSI crosses above the 30 level\" (article p.307; RSI 9-day)",
            "X|\"... and sells when it crosses below the 70 level. The system remains in the market 100% of the time.\" (a sell reverses the long; a buy reverses the short)",
            "F|Published (same test): net $86,395, 73.68% profitable, 133 trades, avg trade $649, PF 1.25 (article p.307)",
        ], claim("133", "73.68%", "$649", "-", "1/5/70-11/27/96, S&P 500 cash index")),
        d("rinadz_rsi14s", "DZ smoothed RSI(14,3) long (70 bars, 15%/10%)", "Long", Status::Live, &[
            ZONE,
            "E|\"Input: Ind(Average(RSI(C,14),3)), N(70), StartPrB(0.15), StartPrS(0.10); ... IF CurrentBar > 1 and Ind crosses above BuyZone then Buy at market;\" (DZ User's Guide p.10)",
            "X|\"IF CurrentBar > 1 and Ind crosses below SellZone then ExitLong at market;\" (p.10)",
        ], None),
        d("rinadz_pctr", "DZ %R system (WAverage(%R(9),3), 70 bars, 12%/12%)", "Long", Status::Live, &[
            ZONE,
            "E|\"Par1(9),Par2(3) ... N(70),StartPrS(0.12),StartPrB(0.12); ... Indicator = WAverage(PercentR(Par1), Par2); IF CurrentBar > 1 and Indicator crosses above BuyZone then Buy at market;\" (article p.309; the manual p.13 uses Jurik AMAfunc2(PercentR(9),3) instead, which is not published; the article's weighted average is the default)",
            "X|\"IF CurrentBar > 1 and Indicator crosses below SellZone then ExitLong at market;\" (article p.309)",
            "F|PercentR (TradeStation) = 100 x (C - lowest low 9) / (highest high 9 - lowest low 9); WAverage = linearly weighted average",
            "F|Published (S&P 500, article p.309-310): net $204,640, 63 trades, 77.78% profitable, avg trade $3,140, PF 6.30, 42.15% time in market; rolling table 1987-97: 67 trades, 79.10% profitable",
        ], claim("63 (table 1987-97: 67)", "77.78% (table: 79.10%)", "$3,140", "23.27 bars winners, 32.36 losers", "1987-1997, S&P 500 cash index")),
        d("rinadz_xmacd", "Extreme MACD dynamic-zone system (70 bars, 12%/12%)", "Long", Status::Live, &[
            ZONE,
            "E|\"Input: Ind(MACD(C,12,26)), N(70), StartPrB(0.12), StartPrS(0.12); ... Indicator = MACD(C,12,26); ... If CurrentBar > 1 and Indicator crosses below BuyZone Then Buy Market;\" (DZ User's Guide p.21; 'crosses below' as printed)",
            "X|\"If CurrentBar > 1 and Indicator[1] > Indicator[2] and Indicator[1] > Indicator and Indicator[1] > SellZone Then Exitlong Market;\" (p.21)",
            "F|MACD(C,12,26) = XAverage(C,12) - XAverage(C,26) (TradeStation)",
        ], None),
        d("rinadz_cci_spike", "CCI Spike dynamic-zone system", "Long", Status::NotTestable("the indicator is AMAfunc2(CCI(10),3), Jurik's adaptive moving average, whose formula the guide does not give (\"available from Jurick Trading\")"), &[
            "E|\"Input: Ind(AMAfunc2(CCI(10),3)), Gapper(1.23), N(70), StartPrB(0.16), StartPrS(0.15), Len(10), BLen(.5); ... IndF = CCI(Len); Gap = (IndF)/(Ind); If Gap[1] > Gapper and IndF crosses over Ind and (IndF[1] < BuyZone or IndF < BuyZone) and C < BollingerBand(C,20,BLen) then Buy\" (DZ User's Guide p.17)",
            "X|\"If IndF < IndF[1] and IndF[1] > IndF[2] and (IndF > SellZone or IndF[1] > SellZone or Ind > SellZone) then exitlong;\" (p.17)",
        ], None),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    Some(match id {
        "rinadz_rsi9" => ("rinadz_rsi9", "RSI(9) zones, stop-and-reverse", "Dynamic zones"),
        "rinadz_rsi9_fixed" => ("rinadz_rsi9", "RSI(9) zones, stop-and-reverse", "Fixed 30/70 (comparison)"),
        "rinadz_rsi14s" => ("rinadz_osc", "Dynamic-zone oscillator systems (long)", "Smoothed RSI(14)"),
        "rinadz_pctr" => ("rinadz_osc", "Dynamic-zone oscillator systems (long)", "Williams %R"),
        "rinadz_xmacd" => ("rinadz_osc", "Dynamic-zone oscillator systems (long)", "Extreme MACD"),
        "rinadz_cci_spike" => ("rinadz_osc", "Dynamic-zone oscillator systems (long)", "CCI Spike (not testable)"),
        _ => return None,
    })
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    let n = p("n", "Zone lookback N (bars)", 70., 20., 200., 5.);
    let kr = p("k_round", "P x N not whole: 0 = round up (count reaches P x N), 1 = round down", 0., 0., 1., 1.);
    Some(match id {
        "rinadz_rsi9" => vec![p("len", "RSI length", 9., 2., 30., 1.), n, p("pb", "Buy-zone probability", 0.10, 0.02, 0.4, 0.01), p("ps", "Sell-zone probability", 0.10, 0.02, 0.4, 0.01), kr],
        "rinadz_rsi9_fixed" => vec![p("len", "RSI length", 9., 2., 30., 1.), p("lo", "Buy when RSI crosses above", 30., 10., 50., 1.), p("hi", "Sell when RSI crosses below", 70., 50., 90., 1.)],
        "rinadz_rsi14s" => vec![p("len", "RSI length", 14., 2., 30., 1.), p("avg", "RSI average length", 3., 1., 10., 1.), n, p("pb", "Buy-zone probability", 0.15, 0.02, 0.4, 0.01), p("ps", "Sell-zone probability", 0.10, 0.02, 0.4, 0.01), kr],
        "rinadz_pctr" => vec![p("len", "%R length", 9., 3., 30., 1.), p("avg", "Weighted average length", 3., 1., 10., 1.), n, p("pb", "Buy-zone probability", 0.12, 0.02, 0.4, 0.01), p("ps", "Sell-zone probability", 0.12, 0.02, 0.4, 0.01), kr],
        "rinadz_xmacd" => vec![p("fast", "MACD fast", 12., 3., 30., 1.), p("slow", "MACD slow", 26., 10., 60., 1.), n, p("pb", "Buy-zone probability", 0.12, 0.02, 0.4, 0.01), p("ps", "Sell-zone probability", 0.12, 0.02, 0.4, 0.01), kr],
        _ => return None,
    })
}

/// (buy zone, sell zone): k-th smallest / k-th largest of the last n values, k from P x n.
pub(crate) fn zones(v: &[f64], n: usize, pb: f64, ps: f64, floor: bool) -> (Vec<f64>, Vec<f64>) {
    let k = |p: f64| { let x = p * n as f64; (if floor { (x + 1e-9).floor() } else { (x - 1e-9).ceil() } as usize).clamp(1, n) };
    let (kb, ks) = (k(pb), k(ps));
    let mut bz = vec![f64::NAN; v.len()];
    let mut sz = vec![f64::NAN; v.len()];
    for t in n.saturating_sub(1)..v.len() {
        let w = &v[t + 1 - n..=t];
        if w.iter().any(|x| !x.is_finite()) { continue; }
        let mut s = w.to_vec();
        s.sort_by(|a, b| a.partial_cmp(b).unwrap());
        bz[t] = s[kb - 1];
        sz[t] = s[n - ks];
    }
    (bz, sz)
}

/// TradeStation XAverage: seeded with the first value, alpha = 2/(n+1).
fn xaverage(v: &[f64], n: usize) -> Vec<f64> {
    let a = 2.0 / (n as f64 + 1.0);
    let mut out = vec![f64::NAN; v.len()];
    let mut e = f64::NAN;
    for i in 0..v.len() {
        e = if e.is_nan() { v[i] } else { e + a * (v[i] - e) };
        out[i] = e;
    }
    out
}

/// TradeStation WAverage: linearly weighted, newest weight n.
fn waverage(v: &[f64], n: usize) -> Vec<f64> {
    let den = (n * (n + 1) / 2) as f64;
    (0..v.len()).map(|t| if t + 1 < n { f64::NAN } else { (0..n).map(|j| v[t - j] * (n - j) as f64).sum::<f64>() / den }).collect()
}

fn cross_up(a: &[f64], b: &[f64], t: usize) -> bool { t > 0 && a[t] > b[t] && a[t - 1] <= b[t - 1] }
fn cross_dn(a: &[f64], b: &[f64], t: usize) -> bool { t > 0 && a[t] < b[t] && a[t - 1] >= b[t - 1] }

fn open_t(dir: f64, i: usize, px: f64, sig: usize) -> Trade {
    Trade { dir, legs: vec![Leg { i, px, w: 1.0 }], exit_i: i, exit_px: px, reason: String::new(), entry_kind: "open", signal_i: sig, open: false, exits: vec![] }
}
fn shut(mut q: Trade, i: usize, px: f64, sig: usize, why: &str) -> Trade { q.exit_i = i; q.exit_px = px; q.signal_i = sig; q.reason = why.into(); q }

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    let defs = params(id)?;
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| defs.iter().find(|d| d.key == k).map(|d| d.def).unwrap_or(f64::NAN));
    let u = |k: &str| g(k).round().max(1.) as usize;
    let to = to.min(x.c.len().saturating_sub(1));
    let (o, h, l, c) = (&x.o, &x.h, &x.l, &x.c);
    let floor = g("k_round") >= 0.5;
    let ind: Vec<f64> = match id {
        "rinadz_rsi9" | "rinadz_rsi9_fixed" => rsi(c, u("len")),
        "rinadz_rsi14s" => sma(&rsi(c, u("len")), u("avg")),
        "rinadz_pctr" => {
            let n = u("len");
            let r: Vec<f64> = (0..c.len()).map(|t| {
                if t + 1 < n { return f64::NAN; }
                let hh = h[t + 1 - n..=t].iter().cloned().fold(f64::MIN, f64::max);
                let ll = l[t + 1 - n..=t].iter().cloned().fold(f64::MAX, f64::min);
                if hh > ll { 100. * (c[t] - ll) / (hh - ll) } else { 50. }
            }).collect();
            waverage(&r, u("avg"))
        }
        "rinadz_xmacd" => { let (f, s) = (xaverage(c, u("fast")), xaverage(c, u("slow"))); f.iter().zip(&s).map(|(a, b)| a - b).collect() }
        _ => return None,
    };
    let (bz, sz) = if id == "rinadz_rsi9_fixed" {
        (vec![g("lo"); c.len()], vec![g("hi"); c.len()])
    } else {
        zones(&ind, u("n"), g("pb"), g("ps"), floor)
    };
    // signal at bar t -> act at the open of t + 1
    let mut out = Vec::new();
    let mut cur: Option<Trade> = None;
    let sar = id == "rinadz_rsi9" || id == "rinadz_rsi9_fixed";
    for t in from.max(2)..to {
        let buy = if id == "rinadz_xmacd" { cross_dn(&ind, &bz, t) } else { cross_up(&ind, &bz, t) };
        let sell = if id == "rinadz_xmacd" {
            ind[t - 1] > ind[t - 2] && ind[t - 1] > ind[t] && ind[t - 1] > sz[t]
        } else { cross_dn(&ind, &sz, t) };
        let e = t + 1;
        let held = cur.as_ref().map(|q| q.dir);
        if buy && held != Some(1.0) {
            if let Some(q) = cur.take() { out.push(shut(q, e, o[e], t, "reversed: indicator crossed above the buy zone")); }
            cur = Some(open_t(1.0, e, o[e], t));
        } else if sell && held == Some(1.0) {
            out.push(shut(cur.take().unwrap(), e, o[e], t, if sar { "reversed: indicator crossed below the sell zone" } else { "exit: sell-zone signal" }));
            if sar { cur = Some(open_t(-1.0, e, o[e], t)); }
        } else if sell && sar && held.is_none() {
            cur = Some(open_t(-1.0, e, o[e], t));
        }
    }
    let _ = l;
    if let Some(mut q) = cur {
        q = shut(q, to, c[to], to, "open");
        q.open = true;
        out.push(q);
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn article_worked_example() {
        // frequencies from S&C V15:7 p.307: 1x1, 3x3, 6x7, 9x16, 11x24, 13x13, 15x8, 19x4, 20x3, 21x1 (80 values)
        let mut v = Vec::new();
        for (x, f) in [(1., 1), (3., 3), (6., 7), (9., 16), (11., 24), (13., 13), (15., 8), (19., 4), (20., 3), (21., 1)] {
            for _ in 0..f { v.push(x); }
        }
        let (bz, sz) = zones(&v, 80, 0.1, 0.1, false);
        assert_eq!(sz[79], 19.0); // the article's answer
        assert_eq!(bz[79], 6.0); // 1 + 3 + 7 >= 8 from the bottom
    }
}

// Instrument admission: Stendahl 2026-10-25 block, moved from instrument_policy.rs on 31 Oct 2026 (lane ST).
// Admission only for the module REVIEWED lists (authors/stendahl/admission/SOURCE-CONTRACT.md); kept in one block to merge.
// Runs on actual indices, ETFs and stocks, US and India (index_etf_or_stock, nifty50 basket, declared application).
pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| (def.book=="rinadz" && crate::books::rinadz::reviewed(def.id)) || (def.book=="hwdti" && crate::books::hwdti::reviewed(def.id)),
    label: "Zamansky & Stendahl tested the Dynamic Zone systems on the S&P 500 (cash index and futures); Helweg & Stendahl tested the moving-average system on Treasury bond futures; runs on other indices, ETFs and single stocks (US and India) are a declared application.",
    basis: "Zamansky & Stendahl, Dynamic Zones, S&C V15:7 (1997) p.306-310 and the RINA Dynamic Zone User's Guide pp.10-21 (DZ RSI(9), fixed RSI 30/70, smoothed RSI(14), %R, Extreme MACD); Helweg & Stendahl, Dynamic Trading Indicators (2002) Figure 6.4 p.130 (WMA20 x SMA56 stop-and-reverse). Index/ETF/stock runs are a declared application.",
};
