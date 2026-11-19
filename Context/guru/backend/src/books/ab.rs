//! AmiBroker-exact number rules shared by the YT Strategies (the team's AFLs). Each rule was proven against
//! AmiBroker's own output in the SV-Audit (share Jay/Projects/SV-Audit, Error files of 18-11-2026, Verification.md):
//! - prices are float32 in AmiBroker; a calculated fill (stop, target, trailing level, Open ± buffer) is the float32
//!   result kept inside the bar's Low-High range and read as AmiBroker's decimal price (`price_ab`, `ab_fill`);
//! - RSI(n) starts Wilder smoothing from 0 on bar 0 (not from the simple average of the first n changes); output from
//!   bar n (`rsi`);
//! - ATR(n) counts bar 0's true range as High - Low in its first average (`true_range`, `atr`).
//! These helpers are for the YT books only; the book strategies keep their own conventions.
//!
//! Applied globally: `books::ab_prices` (books/mod.rs) passes every fill of every `yt*` module through `ab_fill`
//! after the strategy runs, so a new YT strategy gets AmiBroker's float32 price, the bar-range clip and AmiBroker's
//! decimal price without doing anything. For an exact match the strategy should still compute a calculated level
//! with the AFL's own formula shape, because float32 rounding depends on it:
//!   `(1 + P * 0.01) * C`        -> `mul32(1.0 + p * 0.01, c)`
//!   `C + C * P / 100`           -> `addmul(c, p / 100.0)`
//!   `O - O * Buffer / 100`      -> `addmul(o, -buffer / 100.0)`
//! and use `ab::rsi` / `ab::atr` (AmiBroker start-up values) for RSI and ATR.
#![allow(dead_code)]

/// How the YT whole-unit money account sizes positions, as the strategy's AFL tells AmiBroker to.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct Sizing {
    /// `SetOption("UsePrevBarEquityForPosSizing", True)`: a trade entered on the bar the previous one closed is sized
    /// from the previous bar's equity (SV-Audit Ulcer Error 1). AmiBroker's default is False.
    pub prev_bar: bool,
    /// every scale-in lot is `SetPositionSize(w%, spsPercentOfEquity)` of the previous bar's equity (SV-Audit
    /// Supertrend Error 6, proven on ST 5,2 scale-in); otherwise lots share the trade's starting allocation.
    pub per_lot: bool,
    /// the AFL's lot size as a fraction of equity when the trade's leg weights are normalized differently (buy on
    /// dip: PCT = SIZE / 5 = 1/5); 0 = use each leg's weight
    pub lot_w: f64,
}

/// YT strategies whose AFL does NOT set UsePrevBarEquityForPosSizing (checked 19-11-2026 on every AFL the SV-Audit
/// maps to an SV id: 262 of 291 set it, these do not). Strategies not listed follow the team template (set).
const PREV_BAR_OFF: &[&str] = &[
    "ytbb_bb_athc_range_v2",
    "ytbb_bb_fresh_athc",
    "ytbb_fresh_athc",
    "ytne_expiry",
    "ytne_month_end",
    "ytst_bear",
    "ytst_bear_time_exit",
    "ytst_regim2",
    "ytst_regim_2st_cross",
    "ytst_regime_bear",
    "ytst_regime_bull",
    "ytst_simple_long",
    "ytst_simple_short",
    "ytst_st52_54_long",
    "ytst_st_all",
    "ytst_st_all_atr_exit",
    "ytsts_monthly_single_trade",
    "ytsw_diff_entry_exit",
    "ytsw_diff_entry_exit_ls",
    "ytsw_monthly_bear",
    "ytsw_monthly_bull",
    "ytsw_monthly_bull_buy",
    "ytsw_single",
    "ytsw_st_entry_swing_exit_long",
    "ytsw_st_entry_swing_exit_short",
    "ytsw_swing_entry_st_exit",
    "ytsws_single_trade",
];

/// YT strategies whose AFL scales in (`sigScaleIn`) with an active `SetPositionSize(..., spsPercentOfEquity)`
/// (checked 19-11-2026 on the AFL text): each lot is a % of the previous bar's equity.
const PER_LOT: &[&str] = &["ytrgs_red_green_scale", "ytst_st52_scale_in_atr", "ytsw_buy_on_dip", "ytsw_buy_on_dip_filter"];

