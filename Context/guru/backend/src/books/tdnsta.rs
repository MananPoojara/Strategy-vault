//! Thomas R. DeMark, *The New Science of Technical Analysis* (Wiley, 1994). Book id `tdnsta`.
//! Source copy: Book-to-rust/Books/Thomas DeMark/ (image scan, 131 two-page spreads; spread n = printed pages 2n-16, 2n-15).
//! Transcription, verification and README: library/tdnsta/.
//!
//! Only Chapter 7 (TD Sequential) has a complete rule set with an exit and a stop (p.164-178). Every other
//! chapter describes indicators without an exit, or duplicates rules already live in `tdperl`.
//! This file also holds the Sequential / Combo engine shared with `tdnmtt` (DeMark, 1997): every sell rule
//! is the buy rule on mirrored prices (o,h,l,c -> -o,-l,-h,-c), so one code path serves both sides.
use crate::connors::{Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "tdnsta";

fn d(id: &'static str, name: &'static str, group: &'static str, dir: &'static str, status: Status,
     rules: &'static [&'static str], notes: &'static str) -> StrategyDef {
    StrategyDef { id, name, group, session: 0, dir, status, needs_volume: false, needs_vix: false,
        needs_series: &[], book: BOOK, rules, notes, claim: None }
}

const SEQ_RULES: &[&str] = &[
    "F|Setup (p.140-141): \"at least nine consecutive closes less than the close four trading days earlier\"; \"The first day of the nine-day buy setup must be preceded by a close on the trading day immediately before it that is greater than or equal to the close four trading days earlier\". Sell: closes greater than the close four days earlier, preceded by a close less than or equal to it.",
    "F|Setup (p.144-145): \"If the close of a trading day is equal to the close of the trading day four days before it, the setup series is interrupted and must begin anew\".",
    "F|Intersection (p.148-151): \"intersection for a buy setup takes place once the high of either day 8 or day 9 of the setup is greater than or equal to the low three, four, five, six, or seven days earlier ... if intersection does not occur on day 8 or on day 9 of the setup, then it may occur on any subsequent day ... All that is required is that the price intersect the price low three or more days earlier\" (sell: low <= high). Read: \"three or more days earlier\" is bounded at setup day 1 [book silent on the bound; DeMark's 1997 text: \"up to and including the first day of Setup\"].",
    "F|Countdown (p.153, 158): \"the close must be less than the low two days earlier\" (sell: \"greater than the high two trading days earlier\"); \"a total of 13 closes\" that \"need not occur consecutively\"; \"Once intersection has taken place ... and beginning no earlier than day 9 of the setup—the countdown begins\" (day 9 or the intersection day counts).",
    "F|Cancellation (p.152): \"should a subsequent closing price exceed either the highest intraday high—in the case of a buy setup—or the lowest intraday low—in the case of a sell setup—the setup phase is canceled\". (p.158-159): \"The original setup is negated at any time subsequent to setup, and prior to a signal, a setup in the opposing direction occurs\".",
    "F|Recycle (p.158-159): \"a subsequent setup is formed simultaneously as the countdown process is taking place. This new setup replaces the original setup\" (countdown starts over on the new setup).",
    "E|Entry (p.164-172), param entry: 1 [default, first listed] \"enters the market on the close of the day in which countdown is completed\"; 2 price flip, \"a close greater than the close four days earlier\" (sell: less); 3 two-day range flip, \"buy the first time a subsequent close greater than the high two days earlier occurs\" (sell: close less than the low two days earlier). A pending entry 2/3 is void if a new same-direction setup (recycle) or an opposite setup completes first [book: \"the setup can be recycled and the original signal will evaporate\"; exact voiding rule silent].",
    "X|Exit (p.172-173): \"There are two ways in which to exit a trade other than being stopped out ... The first method is to liquidate the position once the current setup is completed and price fails to exceed the furthest price level recorded by the most recent inactive setup ... The other exit also compares the two setups, but in this instance if any price recorded during the current active setup exceeds the furthest price of the inactive setup, then the position is held until a reverse signal is generated.\" Coded: at the close of day 9 of the first opposite setup after entry, if its highest high (sell: lowest low) does not exceed the highest high (sell: lowest low) of the most recent same-direction setup series completed before it began, exit at that close; otherwise hold until the opposite Sequential countdown 13 and exit at its close.",
    "X|Stop (p.176-178): \"Both consider the price range of the lowest range day throughout the entire period of setup and countdown for a buy signal ... the true range for the lowest range day is calculated by subtracting the low that day from the high that day or the close the previous day, whichever is greater. The stop loss level is defined as that value subtracted from the low that day ... a subsequent close must exceed the calculated values in order to activate a stop loss.\" stop=2: \"a stop loss for a buy signal is calculated by subtracting, from the low the difference between the close and the low\". Exit at the close that breaches the level.",
];

