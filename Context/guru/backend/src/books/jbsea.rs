//! Jake Bernstein, "Seasonality: Systems, Strategies, and Signals" (Wiley 1998; scan, 244 PDF pages; printed page = PDF page - 19). Book id `jbsea`.
//! Source: research share "USB HDD 2/tRADING KNO-2/Jack Bernstein - Seasonality. Systems, Strategies & Signals (trade-futures.com)/Seasonality.pdf".
//! Lane CV3 (18 Nov 2026). Chapter 7 "Critical Seasonal Month Concept" (printed p.99-106, PDF 118-125): the critical month indicator (CMI).
//! Each CMI card runs only on the Norgate continuous contract of the market the book prints (futures::CARDS_CV3, R4 #293 (1)).
//! The other chapters are seasonal charts and key-date seasonals (key dates are coded in jbhsms / jbtywr) and are not repeated here.
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};
use super::jb624::week_changes;
use super::tselhs::Book;

pub const REVIEWED: &[&str] = &["jbsea_cmi_copper", "jbsea_cmi_gold"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const NOTE: &str = "[Declared readings, 3 unstated details, R4 #339/#348; UNVERIFIED] Critical month (CM) = the month the book names for the market: December for copper ('The December CMI has been a very powerful indicator in copper', p.103), January for gold ('CMI breakouts in gold above the January high and low', p.103); p.month overrides it. Printed rule (p.99-100): once the CM has ended take its highest high (CMIB) and lowest low (CMIS); 'a weekly close above the CMIB or below the CMIS', the penetration 'must occur on a closing basis for the entire week (the closing price on the last trading day of the week)'; buy above the CMIB, sell below the CMIS, 'a reversing signal below the CMIS' ends a long (the book's copper list, p.101-103, shows reversals: 'a buy followed by a sell'). The three unstated details: (1) the CMIB/CMIS apply from the end of the CM month until the next CM month begins, and a position still open then is closed at the last close before it (the book lists the signals year by year); (2) the signal fills at that weekly close (the book does not name the order); (3) the 'dollar risk management stop loss' the book mentions is not applied (no amount is printed). The signal needs the whole CM month inside the run window. The book prints no trade list or result for the CMI (only a year-by-year commentary for copper and gold), so nothing is compared with a book figure. The book's contract months differ from the Norgate continuous contract (back-adjusted daily bars).";

pub fn catalogue() -> Vec<StrategyDef> {
    let d = |id, name, what| StrategyDef { id, name, group: "Critical Seasonal Month (Bernstein 1998)", session: 0, dir: "Long/Short", status: Status::Live,
        needs_volume: false, needs_vix: false, needs_series: &[], book: "jbsea", rules: &[
            "F|\"Wait until the CM has come to an end ... find the extreme high and extreme low for the entire month ... These two points will become your CMI buy point (CMIB) and your CMI sell point (CMIS).\" (p.100)",
            "E|\"Watch weekly closing prices for a weekly close above the CMIB or below the CMIS. The penetration ... must occur on a closing basis for the entire week (the closing price on the last trading day of the week).\" (p.100)",
            "E|\"Assume ... July corn closed at 2.38, above the CMIB. You would buy it using either a dollar risk management stop loss or a reversing signal below the CMIS.\" (p.100)",
            "X|Reversing weekly close beyond the other end of the critical month (stop-and-reverse); flat when the next critical month begins (declared)"],
        notes: what, claim: None };
    vec![d("jbsea_cmi_copper", "Critical Month Indicator, copper: weekly close beyond the December high / low", NOTE),
         d("jbsea_cmi_gold", "Critical Month Indicator, gold: weekly close beyond the January high / low", NOTE)]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    reviewed(id).then_some(("jbsea_cmi", "Bernstein critical month indicator", "weekly close beyond the CM range"))
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let m = if id == "jbsea_cmi_gold" { 1. } else { 12. };
    reviewed(id).then(|| vec![ParamDef { key: "month", label: "Critical month (book: December for copper, January for gold)", def: m, min: 1., max: 12., step: 1. }])
}

fn month(d: &str) -> u32 { d.get(5..7).and_then(|s| s.parse().ok()).unwrap_or(0) }

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.r.len() { return None; }
    let cm = p.get("month").copied().unwrap_or(if id == "jbsea_cmi_gold" { 1. } else { 12. }).round() as u32;
    let mon: Vec<u32> = x.r.iter().map(|r| month(&r.date)).collect();
    let mut pos = Book::new();
    // the latest critical-month block that ended before the current bar: (high, low); None until one full block lies inside the window
    let mut lv: Option<(f64, f64)> = None;
    let (mut bh, mut bl, mut in_block, mut block_ok) = (f64::NEG_INFINITY, f64::INFINITY, false, false);
    for t in from..=to {
        if mon[t] == cm {
            if !in_block { in_block = true; block_ok = true; bh = f64::NEG_INFINITY; bl = f64::INFINITY; }
            bh = bh.max(x.h[t]); bl = bl.min(x.l[t]);
            if t == to || mon[t + 1] != cm { if block_ok && bh.is_finite() { lv = Some((bh, bl)); } in_block = false; }
            continue;
        }
        let Some((cmib, cmis)) = lv else { continue };
        if week_changes(x, t) {
            if x.c[t] > cmib { pos.enter(1., t, x.c[t], "close"); } else if x.c[t] < cmis { pos.enter(-1., t, x.c[t], "close"); }
        }
        // the next critical month starts on the next bar: flat at this close
        if t < to && mon[t + 1] == cm { pos.exit(t, x.c[t], "next critical month"); }
    }
    Some(pos.done(to, x.c[to]))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;
    #[test]
    fn weekly_close_beyond_december_range_reverses() {
        // Dec 2001 range 100..110; Jan weeks: close 112 on the first Friday -> long; later a weekly close 98 -> reverses short
        let mut b: Vec<(String, f64)> = vec![];
        for d in 3..=31 { b.push((format!("2001-12-{d:02}"), if d % 2 == 0 { 100. } else { 110. })); }
        let jan: [(u32, f64); 14] = [(2, 105.), (3, 106.), (4, 107.), (5, 112.), (8, 111.), (9, 108.), (10, 104.), (11, 99.), (12, 98.), (15, 97.), (16, 96.), (17, 95.), (18, 96.), (19, 95.)];
        for (d, c) in jan { b.push((format!("2002-01-{d:02}"), c)); }
        let r: Vec<Row> = b.iter().map(|(d, c)| Row { ticker: "T".into(), date: d.clone(), open: *c, high: *c, low: *c, close: *c, volume: None }).collect();
        let x = Ctx::new(&r, &r, &[]);
        let t = run("jbsea_cmi_copper", &x, 0, r.len() - 1, &Params::new()).unwrap();
        assert!(t.len() >= 1);
        assert_eq!((t[0].dir, t[0].legs[0].px), (1., 112.));
        assert!(run("jbsea_x", &x, 0, 9, &Params::new()).is_none() && family_of("jbsea_cmi_gold").is_some() && params("jbsea_cmi_gold").is_some());
    }
}
