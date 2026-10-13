//! William Dunnigan, "One-Way Formula for Trading in Stocks and Commodities" (Aug 1957), as reprinted in
//! "New Blueprints for Gains in Stocks and Grains & One-Way Formula" (Traders' Masterclass ed., pp.199-266;
//! original paper pages "Stocks-1..17", "Commodities-1..6"). Page refs, testability table and oracle: library/wdowf/.
//! Stock procedure = weekly bars (built here from the daily rows, Monday-Sunday weeks); entries at the week's
//! closing price (= the close of its last daily bar); Operating-Plan stops are resting orders checked on the
//! daily bars after the entry (gap through a stop fills at the open; stop before target on the same day).
use crate::connors::{Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "wdowf";

const SIGNALS: [&str; 6] = [
    "F|Types of weeks (closing prices ignored): Up-Week = high and low both above the preceding week's high and low; Down-Week = both below; a Long-Range Week is followed by Inside-Weeks inside its range; \"We do not under any circumstances set up Swings Inside a Long-Range Week\" p.206-208",
    "F|\"An UPSWING can be defined as a run of one or more Up-Weeks. The highest price in an Upswing is called a TOP ... The Upswing ends only on the appearance of a Down-Week\"; \"A Downswing should be set up ... if there is a Down-Week with respect to either the preceding week or the high week of the Upswing\" (and the reverse for Upswings); equal highs/lows: use the longest range p.208-210",
    "F|Preliminary Buy: ASC = \"the range of the low week of a Downswing is entirely above the range of the week which is the Maintrend Low\" (or above a later longer-range Downswing low week; Maintrend Low = lowest point since the last Reversal Sell); or P2T = \"the high week of an Upswing closes above the preceding two Tops\" p.211, 218-219",
    "F|\"The Preliminary Bottom is nullified if, before a Reversal Buy Signal comes, a Downswing declines to the point where the high price of its low week is below the low price of the Preliminary Bottom\" p.219",
    "E|Reversal Buy: \"After a Preliminary Buy Signal we wait for a Downswing. If this Downswing does not nullify the Preliminary Bottom, we buy at the closing price of the first Up-week if the closing price is equal to or higher than the closing price of the preceding week. If the closing price is lower, we must then wait for a new Downswing\"; \"we then immediately close out all shorts and go long\" p.214, 220",
    "E|Reversal Sell (mirror): DES (high week of an Upswing entirely below the Maintrend High range, or a later larger-range Top) or P2B (low week of a Downswing closes below the preceding two Bottoms); nullified when the high week of an Upswing is entirely above the Preliminary Top; sell at the close of the first Down-Week after an Upswing if that close is not above the preceding week's close p.214",
];

const PLAN: [&str; 5] = [
    "A|Repeat Buy Patterns on the low week of a Downswing: ASC (\"entire range of the low week of a Downswing is above the range of the last Bottom\"), TOB (\"any part of the range of the low week of a Downswing touches any part of the range of either of the last two Bottoms\") - buy at the close of the first Up-Week (in relation to the low week) if that close is not below the preceding week's close; BR (\"closing price on the low week of a Downswing is above the closing price of the preceding week\") - \"Buy immediately at closing price of BR week\" p.222-223 (Repeat Sells: DES/TOT/TR mirror)",
    "A|\"Repeat Buy Signals ... must occur at progressively higher prices\"; capital in four equal parts: Reversal + first Repeat = Fund L, next two Repeats = Fund S; \"if stopped out of a Fund L position we would enter into another Fund L trade on a Repeat Signal before taking on another Fund S trade\" p.212-214, 225-226",
    "X|Profit Factor PF from the Operating Plan in Stocks table by purchase price (\"If price is midway between two figures, use the higher figure\"); \"the figure for PF represents a rise of 1/2 square root\" p.227 / Stocks-16,17",
    "X|Fund L: limit loss to PF; after 50% PF paper profit limit loss to 50% PF; after 80% PF limit loss to 20% PF; after 100% PF stop to break even; after 120% PF protect 20% PF; after 200% PF protect 40% of maximum profit; closed out by a Reversal Signal p.227-228",
    "X|Fund S: limit loss to PF; after 50% PF limit loss to 50% PF; after 80% PF limit loss to 20% PF; take 100% PF profit p.228",
];

const PLAN_RULES: [&str; 11] = [SIGNALS[0], SIGNALS[1], SIGNALS[2], SIGNALS[3], SIGNALS[4], SIGNALS[5], PLAN[0], PLAN[1], PLAN[2], PLAN[3], PLAN[4]];
const REV_RULES: [&str; 7] = [SIGNALS[0], SIGNALS[1], SIGNALS[2], SIGNALS[3], SIGNALS[4], SIGNALS[5],
    "X|Hold until the opposite Reversal Signal, then reverse (no stops) - the book's Plan #1, which it does NOT recommend"];

const PLAN_NOTE: &str = "Stock procedure on the index (instrument swap): weekly bars built from the daily rows (Mon-Sun weeks; the last, possibly incomplete, week counts). Up to 4 lots (L1, L2, S1, S2) are held at once, each reported as its own trade with weight 1 (per-lot return), so trades OVERLAP and the equity curve is not meaningful - use trade statistics. Lot returns are on equal capital per lot (the book: four equal parts). PF: the printed table (1.5..150) used literally; above 150 the same rule extended with its stated derivation (row prices (PF-0.25)^2, PF steps of 1/2) - an extrapolation, since the book's table stops at $150. Stop/target offsets are exact multiples of PF (the table rounds to $1/8). Stops are checked on daily bars from the day after entry; paper profit uses daily highs (lows for shorts) and a new stop applies from the next day; stop before target on one day; a gap through a level fills at the open. Book silent (necessary assumptions): at the data start the Maintrend is unknown, so Maintrend Low/High are measured from the first bar and the first Reversal Signal in either direction starts trading; a newer Preliminary Signal replaces an older one; equal-range ties are taken at the earliest week. India: same weekly rules on NIFTY (rupee prices into the square-root table).";

const REV_NOTE: &str = "Reversal Signals only, whole capital, stop-and-reverse - the One-Way signal engine without the Operating Plan (Gains in Grains Plan #1, which Dunnigan tested and rejected as an operating plan: \"It is not good practice to buy and then sit back and do nothing until a sell signal appears\", p.224). Included to show the signals alone; NOT the book's recommended plan. Weekly bars, entries at the week's close; one position at a time.";

const COMMODITY_GAP: &str = "Commodity procedure = same rules on daily bars, but the Profit Factor is 30% of the commodity's \"basic margin\" (rough average of broker margin requirements since 1952, Table 2.1: e.g. wheat 16 1/2c -> PF 5c). Needs a dated history of the exchange/broker margin per contract unit for the traded futures (e.g. NSE NIFTY futures SPAN+exposure margin history) and the futures contracts themselves (option rollover rules: no new trade after the 20th of the month before delivery, close the day before the delivery month). The vault has neither.";

pub fn catalogue() -> Vec<StrategyDef> {
    let d = |id, name, dir, status, rules, notes| StrategyDef {
        id, name, group: "One-Way Formula (1957)", session: 0, dir, status, needs_volume: false, needs_vix: false,
        needs_series: &[], book: BOOK, rules, notes, claim: None,
    };
    use Status::*;
    vec![
        d("wdowf_plan", "One-Way Formula · stocks Operating Plan (4 lots, weekly)", "Long/Short", Live, &PLAN_RULES, PLAN_NOTE),
        d("wdowf_reversal", "One-Way Formula · Reversal Signals only (no Operating Plan)", "Long/Short", Live, &REV_RULES, REV_NOTE),
        d("wdowf_commodity", "One-Way Formula · commodities (daily, margin-based PF)", "Long/Short", NeedsData(COMMODITY_GAP), &[
            "F|\"One-Way Formula in commodities is identical to the procedure in stocks except: 1. Daily data is used instead of weekly. 2. The Profit Factor (PF) is based on average margin requirements instead of square roots\" p.235",
            "X|\"The Profit Factor is simply 30% of the basic margin\"; Fund L / Fund S schedules as in stocks p.239-240",
            "F|\"We do not enter into any new trade after the 20th of the month preceding the Delivery Month\"; \"We close out any open trade at the closing price of the day before the Delivery Month\" p.236-237",
        ], COMMODITY_GAP),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    match id {
        "wdowf_plan" => Some(("wdowf", "Dunnigan One-Way Formula", "Stocks Operating Plan (book)")),
        "wdowf_reversal" => Some(("wdowf", "Dunnigan One-Way Formula", "Reversal signals only")),
        "wdowf_commodity" => Some(("wdowf", "Dunnigan One-Way Formula", "Commodities · data missing")),
        _ => None,
    }
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let daily = ParamDef { key: "daily", label: "Bars: 0 = weekly (book, stocks); 1 = daily (book's commodity bars; not the book's stock rule)", def: 0., min: 0., max: 1., step: 1. };
    match id {
        "wdowf_plan" | "wdowf_reversal" => Some(vec![daily]),
        _ => None,
    }
}

/// Printed "Operating Plan in Stocks" price column (Stocks-17); PF = 1.5 + 0.5 * row.
const TABLE_PX: [f64; 23] = [1.5, 3., 5., 7.5, 10.5, 14., 18., 22.5, 27.5, 33., 39., 45.5, 52.5, 60., 68., 76.5, 85.5, 95., 105., 115.5, 126.5, 138., 150.];

/// Profit Factor for a purchase (or sale) price: nearest table price, midway -> higher figure.
/// Above 150 the rows continue at (PF - 0.25)^2 ("a rise of 1/2 square root").
pub fn pf(price: f64) -> f64 {
    let row_px = |k: usize| if k < TABLE_PX.len() { TABLE_PX[k] } else { let p = 1.5 + 0.5 * k as f64 - 0.25; p * p };
    let mut k = 0;
    while price >= (row_px(k) + row_px(k + 1)) / 2.0 { k += 1; }
    1.5 + 0.5 * k as f64
}

struct Bar { s: usize, e: usize, h: f64, l: f64, c: f64 }

fn week_no(date: &str) -> i64 {
    let (y, m, d): (i64, i64, i64) = (date[0..4].parse().unwrap(), date[5..7].parse().unwrap(), date[8..10].parse().unwrap());
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let doy = (153 * (m + if m > 2 { -3 } else { 9 }) + 2) / 5 + d - 1;
    let dn = era * 146_097 + (yoe * 365 + yoe / 4 - yoe / 100 + doy) - 719_468; // days since 1970-01-01 (a Thursday)
    (dn + 3).div_euclid(7) // Monday-based week number
}

fn bars(x: &Ctx, to: usize, daily: bool) -> Vec<Bar> {
    let mut v: Vec<Bar> = vec![];
    for i in 0..=to {
        let same = !daily && v.last().is_some_and(|b| week_no(&x.r[b.e].date) == week_no(&x.r[i].date));
        if same {
            let b = v.last_mut().unwrap();
            b.e = i; b.h = b.h.max(x.h[i]); b.l = b.l.min(x.l[i]); b.c = x.c[i];
        } else {
            v.push(Bar { s: i, e: i, h: x.h[i], l: x.l[i], c: x.c[i] });
        }
    }
    v
}

#[derive(Clone, Copy, PartialEq)]
enum Fund { L, S }

struct Lot { t: Trade, fund: Fund, pf: f64, best: f64, stop: f64 }

impl Lot {
    fn new(dir: f64, i: usize, px: f64, fund: Fund, plan: bool, why: &str) -> Lot {
        let pf = pf(px);
        Lot {
            t: Trade { dir, legs: vec![Leg { i, px, w: 1.0 }], exit_i: i, exit_px: px, reason: why.to_string(), entry_kind: "close", signal_i: i, open: false, exits: vec![] },
            fund, pf, best: 0.0, stop: if plan { px - dir * pf } else { f64::NAN },
        }
    }
    fn entry(&self) -> f64 { self.t.legs[0].px }
    /// One daily bar of the Operating Plan. Returns true if closed.
    fn day(&mut self, x: &Ctx, d: usize) -> bool {
        let (dir, e) = (self.t.dir, self.entry());
        if self.stop.is_finite() {
            let adverse = if dir > 0.0 { x.l[d] } else { x.h[d] };
            if dir * (x.o[d] - self.stop) <= 0.0 { return self.close(d, x.o[d], "stop (gap through, open)"); }
            if dir * (adverse - self.stop) <= 0.0 { return self.close(d, self.stop, "Operating Plan stop"); }
            if self.fund == Fund::S {
                let tgt = e + dir * self.pf;
                if dir * (x.o[d] - tgt) >= 0.0 { return self.close(d, x.o[d], "Fund S 100% PF profit (gap, open)"); }
                let fav = if dir > 0.0 { x.h[d] } else { x.l[d] };
                if dir * (fav - tgt) >= 0.0 { return self.close(d, tgt, "Fund S 100% PF profit"); }
            }
            let fav = if dir > 0.0 { x.h[d] } else { x.l[d] };
            self.best = self.best.max(dir * (fav - e));
            let (b, p) = (self.best, self.pf);
            let mut lock = -p; // stop distance from entry in the favourable direction
            if b >= 0.5 * p { lock = -0.5 * p; }
            if b >= 0.8 * p { lock = -0.2 * p; }
            if b >= p { lock = 0.0; }
            if self.fund == Fund::L {
                if b >= 1.2 * p { lock = 0.2 * p; }
                if b >= 2.0 * p { lock = 0.4 * b; }
            }
            let s = e + dir * lock;
            if dir * (s - self.stop) > 0.0 { self.stop = s; }
        }
        false
    }
    fn close(&mut self, i: usize, px: f64, why: &str) -> bool {
        self.t.exit_i = i; self.t.exit_px = px; self.t.signal_i = i;
        self.t.reason = format!("{} -> {}", self.t.reason, why);
        true
    }
}

/// Swing + signal engine. `plan` = full Operating Plan (4 lots, stops); else Reversal-only stop-and-reverse.
pub fn engine(x: &Ctx, from: usize, to: usize, daily: bool, plan: bool) -> Vec<Trade> {
    let b = bars(x, to, daily);
    let n = b.len();
    let rng = |k: usize| b[k].h - b[k].l;
    let mut out: Vec<Trade> = vec![];
    let mut lots: Vec<Lot> = vec![];
    // swing state
    let (mut state, mut ext) = (0i8, 0usize); // 1 = Upswing (ext = high week), -1 = Downswing (ext = low week)
    let mut mref = 0usize; // comparison week (the Long-Range week while inside it)
    let mut tops: Vec<usize> = vec![];
    let mut bots: Vec<usize> = vec![];
    // maintrend
    let mut mt = 0i8;
    let mut rev_k = 0usize; // first week of the current maintrend measurement
    let mut pre_buy: Option<usize> = None; // Preliminary Bottom week
    let mut pre_sell: Option<usize> = None; // Preliminary Top week
    let mut last_buy = f64::NAN; // last purchase / sale price in the maintrend (progressive rule)
    let mut pend: Option<usize> = None; // pending ASC/TOB (long) or DES/TOT (short) low/high week
    let mut started = false; // first Reversal must come at/after `from`
    for k in 0..n {
        // Operating Plan stops on the daily bars of this week (lots were entered at earlier week closes)
        if plan {
            for d in b[k].s..=b[k].e {
                let mut i = 0;
                while i < lots.len() {
                    if lots[i].day(x, d) { out.push(lots.remove(i).t); } else { i += 1; }
                }
            }
        }
        if k == 0 { continue; }
        let r = mref;
        let inside = b[k].h <= b[r].h && b[k].l >= b[r].l;
        mref = if inside { r } else { k };
        let up_prev = b[k].h > b[r].h && b[k].l > b[r].l;
        let dn_prev = b[k].h < b[r].h && b[k].l < b[r].l;
        let (mut top_ev, mut bot_ev) = (None, None); // (Top week) marked / (Bottom week) marked this week
        let mut new_low = false; // this week became the low week of the current Downswing
        let mut new_high = false;
        if !inside {
            match state {
                1 => {
                    let hw = ext;
                    if dn_prev || (b[k].h < b[hw].h && b[k].l < b[hw].l) {
                        top_ev = Some(hw); tops.push(hw); state = -1; ext = k; new_low = true;
                    } else if b[k].h > b[hw].h || (b[k].h == b[hw].h && rng(k) > rng(hw)) { ext = k; new_high = true; }
                }
                -1 => {
                    let lw = ext;
                    if up_prev || (b[k].h > b[lw].h && b[k].l > b[lw].l) {
                        bot_ev = Some(lw); bots.push(lw); state = 1; ext = k; new_high = true;
                    } else if b[k].l < b[lw].l || (b[k].l == b[lw].l && rng(k) > rng(lw)) { ext = k; new_low = true; }
                }
                _ => {
                    if up_prev { state = 1; ext = k; new_high = true; } else if dn_prev { state = -1; ext = k; new_low = true; }
                }
            }
        }
        let wk_close = b[k].c;
        let prev_close = b[k - 1].c;
        let day = b[k].e;
        // ---- nullification (before any reversal this week)
        if new_low { if let Some(p) = pre_buy { if b[k].h < b[p].l { pre_buy = None; } } }
        if new_high { if let Some(p) = pre_sell { if b[k].l > b[p].h { pre_sell = None; } } }
        // ---- reversal signals
        let mut reversed = 0i8;
        if mt != 1 {
            if let (Some(lw), Some(p)) = (bot_ev, pre_buy) {
                // the Downswing just ended started at a Top after the Preliminary Bottom
                let top_of_swing = tops.last().copied();
                if top_of_swing.is_some_and(|t| t > p) && lw != p && wk_close >= prev_close { reversed = 1; }
            }
        }
        if mt != -1 && reversed == 0 {
            if let (Some(hw), Some(p)) = (top_ev, pre_sell) {
                let bot_of_swing = bots.last().copied();
                if bot_of_swing.is_some_and(|t| t > p) && hw != p && wk_close <= prev_close { reversed = -1; }
            }
        }
        if reversed != 0 && day >= from { started = true; }
        if reversed != 0 {
            mt = reversed; rev_k = k; pre_buy = None; pre_sell = None; pend = None;
            for mut l in lots.drain(..) { l.close(day, wk_close, "Reversal Signal"); out.push(l.t); }
            if started && day < to {
                let why = if reversed > 0 { "Reversal Buy (L1)" } else { "Reversal Sell (L1)" };
                lots.push(Lot::new(reversed as f64, day, wk_close, Fund::L, plan, why));
            }
            last_buy = wk_close;
        }
        // ---- preliminary signals (computed after this week's swing marks)
        // book: preliminaries use the swings that FOLLOW a reversal (p.219), so none on the reversal week
        if mt != 1 && reversed == 0 {
            if let Some(bw) = bot_ev {
                // ASC vs largest-range Bottom from the Maintrend Low on
                let m = (rev_k..k).min_by(|&a, &c| b[a].l.partial_cmp(&b[c].l).unwrap().then(rng(c).partial_cmp(&rng(a)).unwrap())).unwrap_or(k);
                let mut refw = m;
                for &q in bots.iter().filter(|&&q| q > m && q < bw) { if rng(q) > rng(refw) { refw = q; } }
                if bw != m && b[bw].l > b[refw].h { pre_buy = Some(bw); }
            }
            if let Some(tw) = top_ev {
                let nt = tops.len();
                if nt >= 3 && b[tw].c > b[tops[nt - 2]].h && b[tw].c > b[tops[nt - 3]].h {
                    if let Some(&pb) = bots.iter().rev().find(|&&q| q < tw) { pre_buy = Some(pb); }
                }
            }
        }
        if mt != -1 && reversed == 0 {
            if let Some(tw) = top_ev {
                let m = (rev_k..k).min_by(|&a, &c| b[c].h.partial_cmp(&b[a].h).unwrap().then(rng(c).partial_cmp(&rng(a)).unwrap())).unwrap_or(k);
                let mut refw = m;
                for &q in tops.iter().filter(|&&q| q > m && q < tw) { if rng(q) > rng(refw) { refw = q; } }
                if tw != m && b[tw].h < b[refw].l { pre_sell = Some(tw); }
            }
            if let Some(bw) = bot_ev {
                let nb = bots.len();
                if nb >= 3 && b[bw].c < b[bots[nb - 2]].l && b[bw].c < b[bots[nb - 3]].l {
                    if let Some(&pt) = tops.iter().rev().find(|&&q| q < bw) { pre_sell = Some(pt); }
                }
            }
        }
        // ---- repeat signals (Operating Plan only; not on the reversal week)
        if plan && reversed == 0 && mt != 0 && started && day < to {
            let dir = mt as f64;
            let mut sig: Option<&str> = None;
            if mt == 1 {
                if top_ev.is_some() { pend = None; }
                if let Some(lw) = bot_ev {
                    let nb = bots.len();
                    let prev: Vec<usize> = bots[..nb - 1].iter().rev().take(2).copied().collect();
                    let asc = prev.first().is_some_and(|&q| b[lw].l > b[q].h);
                    let tob = prev.iter().any(|&q| b[lw].l <= b[q].h && b[lw].h >= b[q].l);
                    if asc || tob { pend = Some(lw); }
                }
                if new_low && state == -1 && wk_close > prev_close { sig = Some("Repeat Buy BR"); }
                else if let Some(lw) = pend {
                    if k > lw && b[k].h > b[lw].h && b[k].l > b[lw].l {
                        pend = None;
                        if wk_close >= prev_close { sig = Some("Repeat Buy (ASC/TOB, Up-Week)"); }
                    }
                }
            } else {
                if bot_ev.is_some() { pend = None; }
                if let Some(hw) = top_ev {
                    let nt = tops.len();
                    let prev: Vec<usize> = tops[..nt - 1].iter().rev().take(2).copied().collect();
                    let des = prev.first().is_some_and(|&q| b[hw].h < b[q].l);
                    let tot = prev.iter().any(|&q| b[hw].l <= b[q].h && b[hw].h >= b[q].l);
                    if des || tot { pend = Some(hw); }
                }
                if new_high && state == 1 && wk_close < prev_close { sig = Some("Repeat Sell TR"); }
                else if let Some(hw) = pend {
                    if k > hw && b[k].h < b[hw].h && b[k].l < b[hw].l {
                        pend = None;
                        if wk_close <= prev_close { sig = Some("Repeat Sell (DES/TOT, Down-Week)"); }
                    }
                }
            }
            if let Some(why) = sig {
                let progressive = last_buy.is_nan() || dir * (wk_close - last_buy) > 0.0;
                let nl = lots.iter().filter(|l| l.fund == Fund::L).count();
                let ns = lots.len() - nl;
                let fund = if nl < 2 { Some(Fund::L) } else if ns < 2 { Some(Fund::S) } else { None };
                if let (true, Some(f)) = (progressive, fund) {
                    let tag = if f == Fund::L { "Fund L" } else { "Fund S" };
                    lots.push(Lot::new(dir, day, wk_close, f, true, &format!("{why} ({tag})")));
                    last_buy = wk_close;
                }
            }
        }
    }
    for mut l in lots { l.t.exit_i = to; l.t.exit_px = x.c[to]; l.t.signal_i = to; l.t.reason = format!("{} -> open", l.t.reason); l.t.open = true; out.push(l.t); }
    out.retain(|t| t.legs[0].i >= from);
    out.sort_by_key(|t| (t.legs[0].i, t.exit_i));
    out
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !matches!(id, "wdowf_plan" | "wdowf_reversal") || from > to || to >= x.r.len() { return None; }
    let daily = p.get("daily").copied().unwrap_or(0.0) >= 0.5;
    Some(engine(x, from, to, daily, id == "wdowf_plan"))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn profit_factor_matches_the_printed_table_and_examples() {
        assert_eq!(pf(14.0), 4.0); // Stocks-16: "If we buy a stock at $14, the PF is $4"
        assert_eq!(pf(33.0), 6.0); // "another stock at $33, the PF is $6"
        assert_eq!(pf(16.0), 4.5); // footnote: "if purchase price is 16, use 18"
        assert_eq!(pf(64.0), 8.5); // "64, use 68"
        assert_eq!(pf(150.0), 12.5);
        assert_eq!(pf(1.0), 1.5);
        // Coaching Report No.4: Air Reduction L-1 bought 24 1/2, L-2 26 7/8, high 65 3/4 -> stops 41 and 42 3/8
        let (l1, l2): (f64, f64) = (24.5, 26.875);
        assert!((l1 + 0.4 * (65.75 - l1) - 41.0).abs() < 1e-9 && pf(l1) == 5.0);
        assert!((l2 + 0.4 * (65.75 - l2) - 42.425).abs() < 1e-9 && pf(l2) == 5.5);
    }
}
