//! K.A.L.O.S.Z. Archive of MetaStock Formulas (Polish web archive 'Archiwum formul - MetaStock', saved 2001; research share
//! USB HDD 1/MetaStock/@Systems & Indicators/K.A.L.O.S.Z Archive of Metastock Formulas/K.A.L.O.S.Z Archive of Metastock
//! Formulas.pdf). Book id `kzmsf`. Live: Marek Lewandowski's 'MACDhistogram Trading System' and Chris Lindstedt's
//! 'No Name 1' (written for Svein Andersen, MetaStock list, 14 Jul 1998). Lane K6 (titles loop, 31 Oct 2026);
//! independent check authors/titles-1030/K6w/k6_ref.py (header).
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};
use super::mkits::{ema, macd, stoch};
use super::rbtie::{mk, target_trades};
use super::svtvc::sma_exact as sma;

/// Admitted ids (authors/titles-1030/K6w/k6_ref.py (header)).
pub const REVIEWED: &[&str] = &["kzmsf_macd_hist", "kzmsf_no_name_1"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const NOTE_M: &str = "[Declared readings] MetaStock MACD() = exponential averages of the close at 15% and 7.5% (12/26 bars); the histogram = MACD() - Mov(MACD(),9,E), the averages seeded with the simple average of their first values. Stoch(5,3) = MetaStock's 5-day %K slowed over 3 days. The printed system options (Entry and Exit price = Open, Delay 0, Positions Both) mean: the conditions use yesterday's values (Ref(...,-1)) and fill at today's open. There are no close rules, so each opposite entry reverses the position; when neither condition holds the position is kept. Not coded: the 'KAMELEON' variant that switches on the system's own equity curve. Daily bars; indices, ETFs and US/India stocks are a declared application; no results printed.";
const NOTE_N: &str = "[Declared readings] Code read as printed: Mov(C-1,5,S) is the 5-day simple average of the close minus 1 price unit (Lindstedt: 'I have guessed what you mean by 1 day horizontal shift'); option: yesterday's 5-day average. Stoch(5,3) = MetaStock's 5-day %K slowed over 3 days (Andersen's 'double smoothing' is not coded; Lindstedt did not know it). The printed close rule ValueWhen(1,HIGH1,L) would always return today's low and never exit, so it is read as the request states it: exit when the close is below the low of the bar with the highest high since the latest entry signal (the code's HighestSince; a later bar with an equal high counts); option: since the actual entry, as Andersen worded it. Long only; signals and fills at the close (MetaStock defaults); a new entry signal while long only resets the reference bar. Lindstedt tested it on Swedish stocks and prints no figures; indices, ETFs and US/India stocks are a declared application.";

pub fn catalogue() -> Vec<StrategyDef> {
    let d = |id, name, dir, rules, notes| StrategyDef { id, name, group: "K.A.L.O.S.Z. MetaStock formula archive", session: 0, dir, status: Status::Live,
        needs_volume: false, needs_vix: false, needs_series: &[], book: "kzmsf", rules, notes, claim: None };
    vec![
        d("kzmsf_macd_hist", "MACDhistogram system (Lewandowski): MACD histogram sign with a Stoch(5,3) filter, reverse at the open", "Long/Short", &[
            "F|\"MACDhistogram: MACD()-Mov(MACD(),9,E)\"",
            "E|\"Enter Long: Ref(MACDhistogram,-1)>0 AND Ref(Stoch(5,3),-1)<80\"",
            "E|\"Enter Short: Ref(MACDhistogram,-1)<0 AND Ref(Stoch(5,3),-1)>20\"",
            "X|System options \"Entry and Exit price: Open; Delay 0; Positions: Both\": no close rules, the opposite entry reverses"], NOTE_M),
        d("kzmsf_no_name_1", "No Name 1 (Lindstedt for Andersen): close above yesterday's high, out below the low of the highest bar", "Long", &[
            "E|\"Enter Long: C> Ref(H,-1) AND C> Mov(C-1,5,S) AND Stoch(5,3)>Ref(Stoch(5,3),-1)\"",
            "X|\"Close Long: HIGH1:=HighestSince(1,C>Ref(H,-1) AND C> Mov(C-1,5,S) AND Stoch(5,3)>Ref(Stoch(5,3),-1),H); C<ValueWhen(1,HIGH1,L)\"",
            "X|Andersen's rule: \"To days close is lower than the low of the highest bar since enter long\""], NOTE_N),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    match id {
        "kzmsf_macd_hist" => Some(("kzmsf_macd_hist", "Lewandowski MACDhistogram", "MACD 12/26/9, Stoch(5,3) 80/20")),
        "kzmsf_no_name_1" => Some(("kzmsf_no_name_1", "Lindstedt No Name 1", "long only, close rules")),
        _ => None,
    }
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    match id {
        "kzmsf_macd_hist" => Some(vec![p("hi", "Stoch(5,3) long filter: below (code 80)", 80., 50., 100., 1.), p("lo", "Stoch(5,3) short filter: above (code 20)", 20., 0., 50., 1.)]),
        "kzmsf_no_name_1" => Some(vec![p("shift", "Average test: 0 = Mov(C-1,5,S) as coded, 1 = yesterday's 5-day average", 0., 0., 1., 1.),
            p("since_entry", "Highest bar since: 0 = the latest entry signal (code), 1 = the actual entry (request)", 0., 0., 1., 1.)]),
        _ => None,
    }
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.r.len() { return None; }
    let schema = params(id)?;
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).map_or(f64::NAN, |d| d.def));
    let (c, n) = (&x.c, x.c.len());
    let st = stoch(x, 5, 3);
    if id == "kzmsf_macd_hist" {
        let m = macd(c);
        let s = ema(&m, 9);
        let mut tgt = vec![0i8; n];
        for k in 0..n {
            let hst = m[k] - s[k];
            let prev = if k > 0 { tgt[k - 1] } else { 0 };
            tgt[k] = if hst > 0. && st[k] < g("hi") { 1 } else if hst < 0. && st[k] > g("lo") { -1 } else { prev };
        }
        return Some(target_trades(x, &tgt, from, to, true, "reverse"));
    }
    let (shift, since_entry) = (g("shift").round() >= 1., g("since_entry").round() >= 1.);
    let a5 = sma(c, 5);
    let sig = |i: usize| i > 0 && c[i] > x.h[i - 1] && (if shift { c[i] > a5[i - 1] } else { c[i] > a5[i] - 1. }) && st[i] > st[i - 1];
    let (mut out, mut cur, mut rb) = (vec![], None::<(usize, f64)>, 0usize);
    for i in from..=to {
        let e = sig(i);
        match cur {
            Some((ei, ep)) => {
                if e && !since_entry { rb = i; } else if x.h[i] >= x.h[rb] { rb = i; }
                if c[i] < x.l[rb] { out.push(mk(1., ei, ep, "close", i, c[i], "close below the low of the highest bar", false, i)); cur = None; }
            }
            None => if e { cur = Some((i, c[i])); rb = i; },
        }
    }
    if let Some((ei, ep)) = cur { out.push(mk(1., ei, ep, "close", to, c[to], "open", true, to)); }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;
    #[test]
    fn no_name_1_exits_below_the_low_of_the_highest_bar() {
        // (high, low, close): rising run, entry signal, a new high bar, then a close below that bar's low
        let mut b = vec![(10., 9., 9.5); 8];
        b.extend([(10., 9., 9.2), (11., 9.5, 10.8), (13., 11., 12.5), (12.8, 11.5, 12.), (12., 10., 10.5)]);
        let r: Vec<Row> = b.iter().enumerate().map(|(i, &(h, l, c))| Row { ticker: "T".into(), date: format!("2001-01-{:02}", i + 1), open: c, high: h, low: l, close: c, volume: None }).collect();
        let x = Ctx::new(&r, &r, &[]);
        let t = run("kzmsf_no_name_1", &x, 0, r.len() - 1, &Params::new()).unwrap();
        // bar 9: 10.8 > 10, > SMA5 9.7 - 1, Stoch 62.5 > 40 -> buy 10.8; bar 10 signals again and is the highest bar (low 11);
        // bar 12 closes 10.5 < 11 -> out
        assert_eq!(t.iter().map(|z| (z.legs[0].i, z.legs[0].px, z.exit_i, z.exit_px)).collect::<Vec<_>>(), vec![(9, 10.8, 12, 10.5)]);
        let m = run("kzmsf_macd_hist", &x, 0, r.len() - 1, &Params::new()).unwrap();
        assert!(m.is_empty(), "MACD needs 26 + 9 bars");
        assert!(run("kzmsf_x", &x, 0, 5, &Params::new()).is_none());
    }
}

// Instrument admission: K.A.L.O.S.Z. MetaStock archive (Lewandowski, Lindstedt) 2026-10-31 (titles lane K6).
// Admission only for the module REVIEWED lists (authors/titles-1030/K6w/k6_ref.py (header)).
// Runs on actual indices, ETFs and stocks, US and India (index_etf_or_stock, nifty50 basket, declared application).
pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book == "kzmsf" && crate::books::kzmsf::reviewed(def.id),
    label: "MetaStock formulas posted for any security (Lindstedt tested on Swedish stocks, no figures); indices, ETFs and US/India stocks are a declared application.",
    basis: "K.A.L.O.S.Z. Archive of MetaStock Formulas: 'MACDhistogram Trading System by Marek Lewandowski'; 'No Name 1' (Chris Lindstedt for Svein Andersen, 1998)",
};
