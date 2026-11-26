//! TradingMarkets Chairman's Club, "Raptor 3 System Rules" (Raptor3 rules.mdi, six pages of EMF text: Strategy One Power10,
//! Strategy Two SellOff, Strategy Three LLLH, Strategy Four Raptor II Modified, and the "Raptor III Aggressive" order rule; the
//! workbook Raptor3-2007Q3.3s.xls next to it was saved by Laurens Bensdorp, 5 Dec 2007). Book id "prap3". Research share:
//! Trading Knowledge II/@TradingMarkets.com & TradersLibrary.com/Raptor 3 System Rules & Super System SpreadSheet (Chairman's Club System)/.
//! Lane K3 (23 Nov 2026). Independent check authors/conv-1123/k3w/prap3_ref.py. A long-only stock method; the printed rules are
//! per-stock setups plus a 5-slot, highest-HV portfolio layer. The vault runs one NSE stock per run (Liquidity Filter benchmark =
//! NIFTY 50 close x volume, as books::prap2); the slot layer needs a stock universe and is not coded.
#![allow(dead_code)]
use crate::connors::*;

pub const BOOK: &str = "prap3";
pub const REVIEWED: &[&str] = &["prap3_power10", "prap3_selloff", "prap3_lllh", "prap3_raptor2mod"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

macro_rules! common { () => { "Unstated choices (labelled, one independent choice per number; each card continues the list with its own choices and gives its honest total): (1) the Liquidity Filter 'greater than 3' = 1,000,000 x MA20(close x volume) / MA65(benchmark close x volume), the Raptor-family definition of books::prap2; (2) a limit buy is live for the next session only; (3) a limit buy fills only when that session's low is strictly below the limit; (4) the fill price is min(open, limit); (5) the exit condition is tested at every close from the fill day's own close; " }; }

const POWER: &[&str] = &[
    "F|\"1. The stock's close is above the 200-day moving average. 2. The stock's Liquidity Filter is greater than 3. 3. The 10-period ADX is greater than 20. 4. The 100 period Historical Volatility reading is greater than 20.\" (Strategy One - Power10 Rules)",
    "F|\"5. Today's 2 period RSI is less than 10. 6. The stock closes down 3 days in a row. 7. The stock has lost 5% or more in the last 3 days.\"",
    "E|\"8. Buy on a limit order the next day below yesterday's low. Limit Price = 15% - how much stock has lost in percentage terms over the last 3 days. If Limit Price < 3%, then it is 3%.\" (the worked example says '4% below yesterdays close' for an 11% loss, the last sentence '3% below yesterday's low': the reference price is labelled, p.ref_low)",
    "X|\"10. Exit: When the stock closes above its 5 period simple moving average, exit on tomorrow's open.\"",
];
const SELLOFF: &[&str] = &[
    "F|\"1. The close is above the 200-day moving average. 2. The Liquidity Filter is greater than 3. 3. The 10 period ADX is greater than 20. 4. The 100 period HV is greater than 20.\" (Strategy Two - SellOff Rules)",
    "F|\"5. The close of the stock has been down 2 or more days in a row. 6. Today's 2 period RSI is less than 20. 7. Today's low is 3% or more below yesterday's close.\"",
    "E|\"8. Tomorrow buy on a limit 7% below today's low.\"",
    "X|\"The exit is a close which is greater than the 5 day simple moving average. When this occurs, sell on the opening the next day.\"",
];
const LLLH: &[&str] = &[
    "F|\"1. Close is above the 200-day moving average. 2. The Liquidity Filter greater than 3. 3. The stock has made lower highs at least 2 of the last 3 days. 4. The stock has made lower lows at least 2 of the last 3 days.\" (Strategy Three - LLLH Rules)",
    "F|\"5. Today's 2 period RSI is less than 10. 6. Today's high is lower than yesterday's high. 7. Today's low is lower than yesterday's low. 8. The 10 period ADX greater than 35. 9. The 100 period HV greater than 25.\"",
    "E|\"10. Buy on a limit order 3% below today's close, for tomorrow's session.\"",
    "X|\"12. Sell on the next open when the 2 period RSI closes greater than 65.\"",
];
const MOD: &[&str] = &[
    "F|\"1. The close of the stock is above the 200-day moving average. 2. The Liquidity Filter is greater than 3. 3. The close is below the 10-day moving average. 4. Today's 2 period RSI is less than 15. 5. The 10 period ADX greater than 20. 6. The 100 period HV greater than 20.\" (Strategy Four - Raptor II Modified Rules)",
    "F|\"7. The stock drops 15% in last 5 days or 7.5% in the last 2 days.\"",
    "E|\"8. Tomorrow (only) we buy on a limit order 8% below today's close.\"",
    "X|\"10. Sell when the 2-period RSI of the stock is greater than 50. Once this condition is met, we'll sell at the open the next day.\"",
];

pub fn catalogue() -> Vec<StrategyDef> {
    use Status::*;
    let d = |id, name, rules, notes| StrategyDef { id, name, group: "Raptor 3 (Chairman's Club, 2007) · stocks", session: 0, dir: "Long", status: Live,
        needs_volume: true, needs_vix: false, needs_series: &[], book: BOOK, rules, notes, claim: None };
    vec![
        d("prap3_power10", "Raptor 3 Strategy One - Power10 (RSI(2) < 10, 3 down closes, 3-day loss >= 5%, 3-15% limit)", POWER,
          concat!("[Printed rules; unverified] ", common!(), "(6) 'closes down 3 days in a row' = three closes each below the one before; (7) 'lost 5% or more in the last 3 days' = 1 - close / close 3 bars earlier; (8) the limit % = max(15 - the 3-day loss in %, 3) is taken below yesterday's low (the rule's wording; the worked example says below the close, p.ref_low = 0); Honest total: 8 card-specific choices. Audit count 8: its HV(100), ADX, RSI and one-position items moved to engine defaults per #404; its single limit-order item is counted as three (live next session only, fill trigger, fill price) and its limit-reference item and the two card readings (3 down closes, 3-day loss) are counted separately. Engine-wide defaults / declared adaptations (labelled, not counted, R4 #404): the NIFTY 50 close x volume as the Liquidity Filter benchmark (a labelled swap for the S&P 500 x NYSE volume), HV(100) = population st.dev of 100 ln returns x sqrt(252) x 100, Wilder ADX(10) and RSI(2) (the shared engine implementations, the family's AmiBroker definitions), simple moving averages, one position per stock at a time, the sale at the next open after the exit signal, the data window. Exit: first close above the 5-day SMA. ")),
        d("prap3_selloff", "Raptor 3 Strategy Two - SellOff (2 down closes, RSI(2) < 20, low 3% under the prior close, limit 7% under the low)", SELLOFF,
          concat!("[Printed rules; unverified] ", common!(), "(6) 'down 2 or more days in a row' = at least two closes each below the one before; (7) '3% or more below yesterday's close' = today's low against yesterday's close; Honest total: 7 card-specific choices. Audit count 7: its HV(100) and ADX/RSI items moved to engine defaults per #404; its fill item is counted as two (fill trigger, fill price) and its bundled last item as two (the 2-down-closes reading, the low-versus-close reading). Engine-wide defaults / declared adaptations (labelled, not counted, R4 #404): the NIFTY 50 close x volume as the Liquidity Filter benchmark (a labelled swap for the S&P 500 x NYSE volume), HV(100) = population st.dev of 100 ln returns x sqrt(252) x 100, Wilder ADX(10) and RSI(2) (the shared engine implementations, the family's AmiBroker definitions), simple moving averages, one position per stock at a time, the sale at the next open after the exit signal, the data window. Exit: first close above the 5-day SMA. ")),
        d("prap3_lllh", "Raptor 3 Strategy Three - LLLH (lower highs and lows, RSI(2) < 10, ADX(10) > 35, limit 3% under the close)", LLLH,
          concat!("[Printed rules; unverified] ", common!(), "(6) 'lower highs (lows) at least 2 of the last 3 days' counts the three bars ending today, each against its own previous bar; Honest total: 6 card-specific choices. Audit count 7: its HV(100), ADX/RSI and one-position items moved to engine defaults per #404; its single limit-order item is counted as three (live next session only, fill trigger, fill price). Engine-wide defaults / declared adaptations (labelled, not counted, R4 #404): the NIFTY 50 close x volume as the Liquidity Filter benchmark (a labelled swap for the S&P 500 x NYSE volume), HV(100) = population st.dev of 100 ln returns x sqrt(252) x 100, Wilder ADX(10) and RSI(2) (the shared engine implementations, the family's AmiBroker definitions), simple moving averages, one position per stock at a time, the sale at the next open after the exit signal, the data window. Exit: RSI(2) closes above 65. ")),
        d("prap3_raptor2mod", "Raptor 3 Strategy Four - Raptor II Modified (RSI(2) < 15, 15% / 5 days or 7.5% / 2 days, limit 8% under the close)", MOD,
          concat!("[Printed rules; unverified] ", common!(), "(6) 'drops 15% in the last 5 days or 7.5% in the last 2' = today's close at least 15% (7.5%) under the close 5 (2) bars earlier; Honest total: 6 card-specific choices. Audit count 7: its HV(100), ADX/RSI and one-position items moved to engine defaults per #404; its single limit-order item is counted as three (live next session only, fill trigger, fill price). Engine-wide defaults / declared adaptations (labelled, not counted, R4 #404): the NIFTY 50 close x volume as the Liquidity Filter benchmark (a labelled swap for the S&P 500 x NYSE volume), HV(100) = population st.dev of 100 ln returns x sqrt(252) x 100, Wilder ADX(10) and RSI(2) (the shared engine implementations, the family's AmiBroker definitions), simple moving averages, one position per stock at a time, the sale at the next open after the exit signal, the data window. Exit: RSI(2) closes above 50. ")),
        StrategyDef { id: "prap3_portfolio", name: "Raptor 3 portfolio (5 slots per strategy, highest HV(100) first; aggressive: 2 x open slots)", group: "Raptor 3 (Chairman's Club, 2007) · stocks", session: 0, dir: "Long",
            status: NeedsData("a stock universe on one equity curve: 'rank the stocks by their historical volatility and put in orders each day only for the stocks with the highest volatility' with at most 5 positions per strategy (20 across the four), and the Aggressive version's 'open slots times 2' orders (up to 10 positions per strategy, possibly on margin); the engine backtests one ticker per run"),
            needs_volume: true, needs_vix: false, needs_series: &[], book: BOOK,
            rules: &["F|\"9. Buy a maximum of 5 Positions. ... you will rank the stocks by their historical volatility and put in orders each day only for the stocks with the highest volatility.\" (Strategy One; the same in each strategy)", "F|\"Raptor III Aggressive Rules ... Place # of slots not used times 2 orders ... You'll potentially have 10 positions in any given system\""],
            notes: "Portfolio layer of the four strategies.", claim: None },
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    let f = |v| Some(("prap3", "Raptor 3", v));
    match id { "prap3_power10" => f("Power10"), "prap3_selloff" => f("SellOff"), "prap3_lllh" => f("LLLH"), "prap3_raptor2mod" => f("Raptor II Modified"), "prap3_portfolio" => f("Portfolio · needs a universe"), _ => None }
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    let mut v = vec![p("trend", "Close above N-day SMA", 200., 100., 300., 50.), p("lf", "Liquidity Filter above", 3., 0., 40., 1.)];
    match id {
        "prap3_power10" => v.extend([p("adx", "ADX(10) above", 20., 0., 40., 5.), p("hv", "HV(100) above", 20., 0., 60., 5.), p("rsi_lt", "RSI(2) below", 10., 1., 30., 1.),
            p("down", "Closes down in a row at least", 3., 1., 6., 1.), p("loss3", "Lost at least % over the last 3 days", 5., 1., 15., 1.), p("lim_base", "Limit % = this minus the 3-day loss %", 15., 5., 25., 1.),
            p("lim_min", "Limit % at least", 3., 1., 10., 1.), p("ref_low", "Limit taken below yesterday's low (1) or close (0)", 1., 0., 1., 1.), p("exit_ma", "Exit after a close above the N-day SMA", 5., 2., 20., 1.)]),
        "prap3_selloff" => v.extend([p("adx", "ADX(10) above", 20., 0., 40., 5.), p("hv", "HV(100) above", 20., 0., 60., 5.), p("rsi_lt", "RSI(2) below", 20., 1., 30., 1.),
            p("down", "Closes down in a row at least", 2., 1., 6., 1.), p("gap", "Low at least % below yesterday's close", 3., 1., 10., 1.), p("limit", "Buy limit % below today's low", 7., 1., 15., 1.),
            p("exit_ma", "Exit after a close above the N-day SMA", 5., 2., 20., 1.)]),
        "prap3_lllh" => v.extend([p("adx", "ADX(10) above", 35., 0., 50., 5.), p("hv", "HV(100) above", 25., 0., 60., 5.), p("rsi_lt", "RSI(2) below", 10., 1., 30., 1.),
            p("limit", "Buy limit % below today's close", 3., 1., 10., 1.), p("exit_rsi", "Exit next open after RSI(2) closes above", 65., 50., 90., 5.)]),
        "prap3_raptor2mod" => v.extend([p("pull_ma", "Close below N-day SMA", 10., 5., 50., 5.), p("adx", "ADX(10) above", 20., 0., 40., 5.), p("hv", "HV(100) above", 20., 0., 60., 5.),
            p("rsi_lt", "RSI(2) below", 15., 1., 30., 1.), p("drop5", "Down at least % over 5 days", 15., 5., 30., 2.5), p("drop2", "OR down at least % over 2 days", 7.5, 2.5, 15., 2.5),
            p("limit", "Buy limit % below today's close", 8., 2., 12., 1.), p("exit_rsi", "Exit next open after RSI(2) closes above", 50., 40., 90., 5.)]),
        _ => return None,
    }
    Some(v)
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) { return None; }
    let schema = params(id)?;
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).map_or(f64::NAN, |d| d.def));
    let (c, o, h, l, n) = (&x.c, &x.o, &x.h, &x.l, x.c.len());
    let to = to.min(n - 1);
    let trend = x.sma_n(g("trend") as usize);
    let hv = hv_ddof(c, 100, 0.);
    let r = &x.rsi2;
    let exit_sma = if id == "prap3_power10" || id == "prap3_selloff" { Some(x.sma_n(g("exit_ma") as usize)) } else { None };
    let pull = if id == "prap3_raptor2mod" { Some(x.sma_n(g("pull_ma") as usize)) } else { None };
    let base = |i: usize| i >= 5 && c[i] > trend[i] && x.lf[i] > g("lf") && x.adx10[i] > g("adx") && hv[i] > g("hv");
    // the limit price of the order placed at the close of bar i, if the setup holds
    let setup = |i: usize| -> Option<f64> {
        if !base(i) { return None; }
        match id {
            "prap3_power10" => {
                let loss = (1. - c[i] / c[i - 3]) * 100.;
                if !(r[i] < g("rsi_lt") && x.down_closes[i] >= g("down") as usize && loss >= g("loss3")) { return None; }
                let pct = (g("lim_base") - loss).max(g("lim_min"));
                Some((if g("ref_low") >= 0.5 { l[i] } else { c[i] }) * (1. - pct / 100.))
            }
            "prap3_selloff" => (x.down_closes[i] >= g("down") as usize && r[i] < g("rsi_lt") && (l[i] / c[i - 1] - 1.) * 100. <= -g("gap")).then(|| l[i] * (1. - g("limit") / 100.)),
            "prap3_lllh" => {
                let k = |f: &dyn Fn(usize) -> bool| (i - 2..=i).filter(|&j| f(j)).count() >= 2;
                (k(&|j| h[j] < h[j - 1]) && k(&|j| l[j] < l[j - 1]) && r[i] < g("rsi_lt") && h[i] < h[i - 1] && l[i] < l[i - 1]).then(|| c[i] * (1. - g("limit") / 100.))
            }
            _ => (c[i] < pull.as_ref().unwrap()[i] && r[i] < g("rsi_lt")
                && ((c[i] / c[i - 5] - 1.) * 100. <= -g("drop5") || (c[i] / c[i - 2] - 1.) * 100. <= -g("drop2"))).then(|| c[i] * (1. - g("limit") / 100.)),
        }
    };
    let exit_now = |i: usize| match &exit_sma { Some(m) => c[i] > m[i], None => r[i] > g("exit_rsi") };
    let (mut out, mut cur, mut order, mut exit): (Vec<Trade>, Option<Trade>, Option<f64>, bool) = (vec![], None, None, false);
    for i in from..=to {
        if exit {
            let mut t = cur.take().unwrap();
            (t.exit_i, t.exit_px) = (i, o[i]);
            out.push(t);
            exit = false;
        }
        if let Some(px) = order.take() {
            if cur.is_none() && l[i] < px {
                cur = Some(Trade { dir: 1., legs: vec![Leg { i, px: o[i].min(px), w: 1. }], exit_i: i, exit_px: f64::NAN, reason: String::new(),
                    entry_kind: "limit", signal_i: i, open: false, exits: vec![] });
            }
        }
        if let Some(t) = cur.as_mut() {
            if i < to && exit_now(i) {
                exit = true;
                t.reason = "exit rule met at the close, sold next open".into();
                t.signal_i = i;
            }
            continue;
        }
        if i < to { order = setup(i); }
    }
    if let Some(mut t) = cur {
        (t.exit_i, t.exit_px, t.reason, t.signal_i, t.open) = (to, c[to], "open".into(), to, true);
        out.push(t);
    }
    Some(out)
}

pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book == BOOK && reviewed(def.id),
    label: "The Raptor 3 rules page describes per-stock setups traded as a 5-slot-per-strategy portfolio on US stocks; runs on NSE stocks only, against the NIFTY 50's close x volume for the Liquidity Filter (a labelled benchmark swap), as a declared application outside the book's own universe. Not run on indices, ETFs or US stocks.",
    basis: "TradingMarkets Chairman's Club, Raptor 3 System Rules (Raptor3 rules.mdi pages 1-4 and 6); independent check authors/conv-1123/k3w/prap3_ref.py",
};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;
    fn bars() -> Vec<Row> {
        let mut px = 50.0;
        let mut r: Vec<Row> = (0..260).map(|i| {
            px *= if i % 3 == 2 { 0.98 } else { 1.03 };
            Row { ticker: "T".into(), date: format!("d{:04}", i), open: px, high: px * 1.01, low: px * 0.99, close: px, volume: Some(1e6) }
        }).collect();
        let mut c = r[259].close;
        for k in 0..4 { c *= 0.94; r.push(Row { date: format!("d{:04}", 260 + k), open: c, high: c, low: c, close: c, ..r[0].clone() }); }
        r
    }
    #[test]
    fn four_strategies_run_and_have_params() {
        let r = bars();
        let x = Ctx::new(&r, &r, &[]);
        for id in REVIEWED { assert!(run(id, &x, 0, r.len() - 1, &Params::new()).is_some()); assert!(params(id).is_some()); }
        assert!(run("prap3_portfolio", &x, 0, r.len() - 1, &Params::new()).is_none());
    }
    #[test]
    fn power10_limit_is_between_3_and_10_percent_under_the_low() {
        // a 3-day loss of 17% puts the floor (3%) in force: fill at 3% under yesterday's low when the open is above it
        let r = bars();
        let x = Ctx::new(&r, &r, &[]);
        let pr = Params::new();
        let t = run("prap3_power10", &x, 0, r.len() - 1, &pr).unwrap();
        // setup at bar 262 (3 down closes, 16.6% lost): limit = 3% under that bar's low (15 - 16.6 < 3); bar 263 opens far below it
        let z = t.iter().find(|z| z.legs[0].i == 263).expect("fill the day after the setup");
        assert!((z.legs[0].px - r[263].open).abs() < 1e-9 && r[263].open < r[262].low * 0.97);
    }

    /// hand-run dump for the pandas reference: PRAP3_STK=<stock csv>[,..] PRAP3_IDX=<idx_data.csv> PRAP3_OUT=<file> cargo test --release prap3_dump -- --ignored
    #[test]
    #[ignore]
    fn prap3_dump() {
        use std::io::Write;
        let idx = crate::load_csv(&std::env::var("PRAP3_IDX").unwrap()).unwrap();
        let bench = &idx["NIFTY 50"];
        let mut f = std::io::BufWriter::new(std::fs::File::create(std::env::var("PRAP3_OUT").unwrap()).unwrap());
        writeln!(f, "id,file,entry_date,entry_px,exit_date,exit_px,open").unwrap();
        for path in std::env::var("PRAP3_STK").unwrap().split(',') {
            let rows: Vec<Row> = std::fs::read_to_string(path).unwrap().lines().skip(1).map(|l| {
                let v: Vec<&str> = l.split(',').collect();
                let n = |i: usize| v[i].parse::<f64>().unwrap();
                Row { ticker: "STK".into(), date: v[0].into(), open: n(1), high: n(2), low: n(3), close: n(4), volume: v[5].parse().ok().filter(|x: &f64| *x > 0.) }
            }).collect();
            let x = Ctx::new(&rows, bench, &[]);
            for id in REVIEWED {
                for z in run(id, &x, 0, rows.len() - 1, &Params::new()).unwrap() {
                    writeln!(f, "{id},{},{},{},{},{},{}", path.rsplit('/').next().unwrap(), rows[z.legs[0].i].date, z.legs[0].px, rows[z.exit_i].date, z.exit_px, z.open).unwrap();
                }
            }
        }
    }
}
