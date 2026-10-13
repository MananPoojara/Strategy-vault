//! Bulli Castelo, "Aulas Swing Trading System - Como montar um sistema Expert Advisor no Metastock" (Portuguese,
//! 47 pages, Bovespa forum course). Book id "bcsts". Source copy: Books/Bulli Castelo/Bulli Castelo - Aulas Swing
//! Trading System.pdf. Refs = printed page numbers. Notes and the oracle: library/bcsts/.
//! Long-only MetaStock Expert Advisor buy/sell symbols built on a 3-day EMA of Stochastic RSI, latched with
//! ExtFml("ForumDll.Latch") so there is one sell for every buy. Lessons 2, 4 and 11 are complete systems.
#![allow(dead_code)]
use crate::connors::*;
use super::tilma::ema;

pub const BOOK: &str = "bcsts";

const K: &str = "F|K12 = Mov((RSI(12)-LLV(RSI(12),12)) / (HHV(RSI(12),12)-LLV(RSI(12),12)),3,E)*100 and K14 = the same with 14 (MetaStock; RSI = Wilder). Cross(a,b) = a > b today and a <= b yesterday";
const LATCH: &str = "Latch(LE,LX,0,0) [ForumDll, behaviour not printed]: flat -> long on a bar with LE; long -> flat on a bar with LX; one change per bar (no same-bar exit and re-entry). Buy symbol = \"B>0 AND Ref(B,-1)<=0\", sell symbol = \"B=0 AND Ref(B,-1)>0\". Fills: Expert Advisor symbols appear on the bar's close; the vault trades at that close [book silent]. HHV = LLV (zero range) gives no value that bar. Course instrument: Bovespa stocks (PETR4, GRND3, CMET4); India: instrument swap only. No published results (charts only).";
const A2: &[&str] = &[
    K,
    "E|\"LE:= Cross(K12,25)\" (Aula 2, p.6-7)",
    "X|\"LX:= Cross(20,K14)\" = K14 crosses below 20",
    "E|\"SE:=0; SX:=0; B:= ExtFml(\\\"ForumDll.Latch\\\",LE,LX,SE,SX); B>0 AND Ref(B,-1)<=0\" (buy) / \"B=0 AND Ref(B,-1)>0\" (sell)",
];
const A4: &[&str] = &[
    K,
    "E|\"LE:= Cross(K12,25) And C>(ref(mov(C,25,S), -5))\" (Aula 4, p.17) = close above the 25-day SMA of 5 bars ago",
    "X|\"LX:= Cross(20,K14)\"",
    "E|Latch as Aula 2",
];
const A11: &[&str] = &[
    K,
    "F|\"ExtFml(\\\"GV.SetVar\\\",\\\"LEntry\\\", ValueWhen(1, Cross(K12,25), C))\" (Aula 11, p.42-43) = close of the latest K12 cross above 25",
    "E|\"LE:= Cross(K12,25) AND C>(Ref(Mov(C,25,S),-5)) OR C> (ExtFml(\\\"GV.GetVar\\\",\\\"LEntry\\\")* 1.02)\" (MetaStock: AND binds before OR) = signal delay: also buy when the close is 2% above LEntry",
    "X|\"LX:= Cross(20,K14) OR C<(ExtFml(\\\"GV.GetVar\\\",\\\"LEntry\\\")*.97)\" = money-management sell 3% below LEntry",
    "E|Latch as Aula 2",
];

