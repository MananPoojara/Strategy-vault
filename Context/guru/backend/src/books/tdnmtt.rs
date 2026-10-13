//! Thomas R. DeMark, *New Market Timing Techniques* (Wiley, 1997). Book id `tdnmtt`.
//! Source copy: Book-to-rust/Books/Thomas DeMark/ (image scan, 186 two-page spreads; spread n = printed pages ~2n-30, 2n-29).
//! Transcription, verification and README: library/tdnmtt/.
//!
//! Live: TD Sequential and TD Combo (I, II) with DeMark's own profit level and stop loss (Ch 2 p.102-103,
//! Ch 3 p.130-131), and TD Trap with the book's same-day close exit (Fig 6.18). Sequential/Combo engine lives in
//! `tdnsta` (shared, mirrored prices for sells).
use super::tdnsta::{run_countdown, Bars, Cfg, ExitCfg, Ver};
use crate::connors::{Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "tdnmtt";

fn d(id: &'static str, name: &'static str, group: &'static str, dir: &'static str, status: Status,
     rules: &'static [&'static str], notes: &'static str) -> StrategyDef {
    StrategyDef { id, name, group, session: 0, dir, status, needs_volume: false, needs_vix: false,
        needs_series: &[], book: BOOK, rules, notes, claim: None }
}

const SETUP: &str = "F|Setup (p.43-44, 120): \"the first day of a buy Setup series must be initialized by requiring that the previous trading day's closing price be greater than or equal to the close four trading days before that trading day\"; then \"at least nine consecutive closes less than the close four trading days earlier\" (sell mirrored); exec 10: \"not equal\" (an equal close interrupts). The setup \"continues for as long as the series of consecutive price relationships continue\" (p.121).";
const CANCEL: &str = "F|Cancellation (exec 5-7, p.92-93, 124-125): CaHTH - \"a closing price ... above the highest true high recorded during the entire buy Setup period\" (sell: close below the lowest true low) cancels \"anytime after the commencement of Setup until the day Countdown has been completed\"; Reverse - \"a buy Setup would be canceled if a sell Setup is formed prior to completion of the buy Countdown\" (\"This is my preference\"); Within - a redundant setup \"whose price extremes do not exceed the previously active Setup's extreme high and low\" does not recycle.";
const ENTRY: &str = "E|Entry (p.101-102, 130), param entry: 1 [default, first listed] \"take entry on the thirteenth day of Countdown\" (close); 2 \"the first closing price recorded that is greater than the opening price\" (sell: less); 3 option 2 and \"the high that same day is greater than the previous trading day's close\" (sell: low < previous close; e3_true=1: previous true high/true low, \"For more conservative traders\"); 4 \"price flip (close greater than the close four trading days earlier ...)\". A pending entry 2-4 is void if a new same-direction setup or an opposite setup completes first [recycle \"prior to a price flip ... requiring the abort or cancellation of the Countdown\"; exact rule silent].";
const PROFIT: &str = "X|Profit level (p.102 Sequential, p.131 Combo): \"Generally, you may look to take profits once an opposing Setup—in the other direction—is formed and the highest closing price of the current Setup period fails to exceed the intraday peak of the opposing Setup or the lowest closing price of the current Setup period fails to exceed the intraday low of the opposing Setup. This method conforms with the precepts for the TDST indicator. If, however, the closing price subsequent to a price bottom does exceed the intraday true high—or, subsequent to a price peak, the intraday true low—and is confirmed (see foregoing TDST), then generally expect price to continue through the Countdown process, and once Countdown is completed you can take profits\". Coded: at day 9 of the first opposite setup after entry, if no close of that setup exceeds the highest true high (sell: lowest true low) of the most recent same-direction setup series (TDST), exit at that close; a close beyond TDST counts only if confirmed by a higher high (sell: lower low) the next day (TDST p.59-61, Fig 2.7 caption) - if day 9's close is the only one beyond TDST the decision waits one day; otherwise hold until the opposite countdown 13 and exit at its close.";
const STOP: &str = "X|Stop loss (p.102-103, \"identical\" for Combo p.123): \"At a price bottom, identify the lowest low, including all the days of Setup and Countdown. Then calculate the difference between the true high (the high recorded that same day or the previous day's close, whichever is greater) and that same day's price low. This true price range is then subtracted from the low that day. Should price close below that value, the stop loss would be triggered.\" Sell: highest high + (high - true low), close above. stop_closes=2: \"A more liberal method would be to require two closes below and above the calculated stop-loss levels.\"";

