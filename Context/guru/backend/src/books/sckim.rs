//! Jongseon Kim, "Trading In Tempo With TRIX", Technical Analysis of Stocks & Commodities V.22:1 (January 2004) pp.45-49 (research
//! share Trading Knowledge I/Stocks & Commodities DVD 11.26 (1982 - 2008) (traders.com)/VOLUMES/V22/C01/010KIM.pdf). Book id
//! `sckim`: TRIX(5,3) entry/exit/rest rules on daily, weekly and monthly bars. Lane K13 (titles loop, 2 Nov 2026), to-code row
//! 208, lead decision 76; independent check authors/titles-1030/K13w/k13_ref.py.
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};
use super::sctraub::{ema0, period_ends};
use super::tselhs::Book;

pub const REVIEWED: &[&str] = &["sckim_trix_d", "sckim_trix_w", "sckim_trix_m"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const NOTE: &str = "[Declared readings] TRIX(5,3): EMA1..EMA3 = 5-period EMAs of the close, each seeded with its first value; TRIX = (EMA3 - EMA3[1]) / EMA3[1]; signal = 3-period EMA of TRIX. Golden cross = TRIX above the signal now and at or below it on the last bar; 'TRIX fall' = TRIX below the last bar after rising (or flat) the bar before. Enter at the golden cross, exit at the fall, then rest until the next golden cross. Fills at the bar's close (the article's 'approximate value' is (open + close) / 2 of the bar and 'entry and exit times can have some tolerance'). Weekly and monthly bars end on each period's last trading day. Long only, as the article. Article (Nasdaq Composite): monthly 1995-2003 10 trades, 9 winners; weekly 1/2001-6/2003 8 trades, 5 winners; daily 7/2002-1/2003 14 trades, 7 winners. The Nasdaq Composite is not held: NDX/QQQ and the usual instruments, US and India, are declared applications (decision 76). Other vault TRIX cards use other rules and periods.";

pub fn catalogue() -> Vec<StrategyDef> {
    let d = |id, name| StrategyDef { id, name, group: "S&C: Trading In Tempo With TRIX (Kim, V.22:1)", session: 0, dir: "Long", status: Status::Live,
        needs_volume: false, needs_vix: false, needs_series: &[], book: "sckim", rules: &[
            "F|\"I recommend using five. 2 EMA1: Calculate the n-period EMA of the close. 3 EMA2: ... of EMA1. 4 EMA3: ... of EMA2. TRIX = (EMA3[today] - EMA3 [yesterday]) / EMA3[yesterday]\"; the signal line \"is the n-period EMA of the TRIX ... I often use the value 3\" (V.22:1 p.45)",
            "E|\"Enter: When a TRIX golden cross appears\" (TRIX breaks through the signal line upward) (p.45)",
            "X|\"Exit: When a TRIX fall appears. Rest: From a TRIX fall to the next TRIX golden cross.\" TRIX fall: \"TRIX converts the direction from upward to downward\" (p.45)"],
        notes: NOTE, claim: None };
    vec![d("sckim_trix_d", "Kim TRIX(5,3) tempo, daily bars: in at the golden cross, out when TRIX turns down"),
         d("sckim_trix_w", "Kim TRIX(5,3) tempo, weekly bars: in at the golden cross, out when TRIX turns down"),
         d("sckim_trix_m", "Kim TRIX(5,3) tempo, monthly bars: in at the golden cross, out when TRIX turns down")]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    let v = match id { "sckim_trix_d" => "Daily", "sckim_trix_w" => "Weekly", "sckim_trix_m" => "Monthly", _ => return None };
    Some(("sckim_trix", "Kim TRIX tempo (S&C)", v))
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> { reviewed(id).then(Vec::new) }

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, _p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.r.len() { return None; }
    let ends: Vec<usize> = match id { "sckim_trix_d" => (0..x.c.len()).collect(), "sckim_trix_w" => period_ends(x, false), _ => period_ends(x, true) };
    let p: Vec<f64> = ends.iter().map(|&i| x.c[i]).collect();
    let e3 = ema0(&ema0(&ema0(&p, 5), 5), 5);
    let t: Vec<f64> = (0..p.len()).map(|k| if k >= 1 { e3[k] / e3[k - 1] - 1.0 } else { f64::NAN }).collect();
    let s = ema0(&t, 3);
    let mut pos = Book::new();
    for (k, &i) in ends.iter().enumerate() {
        if i < from || i > to || k < 3 { continue; }
        if pos.side() > 0.0 { if t[k] < t[k - 1] && t[k - 1] >= t[k - 2] { pos.exit(i, x.c[i], "TRIX fall"); } }
        else if t[k] > s[k] && t[k - 1] <= s[k - 1] { pos.enter(1.0, i, x.c[i], "close"); }
    }
    Some(pos.done(to, x.c[to]))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn trix_tempo_three_timeframes() {
        let r = super::super::sctraub::tests::bars(4000);
        let x = Ctx::new(&r, &r, &[]);
        for id in REVIEWED { let t = run(id, &x, 0, r.len() - 1, &Params::new()).unwrap(); assert!(!t.is_empty() && t.iter().all(|z| z.dir > 0.0), "{id}"); }
    }
}

// Instrument admission: S&C Kim TRIX 2026-11-02 (titles lane K13).
pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book == "sckim" && reviewed(def.id),
    label: "Jongseon Kim showed the rules on the Nasdaq Composite (not held). Runs on NDX/QQQ and other indices, ETFs and stocks, US and India, are a declared application.",
    basis: "Technical Analysis of Stocks & Commodities V.22:1 pp.45-49 (Trading In Tempo With TRIX); independent check authors/titles-1030/K13w/k13_ref.py",
};
