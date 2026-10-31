//! Daryl Guppy / guppytraders.com, "Metastock Formula Index" (collection of reader, Equis and
//! TASC Trader's Tips formulas, guppymsformulas.pdf, 15 Apr 2002, 352 pp.; older 2001 edition
//! "DBZ_Guppy Formulas Meta.pdf" is a subset). Only the complete MetaStock system tests with fixed
//! parameters (entry AND exit given in code) are Live. Formulas are coded literally; the MetaStock
//! System Tester defaults (trade price = close, trade delay = 0) apply unless the formula says otherwise.
//! Source, page refs and independent checks: library/dgmf/.
use crate::connors::{true_range, wilder, Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "dgmf";
/// Admitted ids (authors/guppyf/laneb/SOURCE-CONTRACT.md).
pub const REVIEWED: &[&str] = &["dgmf_bb7", "dgmf_brow_dmi", "dgmf_cci_spike", "dgmf_ccifp", "dgmf_disp_ema", "dgmf_ema_5_21", "dgmf_evans_ema", "dgmf_fosc_alt", "dgmf_fosc_sys", "dgmf_gapup_5d", "dgmf_gap1", "dgmf_gap2", "dgmf_gap3", "dgmf_karnish_bbh", "dgmf_maas_rsi5", "dgmf_stoch_cross", "dgmf_tezel_ma", "dgmf_tezel_reg", "dgmf_triple_ma", "dgmf_williams_exp", "dgmf_vbo_taylor" /* intraday wave 2b: daily bars decide every trade */];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }
const NAN: f64 = f64::NAN;

// ------------------------------------------------------------------ MetaStock-style indicators
fn fin(v: f64) -> bool { v.is_finite() }
/// Mov(x,n,E): alpha 2/(n+1), seeded with the simple mean of the first n consecutive valid values.
fn ema(v: &[f64], n: usize) -> Vec<f64> {
    let mut out = vec![NAN; v.len()];
    let a = 2.0 / (n as f64 + 1.0);
    let (mut run, mut prev) = (0usize, NAN);
    for i in 0..v.len() {
        if !fin(v[i]) { if !fin(prev) { run = 0; } continue; }
        if !fin(prev) {
            run += 1;
            if run == n { prev = v[i + 1 - n..=i].iter().sum::<f64>() / n as f64; out[i] = prev; }
            continue;
        }
        prev += a * (v[i] - prev);
        out[i] = prev;
    }
    out
}
fn window<F: Fn(&[f64]) -> f64>(v: &[f64], n: usize, f: F) -> Vec<f64> {
    (0..v.len()).map(|i| if i + 1 < n { NAN } else { let w = &v[i + 1 - n..=i]; if w.iter().all(|x| fin(*x)) { f(w) } else { NAN } }).collect()
}
/// Mov(x,n,W): linearly weighted, newest weight n.
fn wma(v: &[f64], n: usize) -> Vec<f64> {
    let den = (n * (n + 1) / 2) as f64;
    window(v, n, |w| w.iter().enumerate().map(|(k, x)| (k + 1) as f64 * x).sum::<f64>() / den)
}
/// Std(x,n): population standard deviation (MetaStock / Bollinger convention).
fn sd(v: &[f64], n: usize) -> Vec<f64> {
    window(v, n, |w| { let m = w.iter().sum::<f64>() / n as f64; (w.iter().map(|x| (x - m) * (x - m)).sum::<f64>() / n as f64).sqrt() })
}
/// Mov(x,n,S) as an exact window mean (no running sum, so equality tests such as C<=Mov(C,5,S) are not
/// flipped by accumulated rounding).
fn sma(v: &[f64], n: usize) -> Vec<f64> { window(v, n, |w| w.iter().sum::<f64>() / n as f64) }
fn hhv(v: &[f64], n: usize) -> Vec<f64> { window(v, n, |w| w.iter().copied().fold(f64::MIN, f64::max)) }
fn llv(v: &[f64], n: usize) -> Vec<f64> { window(v, n, |w| w.iter().copied().fold(f64::MAX, f64::min)) }
fn sum(v: &[f64], n: usize) -> Vec<f64> { window(v, n, |w| w.iter().sum()) }
/// Ref(x,-k)
fn back(v: &[f64], k: usize) -> Vec<f64> { (0..v.len()).map(|i| if i >= k { v[i - k] } else { NAN }).collect() }
fn zip(a: &[f64], b: &[f64], f: impl Fn(f64, f64) -> f64) -> Vec<f64> { a.iter().zip(b).map(|(x, y)| f(*x, *y)).collect() }
/// Cross(a,b): a above b today, a at or below b yesterday.
fn cross(a: &[f64], b: &[f64], i: usize) -> bool {
    i >= 1 && fin(a[i]) && fin(b[i]) && fin(a[i - 1]) && fin(b[i - 1]) && a[i] > b[i] && a[i - 1] <= b[i - 1]
}
fn konst(v: f64, n: usize) -> Vec<f64> { vec![v; n] }
/// Linear regression over the last n bars with x = 1..n: (value at x=n, slope, standard error, correlation with x).
fn linreg(v: &[f64], n: usize) -> (Vec<f64>, Vec<f64>, Vec<f64>, Vec<f64>) {
    let len = v.len();
    let (mut val, mut slope, mut ste, mut cor) = (vec![NAN; len], vec![NAN; len], vec![NAN; len], vec![NAN; len]);
    let nf = n as f64;
    let sx = nf * (nf + 1.) / 2.;
    let sxx = nf * (nf + 1.) * (2. * nf + 1.) / 6.;
    for i in n.saturating_sub(1)..len {
        if i + 1 < n { continue; }
        let w = &v[i + 1 - n..=i];
        if !w.iter().all(|x| fin(*x)) { continue; }
        let sy: f64 = w.iter().sum();
        let sxy: f64 = w.iter().enumerate().map(|(k, y)| (k + 1) as f64 * y).sum();
        let syy: f64 = w.iter().map(|y| y * y).sum();
        let b = (nf * sxy - sx * sy) / (nf * sxx - sx * sx);
        let a = (sy - b * sx) / nf;
        val[i] = a + b * nf;
        slope[i] = b;
        let sse: f64 = w.iter().enumerate().map(|(k, y)| { let e = y - (a + b * (k + 1) as f64); e * e }).sum();
        if n > 2 { ste[i] = (sse / (nf - 2.)).sqrt(); }
        let vy = nf * syy - sy * sy;
        let vx = nf * sxx - sx * sx;
        cor[i] = if vy > 0. { (nf * sxy - sx * sy) / (vx * vy).sqrt() } else { NAN };
    }
    (val, slope, ste, cor)
}
/// ForecastOsc(C,n) = 100*(C - Ref(TSF(C,n),-1))/C, TSF = regression value + slope (the formula given in
/// "TSF Optimised Trading System" of the same collection).
fn forecast_osc(c: &[f64], n: usize) -> Vec<f64> {
    let (v, s, _, _) = linreg(c, n);
    let tsf = zip(&v, &s, |a, b| a + b);
    let t1 = back(&tsf, 1);
    zip(c, &t1, |c, t| 100. * (c - t) / c)
}
/// Wilder +DI, -DI, ADX (MetaStock PDI/MDI/ADX).
fn dmi(x: &Ctx, n: usize) -> (Vec<f64>, Vec<f64>, Vec<f64>) {
    let r = x.r; let len = r.len();
    let (mut pdm, mut mdm, mut tr) = (vec![NAN; len], vec![NAN; len], vec![NAN; len]);
    let trs = true_range(r);
    for i in 1..len {
        let up = r[i].high - r[i - 1].high; let dn = r[i - 1].low - r[i].low;
        pdm[i] = if up > dn && up > 0. { up } else { 0. };
        mdm[i] = if dn > up && dn > 0. { dn } else { 0. };
        tr[i] = trs[i];
    }
    let (sp, sm, st) = (wilder(&pdm, n), wilder(&mdm, n), wilder(&tr, n));
    let (mut pdi, mut mdi, mut dx) = (vec![NAN; len], vec![NAN; len], vec![NAN; len]);
    for i in 0..len {
        if st[i] > 0. {
            pdi[i] = 100. * sp[i] / st[i]; mdi[i] = 100. * sm[i] / st[i];
            dx[i] = if pdi[i] + mdi[i] > 0. { 100. * (pdi[i] - mdi[i]).abs() / (pdi[i] + mdi[i]) } else { 0. };
        }
    }
    let adx = wilder(&dx, n);
    (pdi, mdi, adx)
}
/// CCI(n) on the typical price with the mean absolute deviation.
fn cci(x: &Ctx, n: usize) -> Vec<f64> {
    let tp: Vec<f64> = (0..x.c.len()).map(|i| (x.h[i] + x.l[i] + x.c[i]) / 3.).collect();
    (0..tp.len()).map(|i| {
        if i + 1 < n { return NAN; }
        let w = &tp[i + 1 - n..=i];
        let m = w.iter().sum::<f64>() / n as f64;
        let md = w.iter().map(|v| (v - m).abs()).sum::<f64>() / n as f64;
        if md > 0. { (tp[i] - m) / (0.015 * md) } else { NAN }
    }).collect()
}
/// Stoch(k, slowing) = 100*Sum(C-LLV(L,k),s)/Sum(HHV(H,k)-LLV(L,k),s).
fn stoch(x: &Ctx, k: usize, s: usize) -> Vec<f64> {
    let (hh, ll) = (hhv(&x.h, k), llv(&x.l, k));
    let num = zip(&x.c, &ll, |c, l| c - l);
    let den = zip(&hh, &ll, |h, l| h - l);
    let (sn, sd_) = (sum(&num, s), sum(&den, s));
    zip(&sn, &sd_, |a, b| if b > 0. { 100. * a / b } else { NAN })
}
fn rsi(c: &[f64], n: usize) -> Vec<f64> { crate::connors::rsi(c, n) }
fn roc_pct(v: &[f64], n: usize) -> Vec<f64> { let b = back(v, n); zip(v, &b, |a, b| 100. * (a / b - 1.)) }

