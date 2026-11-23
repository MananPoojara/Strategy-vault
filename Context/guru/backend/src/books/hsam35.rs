//! Samurai 35 (T-Bond System), John R. Hill, TradeStation: research share `USB HDD 1/TradeStation Systems & Indicators/John R.Hill - Samurai 35 (T-Bond System) for TS/
//! Sam_35.rar::Sam_35.txt` (read-only copy read in full). Book id `hsam`. Lane K4 (23 Nov 2026, code pass, user ruling "code has the logic"):
//! daily stop-and-reverse system run on daily NSE indices (NIFTY 50, NIFTYBANK, NIFTYIT, NIFTYMIDCAP100, NIFTYMIDCAPSELECT) as a labelled R4 #340 adaptation (the code is for T-bond futures).
//! Independent check authors/code-1123/ref/hsam_ref.py. Orders run in `rrsim` (daily TradeStation bar model).
use super::rrsim::{g, o, pd, run_ts, sdef, A, P};
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};

const ID: &str = "hsam_samurai35";

const NOTE: &str = "[Declared adaptation, R4 #340 and the user ruling of 23 Nov 2026] The code is the 'Samurai 35' daily T-Bond futures system (no result is printed in the file). The vault runs it literally on daily NSE index bars (NIFTY 50, NIFTYBANK, NIFTYIT, NIFTYMIDCAP100, NIFTYMIDCAPSELECT; index-class reference, R4 #378), UNVERIFIED. Rule as printed, each close: avg = (C + C[1] + H + L)/4; buyDay when C <= avg, else sellDay. dla = max(0.5 x range, 8 ticks), dla1 = max(range, 8 ticks). Buy day: stopb = max(open of tomorrow + dla, Low[1], openWeek), stops = min(open of tomorrow - dla1, High[1], openWeek); sell day: stopb = max(open of tomorrow + dla1, Low[1], openWeek), stops = min(open of tomorrow - dla, High[1], openWeek); openWeek = the open of the first bar of the current week (whichFilter 0; 1 = Open[2]). Always-in stop and reverse: Buy tomorrow at stopb stop when not long, Sell Short tomorrow at stops stop when not short; a profit objective Sell / Buy to Cover at entryprice +/- profObj / bigPointValue limit (profObj 3000, T-bond bigPointValue 1000 = 3 price points). [Declared readings] (1) ONE stated scaling rule, amount x price / 110 (T-bond price level about 110), covers every fixed amount in the code: the 3-point profit objective (3 x entry price / 110) and the 8-tick minimum distance (8 ticks x 1/32 = 0.25 points, so 0.25 x close / 110); one labelled detail, R4 #380 amending #377. (2) 'open of tomorrow' is the next bar's open, known when the order is placed at that open (the vault's convention for stop levels set from the next open); a stop the next open already satisfies fills at the open. (3) The code's one-off data patch (if date = 1070706 then stopb = 999.343) and the commented-out date fixes are not applied. (4) The redundant 'init' orders (same prices as the main orders) are the main orders. Bars before the third are skipped (the code reads Close[1] and Open[2]). Vault conventions: one position, fills as TradeStation bar model.";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![sdef("hsam", ID, "Samurai 35 (Hill): always-in stop and reverse at the next open +/- half the range (or the range), floored by the prior bar and the week open; 3-point profit objective", "Samurai 35 (Sam_35.txt)", "Long/Short", Status::Live, &[
        "F|\"avg = (close + close[1] + high + low)/4.0; if(close<=avg) then buyDay = 1; if(close>avg) then sellDay = 1\"; \"dlat = range * .50; dla = maxlist(dlat,(8*minmove/pricescale)); dla1 = range\"",
        "E|buyDay: \"stopb = maxlist(open of tomorrow + dla,low[1],openWeek); stops = minlist(open of tomorrow - dla1,high[1],openWeek)\"; sellDay: \"stopb = maxlist(open of tomorrow + dla1,low[1],openWeek); stops = minlist(open of tomorrow - dla,high[1],openWeek)\"; \"if(marketposition <> 1) then Buy (\\\"samurai35 buy\\\") tomorrow at stopb stop; if(marketposition <> -1) then Sell Short (\\\"samurai35 sell\\\") tomorrow at stops stop\"",
        "X|stop and reverse; \"if(marketPosition = 1 ) then Sell(\\\"longProf\\\") next bar at entryPrice + (profObj/bigPointValue) limit\" (profObj 3000 / 1000 = 3 points); short mirrored",
        ], NOTE)]
}
pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> { (id == ID).then_some((ID, "Samurai 35 (John R. Hill)", "stop and reverse")) }
pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    (id == ID).then(|| vec![pd("filter", "whichFilter (0 = open of the week, 1 = Open[2])", 0., 0., 1., 1.), pd("prof", "profit objective, T-bond points (profObj 3000 / bigPointValue 1000)", 3., 0.5, 10., 0.5),
        pd("ref", "reference price the code's points belong to (T-bond about 110)", 110., 50., 400., 10.)])
}
pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if id != ID || from >= to || to >= x.r.len() { return None; }
    let (wf, prof, rf) = (g(p, "filter", 0.) >= 0.5, g(p, "prof", 3.), g(p, "ref", 110.));
    let (o_, h, l, c, n) = (&x.o, &x.h, &x.l, &x.c, x.c.len());
    let dow = |i: usize| chrono_dow(&x.r[i].date);
    let mut openweek = 0.0f64;
    Some(run_ts(x, from, to, |i, st, _now, next| {
        if i + 1 >= n || i < 2 { return; }
        if dow(i + 1) < dow(i) { openweek = o_[i + 1]; }
        let avg = (c[i] + c[i - 1] + h[i] + l[i]) / 4.0;
        let (rng, floor) = (h[i] - l[i], 0.25 * c[i] / rf);
        let (dla, dla1) = ((rng * 0.5).max(floor), rng.max(floor));
        let (ot, k2) = (o_[i + 1], if wf { o_[i - 2] } else { openweek });
        let (stopb, stops) = if c[i] <= avg { ((ot + dla).max(l[i - 1]).max(k2), (ot - dla1).min(h[i - 1]).min(k2)) } else { ((ot + dla1).max(l[i - 1]).max(k2), (ot - dla).min(h[i - 1]).min(k2)) };
        if st.pos != 1 { next.push(o(A::Buy, P::Stop(stopb), "samurai35 buy")); }
        if st.pos != -1 { next.push(o(A::Sell, P::Stop(stops), "samurai35 sell")); }
        let pr = prof * st.entry_px / rf;
        if st.pos == 1 { next.push(o(A::Xl, P::Lim(st.entry_px + pr), "longProf")); }
        if st.pos == -1 { next.push(o(A::Xs, P::Lim(st.entry_px - pr), "shortProf")); }
    }))
}
/// Day of week (Monday = 0) of a YYYY-MM-DD date (Zeller-free: days from 1970-01-01, a Thursday).
fn chrono_dow(d: &str) -> i64 {
    let (y, m, dd): (i64, i64, i64) = (d[0..4].parse().unwrap(), d[5..7].parse().unwrap(), d[8..10].parse().unwrap());
    let (y2, m2) = if m <= 2 { (y - 1, m + 12) } else { (y, m) };
    let days = 365 * y2 + y2 / 4 - y2 / 100 + y2 / 400 + (153 * (m2 - 3) + 2) / 5 + dd - 1;
    (days + 2).rem_euclid(7) // 0000-03-01 anchor: checked below against 2024-01-01 (Monday)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn weekday_and_run() {
        assert_eq!(chrono_dow("2024-01-01"), 0); assert_eq!(chrono_dow("2024-01-05"), 4); assert_eq!(chrono_dow("2026-11-23"), 0);
        let r = super::super::sctraub::tests::bars(1500);
        let x = Ctx::new(&r, &r, &[]);
        let t = run(ID, &x, 0, r.len() - 1, &Params::new()).unwrap();
        assert!(t.len() > 5 && t.iter().any(|z| z.dir > 0.) && t.iter().any(|z| z.dir < 0.));
        assert!(run("hsam_x", &x, 0, 5, &Params::new()).is_none());
    }
}

// Instrument admission: Samurai 35 2026-11-23 (lane K4).
pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book == "hsam" && def.id == "hsam_samurai35",
    label: "A daily T-bond futures system with no result shown; runs on the five daily NSE indices only (a declared application; amounts scaled by price, R4 #340), not the author's test.",
    basis: "Sam_35.txt (John R. Hill, Samurai 35 T-Bond System for TS); independent check authors/code-1123/ref/hsam_ref.py",
};
