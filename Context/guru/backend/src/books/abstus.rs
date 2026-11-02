//! "SUPERTREND ALL FOR US MARKET" AmiBroker formula (author not named), research share root: ST ALL FOR US MARKET.afl (3.9 KB,
//! read in full; reader note authors/batch-i/I2w/supertrend_us.txt). Book id `abstus`. The SuperTrend loop below is also used
//! by the ATHC all-time-high-close system (books/abathc.rs), whose AFL carries the same loop. Lane K14 (titles loop,
//! 2 Nov 2026), to-code row 234; independent check authors/titles-1030/K14w/k14_ref.py.
use crate::connors::{true_range, Ctx, ParamDef, Params, Status, StrategyDef, Trade};
use super::tselhs::Book;

/// Admitted ids (authors/titles-1030/K14w/k14_ref.py (header)).
pub const REVIEWED: &[&str] = &["abstus_supertrend"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const NOTE: &str = "[Declared readings] The AFL as written: satr = the simple average of ATR(1) (the true range; the first bar's is High - Low) over Pd1 = 5 bars; Up = (H+L)/2 + Factor1 x satr, Dn = (H+L)/2 - Factor1 x satr, Factor1 = 2 (the Optimize lines are commented out). Loop from the second bar, trend starting at +1: trend = +1 if Close > Up[i-1], -1 if Close < Dn[i-1], else unchanged (no band yet: unchanged). Then while the trend is up Dn may only rise (Dn[i] = Dn[i-1] if lower), while down Up may only fall; on the bar the trend turns down Up is recomputed from that bar, on the bar it turns up Dn is. Buy = Cover = trend up, Sell = Short = trend down, each pair through ExRem: always in the market, long while the trend is up, short while it is down. Fills at the signal bar's close (AmiBroker's default BuyPrice = Close; option p.open = 1: the next open). A run starts on its first bar in the side of the trend there (trend state from the first bar of the data). The file's backtest settings (equity 1,000,000, 20 positions, 0.1% commission) are not part of the rule; the vault's standard costs. Anonymous formula file with no test or result; runs on indices, ETFs and US/India stocks are a declared application.";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef { id: "abstus_supertrend", name: "SuperTrend (5, 2) of the midpoint, always in: long while the trend is up, short while it is down, at the close", group: "SuperTrend for US market (AmiBroker formula)", session: 0, dir: "Long/Short", status: Status::Live,
        needs_volume: false, needs_vix: false, needs_series: &[], book: "abstus", rules: &[
            "F|\"satr=Ref(ATR(1),0); for (i=1 ; i<pd1;i++) { satr=satr + Ref(ATR(1),-i); } satr=satr/pd1; Up=(H+L)/2+(Factor1*satr); Dn=(H+L)/2-(Factor1*satr);\" Pd1=param(\"ATR Periods1 \",5,...); Factor1=param(\"Factor1\",2,...)",
            "F|\"if (Close[i]>Up[i-1]) trend1[i]=1; else if (Close[i]<Dn[i-1]) trend1[i]=-1; ... if (trend1[i]>0 && Dn[i]<Dn[i-1]) Dn[i]=Dn[i-1]; if (trend1[i]<0 && Up[i]>Up[i-1]) Up[i]=Up[i-1]; if (flag1==1) Up[i]=(H[i]+L[i])/2+(Factor1*iATR[i]); if (flagh1==1) Dn[i]=(H[i]+L[i])/2-(Factor1*iATR[i]);\"",
            "E|\"Buy = trend1==1; Short= trend1==-1;\" with ExRem; BuyPrice = ShortPrice = C",
            "X|\"Sell= trend1==-1; Cover= trend1==1;\" with ExRem; SellPrice = CoverPrice = C"],
        notes: NOTE, claim: None }]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    (id == "abstus_supertrend").then_some(("abstus_supertrend", "AmiBroker SuperTrend, always in", "SuperTrend 5 x 2"))
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    (id == "abstus_supertrend").then(|| vec![p("pd", "ATR periods Pd1 (5 in the file)", 5., 3., 20., 1.), p("fac", "Factor1 (2 in the file)", 2., 1., 12., 1.),
        p("open", "Fill: 0 = the signal bar's close (AmiBroker default); 1 = the next open", 0., 0., 1., 1.)])
}

