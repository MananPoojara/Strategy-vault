//! Peter B. Waite, Pegasus (pegasusfutures.com), the open TradeStation code (research share USB HDD 1/TradeStation Systems &
//! Indicators/Pegasus System for TS + Manual (pegasusfutures.com)/Pegasus_Open Code.rar, read-only copy in
//! authors/batch-c/C7w/pegasus/Pegasus_Open Code.txt; fills from About Pegasus.doc in the same folder). Book id `pwpeg`, next to
//! his Abalone (`pwabal`). Lane K9 (titles loop, 1 Nov 2026), to-code row 160; independent check authors/titles-1030/K9w/k9_ref.py.
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};
use super::msws::sma;
use super::rbtie::mk;

/// Admitted ids (authors/titles-1030/K9w/k9_ref.py (header)).
pub const REVIEWED: &[&str] = &["pwpeg_pegasus"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const NOTE: &str = "[Risk cap held] The code's Initial_Risk_Limit(3000) filter (entry only while (Close - 100-day average) x BigPointValue < $3,000 per futures contract) is a contract-dollar amount; it is not applied and not converted (lead decision 41). [Declared readings] Signals on the close, all orders at the next day's open ('All orders, both entry and exit orders are executed on the open of the next day', About Pegasus.doc). Entries only when flat at the signal close (MarketPosition = 0), so a new entry can be signalled on the close of the day whose open closed the previous trade (as the Abalone card). Long: the 25- and 50-day simple averages both rising two days in a row and the close above the 100-day average plus one standard deviation of the last 100 closes; short mirrored. Exits from the fill day's close on, any of: close below the 100-day average ('LX Stop'), the 25-day average falling two days ('LX MA'), or a true range above twice the 10-day average true range on a down close ('LX ATR'); shorts mirrored. StdDev = TradeStation's population standard deviation (divide by 100); AvgTrueRange(10) = simple average of the last 10 true ranges, today included; TrueRange = max(high, yesterday's close) - min(low, yesterday's close). The locked PEGASUS.ELS is not used. The vendor trades futures (currencies, rates, energies); his printed results are futures portfolios, not comparable; indices, ETFs and US/India stocks are a declared application; index shorts are hypothetical.";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef { id: "pwpeg_pegasus", name: "Pegasus: rising 25/50-day averages with a close beyond the 100-day 1-SD band; out on the 100-day line, the 25-day turn or a 2 x ATR bar (risk cap held)",
        group: "Pegasus open code", session: 0, dir: "Long/Short", status: Status::Live, needs_volume: false, needs_vix: false, needs_series: &[], book: "pwpeg", rules: &[
            "F|\"Value1 = Average(Close,100); ... Upper_Band = Value1 + stddev(Close,100); Lower_Band = Value1 - stddev(Close,100); Large_ATR = 2*(AvgTrueRange(10));\"",
            "E|\"If (MarketPosition = 0) and Average(Close,25) > Average(Close,25)[1] and Average(Close,50) > Average(Close,50)[1] and Average(Close,25)[1] > Average(Close,25)[2] and Average(Close,50)[1] > Average(Close,50)[2] and Close > Upper_Band and Value2 < Initial_Risk_Limit then begin Buy (\"LE\") at Market;\"; Short Entry: the mirror with Lower_Band",
            "X|\"If (MarketPosition = +1) and Close < Value1 then ExitLong (\"LX Stop\") at Market;\" ... \"Average(Close,25) < Average(Close,25)[1] and Average(Close,25)[1] < Average(Close,25)[2] then ExitLong (\"LX MA\")\" ... \"TrueRange > Large_ATR and Close < Close[1] then ExitLong (\"LX ATR\")\"; short exits mirrored"],
        notes: NOTE, claim: None }]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    (id == "pwpeg_pegasus").then_some(("pwpeg_pegasus", "Pegasus (Waite)", "open code, risk cap held"))
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> { reviewed(id).then(Vec::new) }

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, _p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.r.len() { return None; }
    let (o, h, l, c) = (&x.o, &x.h, &x.l, &x.c);
    let n = c.len();
    let (m25, m50, m100) = (sma(c, 25), sma(c, 50), sma(c, 100));
    let sd: Vec<f64> = (0..n).map(|i| if i + 1 < 100 { f64::NAN } else {
        let w = &c[i + 1 - 100..=i]; let m = w.iter().sum::<f64>() / 100.;
        (w.iter().map(|v| (v - m) * (v - m)).sum::<f64>() / 100.).sqrt() }).collect();
    let tr: Vec<f64> = (0..n).map(|i| if i == 0 { h[0] - l[0] } else { h[i].max(c[i - 1]) - l[i].min(c[i - 1]) }).collect();
    let atr = sma(&tr, 10);
    let up = |m: &[f64], i: usize| i >= 2 && m[i] > m[i - 1] && m[i - 1] > m[i - 2];
    let dn = |m: &[f64], i: usize| i >= 2 && m[i] < m[i - 1] && m[i - 1] < m[i - 2];
    let mut out = vec![];
    let mut pos: Option<(f64, usize, f64)> = None;
    let mut pend: Option<(f64, usize)> = None; // (0 = exit / +-1 = entry, signal bar)
    for i in from..=to {
        if let Some((d, s)) = pend.take() {
            if d == 0. { let (pd, e, ep) = pos.take().unwrap(); out.push(mk(pd, e, ep, "open", i, o[i], "Pegasus exit · next open", false, s)); }
            else { pos = Some((d, i, o[i])); }
        }
        if i == to { break; }
        let big = tr[i] > 2. * atr[i];
        match pos {
            Some((d, _, _)) if d > 0. => if c[i] < m100[i] || dn(&m25, i) || (big && c[i] < c[i - 1]) { pend = Some((0., i)); },
            Some(_) => if c[i] > m100[i] || up(&m25, i) || (big && c[i] > c[i - 1]) { pend = Some((0., i)); },
            None => {
                if up(&m25, i) && up(&m50, i) && c[i] > m100[i] + sd[i] { pend = Some((1., i)); }
                else if dn(&m25, i) && dn(&m50, i) && c[i] < m100[i] - sd[i] { pend = Some((-1., i)); }
            }
        }
    }
    if let Some((d, e, ep)) = pos { out.push(mk(d, e, ep, "open", to, c[to], "open", true, to)); }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;
    #[test]
    fn band_entries_next_open_and_exits() {
        let r: Vec<Row> = (0..1200).map(|i| { let f = i as f64; let c = 100. + 25. * (f / 90.).sin() + 0.02 * f;
            Row { ticker: "T".into(), date: format!("{:05}", i), open: c + 0.2, high: c + 1., low: c - 1., close: c, volume: None } }).collect();
        let x = Ctx::new(&r, &r, &[]);
        let t = run("pwpeg_pegasus", &x, 0, r.len() - 1, &Params::new()).unwrap();
        assert!(t.iter().any(|z| z.dir > 0.) && t.iter().any(|z| z.dir < 0.));
        for z in &t { assert_eq!(z.legs[0].px, x.o[z.legs[0].i]); }
        for w in t.windows(2) { assert!(w[1].legs[0].i > w[0].exit_i); }
        assert!(run("pwpeg_x", &x, 0, 5, &Params::new()).is_none());
    }
}

// Instrument admission: Peter B. Waite (Pegasus open code) 2026-11-01 (titles lane K9).
// Admission only for the module REVIEWED lists (authors/titles-1030/K9w/k9_ref.py (header)).
pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book == "pwpeg" && reviewed(def.id),
    label: "Peter Waite's Pegasus is a futures system (his results are futures portfolios); its $3,000 per-contract risk cap is held. Runs on indices, ETFs and stocks, US and India, are a declared application.",
    basis: "Peter B. Waite, Pegasus open TradeStation code (Pegasus_Open Code.rar) and About Pegasus.doc (pegasusfutures.com)",
};
