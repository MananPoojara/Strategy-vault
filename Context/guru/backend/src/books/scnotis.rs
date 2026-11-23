//! Steve Notis, "Testing breakout systems", Technical Analysis of Stocks & Commodities V.7:3 (March 1989) pp.96-99
//! (research share Trading Knowledge I/Stocks & Commodities DVD 11.26 (1982 - 2008) (traders.com)/VOLUMES/V07/C03/TESTING.pdf).
//! Book id `scnotis`: Larry Williams' "75% system", a stop-and-reverse on daily bars. Lane I2 (Q385, 24 Nov 2026, R4 #382);
//! independent check authors/conv-1123/i2w/scnotis_ref.py.
use crate::connors::{Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};

const ID: &str = "scnotis_75";
pub const REVIEWED: &[&str] = &[ID];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const NOTE: &str = "A|Declared readings (lane I2, 3; R4 #340/#348/#382): (1) a stop whose level lies beyond the open fills at the open (a gap through the level), otherwise at the level; (2) a bar that touches both levels while flat is taken on the level nearer the open, and a bar that reverses an open position ignores the same bar's second level (the share of such bars is measured on the gate row); (3) the system starts flat and is in the market from the first stop that is touched. The article tests the S&P 500 futures contract (1987-1988, 251.25 points in 115 trades before costs); the vault applies the printed rule to the NSE index series, declared adaptation, no costs. The target factor is 0.75 times the five-day average true range (gap-adjusted true range as printed), added to and subtracted from today's close.";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef { id: ID, name: "Williams 75% system (Notis): buy stop at close + 0.75 x 5-day ATR, sell stop at close - 0.75 x 5-day ATR, always in the market", group: "S&C: Testing breakout systems (Notis, V.7:3)", session: 0, dir: "Long/Short", status: Status::Live,
        needs_volume: false, needs_vix: false, needs_series: &[], book: "scnotis",
        rules: &[
            "F|\"you determine the daily range on the S&P 500 futures contract for each of the past five days, you add the ranges together and then divide by five to get the average range of the past five days. Multiply the average range by 0.75\" (V.7:3, PDF p.1)",
            "E|\"This number-the target factor-is added to today's closing price to arrive at tomorrow's long target, and then subtracted from today's close to arrive at tomorrow's short target. By setting a buy stop at tomorrow's long target and a sell stop at tomorrow's short target, you will be stopped into the appropriate long or short position\" (PDF p.1)",
            "X|\"Once you are long you stay long until the day a short target is hit, at which time you cover your long and go short-so you always have a position.\" (PDF p.1)",
            "F|\"if the market just continued right on up without coming back down to fill the gap ... the distance from today's high to yesterday's close should be considered to be today's true range ... you will be calculating the five-day average true range or ATR\" (PDF p.1-2)",
            NOTE],
        notes: "Notis, S&C V.7:3 pp.96-99: Williams' 75% breakout system, stop-and-reverse. Declared adaptation (R4 #340): the S&P 500 futures test is run on the NSE index series; three labelled readings.",
        claim: None }]
}
pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> { reviewed(id).then_some((ID, "Williams 75% system (Notis, S&C)", "0.75 x 5-day ATR")) }
pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    reviewed(id).then(|| vec![
        ParamDef { key: "n", label: "Average true range length (article 5)", def: 5., min: 2., max: 30., step: 1. },
        ParamDef { key: "f", label: "Target factor (article 0.75)", def: 0.75, min: 0.25, max: 3., step: 0.05 }])
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.c.len() { return None; }
    let n = p.get("n").copied().unwrap_or(5.).round().max(2.) as usize;
    let f = p.get("f").copied().unwrap_or(0.75);
    let (o, h, l, c) = (&x.o, &x.h, &x.l, &x.c);
    let m = c.len();
    let mut tr = vec![f64::NAN; m];
    for i in 1..m { tr[i] = h[i].max(c[i - 1]) - l[i].min(c[i - 1]); }
    let mut out: Vec<Trade> = vec![];
    let mut pos: Option<(f64, usize, f64)> = None; // dir, entry bar, entry px
    let mk = |dir: f64, e: usize, ep: f64, xi: usize, xp: f64, open: bool| Trade { dir, legs: vec![Leg { i: e, px: ep, w: 1.0 }], exit_i: xi, exit_px: xp, reason: if open { "open".into() } else { "reversed on the opposite target".into() }, entry_kind: "limit", signal_i: xi, open, exits: Vec::new() };
    for d in from.max(n + 1)..=to {
        let s = d - 1;
        let atr = tr[s + 1 - n..=s].iter().sum::<f64>() / n as f64;
        if !atr.is_finite() { continue; }
        let (up, dn) = (c[s] + f * atr, c[s] - f * atr);
        let (hit_up, hit_dn) = (h[d] >= up, l[d] <= dn);
        let go = |dir: f64| (dir, if dir > 0. { o[d].max(up) } else { o[d].min(dn) });
        let new = match pos {
            None => if hit_up && (!hit_dn || (o[d] - up).abs() <= (o[d] - dn).abs()) { Some(go(1.)) } else if hit_dn { Some(go(-1.)) } else { None },
            Some((dir, ..)) => if dir > 0. && hit_dn { Some(go(-1.)) } else if dir < 0. && hit_up { Some(go(1.)) } else { None },
        };
        if let Some((dir, ep)) = new {
            if let Some((pd, e, pe)) = pos { out.push(mk(pd, e, pe, d, ep, false)); }
            pos = Some((dir, d, ep));
        }
    }
    if let Some((pd, e, pe)) = pos { out.push(mk(pd, e, pe, to, c[to], true)); }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn stop_and_reverse_always_in() {
        let r = super::super::sctraub::tests::bars(1500);
        let x = Ctx::new(&r, &r, &[]);
        let t = run(ID, &x, 0, r.len() - 1, &Params::new()).unwrap();
        assert!(t.len() > 20);
        for w in t.windows(2) { assert_eq!(w[0].exit_i, w[1].legs[0].i); assert!(w[0].dir == -w[1].dir); }
        for z in &t { assert!(z.legs[0].px >= r[z.legs[0].i].low - 1e-9 && z.legs[0].px <= r[z.legs[0].i].high + 1e-9); }
    }
    /// hand-run dump for the pandas reference: Q385_CSV=<idx_data.csv> Q385_OUT=<file> cargo test --release scnotis_dump -- --ignored
    #[test]
    #[ignore]
    fn scnotis_dump() {
        use std::io::Write;
        let data = crate::load_csv(&std::env::var("Q385_CSV").unwrap()).unwrap();
        let mut f = std::io::BufWriter::new(std::fs::File::create(std::env::var("Q385_OUT").unwrap()).unwrap());
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
    reviewed: |def: &crate::connors::StrategyDef| def.book == "scnotis",
    label: "Notis tested Williams' 75% system on the S&P 500 futures contract (1987-1988); the vault runs the printed rule on four NSE index series (NIFTY 50, NIFTYBANK, NIFTYIT, NIFTYMIDCAPSELECT), a declared adaptation, indices only; NIFTYMIDCAP100 is excluded because 6.3% of its trades fall on bars that touch both levels (over the 5% line, #240).",
    basis: "Technical Analysis of Stocks & Commodities V.7:3 pp.96-99 (Testing breakout systems); independent check authors/conv-1123/i2w/scnotis_ref.py",
};
