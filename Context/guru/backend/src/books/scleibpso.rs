//! Lee Leibfarth, "Premier Stochastic Oscillator", Technical Analysis of Stocks & Commodities V.26:8 (August 2008) pp.30-36
//! (research share Trading Knowledge I/Stocks & Commodities DVD 11.26 (1982 - 2008) (traders.com)/VOLUMES/V26/C08/143LEIB.pdf):
//! the printed TradeStation strategy (sidebar Figure 1): crossings of the premier stochastic at +-0.90 / +-0.20, next-bar market
//! entry, money stop and profit target. Book id `scleibpso`. Lane I2 (Q385 row 88, 24 Nov 2026, R4 #382);
//! independent check authors/conv-1123/i2w/scleibpso_ref.py.
use crate::connors::{Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};

const ID: &str = "scleibpso_bracket";
pub const REVIEWED: &[&str] = &[ID];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const NOTE: &str = "A|Declared readings (lane I2, 4; R4 #340/#348/#380/#382): (1) the strategy's US$800 stop and US$1,600 profit target (E-mini Russell 2000, US$50 per point = 16 and 32 points near the 700 level of the sidebar chart) are carried as ONE price-relative rule: points x close before the entry bar / 700; (2) a stop and a target touched on the same daily bar are taken stop first (the share of such bars is measured on the gate row); (3) an opposite signal while in a trade reverses at the next open, a same-side signal is ignored (TradeStation default, one position); (4) the stochastic's raw %K over 8 bars including the current bar (the printed Stochastic call has smoothing length 1 on the slow line), a flat range reads 50. The printed code: NormStoch = 0.1 x (SlowK - 50), double EMA of length sqrt(25) = 5, Premier = (e^S - 1) / (e^S + 1). Entry: Premier crosses under +0.90 or +0.20 buys, crosses over -0.90 or -0.20 sells short, at the next bar's open. The article tests the daily E-mini Russell 2000 continuous contract (2008); the vault applies the printed rule to the NSE index series, declared adaptation, no costs.";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef { id: ID, name: "Premier stochastic (Leibfarth): buy when it crosses under +0.90 or +0.20, sell short when it crosses over -0.90 or -0.20, money stop and 2:1 profit target", group: "S&C: Premier Stochastic Oscillator (Leibfarth, V.26:8)", session: 0, dir: "Long/Short", status: Status::Live,
        needs_volume: false, needs_vix: false, needs_series: &[], book: "scleibpso",
        rules: &[
            "E|\"if Premier crosses over Line1 or Premier crosses over Line2 then sellshort (\\\"S1 Sell\\\") next bar at market; if Premier crosses under Line1 * -1 or Premier crosses under Line2 * -1 then buy (\\\"L1 Buy\\\") next bar at market;\" with Line1(-.9), Line2(-.2) (V.26:8, sidebar Figure 1, PDF p.5)",
            "X|\"setstopcontract; setstoploss(StopLoss); setprofittarget(ProfitTarget);\" with StopLoss(800), ProfitTarget(1600) (PDF p.5); \"the profit target is set at $1,600 with a stop-loss risk of $800\" (PDF p.4)",
            "F|\"NormStoch = .1 * (oslowK - 50); SmoothStoch = xaverage(xaverage(NormStoch, Length), Length); Premier = (expValue(1 * SmoothStoch) - 1) / (expValue(1 * SmoothStoch) + 1)\"; Length = squareroot(Period), Period 25, StochLength 8 (PDF p.4)",
            NOTE],
        notes: "Leibfarth, S&C V.26:8: premier stochastic oscillator strategy with the printed code. Four labelled readings; declared adaptation to the NSE index series.",
        claim: None }]
}
pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> { reviewed(id).then_some((ID, "Premier stochastic (Leibfarth, S&C)", "8 / 25, 0.90 / 0.20")) }
pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    reviewed(id).then(|| vec![
        ParamDef { key: "len", label: "Stochastic length (article 8)", def: 8., min: 3., max: 30., step: 1. },
        ParamDef { key: "per", label: "Smoothing period, EMA length = sqrt (article 25)", def: 25., min: 4., max: 100., step: 1. },
        ParamDef { key: "stop", label: "Stop in Russell points at the 700 level (article $800 = 16)", def: 16., min: 2., max: 60., step: 1. },
        ParamDef { key: "tgt", label: "Target in Russell points at the 700 level (article $1,600 = 32)", def: 32., min: 4., max: 120., step: 1. }])
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.c.len() { return None; }
    let g = |k: &str, d: f64| p.get(k).copied().unwrap_or(d);
    let (n, per) = (g("len", 8.).round().max(2.) as usize, g("per", 25.).max(4.));
    let (sp, tp) = (g("stop", 16.), g("tgt", 32.));
    let (o, h, l, c) = (&x.o, &x.h, &x.l, &x.c);
    let m = c.len();
    let a = 2.0 / (per.sqrt() + 1.0);
    let mut prem = vec![f64::NAN; m];
    let (mut e1, mut e2) = (f64::NAN, f64::NAN);
    for i in n - 1..m {
        let hh = h[i + 1 - n..=i].iter().copied().fold(f64::NEG_INFINITY, f64::max);
        let ll = l[i + 1 - n..=i].iter().copied().fold(f64::INFINITY, f64::min);
        let k = if hh > ll { 100.0 * (c[i] - ll) / (hh - ll) } else { 50.0 };
        let ns = 0.1 * (k - 50.0);
        e1 = if e1.is_nan() { ns } else { e1 + a * (ns - e1) };
        e2 = if e2.is_nan() { e1 } else { e2 + a * (e1 - e2) };
        let ex = e2.exp();
        prem[i] = (ex - 1.0) / (ex + 1.0);
    }
    let mut out: Vec<Trade> = vec![];
    let mut pos: Option<(f64, usize, f64, f64, f64)> = None; // dir, entry bar, entry px, stop px, target px
    let mut pend = 0.0f64;
    let mk = |dir: f64, e: usize, ep: f64, xi: usize, xp: f64, why: &str, open: bool| Trade { dir, legs: vec![Leg { i: e, px: ep, w: 1.0 }], exit_i: xi, exit_px: xp, reason: why.into(), entry_kind: "open", signal_i: e, open, exits: Vec::new() };
    for d in from.max(n + 1)..=to {
        if pend != 0.0 {
            let dir = pend; pend = 0.0;
            match pos {
                Some((pd, e, ep, ..)) if pd != dir => { out.push(mk(pd, e, ep, d, o[d], "reversed", false)); pos = None; }
                _ => {}
            }
            if pos.is_none() {
                let (ks, kt) = (sp * c[d - 1] / 700.0, tp * c[d - 1] / 700.0);
                pos = Some((dir, d, o[d], o[d] - dir * ks, o[d] + dir * kt));
            }
        }
        if let Some((dir, e, ep, st, tg)) = pos {
            let stop_hit = if dir > 0.0 { l[d] <= st } else { h[d] >= st };
            let tgt_hit = if dir > 0.0 { h[d] >= tg } else { l[d] <= tg };
            if stop_hit { out.push(mk(dir, e, ep, d, if dir > 0.0 { o[d].min(st) } else { o[d].max(st) }, "stop", false)); pos = None; }
            else if tgt_hit { out.push(mk(dir, e, ep, d, if dir > 0.0 { o[d].max(tg) } else { o[d].min(tg) }, "target", false)); pos = None; }
        }
        if d < to && prem[d].is_finite() && prem[d - 1].is_finite() {
            let (p0, p1) = (prem[d - 1], prem[d]);
            if (p0 >= 0.9 && p1 < 0.9) || (p0 >= 0.2 && p1 < 0.2) { pend = 1.0; }
            else if (p0 <= -0.9 && p1 > -0.9) || (p0 <= -0.2 && p1 > -0.2) { pend = -1.0; }
            if pend != 0.0 { if let Some((pd, ..)) = pos { if pd == pend { pend = 0.0; } } }
        }
    }
    if let Some((dir, e, ep, ..)) = pos { out.push(mk(dir, e, ep, to, c[to], "open", true)); }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn premier_bracket_trades() {
        let r = super::super::sctraub::tests::bars(3000);
        let x = Ctx::new(&r, &r, &[]);
        let t = run(ID, &x, 0, r.len() - 1, &Params::new()).unwrap();
        assert!(t.len() > 20);
        for z in &t { assert!(z.exit_i >= z.legs[0].i); assert!(z.legs[0].px >= r[z.legs[0].i].low - 1e-9 && z.legs[0].px <= r[z.legs[0].i].high + 1e-9); }
        for w in t.windows(2) { assert!(w[1].legs[0].i >= w[0].exit_i); }
    }
    /// hand-run dump for the pandas reference: Q385_CSV=<idx_data.csv> Q385_OUT5=<file> cargo test --release scleibpso_dump -- --ignored
    #[test]
    #[ignore]
    fn scleibpso_dump() {
        use std::io::Write;
        let data = crate::load_csv(&std::env::var("Q385_CSV").unwrap()).unwrap();
        let mut f = std::io::BufWriter::new(std::fs::File::create(std::env::var("Q385_OUT5").unwrap()).unwrap());
        writeln!(f, "id,ticker,dir,entry_date,entry_px,exit_date,exit_px,open").unwrap();
        for (tk, rows) in data.iter() {
            let x = Ctx::new(rows, &[], &[]);
            for z in run(ID, &x, 0, rows.len() - 1, &Params::new()).unwrap() {
                writeln!(f, "{ID},{tk},{},{},{},{},{},{}", z.dir, rows[z.legs[0].i].date, z.legs[0].px, rows[z.exit_i].date, z.exit_px, z.open).unwrap();
            }
        }
    }
}

pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book == "scleibpso",
    label: "Leibfarth tested the premier stochastic strategy on the daily E-mini Russell 2000 continuous contract (2008); the vault runs the printed rule on the five NSE index series (NIFTY 50, NIFTYBANK, NIFTYIT, NIFTYMIDCAP100, NIFTYMIDCAPSELECT), a declared adaptation, indices only.",
    basis: "Technical Analysis of Stocks & Commodities V.26:8 pp.30-36 (Premier Stochastic Oscillator); independent check authors/conv-1123/i2w/scleibpso_ref.py",
};
