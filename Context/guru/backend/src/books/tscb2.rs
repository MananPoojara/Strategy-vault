//! Mark Melzl (compiler), "The TradeStation Cookbook II" (GraphFax, May 1997; scanned; book id `tscb2`): a code listing of
//! TradeStation 4.0 indicators and systems, no explanations or results. Reading: claude-continuation-20261020/authors/melzl
//! (OCR of the SYSTEMS section PDF 97-122, unclear lines checked against page images); independent reference
//! authors/lanec3/ref/lanec3_ref.py. Admitted (printed code complete, daily bars): James Bond, Daily Smash YX, BB AVG,
//! Consecutive Closes, CCI 200, Average Volume. Held: Super Balloon (printed condition 8 reads CLOSE[12], an evident typo),
//! CCI ROC crossover (zones look inverted), systems whose functions or pages are missing in the scan, intraday systems.
use crate::books::lc3::{hi, lo, sma, Pos};
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};

pub const REVIEWED: &[&str] = &["tscb2_bond", "tscb2_smash", "tscb2_bbavg", "tscb2_consec", "tscb2_cci200", "tscb2_avgvol"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const G: &str = "TradeStation Cookbook II systems (1997)";
const TS: &str = "TradeStation conventions: 'at market' = the next day's open; 'on close' = the signal day's close; 'buy'/'sell' reverse an open opposite position; no pyramiding; one unit, no costs. Shorts on a cash index are hypothetical.";

fn d(id: &'static str, name: &'static str, dir: &'static str, vol: bool, rules: &'static [&'static str], notes: &'static str) -> StrategyDef {
    StrategyDef { id, name, group: G, session: 0, dir, status: Status::Live, needs_volume: vol, needs_vix: false, needs_series: &[], book: "tscb2", rules, notes, claim: None }
}

pub fn catalogue() -> Vec<StrategyDef> {
    vec![
        d("tscb2_bond", "James Bond: short when 100-day Fast %K is above 97, buy when below 2 (stop and reverse at the next open)", "Long/Short", false,
            &["E|\"if FASTK(100) > +97 then sell at market; if FASTK(100) < 2 then buy at market;\" (printed p.101, PDF 99)", "X|No separate exit in the code: the opposite signal reverses the position (stop and reverse)"],
            "Fast %K(100) = 100 x (close - lowest low of 100 days) / (highest high - lowest low), days including today. The input length(100) is printed but not used by the code."),
        d("tscb2_smash", "Daily Smash YX: higher high but close below yesterday's low, buy the next open if it opens below that close; out on the 7th day's close or a 3000-point money stop", "Long", false,
            &["E|\"If High > High of yesterday and Close of today < Low of yesterday and open of tomorrow < Close of today Then Buy tomorrow at market;\" (printed p.103, PDF 101)",
              "X|\"Inputs: ExitBar(7), MMStop(3000); IncludeSystem:\\\"x bar Exit\\\",ExitBar; IncludeSystem:\\\"MM Stop not entrybar\\\",MMStop;\" - x bar exit: \"BarsSinceEntry = ExitBar Then ExitLong this bar on close\"; stop: \"ExitLong tomorrow at EntryPrice - StopAmt / BigPointValue stop\" (p.103)"],
            "Exit at the close of the 7th day after the entry day (BarsSinceEntry counts the entry day as 0). The money stop is $3000 / BigPointValue; on a stock, ETF or index chart TradeStation's BigPointValue is 1, so the stop is 3000 price points below the entry (p.stop), working from the day after entry, filled at the stop or a lower open. A dollar-amount rule: US instruments only. 'YX' = NYSE Composite futures (the name only; the code says 'This system works on daily data')."),
        d("tscb2_bbavg", "BB AVG: the average price crosses up through the lower 40-day 2-sd Bollinger band = buy; down through the upper band = sell short (next open)", "Long/Short", false,
            &["E|\"INPUTS:PRICE(AVGPRICE),LENGTH1(40),SDEVUP(2),SDEVDN(-2); CONDITION1= AVGPRICE CROSSES ABOVE BOLLINGERBAND(PRICE,LENGTH1,SDEVDN); IF CONDITION1 THEN BUY AT MARKET; CONDITION2= AVGPRICE CROSSES BELOW BOLLINGERBAND(PRICE,LENGTH1,SDEVUP); IF CONDITION2 THEN SELL AT MARKET;\" (printed p.105, PDF 103)", "X|No separate exit in the code: the opposite signal reverses the position (stop and reverse)"],
            "AvgPrice = (open + high + low + close) / 4; band = 40-day simple average of AvgPrice +/- 2 population standard deviations (TradeStation StdDev). 'Crosses above' = today above and yesterday at or below."),
        d("tscb2_consec", "Consecutive Closes: three higher closes in a row = buy on the close; three lower closes = sell short on the close", "Long/Short", false,
            &["E|\"Input: LEConsec(3),SEConsec(3); IF MRO(Close <= Close[1],LEConsec,1) = -1 Then Buy on Close; IF MRO(Close >= Close[1],SEConsec,1) = -1 Then Sell on Close;\" (printed p.117, PDF 115)", "X|No separate exit in the code: the opposite signal reverses the position (stop and reverse)"],
            "MRO(...) = -1: the condition was false on each of the last 3 days including today, i.e. each close above the one before (below, for the short)."),
        d("tscb2_cci200", "CCI 200: 30-day CCI at or above +175 = sell short, at or below -150 = buy (stop and reverse at the next open)", "Long/Short", false,
            &["E|\"if cci(30) >= +175 then sell at market; if cci(30) <= -150 then buy at market;\" (printed p.119, PDF 117)", "X|No separate exit in the code: the opposite signal reverses the position (stop and reverse)"],
            "CCI(30) = (typical price - its 30-day average) / (0.015 x mean absolute deviation), typical price = (high + low + close) / 3 (TradeStation CCI). The input length(100) is printed but not used."),
        d("tscb2_avgvol", "Average Volume: 7-day average volume above the 32-day = buy stop 1 tick above today's high; below = sell stop 1 tick under today's low (stop and reverse)", "Long/Short", true,
            &["E|\"INPUT: LENGTH1(7),LENGTH2(32); VALUE1=AVERAGE(volume,LENGTH1)-AVERAGE(volume,LENGTH2); if value1>0 then buy H + 1 points stop; if value1<0 then sell L - 1 Points stop;\" (printed p.121, PDF 119)", "X|No separate exit in the code: the opposite signal reverses the position (stop and reverse)"],
            "The stop order is placed after today's close for tomorrow only and re-placed each day while the volume condition holds; it fills at the stop or at a worse open. '1 point' = one minimum price move (0.01 US, 0.05 NSE). Needs volume (stocks and ETFs; indices without volume are refused)."),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    let v = match id { "tscb2_bond" => "James Bond", "tscb2_smash" => "Daily Smash YX", "tscb2_bbavg" => "BB AVG", "tscb2_consec" => "Consecutive Closes", "tscb2_cci200" => "CCI 200", "tscb2_avgvol" => "Average Volume", _ => return None };
    REVIEWED.iter().find(|r| **r == id).map(|r| (*r, "TradeStation Cookbook II systems", v))
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    match id {
        "tscb2_smash" => Some(vec![
            ParamDef { key: "exitbar", label: "Exit on the close of this day after entry (book: 7)", def: 7., min: 1., max: 30., step: 1. },
            ParamDef { key: "stop", label: "Money stop in price points below the entry (book: 3000 / BigPointValue = 3000 on a stock chart; 0 = none)", def: 3000., min: 0., max: 5000., step: 1. }]),
        _ if reviewed(id) => Some(vec![]),
        _ => None,
    }
}

fn cci(x: &Ctx, n: usize, to: usize) -> Vec<f64> {
    let tp: Vec<f64> = (0..=to).map(|i| (x.h[i] + x.l[i] + x.c[i]) / 3.).collect();
    let m = sma(&tp, n);
    (0..=to).map(|i| if !m[i].is_finite() { f64::NAN } else {
        let md = tp[i + 1 - n..=i].iter().map(|v| (v - m[i]).abs()).sum::<f64>() / n as f64;
        if md > 0. { (tp[i] - m[i]) / (0.015 * md) } else { 0. }
    }).collect()
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.r.len() { return None; }
    let (o, h, l, c) = (&x.o, &x.h, &x.l, &x.c);
    let mut pos = Pos::new();
    // sig[i]: +1 buy / -1 sell decided at the close of day i
    let mut sig = vec![0i8; to + 1];
    match id {
        "tscb2_bond" => for i in 99..=to {
            let (hh, ll) = (hi(h, i - 99, i), lo(l, i - 99, i));
            if hh > ll { let k = 100. * (c[i] - ll) / (hh - ll); if k > 97. { sig[i] = -1 } else if k < 2. { sig[i] = 1 } }
        },
        "tscb2_cci200" => { let v = cci(x, 30, to); for i in 0..=to { if v[i] >= 175. { sig[i] = -1 } else if v[i] <= -150. { sig[i] = 1 } } }
        "tscb2_bbavg" => {
            let ap: Vec<f64> = (0..=to).map(|i| (o[i] + h[i] + l[i] + c[i]) / 4.).collect();
            let m = sma(&ap, 40);
            let band = |i: usize, k: f64| { let sd = (ap[i - 39..=i].iter().map(|v| (v - m[i]).powi(2)).sum::<f64>() / 40.).sqrt(); m[i] + k * sd };
            for i in 40..=to { if m[i - 1].is_finite() {
                if ap[i] > band(i, -2.) && ap[i - 1] <= band(i - 1, -2.) { sig[i] = 1 }
                // both crossings on one day: the later order (sell) is the one TradeStation keeps
                if ap[i] < band(i, 2.) && ap[i - 1] >= band(i - 1, 2.) { sig[i] = -1 }
            } }
        }
        "tscb2_consec" => {
            for i in 3..=to {
                let up = (0..3).all(|k| c[i - k] > c[i - k - 1]);
                let dn = (0..3).all(|k| c[i - k] < c[i - k - 1]);
                if i >= from { if up { pos.enter(1., i, c[i], "close"); } else if dn { pos.enter(-1., i, c[i], "close"); } }
            }
            return Some(pos.done(to, c[to]));
        }
        "tscb2_avgvol" => {
            let v: Vec<f64> = x.r[..=to].iter().map(|r| r.volume.unwrap_or(f64::NAN)).collect();
            let (a7, a32) = (sma(&v, 7), sma(&v, 32));
            let tk = x.market_tick();
            for i in (from + 1)..=to {
                let j = i - 1;
                if !(a7[j].is_finite() && a32[j].is_finite()) { continue; }
                let d = a7[j] - a32[j];
                if d > 0. && pos.side() <= 0. { let s = h[j] + tk; if h[i] >= s { pos.enter(1., i, o[i].max(s), "stop"); } }
                else if d < 0. && pos.side() >= 0. { let s = l[j] - tk; if l[i] <= s { pos.enter(-1., i, o[i].min(s), "stop"); } }
            }
            return Some(pos.done(to, c[to]));
        }
        "tscb2_smash" => {
            let g = |k: &str, d: f64| p.get(k).copied().unwrap_or(d);
            let (xb, st) = (g("exitbar", 7.).round().max(1.) as usize, g("stop", 3000.));
            let mut e: Option<(usize, f64)> = None;
            for i in (from + 1).max(2)..=to {
                let j = i - 1;
                // open: buy if flat and yesterday (inside the window) was a smash day and today opens below its close
                if e.is_none() && h[j] > h[j - 1] && c[j] < l[j - 1] && o[i] < c[j] { pos.enter(1., i, o[i], "open"); e = Some((i, o[i])); }
                if let Some((ei, ep)) = e {
                    if ei < i && st > 0. && l[i] <= ep - st { pos.exit(i, o[i].min(ep - st), "money stop"); e = None; }
                    else if i - ei == xb { pos.exit(i, c[i], "x bar exit"); e = None; }
                }
            }
            return Some(pos.done(to, c[to]));
        }
        _ => return None,
    }
    for i in from..=to {
        if i >= 1 && i - 1 >= from {
            match sig[i - 1] { 1 => { pos.enter(1., i, o[i], "open"); } -1 => { pos.enter(-1., i, o[i], "open"); } _ => {} }
        }
    }
    Some(pos.done(to, c[to]))
}
