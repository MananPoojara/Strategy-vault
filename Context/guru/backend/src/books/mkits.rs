//! Markos Katsanos, *Intermarket Trading Strategies* (Wiley, 2008). Rules = the book's MetaStock code (Appendix A,
//! pp.297-355); where the prose differs, the code wins and the difference is noted. Source + checks: library/mkits/.
//! Intermarket systems run only when the second market is in the vault (here: the S&P 500 benchmark of Chapter 16);
//! every other intermarket system is NeedsData with the exact missing series. No proxies.
use crate::connors::{sma, Claim, Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "mkits";
/// Admitted ids (authors/katsanos/admission/SOURCE-CONTRACT.md).
pub const REVIEWED: &[&str] = &["mkits_dax_ma", "mkits_euro_ma"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }
const SPX: &[&str] = &["SPX_CLOSE"];

// ---------------------------------------------------------------- MetaStock functions (NaN = not yet defined)

/// Mov(x,n,E): EMA with alpha 2/(n+1). [B] seeded with the SMA of the first n defined values.
pub fn ema(v: &[f64], n: usize) -> Vec<f64> { ema_a(v, n, 2.0 / (n as f64 + 1.0)) }
fn ema_a(v: &[f64], n: usize, a: f64) -> Vec<f64> {
    let mut out = vec![f64::NAN; v.len()];
    let Some(s) = v.iter().position(|x| x.is_finite()) else { return out };
    if v.len() < s + n || v[s..s + n].iter().any(|x| !x.is_finite()) { return out; }
    let mut prev = v[s..s + n].iter().sum::<f64>() / n as f64;
    out[s + n - 1] = prev;
    for i in s + n..v.len() {
        if v[i].is_finite() { prev += a * (v[i] - prev); out[i] = prev; }
    }
    out
}
/// MACD(): MetaStock = EMA(C) at 15% minus EMA(C) at 7.5% (12/26 bars, fixed percentages; MetaStock TAAZ "MACD").
pub fn macd(c: &[f64]) -> Vec<f64> {
    let (f, s) = (ema_a(c, 12, 0.15), ema_a(c, 26, 0.075));
    f.iter().zip(&s).map(|(a, b)| a - b).collect()
}
fn win(v: &[f64], i: usize, n: usize) -> Option<&[f64]> {
    (i + 1 >= n).then(|| &v[i + 1 - n..=i]).filter(|w| w.iter().all(|x| x.is_finite()))
}
pub fn hhv(v: &[f64], n: usize) -> Vec<f64> {
    (0..v.len()).map(|i| win(v, i, n).map_or(f64::NAN, |w| w.iter().copied().fold(f64::MIN, f64::max))).collect()
}
pub fn llv(v: &[f64], n: usize) -> Vec<f64> {
    (0..v.len()).map(|i| win(v, i, n).map_or(f64::NAN, |w| w.iter().copied().fold(f64::MAX, f64::min))).collect()
}
/// Stdev(x,n): [B] population standard deviation (MetaStock "Std Dev").
pub fn stdev(v: &[f64], n: usize) -> Vec<f64> {
    (0..v.len()).map(|i| win(v, i, n).map_or(f64::NAN, |w| {
        let m = w.iter().sum::<f64>() / n as f64;
        (w.iter().map(|x| (x - m).powi(2)).sum::<f64>() / n as f64).sqrt()
    })).collect()
}
/// ROC(x,n,%) = (x - x[n bars ago]) / x[n bars ago] * 100.
pub fn roc(v: &[f64], n: usize) -> Vec<f64> {
    (0..v.len()).map(|i| if i >= n { (v[i] - v[i - n]) / v[i - n] * 100.0 } else { f64::NAN }).collect()
}
/// LinRegSlope(x,n): least-squares slope of the last n values against bar number.
pub fn lrs(v: &[f64], n: usize) -> Vec<f64> {
    let xm = (n as f64 - 1.0) / 2.0;
    let sxx: f64 = (0..n).map(|k| (k as f64 - xm).powi(2)).sum();
    (0..v.len()).map(|i| win(v, i, n).map_or(f64::NAN, |w| {
        let ym = w.iter().sum::<f64>() / n as f64;
        w.iter().enumerate().map(|(k, y)| (k as f64 - xm) * (y - ym)).sum::<f64>() / sxx
    })).collect()
}
/// Correl(a,b,n,0): Pearson correlation of the last n bars.
pub fn correl(a: &[f64], b: &[f64], n: usize) -> Vec<f64> {
    (0..a.len()).map(|i| match (win(a, i, n), win(b, i, n)) {
        (Some(x), Some(y)) => {
            let (mx, my) = (x.iter().sum::<f64>() / n as f64, y.iter().sum::<f64>() / n as f64);
            let sxy: f64 = x.iter().zip(y).map(|(p, q)| (p - mx) * (q - my)).sum();
            let sx: f64 = x.iter().map(|p| (p - mx).powi(2)).sum();
            let sy: f64 = y.iter().map(|q| (q - my).powi(2)).sum();
            sxy / (sx * sy).sqrt()
        }
        _ => f64::NAN,
    }).collect()
}
/// Stoch(k,s) = 100 * Sum(C - LLV(L,k), s) / Sum(HHV(H,k) - LLV(L,k), s) (MetaStock slowed %K).
pub fn stoch(x: &Ctx, k: usize, s: usize) -> Vec<f64> {
    let (hh, ll) = (hhv(&x.h, k), llv(&x.l, k));
    let num: Vec<f64> = (0..x.c.len()).map(|i| x.c[i] - ll[i]).collect();
    let den: Vec<f64> = (0..x.c.len()).map(|i| hh[i] - ll[i]).collect();
    let (sn, sd) = (sma(&num, s), sma(&den, s));
    sn.iter().zip(&sd).map(|(a, b)| 100.0 * a / b).collect()
}
/// Congestion Index (book code): ROC(C,n-1,%) / ((HHV(H,n)-LLV(L,n)) / (LLV(L,n)+.01) + .000001).
pub fn ci(x: &Ctx, n: usize) -> Vec<f64> {
    let (r, hh, ll) = (roc(&x.c, n - 1), hhv(&x.h, n), llv(&x.l, n));
    (0..x.c.len()).map(|i| r[i] / ((hh[i] - ll[i]) / (ll[i] + 0.01) + 0.000001)).collect()
}
/// Cross(a,b): a > b today and a <= b yesterday.
fn cross(a: &[f64], b: &[f64], i: usize) -> bool { i > 0 && a[i] > b[i] && a[i - 1] <= b[i - 1] }
/// Wilder Parabolic SAR(step, max). [B] starts long on bar 0 with SAR = low[0]; SAR never inside the
/// prior two bars' range (Wilder); reversal sets SAR to the extreme point.
pub fn sar(h: &[f64], l: &[f64], step: f64, max: f64) -> Vec<f64> {
    let n = h.len();
    let mut out = vec![f64::NAN; n];
    if n < 2 { return out; }
    let (mut long, mut s, mut ep, mut af) = (true, l[0], h[0], step);
    out[0] = s;
    for i in 1..n {
        s += af * (ep - s);
        if long {
            s = s.min(l[i - 1]).min(l[i.saturating_sub(2)]);
            if l[i] < s { long = false; s = ep; ep = l[i]; af = step; }
            else if h[i] > ep { ep = h[i]; af = (af + step).min(max); }
        } else {
            s = s.max(h[i - 1]).max(h[i.saturating_sub(2)]);
            if h[i] > s { long = true; s = ep; ep = h[i]; af = step; }
            else if l[i] < ep { ep = l[i]; af = (af + step).min(max); }
        }
        out[i] = s;
    }
    out
}
/// Katsanos VFI (Appendix A.6, p.336): returns Mov(VFI,3,E) exactly as the formula's last line.
pub fn vfi(x: &Ctx, period: usize, coef: f64, vcoef: f64) -> Vec<f64> {
    let n = x.c.len();
    let v: Vec<f64> = x.r.iter().map(|r| r.volume.filter(|v| *v > 0.0).unwrap_or(f64::NAN)).collect();
    let typ: Vec<f64> = (0..n).map(|i| (x.h[i] + x.l[i] + x.c[i]) / 3.0).collect();
    let inter: Vec<f64> = (0..n).map(|i| if i > 0 { typ[i].ln() - typ[i - 1].ln() } else { f64::NAN }).collect();
    let vinter = stdev(&inter, 30);
    let mv = sma(&v, period);
    let mut raw = vec![f64::NAN; n];
    let term = |j: usize| -> f64 {
        let vave = if j > 0 { mv[j - 1] } else { f64::NAN };
        let vmax = vave * vcoef; // NaN until PERIOD volumes exist (f64::min would silently drop the NaN)
        let vc = if v[j].is_finite() && vmax.is_finite() { v[j].min(vmax) } else { f64::NAN };
        let mf = typ[j] - if j > 0 { typ[j - 1] } else { f64::NAN };
        let cut = coef * vinter[j] * x.c[j];
        if mf > cut { vc } else if mf < -cut { -vc } else if mf.is_finite() && cut.is_finite() && vc.is_finite() { 0.0 } else { f64::NAN }
    };
    let terms: Vec<f64> = (0..n).map(term).collect();
    for i in 0..n {
        let vave = if i > 0 { mv[i - 1] } else { f64::NAN };
        if let Some(w) = win(&terms, i, period) { raw[i] = w.iter().sum::<f64>() / vave; }
    }
    ema(&raw, 3)
}
/// SPX close aligned to this ticker's bars; days without an SPX bar keep the previous close (MetaStock Security()).
fn spx(x: &Ctx) -> Option<Vec<f64>> {
    let s = x.series("SPX_CLOSE")?;
    let mut last = f64::NAN;
    Some(s.iter().map(|v| { if v.is_finite() { last = *v; } last }).collect())
}

// ---------------------------------------------------------------- catalogue

const G11: &str = "Ch.11 Gold systems (GLD/gold)";
const NEED_GOLD_XAU: &str = "needs gold (GLD / gold bullion) daily OHLC as the traded market and the Philadelphia Gold & Silver Index (XAU) close; neither is in the vault";
const NEED_GOLD_DXY: &str = "needs gold (GLD / gold bullion) daily OHLC as the traded market and the US Dollar Index (DXY) close; neither is in the vault";
const NEED_GOLD_MULTI: &str = "needs gold (GLD / gold bullion) daily OHLC plus XAU, silver (SLV) and Dollar Index (DXY) closes; none is in the vault";
const NEED_T2108: &str = "needs T2108 (Worden/Telechart % of NYSE stocks above their 40-day MA) daily; the vault's NYSE breadth (advances/declines/volumes) is not T2108";
const NEED_ES: &str = "needs 15-minute S&P 500 e-mini (ES) bars and 15-minute Euro Stoxx 50 futures (FESX) bars; the vault has daily data only";
const NEED_CAC: &str = "needs CAC 40 futures (FCE) or CAC 40 index daily closes as the intermarket security (book traded DAX futures); not in the vault";
const NEED_STOXX: &str = "needs Euro Stoxx 50 futures (FESX) / index (STOXX50E) daily closes; not in the vault";
const NEED_DAXSTK: &str = "needs the 10 DAX component stocks (ALV, BAS, BAY, DAI, DBK, DTE, EOA, SIE, RWE, SAP) and CAC 40 futures closes; not in the vault";
const NEED_FTSE: &str = "needs CAC 40, Amex Oil Index (XOI) and S&P Bank Index (BIX) daily closes (book traded the FTSE 100); not in the vault";
const NEED_OIL: &str = "needs an oil-stock universe with volume, light crude futures (CL) and the XOI index closes (S&P 500 alone is in the vault)";
const NEED_GSTK: &str = "needs a gold-stock universe with volume, gold (XGLD) and XAU closes; not in the vault";
const NEED_YEN: &str = "needs USD/JPY (or yen futures) daily OHLC, the Dollar Index (DXY) and 10-year Treasury yield (TNX) closes (S&P 500 alone is in the vault)";
const NEED_EURO: &str = "needs the CRB Index and 10-year Treasury yield (TNX) daily closes (book traded EUR/USD); not in the vault";
const NN: &str = "not testable: NeuroShell-trained neural network; the trained weights/network are not printed (Appendix B gives only the training procedure), so the model cannot be reproduced exactly";

const DAX_MA_RULES: &[&str] = &[
    "F|CI:=ROC(C,39,%)/((HHV(H,40)-LLV(L,40))/(LLV(L,40)+.01)+.000001)  (40-day Congestion Index, App. A.4 p.329)",
    "E|Long: (CI>30 AND STOCH(5,3)>MOV(STOCH(5,3),3,S) AND MOV(C,15,S)>MOV(C,20,S)) OR (ABS(CI)<25 AND STOCH(5,3)>MOV(STOCH(5,3),3,S) AND LLV(STOCH(40,3),2)<30) {CONGESTION}",
    "X|Sell: (HHV(CI,3)-CI>40) OR (ABS(CI)<20 AND STOCH(5,3)<MOV(STOCH(5,3),3,S) AND HHV(STOCH(40,3),4)>85 AND STOCH(40,3)<75) {CONGESTION}",
    "E|Short: (CI<-30 AND ROC(CI,3,%)<0 AND STOCH(5,3)<MOV(STOCH(5,3),3,S) AND MOV(C,10,S)<MOV(C,20,S) AND MOV(C,2,S)<MOV(C,150,S)) OR (ABS(CI)<25 AND ROC(CI,3,%)<0 AND STOCH(5,3)<MOV(STOCH(5,3),3,S) AND HHV(STOCH(40,3),2)>70) {CONGESTION}",
    "X|Buy to cover: LLV(CI,3)-CI<-40 OR (CROSS(MACD(),MOV(MACD(),7,E)) AND C>MOV(C,7,E))",
    "X|Time exit: 60 bars (Table 13.2, p.210)",
    "F|Orders: next day at the open (MetaStock trade delay 1, buy/sell price = open, App. A.4 p.327); one contract, points test",
];
const EURO_MA_RULES: &[&str] = &[
    "F|MA = 10-day simple MA of the close; PC:=Log(MOV(C,10,S)/Ref(MOV(C,10,S),-1)); FILT:=Stdev(PC,20)",
    "F|CI:=ROC(C,40-1,%)/((HHV(H,40)-LLV(L,40))/(LLV(L,40)+.01)+.000001); CI:=Mov(CI,7,E)",
    "E|Long: MOV(C,10,S)>LLV(MOV(C,10,S),3)+.7*FILT AND C>SAR(.04,.1) AND ABS(CI)>40 AND CI>LLV(CI,3)+3  (eq. 17.1-17.2 p.281; code p.355 without its CRB/TNX line 17.3)",
    "X|Sell: C<SAR(.04,.1)  (eq. 17.7)",
    "E|Short: MOV(C,10,S)<HHV(MOV(C,10,S),3)-.7*FILT AND C<SAR(.04,.1) AND ABS(CI)>40 AND CI<HHV(CI,3)-3  (eq. 17.4-17.5)",
    "X|Buy to cover: C>SAR(.04,.1)  (eq. 17.8)",
    "F|Orders: signals executed the same day at the close (p.282); one lot",
];
const RS_FUT_RULES: &[&str] = &[
    "F|SEC2 = S&P 500 (.SPX) close; RS:=Mov((C/SEC2),150,E)  (App. A.7 p.342)",
    "E|entrytrigger:= RS>Mov(RS,120,E) AND ROC(Mov(C,25,E),2,%)>0 AND LinRegSlope(RS,20)>0 AND Correl(C,SEC2,130,0)<.5 AND Stoch(100,3)<80 AND RS>Ref(HHV(RS,250),-20)",
    "X|EXITTRIGGER:=Cross(Mov(RS,90,E),RS) OR ROC(RS,3,%)<-.5",
    "F|Tradesim: entry and exits delayed one bar, filled at the OPEN; long only; open positions closed on the last bar (p.250)",
];
const RS_ETF_RULES: &[&str] = &[
    "F|SEC2 = S&P 500 (.SPX) close; RS:=Mov((C/SEC2),150,E); VFI:=Fml(\"VFI\") (Katsanos VFI, App. A.6 p.336: period 130, coef .2, vcoef 2.5, returns Mov(VFI,3,E))",
    "E|entrytrigger:=RS>Mov(RS,120,E) AND ROC(Mov(C,25,E),2,%)>0 AND LinRegSlope(RS,20)>0 AND Correl(C,SEC2,130,0)<.5 AND Stoch(100,3)<80 AND Mov(VFI,25,E)>0  (App. A.7 p.345)",
    "X|EXITTRIGGER:=Cross(Mov(RS,90,E),RS) OR ROC(RS,3,%)<-.5 OR Mov(VFI,25,E)<0  (p.346)",
    "F|Tradesim: entry and exits delayed one bar, filled at the OPEN; long only",
];

pub fn catalogue() -> Vec<StrategyDef> {
    let d = |id, name, group, dir, status, rules, notes, needs_series: &'static [&'static str], needs_volume, claim| StrategyDef {
        id, name, group, session: 0, dir, status, needs_volume, needs_vix: false, needs_series, book: BOOK, rules, notes, claim,
    };
    let nd = |id, name, group, dir, why: &'static str, rules| d(id, name, group, dir, Status::NeedsData(why), rules, why, &[], false, None);
    let mut v = vec![
        d("mkits_dax_ma", "DAX MA Crossover System", "Ch.13 DAX futures", "Long/Short", Status::Live, DAX_MA_RULES,
          "Book: DAX futures (FDAX), out-of-sample 19-Jan-2004..2-Nov-2007 (Table 13.2 p.210, column 'MA Cross'). Single-market system -> run on the vault's indices (instrument swap). Code wins over prose: prose says the 5-day stochastic 'crossed above its 4-day MA' / 'CI dropped by 40 points or more'; code uses Stoch(5,3) > its 3-day SMA (state, not cross) and HHV(CI,3)-CI>40. [B] One position at a time: an opposite signal while in a trade is ignored (p.203: 'positions wouldn't have been realized because of an active position in the opposite direction'); if long and short both fire on one bar the long is taken. [B] Time exit = the open 60 bars after the entry bar. [B] MetaStock Mov(E) seeded with the SMA of the first n values; MACD() = 15%/7.5% EMAs; Stdev population.",
          &[], false, Some(Claim { trades: "51", win: "51.0%", avg: "EUR 1156 / trade (1 FDAX contract)", hold: "16 bars", period: "1/19/04-11/2/07, DAX futures" })),
        d("mkits_euro_ma", "Euro classic MA system (MA + CI + Parabolic)", "Ch.17 Forex", "Long/Short", Status::Live, EURO_MA_RULES,
          "Book: EUR/USD 22-Jan-2003..21-Jan-2008 (Table 17.6 p.284, column 'MA'). The book prints code only for the intermarket-enhanced version (p.355); the classic system is that code without eq. 17.3 (CRB>MA(CRB,35) and TNX<MA(TNX,5)), which the book states is the only intermarket addition (pp.280-281). Code wins over prose: prose eq. 17.2 prints 'ABS(CI) < 40' but the code (both sides) uses ABS(CI)>40 (a trend filter, as the prose explains). FILT is a log-return std-dev compared with price units (literal code; on EUR/USD ~1.3 it is a small threshold, on an index it is negligible). Single-market -> run on the vault's indices. [B] SAR: Wilder parabolic, start long on the first bar, AF 0.04 step/0.10 max; [B] exit then opposite entry may happen on the same close; an entry is not exited on its own bar.",
          &[], false, Some(Claim { trades: "62", win: "45%", avg: "EUR 28,910 net / 62 trades", hold: "18 days", period: "1/22/03-1/21/08, EUR/USD" })),
        d("mkits_rs_fut", "Relative Strength asset-allocation system - Test I (futures/indices, no volume)", "Ch.16 Relative strength", "Long", Status::Live, RS_FUT_RULES,
          "Book: per-security signals on 42 futures/international indices (incl. BSE SENSEX), benchmark S&P 500, $30,000 per trade, 16-Jul-1999..27-Nov-2007 (Table 16.4 p.256, Test I). The vault runs one security at a time against the real S&P 500 close (series SPX_CLOSE, from us_data.csv; a day without an SPX bar keeps the previous close, as MetaStock Security() does). On SPX itself RS is constant and the system cannot trade. Code wins over prose: prose says RS 'crossed above its 120-day MA' and 'made a new 250-day high' and the sell needs both the cross AND ROC<-0.5; the code uses RS>Mov(RS,120,E), RS>Ref(HHV(RS,250),-20) and an OR exit. India: SPX close of the same calendar date (known before the next NSE open, so the next-open fill has no look-ahead).",
          SPX, false, Some(Claim { trades: "50", win: "74.0%", avg: "$9,613 / trade ($30k per trade, portfolio)", hold: "495 days", period: "8/28/00-11/27/07, 42-futures portfolio" })),
        d("mkits_rs_etf", "Relative Strength asset-allocation system - Test II (ETFs, with VFI)", "Ch.16 Relative strength", "Long", Status::Live, RS_ETF_RULES,
          "Book: 118 ETFs vs the S&P 500 (Table 16.4 p.256, Test II). Needs volume for VFI (NIFTY 50 has volume from 2000; NIFTYIT/MIDCAPSELECT have none -> no trades). Same SPX_CLOSE alignment as Test I. Tradesim volume filter (max 20% of traded volume) is a sizing rule and is not modelled.",
          SPX, true, Some(Claim { trades: "139", win: "59.7%", avg: "$4,322 / trade ($30k per trade, portfolio)", hold: "218 days", period: "8/28/00-11/27/07, 118-ETF portfolio" })),
    ];
    let gold: &[(&str, &str, &'static str, &str)] = &[
        ("mkits_gold_regdiv", "Gold: Regression Divergence (XAU)", NEED_GOLD_XAU, "pp.302-303"),
        ("mkits_gold_rsdiv", "Gold: Relative Strength divergence (XAU)", NEED_GOLD_XAU, "pp.303-304"),
        ("mkits_gold_rs_dxy", "Gold: Relative Strength vs regression line (DXY)", NEED_GOLD_DXY, "pp.304-305"),
        ("mkits_gold_dis_xau", "Gold: Disparity (XAU)", NEED_GOLD_XAU, "pp.305-306"),
        ("mkits_gold_dis_dxy", "Gold: Disparity (DXY)", NEED_GOLD_DXY, "pp.306-307"),
        ("mkits_gold_lrs_xau", "Gold: LRS divergence (XAU)", NEED_GOLD_XAU, "pp.307-309"),
        ("mkits_gold_lrs_dxy", "Gold: LRS divergence (DXY)", NEED_GOLD_DXY, "pp.309-310"),
        ("mkits_gold_bb_xau", "Gold: Bollinger Band divergence (XAU)", NEED_GOLD_XAU, "pp.310-312"),
        ("mkits_gold_z_xau", "Gold: Z-Score divergence (XAU)", NEED_GOLD_XAU, "pp.312-313"),
        ("mkits_gold_z_dxy", "Gold: Z-Score divergence (DXY)", NEED_GOLD_DXY, "pp.313-314"),
        ("mkits_gold_mreg", "Gold: Multiple Regression Divergence (XAU-SLV-DXY)", NEED_GOLD_MULTI, "pp.314-316"),
        ("mkits_gold_lrs2_slv", "Gold: Double LRS divergence (SLV-DXY)", NEED_GOLD_MULTI, "pp.316-318"),
        ("mkits_gold_lrs2_xau", "Gold: Double LRS divergence (XAU-DXY)", NEED_GOLD_MULTI, "pp.318-320"),
        ("mkits_gold_reg2", "Gold: Double Regression divergence (XAU-DXY)", NEED_GOLD_MULTI, "pp.320-323"),
    ];
    for &(id, name, why, _pages) in gold {
        v.push(nd(id, name, G11, "Long/Short", why, &["F|Ch.11 / Appendix A.2 MetaStock code (buy, sell, sell short, buy to cover); orders next day at the open; GLD with gold bullion appended before 18-Nov-2004 (p.302). Full code in library/mkits/extract-mkits.md"]));
    }
    v.extend([
        nd("mkits_spy_t2108", "SPY Daily Regression System (T2108)", "Ch.12 S&P 500", "Long/Short", NEED_T2108,
           &["E|Long: regression divergence of SPY vs T2108 (400-day regression, 20-day ROC), HHV(DIVERG,2)>1 AND ALERT(CROSS(80,IM),5) AND STOCH(5,3)>MOV(STOCH(5,3),3,S) AND ROC(T2108,3,%)>0 (App. A.3 p.323)",
             "X|Sell: DIVERG<0 AND STOCH(5,3)<MOV(STOCH(5,3),4,S) AND LLV(IM,4)<15 AND IM>REF(IM,-1) AND ROC(T2108,3,%)<0",
             "E|Short: HHV(T2108,9)>72 AND T2108<60 AND STOCH(15,3)<MOV(STOCH(15,3),4,S) AND T2108<MOV(T2108,15,E) AND C<MOV(C,15,E)",
             "X|Cover: T2108-based stochastic/ATR-stop conditions (p.324). Book: SPY 18-Feb-1997..17-Sep-2007, long-only 22 trades 86% win (Table 12.1)"]),
        nd("mkits_es_15m", "S&P 500 E-mini 15-min Intraday System", "Ch.12 S&P 500", "Long/Short", NEED_ES,
           &["E|Divergence / congestion / trending entry conditions on 15-minute ES vs FESX (App. A.3 pp.324-327)"]),
        nd("mkits_dax_disparity", "DAX Daily Disparity System (CAC 40)", "Ch.13 DAX futures", "Long/Short", NEED_CAC,
           &["E|Disparity divergence DAX vs CAC 40 futures (10-day), momentum oscillator 200, time exit 80 bars (App. A.4 pp.327-329; Table 13.1)"]),
        nd("mkits_dax_ma_im", "DAX Intermarket Enhanced MA Crossover System (Euro Stoxx)", "Ch.13 DAX futures", "Long/Short", NEED_STOXX,
           &["E|DAX MA crossover plus Euro Stoxx disparity/ROC filters (App. A.4 pp.329-331; Table 13.2)"]),
        nd("mkits_dax_stocks", "DAX component stocks disparity system (Tradesim)", "Ch.13 DAX futures", "Long", NEED_DAXSTK,
           &["E|Disparity system on 10 DAX stocks, long only, 50-bar time stop, stop HHV(C,3)-2.2*ATR(10) (App. A.4 pp.331-332)"]),
        nd("mkits_ftse", "FTSE intermarket regression system (CAC, XOI, BIX)", "Ch.14 FTSE", "Long/Short", NEED_FTSE,
           &["E|Predicted FTSE from a CAC/BIX/XOI regression; momentum oscillator and stochastic triggers (App. A.5 pp.332-335)"]),
        nd("mkits_oil_stocks", "Oil stock system (crude oil, S&P 500, XOI)", "Ch.15 Stocks", "Long", NEED_OIL,
           &["E|Exploration + Tradesim code on oil stocks (App. A.6 pp.335-339)"]),
        nd("mkits_gold_stocks", "Gold stock system (gold, XAU)", "Ch.15 Stocks", "Long", NEED_GSTK,
           &["E|Exploration + Tradesim code on gold stocks (App. A.6 pp.339-341)"]),
        nd("mkits_yen_vol", "Yen (USD/JPY) Daily Intermarket Volatility System", "Ch.17 Forex", "Long/Short", NEED_YEN,
           &["E|Regression divergence vs DXY, TNX disparity, SPX ROC, ADX-volatility breakout (App. A.8 pp.348-350)"]),
        nd("mkits_yen_fut_vol", "Yen Futures (6J) Daily Intermarket Volatility System", "Ch.17 Forex", "Long/Short", NEED_YEN,
           &["E|Inverse of the USD/JPY volatility system (App. A.8 pp.350-352)"]),
        nd("mkits_yen_reg", "Yen (USD/JPY) Daily Intermarket Regression System", "Ch.17 Forex", "Long/Short", NEED_YEN,
           &["E|Regression divergence vs DXY, TNX disparity (OPT1 MA period not printed) (App. A.8 pp.352-355)"]),
        nd("mkits_euro_ma_im", "Euro (EUR/USD) Daily MA-intermarket System (CRB, TNX)", "Ch.17 Forex", "Long/Short", NEED_EURO,
           &["E|Classic MA system + CRB>MOV(CRB,35,S) AND TNX<MOV(TNX,5,S) for longs (reverse for shorts) (App. A.8 p.355; Table 17.6: 38 trades, 58% win)"]),
    ]);
    v.push(d("mkits_ftse_nn", "FTSE neural network systems (5 NeuroShell nets)", "Ch.14 FTSE", "Long/Short", Status::NotTestable(NN),
        &["E|NeuroShell Day Trader prediction of FTSE from CAC/XOI/BIX inputs (Ch.14.5, Appendix B pp.357-366)"], NN, &[], false, None));
    v
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    if !id.starts_with("mkits_") { return None; }
    Some(match id {
        "mkits_dax_ma" => ("mkits_dax", "Katsanos DAX systems", "MA crossover (single market)"),
        "mkits_dax_disparity" | "mkits_dax_ma_im" | "mkits_dax_stocks" => ("mkits_dax", "Katsanos DAX systems", "Intermarket - data missing"),
        "mkits_euro_ma" => ("mkits_fx", "Katsanos forex systems", "Euro classic MA (single market)"),
        "mkits_rs_fut" => ("mkits_rs", "Katsanos relative strength vs S&P 500", "Test I (no volume)"),
        "mkits_rs_etf" => ("mkits_rs", "Katsanos relative strength vs S&P 500", "Test II (VFI)"),
        x if x.starts_with("mkits_gold_") => ("mkits_gold", "Katsanos gold intermarket systems", "Data missing"),
        x if x.starts_with("mkits_yen") || x == "mkits_euro_ma_im" => ("mkits_fx", "Katsanos forex systems", "Intermarket - data missing"),
        _ => ("mkits_other", "Katsanos other intermarket systems", "Data missing / not testable"),
    })
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    match id {
        "mkits_dax_ma" => Some(vec![
            p("ci_n", "Congestion Index bars", 40., 20., 60., 5.),
            p("ci_trend", "CI trend threshold (long >, short < minus)", 30., 20., 40., 5.),
            p("ci_cong", "CI congestion band for entries", 25., 15., 35., 5.),
            p("ci_cong_exit", "CI congestion band for the long exit", 20., 10., 30., 5.),
            p("ci_rev", "CI 3-day reversal points", 40., 20., 60., 5.),
            p("ma_fast_l", "Long fast MA", 15., 5., 30., 5.),
            p("ma_fast_s", "Short fast MA", 10., 5., 30., 5.),
            p("ma_slow", "Slow MA", 20., 10., 40., 5.),
            p("ma_bear", "Bear-filter MA (2-day MA below it)", 150., 100., 250., 25.),
            p("st_slow", "Slow stochastic bars", 40., 20., 60., 5.),
            p("st_buy", "Slow stochastic oversold", 30., 20., 40., 5.),
            p("st_sell_hi", "Slow stochastic overbought peak (long exit)", 85., 75., 95., 5.),
            p("st_sell", "Slow stochastic fall-back level (long exit)", 75., 65., 85., 5.),
            p("st_short", "Slow stochastic overbought (short entry)", 70., 60., 80., 5.),
            p("time_exit", "Time exit bars (0 = off)", 60., 0., 120., 10.),
        ]),
        "mkits_euro_ma" => Some(vec![
            p("ma", "MA bars", 10., 5., 20., 1.),
            p("filt_n", "Filter std-dev bars", 20., 10., 40., 5.),
            p("filt_k", "Filter multiple", 0.7, 0.3, 1.5, 0.1),
            p("ci_n", "CI bars", 40., 20., 60., 5.),
            p("ci_smooth", "CI EMA smoothing", 7., 3., 15., 1.),
            p("ci_min", "|CI| minimum", 40., 20., 60., 5.),
            p("ci_turn", "CI turn points", 3., 1., 10., 1.),
            p("sar_step", "SAR acceleration step", 0.04, 0.02, 0.06, 0.01),
            p("sar_max", "SAR maximum", 0.10, 0.10, 0.30, 0.05),
        ]),
        "mkits_rs_fut" | "mkits_rs_etf" => {
            let mut v = vec![
                p("rs_n", "RS smoothing EMA", 150., 50., 250., 10.),
                p("rs_ma", "RS trigger EMA", 120., 60., 200., 10.),
                p("ma_up", "Uptrend EMA", 25., 10., 50., 5.),
                p("lrs_n", "RS slope bars", 20., 10., 40., 5.),
                p("cor_n", "Correlation bars", 130., 60., 250., 10.),
                p("cor_max", "Maximum correlation with SPX", 0.5, 0.2, 0.9, 0.1),
                p("st_n", "Stochastic bars", 100., 50., 150., 10.),
                p("st_max", "Stochastic maximum", 80., 60., 95., 5.),
                p("exit_ma", "Exit EMA of RS", 90., 50., 150., 10.),
                p("exit_roc", "Exit 3-day RS ROC below -%", 0.5, 0.1, 2.0, 0.1),
            ];
            if id == "mkits_rs_fut" {
                v.push(p("hh_n", "RS new-high window", 250., 100., 300., 10.));
                v.push(p("hh_lag", "RS new-high lag", 20., 0., 40., 5.));
            } else {
                v.push(p("vfi_n", "VFI period", 130., 50., 200., 10.));
                v.push(p("vfi_ma", "VFI EMA", 25., 10., 50., 5.));
            }
            Some(v)
        }
        _ => None,
    }
}

fn new_trade(dir: f64, i: usize, px: f64, sig: usize, kind: &'static str) -> Trade {
    Trade { dir, legs: vec![Leg { i, px, w: 1.0 }], exit_i: i, exit_px: px, reason: String::new(), entry_kind: kind, signal_i: sig, open: false, exits: vec![] }
}
fn close_out(mut t: Trade, i: usize, px: f64, sig: usize, why: &str) -> Trade {
    t.exit_i = i; t.exit_px = px; t.signal_i = sig; t.reason = why.into(); t
}
fn finish(cur: Option<Trade>, x: &Ctx, to: usize, out: &mut Vec<Trade>) {
    if let Some(mut t) = cur { t.exit_i = to; t.exit_px = x.c[to]; t.signal_i = to; t.reason = "open".into(); t.open = true; out.push(t); }
}

/// Next-open engine: entry/exit signals on bar t's close fill at bar t+1's open; one position at a time.
/// `exit(t, dir, entry_i)` returns Some(reason). Time exit = the open `hold` bars after the entry bar (0 = off).
fn next_open(x: &Ctx, from: usize, to: usize, hold: usize,
             entry: impl Fn(usize) -> f64, exit: impl Fn(usize, f64) -> Option<&'static str>) -> Vec<Trade> {
    let (mut out, mut cur): (Vec<Trade>, Option<Trade>) = (vec![], None);
    for t in from..to {
        if let Some(tr) = cur.as_ref() {
            let e = tr.legs[0].i;
            let why = if hold > 0 && t + 1 >= e + hold { Some("time exit") } else { exit(t, tr.dir) };
            if let Some(w) = why { out.push(close_out(cur.take().unwrap(), t + 1, x.o[t + 1], t, w)); }
        } else {
            let d = entry(t);
            if d != 0.0 { cur = Some(new_trade(d, t + 1, x.o[t + 1], t, "open")); }
        }
    }
    finish(cur, x, to, &mut out);
    out
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    let schema = params(id)?;
    if from > to || to >= x.c.len() { return None; }
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).map_or(f64::NAN, |d| d.def));
    let u = |k: &str| g(k).max(0.0) as usize;
    let n = x.c.len();
    match id {
        "mkits_dax_ma" => {
            let ci = ci(x, u("ci_n").max(2));
            let s5 = stoch(x, 5, 3);
            let s5m = sma(&s5, 3);
            let s40 = stoch(x, u("st_slow").max(1), 3);
            let (ll_s40_2, hh_s40_4, hh_s40_2) = (llv(&s40, 2), hhv(&s40, 4), hhv(&s40, 2));
            let (hh_ci3, ll_ci3, roc_ci3) = (hhv(&ci, 3), llv(&ci, 3), roc(&ci, 3));
            let (mfl, mfs, ms, mb, m2) = (sma(&x.c, u("ma_fast_l")), sma(&x.c, u("ma_fast_s")), sma(&x.c, u("ma_slow")), sma(&x.c, u("ma_bear")), sma(&x.c, 2));
            let mac = macd(&x.c);
            let mac7 = ema(&mac, 7);
            let c7 = ema(&x.c, 7);
            let (tr, cg, cge, rv) = (g("ci_trend"), g("ci_cong"), g("ci_cong_exit"), g("ci_rev"));
            let (sb, shi, sl, ss) = (g("st_buy"), g("st_sell_hi"), g("st_sell"), g("st_short"));
            let entry = |t: usize| {
                let up = s5[t] > s5m[t];
                let dn = s5[t] < s5m[t];
                if (ci[t] > tr && up && mfl[t] > ms[t]) || (ci[t].abs() < cg && up && ll_s40_2[t] < sb) { return 1.0; }
                if (ci[t] < -tr && roc_ci3[t] < 0.0 && dn && mfs[t] < ms[t] && m2[t] < mb[t])
                    || (ci[t].abs() < cg && roc_ci3[t] < 0.0 && dn && hh_s40_2[t] > ss) { return -1.0; }
                0.0
            };
            let exit = |t: usize, d: f64| -> Option<&'static str> {
                if d > 0.0 {
                    if hh_ci3[t] - ci[t] > rv { return Some("CI fell 40 (trend reversal)"); }
                    if ci[t].abs() < cge && s5[t] < s5m[t] && hh_s40_4[t] > shi && s40[t] < sl { return Some("congestion stochastic sell"); }
                } else {
                    if ll_ci3[t] - ci[t] < -rv { return Some("CI rose 40 (trend reversal)"); }
                    if cross(&mac, &mac7, t) && x.c[t] > c7[t] { return Some("MACD crossed its 7-day EMA, close > 7-day EMA"); }
                }
                None
            };
            Some(next_open(x, from, to, u("time_exit"), entry, exit))
        }
        "mkits_euro_ma" => {
            let mn = u("ma").max(1);
            let ma = sma(&x.c, mn);
            let pc: Vec<f64> = (0..n).map(|i| if i > 0 { (ma[i] / ma[i - 1]).ln() } else { f64::NAN }).collect();
            let filt = stdev(&pc, u("filt_n").max(1));
            let ci = ema(&ci(x, u("ci_n").max(2)), u("ci_smooth").max(1));
            let (llma, hhma, llci, hhci) = (llv(&ma, 3), hhv(&ma, 3), llv(&ci, 3), hhv(&ci, 3));
            let sr = sar(&x.h, &x.l, g("sar_step"), g("sar_max"));
            let (k, cmin, turn) = (g("filt_k"), g("ci_min"), g("ci_turn"));
            let (mut out, mut cur): (Vec<Trade>, Option<Trade>) = (vec![], None);
            for t in from..=to {
                if let Some(tr) = cur.as_ref() {
                    let stop = if tr.dir > 0.0 { x.c[t] < sr[t] } else { x.c[t] > sr[t] };
                    if stop && t > tr.legs[0].i {
                        out.push(close_out(cur.take().unwrap(), t, x.c[t], t, "close crossed Parabolic SAR(.04,.1)"));
                    }
                }
                if cur.is_none() && t < to {
                    let trend = ci[t].abs() > cmin;
                    if ma[t] > llma[t] + k * filt[t] && x.c[t] > sr[t] && trend && ci[t] > llci[t] + turn {
                        cur = Some(new_trade(1.0, t, x.c[t], t, "close"));
                    } else if ma[t] < hhma[t] - k * filt[t] && x.c[t] < sr[t] && trend && ci[t] < hhci[t] - turn {
                        cur = Some(new_trade(-1.0, t, x.c[t], t, "close"));
                    }
                }
            }
            finish(cur, x, to, &mut out);
            Some(out)
        }
        "mkits_rs_fut" | "mkits_rs_etf" => {
            let Some(s) = spx(x) else { return Some(vec![]) };
            let ratio: Vec<f64> = (0..n).map(|i| x.c[i] / s[i]).collect();
            let rs = ema(&ratio, u("rs_n").max(1));
            let rsm = ema(&rs, u("rs_ma").max(1));
            let up = roc(&ema(&x.c, u("ma_up").max(1)), 2);
            let sl = lrs(&rs, u("lrs_n").max(2));
            let co = correl(&x.c, &s, u("cor_n").max(2));
            let st = stoch(x, u("st_n").max(1), 3);
            let em = ema(&rs, u("exit_ma").max(1));
            let r3 = roc(&rs, 3);
            let (cmax, smax, xr) = (g("cor_max"), g("st_max"), g("exit_roc"));
            let etf = id == "mkits_rs_etf";
            let extra: Vec<f64> = if etf { ema(&vfi(x, u("vfi_n").max(2), 0.2, 2.5), u("vfi_ma").max(1)) } else {
                let hh = hhv(&rs, u("hh_n").max(1));
                let lag = u("hh_lag");
                (0..n).map(|i| if i >= lag { hh[i - lag] } else { f64::NAN }).collect()
            };
            let entry = |t: usize| {
                let f = if etf { extra[t] > 0.0 } else { rs[t] > extra[t] };
                if rs[t] > rsm[t] && up[t] > 0.0 && sl[t] > 0.0 && co[t] < cmax && st[t] < smax && f { 1.0 } else { 0.0 }
            };
            let exit = |t: usize, _d: f64| -> Option<&'static str> {
                if cross(&em, &rs, t) { return Some("RS crossed below its EMA"); }
                if r3[t] < -xr { return Some("3-day RS ROC < -0.5%"); }
                if etf && extra[t] < 0.0 { return Some("Mov(VFI,25,E) < 0"); }
                None
            };
            Some(next_open(x, from, to, 0, entry, exit))
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;
    fn rows(c: &[f64]) -> Vec<Row> {
        c.iter().enumerate().map(|(i, &v)| Row { ticker: "T".into(), date: format!("{:04}-01-01", 1900 + i), open: v, high: v + 1.0, low: v - 1.0, close: v, volume: Some(1000.0) }).collect()
    }
    #[test]
    fn metastock_functions_by_hand() {
        assert_eq!(roc(&[100., 110.], 1)[1], 10.0);
        assert!((lrs(&[1., 3., 5.], 3)[2] - 2.0).abs() < 1e-12);
        assert!((stdev(&[1., 3.], 2)[1] - 1.0).abs() < 1e-12); // population
        let e = ema(&[1., 2., 3., 4.], 3);
        assert!(e[1].is_nan() && (e[2] - 2.0).abs() < 1e-12 && (e[3] - 3.0).abs() < 1e-12);
        assert!(cross(&[0., 2.], &[1., 1.], 1) && !cross(&[2., 2.], &[1., 1.], 1));
        let r = rows(&[10., 11., 12.]);
        let x = Ctx::new(&r, &r, &[]);
        // Stoch(2,1) on bar 2: (12 - LLV(L,2)=10) / (HHV(H,2)=13 - 10) = 66.67
        assert!((stoch(&x, 2, 1)[2] - 200.0 / 3.0).abs() < 1e-9);
    }
    #[test]
    fn next_open_fills_and_time_exit() {
        let c: Vec<f64> = (0..20).map(|i| 100.0 + i as f64).collect();
        let r = rows(&c);
        let x = Ctx::new(&r, &r, &[]);
        let ts = next_open(&x, 0, 19, 5, |t| if t == 2 { 1.0 } else { 0.0 }, |_, _| None);
        assert_eq!(ts.len(), 1);
        assert_eq!((ts[0].legs[0].i, ts[0].legs[0].px), (3, 103.0));
        assert_eq!((ts[0].exit_i, ts[0].exit_px, ts[0].reason.as_str()), (8, 108.0, "time exit"));
    }
    #[test]
    fn catalogue_ids_unique_and_live_count() {
        let cat = catalogue();
        let mut ids: Vec<_> = cat.iter().map(|d| d.id).collect();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), cat.len());
        assert_eq!(cat.iter().filter(|d| d.status == Status::Live).count(), 4);
        for d in &cat { assert!(family_of(d.id).is_some()); }
    }
}

// Instrument admission: Katsanos 2026-10-25 block, moved from instrument_policy.rs on 31 Oct 2026 (lane ST).
// Admission only for the module REVIEWED lists (authors/katsanos/admission/SOURCE-CONTRACT.md); kept in one block to merge.
// Runs on actual indices, ETFs and stocks, US and India (index_etf_or_stock, nifty50 basket, declared application).
pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| (def.book=="mkits" && crate::books::mkits::reviewed(def.id)) || (def.book=="mkfve" && crate::books::mkfve::reviewed(def.id)),
    label: "Katsanos tested these rules on DAX futures, EUR/USD and one US stock (XM Satellite Radio); runs on indices, ETFs and other stocks are a declared application.",
    basis: "Intermarket Trading Strategies (Wiley 2008): DAX MA Crossover System (App. A.4, Table 13.2 p.210) and the Euro classic MA system (eq. 17.1-17.8 p.281-282, code p.355 without its CRB/TNX line); Detecting Breakouts (S&C April 2003) FVE divergence with linear-regression slopes (p.29, 36). Book markets: DAX futures, EUR/USD, XMSR stock. Index/ETF/stock runs are a declared application.",
};
