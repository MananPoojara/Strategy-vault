//! Kishore M., Instant Forex Profits Home Study Course (Powerup Capital), "Instant Pip Profit" Bollinger reversal (research share Trading
//! Knowledge I/Kishore M. - Instant Forex Profits Home Study Course (ifxprofits.com)/Course Materials/SPOT FX Trading Strategies.pdf, pp. 2-4;
//! Bollinger 20 / 2 standard deviations from Course Materials/FX Course.pdf p. 61). Book id `kmifx`. Lane K2 (23 Nov 2026, queue R4 #381).
//! The slides say the method works on any time frame; here it runs on the NSE index daily bars as a labelled #340 adaptation, UNVERIFIED.
use crate::connors::{Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "kmifx";
const ID: &str = "kmifx_bb";

const NOTE: &str = "A|Declared readings (lane K2), unstated card-specific choices (10, each one choice, #400): (1) the strategy prints no Bollinger setting, so the course's 20-day average and 2 standard deviations are used (FX Course p. 61); (2) the deviation is the population deviation of the close; (3) 'hit or come very close' is read as the low (high for shorts) touching the band of that bar; (4) a white (black) candle is close above (below) open; (5) the optional RSI, CCI, Stochastic and Parabolic SAR confirmations are dropped; (6) the exit is a touch of the opposite band as of the previous close (a gap through it fills at the open); (7) the stop is the first candle's extreme or 100 pips from the entry price, whichever is farther (the text's 'whichever is lower'); (8) the stop is live from the entry bar; (9) the stop is taken first when a bar holds both; (10) one position at a time. Honest total: 10 card-specific choices. Engine-wide defaults / declared adaptations (labelled, not counted, R4 #404): ONE currency scaling rule, 1 pip = 0.01% of the price (a pair near 1.00), so the 100-pip stop is 1% of the entry price (point-to-percent scaling); NSE index series in place of the spot pair (declared adaptation); no costs.";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef { id: ID, name: "Instant Pip Profit: Bollinger-band reversal", group: "Instant Forex Profits (Kishore M.)", session: 0, dir: "Long/Short", status: Status::Live,
        needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK,
        rules: &["E|\"For Buy To Enter (UPTREND) 1. Look for the currency pair to hit or come very close to hitting the lower Bollinger. 2. Wait for the next candle to make sure it is a white candle which indicates bullishness ... go long at the open of the third candle.\" Sell: the upper band, a black candle, short at the open of the third candle (p. 4)",
            "X|\"Exit when it reaches the upper Bollinger.\" (shorts: the lower band)",
            "X|\"The initial stop is placed at the first candle's low OR 100 Pips below the buy entry price whichever is lower.\" (shorts: the first candle's high or 100 pips above, whichever is higher)", NOTE],
        notes: "Kishore M., SPOT FX Trading Strategies pp. 2-4; chart examples only, no result printed.", claim: None }]
}
pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> { (id == ID).then_some(("kmifx", "Bollinger reversal", "")) }
pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    (id == ID).then(|| vec![ParamDef { key: "n", label: "Bollinger length (days)", def: 20., min: 10., max: 50., step: 1. }, ParamDef { key: "sd", label: "Standard deviations", def: 2., min: 1., max: 3., step: 0.5 },
        ParamDef { key: "stop_pct", label: "Stop distance (% of entry; 100 pips)", def: 1., min: 0.25, max: 3., step: 0.25 }])
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if id != ID || to >= x.c.len() || from >= to { return None; }
    let n = p.get("n").copied().unwrap_or(20.0).round().max(2.0) as usize;
    let k = p.get("sd").copied().unwrap_or(2.0);
    let sp = p.get("stop_pct").copied().unwrap_or(1.0) / 100.0;
    let len = x.c.len();
    let (mut up, mut lo) = (vec![f64::NAN; len], vec![f64::NAN; len]);
    for i in n - 1..len {
        let w = &x.c[i + 1 - n..=i];
        let m = w.iter().sum::<f64>() / n as f64;
        let sd = (w.iter().map(|v| (v - m) * (v - m)).sum::<f64>() / n as f64).sqrt();
        up[i] = m + k * sd; lo[i] = m - k * sd;
    }
    let mut out: Vec<Trade> = vec![];
    let mut pos: Option<(f64, usize, f64, f64, f64)> = None; // dir, entry bar, entry px, stop, target
    let leg = |ei: usize, ep: f64| vec![Leg { i: ei, px: ep, w: 1.0 }];
    for d in from.max(n + 1)..=to {
        if pos.is_none() {
            let (c1, c2) = (d - 2, d - 1);
            if x.l[c1] <= lo[c1] && x.c[c2] > x.o[c2] {
                let ep = x.o[d]; pos = Some((1.0, d, ep, x.l[c1].min(ep * (1.0 - sp)), up[d - 1]));
            } else if x.h[c1] >= up[c1] && x.c[c2] < x.o[c2] {
                let ep = x.o[d]; pos = Some((-1.0, d, ep, x.h[c1].max(ep * (1.0 + sp)), lo[d - 1]));
            }
        }
        if let Some((dir, ei, ep, stop, tgt)) = pos {
            let (hit_stop, hit_tgt) = if dir > 0.0 { (x.l[d] <= stop, x.h[d] >= tgt) } else { (x.h[d] >= stop, x.l[d] <= tgt) };
            let ex = if hit_stop { Some((if dir > 0.0 { x.o[d].min(stop) } else { x.o[d].max(stop) }, "stop")) }
                else if hit_tgt { Some((if dir > 0.0 { x.o[d].max(tgt) } else { x.o[d].min(tgt) }, "opposite Bollinger band")) } else { None };
            if let Some((px, why)) = ex {
                out.push(Trade { dir, legs: leg(ei, ep), exit_i: d, exit_px: px, reason: why.into(), entry_kind: "open", signal_i: ei, open: false, exits: Vec::new() });
                pos = None;
            }
        }
    }
    if let Some((dir, ei, ep, _, _)) = pos { out.push(Trade { dir, legs: leg(ei, ep), exit_i: to, exit_px: x.c[to], reason: "open".into(), entry_kind: "open", signal_i: ei, open: true, exits: Vec::new() }); }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn sane_runs() { super::super::nijat::tests::sane(run, &[ID]); }
    /// hand-run dump for the pandas reference: KMIFX_CSV=<idx_data.csv> KMIFX_OUT=<file> cargo test --release kmifx_dump -- --ignored
    #[test]
    #[ignore]
    fn kmifx_dump() {
        use std::io::Write;
        let data = crate::load_csv(&std::env::var("KMIFX_CSV").unwrap()).unwrap();
        let mut f = std::io::BufWriter::new(std::fs::File::create(std::env::var("KMIFX_OUT").unwrap()).unwrap());
        writeln!(f, "id,ticker,dir,entry_date,entry_px,exit_date,exit_px,open").unwrap();
        for (t, rows) in data.iter() {
            let x = Ctx::new(rows, &[], &[]);
            for z in run(ID, &x, 0, rows.len() - 1, &Params::new()).unwrap() {
                writeln!(f, "{ID},{t},{},{},{},{},{},{}", z.dir, rows[z.legs[0].i].date, z.legs[0].px, rows[z.exit_i].date, z.exit_px, z.open).unwrap();
            }
        }
    }
}

pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book == "kmifx",
    label: "Daily Bollinger-band reversal; the 100-pip stop is a declared percentage of the entry price (1 pip = 0.01% of the price); run and independently checked on the five NSE index series only (NIFTY 50, NIFTYBANK, NIFTYIT, NIFTYMIDCAP100, NIFTYMIDCAPSELECT).",
    basis: "Instant Forex Profits (Kishore M.), SPOT FX Trading Strategies pp. 2-4; independent check authors/conv-1123/k2w/kmifx_ref.py",
};