// ------------------------------------------------------------------ catalogue
struct S { id: &'static str, name: &'static str, group: &'static str, dir: &'static str, status: Status, vol: bool, rules: &'static [&'static str], notes: &'static str }

const MS: &str = " MetaStock System Tester defaults (trade price = close, trade delay = 0) unless the formula states otherwise; one position at a time; an exit and an opposite entry on the same bar reverse; after an exit no same-direction re-entry on that bar; final position left open and marked to the last close. Indices replace the stocks/futures of the source.";

fn defs() -> Vec<S> {
    use Status::*;
    vec![
        S { id: "dgmf_200dma_1pt", name: "200 dma w/i 1 pt", group: "Formula 14 · Moving Average Violated", dir: "Long/Short", status: Live, vol: false,
            rules: &["E|enter long: c>=mov(c,200,s)+1", "X|exit long: ((if ((c<=prev(llv(c,15)-.5, 1)),1,0)) + (if ((c<=.75*hhv(c,10)),1,0)))>=1",
                     "E|enter short: c<=mov(c,200,s)-1", "X|exit short: c>=hhv(llv(c,15), 15)+.5"],
            notes: "guppymsformulas.pdf p. 29. Printed under 'Moving Average Violated By %'. 'prev(x, 1)' is not valid MetaStock; the same collection corrects it to Ref(x,-1) ('Boomers watchsig 2 (Ref not prev)'), which is used. 1 and .5 are price points, applied literally to the index." },
        S { id: "dgmf_200dma_3pt", name: "200 dma w/i 3pts", group: "Formula 14 · Moving Average Violated", dir: "Short", status: Live, vol: false,
            rules: &["E|enter short: c<=mov(c,200,s)-3", "X|exit short: c>=hhv(llv(c,15), 15)+.5"],
            notes: "guppymsformulas.pdf p. 29. Same page as dgmf_200dma_1pt. 3 and .5 are price points, applied literally." },
        S { id: "dgmf_21d_rev_di", name: "21 d reversal w di", group: "Formula 14", dir: "Long/Short", status: Live, vol: false,
            rules: &["E|enter long: c>prev(hhv(c,21),1) and adx(1)>adx(14) and (pdi(9)>mdi(14))", "X|exit long: c<prev(llv(c,21),1) and (pdi(14)<mdi(9))",
                     "E|enter short: c<prev(llv(c,21),1) and adx(1)>adx(14) and (pdi(9)<mdi(14))", "X|exit short: c>prev(hhv(c,21),1) and (pdi(9)>mdi(14))"],
            notes: "guppymsformulas.pdf p. 29. prev(x,1) read as Ref(x,-1) (the collection's own correction). ADX(1) = Wilder DX over 1 bar." },
        S { id: "dgmf_stoch_ma", name: "Stochastic MA System", group: "Formula 14", dir: "Long/Short", status: Live, vol: false,
            rules: &["E|enter long: mov(stoch(55,21),5,w)>ref(mov(stoch(55,21),5,w),-1) and mov(stoch(55,21),5,w)<75 and mov(stoch(55,21),5,w)>20",
                     "X|exit long: (mov(stoch(55,21),5,w)<75 and ref(mov(stoch(55,21),5,w),-1)>75)",
                     "E|enter short: (mov(stoch(55,21),5,w)<70 and ref(mov(stoch(55,21),5,w),-1)>70) and mov(stoch(55,21),5,w)<ref(mov(stoch(55,21),5,w),-1)",
                     "X|exit short: same formula as enter long"],
            notes: "guppymsformulas.pdf p. 30. Stoch(55,21) = MetaStock %K with 55-bar range and 21-bar slowing." },
        S { id: "dgmf_bb7", name: "Bollinger Bands Formula 7 Day", group: "Formula 14", dir: "Long", status: Live, vol: false,
            rules: &["E|enter long: high>(mov(Close,20,S)-std(Close,20,2)) and ref(low,-7)<ref((mov(Close,20,S)-std(Close,20,2)),-7)",
                     "X|exit long: close<(mov(Close,20,S)+std(Close,20,2)) and ref(close,-7)>ref((mov(Close,20,S)+std(Close,20,2)),-7) and Mov((RSI(14)-LLV(RSI(14),14))/(HHV(RSI(14),14)-(LLV(RSI(14),14))),14,E)*100<70 and ref(that,-3)>70 and (mov(Close,20,S)+std(Close,20,2))>(mov(c,89,s)+(.062*(mov(c,89,s))))"],
            notes: "guppymsformulas.pdf p. 30. std(Close,20,2) has an extra argument in the source; read as 2 standard deviations (the Bollinger band the title names), i.e. 2*Std(C,20), population SD." },
        S { id: "dgmf_ema_5_21", name: "EMA Cross System", group: "Formula 14", dir: "Long", status: Live, vol: false,
            rules: &["E|enter long: mov(c,5,e)>mov(c,21,e) and ref(mov(c,5,e),-1)<=ref(mov(c,21,e),-1)", "X|exit long: mov(c,5,e)<mov(c,21,e) and ref(mov(c,5,e),-1)>=ref(mov(c,21,e),-1)"],
            notes: "guppymsformulas.pdf p. 30. 'avoid using when the market has no clear direction' is advice, not coded." },
        S { id: "dgmf_boomers_buysig", name: "Boomers buysig", group: "Formula 14 · Boomers Trading Signals", dir: "Long", status: Live, vol: false,
            rules: &["E|enter long: ((adx(14)+adx(27))/2)>30 and pdi(27)>mdi(27)", "X|exit long: c<=prev(llv(c,15)-.5, 1) or c<=.75*hhv(c,10)"],
            notes: "guppymsformulas.pdf p. 31. prev(..,1) -> Ref(..,-1) as in the collection's 'watchsig 2 (Ref not prev)'. .5 is a price point." },
        S { id: "dgmf_evans_ema", name: "MACD Crossover System test (EMA 5/13/40)", group: "Formula 2", dir: "Long", status: Live, vol: false,
            rules: &["E|Enter Long: Mov(C,5,E) > Mov(C,13,E) AND Mov(C,13,E) > Mov(C,40,E)", "X|Close Long: Cross(Mov(C,13,E),Mov(C,5,E))",
                     "X|[labelled option, not default] 'change the Close Long to Cross(Mov(C,40,E),Mov(C,5,E))' (p. 'This will keep you in the trade longer')"],
            notes: "guppymsformulas.pdf p. 49. David Evans. Parameter close_ma=13 is the book default; 40 is the book's own named alternative." },
        S { id: "dgmf_tezel_ma", name: "MovAvg Asymmetric Volatility Price Bands (Tezel, TASC Oct 1997)", group: "Formula 4 · TASC Trader's Tips", dir: "Long", status: Live, vol: true,
            rules: &["F|UpperBand := BBandTop(CLOSE,11,S,1.7); LowerBand := BBandBot(CLOSE,11,S,2)",
                     "E|BuySignal1 := Sum(CLOSE > UpperBand,3) = 3; BuySignal2 := CLOSE > UpperBand AND Ref(LOW,-1) > Ref(upperband,-1); BuySignal3 := LOW > UpperBand AND Ref(CLOSE,-1) > Ref(upperband,-1); BuySignal4 := CLOSE > UpperBand AND CLOSE > 1.4 * LLV(LOW,12) AND Mov(VOLUME,3,S) > 2000 {volume in 100's, otherwise use 200000} AND Mov(HIGH,3,S) > UpperBand AND Mov(HIGH - LOW,3,S) > Mov(HIGH - LOW,11,S)",
                     "X|SellSignal1 := Sum(CLOSE < LowerBand,3) = 3; SellSignal2 := CLOSE < (1-.18) * HHV(HIGH,12) AND Sum(CLOSE < LowerBand,2) = 2; SellSignal3 := CLOSE < (1-.18) * HHV(HIGH,12) AND HIGH < LowerBand",
                     "X|STOPS Maximum Loss: LONG ONLY 10.00 Percent", "F|'change the trade delay to 1, then change the Trade Price to Open'"],
            notes: "guppymsformulas.pdf p. 72-73. Fills at the next open. Our volume is in shares, so the book's own alternative 200000 is used. Max-loss stop: intraday at entry*0.90 (gap through = open), checked from the entry bar; a pending signal exit at the open is taken before the stop." },
        S { id: "dgmf_tezel_reg", name: "Regression Asymmetric Volatility Price Bands (Tezel, TASC Oct 1997)", group: "Formula 4 · TASC Trader's Tips", dir: "Long", status: Live, vol: false,
            rules: &["E|Enter Long: UpperBand := STEBandTop(CLOSE,21,1); Sum(CLOSE > UpperBand,3) = 3 AND LinRegSlope(CLOSE,21) > 0 AND ROC(Correl(CLOSE,Cum(1),21,0),2,$) >= .2",
                     "X|Close Long: LowerBand := STEBandBot(CLOSE,21,1.5); SellSignal1 := Sum(CLOSE < LowerBand,3) = 3; SellSignal2 := CLOSE < (1-.18) * HHV(HIGH,22) AND HIGH < LowerBand",
                     "X|STOPS Maximum Loss: LONG ONLY 10.00 Percent", "F|trade delay 1, trade price Open"],
            notes: "guppymsformulas.pdf p. 73. Standard-error band = linear-regression value +/- k * standard error of estimate (SSE/(n-2)), smoothing 1 (the text: '1 for the smoothing periods'). Correl with Cum(1) = correlation with the bar index." },
        S { id: "dgmf_gapup_5d", name: "Gap Up System with Delayed Exit", group: "Formula 8", dir: "Long", status: Live, vol: false,
            rules: &["E|Enter long: GapUp()", "X|Close long: Ref(GapUp(),-5)", "F|Trade price Open, Trade delay 1, Positions Long and short"],
            notes: "guppymsformulas.pdf p. 130. GapUp() = today's low above yesterday's high (same definition as the collection's 'Gap 1 System'). Signals fill at the next open." },
        S { id: "dgmf_roc_ma", name: "ROC Moving Average System Test", group: "Formula 8", dir: "Long/Short", status: Live, vol: false,
            rules: &["E|ENTER LONG: ROC(Mov(C,12,E),1,%)>0 AND ROC(Mov(C,60,E),1,%)>0",
                     "X|EXIT LONG: (ROC(Mov(C,12,E),1,%)<0 AND ROC(Mov(C,60,E),1,%)>0) OR (ROC(Mov(C,12,E),1,%)>0 AND ROC(Mov(C,60,E),1,%)<0)",
                     "E|SHORT: ROC(Mov(C,12,E),1,%)<0 AND ROC(Mov(C,60,E),1,%)<0", "X|EXIT SHORT: same as EXIT LONG"],
            notes: "guppymsformulas.pdf p. 131. Onno Goedknegt. Literal: a long is not closed when both slopes turn negative at once (neither mixed case) - it waits for a mixed-slope bar." },
        S { id: "dgmf_brow_dmi", name: "Working with DMI (Dick Brow)", group: "Formula 7", dir: "Long", status: Live, vol: false,
            rules: &["E|OPEN LONG: close>hhv(low,21)", "X|CLOSE LONG: close<llv(high,21)"],
            notes: "guppymsformulas.pdf p. 112. Printed below the 'Smoothed DMI Index' formula, which the system does not use." },
        S { id: "dgmf_ccifp", name: "CCIF-P Entry/Exit signal (Mike Arnoldi)", group: "Formula 1", dir: "Long", status: Live, vol: false,
            rules: &["F|CCIF-P = (CCI(8)+CCI(13)+CCI(21))/3", "E|buy: Fml(\"CCIF-P\")>Ref(Fml(\"CCIF-P\"),-1) AND Cross(Fml(\"CCIF-P\"),-100) OR Cross(Fml(\"CCIF-P\"),100)",
                     "X|sell: Fml(\"CCIF-P\")<Ref(Fml(\"CCIF-P\"),-1) AND Cross(100,Fml(\"CCIF-P\")) OR Cross(-100,Fml(\"CCIF-P\"))"],
            notes: "guppymsformulas.pdf p. 35-36. buy/sell = MetaStock Buy (enter long) / Sell (close long). AND/OR precedence does not matter: an up-cross implies a rising value." },
        S { id: "dgmf_fosc_alt", name: "Forecast Oscillator System Alternative", group: "Formula 10", dir: "Long", status: Live, vol: false,
            rules: &["E|Enter long: Cross(ForecastOsc(C,21),Mov(ForecastOsc(C,21),3,E)) AND Cross(ForecastOsc(C,21),0)",
                     "X|Exit long: Cross(Mov(ForecastOsc(C,21),3,E),ForecastOsc(C,21)) AND Cross(6,ForecastOsc(C,21))"],
            notes: "guppymsformulas.pdf p. 169. Both crosses must happen on the same bar (the text: 'use alert() ... if you don't require both conditions to fire on the same day')." },
        S { id: "dgmf_fosc_sys", name: "Forecast Oscillator System", group: "Formula 10", dir: "Long", status: Live, vol: false,
            rules: &["E|Enter long: Cross(ForecastOsc(C,21),Mov(C,3,E)) AND Cross(ForecastOsc(C,21),0)", "X|Exit long: Cross(Mov(C,3,E),ForecastOsc(C,21)) AND Cross(6,Mov(C,3,E))"],
            notes: "guppymsformulas.pdf p. 169. Coded literally: it compares the oscillator with a moving average of PRICE, so on index price levels it never fires (0 trades). Kept to show the formula was seen; the 'Alternative' above is the working version." },
        S { id: "dgmf_karnish_bbh", name: "Karnish Bollinger Band Histogram Trading System", group: "Formula 11", dir: "Long/Short", status: Live, vol: false,
            rules: &["F|BBHistogram := (CLOSE + 2*Std(CLOSE,20) - Mov(CLOSE,20,SIMPLE)) / (4*(Std(CLOSE,20))) * 100",
                     "E|{EnterLong and Close Short} Cross(0,BBHistogram)", "E|{Enter Short and Close Long} Cross(BBHistogram,100)", "X|{Enter Short and Close Long} closes a long; {EnterLong and Close Short} closes a short (same formulas)",
                     "F|'Sell the opening days after ...', 'Buy, when the BB Histo dips below zero (the following morning)', 'close to close'"],
            notes: "guppymsformulas.pdf p. 194-195. Stop-and-reverse at the next open. NOT coded: 'Add to positions if the indicator repenetrates' (no size given; the MetaStock code itself has no adds)." },
        S { id: "dgmf_cci_spike", name: "CCI Spike Trading System (Craig Monroe)", group: "Formula 9", dir: "Long", status: Live, vol: false,
            rules: &["E|Enter long: Ref(CCI(13)/(Mov(CCI(13),3,S)),-1)>1.5 AND Cross(CCI(13),(Mov(CCI(13),3,S))) AND Ref(CCI(13),-1)<-25",
                     "X|Exit long: Cross((Mov(CCI(13),3,S)),CCI(13)) AND Ref(CCI(13),-1)>200"], notes: "guppymsformulas.pdf p. 162. Long only as designed (S&P 500)." },
        S { id: "dgmf_gap1", name: "Gap 1 System", group: "Formula 9", dir: "Long", status: Live, vol: false,
            rules: &["E|{BUY} L>Ref(H,-1) OR Cum(1)=LastValue(Cum(1))", "X|{SELL} H<Ref(L,-1) OR Cum(1)=LastValue(Cum(1))"],
            notes: "guppymsformulas.pdf p. 157. BUY/SELL = enter/close long. 'Cum(1)=LastValue(Cum(1))' only forces a trade on the last loaded bar (a test artefact using future knowledge); dropped - the final position is shown open instead." },
        S { id: "dgmf_gap2", name: "Gap 2 System", group: "Formula 9", dir: "Long", status: Live, vol: false,
            rules: &["E|{BUY} N1:=5; L>Ref(HHV(H,N1),-1)", "X|{SELL} N1:=5; H<Ref(LLV(L,N1),-1)"], notes: "guppymsformulas.pdf p. 157. As Gap 1 (LastValue clause dropped)." },
        S { id: "dgmf_gap3", name: "Gap 3 System", group: "Formula 9", dir: "Long/Short", status: Live, vol: false,
            rules: &["E|{Enter Long} N1:=5; L>Ref(HHV(H,N1),-1)", "X|{Exit Long} N2:=3; C<Ref(LLV(L,N2),-1)", "E|{Enter Short} N1:=5; H<Ref(LLV(L,N1),-1)", "X|{Exit Short} N2:=3; C>Ref(HHV(H,N2),-1)"],
            notes: "guppymsformulas.pdf p. 157-158. Ton Maas. LastValue clause dropped as in Gap 1." },
        S { id: "dgmf_katz_cyclical", name: "Cyclical System (J. O. Katz, TASC Feb 1999, MetaStock by Ton Maas)", group: "Formula 9", dir: "Long/Short", status: Live, vol: false,
            rules: &["F|thresh:=4; k:=3; m:=63; hld:=10; Value1:=Stdev(Mov(C,m,S)-Mov(C,m+k,S),20); Value2:=Mov(C,m,S)-Mov(C,m+k,S); tv1:=thresh*Value1",
                     "E|{Enter Long} Value2>tv1", "X|{Close Long} Ref(Cross(Value2,tv1),-hld)", "E|{Enter Short} Value2<tv1", "X|{Close Short} Ref(Cross(tv1,Value2),-hld)",
                     "F|Trade Delay zero, Positions both"],
            notes: "guppymsformulas.pdf p. 160-161. Coded literally, including 'Enter Short Value2<tv1' (not < -tv1), so the system is short most of the time. A position is closed only by its own close rule." },
        S { id: "dgmf_maas_rsi5", name: "Example of a trading system (RSI 5, Ton Maas)", group: "Formula 17", dir: "Long", status: Live, vol: false,
            rules: &["E|Enter Long if 1) today's five-day RSI is greater than yesterday's five-day RSI; and 2) today's close is below the close of five days ago; and 3) today's close is less then or equal to the average of the last five day's closes.",
                     "X|Exit tomorrow at the market if: 1) today's close is higher than the average of the last five days' closes; or 2) you have been in the trade 10 days."],
            notes: "guppymsformulas.pdf p. 251. Entry at the signal close (tester default), exit at the next open. '10 days' = the code's Ref(Long,-10): exit signal on the 10th bar after the entry bar." },
        S { id: "dgmf_disp_ema", name: "Displaced moving average system test (Paul Beattie)", group: "Formula 17", dir: "Long", status: Live, vol: false,
            rules: &["E|Buy: Cross(C,Ref(Mov(C,50,e),-15)) {uses 50ema displaced by 15 days}", "X|Sell: Cross(Ref(Mov(C,50,e),-15),C)"], notes: "guppymsformulas.pdf p. 263. Buy/Sell = enter/close long." },
        S { id: "dgmf_stoch_cross", name: "Stochastic cross trading system", group: "Formula 18", dir: "Long", status: Live, vol: false,
            rules: &["E|Buy: stoch(12,3)<36 and cross(stoch(12,3),mov(stoch(12,3),6,w))", "X|Sell: stoch(12,3)>52 and cross(mov(stoch(12,3),3,w),stoch(12,3)) or mov(stoch(12,3),3,w)>55"],
            notes: "guppymsformulas.pdf p. 277. Sell read as (A and B) or C, matching the question text ('rises above 52 and then falls below its 3 day WMA OR 3 day WMA crosses above 55')." },
        S { id: "dgmf_triple_ma", name: "_Triple MA Crossovers / Positions (25/75/200)", group: "Formula 21", dir: "Long/Short", status: Live, vol: false,
            rules: &["F|ShortMA := Mov(CLOSE,25,S); MediumMA := Mov(CLOSE,75,S); LongMA := Mov(CLOSE,200,S)",
                     "E|LongSignal := Cross(Sma, Mma) AND Lma > Mma; ShortSignal := Cross(Mma, Sma) AND Lma < Mma",
                     "E|Positions: If(BarsSince(Ref(BuyLong,-1)) <= BarsSince(Ref(SellShort,-1)), +1, If(BarsSince(Ref(SellShort,-1)) <= BarsSince(Ref(BuyLong,-1)), -1, 0))",
                     "X|the position switches when the opposite signal of the Positions formula fires (no other exit)", "F|_Triple MA Equity counts OPEN-to-OPEN changes"],
            notes: "guppymsformulas.pdf p. 321-322. Always-in after the first signal; the position changes the bar after a signal, priced at that bar's open (the book's equity uses opens). Its author notes the equity indicator looks wrong; the position rule is used." },
        S { id: "dgmf_williams_exp", name: "Experimental Williams Trading System", group: "Formula 10", dir: "Long", status: Live, vol: false,
            rules: &["F|Chaos Green BL = Ref(Wilders(MP(),5),-3); Chaos Blue BL = Ref(Wilders(MP(),13),-8) (Alligator Indicators, same collection)",
                     "E|Enter Long: Cross(C,Fml(\"chaos green bl\")) AND Fml(\"chaos green bl\") > Fml(\"chaos blue bl\")", "X|Close Long: Cross(Fml(\"chaos green bl\"),C) AND Fml(\"chaos blue bl\") > Fml(\"chaos green bl\")"],
            notes: "guppymsformulas.pdf p. 174 (Alligator lines p. 83/173). MP() = (H+L)/2. Wilders = Wilder smoothing seeded with a simple mean." },
        S { id: "dgmf_vbo_taylor", name: "Volatility Breakout System (Sean Taylor)", group: "Formula 17", dir: "Long", status: Live, vol: false,
            rules: &["E|Buy:=O+((Ref(H,-1)-Ref(C,-1))*.7); Trade:= O<Ref(C,-1) AND H>Buy AND Ref(C,-1)<Ref(C,-2) AND Ref(C,-2)<Ref(C,-3) AND Ref(C,-3)<Ref(C,-4)",
                     "X|Sell:=C; Stop:=O; HitStop:= If(Trade AND Stop>=Sell,TRUE,FALSE); exit at Stop if HitStop else at Sell"],
            notes: "guppymsformulas.pdf p. 255-256. Same-day trade: buy stop at Buy, out at the close, or at the open price when the close is at/below the open. Commission:=21 / TradeAmt:=10000 not applied (the engine's cost settings apply)." },
        // ---- data gaps
        S { id: "dgmf_ipi", name: "Investor Preference Index system (Cyril Smith, TASC Dec 1997)", group: "Formula 4", dir: "Long", status: NeedsData("needs the NYSE Composite index (P) alongside the S&P 500"), vol: false,
            rules: &["F|(Sum(Mov(ROC(Log(C),24,%)-ROC(Log(P),24,%),15,S)-Mov(ROC(Log(C),24,%)-ROC(Log(P),24,%),38,S),54)+1)*100, C=S&P 500, P=NYSE index", "E|Enter Long C=HHV(C,26)", "X|Close Long Fml(\"Investor Preference Index\")<97.6 AND ROC(Fml(\"Investor Preference Index\"),2,$)<=(-.04)"],
            notes: "guppymsformulas.pdf p. 70. Not coded: no NYSE Composite series in the vault." },
        S { id: "dgmf_25x25_bond", name: "\"25 x 25\" Bond Trading System", group: "Formula 8", dir: "Long", status: NeedsData("US T-bond futures (continuous, back-adjusted, day session) - rules are in bond points (2.5 pt stop, 18/32 entry offset)"), vol: false,
            rules: &["E|setup: PDI(14) > MDI(14) AND ADX(14) > 20 AND RSI(4) < 50; buy stop C + .5625 next day", "X|stops: LLV(L,25) until day 24 then LLV(L,2); entry - 2.5; LLV(L,2) once profit > 5*Mov(ATR(1),45)"],
            notes: "guppymsformulas.pdf p. 138-146. Printed with a 1988-1998 bond trade list. Not coded." },
        // ---- not testable (seen, with reason)
        S { id: "dgmf_widner_sr", name: "Automatic Support and Resistance system (Mel Widner, TASC May 1998)", group: "Formula 4", dir: "Long", status: NotTestable("Exits are MetaStock stop settings only ('Breakeven stop: Floor level at 2%', 'Trailing stop: Profit risk of 10 Percent, ignoring 10 periods', 'Maximum loss 7%'); the source does not define how the breakeven floor and profit-risk trail are measured or filled."), vol: false,
            rules: &["E|Enter Long: Fml(\"WSO\") > Mov( Fml(\"WSO\") , 4 , S ) OR Mov( Fml(\"WRO\") , 30 , S ) > 95"], notes: "guppymsformulas.pdf p. 77-78. Not coded." },
        S { id: "dgmf_shark32", name: "Shark-32 (Walter Downs, TASC Oct 1998)", group: "Formula 7", dir: "Long/Short", status: NotTestable("Expert symbols only: buy/sell breakout of the shark pattern; no exit rule ('Seems like entry but no exits in Shark')."), vol: false,
            rules: &["E|Buy:= Buyok=1 AND Ref(Chk,-1)=0 AND ValidChk=1 (close crosses the pattern high within 25 bars)"], notes: "guppymsformulas.pdf p. 112-114. Not coded." },
        S { id: "dgmf_greening", name: "Jim Greening MetaStock Profit system tests 01-05 (R2/Regress Slope, Tema Binary Wave, Tema PDI-MDI, Tema PV Binary Wave, Tema StochRSI)", group: "Formulas 5, 7, 19", dir: "Long/Short", status: NotTestable("Thresholds are optimisation variables (opt1/opt2 given only as ranges); the '05_Tema PV Binary Wave' zero-cross test has entries but no exits."), vol: false,
            rules: &["E|e.g. Open Long: Alert(Cross(Fml(\"Tema PDI - MDI\"),opt1),13) AND ...; Opt1: Min = -1 Max = -5 Step = 2"], notes: "Not coded." },
        S { id: "dgmf_optimised", name: "Optimisation-only systems (Forecast Oscillator basic, TSF Optimised, RMTA/TOSC, Bianchi, Bull/Bear Fear with DX, J2L, Linear Regression (Ledermann))", group: "Formulas 2, 5, 8, 11, 17, 20", dir: "Long/Short", status: NotTestable("Every threshold/length is an optimisation variable (opt1..opt5) given only as a range; the book fixes no value."), vol: false,
            rules: &["E|e.g. Enter Long: Cross(opt1,ForecastOsc(CLOSE,opt3)); Close Long: Cross(ForecastOsc(CLOSE,opt3),opt2)"], notes: "Not coded." },
        S { id: "dgmf_builtin_patterns", name: "Systems on MetaStock built-in candle patterns (Boomers watchsig, Hammer/Hanging Man + CCI, Three-day EngulfingBear short)", group: "Formulas 9, 14, 17", dir: "Long/Short", status: NotTestable("Use BullHarami(), EngulfingBear() or an undefined Fml(\"Candle - Hammer\"); the collection never gives their definitions."), vol: false,
            rules: &["E|e.g. ENTER SHORT Engulfingbear(); EXIT SHORT Ref(Engulfingbear, -3)"], notes: "Not coded." },
        S { id: "dgmf_no_exit", name: "Entry-only systems and explorations (200 dma violated 50%/100%, Jeff Cooper 180s/Lizards/Slingshots/Whoops/1234s/Boomers/Expansion Pivots/Gilligan's, Connors-Raschke HV exploration, SimpleX, Buy trigger, Double stochastics expert, DeMark Sequential experts, Ergodic rules, Weekly TRIX test, SAR/KST/MACD explorations)", group: "various", dir: "Long/Short", status: NotTestable("No exit is given (or the close rule is missing from the printed text); an exit is never added. Cooper and Connors-Raschke items also duplicate vault modules."), vol: false,
            rules: &["E|e.g. 200 dma violated by 50%: enter short c>=(mov(c,200,s)*1.5) - no exit printed"], notes: "Not coded." },
        S { id: "dgmf_exit_tools", name: "Exit-only tools (Chandelier exits, Channel exit with stop loss, ATR / trailing stop-loss indicators, Trading the Trend, Countback line)", group: "various", dir: "Long/Short", status: NotTestable("Exits without an entry rule."), vol: false,
            rules: &["X|e.g. Chandelier: highest high since entry minus 3 ATR"], notes: "Not coded." },
        S { id: "dgmf_dbs_dup", name: "DBS-System (Pruitt / Stridsman dynamic breakout)", group: "Formula 9", dir: "Long/Short", status: NotTestable("Duplicate of the vault's Dynamic Break Out modules (strtw_dbs*, pbwts_dbs2); the collection's adaptive LookBack also relies on PREV/LastValue."), vol: false,
            rules: &["E|Enter Long: H > Ref(Fml(\"BuyBreak\"), -1)", "X|Close Long: L < Ref(Fml(\"BuyExit\"), -1)"], notes: "Skipped as duplicate." },
        S { id: "dgmf_misc_nt", name: "DMTF Trading System (Krausz), Divergence trading system (Arnoldi), Anti Trigger (Raschke)", group: "Formulas 10, 8, 22", dir: "Long/Short", status: NotTestable("DMTF: its author says 'the close is not defined' and it calls unprinted Equis formulas; Divergence: buy/sell text is garbled by the two-column page and has no position rules; Anti Trigger: a state indicator whose stop lines are commented out."), vol: false,
            rules: &["E|e.g. DMTF Enter Long: Fml(\"dynamic balance\")>Fml(\"dynamik balance point steps\") AND ..."], notes: "Not coded." },
        S { id: "dgmf_gmma", name: "Guppy Multiple Moving Average (GMMA) trading - Guppy's own method", group: "GMMA indicators/exploration; 'Better Trading with the GMMA' workbook", dir: "Long/Short", status: NotTestable("Guppy's GMMA rules are chart reading: 'compression', 'separation', 'bubble', 'enter when shortest of averages begin to turn up', 'stop on lower long term group' - no thresholds or formulas."), vol: false,
            rules: &["F|short group EMA 3,5,8,10,12,15; long group EMA 30,35,40,45,50,60", "E|'Conclusion: Enter when shortest of averages begin to turn up' (workbook slide 'GMMA Analysis steps')"], notes: "Not coded." },
        S { id: "dgmf_indicators", name: "All remaining formulas (about 450 indicators, explorations, experts and how-to notes)", group: "whole collection", dir: "Long/Short", status: NotTestable("Indicators, explorations, expert highlights or MetaStock how-to text with no entry and exit rules."), vol: false,
            rules: &["F|see library/dgmf/README.md for the inventory"], notes: "Seen; nothing to test." },
    ]
}