const SEQ_RULES: &[&str] = &[SETUP,
    "F|Intersection (p.78-79, exec 3-4, 12): \"the high of day 8 of Setup or the first subsequent day be greater than or equal to the low three or more trading days earlier, up to and including the first day of Setup\" (sell: low <= high); \"If this period of price Intersection occurs on either day 8 or day 9 of Setup, then Countdown commences on day 9 of Setup, and if Intersection does not occur until after the Setup is complete, then Countdown is deferred until that time\"; \"in or after Setup\". Read: the day-9 check and later days use lows from setup day 1 to three days earlier.",
    "F|Countdown (p.79, exec 2, 11): \"a series of 13 closes less than or equal to the low two trading days earlier\" (sell: >= high two days earlier), not necessarily consecutive; \"equal\" for daily charts.",
    "F|Countdown qualifier (p.80-81): \"recommended as a standard Countdown qualifier default ... require the close of day 13 of buy Countdown to be less than or equal to the close of day 8 of Countdown\" (sell: >=); q13=2: \"At a minimum ... the low of day 13 ... less than or equal to the close of day 8\". Termination count (exec 16) \"defaulted to the closing price of day 13\"; term_open=1: the open may qualify day 13.",
    CANCEL,
    "F|Recycle (p.70-72, exec 13-15): a new same-direction setup before, on or after countdown 13 (prior to the price flip) restarts setup and countdown (\"My preferences are option 1 ... and a combination of options 9 and 11\"; \"Before, on, after\" and \"Ignore smaller true high-true low\"): it is ignored if its true high-true low range is smaller than the prior setup's, or larger than mult times it (\"I prefer a multiplier from 1.618 to 3.0 times\"; default 1.618 = lower end [range given, value not]).",
    ENTRY, PROFIT, STOP];

const COMBO_RULES: &[&str] = &[SETUP,
    "F|Countdown (p.121-122): \"TD Combo refers back to day 1 of Setup and begins Countdown on that day\" once nine closes are recorded. Version 1 buy: \"1. The close is less than or equal to the low two trading days earlier. 2. The Countdown day's low is less than the previous trading day's low. 3. Each successive Countdown day's close is less than the previous Countdown day's close ... and the previous trading day's close.\" Sell mirrored. No intersection (exec 3).",
    "F|Version 2 (p.122): \"days 11, 12, and 13 need neither close less than or equal to the low two trading days earlier nor record a lower intraday low versus the previous trading day's low, needing only to record three successively lower closes\" (vs the previous countdown day's close).",
    "F|Termination count (p.122): \"Typically, I prefer to select the opening price\"; exec 16 (\"identical\" settings): \"defaulted to the closing price of day 13\" - default close, term_open=1 lets the open qualify day 13's close <= low two days earlier.",
    CANCEL,
    "F|Recycle (p.128-129): \"ignore those Setups that encompass less price distance than the prior price Setup distance\" (true high to true low); \"One exception exists when the current Setup distance is greater than the original Setup distance by a factor of at least 2\" (mult default 2).",
    ENTRY, PROFIT, STOP];

