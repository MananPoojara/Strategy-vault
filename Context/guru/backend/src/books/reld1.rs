//! Lane D1 (16 Nov 2026, R4 #339/#340): held-for-intraday methods converted to the held SPX/SPY/NIFTY minute bars or to a
//! stated daily-bar reading. The cards themselves live in their own book modules (clomw.rs ...); this module only holds the
//! lane's reference-dump test (authors/relax-339/D1/w/d1_ref.py, compare d1_cmp.py). No catalogue rows of its own.
use crate::books::n1intra::{minute, Ord};
use crate::connors::{Ctx, ParamDef, Params, StrategyDef, Trade};
use crate::intraday::{day_trade, Bar1};

/// Minute-path cards of the lane: SPX only (S&P points).
pub const SPX_IDS: &[&str] = &["clomw_early_range_breakout", "lpgap_fade_spx"];
/// Minute-path cards of the lane on all four minute tickers (no fixed amount; a tick = #57).
pub const ALL_IDS: &[&str] = &["ptjdt_sp_momentum"];
pub fn catalogue() -> Vec<StrategyDef> { vec![] }
pub fn family_of(_id: &str) -> Option<(&'static str, &'static str, &'static str)> { None }
pub fn params(_id: &str) -> Option<Vec<ParamDef>> { None }
pub fn run(id: &str, x: &Ctx, from: usize, to: usize, _p: &Params) -> Option<Vec<Trade>> {
    if id != "ptjdt_sp_momentum" || to >= x.c.len() || from > to { return None; }
    Some(ptjdt_spm(x, from, to))
}

/// One 20-minute bar of the continuous series: day row, OHLC, its minutes, last bar of its day.
struct B { i: usize, h: f64, l: f64, c: f64, m: Vec<Bar1>, last: bool }
#[derive(Clone, Default)]
struct S { pos: Option<(f64, f64, f64)>, xon: bool, filled: bool, done: Vec<(f64, f64, f64, &'static str)> }

/// PTJ "S&P Momentum Day-Trading System" (20-minute bars): trend up = bar closes above its 20-bar simple MA and RSI(3) < 30 at the
/// bar's close -> buy stop one tick above the previous bar's high on each of the next two bars of the day (re-pegged to the then
/// previous bar), protective stop at the previous bar's low (works from the minute after the fill); trend down mirrored. Exit: the
/// protective stop, else the session's last close (the book leaves the profit exit to the trader: labelled reading).
pub fn ptjdt_spm(x: &Ctx, from: usize, to: usize) -> Vec<Trade> {
    let mut b: Vec<B> = vec![];
    for i in from..=to {
        let Some((_, _, m, v)) = crate::intraday_ep::day_bars(x, i, 20) else { continue };
        let k = v.len();
        b.extend(v.into_iter().enumerate().map(|(j, z)| B { i, h: z.h, l: z.l, c: z.c, m: m[z.a..z.b].to_vec(), last: j + 1 == k }));
    }
    let n = b.len();
    let tick = if n > 0 && x.r[b[0].i].ticker.starts_with("NIFTY") { 0.05 } else { 0.01 };
    let mut rsi = crate::intraday_ep::Rsi::new(3);
    let r3: Vec<Option<f64>> = b.iter().map(|z| rsi.push(z.c)).collect();
    let ma: Vec<f64> = (0..n).map(|k| if k >= 19 { b[k - 19..=k].iter().map(|z| z.c).sum::<f64>() / 20.0 } else { f64::NAN }).collect();
    let (mut out, mut sig, mut min_sig, mut st) = (vec![], vec![0i8; n], 0usize, S::default());
    for k in 0..n {
        let day = b[k].i;
        let live = |s: i8| k >= 1 && (1..=2).any(|a| k >= a && b[k - a].i == day && sig[k - a] == s && k - a >= min_sig);
        let (lb, ls) = (live(1), live(-1));
        if st.pos.is_some() || lb || ls {
            let (hh, ll, bh, bl) = if k >= 1 { (b[k - 1].h + tick, b[k - 1].l - tick, b[k - 1].h, b[k - 1].l) } else { (0., 0., 0., 0.) };
            let orders = |s: &S| -> Vec<Ord> {
                let mut v = vec![];
                if let Some((d, _, stp)) = s.pos { if s.xon { v.push(Ord { up: d < 0.0, lv: stp, tag: 3 }); } }
                if s.pos.is_none() && !s.filled { if lb { v.push(Ord { up: true, lv: hh, tag: 1 }); } if ls { v.push(Ord { up: false, lv: ll, tag: 2 }); } }
                v
            };
            let fill = |s: &mut S, t: u8, px: f64| match t {
                1 | 2 => { s.pos = Some((if t == 1 { 1.0 } else { -1.0 }, px, if t == 1 { bl } else { bh })); s.filled = true; }
                _ => { if let Some((d, e, _)) = s.pos.take() { s.done.push((d, e, px, "stop")); } }
            };
            st.filled = false;
            for q in &b[k].m {
                st.xon = st.pos.is_some();
                let adv = st.pos.map_or(false, |z| z.0 < 0.0);
                minute(q, &mut st, &orders, &fill, adv);
            }
            if st.filled { min_sig = k; }
        }
        // trades closed in this bar carry their entry day: all entries are same-day
        for (d, e, xp, why) in st.done.drain(..) { out.push(day_trade(d, day, e, day, xp, why, "stop", false)); }
        if b[k].last {
            if let Some((d, e, _)) = st.pos.take() { out.push(day_trade(d, day, e, day, b[k].m[b[k].m.len() - 1].c, "end of day", "stop", false)); }
        } else if st.pos.is_none() && ma[k].is_finite() {
            if let Some(r) = r3[k] { sig[k] = if b[k].c > ma[k] && r < 30.0 { 1 } else if b[k].c < ma[k] && r > 70.0 { -1 } else { 0 }; }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    /// The reference comparison (run by hand): VAULT_US=<us_data.csv> VAULT_IDX=<idx_data.csv> VAULT_M1_DIR=<data/intraday>
    /// D1_OUT=<csv> cargo test --release d1_dump -- --ignored; then authors/relax-339/D1/w/d1_cmp.py <csv>.
    #[test]
    #[ignore]
    fn d1_dump() {
        use std::io::Write;
        let mut f = std::io::BufWriter::new(std::fs::File::create(std::env::var("D1_OUT").unwrap()).unwrap());
        for (file, t) in [("VAULT_US", "SPY"), ("VAULT_US", "SPX"), ("VAULT_IDX", "NIFTY 50"), ("VAULT_IDX", "NIFTYBANK")] {
            let rows = crate::load_csv(&std::env::var(file).unwrap()).unwrap().remove(t).unwrap();
            let mut x = Ctx::new(&rows, &rows, &[]);
            x.minutes = Some(Box::leak(Box::new(crate::intraday::Minutes::load(&std::env::var("VAULT_M1_DIR").unwrap(), t).unwrap())));
            let from = rows.partition_point(|r| r.date.as_str() < x.minutes.unwrap().first());
            let ids = std::env::var("D1_IDS").map(|s| s.split(',').map(str::to_string).collect::<Vec<_>>()).unwrap_or_else(|_| SPX_IDS.iter().map(|s| s.to_string()).collect());
            for id in ids {
                if SPX_IDS.contains(&id.as_str()) && t != "SPX" { continue; }
                for z in crate::books::run(&id, &x, from, rows.len() - 1, &Params::new()).unwrap() {
                    writeln!(f, "{id},{t},{},{},{},{},{},{},{}", z.dir, rows[z.legs[0].i].date, z.legs[0].px, rows[z.exit_i].date, z.exit_px, z.reason, z.open).unwrap();
                }
            }
        }
    }
}
