//! Marcel Link, "High Probability Trading" (McGraw-Hill, 2003).
//! Source copy: Books/Marcel Link/★Marcel Link High Probability Trading[1] (2).pdf. Page refs = printed page (PDF page = printed + 20).
//! README and oracle: library/mlhpt/.
use crate::connors::{dmi, sma, Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "mlhpt";
/// Admitted ids (authors/link/admission/SOURCE-CONTRACT.md).
pub const REVIEWED: &[&str] = &["mlhpt_bo10", "mlhpt_sample"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const BO_NOTE: &str = "Run on the daily index (the book says \"periods\" and gives no time frame for this system). Long only: the book states the short side in prose (\"or to short the lowest low\") but gives no short exit. \"Last three bars\" = the three bars before the current one (the current bar cannot close below its own low). Entry at the next bar's open as stated; the exit is also taken at the next bar's open [Book silent on the exit's timing; p.exit_close = 1 exits on the signal bar's close]. One position at a time.";
const SAMPLE_NOTE: &str = "TradeStation code p.263-264, run on daily index bars. Entries \"On Close\" fill at the signal bar's close and reverse an opposite position (TradeStation Buy/Sell); a same-direction signal is ignored. StdDev = TradeStation population standard deviation; the entry buffer uses StdDev(Close,10)[1], the stop uses Close - 2*StdDev(Close,10) of the entry bar (at$ = the entry bar) and is active from the next bar (open through it = open fill). ADX(10) = Wilder ADX (engine dmi). SlowD(14) = legacy TradeStation SlowD (FastK 14, SlowK and SlowD smoothed (2*prev + new)/3). The printed exit block has no begin/end; it is coded as the prose says: ADX > 30 -> exit on the 10/35 SMA cross, ADX < 20 -> exit when BarsSinceEntry = 10, otherwise exit when SlowD > 85 (long) / < 15 (short). Exit orders are market orders for the next bar's open and, as in TradeStation, an exit condition true on the entry bar exits that new position at the next open. Exit at the open is processed before the stop and before that bar's close entry.";
const MA3_GAP: &str = "Needs 5-minute bars: \"looking at three moving averages on a 5-minute chart, say, 10, 35, and 50\" (p.108). The vault has daily bars only. The author adds: \"This is just an idea for a simple system and not one that I would recommend using on its own.\" (p.108)";
const TL_GAP: &str = "Discretionary: needs a hand-drawn downward-sloping trendline (\"Buy if the current bar's close is higher than the trendline plus 10 ticks\", p.173).";
const FRAG_GAP: &str = "Entry fragments only (p.252-257: close above the 10-bar high +5 points or +StdDev; two closes above the 35-bar MA; 10/35 MA crossover; SlowK/SlowD and RSI(10) crossing 30; ADX(10) > 30 and SlowD(14) > 85). The book pairs no exit with them; the exits on p.259-260 are a menu (\"These are just a few examples. You have to test different ideas\"). Combining them would invent a system.";
const MTF_GAP: &str = "Multiple time-frame MA crossover (p.90): a 60-minute crossover allows longs only, taken and exited on 5-minute crossovers. The book gives no moving-average lengths (\"a simple moving average crossover system as in Charts 5-5 and 5-6\") and needs intraday bars.";

pub fn catalogue() -> Vec<StrategyDef> {
    let d = |id, name, group, dir, status, rules, notes| StrategyDef {
        id, name, group, session: 0, dir, status, needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK, rules, notes, claim: None,
    };
    use Status::*;
    vec![
        d("mlhpt_bo10", "10-bar close breakout, 3-bar-low exit", "Ch 9 · Breakouts and Reversals", "Long", Live, &[
            "E|\"Entry signal: Buy on the opening of the next bar if the current bar closes higher than the highest high of the previous 10 bars.\" p.173",
            "X|\"Exit signal: Exit if the current bar closes below the lowest low of the last three bars.\" p.173",
        ], BO_NOTE),
        d("mlhpt_sample", "Sample system: 10-bar breakout + 0.5 SD, ADX-switched exits, 2 SD stop", "Ch 12 · System Trading", "Long/Short", Live, &[
            "E|\"If Close > Highest(High,Length)[1] + StdDev(Close,10)[1]* SD Then Buy(\"Buy1\") On Close;\" Length 10, SD .5 p.263",
            "E|\"If Close < Lowest(Low,Length)[1] - StdDev(Close,10)[1]* SD Then Sell(\"Sell1\") On Close;\" p.263",
            "X|\"ExitLong(\"Stop1\") From Entry(\"Buy1\") at$ Close - 2*StdDev(Close,10) Stop;\" (short: + 2*StdDev) p.263",
            "X|\"If ADX(LengthADX) > 30 Then If Average(Close,Length1) Crosses Below Average(Close,Length2) Then ExitLong\" (crosses above: ExitShort), LengthADX 10, Length1 10, Length2 35 p.263",
            "X|\"If ADX(LengthADX) < 20 Then If BarsSinceEntry=BSE Then ExitLong / ExitShort\" BSE 10 p.263",
            "X|\"Else If SlowD(14) > 85 Then ExitLong; If SlowD(14) < 15 Then ExitShort\" p.264",
        ], SAMPLE_NOTE),
        d("mlhpt_ma3", "Three moving averages 10/35/50 on 5-minute bars", "Ch 6 · Trading with the Trend", "Long/Short", NeedsData(MA3_GAP), &[
            "E|\"Buy when the close is above the 50-period average and the 10-period average is above both the 35- and the 50-period averages.\" p.109",
            "X|\"If the close is above the 50-period average but the 10-period average is below the 35-period average exit longs but do not go short.\" p.109 (short mirrored)",
        ], MA3_GAP),
        d("mlhpt_trendline", "Trendline break + 10 ticks", "Ch 9 · Breakouts and Reversals", "Long", NotTestable(TL_GAP), &[
            "E|\"Buy if the current bar's close is higher than the trendline plus 10 ticks.\" p.173", "X|\"Exit if both the current bar and the previous bar close below the trendline.\" p.173",
        ], TL_GAP),
        d("mlhpt_fragments", "System-writing entry fragments (no exits)", "Ch 12 · System Trading", "Long", NotTestable(FRAG_GAP), &["E|\"If Close > Highest(High,10)[1] + 5 points Then Buy On Close;\" p.253 and similar"], FRAG_GAP),
        d("mlhpt_mtf", "Multiple time-frame MA crossover", "Ch 5 · Multiple Time Frames", "Long", NotTestable(MTF_GAP), &["E|\"get a buy signal on the 60-minute chart when the moving averages crossed to the upside\" p.90"], MTF_GAP),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    Some(match id {
        "mlhpt_bo10" => ("mlhpt_bo", "Link breakout systems", "10-bar close breakout"),
        "mlhpt_sample" => ("mlhpt_bo", "Link breakout systems", "Sample system (ADX exits)"),
        "mlhpt_trendline" => ("mlhpt_bo", "Link breakout systems", "Trendline · not testable"),
        "mlhpt_fragments" => ("mlhpt_bo", "Link breakout systems", "Fragments · not testable"),
        "mlhpt_ma3" => ("mlhpt_ma", "Link moving-average systems", "10/35/50 on 5-min · data missing"),
        "mlhpt_mtf" => ("mlhpt_ma", "Link moving-average systems", "Multi time frame · not testable"),
        _ => return None,
    })
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    Some(match id {
        "mlhpt_bo10" => vec![p("entry", "Breakout: highest high of previous bars", 10.0, 3.0, 50.0, 1.0), p("exit", "Exit: lowest low of last bars", 3.0, 2.0, 20.0, 1.0),
            p("exit_close", "Exit at: 0 = next open, 1 = signal close [Not from the book]", 0.0, 0.0, 1.0, 1.0)],
        "mlhpt_sample" => vec![p("length", "Breakout bars (Length)", 10.0, 5.0, 40.0, 1.0), p("sd", "Entry buffer (SD)", 0.5, 0.0, 2.0, 0.25),
            p("stop_sd", "Stop distance (SD)", 2.0, 0.5, 4.0, 0.5), p("adx_n", "ADX length", 10.0, 5.0, 30.0, 1.0),
            p("adx_hi", "Trending ADX above", 30.0, 20.0, 50.0, 5.0), p("adx_lo", "Choppy ADX below", 20.0, 10.0, 30.0, 5.0),
            p("ma1", "Fast MA (Length1)", 10.0, 5.0, 30.0, 1.0), p("ma2", "Slow MA (Length2)", 35.0, 20.0, 100.0, 5.0),
            p("bse", "Bars since entry (BSE)", 10.0, 3.0, 30.0, 1.0), p("slowd_hi", "SlowD long exit above", 85.0, 60.0, 95.0, 5.0)],
        _ => return None,
    })
}

pub fn validate(id: &str, p: &Params) -> Option<String> {
    let schema = params(id)?;
    for (k, &v) in p {
        let Some(d) = schema.iter().find(|d| d.key == k) else { return Some(format!("unknown parameter {k}")) };
        let int = d.def.fract() == 0.0 && d.step.fract() == 0.0;
        if !v.is_finite() || v < d.min || v > d.max || (int && v.fract() != 0.0) { return Some(format!("{k} must be in {}..{}", d.min, d.max)); }
    }
    None
}

fn trade(dir: f64, i: usize, px: f64, kind: &'static str, sig: usize) -> Trade {
    Trade { dir, legs: vec![Leg { i, px, w: 1.0 }], exit_i: i, exit_px: px, reason: String::new(), entry_kind: kind, signal_i: sig, open: false, exits: vec![] }
}
fn close_out(mut t: Trade, i: usize, px: f64, sig: usize, why: &str) -> Trade { t.exit_i = i; t.exit_px = px; t.signal_i = sig; t.reason = why.into(); t }
/// Population standard deviation of v[i+1-n..=i].
fn sd(v: &[f64], i: usize, n: usize) -> f64 {
    if i + 1 < n { return f64::NAN; }
    let w = &v[i + 1 - n..=i];
    let m = w.iter().sum::<f64>() / n as f64;
    (w.iter().map(|x| (x - m).powi(2)).sum::<f64>() / n as f64).sqrt()
}
/// Legacy TradeStation SlowD: FastK(n), SlowK = (2*SlowK[1] + FastK)/3, SlowD = (2*SlowD[1] + SlowK)/3 (as module stad2).
fn slowd(x: &Ctx, n: usize) -> Vec<f64> {
    let len = x.c.len();
    let (mut k, mut d) = (vec![f64::NAN; len], vec![f64::NAN; len]);
    for i in n.saturating_sub(1)..len {
        let hh = x.h[i + 1 - n..=i].iter().copied().fold(f64::MIN, f64::max);
        let ll = x.l[i + 1 - n..=i].iter().copied().fold(f64::MAX, f64::min);
        let fk = if hh > ll { (x.c[i] - ll) / (hh - ll) * 100.0 } else { 0.0 };
        k[i] = if i == 0 || !k[i - 1].is_finite() { fk } else { (2.0 * k[i - 1] + fk) / 3.0 };
        d[i] = if i == 0 || !d[i - 1].is_finite() { k[i] } else { (2.0 * d[i - 1] + k[i]) / 3.0 };
    }
    d
}

fn bo10(x: &Ctx, from: usize, to: usize, n: usize, m: usize, on_close: bool) -> Vec<Trade> {
    let mut out = vec![];
    let mut cur: Option<Trade> = None;
    for i in from.max(n.max(m))..=to {
        if let Some(t) = cur.take() {
            if x.c[i] < x.l[i - m..i].iter().copied().fold(f64::MAX, f64::min) {
                if on_close { out.push(close_out(t, i, x.c[i], i, "close below the 3-bar low · close")); }
                else if i < to { out.push(close_out(t, i + 1, x.o[i + 1], i, "close below the 3-bar low · next open")); }
                else { cur = Some(t); }
                continue;
            }
            cur = Some(t);
            continue;
        }
        if i < to && x.c[i] > x.h[i - n..i].iter().copied().fold(f64::MIN, f64::max) {
            cur = Some(trade(1.0, i + 1, x.o[i + 1], "open", i));
        }
    }
    // an exit at the next open can land on bar to+1 only when i == to, which is excluded above
    if let Some(t) = cur { let mut t = close_out(t, to, x.c[to], to, "open"); t.open = true; out.push(t); }
    out
}

fn sample(x: &Ctx, from: usize, to: usize, g: &dyn Fn(&str) -> f64) -> Vec<Trade> {
    let u = |k: &str| g(k) as usize;
    let (len, adx_n, m1, m2, bse) = (u("length"), u("adx_n"), u("ma1"), u("ma2"), u("bse"));
    let adx = dmi(x.r, adx_n).1;
    let (f, s) = (sma(&x.c, m1), sma(&x.c, m2));
    let sd_ = slowd(x, 14);
    let mut out = vec![];
    let mut cur: Option<(Trade, f64)> = None; // trade, stop level
    let mut pending: Option<(f64, &'static str)> = None; // (direction, rule) of a market exit for the next open
    for i in from.max(len + 1)..=to {
        // 1. next-bar market exit generated at the previous close
        if let (Some((d, why)), Some((t, _))) = (pending.take(), cur.as_ref()) {
            if t.dir == d { let (t, _) = cur.take().unwrap(); out.push(close_out(t, i, x.o[i], i - 1, why)); }
        }
        // 2. protective stop (active from the bar after entry)
        if let Some((t, st)) = cur.as_ref() {
            let (d, st) = (t.dir, *st);
            if t.legs[0].i < i {
                let adverse = if d > 0.0 { x.l[i] } else { x.h[i] };
                let px = if d * (x.o[i] - st) <= 0.0 { Some(x.o[i]) } else if d * (adverse - st) <= 0.0 { Some(st) } else { None };
                if let Some(px) = px { let (t, _) = cur.take().unwrap(); out.push(close_out(t, i, px, i, "2 SD stop")); }
            }
        }
        // 3. entries on the close (reverse an opposite position)
        let b = g("sd") * sd(&x.c, i - 1, 10);
        let hh = x.h[i - len..i].iter().copied().fold(f64::MIN, f64::max);
        let lo = x.l[i - len..i].iter().copied().fold(f64::MAX, f64::min);
        let sig = if x.c[i] > hh + b { 1.0 } else if x.c[i] < lo - b { -1.0 } else { 0.0 };
        let held = cur.as_ref().map_or(0.0, |c| c.0.dir);
        if sig != 0.0 && sig != held {
            if let Some((t, _)) = cur.take() { out.push(close_out(t, i, x.c[i], i, "reverse on the opposite entry · close")); }
            cur = Some((trade(sig, i, x.c[i], "close", i), x.c[i] - sig * g("stop_sd") * sd(&x.c, i, 10)));
        }
        // 4. exit conditions at the close -> market order for the next bar (applies to whatever is held then)
        if let Some((t, _)) = cur.as_ref() {
            let d = t.dir;
            let (a, e) = (adx[i], t.legs[0].i);
            let fire = if a > g("adx_hi") {
                (if d > 0.0 { f[i] < s[i] && f[i - 1] >= s[i - 1] } else { f[i] > s[i] && f[i - 1] <= s[i - 1] }).then_some("ADX > 30: 10/35 MA cross · next open")
            } else if a < g("adx_lo") {
                (i - e == bse).then_some("ADX < 20: 10 bars since entry · next open")
            } else if a.is_finite() {
                (if d > 0.0 { sd_[i] > g("slowd_hi") } else { sd_[i] < 100.0 - g("slowd_hi") }).then_some("ADX 20-30: SlowD extreme · next open")
            } else { None };
            if let Some(why) = fire { if i < to { pending = Some((d, why)); } }
        }
    }
    if let Some((t, _)) = cur { let mut t = close_out(t, to, x.c[to], to, "open"); t.open = true; out.push(t); }
    out
}
pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    let schema = params(id)?;
    if validate(id, p).is_some() || from > to || to >= x.r.len() { return None; }
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).unwrap().def);
    Some(match id {
        "mlhpt_bo10" => bo10(x, from, to, g("entry") as usize, g("exit") as usize, g("exit_close") == 1.0),
        _ => sample(x, from, to, &g),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;
    fn bar(o: f64, h: f64, l: f64, c: f64) -> Row { Row { ticker: "T".into(), date: String::new(), open: o, high: h, low: l, close: c, volume: None } }
    #[test]
    fn bo10_buys_next_open_and_exits_next_open_after_close_below_3_bar_low() {
        let mut r: Vec<Row> = (0..12).map(|_| bar(100., 101., 99., 100.)).collect();
        r.push(bar(100., 103., 100., 102.)); // 12: close 102 > 101 -> buy open 13
        r.push(bar(102.5, 104., 102., 103.)); // 13: entry 102.5
        r.push(bar(103., 104., 102.5, 103.5));
        r.push(bar(103., 103., 99., 99.5)); // 15: close 99.5 < min low(12..14) = 100 -> exit open 16
        r.push(bar(99., 100., 98., 99.));
        let x = Ctx::new(&r, &r, &[]);
        let t = run("mlhpt_bo10", &x, 0, r.len() - 1, &Params::new()).unwrap();
        assert_eq!(t.len(), 1);
        assert_eq!((t[0].legs[0].i, t[0].legs[0].px, t[0].exit_i, t[0].exit_px), (13, 102.5, 16, 99.0));
    }
}
