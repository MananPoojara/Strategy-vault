//! Robert B. Hanna, *The Hanna ETF Money Flow System* (TradingMarkets / Connors Group, January 2005). Book id `rbhetf`.
//! Source: research drive "Trading Knowledge II/@TradingMarkets.com & TradersLibrary.com/Robert B.Hanna - The Hanna ETF
//! Money Flow System/Hanna ETF 15th January 2005.pdf" (text PDF, read in place; copy refused - COPY PENDING, see
//! library/rbhetf/README.md). Page refs are the manual's printed pages.
//! Lane K3 (24 Nov 2026, Q399 (a), R4 #392): the four scenarios run on the two-leg spread account (crate::spread) on the five NSE indices
//! the vault holds, with India VIX as the VXO: a declared India adaptation of the book's eight US indices. Gate allowed = NIFTY 50.
//! The four scenarios are spread trades between eight indices chosen by their ReIS rank each day, scaled in over up to
//! three VXO triggers. The VXO exists in extra_data (VXO_OHLC), but the system also needs the PHLX Semiconductor,
//! AMEX Pharmaceutical, AMEX Biotechnology, Russell 2000 and S&P 400 index histories from 1995 and a multi-instrument
//! spread engine that picks the instrument by cross-sectional rank; the vault runs one instrument at a time.
use crate::connors::{claim, Ctx, ParamDef, Params, Status, StrategyDef, Trade};
use crate::options::Pos;
use crate::spread::{self, Leg};

pub const BOOK: &str = "rbhetf";

#[allow(dead_code)]
const GAP: &str = "needs daily closes of all eight indices of p.14 (S&P 500, Russell 2000, Nasdaq 100, DJIA, PHLX Semiconductor SOX, AMEX Pharmaceutical DRG, AMEX Biotechnology BTK, S&P 400 Midcap; 1995-) to rank ReIS each day, plus a multi-leg long/short engine; the vault has SPX/NDX/DJI only (IWM/SMH/MDY ETFs start 1993-2000; SOX, DRG and BTK are not in the vault) and runs one instrument at a time. VXO OHLC itself is available (extra_data/us/VXO_OHLC, 1986-2021)";
const NOTE: &str = "[Adapted: India, R4 #340; 4 labelled readings; unverified] The book tested eight US indices (S&P 500, Russell 2000, Nasdaq 100, DJIA, SOX, DRG, BTK, S&P 400) with the VXO; the vault holds five NSE indices and India VIX, so the run is on NIFTY 50 (the S&P 500: 200-day average, the Measured Period and the SPY leg), ranking NIFTY 50, NIFTY BANK, NIFTY IT, NIFTY MIDCAP 100 and NIFTY MIDCAP SELECT, with India VIX as the VXO. Readings: (1) the 10-day average of the VIX closes includes today; the low-VIX test is close <= 0.90 x average or intraday low <= 0.88 x average (the high test mirrored); (2) one third of the portfolio per valid trigger: every later trigger day while the position is open and fewer than three tranches are held adds a tranche, each tranche ranks ReIS afresh on its own day, all tranches exit together; (3) a 3-day look-back that is exactly zero continues the current leg, and a tranche whose lowest-ReIS index is NIFTY 50 itself nets to zero; (4) signals are taken and filled at the close. Conventions: only dates where all five indices and India VIX have a bar count; ties in ReIS go to the first index in the list; a trade's return is the portfolio return, each tranche on its own gross exposure times 1/3. Book method: ReIS (p.16-17): Measured Period starts at 3 on the day the S&P 500's close vs its close 3 days earlier changes sign (upleg/downleg) and grows by 1 each day the leg continues; ReIS = % change of each index over the Measured Period. Scale-in: 1/3 of the portfolio per valid VXO trigger, up to three (p.24-25). Book test: indices (not ETFs), 1/1/1996-6/14/2004: 349 triggers, 65.9% profitable, average +1.24%, max drawdown -8.9%, 17.8% compound annual gain 1/1/95-6/14/04 (p.4, 47-50).";

macro_rules! nd {
    ($id:expr, $name:expr, $dir:expr, $rules:expr, $claim:expr) => {
        StrategyDef { id: $id, name: $name, group: "Part 2 · System trading rules (p.19-25)", session: 0, dir: $dir, status: Status::Live,
            needs_volume: false, needs_vix: true, needs_series: &[], book: BOOK, rules: $rules, notes: NOTE, claim: $claim }
    };
}

