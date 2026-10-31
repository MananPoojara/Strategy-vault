//! John F. Carter: "Mastering the Trade" (3rd ed., McGraw-Hill 2018; book id `jcmtt`), with held cards for "Trading the Euro: Box It,
//! Squeeze It, Love It, Leave It" (article, `jcteu`), "My 5-Minute Method for Trading the Mini-Dow for a Living" (`jcym5`), "Online Day
//! Trading Seminar on Grains" (with Hubert Senters, `jcags`) and the "CBOT Trading Lessons WorkBook" (`jccbot`).
//! Reading: claude-continuation-20261020/authors/jcarter/agents/. Live (daily bars): the Squeeze (Ch.11, p.248-254), HOLP/LOHP
//! (Ch.17, p.349-358) and the Propulsion pull-back to the 8-period EMA (Ch.18, p.373-378). The book trades futures, forex and US
//! stocks; indices, ETFs and stocks, US and India, are a declared application.
use crate::connors::{Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "jcmtt";
pub const REVIEWED: &[&str] = &["jcmtt_squeeze", "jcmtt_holp", "jcmtt_propulsion"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const SQUEEZE_NOTE: &str = "[Declared readings, not from the book] TradeStation defaults as the book says (p.248): Bollinger Bands = 20-bar simple average +/- 2 population standard deviations of the close; Keltner Channels = 20-bar simple average of the close +/- 1.5 x the 20-bar simple average of the true range (first bar: high - low); 'Bollinger Bands inside the Keltner Channels' = upper band below the upper channel and lower band above the lower channel (a black dot); the signal = the first bar that is not inside after at least one inside bar (the first gray dot; one black dot is enough, p.253). Momentum index = close - close 12 bars ago (TradeStation Momentum, 12 periods, p.251). Daily bars: the signal is known at the close, the market order fills at the next open; long if momentum > 0, short if < 0, none at exactly 0 (the 'below zero yet ascending' exception, p.250, contradicts the zero-line rule of p.248/252 and is the option asc = 1). Exit at the next open after the first lower momentum high (long: momentum below the previous bar's; short: above, p.253-254). Protective stop: the book's per-market points/dollars (p.250) do not carry across instruments; its own fallback 'place the 14-period average true range (ATR) on your chart, double the current value, and use that as your stop' (p.249) is the default: entry -/+ 2 x ATR(14, simple) at the signal bar, resting from the bar after entry, filled at the level or at a gapped open; stop_atr = 0 = no stop. Stops are not trailed (p.251). One position at a time; flat at the window start.";
const HOLP_NOTE: &str = "[Declared readings, not from the book] 'Making new 20-day highs' = the bar's high is the highest high of the last 20 bars including it (lows mirrored); that bar is the high bar, and a later higher high replaces it ('the next bar goes even higher, then this new, higher bar becomes the high bar', p.352). Short at the close of the first bar that closes below the high bar's low; long at the close of the first close above the low bar's high (examples p.352-358 enter at the close). Initial stop = the high (low) of the high (low) bar; from the third bar of the trade (the entry bar is the first) the two-bar trailing stop = the highest high (lowest low) of the two previous bars. Exits are on the close: a close beyond the stop level ('I'm looking for a close above these levels as a signal to exit', p.355), filled at that close. The 'held on the stop bar' remark of p.350 rule 5 refers to a chart and is not used. After an exit a signal on the same close can open the opposite side (p.353). A used high/low bar is cleared; one position at a time; flat at the window start.";
const PROP_NOTE: &str = "[Declared readings, not from the book] EMA = TradeStation XAverage (factor 2/(n+1), seeded with the first close). Long set-up on the close: 8-EMA above the 21-EMA and the close above the 8-EMA; the next day a buy limit at that 8-EMA ('pull-back to the 8-period EMA', p.373; 'trailing bid', p.377) fills when the low reaches it, at the level or at a lower open. Initial stop = the lower of the 21-EMA (set-up close) and entry x (1 - 4%) ('the 21-period EMA or 4 percent of the stock price, whichever is greater', p.373); target = entry x (1 + 8%); once a close is 4% above the entry (the watermark, p.373/377) the stop becomes the 21-EMA of each close, never lowered. Stop and target rest from the bar after entry, the stop fills at the level or a gapped open, the target at the level or a gapped open; if both are reachable in one bar the stop is taken. Shorts mirror. The book's index percentages (1% target, 0.5% stop, p.374) are options; it gives no index watermark. One position at a time; flat at the window start.";

pub fn catalogue() -> Vec<StrategyDef> {
    use Status::*;
    let nt = |id, name, group, book, why, rules| StrategyDef { id, name, group, session: 0, dir: "Long/Short", status: NotTestable(why), needs_volume: false,
        needs_vix: false, needs_series: &[], book, rules, notes: "Held after the 2026-10-25 reading (authors/jcarter/agents).", claim: None };
    let live = |id, name, group, rules, notes| StrategyDef { id, name, group, session: 0, dir: "Long/Short", status: Live, needs_volume: false,
        needs_vix: false, needs_series: &[], book: BOOK, rules, notes, claim: None };
    vec![
        live("jcmtt_squeeze", "The Squeeze: Bollinger Bands back outside the Keltner Channels", "Mastering the Trade · Ch.11 The Squeeze (p.248-254)",
            &["F|\"The quiet periods I'm looking for are identified when the Bollinger Bands narrow in width to the point where they are actually inside of the Keltner Channels ... The trade signal occurs when the Bollinger Bands then move back outside the Keltner Channels. I use a 12-period momentum index oscillator to determine whether to go long or short\" (p.248); \"20 and 1.5 for the Keltner Channels and 20 and 2 for the Bollinger Bands\" (p.248)",
              "E|\"Once the first gray dot appears after a series of black dots, I go long if the histogram is above zero. Once the signal fires, I just place a market order\" (p.249-250)",
              "X|\"My target is based purely on the momentum of the trade. Once the momentum index signal starts to weaken, I get out of the trade at the market\" (p.251); \"As long as it makes higher highs, I stay in the trade. When it makes its first lower high, I will get out\" (p.253)",
              "X|\"if you are unsure of what stop to use, then you can place the 14-period average true range (ATR) on your chart, double the current value, and use that as your stop\" (p.249); \"I don't trail stops\" (p.251)"],
            SQUEEZE_NOTE),
        live("jcmtt_holp", "HOLP and LOHP: close beyond the high bar's low / the low bar's high", "Mastering the Trade · Ch.17 HOLP and LOHP (p.349-358)",
            &["F|\"Identify a trending market, or an individual stock, that is ideally making new 20-day (or period) highs\" (p.349)",
              "E|\"Identify the high bar in the uptrend ... Once I identify the high bar, I will then go short once price action closes below the low of this high bar\" (p.350)",
              "X|\"The initial stop is the high of the high bar. If I am still in the trade on the third day, or period, I will start to use a two-bar trailing stop. I will exit this trade when the current bar closes above the price level represented by the two-bar trailing stop\" (p.350)",
              "E|Buys reversed: \"I want to buy a close above the high of the low day\" (p.352); exit \"the S&Ps close below the low of the previous two bars\" (p.353)"],
            HOLP_NOTE),
        live("jcmtt_propulsion", "Propulsion play: pull-back to the 8-EMA above the 21-EMA, 4% stop, 8% target", "Mastering the Trade · Ch.18 Propulsion Plays (p.373-378)",
            &["F|\"The only indicators I place on the daily chart are an 8- and a 21-period exponential moving average (EMA). For longs, I want to see the 8-period EMA trading above the 21-period EMA\" (p.373)",
              "E|\"The specific setup I'm looking for, once the 8-period EMA has crossed above the 21-period EMA, is a pullback to the 8-period EMA\" (p.373); \"my entry point will be a pullback to the 8-period EMA. The next day I am filled at 79.28 when the market pulls back to the 8-period EMA\" (p.376)",
              "X|\"The initial stop is the 21-period EMA or 4 percent of the stock price, whichever is greater\" (p.373); \"Once I'm up 4 percent on the position (I call this my watermark level), I will move up my stop to the 21-period EMA. I will then use this 21-period EMA as a trailing stop until my target or trailing stop is hit\" (p.373)",
              "X|\"My target is an 8 percent move in the price of the stock from my entry price ... On a daily chart [stock index futures], instead of an 8 percent target, I just use a 1 percent target and a 0.5 percent stop to start\" (p.373-374)"],
            PROP_NOTE),
        nt("jcmtt_gaps", "Opening gap fade to the gap fill", "Mastering the Trade · Ch.7 The Opening Gap", BOOK,
           "intraday futures play: gaps measured in YM/ES points (10 YM / 1 ES minimum, stops of 1.5:1 or 1:1 in points) and graded by pre-market volume, which the vault does not hold",
           &["E|\"With a gap down, when the regular cash market opens at 9:30 a.m. eastern, I buy the YM or ES at the market\" (p.172)", "X|\"My target is the gap fill itself\" (p.172)"]),
        nt("jcmtt_intraday", "Pivots, tick fades, scalper alerts, bricks and the 3:52 play", "Mastering the Trade · Ch.8, 9, 14, 15, 16", BOOK,
           "intraday futures plays on NYSE TICK readings, tick charts and floor pivots with fixed YM/ES point stops; the vault has no intraday TICK and the levels are chosen by the trader",
           &["E|Pivot fades and breakouts, TICK +1000/-1000 fades, 3:52 p.m. TICK-based plays (Ch.8-9, 16)"]),
        nt("jcmtt_waves", "Waves, anchor charts and contributors' chapters", "Mastering the Trade · Ch.10, 12, 13, 19-24", BOOK,
           "chart reading and trader essays: the wave indicator settings, 'reverting to the mean' targets and the contributors' setups are not given as complete rules",
           &[]),
        nt("jcteu_box", "Euro box play and squeeze", "Trading the Euro: Box It, Squeeze It, Love It, Leave It", "jcteu",
           "the box is drawn by hand ('take a horizontal line and start marking highs and lows', two tests of each side) on EUR/USD (no forex data in the vault); the squeeze is the Mastering the Trade rule",
           &["E|\"Place a buy stop order one tick above the high end of the box. Place a sell stop order one tick below the low of the box\"", "X|target = the width of the box"]),
        nt("jcym5_pivots", "5-minute pivot entry strategy on the mini-sized Dow", "My 5-Minute Method for Trading the Mini-Dow", "jcym5",
           "intraday YM futures with fixed 20-point stops and pivot +/- 3 point orders; YM intraday data are not in the vault and level choice ('multi-pivot', midpoints when levels are 40+ points apart) is judged",
           &["E|limit orders at daily/weekly pivots and midpoints +/- 3 points; \"My initial stop is almost always 20 points\""]),
        nt("jcags_grains", "Grain futures day and swing setups (with Hubert Senters)", "Online Day Trading Seminar on Grains", "jcags",
           "corn, wheat and soybean futures (not in the vault) with cent-based stops; the setups repeat Mastering the Trade", &[]),
        nt("jccbot_lessons", "CBOT trading lessons (mini-sized Dow)", "CBOT Trading Lessons WorkBook", "jccbot",
           "introductory futures lessons and the YM pivot method (intraday, fixed points)", &[]),
    ]
}

fn mine(id: &str) -> bool { ["jcmtt_", "jcteu_", "jcym5_", "jcags_", "jccbot_"].iter().any(|p| id.starts_with(p)) }

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    Some(match id {
        "jcmtt_squeeze" => ("jcmtt_squeeze", "Carter, the Squeeze", "BB 20/2 inside KC 20/1.5, momentum 12"),
        "jcmtt_holp" => ("jcmtt_holp", "Carter, HOLP / LOHP", "20-day high/low bar, two-bar trail"),
        "jcmtt_propulsion" => ("jcmtt_propulsion", "Carter, Propulsion play", "8/21 EMA, 4% stop, 8% target"),
        _ if mine(id) => ("jcarter_nt", "Carter (held)", "-"),
        _ => return None,
    })
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    Some(match id {
        "jcmtt_squeeze" => vec![p("bb", "Bollinger / Keltner length", 20., 10., 40., 1.), p("bb_k", "Bollinger standard deviations", 2., 1., 3., 0.25),
            p("kc_k", "Keltner ATR multiple", 1.5, 1., 3., 0.25), p("mom", "Momentum length", 12., 5., 30., 1.),
            p("stop_atr", "Stop: x ATR(14) (book fallback 2; 0 none)", 2., 0., 5., 0.5),
            p("asc", "1 = momentum direction decides when it contradicts the zero line (p.250 exception)", 0., 0., 1., 1.),
            p("side", "Sides (0 both, 1 long only, -1 short only)", 0., -1., 1., 1.)],
        "jcmtt_holp" => vec![p("n", "New high/low period (bars; book 20, 17-18 'fine')", 20., 5., 60., 1.),
            p("trail", "Trailing stop bars", 2., 1., 5., 1.), p("side", "Sides (0 both, 1 long only, -1 short only)", 0., -1., 1., 1.)],
        "jcmtt_propulsion" => vec![p("fast", "Fast EMA", 8., 3., 20., 1.), p("slow", "Slow EMA", 21., 10., 50., 1.),
            p("stop_pct", "Initial stop % (book: stocks 4, index 0.5)", 4., 0.25, 10., 0.25), p("target_pct", "Target % (book: stocks 8, index 1)", 8., 0.5, 20., 0.5),
            p("wm_pct", "Watermark % for the 21-EMA trail (book: stocks 4)", 4., 0.25, 10., 0.25), p("side", "Sides (0 both, 1 long only, -1 short only)", 0., -1., 1., 1.)],
        _ if mine(id) => vec![],
        _ => return None,
    })
}

fn sma(v: &[f64], n: usize) -> Vec<f64> { crate::connors::sma(v, n) }
fn ema(v: &[f64], n: usize) -> Vec<f64> {
    let a = 2. / (n as f64 + 1.);
    let mut out = vec![f64::NAN; v.len()];
    for i in 0..v.len() { out[i] = if i == 0 { v[0] } else { out[i - 1] + a * (v[i] - out[i - 1]) }; }
    out
}
fn true_range(x: &Ctx) -> Vec<f64> {
    (0..x.c.len()).map(|i| if i == 0 { x.h[0] - x.l[0] } else { x.h[i].max(x.c[i - 1]) - x.l[i].min(x.c[i - 1]) }).collect()
}
fn new_trade(dir: f64, i: usize, px: f64, kind: &'static str) -> Trade {
    Trade { dir, legs: vec![Leg { i, px, w: 1. }], exit_i: i, exit_px: px, reason: String::new(), entry_kind: kind, signal_i: i, open: false, exits: vec![] }
}
fn close_out(mut t: Trade, i: usize, px: f64, sig: usize, why: &str) -> Trade { t.exit_i = i; t.exit_px = px; t.signal_i = sig; t.reason = why.into(); t }
/// Resting stop for a position of direction d: the level, or the open if the bar opens beyond it.
fn stop_fill(x: &Ctx, i: usize, d: f64, s: f64) -> Option<f64> {
    if d > 0. { if x.o[i] <= s { Some(x.o[i]) } else if x.l[i] <= s { Some(s) } else { None } }
    else if x.o[i] >= s { Some(x.o[i]) } else if x.h[i] >= s { Some(s) } else { None }
}

fn squeeze(x: &Ctx, from: usize, to: usize, g: &dyn Fn(&str) -> f64) -> Vec<Trade> {
    let (n, mn) = (g("bb") as usize, g("mom") as usize);
    let (bk, kk, sa, asc, side) = (g("bb_k"), g("kc_k"), g("stop_atr"), g("asc") == 1., g("side"));
    let c = &x.c;
    let m = sma(c, n);
    let tr = true_range(x);
    let atr_n = sma(&tr, n);
    let atr14 = sma(&tr, 14);
    let len = c.len();
    let inside: Vec<bool> = (0..len).map(|i| {
        if !m[i].is_finite() || !atr_n[i].is_finite() { return false; }
        let sd = (c[i + 1 - n..=i].iter().map(|v| (v - m[i]).powi(2)).sum::<f64>() / n as f64).sqrt();
        m[i] + bk * sd < m[i] + kk * atr_n[i] && m[i] - bk * sd > m[i] - kk * atr_n[i]
    }).collect();
    let mom: Vec<f64> = (0..len).map(|i| if i >= mn { c[i] - c[i - mn] } else { f64::NAN }).collect();
    let mut out = vec![];
    let mut cur: Option<(Trade, f64)> = None;
    let mut pend_exit: Option<usize> = None;
    let mut pend_entry: Option<(f64, usize)> = None;
    for i in from..=to {
        if let Some(sig) = pend_exit.take() {
            let (t, _) = cur.take().unwrap();
            out.push(close_out(t, i, x.o[i], sig, "first lower momentum high · next open"));
        }
        if let Some((d, sig)) = pend_entry.take() {
            let mut t = new_trade(d, i, x.o[i], "open"); t.signal_i = sig;
            let st = if sa > 0. && atr14[sig].is_finite() { x.o[i] - d * sa * atr14[sig] } else { f64::NAN };
            cur = Some((t, st)); // the stop rests from the bar after entry
        }
        if let Some((t, st)) = cur.as_ref() {
            if i > t.legs[0].i && st.is_finite() { if let Some(px) = stop_fill(x, i, t.dir, *st) {
                let (t, _) = cur.take().unwrap(); out.push(close_out(t, i, px, i, "ATR stop")); continue;
            } }
            let d = t.dir;
            if i < to && mom[i].is_finite() && mom[i - 1].is_finite() && d * (mom[i] - mom[i - 1]) < 0. { pend_exit = Some(i); }
            continue;
        }
        if i == 0 || i >= to || !(inside[i - 1] && !inside[i]) || !mom[i].is_finite() { continue; }
        let mut d = if mom[i] > 0. { 1. } else if mom[i] < 0. { -1. } else { 0. };
        if asc && i > 0 && mom[i - 1].is_finite() {
            if d < 0. && mom[i] > mom[i - 1] { d = 1.; } else if d > 0. && mom[i] < mom[i - 1] { d = -1.; }
        }
        if d != 0. && (side == 0. || side == d) { pend_entry = Some((d, i)); }
    }
    if let Some((mut t, _)) = cur { t = close_out(t, to, x.c[to], to, "open"); t.open = true; out.push(t); }
    out
}

fn holp(x: &Ctx, from: usize, to: usize, g: &dyn Fn(&str) -> f64) -> Vec<Trade> {
    let (n, k, side) = (g("n") as usize, g("trail") as usize, g("side"));
    let (h, l, c) = (&x.h, &x.l, &x.c);
    let mut hi_bar: Option<usize> = None; // high bar for shorts
    let mut lo_bar: Option<usize> = None; // low bar for longs
    let mut out = vec![];
    let mut cur: Option<(Trade, f64)> = None; // trade, initial stop
    for i in 0..=to {
        if i + 1 >= n {
            let w = i + 1 - n;
            if h[w..=i].iter().all(|&v| h[i] >= v) || hi_bar.is_some_and(|b| h[i] > h[b]) { hi_bar = Some(i); }
            if l[w..=i].iter().all(|&v| l[i] <= v) || lo_bar.is_some_and(|b| l[i] < l[b]) { lo_bar = Some(i); }
        }
        if i < from { continue; }
        if let Some((t, st)) = cur.as_ref() {
            let (d, e) = (t.dir, t.legs[0].i);
            let lvl = if i >= e + 2 {
                let w = i.saturating_sub(k)..i;
                if d > 0. { l[w].iter().cloned().fold(f64::INFINITY, f64::min) } else { h[w].iter().cloned().fold(f64::NEG_INFINITY, f64::max) }
            } else { *st };
            if i > e && d * (c[i] - lvl) < 0. {
                let (t, _) = cur.take().unwrap();
                out.push(close_out(t, i, c[i], i, if i >= e + 2 { "close beyond the two-bar trailing stop" } else { "close beyond the high/low bar" }));
            }
        }
        if cur.is_none() && i < to {
            let short = hi_bar.is_some_and(|b| b < i && c[i] < l[b]);
            let long = lo_bar.is_some_and(|b| b < i && c[i] > h[b]);
            if short && (side == 0. || side == -1.) && !long { let b = hi_bar.take().unwrap(); cur = Some((new_trade(-1., i, c[i], "close"), h[b])); }
            else if long && (side == 0. || side == 1.) && !short { let b = lo_bar.take().unwrap(); cur = Some((new_trade(1., i, c[i], "close"), l[b])); }
        }
        if hi_bar.is_some_and(|b| b < i && c[i] < l[b]) { hi_bar = None; }
        if lo_bar.is_some_and(|b| b < i && c[i] > h[b]) { lo_bar = None; }
    }
    if let Some((mut t, _)) = cur { t = close_out(t, to, c[to], to, "open"); t.open = true; out.push(t); }
    out
}

fn propulsion(x: &Ctx, from: usize, to: usize, g: &dyn Fn(&str) -> f64) -> Vec<Trade> {
    let (f, s) = (ema(&x.c, g("fast") as usize), ema(&x.c, g("slow") as usize));
    let (sp, tp, wp, side) = (g("stop_pct") / 100., g("target_pct") / 100., g("wm_pct") / 100., g("side"));
    let mut out = vec![];
    // trade, stop, target, watermark reached
    let mut cur: Option<(Trade, f64, f64, bool)> = None;
    for i in from..=to {
        if let Some((t, st, tg, wm)) = cur.as_mut() {
            let (d, e) = (t.dir, t.legs[0].i);
            let px = t.legs[0].px;
            if i > e {
                if let Some(q) = stop_fill(x, i, d, *st) {
                    let (t, ..) = cur.take().unwrap(); out.push(close_out(t, i, q, i, "stop")); continue;
                }
                let hit = if d > 0. { if x.o[i] >= *tg { Some(x.o[i]) } else if x.h[i] >= *tg { Some(*tg) } else { None } }
                          else if x.o[i] <= *tg { Some(x.o[i]) } else if x.l[i] <= *tg { Some(*tg) } else { None };
                if let Some(q) = hit { let (t, ..) = cur.take().unwrap(); out.push(close_out(t, i, q, i, "target")); continue; }
            }
            if !*wm && d * (x.c[i] - px * (1. + d * wp)) >= 0. { *wm = true; }
            if *wm { *st = if d > 0. { st.max(s[i]) } else { st.min(s[i]) }; }
            continue;
        }
        if i == 0 || i == from { continue; }
        let k = i - 1; // set-up on the previous close, limit order today
        let d = if f[k] > s[k] && x.c[k] > f[k] { 1. } else if f[k] < s[k] && x.c[k] < f[k] { -1. } else { 0. };
        if d == 0. || !(side == 0. || side == d) { continue; }
        let bid = f[k];
        let fill = if d > 0. { if x.o[i] <= bid { Some(x.o[i]) } else if x.l[i] <= bid { Some(bid) } else { None } }
                   else if x.o[i] >= bid { Some(x.o[i]) } else if x.h[i] >= bid { Some(bid) } else { None };
        let Some(px) = fill else { continue };
        let pct = px * (1. - d * sp);
        let st = if d > 0. { s[k].min(pct) } else { s[k].max(pct) };
        let mut t = new_trade(d, i, px, "limit"); t.signal_i = k;
        let mut wm = false;
        if d * (x.c[i] - px * (1. + d * wp)) >= 0. { wm = true; }
        let st = if wm { if d > 0. { st.max(s[i]) } else { st.min(s[i]) } } else { st };
        cur = Some((t, st, px * (1. + d * tp), wm));
    }
    if let Some((mut t, ..)) = cur { t = close_out(t, to, x.c[to], to, "open"); t.open = true; out.push(t); }
    out
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !REVIEWED.contains(&id) { return if mine(id) { Some(vec![]) } else { None }; }
    let schema = params(id)?;
    if from > to || to >= x.c.len() { return None; }
    for k in p.keys() { if !schema.iter().any(|d| d.key == k) { return None; } }
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).unwrap().def);
    Some(match id {
        "jcmtt_squeeze" => squeeze(x, from, to, &g),
        "jcmtt_holp" => holp(x, from, to, &g),
        _ => propulsion(x, from, to, &g),
    })
}

// Instrument admission: Carter 2026-10-25 block, moved from instrument_policy.rs on 31 Oct 2026 (lane ST).
// Admission only for the module REVIEWED lists (authors/jcarter/admission/SOURCE-CONTRACT.md); kept in one block to merge.
// Runs on actual indices, ETFs and stocks, US and India (index_etf_or_stock, nifty50 basket, declared application).
pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book==crate::books::jcarter::BOOK && crate::books::jcarter::reviewed(def.id),
    label: "John Carter traded the Squeeze, HOLP/LOHP and Propulsion plays on stock-index futures, forex and US stocks; runs on indices, ETFs and single stocks (US and India) are a declared application.",
    basis: "Mastering the Trade, 3rd ed. (McGraw-Hill 2018): The Squeeze Ch.11 p.248-254, HOLP and LOHP Ch.17 p.349-358, Propulsion Plays Ch.18 p.373-378. Index/ETF/stock runs are a declared application.",
};
