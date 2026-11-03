//! NewMarket Software (tradingalliance.com), "How to build your own Trading System" course, author NAVIGATOR (research share
//! Trading Courses,Seminars,Videos/Trading Systems Course (tradingalliance.com)/ts/DM.pps and PARABS.pps, PowerPoint 97 slides; slide
//! text authors/batch-e/E11w/ta_DM.txt and authors/batch-f/F1w/PARABS.txt). Book id `nmstc`. System 2 'DM Trading System' (TD-line
//! close breaks) and System 4 'Parabolic System' (simple version), labelled teaching examples (R4 decisions 49, 61, 62). Lane K11
//! (titles loop, 1 Nov 2026), to-code rows 190-191; independent check authors/titles-1030/K11w/k11_ref.py.
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};
use super::msws::{ema, sar};
use super::rbtie::target_trades;
use super::tselhs::Book;

/// Admitted ids (authors/titles-1030/K11w/k11_ref.py (header)).
pub const REVIEWED: &[&str] = &["nmstc_dm", "nmstc_parabs"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

macro_rules! common { () => { " A course example (the slides say the systems are for learning to build your own); the slides' futures examples (Euro FX, S&P 500) become indices, ETFs and US/India stocks as a declared application; one unit, no pyramiding; the vault's standard costs." } }

const DM: &str = concat!("[Declared readings] TD point high = a bar whose high is above the highs of the bars before and after it; TD point low = a low below the lows on both sides; a TD point is known after the next bar closes. TD resistance = the line through the two most recent TD point highs, extended to the right, drawn only when it is horizontal or slopes down (RULES 1 and 3); TD support = through the two most recent TD point lows, only when horizontal or sloping up (RULE 2); when the latest pair slopes the wrong way there is no line on that side until a new TD point gives a valid pair (DeMark's own pairing with an earlier point is not used). 'BUY when the bar closes above the TD resistance' = a close above today's line value after a close at or below it the day before, filled at that close; 'Sell when the bar closes below the TD support' mirrors it. '3a. EXIT with a protective stop: Place a sell-stop below the lowest price of the entry bar': a stop exactly at the entry bar's low (no tick offset, not trailed), from the next day, filled at the stop or at the open on a gap; shorts at the entry bar's high. '3b. EXIT with a profit: on the close of the breakout day' = the close that breaks the opposite line (the worked example covers each short 'when the price closed above the new TD resistance'), which also enters the other way. A close that breaks both lines at once is no signal. Not applied: the ADX filter (no length or level printed), the 'next day's retracement' discretion, the extra contract on each new line break, the trailed stop of the example.", common!());
const PB: &str = concat!("[Declared readings] 'Every time Parabolic is below the Exponential Moving Average (EMA) is a BUY signal, when it's above it is a SELL signal'; the simple version: 'buy when the price is in the BUY zone and sell when the price is in the SELL zone' ('The Parabolic system is ready!'). Read: always in; at the first close in a new BUY zone (SAR below the EMA of the close) go long, covering a short; at the first close in a SELL zone go short; on a bar where the two are equal the zone does not change; a run starts with the first zone change inside it. EMA length: the slides print 'a short-term EMA (5-8 period)'; 5 is the labelled default, 6-8 are options (lead decision 61). Parabolic SAR = Wilder's definition, acceleration 0.02 step 0.02, maximum 0.20 (decision 61), started on the second bar in the direction of its close against the first bar's close at the first bar's extreme. EMA alpha 2/(n+1), started at the first close. Fill at the zone-change close; p.open = 1 fills at the next open. Not coded: the 'channel trading' Rules 1-3 version (its channel line length is not printed; decision 62), the DMI filter ('we recommend', no settings) and the 'use common sense' sideways discretion. The slides' S&P 500 futures example (25 trades, 16 winners, 997 points in two years) is the author's report, not a check target.", common!());

pub fn catalogue() -> Vec<StrategyDef> {
    let d = |id, name, rules, notes| StrategyDef { id, name, group: "NewMarket Software trading systems course (tradingalliance.com)", session: 0, dir: "Long/Short", status: Status::Live,
        needs_volume: false, needs_vix: false, needs_series: &[], book: "nmstc", rules, notes, claim: None };
    vec![
        d("nmstc_dm", "DM Trading System: buy a close above the TD resistance line, sell a close below the TD support line, stop at the entry bar's low (high)", &[
            "F|\"A TD point on the bar high is higher than the highs of the two bars next to it.\" \"RULE 1. When drawing a TD line always connect two consecutive TD points. RULE 2. Draw only TD support lines that are horizontal or have a slope up. RULE 3. Accordingly, draw only TD resistance lines that are horizontal or have a slope down.\" (DM.pps)",
            "E|\"BUY when the bar closes above the TD resistance\"; \"Sell the breakout on the daily close below the TD support.\" (DM SYSTEM. TRADING SIGNALS)",
            "X|\"3a. EXIT with a protective stop: Place a sell-stop below the lowest price of the entry bar. 3b. EXIT with a profit: on the close of the breakout day.\" \"5a. EXIT with a protective stop: Place a stop above broken support or above the highest price of the entry bar.\""], DM),
        d("nmstc_parabs", "Parabolic System: long while the Parabolic SAR is below the 5-day EMA, short while it is above, stop and reverse at the close", &[
            "F|\"apply a secondary indicator, such as a short-term EMA (5-8 period) to the chart. Every time Parabolic is below the Exponential Moving Average (EMA) is a BUY signal, when it's above it is a SELL signal.\" (PARABS.pps)",
            "E|\"The rules of this simple version of the Parabolic system are: buy when the price is in the BUY zone and sell when the price is in the SELL zone. The Parabolic system is ready!\" (PARABS.pps)",
            "X|Stop and reverse: the first close in a SELL zone closes the long and sells short, the first close in a BUY zone covers and buys (always in the market)."], PB),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    let f = match id {
        "nmstc_dm" => ("NewMarket course: DM Trading System", "TD lines, entry-bar stop"),
        "nmstc_parabs" => ("NewMarket course: Parabolic System", "SAR vs EMA 5 zones"),
        _ => return None,
    };
    let id = REVIEWED.iter().find(|r| **r == id)?;
    Some((*id, f.0, f.1))
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    Some(match id {
        "nmstc_dm" => vec![],
        "nmstc_parabs" => vec![p("ema", "EMA length (5; the slides print 5-8)", 5., 5., 8., 1.), p("open", "Fill: 0 = the zone-change close, 1 = the next open", 0., 0., 1., 1.)],
        _ => return None,
    })
}

/// Value on bar j of the line through the two most recent TD points (k1 < k2) of one side, if that pair is allowed (`ok`).
fn line(pts: &[usize], v: &[f64], ok: fn(f64, f64) -> bool, j: usize) -> Option<f64> {
    let (&k2, &k1) = (pts.last()?, pts.get(pts.len().checked_sub(2)?)?);
    ok(v[k1], v[k2]).then(|| v[k2] + (j as f64 - k2 as f64) * (v[k2] - v[k1]) / ((k2 - k1) as f64))
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.r.len() { return None; }
    let (o, h, l, c) = (&x.o, &x.h, &x.l, &x.c);
    match id {
        "nmstc_dm" => {
            let mut pos = Book::new();
            let (mut th, mut tl) = (vec![], vec![]);
            for i in 0..=to {
                if i >= 2 {
                    let k = i - 1;
                    if h[k] > h[k - 1] && h[k] > h[k + 1] { th.push(k); }
                    if l[k] < l[k - 1] && l[k] < l[k + 1] { tl.push(k); }
                }
                if i < from || i == 0 { continue; }
                if let Some(e) = pos.entry_bar().filter(|e| *e < i) {
                    if pos.side() > 0. && l[i] <= l[e] { pos.exit(i, o[i].min(l[e]), "stop under the entry bar"); }
                    else if pos.side() < 0. && h[i] >= h[e] { pos.exit(i, o[i].max(h[e]), "stop over the entry bar"); }
                }
                let res = |j| line(&th, h, |a, b| b <= a, j);
                let sup = |j| line(&tl, l, |a, b| b >= a, j);
                let up = matches!((res(i), res(i - 1)), (Some(a), Some(b)) if c[i] > a && c[i - 1] <= b);
                let dn = matches!((sup(i), sup(i - 1)), (Some(a), Some(b)) if c[i] < a && c[i - 1] >= b);
                if up != dn { pos.enter(if up { 1. } else { -1. }, i, c[i], "close"); }
            }
            Some(pos.done(to, c[to]))
        }
        "nmstc_parabs" => {
            let n = p.get("ema").copied().unwrap_or(5.).round().clamp(2., 50.) as usize;
            let (e, s) = (ema(c, n), sar(x, 0.02, 0.2));
            let mut z = 0i8;
            let tgt: Vec<i8> = (0..c.len()).map(|i| { if s[i] < e[i] { z = 1; } else if s[i] > e[i] { z = -1; } z }).collect();
            Some(target_trades(x, &tgt, from, to, p.get("open").copied().unwrap_or(0.) > 0.5, "zone changed"))
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;
    #[test]
    fn dm_and_parabolic_systems() {
        let r: Vec<Row> = (0..1500).map(|i| { let f = i as f64; let c = 100. + 20. * (f / 90.).sin() + 4. * (f / 6.3).sin() + 2. * (f / 2.7).cos();
            Row { ticker: "T".into(), date: format!("{:05}", i), open: c - 0.4 * (f / 1.9).sin(), high: c + 1.2 + 0.3 * (f / 4.1).sin(), low: c - 1.2, close: c, volume: None } }).collect();
        let x = Ctx::new(&r, &r, &[]);
        for id in REVIEWED {
            let t = run(id, &x, 0, r.len() - 1, &Params::new()).unwrap();
            assert!(t.len() > 5 && t.iter().any(|z| z.dir > 0.) && t.iter().any(|z| z.dir < 0.), "{id}");
            for w in t.windows(2) { assert!(w[1].legs[0].i >= w[0].exit_i, "{id}"); }
            for z in &t { assert_eq!(z.legs[0].px, x.c[z.legs[0].i], "{id}"); }
            assert!(family_of(id).is_some() && params(id).is_some());
        }
        // a downward resistance pair: highs 10 at bar 1 and 9 at bar 3 give 8.5 at bar 4
        assert_eq!(line(&[1, 3], &[0., 10., 0., 9., 0.], |a, b| b <= a, 4), Some(8.5));
        assert_eq!(line(&[1, 3], &[0., 9., 0., 10., 0.], |a, b| b <= a, 4), None);
        assert!(run("nmstc_x", &x, 0, 5, &Params::new()).is_none());
    }
}

// Instrument admission: NewMarket Software trading systems course (tradingalliance.com) 2026-11-01 (titles lane K11).
pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book == "nmstc" && reviewed(def.id),
    label: "Course examples by NewMarket Software (author NAVIGATOR), shown on Euro FX and S&P 500 futures. Runs on indices, ETFs and stocks, US and India, are a declared application.",
    basis: "NewMarket Software 'How to build your own Trading System' course slides ts/DM.pps (System 2) and ts/PARABS.pps (System 4); independent check authors/titles-1030/K11w/k11_ref.py",
};