impl Sizing {
    pub(crate) fn of(id: &str) -> Sizing {
        Sizing { prev_bar: id.starts_with("yt") && !PREV_BAR_OFF.contains(&id), per_lot: PER_LOT.contains(&id), lot_w: if id.starts_with("ytsw_buy_on_dip") { 0.2 } else { 0.0 } }
    }
}

/// AFL `Cross(a, b)` (an array function, not a loop comparison): after b's first value a Null b counts as -1e10,
/// as the YT Cross helpers were derived and checked. The "comparison with Null is false" rule is for AFL loops only.
pub(crate) const CROSS_NULL: f64 = -1e10;

/// A number as AmiBroker stores it: IEEE single precision (float32), used again as a double.
pub(crate) fn f32r(x: f64) -> f64 {
    (x as f32) as f64
}

/// The decimal price AmiBroker uses for a stored float32 price `x` (profit, trade list): the clean 2-decimal price
/// when the float32 is one, otherwise `x` rounded (half away from zero) to the significant digits float32 holds
/// at that size (Projects/Verification.md section 3: 1,024-8,192 keeps 3 decimals, 8,192-65,536 keeps 2, ...).
pub(crate) fn price_ab(x: f64) -> f64 {
    if !(x > 0.0) || !x.is_finite() {
        return x;
    }
    let r = (x * 100.0).round() / 100.0;
    if f32r(r) == x && x < 65_536.0 {
        return r;
    }
    let ulp = 2f64.powi(x.log2().floor() as i32 - 23);
    let dec = (-(1.5 * ulp).log10()).floor() as i32;
    let digits = (dec + x.log10().floor() as i32 + 1).max(1);
    let scale = 10f64.powi(digits - 1 - x.log10().floor() as i32);
    (x * scale).round() / scale
}

/// A calculated fill as AmiBroker makes it: the float32 level kept inside the bar's Low-High range (AmiBroker's
/// backtester never fills outside the bar), then read as AmiBroker's decimal price.
pub(crate) fn ab_fill(level32: f64, low: f64, high: f64) -> f64 {
    price_ab(f32r(level32).max(f32r(low)).min(f32r(high)))
}

/// `base x (1 + pct / 100)` in float32 as an AFL array formula computes it (`f32(base) x f32(1 + pct%)`).
pub(crate) fn lvl_pct(base: f64, pct: f64) -> f64 {
    f32r(f32r(base) * f32r(1.0 + pct / 100.0))
}

/// `a x b` as an AFL array formula computes it: both operands and the product are float32.
pub(crate) fn mul32(a: f64, b: f64) -> f64 {
    f32r(f32r(a) * f32r(b))
}

/// `base + base x k` as an AFL formula computes it (`O[i] - O[i] * Buffer / 100` is k = -Buffer / 100): the
/// float32 price, the arithmetic, the result stored as float32.
pub(crate) fn addmul(base: f64, k: f64) -> f64 {
    let b = f32r(base);
    f32r(b + b * k)
}

/// `base + v x m` as an AFL formula computes it on float32 arrays (`Close[i] + MyATR[i] * PT_Multi`): `base` and `v`
/// are float32 array elements, the product and the sum are stored as float32.
pub(crate) fn add_mul32(base: f64, v: f64, m: f64) -> f64 {
    f32r(f32r(base) + f32r(f32r(v) * m))
}

/// AmiBroker MA(v, n) of an array that may start with Nulls: Null until n values exist (counted from the array's
/// first value), then the simple average of the last n; a Null inside the window makes it Null.
pub(crate) fn ma(v: &[Option<f64>], n: usize) -> Vec<Option<f64>> {
    let n = n.max(1);
    let first = v.iter().position(|x| x.is_some()).unwrap_or(v.len());
    (0..v.len())
        .map(|i| {
            if i + 1 < first + n {
                return None;
            }
            v[i + 1 - n..=i].iter().try_fold(0.0, |s, x| x.map(|x| s + x)).map(|s| s / n as f64)
        })
        .collect()
}

