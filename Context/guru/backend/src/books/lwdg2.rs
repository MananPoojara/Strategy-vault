//! Larry Williams, "The Definitive Guide to Futures Trading, Volume II" (Windsor Books, 1989). Book id `lwdg2`.
//! Source copy: Books/Larry Williams/The Definitive Guide To Futures Trading Volume II.pdf (image scan, 286 PDF
//! pages; printed page = PDF page - 8). Transcription and checks: library/lwdg2/.
//! Shares the bar-path simulator of `lwlts` (same intrabar convention: O -> nearer extreme -> other -> C).
use super::lwlts::{simulate, Cfg, Ent};
use crate::connors::{Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "lwdg2";


macro_rules! def {
    ($id:expr, $name:expr, $group:expr, $dir:expr, $status:expr, $vol:expr, $rules:expr, $notes:expr) => {
        StrategyDef { id: $id, name: $name, group: $group, session: 0, dir: $dir, status: $status, needs_volume: $vol,
            needs_vix: false, needs_series: &[], book: BOOK, rules: $rules, notes: $notes, claim: None }
    };
}

const VOL_X: &str = "X|\"buy the open and exit at the close\", $0 slippage and commissions (Tables 1-4, p.149-152)";
const VOL_N: &str = "Tables 1-4 (p.149-152), S&P futures 04/21/82-03/10/87, 1,236 market days. Volume = the instrument's own daily volume (SPX: index volume; NIFTY: index volume where present). Compare win % and frequency, not dollars.";

pub fn catalogue() -> Vec<StrategyDef> {
    use Status::{Live, NeedsData, NotTestable};
    let mut v = vec![
        def!("lwdg2_updown_daytrade", "Down close buy / up close sell, next day", "Ch 8 · Day trading", "Long/Short", Live, false,
            &["E|\"Every time you have a down close you buy the opening the next morning ... If you have an up close, sell the opening in the morning\" (p.124)",
              "X|\"With a 200 point stop, or exit on the close\" (p.124): 2.00 index points, armed on the entry day",
              "F|Unchanged close: no trade [book silent]"],
            "p.124-125: S&P and Value Line together \"about $180,000 since the Standard and Poor's started trading in 1982\"; \"something like 11 losses in a row\". No S&P-only figures, so no claim is set. Same S&P points on every instrument."),
        def!("lwdg2_0105", "Williams day trading system (.0105 of the close)", "Ch 8 · Day trading", "Long/Short", Live, false,
            &["F|\"At the close of each day ... multiply that day's close by .0105\" (p.133)",
              "E|\"add 3.10 points to the close ... a buyer at the higher number, a seller at the lower number\": buy stop at close + 0.0105 x close, sell stop at close - 0.0105 x close, next day (the printed trade list uses the unrounded value: 85.1204 buy 205.99 = 203.85 x 1.0105)",
              "X|\"exiting the position on the close\" (p.133); \"we did not use stops in this system\" (p.133)",
              "X|Option p.reverse=1: \"you may want to reverse in the event the market hits one number and then goes back to the other number\" (p.133) [non-default]"],
            "Printed: 06/01/82-01/23/87 520 trades, 290 won (55.9%), $69,700, max drawdown $5,000 (p.133); trade list 12/04/85-01/23/87: 132 trades, 77 won (58.3%), $25,775 (p.134-142, transcribed in library/lwdg2/trades_0105.csv). Book traded S&P futures (premium to cash), so SPX dates match only partly. One trade per day: the first stop touched (bar path); with reverse=1 the other stop reverses."),
        def!("lwdg2_vol_up_up", "Volume up, close up: buy next open (Table 1)", "Ch 8 · Volume myths", "Long", Live, true,
            &["F|\"IF VOLUME GREATER THAN YESTERDAY AND CLOSE GREATER THAN YESTERDAY\"", "E|\"THEN BUY THE OPEN\"", VOL_X], VOL_N),
        def!("lwdg2_vol_dn_up", "Volume down, close up: buy next open (Table 2)", "Ch 8 · Volume myths", "Long", Live, true,
            &["F|\"IF VOLUME LESS THAN YESTERDAY AND CLOSE GREATER THAN YESTERDAY\"", "E|\"THEN BUY THE OPEN\"", VOL_X], VOL_N),
        def!("lwdg2_vol_up_dn", "Volume up, close down: buy next open (Table 3)", "Ch 8 · Volume myths", "Long", Live, true,
            &["F|\"IF VOLUME GREATER THAN YESTERDAY AND CLOSE LESS THAN YESTERDAY\"", "E|\"THEN BUY THE OPEN\"", VOL_X], VOL_N),
        def!("lwdg2_vol_dn_dn", "Volume down, close down: buy next open (Table 4)", "Ch 8 · Volume myths", "Long", Live, true,
            &["F|\"IF VOLUME LESS THAN YESTERDAY AND CLOSE LESS THAN YESTERDAY\"", "E|\"THEN BUY THE OPEN\"", VOL_X], VOL_N),
        def!("lwdg2_weekly_entry", "Weekly entry signal", "Ch 3 · Weekly trading signal", "Long/Short", Live, false,
            &["F|Buy: \"A Monday morning opening that is lower than the previous week's low\" (p.34); the week's first session is used when Monday is a holiday [Read]",
              "E|\"then a rally in the market that rallies above the previous week's low, by 1 or 2 ticks\" (p.34): buy stop at the previous week's low + p.ticks x p.tick during that week (default 1 tick of 0.05, the S&P futures tick)",
              "E|Sell: \"opens higher than last week's high, then starts trading down and trades below the high of last week\" (p.34): sell stop at the previous week's high - ticks",
              "X|\"use a stop that is below the low of the week that you purchased, or above the high of the week you sell short\" (p.35): 1 tick beyond; armed from the week after entry (the entry week's low is still forming)",
              "X|\"use the low of the week that you buy for the first 3 weeks. Following that, trail up with a stop that is below the lowest low of the last 2 weeks\" (p.35); the trail only moves in the trade's favour",
              "X|\"if you get an opposite signal you should switch\" (p.36): reverse"],
            "Book shows no statistics, only charted examples (S&P 1983-85: Dec 1983 sell stopped out first week of Jan 1984; second week of Jan 1984 sell ~170 out ~159 in Feb; July 1984 buy at the low, a sell 3 weeks later; two small losses before the July 1985 high; no signal at the July or Sept 1985 lows, p.35-36). The alternative \"60% of the true range for the week\" exit (p.39) is not implemented: the book does not say whether it is recalculated each week."),
    ];
    v.extend([
        def!("lwdg2_money_supply_bonds", "Wednesday close predicts Thursday (bonds/bills)", "Ch 8 · Money supply", "Long/Short", NeedsData("T-Bond / T-Bill futures daily data"), false,
            &["E|\"If I have a higher close on Wednesday, I expect a higher close on Thursday\" (p.143-144), Treasury Bills and Bonds; the book found no stock-index relationship"], ""),
        def!("lwdg2_dual_osc_5min", "Day trade dual oscillator after a gap", "Ch 8 · Day trading oscillator", "Long/Short", NeedsData("5-minute intraday bars"), false,
            &["F|5-bar vs 20-bar average of (H+L+C)/3 on 5-minute bars (p.154-155)", "E|Gap up then short crosses below long: sell; gap down then crosses above: buy; first hour only", "X|Opposite crossing"], ""),
        def!("lwdg2_ultimate_osc", "Ultimate Oscillator divergence", "Ch 2 · Ultimate Oscillator", "Long/Short", NotTestable("The oscillator is exact (7/14/28-day buying/true-range ratios, weights 4/2/1, p.21-22), but the divergence ('a low in price that was not matched by a lower low in the oscillator', p.22) does not say which lows are compared; the book identifies them on charts."), false,
            &["F|UO = 4 x BP7/TR7 + 2 x BP14/TR14 + BP28/TR28 (BP = close - true low, TR = true high - true low)", "E|Divergence, after UO went below 30 (buy) / above 50 (sell), then UO breaks its peak (trough) between the two lows (highs) (p.22-24)", "X|Opposite signal; long: flat when UO > 70, stop when UO < 45 after > 50; short: flat when UO <= 30, stop when UO > 65 (p.24)"], ""),
        def!("lwdg2_zero_balance", "Zero Balance", "Ch 5 · Zero Balance", "Long/Short", NotTestable("Swing points (IR High/Low, 7-day rule, p.68-69) and the formula ZB8 = P6 + P5 - P3 are exact, but the signal is 'prices penetrate a Zero Balance level' of a point not yet confirmed, with no exit, applied by judgement ('last ditch move', p.84)."), false,
            &["F|IR High: today's low below the lowest low of the past seven days; IR High = highest high since the last IR Low", "E|Price crosses the Zero Balance level (p.84)"], ""),
        def!("lwdg2_half_cycle", "Half cycle differencing", "Ch 4 · Cycles", "Long/Short", NotTestable("An analysis tool that estimates the dominant cycle length (p.53-63); no trading rule."), false, &["F|Cycle reliability by de-weighted half-cycle differences"], ""),
        def!("lwdg2_spread_seasonal", "Spread trading with seasonal tables", "Ch 6 · Spreads", "Long/Short", NotTestable("Needs futures spread history, and the trade selection uses 'similar years' and a 'feel' for the spread (p.90-92)."), false, &["F|% Hi or % Lo at least 70% and average range at least the margin (p.88)"], ""),
        def!("lwdg2_overnight_formula", "Williams overnight formula", "Ch 8 · Day trading", "Long/Short", NotTestable("An add-on for holding an unspecified day trade overnight (p.126-128); it is run '5 to 10 minutes before the close' and the base trade is not defined."), false, &["F|(true high - open + close - true low) / (true high - true low): hold longs at 144% or more, shorts at 56% or less; out on the next open"], ""),
        def!("lwdg2_three_loss_rule", "Three loss rule (Pork Bellies)", "Ch 10 · Odds of doubling", "Long/Short", NotTestable("A position-size overlay on Williams' 1980 Pork Belly system, whose rules are not in this book (p.203-205)."), false, &["F|After three consecutive losing trades, trade two contracts until a winner"], ""),
    ]);
    v
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    Some(match id {
        "lwdg2_updown_daytrade" => ("lwdg2_daytrade", "Williams day trades (1985 seminar)", "Down close buy / up close sell"),
        "lwdg2_0105" => ("lwdg2_daytrade", "Williams day trades (1985 seminar)", ".0105 breakout"),
        "lwdg2_dual_osc_5min" | "lwdg2_overnight_formula" | "lwdg2_money_supply_bonds" => ("lwdg2_daytrade", "Williams day trades (1985 seminar)", "Not runnable"),
        "lwdg2_vol_up_up" => ("lwdg2_vol", "Volume / price studies", "Vol up, close up"),
        "lwdg2_vol_dn_up" => ("lwdg2_vol", "Volume / price studies", "Vol down, close up"),
        "lwdg2_vol_up_dn" => ("lwdg2_vol", "Volume / price studies", "Vol up, close down"),
        "lwdg2_vol_dn_dn" => ("lwdg2_vol", "Volume / price studies", "Vol down, close down"),
        "lwdg2_weekly_entry" => ("lwdg2_weekly", "Weekly entry signal", ""),
        "lwdg2_ultimate_osc" | "lwdg2_zero_balance" | "lwdg2_half_cycle" | "lwdg2_spread_seasonal" | "lwdg2_three_loss_rule" =>
            ("lwdg2_tools", "Analysis tools", "Not testable"),
        _ => return None,
    })
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    Some(match id {
        "lwdg2_updown_daytrade" => vec![p("stop_pts", "Stop in index points (book: 200 S&P points = 2.00)", 2.0, 0.0, 10.0, 0.5)],
        "lwdg2_0105" => vec![p("pct", "Fraction of the close", 0.0105, 0.004, 0.02, 0.0005), p("reverse", "Reverse when the other number trades (non-default)", 0.0, 0.0, 1.0, 1.0)],
        "lwdg2_vol_up_up" | "lwdg2_vol_dn_up" | "lwdg2_vol_up_dn" | "lwdg2_vol_dn_dn" => vec![],
        "lwdg2_weekly_entry" => vec![p("ticks", "Ticks beyond last week's low/high", 1.0, 1.0, 4.0, 1.0), p("tick", "Tick size in index points (S&P futures 0.05)", 0.05, 0.01, 1.0, 0.01)],
        _ => return None,
    })
}

/// Days since 1970-01-01 for "YYYY-MM-DD".
fn days(date: &str) -> i64 {
    let y: i64 = date[0..4].parse().unwrap_or(1970);
    let m: i64 = date[5..7].parse().unwrap_or(1);
    let d: i64 = date[8..10].parse().unwrap_or(1);
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let doy = (153 * ((m + 9) % 12) + 2) / 5 + d - 1;
    era * 146097 + yoe * 365 + yoe / 4 - yoe / 100 + doy - 719468
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    let schema = params(id)?;
    if from > to || to >= x.r.len() { return None; }
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).map_or(f64::NAN, |d| d.def));
    let c = &x.c;
    let none = (None, None);
    let day = |stop: f64, reverse: bool| Cfg { stop, stop_rng: f64::NAN, day0: true, target: f64::NAN, hold: Some(0), bailout: false, reverse };
    Some(match id {
        "lwdg2_updown_daytrade" => simulate(x, from, to, &day(g("stop_pts"), false), |i| {
            if i < 2 { none } else if c[i - 1] < c[i - 2] { (Some(Ent::Open), None) } else if c[i - 1] > c[i - 2] { (None, Some(Ent::Open)) } else { none }
        }),
        "lwdg2_0105" => {
            let k = g("pct");
            simulate(x, from, to, &day(f64::NAN, g("reverse") != 0.0), |i| {
                let d = k * c[i - 1];
                (Some(Ent::Stop(c[i - 1] + d)), Some(Ent::Stop(c[i - 1] - d)))
            })
        }
        "lwdg2_vol_up_up" | "lwdg2_vol_dn_up" | "lwdg2_vol_up_dn" | "lwdg2_vol_dn_dn" => {
            let vup = id.starts_with("lwdg2_vol_up");
            let cup = id.ends_with("_up");
            simulate(x, from, to, &day(f64::NAN, false), |i| {
                if i < 2 { return none; }
                let (Some(v1), Some(v2)) = (x.r[i - 1].volume, x.r[i - 2].volume) else { return none };
                let vok = if vup { v1 > v2 } else { v1 < v2 };
                let cok = if cup { c[i - 1] > c[i - 2] } else { c[i - 1] < c[i - 2] };
                if vok && cok { (Some(Ent::Open), None) } else { none }
            })
        }
        "lwdg2_weekly_entry" => weekly(x, from, to, g("ticks") * g("tick")),
        _ => return None,
    })
}

