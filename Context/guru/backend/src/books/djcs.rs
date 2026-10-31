//! David Jennings (The Performance Factory), "Code Snippets & How To" for MetaStock, TradeStation and AIQ (Word file,
//! book id `djcs`; a compilation of mailing-list code by other people, each credited in the file). Reading: lane K2,
//! claude-continuation-20261020/authors/jennings (admission/djcs_ref.py; plain-text export of the .doc).
//! Live: Bob Fulks' percent-from-extreme reversal stops (TradeStation "Other How To" section, inputs BZ 2 / SZ 1) and
//! James Taylor's variation of Bryan Strain's TFS trend-following system (TradeStation section, S&C June 2000), with
//! Strain's original cross logic (commented out in the same code) as an option; the MarketSpace Stock System (MetaStock
//! text run by the msws formula engine); Glenn Wallace's Buy trigger system (MetaStock, PREV state written out); the
//! WealthLab Dead Cat Catcher and CMO(20) / CumDown accumulation scripts. Held: Party Girl I Optimized and the CandleCode
//! system (their only exits are WealthLab auto stops whose fill rules the file does not give). The MarketSpace
//! short-term system (the file says only the other system "works with daily data") is live on weekly bars since lane K9
//! (1 Nov 2026): Metastock Zone.pdf p.545-547 prints the same code as Gil Raff's Timing System for weekly data.
//! Source markets: not stated (generic TradeStation code); indices, ETFs and stocks here are a declared application.
use crate::books::msws::{self, Book, Sys, EXEC_DEFAULT};
use crate::connors::{sma, Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};
use std::sync::OnceLock;

// MarketSpace Stock System, printed MetaStock text; TRUE / FALSE written as 1 / 0 (MetaStock: TRUE = 1, FALSE = 0).
const MS_BUY: &str = r##"((MACD()>Mov(MACD(),45,E) AND C>Mov(C,25,E))=1) AND (Ref(MACD()>Mov(MACD(),45,E) AND C>Mov(C,25,E),-1)=0) AND BarsSince(Ref(((MACD()>Mov(MACD(),45,E) AND C>Mov(C,25,E))=1) AND (Ref(MACD()>Mov(MACD(),45,E) AND C>Mov(C,25,E),-1)=0),-1)) >= BarsSince(Ref(Cross(0.9*Ref(C,-7),C),-1))"##;
const MS_SELL: &str = r##"(Cross(0.9*Ref(C,-7),C) AND BarsSince(((MACD()>Mov(MACD(),45,E) AND C>Mov(C,25,E))=1) AND (Ref(MACD()>Mov(MACD(),45,E) AND C>Mov(C,25,E),-1)=0))<= BarsSince(Ref(Cross(0.9*Ref(C,-7),C),-1)))"##;
// Gil Raff MarketSpace Timing System - Weekly Data (Metastock Zone.pdf p.545-547, posted by Erich Kohlhofer with the daily
// Stock System above): the same text with 9/5/15-week averages and a 4.5% two-week drop. TRUE / FALSE written as 1 / 0.
// Lane K9 (1 Nov 2026), to-code row 151; independent check authors/titles-1030/K9w/k9_ref.py.
const MSW_BUY: &str = r##"((MACD()>Mov(MACD(),9,E) AND C>Mov(C,5,E) AND C>Mov(C,15,E))=1) AND (Ref(MACD()>Mov(MACD(),9,E) AND C>Mov(C,5,E) AND C>Mov(C,15,E),-1)=0) AND BarsSince(Ref( ((MACD()>Mov(MACD(),9,E) AND C>Mov(C,5,E) AND C>Mov(C,15,E))=1) AND (Ref(MACD()>Mov(MACD(),9,E) AND C>Mov(C,5,E) AND C>Mov(C,15,E),-1)=0),-1)) >= BarsSince(Ref(Cross(0.955*Ref(C,-2),C),-1))"##;
const MSW_SELL: &str = r##"Cross(0.955*Ref(C,-2),C) AND BarsSince(((MACD()>Mov(MACD(),9,E) AND C>Mov(C,5,E) AND C>Mov(C,15,E))=1) AND (Ref(MACD()>Mov(MACD(),9,E) AND C>Mov(C,5,E)AND C>Mov(C,15,E),-1)=0)) <= BarsSince(Ref(Cross(0.955*Ref(C,-2),C),-1))"##;
static MS_SYS: &[Sys] = &[Sys { id: "djcs_marketspace", name: "MarketSpace Stock System: MACD above its 45-day average and close above the 25-day, out on a close 10% under the close 7 days earlier",
    group: "Jennings code snippets", family: ("djcs_marketspace", "MarketSpace Stock System"), page: "file TOC p.27-28 'MetaStock Code - MarketSpace Stock System'",
    status: Status::Live, f: [Some(MS_BUY), Some(MS_SELL), None, None], exec: EXEC_DEFAULT,
    notes: "Printed as 'Primary Buy Signal' / 'Primary Sell Signal' (long only) for 'the MarketSpace Stock System which works with daily data'. TRUE/FALSE are written 1/0. The BarsSince tests make each buy the first new MACD/average set-up since the last 10% drop signal and each sell the first such drop since the set-up." },
    Sys { id: "djcs_marketspace_weekly", name: "MarketSpace Timing System (weekly): MACD above its 9-week average and close above the 5- and 15-week, out on a close 4.5% under the close 2 weeks earlier",
    group: "Jennings code snippets", family: ("djcs_marketspace_weekly", "MarketSpace Timing System (weekly)"), page: "file TOC p.27 'MetaStock Code - MarketSpace' (the short-term system); same code in Metastock Zone.pdf p.545-547 'Gil Raff MarketSpace Timing System - Weekly Data'",
    status: Status::Live, f: [Some(MSW_BUY), Some(MSW_SELL), None, None], exec: EXEC_DEFAULT,
    notes: "Gil Raff's MarketSpace Timing System, printed as 'Primary Buy Signal' / 'Primary Sell Signal' (long only). The Jennings file gives no time frame for it (it was held here); the Metastock Zone post of the same code says 'Remember your data must be in weekly format' (to-code row 151). Weekly bars are built from the vault's daily bars: one bar per calendar week (Monday-Sunday) with the week's first open, highest high, lowest low and last close; the week of the window's last day ends on that day. Signals and fills at the weekly close, i.e. the close of the week's last trading day. TRUE/FALSE are written 1/0." }];
