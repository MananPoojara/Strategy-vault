//! Spyros Raftopoulos, "The Zigzag Trend Indicator", Technical Analysis of Stocks & Commodities V.21:11 (Nov 2003) pp.26-32 (research share
//! Trading Knowledge I/Stocks & Commodities DVD 11.26 (1982 - 2008) (traders.com)/VOLUMES/V21/C11/233RAFT.pdf). Book id `scrzzt`.
//! Lane K2 (24 Nov 2026, Q396: an article the 30 Oct keyword scan had marked NO-RULES; the full read finds the indicator code and the trading rule).
//! Independent check authors/conv-1123/k2w/q396/scrzzt_ref.py (a literal port of the printed MetaStock code with a hindsight zigzag).
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};
use super::tselhs::Book;

pub const REVIEWED: &[&str] = &["scrzzt_close"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const NOTE: &str = "A|[Declared readings, lane K2, 4] (1) The article prints the ZZT code (1 = confirmed uptrend, -1 = confirmed downtrend, carried until the other confirms) and says it is 'tradable in and of itself'; the trade is read as: long when it turns 1, short when it turns -1, always in the market (stop and reverse), no stop printed. (2) Reversal amount 10 percent on the close = the Nasdaq example of Figure 2 (the Boeing and MACD comparison examples use 15 and 18 percent; options). (3) The code calls MetaStock's Zig (hindsight swing points); here the same confirmed state is computed causally from the swing extreme since the last confirmed turn (a turn is confirmed when the close retraces the reversal amount from that extreme), which is what the article describes ('the exact moment each zig and zag became valid') and avoids look-ahead (R4 #373); a first swing point at the very first bar is not a pivot (the code's zz1<zz2 test cannot see it), so the first signal then comes only at the next turn, as the article says ('first pair of swing points'). (4) Fills: the signal is known at the close, the order fills at the NEXT bar's close (R4 #374).";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef { id: "scrzzt_close", name: "Raftopoulos zigzag trend indicator (10% on the close): long on +1, short on -1, stop and reverse", group: "S&C: The Zigzag Trend Indicator (Raftopoulos, V.21:11)", session: 0, dir: "Long/Short", status: Status::Live,
        needs_volume: false, needs_vix: false, needs_series: &[], book: "scrzzt", rules: &[
            "F|ZZT code (sidebar, PDF p.3): PU = last trough + trough x amnt/100; PD = last peak - peak x amnt/100; res = 1 if price >= PU and zigzag rising, -1 if price <= PD and zigzag falling, else the previous non-zero value; percent method, close, amnt 10 (Figure 2: 10% zigzag trend on the Nasdaq)",
            "E|\"zigzag trend returns 1 for valid uptrends and -1 for valid downtrends\", all confirmed, not revisable, 'suitable' for backtests (Figure 1); buy when it rises to 1, sell when it drops to -1 [stop and reverse, reading]",
            "X|No separate exit: the opposite confirmed trend reverses the position (the article prints no stop or target) [reading]",
            NOTE], notes: NOTE, claim: None }]
}
pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> { reviewed(id).then_some(("scrzzt", "Raftopoulos zigzag trend (S&C)", "10% close")) }
pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    reviewed(id).then(|| vec![ParamDef { key: "amnt", label: "Reversal amount, % (Nasdaq example 10; Boeing 15; MACD comparison 18)", def: 10., min: 1., max: 40., step: 0.5 }])
}

