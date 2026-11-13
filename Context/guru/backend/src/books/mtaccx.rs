//! Accelerator Crossover expert advisor (Rob Turcotte 2008, accelerator-fx.com), MetaTrader 4 source. Research share USB HDD 1/
//! META TRADER/MetaTrader/Accelerator Forex (accelerator-fx.com)/Accelerator_Crossover.mq4 (copy authors/batch-j/J1w/accel.mq4).
//! Book id `mtaccx`. Lane PD (A1 pool D, 13 Nov 2026), batch-j J1 to-code row, lead decisions 19, 88, 114; independent check
//! authors/pd/w/pd_ref.py.
use crate::connors::{rsi, Ctx, ParamDef, Params, Status, StrategyDef, Trade};
use super::rrsim::{sma, xavg};
use super::tselhs::Book;

pub const REVIEWED: &[&str] = &["mtaccx_cross"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const NOTE: &str = "[Declared readings] The EA acts once per bar, on the first tick of a new bar (gi_224 != Time[0]), with values of the last closed bar (shift 1) and the bar before it (shift 2); so every order here is decided at a daily close and filled at the next open. Buy: iRSI(9) shift 1 > 50 and shift 2 < 50, EMA(close,3) > SMA(close,13) (shift 1), Stochastic(14,8,5) main > signal (shift 1); sell short: the mirror (RSI from above 50 to below, EMA3 < SMA13, main < signal). AutoClose (true): at the same new bar a buy closes when EMA3 < SMA13 and a sell when EMA3 > SMA13 (shift 1); so a sell signal also closes a buy (a reversal at that open). Indicator readings (MetaTrader's built-in sources are not printed): RSI = Wilder's RSI as the vault computes it; EMA seeded with the first close; Stochastic with price field 0 (Low/High): main = 100 x sum over the last 5 bars of (close - lowest low of 14) / sum of (highest high of 14 - lowest low of 14) (100 when that sum is 0), signal = EMA(8) of main (method MODE_EMA), seeded with the first main value. Unstated in the code, vault reading: one position at a time (the EA sends an order on every signal; a buy signal while a buy is open is ignored here). Held (R4 #19, #114): the 100-pip stop (sl = 100 x Point), a forex pip amount. Timeframe-free forex EA; daily indices, ETFs and US/India stocks are a declared application (lead decisions 88, 114). The source shows no results.";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef { id: "mtaccx_cross", name: "Accelerator Crossover EA: RSI(9) through 50 with EMA3 over SMA13 and Stochastic main over signal, out when EMA3 crosses back, next open", group: "MetaTrader 4 code: Accelerator Crossover (accelerator-fx.com)", session: 0, dir: "Long/Short", status: Status::Live,
        needs_volume: false, needs_vix: false, needs_series: &[], book: "mtaccx", rules: &[
            "F|\"extern int rsi = 9; extern int K = 14; extern int D = 8; extern int Slowing = 5; ... extern int sl = 100; ... extern bool AutoClose = TRUE;\" iStochastic(Symbol(), 0, K, D, Slowing, MODE_EMA, 0, MODE_MAIN / MODE_SIGNAL, 1); iRSI(Symbol(), 0, 9, PRICE_CLOSE, 1 / 2); iMA(..., 3, 0, MODE_EMA, PRICE_CLOSE, 1); iMA(..., 13, 0, MODE_SMA, PRICE_CLOSE, 1)",
            "E|\"if (g_irsi_176 > 50.0 && g_irsi_184 < 50.0 && g_ima_192 > g_ima_208 && g_istochastic_128 > g_istochastic_160) OrderSend(Symbol(), OP_BUY, lots, Ask, 3, Ask - sl * Point, ...)\"",
            "E|\"if (g_irsi_176 < 50.0 && g_irsi_184 > 50.0 && g_ima_192 < g_ima_208 && g_istochastic_128 < g_istochastic_160) OrderSend(Symbol(), OP_SELL, lots, Bid, 3, Bid + sl * Point, ...)\"",
            "X|\"if (OrderType() == OP_BUY) if (g_ima_192 < g_ima_208) OrderClose(...); if (OrderType() == OP_SELL) if (g_ima_192 > g_ima_208) OrderClose(...)\"",
            "X|HELD: the 100-pip stop (Ask - sl * Point), a forex pip amount (R4 #19, #114)"],
        notes: NOTE, claim: None }]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    reviewed(id).then_some(("mtaccx_cross", "Accelerator Crossover EA", "RSI 9 / EMA 3 x SMA 13 / Stochastic 14,8,5"))
}
pub fn params(id: &str) -> Option<Vec<ParamDef>> { reviewed(id).then(Vec::new) }