pub fn catalogue() -> Vec<StrategyDef> {
    vec![
        nd!("rbhetf_s1", "Scenario #1: low VXO stretch, S&P 500 below its 200-day SMA", "Long/Short",
            &["F|\"If S&P 500 is below its 200 day simple moving average\" (p.19)",
              "E|\"The VXO must close at least 10% below its 10-day moving average or trade at least 12% below its 10-day moving average intraday.\"",
              "E|\"Short one unit of the SPY's and buy one unit of ETF with the lowest ReIS rating.\"",
              "X|\"Exit both positions when the VXO closes above its 10-day moving average.\""],
            claim("43", "69.77%", "+1.26%", "", "Scenario #1, 1/1/1996-6/14/2004, 3-tier scale-in (p.51)")),
        nd!("rbhetf_s2", "Scenario #2: high VXO stretch, S&P 500 below its 200-day SMA", "Long/Short",
            &["F|\"If S&P 500 is below its 200 day simple moving average\" (p.19)",
              "E|\"The VXO must close at least 10% above its 10-day moving average or trade at least 12% above its 10-day moving average intraday.\"",
              "E|\"Short one unit of the ETF with the highest ReIS rating and buy one unit of the ETF with the lowest ReIS rating.\"",
              "X|\"Exit when VXO closes below its 10-day moving average.\""],
            claim("80", "57.50%", "+0.95%", "", "Scenario #2, 1/1/1996-6/14/2004 (p.51)")),
        nd!("rbhetf_s3", "Scenario #3: low VXO stretch, S&P 500 above its 200-day SMA", "Long/Short",
            &["F|\"If S&P 500 is above its 200 day simple moving average\" (p.19)",
              "E|\"The VXO must close at least 10% below its 10-day moving average or trade at least 12% below its 10-day moving average intraday.\"",
              "E|\"Short one unit of the SPY's and buy one unit of the ETF with the lowest ReIS rating.\"",
              "X|\"Exit both positions when the VXO closes above its 10-day moving average.\""],
            claim("84", "55.95%", "+0.34%", "", "Scenario #3, 1/1/1996-6/14/2004 (p.52)")),
        nd!("rbhetf_s4", "Scenario #4: high VXO stretch, S&P 500 above its 200-day SMA", "Long",
            &["F|\"If S&P 500 is above its 200 day simple moving average\" (p.19)",
              "E|\"The VXO must close at least 10% above its 10-day moving average or trade at least 12% above its 10-day moving average intraday.\"",
              "E|\"Buy 2 units of the ETF with the lowest ReIS rating.\"",
              "X|\"Exit when the VXO closes below its 10-day moving average.\""],
            claim("142", "75.35%", "+1.92%", "", "Scenario #4, 1/1/1996-6/14/2004 (p.52)")),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    Some(match id {
        "rbhetf_s1" => ("rbhetf_money_flow", "Hanna ETF Money Flow", "Scenario 1"),
        "rbhetf_s2" => ("rbhetf_money_flow", "Hanna ETF Money Flow", "Scenario 2"),
        "rbhetf_s3" => ("rbhetf_money_flow", "Hanna ETF Money Flow", "Scenario 3"),
        "rbhetf_s4" => ("rbhetf_money_flow", "Hanna ETF Money Flow", "Scenario 4"),
        _ => return None,
    })
}
pub const IDS: &[&str] = &["rbhetf_s1", "rbhetf_s2", "rbhetf_s3", "rbhetf_s4"];
pub fn is_spread(id: &str) -> bool { IDS.contains(&id) }
/// The NSE indices ranked by ReIS (the first is the S&P 500 / SPY stand-in, the run's own ticker); the others are loaded as `IDX.<ticker>.Close` series.
pub const UNIVERSE: &[&str] = &["NIFTY 50", "NIFTYBANK", "NIFTYIT", "NIFTYMIDCAP100", "NIFTYMIDCAPSELECT"];
pub fn idx_key(t: &str) -> String { format!("IDX.{t}.Close") }

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    is_spread(id).then(|| vec![p("tiers", "Scale-in tranches (the book: 3)", 3., 1., 5., 1.), p("lookback", "Look-back days for the Measured Period (the book: 3)", 3., 1., 10., 1.)])
}
pub fn run(_id: &str, _x: &Ctx, _from: usize, _to: usize, _p: &Params) -> Option<Vec<Trade>> { None }

