//! Bill Williams, Trading Chaos (1st ed., Wiley 1995). Scanned book (no text layer); rules transcribed from
//! the page images. Only the fractal start/signal/stop system (ch. 8) and the 5/13/34 TWR rhythm rule (ch. 10)
//! state exact entries and exits. Page refs are printed pages (PDF page = printed + 3 in
//! Books/Bill Williams/Bill Williams - Trading Chaos.pdf). Sources and verification: library/bwtc1/.
use super::bwntd::{close_pos, fill_time, fractals, open_pos, path_key, tick_of, Pos, Stop};
use crate::connors::{sma, Ctx, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "bwtc1";

pub fn catalogue() -> Vec<StrategyDef> {
    let make = |id, name, group, dir, status, rules, notes| StrategyDef {
        id, name, group, session: 0, dir, status, needs_volume: false, needs_vix: false,
        needs_series: &[], book: BOOK, rules, notes, claim: None,
    };
    vec![
        make("bwtc1_fractal", "Fractal start / signal / stop (stop and reverse)", "TC1 ch. 8 · Using Fractals and Leverage", "Long/Short", Status::Live,
            &["F|Fractal (p.135-136): \"the middle bar must have a higher high (or lower low) than the two preceding bars and the two following bars\"; \"If a bar's high is parallel to the middle or high (low) bar, it does not count as one of the five bars\".",
              "F|\"Fractal start is any fractal that is followed by a fractal in the opposite direction.\" \"Fractal signal is any fractal that follows a fractal in the opposite direction.\" (p.138-139) Fractals are taken in the order they are completed.",
              "E|\"Should the market again move up and then go beyond (higher) the price at point A ... One tick above point A then becomes a breakout buy.\" (p.138) Buy stop one tick above an up fractal signal; sell stop one tick below a down fractal signal. A newer signal in the same direction replaces it.",
              "F|Leverage (p.140): \"If the market comes all the way back to the topmost (bottommost) bar of the start plus one tick, you have lost all your leverage and the buy or sell signal is immediately canceled.\" Buy signal cancelled when price trades one tick below the low of its down-fractal start (sell mirrored), in bar-path order.",
              "X|\"Fractal stop is the most distant fractal peak of the last two fractals in the opposite direction. Usually, but not always, this will be two fractals back\" (p.139). \"If you determine you are in a trend run, you will maximize your profits by using the regular trailing stop (two fractals back in the opposite direction)\" (p.142). Long stop = lower low of the last two down fractals minus one tick, trailed (only moves up) as fractals form; from the bar after entry; fill at the stop or the open if gapped.",
              "X|\"A fractal stop generally occurs two fractals back in the opposite direction, or if a buy/sell signal is created in the opposite direction. In the first case, we would have a stop. In the second case, we would have a stop-and-reverse signal.\" (p.142) An opposite signal hit reverses the position.",
              "F|p.macd=1 [non-default, Level Three worksheet]: \"Take fractal signals only when this momentum indicator is in the direction of the fractal signal\" (p.186) with \"If the histogram is below the signal line, we only take shorts. If the histogram is above the signal line, we only take longs\" (p.130); histogram = SMA5 - SMA34 of (H+L)/2 (CQG '5 SIM(MI) - 34 SIM(MI)'), signal line = its 5-bar SMA (p.122, 182), read at the previous close.",
              "X|Same-bar order [book silent]: open -> nearer extreme -> other extreme; gaps fill at the open. The bracketed-market alternative (exit or reverse on a thumb signal, p.142) is chart judgement and not coded."],
            "Printed p.133-151 (PDF 136-154). The book's examples (Figs 8-15 to 8-19) are 5-minute S&P futures on Feb 4 and Oct 4, 1994: intraday data the vault lacks, so not compared. One tick = 0.01 on US indices/ETFs, 0.05 on NSE. Run on the index (instrument swap)."),
        make("bwtc1_twr", "Tide/wave/ripple 5/13/34 rhythm (long / short / out)", "TC1 ch. 10 · Profitunity Trading Partner", "Long/Short", Status::Live,
            &["F|\"The best way to monitor this market rhythm is to use a 5/13/34 moving average filter\" (p.181); chart legend '5 SIM(MI) / 13 SIM(MI) / 34 SIM(MI)' = simple averages of the bar midpoint (H+L)/2 (Fig. 10-3, p.182).",
              "E|\"i. If 5 is above 13 and 34, you should be long; ii. If 5 is below 13 and 34, you should be short; iii. If 5 is between 13 and 34, you should be out\" (Fig. 10-6, p.186; p.181 \"Be long any time the 5-bar average is higher than both the 13-bar and 34-bar averages\").",
              "X|Position changes when the state changes at a daily close, executed at the next open [book silent on the fill]."],
            "Printed p.180-186. The book uses this as the 'Rhythm' column of its worksheet; the outline states it as a position rule, coded literally. No published results. Run on the index."),
        make("bwtc1_thumb", "Thumb trade (responsive fractal)", "TC1 ch. 8 · Looking inside the fractal", "Long/Short",
            Status::NotTestable("used only \"if you determine you are in a bracketed market\" (p.142, judgement); the thumb sell has no stated exit other than reversing at the middle-finger top (p.145), and needs volume/MFI (squat, green) bars"),
            &["E|Three bars with higher highs and higher lows (inside bars do not count); two of the three must be squat or green (increasing volume); sell one tick below the thumb bar (p.144-145)",
              "X|\"If the market moves down and triggers your stop, you immediately place a stop and reverse to go long at the top of the middle finger\" (p.145)"],
            "Volume: NIFTY 50 has volume only in part of idx_data; SPX has volume."),
        make("bwtc1_air_bag", "Air bag: two-bar reversal one time frame higher", "TC1 ch. 6 & 10", "Long/Short",
            Status::NotTestable("protective stop only - \"it is not necessarily an entry signal\" (p.95); no entry rule"),
            &["X|\"We place the air bag one tick above/below the high/low of the second bar back on one significantly higher time frame (or the next bar further back that has both a higher high and a higher low or a lower high and a lower low)\" (p.95)"],
            "Exit device without an entry."),
        make("bwtc1_ppt", "Profitunity Planned Trading (Elliott-wave campaign)", "TC1 ch. 9", "Long/Short",
            Status::NotTestable("needs an Elliott-wave count, Point Zero, five magic bullets and target zones judged by the trader (p.153-174)"),
            &["E|Buy 3 of 10 contracts at a 50-62% pullback after Point Zero, stop and reverse 5 below Point Zero (p.159)"],
            "Discretionary wave counting."),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    Some(match id {
        "bwtc1_fractal" => ("bwtc1_fractal", "Trading Chaos fractals", "Start/signal/stop, SAR"),
        "bwtc1_thumb" => ("bwtc1_fractal", "Trading Chaos fractals", "Thumb trade · not testable"),
        "bwtc1_air_bag" => ("bwtc1_fractal", "Trading Chaos fractals", "Air bag · not testable"),
        "bwtc1_twr" => ("bwtc1_rhythm", "Trading Chaos rhythm & waves", "5/13/34 TWR"),
        "bwtc1_ppt" => ("bwtc1_rhythm", "Trading Chaos rhythm & waves", "Elliott PPT · not testable"),
        _ => return None,
    })
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    match id {
        "bwtc1_fractal" => Some(vec![
            p("back", "Stop: most distant of the last N opposite fractals", 2., 1., 4., 1.),
            p("macd", "Take signals only on the 5/34/5 MACD side (p.130/186)", 0., 0., 1., 1.),
        ]),
        "bwtc1_twr" => Some(vec![p("fast", "Ripple average", 5., 2., 13., 1.), p("mid", "Wave average", 13., 5., 34., 1.), p("slow", "Tide average", 34., 13., 89., 1.)]),
        _ => None,
    }
}

fn twr(x: &Ctx, from: usize, to: usize, f: usize, m: usize, s: usize) -> Vec<Trade> {
    let med: Vec<f64> = (0..x.r.len()).map(|i| (x.h[i] + x.l[i]) / 2.0).collect();
    let (a, b, c) = (sma(&med, f), sma(&med, m), sma(&med, s));
    let mut out = vec![];
    let mut pos: Option<Pos> = None;
    for i in from..to {
        let want: i8 = if a[i] > b[i] && a[i] > c[i] { 1 } else if a[i] < b[i] && a[i] < c[i] { -1 } else { 0 };
        let have = pos.as_ref().map_or(0, |p| p.t.dir as i8);
        if want == have { continue; }
        let (k, px) = (i + 1, x.o[i + 1]);
        if let Some(p) = pos.take() { close_pos(p, k, px, "5/13/34 rhythm changed · next open", &mut out); }
        if want != 0 { let mut p = open_pos(want as f64, k, px, 1.0, "open"); p.t.signal_i = i; pos = Some(p); }
    }
    if let Some(p) = pos {
        let mut t = p.t;
        t.exit_i = to; t.exit_px = x.c[to]; t.signal_i = to; t.reason = "open".into(); t.open = true;
        out.push(t);
    }
    out
}

/// A pending fractal signal: stop price and the cancel level beyond its start.
#[derive(Clone, Copy)]
struct Sig { stop: f64, cancel: f64 }

fn fractal_sar(x: &Ctx, from: usize, to: usize, back: usize, use_macd: bool) -> Vec<Trade> {
    let tick = tick_of(x);
    let (fup, fdn) = fractals(x);
    let med: Vec<f64> = (0..x.r.len()).map(|i| (x.h[i] + x.l[i]) / 2.0).collect();
    let osc: Vec<f64> = sma(&med, 5).iter().zip(sma(&med, 34)).map(|(a, b)| a - b).collect();
    let line = sma(&osc, 5);
    let mut out = vec![];
    let mut pos: Option<Pos> = None;
    let mut seq: Vec<(i8, usize)> = vec![]; // completed fractals in order: (+1 up / -1 down, middle bar)
    let (mut buy, mut sell): (Option<Sig>, Option<Sig>) = (None, None);
    let mut stop: Option<f64> = None;
    for i in 0..=to {
        let live = i >= from;
        // events: 0 protective stop, 1 buy signal, 2 sell signal, 3 cancel buy, 4 cancel sell
        let mut ev: Vec<((u8, f64), u8, f64, f64)> = vec![];
        let mut push = |s: Stop, src: u8| if let Some((t, f)) = fill_time(x, i, s) { ev.push((path_key(t, f, s.dir), src, f, s.px)) };
        if let (Some(p), Some(s)) = (&pos, stop) { push(Stop { px: s, dir: -(p.t.dir as i8) }, 0); }
        if let Some(b) = buy { push(Stop { px: b.stop, dir: 1 }, 1); push(Stop { px: b.cancel, dir: -1 }, 3); }
        if let Some(s) = sell { push(Stop { px: s.stop, dir: -1 }, 2); push(Stop { px: s.cancel, dir: 1 }, 4); }
        ev.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap().then(a.1.cmp(&b.1)));
        for (_, src, fill, _) in ev {
            match src {
                0 => {
                    if pos.is_some() && stop.is_some() {
                        close_pos(pos.take().unwrap(), i, fill, &format!("Fractal stop ({back} fractals back)"), &mut out);
                        stop = None;
                    }
                }
                3 => { if buy.is_some() { buy = None; } }
                4 => { if sell.is_some() { sell = None; } }
                _ => {
                    let dir: i8 = if src == 1 { 1 } else { -1 };
                    let pending = if dir > 0 { buy.take() } else { sell.take() };
                    if pending.is_none() || !live { continue; }
                    if use_macd && i > 0 && !((osc[i - 1] - line[i - 1]) * dir as f64 > 0.0) { continue; }
                    match pos.as_ref().map(|p| p.t.dir as i8) {
                        Some(d) if d == dir => {}
                        Some(_) => {
                            close_pos(pos.take().unwrap(), i, fill, "Opposite fractal signal · stop and reverse", &mut out);
                            pos = Some(open_pos(dir as f64, i, fill, 1.0, "fractal")); stop = None;
                        }
                        None => { pos = Some(open_pos(dir as f64, i, fill, 1.0, "fractal")); stop = None; }
                    }
                }
            }
        }
        if i == to { break; }
        // fractals completed on this bar, in middle-bar order
        let mut done: Vec<(usize, i8)> = vec![];
        if let Some(m) = fup[i] { done.push((m, 1)); }
        if let Some(m) = fdn[i] { done.push((m, -1)); }
        done.sort();
        for (m, d) in done {
            if let Some(&(pd, pm)) = seq.last() {
                if pd == -d {
                    // signal: this fractal follows an opposite one (its start)
                    let s = if d > 0 { Sig { stop: x.h[m] + tick, cancel: x.l[pm] - tick } } else { Sig { stop: x.l[m] - tick, cancel: x.h[pm] + tick } };
                    if d > 0 { buy = Some(s) } else { sell = Some(s) }
                }
            }
            seq.push((d, m));
        }
        // trailing fractal stop for the next bar
        if let Some(p) = &pos {
            let dir = p.t.dir as i8;
            let opp: Vec<usize> = seq.iter().rev().filter(|f| f.0 == -dir).take(back).map(|f| f.1).collect();
            if opp.len() == back {
                let lvl = if dir > 0 { opp.iter().map(|&m| x.l[m]).fold(f64::INFINITY, f64::min) - tick } else { opp.iter().map(|&m| x.h[m]).fold(f64::NEG_INFINITY, f64::max) + tick };
                stop = Some(match stop { Some(s) => if dir > 0 { s.max(lvl) } else { s.min(lvl) }, None => lvl });
            }
        }
    }
    if let Some(p) = pos {
        let mut t = p.t;
        t.exit_i = to; t.exit_px = x.c[to]; t.signal_i = to; t.reason = "open".into(); t.open = true;
        out.push(t);
    }
    out
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    let schema = params(id)?;
    if from > to || to >= x.r.len() { return None; }
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).map_or(0.0, |d| d.def));
    let u = |k: &str| g(k).max(1.0).round() as usize;
    match id {
        "bwtc1_twr" => Some(twr(x, from, to, u("fast"), u("mid"), u("slow"))),
        "bwtc1_fractal" => Some(fractal_sar(x, from, to, u("back"), g("macd") >= 0.5)),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;
    #[test]
    fn twr_is_long_above_both_short_below_both_out_between() {
        let r: Vec<Row> = (0..120).map(|i| { let v = if i < 60 { 100. + i as f64 } else { 160. - 2. * (i - 60) as f64 };
            Row { ticker: "SPX".into(), date: format!("{:04}-01-01", 1800 + i), open: v, high: v + 1., low: v - 1., close: v, volume: None } }).collect();
        let x = Ctx::new(&r, &r, &[]);
        let t = run("bwtc1_twr", &x, 40, 119, &Params::new()).unwrap();
        assert_eq!(t[0].dir, 1.0); assert_eq!(t[0].legs[0].i, 41); // long from the first bar of the window
        assert!(t.iter().any(|t| t.dir < 0.0)); // turns short in the fall
    }
}
