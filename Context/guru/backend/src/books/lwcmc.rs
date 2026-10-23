//! Larry Williams, "Cracking the Money Code" (course manual, Commodity Timing 2006) with its companion booklets
//! "Market Mastery: The Best Kept Secrets of Experienced Traders" (2006), "The Master Cycle of Stock Market
//! Activity" (2006) and the "Target Shooter" overlay. Book id `lwcmc`.
//! Source copies: Books/Larry Williams/Larry Williams - Cracking the Money Code - *.pdf (image scans).
//! Page refs are the manual's chapter.page stamps (e.g. 7.4). README: library/lwcmc/README.md.
//!
//! Only one rule in these texts has both a mechanical entry and a mechanical exit and is not already in the vault:
//! the Turtle short-term system exactly as the manual states it (p.7.4-7.5): buy at the highest high of the last 20
//! days, exit at the lowest low of the last 10 days, and the mirror for shorts. No stop, no adds, no filter
//! ("we'll take all the signals", p.7.5). Own small simulator below (not jkbwts::sim).
use crate::connors::{claim, Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "lwcmc";

/// Larry Williams block: rules admitted after the 23 Oct 2026 reading and independent ledgers
/// (authors/lwilliams/admission/SOURCE-CONTRACT.md). SPX_POINTS = stops in S&P points: actual SPX only.
pub const REVIEWED: &[&str] = &["lwcmc_turtle_20_10"];
pub const SPX_POINTS: &[&str] = &[];

macro_rules! off {
    ($id:expr, $name:expr, $group:expr, $dir:expr, $status:expr, $rules:expr) => {
        StrategyDef { id: $id, name: $name, group: $group, session: 0, dir: $dir, status: $status, needs_volume: false,
            needs_vix: false, needs_series: &[], book: BOOK, rules: $rules, notes: "", claim: None }
    };
}

pub fn catalogue() -> Vec<StrategyDef> {
    use Status::{NeedsData, NotTestable};
    vec![
        StrategyDef { id: "lwcmc_turtle_20_10", name: "Turtle short-term system as stated: 20-day breakout, 10-day exit",
            group: "Ch 7 · Trend identification", session: 0, dir: "Long/Short", status: Status::Live, needs_volume: false, needs_vix: false,
            needs_series: &[], book: BOOK,
            rules: &["E|\"BUY LONG RULE: Buy at the Highest High of the Last 20 Days\" (p.7.4): buy stop at the highest high of the 20 bars before today",
                     "X|\"EXIT LONG RULE: Exit at the Lowest Low of the Last 10 Days\" (p.7.4): sell stop at the lowest low of the 10 bars before today",
                     "E|\"SELL SHORT RULE: Go Short at the Lowest Low of the Last 20 Days\" (p.7.5)",
                     "X|\"EXIT SHORT RULE: Exit at the Highest High of the Last 10 Days\" (p.7.5)",
                     "F|\"That's all there is to it ... we'll take all the signals\" (p.7.5): no stop, no filter, one unit"],
            notes: "Book: S&P 500 futures 'since the inception of trading in 1982' (end of window not printed; the commodity tables run to 2001) and 18 commodities. India: same rule on NIFTY 50 (instrument swap). Not cov_turtle_s1: that one is Covel's full Turtle rules (2N stop, 1/2N adds, last-breakout filter, 55-day failsafe). [Book silent] stops fill at their price or at the open on a gap; TradeStation bar path; the exit stop is live on the entry bar; when one bar triggers the exit and the opposite 20-day entry, both fill (exit first).",
            claim: claim("226", "33%", "net -$142,670", "", "S&P 1982-? (p.7.9)") },
        off!("lwcmc_turtle_commodities", "Turtle 20/10 on 18 commodities (tables p.7.7, 7.9)", "Ch 7 · Trend identification", "Long/Short",
            NeedsData("Needs daily futures for silver, gold, Swiss franc, pound, yen, cattle, bellies, copper, cocoa, sugar, soybeans, bean oil, wheat, corn, cotton, lumber, coffee, T-bonds."),
            &["E|Same 20/10 rules as lwcmc_turtle_20_10 (p.7.4-7.5)"]),
        off!("lwcmc_setup_channel_vstop", "Set-up market + 14-day channel or volatility-stop entry (Mock Turtle)", "Ch 7 · Trend identification", "Long/Short",
            NeedsData("The entry is allowed only 'if the market has a bullish/bearish set up' (p.7.12): commercials >= 75% (<= 20%) on the COT index, advisors <= 25% (>= 75%), premium, accumulation, seasonal (p.10.7); needs CFTC COT and advisory sentiment, and the set-up is a judged cluster."),
            &["F|\"If the Market has a Bullish Set up\" (p.7.12)", "E|\"Buy at the Highest High of the Last 14 Days\" (p.7.12); or the first daily close above the volatility stop: 7-day average true range x 2 added to the lowest close (p.7.16, 7.20)",
              "X|14-day low trailing stop (p.7.12) or a close below the volatility stop or a price target (p.7.20)"]),
        off!("lwcmc_sp_patterns", "Special bonus: 9 S&P sell patterns and 4 buy patterns (Genesis printouts)", "Ch 12 · Short-term trading patterns", "Long/Short",
            NotTestable("Each pattern gives only the entry order (e.g. 'THEN BUY at Next Bar Open + Range * 0.20 Stop'); the manual never states the exit or stop used for the printouts (p.12.4-12.11). 11 of the 13 also need T-bond futures (Bonds < Bonds.4 etc.)."),
            &["E|e.g. Buy #1: \"IF Outside Bar And Close < Close.1 And Close > Close.2 Or Close > Close.12 Or Close > Close.19 THEN BUY at Next Bar Open + Range * 0.20 Stop\" (p.12.9): 128 trades, 91.4%",
              "E|Sell #3: \"IF DayOfWeek = 4 And Close > Close.1 And Down Range And Close > Close.6 Or Close > Close.11 Or Close > Close.17 THEN SELL at Low.1 Stop\" (p.12.5): 18 trades, 88.9%",
              "X|Not stated. Combined 800 trades 82.5% $668.56, 05/06/1982-07/29/2002 (p.12.11)"]),
        off!("lwcmc_tools_exits", "COT index, sentiment, premiums, OI play, seasonals, A/D (open-to-close), %R, Oops!, stops, Target Shooter, money management", "Ch 1-11 · Tools", "Long/Short",
            NotTestable("Set-up tools and exits without a complete system. Duplicates: %R with the 50-day trend = lwic6_pctr_ma50_long; Oops! = lwlts_oops_*; 'buy in October, batten down in May' and the decennial / 5-and-8 years (Master Cycle booklet) = lwrsr_oct_apr / lwrsr_decennial; COT index = lwcot. Exits ($ stop, 90% of the 3-day average range, 12/14/17-day trailing stops, swing-point stop, Target Shooter) are given without an entry rule."),
            &["F|Commercial index = (current - 3-year low) / (3-year high - 3-year low); bullish > 75%, bearish < 25% (Best Kept Secrets p.7-8)",
              "X|\"a stop that is equal to about 90 percent of the average range of the last three days\" (p.9.1); 12-day trailing stop (p.9.3, 9.11)"]),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    let f = |a, b, c| Some((a, b, c));
    match id {
        "lwcmc_turtle_20_10" => f("lwcmc_turtle", "Turtle as stated by Williams", "S&P / index"),
        "lwcmc_turtle_commodities" => f("lwcmc_turtle", "Turtle as stated by Williams", "Commodities (needs data)"),
        "lwcmc_setup_channel_vstop" | "lwcmc_tools_exits" => f("lwcmc_setups", "Set-ups and tools", ""),
        "lwcmc_sp_patterns" => f("lwcmc_patterns", "S&P short-term patterns", ""),
        _ => None,
    }
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    match id {
        "lwcmc_turtle_20_10" => Some(vec![p("entry", "Entry: highest high / lowest low of the last N days", 20.0, 5.0, 100.0, 1.0),
                                          p("exit", "Exit: lowest low / highest high of the last N days", 10.0, 2.0, 50.0, 1.0)]),
        _ => None,
    }
}

struct Pos { dir: f64, i: usize, px: f64 }

fn done(p: &Pos, i: usize, px: f64, why: &str, open: bool) -> Trade {
    Trade { dir: p.dir, legs: vec![Leg { i: p.i, px: p.px, w: 1.0 }], exit_i: i, exit_px: px, reason: why.into(),
        entry_kind: "stop", signal_i: i, open, exits: vec![] }
}

/// Channel breakout: entry stops at the N-day extreme, exit stops at the M-day opposite extreme (levels from the
/// bars before today). Path: open, then O->H->L->C if the open is nearer the high, else O->L->H->C.
fn channel(x: &Ctx, from: usize, to: usize, ne: usize, nx: usize) -> Vec<Trade> {
    let (o, h, l, c) = (&x.o, &x.h, &x.l, &x.c);
    let mut out = vec![];
    let mut pos: Option<Pos> = None;
    let n = ne.max(nx);
    for i in from.max(n)..=to {
        let hi = |k: usize| h[i - k..i].iter().copied().fold(f64::NEG_INFINITY, f64::max);
        let lo = |k: usize| l[i - k..i].iter().copied().fold(f64::INFINITY, f64::min);
        let (be, se, lx, sx) = (hi(ne), lo(ne), lo(nx), hi(nx));
        let mut used = [false, false];
        // at the open
        if let Some(p) = &pos {
            if (p.dir > 0.0 && o[i] <= lx) || (p.dir < 0.0 && o[i] >= sx) { out.push(done(p, i, o[i], "channel exit (gap at the open)", false)); pos = None; }
        }
        if pos.is_none() {
            if o[i] >= be { pos = Some(Pos { dir: 1.0, i, px: o[i] }); used[0] = true; }
            else if o[i] <= se { pos = Some(Pos { dir: -1.0, i, px: o[i] }); used[1] = true; }
        }
        let path = if h[i] - o[i] <= o[i] - l[i] { [o[i], h[i], l[i], c[i]] } else { [o[i], l[i], h[i], c[i]] };
        for s in 0..3 {
            let (mut cur, end) = (path[s], path[s + 1]);
            let up = end > cur;
            loop {
                let hit = |lv: f64| if up { lv > cur && lv <= end } else { lv < cur && lv >= end };
                // the nearest pending order in the direction of travel
                let mut best: Option<(f64, u8)> = None; // 0 exit, 1 long entry, 2 short entry
                let mut take = |lv: f64, a: u8| if hit(lv) && best.map_or(true, |(b, _)| if up { lv < b } else { lv > b }) { best = Some((lv, a)); };
                match &pos {
                    Some(p) if p.dir > 0.0 && !up => take(lx, 0),
                    Some(p) if p.dir < 0.0 && up => take(sx, 0),
                    None if up && !used[0] => take(be, 1),
                    None if !up && !used[1] => take(se, 2),
                    _ => {}
                }
                let Some((lv, a)) = best else { break };
                match a {
                    0 => { let p = pos.take().unwrap(); out.push(done(&p, i, lv, "channel exit", false)); }
                    1 => { used[0] = true; pos = Some(Pos { dir: 1.0, i, px: lv }); }
                    _ => { used[1] = true; pos = Some(Pos { dir: -1.0, i, px: lv }); }
                }
                cur = lv;
            }
        }
    }
    if let Some(p) = pos { out.push(done(&p, to, c[to], "open", true)); }
    out
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    let schema = params(id)?;
    if from > to || to >= x.r.len() { return None; }
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).map_or(f64::NAN, |d| d.def));
    let (ne, nx) = (g("entry").max(1.0) as usize, g("exit").max(1.0) as usize);
    Some(channel(x, from, to, ne, nx))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;
    fn bar(d: &str, o: f64, h: f64, l: f64, c: f64) -> Row {
        Row { ticker: "T".into(), date: d.into(), open: o, high: h, low: l, close: c, volume: None }
    }
    #[test]
    fn breakout_then_channel_exit() {
        // entry 2 / exit 1: bar 2 breaks the 2-day high 101 -> long at 101; bar 3 falls through bar 2's low 100.5 -> exit.
        let r = vec![bar("2020-01-01", 100., 101., 99., 100.), bar("2020-01-02", 100., 100.8, 99.5, 100.),
            bar("2020-01-03", 100.6, 102., 100.5, 101.8), bar("2020-01-06", 101.5, 101.6, 100., 100.2)];
        let x = Ctx::new(&r, &r, &[]);
        let p = Params::from([("entry".to_string(), 2.0), ("exit".to_string(), 1.0)]);
        let t = run("lwcmc_turtle_20_10", &x, 0, 3, &p).unwrap();
        assert_eq!((t[0].dir, t[0].legs[0].i, t[0].legs[0].px, t[0].exit_i, t[0].exit_px), (1.0, 2, 101.0, 3, 100.5));
    }
}