/// AmiBroker RSI(n): average gain / loss start at 0 on bar 0 and follow Wilder's (prev x (n-1) + x) / n from bar 1;
/// Null before bar n (and everywhere for n <= 1).
pub(crate) fn rsi(c: &[f64], n: usize) -> Vec<Option<f64>> {
    let len = c.len();
    let mut out = vec![None; len];
    if n <= 1 || len <= n {
        return out;
    }
    let nf = n as f64;
    let (mut au, mut ad) = (0.0, 0.0);
    for i in 1..len {
        let d = c[i] - c[i - 1];
        au = (au * (nf - 1.0) + d.max(0.0)) / nf;
        ad = (ad * (nf - 1.0) + (-d).max(0.0)) / nf;
        if i >= n {
            out[i] = Some(if au + ad == 0.0 { 50.0 } else { 100.0 * au / (au + ad) });
        }
    }
    out
}

/// `rsi` with Null as NaN (for code written on `connors::rsi`'s Vec<f64> shape).
pub(crate) fn rsi_nan(c: &[f64], n: usize) -> Vec<f64> {
    rsi(c, n).into_iter().map(|v| v.unwrap_or(f64::NAN)).collect()
}

/// AmiBroker true range: High - Low on bar 0, then max(H, prev C) - min(L, prev C).
pub(crate) fn true_range(h: &[f64], l: &[f64], c: &[f64]) -> Vec<f64> {
    (0..c.len()).map(|i| if i == 0 { h[0] - l[0] } else { h[i].max(c[i - 1]) - l[i].min(c[i - 1]) }).collect()
}

/// AmiBroker ATR(n), computed in float32 as AmiBroker does (true range, seed and every Wilder step): seeded with the
/// simple average of the first n true ranges (bar 0's is High - Low), Wilder after, reported from bar n (SV-Audit
/// Date Sum: Ref(ATR(5), -1) is Null on bar 5; the float32 arithmetic reproduces AmiBroker's ATR levels to the
/// last digit where 64-bit was 1 unit off).
pub(crate) fn atr(h: &[f64], l: &[f64], c: &[f64], n: usize) -> Vec<Option<f64>> {
    let len = c.len();
    let n = n.max(1);
    let mut out = vec![None; len];
    if len < n {
        return out;
    }
    let tr: Vec<f32> = (0..len)
        .map(|i| {
            let (hi, lo) = (h[i] as f32, l[i] as f32);
            if i == 0 { hi - lo } else { hi.max(c[i - 1] as f32) - lo.min(c[i - 1] as f32) }
        })
        .collect();
    let nf = n as f32;
    let mut prev = tr[..n].iter().fold(0f32, |s, v| s + v) / nf;
    if n == 1 {
        out[0] = Some(prev as f64);
    }
    for i in n..len {
        prev = (prev * (nf - 1.0) + tr[i]) / nf;
        out[i] = Some(prev as f64);
    }
    out
}

#[cfg(test)]
pub(crate) fn near_ab(a: f64, b: f64) -> bool {
    (a - b).abs() <= 1e-3
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn price_conversion_matches_amibroker() {
        assert_eq!(price_ab(f32r(14.65)), 14.65);
        assert_eq!(price_ab(f32r(759.68994)), 759.6899);
        assert_eq!(price_ab(92.65625), 92.6563);
        assert_eq!(price_ab(f32r(75_502.45)), 75_502.5);
        // Double 7 target, NIFTY 50 2014-05-16: AmiBroker's profit uses 7317.805
        assert_eq!(ab_fill(lvl_pct(6652.55, 10.0), 7130.65, 7563.5), 7317.805);
        // a cover that gaps above the bar is filled at the High (NIFTYBANK 2006-09-29)
        assert_eq!(ab_fill(5519.55 * 1.0025, 5255.95, 5519.55), 5519.55);
    }

    #[test]
    fn rsi_starts_from_zero_and_atr_counts_bar_zero() {
        let c = [10.0, 11.0, 10.5, 11.5, 12.0];
        let r = rsi(&c, 2);
        // bar 1: au = 0.5, ad = 0; bar 2: au = 0.25, ad = 0.25 -> 50
        assert_eq!(r[1], None);
        assert!((r[2].unwrap() - 50.0).abs() < 1e-12);
        let (h, l) = ([11.0, 12.0, 11.0], [9.0, 10.0, 10.0]);
        let a = atr(&h, &l, &[10.0, 11.0, 10.5], 2);
        assert_eq!((a[0], a[1]), (None, None)); // seed (2 + 2) / 2 = 2 on bar 1, reported from bar 2
        assert!((a[2].unwrap() - 1.5).abs() < 1e-12); // (2 + 1) / 2
    }
}
