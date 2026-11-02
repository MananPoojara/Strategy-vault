//! Currency Trader magazine (currencytradermag.com), 2006-2010: the Wealth-Lab "Currency System Analysis" columns of Jan-Oct 2006
//! and the "Trading Strategies" systems of 2008-2010. Research share Trading Knowledge I/@Newsletters/Currency Trader (2004 - 2010)
//! (currencytradermag.com)/2006, /2008, /2009, /2010; text authors/titles-1030/K15w/src/ct*.txt (pdftotext of the article pages).
//! Book id `ctm`. Lane K15 (titles loop, 2 Nov 2026), to-code rows 256-267; lead decisions 88-90; independent check
//! authors/titles-1030/K15w/k15_ref.py. Also holds the next-bar order engine used by `fot08`.
use crate::connors::{atr, true_range, Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};
/// Simple averages as plain window sums (msws::sma), so equal averages stay exactly equal.
use super::msws::sma;
use super::lc3::{hi, high_first, lo};
use super::tselhs::Book;

/// Admitted ids (authors/titles-1030/K15w/k15_ref.py (header)).
pub const REVIEWED: &[&str] = &["ctm_hh2", "ctm_invmart", "ctm_trend_str", "ctm_ts_cross", "ctm_mid_cross", "ctm_hlr40", "ctm_outside",
    "ctm_euro_mom", "ctm_inside", "ctm_bo_timing", "ctm_mom20", "ctm_sma12"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

// ------------------------------------------------------------------ next-bar orders (shared with fot08)
#[derive(Clone, Copy, PartialEq)]
pub enum K { Mkt, Stop, Lim }
/// An order for the next bar: `exit` = closes only the position held when it was placed; otherwise an entry (reverses an
/// opposite position). `d` = +1 buy, -1 sell.
#[derive(Clone, Copy)]
pub struct Ord { pub exit: bool, pub d: f64, pub k: K, pub px: f64 }
pub fn ent(d: f64, k: K, px: f64) -> Ord { Ord { exit: false, d, k, px } }
pub fn ext(d: f64, k: K, px: f64) -> Ord { Ord { exit: true, d, k, px } }

/// (rank, price) of an order on bar i: rank 0 = at the open, 1 = on the leg to the extreme nearer the open, 2 = the other leg.
pub fn reach(x: &Ctx, i: usize, z: &Ord) -> Option<(u8, f64)> {
    let (o, h, l) = (x.o[i], x.h[i], x.l[i]);
    if z.k == K::Mkt { return Some((0, o)); }
    let up = (z.k == K::Stop) == (z.d > 0.);
    let leg = if up == high_first(o, h, l) { 1 } else { 2 };
    if up { if o >= z.px { Some((0, o)) } else if h >= z.px { Some((leg, z.px)) } else { None } }
    else if o <= z.px { Some((0, o)) } else if l <= z.px { Some((leg, z.px)) } else { None }
}

/// Bar i: a market exit fills at the open; then the first still-valid order the price reaches fills (one per bar): by leg,
/// and on one leg the price nearer the open first (ties: the order listed first).
pub fn bar(pos: &mut Book, x: &Ctx, i: usize, ords: &[Ord]) {
    for z in ords { if z.exit && z.k == K::Mkt && pos.side() == -z.d { pos.exit(i, x.o[i], "market exit at the open"); } }
    let mut best: Option<(u8, f64, f64, Ord)> = None;
    for z in ords {
        if z.exit && (z.k == K::Mkt || pos.side() != -z.d) { continue; }
        if !z.exit && pos.side() == z.d { continue; }
        if let Some((r, px)) = reach(x, i, z) {
            let dist = (px - x.o[i]).abs();
            if best.map_or(true, |b| r < b.0 || (r == b.0 && dist < b.2)) { best = Some((r, px, dist, *z)); }
        }
    }
    if let Some((_, px, _, z)) = best {
        if z.exit { pos.exit(i, px, if z.k == K::Lim { "limit" } else { "stop" }); }
        else { pos.enter(z.d, i, px, match z.k { K::Mkt => "open", K::Stop => "stop", K::Lim => "limit" }); }
    }
}

/// `make(t, pos)` acts at close t (from <= t <= to) and returns the orders for bar t+1 (ignored at `to`).
pub fn orders(x: &Ctx, from: usize, to: usize, mut make: impl FnMut(usize, &mut Book) -> Vec<Ord>) -> Vec<Trade> {
    let mut pos = Book::new();
    let mut ords: Vec<Ord> = vec![];
    for t in from..=to {
        if t > from && !ords.is_empty() { bar(&mut pos, x, t, &ords); }
        ords = make(t, &mut pos);
    }
    pos.done(to, x.c[to])
}

pub fn g(p: &Params, k: &str, defs: Option<Vec<ParamDef>>) -> f64 {
    p.get(k).copied().unwrap_or_else(|| defs.and_then(|s| s.iter().find(|d| d.key == k).map(|d| d.def)).unwrap_or(f64::NAN))
}
pub fn fin(v: f64) -> bool { v.is_finite() }

// ------------------------------------------------------------------ catalogue
macro_rules! wl { () => { " Wealth-Lab column (José Cruset): tested on BP, EC, JY and SF futures (Pinnacle ratio-adjusted, 1990/91-2005, $20 commission, two ticks slippage on stops, 2-3% equity risk sizing, not applied). Orders are placed at each close for the next bar (market = the open; a stop at its price or the open if the open is through it; one fill per bar: the order the price reaches first, the extreme nearer the open first). One position, one unit. Daily indices, ETFs and US/India stocks are a declared application (lead decision 88); the vault's standard costs; the column's figures are the magazine's (gate book field), not check targets." } }
macro_rules! ts { () => { " Currency Trader 'Trading Strategies' system on spot FX pairs; one position, one unit. Daily indices, ETFs and US/India stocks are a declared application (lead decision 88); the vault's standard costs; the article's figures are its own (gate book field), not check targets." } }

const HH2: &str = concat!("[Declared readings] CT Jan 2006 p.36-38, 'Narrowing breakout channels': at each close a buy stop at the second-highest high of the last 20 days (today included; the second value of the sorted 20 highs, so a tie at the top counts) and a sell-short stop at the second-lowest low, each placed while the position is not already on that side, so the system stops and reverses.", wl!());
const INV: &str = concat!("[Declared readings] CT Feb 2006 p.38-40, 'Inverse Martingale rule': flat -> a buy stop 3 x ATR(10) (Wilder) above the close and a sell-short stop 3 x ATR(10) below it. After a fill, a profit target (limit) and a stop each 3 x ATR(10) from the entry price, with the ATR of the close before the fill, fixed until the next event; one event per bar, checked from the bar after it. Stop hit -> close everything at the stop and reverse with one unit at the same price. Target hit -> double the position at the target price (1 -> 2 -> 4 -> 8 units: legs of 1/8, 1/8, 2/8 and 4/8 of a full position, so a full streak holds the whole allocation and a first-unit trade one eighth) and set both 3 x ATR(10) from that newest entry; after the third doubling both are 2 x ATR(10) from it and either one closes everything (flat, no reversal).", wl!());
const TSI: &str = concat!("[Declared readings] CT Mar 2006 p.28-30, 'Trend Strength indicator': TS = (number of the six SMAs 10, 20 ... 60 below the close - number above) / 6 x 100. Flat: TS reaches +100 (now 100, below 100 the day before) -> buy at the next open; reaches -100 -> short at the next open. Exit at the next open when TS crosses zero against the position (long: below zero today, at or above yesterday). Stop-loss as a stop order at an SMA value of the signal day: the printed rule says 'a move above the highest SMA value at the time of entry' for both sides, which cannot be a loss level for a long (every SMA is below the price); the default (lead decision 89) reads it as the outer SMA: long stop at the lowest SMA, short stop at the highest. p.stop = 1: the printed 'highest SMA' for the long side too.", wl!());
const TSC: &str = concat!("[Declared readings] CT Apr 2006 p.36-38, 'Trend Strength Crossover': ten SMAs 10, 20 ... 100. Daily value = +1 for every pair (shorter, longer) where the shorter SMA crosses above the longer that day, -1 where it crosses below (all 45 pairs: 'adding 1 when a shorter-term moving average crosses above a longer-term moving average'); the system uses its 20-day sum. The sum rises above zero -> buy at the next open (covering a short); falls below zero -> short at the next open. Stop-loss: a stop order 3 x ATR(10) (Wilder, the signal day's) against the entry price (printed 'a move above' for both sides, read as an adverse move); a stopped trade waits for the next crossing.", wl!());
const MID: &str = concat!("[Declared readings] CT May 2006 p.30-32, 'Channel Midpoint': midpoint(n) = (highest high + lowest low of the last n days)/2. The 150-day midpoint crosses above the 250-day midpoint -> long at the next open (reversing a short); crosses below -> short at the next open.", wl!());
const HLR: &str = concat!("[Declared readings] CT Aug 2006 p.28-30, 'HLR breakout': HLR = (close - lowest low of 40 days)/(highest high - lowest low of 40 days), today included ('price's relative location within the high-low range of the past x bars'). Not long and HLR above 0.8 -> long at the next open; not short and HLR below 0.2 -> short at the next open (stop and reverse).", wl!());
const OUT: &str = concat!("[Declared readings] CT Oct 2006 p.28-30, 'Outside-bar opportunity' (euro futures): today an outside day (high above and low below yesterday's) that closed above its open, above yesterday's close, with a high above the highs of the three days before -> buy at the next open. Exit at the close of the 10th day after the outside day, or at a close below the outside day's low. TARGET HELD: the 0.0250 profit limit is in euro price points (lead decisions 19, 88).", ts!());
const EUR: &str = concat!("[Declared readings] CT Euro momentum system (May, Jun, Jul 2008; review Apr 2009 p.24-26): momentum(n) = 2 x (close - lowest low)/(highest high - lowest low) - 1 over the last n days (today included), -1..+1. Default = the Apr 2009 rules: flat, 30-day > 0.60 and 5-day < -0.80 -> long at the next open; 60-day < -0.60 and 10-day > +0.80 -> short at the next open (the to-code row's '62-day' is a transcription slip; the article prints 60); exit at the close 9 days after the entry day ('Exit any remaining trades after nine days'). STOP/TARGET HELD: the 2008 columns' targets and stop are euro price points, and the 2009 review's '3 percent' target with a '0.085 percent' stop do not match them (lead decision 88). p.set = 1: the May 2008 study triggers (30-day > 0.40 with 5-day < -0.85, short mirrored) at the signal close, out at the close 5 days later.", ts!());
const INS: &str = concat!("[Declared readings] CT Jan 2009 p.20-24, 'Inside-day setups': enter at today's close, exit at the close two days later. Default = the second pattern with the 40-day trend filter (the article's TradeStation code: inside day, (C-L)/(H-L) <= 0.35, close > close 40 days ago -> buy this bar on close); the short side is the mirror (close in the upper 35% of the range, close below the close 40 days ago). p.pat = 1: the first pattern (inside day closing below yesterday's close -> long, above -> short); p.filt = 0: no trend filter. Flat only; an exit and a new entry can share a close.", ts!());
const BOT: &str = concat!("[Declared readings] CT Jul 2009 p.20-22, 'Breakout timing' base system: a buy stop at the highest high and a sell-short stop at the lowest low of the past 10 days (today included), for the next day; a position exits at the close five days after the entry day unless an opposite breakout reverses it first; a same-direction breakout while in a trade changes nothing. The day-of-month splits are the article's analysis, not a rule.", ts!());
const M20: &str = concat!("[Declared readings] CT Aug 2009 p.22-24, 20-day momentum (Harris, Stoja and Yilmaz study): flat, close above the close 20 days ago -> long at that close, below -> short; exit all trades at the close 20 days later; a new signal during a hold does nothing. The article's own test lost 3% in ten years (reported, not tuned).", ts!());
const S12: &str = concat!("[Declared readings] CT Nov 2010 p.18-20, 'Multiple average trend-following' (MetaTrader): long when the 12 SMAs 3, 5, 8, 10, 12, 15, 30, 35, 40, 45, 50 and 60 are in strict order on the last closed candle (3 > 5 > ... > 60), short in the reverse order; entry at the next open; a trade closes at the next open when its alignment is lost, or on a stop 2 x ATR(20) from the entry price (MetaTrader's ATR: the simple 20-day mean of the true range, at the signal close). Position sizing (0.4% of equity per ATR) is not applied.", ts!());

pub fn catalogue() -> Vec<StrategyDef> {
    let d = |id, name, dir, rules, notes| StrategyDef { id, name, group: "Currency Trader 2006-10 (Currency System Analysis, Trading Strategies)", session: 0, dir, status: Status::Live,
        needs_volume: false, needs_vix: false, needs_series: &[], book: "ctm", rules, notes, claim: None };
    vec![
        d("ctm_hh2", "Narrowing breakout channels: stop and reverse at the second-highest high / second-lowest low of 20 days", "Long/Short", &[
            "E|\"1. Go long at stop at the second highest high of the last 20 days.\" (CT Jan 2006 p.37)",
            "X|\"2. Exit long and go short at stop at the second lowest low of the last 20 days.\" (p.37)"], HH2),
        d("ctm_invmart", "Inverse Martingale: 3 ATR stop entries, double the position at each 3 ATR target up to eight units, stop and reverse", "Long/Short", &[
            "E|\"1. a) Go long with a stop order at three times the 10-day ATR above the close, or b) sell short at stop at three times the 10-day ATR below the close.\" (CT Feb 2006 p.38)",
            "X|\"2. Set a profit target and a stop order at three times the 10-day ATR from the entry price. Exit if the stop is reached, reverse direction, and start again with one position.\" (p.38)",
            "E|\"3. Double the position if profit target is reached. Set both profit targets at three times the 10-day ATR below (or above) the most recent entry price. 4. Double the position a maximum of three times. After that, exit all positions either at limit (profit target) or at stop (stop loss) at twice the 10-day ATR from the last entry.\" (p.38-39)"], INV),
        d("ctm_trend_str", "Trend Strength indicator: enter when price is above (below) all six SMAs 10-60, exit when the indicator crosses zero", "Long/Short", &[
            "E|\"1. Enter long next day at market when the Trend Strength indicator reaches +100. ... 4. Enter short next day at market when the Trend Strength indicator reaches -100.\" (CT Mar 2006 p.29)",
            "X|\"2. Exit long next day at market when the Trend Strength indicator crosses below zero. 3. Stop-loss: Exit with a loss on a move above the highest SMA value at the time of entry.\" (p.29; rules 5-6 the same for shorts)"], TSI),
        d("ctm_ts_cross", "Trend Strength Crossover: 20-day sum of SMA crossovers among ten SMAs 10-100 crosses zero, 3 ATR stop", "Long/Short", &[
            "F|\"The Trend Strength Crossover indicator is calculated by adding 1 when a shorter-term moving average crosses above a longer-term moving average and subtracting 1 when a shorter-term SMA crosses below a longer-term SMA.\" (CT Apr 2006 p.36)",
            "E|\"1. Go long and exit short next day at market if the Trend Strength Crossover indicator's 20-day sum rises above zero. ... 3. Go short and exit long next day at market if ... drops below zero.\" (p.37)",
            "X|\"2. Stop-loss: Exit with a loss on a move above three times the 10-day average true range (ATR).\" (p.37)"], TSC),
        d("ctm_mid_cross", "Channel Midpoint crossover: the 150-day high/low midpoint crosses the 250-day one, stop and reverse", "Long/Short", &[
            "E|\"1. Go long and exit short the next day at the market if the 150-day Channel Midpoint indicator crosses above the 250-day Channel Midpoint indicator. 2. Go short and exit long ... if [it] crosses below\" (CT May 2006 p.31)", "X|Stop and reverse: the opposite crossing closes the trade and opens the other side (p.31)"], MID),
        d("ctm_hlr40", "HLR breakout: long when the close is in the top 20% of the 40-day range, short in the bottom 20%", "Long/Short", &[
            "F|HighestLowestRange (HLR): \"shows price's relative location within the high-low range of the past x bars\" (CT Aug 2006 p.28), 40 days",
            "E|\"1. Go long the next day at the market when the HLR indicator rises above 0.8. 2. Exit long and go short the next day at the market when the HLR indicator drops below 0.2.\" (p.29)", "X|Stop and reverse: the opposite HLR signal (p.29)"], HLR),
        d("ctm_outside", "Outside-bar opportunity: buy after a strong up outside day, out after 10 days or on a close below its low (target held)", "Long", &[
            "E|\"1. Go long on tomorrow's open if today is an outside day that closed above today's open, closed above yesterday's close, and has a high above the highs of the past three days.\" (CT Oct 2006 p.28)",
            "X|\"3. Exit any open position on the close of the 10th day after the outside day. 4. Exit with a loss if the market closed below the low of the outside day.\" (p.28; rule 2's 0.0250 limit held)"], OUT),
        d("ctm_euro_mom", "Euro momentum: buy a 5-day oversold dip in a 30-day uptrend, short a 10-day spike in a 60-day downtrend, out after 9 days", "Long/Short", &[
            "E|\"1. Go long on the next day's open when the 30-day momentum is above 0.60 and the five-day momentum is below -0.80 (-80 percent). 2. Go short on the next day's open when the 60-day momentum is below -0.60 and the 10-day momentum is above +0.80\" (CT Apr 2009 p.24)",
            "X|\"3. Exit any remaining trades after nine days.\" (p.26)", "F|\"Momentum indicator = current price/(highn - lown) ... modified to range between -1.00 and +1.00\" (p.24)"], EUR),
        d("ctm_inside", "Inside-day setups: fade an inside day closing in the bottom (top) 35% of its range, with the 40-day trend, for two days", "Long/Short", &[
            "E|\"if High < High[1] and Low > Low[1] and (Close-Low)/(High-Low) <= closethresh and close > close[trendlength] then Buy this bar on close;\" inputs trendlength(40), closethresh(.35) (CT Jan 2009 p.22)",
            "X|\"Exit position at the close two days after entry.\" (p.21)"], INS),
        d("ctm_bo_timing", "Breakout timing base system: 10-day high/low stop entries, out at the close five days later", "Long/Short", &[
            "E|\"Go long on a move above the highest high of the past 10 days and go short on a move below the lowest low of the past 10 days.\" (CT Jul 2009 p.20)",
            "X|\"Positions are exited at the close five days after entry if not reversed by a signal in the opposite direction.\" (p.20)"], BOT),
        d("ctm_mom20", "20-day momentum: long above the close 20 days ago, short below, every trade held 20 days", "Long/Short", &[
            "E|\"1. Go long when the close is above the close 20 days ago. 2. Go short when the close is below the close 20 days ago.\" (CT Aug 2009 p.23)", "X|\"3. Exit all trades after 20 days.\" (p.23)"], M20),
        d("ctm_sma12", "Multiple average alignment: twelve SMAs 3-60 in order, out when the order breaks or on a 2 ATR stop", "Long/Short", &[
            "E|\"on the last closed candle all 12 moving averages align in increasing order — that is, successively shorter moving averages are above the longer averages\"; shorts \"when the averages align in descending order\" (CT Nov 2010 p.18-19)",
            "X|\"Any open trades are closed when the SMAs lose their proper alignments on the last closed candle\"; \"a stop-loss of two times the 20-day average true range (ATR)\" (p.19)"], S12),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    let f = match id {
        "ctm_hh2" => ("CT second-highest channel", "20-day 2nd high/low, stop and reverse"),
        "ctm_invmart" => ("CT inverse Martingale", "3 ATR targets, doubling to 8 units"),
        "ctm_trend_str" => ("CT Trend Strength", "six SMAs 10-60, +/-100 entries"),
        "ctm_ts_cross" => ("CT Trend Strength Crossover", "20-day sum of SMA crossovers"),
        "ctm_mid_cross" => ("CT Channel Midpoint", "150 vs 250-day midpoint"),
        "ctm_hlr40" => ("CT HLR breakout", "40-day HLR 0.8 / 0.2"),
        "ctm_outside" => ("CT outside bar", "strong outside day, 10-day exit"),
        "ctm_euro_mom" => ("CT Euro momentum", "30/5 and 60/10-day momentum"),
        "ctm_inside" => ("CT inside-day fade", "close in 35% of range, 2 days"),
        "ctm_bo_timing" => ("CT breakout timing", "10-day breakout, 5-day exit"),
        "ctm_mom20" => ("CT 20-day momentum", "close vs 20 days ago, 20-day hold"),
        "ctm_sma12" => ("CT twelve-SMA alignment", "SMAs 3-60 in order"),
        _ => return None,
    };
    let id = REVIEWED.iter().find(|r| **r == id)?;
    Some((*id, f.0, f.1))
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    Some(match id {
        "ctm_trend_str" => vec![p("stop", "Long stop: 0 = lowest SMA at the signal (lead decision 89); 1 = highest SMA (printed wording)", 0., 0., 1., 1.)],
        "ctm_euro_mom" => vec![p("set", "Rule set: 0 = Apr 2009 (30/5 long, 60/10 short, next open, 9 days); 1 = May 2008 study (30/5 at 0.40/0.85, signal close, 5 days)", 0., 0., 1., 1.)],
        "ctm_inside" => vec![p("pat", "Pattern: 0 = close in the bottom/top 35% of the inside day; 1 = close below/above yesterday's close", 0., 0., 1., 1.),
            p("filt", "40-day trend filter: 1 = on (the article's code); 0 = off", 1., 0., 1., 1.)],
        _ if reviewed(id) => vec![],
        _ => return None,
    })
}

fn rng_pos(x: &Ctx, t: usize, n: usize) -> f64 {
    if t + 1 < n { return f64::NAN; }
    let (hh, ll) = (hi(&x.h, t + 1 - n, t), lo(&x.l, t + 1 - n, t));
    if hh > ll { (x.c[t] - ll) / (hh - ll) } else { f64::NAN }
}

fn invmart(x: &Ctx, from: usize, to: usize) -> Vec<Trade> {
    let a = atr(x.r, 10);
    let mut out = vec![];
    // (dir, legs, doublings, target, stop)
    let mut cur: Option<(f64, Vec<Leg>, u32, f64, f64)> = None;
    let open = |d: f64, i: usize, px: f64, at: f64| (d, vec![Leg { i, px, w: 0.125 }], 0u32, px + d * 3. * at, px - d * 3. * at);
    let close = |c: (f64, Vec<Leg>, u32, f64, f64), i: usize, px: f64, why: &str, op: bool| Trade { dir: c.0, legs: c.1, exit_i: i, exit_px: px, reason: why.into(), entry_kind: "stop", signal_i: i, open: op, exits: vec![] };
    let mut entry: Option<(f64, f64)> = None;
    for t in from..=to {
        if t > from {
            if cur.is_none() {
                if let Some((up, dn)) = entry {
                    let (f1, f2) = (reach(x, t, &ent(1., K::Stop, up)), reach(x, t, &ent(-1., K::Stop, dn)));
                    match (f1, f2) {
                        (Some(u), Some(v)) if u.0 <= v.0 => cur = Some(open(1., t, u.1, a[t - 1])),
                        (Some(u), None) => cur = Some(open(1., t, u.1, a[t - 1])),
                        (_, Some(v)) => cur = Some(open(-1., t, v.1, a[t - 1])),
                        _ => {}
                    }
                }
            } else if cur.as_ref().map_or(false, |c| c.1.last().unwrap().i < t) {
                let mut c = cur.take().unwrap();
                let d = c.0;
                let (ft, fs) = (reach(x, t, &ext(-d, K::Lim, c.3)), reach(x, t, &ext(-d, K::Stop, c.4)));
                match (ft, fs) {
                    (Some(u), s) if s.map_or(true, |s| u.0 < s.0) => {
                        if c.2 < 3 {
                            c.1.push(Leg { i: t, px: u.1, w: 0.125 * 2f64.powi(c.2 as i32) });
                            c.2 += 1;
                            let m = if c.2 < 3 { 3. } else { 2. };
                            c.3 = u.1 + d * m * a[t - 1];
                            c.4 = u.1 - d * m * a[t - 1];
                            cur = Some(c);
                        } else { out.push(close(c, t, u.1, "final target", false)); }
                    }
                    (_, Some(s)) => {
                        let k = c.2;
                        out.push(close(c, t, s.1, "stop", false));
                        if k < 3 { cur = Some(open(-d, t, s.1, a[t - 1])); }
                    }
                    _ => cur = Some(c),
                }
            }
        }
        entry = None;
        if t == to { break; }
        if cur.is_none() && fin(a[t]) { entry = Some((x.c[t] + 3. * a[t], x.c[t] - 3. * a[t])); }
    }
    if let Some(c) = cur { out.push(close(c, to, x.c[to], "open", true)); }
    out
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.r.len() { return None; }
    let (o, h, l, c) = (&x.o, &x.h, &x.l, &x.c);
    let n = x.r.len();
    let gp = |k: &str| g(p, k, params(id));
    Some(match id {
        "ctm_hh2" => orders(x, from, to, |t, pos| {
            if t < 19 { return vec![]; }
            let mut hs: Vec<f64> = h[t - 19..=t].to_vec(); hs.sort_by(|a, b| b.partial_cmp(a).unwrap());
            let mut ls: Vec<f64> = l[t - 19..=t].to_vec(); ls.sort_by(|a, b| a.partial_cmp(b).unwrap());
            let mut z = vec![];
            if pos.side() != 1. { z.push(ent(1., K::Stop, hs[1])); }
            if pos.side() != -1. { z.push(ent(-1., K::Stop, ls[1])); }
            z
        }),
        "ctm_invmart" => invmart(x, from, to),
        "ctm_trend_str" => {
            let s: Vec<Vec<f64>> = (1..=6).map(|k| sma(c, 10 * k)).collect();
            let ts: Vec<f64> = (0..n).map(|i| if !fin(s[5][i]) { f64::NAN } else {
                100. * (s.iter().filter(|v| v[i] < c[i]).count() as f64 - s.iter().filter(|v| v[i] > c[i]).count() as f64) / 6. }).collect();
            let high = gp("stop").round() == 1.;
            let mut lvl = f64::NAN;
            orders(x, from, to, |t, pos| {
                if t < 1 || !fin(ts[t - 1]) { return vec![]; }
                let sd = pos.side();
                let (mx, mn) = (s.iter().map(|v| v[t]).fold(f64::MIN, f64::max), s.iter().map(|v| v[t]).fold(f64::MAX, f64::min));
                if sd == 0. {
                    if ts[t] == 100. && ts[t - 1] < 100. { lvl = if high { mx } else { mn }; return vec![ent(1., K::Mkt, f64::NAN)]; }
                    if ts[t] == -100. && ts[t - 1] > -100. { lvl = mx; return vec![ent(-1., K::Mkt, f64::NAN)]; }
                    return vec![];
                }
                let mut z = vec![];
                if (sd > 0. && ts[t] < 0. && ts[t - 1] >= 0.) || (sd < 0. && ts[t] > 0. && ts[t - 1] <= 0.) { z.push(ext(-sd, K::Mkt, f64::NAN)); }
                z.push(ext(-sd, K::Stop, lvl));
                z
            })
        }
        "ctm_ts_cross" => {
            let s: Vec<Vec<f64>> = (1..=10).map(|k| sma(c, 10 * k)).collect();
            let mut dd = vec![0f64; n];
            for i in 1..n {
                let mut v = 0.;
                for p_ in 0..10 { for q in p_ + 1..10 {
                    let (s0, s1, l0, l1) = (s[p_][i - 1], s[p_][i], s[q][i - 1], s[q][i]);
                    if !(fin(s0) && fin(l0)) { continue; }
                    if s1 > l1 && s0 <= l0 { v += 1.; } else if s1 < l1 && s0 >= l0 { v -= 1.; }
                } }
                dd[i] = v;
            }
            let ss: Vec<f64> = (0..n).map(|i| if i >= 119 { dd[i - 19..=i].iter().sum() } else { f64::NAN }).collect();
            let a = atr(x.r, 10);
            let (mut pend, mut cur) = (f64::NAN, f64::NAN);
            orders(x, from, to, |t, pos| {
                let sd = pos.side();
                if sd != 0. && pos.entry_bar() == Some(t) { cur = pend; }
                let mut z = vec![];
                if t >= 1 && fin(ss[t]) && fin(ss[t - 1]) {
                    if ss[t] > 0. && ss[t - 1] <= 0. && sd != 1. { z.push(ent(1., K::Mkt, f64::NAN)); pend = a[t]; }
                    else if ss[t] < 0. && ss[t - 1] >= 0. && sd != -1. { z.push(ent(-1., K::Mkt, f64::NAN)); pend = a[t]; }
                }
                if sd != 0. { z.push(ext(-sd, K::Stop, pos.entry_px() - sd * 3. * cur)); }
                z
            })
        }
        "ctm_mid_cross" => {
            let mid = |m: usize| -> Vec<f64> { (0..n).map(|i| if i + 1 < m { f64::NAN } else { (hi(h, i + 1 - m, i) + lo(l, i + 1 - m, i)) / 2. }).collect() };
            let (m1, m2) = (mid(150), mid(250));
            orders(x, from, to, |t, pos| {
                if t < 250 { return vec![]; }
                if m1[t] > m2[t] && m1[t - 1] <= m2[t - 1] && pos.side() != 1. { return vec![ent(1., K::Mkt, f64::NAN)]; }
                if m1[t] < m2[t] && m1[t - 1] >= m2[t - 1] && pos.side() != -1. { return vec![ent(-1., K::Mkt, f64::NAN)]; }
                vec![]
            })
        }
        "ctm_hlr40" => orders(x, from, to, |t, pos| {
            let r = rng_pos(x, t, 40);
            if r > 0.8 && pos.side() != 1. { vec![ent(1., K::Mkt, f64::NAN)] } else if r < 0.2 && pos.side() != -1. { vec![ent(-1., K::Mkt, f64::NAN)] } else { vec![] }
        }),
        "ctm_outside" => {
            let mut pos = Book::new();
            let (mut s, mut pend) = (0usize, false);
            for t in from..=to {
                if pend { pos.enter(1., t, o[t], "open"); pend = false; }
                if pos.side() > 0. && (c[t] < l[s] || t >= s + 10) { pos.exit(t, c[t], if c[t] < l[s] { "close below the outside day's low" } else { "10th day after the outside day · close" }); }
                if t < to && pos.side() == 0. && t >= 3 && h[t] > h[t - 1] && l[t] < l[t - 1] && c[t] > o[t] && c[t] > c[t - 1] && h[t] > hi(h, t - 3, t - 1) { s = t; pend = true; }
            }
            pos.done(to, c[to])
        }
        "ctm_euro_mom" => {
            let mom = |m: usize| -> Vec<f64> { (0..n).map(|i| { let r = rng_pos(x, i, m); 2. * r - 1. }).collect() };
            let (m30, m5, m60, m10) = (mom(30), mom(5), mom(60), mom(10));
            let one = gp("set").round() == 1.;
            let hold = if one { 5 } else { 9 };
            let mut pos = Book::new();
            let mut pend = 0.;
            for t in from..=to {
                if pend != 0. { pos.enter(pend, t, o[t], "open"); pend = 0.; }
                if pos.side() != 0. && t - pos.entry_bar().unwrap() >= hold { pos.exit(t, c[t], "time exit · close"); }
                if pos.side() == 0. && t < to {
                    if one {
                        let d = if m30[t] > 0.4 && m5[t] < -0.85 { 1. } else if m30[t] < -0.4 && m5[t] > 0.85 { -1. } else { 0. };
                        if d != 0. { pos.enter(d, t, c[t], "close"); }
                    } else {
                        pend = if m30[t] > 0.6 && m5[t] < -0.8 { 1. } else if m60[t] < -0.6 && m10[t] > 0.8 { -1. } else { 0. };
                    }
                }
            }
            pos.done(to, c[to])
        }
        "ctm_inside" => {
            let (pat, filt) = (gp("pat").round(), gp("filt").round());
            let mut pos = Book::new();
            for t in from..=to {
                if pos.side() != 0. && t - pos.entry_bar().unwrap() >= 2 { pos.exit(t, c[t], "two days · close"); }
                if pos.side() == 0. && t < to && t >= 40 && h[t] < h[t - 1] && l[t] > l[t - 1] {
                    let mut d = if pat == 1. { if c[t] < c[t - 1] { 1. } else if c[t] > c[t - 1] { -1. } else { 0. } } else {
                        let q = if h[t] > l[t] { (c[t] - l[t]) / (h[t] - l[t]) } else { f64::NAN };
                        if q <= 0.35 { 1. } else if q >= 0.65 { -1. } else { 0. } };
                    if d != 0. && filt == 1. && !(d * (c[t] - c[t - 40]) > 0.) { d = 0.; }
                    if d != 0. { pos.enter(d, t, c[t], "close"); }
                }
            }
            pos.done(to, c[to])
        }
        "ctm_bo_timing" => orders(x, from, to, |t, pos| {
            if pos.side() != 0. && t - pos.entry_bar().unwrap() >= 5 { pos.exit(t, c[t], "five days · close"); }
            if t < 9 { return vec![]; }
            let mut z = vec![];
            if pos.side() != 1. { z.push(ent(1., K::Stop, hi(h, t - 9, t))); }
            if pos.side() != -1. { z.push(ent(-1., K::Stop, lo(l, t - 9, t))); }
            z
        }),
        "ctm_mom20" => {
            let mut pos = Book::new();
            for t in from..=to {
                if pos.side() != 0. && t - pos.entry_bar().unwrap() >= 20 { pos.exit(t, c[t], "20 days · close"); }
                if pos.side() == 0. && t < to && t >= 20 && c[t] != c[t - 20] { pos.enter(if c[t] > c[t - 20] { 1. } else { -1. }, t, c[t], "close"); }
            }
            pos.done(to, c[to])
        }
        "ctm_sma12" => {
            let s: Vec<Vec<f64>> = [3, 5, 8, 10, 12, 15, 30, 35, 40, 45, 50, 60].iter().map(|&k| sma(c, k)).collect();
            let a20 = sma(&true_range(x.r), 20);
            let mut sa = f64::NAN;
            orders(x, from, to, |t, pos| {
                if !fin(s[11][t]) { return vec![]; }
                let up = (0..11).all(|k| s[k][t] > s[k + 1][t]);
                let dn = (0..11).all(|k| s[k][t] < s[k + 1][t]);
                let sd = pos.side();
                if sd == 0. {
                    if up || dn { sa = a20[t]; return vec![ent(if up { 1. } else { -1. }, K::Mkt, f64::NAN)]; }
                    return vec![];
                }
                let mut z = vec![];
                if (sd > 0. && !up) || (sd < 0. && !dn) { z.push(ext(-sd, K::Mkt, f64::NAN)); }
                z.push(ext(-sd, K::Stop, pos.entry_px() - sd * 2. * sa));
                z
            })
        }
        _ => return None,
    })
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::Row;
    pub fn walk(n: usize) -> Vec<Row> {
        let (mut s, mut c) = (987654321u64, 100f64);
        let mut u = || { s = s.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407); (s >> 11) as f64 / (1u64 << 53) as f64 };
        (0..n).map(|i| { let o = c * (1. + 0.012 * (u() - 0.5)); c = o * (1. + if i % 47 == 0 { 0.2 } else { 0.024 } * (u() - 0.48));
            let (h, l) = (o.max(c) * (1. + 0.006 * u()), o.min(c) * (1. - 0.006 * u()));
            Row { ticker: "T".into(), date: format!("2000-{:02}-{:02}", 1 + (i / 28) % 12, 1 + i % 28), open: o, high: h, low: l, close: c, volume: Some(1000. + (i % 7) as f64) } }).collect()
    }
    #[test]
    fn currency_trader_systems_trade() {
        let r = walk(4000);
        let x = Ctx::new(&r, &r, &[]);
        for id in REVIEWED {
            let t = run(id, &x, 0, r.len() - 1, &Params::new()).unwrap();
            assert!(!t.is_empty(), "{id}");
            for w in t.windows(2) { assert!(w[1].legs[0].i >= w[0].exit_i, "{id}"); }
            assert!(family_of(id).is_some() && params(id).is_some());
        }
        // inverse Martingale: legs 1/8, 1/8, 2/8, 4/8 at most
        for t in run("ctm_invmart", &x, 0, r.len() - 1, &Params::new()).unwrap() {
            assert!(t.legs.len() <= 4 && t.legs.iter().zip([0.125, 0.125, 0.25, 0.5]).all(|(l, w)| l.w == w));
        }
        assert!(run("ctm_x", &x, 0, 5, &Params::new()).is_none());
    }
}

// Instrument admission: Currency Trader 2006-10 2026-11-02 (titles lane K15).
pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book == "ctm" && reviewed(def.id),
    label: "Currency Trader systems tested on currency futures or spot FX pairs. Runs on daily indices, ETFs and stocks, US and India, are a declared application.",
    basis: "Currency Trader magazine Jan-Oct 2006 (Currency System Analysis) and 2008-2010 (Trading Strategies), article pages cited per card; independent check authors/titles-1030/K15w/k15_ref.py",
};