const MS_CONV: &str = "One position at a time; a position closes only by its own Close formula (from the bar after a close-filled entry). Indicators: MetaStock definitions as in module msws (MACD() = 0.15 / 0.075 exponential averages of the close, Mov E seeded with the first value, BarsSince undefined before the first occurrence). Source markets not stated; indices, ETFs and stocks are a declared application.";
static MS: Book = Book { id: "djcs", sys: MS_SYS, fml: &[], conv: MS_CONV, cat: OnceLock::new(), par: OnceLock::new() };
/// Admitted ids (authors/jennings/admission/djcs_ref.py (header)).
pub const REVIEWED: &[&str] = &["djcs_pct_reversal", "djcs_tfs", "djcs_marketspace", "djcs_buy_trigger", "djcs_dead_cat", "djcs_cmo_cumdown", "djcs_marketspace_weekly"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const NOTE_P: &str = "[Declared readings] Always in after the first fill, stop and reverse. HHigh = the highest high and LLow = the lowest low since the position last changed (both restart at the high and low of the bar where it changed; the code's zero start value is read as the first bar of the test window, where the run starts flat). While long or flat: sell stop for the next bar at HHigh less SZ%; while short or flat: buy stop at LLow plus BZ%. A stop fills at its level or at the open if the day opens beyond it; the reverse position starts at the same price. While flat, a day that reaches both levels is skipped (order inside the day unknown). On the last bar a stop still closes the trade but opens nothing; a position still open is marked at the last close.";
const NOTE_B: &str = "[Declared readings] MetaStock System Tester on the close (as the code says). Exponential averages as MetaStock computes them (module msws: seeded with the first close, shown from bar n). The PREV state is the position itself: flat at the start of the test window; a buy at the close of the first bar with BuyTrigger above 0.05; BuyPrice = that close; sold at the close of the first later bar whose close is at or above BuyPrice x (1 + that bar's BuyTrigger); no re-entry on the exit bar (BuyPrice is then negative). No stop. A position still open is marked at the last close.";
const NOTE_D: &str = "[Declared readings] WealthLab: CMO = 100 x (sum of up closes - sum of down closes) / (their total) over the last 5 close-to-close changes; ROC = percent change of the close over 5 bars. Signal on a bar's close, buy at the next open; the exit order is placed on the bar BarSave + hold (BarSave = the entry bar) and fills at the next open, so with hold 1 the trade is out at the open two bars after the entry; a new signal can come on the exit bar. The stop-loss and trailing-stop OptVars have no printed default and are off. Short side only by option. A position still open is marked at the last close.";
const NOTE_C: &str = "[Declared readings] WealthLab CumDown(close, 4) = the number of consecutive bars whose close is below the close 4 bars earlier; CMO(close, 20) = 100 x (up - down) / (up + down) over 20 close-to-close changes. Signal on a bar's close, fill at the next open. Each buy adds one unit of a third of the capital (the code buys a fixed 5000 per signal, at most 3 open); a new unit needs 9 bars since the last buy (LastBar >= 9). When CMO(20) closes above 50 every unit is sold at the next open, shown as one trade with its units as legs. The counters start in the test window. A position still open is marked at the last close.";
const NOTE_T: &str = "[Declared readings] VolOsc = +volume on an up day (close above open), -volume on a down day, unchanged on an unchanged day (TradeStation variables keep their value); a missing volume blanks it until the next up or down day. Mid = (highest high + lowest low of the last 50 bars, today included) / 2. Taylor variation (default): close above Mid with the 7-day average of VolOsc above 0 buys (covering a short); close below Mid with the average below 0 sells short (closing a long). Strain's original (option 1): only a close crossing Mid acts - crossing above buys if the average is above 0, else only covers a short; crossing below mirrors. TradeStation 'Buy;' = next bar at the open; a signal on the last bar is not filled; a position still open is marked at the last close.";

pub fn catalogue() -> Vec<StrategyDef> {
    let d = |id: &'static str, name: &'static str, vol: bool, rules: &'static [&'static str], notes: &'static str| StrategyDef { id, name,
        group: "Jennings code snippets", session: 0, dir: "Long/Short", status: Status::Live, needs_volume: vol, needs_vix: false, needs_series: &[],
        book: "djcs", rules, notes, claim: None };
    let mut v = vec![
        d("djcs_pct_reversal", "Percent-from-extreme reversal stops: reverse short 1% under the high since entry, long 2% over the low", false, &[
            "E|Bob Fulks: \"Inputs: BZ(2), SZ(1); ... HHigh = iff(High > HHigh or MP <> MP[1], High, HHigh); LLow = iff(Low < LLow or MP <> MP[1], Low, LLow);\" (TradeStation, 'Highest High' how-to)",
            "X|\"if MP >= 0 then Sell next bar at HHigh * (1 - 0.01 * SZ) stop; if MP <= 0 then Buy next bar at LLow * (1 + 0.01 * BZ) stop;\" - always in, stop and reverse"], NOTE_P),
        d("djcs_tfs", "TFS trend following: close beyond the 50-day mid-channel with the 7-day volume oscillator agreeing, stop and reverse", true, &[
            "F|\"If Close > Open then VolOsc = Volume; If Close < Open then VolOsc = - Volume;\"; mid-channel \"(Highest(High, 50) + Lowest(Low, 50)) / 2\" (TFS, James Taylor, variation of Bryan Strain, S&C June 2000)",
            "E|\"If Close > ((Highest(High, 50) + Lowest(Low, 50)) / 2) AND Average(VolOsc, 7) > 0 then begin Buy; ExitShort; End;\"",
            "X|\"If Close < ((Highest(High, 50) + Lowest(Low, 50)) / 2) AND Average(VolOsc, 7) < 0 then begin Sell; ExitLong; End;\" (\"This variation appears to be more profitable than the original\"; the original cross logic is an option)"], NOTE_T),
        StrategyDef { dir: "Long", ..d("djcs_buy_trigger", "Buy trigger (Glenn Wallace): buy when the 10-day EMA is 5% over the 30-day, sell when the close gains that same percentage", false, &[
            "E|\"{System Tester options are set to enter on the Close} BuyTrigger:=(MOV(C,10,E) - MOV(C,30,E)) / MOV(C,30,E); BuyTrigger > 0.05\"",
            "X|\"BuyPrice:= If(PREV <= 0, If(BuyTrigger > 0.05, CLOSE, 0), If(Close >= PREV*(1+BuyTrigger), -PREV, PREV)); BuyPrice < 0\" - \"BuyPrice is the close on the date BuyTrigger exceeds your threshold\" (from Glenn Wallace)"], NOTE_B) },
        StrategyDef { dir: "Long", ..d("djcs_dead_cat", "Dead Cat Catcher: buy after a 5-day CMO under -99 and a 5-day fall of over 30%, out two days later", false, &[
            "E|\"{#OptVar6 Default: -30} {#OptVar5 Default: 1} {#OptVar4 Default: 5} {#OptVar1 Default: 1} ... if (#OptVar1 >= 0) and (CMO(Bar,#Close,CMOPeriod) < -99.0) and (ROC(Bar,#Close,CMOPeriod) < longROCLimit) then BuyAtMarket( Bar + 1 ); BarSave := Bar + 1\" (WealthScript)",
            "X|\"if ( Bar >= BarSave + holdPeriod ) then SellAtMarket( Bar + 1 )\"",
            "F|Short side (#OptVar1 <= 0: CMO above 99 and ROC above +30) is an option; the default OptVar1 = 1 trades longs only"], NOTE_D) },
        StrategyDef { dir: "Long", ..d("djcs_cmo_cumdown", "CMO(20) / CumDown accumulation: add up to three units after 9 closes under the close 4 days earlier with CMO(20) under -50, sell all when CMO(20) passes 50", false, &[
            "E|\"if ( CumDown( Bar, #Close, 4 ) >= 9 ) and ( CMO( Bar, #Close, 20 ) < -50 ) then if ( LastBar >= 9 ) and ( AT < 3 ) then begin BuyAtMarket( Bar + 1, 5000 ); LastBar := 0; Inc( AT ); end\" (WealthScript)",
            "X|\"for P := 0 to PositionCount() - 1 do if PositionActive( P ) then if CMO( Bar, #CLose, 20 ) > 50 then begin SellAtMarket( Bar + 1, P ); LastBar := 999; Dec( AT ); end\""], NOTE_C) },
    ];
    v.extend(MS.catalogue());
    v
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    match id {
        "djcs_pct_reversal" => Some(("djcs_pct_reversal", "Fulks percent reversal stops", "2% / 1%")),
        "djcs_tfs" => Some(("djcs_tfs", "Strain / Taylor TFS", "50-bar mid, 7-day volume oscillator")),
        "djcs_buy_trigger" => Some(("djcs_buy_trigger", "Wallace Buy trigger", "10/30 EMA, 5%")),
        "djcs_dead_cat" => Some(("djcs_dead_cat", "Dead Cat Catcher", "CMO 5, ROC -30%, hold 1")),
        "djcs_cmo_cumdown" => Some(("djcs_cmo_cumdown", "CMO / CumDown accumulation", "CumDown 9, CMO 20 +/-50, 3 units")),
        _ => MS.family_of(id),
    }
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    match id {
        "djcs_pct_reversal" => Some(vec![p("bz", "BZ: buy stop % above the lowest low (code 2)", 2., 0.1, 20., 0.1), p("sz", "SZ: sell stop % below the highest high (code 1)", 1., 0.1, 20., 0.1)]),
        "djcs_tfs" => Some(vec![p("n", "Channel bars (code 50)", 50., 2., 250., 1.), p("vo", "Volume oscillator average bars (code 7)", 7., 1., 50., 1.),
            p("orig", "Logic: 0 = Taylor variation (active code), 1 = Strain's original cross (commented out in the file)", 0., 0., 1., 1.)]),
        "djcs_buy_trigger" => Some(vec![p("fast", "Fast EMA days (code 10)", 10., 2., 100., 1.), p("slow", "Slow EMA days (code 30)", 30., 3., 250., 1.), p("trig", "Buy trigger (code 0.05)", 0.05, 0.005, 0.5, 0.005)]),
        "djcs_dead_cat" => Some(vec![p("side", "OptVar1: 1 = long only (default), 0 = both, -1 = short only", 1., -1., 1., 1.), p("cmo", "CMO / ROC bars (OptVar4 5)", 5., 2., 50., 1.),
            p("hold", "Hold period (OptVar5 1)", 1., 0., 20., 1.), p("roc_long", "Long ROC limit % (OptVar6 -30)", -30., -90., 0., 1.), p("roc_short", "Short ROC limit % (OptVar7 30)", 30., 0., 300., 1.)]),
        "djcs_cmo_cumdown" => Some(vec![p("units", "Most units held at once (code AT < 3)", 3., 1., 10., 1.)]),
        _ => MS.params(id),
    }
}

