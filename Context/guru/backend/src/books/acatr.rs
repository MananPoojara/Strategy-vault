//! E. Acar & S. Satchell (eds.), Advanced Trading Rules, 2nd ed. (Butterworth-Heinemann 2002).
//! Only rules whose complete entry AND exit are printed exactly are Live. Source: library/acatr/.
use crate::connors::{Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "acatr";
/// Admitted ids (authors/satchell/admission/SOURCE-CONTRACT.md).
pub const REVIEWED: &[&str] = &["acatr_kuo_gma21", "acatr_gasser_brk", "acatr_filter_05", "acatr_filter_1", "acatr_filter_2"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const FX: &str = "needs daily FX spot rates (DEM, JPY, CAD, CHF, FRF, GBP vs USD) and, for ch.1, eurodollar/call-money interest rates; not in the vault";

pub fn catalogue() -> Vec<StrategyDef> {
    let d = |id, name, group, dir, status, rules, notes| StrategyDef { id, name, group, session: 0, dir, status, rules, notes,
        needs_volume: false, needs_vix: false, needs_series: &[], book: "acatr", claim: None };
    vec![
        d("acatr_kuo_gma21", "Geometric moving-average (2,1) rule (Kuo)", "Ch.5 Moving-average rules", "Long", Status::Live,
          &["E|GMA(n,1): \"Bt >= 0 if Pt >= (Prod_{j=0..n-1} Pt-j)^(1/n)\" (Eq. 5.3, p.156); n = 2, i.e. Pt >= sqrt(Pt * Pt-1) <=> Pt >= Pt-1",
            "E|\"a moving-average (n,m) rule will generate a buying signal for the next period when Bt >= 0\" (p.156): buy / hold at today's close",
            "X|\"Otherwise, it will issue a selling signal suggesting investors to close their positions at the beginning of the next period\" (p.157): exit at today's close"],
          "Kuo applies GMA(2,1) to the FT100 index and futures (p.169) only through a fitted Markov model (expected 0.28%/day index, 0.014%/day futures); no backtest is printed, so nothing to reproduce. Prices = daily closes; signal and fill at the same close (\"for the next period\"). Long-only: a sell signal closes the position. India = run on the index instead of the FT100."),
        d("acatr_gasser_brk", "45-bar close breakout, stop-and-reverse (Gasser)", "Ch.15 Performance evaluation", "Long/Short", Status::Live,
          &["E|\"IF Close >= Highest(c,Length)[1] Then Buy on Close\" (p.430, Length = 45)",
            "E|\"IF Close <= Lowest(c,Length)[1] Then Sell on Close\" (p.430); TradeStation Buy/Sell reverse an opposite position",
            "X|Exit only by the opposite signal (always in the market after the first signal)"],
          "TradeStation code printed in Gasser's 'Send Average Trade to file' listing, applied to Swiss franc IMM futures 1980-2000 (Figs 15.6-15.7, charts only, no numbers). Highest/Lowest of the prior 45 closes (the [1] offset excludes today). Run on the index instead of CHF futures."),
        d("acatr_filter_05", "Filter rule 0.5% (Chang & Osler)", "Ch.9 Filter rules", "Long/Short", Status::Live, FILTER, FNOTE),
        d("acatr_filter_1", "Filter rule 1% (Chang & Osler)", "Ch.9 Filter rules", "Long/Short", Status::Live, FILTER, FNOTE),
        d("acatr_filter_2", "Filter rule 2% (Chang & Osler)", "Ch.9 Filter rules", "Long/Short", Status::Live, FILTER, FNOTE),
        d("acatr_hs", "Head-and-shoulders zigzag rule (Chang & Osler)", "Ch.9 Head-and-shoulders", "Long/Short", Status::NotTestable("zigzag cutoffs are multiples of 'one standard deviation of daily exchange rate changes' without a defined estimation window (full-sample = look-ahead), and the handling of duplicate patterns across the 10 cutoffs is not specified (p.267, fn 3/5)"),
          &["E|Zigzag peaks/troughs at 10 cutoffs (1, 1.25 ... 4.5 x std of daily changes); 5 geometric criteria (p.267-268); enter on neckline cross",
            "X|Exit on the next zigzag trough/peak; stop-loss 1% of entry; 25% 'bounce' allowance (p.270-271)"], "FX only in the book."),
        d("acatr_lebaron_ma", "Price vs 30-period moving average, weekly, with interest carry (LeBaron)", "Ch.1 FX regime shifts", "Long/Short", Status::NeedsData(FX),
          &["E|\"When the current price is above the long moving-average buy is indicated and when it is below a sell is indicated ... thirty-day moving average\" (p.28)", "E|Trading once a week, 50% leveraged, foreign bonds vs dollar borrowing (p.28-29)"],
          "Return includes the interest differential; meaningless without FX + rate data."),
        d("acatr_karjalainen_ga", "Genetic-algorithm S&P 500 futures rules (Karjalainen)", "Ch.12 GA rules", "Long/Short", Status::NotTestable("the 100 evolved rule trees are not printed (only Fig 12.1 illustrative parents)"), &[], ""),
        d("acatr_lo_patterns", "Kernel-regression chart patterns (Lo, Mamaysky & Wang)", "Ch.2 Foundations of TA", "Both", Status::NotTestable("an event study of conditional return distributions; no trading exit is defined"), &[], ""),
        d("acatr_barep_sr", "Support/resistance breakout (BAREP)", "Ch.14 BAREP futures funds", "Long/Short", Status::NotTestable("'significant penetration', look-back and stop/take-profit values are not given (only a 30-90 day / 1-3 point optimisation map on JGB, p.394)"), &[], ""),
    ]
}

const FILTER: &[&str] = &[
    "E|\"the trader begins with no position at all, and keeps track of both the maximum and a minimum price observed over time. (On day one, the maximum and minimum are both set to that day's price.)\" (p.269)",
    "E|\"As soon as the price is either at least x per cent above the minimum or at least x per cent below the maximum, the trader enters a long or short position respectively\" (p.269)",
    "X|\"as soon as a trader enters a long position, he or she begins to keep track of the maximum price attained since entry ... As soon as the price has dropped x per cent from that maximum, the long position is closed and a short position entered\" and the mirror for shorts (p.265-266)",
];
const FNOTE: &str = "Filter sizes 0.5%, 1% and 2% are the three the chapter tests (Tables 9.1-9.3) on six USD exchange rates; results are daily average returns on FX only, so nothing to reproduce on indices. Price = daily close; fills at that close. 'Day one' = the first bar of the selected test window. Run on the index instead of the exchange rate.";

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    Some(match id {
        "acatr_filter_05" => ("acatr_filter", "Filter rule (Chang & Osler)", "x = 0.5%"),
        "acatr_filter_1" => ("acatr_filter", "Filter rule (Chang & Osler)", "x = 1%"),
        "acatr_filter_2" => ("acatr_filter", "Filter rule (Chang & Osler)", "x = 2%"),
        _ => return catalogue().into_iter().find(|d| d.id == id).map(|d| (d.id, d.name, "")),
    })
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    Some(match id {
        "acatr_kuo_gma21" => vec![p("n", "GMA length n (book: 2)", 2., 2., 10., 1.)],
        "acatr_gasser_brk" => vec![p("len", "Breakout length (book: 45)", 45., 10., 100., 5.)],
        "acatr_filter_05" => vec![p("x", "Filter size %", 0.5, 0.25, 5., 0.25)],
        "acatr_filter_1" => vec![p("x", "Filter size %", 1., 0.25, 5., 0.25)],
        "acatr_filter_2" => vec![p("x", "Filter size %", 2., 0.25, 5., 0.25)],
        _ => return None,
    })
}

fn open_at(dir: f64, i: usize, px: f64) -> Trade {
    Trade { dir, legs: vec![Leg { i, px, w: 1. }], exit_i: i, exit_px: px, reason: String::new(), entry_kind: "close", signal_i: i, open: false, exits: vec![] }
}
fn close_out(out: &mut Vec<Trade>, cur: &mut Option<Trade>, i: usize, px: f64, why: &str) {
    if let Some(mut t) = cur.take() { t.exit_i = i; t.exit_px = px; t.signal_i = i; t.reason = why.into(); out.push(t); }
}
/// Desired position at each close is `want(i)`; Some(dir) changes it, None keeps it. Fills at the close.
fn at_close(x: &Ctx, from: usize, to: usize, mut want: impl FnMut(usize) -> Option<f64>) -> Vec<Trade> {
    let (mut out, mut cur): (Vec<Trade>, Option<Trade>) = (vec![], None);
    for i in from..=to {
        let Some(d) = want(i) else { continue };
        if cur.as_ref().map_or(0., |t| t.dir) == d { continue; }
        close_out(&mut out, &mut cur, i, x.c[i], if d == 0. { "Sell signal · close" } else { "Opposite signal · reverse at close" });
        if d != 0. && i < to { cur = Some(open_at(d, i, x.c[i])); }
    }
    if let Some(mut t) = cur { t.exit_i = to; t.exit_px = x.c[to]; t.signal_i = to; t.reason = "open".into(); t.open = true; out.push(t); }
    out
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    let schema = params(id)?;
    if from > to || to >= x.c.len() { return None; }
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).unwrap().def);
    let c = &x.c;
    Some(match id {
        "acatr_kuo_gma21" => {
            let n = g("n").max(2.) as usize;
            at_close(x, from, to, |i| {
                if i + 1 < n { return None; }
                let lg = (i + 1 - n..=i).map(|j| c[j].ln()).sum::<f64>() / n as f64;
                Some(if c[i].ln() >= lg { 1. } else { 0. })
            })
        }
        "acatr_gasser_brk" => {
            let n = g("len").max(1.) as usize;
            at_close(x, from, to, |i| {
                if i < n { return None; }
                let w = &c[i - n..i];
                let (hi, lo) = (w.iter().copied().fold(f64::MIN, f64::max), w.iter().copied().fold(f64::MAX, f64::min));
                if c[i] >= hi { Some(1.) } else if c[i] <= lo { Some(-1.) } else { None }
            })
        }
        _ => {
            let f = g("x") / 100.;
            let (mut pos, mut hi, mut lo) = (0f64, c[from], c[from]);
            at_close(x, from, to, |i| {
                if i == from { return None; }
                let px = c[i];
                let next = match pos as i8 {
                    0 => if px >= lo * (1. + f) { 1. } else if px <= hi * (1. - f) { -1. } else { 0. },
                    1 => if px <= hi * (1. - f) { -1. } else { 1. },
                    _ => if px >= lo * (1. + f) { 1. } else { -1. },
                };
                if next != pos { pos = next; hi = px; lo = px; } else { hi = hi.max(px); lo = lo.min(px); }
                (pos != 0.).then_some(pos)
            })
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;
    fn ctx(cl: &[f64]) -> Vec<Row> {
        cl.iter().enumerate().map(|(i, &c)| Row { ticker: "T".into(), date: format!("{:04}-01-01", 2000 + i), open: c, high: c, low: c, close: c, volume: None }).collect()
    }
    #[test]
    fn filter_one_percent_by_hand() {
        let r = ctx(&[100., 100.5, 101., 102., 100.9, 100.5, 101.6]);
        let x = Ctx::new(&r, &r, &[]);
        let t = run("acatr_filter_1", &x, 0, 6, &Params::new()).unwrap();
        // long at 101 (>=1% over 100), max 102, short at 100.9 (<=102*0.99=100.98), min 100.5, long at 101.6 (>=101.505)
        // the final long signal falls on the last bar, so no position is opened there
        assert_eq!(t.len(), 2);
        assert_eq!((t[0].dir, t[0].legs[0].i, t[0].exit_i), (1., 2, 4));
        assert_eq!((t[1].dir, t[1].exit_i, t[1].open), (-1., 6, false));
    }
    #[test]
    fn gma21_is_up_close() {
        let r = ctx(&[10., 11., 12., 11., 13.]);
        let x = Ctx::new(&r, &r, &[]);
        let t = run("acatr_kuo_gma21", &x, 0, 4, &Params::new()).unwrap();
        // up closes hold the long; the down close at bar 3 exits; the bar-4 signal is on the last bar
        assert_eq!(t.len(), 1);
        assert_eq!((t[0].legs[0].i, t[0].exit_i), (1, 3));
    }
}

// Instrument admission: Satchell 2026-10-24 block, moved from instrument_policy.rs on 31 Oct 2026 (lane ST).
// Admission only for the module REVIEWED lists (authors/satchell/admission/SOURCE-CONTRACT.md); kept in one block to merge.
// Runs on actual indices, ETFs and stocks, US and India (index_etf_or_stock, nifty50 basket, declared application).
pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| matches!(def.book,"acatr") && (crate::books::acatr::reviewed(def.id)),
    label: "The chapter authors of Advanced Trading Rules (Acar and Satchell, eds.) studied these rules on currencies, Swiss franc futures and the FT100; runs on indices, ETFs and stocks, US and India, are a declared application, not the authors' test.",
    basis: "E. Acar and S. Satchell (eds.), Advanced Trading Rules, 2nd ed. (2002): Kuo's geometric moving-average (2,1) rule, Eq. 5.3-5.4 p.156-160; Gasser's 45-bar close breakout, TradeStation code p.430; Chang and Osler's filter rules 0.5/1/2 per cent, p.265-269. Signals and fills on the close. Reviewed in authors/satchell/admission/SOURCE-CONTRACT.md with independent chronological ledgers; no costs.",
};
