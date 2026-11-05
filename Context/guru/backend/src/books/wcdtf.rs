//! Cole Wilcox & Eric Crittenden, "Does Trend Following Work on Stocks?" (Blackstar Funds, November 2005; research share
//! USB HDD 2/tRADING KNO-2/Cole Wilcox - Does Trend Following Work on Stocks (Article), pdftotext p.3-7). Book id `wcdtf`.
//! Lane K34 (4 Nov 2026), D1-14 / decision 186: the paper prints the entry and the 10-ATR trailing stop but not the ATR length
//! or the trailing anchor; both are filled with the plainest labelled reading (the user's 30 Oct rule for unprinted periods).
//! Independent check authors/titles-1030/K34w/k34_ref.py.
use crate::connors::{atr, claim, Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "wcdtf";
pub const REVIEWED: &[&str] = &["wcdtf_ath_atr"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const NOTE: &str = "[Declared readings] All-time high = the highest close of every bar the vault holds for the instrument up to today (today included; the paper's database starts in 1983, ours where the file starts), checked once the ATR has its 14 bars. One position at a time, long only. ATR = Wilder's 14-day average true range (the paper prints the true range but no length: 14 is Wilder's own, labelled; p.atr_n). Trailing stop = the highest close since the signal day minus 10 x today's ATR, never lowered (the paper does not say what the stop trails: the highest close is the plainest reading, labelled). 'Breached' = a close below the stop level set at the previous close; the exit is at the next day's open, as printed. The paper's minimum-price and liquidity filters (appendix 4) and its 0.5% round-turn costs are not applied. Book: 24,000+ US stocks incl. delisted, 1983-2005: 18,000+ trades, 49.3% winners, average win / average loss 2.56, expectancy about 15.2% a trade, average hold 305 calendar days (p.7). Runs on indices, ETFs and stocks, US and India, are a declared application.";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef { id: "wcdtf_ath_atr", name: "All-time-high close, 10-ATR trailing stop (Does Trend Following Work on Stocks?)", group: "Wilcox & Crittenden 2005",
        session: 0, dir: "Long", status: Status::Live, needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK, rules: &[
            "E|\"if today's close is greater than or equal to the highest close during the stock's entire history then buy tomorrow on the open\" (p.3)",
            "X|\"We decided to use average true range trailing stops\"; \"a 10 ATR stop\" (p.4-5); true range = the greatest of today's high minus today's low, today's high minus yesterday's close, yesterday's close minus today's low (p.4)",
            "X|\"we chose to exit a stock on the open the day after the exit level was breached\" (p.5)"],
        notes: NOTE, claim: claim("18,000+", "49.3%", "expectancy ~15.2%", "305 calendar days", "24,000+ US stocks incl. delisted, 1983-2005, 0.5% round-turn costs") }]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    reviewed(id).then_some(("wcdtf_ath_atr", "All-time high + ATR trailing stop (Wilcox & Crittenden)", "10 x ATR(14)"))
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    reviewed(id).then(|| vec![p("mult", "Stop: ATRs below the highest close (paper 10; it tested 8-12)", 10., 2., 20., 0.5),
        p("atr_n", "ATR length (not printed: Wilder's 14, labelled)", 14., 2., 100., 1.)])
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.r.len() { return None; }
    let schema = params(id)?;
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).unwrap().def);
    let (m, n) = (g("mult"), g("atr_n").round().max(1.) as usize);
    let a = atr(x.r, n);
    let c = &x.c;
    let mut out = vec![];
    let mut ath = f64::NEG_INFINITY;
    // (entry bar, entry price, highest close since the signal day, stop level in force)
    let mut cur: Option<(usize, f64, f64, f64)> = None;
    let mut sig: Option<usize> = None; // signal close waiting for the next open
    let mut ex: Option<usize> = None; // breach close waiting for the next open
    for i in 0..=to {
        if i >= from {
            if let (Some(s), Some((ei, ep, _, _))) = (ex.take(), cur) {
                out.push(Trade { dir: 1., legs: vec![Leg { i: ei, px: ep, w: 1. }], exit_i: i, exit_px: x.o[i], reason: "close below the ATR trailing stop: out at the next open".into(),
                    entry_kind: "open", signal_i: s, open: false, exits: vec![] });
                cur = None;
            }
            if let Some(s) = sig.take() {
                cur = Some((i, x.o[i], c[s], c[s] - m * a[s]));
            }
        }
        ath = ath.max(c[i]);
        if let Some((ei, ep, hc, lv)) = cur {
            if c[i] < lv && i < to { ex = Some(i); }
            cur = Some((ei, ep, hc.max(c[i]), lv.max(hc.max(c[i]) - m * a[i])));
        } else if i >= from && i < to && a[i].is_finite() && c[i] >= ath {
            sig = Some(i);
        }
    }
    if let Some((ei, ep, _, _)) = cur {
        out.push(Trade { dir: 1., legs: vec![Leg { i: ei, px: ep, w: 1. }], exit_i: to, exit_px: c[to], reason: "open".into(), entry_kind: "open", signal_i: to, open: true, exits: vec![] });
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;
    #[test]
    fn all_time_high_entry_and_trailing_atr_exit() {
        // flat 100 for 20 bars (range 2) -> close 101 is an all-time high: buy next open; mult 2: stop = 101 - 2 x ATR(~2) ~ 97
        let mut cl: Vec<f64> = vec![100.; 20];
        cl.extend([101., 102., 103., 99., 96., 95., 95.]);
        let r: Vec<Row> = cl.iter().enumerate().map(|(i, &c)| Row { ticker: "T".into(), date: format!("2020-{:02}-{:02}", i / 28 + 1, i % 28 + 1), open: c + 0.5, high: c + 1., low: c - 1., close: c, volume: None }).collect();
        let x = Ctx::new(&r, &r, &[]);
        let t = run("wcdtf_ath_atr", &x, 0, r.len() - 1, &Params::from([("mult".into(), 2.)])).unwrap();
        assert_eq!(t.len(), 1, "{t:?}");
        // entry on the open after the first all-time-high close with a ready ATR (the flat 100s are all-time highs)
        let first = atr(&r, 14).iter().position(|v| v.is_finite()).unwrap();
        assert_eq!((t[0].legs[0].i, t[0].legs[0].px), (first + 1, 100.5));
        assert!(!t[0].open && t[0].exit_px == x.o[t[0].exit_i] && t[0].signal_i + 1 == t[0].exit_i);
        assert!(run("wcdtf_x", &x, 0, 5, &Params::new()).is_none());
    }
}

// Instrument admission: Wilcox & Crittenden 2026-11-04 (titles lane K34, D1-14).
pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book == BOOK && reviewed(def.id),
    label: "Wilcox & Crittenden tested this on 24,000+ US stocks (1983-2005, delisted included); runs on indices, ETFs and stocks, US and India, are a declared application, not the authors' test.",
    basis: "Cole Wilcox & Eric Crittenden, Does Trend Following Work on Stocks? (Blackstar Funds, Nov 2005) p.3-7; independent check authors/titles-1030/K34w/k34_ref.py",
};