pub fn catalogue() -> Vec<StrategyDef> {
    use Status::*;
    let nt = "Not testable; see README for the page and reason.";
    vec![
        d("tdnsta_seq_long", "TD Sequential buy (1994 rules, DeMark's exit and stop)", "Ch 7 · Sequential", "Long", Live, SEQ_RULES,
          "DeMark's original 1994 Sequential with the exit and stop he gives on p.172-178 (Perl's 2008 book gives no exit). Book prints no statistics (charts only). Ties for the lowest-low day: the latest bar [book silent]. Sell stop: the text on p.177 says \"the low of that day or the close one day earlier, whichever is greater\"; the figure 7.43 caption and DeMark 1997 (p.103) use the true low (lesser) - the true low is used. India: instrument swap only."),
        d("tdnsta_seq_short", "TD Sequential sell (1994 rules, DeMark's exit and stop)", "Ch 7 · Sequential", "Short", Live, SEQ_RULES,
          "Mirror of tdnsta_seq_long. Short on the cash index is hypothetical (no borrow/futures basis). India: instrument swap only."),
        d("tdnsta_ma", "TD moving averages (5/21 projected, 13-day high/low MA, 110%/90% band, TD MA 13/55)", "Ch 6 · Moving averages", "Long/Short",
          NotTestable("no exit: p.129-134 give look-to-buy/sell zones and trigger days only"), &[], nt),
        d("tdnsta_accdist", "Accumulation / distribution indexes and buying-pressure ratios", "Ch 5 · Accumulation/distribution", "Long/Short",
          NotTestable("no trade rule or exit (p.109-127)"), &[], nt),
        d("tdnsta_gaps", "Gap observations", "Ch 8 · Gaps", "Long/Short", NotTestable("observations only, no entry or exit (p.183-192)"), &[], nt),
        d("tdnsta_roc", "Rate of change vs one year earlier with market-specific bands", "Ch 10 · Rate of change", "Long/Short",
          NotTestable("bands drawn per market by judgement; no exit (p.197-214)"), &[], nt),
        d("tdnsta_equities", "Equity timing: IPOs, buy-outs, TD New High/Low index, A-D matrix", "Ch 11 · Equities", "Long",
          NotTestable("needs stock universe/fundamentals/breadth and gives no exit (p.215-225)"), &[], nt),
        d("tdnsta_options", "Option buying checklist, TD Dollar-Weighted Option Ratio", "Ch 12 · Options", "Long/Short",
          NotTestable("needs option prices and gives no exit (p.227-231)"), &[], nt),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    match id {
        "tdnsta_seq_long" => Some(("tdnsta_seq", "TD Sequential (1994)", "Buy")),
        "tdnsta_seq_short" => Some(("tdnsta_seq", "TD Sequential (1994)", "Sell")),
        _ if id.starts_with("tdnsta_") && catalogue().iter().any(|s| s.id == id) => Some(("tdnsta_other", "New Science: indicators without exits", "")),
        _ => None,
    }
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    match id {
        "tdnsta_seq_long" | "tdnsta_seq_short" => Some(vec![
            p("setup_n", "Setup: consecutive closes", 9., 7., 11., 1.), p("look", "Setup: vs close N days earlier", 4., 3., 5., 1.),
            p("cd_n", "Countdown closes", 13., 11., 15., 1.), p("cd_look", "Countdown: vs low/high N days earlier", 2., 1., 3., 1.),
            p("entry", "Entry: 1 close of 13, 2 price flip, 3 two-day range flip", 1., 1., 3., 1.),
            p("stop", "Stop: 1 true range of the lowest day, 2 close-to-low of that day", 1., 1., 2., 1.),
        ]),
        _ if id.starts_with("tdnsta_") && catalogue().iter().any(|s| s.id == id) => Some(vec![]),
        _ => None,
    }
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    let short = match id { "tdnsta_seq_long" => false, "tdnsta_seq_short" => true, _ => return None };
    if from > to || to >= x.c.len() { return None; }
    let defs = params(id)?;
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| defs.iter().find(|d| d.key == k).unwrap().def).round() as usize;
    let cfg = Cfg { ver: Ver::Nsta, setup_n: g("setup_n").max(5), look: g("look").max(1), cd_n: g("cd_n").max(8), cd_look: g("cd_look").max(1),
        mult: 0., q13: 0, term_open: false };
    let ex = ExitCfg { nsta: true, entry: g("entry") as u8, stop_kind: g("stop") as u8, stop_closes: 1, e3_true: false };
    Some(run_countdown(x, from, to, short, &cfg, &ex))
}

