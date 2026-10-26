//! William Dunnigan, "Gains in Grains" (1952, revised June 1954), as Part IV of "New Blueprints for Gains in
//! Stocks and Grains" (Traders' Masterclass reprint pp.95-198); original research-report scan also on file.
//! Refs/oracle: library/wdgig/.
use crate::connors::{Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "wdgig";
/// Admitted ids (authors/dunnigan/admission/SOURCE-CONTRACT.md).
pub const REVIEWED: &[&str] = &["wdgig_swing25"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const SWING_NOTE: &str = "1940-42 wheat research method (\"fitted ... to the 1930-1941 period\", p.112). Wheat futures rule run on the index (instrument swap; all its numbers are percentages). Swings from daily highs and lows: a Bottom is set when, at least 3 trading days after the lowest low since the last Top, the day's high is 2 1/2% or more above that low (or 6% or more in fewer days); Tops mirror. Signals use swings set by the previous day's close: buy stop 1% above the last Top, sell stop 1% below the last Bottom (\"open stop orders\", p.117-118), fill at the level or at the open if it gaps through; stop-and-reverse with no other exit, as in the book's own March 1954 Wheat test (\"buying on a Buy Signal and this was held, come what may, until a Sell Signal\", p.114). Book silent (assumptions): the 3 days are counted as trading days between the Bottom day and the confirming day; equal lows/highs keep the earliest; if both stops are hit on one day while flat, the one nearer the open is taken. Book result (March 1954 Wheat, 2 gains, 0 losses, 50 3/4c gross, 49 3/4c net per bushel) cannot be compared: no grain futures data.";

const GRAIN_GAP: &str = "Needs daily Chicago grain futures (wheat/corn/oats/rye/soybeans), individual contracts with their delivery months: every threshold is in cents per bushel (e.g. TOB/TOT within 1c, Thrust between 1/2c and 1 1/2c, stops in cents). No percentage or index-point equivalent is given; the author says other markets need other figures he does not supply (\"some smaller figure should be used for oats and some larger figure for soybeans\", p.148).";
const COTTON_GAP: &str = "Needs daily cotton futures contracts (thresholds in cotton points: TOB/TOT 10 points, Thrust 15-25 points, stops 50 points, p.175).";
const DB_RISE_GAP: &str = "Entry technique only: the book gives the double-bottom rise / double-top decline as how the 1940-41 second method \"sought to buy and sell at more favorable prices\" but does not say whether it is a stand-alone stop-and-reverse or combined with the 1% penetration signals, nor any exit (p.118-119).";
const PLAN8_GAP: &str = "The signal source for Plan #8 is not stated (\"tried in the 1941-1942 studies\"), \"2% net profit\" depends on 1941 grain commissions, and what happens after a profit/loss exit (re-entry) is not given (p.161).";

pub fn catalogue() -> Vec<StrategyDef> {
    let d = |id, name, group, dir, status, rules, notes| StrategyDef {
        id, name, group, session: 0, dir, status, needs_volume: false, needs_vix: false,
        needs_series: &[], book: BOOK, rules, notes, claim: None,
    };
    use Status::*;
    vec![
        d("wdgig_swing25", "2 1/2%-3-day swings, 1% penetration (1940-42)", "Evolution of a trading barometer", "Long/Short", Live, &[
            "F|\"the swings which gave the best signals during the years 1930-1941 were those which measured at least 2 1/2% provided they occupied at least three trading days. (Swings of 6% or more which occurred in less than three days were also considered meaningful ...)\" p.113-114",
            "F|\"it was required that the time elapsed between the Bottom and the Top was at least three trading days, and that during this time the market rose at least 2 1/2%. Hence, movements of one or two days were ignored (unless the move was 6% or more)\" p.114",
            "E|Fig 4.3: \"BUY if rise goes 1% above this Top\"; \"SELL if decline goes 1% below this Bottom\" p.114",
            "X|\"buying on a Buy Signal and this was held, come what may, until a Sell Signal was later given, at which time he went short and held on until a new Buy Signal\" p.114-115",
        ], SWING_NOTE),
        d("wdgig_breakaway", "Breakaway Method (5c swings, 1c penetration, 1934)", "Evolution of a trading barometer", "Long/Short", NeedsData(GRAIN_GAP), &[
            "E|\"plotted swings of 5c or more in prices and then proceeded to buy and sell in 1c penetrations of the Tops and Bottoms of those swings\" p.107",
            "X|Operating Plans #3 (2 1/2c profit / 5c loss), #4 (5c / 4 1/2c), #5 (trail 6c) p.157-158",
        ], GRAIN_GAP),
        d("wdgig_238", "2 3/8c Swing Method (1939)", "Evolution of a trading barometer", "Long/Short", NeedsData(GRAIN_GAP), &[
            "E|Swings of 2 3/8c (2c below 50c wheat); buy 1 1/8c above the last Top, sell 1 1/8c below the last Bottom (above $1.10) p.108-110",
            "X|Plan #6 ride/quick-profit schedule in cents p.158-159",
        ], GRAIN_GAP),
        d("wdgig_pct3", "Percentage Wheat Method (3% swings, 1940)", "Evolution of a trading barometer", "Long/Short", NeedsData(GRAIN_GAP), &[
            "E|\"a Buy Signal was given when the last Top of one of these 3%-or-more swings was penetrated upward by 1 1/4c and a Sell Signal was given when the last Bottom was broken downward by 1 1/4c\" p.111",
        ], GRAIN_GAP),
        d("wdgig_double", "Double Top/Bottom by-products (2c)", "Evolution of a trading barometer", "Long/Short", NeedsData(GRAIN_GAP), &[
            "F|\"a Double Bottom pattern was present if the last Bottom was 2c or less, either above or below, the preceding Bottom\" p.115",
            "E|\"wait until a Double Top is set up, and then we buy 1% above the highest of these two Tops\" (Tops within 2c) p.116",
        ], GRAIN_GAP),
        d("wdgig_db_rise", "Buying on rise from Double Bottom (2%, 3 days, 2 1/2%)", "Evolution of a trading barometer", "Long/Short", NotTestable(DB_RISE_GAP), &[
            "E|\"A possible Double Bottom exists today if the Low Price in the current Downswing is within 2% (above or below) either or both of the two preceding Bottoms. In such case, BUY if prices rise 3 or more days and 2 1/2% or more above the Low Price.\" p.119",
            "E|Sell short on a 3-day, 2 1/2% decline from a High within 2% of either of the two preceding Tops p.119",
        ], DB_RISE_GAP),
        d("wdgig_plan8", "Plan #8: take 2% net profit, limit loss to 2 1/2%", "Growth of operating plans", "Long/Short", NotTestable(PLAN8_GAP), &[
            "X|\"take a 2% net profit or a 2-1/2% loss (plus commission) whichever came first\" p.161",
        ], PLAN8_GAP),
        d("wdgig_thrust", "The Thrust Method (Rules 1-3, Repeat Signals)", "The thrust method", "Long/Short", NeedsData(GRAIN_GAP), &[
            "F|\"An UPSWING starts when a day's high and low prices rise above the respective high and low prices of the latest Downswing ... A DOWNSWING starts when a day's high and low prices decline below the respective high and low prices of the highest day of the latest Upswing\" p.123",
            "F|Patterns: TOB/TOT (within 1c of either of the two preceding Bottoms/Tops), CPR, NR, IR, PT/PB p.125-133",
            "E|Rule 1: TOB or CPR at the last Bottom, today the lowest day not more than 1c below it and a TOB or CPR, Thrust tomorrow above today's high by today's range (min 1/2c, max 1 1/2c); Rule 2: two Pattern days in one swing + Thrust; Rule 3: PT then Thrust above the PT day's high p.134-141",
            "A|Repeat Signals: CPR on the low day, TOB followed by IR, or TOB + 1/2-range Thrust; one per swing, never in the Reversal swing, never on PT/PB p.145-148",
        ], GRAIN_GAP),
        d("wdgig_plan9", "Plan #9: two funds (L and S) with the Thrust Method", "Growth of operating plans", "Long/Short", NeedsData(GRAIN_GAP), &[
            "X|Fund L: limit loss 5c, break even after 4c, take 10c or Reversal; Fund S on Repeat Signal with 2c paper profit in L, 5c stop, 5c profit (grains $2.00-$2.75) p.161-163",
        ], GRAIN_GAP),
        d("wdgig_sr", "Operating Plan S/R (three funds, square-root tables)", "Operating plan S/R", "Long/Short", NeedsData(GRAIN_GAP), &[
            "X|Funds L, M, S on Thrust Method Reversal/Repeat Signals; stops and profits from Tables 4.3/4.4 by grain price in cents (e.g. $2.00 wheat: 4 1/4c stop, break even after 3 1/2c, Fund L protects half of paper profit) p.164-172",
        ], GRAIN_GAP),
        d("wdgig_cotton", "The Thrust Method in Cotton", "Odds and ends", "Long/Short", NeedsData(COTTON_GAP), &[
            "E|TOB/TOT within 10 points; Reversal Thrust = today's range, 15-25 points; Repeat Thrust = half range, 10-15 points p.175",
            "X|Limit loss 50 points; break even after 35; protect 25 after 75; 50 after 100; take 200 (confirmed) / 100 (repeat) p.175",
        ], COTTON_GAP),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    match id {
        "wdgig_swing25" => Some(("wdgig_swing", "Dunnigan swing-penetration methods", "2 1/2%-3-day swings (book)")),
        _ if id.starts_with("wdgig_") && catalogue().iter().any(|s| s.id == id) => Some(("wdgig_other", "Dunnigan Gains in Grains · grain methods", "Not runnable here")),
        _ => None,
    }
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    match id {
        "wdgig_swing25" => Some(vec![
            p("swing", "Swing size %", 2.5, 1., 6., 0.5),
            p("days", "Minimum trading days between Bottom and Top", 3., 1., 6., 1.),
            p("fast", "Swing % that counts in fewer days (0 = off)", 6., 0., 10., 1.),
            p("pen", "Penetration % beyond the last Top/Bottom", 1., 0., 3., 0.25),
        ]),
        _ => None,
    }
}

/// Swing levels known at the close of each day: (last Top, last Bottom), NaN until set.
pub fn swings(h: &[f64], l: &[f64], to: usize, sw: f64, days: usize, fast: f64) -> Vec<(f64, f64)> {
    let mut v = Vec::with_capacity(to + 1);
    let (mut top, mut bot) = (f64::NAN, f64::NAN);
    let (mut leg, mut ext) = (0i8, 0usize); // 1 = up leg from a Bottom (ext = highest high), -1 = down leg (ext = lowest low)
    let (mut lo0, mut hi0) = (0usize, 0usize);
    let ok_up = |b: usize, k: usize| (k >= b + days && h[k] >= l[b] * (1.0 + sw)) || (fast > 0.0 && k > b && h[k] >= l[b] * (1.0 + fast));
    let ok_dn = |t: usize, k: usize| (k >= t + days && l[k] <= h[t] * (1.0 - sw)) || (fast > 0.0 && k > t && l[k] <= h[t] * (1.0 - fast));
    for k in 0..=to {
        match leg {
            1 => {
                if h[k] > h[ext] { ext = k; }
                if ok_dn(ext, k) { top = h[ext]; leg = -1; ext = (ext..=k).fold(ext, |a, j| if l[j] < l[a] { j } else { a }); }
            }
            -1 => {
                if l[k] < l[ext] { ext = k; }
                if ok_up(ext, k) { bot = l[ext]; leg = 1; ext = (ext..=k).fold(ext, |a, j| if h[j] > h[a] { j } else { a }); }
            }
            _ => {
                if l[k] < l[lo0] { lo0 = k; }
                if h[k] > h[hi0] { hi0 = k; }
                if ok_up(lo0, k) && (lo0 > hi0 || !ok_dn(hi0, k)) { bot = l[lo0]; leg = 1; ext = (lo0..=k).fold(lo0, |a, j| if h[j] > h[a] { j } else { a }); }
                else if ok_dn(hi0, k) { top = h[hi0]; leg = -1; ext = (hi0..=k).fold(hi0, |a, j| if l[j] < l[a] { j } else { a }); }
            }
        }
        v.push((top, bot));
    }
    v
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    let schema = params(id)?;
    if from > to || to >= x.r.len() { return None; }
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).unwrap().def);
    let (sw, days, fast, pen) = (g("swing") / 100.0, g("days") as usize, g("fast") / 100.0, g("pen") / 100.0);
    let lv = swings(&x.h, &x.l, to, sw, days, fast);
    let mut out = vec![];
    let mut cur: Option<Trade> = None;
    for i in from.max(1)..=to {
        let (top, bot) = lv[i - 1];
        let buy = top * (1.0 + pen);
        let sell = bot * (1.0 - pen);
        let dir = cur.as_ref().map(|t| t.dir).unwrap_or(0.0);
        let hit_b = dir <= 0.0 && top.is_finite() && x.h[i] >= buy;
        let hit_s = dir >= 0.0 && bot.is_finite() && x.l[i] <= sell;
        let sig = match (hit_b, hit_s) {
            (true, true) => if (x.o[i] - buy).abs() <= (x.o[i] - sell).abs() { 1.0 } else { -1.0 },
            (true, false) => 1.0,
            (false, true) => -1.0,
            _ => continue,
        };
        let px = if sig > 0.0 { x.o[i].max(buy) } else { x.o[i].min(sell) };
        let why = if sig > 0.0 { "Buy stop 1% above last Top" } else { "Sell stop 1% below last Bottom" };
        if let Some(mut t) = cur.take() { t.exit_i = i; t.exit_px = px; t.signal_i = i; t.reason = format!("{} -> {}", t.reason, why); out.push(t); }
        if i < to {
            cur = Some(Trade { dir: sig, legs: vec![Leg { i, px, w: 1.0 }], exit_i: i, exit_px: px, reason: why.into(), entry_kind: "stop", signal_i: i, open: false, exits: vec![] });
        }
    }
    if let Some(mut t) = cur { t.exit_i = to; t.exit_px = x.c[to]; t.signal_i = to; t.reason = format!("{} -> open", t.reason); t.open = true; out.push(t); }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn swing_needs_three_days_unless_six_percent() {
        // low 100 on day 1; day 2 high +3% (too soon), day 4 high +2.6% -> Bottom set on day 4
        let l = [101., 100., 101., 101., 101., 101.];
        let h = [102., 101., 103., 102., 102.6, 102.];
        let v = swings(&h, &l, 5, 0.025, 3, 0.06);
        assert!(v[3].1.is_nan());
        assert_eq!(v[4].1, 100.0);
        // a 6% day sets it at once
        let h2 = [102., 101., 106.1, 102., 102., 102.];
        assert_eq!(swings(&h2, &l, 5, 0.025, 3, 0.06)[2].1, 100.0);
    }
}
