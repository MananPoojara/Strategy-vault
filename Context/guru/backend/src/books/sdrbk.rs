//! R-Breaker (Rich Saidenberg), TradeStation open code: research share `USB HDD 1/TradeStation Systems & Indicators/Rich Saidenberg - R-Breaker for TS/
//! Open Code.rar::Open Code.txt` (read-only copy read in full). Book id `sdrbk`. Lane K4 (23 Nov 2026, code pass, user ruling "code has the logic"):
//! coded literally on the NIFTY 50 / NIFTYBANK futures 1-minute series (Ctx::fut, lane DI) as 10-minute session-aligned bars (the file's own
//! "10 min Time Frame"), a labelled R4 #340 adaptation. Shared helper books::k4x. Independent check authors/code-1123/ref/sdrbk_ref.py.
use super::k4x::{fut_bars, hhmm, o, run_fb, A, P};
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};

const ID: &str = "sdrbk_rbreaker";

const NOTE: &str = "[Declared adaptation, R4 #340 and the user ruling of 23 Nov 2026] The code is the S&P 500 e-mini day system 'R-Breaker' (header: \"Trading between 9:15 and 14:29 ChicagoTime only; MMStop $1000; Close End of Day; 10 min Time Frame\"). The vault runs it literally on the NIFTY 50 / NIFTYBANK futures 1-minute series as 10-minute session-aligned bars (stamp = bar end, as TradeStation), UNVERIFIED, no published result to compare. Printed inputs: notbef 715, notaft 1229, f1 .35, f2 .07, f3 .25, reverse 2.00, rangemin 1.15, xdiv 3. Each day the code derives from yesterday's high H, low L and close C: ssetup = H + f1(C-L), bsetup = L - f1(H-C), senter = ((1+f2)/2)(H+C) - f2 L, benter = ((1+f2)/2)(L+C) - f2 H, bbreak = ssetup + f3(ssetup-bsetup), sbreak = bsetup - f3(ssetup-bsetup); yesterday's range must be >= rangemin; from the second day on, inside the window it sells short on a stop at senter + (today's high - ssetup)/xdiv once today's high reached ssetup (buys at benter - (bsetup - today's low)/xdiv once the low reached bsetup), reverses at entryprice +- reverse, and from the flat state buys at bbreak / sells at sbreak; one entry per day (date > EntryDate(1)); from notaft it exits at entryprice +- reverse and on a stop 0.05 beyond the bar's high / low (a literal port: the 'Late' stops are re-sent every bar). [Declared readings] (1) The clock inputs (715 / 1229) are read on the exchange clock as printed, so trading runs from the NSE open to 12:29 (the code's own Chicago 9:15-14:29 note is a different clock). (2) The code's point amounts (reverse 2.00, rangemin 1.15, the 0.05 late offset) are S&P 500 points around the 2002-2003 index level of about 1,000: scaled to NIFTY by price, amount x previous close / 1,000 (one labelled detail, R4 #364 superseded by the user ruling). (3) 'Close End of Day' (the header's setup line; EasyLanguage 'exit on close' is a chart setting, not code) closes any open position at the last bar's close. (4) The header's $1000 money-management stop is a platform setting that is not in the code and is not applied. Vault conventions: one position at a time, fills as TradeStation bar model (market at the next open, stop/limit levels along the open-to-extreme path). The futures series is nearest-expiry continuous, not back-adjusted (roll gaps inside multi-day trades are possible).";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef {
        id: ID, name: "R-Breaker (Saidenberg: previous-day pivot setup/enter/break levels with reversal, 10-minute NIFTY / BANKNIFTY futures)", group: "TradeStation open code (Saidenberg R-Breaker)", session: 0,
        dir: "Long/Short", status: Status::Live, needs_volume: false, needs_vix: false, needs_series: &[], book: "sdrbk",
        rules: &[
            "F|\"ssetup=hitoday[1]+f1*(c[1]-ltoday[1]); senter=((1+f2)/2)*(hitoday[1]+c[1])-(f2)*ltoday[1]; benter=((1+f2)/2)*(ltoday[1]+c[1])-(f2)*hitoday[1]; bsetup=ltoday[1]-f1*(hitoday[1]-c[1]); bbreak=ssetup+f3*(ssetup-bsetup); sbreak=bsetup-f3*(ssetup-bsetup)\" at the first bar of each day (f1 .35, f2 .07, f3 .25); \"rfilter=hitoday[1]-ltoday[1]>=rangemin\" (1.15)",
            "E|\"if t>=notbef and t<notaft and startnow>=2 and rfilter and date>entrydate(1) then: if hitoday>=ssetup and marketposition>-1 then Sell Short Next Bar at senter+(hitoday-ssetup)/div stop; if ltoday<=bsetup and marketposition<1 then Buy Next Bar at benter-(bsetup-ltoday)/div stop\" (notbef 715, notaft 1229, xdiv 3)",
            "E|\"if marketposition=-1 then Buy Next Bar at entryprice+reverse stop; if marketposition=1 then Sell Short Next Bar at entryprice-reverse stop; if marketposition=0 then Buy Next Bar at bbreak stop and Sell Short Next Bar at sbreak stop\" (reverse 2.00)",
            "X|\"if t>=notaft and t<>sess1endtime then: Buy to Cover at entryprice+reverse stop; Sell at entryprice-reverse stop; Buy to Cover Next Bar at h+.05 stop; Sell Next Bar at l-.05 stop\"; header: \"Close End of Day\"",
        ],
        notes: NOTE,
        claim: None,
    }]
}
pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> { (id == ID).then_some(("sdrbk", "R-Breaker (Saidenberg)", "10-minute NIFTY / BANKNIFTY futures")) }
pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    (id == ID).then(|| vec![p("bar", "Bar size (minutes; the code's own 10)", 10., 1., 30., 1.), p("notbef", "notbef (hhmm)", 715., 0., 1530., 1.), p("notaft", "notaft (hhmm)", 1229., 0., 1530., 1.),
        p("f1", "f1", 0.35, 0.05, 1., 0.01), p("f2", "f2", 0.07, 0., 0.5, 0.01), p("f3", "f3", 0.25, 0., 1., 0.01), p("reverse", "reverse (S&P points at 1,000, scaled by price)", 2.0, 0., 20., 0.25),
        p("rangemin", "rangemin (S&P points at 1,000, scaled by price)", 1.15, 0., 20., 0.05), p("xdiv", "xdiv", 3., 1., 10., 1.)])
}
pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if id != ID || to >= x.c.len() || from > to { return None; }
    let g = |k: &str| *p.get(k).unwrap_or(&params(id).unwrap().iter().find(|d| d.key == k).unwrap().def);
    let Some(fb) = fut_bars(x, from, to, g("bar").round().max(1.0) as u16) else { return Some(vec![]) };
    let (notbef, notaft) = (g("notbef").round() as i64, g("notaft").round() as i64);
    let (f1, f2, f3, rev0, rmin0, xdiv) = (g("f1"), g("f2"), g("f3"), g("reverse"), g("rangemin"), g("xdiv").max(1.0));
    let (mut ssetup, mut bsetup, mut senter, mut benter, mut bbreak, mut sbreak) = (0.0f64, 0.0, 0.0, 0.0, 0.0, 0.0);
    let (mut ltoday, mut hitoday, mut startnow, mut rfilter, mut scale) = (0.0f64, 9999.0f64, 0i64, false, 1.0f64);
    Some(run_fb(&fb, |k, st, now, next| {
        let (h, l, c) = (fb.h[k], fb.l[k], fb.c[k]);
        if k == 0 || fb.date[k] != fb.date[k - 1] {
            startnow += 1;
            let pc = if k == 0 { 0.0 } else { fb.c[k - 1] };
            ssetup = hitoday + f1 * (pc - ltoday);
            senter = ((1.0 + f2) / 2.0) * (hitoday + pc) - f2 * ltoday;
            benter = ((1.0 + f2) / 2.0) * (ltoday + pc) - f2 * hitoday;
            bsetup = ltoday - f1 * (hitoday - pc);
            bbreak = ssetup + f3 * (ssetup - bsetup);
            sbreak = bsetup - f3 * (ssetup - bsetup);
            scale = pc / 1000.0;
            rfilter = hitoday - ltoday >= rmin0 * scale;
            hitoday = h; ltoday = l;
        }
        if h > hitoday { hitoday = h; }
        if l < ltoday { ltoday = l; }
        let t = hhmm(fb.stamp[k]);
        let (rev, late) = (rev0 * scale, 0.05 * scale);
        let today_entry = st.last_entry.map_or(false, |e| fb.date[e] == fb.date[k]);
        if t >= notbef && t < notaft && startnow >= 2 && rfilter && !today_entry {
            if hitoday >= ssetup && st.pos > -1 { next.push(o(A::Sell, P::Stop(senter + (hitoday - ssetup) / xdiv), "Rlev SE")); }
            if ltoday <= bsetup && st.pos < 1 { next.push(o(A::Buy, P::Stop(benter - (bsetup - ltoday) / xdiv), "Rlev LE")); }
            if st.pos == -1 { next.push(o(A::Buy, P::Stop(st.ep + rev), "RbUP LE")); }
            if st.pos == 1 { next.push(o(A::Sell, P::Stop(st.ep - rev), "RbDN SE")); }
            if st.pos == 0 { next.push(o(A::Buy, P::Stop(bbreak), "Break LE")); next.push(o(A::Sell, P::Stop(sbreak), "Break SE")); }
        }
        if t >= notaft && !fb.last[k] {
            if st.pos == -1 { next.push(o(A::Xs, P::Stop(st.ep + rev), "RbUP SX")); }
            if st.pos == 1 { next.push(o(A::Xl, P::Stop(st.ep - rev), "RbDN LX")); }
            next.push(o(A::Xs, P::Stop(h + late), "Late SX"));
            next.push(o(A::Xl, P::Stop(l - late), "Late LX"));
        }
        if fb.last[k] && st.pos != 0 { now.push(o(if st.pos > 0 { A::Xl } else { A::Xs }, P::Mkt, "end of day")); }
        let _ = c;
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn params_defaults() {
        assert_eq!(params(ID).unwrap().iter().map(|p| p.def).collect::<Vec<_>>(), vec![10., 715., 1229., 0.35, 0.07, 0.25, 2.0, 1.15, 3.]);
        assert!(params("x").is_none());
    }
}
