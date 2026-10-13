//! Lee E. Gettess, "TT30 with Market Mapping" (1994). Book id `lgtt30`. Source copy:
//! Books/Lee E. Gettess/TT30 with market mapping.pdf (62 scanned pages; refs = PDF page numbers). Library: library/lgtt30/.
//!
//! Both Live systems are ports of the printed System Writer Plus code (entry + exit signals) on daily bars with the
//! vault's shared simulator (books::jkbwts::sim): orders are computed at the close for the next bar ("open of tomorrow"
//! is known when the stop is placed at the open); market orders fill at the open; a stop/limit already through the open
//! fills at the open; intrabar path open -> nearer extreme -> other extreme (tie: low first). Buy and sell entry stops
//! are both working every day, so an opposite entry reverses the position (System Writer behaviour). Non-re-entry rule
//! (p.5, p.56): no entry in the direction the position had coming into the day.
use crate::books::jkbwts::{entry, exit, sim, Ty};
use crate::books::lg3k::roundup;
use crate::books::jkbwts::Ord;
use crate::connors::{claim, Ctx, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "lgtt30";

/// A stop/limit already at or through tomorrow's open is sent as a market order (it fills at the open, which the book
/// states for stops: "If the open of the next day is beyond your stop, you would exit at that point", p.55). Done here
/// because the shared jkbwts::sim can rank a later intrabar fill ahead of a fill at the open (see README).
fn at_open(mut q: Ord, o1: f64) -> Ord {
    let buy = if q.entry { q.dir > 0 } else { q.dir < 0 };
    let through = match q.ty { Ty::Stop => if buy { q.px <= o1 } else { q.px >= o1 }, Ty::Limit => if buy { q.px >= o1 } else { q.px <= o1 }, Ty::Mkt => false };
    if through { q.ty = Ty::Mkt; }
    q
}

pub fn catalogue() -> Vec<StrategyDef> {
    use Status::*;
    let d = |id, name, group, status, rules, notes, claim| StrategyDef { id, name, group, session: 0, dir: "Long/Short", status,
        needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK, rules, notes, claim };
    vec![
        d("lgtt30_tt30", "TT30 (Kris-Kross volatility breakout, profitable-open exit)", "TT30 (p.2-21)", Live,
          &["F|Kris-Kross: larger of (highest high - lowest close) and (highest close - lowest low) of the last 3 days; x 0.5, rounded UP to the price increment (p.3, p.6; code Value2/Value3/Value4)",
            "E|Buy tomorrow at open of tomorrow + Value4/2 stop; Sell tomorrow at open of tomorrow - Value4/2 stop (code p.9)",
            "X|\"If open of tomorrow > @entryprice then exit long tomorrow at the market\" (Profitable open, p.10); short mirror",
            "X|\"If open of tomorrow > h(0) then exit long tomorrow at the market\" (Goodgap, p.10; text p.4 'exit on any open that gaps above the previous day's high'); short mirror",
            "X|Place profit target at high of tomorrow (= high of the entry day); place protective stop at l(0) (= low of the day before entry) (code p.9; framework step 7, p.6); short mirror",
            "X|Reversal: a new Kris-Kross sell (buy) stop from each open reverses the position (p.5, framework step 8)",
            "F|Non-re-entry: \"we will not go long again if we came into a day long, and we will not go short again if we came into a day short\" (p.5)"],
          "Book (System Writer Plus, $50 commission, 1 contract): S&P INDEX 4/29/82-6/30/93 (CSI continuous): 1,059 trades, 72% profitable, avg trade $162.09, net $171,650 (p.19); 11 other futures printed (p.11-22). Code vs framework: framework step 5 says 'Do not reverse' on the entry day, the printed code (which produced the results) reverses through the opposite entry stop; default = code. Target/protective stop start the day after entry (framework step 7). Run on SPX/SPY/NIFTY (instrument swap), tick 0.05.",
          claim("1,059", "72%", "$162.09", "1 bar", "S&P 500 futures 4/1982-6/1993 (p.19)")),
        d("lgtt30_sp_plan", "S&P Plan (Daily Market Mapping: day type x close/open pattern)", "Daily Market Mapping (p.37-58)", Live,
          &["F|Day types: Inside (high <= previous high and low >= previous low), Outside (higher high and lower low), Higher (higher high, low >= previous low), Lower (lower low, high <= previous high) (p.38)",
            "F|Close vs previous day: '+' up but <= previous high, '+G' above previous high, '-' down but >= previous low, '-G' below previous low; equal closes ignored. Open of today vs yesterday's close/range the same way (p.50)",
            "E|Look up the S&P500 column of the table for yesterday's day type and the close/open pattern: buy stop = open + Buy% of yesterday's range, sell stop = open - Sell% ; NT = no trade, Open = at the open (tables p.51-54, = S&PPlan Hi/In/Lo/Out code p.45-48)",
            "X|Profitable open: exit at the open if it is profitable basis the entry (p.49, p.55)",
            "X|Profit target: Maxlist(high of the entry day, entry + 20 points) for longs; Minlist(low of the entry day, entry - 20 points) for shorts (code; p.55 'minimum target of ... 20 points for the S&P')",
            "X|Money-management stop $1,250 (S&P) from the day after entry; an open beyond the stop exits at the open (p.55: 'not place stops on day of entry ... That is the strategy that the SystemWriter track record employs')",
            "F|Reversal signals act; no entry in the direction you came into the day (p.56)"],
          "Book (System Writer, 'S&P Plan 93', S&P INDEX roll, 1/1/93-12/31/93): 48 trades, 81% profitable, avg $162.50, net $7,800 (p.58). 20 S&P points = 0.20 index points ($500/point => $1,250 = 2.50 points). Run on SPY/SPX/NIFTY (instrument swap): the 20-point target and the $1,250 stop are kept in index points (params). The book: 'if you are in any doubt at all as to whether a market opened higher or lower ... DON'T TAKE ANY TRADE' - with vault data an open equal to the prior close is 'no pattern' (no trade).",
          claim("48", "81%", "$162.50", "1 bar", "S&P 500 futures 1993 (p.58)")),
        d("lgtt30_bond_plan", "T-Bond Plan (Daily Market Mapping)", "Daily Market Mapping (p.37-58)",
          NeedsData("T-Bond futures day-session data (the vault has no bond futures)"),
          &["E|T-Bonds columns of the same day-type tables (p.51-54); BndPlan Hi/In/Lo/Out code (p.41-44)", "X|Profitable open; target max(high of entry day, entry + 2 ticks); $625 money stop from the day after entry"],
          "Book 'Bond Plan 93' 1993: 57 trades, 75%, avg $92 (p.57).", claim("57", "75%", "$92.00", "1 bar", "T-Bond futures 1993 (p.57)")),
        d("lgtt30_intraday", "Intraday Market Mapping (1-2-3 patterns, 3-bar filter)", "Intraday Market Mapping (p.23-36)",
          NotTestable("discretionary 1-2-3 swing recognition on 15-30 minute charts (swings 'of some degree'); also needs intraday bars"),
          &["F|1-2-3 swing patterns and a 3-bar average filter used to fine-tune system entries (p.23-36)"], "", None),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    catalogue().into_iter().find(|d| d.id == id).map(|d| (d.id, d.name, ""))
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    Some(match id {
        "lgtt30_tt30" => vec![p("days", "Kris-Kross days", 3., 2., 10., 1.), p("mult", "Multiplier of the Kris-Kross figure", 0.5, 0.1, 2.0, 0.05),
            p("tick", "Rounding increment (0.05 = S&P / NSE)", 0.05, 0.0, 1.0, 0.01),
            p("stop_ref", "Protective stop: 0 = low of the day before entry (code l(0), framework 7), 1 = low of the entry day (text p.4)", 0., 0., 1., 1.)],
        "lgtt30_sp_plan" => vec![p("target_min", "Minimum profit target in index points (book: 20 S&P points = 0.20)", 0.20, 0.0, 5.0, 0.05),
            p("stop_pts", "Money-management stop in index points (book: $1,250 / $500 = 2.50)", 2.5, 0.0, 20.0, 0.25)],
        _ => return None,
    })
}

/// S&P500 column of the Market Mapping tables: (buy %, sell %), None = NT, Some(0) = Open.
/// day: 0 Higher, 1 Lower, 2 Outside, 3 Inside; close/open codes: 0 '+', 1 '+G', 2 '-', 3 '-G'.
fn sp_table(day: usize, cl: usize, op: usize) -> (Option<f64>, Option<f64>) {
    const NT: i32 = -1;
    // rows indexed [close][open] with close order (+, -, +G, -G) as printed; we map codes below
    let hi: [[(i32, i32); 4]; 4] = [ // close '+': open +, +G, -, -G
        [(130, 80), (80, 60), (60, 130), (40, 50)],
        [(NT, NT), (NT, NT), (NT, NT), (NT, NT)], // close '+G' handled below
        [(90, 200), (NT, NT), (150, NT), (20, 80)], // close '-'
        [(NT, NT), (NT, NT), (NT, NT), (NT, NT)],   // close '-G' impossible on a higher day
    ];
    let hi_g = [(110, 200), (NT, NT), (120, NT), (60, 110)];
    let lo: [[(i32, i32); 4]; 4] = [
        [(NT, NT), (40, 170), (140, NT), (0, NT)],
        [(NT, NT); 4],
        [(120, NT), (70, 70), (NT, 70), (160, 70)],
        [(150, 140), (80, 60), (NT, NT), (190, 100)],
    ];
    let out: [[(i32, i32); 4]; 4] = [
        [(NT, 20), (NT, NT), (NT, NT), (NT, NT)],
        [(NT, 50), (20, NT), (70, 70), (NT, NT)],
        [(50, NT), (NT, NT), (50, 70), (NT, 10)],
        [(30, NT), (NT, NT), (0, NT), (50, NT)],
    ];
    let ins: [[(i32, i32); 4]; 4] = [
        [(70, NT), (70, 190), (80, NT), (NT, 90)],
        [(NT, NT); 4],
        [(140, 130), (30, NT), (NT, 140), (130, NT)],
        [(NT, NT); 4],
    ];
    let v = match day {
        0 => if cl == 1 { hi_g[op] } else { hi[cl][op] },
        1 => lo[cl][op],
        2 => out[cl][op],
        _ => ins[cl][op],
    };
    let f = |q: i32| (q >= 0).then_some(q as f64);
    (f(v.0), f(v.1))
}
// code index: 0 '+', 1 '+G', 2 '-', 3 '-G'  (tables above use the same index)

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    let schema = params(id)?;
    if from > to || to >= x.c.len() { return None; }
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).map_or(f64::NAN, |d| d.def));
    let (o, h, l, c) = (&x.o, &x.h, &x.l, &x.c);
    Some(match id {
        "lgtt30_tt30" => {
            let n = g("days").max(1.0).round() as usize;
            let (mult, tick, sref) = (g("mult"), g("tick"), g("stop_ref") >= 0.5);
            sim(x, from, to, 0.0, false, |i, pos, _| {
                if i + 1 < n { return vec![]; }
                let w = i + 1 - n..=i;
                let hh = w.clone().map(|j| h[j]).fold(f64::MIN, f64::max); let ll = w.clone().map(|j| l[j]).fold(f64::MAX, f64::min);
                let hc = w.clone().map(|j| c[j]).fold(f64::MIN, f64::max); let lc = w.clone().map(|j| c[j]).fold(f64::MAX, f64::min);
                let k = roundup((hh - lc).max(hc - ll) * mult, tick);
                let o1 = o[i + 1];
                let mut v = vec![];
                match pos {
                    None => { v.push(entry(1, Ty::Stop, o1 + k, "TT30 buy stop")); v.push(entry(-1, Ty::Stop, o1 - k, "TT30 sell stop")); }
                    Some(q) if q.dir > 0 => {
                        if o1 > q.px { v.push(exit(1, Ty::Mkt, 0.0, "Profitable open")); }
                        else if o1 > h[i] { v.push(exit(1, Ty::Mkt, 0.0, "Goodgap (open above yesterday's high)")); }
                        v.push(exit(1, Ty::Limit, h[q.i], "Target: high of entry day"));
                        let sp = if sref { l[q.i] } else { l[q.i.saturating_sub(1)] };
                        v.push(exit(1, Ty::Stop, sp, "Protective stop"));
                        v.push(entry(-1, Ty::Stop, o1 - k, "TT30 reversal sell stop"));
                    }
                    Some(q) => {
                        if o1 < q.px { v.push(exit(-1, Ty::Mkt, 0.0, "Profitable open")); }
                        else if o1 < l[i] { v.push(exit(-1, Ty::Mkt, 0.0, "Goodgap (open below yesterday's low)")); }
                        v.push(exit(-1, Ty::Limit, l[q.i], "Target: low of entry day"));
                        let sp = if sref { h[q.i] } else { h[q.i.saturating_sub(1)] };
                        v.push(exit(-1, Ty::Stop, sp, "Protective stop"));
                        v.push(entry(1, Ty::Stop, o1 + k, "TT30 reversal buy stop"));
                    }
                }
                v.into_iter().map(|q| at_open(q, o1)).collect()
            })
        }
        "lgtt30_sp_plan" => {
            let (tmin, stp) = (g("target_min"), g("stop_pts"));
            sim(x, from, to, 0.0, false, |i, pos, _| {
                if i < 1 { return vec![]; }
                let o1 = o[i + 1];
                let day = if h[i] > h[i - 1] && l[i] >= l[i - 1] { 0 } else if l[i] < l[i - 1] && h[i] <= h[i - 1] { 1 } else if h[i] > h[i - 1] && l[i] < l[i - 1] { 2 } else { 3 };
                let cl = if c[i] > h[i - 1] { Some(1) } else if c[i] > c[i - 1] { Some(0) } else if c[i] < l[i - 1] { Some(3) } else if c[i] < c[i - 1] { Some(2) } else { None };
                let op = if o1 > h[i] { Some(1) } else if o1 > c[i] { Some(0) } else if o1 < l[i] { Some(3) } else if o1 < c[i] { Some(2) } else { None };
                let (bp, sp) = match (cl, op) { (Some(a), Some(b)) => sp_table(day, a, b), _ => (None, None) };
                let r = h[i] - l[i];
                let mut v = vec![];
                let dir = pos.map_or(0, |q| q.dir);
                if let Some(q) = pos {
                    let d = q.dir as f64;
                    if (o1 - q.px) * d > 0.0 { v.push(exit(q.dir, Ty::Mkt, 0.0, "Profitable open")); }
                    let tgt = if q.dir > 0 { h[q.i].max(q.px + tmin) } else { l[q.i].min(q.px - tmin) };
                    v.push(exit(q.dir, Ty::Limit, tgt, "Profit target"));
                    if stp > 0.0 { v.push(exit(q.dir, Ty::Stop, q.px - d * stp, "Money management stop")); }
                }
                if dir <= 0 { if let Some(b) = bp { v.push(entry(1, Ty::Stop, o1 + r * b / 100.0, "S&P Plan buy")); } }
                if dir >= 0 { if let Some(s) = sp { v.push(entry(-1, Ty::Stop, o1 - r * s / 100.0, "S&P Plan sell")); } }
                v.into_iter().map(|q| at_open(q, o1)).collect()
            })
        }
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn table_spot_checks() {
        // Lower day, close '-' open '+' -> buy 120 (= S&PPlan Lo code: c<c(1), c>=l(1), open>c, open<=h -> 120/100)
        assert_eq!(sp_table(1, 2, 0), (Some(120.0), None));
        // Lower day, close '+' open '-G' -> buy at the open (code 00/100)
        assert_eq!(sp_table(1, 0, 3), (Some(0.0), None));
        // Higher day '+G' '+' -> 110 / 200 ; Inside '-' '+' -> 140 / 130 ; Outside '+' '+' -> NT / 20
        assert_eq!(sp_table(0, 1, 0), (Some(110.0), Some(200.0)));
        assert_eq!(sp_table(3, 2, 0), (Some(140.0), Some(130.0)));
        assert_eq!(sp_table(2, 0, 0), (None, Some(20.0)));
    }
}
