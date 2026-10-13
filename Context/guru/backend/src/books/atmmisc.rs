//! Active Trader Magazine, "Misc Numbers" binders (research drive Trading Knowledge II/@NewsLetters-Mags/Active Trader Magazine/
//! Misc Numbers/). Book id "atmmisc". Only the binder columns not already covered by atm00-atm04 are coded here (Trading System
//! Labs V2 "Countertrend Trading": Jan 2005 - Nov 2006). Notes and the list of remaining binders: library/atmmisc/README.md.
//! Uses the atm03 helpers (own simulator; no jkbwts::sim).
use crate::books::atm03::{by_open, cmo, done, ema, finish, hh, lim_up, ll, new_trade};
use crate::connors::{sma, Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};
use Status::*;

pub const BOOK: &str = "atmmisc";

pub fn catalogue() -> Vec<StrategyDef> {
    let d = |id, name, group, dir, status, rules, notes, claim| StrategyDef { id, name, group, session: 0, dir, status, needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK, rules, notes, claim };
    let c = crate::connors::claim;
    vec![
        d("atmmisc_sma_contra", "SMA Contra System · buy 5% below SMA(14), exit above it", "TSL Jan 2005 (binder TSL V2 p.16-17)", "Long", Live,
          &["E|\"1. Enter long on the next bar at the market open after price closes 5 percent below the 14-bar SMA.\"", "X|\"2. Exit the long position at the market open after price closes above the 14-bar SMA.\""],
          "One position at a time. The article's 20% stop-loss experiment (with a re-entry filter) is not the system's rule and is not coded. Book: 18-stock AT portfolio 10/1994-10/2004, 8% of equity per trade.",
          c("n/a", "70.69%", "", "", "AT 18-stock portfolio 10/1994-10/2004: +125.03% (8.46%/yr)")),
        d("atmmisc_kt_opp", "KT Opportunity System · %K(10..30) < 5 and index CMO(14) < -40, 4% target", "TSL Jun 2006 (binder TSL V2 p.19-21)", "Long", Live,
          &["F|\"1. %K line based on time intervals 10, 15, 20, 25, and 30 bars (trading days) must all be below 5. 2. Determine the index or ETF that represents the stock(s) you are trading ... 3. The 14-day Chande Momentum Oscillator (CMO) of the index or ETF must be below -40 (oversold). 4. The stock's closing price must be above $5.\"",
            "E|Buy at the next bar's open when rules 1-4 hold [B: the rules list no order; Trading System Lab convention 'next bar at market']",
            "X|\"1. Starting with the first trading day after the position is established, place a 4-percent profit-target limit order. 2. If any of the 10-, 15-, 20-, 25-, or 30-bar %K lines close above 70, exit the position at market at the beginning of the next trading day.\""],
          "%K = 100 x (close - lowest low) / (highest high - lowest low). Index = the same instrument when trading an index (instrument swap); the $5 rule is a stock-price filter (always true on the indices). Book: Nasdaq-100 stocks 2/1996-2/2006 with the Nasdaq Composite.",
          c("n/a", "~80%", "", "5-6 days", "Nasdaq 100 stocks 2/1996-2/2006: +239.98%, PF 2.18, max DD -34.38%")),
        d("atmmisc_antitrend_ema", "Anti-Trend EMA · buy EMA(7) crosses below EMA(10) x 1.00/0.99/0.98/0.97", "TSL Nov 2006 (binder TSL V2 p.24-26)", "Long", NotTestable("the printed exit (EMA(7) above EMA(10) on the bar before the last) fires on every entry's own close; reading it as the last bar only matches the published results, which is fitting, not the column's rule"),
          &["E|\"Go long the next day at the close if the EMA(7) of the last bar is lower than the EMA(10) of the last bar and the EMA(7) of the previous bar is higher than the EMA(10) of the previous bar\"; the same with EMA(10) multiplied by 0.99, 0.98 and 0.97 (\"a maximum of four positions for every basic trade signal\")",
            "X|\"Exit all long positions next day at the close if the EMA(7) of the previous bar is higher than the EMA(10) of the previous bar.\""],
          "Reading [C]: the exit is taken when EMA(7) > EMA(10) on the last completed bar; read literally ('previous bar' = the bar before it) every entry would be exited at its own entry close, which cannot produce the published ~67% winners / +1.0% average. Each multiplier's cross adds one leg (weight p.w = 0.10, book 10% per position), at most four open. EMA = Wealth-Lab EMA (SMA seed) [B]. Book: 17-stock AT portfolio 8/1996-7/2006, +211%, max DD -29.61%, ~67% winners.",
          c("n/a", "~67%", "+1.0%", "", "AT 17-stock portfolio 8/1/1996-7/31/2006: +211%")),
        d("atmmisc_hlr", "Countertrend HLR · buy HLR(10) < 0.3, hold 10 days", "TSL Sep 2006 (binder TSL V2 p.22-23)", "Long",
          NotTestable("the HLR indicator ('based on 10-day highs and lows') is defined only in a Futures System Lab column that is not in the binder; its formula is not given"),
          &["E|\"1. Go long tomorrow when the HLR indicator falls below 0.3.\"", "X|\"2. Exit long after holding position 10 days.\""], "Not completed.", None),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    let v: &'static str = catalogue().into_iter().find(|d| d.id == id)?.name;
    match id { s if s.starts_with("atmmisc_") => Some(("atmmisc", "Active Trader binders (TSL 2005-2006)", v)), _ => None }
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    Some(match id {
        "atmmisc_sma_contra" => vec![p("n", "SMA length", 14., 5., 50., 1.), p("pct", "Close % below the SMA", 5., 1., 15., 0.5)],
        "atmmisc_kt_opp" => vec![p("k_lo", "All %K below", 5., 1., 30., 1.), p("cmo_n", "Index CMO length", 14., 5., 30., 1.), p("cmo_lvl", "Index CMO below", -40., -80., 0., 5.),
            p("tgt", "Profit target %", 4., 1., 15., 0.5), p("k_hi", "Exit when any %K closes above", 70., 50., 95., 1.), p("min_px", "Minimum close ($)", 5., 0., 50., 1.)],
        "atmmisc_antitrend_ema" => vec![p("fast", "Fast EMA", 7., 2., 20., 1.), p("slow", "Slow EMA", 10., 3., 40., 1.), p("w", "Weight per position (fraction of equity)", 0.10, 0.01, 1., 0.01)],
        "atmmisc_hlr" => vec![],
        _ => return None,
    })
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    let schema = params(id)?;
    let n = x.r.len();
    if !p.keys().all(|k| schema.iter().any(|d| d.key == k)) || from > to || to >= n { return None; }
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).map_or(0., |d| d.def));
    let u = |k: &str| g(k).max(1.) as usize;
    let (o, h, l, c) = (&x.o, &x.h, &x.l, &x.c);
    Some(match id {
        "atmmisc_sma_contra" => {
            let (m, f) = (sma(c, u("n")), 1. - g("pct") / 100.);
            by_open(x, from, to, |i, pos, _| if pos == 0 { if c[i] < m[i] * f { Some(1) } else { None } } else if c[i] > m[i] { Some(0) } else { None })
        }
        "atmmisc_kt_opp" => {
            let ks: Vec<Vec<f64>> = [10usize, 15, 20, 25, 30].iter().map(|&nn| (0..n).map(|i| { let (lo, hi) = (ll(l, i, nn), hh(h, i, nn)); if hi > lo { 100. * (c[i] - lo) / (hi - lo) } else { f64::NAN } }).collect()).collect();
            let cm = cmo(c, u("cmo_n"));
            let (klo, khi, cl, tg, mp) = (g("k_lo"), g("k_hi"), g("cmo_lvl"), 1. + g("tgt") / 100., g("min_px"));
            let (mut out, mut cur) = (vec![], None::<Trade>);
            for j in from.max(1)..=to {
                let i = j - 1;
                if let Some(t) = cur.take() {
                    // market exit at the open (signal on the previous close) first, then the 4% limit from the bar after entry
                    if j > t.legs[0].i && ks.iter().any(|k| k[i] > khi) { out.push(done(t, j, o[j], i, "a %K closed above 70: exit at the open")); }
                    else if j > t.legs[0].i { match lim_up(x, j, t.legs[0].px * tg) { Some(px) => out.push(done(t, j, px, j, "4% profit target")), None => cur = Some(t) } }
                    else { cur = Some(t); }
                    continue;
                }
                if ks.iter().all(|k| k[i] < klo) && cm[i] < cl && c[i] > mp { cur = Some(new_trade(1., j, o[j], "open")); }
            }
            finish(&mut out, cur, x, to);
            out
        }
        "atmmisc_antitrend_ema" => {
            let (f, s, w) = (ema(c, u("fast")), ema(c, u("slow")), g("w"));
            const M: [f64; 4] = [1.00, 0.99, 0.98, 0.97];
            let (mut out, mut cur) = (vec![], None::<Trade>);
            for j in from.max(2)..=to {
                let i = j - 1;
                if s[i - 1].is_nan() { continue; }
                if let Some(t) = cur.take() {
                    if f[i] > s[i] { out.push(done(t, j, c[j], i, "EMA7 above EMA10 on the previous bar: exit at the close")); continue; }
                    cur = Some(t);
                }
                for &m in M.iter() {
                    if f[i] < s[i] * m && f[i - 1] > s[i - 1] * m {
                        match cur.as_mut() {
                            Some(t) => if t.legs.len() < 4 { t.legs.push(Leg { i: j, px: c[j], w }) },
                            None => { let mut t = new_trade(1., j, c[j], "close"); t.legs[0].w = w; cur = Some(t); }
                        }
                    }
                }
            }
            finish(&mut out, cur, x, to);
            out
        }
        "atmmisc_hlr" => vec![],
        _ => return None,
    })
}