/// Confirmed zigzag trend (+1 / -1 / 0 before the first confirmed pair of swing points), computed causally.
pub fn zzt(c: &[f64], amnt: f64) -> Vec<i8> {
    let a = amnt / 100.0;
    let mut out = vec![0i8; c.len()];
    if c.is_empty() { return out; }
    let (mut lo, mut hi, mut lo_i, mut hi_i) = (c[0], c[0], 0usize, 0usize);
    let (mut dir, mut cur) = (0i8, 0i8);
    for t in 0..c.len() {
        let v = c[t];
        match dir {
            0 => {
                if v < lo { lo = v; lo_i = t; }
                if v > hi { hi = v; hi_i = t; }
                // the first swing point counts only if it is an interior extreme (the code's zz1<zz2 test cannot see bar 0)
                if v >= lo * (1.0 + a) { dir = 1; hi = v; if lo_i > 0 { cur = 1; } } else if v <= hi * (1.0 - a) { dir = -1; lo = v; if hi_i > 0 { cur = -1; } }
            }
            1 => {
                hi = hi.max(v);
                if v <= hi * (1.0 - a) { dir = -1; lo = v; cur = -1; }
            }
            _ => {
                lo = lo.min(v);
                if v >= lo * (1.0 + a) { dir = 1; hi = v; cur = 1; }
            }
        }
        out[t] = cur;
    }
    out
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.r.len() { return None; }
    let z = zzt(&x.c[from..=to], p.get("amnt").copied().unwrap_or(10.));
    let mut pos = Book::new();
    let mut pend = 0.0;
    for t in from..=to {
        if pend != 0.0 { pos.exit(t, x.c[t], "reversed"); pos.enter(pend, t, x.c[t], "close"); pend = 0.0; }
        let s = z[t - from] as f64;
        if t < to && s != 0.0 && s != pos.side() { pend = s; }
    }
    Some(pos.done(to, x.c[to]))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn first_signal_waits_for_the_first_pair_of_swing_points() {
        // first bar is the trough of the first leg: no pivot there, so up 10% gives nothing; down 10% from the peak -> -1; up 10% from the trough -> +1
        let c = [100., 105., 111., 105., 99., 95., 98., 105., 106.];
        assert_eq!(zzt(&c, 10.), vec![0, 0, 0, 0, -1, -1, -1, 1, 1]);
        // an interior first trough is a pivot: the up turn signals at once
        let c = [100., 95., 99., 105.5, 106.];
        assert_eq!(zzt(&c, 10.), vec![0, 0, 0, 1, 1]);
    }
    #[test]
    fn reverses_at_the_next_close() {
        let r = super::super::sctraub::tests::bars(1500);
        let x = Ctx::new(&r, &r, &[]);
        let t = run("scrzzt_close", &x, 0, r.len() - 1, &Params::new()).unwrap();
        assert!(t.len() > 3);
        for z in &t { assert_eq!(z.legs[0].px, r[z.legs[0].i].close); }
        for w in t.windows(2) { assert_eq!(w[0].exit_i, w[1].legs[0].i); assert!(w[0].dir != w[1].dir); }
    }
    /// hand-run dump for the pandas reference: SCRZZT_CSV=<idx_data.csv> SCRZZT_OUT=<file> cargo test --release scrzzt_dump -- --ignored
    #[test]
    #[ignore]
    fn scrzzt_dump() {
        use std::io::Write;
        let data = crate::load_csv(&std::env::var("SCRZZT_CSV").unwrap()).unwrap();
        let mut f = std::io::BufWriter::new(std::fs::File::create(std::env::var("SCRZZT_OUT").unwrap()).unwrap());
        writeln!(f, "id,ticker,dir,entry_date,entry_px,exit_date,exit_px,open").unwrap();
        for (t, rows) in data.iter() {
            let x = Ctx::new(rows, &[], &[]);
            for z in run("scrzzt_close", &x, 0, rows.len() - 1, &Params::new()).unwrap() {
                writeln!(f, "scrzzt_close,{t},{},{},{},{},{},{}", z.dir, rows[z.legs[0].i].date, z.legs[0].px, rows[z.exit_i].date, z.exit_px, z.open).unwrap();
            }
        }
    }
}

// Instrument admission: S&C Raftopoulos zigzag trend 2026-11-24 (lane K2, Q396).
pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book == "scrzzt" && reviewed(def.id),
    label: "Raftopoulos shows the indicator on the Nasdaq index and Boeing; the article prints no trade results. Runs on indices, ETFs and stocks, US and India, are a declared application.",
    basis: "Technical Analysis of Stocks & Commodities V.21:11 pp.26-32 (The Zigzag Trend Indicator); independent check authors/conv-1123/k2w/q396/scrzzt_ref.py",
};
