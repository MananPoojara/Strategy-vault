//! "ATHC" AmiBroker formulas (author not named): research share USB HDD 2/US market database/ATHC FOR US MARKET/ (5 .afl
//! variants with the same signals; only sizing and commission differ; read in full: "ATHC SL ST ALL VARIATION ENTRY CL EXIT CL
//! for optimize with pct bkg.afl"). Book id `abathc`. Uses the SuperTrend loop of books/abstus.rs (the same AFL loop).
//! Lane K14 (titles loop, 2 Nov 2026), to-code row 232, lead decision 82; independent check authors/titles-1030/K14w/k14_ref.py.
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};
use super::abstus::supertrend;
use super::tselhs::Book;

/// Admitted ids (authors/titles-1030/K14w/k14_ref.py (header)).
pub const REVIEWED: &[&str] = &["abathc_ath_st"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const NOTE: &str = "[Declared readings] The AFL as written. Hcl = HHV(C, BarCount): the highest close from the first bar of the data; Buy = C > Ref(Hcl,-1), a close above every earlier close ('all-time' = the data the vault holds). SuperTrend on the close (the ParamList default 'CLOSE'): satr = the simple average of ATR(1) (the true range; the first bar's is High - Low) over 'ST Period' 10 bars; Up = C + 1 x satr (the code's fixed 1), Dn = C - Factor x satr; the same trend loop as abstus_supertrend (trend +1 at the start; +1 on a close above the previous Up, -1 below the previous Dn; Dn only rises in an uptrend, Up only falls in a downtrend, the band reset on the bar the trend turns). Sell = trend -1. Buy = ExRem(Buy, Sell), Sell = ExRem(Sell, Buy) (ExRem: the first signal is kept, later ones are dropped until the other array fires on a later bar), started on the run's first bar. Long only; at each close the exit is taken first, then a flat book buys; entry and exit at that close ('ENTRY/EXIT AT CLOSING SAME DAY'). Factor: the code's Optimize default 10 is the default (lead decision 82); the file comment's 'ST 10/4' is option p.fac = 4. The backtest settings (equity, 100,000 per position, 20 positions, 0.1% commission) are not part of the rule; the vault's standard costs. Anonymous formula file with no test or result, written for stocks; indices and ETFs, US and India, are a declared application.";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef { id: "abathc_ath_st", name: "All-time-high close: buy a close above every earlier close, sell at the close when the SuperTrend (10, factor 10) of the close turns down", group: "ATHC all-time-high close (AmiBroker formula)", session: 0, dir: "Long", status: Status::Live,
        needs_volume: false, needs_vix: false, needs_series: &[], book: "abathc", rules: &[
            "E|\"// BUY WHEN STOCK CLOSE ABOVE ALL TIME HIGH CLOSE\" \"Hcl=HHV(C,BarCount);\" \"Buy= C > Ref(Hcl,-1) AND BarIndex()>=TSTART;\" (TSTART 0) \"// ENTRY/EXIT AT CLOSING SAME DAY\"",
            "F|\"BPd1=optimize(\\\"ST Period \\\",10,5,20,1); BFactor1=Optimize(\\\"ST Factor\\\",10,1,12,1);\" \"Up=stcal+(1*satr); Dn=stcal-(V2*satr);\" stcal = C (ParamList default \"CLOSE\"); comment: \"// STOPLOSS SUPER TREND 10/4 FROM CLOSE\"",
            "X|\"Sell=Btrend1==-1; Buy=ExRem(Buy,Sell); Sell=ExRem(Sell,Buy);\" SellPrice = C"],
        notes: NOTE, claim: None }]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    (id == "abathc_ath_st").then_some(("abathc_ath_st", "AmiBroker all-time-high close", "SuperTrend exit 10/10"))
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    (id == "abathc_ath_st").then(|| vec![p("pd", "ST Period (10 in the code)", 10., 5., 20., 1.), p("fac", "ST Factor (10 = the code's default; 4 = the file comment's 'ST 10/4')", 10., 1., 12., 1.)])
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.r.len() { return None; }
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| params(id).unwrap().iter().find(|d| d.key == k).unwrap().def);
    let c = &x.c;
    let tr = supertrend(x, c, g("pd").round().max(1.) as usize, 1., g("fac"));
    let mut best = f64::NEG_INFINITY; // highest close before bar t
    for v in &c[..from] { best = best.max(*v); }
    let mut pos = Book::new();
    let (mut bf, mut sf) = (false, false); // ExRem states of Buy and Sell
    for t in from..=to {
        let braw = t >= 1 && c[t] > best;
        best = best.max(c[t]);
        let sraw = tr[t] < 0;
        let buy = if !bf && braw { bf = true; true } else { if bf && sraw { bf = false; } false };
        let sell = if !sf && sraw { sf = true; true } else { if sf && buy { sf = false; } false };
        if pos.side() > 0. && sell { pos.exit(t, c[t], "SuperTrend down"); }
        if pos.side() == 0. && buy { pos.enter(1., t, c[t], "close above the all-time high close"); }
    }
    Some(pos.done(to, c[to]))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;
    #[test]
    fn ath_close_with_supertrend_exit() {
        let r: Vec<Row> = (0..1500).map(|i| { let f = i as f64; let c = 100. + 20. * (f / 50.).sin() + 0.05 * f;
            Row { ticker: "T".into(), date: format!("{:05}", i), open: c, high: c + 0.5, low: c - 0.5, close: c, volume: None } }).collect();
        let x = Ctx::new(&r, &r, &[]);
        let mut p = Params::new(); p.insert("fac".into(), 4.);
        let t = run("abathc_ath_st", &x, 0, r.len() - 1, &p).unwrap();
        assert!(t.len() > 3 && t.iter().all(|z| z.dir > 0.));
        for z in &t { let e = z.legs[0].i; assert!(r[..e].iter().all(|q| q.close < r[e].close)); }
        assert!(family_of("abathc_ath_st").is_some() && run("abathc_x", &x, 0, 9, &Params::new()).is_none());
    }
}

// Instrument admission: ATHC (AmiBroker) 2026-11-02 (titles lane K14).
pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book == "abathc" && reviewed(def.id),
    label: "Anonymous AmiBroker stock formula with no test or result. Runs on stocks, indices and ETFs, US and India, are a declared application.",
    basis: "ATHC FOR US MARKET/*.afl (USB HDD 2/US market database), read in full; independent check authors/titles-1030/K14w/k14_ref.py",
};
