//! Arthur G. Putt and William Brower, "Using EasyLanguage 2000" (research share USB HDD 2/tRADING KNO-2/Arthur G.Putt, William
//! Brower - Using EasyLanguage 2000/Using EL 2k by Arthur G. Putt & William Brower.pdf; printed page = PDF page - 7; text
//! authors/batch-g/G2w/putt_38_50.txt and putt_67_70.txt). Book id `pbuel`. The book's three daily tutorial systems with full code:
//! Sidebar 4:5 2% Reversal, Sidebar 4:4 Trend Daily 1.00 and Sidebar 6:2 RSI-%R Daily 1.00. Lane K12 (titles loop, 2 Nov 2026),
//! to-code rows 194-196; independent check authors/titles-1030/K12w/k12_ref.py.
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};
use super::lc3::{hi, high_first, lo};
use super::msws::{rsi, sma};
use super::tselhs::{india, over, under, Book};

/// Admitted ids (authors/titles-1030/K12w/k12_ref.py (header)).
pub const REVIEWED: &[&str] = &["pbuel_rev2", "pbuel_trend_daily", "pbuel_rsi_pr"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

macro_rules! common { () => { " TradeStation semantics: the system runs at each bar's close and its orders are for the next bar only (market = the open; a stop fills at its price, or at the open if the open is already through it; a limit at its price or the better open). Exit orders written under 'If MarketPosition = 1' are placed at a close where the position is held, so they act from the bar after the entry bar. Inside one bar the orders fill in the order the price reaches them: first the open (orders it is already through, exits before entries), then the nearer of the high and the low, then the other (lower sell levels after higher ones on the way down, higher buy levels after lower ones on the way up); an exit order only closes the position it was placed for. One position, no pyramiding: a same-direction entry is ignored, an opposite one reverses. A tutorial system from the book (no claim that it should be traded); the book's T-Bond, NYFE, IBM and soybean figures are its own reports, not check targets; daily bars on indices, ETFs and US/India stocks are a declared application; the vault's standard costs." } }
macro_rules! usd { () => { " Dollar amounts: 'MMstop/BigPointValue' on the book's contract basis, BigPointValue = 1 for stocks (p.37: 'a 1.0 move in a stock is equal to one dollar'), so $1 = 1 price point on 1 unit; p.lot = 100 divides by the 100-share lot of the book's LotSz example (p.37). US instruments only (lead decision 68); Indian instruments run without them ('dollar exit held'), as dollar amounts are not converted (decisions 19, 32, 58). 0 turns an amount off." } }

const REV: &str = concat!("[Declared readings] Sidebar 4:5 '2% Reversal System' (printed p.41-43): at each close, if Close > Close[5]: UpTrend, ShEntry = Highest(High,5) x 0.98 (this bar included); if Close < Close[5]: DnTrend, LgEntry = Lowest(Low,5) x 1.02; an equal close keeps the last trend and level. While DnTrend: Buy next bar at LgEntry on a stop; while UpTrend: Sell (short) next bar at ShEntry on a stop; each fill reverses the position, so after the first fill it is always in the market ('you are always in the market'). A buy stop already below the open fills at the open (TradeStation). The trend flags start empty on the first bar of a run. The book shows weekly IBM (251 trades, -$120.38 a share) and soybeans (49 trades, +$13,000); daily bars are the declared application, weekly is not offered (a stop fill inside a week has no daily date).", common!());
const TD: &str = concat!("[Declared readings] Sidebar 4:4 'Trend Daily 1.00' (printed p.39): MA = Average(Close,32); IF Close > MA and LowestBar(Low,3) = 0 (this bar's low is the lowest of the last 3, a tie counts) Then Buy next bar at High + 1 Point Stop; IF Close < MA and HighestBar(High,3) = 0 Then Sell (short) next bar at Low - 1 Point Stop. '1 Point' = one minimum price tick (0.01 US, 0.05 NSE; lead decision 57); p.tick = 0 removes it. Exits while long: profit limit at EntryPrice + PO/BigPointValue, stop at EntryPrice - MMstop/BigPointValue (short mirrored); Inputs Len(32), MMstop(400), PO(2100) as printed; the book's optimised T-Bond values $300 / $1,700 are options (p.mm, p.po). An opposite entry reverses. Option p.bars = 11: the 'Bar Limit' exit of version 1.03 (printed p.59-60: if BarsSinceEntry >= 11, exit next bar at market; the entry bar is 0); 1.03's other changes (2-bar order life, holiday date filter) are not applied. Book: daily T-Bonds Oct 1990 - Aug 1994, 48 trades, 54% profitable, net $11,843.75 (the text says those used $450 / $850).", usd!(), common!());
const RP: &str = concat!("[Declared readings] Sidebar 6:2 'RSI-%R Daily 1.00' (printed p.60-62): Inputs RSILen(6), BuyZone(20), SellZone(80), RLen(5), RMMstop(900). At each close, in the printed order: %R(5) > 80 sets SellFlag and clears BuyFlag; %R < 20 sets BuyFlag and clears SellFlag; while a position is held both flags are cleared; then %R > 20 and BuyFlag and %R crosses over RSI(Close,6) -> Buy next bar at market; %R < 80 and SellFlag and %R crosses under RSI -> Sell (short) next bar at market. Exits next bar at market: a long when %R crosses under RSI or %R > 80, a short when %R crosses over RSI or %R < 20; stop at EntryPrice -/+ RMMstop/BigPointValue. %R = TradeStation PercentR = (Close - lowest low of 5) / (highest high of 5 - lowest low) x 100, 0 when the range is 0, so low = oversold as the zones imply (the book: 'the inverse of the stochastic formula'). RSI = Wilder's (seeded with the mean of the first 6 changes). Crosses over = above now, at or below on the bar before. The flags start empty on the first bar of a run. Not applied: version 1.01's Thanksgiving and Christmas date filter. Book: 4-year daily T-Bonds, 44 trades, 55% profitable, net $6,218.75.", usd!(), " Lead decision 195: the $900 stop is part of the rule, so this card is published only on US index and ETF prices (where the amount runs literally); results without the stop on other instruments are withdrawn, and US single stocks are withheld while only split-adjusted prices exist (decision 166).", common!());

pub fn catalogue() -> Vec<StrategyDef> {
    let d = |id, name, rules, notes| StrategyDef { id, name, group: "Using EasyLanguage 2000 (Putt & Brower) tutorial systems", session: 0, dir: "Long/Short", status: Status::Live,
        needs_volume: false, needs_vix: false, needs_series: &[], book: "pbuel", rules, notes, claim: None };
    vec![
        d("pbuel_rev2", "2% Reversal: always in, buy 2% above the 5-bar low in a 5-bar downtrend, sell short 2% under the 5-bar high in an uptrend, on stops", &[
            "F|\"If Close > Close[5] Then Begin UpTrend = 1; DnTrend = 0; HiPrice = Highest(High,5); Hi2 = HiPrice * .02; ShEntry = HiPrice - Hi2; End; If Close < Close[5] Then Begin UpTrend = 0; DnTrend = 1; LoPrice = Lowest(Low,5); Lo2 = LoPrice * .02; LgEntry = LoPrice + Lo2; End;\" (Sidebar 4:5)",
            "E|\"If DnTrend = 1 Then Buy(\\\"2% LE\\\") at LgEntry Stop; If UpTrend = 1 Then Sell(\\\"2% SE\\\") at ShEntry Stop;\" (Sidebar 4:5)",
            "X|\"With this system, you are always in the market\": each stop fill reverses the position (p.41)."], REV),
        d("pbuel_trend_daily", "Trend Daily 1.00: above the 32-day average buy a stop over the high of a 3-day low bar, below it sell a stop under the low of a 3-day high bar; dollar stop and target (US)", &[
            "E|\"IF Close > MA and LowestBar(Low,3) = 0 Then Buy(\\\"Trend LE\\\") at High + 1 Point Stop; IF Close < MA and HighestBar(High,3) = 0 Then Sell(\\\"Trend SE\\\") at Low - 1 Point Stop;\" Inputs: Len(32), MMstop(400), PO(2100); MA = Average(Close,Len) (Sidebar 4:4)",
            "X|\"If MarketPosition = 1 then ExitLong(\\\"Trend LPO\\\") at EntryPrice + (PO/BigPointValue) Limit; ... ExitLong(\\\"Trend LX\\\") at EntryPrice - (MMSTOP/BigPointValue) Stop;\" short mirrored (Sidebar 4:4)"], TD),
        d("pbuel_rsi_pr", "RSI-%R Daily 1.00: after %R(5) dips under 20, buy when it crosses over RSI(6); after it tops 80, short when it crosses under; out on the cross back or the opposite zone", &[
            "F|\"If PercentR(RLen) > SellZone then begin SellFlag = True; BuyFlag = False; End; If PercentR(RLen) < BuyZone then begin SellFlag = False; BuyFlag = True; End; If MarketPosition <> 0 then begin SellFlag = False; BuyFlag = False; End;\" Input: RSILen(6), BuyZone(20), SellZone(80), RLen(5), RMMstop(900) (Sidebar 6:2)",
            "E|\"If PercentR(RLen) > BuyZone and BuyFlag and PercentR(RLen) crosses over RSI(C,RSILen) then Buy(\\\"RSI-%R LE\\\") at Market; If PercentR(RLen) < SellZone and SellFlag and PercentR(RLen) crosses under RSI(C,RSILen) then Sell(\\\"RSI-%R SE\\\") at Market;\" (Sidebar 6:2)",
            "X|\"If PercentR(RLen) crosses under RSI(C,RSILen) then ExitLong at Market; If PercentR(RLen) crosses over RSI(C,RSILen) then ExitShort at Market; If PercentR(RLen) > SellZone then ExitLong Market; If PercentR(RLen) < BuyZone then ExitShort Market;\" stop \"EntryPrice - (RMMstop/BigPointValue) Stop\" (Sidebar 6:2)"], RP),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    let f = match id {
        "pbuel_rev2" => ("Putt & Brower 2% Reversal", "5-bar trend, 2% stops"),
        "pbuel_trend_daily" => ("Putt & Brower Trend Daily", "32-day average, 3-bar setups"),
        "pbuel_rsi_pr" => ("Putt & Brower RSI-%R Daily", "%R(5) over RSI(6)"),
        _ => return None,
    };
    let id = REVIEWED.iter().find(|r| **r == id)?;
    Some((*id, f.0, f.1))
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    let lot = || p("lot", "Units the dollar amounts are spread over (1 = the code's contract basis; 100 = the book's 100-share lot)", 1., 1., 100., 99.);
    Some(match id {
        "pbuel_rev2" => vec![],
        "pbuel_trend_daily" => vec![p("mm", "Dollar stop MMstop, US only (400 printed; 300 optimised; 0 off)", 400., 0., 5000., 50.),
            p("po", "Dollar profit target PO, US only (2100 printed; 1700 optimised; 0 off)", 2100., 0., 10000., 100.), lot(),
            p("tick", "Entry stop offset in ticks (1 = '1 Point'; 0 = none)", 1., 0., 10., 1.),
            p("bars", "Bar Limit exit, bars after entry (0 off; 11 = version 1.03)", 0., 0., 100., 1.)],
        "pbuel_rsi_pr" => vec![p("mm", "Dollar stop RMMstop, US only (900 printed; 0 off)", 900., 0., 5000., 50.), lot()],
        _ => return None,
    })
}

fn g(p: &Params, k: &str, id: &str) -> f64 {
    p.get(k).copied().unwrap_or_else(|| params(id).and_then(|s| s.iter().find(|d| d.key == k).map(|d| d.def)).unwrap_or(f64::NAN))
}

/// A next-bar order. `up`: it fills when the price rises to `px` (buy stop, sell limit), else when it falls to it.
/// `exit`: closes a position of side `d` held before this bar; otherwise enters side `d`. `px` NaN = at market (the open).
#[derive(Clone, Copy)]
struct Ord { up: bool, px: f64, d: f64, exit: bool, why: &'static str }

/// Fill bar i's orders in the order the price path reaches them (see the notes).
fn fill(x: &Ctx, i: usize, pos: &mut Book, ords: &[Ord]) {
    let (o, h, l) = (x.o[i], x.h[i], x.l[i]);
    let act = |pos: &mut Book, z: &Ord, px: f64| {
        if z.exit { if pos.side() == z.d && pos.entry_bar().is_some_and(|e| e < i) { pos.exit(i, px, z.why); } }
        else if pos.side() != z.d { pos.enter(z.d, i, px, z.why); }
    };
    let at_open = |z: &Ord| z.px.is_nan() || if z.up { o >= z.px } else { o <= z.px };
    for ex in [true, false] { for z in ords.iter().filter(|z| z.exit == ex && at_open(z)) { act(pos, z, o); } }
    let legs = if high_first(o, h, l) { [true, false] } else { [false, true] };
    for up in legs {
        let mut v: Vec<&Ord> = ords.iter().filter(|z| z.up == up && !at_open(z) && if up { h >= z.px } else { l <= z.px }).collect();
        v.sort_by(|a, b| if up { a.px.total_cmp(&b.px) } else { b.px.total_cmp(&a.px) });
        for z in v { act(pos, z, z.px); }
    }
}

/// TradeStation PercentR(n) = (C - lowest low) / (highest high - lowest low) x 100, 0 when the range is 0.
pub fn percent_r(x: &Ctx, n: usize) -> Vec<f64> {
    (0..x.c.len()).map(|i| if i + 1 < n { f64::NAN } else {
        let (hh, ll) = (hi(&x.h, i + 1 - n, i), lo(&x.l, i + 1 - n, i));
        if hh - ll > 0. { (x.c[i] - ll) / (hh - ll) * 100. } else { 0. }
    }).collect()
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.r.len() { return None; }
    let (h, l, c) = (&x.h, &x.l, &x.c);
    let mut pos = Book::new();
    let mut ords: Vec<Ord> = vec![];
    let lev = |up, px, d, exit, why| Ord { up, px, d, exit, why };
    // dollar amounts in points: US only (lead decision 68)
    let pts = |k: &str| { let v = g(p, k, id) / g(p, "lot", id).max(1.); (!india(x) && v > 0.).then_some(v) };
    let (mut up, mut dn, mut lg, mut sh) = (false, false, f64::NAN, f64::NAN); // 2% Reversal state
    let (mut bf, mut sf) = (false, false); // RSI-%R flags
    let ma = if id == "pbuel_trend_daily" { sma(c, 32) } else { vec![] };
    let (pr, rs) = if id == "pbuel_rsi_pr" { (percent_r(x, 5), rsi(c, 6)) } else { (vec![], vec![]) };
    for t in from..=to {
        if t > from { fill(x, t, &mut pos, &std::mem::take(&mut ords)); }
        if t == to { break; }
        let (d, held) = (pos.side(), pos.entry_bar());
        match id {
            "pbuel_rev2" => {
                if t >= 5 && c[t] > c[t - 5] { (up, dn, sh) = (true, false, hi(h, t - 4, t) * 0.98); }
                else if t >= 5 && c[t] < c[t - 5] { (up, dn, lg) = (false, true, lo(l, t - 4, t) * 1.02); }
                if dn { ords.push(lev(true, lg, 1., false, "2% LE stop")); }
                if up { ords.push(lev(false, sh, -1., false, "2% SE stop")); }
            }
            "pbuel_trend_daily" => {
                let tick = g(p, "tick", id).round().max(0.) * x.market_tick();
                if t >= 2 && c[t] > ma[t] && l[t] <= l[t - 1] && l[t] <= l[t - 2] { ords.push(lev(true, h[t] + tick, 1., false, "Trend LE stop")); }
                if t >= 2 && c[t] < ma[t] && h[t] >= h[t - 1] && h[t] >= h[t - 2] { ords.push(lev(false, l[t] - tick, -1., false, "Trend SE stop")); }
                if d != 0. {
                    let ep = pos.entry_px();
                    if let Some(v) = pts("po") { ords.push(lev(d > 0., ep + d * v, d, true, "profit objective")); }
                    if let Some(v) = pts("mm") { ords.push(lev(d < 0., ep - d * v, d, true, "dollar stop")); }
                    let bars = g(p, "bars", id).round();
                    if bars > 0. && held.is_some_and(|e| (t - e) as f64 >= bars) { ords.push(lev(true, f64::NAN, d, true, "bar limit")); }
                }
            }
            "pbuel_rsi_pr" => {
                if pr[t] > 80. { (sf, bf) = (true, false); }
                if pr[t] < 20. { (sf, bf) = (false, true); }
                if d != 0. { (sf, bf) = (false, false); }
                if pr[t] > 20. && bf && over(&pr, &rs, t) { ords.push(lev(true, f64::NAN, 1., false, "RSI-%R LE")); }
                if pr[t] < 80. && sf && under(&pr, &rs, t) { ords.push(lev(true, f64::NAN, -1., false, "RSI-%R SE")); }
                if (d > 0. && (under(&pr, &rs, t) || pr[t] > 80.)) || (d < 0. && (over(&pr, &rs, t) || pr[t] < 20.)) {
                    ords.push(lev(true, f64::NAN, d, true, "cross back or opposite zone"));
                }
                if d != 0. { if let Some(v) = pts("mm") { ords.push(lev(d < 0., pos.entry_px() - d * v, d, true, "dollar stop")); } }
            }
            _ => return None,
        }
    }
    Some(pos.done(to, c[to]))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;
    fn bars(t: &str) -> Vec<Row> {
        (0..1200).map(|i| { let f = i as f64; let c = 100. + 12. * (f / 23.).sin() + 5. * (f / 6.1).cos() + 0.03 * f;
            Row { ticker: t.into(), date: format!("{:05}", i), open: c - 0.7 * (f / 2.3).sin(), high: c + 1.4 + 0.3 * (f / 3.7).cos(), low: c - 1.2, close: c, volume: None } }).collect()
    }
    #[test]
    fn putt_brower_tutorial_systems() {
        let r = bars("T");
        let x = Ctx::new(&r, &r, &[]);
        for id in REVIEWED {
            let t = run(id, &x, 0, r.len() - 1, &Params::new()).unwrap();
            assert!(t.len() > 5 && t.iter().any(|z| z.dir > 0.) && t.iter().any(|z| z.dir < 0.), "{id}");
            for w in t.windows(2) { assert!(w[1].legs[0].i >= w[0].exit_i, "{id}"); }
            assert!(family_of(id).is_some() && params(id).is_some());
        }
        // 2% Reversal is always in after its first fill
        let t = run("pbuel_rev2", &x, 0, r.len() - 1, &Params::new()).unwrap();
        for w in t.windows(2) { assert_eq!(w[1].legs[0].i, w[0].exit_i); }
        // Trend Daily: a 1-point dollar stop on a US ticker exits; on a NIFTY ticker the dollar exits are not applied
        let mut tight = Params::new(); tight.insert("mm".into(), 1.);
        assert!(run("pbuel_trend_daily", &x, 0, r.len() - 1, &tight).unwrap().iter().any(|z| z.reason == "dollar stop"));
        let ri = bars("NIFTY 50");
        let xi = Ctx::new(&ri, &ri, &[]);
        assert!(run("pbuel_trend_daily", &xi, 0, ri.len() - 1, &tight).unwrap().iter().all(|z| z.reason != "dollar stop"));
        // path order: long from 100, stop 95 and target 110 on a bar 108 -> 112 -> 94 (high nearer) takes the target
        let mut pos = Book::new(); pos.enter(1., 0, 100., "x");
        let rb: Vec<Row> = [(100., 112., 94.), (108., 112., 94.)].iter().enumerate().map(|(i, &(o, h, l))| Row { ticker: "T".into(), date: format!("{i}"), open: o, high: h, low: l, close: o, volume: None }).collect();
        let xb = Ctx::new(&rb, &rb, &[]);
        fill(&xb, 1, &mut pos, &[Ord { up: false, px: 95., d: 1., exit: true, why: "s" }, Ord { up: true, px: 110., d: 1., exit: true, why: "t" }]);
        assert_eq!((pos.out[0].exit_px, pos.side()), (110., 0.));
        assert!(run("pbuel_x", &x, 0, 5, &Params::new()).is_none());
    }
}

// Instrument admission: Using EasyLanguage 2000 2026-11-02 (titles lane K12).
pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book == "pbuel" && reviewed(def.id),
    label: "Tutorial systems from Putt and Brower's Using EasyLanguage 2000, shown on daily T-Bonds, weekly IBM and soybeans. Runs on daily indices, ETFs and stocks, US and India, are a declared application; the dollar stop and target run on US instruments only.",
    basis: "Using EasyLanguage 2000 (Putt & Brower), Sidebars 4:4, 4:5 and 6:2, printed p.39-43 and p.60-62 (PDF p.46-50, 67-70); independent check authors/titles-1030/K12w/k12_ref.py",
};
