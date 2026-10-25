//! Michael Bryant (Breakout Futures, breakoutfutures.com), "User's Guide: MiniMax Trading Systems" (MiniMax ESL,
//! MiniMax NQL, MiniMax S; 2001, test 5/1/98-6/8/01) and "User's Guide: MiniMax II System" v2 (2002, test
//! 1/1/98-6/25/02). Book id `bfmm`. Source (read in place, copy pending): research drive "USB HDD 1/TradeStation
//! Systems & Indicators/MiniMax I & II for TS,TS8, Excel + Manuals, $525 (breakoutfutures.com)/v1/MiniMaxUG.pdf"
//! (ch.2 "System Rules" pp.10-11, Appendix pp.37-38) and ".../v2/MiniMaxIIv2UG.pdf" (ch.1 p.4 parameter table,
//! ch.2 "System Rules" pp.10-11).
//! Daily E-mini S&P 500 / Nasdaq 100 futures in the book; here run on the index (instrument swap, one unit,
//! percentage returns). Orders are next-day stop entries computed at the close.
//! Conventions the guides do not state (class B, neutral): a stop fills at the stop price, or at the open when the
//! open is already through it; a limit target fills at the target or the better open; when a stop and a target could
//! both fill on one bar the stop is taken first; one position at a time per direction (TradeStation default).
//! Entry-day stop (class C): the guide says "If filled, place a money management stop", but a daily bar cannot tell
//! whether the low came before or after the entry. Default `same_bar` = 0: exit orders work from the bar after entry,
//! as in TradeStation daily backtests; it reproduces the printed figures (ESL 94 trades 47.9% vs book 87 / 54.0%;
//! MiniMax II ES 231 / 53.7% vs 242 / 47.9%), while `same_bar` = 1 (stop counted on the entry bar) gives 101 / 32.7% and
//! 102 / 14.7%. MiniMax II setups stay TRUE until filled ("Cancel the setup conditions once the trade has been
//! entered"; `latch` = 1): one-day orders (`latch` = 0) give 100 trades against the book's 242.
//! DMI+/DMI-/ADX follow the Appendix (simple N-bar sums of DM and true range; ADX = N-bar simple average of DMI),
//! not Wilder smoothing. "Average range" = simple average of high - low; "average true range" = simple average of
//! true range (TradeStation AvgTrueRange).
use crate::connors::{claim, sma, true_range, Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "bfmm";
/// Admitted ids (authors/bryant/admission/SOURCE-CONTRACT.md); bfmm_s_nq stays gate-withheld.
pub const REVIEWED: &[&str] = &["bfmm_esl", "bfmm_nql", "bfmm_s_es", "bfmm_mm2_es", "bfmm_mm2_nq"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const NOTE1: &str = "MiniMax (2001) on E-mini futures, $75 per trade deducted; here run on the index (S&P 500 for ES, Nasdaq 100 for NQ) as an instrument swap with percentage returns, so only trade counts and win rates are comparable. Day-session data in the book. DMI/ADX per the guide's Appendix (simple 14-bar sums, ADX = 14-bar average of DMI). Stops fill at the stop or the worse open; exits work from the bar after entry (same_bar = 1 also tests the money-management stop on the entry bar, which does not reproduce the book).";
const NOTE2: &str = "MiniMax II (2002) on E-mini futures, $75 per trade deducted; here run on the index (instrument swap, percentage returns). Long and short sides in one strategy; an opposite entry stop that fills while in a position reverses it at the fill price (TradeStation stop-and-reverse). When a stop and the profit target could both fill on one bar the stop is taken first (conservative). The trailing stop starts at the entry day's close; the target is re-set at each close.";

const R_ESL: &[&str] = &[
    "F|MiniMax ESL rule 1: \"If you are flat the market or the current trade will exit on today's close, AND\" (MiniMax UG p.10)",
    "F|rule 2: \"DMI+ < DMI- or today's close is below the close of 5 days ago, AND\" rule 3: \"ADX > 20\"; \"DMI+, DMI-, and ADX are evaluated using a period of 14 days\" (p.10); formulas: Appendix pp.37-38 (N-bar sums of DM over N-bar sum of true range; ADX = N-bar average of DMI)",
    "E|\"Buy tomorrow at today's close plus 0.7 times today's true range. Limit the amount of the breakout from today's close to no more than 1.5 times the 14-day average range and to no less than 0.5 times the 14-day average range.\" (p.10)",
    "X|\"If filled, place a money management stop at 0.75 times the 10-day average range below the entry.\" (p.10)",
    "X|\"If the market closes at the entry price or above, and DMI+ > DMI-, then trail a stop at the close minus 1.5 times today's true range. Reset the stop each day in this manner provided the new stop is higher than the prior level.\" (p.10)",
    "X|\"If DMI+ < DMI-, then (1) remove the trailing stop, and (2) exit market-on-close provided the open is above the entry price and it is not the day of entry.\" (p.10); the EasyLanguage exits MOC only if DMI- > DMI+ at that day's close (p.25)",
];
const R_NQL: &[&str] = &[
    "E|MiniMax NQL: \"If today's close is below the close of 2 days ago, then buy tomorrow at today's close plus 0.65 times today's true range. Limit the amount of the breakout from today's close to no more than 1.5 times the 14-day average true range and to no less than 0.5 times the 14-day average true range.\" (MiniMax UG p.10)",
    "X|\"If filled, place a money management stop at 0.75 times the 10-day average range below the entry.\" (p.10)",
    "X|\"If the market closes at the entry price or above, then trail a stop at the close minus 0.75 times today's range. Reset the stop each day in this manner provided the new stop is higher than the prior level.\" (p.11)",
];
const R_S: &[&str] = &[
    "F|MiniMax S: \"1. If DMI- > DMI+, AND 2. today's close is greater than yesterday's close, AND 3. yesterday's close is greater than the close of the day before yesterday, THEN\" (MiniMax UG p.11)",
    "E|\"Sell tomorrow at today's close minus EntFr times the 10-day average range.\" (p.11)",
    "X|\"If filled, place a money management stop at MMFr times the 10-day average range above the entry.\" (p.11)",
    "X|\"If it has been three or more days since entry or the market has moved in your favor by more than twice the 10-day average range, then trail a stop at the close plus TrailX times the 10-day average range. Reset the stop each day in this manner provided the new stop is lower than the prior level.\" (p.11). Read: days since entry = bars after the entry bar; 'moved in your favor' = entry - close (favor_mode 1 = entry - low)",
    "F|\"For the E-mini S&P 500 futures ... EntFr = 0.75, MMFr = 1.0, TrailX = 0.5. For the E-mini Nasdaq 100 futures ... EntFr = 0.65, MMFr = 0.5, TrailX = 0.5.\" (p.11)",
];
const R_MM2: &[&str] = &[
    "F|Trend filter (MiniMax II UG p.10 note 1): \"While the trend is up, the system keeps track of the highest close. When the market closes below the highest close by more than TrendFr times the 10-day average true range, the trend reverses to down. While the trend is down, the system keeps track of the lowest close. When the market closes above the lowest close by more than TrendFr times the 10-day average true range, the trend reverses to up. To get the process started, the trend on the first bar is assumed to be up.\"",
    "F|Long setup: \"1. If you are not already long, AND 2. The low is below the previous low and the close is below the close of two bars ago, AND 3. The trend ... on the previous bar is up\" (p.10)",
    "E|\"buy tomorrow at today's close plus EntryFrL times today's true range. Limit the amount of the breakout from today's close to no more than 1.5 times the 10-day average true range and to no less than 0.5 times the 10-day average true range.\" (p.10)",
    "X|\"If filled, place a money management stop at ExitFrL times the 10-day average true range below the entry.\" \"Trail a stop at the close minus ExitFrL times today's true range. Reset the stop each day in this manner provided the new stop is above the prior level.\" (p.10)",
    "X|\"Set a profit target equal to either TargFr1 or TargFr2 times the 10-day average true range above the entry price. If the trend is up ... use TargFr2. If the trend is down, use TargFr1. Recalculate the profit target in this manner each day. If the trend is down, exit on the close on the first day for which the trade is profitable at the open of the regular trading session.\" (p.10)",
    "E|Short trades mirror (p.11): setup \"not already short\", high above the previous high and close above the close of two bars ago, trend on the previous bar down; sell at close - EntryFrS x TR (clamped 0.5-1.5 x ATR10); MM stop ExitFrS x ATR10 above; trail close + ExitFrS x TR; target TargFr2 (trend down) / TargFr1 (trend up) x ATR10 below; trend up -> exit on the close of the first day profitable at the open",
    "F|Parameters (p.4): E-mini S&P 500 EntryFrL 0.3, EntryFrS 0.7, ExitFrL 0.5, ExitFrS 0.6, TrendFr 1.0, TargFr1 1.0, TargFr2 2.0; E-mini Nasdaq 100 EntryFrL 0.6, EntryFrS 0.3, ExitFrL 0.4, ExitFrS 0.8, TrendFr 1.0, TargFr1 1.0, TargFr2 2.0 (RiskPer = position sizing only, not used)",
];

pub fn catalogue() -> Vec<StrategyDef> {
    let d = |id, name, group, dir, rules, notes, claim| StrategyDef { id, name, group, session: 0, dir, status: Status::Live,
        needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK, rules, notes, claim };
    vec![
        d("bfmm_esl", "MiniMax ESL (E-mini S&P long)", "MiniMax (2001) ch.2", "Long", R_ESL, NOTE1,
          claim("87", "54.0% (47/87)", "$201.90 per contract (net $75)", "3 bars winners", "5/1/98-6/8/01, E-mini S&P 500 day session")),
        d("bfmm_nql", "MiniMax NQL (E-mini Nasdaq long)", "MiniMax (2001) ch.2", "Long", R_NQL, NOTE1,
          claim("91", "53.85%", "$619.55 per contract (net $75)", "5 bars winners", "5/1/98-6/8/01, E-mini Nasdaq 100")),
        d("bfmm_s_es", "MiniMax S short (E-mini S&P parameters)", "MiniMax (2001) ch.2", "Short", R_S, NOTE1,
          claim("22", "59.1% (13/22)", "$887 per contract (net $75)", "5 bars winners", "5/1/98-6/8/01, E-mini S&P 500")),
        d("bfmm_s_nq", "MiniMax S short (E-mini Nasdaq parameters)", "MiniMax (2001) ch.2", "Short", R_S, NOTE1,
          claim("14", "64.29%", "$3,223.14 per contract (net $75)", "-", "5/1/98-6/8/01, E-mini Nasdaq 100")),
        d("bfmm_mm2_es", "MiniMax II (E-mini S&P parameters)", "MiniMax II (2002) ch.2", "Long/Short", R_MM2, NOTE2,
          claim("242 (133 long 44.36%, 109 short 52.29%)", "47.93%", "$175.41 per contract (net $75)", "2 bars winners, 1 bar losers", "1/1/98-6/25/02, E-mini S&P 500")),
        d("bfmm_mm2_nq", "MiniMax II (E-mini Nasdaq parameters)", "MiniMax II (2002) ch.2", "Long/Short", R_MM2, NOTE2,
          claim("161 (82 long 51.22%, 79 short 53.16%)", "52.17%", "-", "-", "6/22/99-6/25/02, E-mini Nasdaq 100")),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    Some(match id {
        "bfmm_esl" => ("bfmm_mm1", "MiniMax (Bryant)", "ESL long"),
        "bfmm_nql" => ("bfmm_mm1", "MiniMax (Bryant)", "NQL long"),
        "bfmm_s_es" => ("bfmm_mm1", "MiniMax (Bryant)", "S short, ES parameters"),
        "bfmm_s_nq" => ("bfmm_mm1", "MiniMax (Bryant)", "S short, NQ parameters"),
        "bfmm_mm2_es" => ("bfmm_mm2", "MiniMax II (Bryant)", "ES parameters"),
        "bfmm_mm2_nq" => ("bfmm_mm2", "MiniMax II (Bryant)", "NQ parameters"),
        _ => return None,
    })
}

fn sb() -> ParamDef { ParamDef { key: "same_bar", label: "Entry-day low through the money-management stop counts (0 = no: stops work from the bar after entry, as in the TradeStation daily backtests that produced the book figures; 1 = yes, conservative)", def: 0., min: 0., max: 1., step: 1. } }

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    Some(match id {
        "bfmm_esl" => vec![
            p("look", "Close below the close of N days ago", 5., 2., 10., 1.), p("adx_min", "ADX above", 20., 10., 40., 1.),
            p("dmi_n", "DMI/ADX period", 14., 7., 28., 1.), p("ent", "Entry: x today's true range", 0.7, 0.3, 1.5, 0.05),
            p("rng_n", "Average range days (breakout limits)", 14., 5., 30., 1.), p("lo", "Breakout at least x average range", 0.5, 0., 1., 0.1),
            p("hi", "Breakout at most x average range", 1.5, 1., 3., 0.1), p("mm", "MM stop: x 10-day average range", 0.75, 0.25, 2., 0.05),
            p("trail", "Trail: close - x today's true range", 1.5, 0.5, 3., 0.1), sb()],
        "bfmm_nql" => vec![
            p("look", "Close below the close of N days ago", 2., 1., 10., 1.), p("ent", "Entry: x today's true range", 0.65, 0.3, 1.5, 0.05),
            p("rng_n", "Average true range days (breakout limits)", 14., 5., 30., 1.), p("lo", "Breakout at least x ATR", 0.5, 0., 1., 0.1),
            p("hi", "Breakout at most x ATR", 1.5, 1., 3., 0.1), p("mm", "MM stop: x 10-day average range", 0.75, 0.25, 2., 0.05),
            p("trail", "Trail: close - x today's range", 0.75, 0.25, 2., 0.05), sb()],
        "bfmm_s_es" | "bfmm_s_nq" => {
            let es = id == "bfmm_s_es";
            vec![p("dmi_n", "DMI period", 14., 7., 28., 1.), p("entfr", "EntFr (x 10-day average range below the close)", if es { 0.75 } else { 0.65 }, 0.2, 1.5, 0.05),
                 p("mmfr", "MMFr (x 10-day average range above entry)", if es { 1.0 } else { 0.5 }, 0.25, 2., 0.05),
                 p("trailx", "TrailX (x 10-day average range above the close)", 0.5, 0.25, 2., 0.05),
                 p("days", "Trail after this many days since entry", 3., 1., 10., 1.), p("favor", "... or a favourable move of x 10-day average range", 2., 0.5, 5., 0.25),
                 p("favor_mode", "Favourable move measured by: 0 = close, 1 = low", 0., 0., 1., 1.), sb()]
        }
        "bfmm_mm2_es" | "bfmm_mm2_nq" => {
            let es = id == "bfmm_mm2_es";
            vec![p("entry_fr_l", "EntryFrL", if es { 0.3 } else { 0.6 }, 0.1, 1.5, 0.05), p("entry_fr_s", "EntryFrS", if es { 0.7 } else { 0.3 }, 0.1, 1.5, 0.05),
                 p("exit_fr_l", "ExitFrL", if es { 0.5 } else { 0.4 }, 0.1, 1.5, 0.05), p("exit_fr_s", "ExitFrS", if es { 0.6 } else { 0.8 }, 0.1, 1.5, 0.05),
                 p("trend_fr", "TrendFr", 1.0, 0.25, 3., 0.25), p("targ_fr1", "TargFr1 (counter-trend target)", 1.0, 0.25, 3., 0.25),
                 p("targ_fr2", "TargFr2 (with-trend target)", 2.0, 0.5, 5., 0.25),
                 p("latch", "Setup stays TRUE until the trade is entered (1, 'Cancel the setup conditions once the trade has been entered'); 0 = one-day order", 1., 0., 1., 1.), sb()]
        }
        _ => return None,
    })
}

/// Appendix pp.37-38: DMI+ = 100 sum(DM+, n) / sum(TR, n), DMI- likewise, DMI(DX) = 100 |DMI- - DMI+| / (DMI- + DMI+),
/// ADX = n-bar simple average of DX. Returns (DMI+, DMI-, ADX).
pub(crate) fn dmi_sum(h: &[f64], l: &[f64], tr: &[f64], n: usize) -> (Vec<f64>, Vec<f64>, Vec<f64>) {
    let len = h.len();
    let (mut pdm, mut mdm) = (vec![f64::NAN; len], vec![f64::NAN; len]);
    let mut trv = vec![f64::NAN; len];
    for i in 1..len {
        let mut p = if h[i] > h[i - 1] { h[i] - h[i - 1] } else { 0. };
        let mut m = if l[i] < l[i - 1] { l[i - 1] - l[i] } else { 0. };
        if p > m { m = 0. }
        if m > p { p = 0. }
        pdm[i] = p;
        mdm[i] = m;
        trv[i] = tr[i];
    }
    let (sp, sm, st) = (sma(&pdm, n), sma(&mdm, n), sma(&trv, n));
    let mut dp = vec![f64::NAN; len];
    let mut dm = vec![f64::NAN; len];
    let mut dx = vec![f64::NAN; len];
    for i in 0..len {
        if st[i] > 0. {
            dp[i] = 100. * sp[i] / st[i];
            dm[i] = 100. * sm[i] / st[i];
            let s = dp[i] + dm[i];
            dx[i] = if s > 0. { 100. * (dm[i] - dp[i]).abs() / s } else { 0. };
        }
    }
    (dp, dm, sma(&dx, n))
}

fn new_trade(dir: f64, i: usize, px: f64, sig: usize) -> Trade {
    Trade { dir, legs: vec![Leg { i, px, w: 1.0 }], exit_i: i, exit_px: px, reason: String::new(), entry_kind: "stop", signal_i: sig, open: false, exits: vec![] }
}
fn close_t(mut t: Trade, i: usize, px: f64, why: &str) -> Trade {
    t.exit_i = i; t.exit_px = px; t.signal_i = i; t.reason = why.into(); t
}

/// Open position state shared by the MiniMax systems.
struct Pos { t: Trade, entry: f64, ei: usize, mm: f64, trail: Option<f64>, target: Option<f64> }

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    let defs = params(id)?;
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| defs.iter().find(|d| d.key == k).map(|d| d.def).unwrap_or(f64::NAN));
    let to = to.min(x.c.len().saturating_sub(1));
    let (o, h, l, c) = (&x.o, &x.h, &x.l, &x.c);
    let tr = true_range(x.r);
    let rng: Vec<f64> = (0..c.len()).map(|i| h[i] - l[i]).collect();
    let same_bar = g("same_bar") >= 0.5;
    let mut out = Vec::new();
    let clamp = |v: f64, a: f64, lo: f64, hi: f64| v.max(lo * a).min(hi * a);
    match id {
        "bfmm_esl" | "bfmm_nql" | "bfmm_s_es" | "bfmm_s_nq" => {
            let esl = id == "bfmm_esl";
            let short = id.starts_with("bfmm_s_");
            let dir = if short { -1.0 } else { 1.0 };
            let dmi_n = if id == "bfmm_nql" { 14 } else { g("dmi_n").round().max(2.) as usize };
            let (dp, dm, adx) = dmi_sum(h, l, &tr, dmi_n);
            let ar10 = sma(&rng, 10);
            let limits = if short { vec![] } else {
                let n = g("rng_n").round().max(2.) as usize;
                if esl { sma(&rng, n) } else { sma(&tr, n) }
            };
            let mut pos: Option<Pos> = None;
            // pending entry for bar t: (stop price, money-management distance, signal bar)
            let mut pend: Option<(f64, f64, usize)> = None;
            for t in from.max(11)..=to {
                // 1) entry
                if pos.is_none() {
                    if let Some((px, mmd, sig)) = pend.take() {
                        let hit = if short { l[t] <= px } else { h[t] >= px };
                        if hit {
                            let fill = if short { o[t].min(px) } else { o[t].max(px) };
                            let mm = fill - dir * mmd;
                            pos = Some(Pos { t: new_trade(dir, t, fill, sig), entry: fill, ei: t, mm, trail: None, target: None });
                            if same_bar && (if short { h[t] >= mm } else { l[t] <= mm }) {
                                let q = pos.take().unwrap();
                                out.push(close_t(q.t, t, mm, "money management stop (entry day)"));
                            }
                        }
                    }
                } else {
                    pend = None;
                }
                // 2) exits on later bars
                let mut exited = false;
                if let Some(q) = pos.as_mut().filter(|q| q.ei < t) {
                    let stop = match q.trail { Some(s) => if short { q.mm.min(s) } else { q.mm.max(s) }, None => q.mm };
                    let why = if q.trail.map_or(false, |s| if short { s < q.mm } else { s > q.mm }) { "trailing stop" } else { "money management stop" };
                    if (short && o[t] >= stop) || (!short && o[t] <= stop) {
                        out.push(close_t(pos.take().unwrap().t, t, o[t], why)); exited = true;
                    } else if (short && h[t] >= stop) || (!short && l[t] <= stop) {
                        out.push(close_t(pos.take().unwrap().t, t, stop, why)); exited = true;
                    } else if esl && dm[t] > dp[t] && o[t] > q.entry {
                        out.push(close_t(pos.take().unwrap().t, t, c[t], "market-on-close: DMI+ < DMI- and open above entry")); exited = true;
                    }
                }
                let _ = exited;
                // 3) end-of-day trailing stop update
                if let Some(q) = pos.as_mut() {
                    if esl {
                        if c[t] >= q.entry && dp[t] > dm[t] {
                            let s = c[t] - g("trail") * tr[t];
                            q.trail = Some(q.trail.map_or(s, |v| v.max(s)));
                        } else if dp[t] < dm[t] {
                            q.trail = None;
                        }
                    } else if id == "bfmm_nql" {
                        if c[t] >= q.entry {
                            let s = c[t] - g("trail") * rng[t];
                            q.trail = Some(q.trail.map_or(s, |v| v.max(s)));
                        }
                    } else {
                        let fav = if g("favor_mode") >= 0.5 { q.entry - l[t] } else { q.entry - c[t] };
                        if t - q.ei >= g("days").round() as usize || fav > g("favor") * ar10[t] {
                            let s = c[t] + g("trailx") * ar10[t];
                            q.trail = Some(q.trail.map_or(s, |v| v.min(s)));
                        }
                    }
                }
                // 4) setup at today's close for tomorrow
                if pos.is_none() && t < to {
                    let sig = if esl {
                        (dp[t] < dm[t] || c[t] < c[t - g("look").round() as usize]) && adx[t] > g("adx_min")
                    } else if id == "bfmm_nql" {
                        c[t] < c[t - g("look").round().max(1.) as usize]
                    } else {
                        dm[t] > dp[t] && c[t] > c[t - 1] && c[t - 1] > c[t - 2]
                    };
                    if sig {
                        let px = if short { c[t] - g("entfr") * ar10[t] } else { c[t] + clamp(g("ent") * tr[t], limits[t], g("lo"), g("hi")) };
                        let mmd = if short { g("mmfr") * ar10[t] } else { g("mm") * ar10[t] };
                        if px.is_finite() && mmd.is_finite() { pend = Some((px, mmd, t)); }
                    }
                }
            }
            if let Some(q) = pos {
                let mut t = close_t(q.t, to, c[to], "open");
                t.open = true;
                out.push(t);
            }
        }
        "bfmm_mm2_es" | "bfmm_mm2_nq" => {
            let atr10 = sma(&tr, 10);
            let tf = g("trend_fr");
            // trend[i]: true = up
            let mut trend = vec![true; c.len()];
            let (mut up, mut ext) = (true, c[0]);
            for i in 0..c.len() {
                if up {
                    ext = ext.max(c[i]);
                    if atr10[i].is_finite() && c[i] < ext - tf * atr10[i] { up = false; ext = c[i]; }
                } else {
                    ext = ext.min(c[i]);
                    if atr10[i].is_finite() && c[i] > ext + tf * atr10[i] { up = true; ext = c[i]; }
                }
                trend[i] = up;
            }
            let (efl, efs, xfl, xfs, t1, t2) = (g("entry_fr_l"), g("entry_fr_s"), g("exit_fr_l"), g("exit_fr_s"), g("targ_fr1"), g("targ_fr2"));
            let mut pos: Option<Pos> = None;
            let latch = g("latch") >= 0.5;
            // setup state: Some(signal bar) while the long / short setup conditions are TRUE
            let (mut set_l, mut set_s): (Option<usize>, Option<usize>) = (None, None);
            // orders for bar t: (dir, stop price, mm distance, signal bar)
            let mut pend: Vec<(f64, f64, f64, usize)> = vec![];
            for t in from.max(2)..=to {
                let pe = std::mem::take(&mut pend);
                // 1) exits of a position held from before today (adverse orders first)
                if let Some(q) = pos.as_mut().filter(|q| q.ei < t) {
                    let d = q.t.dir;
                    let stop = match q.trail { Some(s) => if d > 0. { q.mm.max(s) } else { q.mm.min(s) }, None => q.mm };
                    // an opposite entry stop on the same side as this position's stop acts as a reversal
                    let rev = pe.iter().find(|e| e.0 != d).map(|e| e.1);
                    let eff = match rev { Some(r) => if d > 0. { stop.max(r) } else { stop.min(r) }, None => stop };
                    let why = if rev.map_or(false, |r| r == eff && r != stop) { "reversed by opposite entry stop" } else if q.trail.map_or(false, |s| s == stop && s != q.mm) { "trailing stop" } else { "money management stop" };
                    let tgt = q.target.unwrap_or(f64::NAN);
                    if (d > 0. && o[t] <= eff) || (d < 0. && o[t] >= eff) {
                        out.push(close_t(pos.take().unwrap().t, t, o[t], why));
                    } else if (d > 0. && l[t] <= eff) || (d < 0. && h[t] >= eff) {
                        out.push(close_t(pos.take().unwrap().t, t, eff, why));
                    } else if (d > 0. && o[t] >= tgt) || (d < 0. && o[t] <= tgt) {
                        out.push(close_t(pos.take().unwrap().t, t, o[t], "profit target"));
                    } else if (d > 0. && h[t] >= tgt) || (d < 0. && l[t] <= tgt) {
                        out.push(close_t(pos.take().unwrap().t, t, tgt, "profit target"));
                    } else {
                        let counter = if d > 0. { !trend[t] } else { trend[t] };
                        if counter && d * (o[t] - q.entry) > 0. {
                            out.push(close_t(pos.take().unwrap().t, t, c[t], "counter-trend: exit on the close, profitable at the open"));
                        }
                    }
                }
                // 2) entry when flat (a reversal closed the old side at this stop above). If both stops of an outside bar
                //    could fill, the one nearer the open is taken (the open decides when it is already through one).
                if pos.is_none() {
                    let mut hits: Vec<&(f64, f64, f64, usize)> = pe.iter().filter(|e| if e.0 > 0. { h[t] >= e.1 } else { l[t] <= e.1 }).collect();
                    hits.sort_by(|a, b| (a.1 - o[t]).abs().partial_cmp(&(b.1 - o[t]).abs()).unwrap());
                    if let Some(&&(d, px, mmd, sig)) = hits.first() {
                        let fill = if d > 0. { o[t].max(px) } else { o[t].min(px) };
                        let mm = fill - d * mmd;
                        pos = Some(Pos { t: new_trade(d, t, fill, sig), entry: fill, ei: t, mm, trail: None, target: None });
                        if d > 0. { set_l = None } else { set_s = None }
                        if same_bar && (if d > 0. { l[t] <= mm } else { h[t] >= mm }) {
                            out.push(close_t(pos.take().unwrap().t, t, mm, "money management stop (entry day)"));
                        }
                    }
                }
                // 3) end-of-day: trailing stop and target
                if let Some(q) = pos.as_mut() {
                    let d = q.t.dir;
                    let xf = if d > 0. { xfl } else { xfs };
                    let s = c[t] - d * xf * tr[t];
                    q.trail = Some(match q.trail { Some(v) => if d > 0. { v.max(s) } else { v.min(s) }, None => s });
                    let with = if d > 0. { trend[t] } else { !trend[t] };
                    q.target = Some(q.entry + d * (if with { t2 } else { t1 }) * atr10[t]);
                }
                // 4) setups at the close; orders for tomorrow are re-priced from today's close while a setup is TRUE
                let cur = pos.as_ref().map(|q| q.t.dir);
                if !latch { set_l = None; set_s = None; }
                if cur == Some(1.0) { set_l = None }
                if cur == Some(-1.0) { set_s = None }
                if cur != Some(1.0) && l[t] < l[t - 1] && c[t] < c[t - 2] && trend[t - 1] { set_l = Some(t) }
                if cur != Some(-1.0) && h[t] > h[t - 1] && c[t] > c[t - 2] && !trend[t - 1] { set_s = Some(t) }
                if t < to && atr10[t].is_finite() {
                    if let Some(sig) = set_l { pend.push((1.0, c[t] + clamp(efl * tr[t], atr10[t], 0.5, 1.5), xfl * atr10[t], sig)); }
                    if let Some(sig) = set_s { pend.push((-1.0, c[t] - clamp(efs * tr[t], atr10[t], 0.5, 1.5), xfs * atr10[t], sig)); }
                }
            }
            if let Some(q) = pos {
                let mut t = close_t(q.t, to, c[to], "open");
                t.open = true;
                out.push(t);
            }
        }
        _ => return None,
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn dmi_sum_hand_check() {
        // two up bars then a down bar, n = 2
        let h: Vec<f64> = vec![10., 11., 12., 11.];
        let l: Vec<f64> = vec![9., 10., 11., 9.];
        let c: Vec<f64> = vec![9.5, 10.5, 11.5, 9.5];
        let tr: Vec<f64> = (0..4).map(|i| if i == 0 { h[0] - l[0] } else { (h[i] - l[i]).max((h[i] - c[i - 1]).abs()).max((l[i] - c[i - 1]).abs()) }).collect();
        let (dp, dm, _) = dmi_sum(&h, &l, &tr, 2);
        // bar 2: DM+ = 1 + 1, TR = 1.5 + 1.5 -> 66.67, DM- = 0
        assert!((dp[2] - 200. / 3.).abs() < 1e-9 && dm[2] == 0.);
        // bar 3: DM+ (1, 0) DM- (0, 2); TR (1.5, 2.5) -> DMI+ 25, DMI- 50
        assert!((dp[3] - 25.).abs() < 1e-9 && (dm[3] - 50.).abs() < 1e-9);
    }
}
