//! Jake Bernstein, "Trade Your Way to Riches Now!" (MBH Commodity Advisors, 1996), Seasonals Part II, Key Date Seasonals: Figures 4-5 (December wheat),
//! 4-6 (June T-bonds) and 4-8 (December live hogs). Book id `jbtywr` (these three were NeedsData "no grain / bond / livestock futures" in books/jbtywr.rs;
//! the Norgate continuous contracts &ZW, &ZB, &HE are held, R4 #414). Lane K3 (25 Nov 2026). Independent check authors/conv-1123/k3w/jbtywrkd_ref.py.
//! Same table header as the live Dow key dates (ENTRY / EXIT / SL / BUY / CLOSE): enter on the close of the entry date, exit on the close of the exit date,
//! stop on a closing basis as a % of the entry price; the stop is judged on the UNADJUSTED close and entry (R4 #301: the % base is the unadjusted fill).
use super::jb624::{finish, mark_open, new_trade};
use crate::connors::{claim as c, Ctx, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "jbtywr";
/// (id, name, direction, entry month/day, exit month/day, stop %, rule text, trades, win%, period, note)
const KD: &[(&str, &str, f64, (u32, u32), (u32, u32), f64, &str, &str, &str, &str)] = &[
    ("jbtywr_wheat_0901", "December wheat key date · long Sep 1 to Oct 24, 5% stop", 1.0, (9, 1), (10, 24), 5.0,
     "F|Figure 4-5 \"Key Date Seasonal in Wheat\": ENTRY 9/1, EXIT 10/24, SL 5%, BUY, CLOSE; 45 trades 1950-94, 75.56%", "45", "75.56%", "1950-1994"),
    ("jbtywr_tbond_0418", "June T-bond key date · short Apr 18 to Apr 26, 1% stop", -1.0, (4, 18), (4, 26), 1.0,
     "F|Figure 4-6 \"Key Date Seasonal in TBond Futures\": ENTRY 4/18, EXIT 4/26, SL 1%, SELL, CLOSE; 16 trades 1978-93, 87.50% (the caption's \"April 10th\" contradicts the table's 4/18)", "16", "87.50%", "1978-1993"),
    ("jbtywr_hogs_1030", "December live hogs key date · long Oct 30 to Nov 27, 6% stop", 1.0, (10, 30), (11, 27), 6.0,
     "F|Figure 4-8 \"Key Date Seasonal Live Hogs\": ENTRY 10/30, EXIT 11/27, SL 6%, BUY, CLOSE; 27 trades 1967-93, 81.48%", "27", "81.48%", "1967-1993"),
];
const NOTE: &str = "[Adapted: 4 unstated details, R4 #400/#414] The book's record is the December (wheat, hogs) or June (T-bond) CONTRACT; here the Norgate continuous contract of the market (back-adjusted, P/L on the adjusted series, the stop and its base on the unadjusted close). Unstated, labelled: (1) the entry is the close of the first trading day on or after the entry date (the table's closed-market rule), the exit the first close on or after the exit date; (2) the closing stop is checked from the day after the entry, a close below (above for the short) entry x (1 -/+ stop%) exits on that close; (3) a trade open at the end of the data is marked open; (4) one unit, no costs. The book's figures are for the single contract over its own window, so the printed trade count and percent winners are not expected to match exactly.";

pub fn catalogue() -> Vec<StrategyDef> {
    KD.iter().map(|k| StrategyDef { id: k.0, name: k.1, group: "Seasonals Part II · Key Date Seasonals", session: 0, dir: if k.2 > 0.0 { "Long" } else { "Short" },
        status: Status::Live, needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK,
        rules: Box::leak(vec![k.6, "E|Table header \"ENTRY / EXIT / SL / BUY / CLOSE\": enter on the close of the entry date; a closed market moves the date to the next trading day (Seasonals Part II, Figs 4-5 to 4-8)", "X|Exit on the close of the exit date; \"SL\" = stop loss as a % of the entry price on a closing basis"].into_boxed_slice()),
        notes: NOTE, claim: c(k.7, k.8, "", k.9, "futures key date, one contract") }).collect()
}
pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    KD.iter().find(|k| k.0 == id).map(|k| ("jbtywr_kd", "Key-date seasonals (Trade Your Way to Riches Now!)", ["Wheat Sep 1-Oct 24", "T-bond Apr 18-Apr 26", "Hogs Oct 30-Nov 27"][KD.iter().position(|q| q.0 == k.0).unwrap()]))
}
pub fn params(_id: &str) -> Option<Vec<ParamDef>> { None }

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, _p: &Params) -> Option<Vec<Trade>> {
    let k = KD.iter().find(|k| k.0 == id)?;
    if from > to || to >= x.c.len() { return None; }
    let raw = crate::futures::raw_close(x)?;
    let (dir, (em, ed), (xm, xd), stop) = (k.2, k.3, k.4, k.5 / 100.0);
    let key = |i: usize| -> (u32, u32) { let d = &x.r[i].date; (d[5..7].parse().unwrap_or(0), d[8..10].parse().unwrap_or(0)) };
    let (mut out, mut cur): (Vec<Trade>, Option<(Trade, usize, i32)>) = (vec![], None);
    for i in from.max(1)..=to {
        let year: i32 = x.r[i].date[0..4].parse().unwrap_or(0);
        if let Some((_, e, y)) = cur.as_ref() {
            let past = year > *y || key(i) >= (xm, xd);
            let stopped = !past && dir * (raw[i] - raw[*e]) < -stop * raw[*e];
            if past || stopped { let (t, _, _) = cur.take().unwrap(); out.push(finish(t, i, x.c[i], i, if past { "Exit date (close)" } else { "Close-only stop" })); }
            continue;
        }
        let first_of_window = key(i) >= (em, ed) && key(i) < (xm, xd) && (i == 0 || { let pk = key(i - 1); x.r[i - 1].date[0..4] != x.r[i].date[0..4] || pk < (em, ed) });
        if i < to && first_of_window { cur = Some((new_trade(dir, i, x.c[i], "close", i), i, year)); }
    }
    mark_open(&mut out, cur.map(|c| c.0), x, to);
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;
    fn bar(d: &str, c: f64) -> Row { Row { ticker: "T".into(), date: d.into(), open: c, high: c, low: c, close: c, volume: None } }
    #[test]
    fn needs_raw_series_and_nothing_else() {
        let r = vec![bar("2000-04-17", 100.), bar("2000-04-18", 100.), bar("2000-04-19", 102.), bar("2000-04-26", 99.)];
        let x = Ctx::new(&r, &r, &[]);
        assert!(run("jbtywr_tbond_0418", &x, 0, 3, &Params::new()).is_none()); // no RAW.Close on a non-futures context: no run
        assert!(run("jbtywr_dj_1215", &x, 0, 3, &Params::new()).is_none());
    }
}