fn trade(dir: f64, i: usize, px: f64, xi: usize, xpx: f64, why: &str) -> Trade {
    Trade { dir, legs: vec![Leg { i, px, w: 1.0 }], exit_i: xi, exit_px: xpx, reason: why.into(), entry_kind: "stop",
        signal_i: xi, open: false, exits: vec![] }
}

/// Weekly entry signal on daily bars. Weeks are Monday-Sunday calendar weeks.
fn weekly(x: &Ctx, from: usize, to: usize, off: f64) -> Vec<Trade> {
    let (o, h, l) = (&x.o, &x.h, &x.l);
    let n = x.r.len();
    let wk: Vec<i64> = x.r.iter().map(|r| (days(&r.date) + 3).div_euclid(7)).collect();
    // per-bar index of its week (0..) and week extremes
    let mut wid = vec![0usize; n];
    let (mut wh, mut wl, mut wopen): (Vec<f64>, Vec<f64>, Vec<f64>) = (vec![], vec![], vec![]);
    for i in 0..n {
        if i == 0 || wk[i] != wk[i - 1] { wh.push(h[i]); wl.push(l[i]); wopen.push(o[i]); }
        let k = wh.len() - 1;
        wid[i] = k;
        wh[k] = wh[k].max(h[i]);
        wl[k] = wl[k].min(l[i]);
    }
    let mut out = vec![];
    // position: (dir, entry bar, entry px, entry week, stop)
    let mut pos: Option<(f64, usize, f64, usize, f64)> = None;
    for i in from.max(1)..=to {
        let w = wid[i];
        if w == 0 { continue; }
        // stop for this bar (weeks are complete for w-1, w-2)
        if let Some((d, _, _, we, st)) = pos.as_mut() {
            if w > *we {
                let lvl = if w <= *we + 2 { if *d > 0.0 { wl[*we] - off } else { wh[*we] + off } }
                    else if *d > 0.0 { wl[w - 1].min(wl[w - 2]) - off } else { wh[w - 1].max(wh[w - 2]) + off };
                *st = if st.is_nan() { lvl } else if *d > 0.0 { st.max(lvl) } else { st.min(lvl) };
            }
        }
        let buy = (wopen[w] < wl[w - 1]).then(|| wl[w - 1] + off);
        let sell = (wopen[w] > wh[w - 1]).then(|| wh[w - 1] - off);
        // 1. at the open: a stop gapped through, then an entry level gapped through
        let fill = |pos: &mut Option<(f64, usize, f64, usize, f64)>, out: &mut Vec<Trade>, a: u8, px: f64| {
            if let Some((d, ei, epx, _, _)) = pos.take() {
                out.push(trade(d, ei, epx, i, px, if a == 0 { "stop" } else { "reverse: opposite weekly signal" }));
            }
            if a == 1 { *pos = Some((1.0, i, px, w, f64::NAN)); }
            if a == 2 { *pos = Some((-1.0, i, px, w, f64::NAN)); }
        };
        if let Some((d, _, _, _, st)) = pos { if st.is_finite() && d * (o[i] - st) <= 0.0 { fill(&mut pos, &mut out, 0, o[i]); } }
        if let Some(b) = buy { if o[i] >= b && pos.map_or(true, |q| q.0 < 0.0) { fill(&mut pos, &mut out, 1, o[i]); } }
        if let Some(sl) = sell { if o[i] <= sl && pos.map_or(true, |q| q.0 > 0.0) { fill(&mut pos, &mut out, 2, o[i]); } }
        // 2. along the bar path O -> nearer extreme -> other extreme -> C
        let path = if h[i] - o[i] <= o[i] - l[i] { [o[i], h[i], l[i], x.c[i]] } else { [o[i], l[i], h[i], x.c[i]] };
        for s in 0..3 {
            let (mut cur, end) = (path[s], path[s + 1]);
            let up = end > cur;
            loop {
                let mut best: Option<(f64, u8)> = None; // 0 stop, 1 buy entry, 2 sell entry
                let mut add = |lv: f64, a: u8| {
                    let inside = lv.is_finite() && if up { lv > cur && lv <= end } else { lv < cur && lv >= end };
                    if inside && best.map_or(true, |(b, _)| if up { lv < b } else { lv > b }) { best = Some((lv, a)); }
                };
                if let Some((d, _, _, _, st)) = pos { if (d > 0.0) != up { add(st, 0); } }
                if up && pos.map_or(true, |q| q.0 < 0.0) { if let Some(b) = buy { add(b, 1); } }
                if !up && pos.map_or(true, |q| q.0 > 0.0) { if let Some(sl) = sell { add(sl, 2); } }
                let Some((lv, a)) = best else { break };
                fill(&mut pos, &mut out, a, lv);
                cur = lv;
            }
        }
    }
    if let Some((d, ei, epx, _, _)) = pos {
        let mut t = trade(d, ei, epx, to, x.c[to], "open");
        t.open = true;
        out.push(t);
    }
    for t in &mut out { t.entry_kind = if t.legs[0].px == x.o[t.legs[0].i] { "open" } else { "stop" }; }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;
    fn bar(d: &str, o: f64, h: f64, l: f64, c: f64) -> Row {
        Row { ticker: "T".into(), date: d.into(), open: o, high: h, low: l, close: c, volume: None }
    }
    #[test]
    fn day_0105_buys_at_close_plus_pct_and_exits_close() {
        let r = vec![bar("1985-12-03", 200., 204., 199., 203.85), bar("1985-12-04", 204., 208., 203.5, 207.35)];
        let x = Ctx::new(&r, &r, &[]);
        let t = run("lwdg2_0105", &x, 1, 1, &Params::new()).unwrap();
        assert_eq!(t.len(), 1);
        assert!((t[0].legs[0].px - 205.990425).abs() < 1e-6 && t[0].exit_px == 207.35 && t[0].dir == 1.0);
    }
    #[test]
    fn weekly_buy_after_open_below_last_week_low() {
        // week 1 (Mon 1984-07-16 .. Fri 07-20) low 150; Monday 07-23 opens 149, rallies to 151.
        let mut r = vec![];
        for d in 16..=20 { r.push(bar(&format!("1984-07-{d}"), 152., 155., 150., 153.)); }
        r.push(bar("1984-07-23", 149., 151., 148.5, 150.5));
        let x = Ctx::new(&r, &r, &[]);
        let t = run("lwdg2_weekly_entry", &x, 1, 5, &Params::new()).unwrap();
        assert_eq!(t.len(), 1);
        assert!((t[0].legs[0].px - 150.05).abs() < 1e-9 && t[0].open);
    }
}