// ---------------------------------------------------------------- shared engine (also used by tdnmtt)

/// Bars up to `to`; `short` mirrors them so that every sell rule is the buy rule.
pub(crate) struct Bars { pub o: Vec<f64>, pub h: Vec<f64>, pub l: Vec<f64>, pub c: Vec<f64>, pub th: Vec<f64>, pub tl: Vec<f64>, pub s: f64 }

impl Bars {
    pub(crate) fn new(x: &Ctx, to: usize, short: bool) -> Bars {
        let n = to + 1;
        let s = if short { -1. } else { 1. };
        let o: Vec<f64> = (0..n).map(|i| s * x.o[i]).collect();
        let c: Vec<f64> = (0..n).map(|i| s * x.c[i]).collect();
        let (h, l): (Vec<f64>, Vec<f64>) = (0..n).map(|i| if short { (-x.l[i], -x.h[i]) } else { (x.h[i], x.l[i]) }).unzip();
        let th = (0..n).map(|i| if i == 0 { h[0] } else { h[i].max(c[i - 1]) }).collect();
        let tl = (0..n).map(|i| if i == 0 { l[0] } else { l[i].min(c[i - 1]) }).collect();
        Bars { o, h, l, c, th, tl, s }
    }
    pub(crate) fn n(&self) -> usize { self.c.len() }
    pub(crate) fn px(&self, v: f64) -> f64 { self.s * v }
}

