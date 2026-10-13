//! Universal LT & II Trading System (staffordtrading.com). Sources (research drive, read in place, read-only):
//! `USB HDD 1/TradeStation Systems & Indicators/Universal LT & II Trading System (staffordtrading.com)/`
//! `Open Code Universal2M.rar` -> `Open Code Universal2M.txt` and `UNIVERSAL LT.rar` -> `UNIVERSAL LT.txt` (extracted as data).
//! Book id `stfun`. Notes: library/stfun/README.md. Engine: books::rrsim.
use crate::books::rrsim::{g, o, pd, run_ts, sdef, A, P};
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "stfun";
const U2M: &str = "stfun_universal2m";
const ULT: &str = "stfun_universal_lt";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![
        sdef(BOOK, U2M, "Universal 2M (buy day / sell day thrust stops, stop and reverse)", "Universal 2M (Open Code Universal2M.txt)", "Long/Short", Status::Live, &[
            "F|\"Inputs: pct1(37),pct2(90),tickamt(10)\" (the per-commodity table is commented out)",
            "F|\"buyday=bd[1]; sellday=sd[1]\" (start: bd 1, sd -1); \"if close[0]<open[0] and c[0]<close[1] then buyday=1, sellday=-1\"; \"if close[0]>open[0] and c[0]>close[1] then buyday=-1, sellday=1\"",
            "F|\"thrust1st=maxlist(range*pct1/100,tickamt); thrust2nd=maxlist(range*pct2/100,tickamt)\"",
            "E|buyday=1: \"if marketposition<>1 then Buy(\\\"1\\\") Next Bar at maxlist of (open tomorrow + thrust1st,low[1]) stop\"; \"if marketposition<>-1 then Sell Short(\\\"2\\\") Next Bar at minlist of (open tomorrow -thrust2nd,high[1]) stop\"",
            "E|sellday=1: \"if marketposition<>-1 then Sell Short(\\\"3\\\") Next Bar at minlist of (open tomorrow -thrust1st,high[1]) stop\"; \"if marketposition<>1 then Buy(\\\"4\\\") Next Bar at maxlist of (open tomorrow + thrust2nd,low[1]) stop\"",
            "X|No separate exits: stop and reverse",
        ], "Daily system (thrusts from the next open and the day's range). tickamt is the plain input 10 (price units); the commented-out lines once read 'tickamt=10 points' (p.tickamt to vary). \
Fills: stops at their level along the daily path O -> nearer extreme -> other extreme -> C (rrsim). India: instrument swap (NIFTY 50 instead of the futures list in the code); index shorts hypothetical."),
        sdef(BOOK, ULT, "Universal LT (trendline breakout with objective exits)", "Universal LT (UNIVERSAL LT.txt)", "Long/Short",
            Status::NotTestable("The objective exits LONGX / SHORTX add the swing measured only over bars 'IF W<=MAXBARSBACK', a TradeStation chart setting the code does not state, so the limit exit price depends on an unstated value (same construction as rarsys_weaver_dt)."), &[
            "E|\"Buy(\\\"Buy\\\") Next Bar LINE STOP\" / \"Sell Short(\\\"Sell\\\") Next Bar ULINE STOP\" (trendlines through the last two pivot highs / lows, STRENGTH 1)",
            "X|\"Sell(\\\"ObjectiveLX\\\") Next Bar LONGX LIMIT\"; \"Buy to Cover(\\\"ObjectiveSX\\\") Next Bar SHORTX LIMIT\"",
        ], "Code text only; the folder's reports are not readable text."),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    match id { U2M => Some(("stfun", "Universal (staffordtrading.com)", "Universal 2M")), ULT => Some(("stfun", "Universal (staffordtrading.com)", "Universal LT")), _ => None }
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    (id == U2M).then(|| vec![pd("pct1", "pct1 (% of range, with-day thrust)", 37., 10., 150., 1.), pd("pct2", "pct2 (% of range, counter thrust)", 90., 10., 150., 1.),
        pd("tickamt", "tickamt (minimum thrust, price units)", 10., 0., 50., 1.)])
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if id != U2M || from >= to || to >= x.r.len() { return None; }
    let (p1, p2, tk) = (g(p, "pct1", 37.), g(p, "pct2", 90.), g(p, "tickamt", 10.));
    let (mut bd, mut sd) = (1i8, -1i8);
    Some(run_ts(x, from, to, |i, st, _now, next| {
        if i == 0 { return; }
        let (o_, h, l, c) = (&x.o, &x.h, &x.l, &x.c);
        let (mut buyday, mut sellday) = (bd, sd);
        if c[i] < o_[i] && c[i] < c[i - 1] { buyday = 1; sellday = -1; }
        if c[i] > o_[i] && c[i] > c[i - 1] { buyday = -1; sellday = 1; }
        bd = buyday; sd = sellday;
        let rg = h[i] - l[i];
        let (t1, t2) = ((rg * p1 / 100.).max(tk), (rg * p2 / 100.).max(tk));
        let Some(&on) = o_.get(i + 1) else { return };
        if buyday == 1 {
            if st.pos != 1 { next.push(o(A::Buy, P::Stop((on + t1).max(l[i - 1])), "1")); }
            if st.pos != -1 { next.push(o(A::Sell, P::Stop((on - t2).min(h[i - 1])), "2")); }
        }
        if sellday == 1 {
            if st.pos != -1 { next.push(o(A::Sell, P::Stop((on - t1).min(h[i - 1])), "3")); }
            if st.pos != 1 { next.push(o(A::Buy, P::Stop((on + t2).max(l[i - 1])), "4")); }
        }
    }))
}