const TRAP_RULES: &[&str] = &[
    "E|TD Trap (p.233): \"If the current trading day's open is contained by the previous trading day's price range (i.e., is less than or equal to the previous day's high and greater than or equal to the previous day's low), and either the current day's high is greater than the previous day's high or the current day's low is less than the previous day's low a TD Trap breakout is identified, either one tick above the previous trading day's high or one tick below the previous trading day's low.\"",
    "F|Recommended settings (p.235): \"hits, up-true high/down-true low for the reference price as well as the penetration level, one trading day ago, with equal\" (ref_true=1 default; 0 = plain high/low as in Fig 6.19); the \"occasional qualifiers and occasional range penetration\" are not applied.",
    "E|Buy stop one tick above the previous true high (sell: one tick below the previous true low); tick = 0.05 (S&P futures 1997 / NSE tick).",
    "F|\"Whichever direction breaks out is the direction in which the trade should be taken\" (Fig 6.19): on a day that breaks both sides the first one cannot be known from daily bars - skipped (skip_both=1 default) [book silent on daily data].",
    "X|Exit (Fig 6.18 caption): \"As with TD Open, it is often better to exit a low-risk entry on the close\" - exit at the close of the entry day.",
];

pub fn catalogue() -> Vec<StrategyDef> {
    use Status::*;
    let nt = "Not testable; see README for the page and reason.";
    let seqn = "DeMark's 1997 TD Sequential with his own profit level and stop (Perl 2008 gives no exit). The book is an indicator manual (\"presented intentionally in the context of an indicator rather than as a system\", p.98) and prints no statistics. Lowest-low ties: latest bar [book silent]. India: instrument swap only; cash-index shorts are hypothetical.";
    let combon = "DeMark's 1997 TD Combo with the profit level and stop of Ch 3 (p.130-131, identical to Sequential). No statistics printed. India: instrument swap only; cash-index shorts are hypothetical.";
    let trapn = "TD Trap as a stand-alone day trade with the book's close exit. Cash-index opens are partly stale (see README), which affects open-based rules. India: instrument swap only.";
    vec![
        d("tdnmtt_seq_long", "TD Sequential buy (1997 rules, DeMark's profit level and stop)", "Ch 2 · TD Sequential", "Long", Live, SEQ_RULES, seqn),
        d("tdnmtt_seq_short", "TD Sequential sell (1997 rules, DeMark's profit level and stop)", "Ch 2 · TD Sequential", "Short", Live, SEQ_RULES, seqn),
        d("tdnmtt_combo1_long", "TD Combo version 1 buy (DeMark's profit level and stop)", "Ch 3 · TD Combo", "Long", Live, COMBO_RULES, combon),
        d("tdnmtt_combo1_short", "TD Combo version 1 sell (DeMark's profit level and stop)", "Ch 3 · TD Combo", "Short", Live, COMBO_RULES, combon),
        d("tdnmtt_combo2_long", "TD Combo version 2 buy (DeMark's profit level and stop)", "Ch 3 · TD Combo", "Long", Live, COMBO_RULES, combon),
        d("tdnmtt_combo2_short", "TD Combo version 2 sell (DeMark's profit level and stop)", "Ch 3 · TD Combo", "Short", Live, COMBO_RULES, combon),
        d("tdnmtt_trap_long", "TD Trap upside breakout, out on the close", "Ch 6 · Price breakouts", "Long", Live, TRAP_RULES, trapn),
        d("tdnmtt_trap_short", "TD Trap downside breakout, out on the close", "Ch 6 · Price breakouts", "Short", Live, TRAP_RULES, trapn),
        d("tdnmtt_plurality", "TD Plurality", "Ch 1 · Oscillators", "Long/Short", NotTestable("composite chosen by the trader; no exit (p.34-39)"), &[], nt),
        d("tdnmtt_tdst", "TD Setup Trend (TDST) breakout", "Ch 2 · TD Sequential", "Long/Short",
          NotTestable("trend-reversal level, not a trade rule: entry timing (breakout close vs confirmation) and exit (\"through sell Setup and Countdown or until ...\") are descriptions (p.58-61)"), &[], nt),
        d("tdnmtt_spring_lv_pr", "TD Spring, TD LV, TD Point Reversal, TD Double TD Point", "Ch 6 · Price breakouts", "Long/Short",
          NotTestable("entries only, no exit or stop (p.219-231)"), &[], nt),
        d("tdnmtt_tdma", "TD Moving Average I and II", "Ch 7 · TD moving averages", "Long/Short",
          NotTestable("MA I is a stop/qualifier without an entry-exit pair; MA II is a trend view (p.244-249)"), &[], nt),
        d("tdnmtt_systems", "TD TJ, TD Carrie, TD Meghan, TD Rocke", "Ch 8 · Systems", "Long/Short",
          NotTestable("only \"the basics in general terms without an extended discussion of specific details\" (p.251); exits not given"), &[], nt),
        d("tdnmtt_triangulation", "TD Triangulation and TD Propulsion", "Ch 9", "Long/Short",
          NotTestable("the reference price low/high and the \"reaction bottom in an uptrend\" are not mechanically defined; \"apply ... only to the futures contracts with the largest open interest\" (p.257-268)"), &[], nt),
        d("tdnmtt_tdt_pivot_pressure", "TDT, TD Pivot, TD Pressure", "Ch 10", "Long/Short", NotTestable("no exit; reference choices open (p.269-283)"), &[], nt),
        d("tdnmtt_channels", "TD Trend Factors, TD Channels I-III", "Ch 11", "Long/Short", NotTestable("levels only, no trade rules (p.284-296)"), &[], nt),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    Some(match id {
        "tdnmtt_seq_long" => ("tdnmtt_seq", "TD Sequential (1997)", "Buy"),
        "tdnmtt_seq_short" => ("tdnmtt_seq", "TD Sequential (1997)", "Sell"),
        "tdnmtt_combo1_long" => ("tdnmtt_combo", "TD Combo (1997)", "Version 1 · Buy"),
        "tdnmtt_combo1_short" => ("tdnmtt_combo", "TD Combo (1997)", "Version 1 · Sell"),
        "tdnmtt_combo2_long" => ("tdnmtt_combo", "TD Combo (1997)", "Version 2 · Buy"),
        "tdnmtt_combo2_short" => ("tdnmtt_combo", "TD Combo (1997)", "Version 2 · Sell"),
        "tdnmtt_trap_long" => ("tdnmtt_trap", "TD Trap", "Upside"),
        "tdnmtt_trap_short" => ("tdnmtt_trap", "TD Trap", "Downside"),
        _ if id.starts_with("tdnmtt_") && catalogue().iter().any(|s| s.id == id) => ("tdnmtt_other", "New Market Timing: indicators without exits", ""),
        _ => return None,
    })
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    let common = |mult: f64| vec![
        p("setup_n", "Setup: consecutive closes", 9., 7., 11., 1.), p("look", "Setup: vs close N bars earlier", 4., 3., 5., 1.),
        p("cd_n", "Countdown bars", 13., 11., 15., 1.), p("cd_look", "Countdown: vs low/high N bars earlier", 2., 1., 3., 1.),
        p("mult", "Recycle: ignore setups larger than N x the prior one", mult, 1.618, 3., 0.191),
        p("term_open", "Termination count: open may qualify day 13", 0., 0., 1., 1.),
        p("entry", "Entry: 1 close of 13, 2 close > open, 3 close > open and high > prior close, 4 price flip", 1., 1., 4., 1.),
        p("e3_true", "Entry 3 uses the prior true high/low", 0., 0., 1., 1.),
        p("stop_closes", "Stop: closes beyond the level needed", 1., 1., 2., 1.),
    ];
    Some(match id {
        "tdnmtt_seq_long" | "tdnmtt_seq_short" => {
            let mut v = common(1.618);
            v.push(p("q13", "Day-13 qualifier: 0 off, 1 close13 <= close8, 2 low13 <= close8", 1., 0., 2., 1.));
            v
        }
        "tdnmtt_combo1_long" | "tdnmtt_combo1_short" | "tdnmtt_combo2_long" | "tdnmtt_combo2_short" => common(2.),
        "tdnmtt_trap_long" | "tdnmtt_trap_short" => vec![p("tick", "One tick (price units)", 0.05, 0., 1., 0.01),
            p("ref_true", "Reference: 1 true high/low, 0 high/low", 1., 0., 1., 1.), p("skip_both", "Skip days breaking both sides", 1., 0., 1., 1.)],
        _ if id.starts_with("tdnmtt_") && catalogue().iter().any(|s| s.id == id) => vec![],
        _ => return None,
    })
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !id.starts_with("tdnmtt_") || from > to || to >= x.c.len() { return None; }
    if !catalogue().iter().any(|d| d.id == id && d.status == Status::Live) { return None; }
    let defs = params(id)?;
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| defs.iter().find(|d| d.key == k).unwrap().def);
    let short = id.ends_with("_short");
    if id.starts_with("tdnmtt_trap") { return Some(trap(x, from, to, short, g("tick"), g("ref_true") >= 0.5, g("skip_both") >= 0.5)); }
    let u = |k: &str| g(k).round() as usize;
    let ver = if id.starts_with("tdnmtt_seq") { Ver::Nmtt } else if id.starts_with("tdnmtt_combo1") { Ver::Combo1 } else { Ver::Combo2 };
    let cfg = Cfg { ver, setup_n: u("setup_n").max(5), look: u("look").max(1), cd_n: u("cd_n").max(8), cd_look: u("cd_look").max(1),
        mult: g("mult"), q13: if ver == Ver::Nmtt { u("q13") as u8 } else { 0 }, term_open: g("term_open") >= 0.5 };
    let ex = ExitCfg { nsta: false, entry: u("entry") as u8, stop_kind: 1, stop_closes: u("stop_closes"), e3_true: g("e3_true") >= 0.5 };
    Some(run_countdown(x, from, to, short, &cfg, &ex))
}