fn mk(d: f64, ei: usize, ep: f64, xi: usize, xp: f64, why: &str, kind: &'static str, si: usize, open: bool) -> Trade {
    Trade { dir: d, legs: vec![Leg { i: ei, px: ep, w: 1. }], exit_i: xi, exit_px: xp, reason: why.into(), entry_kind: kind, signal_i: si, open, exits: vec![] }
}

fn pct_reversal(x: &Ctx, from: usize, to: usize, bz: f64, sz: f64) -> Vec<Trade> {
    let (o, h, l) = (&x.o, &x.h, &x.l);
    let (mut out, mut pos) = (vec![], None::<(f64, usize, f64)>);
    let (mut hh, mut ll) = (h[from], l[from]);
    for t in from + 1..=to {
        let mp = pos.map_or(0., |p| p.0);
        let (sell, buy) = (hh * (1. - 0.01 * sz), ll * (1. + 0.01 * bz));
        let (s_hit, b_hit) = (mp >= 0. && l[t] <= sell, mp <= 0. && h[t] >= buy);
        let new_dir = match (s_hit, b_hit) { (true, false) => -1., (false, true) => 1., _ => 0. };
        if new_dir != 0. {
            let px = if new_dir < 0. { o[t].min(sell) } else { o[t].max(buy) };
            if let Some((d, ei, ep)) = pos { out.push(mk(d, ei, ep, t, px, "reversal stop", "stop", t, false)); }
            pos = if t < to { Some((new_dir, t, px)) } else { None };
        }
        if new_dir != 0. { hh = h[t]; ll = l[t]; } else { hh = hh.max(h[t]); ll = ll.min(l[t]); }
    }
    if let Some((d, ei, ep)) = pos { out.push(mk(d, ei, ep, to, x.c[to], "open", "stop", to, true)); }
    out
}

