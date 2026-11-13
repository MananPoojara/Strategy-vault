//! Weaver-DT (alfaranda.com), TradeStation strategy code "Code.txt" (research share USB HDD 1/TradeStation Systems & Indicators/
//! Weaver-DT System for TS (alfaranda.com)/Code.rar -> Code.txt). Book id `alwdt`. Lane K18 (titles loop, 2 Nov 2026), to-code row
//! 350; independent check authors/titles-1030/K18w/k18_ref.py.
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};
use super::rrsim::{o, run_ts, A, P};

pub const REVIEWED: &[&str] = &["alwdt_trendline"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const NOTE: &str = "[Declared readings] TradeStation code as printed (STRENGTH 1), on daily bars. Swing low: Low[1] below the TrueLow of the current bar and below the TrueLow of the first earlier bar whose low differs from it (equal lows skipped, the code's E counter); swing highs mirrored with TrueHigh. The last 20 swing lows and highs are kept with their ages in bars. Down trendline: POINT2 = the latest swing high, POINT1 = the most recent earlier swing high above it; SLOPE = (POINT2 - POINT1) / bars between them; the line is walked from POINT1 one bar at a time to the next bar, and ADD = line - Low at the bar of the lowest low met on the walk (the channel width). When not long and the close is under the line (or the high under the line - SLOPE): buy next bar on a stop at the line, with the target LONGX = line + ADD as a sell limit while long. Up trendline under the swing lows mirrored: sell short on a stop at the line, cover limit at line + ADD (ADD is negative there); the short condition uses the long side's SLOPE variable, as printed. The opposite stop reverses the position; there is no stop loss. The walk's 'IF W <= MAXBARSBACK' guard only protects TradeStation's reference buffer (the strategy's MaxBarsBack setting is not printed), so the walk is limited by the vault reading mbb = 50, TradeStation's default (mbb = 0: every bar back to POINT1, the K18 run). Swing arrays start at 0 as the code's arrays do. Fills on the vault's TradeStation daily-bar engine (rrsim). No market is named and no results are printed. Runs on daily indices, ETFs and stocks, US and India, are a declared application. [R2E, R4 #297 / #309 (1)] MaxBarsBack = 50 is a labelled reading of an unstated setting. The card stays WITHHELD: the engine (rrsim) also works the LX/SX profit-target limits sent while flat, so a stop entry and its target can both fill on the entry bar, while TradeStation acts on an exit order only for a position held when it is placed; until the engine follows that, the exit rule is not what the code does.";

/// Withheld (prepared 3 Nov 2026, lane K23; the live card changes only after the user's OK). The run code stays for a re-admission.
const HELD: &str = "withheld (R4 #309 (1), lane R2E 13 Nov 2026: MaxBarsBack is an unstated detail under #297, read as TradeStation's default 50, but the engine fills the LX/SX targets on the entry bar, unlike TradeStation; earlier reason): the trendline walk is limited by 'IF W<=MAXBARSBACK', TradeStation's Maximum Bars Back chart setting, which the code does not state; the limit changes the channel width (ADD) and so the LONGX / SHORTX profit targets. The same unprinted setting is why the TradeStation trend-line family of row 308 is held (lead decisions 36, 135, 157). Also open if it is re-admitted: the code's limit exit is sent on the order bar while flat, which cashpf_adx reads as TradeStation does (an exit acts only on a position held at the close it is placed)";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef { id: "alwdt_trendline", name: "Weaver-DT: break of the trendline through the last two falling swing highs (rising swing lows) on a stop, channel-width target", group: "TradeStation code: Weaver-DT (alfaranda.com)", session: 0, dir: "Long/Short", status: Status::NotTestable(HELD),
        needs_volume: false, needs_vix: false, needs_series: &[], book: "alwdt", rules: &[
            "F|POINT2 = HP[0] (latest swing high), POINT1 = the latest earlier HP[Y] > HP[0]; SLOPE = (POINT2 - POINT1) / (POINT1BAR - POINT2BAR); LINE walked from POINT1 to the next bar; ADD = LINE - LL at the lowest low on the walk",
            "E|IF MARKETPOSITION<1 AND (C<LINE OR H<LINE-SLOPE) THEN BEGIN LONGX=LINE+ADD; Buy(\"Buy\") Next Bar LINE STOP; END; (short side mirrored on the swing lows, Sell Short at ULINE STOP)",
            "X|IF LONGX<99999 THEN Sell(\"LX\") Next Bar LONGX LIMIT; IF SHORTX>0 THEN Buy to Cover(\"SX\") Next Bar SHORTX LIMIT; no stop loss, the opposite entry reverses"],
        notes: NOTE, claim: None }]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> { reviewed(id).then_some(("alwdt_trendline", "Weaver-DT (TradeStation)", "STRENGTH 1")) }

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    reviewed(id).then(|| vec![p("mbb", "MaxBarsBack limit on the walk (vault reading = TradeStation's default 50; 0 = no limit)", 50., 0., 500., 1.)])
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.r.len() { return None; }
    let mbb = p.get("mbb").copied().unwrap_or(50.);
    let (h, l, c) = (&x.h, &x.l, &x.c);
    let tl = |k: usize| if k == 0 { l[0] } else { l[k].min(c[k - 1]) };
    let th = |k: usize| if k == 0 { h[0] } else { h[k].max(c[k - 1]) };
    let (mut lp, mut lpb, mut hp, mut hpb) = ([0f64; 20], [0f64; 20], [0f64; 20], [0f64; 20]);
    let (mut slope, mut add, mut longx, mut shortx) = (0f64, 0f64, 99999f64, 0f64);
    let (mut lp1bar, mut p1bar) = (0f64, 0f64);
    let within = |w: f64| mbb <= 0. || w <= mbb;
    Some(run_ts(x, from, to, |i, st, _now, next| {
        let mp = st.pos;
        if mp < 1 { longx = 99999.; }
        if mp > -1 { shortx = 0.; }
        for z in 0..20 { lpb[z] += 1.; hpb[z] += 1.; }
        // swing low at bar i-1
        if i >= 1 && l[i - 1] < tl(i) {
            let mut e = 1;
            while i >= 1 + e && l[i - 1] == l[i - 1 - e] { e += 1; }
            if i >= 1 + e && l[i - 1] < tl(i - 1 - e) {
                for z in (1..20).rev() { lp[z] = lp[z - 1]; lpb[z] = lpb[z - 1]; }
                lp[0] = l[i - 1]; lpb[0] = 1.;
            }
        }
        let (lpt2, lpt2b) = (lp[0], lpb[0]);
        let mut lpt1 = 0.;
        let mut y = 1;
        while lpt1 == 0. && y < 20 { if lp[y] < lp[0] { lpt1 = lp[y]; lp1bar = lpb[y]; } y += 1; }
        if lpt1 > 0. && lp1bar > lpt2b {
            let us = (lpt2 - lpt1) / (lp1bar - lpt2b);
            let mut ul = lpt1;
            let mut hh = 0.;
            let mut w = lp1bar;
            while w >= 0. {
                let k = w as usize;
                if within(w) && k <= i && h[i - k] > hh { hh = h[i - k]; add = ul - hh; }
                ul += us; w -= 1.;
            }
            if mp > -1 && (c[i] > ul || l[i] > ul - slope) { shortx = ul + add; next.push(o(A::Sell, P::Stop(ul), "Sell")); }
        }
        if shortx > 0. { next.push(o(A::Xs, P::Lim(shortx), "SX")); }
        // swing high at bar i-1
        if i >= 1 && h[i - 1] > th(i) {
            let mut e = 1;
            while i >= 1 + e && h[i - 1] == h[i - 1 - e] { e += 1; }
            if i >= 1 + e && h[i - 1] > th(i - 1 - e) {
                for z in (1..20).rev() { hp[z] = hp[z - 1]; hpb[z] = hpb[z - 1]; }
                hp[0] = h[i - 1]; hpb[0] = 1.;
            }
        }
        let (pt2, pt2b) = (hp[0], hpb[0]);
        let mut pt1 = 0.;
        let mut y = 1;
        while pt1 == 0. && y < 20 { if hp[y] > hp[0] { pt1 = hp[y]; p1bar = hpb[y]; } y += 1; }
        if pt1 > 0. && p1bar > pt2b {
            slope = (pt2 - pt1) / (p1bar - pt2b);
            let mut line = pt1;
            let mut ll = 99999.;
            let mut w = p1bar;
            while w >= 0. {
                let k = w as usize;
                if within(w) && k <= i && l[i - k] < ll { ll = l[i - k]; add = line - ll; }
                line += slope; w -= 1.;
            }
            if mp < 1 && (c[i] < line || h[i] < line - slope) { longx = line + add; next.push(o(A::Buy, P::Stop(line), "Buy")); }
        }
        if longx < 99999. { next.push(o(A::Xl, P::Lim(longx), "LX")); }
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn weaver_trades() {
        let r = super::super::sctraub::tests::bars(2500);
        let x = Ctx::new(&r, &r, &[]);
        let t = run("alwdt_trendline", &x, 0, r.len() - 1, &Params::new()).unwrap();
        assert!(t.len() > 3);
        for w in t.windows(2) { assert!(w[1].legs[0].i >= w[0].exit_i); }
        assert!(run("alwdt_x", &x, 0, 5, &Params::new()).is_none());
        // R2E (R4 #297): the default is the MaxBarsBack 50 reading
        let t50 = run("alwdt_trendline", &x, 0, r.len() - 1, &[("mbb".to_string(), 50.)].into_iter().collect()).unwrap();
        assert_eq!(t.iter().map(|z| (z.legs[0].i, z.exit_i)).collect::<Vec<_>>(), t50.iter().map(|z| (z.legs[0].i, z.exit_i)).collect::<Vec<_>>());
        assert!(matches!(catalogue()[0].status, Status::NotTestable(_))); // R4 #309 (1): withheld
    }
}

// Instrument admission: Weaver-DT 2026-11-02 (titles lane K18).
pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book == "alwdt" && reviewed(def.id),
    label: "TradeStation trendline-breakout code with no named market and no published results. Runs on daily indices, ETFs and stocks, US and India, are a declared application.",
    basis: "Weaver-DT Code.txt (alfaranda.com, printed code); R4 #297 (MaxBarsBack = TradeStation default 50, labelled); independent check authors/titles-1030/K18w/k18_ref.py (re-run by lane R2E)",
};