/// MetaTrader-style Stochastic main (slowing = ratio of sums) and its EMA signal line.
pub fn stoch_mt4(x: &Ctx, k: usize, d: usize, slow: usize) -> (Vec<f64>, Vec<f64>) {
    let n = x.c.len();
    let (mut num, mut den) = (vec![f64::NAN; n], vec![f64::NAN; n]);
    for i in k.saturating_sub(1)..n {
        let lo = x.l[i + 1 - k..=i].iter().copied().fold(f64::MAX, f64::min);
        let hi = x.h[i + 1 - k..=i].iter().copied().fold(f64::MIN, f64::max);
        num[i] = x.c[i] - lo; den[i] = hi - lo;
    }
    let mut main = vec![f64::NAN; n];
    for i in (k + slow).saturating_sub(2)..n {
        let (a, b): (f64, f64) = (num[i + 1 - slow..=i].iter().sum(), den[i + 1 - slow..=i].iter().sum());
        main[i] = if b == 0. { 100. } else { 100. * a / b };
    }
    let mut sig = vec![f64::NAN; n];
    let alpha = 2. / (d as f64 + 1.);
    for i in 0..n {
        if main[i].is_nan() { continue; }
        sig[i] = if i > 0 && sig[i - 1].is_finite() { sig[i - 1] + alpha * (main[i] - sig[i - 1]) } else { main[i] };
    }
    (main, sig)
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, _p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.r.len() { return None; }
    let (r, e3, s13) = (rsi(&x.c, 9), xavg(&x.c, 3), sma(&x.c, 13));
    let (m, sg) = stoch_mt4(x, 14, 8, 5);
    let mut pos = Book::new();
    for i in (from + 1).max(2)..=to {
        let t = i - 1;
        let (up, dn) = (e3[t] > s13[t], e3[t] < s13[t]);
        if (pos.side() > 0. && dn) || (pos.side() < 0. && up) { pos.exit(i, x.o[i], "EMA3 crossed back over SMA13 · next open"); }
        if r[t] > 50. && r[t - 1] < 50. && up && m[t] > sg[t] { pos.enter(1., i, x.o[i], "open"); }
        else if r[t] < 50. && r[t - 1] > 50. && dn && m[t] < sg[t] { pos.enter(-1., i, x.o[i], "open"); }
    }
    Some(pos.done(to, x.c[to]))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn accelerator_crossover() {
        let r = super::super::ctm::tests::walk(3000);
        let x = Ctx::new(&r, &r, &[]);
        let (m, s) = stoch_mt4(&x, 14, 8, 5);
        assert!(m[16].is_nan() && m[17].is_finite() && s[17] == m[17]);
        let t = run("mtaccx_cross", &x, 0, r.len() - 1, &Params::new()).unwrap();
        assert!(t.len() > 5 && t.iter().any(|z| z.dir > 0.) && t.iter().any(|z| z.dir < 0.));
        for z in &t { assert_eq!(z.legs[0].px, x.o[z.legs[0].i]); }
        assert!(family_of("mtaccx_cross").is_some());
    }
}

// Instrument admission: Accelerator Crossover EA 2026-11-13 (lane PD).
pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book == "mtaccx" && reviewed(def.id),
    label: "A timeframe-free MetaTrader 4 forex EA with no results shown; runs on daily indices, ETFs and stocks, US and India, are a declared application (its 100-pip stop is held).",
    basis: "Accelerator_Crossover.mq4 (Rob Turcotte 2008, accelerator-fx.com), research share USB HDD 1/META TRADER; independent check authors/pd/w/pd_ref.py",
};