fn tfs(x: &Ctx, from: usize, to: usize, n: usize, vo: usize, orig: bool) -> Vec<Trade> {
    let (o, h, l, c) = (&x.o, &x.h, &x.l, &x.c);
    let mut osc = vec![f64::NAN; c.len()];
    let mut last = 0.; // TradeStation Vars start at 0
    for i in 0..c.len() {
        let v = x.r[i].volume.filter(|v| *v > 0.).unwrap_or(f64::NAN);
        if c[i] > o[i] { last = v; } else if c[i] < o[i] { last = -v; }
        osc[i] = last;
    }
    let avg = sma(&osc, vo);
    let mid: Vec<f64> = (0..c.len()).map(|i| if i + 1 < n { f64::NAN } else {
        (h[i + 1 - n..=i].iter().cloned().fold(f64::MIN, f64::max) + l[i + 1 - n..=i].iter().cloned().fold(f64::MAX, f64::min)) / 2. }).collect();
    let (mut out, mut pos) = (vec![], None::<(f64, usize, f64)>);
    for t in from..to {
        let (up, dn) = if orig { (t > 0 && c[t] > mid[t] && c[t - 1] <= mid[t - 1], t > 0 && c[t] < mid[t] && c[t - 1] >= mid[t - 1]) } else { (c[t] > mid[t], c[t] < mid[t]) };
        let (go_l, go_s) = (up && avg[t] > 0., dn && avg[t] < 0.);
        // up without a positive oscillator only covers a short in the original logic (the variation needs both)
        let (x_s, x_l) = if orig { (up, dn) } else { (go_l, go_s) };
        let px = o[t + 1];
        if let Some((d, ei, ep)) = pos {
            if (d > 0. && x_l) || (d < 0. && x_s) {
                let rev = (d > 0. && go_s) || (d < 0. && go_l);
                out.push(mk(d, ei, ep, t + 1, px, if rev { "reverse" } else { "exit" }, "open", t, false)); pos = None;
            }
        }
        if pos.is_none() { if go_l { pos = Some((1., t + 1, px)); } else if go_s { pos = Some((-1., t + 1, px)); } }
    }
    if let Some((d, ei, ep)) = pos { out.push(mk(d, ei, ep, to, c[to], "open", "open", to, true)); }
    out
}

