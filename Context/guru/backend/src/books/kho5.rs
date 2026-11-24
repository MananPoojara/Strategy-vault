//! Kevin Ho, "Scalp Trading Methods" (Chartpoint, Aug 2003), Pattern 5 "The 5-Minute Standard Deviation Scalp": research share
//! `Trading Knowledge III/!PDFs/Scalp Trading Methods.pdf` p.36 (read in full). Book id `kho5`. Lane K1 (24 Nov 2026): 5-minute NIFTY 50 /
//! NIFTYBANK futures bars as a labelled R4 #340 adaptation, one scaling rule R4 #380. Helper books::k4x. Reference authors/code-1123/ref/kho5_ref.py.
use super::k4x::{fut_bars, o, run_fb, A, P};
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};

const ID: &str = "kho5_sd_scalp";
const NOTE: &str = "[Declared adaptation, R4 #340 / #380] Kevin Ho, Scalp Trading Methods (Chartpoint, Aug 2003), Pattern 5, p.36: on 5-minute bars a 10-bar moving average with an upper and lower band exactly 1 standard deviation from it. In an up trend buy the dip that penetrates the LOWER band provided the bands still slope up; in a down trend sell the rally that penetrates the UPPER band provided the bands slope down. Exit for profit at a 2-point gain or when price touches the other band, whichever is sooner; stop loss 1.25 points, or exit at market if the slope of the bands changes after entry. Labelled readings (4): (1) the standard deviation is the population standard deviation of the last 10 closes (the book names no convention); (2) the band slope is the slope of the moving average (the bands are the average plus and minus the deviation; the book does not say which line 'slope' means): up = this bar's average above the previous bar's; (3) 'penetrates the band' is a limit order at the band level computed at the previous bar's close, live for one bar, filled at the band or better (at the open when the bar opens beyond it); the exits are a limit at the nearer of entry + 2 points and the opposite band (recomputed each bar) and a stop 1.25 points away, and a slope reversal sends a market exit at the next open; (4) the book gives no session rule, so a trade open at the last bar of a session is closed at that close and no order crosses a session. The book's S&P point is scaled by ONE rule (#380): points x price / 1,000 (the S&P 500 stood near 1,000 when the article was written, Aug 2003), so the target is 0.2 percent and the stop 0.125 percent of the entry price. Indicators run continuously over the futures series, no signal before 10 bars. No profit figure is printed except 'a decent 75 percent accuracy' for all six patterns together.";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef {
        id: ID, name: "Kevin Ho Pattern 5: 5-minute standard-deviation retracement scalp (10-bar average +- 1 SD, 5-minute NIFTY / BANKNIFTY futures)", group: "Scalp Trading Methods (Kevin Ho, Chartpoint 2003)", session: 0,
        dir: "Long/Short", status: Status::Live, needs_volume: false, needs_vix: false, needs_series: &[], book: "kho5",
        rules: &[
            "F|\"On 5-minute bar chart, impose a 10-bar moving average. From this moving average, expand an upper and lower band exactly 1 standard deviation from it.\"",
            "E|\"In an up trend, we are looking only to buy the dip that penetrates the LOWER band, provided the slope of the bands are still up. In downtrends, we look to sell the rally that penetrates the UPPER band, provided the bands are sloping downwards.\"",
            "X|\"Close out for profit on a 2-point gain or if the price touches the other band, whichever is sooner. Take a 1.25-point stop loss or exit at market if the slope of the bands changes after entry.\"",
        ],
        notes: NOTE, claim: None,
    }]
}
pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> { (id == ID).then_some(("kho5", "Kevin Ho Pattern 5 (Scalp Trading Methods)", "5-minute NIFTY / BANKNIFTY futures")) }
pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    (id == ID).then(|| vec![p("len", "Moving average length (bars)", 10., 3., 50., 1.), p("sd", "Band width (standard deviations)", 1., 0.5, 3., 0.25),
        p("target", "Profit target (S&P points)", 2., 0.5, 10., 0.25), p("stop", "Stop loss (S&P points)", 1.25, 0.25, 10., 0.25)])
}
pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if id != ID || to >= x.c.len() || from > to { return None; }
    let g = |k: &str| *p.get(k).unwrap_or(&params(id).unwrap().iter().find(|d| d.key == k).unwrap().def);
    let Some(fb) = fut_bars(x, from, to, 5) else { return Some(vec![]) };
    let (len, nsd, tgt, stp) = (g("len").round() as usize, g("sd"), g("target") / 1000.0, g("stop") / 1000.0);
    let n = fb.c.len();
    let mut ma = vec![f64::NAN; n];
    let mut sd = vec![f64::NAN; n];
    for k in len - 1..n {
        let w = &fb.c[k + 1 - len..=k];
        let m = w.iter().sum::<f64>() / len as f64;
        ma[k] = m;
        sd[k] = (w.iter().map(|v| (v - m) * (v - m)).sum::<f64>() / len as f64).sqrt();
    }
    Some(run_fb(&fb, |k, st, now, next| {
        if k < len || fb.last[k] {
            if fb.last[k] && st.pos != 0 { now.push(o(if st.pos > 0 { A::Xl } else { A::Xs }, P::Mkt, "end of day")); }
            return;
        }
        let (up, dn) = (ma[k] - ma[k - 1] > 1e-9, ma[k - 1] - ma[k] > 1e-9); // equal averages (prices are on a 0.05 grid) are a flat slope, not float noise
        let (hi, lo) = (ma[k] + nsd * sd[k], ma[k] - nsd * sd[k]);
        if st.pos == 0 {
            if up { next.push(o(A::Buy, P::Lim(lo), "Buy lower band")); }
            if dn { next.push(o(A::Sell, P::Lim(hi), "Sell upper band")); }
        } else if st.pos == 1 {
            if dn { next.push(o(A::Xl, P::Mkt, "slope change")); }
            next.push(o(A::Xl, P::Lim(hi.min(st.ep * (1.0 + tgt))), "target or upper band"));
            next.push(o(A::Xl, P::Stop(st.ep * (1.0 - stp)), "stop"));
        } else {
            if up { next.push(o(A::Xs, P::Mkt, "slope change")); }
            next.push(o(A::Xs, P::Lim(lo.max(st.ep * (1.0 - tgt))), "target or lower band"));
            next.push(o(A::Xs, P::Stop(st.ep * (1.0 + stp)), "stop"));
        }
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn defaults() {
        assert_eq!(params(ID).unwrap().iter().map(|p| p.def).collect::<Vec<_>>(), vec![10., 1., 2., 1.25]);
        assert!(params("x").is_none());
    }
}
