//! MiniMax II(4) v2 — Mike Bryant, Breakout Futures (June 2002), the TradeStation 4.0 EasyLanguage code itself. Source (research
//! drive, read in place, read-only): `USB HDD 1/TradeStation Systems & Indicators/MiniMax I & II for TS,TS8, Excel + Manuals, $525
//! (breakoutfutures.com)/v2/Code.rar` -> `Code.txt` (extracted as data). Book id `bfm2c`. Notes: library/bfm2c/README.md.
//! The User's Guide version of the same system is `bfmm_mm2_es` / `bfmm_mm2_nq` (module bfmm, own simulator); this module runs the
//! code literally on the shared TradeStation engine books::rrsim. Code-vs-guide differences: see the README.
use crate::books::rrsim::{g, o, pd, run_ts, sdef, sma, true_range, A, P};
use crate::connors::{claim, Ctx, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "bfm2c";

const RULES: &[&str] = &[
    "F|\"TrRev = TrendFr * Average(TrueRange, 10); If BarNumber = 1 then Begin Trend = 1; HC = Close; End; If Trend > 0 then Begin If Close > HC then HC = Close else If (HC - Close) > TrRev then Begin Trend = -1; LC = Close; End; End; If Trend < 0 then Begin If Close < LC then LC = Close else If (Close - LC) > TrRev then Begin Trend = 1; HC = Close; End; End;\"",
    "F|\"If BarNumber > 1 and SetupL and High >= EntPrL then SetupL = false\" (short mirror with Low <= EntPrS); \"If BarNumber > 1 and Low < Low[1] and Close < Close[2] and Trend[1] > 0 and MarketPosition <> 1 then SetupL = true\"; \"... High > High[1] and Close > Close[2] and Trend[1] < 0 and MarketPosition <> -1 then SetupS = true\"",
    "E|\"If SetupL then ... BrkAmntL = MaxList(EntryFrL * TrueRange, 0.5 * Average(TrueRange, 10)); BrkAmntL = MinList(BrkAmntL, 1.5 * Average(TrueRange, 10)); EntPrL = C + BrkAmntL; MMSizeL = ExitFrL * Average(TrueRange, 10); XMML = EntPrL - MMSizeL; Buy(\\\"EnterL\\\") NConL contracts next bar at EntPrL stop; if Trend < 0 then XTargL = EntPrL + TargFr1 * Average(TrueRange, 10) else XTargL = EntPrL + TargFr2 * Average(TrueRange, 10); Sell(\\\"TargL0\\\") next bar at XTargL limit\" (short mirror: EntPrS = C - BrkAmntS, Sell Short stop, target below; Trend > 0 -> TargFr1)",
    "X|\"If MarketPosition = 1 and BarsSinceEntry = 0 then XMML = EntryPrice - MMSizeL; XTrailL = 0\"; \"Sell(\\\"MM StopL\\\") next bar at XMML stop; Buy to Cover(\\\"MM StopS\\\") next bar at XMMS stop\" (placed every bar)",
    "X|\"If MarketPosition = 1 then XTrailL = MaxList(XTrailL, C - ExitFrL * TrueRange); Sell(\\\"TrailL\\\") next bar at XTrailL stop\" (short: MinList(XTrailS, C + ExitFrS * TrueRange), from 9999999)",
    "X|\"If MarketPosition = 1 and Trend[1] < 0 and open > EntryPrice then Sell(\\\"BailL\\\") this bar at close\"; short: \"Trend[1] > 0 and open < EntryPrice then Buy to Cover(\\\"BailS\\\") this bar at close\"",
    "X|\"If MarketPosition = 1 then if Trend < 0 then XTargL = EntryPrice + TargFr1 * Average(TrueRange, 10) else XTargL = EntryPrice + TargFr2 * Average(TrueRange, 10); Sell(\\\"TargL\\\") next bar at XTargL limit\" (short mirror, Trend > 0 -> TargFr1)",
    "F|\"Recommended Input Parameter Values: PARAMETER S&P NASDAQ / EntryFrL 0.3 0.6 / EntryFrs 0.7 0.3 / ExitFrL 0.5 0.4 / ExitFrs 0.6 0.8 / TrendFr 1.0 1.0 / TargFr1 1.0 1.0 / TargFr2 2.0 2.0 / RiskPer 4.0 4.0\" (code header; the Input defaults are the NASDAQ set); Equity(0) -> 1 contract (NConL = 1)",
];
const NOTE: &str = "Code literal on the rrsim TradeStation daily-bar engine: next-bar stop/limit orders fill at their price (or the open when the open \
is already through), the bar runs open -> nearer extreme -> other extreme -> close, and an exit acts on a position entered earlier on the same bar \
(so the entry-day MM stop and the 'TargL0' target placed with the entry order can fill on the entry day, unlike bfmm_mm2_* which starts exits the day \
after). An opposite entry stop reverses. BarNumber 1 = the first bar on which Average(TrueRange,10) exists (mbb). E-mini S&P / Nasdaq 100 futures in \
the code; run on the index (instrument swap, no costs; the guide's figures deduct $75 per trade). India: instrument swap; index shorts hypothetical.";

pub fn catalogue() -> Vec<StrategyDef> {
    let mut a = sdef(BOOK, "bfm2c_es", "MiniMax II v2 code (S&P parameters)", "MiniMax II v2 (EasyLanguage code)", "Long/Short", Status::Live, RULES, NOTE);
    a.claim = claim("242 (133 long 44.36%, 109 short 52.29%)", "47.93%", "$175.41 per contract (net $75)", "2 bars winners, 1 bar losers", "1/1/98-6/25/02, E-mini S&P 500 (User's Guide, same system)");
    let mut b = sdef(BOOK, "bfm2c_nq", "MiniMax II v2 code (NASDAQ parameters = code Input defaults)", "MiniMax II v2 (EasyLanguage code)", "Long/Short", Status::Live, RULES, NOTE);
    b.claim = claim("161 (82 long 51.22%, 79 short 53.16%)", "52.17%", "-", "-", "6/22/99-6/25/02, E-mini Nasdaq 100 (User's Guide, same system)");
    vec![a, b]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    match id {
        "bfm2c_es" => Some(("bfmm_mm2", "MiniMax II (Bryant)", "EasyLanguage code, S&P parameters")),
        "bfm2c_nq" => Some(("bfmm_mm2", "MiniMax II (Bryant)", "EasyLanguage code, NASDAQ parameters")),
        _ => None,
    }
}

fn defs(id: &str) -> Option<[f64; 7]> {
    match id {
        "bfm2c_es" => Some([0.3, 0.7, 0.5, 0.6, 1.0, 1.0, 2.0]),
        "bfm2c_nq" => Some([0.6, 0.3, 0.4, 0.8, 1.0, 1.0, 2.0]),
        _ => None,
    }
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let d = defs(id)?;
    Some(vec![
        pd("entry_fr_l", "EntryFrL (x prior day's true range, long breakout)", d[0], 0.1, 1.5, 0.05),
        pd("entry_fr_s", "EntryFrS (short breakout)", d[1], 0.1, 1.5, 0.05),
        pd("exit_fr_l", "ExitFrL (MM stop x ATR10; trail x TR)", d[2], 0.1, 1.5, 0.05),
        pd("exit_fr_s", "ExitFrS", d[3], 0.1, 1.5, 0.05),
        pd("trend_fr", "TrendFr (x ATR10 trend reversal)", d[4], 0.25, 3., 0.25),
        pd("targ_fr1", "TargFr1 (target against the trend, x ATR10)", d[5], 0.25, 3., 0.25),
        pd("targ_fr2", "TargFr2 (target with the trend, x ATR10)", d[6], 0.5, 5., 0.25),
        pd("mbb", "Bars before BarNumber 1 (MaxBarsBack)", 10., 10., 100., 1.),
    ])
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    let d = defs(id)?;
    if from >= to || to >= x.r.len() { return None; }
    let (efl, efs, xfl, xfs, tf, t1, t2) = (g(p, "entry_fr_l", d[0]), g(p, "entry_fr_s", d[1]), g(p, "exit_fr_l", d[2]), g(p, "exit_fr_s", d[3]),
        g(p, "trend_fr", d[4]), g(p, "targ_fr1", d[5]), g(p, "targ_fr2", d[6]));
    let mbb = g(p, "mbb", 10.).max(10.) as usize;
    let tr = true_range(x);
    let atr = sma(&tr, 10);
    let (o_, h, l, c) = (&x.o, &x.h, &x.l, &x.c);
    let mut trend = vec![0f64; c.len()];
    let (mut hc, mut lc) = (0f64, 9999999f64);
    let (mut setup_l, mut setup_s) = (false, false);
    let (mut ent_l, mut ent_s, mut mm_l, mut mm_s) = (0f64, 0f64, 0f64, 0f64);
    let (mut xmml, mut xmms, mut trail_l, mut trail_s) = (0f64, 0f64, 0f64, 9999999f64);
    Some(run_ts(x, from, to, |i, st, now, next| {
        if i < mbb { return; }
        let bn = i - mbb + 1;
        let a = atr[i];
        let trrev = tf * a;
        let mut t = if bn == 1 { 0. } else { trend[i - 1] };
        if bn == 1 { t = 1.; hc = c[i]; }
        if t > 0. { if c[i] > hc { hc = c[i]; } else if hc - c[i] > trrev { t = -1.; lc = c[i]; } }
        if t < 0. { if c[i] < lc { lc = c[i]; } else if c[i] - lc > trrev { t = 1.; hc = c[i]; } }
        trend[i] = t;
        let t1b = if bn > 1 { trend[i - 1] } else { 0. };
        if bn > 1 && setup_l && h[i] >= ent_l { setup_l = false; }
        if bn > 1 && setup_s && l[i] <= ent_s { setup_s = false; }
        if bn > 1 && l[i] < l[i - 1] && c[i] < c[i - 2] && t1b > 0. && st.pos != 1 { setup_l = true; }
        if bn > 1 && h[i] > h[i - 1] && c[i] > c[i - 2] && t1b < 0. && st.pos != -1 { setup_s = true; }
        if setup_l {
            let brk = (efl * tr[i]).max(0.5 * a).min(1.5 * a);
            ent_l = c[i] + brk; mm_l = xfl * a; xmml = ent_l - mm_l;
            next.push(o(A::Buy, P::Stop(ent_l), "EnterL"));
            let tg = ent_l + (if t < 0. { t1 } else { t2 }) * a;
            next.push(o(A::Xl, P::Lim(tg), "TargL0"));
        }
        if setup_s {
            let brk = (efs * tr[i]).max(0.5 * a).min(1.5 * a);
            ent_s = c[i] - brk; mm_s = xfs * a; xmms = ent_s + mm_s;
            next.push(o(A::Sell, P::Stop(ent_s), "EnterS"));
            let tg = ent_s - (if t > 0. { t1 } else { t2 }) * a;
            next.push(o(A::Xs, P::Lim(tg), "TargS0"));
        }
        if st.pos == 1 && st.bse(i) == 0 { xmml = st.entry_px - mm_l; trail_l = 0.; }
        if st.pos == -1 && st.bse(i) == 0 { xmms = st.entry_px + mm_s; trail_s = 9999999.; }
        next.push(o(A::Xl, P::Stop(xmml), "MM StopL"));
        next.push(o(A::Xs, P::Stop(xmms), "MM StopS"));
        if st.pos == 1 { trail_l = trail_l.max(c[i] - xfl * tr[i]); next.push(o(A::Xl, P::Stop(trail_l), "TrailL")); }
        if st.pos == -1 { trail_s = trail_s.min(c[i] + xfs * tr[i]); next.push(o(A::Xs, P::Stop(trail_s), "TrailS")); }
        if st.pos == 1 && t1b < 0. && o_[i] > st.entry_px { now.push(o(A::Xl, P::Mkt, "BailL")); }
        if st.pos == -1 && t1b > 0. && o_[i] < st.entry_px { now.push(o(A::Xs, P::Mkt, "BailS")); }
        if st.pos == 1 { let tg = st.entry_px + (if t < 0. { t1 } else { t2 }) * a; next.push(o(A::Xl, P::Lim(tg), "TargL")); }
        if st.pos == -1 { let tg = st.entry_px - (if t > 0. { t1 } else { t2 }) * a; next.push(o(A::Xs, P::Lim(tg), "TargS")); }
    }))
}