/// MetaStock: enter on the close when flat and the trigger is above its level; BuyPrice = that close; close the long on
/// the first later close at or above BuyPrice x (1 + today's trigger). PREV starts at 0 (flat) in the test window.
fn buy_trigger(x: &Ctx, from: usize, to: usize, fast: usize, slow: usize, lvl: f64) -> Vec<Trade> {
    let (f, s, c) = (msws::ema(&x.c, fast), msws::ema(&x.c, slow), &x.c);
    let (mut out, mut pos) = (vec![], None::<(usize, f64)>);
    for t in from..=to {
        let trig = (f[t] - s[t]) / s[t];
        match pos {
            Some((ei, ep)) if t > ei && c[t] >= ep * (1. + trig) => { out.push(mk(1., ei, ep, t, c[t], "close reached BuyPrice x (1 + BuyTrigger)", "close", t, false)); pos = None; }
            None if trig > lvl => pos = Some((t, c[t])),
            _ => {}
        }
    }
    if let Some((ei, ep)) = pos { out.push(mk(1., ei, ep, to, c[to], "open", "close", to, true)); }
    out
}

/// Chande momentum oscillator of the close over n changes; undefined when nothing moved.
fn cmo(c: &[f64], n: usize, t: usize) -> f64 {
    if t < n { return f64::NAN; }
    let (mut up, mut dn) = (0., 0.);
    for i in t + 1 - n..=t { let d = c[i] - c[i - 1]; if d > 0. { up += d } else { dn -= d } }
    if up + dn > 0. { 100. * (up - dn) / (up + dn) } else { f64::NAN }
}

