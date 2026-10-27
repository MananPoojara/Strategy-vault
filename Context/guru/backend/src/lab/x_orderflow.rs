//! Order flow (2026-10-26; spec: sv-orderflow/RESEARCH.md). Computed from a per-ticker FOOTPRINT file (footprint.rs:
//! volume at the bid and at the ask per price level per session) on daily bars; every output is NaN on a bar without
//! footprint rows, and a ticker without a file, or any W/M timeframe, gets needs_data. Delta = ask − bid = buy − sell on
//! every platform. Session extremes are the highest / lowest rows with volume (not the bar's high / low: the two feeds can
//! differ by a tick). Ids that need the time order inside the session or order-book depth are listed, never computed.
//! UNCONFIRMED (no exact public rule; not built): absorption, exhaustion, most Orderflows Trader studies, swing CVD
//! divergence, and trade counts (the footprint has no count column).
use super::bars::Bars;
use super::catalogue::{int, len, n, p, real, IndicatorDef, Needs, Output, ParamSpec, Params, Placement};
use super::footprint::{Day, Footprint};
use super::resample::{key, Tf};
use super::ta::NA;
use std::collections::BTreeMap;

#[allow(clippy::too_many_arguments)]
const fn def(id: &'static str, name: &'static str, params: &'static [ParamSpec], outputs: &'static [&'static str], placement: Placement,
             definition: &'static str, first: &'static [usize], sparse: bool, f: fn(&Bars, &Params) -> Output) -> IndicatorDef {
    IndicatorDef { id, name, category: CAT, params, outputs, placement, needs: OF, definition, first, sparse, compute: Some(f), note: "" }
}
const fn listed(id: &'static str, name: &'static str, needs: Needs, note: &'static str) -> IndicatorDef {
    IndicatorDef { id, name, category: CAT, params: &[], outputs: &["value"], placement: Placement::Pane, needs, definition: name,
                   first: &[usize::MAX], sparse: false, compute: None, note }
}
const CAT: &str = "Order flow";
const OF: Needs = Needs { order_flow: true, ..Needs::NONE };
const OF_TIME: Needs = Needs { intraday: true, order_flow: true, ..Needs::NONE };
const BOOK: Needs = Needs { order_book: true, ..Needs::NONE };
const TIME_NOTE: &str = "not available yet — needs time-ordered intraday (tick) order-flow data; a daily footprint has no time order";
const BOOK_NOTE: &str = "not available yet — needs order-book (depth-of-market) data";
const PR: Placement = Placement::Price;
const PN: Placement = Placement::Pane;

fn fp(b: &Bars) -> &Footprint { b.footprint.as_ref().expect("catalogue::run checks needs.order_flow") }
fn sums(d: &Day) -> (f64, f64) { (d.a.iter().sum(), d.b.iter().sum()) }
/// Dense rows lo..=hi between the lowest and highest rows with volume (missing rows are zero): (lo, bid, ask).
fn dense(d: &Day) -> Option<(i64, Vec<f64>, Vec<f64>)> {
    let live: Vec<usize> = (0..d.k.len()).filter(|&j| d.a[j] + d.b[j] > 0.).collect();
    let (lo, hi) = (d.k[*live.first()?], d.k[*live.last()?]);
    let (mut b, mut a) = (vec![0.; (hi - lo + 1) as usize], vec![0.; (hi - lo + 1) as usize]);
    for &j in &live { let r = (d.k[j] - lo) as usize; b[r] = d.b[j]; a[r] = d.a[j]; }
    Some((lo, b, a))
}
fn per_day(b: &Bars, outs: usize, f: impl Fn(usize, &Day) -> Vec<f64>) -> Output {
    let f2 = fp(b);
    let mut o = vec![vec![NA; b.len()]; outs];
    for i in 0..b.len() { if let Some(d) = f2.day(i) { for (k, v) in f(i, d).into_iter().enumerate() { o[k][i] = v; } } }
    Ok(o)
}
fn div(a: f64, b: f64) -> f64 { if b == 0. { NA } else { a / b } }

/// Point of control: the row with the most volume; ties go to the row closest to the profile middle, then the lower
/// (Sierra Chart). Rows are relative to `v[0]`.
fn poc(v: &[f64]) -> Option<usize> {
    let max = v.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    if !(max > 0.) { return None; }
    let mid2 = (v.len() - 1) as i64; // 2 × middle row
    (0..v.len()).filter(|&r| v[r] == max).min_by_key(|&r| ((2 * r as i64 - mid2).abs(), r))
}
/// Value area (rows lo..=hi relative to v[0]) around the POC; method 0 TradingView, 1 Sierra Chart, 2 two rows at a time.
fn value_area(v: &[f64], pc: usize, pct: f64, method: f64) -> (usize, usize) {
    let target = pct / 100. * v.iter().sum::<f64>();
    let (mut acc, mut lo, mut hi) = (v[pc], pc, pc);
    let up = |hi: usize, n: usize| (hi + 1..(hi + 1 + n).min(v.len())).map(|r| v[r]).sum::<f64>();
    let down = |lo: usize, n: usize| (lo.saturating_sub(n)..lo).map(|r| v[r]).sum::<f64>();
    let u_n = |hi: usize| v.len() - 1 - hi; // rows left above
    let d_n = |lo: usize| lo; // rows left below
    if method == 0. {
        // TradingView: add the larger next row (equal → the closer to the POC, then above) while it fits the target
        loop {
            let (cu, cd) = (u_n(hi) > 0, d_n(lo) > 0);
            let take_up = match (cu, cd) {
                (false, false) => break,
                (true, false) => true, (false, true) => false,
                (true, true) => { let (x, y) = (v[hi + 1], v[lo - 1]); x > y || (x == y && hi + 1 - pc <= pc - (lo - 1)) }
            };
            let add = if take_up { v[hi + 1] } else { v[lo - 1] };
            if acc + add > target { break; }
            acc += add;
            if take_up { hi += 1 } else { lo -= 1 }
        }
    } else {
        // Sierra Chart (1 row) / Market Profile (2 rows): add the larger side, both when equal, until the target is reached
        let n = if method == 1. { 1 } else { 2 };
        while acc < target {
            let (cu, cd) = (u_n(hi) > 0, d_n(lo) > 0);
            if !cu && !cd { break; }
            let (x, y) = (if cu { up(hi, n) } else { f64::NEG_INFINITY }, if cd { down(lo, n) } else { f64::NEG_INFINITY });
            if x >= y { acc += x; hi = (hi + n).min(v.len() - 1); }
            if y >= x { acc += y; lo = lo.saturating_sub(n); }
        }
    }
    (lo, hi)
}
/// (poc, vah, val) prices of a dense profile, or NaN when it has no volume.
fn profile(f: &Footprint, lo: i64, v: &[f64], pct: f64, method: f64) -> [f64; 3] {
    let Some(pc) = poc(v) else { return [NA; 3] };
    let (l, h) = value_area(v, pc, pct, method);
    [f.price(lo + pc as i64), f.price(lo + h as i64), f.price(lo + l as i64)]
}
fn total(b: &[f64], a: &[f64]) -> Vec<f64> { b.iter().zip(a).map(|(x, y)| x + y).collect() }

fn volume_delta(b: &Bars, _: &Params) -> Output { per_day(b, 1, |_, d| { let (a, s) = sums(d); vec![a - s] }) }
fn cvd(b: &Bars, prm: &Params) -> Output {
    let (f, anchor) = (fp(b), p(prm, "anchor"));
    let tf = [None, Some(Tf::W), Some(Tf::M), Some(Tf::Y)][anchor as usize];
    let (mut out, mut sum) = (vec![NA; b.len()], 0.);
    for i in 0..b.len() {
        if let (Some(t), true) = (tf, i > 0) { if key(&b.date[i], t) != key(&b.date[i - 1], t) { sum = 0.; } }
        if let Some(d) = f.day(i) { let (a, s) = sums(d); sum += a - s; out[i] = sum; }
    }
    Ok(vec![out])
}
fn up_down_volume(b: &Bars, _: &Params) -> Output { per_day(b, 3, |_, d| { let (a, s) = sums(d); vec![a, -s, a - s] }) }
fn delta_percent(b: &Bars, prm: &Params) -> Output {
    let basis = p(prm, "basis");
    per_day(b, 1, |_, d| { let (a, s) = sums(d); vec![100. * div(if basis == 0. { a - s } else { a.max(s) }, a + s)] })
}
const VA: &[ParamSpec] = &[real("value_area_pct", "Value area %", 70., 1., 100., 1.),
    int("method", "Value area method (0 TradingView, 1 Sierra Chart, 2 Market Profile two rows)", 0., 0., 2.)];
fn volume_profile(b: &Bars, prm: &Params) -> Output {
    let (f, pct, m) = (fp(b), p(prm, "value_area_pct"), p(prm, "method"));
    per_day(b, 3, |_, d| match dense(d) { Some((lo, bid, ask)) => profile(f, lo, &total(&bid, &ask), pct, m).to_vec(), None => vec![NA; 3] })
}
const COMPOSITE: &[ParamSpec] = &[len("sessions", "Sessions", 20.), real("value_area_pct", "Value area %", 70., 1., 100., 1.),
    int("method", "Value area method (0 TradingView, 1 Sierra Chart, 2 Market Profile two rows)", 0., 0., 2.)];
fn volume_profile_composite(b: &Bars, prm: &Params) -> Output {
    let (f, nn, pct, m) = (fp(b), n(prm, "sessions"), p(prm, "value_area_pct"), p(prm, "method"));
    per_day(b, 3, |i, _| {
        if i + 1 < nn { return vec![NA; 3]; }
        let mut rows: BTreeMap<i64, f64> = BTreeMap::new();
        for j in i + 1 - nn..=i { if let Some(d) = f.day(j) { for r in 0..d.k.len() { *rows.entry(d.k[r]).or_default() += d.a[r] + d.b[r]; } } }
        let live: Vec<(i64, f64)> = rows.into_iter().filter(|r| r.1 > 0.).collect();
        let (Some(lo), Some(hi)) = (live.first().map(|r| r.0), live.last().map(|r| r.0)) else { return vec![NA; 3] };
        let mut v = vec![0.; (hi - lo + 1) as usize];
        for (k, x) in live { v[(k - lo) as usize] = x; }
        profile(f, lo, &v, pct, m).to_vec()
    })
}
/// Session POC grid row of bar i (None without a footprint or volume).
fn poc_k(f: &Footprint, i: usize) -> Option<i64> {
    let (lo, bid, ask) = dense(f.day(i)?)?;
    poc(&total(&bid, &ask)).map(|r| lo + r as i64)
}
const NAKED: &[ParamSpec] = &[int("select", "Level (0 most recent, 1 nearest to the close)", 0., 0., 1.), int("lookback", "Look back (bars, 0 = unlimited)", 0., 0., 2000.)];
fn naked_poc(b: &Bars, prm: &Params) -> Output {
    let (f, sel, lb) = (fp(b), p(prm, "select"), n(prm, "lookback"));
    let (mut level, mut dist, mut count) = (vec![NA; b.len()], vec![NA; b.len()], vec![NA; b.len()]);
    let (mut open, mut started): (Vec<(usize, f64)>, bool) = (Vec::new(), false); // (session bar, POC price), untested
    for i in 0..b.len() {
        open.retain(|&(s, px)| !(b.low[i] <= px && px <= b.high[i]) && (lb == 0 || i - s <= lb));
        started |= f.day(i).is_some();
        if started {
            count[i] = open.len() as f64;
            let pick = if sel == 0. { open.last() } else {
                open.iter().min_by(|x, y| (b.close[i] - x.1).abs().total_cmp(&(b.close[i] - y.1).abs()).then(x.1.total_cmp(&y.1)))
            };
            if let Some(&(_, px)) = pick { level[i] = px; dist[i] = b.close[i] - px; }
        }
        if let Some(k) = poc_k(f, i) { open.push((i, f.price(k))); }
    }
    Ok(vec![level, dist, count])
}
fn poc_migration(b: &Bars, _: &Params) -> Output {
    let f = fp(b);
    let ks: Vec<Option<i64>> = (0..b.len()).map(|i| poc_k(f, i)).collect();
    Ok(vec![(0..b.len()).map(|i| match (i > 0).then(|| (ks[i - 1], ks[i])) { Some((Some(x), Some(y))) => (y - x).signum() as f64, _ => NA }).collect()])
}
const IMB: &[ParamSpec] = &[real("ratio_pct", "Imbalance ratio %", 300., 100., 10000., 1.), int("mode", "Comparison (0 diagonal, 1 horizontal)", 0., 0., 1.),
    int("zero", "Zero volume (0 counts, TradingView; 1 ignored, ATAS / Sierra Chart)", 0., 0., 1.), int("stack", "Stacked rows (at least)", 3., 2., 50.)];
fn runs(flags: &[bool], min: usize) -> f64 {
    let (mut count, mut run) = (0, 0);
    for &x in flags.iter().chain([false].iter()) { if x { run += 1 } else { if run >= min { count += 1; } run = 0; } }
    count as f64
}
fn footprint_imbalance(b: &Bars, prm: &Params) -> Output {
    let (r, diag, ignore, stack) = (p(prm, "ratio_pct") / 100., p(prm, "mode") == 0., p(prm, "zero") == 1., n(prm, "stack"));
    let beats = |x: f64, y: f64| x > y && x >= r * y && (!ignore || y > 0.);
    per_day(b, 4, |_, d| {
        let Some((_, bid, ask)) = dense(d) else { return vec![0.; 4] };
        let m = bid.len();
        let buy: Vec<bool> = (0..m).map(|k| if diag { k > 0 && beats(ask[k], bid[k - 1]) } else { beats(ask[k], bid[k]) }).collect();
        let sell: Vec<bool> = (0..m).map(|k| if diag { k + 1 < m && beats(bid[k], ask[k + 1]) } else { beats(bid[k], ask[k]) }).collect();
        let c = |v: &[bool]| v.iter().filter(|x| **x).count() as f64;
        vec![c(&buy), c(&sell), runs(&buy, stack), runs(&sell, stack)]
    })
}
const UA: &[ParamSpec] = &[real("min_volume", "Minimum volume per side", 0., 0., 1e12, 1.), int("rule", "Rule (0 both sides at the extreme, LuxAlgo; 1 ATAS)", 0., 0., 1.)];
fn unfinished_auction(b: &Bars, prm: &Params) -> Output {
    let (mv, rule) = (p(prm, "min_volume"), p(prm, "rule"));
    per_day(b, 2, |_, d| {
        let Some((_, bid, ask)) = dense(d) else { return vec![NA; 2] };
        let h = bid.len() - 1;
        let flag = |x: bool| if x { 1. } else { 0. };
        if rule == 0. { vec![flag(ask[h] > mv && bid[h] > mv), flag(ask[0] > mv && bid[0] > mv)] } else { vec![flag(bid[h] > mv), flag(ask[0] > mv)] }
    })
}
fn delta_divergence(b: &Bars, _: &Params) -> Output {
    per_day(b, 1, |i, d| {
        let (a, s) = sums(d);
        let (o, c, dl) = (b.open[i], b.close[i], a - s);
        vec![if c > o && dl < 0. { 1. } else if c < o && dl > 0. { -1. } else { 0. }]
    })
}
fn level_delta_extremes(b: &Bars, _: &Params) -> Output {
    let f = fp(b);
    per_day(b, 4, |_, d| {
        let live: Vec<usize> = (0..d.k.len()).filter(|&j| d.a[j] + d.b[j] > 0.).collect();
        let (Some(&first), Some(&last)) = (live.first(), live.last()) else { return vec![NA; 4] };
        let mid2 = d.k[first] + d.k[last];
        // ties: closest to the profile middle, then the lower (the POC rule)
        let rank = |j: &usize| ((2 * d.k[*j] - mid2).abs(), d.k[*j]);
        let delta = |j: usize| d.a[j] - d.b[j];
        let mx = live.iter().copied().max_by(|x, y| delta(*x).total_cmp(&delta(*y)).then(rank(y).cmp(&rank(x)))).unwrap();
        let mn = live.iter().copied().min_by(|x, y| delta(*x).total_cmp(&delta(*y)).then(rank(x).cmp(&rank(y)))).unwrap();
        vec![delta(mx), delta(mn), f.price(d.k[mx]), f.price(d.k[mn])]
    })
}
fn buy_sell_vwap(b: &Bars, _: &Params) -> Output {
    let f = fp(b);
    per_day(b, 3, |_, d| {
        let w = |vol: &dyn Fn(usize) -> f64| { let (mut pv, mut v) = (0., 0.); for j in 0..d.k.len() { pv += f.price(d.k[j]) * vol(j); v += vol(j); } div(pv, v) };
        vec![w(&|j| d.a[j] + d.b[j]), w(&|j| d.a[j]), w(&|j| d.b[j])]
    })
}

pub const DEFS: &[IndicatorDef] = &[
    def("volume_delta", "Volume Delta", &[], &["delta"], PN,
        "Volume Delta: Σ ask volume − Σ bid volume of the session footprint (buy − sell aggressor volume; NinjaTrader 8 bar Delta, GoCharting orderflow.delta). It matches TradingView's Volume Delta close in its 1-tick bid/ask mode, not its candle-direction fallback. NaN on a bar without footprint rows.",
        &[0], false, volume_delta),
    def("cvd", "Cumulative Volume Delta", &[int("anchor", "Reset (0 never, 1 week, 2 month, 3 year)", 0., 0., 3.)], &["cvd"], PN,
        "Cumulative Volume Delta: running sum of each session's delta (ask − bid volume), reset to 0 on the first bar of each anchor period (TradingView \"Anchor period\": week, month, year; 0 never resets, like Bookmap CVD; the default is a lab choice, TradingView's default is not published). A bar without footprint is NaN and the total carries forward unchanged (GoCharting); a period boundary still resets it. GoCharting orderflow.cvd() resets every session, which on daily bars equals Volume Delta.",
        &[0], false, cvd),
    def("up_down_volume", "Up/Down Volume", &[], &["up", "down", "delta"], PN,
        "Up/Down Volume (TradingView ta.requestUpAndDownVolume read on the footprint): up = Σ ask volume (buy volume), down = −Σ bid volume (sell volume, negative), delta = up + down. The footprint total can differ from the daily bar's volume (different feed, off-book trades) and is not reconciled.",
        &[0, 0, 0], false, up_down_volume),
    def("delta_percent", "Delta %", &[int("basis", "Basis (0 delta ÷ volume, NinjaTrader / Quantower / GoCharting bar statistics; 1 prominent side ÷ volume, GoCharting cluster)", 0., 0., 1.)], &["pct"], PN,
        "Delta %: basis 0 = 100 × (ask − bid) ÷ (ask + bid) (NinjaTrader 8, Quantower, GoCharting Bar Statistics; −100..100); basis 1 = 100 × max(ask, bid) ÷ volume (GoCharting Cluster page; 50..100). NaN when the session has no volume.",
        &[0], false, delta_percent),
    def("volume_profile", "Volume Profile (session POC/VAH/VAL)", VA, &["poc", "vah", "val"], PR,
        "Session volume profile from the footprint (volume per price row). POC = the row with the most volume; ties go to the row closest to the middle of the profile, then the lower (Sierra Chart). Value area around the POC up to Value area % of the session volume (70 = TradingView, Sierra Chart, GoCharting; NinjaTrader uses 68): method 0 TradingView adds the larger next row above or below (equal: the closer to the POC, then above) while it does not exceed the target; method 1 Sierra Chart adds the larger row (both when equal) until the target is reached or passed; method 2 Market Profile compares the next two rows above with the next two below (equal pairs: both, a declared reading). Rows missing between the session's lowest and highest rows count as zero. NaN when the session has no volume.",
        &[0, 0, 0], false, volume_profile),
    def("volume_profile_composite", "Composite Volume Profile (N sessions)", COMPOSITE, &["poc", "vah", "val"], PR,
        "Composite volume profile: the footprints of the last N daily bars (this bar included) summed row by row, then POC and value area as in Volume Profile (session). Sessions without footprint add nothing (NinjaTrader composite profiles); NaN on a bar without footprint and before N bars. NinjaTrader 'Days Back' has no published default; 20 is the lab's.",
        &[19, 19, 19], false, volume_profile_composite),
    def("naked_poc", "Naked (untested) POC", NAKED, &["level", "distance", "count"], PR,
        "Naked POC: session POCs of earlier bars that no later bar has traded through (daily low ≤ POC ≤ high), as of this bar's close (Sierra Chart extension line until future intersection, TradingView 'Extend POC Right', NinjaTrader naked POC, GoCharting). level = the most recent untested POC (select 0) or the one nearest the close (select 1; equidistant → the lower); distance = close − level; count = untested POCs. Look back limits them to POCs from the last N bars (0 = unlimited, as the platforms). count is NaN only before the first footprint session.",
        &[usize::MAX, usize::MAX, 0], true, naked_poc),
    def("poc_migration", "POC migration", &[], &["dir"], PN,
        "POC migration (lab definition on the published session POC; no platform ships it as a study): +1 when this session's POC is above the previous bar's, −1 below, 0 on the same row; NaN when either has no footprint.",
        &[1], true, poc_migration),
    def("footprint_imbalance", "Footprint imbalances (bid/ask, stacked)", IMB, &["buy", "sell", "buy_stacks", "sell_stacks"], PN,
        "Bid/ask imbalances in the session footprint. Diagonal (default, every platform): a buy imbalance at a row when its ask volume exceeds the bid volume one row lower and is at least Imbalance ratio % of it; a sell imbalance when the bid volume exceeds the ask volume one row higher by the same ratio (no comparison below the session low or above its high). Horizontal (NinjaTrader 8): ask vs bid on the same row. Zero volume: TradingView compares against zero (any volume beats 0); ATAS 'Ignore zero values' and Sierra Chart skip zero denominators. Outputs the number of buy and sell imbalance rows and the number of stacks: runs of at least 'Stacked rows' consecutive rows with a same-side imbalance (LuxAlgo, Orderflows Trader). Defaults: TradingView 300 %; LuxAlgo 300 % and 3 rows; GoCharting ratio 3; ATAS 150 %; NinjaTrader 1.5 (set Imbalance ratio % to match).",
        &[0, 0, 0, 0], false, footprint_imbalance),
    def("unfinished_auction", "Unfinished auction (session high/low)", UA, &["high", "low"], PN,
        "Unfinished auction at the session's highest and lowest rows with volume: rule 0 (LuxAlgo, MarketDelta) = both ask and bid volume above Minimum volume at that row; rule 1 (ATAS) = bid volume at the high, ask volume at the low. 1 / 0 events; NaN when the session has no volume.",
        &[0, 0], true, unfinished_auction),
    def("delta_divergence", "Delta divergence (bar vs delta)", &[], &["div"], PN,
        "Delta divergence (Orderflows Trader on GoCharting): +1 when the bar closes up (close > open) with a negative session delta, −1 when it closes down with a positive delta, else 0. Swing CVD-versus-price divergence has no published pivot rule and is not offered.",
        &[0], true, delta_divergence),
    def("level_delta_extremes", "Max / min level delta", &[], &["max_delta", "min_delta", "max_price", "min_price"], PN,
        "Largest and smallest per-row delta (ask − bid) in the session footprint and their prices (NinjaTrader 8 'Show maximum' in Delta style; Sierra Chart Numbers Bars per-row difference). Ties: closest to the profile middle, then the lower. Not the intrabar running min/max delta (listed: Min / Max Delta).",
        &[0, 0, 0, 0], false, level_delta_extremes),
    def("buy_sell_vwap", "Session VWAP, Buy VWAP, Sell VWAP", &[], &["vwap", "buy_vwap", "sell_vwap"], PR,
        "End-of-session VWAP from the footprint (GoCharting VWAP, BVWAP, SVWAP): Σ price × volume ÷ Σ volume over all, ask-only and bid-only volume. Prices are the footprint rows, so it can differ from a trade-by-trade VWAP by up to half a row. NaN when the volume is zero.",
        &[0, 0, 0], false, buy_sell_vwap),
    listed("delta_min_max", "Min / Max Delta (delta bars high/low)", OF_TIME, TIME_NOTE),
    listed("delta_since_high_low", "Delta since high / low", OF_TIME, TIME_NOTE),
    listed("extreme_delta", "Extreme delta (Orderflows Trader)", OF_TIME, TIME_NOTE),
    listed("big_trades", "Big / Power trades", OF_TIME, TIME_NOTE),
    listed("market_profile_tpo", "Market Profile (TPO)", OF_TIME, TIME_NOTE),
    listed("developing_value_area", "Developing POC / VAH / VAL", OF_TIME, TIME_NOTE),
    listed("order_book_imbalance", "Order-book (depth) imbalance", BOOK, BOOK_NOTE),
    listed("liquidity_heatmap", "Liquidity heatmap", BOOK, BOOK_NOTE),
    listed("dom_pulling_stacking", "DOM pulling / stacking", BOOK, BOOK_NOTE),
    listed("iceberg_detection", "Iceberg orders", BOOK, BOOK_NOTE),
    listed("spoofing_detection", "Spoofing / flashing liquidity", BOOK, BOOK_NOTE),
];
pub const SOURCES: &[(&str, &str, &str)] = &[
    ("volume_delta", "Volume Delta", ""),
    ("cvd", "Cumulative Volume Delta", ""),
    ("up_down_volume", "Up/Down Volume", ""),
    ("delta_percent", "NinjaTrader 8 / Quantower / GoCharting: Delta %", ""),
    ("volume_profile", "Volume Profile Fixed Range; Volume Profile Visible Range; Session Volume Profile", "PlotVAPOverlayA"),
    ("volume_profile_composite", "NinjaTrader 8 / GoCharting: Composite profile", ""),
    ("naked_poc", "Session Volume Profile: Extend POC Right; Sierra Chart / NinjaTrader / GoCharting: naked POC", ""),
    ("poc_migration", "Session POC (TradingView SVP / Sierra Chart) — lab comparison", ""),
    ("footprint_imbalance", "Volume Footprint (imbalance); NinjaTrader / ATAS / Sierra Chart / GoCharting imbalances", ""),
    ("unfinished_auction", "ATAS / LuxAlgo: Unfinished auction", ""),
    ("delta_divergence", "Orderflows Trader (GoCharting): Delta Divergence", ""),
    ("level_delta_extremes", "NinjaTrader 8 / Sierra Chart: max/min row delta", ""),
    ("buy_sell_vwap", "GoCharting: VWAP, Buy VWAP and Sell VWAP", ""),
    ("delta_min_max", "Volume Delta (high/low); NinjaTrader / GoCharting min/max delta", ""),
    ("delta_since_high_low", "NinjaTrader 8: Delta SH/SL", ""),
    ("extreme_delta", "Orderflows Trader (GoCharting): Extreme Delta", ""),
    ("big_trades", "GoCharting Power Trades; Bookmap Large Lot Tracker", ""),
    ("market_profile_tpo", "Sierra Chart / GoCharting: Market Profile (TPO)", ""),
    ("developing_value_area", "Session Volume Profile: Developing POC/VA", ""),
    ("order_book_imbalance", "GoCharting / Bookmap: order book (DOM)", ""),
    ("liquidity_heatmap", "Bookmap / GoCharting: liquidity heatmap", ""),
    ("dom_pulling_stacking", "Jigsaw: Depth & Sales pulling / stacking", ""),
    ("iceberg_detection", "Bookmap: Stops & Icebergs", ""),
    ("spoofing_detection", "Bookmap / GoCharting: spoofing", ""),
];

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lab::catalogue::{get, run};
    use crate::lab::footprint::File;
    use std::sync::Arc;

    /// Daily bars over `dates` (open, close from `oc`; high/low from the footprint rows ±0.5) with a footprint from rows.
    fn bars(dates: &[&str], oc: &[(f64, f64)], rows: &[(&str, f64, f64, f64)]) -> Bars {
        let rws: Vec<crate::Row> = dates.iter().zip(oc).map(|(d, &(o, c))| crate::Row { ticker: "T".into(), date: d.to_string(), open: o, high: o.max(c) + 50., low: o.min(c) - 50., close: c, volume: Some(1.) }).collect();
        let mut b = Bars::from_rows(&rws);
        let file = File::from_rows(&rows.iter().map(|r| (r.0.to_string(), r.1, r.2, r.3)).collect::<Vec<_>>()).unwrap();
        b.footprint = Some(Footprint::align(Arc::new(file), &b.date));
        b
    }
    fn out(id: &str, b: &Bars, prm: &[(&str, f64)]) -> Vec<Vec<f64>> {
        run(&get(id).unwrap(), b, &prm.iter().map(|(k, v)| (k.to_string(), *v)).collect()).unwrap()
    }
    fn same(a: &[f64], b: &[f64]) -> bool { a.len() == b.len() && a.iter().zip(b).all(|(x, y)| (x.is_nan() && y.is_nan()) || (x - y).abs() < 1e-9) }

    #[test] fn deltas_and_cvd_anchors_with_a_missing_day() {
        // Mon 2024-01-29 .. Fri 2024-02-02 (month and week boundary on Feb 1 / Jan 29); no footprint on Jan 31
        let d = ["2024-01-29", "2024-01-30", "2024-01-31", "2024-02-01", "2024-02-02"];
        let rows = [("2024-01-29", 100., 10., 30.), ("2024-01-30", 100., 40., 10.), ("2024-02-01", 101., 5., 25.), ("2024-02-02", 100., 10., 10.)];
        let b = bars(&d, &[(100., 101.); 5], &rows);
        assert!(same(&out("volume_delta", &b, &[])[0], &[20., -30., NA, 20., 0.]));
        assert!(same(&out("cvd", &b, &[])[0], &[20., -10., NA, 10., 10.]));
        assert!(same(&out("cvd", &b, &[("anchor", 2.)])[0], &[20., -10., NA, 20., 20.]), "month resets on Feb 1");
        assert!(same(&out("cvd", &b, &[("anchor", 1.)])[0], &[20., -10., NA, 10., 10.]), "one ISO week");
        let ud = out("up_down_volume", &b, &[]);
        assert!(same(&ud[0], &[30., 10., NA, 25., 10.]) && same(&ud[1], &[-10., -40., NA, -5., -10.]) && same(&ud[2], &[20., -30., NA, 20., 0.]));
        assert!(same(&out("delta_percent", &b, &[])[0], &[50., -60., NA, 200. / 3., 0.]));
        assert!(same(&out("delta_percent", &b, &[("basis", 1.)])[0], &[75., 80., NA, 250. / 3., 50.]));
        let dv = &out("delta_divergence", &b, &[])[0]; // every bar closes up
        assert!(same(dv, &[0., 1., NA, 0., 0.]));
    }
    #[test] fn poc_ties_and_the_three_value_area_methods() {
        // rows 0..=6 with volumes [4, 8, 8, 1, 8, 3, 2]: POC ties at rows 1, 2, 4 → row 2 is nearer the middle (3) than row 4? |2·2−6| = 2, |2·4−6| = 2 → lower: row 2
        let v = [4., 8., 8., 1., 8., 3., 2.];
        assert_eq!(poc(&v), Some(2));
        // total 34, 70 % = 23.8. TradingView from row 2 (8): up row 3 (1) vs down row 1 (8) → down 16; then row 3 (1) vs row 0 (4) → row 0 20;
        // then row 3 (1) only above → 21; row 4 (8) → 29 > 23.8 stop → rows 0..=3
        assert_eq!(value_area(&v, 2, 70., 0.), (0, 3));
        // Sierra: 8 → +row 1 (8) = 16 → row 3 (1) vs row 0 (4): +4 = 20 → row 3 (1) only: 21 → row 4 (8): 29 ≥ 23.8 → rows 0..=4
        assert_eq!(value_area(&v, 2, 70., 1.), (0, 4));
        // two rows: up rows 3+4 = 9 vs down rows 1+0 = 12 → down: 20 → up 9 → 29 ≥ 23.8 → rows 0..=4
        assert_eq!(value_area(&v, 2, 70., 2.), (0, 4));
        // equal neighbours, TradingView: the closer to the POC, then above
        assert_eq!(value_area(&[1., 5., 9., 5., 1.], 2, 70., 0.), (2, 3));
        assert_eq!(value_area(&[1., 5., 9., 5., 1.], 2, 60., 0.), (2, 2), "TradingView never exceeds the target");
        // Sierra includes both equal rows
        assert_eq!(value_area(&[1., 5., 9., 5., 1.], 2, 60., 1.), (1, 3));
        assert_eq!(value_area(&[7.], 0, 70., 0.), (0, 0));
        assert_eq!(poc(&[0., 0.]), None);
    }
    #[test] fn session_profile_composite_and_naked_poc() {
        // grid step 1: day 0 POC at 101, day 1 POC at 103, day 2 POC at 100
        let d = ["2024-03-04", "2024-03-05", "2024-03-06"];
        let rows = [("2024-03-04", 100., 1., 1.), ("2024-03-04", 101., 3., 3.), ("2024-03-04", 102., 1., 0.),
                    ("2024-03-05", 102., 1., 1.), ("2024-03-05", 103., 4., 4.), ("2024-03-06", 100., 5., 5.), ("2024-03-06", 101., 1., 0.)];
        let mut b = bars(&d, &[(100., 101.); 3], &rows);
        let vp = out("volume_profile", &b, &[]);
        assert!(same(&vp[0], &[101., 103., 100.]));
        // day 0: total 9, target 6.3: POC 6 → +row 100 (2) = 8 > 6.3 stop → VA = POC only
        assert!(same(&vp[1], &[101., 103., 100.]) && same(&vp[2], &[101., 103., 100.]));
        let c = out("volume_profile_composite", &b, &[("sessions", 2.)]);
        // bars 0+1: rows 100:2 101:6 102:3 103:8 → POC 103
        assert!(same(&c[0], &[NA, 103., 100.]), "bars 1+2: 100:10 101:1 102:2 103:8 → POC 100");
        // naked POC: the bars' ranges (low/high ±50 around 100/101) cover every POC, so nothing stays untested …
        assert!(same(&out("naked_poc", &b, &[])[2], &[0., 0., 0.]));
        // … narrow the ranges: bar 1 trades 102.5..104, bar 2 trades 99.5..100.5
        b.low = vec![99.5, 102.5, 99.5]; b.high = vec![101.5, 104., 100.5]; b.close = vec![101., 103., 100.];
        let np = out("naked_poc", &b, &[]);
        assert!(same(&np[2], &[0., 1., 2.]) && same(&np[0], &[NA, 101., 103.]) && same(&np[1], &[NA, 2., -3.]));
        assert!(same(&out("naked_poc", &b, &[("select", 1.)])[0], &[NA, 101., 101.]), "nearest to the close 100");
        assert!(same(&out("naked_poc", &b, &[("lookback", 1.)])[2], &[0., 1., 1.]));
        assert!(same(&out("poc_migration", &b, &[])[0], &[NA, 1., -1.]));
    }
    #[test] fn imbalances_stacks_unfinished_auctions_extremes_and_vwap() {
        // rows 100..104 (bid, ask): 100 (5,1) 101 (1,20) 102 (2,9) 103 (0,7) 104 (3,3)
        let d = ["2024-04-01"];
        let rows = [("2024-04-01", 100., 5., 1.), ("2024-04-01", 101., 1., 20.), ("2024-04-01", 102., 2., 9.), ("2024-04-01", 103., 0., 7.), ("2024-04-01", 104., 3., 3.)];
        let b = bars(&d, &[(100., 101.)], &rows);
        // diagonal buy: a(101)=20 vs b(100)=5 ✓; a(102)=9 vs b(101)=1 ✓; a(103)=7 vs b(102)=2 ✓ (≥ 6); a(104)=3 vs b(103)=0 ✓ (TradingView counts zero)
        // diagonal sell: b(100)=5 vs a(101)=20 ✗; b(101)=1 vs 9 ✗; b(102)=2 vs 7 ✗; b(103)=0 ✗
        let im = out("footprint_imbalance", &b, &[]);
        assert!(same(&im.iter().map(|o| o[0]).collect::<Vec<_>>(), &[4., 0., 1., 0.]));
        let ig = out("footprint_imbalance", &b, &[("zero", 1.), ("stack", 2.)]);
        assert!(same(&ig.iter().map(|o| o[0]).collect::<Vec<_>>(), &[3., 0., 1., 0.]), "104 vs a zero bid is skipped");
        // horizontal: buy where ask ≥ 3 × bid and above it: 101, 102, 103; sell where bid ≥ 3 × ask: 100
        let h = out("footprint_imbalance", &b, &[("mode", 1.)]);
        assert!(same(&h.iter().map(|o| o[0]).collect::<Vec<_>>(), &[3., 1., 1., 0.]));
        let ua = out("unfinished_auction", &b, &[]);
        assert!(same(&[ua[0][0], ua[1][0]], &[1., 1.]), "both sides traded at 104 and at 100");
        let ua1 = out("unfinished_auction", &b, &[("rule", 1.), ("min_volume", 3.)]);
        assert!(same(&[ua1[0][0], ua1[1][0]], &[0., 0.]), "ATAS: bid 3 at the high and ask 1 at the low are not above 3");
        let ex = out("level_delta_extremes", &b, &[]);
        assert!(same(&ex.iter().map(|o| o[0]).collect::<Vec<_>>(), &[19., -4., 101., 100.]));
        let w = out("buy_sell_vwap", &b, &[]);
        let vw = (100. * 6. + 101. * 21. + 102. * 11. + 103. * 7. + 104. * 6.) / 51.;
        let bw = (100. + 101. * 20. + 102. * 9. + 103. * 7. + 104. * 3.) / 40.;
        let sw = (100. * 5. + 101. + 102. * 2. + 104. * 3.) / 11.;
        assert!(same(&[w[0][0], w[1][0], w[2][0]], &[vw, bw, sw]));
    }
    #[test] fn zero_volume_session_and_missing_data() {
        let b = bars(&["2024-05-02", "2024-05-03"], &[(1., 2.); 2], &[("2024-05-02", 100., 0., 0.)]);
        assert!(same(&out("volume_delta", &b, &[])[0], &[0., NA]));
        assert!(out("volume_profile", &b, &[])[0][0].is_nan() && out("delta_percent", &b, &[])[0][0].is_nan());
        assert!(same(&out("footprint_imbalance", &b, &[])[0], &[0., NA]));
        // no footprint → the catalogue refuses with the file name; W bars never carry one
        let mut plain = b.clone(); plain.footprint = None; plain.footprint_file = "NIFTY_50__FOOTPRINT.csv".into();
        let e = run(&get("cvd").unwrap(), &plain, &Params::new()).unwrap_err();
        let msg = crate::lab::series::catalogue_error(&get("cvd").unwrap(), e);
        assert!(matches!(&msg, crate::lab::rules::LabError::NeedsData(m) if m.contains("NIFTY_50__FOOTPRINT.csv") && m.contains("order-flow")), "{msg:?}");
        let w = crate::lab::resample::resample(&b, Tf::W);
        assert!(w.footprint.is_none());
        assert!(run(&get("order_book_imbalance").unwrap(), &b, &Params::new()).is_err());
    }
    #[test] fn order_flow_outputs_are_causal() {
        let dates: Vec<String> = (0..60).map(|i| super::super::resample::civil(19_725 + i * 7 / 5)).collect();
        let mut rows = Vec::new();
        for (i, d) in dates.iter().enumerate().filter(|(i, _)| i % 7 != 3) {
            for r in 0..((i * 5) % 9 + 1) { rows.push((d.as_str(), 100. + r as f64 * 0.5 + (i % 4) as f64, ((i * 3 + r * 7) % 11) as f64, ((i * 5 + r * 3) % 13) as f64)); }
        }
        let ds: Vec<&str> = dates.iter().map(String::as_str).collect();
        let oc: Vec<(f64, f64)> = (0..60).map(|i| (100. + (i % 5) as f64, 101. + (i % 3) as f64)).collect();
        let b = bars(&ds, &oc, &rows);
        for d in DEFS.iter().filter(|d| d.compute.is_some()) {
            let full = run(d, &b, &Params::new()).unwrap();
            for cut in 5..60 {
                let mut pre = b.clone();
                for v in [&mut pre.open, &mut pre.high, &mut pre.low, &mut pre.close] { v.truncate(cut); }
                pre.date.truncate(cut); pre.end.truncate(cut);
                if let Some(v) = pre.volume.as_mut() { v.truncate(cut); }
                if let Some(f) = pre.footprint.as_mut() { f.truncate(cut); }
                let part = run(d, &pre, &Params::new()).unwrap();
                for (k, s) in part.iter().enumerate() { assert!(same(s, &full[k][..cut]), "{}.{} changes before bar {cut}", d.id, d.outputs[k]); }
            }
        }
    }
}