pub fn catalogue() -> Vec<StrategyDef> {
    defs().into_iter().map(|s| StrategyDef {
        id: s.id, name: s.name, group: s.group, session: 0, dir: s.dir, status: s.status, needs_volume: s.vol, needs_vix: false,
        needs_series: &[], book: BOOK, rules: s.rules, notes: if s.status == Status::Live { leak(format!("{}{}", s.notes, MS)) } else { s.notes }, claim: None,
    }).collect()
}
fn leak(s: String) -> &'static str { Box::leak(s.into_boxed_str()) }

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    let d = defs();
    let s = d.iter().find(|s| s.id == id)?;
    let fam = match id {
        "dgmf_200dma_1pt" | "dgmf_200dma_3pt" => ("dgmf_200dma", "200-day MA violated (Formula 14)"),
        "dgmf_tezel_ma" | "dgmf_tezel_reg" => ("dgmf_tezel", "Tezel asymmetric volatility bands"),
        "dgmf_gap1" | "dgmf_gap2" | "dgmf_gap3" | "dgmf_gapup_5d" => ("dgmf_gap", "Gap systems"),
        "dgmf_fosc_alt" | "dgmf_fosc_sys" => ("dgmf_fosc", "Forecast Oscillator systems"),
        _ => (s.id, s.name),
    };
    let variant = match s.status { Status::Live => "MetaStock code, literal", Status::NeedsData(_) => "data missing", Status::NotTestable(_) => "not testable" };
    Some((fam.0, fam.1, variant))
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    Some(match id {
        "dgmf_200dma_1pt" => vec![p("ma", "MA length", 200., 100., 300., 20.), p("pts", "Points beyond MA", 1., 0., 10., 1.), p("llv", "Exit LLV length", 15., 5., 30., 5.), p("off", "Exit offset (points)", 0.5, 0., 5., 0.5), p("hhv", "Exit HHV length", 10., 5., 30., 5.), p("frac", "Give-back fraction of HHV", 0.75, 0.5, 0.95, 0.05)],
        "dgmf_200dma_3pt" => vec![p("ma", "MA length", 200., 100., 300., 20.), p("pts", "Points beyond MA", 3., 0., 10., 1.), p("llv", "Exit LLV/HHV length", 15., 5., 30., 5.), p("off", "Exit offset (points)", 0.5, 0., 5., 0.5)],
        "dgmf_21d_rev_di" => vec![p("n", "Breakout length", 21., 10., 40., 1.)],
        "dgmf_stoch_ma" => vec![p("k", "Stoch range", 55., 20., 80., 5.), p("s", "Stoch slowing", 21., 3., 30., 1.), p("w", "WMA", 5., 3., 10., 1.), p("hi", "Upper level", 75., 60., 90., 5.), p("lo", "Lower level", 20., 10., 40., 5.), p("sh", "Short level", 70., 60., 90., 5.)],
        "dgmf_bb7" => vec![p("n", "Band length", 20., 10., 40., 1.), p("k", "Band SDs", 2., 1., 3., 0.5), p("lag", "Lookback bars", 7., 3., 15., 1.)],
        "dgmf_ema_5_21" => vec![p("fast", "Fast EMA", 5., 2., 20., 1.), p("slow", "Slow EMA", 21., 10., 60., 1.)],
        "dgmf_boomers_buysig" => vec![p("adx", "ADX threshold", 30., 20., 40., 5.), p("llv", "Exit LLV length", 15., 5., 30., 5.), p("frac", "Give-back fraction", 0.75, 0.5, 0.95, 0.05)],
        "dgmf_evans_ema" => vec![p("close_ma", "Close-long EMA (13 book, 40 book alternative)", 13., 13., 40., 27.)],
        "dgmf_tezel_ma" => vec![p("n", "Band length", 11., 5., 30., 1.), p("up", "Upper SDs", 1.7, 1., 3., 0.1), p("dn", "Lower SDs", 2., 1., 3., 0.1), p("stop", "Max loss %", 10., 5., 20., 1.), p("vol_min", "Min 3-day avg volume (shares)", 200000., 0., 1e6, 1e5)],
        "dgmf_tezel_reg" => vec![p("n", "Regression length", 21., 10., 40., 1.), p("dn", "Lower std errors", 1.5, 1., 3., 0.1), p("corr", "Correlation rise", 0.2, 0.05, 0.5, 0.05), p("stop", "Max loss %", 10., 5., 20., 1.)],
        "dgmf_gapup_5d" => vec![p("hold", "Bars after gap", 5., 1., 20., 1.)],
        "dgmf_roc_ma" => vec![p("fast", "Fast EMA", 12., 5., 30., 1.), p("slow", "Slow EMA", 60., 30., 120., 5.)],
        "dgmf_brow_dmi" => vec![p("n", "Channel length", 21., 10., 50., 1.)],
        "dgmf_ccifp" => vec![p("lvl", "CCI level", 100., 50., 200., 25.)],
        "dgmf_fosc_alt" | "dgmf_fosc_sys" => vec![p("n", "Forecast length", 21., 5., 50., 1.), p("sig", "Signal EMA", 3., 2., 10., 1.), p("exit_lvl", "Exit level", 6., 0., 10., 1.)],
        "dgmf_karnish_bbh" => vec![p("n", "Band length", 20., 10., 40., 1.)],
        "dgmf_cci_spike" => vec![p("n", "CCI length", 13., 5., 30., 1.), p("ratio", "Spike ratio", 1.5, 1., 3., 0.25), p("lo", "Entry CCI below", -25., -100., 0., 25.), p("hi", "Exit CCI above", 200., 100., 300., 25.)],
        "dgmf_gap2" | "dgmf_gap3" => vec![p("n1", "Gap lookback", 5., 1., 20., 1.), p("n2", "Exit lookback (Gap 3)", 3., 1., 20., 1.)],
        "dgmf_katz_cyclical" => vec![p("m", "Cycle period", 63., 20., 120., 1.), p("k", "ROC comparison period", 3., 1., 10., 1.), p("thresh", "Threshold multiple", 4., 0., 6., 0.5), p("hld", "Holding bars", 10., 1., 30., 1.)],
        "dgmf_maas_rsi5" => vec![p("rsi", "RSI length", 5., 2., 14., 1.), p("hold", "Max bars", 10., 3., 20., 1.)],
        "dgmf_disp_ema" => vec![p("n", "EMA length", 50., 10., 100., 5.), p("disp", "Displacement", 15., 0., 30., 1.)],
        "dgmf_stoch_cross" => vec![p("buy", "Buy below", 36., 20., 50., 2.), p("sell", "Sell above", 52., 40., 70., 2.), p("w3", "Exit WMA level", 55., 40., 80., 5.)],
        "dgmf_triple_ma" => vec![p("s", "Short MA", 25., 10., 50., 5.), p("m", "Medium MA", 75., 50., 150., 5.), p("l", "Long MA", 200., 100., 300., 10.)],
        "dgmf_williams_exp" => vec![],
        "dgmf_vbo_taylor" => vec![p("mult", "Range fraction", 0.7, 0.3, 1., 0.1)],
        "dgmf_gap1" => vec![],
        _ => return None,
    })
}