fn dead_cat(x: &Ctx, from: usize, to: usize, side: i32, n: usize, hold: usize, roc_l: f64, roc_s: f64) -> Vec<Trade> {
    let (o, c) = (&x.o, &x.c);
    let (mut out, mut pos) = (vec![], None::<(f64, usize, f64)>);
    for t in from..=to {
        if let Some((d, ei, ep)) = pos {
            if t < ei { continue; }
            if t >= ei + hold {
                if t < to { out.push(mk(d, ei, ep, t + 1, o[t + 1], "hold period over", "open", t, false)); pos = None; }
            }
            continue;
        }
        if t >= to || t < n { continue; }
        let (m, roc) = (cmo(c, n, t), 100. * (c[t] / c[t - n] - 1.));
        let d = if side >= 0 && m < -99. && roc < roc_l { 1. } else if side <= 0 && m > 99. && roc > roc_s { -1. } else { 0. };
        if d != 0. { pos = Some((d, t + 1, o[t + 1])); }
    }
    if let Some((d, ei, ep)) = pos { out.push(mk(d, ei, ep, to, c[to], "open", "open", to, true)); }
    out
}

/// Up to `units` legs of 1/units each, all sold together.
fn cmo_cumdown(x: &Ctx, from: usize, to: usize, units: usize) -> Vec<Trade> {
    let (o, c) = (&x.o, &x.c);
    let w = 1. / units as f64;
    let (mut out, mut legs, mut since) = (vec![], Vec::<Leg>::new(), 999usize);
    let mut cum = 0usize; // consecutive closes below the close 4 bars earlier
    for t in 0..=to {
        cum = if t >= 4 && c[t] < c[t - 4] { cum + 1 } else { 0 };
        if t < from { continue; }
        since += 1;
        let m = cmo(c, 20, t);
        let held: Vec<Leg> = legs.iter().filter(|l| l.i <= t).cloned().collect();
        if !held.is_empty() && m > 50. && t < to {
            out.push(Trade { dir: 1., legs: held, exit_i: t + 1, exit_px: o[t + 1], reason: "CMO(20) above 50".into(), entry_kind: "open", signal_i: t, open: false, exits: vec![] });
            legs.retain(|l| l.i > t); since = 999;
        }
        if cum >= 9 && m < -50. && since >= 9 && legs.len() < units && t < to {
            legs.push(Leg { i: t + 1, px: o[t + 1], w }); since = 0;
        }
    }
    if !legs.is_empty() { out.push(Trade { dir: 1., legs, exit_i: to, exit_px: c[to], reason: "open".into(), entry_kind: "open", signal_i: to, open: true, exits: vec![] }); }
    out
}

