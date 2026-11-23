//! YM Fade The Gap Strategy (forum demo code "based on John Carter's YM gap strategy"), TradeStation: research share `USB HDD 1/TradeStation
//! Systems & Indicators/YM Fade The Gap Strategy (Based on John Carter YM Gap Strategy)/Alternate.rar::Alternate.txt` (read-only copy read in full).
//! Book id `ymgap`. Lane K4 (23 Nov 2026, code pass): coded literally on the NIFTY 50 / NIFTYBANK futures 1-minute series as 5-minute session-aligned
//! bars (the file's "5 min chart"), a labelled R4 #340 adaptation. Helper books::k4x. Independent check authors/code-1123/ref/ymgap_ref.py.
use super::k4x::{fut_bars, o, run_fb, A, P};
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};

const ID: &str = "ymgap_fade";

const NOTE: &str = "[Declared adaptation, R4 #340 and the user ruling of 23 Nov 2026] Forum demo code for the Dow e-mini (YM): \"Use 5 min chart on ym, custom session 9:30am - 4:15pm\"; the author says it \"doesn't do well when back testing\" (no result printed). Run literally on the NIFTY 50 / NIFTYBANK futures 1-minute series as 5-minute session-aligned bars, UNVERIFIED. Rule as printed: GAP = |prior session close - today's open|. A gap larger than iGAPSizeMin (10) and smaller than iGAPSizeMax (70) is faded (gap up: Sell Short; gap down: Buy) at the CLOSE of the first bar (flat, no entry yet today); a gap larger than the maximum is not faded but followed (gap up: Buy, gap down: Sell Short). Faded gap: profit target = the open -/+ the gap size, stop = the open +/- 1.5 x gap when the gap < 40, else 1 x gap. Followed gap: stop and target = the open -/+ gap/2 (1:1). Exits are 'next bar' stop / limit orders re-sent each bar; unfilled positions exit on the close (setexitonclose). [Declared readings] (1) The code's gap amounts (10, 70, 40) are YM index points around the Dow level of about 10,000 when the code was written: scaled to NIFTY by price, amount x prior close / 10,000 (one labelled detail, R4 #364 superseded by the user ruling). (2) 'Prior session close' and 'today's open' are the futures series' own last close and first-minute open (nearest-expiry continuous, not back-adjusted, so a roll-day gap includes the contract basis jump). (3) setexitonclose is read as a market exit at the last bar's close of the session. Vault conventions: one position, one entry per day, fills as TradeStation bar model (stop / limit levels along the open-to-extreme path).";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef {
        id: ID, name: "YM Fade The Gap (gap of 10-70 YM points faded at the first bar's close, larger gaps followed; 5-minute NIFTY / BANKNIFTY futures)", group: "TradeStation forum code (YM gap)", session: 0,
        dir: "Long/Short", status: Status::Live, needs_volume: false, needs_vix: false, needs_series: &[], book: "ymgap",
        rules: &[
            "F|\"vGAPSize = absvalue(closed(1) - opend(0))\"; \"if(vGAPSize > iGAPSizeMin and vGAPSize < iGAPSizeMax) then vIsGAPDay = true\"; \"if(vGAPSize > iGAPSizeMax and iGAPBuyMax) then vIsGAPDay = true\" (10 / 70)",
            "E|\"if (MarketPosition(0) = 0 and EntriesToday(Date) = 0 and vIsGAPDay) then: gap up: (gap > max: buy LEGB) else sellshort SE this bar on close; gap down: (gap > max: sellshort SEGB) else buy LE this bar on close\"",
            "X|fade: \"vStopLoss = opend(0) + (vGAPSize * 1.5)\" (gap < 40, else + vGAPSize), \"vProfitTarget = opend(0) - vGAPSize\" (mirrored for gap down); followed gap: stop and target opend(0) -/+ vGAPSize / 2; \"setexitonclose\"",
        ],
        notes: NOTE,
        claim: None,
    }]
}
pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> { (id == ID).then_some(("ymgap", "YM Fade The Gap", "5-minute NIFTY / BANKNIFTY futures")) }
pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    (id == ID).then(|| vec![p("bar", "Bar size (minutes; the code's 5)", 5., 1., 30., 1.), p("gmin", "iGAPSizeMin (YM points at 10,000, scaled by price)", 10., 0., 100., 1.), p("gmax", "iGAPSizeMax (YM points at 10,000, scaled by price)", 70., 10., 300., 1.),
        p("g40", "gap below which the stop is 1.5 x gap (YM points at 10,000)", 40., 0., 200., 1.), p("follow", "iGAPBuyMax: 1 = follow a gap above the maximum", 1., 0., 1., 1.)])
}
pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if id != ID || to >= x.c.len() || from > to { return None; }
    let g = |k: &str| *p.get(k).unwrap_or(&params(id).unwrap().iter().find(|d| d.key == k).unwrap().def);
    let Some(fb) = fut_bars(x, from, to, g("bar").round().max(1.0) as u16) else { return Some(vec![]) };
    let (gmin0, gmax0, g40_0, follow) = (g("gmin"), g("gmax"), g("g40"), g("follow") > 0.5);
    // per-day state: (open, gap size, gap up, stop, target) of today's trade plan
    let mut plan: Option<(f64, f64, bool, bool)> = None; // (open, gap, up, followed)
    Some(run_fb(&fb, |k, st, now, next| {
        let first = k > 0 && fb.date[k] != fb.date[k - 1];
        if k == 0 || first { plan = None; }
        let new_day = k == 0 || first;
        if new_day && k > 0 {
            let (pc, op) = (fb.c[k - 1], fb.o[k]);
            let sc = pc / 10000.0;
            let gap = (pc - op).abs();
            let (gmin, gmax) = (gmin0 * sc, gmax0 * sc);
            let faded = gap > gmin && gap < gmax;
            let followed = gap > gmax && follow;
            if faded || followed { plan = Some((op, gap, pc < op, followed)); }
        }
        if let Some((op, gap, up, followed)) = plan {
            if new_day && k > 0 && st.last_entry.map_or(true, |e| fb.date[e] != fb.date[k]) && st.pos == 0 {
                let long = if followed { up } else { !up };
                now.push(o(if long { A::Buy } else { A::Sell }, P::Mkt, if followed { "gap follow" } else { "gap fade" }));
            }
            if st.pos != 0 {
                let g40 = g40_0 * (fb_prev_close(&fb, k) / 10000.0);
                let (sl, pt) = if followed {
                    if up { (op - gap / 2.0, op + gap / 2.0) } else { (op + gap / 2.0, op - gap / 2.0) }
                } else if up {
                    (op + if gap < g40 { gap * 1.5 } else { gap }, op - gap)
                } else {
                    (op - if gap < g40 { gap * 1.5 } else { gap }, op + gap)
                };
                if st.pos > 0 { next.push(o(A::Xl, P::Stop(sl), "stop")); next.push(o(A::Xl, P::Lim(pt), "target")); }
                else { next.push(o(A::Xs, P::Stop(sl), "stop")); next.push(o(A::Xs, P::Lim(pt), "target")); }
            }
        }
        if fb.last[k] && st.pos != 0 { now.push(o(if st.pos > 0 { A::Xl } else { A::Xs }, P::Mkt, "end of day")); }
    }))
}
/// Close of the last bar before today's first bar (the prior session close) for bar k of today.
fn fb_prev_close(fb: &super::k4x::Fb, k: usize) -> f64 {
    let mut j = k;
    while j > 0 && fb.date[j - 1] == fb.date[k] { j -= 1; }
    if j == 0 { fb.c[0] } else { fb.c[j - 1] }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn params_defaults() {
        assert_eq!(params(ID).unwrap().iter().map(|p| p.def).collect::<Vec<_>>(), vec![5., 10., 70., 40., 1.]);
        assert!(params("x").is_none());
    }
}
