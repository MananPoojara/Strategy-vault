//! Jake Bernstein, "30 Days to Market Mastery: A Step-by-Step Guide to Profitable Trading" (Wiley Trading, 2007).
//! Book id `jb30d`. Source copy: Books/Jake Bernstein/30 Days to Market Mastery (Wiley 2007).pdf
//! (printed page = PDF page - 15 in the lessons). Transcription and checks: library/jb30d/.
//! The MAC (lessons 10-11), 2-day breakaway gap (lesson 13), PMF (6-9) and COT (16-17) repeat the 6-2-4
//! report word for word and are not re-coded here (see jb624_*). Helpers come from `jb624`.
use super::jb624::{bar_path, finish, getp, limit_fill, mac, mark_open, new_trade, stop_fill};
use crate::connors::{sma, Ctx, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "jb30d";

macro_rules! def {
    ($id:expr, $name:expr, $group:expr, $dir:expr, $status:expr, $rules:expr, $notes:expr) => {
        StrategyDef { id: $id, name: $name, group: $group, session: 0, dir: $dir, status: $status, needs_volume: false,
            needs_vix: false, needs_series: &[], book: BOOK, rules: $rules, notes: $notes, claim: None }
    };
}

const REV_BUY: &str = "E|\"low of day 2 is lower than low of day 1; close of day 2 is higher than close of day 1 and close of day 3 is lower than the low of day 2, then buy on the close of day 3\" (Lesson 2, p.13)";
const REV_NOTE: &str = "Book: S&P 500 futures, Genesis SP-067 back-adjusted continuous contract, 1982-2004, dollars at $250 per point. Stops are kept in S&P points on every index (instrument swap only). SPX opens before about 1999 often equal the prior close, which changes first-profitable-opening exits there.";

/// Daily rules admitted after the 2026-10-22 full reading (authors/bernstein/agents/jb30d) and an independent
/// chronological Python ledger check (authors/bernstein/g2-admission). Held: the gap trades (same-day stop order
/// unknown on daily bars), the sell reversal (no stop: a short stays open for decades), the simple reversal, the triggered seasonal, the 3-unit gap, the 8-bar target exits,
/// the commodity seasonals and the 30-minute breakout.
pub const REVIEWED_DAILY: &[&str] = &["jb30d_rev_buy_fpo", "jb30d_rev_buy_stop", "jb30d_rev_buy_3pc", "jb30d_sp_seasonal", "jb30d_cmc", "jb30d_oc8"];
pub fn reviewed_daily(id: &str) -> bool { REVIEWED_DAILY.contains(&id) }
/// Stops in S&P points (book: dollars on one S&P contract at $250 a point, p.16 and p.44): actual SPX only.
pub const SPX_POINTS: &[&str] = &["jb30d_rev_buy_stop", "jb30d_rev_buy_3pc"];
/// Every admitted rule except the close-only seasonal reads the open (FPO exits, stop gaps, channel-target fills, MA of opens).
pub fn reads_open(id: &str) -> bool { reviewed_daily(id) && id != "jb30d_sp_seasonal" }

pub fn catalogue() -> Vec<StrategyDef> {
    use Status::{Live, NeedsData, NotTestable};
    use crate::connors::claim;
    vec![
        StrategyDef { claim: claim("101", "91.1%", "$787", "5.13 bars", "S&P futures SP-067, PF 2.50"),
          ..def!("jb30d_rev_buy_fpo", "Reversal-up with trigger · first profitable opening", "Lesson 2 · Setup, Trigger, Follow-through", "Long", Live,
            &[REV_BUY, "X|\"and take profit on first profitable opening\" (p.13)", "X|No stop is stated for Table 2.1"],
            "Table 2.1 (p.14) lists 9 losing trades (largest -$7,400), which a first-profitable-opening exit without a stop cannot produce, so the book's test had an exit it does not state. Coded as printed; see jb30d_rev_buy_stop for the stated stop.") },
        StrategyDef { claim: claim("115", "87.8%", "$3", "9.41 bars", "S&P futures SP-067, PF 1.00"),
          ..def!("jb30d_rev_sell_fpo", "Reversal-down with trigger · first profitable opening", "Lesson 2 · Setup, Trigger, Follow-through", "Short", Live,
            &["E|\"high of day 2 is higher than high of day 1; close of day 2 is lower than close of day 1 and close of day 3 is higher than high of day 2, then sell on close of day 3\" (p.13-14)",
              "X|\"and take profit on first profitable opening\" (p.14)", "X|No stop is stated for Table 2.2"],
            "Table 2.2 (p.15) also has losers (14, largest -$8,950): an unstated exit, as for the buy side. Author: \"the sell pattern ... fails to make money ... we avoid it\".") },
        StrategyDef { claim: claim("105", "74.3%", "$363", "3.77 bars", "S&P futures SP-067, PF 1.55"),
          ..def!("jb30d_rev_buy_stop", "Reversal-up with trigger · $2,500 stop, first profitable opening", "Lesson 2 · Adding a Follow-through", "Long", Live,
            &[REV_BUY, "X|\"Using a stop-loss of $2,500 with exit on the first profitable opening\" (p.16): 10.0 S&P points at $250 per point, a resting stop from the day after entry"],
            "Author: \"these examples are not recommended as trading strategies. They are merely included as illustrations\" (p.16). A gap through the stop fills at the open.") },
        StrategyDef { claim: claim("102", "85.3%", "$923", "5.22 bars", "S&P futures SP-067, PF 2.27"),
          ..def!("jb30d_rev_buy_3pc", "Reversal-up with trigger · $4,900 stop, exit on the 3rd profitable close", "Lesson 2 · Adding a Follow-through", "Long", Live,
            &[REV_BUY, "X|\"change the exit method and the stop-loss to exit on the third profitable CLOSE with a $4900 stop-loss\" (p.16): 19.6 points; exit on the close of the third close above the entry price"],
            REV_NOTE) },
        def!("jb30d_rev_simple", "Simple daily reversal up / down", "Lesson 2 · Common Setups", "Long/Short", NotTestable("the exit used for Figures 2.3-2.4 is not stated (\"How often ... could a profit have been achieved the next day\")"),
            &["E|\"Daily reversal up. The market drops below the previous daily low and closes above the previous daily close\"; down mirrored (p.10)"],
            "Printed (p.11-12): up 589 trades 75.9% $67; down 550 trades 70.0% $28, S&P futures 1982-2004."),
        StrategyDef { claim: claim("23", "95.65%", "14.9068 pts avg profit", "10/27-11/1", "Dec S&P futures 1982-2004"),
          ..def!("jb30d_sp_seasonal", "S&P key-date seasonal: long Oct 27 to Nov 1", "Lessons 3-5 · Seasonality", "Long", Live,
            &["E|\"Long Dec S&P  Enter: 10/27  Exit: 11/1  Stop %: 1.50\" (Table 5.1, p.46): buy on the close of the entry date",
              "E|\"If a market is closed on the ideal day of entry or exit, then the trade is executed on the close of trading the next business day\" (p.47)",
              "X|Exit on the close of the exit date (\"On the exit date for the trade, it is closed out at the end of the day using a market-on-close (MOC) order\", p.43)",
              "X|\"The stops are close only\" (p.43): exit on the close if it is 1.50% or more below the entry price"],
            "This is the seasonal setup history exactly as Table 5.1 lists it (no stochastic trigger, no break-even move): the table is the setup's own record. Dec S&P futures replaced by the index (same underlying). The triggered version is listed separately.") },
        def!("jb30d_sp_seasonal_trig", "S&P key-date seasonal with stochastic trigger and follow-through", "Lessons 3-5 · Seasonality", "Long", NotTestable("how long a stochastic \"has been below 25 percent and has gone back above it\" stays valid is not defined, and the 15% time allowance is given two different ways"),
            &["E|\"A buy trigger occurs when SI %K has been below 25 percent and has gone back above it\" (14-period slow %K, 5-bar smoothing) (p.31-32)",
              "E|\"If it does not trigger on the entry date, or on the close of trading the next business day, then allow the trade 15 percent more time\" (p.33)",
              "X|\"Once a seasonal trade has achieved its average profit per trade ... the stop is moved to a break-even stop\" (p.43); 75% trailing stop after the exit date (p.44)"],
            "Example (p.44-45): 2004 long on the close 10/27/04 at 1128.80, exit 11/1/04 at 1130.80, +6.00 points."),
        def!("jb30d_seasonal_futures", "Commodity key-date seasonals (hogs, heating oil, soybeans, Canadian dollar)", "Lessons 3-5 · Seasonality", "Long/Short", NeedsData("individual futures contract histories (July lean hogs, September heating oil, September soybeans, September Canadian dollar)"),
            &["E|Same four setups and stochastic trigger as the 6-2-4 report (Tables 3.1-3.4, 4.1-4.2, p.24-36)"], "Duplicate of jb624_seasonal."),
        def!("jb30d_gap_tick", "Gap trade · futures rule (2-tick gap and penetration)", "Lesson 12 · Gap Trade", "Long/Short", Live,
            &["F|\"Use day-session daily-price data only\"; \"If a market opens below the low of the previous day, the market has set up a gap buy trade\" (p.100)",
              "F|\"In futures markets, the GS should be at least two ticks\"; \"In futures the PA should be at least two ticks\" (p.102)",
              "E|\"a trigger occurs when and if the market goes back up through the low of the previous day by a certain number of ticks\" (p.100): buy stop at yesterday's low + PA; sell mirrored at yesterday's high - PA",
              "X|\"Use a stop-loss that is 25 percent of the previous daily range above the price at which your trade triggered\" (p.102); below for a long",
              "X|\"Exit your trade at the end of the day if not stopped out\" (p.102)"],
            "One tick = p.tick (NSE index tick 0.05). Entry-day stop counted along the bar path (open, nearer extreme, other extreme, close). The 3-unit version (1/3 at a profit equal to yesterday's range, 1/3 at the close, 1/3 break-even stop or first profitable opening) is listed separately."),
        def!("jb30d_gap_pct", "Gap trade · stock rule (5% of yesterday's range)", "Lesson 12 · Gap Trade", "Long/Short", Live,
            &["F|\"In stocks the GS should be at least 5 percent of the previous daily trading range ... if a stock had a high of 35 and a low of 33 ... the size of the opening of the next day should be at least 35.10 for a gap up open or a 32.90 for a gap down open\" (p.102)",
              "E|\"In stocks the PA should be at least 5 percent of the previous daily range\" (p.102): buy stop at yesterday's low + 5% of yesterday's range",
              "X|Stop 25% of yesterday's range from the entry; exit at the end of the day (p.102)"],
            "Cash indices are run with the stock rule here; jb30d_gap_tick is the futures rule."),
        def!("jb30d_gap_thirds", "Gap trade · three-unit follow-through", "Lesson 12 · Gap Trade", "Long/Short", NotTestable("what the last third does when the first target is not reached but the trade is profitable at the close is not stated"),
            &["X|\"When the trade has made a profit equal to the range of the previous day take profit on one-third of your position and place a follow-up stop-loss at breakeven. Exit another third of your position at the end of the day. Exit the remaining third of your position either at the breakeven stop-loss or on the first profitable opening\" (p.102)"], ""),
        def!("jb30d_cmc", "CMC (Cuban Missile Crisis) pattern", "Lesson 14 · CMC", "Long/Short", Live,
            &["F|MAC: 10-period simple MA of highs, 8-period simple MA of lows; two complete bars outside the channel set the trend (p.86, 118)",
              "E|\"Buy trigger. After an MAC buy signal has occurred and the market is in an uptrend, buy on the close of trading of a bar that is completely below the moving average channel low\" (p.119); sell mirrored",
              "X|\"Exit your position if the next day is completely below the channel low if you are long or completely above the channel high if you are short\" (p.119): exit on that day's close",
              "X|\"Exit your entire position at the channel high MA if you are long or the channel low MA if you are short\" (p.119): resting limit at yesterday's MA"],
            "The first two of the three printed follow-throughs are combined (the worst case \"one more consecutive bar below the channel\" and the channel target, p.118); the partial exit with a 75% trailing stop is not coded. No stop after the first day."),
        def!("jb30d_oc8", "Eight-bar open/close pattern · reverse on the opposite cross", "Lesson 15 · Eight-Bar Open/Close", "Long/Short", Live,
            &["E|\"A buy setup and trigger occur when the eight-bar moving average of the close goes above the eight-bar moving average of the open. Entry is on the close of the bar or on the open of the next bar\" (p.127); sell mirrored",
              "X|\"Exit your position by reversing when the moving averages change their orientation. This is not recommended because you will give up too much profit\" (p.128)"],
            "Only follow-through 1 is fully mechanical; the author calls the pattern \"not a trading system\" (p.127) and filters for \"small or minor crossovers\" have no printed size. Entry on the close."),
        def!("jb30d_oc8_target", "Eight-bar open/close · target / fifth-bar exits", "Lesson 15 · Eight-Bar Open/Close", "Long/Short", NotTestable("no stop or handling of an opposite cross before the target is given; the author: \"this method is not totally objective\""),
            &["X|\"Exit a portion of your position at a profit target that is equal to the range of the last eight bars prior to the trigger\"; \"Exit part or all of your position on the close of the fifth bar after entry if it is profitable\" (p.128)"], ""),
        def!("jb30d_or30", "Advanced 30-minute breakout (S&P, E-mini, SPY)", "Lesson 18 · 30MBO", "Long/Short", NeedsData("intraday 30-minute bars"),
            &["E|\"A buy trigger occurs when and if any subsequent 30-minute price bar ends above the high of the first 30-minute price bar\" (p.152)",
              "X|Stop = the opposite trigger, reverse once per day; first target = the first 30-minute range; break-even and second-target stops; \"Exit MOC\" (p.153)"], ""),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    Some(match id {
        "jb30d_rev_buy_fpo" => ("jb30d_rev", "Reversal pattern with trigger (S&P)", "Buy · first profitable opening"),
        "jb30d_rev_sell_fpo" => ("jb30d_rev", "Reversal pattern with trigger (S&P)", "Sell · first profitable opening"),
        "jb30d_rev_buy_stop" => ("jb30d_rev", "Reversal pattern with trigger (S&P)", "Buy · $2,500 stop + FPO"),
        "jb30d_rev_buy_3pc" => ("jb30d_rev", "Reversal pattern with trigger (S&P)", "Buy · $4,900 stop + 3rd profitable close"),
        "jb30d_rev_simple" => ("jb30d_rev", "Reversal pattern with trigger (S&P)", "Simple reversal · not testable"),
        "jb30d_sp_seasonal" => ("jb30d_seasonal", "Key-date seasonals", "S&P Oct 27-Nov 1 · setup record"),
        "jb30d_sp_seasonal_trig" => ("jb30d_seasonal", "Key-date seasonals", "S&P with trigger · not testable"),
        "jb30d_seasonal_futures" => ("jb30d_seasonal", "Key-date seasonals", "Commodities · needs data"),
        "jb30d_gap_tick" => ("jb30d_gap", "Gap trade (30 Days)", "Futures rule · 2 ticks"),
        "jb30d_gap_pct" => ("jb30d_gap", "Gap trade (30 Days)", "Stock rule · 5% of range"),
        "jb30d_gap_thirds" => ("jb30d_gap", "Gap trade (30 Days)", "Three units · not testable"),
        "jb30d_cmc" => ("jb30d_cmc", "CMC pattern", "Channel target, next-day exit"),
        "jb30d_oc8" => ("jb30d_oc8", "Eight-bar open/close", "Reverse on the opposite cross"),
        "jb30d_oc8_target" => ("jb30d_oc8", "Eight-bar open/close", "Target exits · not testable"),
        "jb30d_or30" => ("jb30d_or30", "Advanced 30-minute breakout", "Needs intraday"),
        _ => return None,
    })
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    Some(match id {
        "jb30d_rev_buy_fpo" | "jb30d_rev_sell_fpo" => vec![],
        "jb30d_rev_buy_stop" => vec![p("stop", "Stop loss (points; $2,500 / $250)", 10., 2., 40., 0.5)],
        "jb30d_rev_buy_3pc" => vec![p("stop", "Stop loss (points; $4,900 / $250)", 19.6, 2., 40., 0.2), p("closes", "Exit on this profitable close", 3., 1., 6., 1.)],
        "jb30d_sp_seasonal" => vec![p("entry_m", "Entry month", 10., 1., 12., 1.), p("entry_d", "Entry day", 27., 1., 31., 1.), p("exit_m", "Exit month", 11., 1., 12., 1.),
            p("exit_d", "Exit day", 1., 1., 31., 1.), p("stop_pct", "Close-only stop (% below entry)", 1.5, 0.5, 10., 0.5)],
        "jb30d_gap_tick" => vec![p("tick", "One tick (index points)", 0.05, 0.01, 1., 0.01), p("gs", "Gap size, ticks (at least)", 2., 0., 10., 1.), p("pa", "Penetration, ticks", 2., 0., 10., 1.),
            p("stop_frac", "Stop as a fraction of yesterday's range", 0.25, 0.1, 1., 0.05)],
        "jb30d_gap_pct" => vec![p("gs_pct", "Gap size, % of yesterday's range", 5., 0., 25., 1.), p("pa_pct", "Penetration, % of yesterday's range", 5., 0., 25., 1.),
            p("stop_frac", "Stop as a fraction of yesterday's range", 0.25, 0.1, 1., 0.05)],
        "jb30d_cmc" => vec![p("mah", "MA of highs (bars)", 10., 5., 20., 1.), p("mal", "MA of lows (bars)", 8., 4., 20., 1.), p("bars", "Consecutive complete bars for a trigger", 2., 1., 4., 1.)],
        "jb30d_oc8" => vec![p("n", "MA of opens and of closes (bars)", 8., 3., 20., 1.)],
        _ => return None,
    })
}

/// Key-date seasonal (long): buy on the close of the first trading day on/after the entry date, sell on the close of the
/// first trading day on/after the exit date, close-only stop `stop_pct`% below the entry. Shared with jbssf.
pub fn seasonal(x: &Ctx, from: usize, to: usize, ent_md: (f64, f64), ext_md: (f64, f64), stop_pct: f64) -> Vec<Trade> {
    let stop = stop_pct / 100.0;
    let (mut out, mut cur): (Vec<Trade>, Option<Trade>) = (Vec::new(), None);
    for i in from.max(1)..=to {
        let d = &x.r[i].date;
        let (ent, ext) = (md_target(d, ent_md.0, ent_md.1), md_target(d, ext_md.0, ext_md.1));
        if let Some(t) = cur.as_ref().filter(|t| t.legs[0].i < i) {
            let e = t.legs[0].px;
            let ext_e = md_target(&x.r[t.legs[0].i].date, ext_md.0, ext_md.1);
            if d.as_str() >= ext_e.as_str() {
                out.push(finish(cur.take().unwrap(), i, x.c[i], i, "Exit date (close)"));
            } else if x.c[i] <= e * (1.0 - stop) {
                out.push(finish(cur.take().unwrap(), i, x.c[i], i, "Close-only stop"));
            }
            continue;
        }
        let first_on_or_after = d.as_str() >= ent.as_str() && x.r[i - 1].date.as_str() < ent.as_str();
        if cur.is_none() && i < to && first_on_or_after && d.as_str() < ext.as_str() {
            cur = Some(new_trade(1.0, i, x.c[i], "close", i));
        }
    }
    mark_open(&mut out, cur, x, to);
    out
}

fn md_target(date: &str, m: f64, d: f64) -> String {
    format!("{}-{:02}-{:02}", &date[0..4], m.round() as u32, d.round() as u32)
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !id.starts_with("jb30d_") || params(id).is_none() || to >= x.c.len() || from > to {
        return None;
    }
    let g = |k: &str| getp(id, p, k);
    let u = |k: &str| g(k).round().max(1.0) as usize;
    let mut out = Vec::new();
    let mut cur: Option<Trade> = None;
    match id {
        "jb30d_rev_buy_fpo" | "jb30d_rev_sell_fpo" | "jb30d_rev_buy_stop" | "jb30d_rev_buy_3pc" => {
            let d = if id == "jb30d_rev_sell_fpo" { -1.0 } else { 1.0 };
            let stop_pts = if id.ends_with("_stop") || id.ends_with("_3pc") { g("stop") } else { f64::NAN };
            let n_closes = if id.ends_with("_3pc") { u("closes") } else { 0 };
            let mut profitable = 0usize;
            for i in from.max(2)..=to {
                if let Some(t) = cur.as_ref().filter(|t| t.legs[0].i < i) {
                    let e = t.legs[0].px;
                    if n_closes == 0 && (x.o[i] - e) * d > 0.0 {
                        out.push(finish(cur.take().unwrap(), i, x.o[i], i, "First profitable opening"));
                        continue;
                    }
                    if let Some(px) = stop_fill(x, i, -d, e - d * stop_pts) {
                        out.push(finish(cur.take().unwrap(), i, px, i, "Stop loss"));
                        continue;
                    }
                    if n_closes > 0 && (x.c[i] - e) * d > 0.0 {
                        profitable += 1;
                        if profitable >= n_closes {
                            out.push(finish(cur.take().unwrap(), i, x.c[i], i, "Third profitable close"));
                        }
                    }
                    continue;
                }
                if cur.is_some() || i == to {
                    continue;
                }
                let sig = if d > 0.0 {
                    x.l[i - 1] < x.l[i - 2] && x.c[i - 1] > x.c[i - 2] && x.c[i] < x.l[i - 1]
                } else {
                    x.h[i - 1] > x.h[i - 2] && x.c[i - 1] < x.c[i - 2] && x.c[i] > x.h[i - 1]
                };
                if sig {
                    profitable = 0;
                    cur = Some(new_trade(d, i, x.c[i], "close", i));
                }
            }
        }
        "jb30d_sp_seasonal" => {
            out = seasonal(x, from, to, (g("entry_m"), g("entry_d")), (g("exit_m"), g("exit_d")), g("stop_pct"));
            return Some(out);
        }
        "jb30d_gap_tick" | "jb30d_gap_pct" => {
            let sf = g("stop_frac");
            for i in from.max(1)..=to {
                let rng = x.h[i - 1] - x.l[i - 1];
                let (gs, pa) = if id == "jb30d_gap_tick" { (g("gs") * g("tick"), g("pa") * g("tick")) } else { (g("gs_pct") / 100.0 * rng, g("pa_pct") / 100.0 * rng) };
                let d = if x.o[i] <= x.l[i - 1] - gs && x.o[i] < x.l[i - 1] { 1.0 } else if x.o[i] >= x.h[i - 1] + gs && x.o[i] > x.h[i - 1] { -1.0 } else { continue };
                let lvl = if d > 0.0 { x.l[i - 1] + pa } else { x.h[i - 1] - pa };
                let Some(px) = stop_fill(x, i, d, lvl) else { continue };
                let stop = px - d * sf * rng;
                let t = new_trade(d, i, px, "stop", i - 1);
                let p4 = bar_path(x, i);
                let k = (0..4).find(|&k| (p4[k] - lvl) * d >= 0.0).unwrap_or(0);
                if (k..4).any(|m| (p4[m] - stop) * d <= 0.0) {
                    out.push(finish(t, i, stop, i, "Stop: 25% of yesterday's range"));
                } else {
                    out.push(finish(t, i, x.c[i], i, "End of day"));
                }
            }
        }
        "jb30d_cmc" => {
            let m = mac(x, u("mah"), u("mal"), u("bars"));
            for i in from.max(1)..=to {
                if let Some(t) = cur.as_ref().filter(|t| t.legs[0].i < i) {
                    let d = t.dir;
                    let lvl = if d > 0.0 { m.mah[i - 1] } else { m.mal[i - 1] };
                    if let Some(px) = limit_fill(x, i, -d, lvl) {
                        out.push(finish(cur.take().unwrap(), i, px, i, "Channel target"));
                        continue;
                    }
                    let next_day = t.legs[0].i + 1 == i;
                    let outside = if d > 0.0 { m.dn_run[i] > 0 } else { m.up_run[i] > 0 };
                    if next_day && outside {
                        out.push(finish(cur.take().unwrap(), i, x.c[i], i, "Second bar outside the channel"));
                    }
                    continue;
                }
                if cur.is_some() || i == to {
                    continue;
                }
                if m.trend[i] == 1 && m.dn_run[i] == 1 {
                    cur = Some(new_trade(1.0, i, x.c[i], "close", i));
                } else if m.trend[i] == -1 && m.up_run[i] == 1 {
                    cur = Some(new_trade(-1.0, i, x.c[i], "close", i));
                }
            }
        }
        "jb30d_oc8" => {
            let n = u("n");
            let (mo, mc) = (sma(&x.o, n), sma(&x.c, n));
            // above / below with a float-noise tolerance (running-sum MAs of equal values must compare equal)
            let st = |i: usize| -> i8 { let e = 1e-9 * mo[i].abs(); if mc[i] - mo[i] > e { 1 } else if mo[i] - mc[i] > e { -1 } else { 0 } };
            for i in from.max(1)..=to {
                if !mc[i - 1].is_finite() || !mc[i].is_finite() {
                    continue;
                }
                let (s0, s1) = (st(i - 1), st(i));
                let side = if s1 != 0 && s1 != s0 { s1 as f64 } else { 0.0 };
                if side == 0.0 {
                    continue;
                }
                if let Some(t) = cur.as_ref() {
                    if t.dir == side {
                        continue;
                    }
                    out.push(finish(cur.take().unwrap(), i, x.c[i], i, "Opposite open/close cross (reverse)"));
                }
                if i < to {
                    cur = Some(new_trade(side, i, x.c[i], "close", i));
                }
            }
        }
        _ => return Some(vec![]),
    }
    mark_open(&mut out, cur, x, to);
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;
    fn row(d: &str, o: f64, h: f64, l: f64, c: f64) -> Row {
        Row { ticker: "T".into(), date: d.into(), open: o, high: h, low: l, close: c, volume: None }
    }
    #[test]
    fn sp_seasonal_rolls_to_next_business_day() {
        // 1986: Oct 27 Monday entry; Nov 1 is a Saturday -> exit Monday Nov 3 (Table 5.1)
        let r = vec![
            row("1986-10-24", 100., 101., 99., 100.),
            row("1986-10-27", 100., 101., 99., 100.),
            row("1986-10-31", 100., 102., 99., 101.),
            row("1986-11-03", 101., 103., 100., 102.),
            row("1986-11-04", 102., 103., 100., 102.5),
        ];
        let x = Ctx::new(&r, &r, &[]);
        let ts = run("jb30d_sp_seasonal", &x, 0, 4, &Params::new()).unwrap();
        assert_eq!(ts.len(), 1);
        assert_eq!((ts[0].legs[0].i, ts[0].exit_i), (1, 3));
        assert_eq!(ts[0].exit_px, 102.);
    }
}