/// The AFL SuperTrend loop. `src` = the band centre (C or (H+L)/2), `pd` = bars averaged, `fu` / `fd` = multiples for the
/// upper / lower band. Returns the trend (+1 / -1) for every bar.
pub fn supertrend(x: &Ctx, src: &[f64], pd: usize, fu: f64, fd: f64) -> Vec<i8> {
    let n = src.len();
    let tr = true_range(x.r);
    let satr: Vec<f64> = (0..n).map(|i| if i + 1 < pd { f64::NAN } else { tr[i + 1 - pd..=i].iter().sum::<f64>() / pd as f64 }).collect();
    let mut up: Vec<f64> = (0..n).map(|i| src[i] + fu * satr[i]).collect();
    let mut dn: Vec<f64> = (0..n).map(|i| src[i] - fd * satr[i]).collect();
    let mut tr_ = vec![1i8; n];
    for i in 1..n {
        tr_[i] = if x.c[i] > up[i - 1] { 1 } else if x.c[i] < dn[i - 1] { -1 } else { tr_[i - 1] };
        if tr_[i] > 0 && dn[i] < dn[i - 1] { dn[i] = dn[i - 1]; }
        if tr_[i] < 0 && up[i] > up[i - 1] { up[i] = up[i - 1]; }
        if tr_[i] < 0 && tr_[i - 1] > 0 { up[i] = src[i] + fu * satr[i]; }
        if tr_[i] > 0 && tr_[i - 1] < 0 { dn[i] = src[i] - fd * satr[i]; }
    }
    tr_
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.r.len() { return None; }
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| params(id).unwrap().iter().find(|d| d.key == k).unwrap().def);
    let mid: Vec<f64> = x.h.iter().zip(&x.l).map(|(h, l)| (h + l) / 2.).collect();
    let fac = g("fac");
    let tr = supertrend(x, &mid, g("pd").round().max(1.) as usize, fac, fac);
    let open = g("open").round() == 1.;
    let mut pos = Book::new();
    for t in from..=to {
        let d = tr[t] as f64;
        if pos.side() == d { continue; }
        let why = if d > 0. { "trend up" } else { "trend down" };
        if !open { pos.enter(d, t, x.c[t], why); }
        else if t < to { pos.enter(d, t + 1, x.o[t + 1], why); }
    }
    Some(pos.done(to, x.c[to]))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;
    #[test]
    fn supertrend_always_in() {
        let r: Vec<Row> = (0..1200).map(|i| { let f = i as f64; let c = 100. + 15. * (f / 40.).sin() + 6. * (f / 5.).sin() + 0.02 * f;
            Row { ticker: "T".into(), date: format!("{:05}", i), open: c - 0.3, high: c + 1., low: c - 1., close: c, volume: None } }).collect();
        let x = Ctx::new(&r, &r, &[]);
        let t = run("abstus_supertrend", &x, 0, r.len() - 1, &Params::new()).unwrap();
        assert!(t.len() > 10);
        for w in t.windows(2) { assert_eq!(w[1].legs[0].i, w[0].exit_i); assert_eq!(w[1].dir, -w[0].dir); }
        let mut o = Params::new(); o.insert("open".into(), 1.);
        let t2 = run("abstus_supertrend", &x, 0, r.len() - 1, &o).unwrap();
        assert!(t2.iter().zip(&t).all(|(a, b)| a.legs[0].i == b.legs[0].i + 1 || b.legs[0].i == 0));
        assert!(family_of("abstus_supertrend").is_some() && run("abstus_x", &x, 0, 9, &Params::new()).is_none());
    }
}

// Instrument admission: SuperTrend for US market (AmiBroker) 2026-11-02 (titles lane K14).
pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book == "abstus" && reviewed(def.id),
    label: "Anonymous AmiBroker formula with no test or result. Runs on daily indices, ETFs and stocks, US and India, are a declared application.",
    basis: "ST ALL FOR US MARKET.afl (research share root), read in full; independent check authors/titles-1030/K14w/k14_ref.py",
};
