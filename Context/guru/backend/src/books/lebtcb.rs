//! Chuck LeBeau's System Traders Club (traderclub.com): the "Traders Club Bulletins" and the "Traderclub Forum" as
//! captured on 14/06/2001 (research drive Trading Knowledge II/@NewsLetters-Mags/Traders Club Full Bulletins/:
//! "Traders Club Full Bulletins.pdf", "TRADERSCLUBBULLETIN.PDF", "TRADERCLUBFORUM.PDF"; copies in
//! Books/Charles LeBeau/Traders Club Full Bulletins/). Book id "lebtcb". Notes: library/lebtcb/README.md.
//! Only the posts that state a complete system (entry + exit, exact numbers) are Live. Own simulator (no jkbwts::sim).
use crate::connors::{dmi, sma, true_range, Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade, claim};


pub fn catalogue() -> Vec<StrategyDef> {
    let d = |id, name, group, dir, status, rules, notes, claim| StrategyDef { id, name, group, session: 0, dir, status, rules, notes,
        needs_volume: false, needs_vix: false, needs_series: &[], book: "lebtcb", claim };
    vec![
        d("lebtcb_acb", "Adaptive channel breakout · X = 150/ADX(14) (Riccardo Ronco)", "Bulletin #4", "Long/Short", Status::Live,
          &["F|\"X represents the number of days of the channel breakout. X = 150/ADX(14 bars)\" (Bulletin #4)",
            "E|\"BUY HIGHEST(HIGH, X) + 1 POINT STOP; SELL LOWEST(LOW,X) - 1 POINT STOP;\" (Bulletin #4)",
            "X|\"This is the most basic version of the system and it is employed as a reversal system without stops\" (Bulletin #4): each stop reverses the position"],
          concat!("Source: ", "Bulletin #4 (Feb 8 1999). ",
            "Read [B]: X is not rounded in the code; TradeStation's Highest(High, X) loops over bars 0..X-1, i.e. floor(X) bars (at least 1). ",
            "ADX = Wilder ADX(14) (engine dmi). Orders are set at the close and work on the next bar; a gap through a stop fills at the open. ",
            "Flat only before the first fill: if both stops trade on that bar, TradeStation's bar path is used (open nearer the high = high first). ",
            "One fill per bar (a reversal is not reversed back on the same bar) [B]. '1 point' = one price unit of the data, p.pt = 0.01. ",
            "The bulletin says to use the complex version (lebtcb_acb_int) when X < 1 or X > 40; X < 1 cannot happen (ADX <= 100). Futures in the club; run on the indices (instrument swap)."), None),
        d("lebtcb_acb_int", "Adaptive channel breakout · INT(100/ADX(14)), 1-30 days (Riccardo Ronco)", "Bulletin #4", "Long/Short", Status::Live,
          &["F|\"We need to use a more complex version of the system when X is less than 1 or greater than 40. Here is an example: INPUTS: LEVEL(100), LENGTH(14); VALUE1 = INT(LEVEL / ADX(LENGTH)); IF VALUE1 > 30 THEN VALUE1 = 30; IF VALUE1 < 1 THEN VALUE1 = 1;\" (Bulletin #4)",
            "E|\"BUY AT HIGHEST(HIGH, VALUE1) + 1 POINT STOP; SELL AT LOWEST(LOW, VALUE1) - 1 POINT STOP;\" (Bulletin #4)",
            "X|Reversal system without stops (Bulletin #4): each stop reverses the position"],
          "Source: Bulletin #4 (Feb 8 1999). The code's LEVEL 100 and the 30-day cap are the defaults (the basic version's 150 is lebtcb_acb). ADX = Wilder ADX(LENGTH). Order handling as lebtcb_acb: next-bar stops, gap fills at the open, bar path for the first fill only, one fill per bar [B]; p.pt = one price unit (0.01). Run on the indices.", None),
        d("lebtcb_adavol", "ADAptive VOLatility breakout · 'volatility engine' (Riccardo Ronco)", "Traderclub Forum", "Long/Short", Status::Live,
          &["F|\"inputs: len(5), LT(21), ST(6), Price(Close); activity = xaverage(truerange, len); factor = xaverage(TrueRange, LT)/xaverage(TrueRange, ST); Breakout = activity * factor;\" (forum 'Volatility Engine to make VBS adaptive', R. Ronco, Jan 19 1999)",
            "E|\"If marketposition <> 1 then buy Price + Breakout stop; If marketposition <> -1 then sell Price - Breakout stop;\" (same post)",
            "X|No other exit: the opposite stop reverses the position (same post)"],
          concat!("Source: forum thread 'Volatility Engine to make VBS adaptive' (Jan 19-21 1999). Defaults are the posted code's inputs; Ronco later wrote (Jan 21 1999) that the listing's values were examples and suggested ranges (LEN 3-7, ST 10-20, LT 20-40), so those stay non-default options. ",
            "XAverage = TradeStation's exponential average (factor 2/(n+1), seeded with the first bar's value); TrueRange = max(H, C[1]) - min(L, C[1]). ",
            "Stops priced off today's close work on the next bar; gap fills at the open; first fill uses the TradeStation bar path; one fill per bar [B]; no orders before bar max(len, LT, ST) [B: MaxBarsBack]. ",
            "Designed for interest-rate futures (BTP, Gilt, Bund, Bond); run on the indices (instrument swap)."), None),
        d("lebtcb_tba", "TBond Attack! 1.0 · SMA(80) +/- 2 x ATR breakout (Riccardo Ronco)", "Traderclub Forum", "Long/Short", Status::Live,
          &["F|\"Input: Price(Close), Length(80), Pct(2); value1 = average(Price, Length); value2 = average(Truerange, Length);\" (forum 'T Bond Attack! 1.0 trading system', R. Ronco, Feb 28 1999)",
            "E|\"If Price cross above (value1 + Pct * value2) Then Buy (\\\"Trend LE\\\") at Market; If Price cross below (value1 - Pct * value2) Then Sell (\\\"Trend SE\\\") at Market;\" (same post; \"All trades are executed next day on open\")",
            "X|\"If mp = 1 and Price cross below value1 Then ExitLong(\\\"Trend LX\\\") at Market; If mp = -1 and Price cross above value1 Then ExitShort(\\\"Trend SX\\\") at Market;\" (same post); \"There are no stop loss\""],
          concat!("Source: forum post of Feb 28 1999. Defaults = the code's inputs 80 / 2 (the prose says tests favoured \"40 days and 3 pct\": $70,000 gross profit and $7,000 maximum drawdown on ~20 years of CSI T-bond data, $100 commission; that is p.length=40, p.pct=3). ",
            "'cross above' = Close > band and Close[1] <= band[1]; average = simple moving average; orders fill at the next open. An entry in the other direction reverses (TradeStation Sell when long). ",
            "T-bond futures in the post; nothing in the rules is in dollars, so it runs on the indices (instrument swap)."), None),
        d("lebtcb_pgo", "Pretty Good Oscillator system · PGO(89) +/-3, exit at 0 (Mark Johnson)", "Traderclub Forum", "Long/Short", Status::Live,
          &["F|PGO = (C - Average(C,89)) / XAverage(TrueRange,89) (forum 'Here's 1 system & 1 set of params, trades 20 mkts': J. Payne Jan 11 2001, Burgy Jan 13 2001; Paul Jan 29 2001: \"a channel that's 3 * AvgTrueRange above the average price ... Mark's used the ExpAvg of the True Range\")",
            "E|\"if indicator > +3.0 then go long / if indicator < -3.0 then go short\" (M. Johnson, Nov 4 and Nov 13 1999); \"the simple trading system only buys and sells at market-on-the-open\"",
            "X|\"if indicator crosses 0.0 then exit and go flat\" (M. Johnson, same posts)"],
          concat!("Source: forum thread 'Here's 1 system & 1 set of params, trades 20 mkts' (Nov 1999 - Apr 2001). Johnson's own code was a download from his website and is not in the capture; the formula is the forum's algebraic reduction of it (posted by J. Payne and repeated by three members), and it matches Johnson's +/-3 / 0 thresholds. ",
            "Signals at the close, orders at the next open. 'go long' is a state: whenever PGO > 3 and not long, buy (reversing a short); 'crosses 0' = PGO < 0 after >= 0 (long) or > 0 after <= 0 (short). ",
            "XAverage = TradeStation exponential average (2/(n+1), seeded with the first bar). The published test is 20 futures with every fill at the day's worst price; run here on the indices with normal next-open fills."),
          claim("648 (269 won, 379 lost)", "41.5%", "$438.95 per one-lot trade", "", "20 futures (BP CD CL CT DM DX FY HG HO HU JO JY KC LB MB NG SF TU TY US), 1/1/1987-8/7/1999, buys at the day's high, sells at the low, $25 commission")),
        d("lebtcb_atr_bands", "ATR bands breakout (Nortwangler)", "Traderclub Forum", "Long/Short", Status::NotTestable("the post gives no parameter values, only ranges: 'Ndays ... anywhere between 70 and 110', 'BWidth ... anywhere between 2 and 4'"),
          &["F|\"middle line\" = EXPONENTIAL moving average of the close over Ndays; Z = EXPONENTIAL moving average of TrueRange over Ndays; bands = middle +/- BWidth x Z (forum 'ATR Bands', Oct 14 1999)",
            "E|\"When price closes above the 'upper band', BUY on the next day's open ... When price closes below the 'lower band', SELL on the next day's open\"",
            "X|\"If you're long and price closes below the 'middle line', EXITLONG on the next day's open ... If you're short and proce closes above the 'middle line', EXITSHORT on the next day's open\""],
          "Forum 'ATR Bands', post of Oct 14 1999; the rules are complete but no single Ndays/BWidth is stated, so there is no book default.", None),
        d("lebtcb_ma_consec", "Moving average consecutive close (Bryan Bonner)", "Traderclub Forum", "Long/Short", Status::NotTestable("the moving-average length is given only as a range ('profitable anywhere from 25 to 50'); no default"),
          &["E|\"Buy if the close is above the X days moving average for Y days in a row. Sell if the close is below the X days moving average for Y days in a row. I usually use 3 days for Y.\" (forum, B. Bonner, Oct 4 1998)"],
          "Forum post 'Moving Average Consecutive Close' (Oct 4 1998). X has no stated value.", None),
        d("lebtcb_cooper_5dm", "Jeff Cooper 'Five Day Momentum' setup (posted by Johan Ginyard)", "Traderclub Forum", "Long/Short", Status::NotTestable("entry only; the post gives no exit"),
          &["E|\"When ADX (14 days) > 35 and Stochastic(8 days) < 40 to go long (or > 60 to go short) then buy next bar when price exceeds highest price in the set up-bar\" (forum, J. Ginyard, Aug 28 1998)"],
          "No exit is stated; none is added.", None),
        d("lebtcb_weekly_tse", "Weekly ADX-adaptive breakout for TSE stocks (Don M)", "Traderclub Forum", "Long", Status::NotTestable("the second exit's code is lost in the capture ('OR L2 AND L<ENTRYPRICE*.9)'); it also needs a 90-stock TSE universe on weekly bars"),
          &["F|\"X IS AN INPUT CALCULATED AS FOLLOWS: @ROUND(150/ADX(14),0)\" (forum, Don M, Jan 4 2001)",
            "E|\"IF H=HIGHEST(H,X) AND L>AVERAGE(C,40) THEN BUY NEXT BAR AT CLOSE + .75*AVGTRUERANGE (45) STOP\" (weekly bars)",
            "X|\"EXIT WHEN : (BARSSINCEENTRY>25 AND L=LOWEST(L,3)) OR L2 AND L<ENTRYPRICE*.9)\"; prose: exit after 25 weeks on a 3-week low, below the last short-term low, or 10% below entry"],
          "The 'short-term low' exit cannot be reconstructed from the garbled code, and no rule may be completed.", None),
        d("lebtcb_atr_dip", "ATR dip-in-uptrend entry trigger (Bulletin #13)", "Bulletin #13", "Long", Status::NotTestable("an entry example only; the bulletin gives no exit"),
          &["E|\"If the close today is 2.0 ATRs greater than the 40 day moving average ... and the close today is 2 ATRs or more below the close seven days ago ... then buy tomorrow if the price rises 0.8 ATRs above todays low\" (Bulletin #13 'Average True Range (3rd Article)', Feb 11 1999)"],
          "No exit is given; none is added.", None),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    Some(match id {
        "lebtcb_acb" => ("lebtcb_acb", "Adaptive channel breakout (Ronco)", "Basic · 150/ADX(14)"),
        "lebtcb_acb_int" => ("lebtcb_acb", "Adaptive channel breakout (Ronco)", "Complex · INT(100/ADX(14)), 1-30"),
        "lebtcb_adavol" => ("lebtcb_adavol", "ADAVOL volatility breakout (Ronco)", "len 5 / LT 21 / ST 6"),
        "lebtcb_tba" => ("lebtcb_tba", "TBond Attack! 1.0 (Ronco)", "80 / 2"),
        "lebtcb_pgo" => ("lebtcb_pgo", "Pretty Good Oscillator (Johnson)", "89, +/-3, 0"),
        "lebtcb_atr_bands" => ("lebtcb_ideas", "Traderclub forum ideas (not testable)", "ATR bands"),
        "lebtcb_ma_consec" => ("lebtcb_ideas", "Traderclub forum ideas (not testable)", "MA consecutive closes"),
        "lebtcb_cooper_5dm" => ("lebtcb_ideas", "Traderclub forum ideas (not testable)", "Cooper 5-day momentum"),
        "lebtcb_weekly_tse" => ("lebtcb_ideas", "Traderclub forum ideas (not testable)", "Weekly TSE breakout"),
        "lebtcb_atr_dip" => ("lebtcb_ideas", "Traderclub forum ideas (not testable)", "ATR dip trigger"),
        _ => return None,
    })
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    let pt = || p("pt", "'1 point' added to / subtracted from the channel (price units)", 0.01, 0., 1., 0.01);
    Some(match id {
        "lebtcb_acb" => vec![p("num", "Numerator: X = num / ADX", 150., 50., 300., 10.), p("adx_n", "ADX length", 14., 5., 40., 1.), pt()],
        "lebtcb_acb_int" => vec![p("level", "LEVEL: VALUE1 = INT(LEVEL / ADX)", 100., 50., 300., 10.), p("adx_n", "ADX length", 14., 5., 40., 1.),
            p("cap", "Longest channel (days)", 30., 10., 60., 1.), pt()],
        "lebtcb_adavol" => vec![p("len", "len: XAverage(TrueRange) for 'activity'", 5., 2., 10., 1.), p("lt", "LT: long XAverage(TrueRange)", 21., 10., 50., 1.),
            p("st", "ST: short XAverage(TrueRange)", 6., 3., 30., 1.)],
        "lebtcb_tba" => vec![p("length", "Length: SMA of the close and of TrueRange", 80., 20., 150., 5.), p("pct", "Pct: ATR multiple for the breakout", 2., 0.5, 5., 0.25)],
        "lebtcb_pgo" => vec![p("n", "PGO length (SMA of close, XAverage of TrueRange)", 89., 20., 200., 1.), p("thr", "Entry threshold (+/-)", 3., 1., 6., 0.25)],
        _ => return None,
    })
}

/// TradeStation XAverage: seeded with the first value, factor 2/(n+1).
fn xavg(v: &[f64], n: usize) -> Vec<f64> {
    let a = 2. / (n as f64 + 1.);
    let mut out = vec![f64::NAN; v.len()];
    for i in 0..v.len() { out[i] = if i == 0 { v[0] } else { out[i - 1] + a * (v[i] - out[i - 1]) }; }
    out
}

fn new_trade(dir: f64, i: usize, px: f64, kind: &'static str) -> Trade {
    Trade { dir, legs: vec![Leg { i, px, w: 1. }], exit_i: i, exit_px: px, reason: String::new(), entry_kind: kind, signal_i: i, open: false, exits: vec![] }
}
fn close(out: &mut Vec<Trade>, cur: Option<Trade>, i: usize, px: f64, sig: usize, why: &str) {
    if let Some(mut t) = cur { t.exit_i = i; t.exit_px = px; t.signal_i = sig; t.reason = why.into(); out.push(t); }
}
fn finish(out: &mut Vec<Trade>, cur: Option<Trade>, x: &Ctx, to: usize) {
    if let Some(mut t) = cur { t.exit_i = to; t.exit_px = x.c[to]; t.signal_i = to; t.reason = "open".into(); t.open = true; out.push(t); }
}

/// Stop-and-reverse: buy[i] / sell[i] are stop prices set at the close of bar i for bar i+1 (NaN = no order).
/// Long: only the sell stop works; short: only the buy stop; flat: both (bar path if both trade). One fill per bar.
fn sar_stops(x: &Ctx, from: usize, to: usize, buy: &[f64], sell: &[f64]) -> Vec<Trade> {
    let (mut out, mut cur, mut pos) = (vec![], None::<Trade>, 0i8);
    for j in from.max(1)..=to {
        let (b, s) = (buy[j - 1], sell[j - 1]);
        let hb = !b.is_nan() && x.h[j] >= b;
        let hs = !s.is_nan() && x.l[j] <= s;
        let (bpx, spx) = (x.o[j].max(b), x.o[j].min(s));
        let go = match pos {
            1 if hs => -1,
            -1 if hb => 1,
            0 if hb && hs => if x.h[j] - x.o[j] <= x.o[j] - x.l[j] { 1 } else { -1 },
            0 if hb => 1,
            0 if hs => -1,
            _ => 0,
        };
        if go != 0 {
            let px = if go == 1 { bpx } else { spx };
            close(&mut out, cur.take(), j, px, j, if go == 1 { "buy stop (reverse)" } else { "sell stop (reverse)" });
            cur = Some(new_trade(go as f64, j, px, "stop"));
            pos = go;
        }
    }
    finish(&mut out, cur, x, to);
    out
}

/// Market-on-open: decide(i, pos) at the close of bar i returns the wanted position (Some(1/-1/0)) or None (hold);
/// a change fills at the open of bar i+1.
fn sar_open(x: &Ctx, from: usize, to: usize, decide: impl Fn(usize, i8) -> Option<i8>) -> Vec<Trade> {
    let (mut out, mut cur, mut pos) = (vec![], None::<Trade>, 0i8);
    for i in from.max(1).saturating_sub(1)..to {
        let Some(want) = decide(i, pos) else { continue };
        if want == pos { continue; }
        let j = i + 1;
        close(&mut out, cur.take(), j, x.o[j], i, match want { 1 => "buy at market (reverse)", -1 => "sell at market (reverse)", _ => "exit at market" });
        if want != 0 { cur = Some(new_trade(want as f64, j, x.o[j], "open")); }
        pos = want;
    }
    finish(&mut out, cur, x, to);
    out
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    let schema = params(id)?;
    let n = x.r.len();
    if from > to || to >= n { return None; }
    for k in p.keys() { if !schema.iter().any(|d| d.key == k) { return None; } }
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).unwrap().def);
    let u = |k: &str| (g(k).max(1.)) as usize;
    let tr = true_range(x.r);
    Some(match id {
        "lebtcb_acb" | "lebtcb_acb_int" => {
            let int = id == "lebtcb_acb_int";
            let (num, pt) = (if int { g("level") } else { g("num") }, g("pt"));
            let cap = if int { u("cap") } else { usize::MAX };
            let adx = dmi(x.r, u("adx_n")).1;
            let (mut buy, mut sell) = (vec![f64::NAN; n], vec![f64::NAN; n]);
            for i in 0..n {
                if !(adx[i] > 0.) { continue; }
                let len = ((num / adx[i]).floor() as usize).clamp(1, cap);
                if len > i + 1 { continue; }
                let w = i + 1 - len..=i;
                buy[i] = x.h[w.clone()].iter().cloned().fold(f64::MIN, f64::max) + pt;
                sell[i] = x.l[w].iter().cloned().fold(f64::MAX, f64::min) - pt;
            }
            sar_stops(x, from, to, &buy, &sell)
        }
        "lebtcb_adavol" => {
            let (len, lt, st) = (u("len"), u("lt"), u("st"));
            let (a, l, s) = (xavg(&tr, len), xavg(&tr, lt), xavg(&tr, st));
            let warm = len.max(lt).max(st);
            let (mut buy, mut sell) = (vec![f64::NAN; n], vec![f64::NAN; n]);
            for i in warm..n { if s[i] > 0. { let b = a[i] * l[i] / s[i]; buy[i] = x.c[i] + b; sell[i] = x.c[i] - b; } }
            sar_stops(x, from, to, &buy, &sell)
        }
        "lebtcb_tba" => {
            let (len, pct) = (u("length"), g("pct"));
            let (ma, atr) = (sma(&x.c, len), sma(&tr, len));
            let up: Vec<f64> = (0..n).map(|i| ma[i] + pct * atr[i]).collect();
            let dn: Vec<f64> = (0..n).map(|i| ma[i] - pct * atr[i]).collect();
            let c = &x.c;
            let above = |v: &[f64], i: usize| c[i] > v[i] && c[i - 1] <= v[i - 1];
            let below = |v: &[f64], i: usize| c[i] < v[i] && c[i - 1] >= v[i - 1];
            sar_open(x, from, to, |i, pos| {
                if i == 0 || ma[i - 1].is_nan() { return None; }
                if above(&up, i) { Some(1) } else if below(&dn, i) { Some(-1) }
                else if pos == 1 && below(&ma, i) { Some(0) } else if pos == -1 && above(&ma, i) { Some(0) } else { None }
            })
        }
        "lebtcb_pgo" => {
            let (len, thr) = (u("n"), g("thr"));
            let (ma, xa) = (sma(&x.c, len), xavg(&tr, len));
            let pgo: Vec<f64> = (0..n).map(|i| if xa[i] > 0. { (x.c[i] - ma[i]) / xa[i] } else { f64::NAN }).collect();
            sar_open(x, from, to, |i, pos| {
                let v = pgo[i];
                if v.is_nan() { return None; }
                let prev = if i > 0 { pgo[i - 1] } else { f64::NAN };
                if v > thr { Some(1) } else if v < -thr { Some(-1) }
                else if pos == 1 && v < 0. && prev >= 0. { Some(0) } else if pos == -1 && v > 0. && prev <= 0. { Some(0) } else { None }
            })
        }
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;
    fn rows(b: &[(f64, f64, f64, f64)]) -> Vec<Row> {
        b.iter().enumerate().map(|(i, &(o, h, l, c))| Row { ticker: "T".into(), date: format!("2001-01-{:02}", i + 1), open: o, high: h, low: l, close: c, volume: None }).collect()
    }
    #[test]
    fn stop_and_reverse_hand_checked() {
        // bar1: flat, buy stop 10.5 traded (high 11) -> long 10.5; bar2: sell stop 9 gapped (open 8.5) -> short 8.5;
        // bar3: buy stop 9.6, high 9.5 -> nothing; bar4: buy stop 9.6 traded -> long 9.6 (open 9.2).
        let r = rows(&[(10., 10., 10., 10.), (10., 11., 10., 10.8), (8.5, 8.8, 8., 8.2), (8.3, 9.5, 8.2, 9.), (9.2, 9.9, 9.1, 9.8)]);
        let x = Ctx::new(&r, &r, &[]);
        let nan = f64::NAN;
        let buy = [10.5, nan, 9.6, 9.6, nan];
        let sell = [8., 9., nan, nan, nan];
        let t = sar_stops(&x, 0, 4, &buy, &sell);
        let got: Vec<(f64, usize, f64, usize, f64)> = t.iter().map(|t| (t.dir, t.legs[0].i, t.legs[0].px, t.exit_i, t.exit_px)).collect();
        assert_eq!(got, vec![(1., 1, 10.5, 2, 8.5), (-1., 2, 8.5, 4, 9.6), (1., 4, 9.6, 4, 9.8)]);
        assert!(t[2].open);
    }
}