// ------------------------------------------------------------------ simulator
struct Sig { el: Vec<bool>, xl: Vec<bool>, es: Vec<bool>, xs: Vec<bool> }
#[derive(Clone, Copy, Default)]
struct Exec { entry_open: bool, exit_open: bool, max_loss: Option<f64>, hold: Option<usize> }

fn new_trade(dir: f64, i: usize, px: f64, sig: usize, kind: &'static str) -> Trade {
    Trade { dir, legs: vec![Leg { i, px, w: 1. }], exit_i: i, exit_px: px, reason: String::new(), entry_kind: kind, signal_i: sig, open: false, exits: vec![] }
}

fn simulate(x: &Ctx, from: usize, to: usize, s: &Sig, e: Exec) -> Vec<Trade> {
    let mut out = vec![];
    let mut cur: Option<Trade> = None;
    let mut entry_sig = 0usize; // bar of the entry signal
    let mut pend_exit = false;
    let mut pend_entry: Option<(f64, usize)> = None;
    for i in from..=to {
        // 1. orders at the open
        if pend_exit {
            if let Some(mut t) = cur.take() { t.exit_i = i; t.exit_px = x.o[i]; t.signal_i = i - 1; t.reason = "Close rule · next open".into(); out.push(t); }
            pend_exit = false;
        }
        if let Some((d, sg)) = pend_entry.take() { cur = Some(new_trade(d, i, x.o[i], sg, "open")); entry_sig = sg; }
        // 2. intraday max-loss stop
        let mut exited_dir = 0.;
        if let (Some(ml), Some(t)) = (e.max_loss, cur.as_ref()) {
            let ep = t.legs[0].px;
            let (hit, px) = if t.dir > 0. { let st = ep * (1. - ml); (x.l[i] <= st, x.o[i].min(st)) } else { let st = ep * (1. + ml); (x.h[i] >= st, x.o[i].max(st)) };
            if hit && !(t.entry_kind == "close" && t.legs[0].i == i) {
                let mut t = cur.take().unwrap();
                t.exit_i = i; t.exit_px = px; t.signal_i = i; t.reason = "Maximum loss stop".into(); exited_dir = t.dir; out.push(t);
            }
        }
        // 3. close-of-bar rules
        if let Some(t) = cur.as_ref() {
            let live = if t.entry_kind == "close" { i > t.legs[0].i } else { i >= t.legs[0].i };
            let rule = if t.dir > 0. { s.xl[i] } else { s.xs[i] };
            let timed = e.hold.is_some_and(|h| i >= entry_sig + h);
            if live && (rule || timed) {
                let why = if rule { "Close rule" } else { "Bars in trade" };
                if e.exit_open {
                    if i < to { pend_exit = true; exited_dir = t.dir; }
                } else {
                    let mut t = cur.take().unwrap();
                    t.exit_i = i; t.exit_px = x.c[i]; t.signal_i = i; t.reason = format!("{why} · close"); exited_dir = t.dir; out.push(t);
                }
            }
        }
        // 4. entries (flat, or exiting; never the same direction on the exit bar)
        if (cur.is_none() || pend_exit) && pend_entry.is_none() {
            let want = if s.el[i] && exited_dir <= 0. { 1. } else if s.es[i] && exited_dir >= 0. { -1. } else { 0. };
            if want != 0. {
                if e.entry_open {
                    if i < to { pend_entry = Some((want, i)); }
                } else if cur.is_none() {
                    cur = Some(new_trade(want, i, x.c[i], i, "close")); entry_sig = i;
                }
            }
        }
    }
    if let Some(mut t) = cur { t.exit_i = to; t.exit_px = x.c[to]; t.signal_i = to; t.reason = "open".into(); t.open = true; out.push(t); }
    out
}

