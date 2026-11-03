//! "Volatility Breakout Pattern" NinjaScript strategy (third-party forum code, author not named) from the NinjaTrader 6.5 pack:
//! research share USB HDD 1/NINJA TARDER 6.5/NinjaTrader 6.5/@Unsorted/Volatility_Breakout_Strategy_With_Indicator.zip
//! (Strategy/VolatilityBreakoutPattern.cs, Indicator/VolatilityBreakout.cs, Indicator/BollingerBandWidth.cs, Indicator/@KeltnerChannel.cs; unpacked read-only
//! copy authors/batch-i/i1w/nt65/vb). Book id `ntvbp`. Lane K14 (titles loop, 2 Nov 2026), to-code row 231; independent check
//! authors/titles-1030/K14w/k14_ref.py.
use crate::connors::{sma, Ctx, ParamDef, Params, Status, StrategyDef, Trade};
use super::tselhs::Book;

/// Admitted ids (authors/titles-1030/K14w/k14_ref.py (header)).
pub const REVIEWED: &[&str] = &["ntvbp_squeeze"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const NOTE: &str = "[Declared readings] The code as written, CalculateOnBarClose = true (it runs at each bar's close; EnterLong/ExitLong are market orders filled at the next open). Indicator VolatilityBreakout(2.0, 1.5, 12, 20) from bar 128 on (CurrentBar > 127): a squeeze bar = Keltner upper > Bollinger upper AND Keltner lower < Bollinger lower AND BollingerBandWidth equals its minimum of the last 126 bars (this bar included); it sets a flag. A later bar with Keltner upper <= Bollinger upper and Keltner lower >= Bollinger lower while the flag is set gives +2 if Momentum(12) = C - C[12] > 0, otherwise -2, and clears the flag. Bollinger = SMA(C,20) +/- 2 population standard deviations; BollingerBandWidth (the pack's indicator) = (upper - lower) / SMA(C,20), 0 for the first 21 bars. Keltner = KeltnerChannel(1.5, 20) as the pack's own Indicator/@KeltnerChannel.cs computes it: SMA of the typical price (H+L+C)/3 +/- 1.5 x SMA(High - Low, 20) (NinjaTrader's built-in definition). Indicator state runs from the first bar of the data. Strategy: flat and +2 -> buy at the next open; flat and -2 -> short (the BBWidth < 999 filter never binds). On each close while long (from the fill bar): on the first one, trail = stop = entry x 0.98; then if the high is above the entry and the last high mark (the entry at first), the trail rises by the difference and the mark moves to the high; exit at the next open if the low <= trail while the trail is above the entry ('Trailing Stop'), else if the low <= stop ('Stop Loss'). Short mirrored with the lows and 1.02. The stops are checked on the close and filled at the next open, as the code does; 100 shares -> one unit. Third-party sample code with no test or result; daily bars on indices, ETFs and US/India stocks are a declared application; the vault's standard costs. Compare jcmtt_squeeze (Carter's TTM Squeeze, ATR Keltner): this one adds the 126-bar BandWidth minimum and fixed 2% stops.";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef { id: "ntvbp_squeeze", name: "Volatility Breakout Pattern (NinjaTrader): after a squeeze at a 126-bar BandWidth low, enter when the bands leave the Keltner Channel, Momentum(12) sets the side; 2% stop and trailing stop", group: "NinjaTrader 6.5 pack: Volatility Breakout Pattern", session: 0, dir: "Long/Short", status: Status::Live,
        needs_volume: false, needs_vix: false, needs_series: &[], book: "ntvbp", rules: &[
            "F|\"if (KeltnerChannel(kCDeviation, Period).Upper[0] > Bollinger(bBDeviation, Period).Upper[0] && KeltnerChannel(kCDeviation, Period).Lower[0] < Bollinger(bBDeviation, Period).Lower[0] && (BollingerBandWidth(bBDeviation,Period)[0] == MIN(BollingerBandWidth(bBDeviation,Period), 126)[0])) { Plot0.Set(1); blnFlag = true; }\" (VolatilityBreakout.cs; Period 20, KC 1.5, BB 2.0)",
            "E|\"else if (blnFlag && KeltnerChannel(...).Upper[0] <= Bollinger(...).Upper[0] && KeltnerChannel(...).Lower[0] >= Bollinger(...).Lower[0]) { Plot0.Set(Momentum(momentumPeriod)[0] > 0 ? 2 : -2); blnFlag = false; }\"; \"if (VolatilityBreakout(...).Plot0[0] == 2 && ... Position.MarketPosition == MarketPosition.Flat) EnterLong(100, \\\"Long\\\");\" (-2: EnterShort) (VolatilityBreakoutPattern.cs)",
            "X|\"trailStop = Position.AvgPrice - (trailingStopPct * Position.AvgPrice); StopLoss = Position.AvgPrice - (stopLossPct * Position.AvgPrice);\" 0.020 each; \"if (High[0] > Position.AvgPrice && High[0] > prevValue) { trailStop = trailStop + (High[0] - prevValue); prevValue = High[0]; }\" \"if (Low[0] <= trailStop && ... Position.AvgPrice < trailStop) ExitLong(...); else if (Low[0] <= StopLoss && ... Position.AvgPrice > StopLoss) ExitLong(...);\" (VolatilityBreakoutPattern.cs)"],
        notes: NOTE, claim: None }]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    (id == "ntvbp_squeeze").then_some(("ntvbp_squeeze", "NinjaTrader Volatility Breakout Pattern", "BB/Keltner squeeze, 2% stops"))
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> { (id == "ntvbp_squeeze").then(Vec::new) }