fn day_number(d: &str) -> i64 {
    let (y, m, dd): (i64, i64, i64) = (d[0..4].parse().unwrap_or(0), d[5..7].parse().unwrap_or(1), d[8..10].parse().unwrap_or(1));
    let y = if m <= 2 { y - 1 } else { y };
    let (era, yoe) = (y.div_euclid(400), y.rem_euclid(400));
    let doy = (153 * ((m + 9) % 12) + 2) / 5 + dd - 1;
    era * 146097 + yoe * 365 + yoe / 4 - yoe / 100 + doy - 719468
}

/// Runs a weekly MetaStock system on weekly bars built from days 0..=to; trade bars map back to each week's last day.
fn weekly_run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    let (mut rows, mut last): (Vec<crate::Row>, Vec<usize>) = (vec![], vec![]);
    let wk = |i: usize| (day_number(&x.r[i].date) + 3).div_euclid(7);
    for i in 0..=to {
        let r = &x.r[i];
        if i > 0 && wk(i) == wk(i - 1) {
            let w = rows.last_mut().unwrap();
            w.high = w.high.max(r.high); w.low = w.low.min(r.low); w.close = r.close; w.date = r.date.clone();
            *last.last_mut().unwrap() = i;
        } else {
            rows.push(crate::Row { ticker: r.ticker.clone(), date: r.date.clone(), open: r.open, high: r.high, low: r.low, close: r.close, volume: None });
            last.push(i);
        }
    }
    let wf = last.iter().position(|&e| e >= from)?;
    let wx = Ctx::new(&rows, &rows, &[]);
    let mut t = MS.run(id, &wx, wf, rows.len() - 1, p)?;
    for z in t.iter_mut() {
        for l in z.legs.iter_mut() { l.i = last[l.i]; }
        z.exit_i = last[z.exit_i]; z.signal_i = last[z.signal_i];
    }
    Some(t)
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.r.len() { return None; }
    let schema = params(id)?;
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).map_or(f64::NAN, |d| d.def));
    let u = |k: &str| g(k).round().max(1.) as usize;
    Some(match id {
        "djcs_pct_reversal" => pct_reversal(x, from, to, g("bz"), g("sz")),
        "djcs_tfs" => tfs(x, from, to, u("n"), u("vo"), g("orig").round() >= 1.),
        "djcs_buy_trigger" => buy_trigger(x, from, to, u("fast"), u("slow"), g("trig")),
        "djcs_dead_cat" => dead_cat(x, from, to, g("side").round() as i32, u("cmo"), g("hold").round().max(0.) as usize, g("roc_long"), g("roc_short")),
        "djcs_cmo_cumdown" => cmo_cumdown(x, from, to, u("units")),
        "djcs_marketspace" => return MS.run(id, x, from, to, p),
        "djcs_marketspace_weekly" => return weekly_run(id, x, from, to, p),
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn ids_and_params() {
        assert!(reviewed("djcs_pct_reversal") && reviewed("djcs_tfs") && !reviewed("djcs_party_girl"));
        assert_eq!(catalogue().len(), 7);
        assert!(catalogue().iter().all(|d| reviewed(d.id)));
        assert!(catalogue().iter().find(|d| d.id == "djcs_tfs").unwrap().needs_volume);
    }
}
