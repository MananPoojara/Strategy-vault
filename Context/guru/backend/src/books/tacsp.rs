//! John Summa, "Trading Against the Crowd" (Wiley 2004), Squeeze Play I with the price trigger (Table 8.1 p.64; MetaStock code pp.179, 184-185; VIX tests
//! pp.117-120; oscillator App. A p.182, App. B p.183; research share USB HDD 2/tRADING KNO-2/John F.Summa - Trading Against the Crowd). Book id `tacsp`.
//! Lane K2 (24 Nov 2026, tier 5 row I462). Independent check authors/conv-1123/k2w/qi/tacsp_ref.py.
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};
use super::tselhs::Book;

pub const REVIEWED: &[&str] = &["tacsp_squeeze1"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const NOTE: &str = "A|[Declared readings, lane K2, 4] (1) The book runs the sentiment series (the VIX) against the S&P 500; here India VIX against NIFTY 50. (2) The price trigger must fire on the day of the oscillator cross (the text does not say whether a later day also counts). (3) The EMAs start from the first VIX close and signals begin after 50 VIX closes (the seeding is not printed). (4) The short side is the mirror of the long (the printed MetaStock close-short line is garbled, p.179); the oscillator is 100 x (EMA a - EMA b) / EMA b of the VIX close, known at the close, and orders fill at the next open.";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef { id: "tacsp_squeeze1", name: "Summa Squeeze Play I on India VIX: EMA5-21 oscillator cross plus price trigger, exit on EMA21-50 cross", group: "Summa: Trading Against the Crowd (Squeeze Play)", session: 0, dir: "Long/Short", status: Status::Live,
        needs_volume: false, needs_vix: true, needs_series: &[], book: "tacsp", rules: &[
            "F|Oscillator EMAa-b = 100 x (EMA(a) - EMA(b)) / EMA(b) of the sentiment series (the VIX close), App. A p.182, App. B p.183",
            "E|Long when EMA5-21 of the VIX crosses from above zero to below zero on a close and the instrument closes above the prior day's high; short when it crosses from below to above zero and the close is below the prior day's low; buy / sell short at the next open (Table 8.1 p.64, code pp.179, 184-185)",
            "X|Exit at the next open when EMA21-50 of the VIX crosses zero against the position (long: upward, short: downward); an opposite entry reverses; no stops (pp.117-120)",
            NOTE], notes: NOTE, claim: None }]
}
pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> { reviewed(id).then_some(("tacsp", "Summa Squeeze Play (VIX)", "EMA5-21 cross + price trigger")) }
pub fn params(_id: &str) -> Option<Vec<ParamDef>> { None }

fn ema_ff(v: &[f64], n: usize) -> Vec<f64> {
    let a = 2. / (n as f64 + 1.);
    let mut out = vec![f64::NAN; v.len()];
    let mut e = f64::NAN;
    for i in 0..v.len() {
        if v[i].is_finite() { e = if e.is_finite() { e + a * (v[i] - e) } else { v[i] }; }
        out[i] = e;
    }
    out
}
fn osc(v: &[f64], a: usize, b: usize) -> Vec<f64> {
    let (ea, eb) = (ema_ff(v, a), ema_ff(v, b));
    ea.iter().zip(&eb).map(|(x, y)| 100. * (x - y) / y).collect()
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, _p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.r.len() { return None; }
    let (o1, o2) = (osc(&x.vix.close, 5, 21), osc(&x.vix.close, 21, 50));
    let mut seen = 0usize;
    let mut pos = Book::new();
    let mut pend = 0.0;
    for t in 0..=to {
        if x.vix.close[t].is_finite() { seen += 1; }
        if t < from { continue; }
        if pend != 0.0 {
            if pend == 2. { pos.exit(t, x.o[t], "EMA21-50 cross"); } else { pos.enter(pend, t, x.o[t], "open"); }
            pend = 0.0;
        }
        if t >= to || t == 0 || seen < 50 || !o1[t].is_finite() || !o1[t - 1].is_finite() || !o2[t - 1].is_finite() { continue; }
        if o1[t - 1] > 0. && o1[t] <= 0. && x.c[t] > x.h[t - 1] { pend = 1.; }
        else if o1[t - 1] < 0. && o1[t] >= 0. && x.c[t] < x.l[t - 1] { pend = -1.; }
        else if (pos.side() > 0. && o2[t - 1] <= 0. && o2[t] > 0.) || (pos.side() < 0. && o2[t - 1] >= 0. && o2[t] < 0.) { pend = 2.; }
    }
    Some(pos.done(to, x.c[to]))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn oscillator_is_percent_gap_of_two_emas() {
        let v = [10., 12., 11., 13.];
        let o = osc(&v, 2, 3);
        let (a2, a3) = (2. / 3., 0.5);
        let (mut e2, mut e3) = (10., 10.);
        for x in &v[1..] { e2 += a2 * (x - e2); e3 += a3 * (x - e3); }
        assert!((o[3] - 100. * (e2 - e3) / e3).abs() < 1e-9);
    }
    /// hand-run dump for the pandas reference: TACSP_CSV=<idx_data.csv> TACSP_VIX=<india_vix dir> TACSP_OUT=<file> cargo test --release tacsp_dump -- --ignored
    #[test]
    #[ignore]
    fn tacsp_dump() {
        use std::io::Write;
        let data = crate::load_csv(&std::env::var("TACSP_CSV").unwrap()).unwrap();
        let vix = crate::load_vix_dir(&std::env::var("TACSP_VIX").unwrap());
        let mut f = std::io::BufWriter::new(std::fs::File::create(std::env::var("TACSP_OUT").unwrap()).unwrap());
        writeln!(f, "id,ticker,dir,entry_date,entry_px,exit_date,exit_px,open").unwrap();
        for (t, rows) in data.iter() {
            if t != "NIFTY 50" { continue; }
            let x = Ctx::new(rows, rows, &vix);
            for z in run("tacsp_squeeze1", &x, 0, rows.len() - 1, &Params::new()).unwrap() {
                writeln!(f, "tacsp_squeeze1,{t},{},{},{},{},{},{}", z.dir, rows[z.legs[0].i].date, z.legs[0].px, rows[z.exit_i].date, z.exit_px, z.open).unwrap();
            }
        }
    }
}

// Instrument admission: Summa Squeeze Play I 2026-11-24 (lane K2, tier 5).
pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book == "tacsp" && reviewed(def.id),
    label: "Summa runs the VIX against the S&P 500 (Table 8.1 prints no figure comparable here); run on NIFTY 50 with India VIX (from 2009-03-02) as a labelled adaptation; the gate row allows NIFTY 50 only (R4 #378).",
    basis: "Summa, Trading Against the Crowd (2004) Table 8.1 p.64, code pp.179, 184-185; independent check authors/conv-1123/k2w/qi/tacsp_ref.py",
};