fn bools(n: usize, f: impl Fn(usize) -> bool) -> Vec<bool> { (0..n).map(f).collect() }

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    let schema = params(id)?;
    if from >= to || to >= x.r.len() { return None; }
    if defs().iter().find(|s| s.id == id).map(|s| s.status) != Some(Status::Live) { return None; }
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).map_or(NAN, |d| d.def));
    let u = |k: &str| g(k).round().max(1.) as usize;
    let n = x.c.len();
    let (c, h, l, o) = (&x.c, &x.h, &x.l, &x.o);
    let close = Exec::default();
    let no = vec![false; n];
    let (sig, ex): (Sig, Exec) = match id {
        "dgmf_200dma_1pt" | "dgmf_200dma_3pt" => {
            let ma = sma(c, u("ma")); let pts = g("pts"); let off = g("off");
            let llv15 = llv(c, u("llv"));
            let hh_ll = hhv(&llv15, u("llv"));
            let el = if id == "dgmf_200dma_1pt" { bools(n, |i| fin(ma[i]) && c[i] >= ma[i] + pts) } else { no.clone() };
            let xl = if id == "dgmf_200dma_1pt" {
                let hh = hhv(c, u("hhv")); let fr = g("frac");
                bools(n, |i| i >= 1 && ((fin(llv15[i - 1]) && c[i] <= llv15[i - 1] - off) || (fin(hh[i]) && c[i] <= fr * hh[i])))
            } else { no.clone() };
            let es = bools(n, |i| fin(ma[i]) && c[i] <= ma[i] - pts);
            let xs = bools(n, |i| fin(hh_ll[i]) && c[i] >= hh_ll[i] + off);
            (Sig { el, xl, es, xs }, close)
        }
        "dgmf_21d_rev_di" => {
            let k = u("n");
            let (hh, ll) = (back(&hhv(c, k), 1), back(&llv(c, k), 1));
            let (_, _, adx1) = dmi(x, 1); let (_, _, adx14) = dmi(x, 14);
            let (pdi9, mdi9, _) = dmi(x, 9); let (pdi14, mdi14, _) = dmi(x, 14);
            let up = |i: usize| fin(hh[i]) && c[i] > hh[i]; let dn = |i: usize| fin(ll[i]) && c[i] < ll[i];
            (Sig { el: bools(n, |i| up(i) && adx1[i] > adx14[i] && pdi9[i] > mdi14[i]), xl: bools(n, |i| dn(i) && pdi14[i] < mdi9[i]),
                   es: bools(n, |i| dn(i) && adx1[i] > adx14[i] && pdi9[i] < mdi14[i]), xs: bools(n, |i| up(i) && pdi9[i] > mdi14[i]) }, close)
        }
        "dgmf_stoch_ma" => {
            let s = wma(&stoch(x, u("k"), u("s")), u("w")); let (hi, lo, sh) = (g("hi"), g("lo"), g("sh"));
            let ok = |i: usize| i >= 1 && fin(s[i]) && fin(s[i - 1]);
            let el = bools(n, |i| ok(i) && s[i] > s[i - 1] && s[i] < hi && s[i] > lo);
            (Sig { xl: bools(n, |i| ok(i) && s[i] < hi && s[i - 1] > hi), es: bools(n, |i| ok(i) && s[i] < sh && s[i - 1] > sh && s[i] < s[i - 1]), xs: el.clone(), el }, close)
        }
        "dgmf_bb7" => {
            let (m, sdv, k, lag) = (sma(c, u("n")), sd(c, u("n")), g("k"), u("lag"));
            let lower = zip(&m, &sdv, |a, b| a - k * b); let upper = zip(&m, &sdv, |a, b| a + k * b);
            let r = rsi(c, 14); let (rh, rl) = (hhv(&r, 14), llv(&r, 14));
            let raw: Vec<f64> = (0..n).map(|i| { let d = rh[i] - rl[i]; if fin(d) && d > 0. { (r[i] - rl[i]) / d } else { NAN } }).collect();
            let srsi: Vec<f64> = ema(&raw, 14).iter().map(|v| v * 100.).collect();
            let m89 = sma(c, 89);
            let el = bools(n, |i| i >= lag && fin(lower[i]) && fin(lower[i - lag]) && h[i] > lower[i] && l[i - lag] < lower[i - lag]);
            let xl = bools(n, |i| i >= lag.max(3) && fin(upper[i]) && fin(upper[i - lag]) && fin(srsi[i]) && fin(srsi[i - 3]) && fin(m89[i])
                && c[i] < upper[i] && c[i - lag] > upper[i - lag] && srsi[i] < 70. && srsi[i - 3] > 70. && upper[i] > m89[i] + 0.062 * m89[i]);
            (Sig { el, xl, es: no.clone(), xs: no.clone() }, close)
        }
        "dgmf_ema_5_21" => {
            let (f, s) = (ema(c, u("fast")), ema(c, u("slow")));
            (Sig { el: bools(n, |i| cross(&f, &s, i)), xl: bools(n, |i| i >= 1 && fin(f[i - 1]) && f[i] < s[i] && f[i - 1] >= s[i - 1]), es: no.clone(), xs: no.clone() }, close)
        }
        "dgmf_boomers_buysig" => {
            let (_, _, a14) = dmi(x, 14); let (p27, m27, a27) = dmi(x, 27);
            let ll = llv(c, u("llv")); let hh = hhv(c, 10); let fr = g("frac"); let th = g("adx");
            (Sig { el: bools(n, |i| fin(a14[i]) && fin(a27[i]) && (a14[i] + a27[i]) / 2. > th && p27[i] > m27[i]),
                   xl: bools(n, |i| i >= 1 && ((fin(ll[i - 1]) && c[i] <= ll[i - 1] - 0.5) || (fin(hh[i]) && c[i] <= fr * hh[i]))), es: no.clone(), xs: no.clone() }, close)
        }
        "dgmf_evans_ema" => {
            let (e5, e13, e40) = (ema(c, 5), ema(c, 13), ema(c, 40)); let ec = ema(c, u("close_ma"));
            (Sig { el: bools(n, |i| fin(e40[i]) && e5[i] > e13[i] && e13[i] > e40[i]), xl: bools(n, |i| cross(&ec, &e5, i)), es: no.clone(), xs: no.clone() }, close)
        }
        "dgmf_tezel_ma" => {
            let k = u("n"); let (m, sdv) = (sma(c, k), sd(c, k));
            let ub = zip(&m, &sdv, |a, b| a + g("up") * b); let lb = zip(&m, &sdv, |a, b| a - g("dn") * b);
            let vol: Vec<f64> = x.r.iter().map(|r| r.volume.unwrap_or(NAN)).collect();
            let (v3, h3) = (sma(&vol, 3), sma(h, 3)); let rng: Vec<f64> = (0..n).map(|i| h[i] - l[i]).collect();
            let (r3, rk) = (sma(&rng, 3), sma(&rng, k)); let ll = llv(l, k + 1); let hh = hhv(h, k + 1);
            let above: Vec<f64> = (0..n).map(|i| if fin(ub[i]) { (c[i] > ub[i]) as u8 as f64 } else { NAN }).collect();
            let below: Vec<f64> = (0..n).map(|i| if fin(lb[i]) { (c[i] < lb[i]) as u8 as f64 } else { NAN }).collect();
            let (a3, b3, b2) = (sum(&above, 3), sum(&below, 3), sum(&below, 2));
            let vm = g("vol_min");
            let el = bools(n, |i| i >= 1 && fin(ub[i]) && fin(ub[i - 1]) && (a3[i] == 3.
                || (c[i] > ub[i] && l[i - 1] > ub[i - 1]) || (l[i] > ub[i] && c[i - 1] > ub[i - 1])
                || (c[i] > ub[i] && c[i] > 1.4 * ll[i] && v3[i] > vm && h3[i] > ub[i] && r3[i] > rk[i])));
            let xl = bools(n, |i| fin(lb[i]) && (b3[i] == 3. || (c[i] < 0.82 * hh[i] && b2[i] == 2.) || (c[i] < 0.82 * hh[i] && h[i] < lb[i])));
            (Sig { el, xl, es: no.clone(), xs: no.clone() }, Exec { entry_open: true, exit_open: true, max_loss: Some(g("stop") / 100.), hold: None })
        }
        "dgmf_tezel_reg" => {
            let k = u("n"); let (v, sl, se, cor) = linreg(c, k);
            let ub = zip(&v, &se, |a, b| a + b); let lb = zip(&v, &se, |a, b| a - g("dn") * b); let hh = hhv(h, k + 1);
            let above: Vec<f64> = (0..n).map(|i| if fin(ub[i]) { (c[i] > ub[i]) as u8 as f64 } else { NAN }).collect();
            let below: Vec<f64> = (0..n).map(|i| if fin(lb[i]) { (c[i] < lb[i]) as u8 as f64 } else { NAN }).collect();
            let (a3, b3) = (sum(&above, 3), sum(&below, 3));
            let el = bools(n, |i| i >= 2 && a3[i] == 3. && sl[i] > 0. && fin(cor[i]) && fin(cor[i - 2]) && cor[i] - cor[i - 2] >= g("corr"));
            let xl = bools(n, |i| fin(lb[i]) && (b3[i] == 3. || (c[i] < 0.82 * hh[i] && h[i] < lb[i])));
            (Sig { el, xl, es: no.clone(), xs: no.clone() }, Exec { entry_open: true, exit_open: true, max_loss: Some(g("stop") / 100.), hold: None })
        }
        "dgmf_gapup_5d" => {
            let gap = |i: usize| i >= 1 && l[i] > h[i - 1]; let hd = u("hold");
            (Sig { el: bools(n, gap), xl: bools(n, |i| i >= hd && gap(i - hd)), es: no.clone(), xs: no.clone() }, Exec { entry_open: true, exit_open: true, ..close })
        }
        "dgmf_roc_ma" => {
            let (a, b) = (roc_pct(&ema(c, u("fast")), 1), roc_pct(&ema(c, u("slow")), 1));
            let ok = |i: usize| fin(a[i]) && fin(b[i]);
            let mixed = bools(n, |i| ok(i) && ((a[i] < 0. && b[i] > 0.) || (a[i] > 0. && b[i] < 0.)));
            (Sig { el: bools(n, |i| ok(i) && a[i] > 0. && b[i] > 0.), es: bools(n, |i| ok(i) && a[i] < 0. && b[i] < 0.), xl: mixed.clone(), xs: mixed }, close)
        }
        "dgmf_brow_dmi" => {
            let (hl, lh) = (hhv(l, u("n")), llv(h, u("n")));
            (Sig { el: bools(n, |i| fin(hl[i]) && c[i] > hl[i]), xl: bools(n, |i| fin(lh[i]) && c[i] < lh[i]), es: no.clone(), xs: no.clone() }, close)
        }
        "dgmf_ccifp" => {
            let (a, b, d) = (cci(x, 8), cci(x, 13), cci(x, 21));
            let f: Vec<f64> = (0..n).map(|i| (a[i] + b[i] + d[i]) / 3.).collect();
            let lv = g("lvl"); let (pl, ml) = (konst(lv, n), konst(-lv, n));
            let rising = |i: usize| i >= 1 && f[i] > f[i - 1];
            (Sig { el: bools(n, |i| (rising(i) && cross(&f, &ml, i)) || cross(&f, &pl, i)),
                   xl: bools(n, |i| (i >= 1 && f[i] < f[i - 1] && cross(&pl, &f, i)) || cross(&ml, &f, i)), es: no.clone(), xs: no.clone() }, close)
        }
        "dgmf_fosc_alt" | "dgmf_fosc_sys" => {
            let fo = forecast_osc(c, u("n"));
            let ref_line = if id == "dgmf_fosc_alt" { ema(&fo, u("sig")) } else { ema(c, u("sig")) };
            let (zero, lvl) = (konst(0., n), konst(g("exit_lvl"), n));
            let xl = if id == "dgmf_fosc_alt" { bools(n, |i| cross(&ref_line, &fo, i) && cross(&lvl, &fo, i)) } else { bools(n, |i| cross(&ref_line, &fo, i) && cross(&lvl, &ref_line, i)) };
            (Sig { el: bools(n, |i| cross(&fo, &ref_line, i) && cross(&fo, &zero, i)), xl, es: no.clone(), xs: no.clone() }, close)
        }
        "dgmf_karnish_bbh" => {
            let k = u("n"); let (m, sdv) = (sma(c, k), sd(c, k));
            let bbh: Vec<f64> = (0..n).map(|i| if fin(sdv[i]) && sdv[i] > 0. { (c[i] + 2. * sdv[i] - m[i]) / (4. * sdv[i]) * 100. } else { NAN }).collect();
            let (zero, hund) = (konst(0., n), konst(100., n));
            let el = bools(n, |i| cross(&zero, &bbh, i)); let es = bools(n, |i| cross(&bbh, &hund, i));
            (Sig { xl: es.clone(), xs: el.clone(), el, es }, Exec { entry_open: true, exit_open: true, ..close })
        }
        "dgmf_cci_spike" => {
            let k = cci(x, u("n")); let m = sma(&k, 3);
            (Sig { el: bools(n, |i| i >= 1 && fin(m[i - 1]) && k[i - 1] / m[i - 1] > g("ratio") && cross(&k, &m, i) && k[i - 1] < g("lo")),
                   xl: bools(n, |i| cross(&m, &k, i) && k[i - 1] > g("hi")), es: no.clone(), xs: no.clone() }, close)
        }
        "dgmf_gap1" => (Sig { el: bools(n, |i| i >= 1 && l[i] > h[i - 1]), xl: bools(n, |i| i >= 1 && h[i] < l[i - 1]), es: no.clone(), xs: no.clone() }, close),
        "dgmf_gap2" | "dgmf_gap3" => {
            let (hh5, ll5) = (back(&hhv(h, u("n1")), 1), back(&llv(l, u("n1")), 1));
            let up = bools(n, |i| fin(hh5[i]) && l[i] > hh5[i]); let dn = bools(n, |i| fin(ll5[i]) && h[i] < ll5[i]);
            if id == "dgmf_gap2" { (Sig { el: up, xl: dn, es: no.clone(), xs: no.clone() }, close) } else {
                let (hh3, ll3) = (back(&hhv(h, u("n2")), 1), back(&llv(l, u("n2")), 1));
                (Sig { el: up, es: dn, xl: bools(n, |i| fin(ll3[i]) && c[i] < ll3[i]), xs: bools(n, |i| fin(hh3[i]) && c[i] > hh3[i]) }, close)
            }
        }
        "dgmf_katz_cyclical" => {
            let (m, k) = (u("m"), g("k").round().max(0.) as usize);
            let v2 = zip(&sma(c, m), &sma(c, m + k), |a, b| a - b); let v1 = sd(&v2, 20);
            let tv: Vec<f64> = v1.iter().map(|v| g("thresh") * v).collect(); let hd = u("hld");
            (Sig { el: bools(n, |i| fin(tv[i]) && v2[i] > tv[i]), es: bools(n, |i| fin(tv[i]) && v2[i] < tv[i]),
                   xl: bools(n, |i| i >= hd && cross(&v2, &tv, i - hd)), xs: bools(n, |i| i >= hd && cross(&tv, &v2, i - hd)) }, close)
        }
        "dgmf_maas_rsi5" => {
            let r = rsi(c, u("rsi")); let m5 = sma(c, 5);
            (Sig { el: bools(n, |i| i >= 5 && fin(r[i - 1]) && fin(m5[i]) && r[i] > r[i - 1] && c[i] < c[i - 5] && c[i] <= m5[i]),
                   xl: bools(n, |i| fin(m5[i]) && c[i] > m5[i]), es: no.clone(), xs: no.clone() }, Exec { exit_open: true, hold: Some(u("hold")), ..close })
        }
        "dgmf_disp_ema" => {
            let d = back(&ema(c, u("n")), g("disp").round().max(0.) as usize);
            (Sig { el: bools(n, |i| cross(c, &d, i)), xl: bools(n, |i| cross(&d, c, i)), es: no.clone(), xs: no.clone() }, close)
        }
        "dgmf_stoch_cross" => {
            let s = stoch(x, 12, 3); let (w6, w3) = (wma(&s, 6), wma(&s, 3));
            (Sig { el: bools(n, |i| fin(s[i]) && s[i] < g("buy") && cross(&s, &w6, i)),
                   xl: bools(n, |i| (fin(s[i]) && s[i] > g("sell") && cross(&w3, &s, i)) || (fin(w3[i]) && w3[i] > g("w3"))), es: no.clone(), xs: no.clone() }, close)
        }
        "dgmf_triple_ma" => {
            let (s, m, lg) = (sma(c, u("s")), sma(c, u("m")), sma(c, u("l")));
            let el = bools(n, |i| cross(&s, &m, i) && fin(lg[i]) && lg[i] > m[i]);
            let es = bools(n, |i| cross(&m, &s, i) && fin(lg[i]) && lg[i] < m[i]);
            (Sig { xl: es.clone(), xs: el.clone(), el, es }, Exec { entry_open: true, exit_open: true, ..close })
        }
        "dgmf_williams_exp" => {
            let mp: Vec<f64> = (0..n).map(|i| (h[i] + l[i]) / 2.).collect();
            let (gr, bl) = (back(&wilder(&mp, 5), 3), back(&wilder(&mp, 13), 8));
            (Sig { el: bools(n, |i| cross(c, &gr, i) && gr[i] > bl[i]), xl: bools(n, |i| cross(&gr, c, i) && fin(bl[i]) && bl[i] > gr[i]), es: no.clone(), xs: no.clone() }, close)
        }
        "dgmf_vbo_taylor" => {
            let mult = g("mult");
            let mut out = vec![];
            for i in from.max(4)..=to {
                let buy = o[i] + (h[i - 1] - c[i - 1]) * mult;
                if o[i] < c[i - 1] && h[i] > buy && c[i - 1] < c[i - 2] && c[i - 2] < c[i - 3] && c[i - 3] < c[i - 4] {
                    let mut t = new_trade(1., i, buy, i, "stop");
                    let stopped = o[i] >= c[i];
                    t.exit_px = if stopped { o[i] } else { c[i] }; t.exit_i = i; t.signal_i = i;
                    t.reason = if stopped { "Stop at the open price" } else { "Sell at the close" }.into();
                    out.push(t);
                }
            }
            return Some(out);
        }
        _ => return None,
    };
    Some(simulate(x, from, to, &sig, ex))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;
    fn rows(c: &[f64]) -> Vec<Row> {
        c.iter().enumerate().map(|(i, &v)| Row { ticker: "T".into(), date: format!("{:04}-{:02}-{:02}", 2000 + i / 300, 1 + (i / 25) % 12, 1 + i % 25), open: v, high: v + 1., low: v - 1., close: v, volume: Some(1e6) }).collect()
    }
    #[test]
    fn indicators_match_hand_values() {
        let e = ema(&[1., 2., 3., 4., 5.], 3); assert!(e[1].is_nan()); assert_eq!(e[2], 2.); assert_eq!(e[3], 3.); assert_eq!(e[4], 4.);
        let w = wma(&[1., 2., 3.], 3); assert!((w[2] - 14. / 6.).abs() < 1e-12);
        let (v, s, se, co) = linreg(&[1., 2., 3., 4.], 4); assert!((v[3] - 4.).abs() < 1e-12 && (s[3] - 1.).abs() < 1e-12 && se[3].abs() < 1e-9 && (co[3] - 1.).abs() < 1e-12);
        assert!(cross(&[0., 2.], &[1., 1.], 1) && !cross(&[2., 2.], &[1., 1.], 1));
    }
    #[test]
    fn ema_cross_enters_and_exits_on_closes() {
        let mut c = vec![100.; 40]; c.extend((0..15).map(|k| 101. + k as f64)); c.extend((0..15).map(|k| 110. - 3. * k as f64));
        let r = rows(&c); let x = Ctx::new(&r, &r, &[]);
        let t = run("dgmf_ema_5_21", &x, 25, r.len() - 1, &Params::new()).unwrap();
        assert_eq!(t[0].legs[0].i, 40); assert_eq!(t[0].legs[0].px, 101.); assert!(t[0].exit_i > 55 && !t[0].open);
    }
    #[test]
    fn next_open_reversal_and_stop() {
        let r = rows(&[100., 100., 100., 100.]); let x = Ctx::new(&r, &r, &[]);
        let s = Sig { el: vec![true, false, false, false], xl: vec![false, true, false, false], es: vec![false, true, false, false], xs: vec![false; 4] };
        let t = simulate(&x, 0, 3, &s, Exec { entry_open: true, exit_open: true, ..Default::default() });
        assert_eq!(t.len(), 2); assert_eq!((t[0].legs[0].i, t[0].exit_i), (1, 2)); assert_eq!((t[1].dir, t[1].legs[0].i), (-1., 2)); assert!(t[1].open);
        let mut r = rows(&[100., 100., 100., 100.]); r[2].low = 80.; r[2].open = 95.; let x = Ctx::new(&r, &r, &[]);
        let s = Sig { el: vec![true, false, false, false], xl: vec![false; 4], es: vec![false; 4], xs: vec![false; 4] };
        let t = simulate(&x, 0, 3, &s, Exec { entry_open: true, exit_open: true, max_loss: Some(0.1), hold: None });
        assert_eq!((t[0].exit_i, t[0].exit_px), (2, 90.));
    }
    #[test]
    fn every_live_strategy_runs_and_non_live_do_not() {
        let c: Vec<f64> = (0..600).map(|i| 100. + 10. * ((i as f64) / 17.).sin() + (i as f64) * 0.05).collect();
        let r = rows(&c); let x = Ctx::new(&r, &r, &[]);
        for s in defs() {
            let res = run(s.id, &x, 250, 599, &Params::new());
            assert_eq!(res.is_some(), s.status == Status::Live, "{}", s.id);
            for t in res.unwrap_or_default() { assert!(t.legs[0].i >= 250 && t.exit_i <= 599 && t.exit_i >= t.legs[0].i, "{}", s.id); }
        }
    }
}

// Instrument admission: Guppy MetaStock Formula Index 2026-10-26 (lane B) block, moved from instrument_policy.rs on 31 Oct 2026 (lane ST).
// Admission only for the module REVIEWED lists (authors/guppyf/laneb/SOURCE-CONTRACT.md); kept in one block to merge.
// Runs on actual indices, ETFs and stocks, US and India (index_etf_or_stock, nifty50 basket, declared application).
pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book=="dgmf" && crate::books::dgmf::reviewed(def.id),
    label: "MetaStock System Tester code with no per-market test the vault can compare; runs on indices, ETFs and single stocks (US and India) are a declared application, one unit per trade, fills as stated on the card.",
    basis: "Daryl Guppy / guppytraders.com, MetaStock Formula Index (2002)",
};