pub fn catalogue() -> Vec<StrategyDef> {
    use Status::*;
    let d = |id, name, group, rules| StrategyDef {
        id, name, group, session: 0, dir: "Long", status: Live, needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK, rules,
        notes: LATCH, claim: None,
    };
    vec![
        d("bcsts_a2", "Stochastic RSI swing (Aula 2: K12 crosses 25 / K14 crosses below 20)", "Aula 2 · latched swing", A2),
        d("bcsts_a4", "Stochastic RSI swing + SMA(25) shifted 5 filter (Aula 4)", "Aula 4 · trend filter", A4),
        d("bcsts_a11", "Stochastic RSI swing + 2% signal delay / 3% stop (Aula 11)", "Aula 11 · complete system", A11),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    let f = ("bcsts_swing", "Castelo Stochastic RSI swing (MetaStock lessons)");
    Some(match id { "bcsts_a2" => (f.0, f.1, "Aula 2"), "bcsts_a4" => (f.0, f.1, "Aula 4"), "bcsts_a11" => (f.0, f.1, "Aula 11"), _ => return None })
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    let mut v = vec![
        p("n_in", "Entry Stochastic RSI length", 12., 5., 30., 1.),
        p("lvl_in", "Buy when K crosses above", 25., 5., 50., 5.),
        p("n_out", "Exit Stochastic RSI length", 14., 5., 30., 1.),
        p("lvl_out", "Sell when K crosses below", 20., 5., 50., 5.),
    ];
    match id {
        "bcsts_a2" => {}
        "bcsts_a4" | "bcsts_a11" => {
            v.push(p("ma", "Filter SMA length", 25., 5., 100., 5.));
            v.push(p("shift", "Filter SMA shifted back (bars)", 5., 0., 20., 1.));
            if id == "bcsts_a11" {
                v.push(p("delay_pct", "Signal delay: buy when close is % above LEntry", 2., 0., 10., 0.5));
                v.push(p("stop_pct", "Sell when close is % below LEntry", 3., 1., 10., 0.5));
            }
        }
        _ => return None,
    }
    Some(v)
}

pub fn stoch_rsi_k(c: &[f64], n: usize) -> Vec<f64> {
    let r = rsi(c, n);
    let raw: Vec<f64> = (0..r.len()).map(|i| {
        if i + 1 < n { return f64::NAN; }
        let w = &r[i + 1 - n..=i];
        if w.iter().any(|x| x.is_nan()) { return f64::NAN; }
        let (lo, hi) = w.iter().fold((f64::INFINITY, f64::NEG_INFINITY), |(a, b), &x| (a.min(x), b.max(x)));
        if hi > lo { (r[i] - lo) / (hi - lo) } else { f64::NAN }
    }).collect();
    ema(&raw, 3).iter().map(|v| v * 100.).collect()
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    let schema = params(id)?;
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).map_or(f64::NAN, |d| d.def));
    let c = &x.c;
    let (ki, ko) = (stoch_rsi_k(c, g("n_in") as usize), stoch_rsi_k(c, g("n_out") as usize));
    let (li, lo) = (g("lvl_in"), g("lvl_out"));
    let fin = |v: f64| v.is_finite();
    let cross_up = |i: usize| i >= 1 && fin(ki[i]) && fin(ki[i - 1]) && ki[i] > li && ki[i - 1] <= li;
    let cross_dn = |i: usize| i >= 1 && fin(ko[i]) && fin(ko[i - 1]) && lo > ko[i] && lo <= ko[i - 1];
    let filt: Option<(Vec<f64>, usize)> = (id != "bcsts_a2").then(|| (sma(c, g("ma") as usize), g("shift") as usize));
    let filter_ok = |i: usize| filt.as_ref().map_or(true, |(m, s)| i >= *s && fin(m[i - s]) && c[i] > m[i - s]);
    // LEntry = ValueWhen(1, Cross(K12,25), C)
    let mut lentry = vec![f64::NAN; c.len()];
    for i in 0..c.len() { lentry[i] = if cross_up(i) { c[i] } else if i > 0 { lentry[i - 1] } else { f64::NAN }; }
    let a11 = id == "bcsts_a11";
    let (dly, stp) = if a11 { (g("delay_pct") / 100., g("stop_pct") / 100.) } else { (0., 0.) };
    let le = |i: usize| (cross_up(i) && filter_ok(i)) || (a11 && fin(lentry[i]) && c[i] > lentry[i] * (1. + dly));
    let lx = |i: usize| cross_dn(i) || (a11 && fin(lentry[i]) && c[i] < lentry[i] * (1. - stp));
    let to = to.min(c.len() - 1);
    let (mut out, mut cur): (Vec<Trade>, Option<Trade>) = (vec![], None);
    for i in from..=to {
        if cur.is_some() {
            if lx(i) {
                let mut t = cur.take().unwrap();
                (t.exit_i, t.exit_px, t.signal_i, t.reason) = (i, c[i], i, "latch LX (sell symbol)".into());
                out.push(t);
            }
            continue;
        }
        if le(i) {
            cur = Some(Trade { dir: 1., legs: vec![Leg { i, px: c[i], w: 1. }], exit_i: i, exit_px: f64::NAN, reason: String::new(),
                entry_kind: "close", signal_i: i, open: false, exits: vec![] });
        }
    }
    if let Some(mut t) = cur {
        (t.exit_i, t.exit_px, t.reason, t.signal_i, t.open) = (to, c[to], "open".into(), to, true);
        out.push(t);
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn stoch_rsi_bounds_and_latch() {
        let c: Vec<f64> = (0..300).map(|i| 100. + 10. * ((i as f64) / 7.).sin() + i as f64 * 0.05).collect();
        let k = stoch_rsi_k(&c, 12);
        assert!(k.iter().filter(|v| v.is_finite()).all(|v| *v >= -1e-9 && *v <= 100. + 1e-9));
        let rows: Vec<crate::Row> = c.iter().enumerate().map(|(i, &v)| crate::Row { ticker: "T".into(), date: format!("d{:04}", i), open: v, high: v, low: v, close: v, volume: None }).collect();
        let x = Ctx::new(&rows, &rows, &[]);
        let t = run("bcsts_a2", &x, 0, rows.len() - 1, &Params::new()).unwrap();
        assert!(t.len() > 3);
        assert!(t.windows(2).all(|w| w[1].legs[0].i > w[0].exit_i));
    }
}