/// Spread positions of the four scenarios (None = not one of them; Err = not on this context or data missing). One position per
/// episode: its P&L is the sum of the tranches' returns (each on its own gross exposure) times 1/tiers; s0 = 1.
pub fn positions(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Result<Vec<Pos>, String>> {
    if !is_spread(id) { return None; }
    Some(episodes(id, x, from, to, p))
}

fn episodes(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Result<Vec<Pos>, String> {
    let miss = || "needs_data: the NSE index closes and India VIX of the Hanna basket are not loaded on this context (run it on NIFTY 50)".to_string();
    if x.r.first().map_or(true, |r| r.ticker != UNIVERSE[0]) { return Err("not_testable: the Hanna basket runs only on the NIFTY 50 calendar (NIFTY 50 stands for the S&P 500)".into()); }
    let schema = params(id).ok_or_else(miss)?;
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).map_or(0., |d| d.def));
    let (tiers, lb) = (g("tiers").round().max(1.) as usize, g("lookback").round().max(1.) as usize);
    let mut cl: Vec<&[f64]> = vec![&x.c[..]];
    for t in &UNIVERSE[1..] { cl.push(x.series(&idx_key(t)).ok_or_else(miss)?); }
    let (vc, vh, vl) = (x.series("VIX.Close").ok_or_else(miss)?, x.series("VIX.High").ok_or_else(miss)?, x.series("VIX.Low").ok_or_else(miss)?);
    let mut all: Vec<&[f64]> = cl.clone();
    all.extend([vc, vh, vl]);
    let j = spread::joint(&all);
    let m = j.len();
    let sma200 = |t: usize| (t >= 199).then(|| j[t - 199..=t].iter().map(|&b| x.c[b]).sum::<f64>() / 200.);
    let vma = |t: usize| (t >= 9).then(|| j[t - 9..=t].iter().map(|&b| vc[b]).sum::<f64>() / 10.);
    let mut mp = vec![0usize; m];
    let mut up = true;
    for t in lb..m {
        let (a, b0) = (x.c[j[t]], x.c[j[t - lb]]);
        let new_up = if a > b0 { true } else if a < b0 { false } else { up };
        mp[t] = if t == lb || new_up != up { 3 } else { mp[t - 1] + 1 };
        up = new_up;
    }
    let low = id == "rbhetf_s1" || id == "rbhetf_s3";
    let below = id == "rbhetf_s1" || id == "rbhetf_s2";
    let reis = |t: usize| -> Option<Vec<f64>> { let k = mp[t]; (k >= 1 && t >= k).then(|| cl.iter().map(|c| c[j[t]] / c[j[t - k]] - 1.).collect()) };
    let pick = |v: &[f64], hi: bool| -> usize { let mut b = 0; for (i, &z) in v.iter().enumerate() { if (hi && z > v[b]) || (!hi && z < v[b]) { b = i; } } b };
    let mut out: Vec<Pos> = vec![];
    let mut legs_of: Vec<(usize, Vec<(usize, f64)>)> = vec![]; // (entry bar, [(universe index, qty)])
    let finish = |legs_of: &mut Vec<(usize, Vec<(usize, f64)>)>, x_i: usize, reason: String, open: bool, out: &mut Vec<Pos>| {
        let e0 = legs_of[0].0;
        let mut pnl = vec![0.; x_i - e0 + 1];
        for (e, lg) in legs_of.iter() {
            let legs: Vec<Leg> = lg.iter().map(|&(u, q)| Leg { qty: q, px: cl[u], base: cl[u], mult: 1. }).collect();
            let Some(ps) = spread::position(&legs, 1., *e, x_i, String::new(), false) else { legs_of.clear(); return; };
            for k in *e..=x_i { pnl[k - e0] += ps.pnl[k - *e] / ps.s0 / tiers as f64; }
        }
        out.push(Pos { dir: 1., s0: 1., entry_i: e0, exit_i: x_i, pnl, reason, open });
        legs_of.clear();
    };
    let mut last_b = 0;
    for t in 0..m {
        let b = j[t];
        if b < from || b > to { continue; }
        last_b = b;
        let Some(ma) = vma(t) else { continue };
        if !legs_of.is_empty() {
            let out_now = if low { vc[b] > ma } else { vc[b] < ma };
            if out_now { finish(&mut legs_of, b, format!("India VIX closed {} its 10-day average", if low { "above" } else { "below" }), false, &mut out); continue; }
        }
        let trig = if low { vc[b] <= 0.90 * ma || vl[b] <= 0.88 * ma } else { vc[b] >= 1.10 * ma || vh[b] >= 1.12 * ma };
        let Some(s) = sma200(t) else { continue };
        let regime = if below { x.c[b] < s } else { x.c[b] > s };
        if trig && regime && legs_of.len() < tiers {
            if let Some(r) = reis(t) {
                let (lo, hi) = (pick(&r, false), pick(&r, true));
                let lg = match id { "rbhetf_s1" | "rbhetf_s3" => vec![(0, -1.), (lo, 1.)], "rbhetf_s2" => vec![(hi, -1.), (lo, 1.)], _ => vec![(lo, 2.)] };
                legs_of.push((b, lg));
            }
        }
    }
    if !legs_of.is_empty() && last_b > legs_of[0].0 { finish(&mut legs_of, last_b, "open".into(), true, &mut out); }
    Ok(out)
}