/// The indicator's Plot0 for every bar: 1 squeeze, +2 / -2 trigger, 0 otherwise.
pub fn signal(x: &Ctx) -> Vec<i8> {
    let n = x.c.len();
    let m = sma(&x.c, 20);
    let sd: Vec<f64> = (0..n).map(|i| if i < 19 { f64::NAN } else {
        let w = &x.c[i - 19..=i]; let a = w.iter().sum::<f64>() / 20.; (w.iter().map(|v| (v - a).powi(2)).sum::<f64>() / 20.).sqrt() }).collect();
    let typ: Vec<f64> = (0..n).map(|i| (x.h[i] + x.l[i] + x.c[i]) / 3.).collect();
    let rng: Vec<f64> = (0..n).map(|i| x.h[i] - x.l[i]).collect();
    let (km, kr) = (sma(&typ, 20), sma(&rng, 20));
    let bbw: Vec<f64> = (0..n).map(|i| if i > 20 { ((m[i] + 2. * sd[i]) - (m[i] - 2. * sd[i])) / m[i] } else { 0. }).collect();
    let mut out = vec![0i8; n];
    let mut flag = false;
    for i in 128..n {
        let (bu, bl, ku, kl) = (m[i] + 2. * sd[i], m[i] - 2. * sd[i], km[i] + 1.5 * kr[i], km[i] - 1.5 * kr[i]);
        let low = bbw[i - 125..=i].iter().copied().fold(f64::INFINITY, f64::min);
        if ku > bu && kl < bl && bbw[i] == low { out[i] = 1; flag = true; }
        else if flag && ku <= bu && kl >= bl { out[i] = if x.c[i] - x.c[i - 12] > 0. { 2 } else { -2 }; flag = false; }
    }
    out
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, _p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.r.len() { return None; }
    let sig = signal(x);
    let mut pos = Book::new();
    let (mut pend, mut why): (f64, &'static str) = (0., ""); // next-open order: +1/-1 entry side, or exit when held
    let (mut just, mut trail, mut stop, mut prev) = (false, 0., 0., 0.);
    for t in from..=to {
        if pend != 0. {
            if pos.side() != 0. { pos.exit(t, x.o[t], why); } else { pos.enter(pend, t, x.o[t], why); }
            pend = 0.;
        }
        if t == to { break; }
        let d = pos.side();
        if d == 0. && sig[t].abs() == 2 { pend = sig[t].signum() as f64; why = if pend > 0. { "EnterLong at the open" } else { "EnterShort at the open" }; just = true; continue; }
        if d == 0. { continue; }
        let a = pos.entry_px();
        if just { trail = a - d * 0.02 * a; stop = trail; prev = a; just = false; }
        if d > 0. {
            if x.h[t] > a && x.h[t] > prev { trail += x.h[t] - prev; prev = x.h[t]; }
            if x.l[t] <= trail && a < trail { pend = -1.; why = "Trailing Stop"; }
            else if x.l[t] <= stop && a > stop { pend = -1.; why = "Stop Loss"; }
        } else {
            if x.l[t] < a && x.l[t] < prev { trail -= prev - x.l[t]; prev = x.l[t]; }
            if x.h[t] >= trail && a > trail { pend = 1.; why = "Trailing Stop"; }
            else if x.h[t] >= stop && a < stop { pend = 1.; why = "Stop Loss"; }
        }
    }
    Some(pos.done(to, x.c[to]))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;
    #[test]
    fn squeeze_breakout_trades_and_stops() {
        // quiet stretches (squeezes) followed by swings
        let r: Vec<Row> = (0..2000).map(|i| { let f = i as f64; let amp = if (i / 150) % 2 == 0 { 0.3 } else { 6. };
            let c = 100. + amp * (f / 7.).sin() + 4. * (f / 90.).sin() + 0.01 * f;
            Row { ticker: "T".into(), date: format!("{:05}", i), open: c - 0.1, high: c + 0.4 + 0.1 * amp, low: c - 0.4 - 0.1 * amp, close: c, volume: None } }).collect();
        let x = Ctx::new(&r, &r, &[]);
        assert!(signal(&x).iter().any(|s| s.abs() == 2));
        let t = run("ntvbp_squeeze", &x, 0, r.len() - 1, &Params::new()).unwrap();
        assert!(!t.is_empty());
        for z in &t { assert!(z.open || z.reason == "Trailing Stop" || z.reason == "Stop Loss", "{}", z.reason); }
        for w in t.windows(2) { assert!(w[1].legs[0].i > w[0].exit_i); }
        assert!(family_of("ntvbp_squeeze").is_some() && run("ntvbp_x", &x, 0, 9, &Params::new()).is_none());
    }
}

// Instrument admission: NinjaTrader Volatility Breakout Pattern 2026-11-02 (titles lane K14).
pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book == "ntvbp" && reviewed(def.id),
    label: "Third-party NinjaScript sample strategy with no test or result. Runs on daily indices, ETFs and stocks, US and India, are a declared application.",
    basis: "NinjaTrader 6.5 pack, Volatility_Breakout_Strategy_With_Indicator.zip (VolatilityBreakoutPattern.cs, VolatilityBreakout.cs, BollingerBandWidth.cs); independent check authors/titles-1030/K14w/k14_ref.py",
};
