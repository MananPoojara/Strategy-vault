//! Mark Conway & Aaron Behle, "Professional Stock Trading: System Design and Automation" (Acme Trader, 2003).
//! The Acme systems are coded from the book's EasyLanguage (ch.11, printed p.234-299), which wins over prose; the
//! chapter rule tables (p.73, 109-110, 127, 141-142) are quoted where they agree. Every Live system exits through the
//! Acme Trade Manager (p.16-18, p.272-273). Stock systems: run on NSE stock baskets (basket:<list>:<SYMBOL>) or an index.
//! Source and checks: library/cbpst/.
use crate::connors::{wilder, Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "cbpst";

const TM: &str = "X|Acme Trade Manager (code p.272-273; Example 1.3): ATR = Volatility(20); SellStop = Lowest(Low, StopBars=1) - ExitFactor(0.25) x ATR, all shares, next bar stop; CoverStop = Highest(High,1) + 0.25 x ATR";
const TM2: &str = "X|Profit targets (ProfitTarget=True): CurrentContracts/2 at High + ProfitFactor(0.9) x ATR, and CurrentContracts/2 at High[ProfitBars] + 2 x 0.9 x ATR, next bar limit; ProfitBars = IntPortion(HoldBars/2)+1 = 3; short side mirrored (Low - ..., Low[ProfitBars] - ...)";
const TM3: &str = "X|Time exit: If BarsSinceEntry >= HoldBars(5) - 1 then exit next bar on open";
const NOTE: &str = "F|Reading (simulation conventions, not book rules): TradeStation order semantics: entries are next-bar stop orders (gap through = open); same-direction signals while in a position are ignored, opposite ones reverse. Trade Manager orders start the bar after entry. Each named target fills once per entry (TradeStation rule), sized at half the shares held at the prior close. Same-bar order sequence follows TradeStation's daily-bar assumption: price goes from the open to the nearer extreme first, then the other, then the close; ties at one price: stop loss, then targets, then entries. Fractional position (no share rounding; book sizing $100,000 x 2% / ATR is not used: returns are per unit). TS Volatility(n) = Wilder-smoothed true range seeded with its n-bar average (as researched for stad, TradeStation help 'Volatility'); ADX/DMI = Wilder (TradeStation); StdDev = population.";

fn mk(id: &'static str, name: &'static str, dir: &'static str, status: Status, rules: &'static [&'static str], notes: &'static str) -> StrategyDef {
    StrategyDef { id, name, dir, status, rules, notes, group: "Acme Trading Systems (Conway & Behle)", session: 0, needs_volume: false,
        needs_vix: false, needs_series: &[], book: BOOK, claim: None }
}

pub fn catalogue() -> Vec<StrategyDef> {
    vec![
        mk("cbpst_m", "Acme M · pattern combinations", "Long/Short", Status::Live, &[
            "F|FiltersOn(True): Close > MinimumPrice(15) and ATR >= MinimumATR(1.0), ATR = Average(Range,20) (code p.248-249). The book turns filters off for indices (p.79): p.filters=0",
            "F|Patterns counted per bar (letters): Cobra C, Hook H, Inside Day 2 I, Tail L, Harami M, Pullback P, Test T, V Zone V; qualifiers A (bar on its 50-bar MA, both sides) and N (NR2/ID2/NR10/IDNR4/Range <= 0.7 x ATR, added only to a side that already has a letter) (code p.249-251, functions p.278-298)",
            "E|Long: StrLen(LongString) >= MinimumPatterns(3) and Low < Lowest(Low, MRange-1)[1] and AcmeRangePercent(High, MRange=5) <= 0.4 and High > Low[1]: Buy next bar at High + EntryFactor(0.10) x ATR stop (p.251-252)",
            "E|Short: StrLen(ShortString) >= 3 and High > Highest(High,4)[1] and AcmeRangePercent(Low,5) >= 0.6 and Low < High[1]: Sell short next bar at Low - 0.10 x ATR stop (p.252)",
            TM, TM2, TM3, NOTE],
            "Code wins over the chapter table (p.73: ATR14, '>= 3'); the code uses Average(Range,20) for the M system's ATR and entry. Pattern functions exactly as printed (AcmeCobra p.278-279, AcmeHook p.289, AcmeInsideDay2 p.289, AcmeTail p.295, AcmeHarami p.287-288, AcmePullback p.291-292, AcmeTest p.296, AcmeVHigh/VLow p.298 with HighW/LowW = prior calendar week, AcmeOnAverage p.291). NthLowestBar ties: the more recent bar counts first."),
        mk("cbpst_n", "Acme N · narrow-range pullback breakout", "Long/Short", Status::Live, &[
            "F|LowVolatility = NR5 today and yesterday (AcmeNarrowRange(5,1) and AcmeNarrowRange(5,0)[1]) or ID2 or NR10 or IDNR4 or AcmeRangeRatio(1,7) <= 0.7 (code p.263; prose p.140-141)",
            "F|Range <= RangePercent(0.7) x ATR20 (ATR = Volatility(20)); FiltersOn(True): Close > 20, ADX(14) >= 18, AcmeVolatility(14) >= 0.5 (annualised HV, sqrt 365) (p.141-142, code p.263)",
            "E|Long: MedianPrice > MA50 (with filters on) and AcmeRetraceDown(2): Buy next bar at High + EntryFactor(0.10) x ATR stop (p.141, code p.263-264)",
            "E|Short: MedianPrice < MA50 and AcmeRetraceUp(2): Sell short next bar at Low - 0.10 x ATR stop (p.142, code p.264)",
            TM, TM2, TM3, NOTE],
            "AcmeRetraceDown(n): of the last n bars, all lower (L<L1 and H<H1), or one inside day and n-1 lower; RetraceUp mirrored with higher bars (p.294-295)."),
        mk("cbpst_r", "Acme R · rectangle breakout", "Long/Short", Status::Live, &[
            "F|AcmeRectangular(4,12,1.0,0.3): RectangleHeight = Highest(H,4) - Lowest(L,4); RangeHeight = Highest(H,12)[4] - Lowest(L,12)[4]; rectangle when RectangleHeight / RangeHeight <= 0.3 and RectangleHeight <= 1.0 x Average(Range,30) (p.106-108, code p.293-294)",
            "F|FiltersOn(True): ATR >= MinimumATR(1.0), ATR = Volatility(20) (code p.268-269)",
            "E|In a rectangle: Buy next bar at Highest(H,4) + EntryFactor(0.25) x ATR stop AND Sell short next bar at Lowest(L,4) - 0.25 x ATR stop (both orders; p.109-110, code p.269)",
            TM, TM2, TM3, NOTE],
            "The chapter table prints ATR30 for entries/exits (p.110); the code (Volatility(20)) wins."),
        mk("cbpst_v", "Acme V · Monday/Tuesday V bottom (linear regression)", "Long", Status::Live, &[
            "E|Today is Monday or Tuesday (DayOfWeek 1 or 2) and High < Lowest(High,4)[1] and Range <= RangeFactor(1.0) x ATR20 and Low > LinearRegValue(Low,5,-1) and Low > MA50 and High > Low[1] (p.127, code p.128/278)",
            "E|Buy next bar at Close + EntryFactor(0.25) x ATR20 stop (Example 6.1 p.128 and the rule table p.127; the ch.11 listing p.278 omits 'x ATR' in the order line but not in the drawn target - the chapter version is used)",
            TM, TM2, TM3, NOTE],
            "Long only (p.127: the short side is 'an exercise left to the reader' - not coded). LinearRegValue(Low,5,-1) = least-squares line through the last 5 lows projected one bar ahead. The chapter rule table prints 'Range < ...'; the code '<=' is used."),
        mk("cbpst_f", "Acme F · float breakout / pullback (FB, FP)", "Long/Short", Status::NeedsData("needs each stock's float (shares outstanding available for trading) history: the book's AcmeGetFloat holds 2002 US float values; no NSE free-float history is in the vault"), &[
            "F|FloatBars = bars back until cumulative volume reaches the float (FloatFactor 1.0); FloatHigh/Low = Highest(High)/Lowest(Low) over FloatBars (p.86-92, code p.238-240, 283-286)",
            "E|FB long: HighBaseCount >= BaseBars(7) and High > FloatHigh - ATR and Highest(High,3) - Lowest(High,3) <= 0.35 x ATR and Close > MA50: buy next bar at High + 0.3 x ATR stop; FB short mirrored (p.239)",
            "E|FP long: Low <= FloatLow + ATR and DMIPlus(30) > DMIMinus(30) and Close > MA50: buy stop; FP short mirrored (p.239-240)",
            TM, TM2, TM3],
            "Needs volume and the float; coded nowhere until a float series exists."),
        mk("cbpst_p", "Acme P · intraday pair trading", "Long/Short", Status::NeedsData("needs intraday (3-5 minute) bars for both stocks of each pair; the vault has daily bars only"), &[
            "F|VolatilityBand = 0.0523 x (HV1 + HV2) x (1 - Correlation(Price1, Price2, 30)) from daily data; bands = +/- 1.5 x band; Spread = Close1/Price1 - Close2/Price2 (prior daily closes) on intraday bars (p.46-50, code p.266-267)",
            "E|Spread crosses above LowerBand: buy A / short B this bar on close; Spread crosses below UpperBand: short A / buy B (p.47, code p.267)",
            "X|Exit long when Spread crosses above 0 (target) or Spread <= 1.5 x LowerBand (stop), on the close; short mirrored (code p.267)"],
            "Own exits, not the Trade Manager (p.266)."),
        mk("cbpst_mkt", "Acme Market System · sentiment pattern count", "Long/Short", Status::NeedsData("needs daily equity put/call ratio, new highs, new lows, TRIN, Market Vane bullish consensus and public/specialist short-sales ratio; the vault has VIX/India VIX and NSE TRIN only, and the book requires >= 2 agreeing series"), &[
            "F|AcmeHighLowIndex(Price,20): 1 when yesterday was the 20-bar highest and today is lower, 2 when yesterday was the lowest and today is higher (p.288)",
            "E|Long letters: VIX 1, put/call 1, new highs 2, new lows 1, TRIN (4-bar average) 1, bullish consensus 2, short-sales ratio 1 (shorts mirrored); >= MinimumPatterns(2): buy next bar at High + 0.2 x ATR stop / sell short at Low - 0.2 x ATR (code p.259-261)",
            TM, TM2, TM3],
            "Would run on the market index once the series exist."),
        mk("cbpst_day", "Day-trading techniques (ch.10)", "Long/Short", Status::NotTestable("intraday Level II / opening-range / news techniques described by example without complete mechanical rules (p.193-224)"), &[
            "F|Gap, continuation, block and spread trading on intraday charts and Level II quotes (p.206-216)"],
            "Discretionary and intraday."),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    let v = match id { "cbpst_m" => "M patterns", "cbpst_n" => "N range", "cbpst_r" => "R rectangle", "cbpst_v" => "V regression",
        "cbpst_f" => "F float", "cbpst_p" => "P pairs", "cbpst_mkt" => "Market model", "cbpst_day" => "Day trading", _ => return None };
    Some(("cbpst_acme", "Acme trading systems (Conway & Behle)", v))
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    let tm = vec![p("exit_factor", "Stop: ATR fraction below the StopBars low", 0.25, 0., 2., 0.05), p("stop_bars", "StopBars", 1., 1., 10., 1.),
        p("profit_target", "Profit targets on (1) / off (0)", 1., 0., 1., 1.), p("profit_factor", "ProfitFactor (ATR multiple)", 0.9, 0.1, 3., 0.1),
        p("hold_bars", "HoldBars", 5., 1., 30., 1.)];
    let mut v = match id {
        "cbpst_m" => vec![p("filters", "FiltersOn (1 = code default; 0 = the book's index setting p.79)", 1., 0., 1., 1.), p("min_patterns", "MinimumPatterns", 3., 1., 6., 1.),
            p("entry_factor", "EntryFactor (ATR)", 0.10, 0., 1., 0.05), p("min_price", "MinimumPrice", 15., 0., 100., 5.), p("min_atr", "MinimumATR", 1., 0., 5., 0.5),
            p("mrange", "MRange bars", 5., 3., 10., 1.), p("mrange_pct", "MRangePercent", 0.4, 0.1, 0.9, 0.05)],
        "cbpst_n" => vec![p("filters", "FiltersOn", 1., 0., 1., 1.), p("entry_factor", "EntryFactor (ATR)", 0.10, 0., 1., 0.05), p("range_pct", "RangePercent (ATR)", 0.7, 0.3, 1.5, 0.05),
            p("max_range_ratio", "MaxRangeRatio", 0.7, 0.3, 1.5, 0.05), p("retrace_bars", "RetraceBars", 2., 1., 5., 1.), p("min_price", "MinimumPrice", 20., 0., 100., 5.),
            p("min_adx", "MinimumADX", 18., 0., 40., 1.), p("min_hv", "MinimumHV", 0.5, 0., 1.5, 0.05)],
        "cbpst_r" => vec![p("filters", "FiltersOn", 1., 0., 1., 1.), p("entry_factor", "EntryFactor (ATR)", 0.25, 0., 1., 0.05), p("rect_len", "RectangleLength", 4., 2., 10., 1.),
            p("rect_range", "RectangleRange (reference bars)", 12., 5., 30., 1.), p("rect_factor", "RectangleFactor (ATR30)", 1., 0.3, 3., 0.1), p("rect_ratio", "RectangleRatio", 0.3, 0.1, 0.8, 0.05),
            p("min_atr", "MinimumATR", 1., 0., 5., 0.5)],
        "cbpst_v" => vec![p("entry_factor", "EntryFactor (ATR)", 0.25, 0., 1., 0.05), p("reg_bars", "RegressionBars", 5., 3., 10., 1.), p("range_factor", "RangeFactor (ATR)", 1., 0.3, 3., 0.1)],
        _ => return None,
    };
    v.extend(tm);
    Some(v)
}

// ------------------------------------------------------------------ TradeStation helpers
fn tr(x: &Ctx, i: usize) -> f64 { if i == 0 { x.h[0] - x.l[0] } else { x.h[i].max(x.c[i - 1]) - x.l[i].min(x.c[i - 1]) } }
fn rng(x: &Ctx, i: usize) -> f64 { x.h[i] - x.l[i] }
fn sma(v: &[f64], n: usize) -> Vec<f64> {
    let mut out = vec![f64::NAN; v.len()];
    let mut s = 0.;
    for i in 0..v.len() { s += v[i]; if i >= n { s -= v[i - n]; } if i + 1 >= n { out[i] = s / n as f64; } }
    out
}
/// TradeStation Volatility(n): Wilder-smoothed true range seeded with the n-bar average true range.
fn ts_vol(x: &Ctx, n: usize) -> Vec<f64> {
    let t: Vec<f64> = (0..x.c.len()).map(|i| tr(x, i)).collect();
    let a = sma(&t, n);
    let mut out = vec![f64::NAN; t.len()];
    for i in 0..t.len() { out[i] = if i == 0 || !out[i - 1].is_finite() { a[i] } else { ((n as f64 - 1.) * out[i - 1] + t[i]) / n as f64 }; }
    out
}
/// TradeStation DMIPlus / DMIMinus / ADX (Wilder smoothing).
fn dmi(x: &Ctx, n: usize) -> (Vec<f64>, Vec<f64>, Vec<f64>) {
    let len = x.c.len();
    let (mut pdm, mut mdm, mut t) = (vec![f64::NAN; len], vec![f64::NAN; len], vec![f64::NAN; len]);
    for i in 1..len {
        let (u, d) = (x.h[i] - x.h[i - 1], x.l[i - 1] - x.l[i]);
        pdm[i] = if u > d && u > 0. { u } else { 0. };
        mdm[i] = if d > u && d > 0. { d } else { 0. };
        t[i] = tr(x, i);
    }
    let (sp, sm, st) = (wilder(&pdm, n), wilder(&mdm, n), wilder(&t, n));
    let (mut p, mut m, mut dx) = (vec![f64::NAN; len], vec![f64::NAN; len], vec![f64::NAN; len]);
    for i in 0..len {
        if st[i] > 0. {
            p[i] = 100. * sp[i] / st[i]; m[i] = 100. * sm[i] / st[i];
            dx[i] = if p[i] + m[i] > 0. { 100. * (p[i] - m[i]).abs() / (p[i] + m[i]) } else { 0. };
        }
    }
    let adx = wilder(&dx, n);
    (p, m, adx)
}
/// AcmeVolatility(n): population StdDev of ln(C/C1) over n bars x sqrt(365).
fn acme_vol(x: &Ctx, n: usize) -> Vec<f64> {
    let len = x.c.len();
    let lr: Vec<f64> = (0..len).map(|i| if i == 0 { f64::NAN } else { (x.c[i] / x.c[i - 1]).ln() }).collect();
    (0..len).map(|i| {
        if i < n { return f64::NAN; }
        let w = &lr[i + 1 - n..=i];
        let m = w.iter().sum::<f64>() / n as f64;
        (w.iter().map(|v| (v - m).powi(2)).sum::<f64>() / n as f64).sqrt() * 365f64.sqrt()
    }).collect()
}
fn hi(v: &[f64], i: usize, n: usize) -> f64 { if i + 1 < n { f64::NAN } else { v[i + 1 - n..=i].iter().copied().fold(f64::NEG_INFINITY, f64::max) } }
fn lo(v: &[f64], i: usize, n: usize) -> f64 { if i + 1 < n { f64::NAN } else { v[i + 1 - n..=i].iter().copied().fold(f64::INFINITY, f64::min) } }
/// AcmeRangePercent(price, n): position of price in the n-bar high-low range (0 if no range).
fn rp(x: &Ctx, price: f64, i: usize, n: usize) -> f64 {
    let (h, l) = (hi(&x.h, i, n), lo(&x.l, i, n));
    if h - l > 0. { (price - l) / (h - l) } else { 0. }
}
fn rng_low(x: &Ctx, i: usize, n: usize) -> f64 { // Lowest(Range, n) at bar i
    if i + 1 < n { return f64::NAN; }
    (i + 1 - n..=i).map(|k| rng(x, k)).fold(f64::INFINITY, f64::min)
}
/// AcmeNarrowRange(L, idx): Range < Lowest(Range, L-1)[idx+1]
fn nr(x: &Ctx, i: usize, l: usize, idx: usize) -> bool { i > idx && rng(x, i) < rng_low(x, i - idx - 1, l - 1) }
fn inside(x: &Ctx, i: usize) -> bool { i > 0 && x.l[i] > x.l[i - 1] && x.h[i] < x.h[i - 1] }
fn id2(x: &Ctx, i: usize) -> bool { i > 1 && inside(x, i) && inside(x, i - 1) }
fn idnr(x: &Ctx, i: usize, l: usize) -> bool { nr(x, i, l, 0) && inside(x, i) }
fn retrace(x: &Ctx, i: usize, n: usize, down: bool) -> bool {
    if i < n { return false; }
    let ins = (i + 1 - n..=i).filter(|&k| inside(x, k)).count();
    let mv = (i + 1 - n..=i).filter(|&k| if down { x.l[k] < x.l[k - 1] && x.h[k] < x.h[k - 1] } else { x.l[k] > x.l[k - 1] && x.h[k] > x.h[k - 1] }).count();
    mv == n || (ins == 1 && mv == n - 1)
}
fn weekday(date: &str) -> u32 { // 0 = Sunday .. 6 = Saturday (TradeStation DayOfWeek)
    let (y, m, d): (i64, i64, i64) = (date[..4].parse().unwrap_or(2000), date[5..7].parse().unwrap_or(1), date[8..10].parse().unwrap_or(1));
    let t = [0, 3, 2, 5, 0, 3, 5, 1, 4, 6, 2, 4];
    let yy = if m < 3 { y - 1 } else { y };
    ((yy + yy / 4 - yy / 100 + yy / 400 + t[(m - 1) as usize] + d) % 7) as u32
}
fn day_number(date: &str) -> i64 { // days since 1970-01-01 (civil)
    let (y, m, d): (i64, i64, i64) = (date[..4].parse().unwrap_or(2000), date[5..7].parse().unwrap_or(1), date[8..10].parse().unwrap_or(1));
    let y2 = if m <= 2 { y - 1 } else { y };
    let era = y2.div_euclid(400);
    let yoe = y2 - era * 400;
    let doy = (153 * (if m > 2 { m - 3 } else { m + 9 }) + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146097 + doe - 719468
}
/// HighW(1)/LowW(1): high/low of the previous calendar week (Monday-Sunday), aligned per bar.
fn prev_week(x: &Ctx) -> (Vec<f64>, Vec<f64>) {
    let n = x.c.len();
    let wk: Vec<i64> = x.r.iter().map(|r| (day_number(&r.date) + 3).div_euclid(7)).collect(); // 1970-01-01 is a Thursday
    let (mut ph, mut pl) = (vec![f64::NAN; n], vec![f64::NAN; n]);
    let (mut ch, mut cl, mut lh, mut ll) = (f64::NAN, f64::NAN, f64::NAN, f64::NAN);
    for i in 0..n {
        if i > 0 && wk[i] != wk[i - 1] { lh = ch; ll = cl; ch = f64::NAN; cl = f64::NAN; }
        ch = if ch.is_finite() { ch.max(x.h[i]) } else { x.h[i] };
        cl = if cl.is_finite() { cl.min(x.l[i]) } else { x.l[i] };
        ph[i] = lh; pl[i] = ll;
    }
    (ph, pl)
}
/// LinearRegValue(v, n, -1): least-squares line over the last n values, projected one bar ahead.
fn linreg_next(v: &[f64], i: usize, n: usize) -> f64 {
    if i + 1 < n { return f64::NAN; }
    let nf = n as f64;
    let (sx, sxx) = (nf * (nf - 1.) / 2., (nf - 1.) * nf * (2. * nf - 1.) / 6.);
    let (mut sy, mut sxy) = (0., 0.);
    for k in 0..n { let y = v[i + 1 - n + k]; sy += y; sxy += k as f64 * y; }
    let b = (nf * sxy - sx * sy) / (nf * sxx - sx * sx);
    let a = (sy - b * sx) / nf;
    a + b * nf
}

// ------------------------------------------------------------------ pattern functions (return 0 / 1 bull / 2 bear)
fn cobra(x: &Ctx, atr20v: &[f64], i: usize, pct: f64, rf: f64) -> u8 {
    let a = atr20v[i];
    let (ro, rc) = (rp(x, x.o[i], i, 1), rp(x, x.c[i], i, 1));
    let mut r = 0;
    if ro <= pct && rc >= 1. - pct && rng(x, i) <= rf * a { r = 1; }
    if ro >= 1. - pct && rc <= pct && rng(x, i) <= rf * a { r = 2; }
    r
}
fn hook(x: &Ctx, i: usize, len: usize, pct: f64) -> u8 {
    if i < 1 { return 0; }
    let mut r = 0;
    if retrace(x, i - 1, len, false) && x.l[i] < x.l[i - 1] && rp(x, x.c[i], i, 1) >= 1. - pct { r = 1; }
    if retrace(x, i - 1, len, true) && x.h[i] > x.h[i - 1] && rp(x, x.c[i], i, 1) <= pct { r = 2; }
    r
}
fn tail(x: &Ctx, i: usize, pct: f64, len: usize) -> u8 {
    let (ro, rc) = (rp(x, x.o[i], i, 1), rp(x, x.c[i], i, 1));
    let m = (ro + rc) / 2.;
    let mut r = 0;
    if (m >= 1. - pct || rc >= 1. - 0.25) && rp(x, x.l[i], i, len) <= pct && ro > 0.5 && rc > 0.5 { r = 1; }
    if (m <= pct || rc <= 0.25) && rp(x, x.h[i], i, len) >= 1. - pct && ro < 0.5 && rc < 0.5 { r = 2; }
    r
}
fn harami(x: &Ctx, i: usize) -> u8 {
    if i < 3 || !(rng(x, i - 1) > 0. && rng(x, i - 2) > 0. && rng(x, i - 3) > 0.) { return 0; }
    let (rpl, rrl) = (0.45, 0.60);
    let within = |a: usize, b: usize| x.l[a] >= x.l[b] && x.h[a] <= x.h[b];
    let side = |bull: bool| -> bool {
        let body = if bull { x.c[i] > x.o[i] } else { x.c[i] < x.o[i] };
        let mother = |k: usize| { let (rc, ro) = (rp(x, x.c[k], k, 1), rp(x, x.o[k], k, 1)); if bull { rc <= rpl && ro >= 1. - rpl } else { ro <= rpl && rc >= 1. - rpl } };
        let ratio = |k: usize| rng(x, i) / rng(x, k) <= rrl;
        let h1 = within(i, i - 1) && body && mother(i - 1) && ratio(i - 1);
        let h2 = within(i, i - 2) && within(i - 1, i - 2) && body && mother(i - 2) && ratio(i - 2);
        let h3 = within(i, i - 3) && within(i - 1, i - 3) && within(i - 2, i - 3) && body && mother(i - 3) && ratio(i - 3);
        h1 || h2 || h3
    };
    let mut r = 0;
    if side(true) { r = 1; }
    if side(false) { r = 2; }
    r
}
fn pullback(x: &Ctx, ind: &Ind, i: usize) -> u8 {
    let (adx, p, m) = (ind.adx14[i], ind.pdi14[i], ind.mdi14[i]);
    let rc = rp(x, x.c[i], i, 5);
    let mut r = 0;
    if adx >= 25. && p > m && rc <= 0.35 && (retrace(x, i, 3, true) || tail(x, i, 0.35, 5) == 1 || cobra(x, &ind.vol20, i, 0.35, 0.7) == 1) { r = 1; }
    if adx >= 25. && m > p && rc >= 0.65 && (retrace(x, i, 3, false) || tail(x, i, 0.35, 5) == 2 || cobra(x, &ind.vol20, i, 0.35, 0.7) == 2) { r = 2; }
    r
}
/// NthLowestBar(2, v, n) (or highest): bars ago of the 2nd extreme in the last n bars; ties: more recent first.
fn nth_bar(v: &[f64], i: usize, n: usize, lowest: bool) -> usize {
    let mut idx: Vec<usize> = (0..n).collect(); // bars ago
    idx.sort_by(|&a, &b| { let (va, vb) = (v[i - a], v[i - b]); let o = if lowest { va.partial_cmp(&vb) } else { vb.partial_cmp(&va) }; o.unwrap().then(a.cmp(&b)) });
    idx[1]
}
fn test(x: &Ctx, i: usize) -> u8 {
    if i < 20 { return 0; }
    let mut r = 0;
    if x.l[i] < lo(&x.l, i - 1, 19) && nth_bar(&x.l, i, 20, true) >= 2 && rp(x, x.c[i], i, 1) > 0.5 { r = 1; }
    if x.h[i] > hi(&x.h, i - 1, 19) && nth_bar(&x.h, i, 20, false) >= 2 && rp(x, x.c[i], i, 1) < 0.5 { r = 2; }
    r
}

struct Ind { vol20: Vec<f64>, vol14: Vec<f64>, avgr20: Vec<f64>, avgr30: Vec<f64>, ma50: Vec<f64>, adx14: Vec<f64>, pdi14: Vec<f64>, mdi14: Vec<f64>, hv14: Vec<f64>, wh: Vec<f64>, wl: Vec<f64> }
fn ind(x: &Ctx) -> Ind {
    let r: Vec<f64> = (0..x.c.len()).map(|i| rng(x, i)).collect();
    let (pdi14, mdi14, adx14) = dmi(x, 14);
    let (wh, wl) = prev_week(x);
    Ind { vol20: ts_vol(x, 20), vol14: ts_vol(x, 14), avgr20: sma(&r, 20), avgr30: sma(&r, 30), ma50: sma(&x.c, 50), adx14, pdi14, mdi14, hv14: acme_vol(x, 14), wh, wl }
}

/// M system pattern strings: (long count, short count).
fn m_counts(x: &Ctx, n: &Ind, i: usize) -> (usize, usize) {
    let (mut ls, mut ss) = (0usize, 0usize);
    let rs = [cobra(x, &n.vol20, i, 0.25, 0.8), hook(x, i, 1, 0.2), tail(x, i, 0.3, 7), harami(x, i), pullback(x, n, i), test(x, i)];
    for r in rs { if r == 1 { ls += 1; } else if r == 2 { ss += 1; } }
    if id2(x, i) { ls += 1; ss += 1; }
    let vf = n.vol14[i];
    let vh = if n.wh[i] > 0. && vf > 0. { n.wh[i] + 1.5 * vf } else { 0. };
    let vl = if n.wl[i] > 0. && vf > 0. { n.wl[i] - 1.5 * vf } else { 0. };
    if x.h[i] >= vh { ss += 1; }
    if x.l[i] <= vl { ls += 1; }
    let ma = n.ma50[i];
    if ma >= x.l[i] && ma <= x.h[i] { ls += 1; ss += 1; }
    let c1 = i > 1 && nr(x, i, 5, 1) && nr(x, i - 1, 5, 0);
    let c5 = rng(x, i) <= 0.7 * n.avgr20[i];
    if c1 || id2(x, i) || nr(x, i, 10, 0) || idnr(x, i, 4) || c5 {
        if ls > 0 { ls += 1; }
        if ss > 0 { ss += 1; }
    }
    (ls, ss)
}

// ------------------------------------------------------------------ TradeStation-style simulator with the Acme Trade Manager
struct Pos { dir: f64, ei: usize, ep: f64, rem: f64, t1: bool, t2: bool, exits: Vec<(usize, f64, f64)> }
#[derive(Clone, Copy)]
struct Tm { ef: f64, sb: usize, pt: bool, pf: f64, hold: usize }

fn close_pos(p: Pos, x: &Ctx, out: &mut Vec<Trade>, why: &str, sig: usize, open: bool) {
    let n: f64 = p.exits.iter().map(|e| e.2).sum();
    let xp = if n > 0. { p.exits.iter().map(|e| e.1 * e.2).sum::<f64>() / n } else { p.ep };
    let xi = p.exits.last().map_or(p.ei, |e| e.0);
    let exits = if p.exits.len() > 1 { p.exits.clone() } else { vec![] };
    let _ = x;
    out.push(Trade { dir: p.dir, legs: vec![Leg { i: p.ei, px: p.ep, w: 1. }], exit_i: xi, exit_px: xp, reason: why.into(), entry_kind: "stop", signal_i: sig, open, exits });
}

/// entries(i) -> (buy stop, short stop) for bar i+1, computed on bar i's close.
fn simulate(x: &Ctx, from: usize, to: usize, tm: Tm, entries: &dyn Fn(usize) -> (Option<f64>, Option<f64>)) -> Vec<Trade> {
    let atr = ts_vol(x, 20);
    let mut out = vec![];
    let mut pos: Option<Pos> = None;
    let (mut buy, mut sell): (Option<f64>, Option<f64>) = (None, None);
    // exit orders for the current position: (stop, target1 (px, size), target2 (px, size), time exit at open)
    let mut xo: Option<(f64, Option<(f64, f64)>, Option<(f64, f64)>, bool)> = None;
    let pb = tm.hold / 2 + 1;
    for i in from..=to {
        if i > from {
            let (o, h, l, c) = (x.o[i], x.h[i], x.l[i], x.c[i]);
            let path = if h - o <= o - l { [o, h, l, c] } else { [o, l, h, c] };
            // when along the path a level is reached: (time, fill)
            let when = |lvl: f64, up: bool| -> Option<(f64, f64)> {
                if (up && o >= lvl) || (!up && o <= lvl) { return Some((0., o)); }
                for k in 0..3 {
                    let (a, b) = (path[k], path[k + 1]);
                    if up && b > a && lvl > a && lvl <= b { return Some((k as f64 + (lvl - a) / (b - a), lvl)); }
                    if !up && b < a && lvl < a && lvl >= b { return Some((k as f64 + (a - lvl) / (a - b), lvl)); }
                }
                None
            };
            // events: (time, rank, kind, fill, size)
            let mut ev: Vec<(f64, u8, u8, f64, f64)> = vec![];
            if let (Some(p), Some((stop, t1, t2, tx))) = (pos.as_ref(), xo) {
                let up = p.dir < 0.;
                if tx { ev.push((-1., 0, 0, o, 1.)); }
                if let Some(e) = when(stop, up) { ev.push((e.0, 1, 1, e.1, 1.)); }
                for (k, t) in [(2u8, t1), (3u8, t2)] { if let Some((lvl, sz)) = t { if let Some(e) = when(lvl, !up) { ev.push((e.0, 2, k, e.1, sz)); } } }
            }
            if let Some(b) = buy { if let Some(e) = when(b, true) { ev.push((e.0, 3, 4, e.1, 0.)); } }
            if let Some(s) = sell { if let Some(e) = when(s, false) { ev.push((e.0, 3, 5, e.1, 0.)); } }
            ev.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap().then(a.1.cmp(&b.1)));
            let pid = pos.as_ref().map(|p| p.ei);
            for (_, _, kind, px, sz) in ev {
                match kind {
                    0 | 1 | 2 | 3 => {
                        let Some(p) = pos.as_mut() else { continue };
                        if Some(p.ei) != pid { continue; } // orders belong to the position held at the prior close
                        let q = if kind <= 1 { p.rem } else { sz.min(p.rem) };
                        if q <= 1e-12 { continue; }
                        p.exits.push((i, px, q)); p.rem -= q;
                        if kind == 2 { p.t1 = true; } if kind == 3 { p.t2 = true; }
                        if p.rem <= 1e-12 {
                            let why = match kind { 0 => "time exit (HoldBars) at open", 1 => "stop loss", 2 => "profit target 1", _ => "profit target 2" };
                            close_pos(pos.take().unwrap(), x, &mut out, why, if kind == 0 { i - 1 } else { i }, false);
                        }
                    }
                    _ => {
                        let d = if kind == 4 { 1. } else { -1. };
                        if pos.as_ref().is_some_and(|p| p.dir == d) { continue; }
                        if let Some(mut p) = pos.take() { let r = p.rem; p.exits.push((i, px, r)); p.rem = 0.; close_pos(p, x, &mut out, "reversed by opposite entry", i, false); }
                        pos = Some(Pos { dir: d, ei: i, ep: px, rem: 1., t1: false, t2: false, exits: vec![] });
                    }
                }
            }
        }
        if i == to { break; }
        // orders for bar i+1, from bar i's close
        let (b, s) = entries(i);
        buy = b; sell = s;
        xo = pos.as_ref().map(|p| {
            let a = atr[i];
            let pfa = tm.pf * a;
            let half = p.rem / 2.;
            if p.dir > 0. {
                let stop = lo(&x.l, i, tm.sb) - tm.ef * a;
                let t1 = (tm.pt && !p.t1).then(|| (x.h[i] + pfa, half));
                let t2 = (tm.pt && !p.t2 && i >= pb).then(|| (x.h[i - pb] + 2. * pfa, half));
                (stop, t1, t2, i - p.ei >= tm.hold - 1)
            } else {
                let stop = hi(&x.h, i, tm.sb) + tm.ef * a;
                let t1 = (tm.pt && !p.t1).then(|| (x.l[i] - pfa, half));
                let t2 = (tm.pt && !p.t2 && i >= pb).then(|| (x.l[i - pb] - 2. * pfa, half));
                (stop, t1, t2, i - p.ei >= tm.hold - 1)
            }
        });
    }
    if let Some(mut p) = pos.take() { let r = p.rem; p.exits.push((to, x.c[to], r)); p.rem = 0.; close_pos(p, x, &mut out, "open", to, true); }
    out
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    let schema = params(id)?;
    if from >= to || to >= x.r.len() { return None; }
    for (k, v) in p { let d = schema.iter().find(|d| d.key == k)?; if !v.is_finite() || *v < d.min || *v > d.max { return None; } }
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).unwrap().def);
    let tm = Tm { ef: g("exit_factor"), sb: g("stop_bars") as usize, pt: g("profit_target") >= 0.5, pf: g("profit_factor"), hold: g("hold_bars") as usize };
    let n = ind(x);
    let on = id != "cbpst_v" && g("filters") >= 0.5;
    let ef = g("entry_factor");
    let trades = match id {
        "cbpst_m" => {
            let (minp, mr, mpct, mp, ma) = (g("min_patterns") as usize, g("mrange") as usize, g("mrange_pct"), g("min_price"), g("min_atr"));
            let f = |i: usize| -> (Option<f64>, Option<f64>) {
                let a = n.avgr20[i];
                if i < 60 || !a.is_finite() { return (None, None); }
                if on && !(x.c[i] > mp && a >= ma) { return (None, None); }
                let (ls, ss) = m_counts(x, &n, i);
                let b = (ls >= minp && x.l[i] < lo(&x.l, i - 1, mr - 1) && rp(x, x.h[i], i, mr) <= mpct && x.h[i] > x.l[i - 1]).then(|| x.h[i] + ef * a);
                let s = (ss >= minp && x.h[i] > hi(&x.h, i - 1, mr - 1) && rp(x, x.l[i], i, mr) >= 1. - mpct && x.l[i] < x.h[i - 1]).then(|| x.l[i] - ef * a);
                (b, s)
            };
            simulate(x, from, to, tm, &f)
        }
        "cbpst_n" => {
            let (rpct, mrr, rb, mp, madx, mhv) = (g("range_pct"), g("max_range_ratio"), g("retrace_bars") as usize, g("min_price"), g("min_adx"), g("min_hv"));
            let f = |i: usize| -> (Option<f64>, Option<f64>) {
                let a = n.vol20[i];
                if i < 60 || !a.is_finite() || !n.ma50[i].is_finite() { return (None, None); }
                let rr = { let s7: f64 = (i - 6..=i).map(|k| rng(x, k)).sum::<f64>() / 7.; if rng(x, i) != 0. && s7 != 0. { rng(x, i) / s7 } else { 0. } };
                let lowvol = (nr(x, i, 5, 1) && nr(x, i - 1, 5, 0)) || id2(x, i) || nr(x, i, 10, 0) || idnr(x, i, 4) || rr <= mrr;
                let filt = !on || (x.c[i] > mp && n.adx14[i] >= madx && n.hv14[i] >= mhv);
                if !(lowvol && rng(x, i) <= rpct * a && filt) { return (None, None); }
                let med = (x.h[i] + x.l[i]) / 2.;
                let b = ((!on || med > n.ma50[i]) && retrace(x, i, rb, true)).then(|| x.h[i] + ef * a);
                let s = ((!on || med < n.ma50[i]) && retrace(x, i, rb, false)).then(|| x.l[i] - ef * a);
                (b, s)
            };
            simulate(x, from, to, tm, &f)
        }
        "cbpst_r" => {
            let (rl, rr, rf, rat, matr) = (g("rect_len") as usize, g("rect_range") as usize, g("rect_factor"), g("rect_ratio"), g("min_atr"));
            let f = |i: usize| -> (Option<f64>, Option<f64>) {
                let a = n.vol20[i];
                if i < rl + rr + 30 || !a.is_finite() { return (None, None); }
                if on && !(a >= matr) { return (None, None); }
                let (rh, rlw) = (hi(&x.h, i, rl), lo(&x.l, i, rl));
                let rect_h = rh - rlw;
                let (gh, gl) = (hi(&x.h, i - rl, rr), lo(&x.l, i - rl, rr));
                let range_h = gh - gl;
                let inr = rect_h > 0. && range_h > 0. && rect_h / range_h <= rat && rect_h <= rf * n.avgr30[i];
                if !inr { return (None, None); }
                (Some(rh + ef * a), Some(rlw - ef * a))
            };
            simulate(x, from, to, tm, &f)
        }
        "cbpst_v" => {
            let (rb, rf) = (g("reg_bars") as usize, g("range_factor"));
            let f = |i: usize| -> (Option<f64>, Option<f64>) {
                let a = n.vol20[i];
                if i < 60 || !a.is_finite() || !n.ma50[i].is_finite() { return (None, None); }
                let dw = weekday(&x.r[i].date);
                if !(dw == 1 || dw == 2) { return (None, None); }
                let ok = x.h[i] < lo(&x.h, i - 1, rb - 1) && rng(x, i) <= rf * a && x.l[i] > linreg_next(&x.l, i, rb) && x.l[i] > n.ma50[i] && x.h[i] > x.l[i - 1];
                (ok.then(|| x.c[i] + ef * a), None)
            };
            simulate(x, from, to, tm, &f)
        }
        _ => return None,
    };
    Some(trades)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;
    #[test]
    fn weekday_and_week_alignment() {
        assert_eq!(weekday("2024-01-01"), 1); // Monday
        assert_eq!(weekday("2001-09-11"), 2);
        assert_eq!((day_number("1970-01-05") + 3).div_euclid(7), (day_number("1970-01-11") + 3).div_euclid(7)); // Mon..Sun one week
        assert_ne!((day_number("1970-01-11") + 3).div_euclid(7), (day_number("1970-01-12") + 3).div_euclid(7));
    }
    #[test]
    fn stop_entry_then_time_exit_at_open_of_hold_bar() {
        let r: Vec<Row> = (0..40).map(|i| Row { ticker: "T".into(), date: format!("2001-01-{:02}", 1 + i % 28), open: 100., high: 101., low: 99., close: 100., volume: None }).collect();
        let x = Ctx::new(&r, &r, &[]);
        let tm = Tm { ef: 5., sb: 1, pt: false, pf: 0.9, hold: 5 };
        // a buy stop at 100.5 only for bar 21 -> fills at 100.5 on bar 21; exit at the open of bar 26
        let f = |i: usize| if i == 20 { (Some(100.5), None) } else { (None, None) };
        let ts = simulate(&x, 0, 39, tm, &f);
        assert_eq!(ts.len(), 1);
        assert_eq!(ts[0].legs[0].i, 21); assert_eq!(ts[0].legs[0].px, 100.5);
        assert_eq!(ts[0].exit_i, 26); assert_eq!(ts[0].exit_px, 100.);
    }
}
