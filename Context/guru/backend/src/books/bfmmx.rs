//! MiniMax I for TS (Michael R. Bryant, Breakout Futures, 2001): ESL4 (e-mini S&P buy), NQL4 (e-mini Nasdaq buy),
//! S4 (e-mini S&P / Nasdaq short). Source (research drive, read in place, read-only): `USB HDD 1/TradeStation Systems &
//! Indicators/MiniMax I & II for TS,TS8, Excel + Manuals, $525 (breakoutfutures.com)/v1/Code.rar` -> `Code.txt`
//! (extracted as data). Book id `bfmmx`. Notes: library/bfmmx/README.md. Engine: books::rrsim.
use crate::books::rrsim::{g, o, pd, run_ts, sdef, sma, true_range, A, P};
use crate::connors::{wilder, Ctx, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "bfmmx";
/// Admitted ids (authors/lanea/vendor2/decisions.json).
pub const REVIEWED_LANEA: &[&str] = &["bfmmx_esl4", "bfmmx_nql4", "bfmmx_s4"];
pub fn reviewed_lanea(id: &str) -> bool { REVIEWED_LANEA.contains(&id) }
const ESL4: &str = "bfmmx_esl4";
const NQL4: &str = "bfmmx_nql4";
const S4: &str = "bfmmx_s4";
const FILLS: &str = "Fills (rrsim, TradeStation conventions): the buy/sell stop fills at its level, or at the open when the open is beyond it; exit stops placed at a close act on the position of their side that exists when the price is reached, \
including one entered earlier on the same bar (the money-management stop is placed every bar, also while flat, as in the code); 'this bar at close' fills at the close. Daily bars ('Data series: day session full S&P 500'). \
India: instrument swap (NIFTY 50 instead of the e-mini futures; levels are index points, as in the code).";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![
        sdef(BOOK, ESL4, "MiniMax ESL4 (e-mini S&P buy: breakout from the close after weakness)", "MiniMax I (Code.txt)", "Long", Status::Live, &[
            "F|\"DPlus2 = DMIPlus(14); DMin2 = DMIMinus(14); TRData2 = TrueRange; Ave14 = Average(H - L, 14); Ave10 = Average(H - L, 10)\"",
            "F|\"If MarketPosition = 1 and TrailOn = false and C >= EntryPrice and DPlus2 > DMin2 then TrailOn = true\"; \"If MarketPosition = 1 and TrailOn = true and DPlus2 < DMin2 then TrailOn = false\"",
            "E|\"If (MarketPosition <> 1 or Condition1) and Condition2 and Condition3\" (Condition1 = TrailOn = False and Open > EntryPrice and BarsSinceEntry > 0; Condition2 = DPlus2 < DMin2 or C < C[5]; Condition3 = ADX(14) > 20): BrkAmnt = MinList(MaxList(EntFrac .70 x TRData2, 0.5 x Ave14), 1.5 x Ave14); \"Buy next bar at EntPr Stop\" (EntPr = C + BrkAmnt); XMM = EntPr - MMmult .75 x Ave10; XTrail = C - TrailFr 1.5 x TRData2; TrailOn = false",
            "X|\"If MarketPosition = 1 and BarsSinceEntry = 0 then XMM = XMM - EntPr + EntryPrice\"; \"Sell(\\\"MMStop\\\") next bar at XMM stop\"",
            "X|\"If TrailOn and MarketPosition = 1 then XTrail = MaxList(XTrail, C - TrailFr * TrData2); Sell next bar at XTrail stop\"",
            "X|\"If TrailOn = False and MarketPosition = 1 then If Open > EntryPrice and BarsSinceEntry > 0 then Sell this bar at close\"",
        ], FILLS),
        sdef(BOOK, NQL4, "MiniMax NQL4 (e-mini Nasdaq buy: breakout after C < C[2])", "MiniMax I (Code.txt)", "Long", Status::Live, &[
            "E|\"Condition1 = C < C[2]\": BrkAmnt = MinList(MaxList(EntFrac .65 x TrueRange, 0.5 x Average(TrueRange,14)), 1.5 x Average(TrueRange,14)); \"Buy next bar at EntPr Stop\" (EntPr = C + BrkAmnt); XMM = EntPr - MMmult .75 x Average(H - L, 10); \"If MarketPosition <> 1 then XTrail = C - MMmult * TrueRange\"; TrailOn = false",
            "X|\"If MarketPosition = 1 then MMSize = MMmult * Average(H - L, 10)[BarsSinceEntry + 1]; XMM = EntryPrice - MMSize\"; \"Sell(\\\"MMStop\\\") next bar at XMM stop\"",
            "X|\"If MarketPosition = 1 and TrailOn = false and C >= EntryPrice then TrailOn = true\"; \"If TrailOn and MarketPosition = 1 then XTrail = MaxList(XTrail, C - MMmult * (H - L)); Sell next bar at XTrail stop\"",
        ], FILLS),
        sdef(BOOK, S4, "MiniMax S4 (short a breakdown after 2 up closes in a downtrend)", "MiniMax I (Code.txt)", "Short", Status::Live, &[
            "E|\"Condition1 = DMIMinus(14) > DMIPlus(14) and C > C[1] and C[1] > C[2]\": EntPr = C - EntFr .75 x Average(H - L, 10); \"Sell Short next bar at EntPr Stop\"",
            "X|\"If MarketPosition = 0 then XShort = EntPr + MMFr * Average(H - L, 10)\" (MMFr 1.0)",
            "X|\"If MarketPosition = -1 then If BarsSinceEntry > 2 or (EntryPrice - C) > 2* Average(H - L, 10) then XShort = MinList(XShort, C + TrailX * Average(H - L, 10)) else XShort = EntryPrice + MMFr * Average(H - L, 10)\" (TrailX .5)",
            "X|\"Buy to Cover(\\\"XShort\\\") next bar at XShort stop\"",
        ], "Inputs = the code defaults, which equal the header's e-mini S&P 500 recommendation (EntFr .75, MMFr 1.0, TrailX .5); the header's e-mini Nasdaq 100 set is EntFr .65, MMFr .5, TrailX .5 (params). \
Fills (rrsim): stops at their level or at the open when gapped; the cover stop acts also on the entry bar. Daily bars. India: instrument swap; index shorts hypothetical."),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    match id {
        ESL4 => Some(("bfmmx", "MiniMax I (Breakout Futures)", "ESL4 S&P buy")),
        NQL4 => Some(("bfmmx", "MiniMax I (Breakout Futures)", "NQL4 Nasdaq buy")),
        S4 => Some(("bfmmx", "MiniMax I (Breakout Futures)", "S4 short")),
        _ => None,
    }
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    match id {
        ESL4 => Some(vec![pd("ent_frac", "EntFrac (x prior day's true range)", 0.70, 0.2, 1.5, 0.05), pd("mm_mult", "MMmult (x 10-day range)", 0.75, 0.25, 2., 0.05), pd("trail_fr", "TrailFr (x true range)", 1.5, 0.5, 3., 0.1)]),
        NQL4 => Some(vec![pd("ent_frac", "EntFrac (x prior day's true range)", 0.65, 0.2, 1.5, 0.05), pd("mm_mult", "MMmult (x 10-day range; also trail)", 0.75, 0.25, 2., 0.05)]),
        S4 => Some(vec![pd("ent_fr", "EntFr (x 10-day range)", 0.75, 0.2, 1.5, 0.05), pd("mm_fr", "MMFr (x 10-day range)", 1.0, 0.25, 2., 0.05), pd("trail_x", "TrailX (x 10-day range)", 0.5, 0.1, 2., 0.05)]),
        _ => None,
    }
}

/// Wilder +DI, -DI and ADX (TradeStation DMIPlus / DMIMinus / ADX; same smoothing as connors::dmi).
pub fn dmi3(x: &Ctx, n: usize) -> (Vec<f64>, Vec<f64>, Vec<f64>) {
    let len = x.c.len();
    let (mut pdm, mut mdm, mut tr) = (vec![f64::NAN; len], vec![f64::NAN; len], vec![f64::NAN; len]);
    let trs = true_range(x);
    for i in 1..len {
        let (up, dn) = (x.h[i] - x.h[i - 1], x.l[i - 1] - x.l[i]);
        pdm[i] = if up > dn && up > 0. { up } else { 0. };
        mdm[i] = if dn > up && dn > 0. { dn } else { 0. };
        tr[i] = trs[i];
    }
    let (sp, sm, st) = (wilder(&pdm, n), wilder(&mdm, n), wilder(&tr, n));
    let (mut p, mut m, mut dx) = (vec![f64::NAN; len], vec![f64::NAN; len], vec![f64::NAN; len]);
    for i in 0..len {
        if st[i] > 0. {
            p[i] = 100. * sp[i] / st[i]; m[i] = 100. * sm[i] / st[i];
            dx[i] = if p[i] + m[i] > 0. { 100. * (p[i] - m[i]).abs() / (p[i] + m[i]) } else { 0. };
        }
    }
    (p, m, wilder(&dx, n))
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if from >= to || to >= x.r.len() { return None; }
    let (h, l, c, op) = (&x.h, &x.l, &x.c, &x.o);
    let tr = true_range(x);
    let hl: Vec<f64> = (0..c.len()).map(|i| h[i] - l[i]).collect();
    let ave10 = sma(&hl, 10);
    match id {
        ESL4 => {
            let (ef, mm, tf) = (g(p, "ent_frac", 0.70), g(p, "mm_mult", 0.75), g(p, "trail_fr", 1.5));
            let (dp, dm, adx) = dmi3(x, 14);
            let ave14 = sma(&hl, 14);
            let (mut entpr, mut xmm, mut xtrail, mut trail_on) = (0f64, 0f64, 0f64, false);
            Some(run_ts(x, from, to, |i, st, now, next| {
                let ep = if st.pos != 0 { st.entry_px } else { 0. };
                let bse = if st.pos != 0 { st.bse(i) } else { 0 };
                if st.pos == 1 && !trail_on && c[i] >= ep && dp[i] > dm[i] { trail_on = true; }
                if st.pos == 1 && trail_on && dp[i] < dm[i] { trail_on = false; }
                let cond1 = !trail_on && op[i] > ep && bse > 0;
                let cond2 = dp[i] < dm[i] || (i >= 5 && c[i] < c[i - 5]);
                let cond3 = adx[i] > 20.;
                if (st.pos != 1 || cond1) && cond2 && cond3 && ave14[i].is_finite() && ave10[i].is_finite() {
                    let brk = (ef * tr[i]).max(0.5 * ave14[i]).min(1.5 * ave14[i]);
                    entpr = c[i] + brk;
                    xmm = entpr - mm * ave10[i];
                    next.push(o(A::Buy, P::Stop(entpr), "Buy (ESL4)"));
                    xtrail = c[i] - tf * tr[i];
                    trail_on = false;
                }
                if st.pos == 1 && bse == 0 { xmm = xmm - entpr + st.entry_px; }
                next.push(o(A::Xl, P::Stop(xmm), "MMStop"));
                if trail_on && st.pos == 1 { xtrail = xtrail.max(c[i] - tf * tr[i]); next.push(o(A::Xl, P::Stop(xtrail), "Trailing stop")); }
                if !trail_on && st.pos == 1 && op[i] > st.entry_px && bse > 0 { now.push(o(A::Xl, P::Mkt, "Open > EntryPrice: sell this bar at close")); }
            }))
        }
        NQL4 => {
            let (ef, mm) = (g(p, "ent_frac", 0.65), g(p, "mm_mult", 0.75));
            let atr14 = sma(&tr, 14);
            let (mut xmm, mut xtrail, mut trail_on) = (0f64, 0f64, false);
            Some(run_ts(x, from, to, |i, st, _now, next| {
                if i >= 2 && c[i] < c[i - 2] && atr14[i].is_finite() && ave10[i].is_finite() {
                    let brk = (ef * tr[i]).max(0.5 * atr14[i]).min(1.5 * atr14[i]);
                    let entpr = c[i] + brk;
                    xmm = entpr - mm * ave10[i];
                    next.push(o(A::Buy, P::Stop(entpr), "Buy (NQL4)"));
                    if st.pos != 1 { xtrail = c[i] - mm * tr[i]; }
                    trail_on = false;
                }
                if st.pos == 1 {
                    let k = i - (st.bse(i) + 1);
                    xmm = st.entry_px - mm * ave10[k];
                }
                next.push(o(A::Xl, P::Stop(xmm), "MMStop"));
                if st.pos == 1 && !trail_on && c[i] >= st.entry_px { trail_on = true; }
                if trail_on && st.pos == 1 { xtrail = xtrail.max(c[i] - mm * (h[i] - l[i])); next.push(o(A::Xl, P::Stop(xtrail), "Trailing stop")); }
            }))
        }
        S4 => {
            let (ef, mmf, tx) = (g(p, "ent_fr", 0.75), g(p, "mm_fr", 1.0), g(p, "trail_x", 0.5));
            let (dp, dm, _) = dmi3(x, 14);
            let (mut entpr, mut xshort) = (0f64, 99999999f64);
            Some(run_ts(x, from, to, |i, st, _now, next| {
                if i >= 2 && dm[i] > dp[i] && c[i] > c[i - 1] && c[i - 1] > c[i - 2] && ave10[i].is_finite() {
                    entpr = c[i] - ef * ave10[i];
                    next.push(o(A::Sell, P::Stop(entpr), "Sell Short (S4)"));
                }
                if st.pos == 0 { xshort = entpr + mmf * ave10[i]; }
                if st.pos == -1 {
                    xshort = if st.bse(i) > 2 || (st.entry_px - c[i]) > 2. * ave10[i] { xshort.min(c[i] + tx * ave10[i]) } else { st.entry_px + mmf * ave10[i] };
                }
                if xshort.is_finite() { next.push(o(A::Xs, P::Stop(xshort), "XShort")); }
            }))
        }
        _ => None,
    }
}