/// TD Trap: open inside the prior (true) range, stop one tick beyond it, out on the close.
fn trap(x: &Ctx, from: usize, to: usize, short: bool, tick: f64, true_ref: bool, skip_both: bool) -> Vec<Trade> {
    let b = Bars::new(x, to, short);
    let m = Bars::new(x, to, !short);
    let lvl = |z: &Bars, i: usize| -> (f64, f64) { if true_ref { (z.th[i - 1], z.tl[i - 1]) } else { (z.h[i - 1], z.l[i - 1]) } };
    let mut out = vec![];
    for i in from.max(1)..=to {
        let (hi, lo) = lvl(&b, i);
        if !(b.o[i] <= hi && b.o[i] >= lo) { continue; }
        let stop = hi + tick;
        if b.h[i] < stop { continue; }
        let (mhi, _) = lvl(&m, i);
        if skip_both && m.h[i] >= mhi + tick { continue; }
        let (px, ex) = (b.px(stop), b.px(b.c[i]));
        out.push(Trade { dir: b.s, legs: vec![Leg { i, px, w: 1. }], exit_i: i, exit_px: ex, reason: "Close of the entry day".into(),
            entry_kind: "stop", signal_i: i, open: false, exits: vec![] });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;
    fn row(i: usize, o: f64, h: f64, l: f64, c: f64) -> Row { Row { ticker: "T".into(), date: format!("{:04}-01-01", 1900 + i), open: o, high: h, low: l, close: c, volume: None } }
    #[test]
    fn trap_fills_one_tick_above_prior_true_high_and_exits_on_close() {
        let r = vec![row(0, 100., 101., 99., 100.), row(1, 100., 102., 98., 99.), row(2, 101., 103., 100.5, 102.5), row(3, 102., 102.2, 97., 98.)];
        let x = Ctx::new(&r, &r, &[]);
        let t = trap(&x, 0, 3, false, 0.05, true, true);
        assert_eq!(t.len(), 1);
        assert_eq!(t[0].legs[0].i, 2); assert!((t[0].legs[0].px - 102.05).abs() < 1e-9); assert_eq!(t[0].exit_px, 102.5);
        let s = trap(&x, 0, 3, true, 0.05, true, true);
        assert_eq!(s.len(), 1); assert_eq!(s[0].legs[0].i, 3); assert!((s[0].legs[0].px - 98.95).abs() < 1e-9);
    }
}
