//! Futures & Options Trader magazine (futuresandoptionstrader.com), "Futures Trading System Lab" columns of May 2007 - Jan 2008
//! by Volker Knapp (Wealth-Lab). Research share Trading Knowledge I/@Newsletters/Futures & Options Trader Magazine 2005 - 2010
//! (futuresandoptionstrader.com)/2007/ and /2008/; text authors/batch-i/i1w/full2/fot025-fot033.txt (pdftotext). Book id `fot07`.
//! Lane K14 (titles loop, 2 Nov 2026), to-code rows 224-230; lead decisions 83-84; independent check authors/titles-1030/K14w/k14_ref.py.
use crate::connors::{atr, dmi, sma, Ctx, ParamDef, Params, Status, StrategyDef, Trade};
use super::lc3::{hi, high_first, lo};
use super::tselhs::Book;

/// Admitted ids (authors/titles-1030/K14w/k14_ref.py (header)).
pub const REVIEWED: &[&str] = &["fot07_adx_bo", "fot07_kama_range", "fot07_csi_channel", "fot07_pmi_basic", "fot07_pullback_target", "fot07_gap_closer", "fot07_bb_squeeze"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

macro_rules! wl { () => { " Wealth-Lab semantics as the columns describe them: the system runs at each close; entry orders are placed only while flat and exit orders only for a position held at that close, both for the next bar only (market = the open; a stop fills at its price, or at the open if the open is already through it; a limit at its price or the better open). Two entry stops (long and short) fill in the order the price reaches them (the nearer extreme of the open first), and the other is cancelled. One position at a time, one unit; the columns' 1-2 percent equity risk sizing is not applied. The columns tested a 20-market ratio-adjusted futures portfolio (Pinnacle data, $8 commission and slippage); their results are the magazine's, not check targets (lead decision 70). Daily indices, ETFs and US/India stocks are a declared application; the vault's standard costs." } }

const ADX: &str = concat!("[Declared readings] F&OT May 2007 p.28-30, 'ADX Consolidation Breakout 1': if the 14-day ADX (Wilder) has been under 20 on each of the last 30 sessions (today included), place for tomorrow a buy stop at the highest high and a sell-short stop at the lowest low of the past 20 days (today included). A long exits on a stop at the lowest low of the past 5 days, a short on a stop at the highest high of the past 5 days, re-set every day (trailing). 'No stops are used' beyond those.", wl!());
const KAMA: &str = concat!("[Declared readings] F&OT Jun 2007 p.28-30, 'KAMA range trader': KAMA = Kaufman's adaptive average of the close, look-back 10, fastest 2 and slowest 30 periods (the magazine's Key Concepts p.47-48: sc = [ER x (fastest - slowest) + slowest]^2, fastest = 2/3, slowest = 2/31, ER = |C - C[10]| / sum of the 10 one-day absolute changes; 0 when there is no change); it starts at the close of the 10th bar. When the 5-day rate of change of KAMA, (KAMA / KAMA 5 bars ago - 1) x 100, is between -0.5 and +0.5 (inclusive): tomorrow a buy stop at yesterday's high if yesterday's open was below the previous day's low; a sell-short stop at yesterday's low if yesterday's open was above the previous day's high. 'Yesterday' is read literally from today's close (the day before the signal day; option p.day = 0 reads it as the signal day itself: today's open below the prior low, stop at today's high). A long exits on a stop at the lowest low of the past 2 days, a short at the highest high of the past 2 days (today included).", wl!());
const CSI: &str = concat!("[Declared readings] F&OT Jul 2007 p.28-30, 'Commodity Selection Index', TEST 1 ONLY (the plain system): buy tomorrow at the market if today's close is the highest close of the past 20 days (today included, a tie counts); sell short if it is the lowest close. A long exits on a stop at the lowest close of the past 10 days (today included), a short at the highest close. Test 2 (only the five highest-CSI contracts of a 20-futures portfolio) is held: it ranks futures the vault does not hold (lead decision 84).", wl!());
const PMI: &str = concat!("[Declared readings] F&OT Aug 2007 p.32-34, 'Price Movement Index', BASIC SYSTEM ONLY: buy tomorrow at the market when each of the last 10 closes (today included) was above the close 20 days before it; sell short when each was below. A long exits on a stop at the lowest close of the most recent 20 days, a short at the highest close. The PMI market selection (the five highest 14-day PMI readings of 20 futures) is held: it needs the futures basket (lead decision 84).", wl!());
const PB: &str = concat!("[Declared readings] F&OT Sep 2007 p.28-30, 'Trend pullback with progressive target': if today's close is above its 20-day SMA, a buy limit for tomorrow at today's close - 1.5 x ATR(14) (Wilder); if below, a sell-short limit at the close + 1.5 x ATR(14). A long exits with a limit at the most recent two-day swing high + 3 x ATR(14), a short with a limit at the most recent two-day swing low - 3 x ATR(14), re-set every day. The column defines the swing (p.28): 'a swing high is a price bar preceded and followed by bars with lower highs ... two-day swings, which requires two consecutive lower highs before and after a swing high bar', read as the high above the highs of the 2 bars before and the 2 bars after (strict), so a swing is known 2 bars after it; no exit order until one exists. Lead decision 83 had taken 'recent two-day swing high' as the highest high of the last two bars because the reader found no definition; that reading is option p.swing = 1. ATR multiple 3 ('selected from the area of stable values', 1.0-4.0 tested) is p.atrx. The column notes the exit 'doesn't offer any loss protection'.", wl!());
const GAP: &str = "[Declared readings] F&OT Oct 2007 p.32-34, 'Consecutive gap closer': buy today at the open when (a) today's open is below yesterday's low and yesterday's open was below the low of the day before, (b) yesterday's high is below the low of the day before (the first gap not filled) and (c) today's gap (yesterday's low - today's open) is smaller than yesterday's gap (the low of the day before - yesterday's open), in points. Short = the mirror (opens above the prior highs, yesterday's low above the high of the day before, a smaller up gap). Exit today with a limit at yesterday's close, otherwise at today's close (the limit fills at its price; the open is beyond it by construction). A one-day trade. Different from atm03_gap_closer (Active Trader May 2003: one down gap larger than ATR(20)). The column's result is weak (1,063 trades, net +12.03% in 20 years on 20 futures) and it says so. Daily indices, ETFs and US/India stocks are a declared application; the vault's standard costs.";
const BB: &str = concat!("[Declared readings] F&OT Jan 2008 p.22-23, 'Bollinger Band breakout-anticipation system': Bollinger Bands 20-day SMA +/- 2 population standard deviations; BandWidth = (upper - lower) / SMA. If today's BandWidth is the lowest of the last 100 days (today included) and upper - lower < 2.5 x ATR(10) (Wilder): tomorrow a buy stop at today's high + 1 tick and a sell-short stop at today's low - 1 tick, whichever comes first (tick 0.01 US, 0.05 NSE, lead decision 57; p.tick = 0 removes it). Exits: after 10 bars in the position, at the next open (a close 10 or more bars after the entry bar), and a stop at the 5-day low (long) / 5-day high (short), today included. Default = both exits, whichever comes first: the column's own statistics need the channel exit (losers held 5.13 days on average, winners 10.29, all 7.36), and the time exit fits 10.29 days only when the 10 bars are counted after the entry bar. p.exit = 1 time exit only, 2 channel exit only.", wl!());

pub fn catalogue() -> Vec<StrategyDef> {
    let d = |id, name, rules, notes| StrategyDef { id, name, group: "Futures & Options Trader 2007-08 (Futures Trading System Lab)", session: 0, dir: "Long/Short", status: Status::Live,
        needs_volume: false, needs_vix: false, needs_series: &[], book: "fot07", rules, notes, claim: None };
    vec![
        d("fot07_adx_bo", "ADX Consolidation Breakout 1: after 30 days of ADX(14) under 20, stop entries at the 20-day high or low, trail out at the 5-day low/high", &[
            "F|\"If the 14-day ADX has remained under 20 for the past 30 trading sessions, then either:\" (F&OT May 2007 p.28)",
            "E|\"a) enter long the next day with a stop at the highest high of the past 20 days, or b) enter short the next day with a stop at the lowest low of the past 20 days.\" (p.28)",
            "X|\"Close long position tomorrow with a trailing stop at the lowest low of the past five days. Cover short position tomorrow with a trailing stop at the highest high of the past five days.\" (p.28)"], ADX),
        d("fot07_kama_range", "KAMA range trader: while the 5-day ROC of KAMA(10) is within +/-0.5%, fade an outside open with a stop at the prior high/low, out at the 2-day low/high", &[
            "F|\"If the five-day rate of change (ROC) of the 10-day KAMA is between -0.5 percent and +0.5 percent:\" (F&OT Jun 2007 p.29)",
            "E|\"1. Enter long tomorrow with a stop at yesterday's high if yesterday's opening price was below the previous day's low. 2. Enter short tomorrow with a stop at yesterday's low if yesterday's opening price was above the previous day's high.\" (p.29)",
            "X|\"3. Exit long position with a stop at the lowest low of the past two days. 4. Cover short with a stop at the highest high of the past two days.\" (p.29)"], KAMA),
        d("fot07_csi_channel", "Commodity Selection Index column, test 1: 20-day closing-high/low breakout at the next open, out on a stop at the 10-day closing low/high", &[
            "E|\"1. Enter long tomorrow at the market if today's close is the highest close of the past 20 days. 2. Enter short tomorrow at the market if today's close is the lowest close of the past 20 days.\" (F&OT Jul 2007 p.30, test 1)",
            "X|\"3. Exit long position on a stop at the lowest close of the past 10 days. 4. Exit short position on a stop at the highest close of the past 10 days.\" (p.30)"], CSI),
        d("fot07_pmi_basic", "Price Movement Index column, basic system: after 10 closes in a row above (below) the close 20 days earlier, enter at the next open; out at the 20-day closing low/high", &[
            "E|\"2. Enter long tomorrow at the market when there have been 10 consecutive closes above the trailing close from 20 days ago. 3. Enter short tomorrow at the market when there have been 10 consecutive closes below the trailing close from 20 days ago.\" (F&OT Aug 2007 p.33-34)",
            "X|\"4. Exit long position on a stop at the lowest close of the most recent 20 days. 5. Exit short position on a stop at the highest close of the most recent 20 days.\" (p.33-34)"], PMI),
        d("fot07_pullback_target", "Trend pullback with progressive target: above the 20-day SMA buy a limit 1.5 ATR under the close, below it short 1.5 ATR over; exit at the last two-day swing + 3 ATR", &[
            "E|\"1. If yesterday's close was above the 20-day moving average of closing prices, enter long with a limit order at yesterday's close minus 1.5 times the 14-day ATR. 2. If yesterday's close was below the 20-day moving average of closing prices, enter short with a limit order at yesterday's close plus 1.5 times the 14-day ATR.\" (F&OT Sep 2007 p.29)",
            "X|\"3. Exit long position with a limit order at the recent two-day swing high point plus three times the 14-day ATR. 4. Exit short position with a limit order at the recent two-day swing low point minus three times the 14-day ATR.\" (p.29-30)",
            "F|\"a swing high is a price bar preceded and followed by bars with lower highs ... This approach will use two-day swings, which requires two consecutive lower highs before and after a swing high bar\" (p.28)"], PB),
        d("fot07_gap_closer", "Consecutive gap closer: fade the second opening gap in a row at the open when the first is unfilled and the second smaller; out at yesterday's close or today's close", &[
            "E|\"1. Enter long today at the market when: a) today's open is below yesterday's low and yesterday's open is below the previous day's low; b) yesterday's high is below the previous day's low; c) today's gap is smaller than yesterday's gap (in points).\" (F&OT Oct 2007 p.32)",
            "E|\"2. Enter short today at the market when: a) today's open is above yesterday's high and yesterday's open is above the previous day's high; b) yesterday's low is above the previous day's high; c) today's gap is smaller than yesterday's gap (in points).\" (p.32)",
            "X|\"Exit position: a) today with a limit order at yesterday's closing price, or b) today at the close.\" (p.32)"], GAP),
        d("fot07_bb_squeeze", "Bollinger Band breakout-anticipation: at a 100-day low in BandWidth with the bands inside 2.5 ATR(10), stop entries 1 tick beyond the day's range; out after 10 bars or at the 5-day low/high", &[
            "F|\"If the Band Width is the lowest it's been in 100 days and the absolute distance between the upper and lower Bollinger Bands is less than 2.5 times the 10-day average true range (ATR):\" (F&OT Jan 2008 p.22)",
            "E|\"1. Go long tomorrow with a stop loss order at today's high plus one tick, or go short tomorrow with a stop-loss order at today's low minus one tick, whichever comes first.\" (p.22)",
            "X|\"2. After 10 bars in a position, exit tomorrow at market. 3. Alternately, exit on a penetration of the five-day low (for a long position) or the five-day high\" (p.22)"], BB),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    let f = match id {
        "fot07_adx_bo" => ("F&OT ADX consolidation breakout", "ADX(14) < 20 for 30 days, 20-day stops"),
        "fot07_kama_range" => ("F&OT KAMA range trader", "KAMA(10) ROC(5) within 0.5%"),
        "fot07_csi_channel" => ("F&OT close-channel system (CSI column)", "20-day close breakout, 10-day close stop"),
        "fot07_pmi_basic" => ("F&OT PMI column basic system", "10 closes beyond the close 20 days back"),
        "fot07_pullback_target" => ("F&OT trend pullback, progressive target", "1.5 ATR pullback, swing + 3 ATR"),
        "fot07_gap_closer" => ("F&OT consecutive gap closer", "second opening gap, same-day exit"),
        "fot07_bb_squeeze" => ("F&OT Bollinger breakout anticipation", "100-day BandWidth low, 10-bar exit"),
        _ => return None,
    };
    let id = REVIEWED.iter().find(|r| **r == id)?;
    Some((*id, f.0, f.1))
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    Some(match id {
        "fot07_adx_bo" | "fot07_csi_channel" | "fot07_pmi_basic" | "fot07_gap_closer" => vec![],
        "fot07_kama_range" => vec![p("day", "Pattern day: 1 = 'yesterday' read literally (the day before the signal day); 0 = the signal day itself", 1., 0., 1., 1.)],
        "fot07_pullback_target" => vec![p("swing", "Swing for the exit: 0 = the column's two-day swing high/low; 1 = highest high / lowest low of the last 2 bars (lead decision 83)", 0., 0., 1., 1.),
            p("atrx", "Exit ATR multiple (3 chosen by the column; 1-4 tested)", 3., 1., 4., 0.5)],
        "fot07_bb_squeeze" => vec![p("exit", "Exits: 0 = 10-bar time exit and 5-day channel stop, whichever first; 1 = time exit only; 2 = channel stop only", 0., 0., 2., 1.),
            p("tick", "Entry stop offset in ticks (1 = 'one tick'; 0 = none)", 1., 0., 10., 1.)],
        _ => return None,
    })
}

fn g(p: &Params, k: &str, id: &str) -> f64 {
    p.get(k).copied().unwrap_or_else(|| params(id).and_then(|s| s.iter().find(|d| d.key == k).map(|d| d.def)).unwrap_or(f64::NAN))
}

#[derive(Clone, Copy, PartialEq)]
enum K { Mkt, Stop, Lim }
/// A next-bar order: `d` = +1 buy, -1 sell.
#[derive(Clone, Copy)]
struct Ord { d: f64, k: K, px: f64, why: &'static str }

/// When and where an order fills on bar i: (0 = at the open, 1 = first leg, 2 = second leg; price), or None.
fn when(x: &Ctx, i: usize, z: &Ord) -> Option<(u8, f64)> {
    let (o, h, l) = (x.o[i], x.h[i], x.l[i]);
    if z.k == K::Mkt { return Some((0, o)); }
    let up = (z.k == K::Stop) == (z.d > 0.); // buy stop / sell limit fill on the way up
    let leg = if up == high_first(o, h, l) { 1 } else { 2 };
    if up { if o >= z.px { Some((0, o)) } else if h >= z.px { Some((leg, z.px)) } else { None } }
    else if o <= z.px { Some((0, o)) } else if l <= z.px { Some((leg, z.px)) } else { None }
}

/// The first order on bar i's path (ties: the order listed first).
fn first(x: &Ctx, i: usize, ords: &[Ord]) -> Option<(Ord, f64)> {
    let mut best: Option<(u8, Ord, f64)> = None;
    for z in ords { if let Some((w, px)) = when(x, i, z) { if best.map_or(true, |b| w < b.0) { best = Some((w, *z, px)); } } }
    best.map(|b| (b.1, b.2))
}

/// Kaufman's adaptive moving average of `c` (look-back n, fastest 2, slowest 30), from bar n-1 (seed = that close).
pub fn kama(c: &[f64], n: usize) -> Vec<f64> {
    let (fast, slow) = (2. / 3., 2. / 31.);
    let mut k = vec![f64::NAN; c.len()];
    if c.len() < n { return k; }
    k[n - 1] = c[n - 1];
    for i in n..c.len() {
        let noise: f64 = (i + 1 - n..=i).map(|j| (c[j] - c[j - 1]).abs()).sum();
        let er = if noise > 0. { (c[i] - c[i - n]).abs() / noise } else { 0. };
        let sc = (er * (fast - slow) + slow).powi(2);
        k[i] = k[i - 1] + sc * (c[i] - k[i - 1]);
    }
    k
}

/// Population standard deviation of the last n values (NaN before n).
fn psd(v: &[f64], n: usize) -> Vec<f64> {
    (0..v.len()).map(|i| if i + 1 < n { f64::NAN } else {
        let w = &v[i + 1 - n..=i]; let m = w.iter().sum::<f64>() / n as f64;
        (w.iter().map(|x| (x - m).powi(2)).sum::<f64>() / n as f64).sqrt()
    }).collect()
}

/// The most recent two-day swing high (`up`) or low known at close t: bar k <= t-2 beyond the 2 bars on each side.
fn swing(v: &[f64], t: usize, up: bool) -> Option<f64> {
    (2..=t.checked_sub(2)?).rev().find(|&k| (k - 2..=k + 2).filter(|&j| j != k).all(|j| if up { v[j] < v[k] } else { v[j] > v[k] })).map(|k| v[k])
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.r.len() { return None; }
    let (o, h, l, c) = (&x.o, &x.h, &x.l, &x.c);
    let mut pos = Book::new();
    if id == "fot07_gap_closer" {
        for t in from.max(2)..=to {
            let long = o[t] < l[t - 1] && o[t - 1] < l[t - 2] && h[t - 1] < l[t - 2] && l[t - 1] - o[t] < l[t - 2] - o[t - 1];
            let short = o[t] > h[t - 1] && o[t - 1] > h[t - 2] && l[t - 1] > h[t - 2] && o[t] - h[t - 1] < o[t - 1] - h[t - 2];
            let d = if long { 1. } else if short { -1. } else { continue };
            pos.enter(d, t, o[t], if d > 0. { "buy at the open" } else { "short at the open" });
            if (d > 0. && h[t] >= c[t - 1]) || (d < 0. && l[t] <= c[t - 1]) { pos.exit(t, c[t - 1], "limit at yesterday's close"); }
            else { pos.exit(t, c[t], "at the close"); }
        }
        return Some(pos.out);
    }
    let n = x.r.len();
    let none = || vec![f64::NAN; n];
    let adx = if id == "fot07_adx_bo" { dmi(x.r, 14).1 } else { vec![] };
    let km = if id == "fot07_kama_range" { kama(c, 10) } else { vec![] };
    let (ma20, a14) = if id == "fot07_pullback_target" { (sma(c, 20), atr(x.r, 14)) } else { (none(), none()) };
    let (bw, bwd, a10) = if id == "fot07_bb_squeeze" {
        let (m, s) = (sma(c, 20), psd(c, 20));
        ((0..n).map(|i| 4. * s[i] / m[i]).collect::<Vec<f64>>(), s.iter().map(|s| 4. * s).collect::<Vec<f64>>(), atr(x.r, 10))
    } else { (vec![], vec![], vec![]) };
    let day = g(p, "day", id).round() as usize;
    let tick = g(p, "tick", id).round().max(0.) * x.market_tick();
    let mut ords: Vec<Ord> = vec![];
    for t in from..=to {
        if t > from {
            let held = pos.side();
            if let Some((z, px)) = first(x, t, &std::mem::take(&mut ords)) {
                if held != 0. { pos.exit(t, px, z.why); } else { pos.enter(z.d, t, px, z.why); }
            }
        }
        if t == to { break; }
        let d = pos.side();
        let o = |d, k, px, why| Ord { d, k, px, why };
        if d == 0. {
            match id {
                "fot07_adx_bo" => if t >= 29 && (t - 29..=t).all(|k| adx[k] < 20.) {
                    ords.push(o(1., K::Stop, hi(h, t.saturating_sub(19), t), "buy stop, 20-day high"));
                    ords.push(o(-1., K::Stop, lo(l, t.saturating_sub(19), t), "short stop, 20-day low"));
                },
                "fot07_kama_range" => {
                    let s = t.saturating_sub(day); // the pattern day
                    if t >= 5 && s >= 2 && ((km[t] / km[t - 5] - 1.) * 100.).abs() <= 0.5 {
                        if x.o[s] < l[s - 1] { ords.push(o(1., K::Stop, h[s], "buy stop at the pattern day's high")); }
                        if x.o[s] > h[s - 1] { ords.push(o(-1., K::Stop, l[s], "short stop at the pattern day's low")); }
                    }
                }
                "fot07_csi_channel" => if t >= 19 {
                    if c[t] >= hi(c, t - 19, t) { ords.push(o(1., K::Mkt, f64::NAN, "buy at the open, 20-day closing high")); }
                    else if c[t] <= lo(c, t - 19, t) { ords.push(o(-1., K::Mkt, f64::NAN, "short at the open, 20-day closing low")); }
                },
                "fot07_pmi_basic" => if t >= 29 {
                    if (t - 9..=t).all(|k| c[k] > c[k - 20]) { ords.push(o(1., K::Mkt, f64::NAN, "buy at the open, 10 closes above")); }
                    else if (t - 9..=t).all(|k| c[k] < c[k - 20]) { ords.push(o(-1., K::Mkt, f64::NAN, "short at the open, 10 closes below")); }
                },
                "fot07_pullback_target" => {
                    if c[t] > ma20[t] { ords.push(o(1., K::Lim, c[t] - 1.5 * a14[t], "buy limit, close - 1.5 ATR")); }
                    else if c[t] < ma20[t] { ords.push(o(-1., K::Lim, c[t] + 1.5 * a14[t], "short limit, close + 1.5 ATR")); }
                }
                "fot07_bb_squeeze" => if t >= 118 && bw[t] <= lo(&bw, t - 99, t) && bwd[t] < 2.5 * a10[t] {
                    ords.push(o(1., K::Stop, h[t] + tick, "buy stop, high + tick"));
                    ords.push(o(-1., K::Stop, l[t] - tick, "short stop, low - tick"));
                },
                _ => return None,
            }
        } else {
            let e = pos.entry_bar().unwrap_or(t);
            match id {
                "fot07_adx_bo" => ords.push(if d > 0. { o(-1., K::Stop, lo(l, t.saturating_sub(4), t), "5-day low trailing stop") } else { o(1., K::Stop, hi(h, t.saturating_sub(4), t), "5-day high trailing stop") }),
                "fot07_kama_range" => ords.push(if d > 0. { o(-1., K::Stop, lo(l, t.saturating_sub(1), t), "2-day low stop") } else { o(1., K::Stop, hi(h, t.saturating_sub(1), t), "2-day high stop") }),
                "fot07_csi_channel" => ords.push(if d > 0. { o(-1., K::Stop, lo(c, t.saturating_sub(9), t), "10-day closing low stop") } else { o(1., K::Stop, hi(c, t.saturating_sub(9), t), "10-day closing high stop") }),
                "fot07_pmi_basic" => ords.push(if d > 0. { o(-1., K::Stop, lo(c, t.saturating_sub(19), t), "20-day closing low stop") } else { o(1., K::Stop, hi(c, t.saturating_sub(19), t), "20-day closing high stop") }),
                "fot07_pullback_target" => {
                    let m = g(p, "atrx", id) * a14[t];
                    let s = if g(p, "swing", id).round() == 1. { Some(if d > 0. { hi(h, t.saturating_sub(1), t) } else { lo(l, t.saturating_sub(1), t) }) }
                        else if d > 0. { swing(h, t, true) } else { swing(l, t, false) };
                    if let Some(s) = s { ords.push(if d > 0. { o(-1., K::Lim, s + m, "limit at swing high + 3 ATR") } else { o(1., K::Lim, s - m, "limit at swing low - 3 ATR") }); }
                }
                "fot07_bb_squeeze" => {
                    let ex = g(p, "exit", id).round();
                    if ex != 2. && t - e >= 10 { ords.push(o(-d, K::Mkt, f64::NAN, "10 bars in the position")); }
                    if ex != 1. { ords.push(if d > 0. { o(-1., K::Stop, lo(l, t.saturating_sub(4), t), "5-day low stop") } else { o(1., K::Stop, hi(h, t.saturating_sub(4), t), "5-day high stop") }); }
                }
                _ => return None,
            }
        }
    }
    Some(pos.done(to, c[to]))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;
    /// A deterministic random walk (64-bit LCG): quiet and trending stretches like real daily bars.
    fn walk(t: &str, n: usize) -> Vec<Row> {
        let (mut s, mut c) = (12345u64, 100f64);
        let mut u = || { s = s.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407); (s >> 11) as f64 / (1u64 << 53) as f64 };
        (0..n).map(|i| { let o = c * (1. + 0.012 * (u() - 0.5)); c = o * (1. + 0.02 * (u() - 0.5));
            let (h, l) = (o.max(c) * (1. + 0.006 * u()), o.min(c) * (1. - 0.006 * u()));
            Row { ticker: t.into(), date: format!("{:05}", i), open: o, high: h, low: l, close: c, volume: None } }).collect()
    }
    #[test]
    fn fot_labs_trade_both_sides() {
        let r = walk("T", 3000);
        let x = Ctx::new(&r, &r, &[]);
        for id in REVIEWED {
            let t = run(id, &x, 0, r.len() - 1, &Params::new()).unwrap();
            assert!(!t.is_empty() && t.iter().any(|z| z.dir > 0.) && t.iter().any(|z| z.dir < 0.), "{id} {}", t.len());
            for w in t.windows(2) { assert!(w[1].legs[0].i >= w[0].exit_i, "{id}"); }
            assert!(family_of(id).is_some() && params(id).is_some());
        }
        // gap closer: one-day trades only
        assert!(run("fot07_gap_closer", &x, 0, r.len() - 1, &Params::new()).unwrap().iter().all(|z| z.exit_i == z.legs[0].i));
        // KAMA reproduces a constant series and the swing finder needs two lower bars on each side
        assert!(kama(&[5.; 30], 10)[29] == 5.);
        assert_eq!(swing(&[1., 2., 5., 3., 2., 1., 0.], 6, true), Some(5.));
        assert_eq!(swing(&[1., 2., 5., 3., 6., 1., 0.], 6, true), Some(6.));
        assert_eq!(swing(&[1., 2., 5., 6., 2.], 4, true), None);
        assert!(run("fot07_x", &x, 0, 5, &Params::new()).is_none());
    }
}

// Instrument admission: Futures & Options Trader 2007-08 2026-11-02 (titles lane K14).
pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book == "fot07" && reviewed(def.id),
    label: "Futures Trading System Lab columns tested on a 20-market futures portfolio. Runs on daily indices, ETFs and stocks, US and India, are a declared application.",
    basis: "Futures & Options Trader, May 2007 - Jan 2008, Futures Trading System Lab (Volker Knapp, Wealth-Lab), p.22-34 of each issue; independent check authors/titles-1030/K14w/k14_ref.py",
};