/// Buy-setup count: consecutive closes < close `look` earlier, the first one preceded by a close >= close `look` earlier.
pub(crate) fn setups(b: &Bars, look: usize) -> Vec<u32> {
    let mut v = vec![0u32; b.n()];
    for i in look + 1..b.n() {
        if b.c[i] < b.c[i - look] {
            v[i] = if v[i - 1] > 0 { v[i - 1] + 1 } else if b.c[i - 1] >= b.c[i - 1 - look] { 1 } else { 0 };
        }
    }
    v
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub(crate) enum Ver { Nsta, Nmtt, Combo1, Combo2 }

pub(crate) struct Cfg { pub ver: Ver, pub setup_n: usize, pub look: usize, pub cd_n: usize, pub cd_look: usize,
    /// recycle multiplier (1997 rules): a new setup replaces the active one only if 1 <= range ratio < mult
    pub mult: f64,
    /// 1997 Sequential 13th-day qualifier: 0 off, 1 close13 <= close8, 2 low13 <= close8
    pub q13: u8, pub term_open: bool }

/// A completed countdown (mirrored coordinates for sells): bar of the 13 and setup day 1 of the active setup.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Sig { pub i: usize, pub s1: usize }

struct Act { s1: usize, s9: usize, end: usize, hh: f64, hth: f64, ll: f64, ltl: f64, start: Option<usize>, counted: Vec<usize> }

impl Act {
    fn new(b: &Bars, s1: usize, s9: usize) -> Act {
        let mut a = Act { s1, s9, end: s9, hh: f64::MIN, hth: f64::MIN, ll: f64::MAX, ltl: f64::MAX, start: None, counted: vec![] };
        for j in s1..=s9 { a.add(b, j) }
        a
    }
    fn add(&mut self, b: &Bars, j: usize) {
        self.hh = self.hh.max(b.h[j]); self.hth = self.hth.max(b.th[j]); self.ll = self.ll.min(b.l[j]); self.ltl = self.ltl.min(b.tl[j]);
    }
}

/// All buy countdown completions on bars `b`. `own` = buy setups of `b`, `opp` = buy setups of the mirrored bars (= sell setups of `b`).
pub(crate) fn countdowns(b: &Bars, own: &[u32], opp: &[u32], cfg: &Cfg) -> Vec<Sig> {
    let combo = matches!(cfg.ver, Ver::Combo1 | Ver::Combo2);
    let sn = cfg.setup_n as u32;
    let qualifies = |a: &Act, j: usize| -> bool {
        if j < cfg.cd_look.max(1) { return false; }
        let k = a.counted.len();
        let last13 = k + 1 == cfg.cd_n;
        let below = b.c[j] <= b.l[j - cfg.cd_look] || (cfg.term_open && last13 && b.o[j] <= b.l[j - cfg.cd_look]);
        match cfg.ver {
            Ver::Nsta => b.c[j] < b.l[j - cfg.cd_look],
            Ver::Nmtt => {
                if !below { return false; }
                if last13 && cfg.cd_n >= 8 {
                    let c8 = b.c[a.counted[7]];
                    return match cfg.q13 { 1 => b.c[j] <= c8, 2 => b.l[j] <= c8, _ => true };
                }
                true
            }
            Ver::Combo1 | Ver::Combo2 => {
                let succ = a.counted.last().map_or(true, |&m| b.c[j] < b.c[m]);
                if cfg.ver == Ver::Combo2 && k + 3 >= cfg.cd_n { return succ; }
                below && b.l[j] < b.l[j - 1] && b.c[j] < b.c[j - 1] && succ
            }
        }
    };
    let mut out = vec![];
    let mut act: Option<Act> = None;
    for i in 0..b.n() {
        // the active setup series keeps extending while it continues
        if let Some(a) = act.as_mut() {
            if own[i] > 0 && i + 1 - own[i] as usize == a.s1 && i > a.end { a.end = i; a.add(b, i); }
        }
        // an opposite setup cancels
        if opp[i] == sn { act = None; }
        // a close beyond the setup's extreme cancels (1994: highest high; 1997: highest true high)
        if let Some(a) = act.as_ref() {
            let lvl = if cfg.ver == Ver::Nsta { a.hh } else { a.hth };
            if i > a.s9 && b.c[i] > lvl { act = None; }
        }
        // a new same-direction setup: start, or recycle
        let mut retro = None;
        if own[i] == sn {
            let s1 = i + 1 - cfg.setup_n;
            let new = Act::new(b, s1, i);
            let replace = match act.as_ref() {
                None => true,
                Some(old) => match cfg.ver {
                    Ver::Nsta => true,
                    _ => {
                        let within = new.hh <= old.hh && new.ll >= old.ll;
                        let r = (new.hth - new.ltl) / (old.hth - old.ltl);
                        !within && r >= 1.0 && r < cfg.mult
                    }
                },
            };
            if replace { act = Some(new); if combo { retro = Some(s1); } }
        }
        let Some(a) = act.as_mut() else { continue };
        if let Some(r) = retro {
            for j in r..i { if qualifies(a, j) { a.counted.push(j); } }
        }
        // intersection (Sequential only)
        if !combo && a.start.is_none() && i >= a.s9 {
            let (s1, d8, d9) = (a.s1, a.s1 + 7, a.s1 + 8);
            let hit = if i == d9 {
                let lo9 = if cfg.ver == Ver::Nsta { s1 + 1 } else { s1 };
                (s1..=s1 + 4).any(|k| b.h[d8] >= b.l[k]) || (lo9..=s1 + 5).any(|k| b.h[d9] >= b.l[k])
            } else { i >= s1 + 3 && (s1..=i - 3).any(|k| b.h[i] >= b.l[k]) };
            if hit { a.start = Some(i); }
        }
        if (combo || a.start.is_some()) && qualifies(a, i) {
            a.counted.push(i);
            if a.counted.len() == cfg.cd_n { out.push(Sig { i, s1: a.s1 }); act = None; }
        }
    }
    out
}

pub(crate) struct ExitCfg {
    /// true: 1994 exit (any price of the opposite setup vs the furthest price of the inactive setup);
    /// false: 1997 exit (highest close of the opposite setup vs the true high of the inactive setup, confirmed)
    pub nsta: bool,
    pub entry: u8, pub stop_kind: u8, pub stop_closes: usize, pub e3_true: bool,
}

/// Completed own setup series (s1, day9, series end, highest high, highest true high), in order of completion.
fn series(b: &Bars, own: &[u32], sn: usize) -> Vec<(usize, usize, usize, f64, f64)> {
    let mut v = vec![];
    for i in 0..b.n() {
        if own[i] as usize == sn {
            let s1 = i + 1 - sn;
            let mut e = i;
            while e + 1 < b.n() && own[e + 1] > 0 { e += 1; }
            let hh = (s1..=e).map(|j| b.h[j]).fold(f64::MIN, f64::max);
            let hth = (s1..=e).map(|j| b.th[j]).fold(f64::MIN, f64::max);
            v.push((s1, i, e, hh, hth));
        }
    }
    v
}

/// Long side (mirrored for shorts) of a Sequential/Combo countdown with DeMark's exit and stop.
pub(crate) fn run_countdown(x: &Ctx, from: usize, to: usize, short: bool, cfg: &Cfg, ex: &ExitCfg) -> Vec<Trade> {
    let b = Bars::new(x, to, short);
    let m = Bars::new(x, to, !short);
    let (own, opp) = (setups(&b, cfg.look), setups(&m, cfg.look));
    let sigs = countdowns(&b, &own, &opp, cfg);
    let osigs: Vec<usize> = countdowns(&m, &opp, &own, cfg).iter().map(|s| s.i).collect();
    let inact = series(&b, &own, cfg.setup_n);
    let n = b.n();
    let sn = cfg.setup_n as u32;
    let mut out = vec![];
    struct Pos { t: Trade, stop: f64, below: usize, decided: Option<usize>, wait: Option<usize> }
    let mut pos: Option<Pos> = None;
    let mut pending: Option<Sig> = None;
    let (mut si, mut last_exit) = (0usize, None::<usize>);
    let stop_of = |s: &Sig| -> f64 {
        let j = (s.s1..=s.i).rev().min_by(|&p, &q| b.l[p].partial_cmp(&b.l[q]).unwrap()).unwrap();
        if ex.stop_kind == 2 { b.l[j] - (b.c[j] - b.l[j]) } else { b.l[j] - (b.th[j] - b.l[j]) }
    };
    let close_t = |mut t: Trade, i: usize, why: &str| { t.exit_i = i; t.exit_px = b.px(b.c[i]); t.signal_i = i; t.reason = why.into(); t };
    for i in 0..n {
        let sig_now = { while si < sigs.len() && sigs[si].i < i { si += 1 } sigs.get(si).filter(|s| s.i == i).copied() };
        if let Some(mut p) = pos.take() {
            let e = p.t.legs[0].i;
            if b.c[i] < p.stop { p.below += 1 } else { p.below = 0 }
            if p.below >= ex.stop_closes.max(1) { out.push(close_t(p.t, i, "TD stop loss (close beyond the level)")); last_exit = Some(i); continue; }
            if let Some(d) = p.decided {
                if i > d && osigs.binary_search(&i).is_ok() { out.push(close_t(p.t, i, "Reverse signal: opposite countdown 13")); last_exit = Some(i); continue; }
                pos = Some(p); continue;
            }
            if let Some(k) = p.wait {
                // 1997: a close beyond TDST on setup day 9 needs a higher high the next day to be confirmed
                if b.h[i] > b.h[k] { p.wait = None; p.decided = Some(i); pos = Some(p); }
                else { out.push(close_t(p.t, i, "Opposite setup failed to exceed TDST (not confirmed)")); last_exit = Some(i); }
                continue;
            }
            if i > e && opp[i] == sn {
                let s1 = i + 1 - cfg.setup_n;
                let ref_setup = inact.iter().rev().find(|v| v.1 < s1).copied();
                let Some((_, _, _, hh, hth)) = ref_setup else { p.decided = Some(i); pos = Some(p); continue };
                if ex.nsta {
                    if (s1..=i).any(|j| b.h[j] > hh) { p.decided = Some(i); pos = Some(p); }
                    else { out.push(close_t(p.t, i, "Opposite setup failed to exceed the inactive setup")); last_exit = Some(i); }
                } else {
                    let ks: Vec<usize> = (s1..=i).filter(|&k| b.c[k] > hth).collect();
                    if ks.is_empty() { out.push(close_t(p.t, i, "Opposite setup's highest close failed to exceed TDST")); last_exit = Some(i); }
                    else if ks.iter().any(|&k| k < i && b.h[k + 1] > b.h[k]) { p.decided = Some(i); pos = Some(p); }
                    else if *ks.last().unwrap() == i { p.wait = Some(i); pos = Some(p); }
                    else { out.push(close_t(p.t, i, "Opposite setup failed to exceed TDST (not confirmed)")); last_exit = Some(i); }
                }
                continue;
            }
            pos = Some(p);
            continue;
        }
        if last_exit == Some(i) { continue; }
        if let Some(s) = sig_now { pending = Some(s); }
        let Some(s) = pending else { continue };
        let fill = if ex.entry <= 1 { true } else {
            if i == s.i { false } else if own[i] == sn || opp[i] == sn { pending = None; false } else {
                let (o, c) = (b.o[i], b.c[i]);
                match (ex.nsta, ex.entry) {
                    (true, 2) => i >= cfg.look && c > b.c[i - cfg.look],
                    (true, _) => i >= 2 && c > b.h[i - 2],
                    (false, 2) => c > o,
                    (false, 3) => c > o && if ex.e3_true { b.h[i] > b.th[i - 1] } else { b.h[i] > b.c[i - 1] },
                    (false, _) => i >= cfg.look && c > b.c[i - cfg.look],
                }
            }
        };
        if ex.entry <= 1 { pending = None; }
        if fill {
            pending = None;
            if i >= from && i < to {
                let px = b.px(b.c[i]);
                let t = Trade { dir: b.s, legs: vec![Leg { i, px, w: 1. }], exit_i: i, exit_px: px, reason: String::new(),
                    entry_kind: "close", signal_i: i, open: false, exits: vec![] };
                pos = Some(Pos { t, stop: stop_of(&s), below: 0, decided: None, wait: None });
            }
        }
    }
    if let Some(p) = pos {
        let mut t = p.t; t.exit_i = to; t.exit_px = x.c[to]; t.signal_i = to; t.reason = "open".into(); t.open = true; out.push(t);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;
    fn rows(c: &[f64]) -> Vec<Row> {
        c.iter().enumerate().map(|(i, &v)| Row { ticker: "T".into(), date: format!("{:04}-01-01", 1900 + i), open: v, high: v + 1., low: v - 1., close: v, volume: None }).collect()
    }
    #[test]
    fn setup_needs_flip_and_equal_close_interrupts() {
        // 5 flat bars (close == close 4 earlier -> prerequisite ">=" holds), then 9 falling closes
        let mut c = vec![100.; 6];
        for k in 1..=10 { c.push(100. - k as f64); }
        let r = rows(&c); let x = Ctx::new(&r, &r, &[]);
        let b = Bars::new(&x, r.len() - 1, false);
        let s = setups(&b, 4);
        assert_eq!(s[6], 1); assert_eq!(s[14], 9);
        let mut c2 = c.clone(); c2[10] = c2[6]; // equal to close four earlier -> interrupted
        let r2 = rows(&c2); let x2 = Ctx::new(&r2, &r2, &[]);
        let s2 = setups(&Bars::new(&x2, r2.len() - 1, false), 4);
        assert_eq!(s2[10], 0);
    }
    #[test]
    fn nsta_countdown_starts_at_day9_and_needs_13_strict_closes() {
        // flat, then a steady decline: each close 1 lower, low = close-1, so close < low 2 bars earlier every day
        let mut c = vec![100.; 6];
        for k in 1..=40 { c.push(100. - k as f64); }
        let r = rows(&c); let x = Ctx::new(&r, &r, &[]);
        let b = Bars::new(&x, r.len() - 1, false); let m = Bars::new(&x, r.len() - 1, true);
        let (own, opp) = (setups(&b, 4), setups(&m, 4));
        let cfg = Cfg { ver: Ver::Nsta, setup_n: 9, look: 4, cd_n: 13, cd_look: 2, mult: 0., q13: 0, term_open: false };
        let s = countdowns(&b, &own, &opp, &cfg);
        // setup day 1 = bar 6, day 9 = bar 14; intersection fails in a steep fall (high = c+1 < lows of days 3+ earlier),
        // so countdown cannot start until intersection: none here.
        assert!(s.is_empty());
        // gentle decline with wide bars: intersection on day 8, countdown day 1 = bar 14, 13 = bar 26
        let r2: Vec<Row> = c.iter().enumerate().map(|(i, &v)| Row { ticker: "T".into(), date: format!("{:04}-01-01", 1900 + i), open: v, high: v + 6., low: v - 1.5, close: v, volume: None }).collect();
        let x2 = Ctx::new(&r2, &r2, &[]);
        let b2 = Bars::new(&x2, r2.len() - 1, false); let m2 = Bars::new(&x2, r2.len() - 1, true);
        let s2 = countdowns(&b2, &setups(&b2, 4), &setups(&m2, 4), &cfg);
        assert_eq!(s2[0], Sig { i: 26, s1: 6 });
    }
}
