//! Jeff Cooper, "The 5 Day Momentum Method" (M. Gordon Publishing Group, 1997). Book id "jc5dm".
//! Source: Books/Jeff Cooper/The 5 Day Momentum Method.pdf (text layer, page numbers = the book's
//! "Page N" footers; checked against the 1997 scan "Five Day Momentum (TS Code pack).pdf").
//! The TradeStation 5DAYMOMENTUM.ELD is a compressed binary and could not be read.
//! Notes, extraction and the independent oracle: library/jc5dm/.
use crate::connors::{true_range, wilder, Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "jc5dm";

const LONG: &[&str] = &[
    "F|\"we will only buy stocks whose ADX is 35 or higher and whose +DI is greater than its -DI reading\" (p.10). ADX/DI period: \"Default: 14 days\" (Appendix p.55); Wilder smoothing as printed on p.55-56",
    "F|\"we will only trade stocks on the long-side whose price is above $50/share\" (p.10): close of Day One > 50 (in the ticker's own price units)",
    "F|\"we use an eight period Fast % K\" (p.9); %K = (Close - Low for Period)/(High for Period - Low for Period) x 100 (p.56)",
    "E|\"Today (Day One), the Fast % K must close under 40 %\" (p.11)",
    "E|\"We will buy tomorrow (Day Two), one tick above today's high (1/16 point). If you are not filled tomorrow (Day Two) you will look to buy the next day (Day Three, a tick above the Day Two high).\" (p.11). Buy stop; a gap above it fills at the open",
    "X|\"Upon being filled, our stop is at or 1 tick under the previous day's low\" (p.11): default = at the low (stop_ticks 0); 1 = one tick under. A gap through the stop fills at the open; on the entry bar the stop counts as hit if the low reaches it [Book silent: daily bars cannot order the fill and the stop; conservative]",
    "X|\"If you are not stopped out, exit the trade on the close four trading days from today ... (this keeps you in the position for 5 trading days, Monday inclusive)\" (p.24): exit on the close of the 5th bar counting the fill bar",
];
const SHORT: &[&str] = &[
    "F|\"For downtrending stocks (short selling), the ADX must be 35 or higher and the -DI reading will be above the +DI reading\" (p.10); 14-day ADX/DI (Appendix p.55)",
    "F|\"For short sales, we will drop the requirement to above $40/share\" (p.10): close of Day One > 40",
    "E|\"Today (Day One), the Fast % K must close above 60 %\" (p.11), eight-period Fast %K",
    "E|\"We will sell short tomorrow (Day Two), one tick below today's low (1/16 point). If you are not filled tomorrow (Day Two) you will look to sell the next day (Day Three, a tick below the Day Two low).\" (p.11-12)",
    "X|\"Upon being filled, our stop is at or 1 tick above the previous day's high\" (p.12): default at the high (stop_ticks 0)",
    "X|5 Day Exit: buy back on the close of the 5th trading day counting the fill day (p.24)",
];
const NOTES: &str = "Book tested \"a universe of over 500 stocks (above $50/share) over a seven year period\": about 1 5/8 points average gain per trade over five days, \"successful a little under 50% of the time\", profit to loss about 3-1, average price about $71 (p.39). \
That US stock universe and period (to 1997) are not in the vault, so the published figures cannot be reproduced; run on an NSE stock (basket tickers) or an index = instrument swap only. Thresholds are literal: price > 50 / > 40 in the ticker's own units (always true on the indices), one tick = 1/16. \
The Day Three order exists only if the trader was flat and trying on Day Two. One position at a time; signals while in a trade are ignored. The optional sell-half \"on profits you are happy with\" (p.25) is discretionary and not coded.";

pub fn catalogue() -> Vec<StrategyDef> {
    let d = |id, name, dir, status, rules, notes, claim| StrategyDef {
        id, name, group: "5 Day Momentum Method", session: 0, dir, status, needs_volume: false,
        needs_vix: false, needs_series: &[], book: BOOK, rules, notes, claim,
    };
    let claim = crate::connors::claim("n/a (500+ stocks, 7 years)", "a little under 50%", "about 1 5/8 points (1.58% net)", "5 days", "US stocks above $50, to 1997 (p.39)");
    vec![
        d("jc5dm_long", "5 Day Momentum Method · buys, 5 Day Exit", "Long", Status::Live, LONG, NOTES, claim),
        d("jc5dm_short", "5 Day Momentum Method · short sales, 5 Day Exit", "Short", Status::Live, SHORT, NOTES, None),
        d("jc5dm_trailing", "5 Day Momentum Method · Trailing Stop Exit", "Long/Short",
          Status::NotTestable("the final half is held \"as you see fit\" (p.31): no exit rule for it"),
          &["E|Same entry as the 5 Day Exit version (p.11-12)",
            "X|\"Measure the risk of your trade and upon profiting by that much move your stop to break-even\" (p.30)",
            "X|\"When the stock moves to a price that is double your initial risk, sell half\" (p.30)",
            "X|\"Let the other half run as you see fit\" (p.31) - not mechanical"],
          "Examples (NOI, Visio, Ace) trail the stop and exit on judgement (\"we should strongly consider exiting\").", None),
        d("jc5dm_ibd_rs", "5 Day Momentum Method · IBD Relative Strength filter", "Long/Short",
          Status::NeedsData("Investor's Business Daily Relative Strength ratings (RS >= 95 for buys; IBD lists of stocks recently under RS 50/30 for shorts) - not in the vault"),
          &["F|\"If we do not want to use ADX, we will use Investors Business Daily Relative Strength Reading and only trade stocks whose Relative Strength reading is at 95 or higher\" (p.10)",
            "F|Shorts: \"stocks who have recently dropped to under 50 or 30 as mentioned in Investors Business Daily\" (p.10)",
            "E|Same Fast %K entry and exits as jc5dm_long / jc5dm_short"], "RS is a cross-sectional rating of the whole US market.", None),
        d("jc5dm_options", "5 Day Momentum Method · options", "Long/Short",
          Status::NotTestable("needs option chains (deep in-the-money calls/puts, 2 strikes ITM, front month)"),
          &["E|\"buying deep in the money calls for long set-ups and deep in the money puts for short set-ups ... only after the signal is triggered on the underlying\" (p.41)",
            "F|\"You should be at least 2 strike prices in-the-money ... (if the expiration date is within 5 trading days, trade the following month options)\" (p.41-42)",
            "X|\"exit strategies ... are identical to the equities\" (p.42)"], "", None),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    let f = ("jc5dm", "5 Day Momentum Method");
    Some(match id {
        "jc5dm_long" => (f.0, f.1, "Buys · 5 Day Exit"),
        "jc5dm_short" => (f.0, f.1, "Short sales · 5 Day Exit"),
        "jc5dm_trailing" => (f.0, f.1, "Trailing Stop Exit · not testable"),
        "jc5dm_ibd_rs" => (f.0, f.1, "IBD RS filter · data missing"),
        "jc5dm_options" => (f.0, f.1, "Options · not testable"),
        _ => return None,
    })
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    let long = id == "jc5dm_long";
    if !long && id != "jc5dm_short" { return None; }
    Some(vec![
        p("adx_n", "ADX / DI days (Appendix default 14)", 14., 7., 28., 1.),
        p("adx_min", "ADX at least (book 35)", 35., 20., 50., 5.),
        p("k_n", "Fast %K periods (book 8)", 8., 5., 14., 1.),
        if long { p("k_lvl", "Fast %K closes under (book 40)", 40., 20., 50., 5.) }
        else { p("k_lvl", "Fast %K closes above (book 60)", 60., 50., 80., 5.) },
        if long { p("price", "Close above (book $50)", 50., 0., 100., 10.) }
        else { p("price", "Close above (book $40)", 40., 0., 100., 10.) },
        p("tick", "One tick in price units (book 1/16)", 0.0625, 0.01, 0.25, 0.0125),
        p("stop_ticks", "Stop ticks beyond the previous day's extreme (book: 'at or 1 tick'; 0 = at)", 0., 0., 1., 1.),
        p("hold", "Trading days held, fill day inclusive (book 5)", 5., 1., 10., 1.),
    ])
}

/// Wilder +DI, -DI, ADX exactly as the Appendix (p.55-56) prints them: +DM/-DM ignored when the high/low
/// does not exceed yesterday's, the smaller of the two zeroed (both kept on an exact tie), Wilder smoothing.
pub fn dmi3(x: &Ctx, n: usize) -> (Vec<f64>, Vec<f64>, Vec<f64>) {
    let len = x.r.len();
    let (mut pdm, mut mdm, mut tr) = (vec![f64::NAN; len], vec![f64::NAN; len], vec![f64::NAN; len]);
    let trs = true_range(x.r);
    for i in 1..len {
        let (mut up, mut dn) = ((x.h[i] - x.h[i - 1]).max(0.), (x.l[i - 1] - x.l[i]).max(0.));
        if dn > up { up = 0. } else if up > dn { dn = 0. }
        pdm[i] = up; mdm[i] = dn; tr[i] = trs[i];
    }
    let (sp, sm, st) = (wilder(&pdm, n), wilder(&mdm, n), wilder(&tr, n));
    let (mut pdi, mut mdi, mut dx) = (vec![f64::NAN; len], vec![f64::NAN; len], vec![f64::NAN; len]);
    for i in 0..len {
        if st[i] > 0. {
            pdi[i] = 100. * sp[i] / st[i];
            mdi[i] = 100. * sm[i] / st[i];
            let s = pdi[i] + mdi[i];
            dx[i] = if s > 0. { 100. * (pdi[i] - mdi[i]).abs() / s } else { 0. };
        }
    }
    (pdi, mdi, wilder(&dx, n))
}

/// Unsmoothed Fast %K over n bars including today; NaN when the n-bar range is zero.
pub fn fast_k(x: &Ctx, n: usize) -> Vec<f64> {
    (0..x.r.len()).map(|i| {
        if i + 1 < n { return f64::NAN; }
        let (hh, ll) = x.h[i + 1 - n..=i].iter().zip(&x.l[i + 1 - n..=i]).fold((f64::MIN, f64::MAX), |(a, b), (&h, &l)| (a.max(h), b.min(l)));
        if hh > ll { 100. * (x.c[i] - ll) / (hh - ll) } else { f64::NAN }
    }).collect()
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    let dir = match id { "jc5dm_long" => 1., "jc5dm_short" => -1., _ => return None };
    let schema = params(id)?;
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).unwrap().def);
    if from >= to || to >= x.r.len() { return Some(vec![]); }
    let (adx_n, k_n, hold) = (g("adx_n").max(1.) as usize, g("k_n").max(1.) as usize, g("hold").max(1.) as usize);
    let (adx_min, lvl, price, tick, stop_off) = (g("adx_min"), g("k_lvl"), g("price"), g("tick"), g("stop_ticks") * g("tick"));
    let (pdi, mdi, adx) = dmi3(x, adx_n);
    let k = fast_k(x, k_n);
    let sig = |s: usize| s >= from && adx[s] >= adx_min && if dir > 0. {
        pdi[s] > mdi[s] && x.c[s] > price && k[s] < lvl
    } else {
        mdi[s] > pdi[s] && x.c[s] > price && k[s] > lvl
    };
    let mut out = vec![];
    let mut cur: Option<(Trade, f64)> = None; // (trade, stop)
    let mut day3 = false; // a Day Two order was tried and not filled -> Day Three order today
    for t in from + 1..=to {
        if let Some((mut tr, stop)) = cur.take() {
            let e = tr.legs[0].i;
            let (gap, hit) = if dir > 0. { (x.o[t] <= stop, x.l[t] <= stop) } else { (x.o[t] >= stop, x.h[t] >= stop) };
            if gap || hit {
                tr.exit_px = if gap { x.o[t] } else { stop };
                tr.reason = if gap { "Stop · gap through, open".into() } else { "Protective stop".into() };
            } else if t - e + 1 >= hold {
                tr.exit_px = x.c[t];
                tr.reason = format!("5 Day Exit · close of day {hold}");
            } else { cur = Some((tr, stop)); continue; }
            tr.exit_i = t; tr.signal_i = t; out.push(tr);
            day3 = false;
            continue;
        }
        let armed = sig(t - 1) || day3;
        day3 = false;
        if !armed { continue; }
        let level = if dir > 0. { x.h[t - 1] + tick } else { x.l[t - 1] - tick };
        let filled = if dir > 0. { x.h[t] >= level } else { x.l[t] <= level };
        if !filled { day3 = sig(t - 1); continue; }
        let px = if dir > 0. { x.o[t].max(level) } else { x.o[t].min(level) };
        let stop = if dir > 0. { x.l[t - 1] - stop_off } else { x.h[t - 1] + stop_off };
        let mut tr = Trade { dir, legs: vec![Leg { i: t, px, w: 1. }], exit_i: t, exit_px: px, reason: String::new(),
            entry_kind: "stop", signal_i: t - 1, open: false, exits: vec![] };
        // conservative: on the fill bar the stop counts as hit if the bar reaches it
        if (dir > 0. && x.l[t] <= stop) || (dir < 0. && x.h[t] >= stop) {
            tr.exit_px = stop; tr.reason = "Protective stop · entry bar".into(); out.push(tr);
        } else if hold <= 1 {
            tr.exit_px = x.c[t]; tr.reason = "5 Day Exit · close of day 1".into(); out.push(tr);
        } else { cur = Some((tr, stop)); }
    }
    if let Some((mut tr, _)) = cur {
        tr.exit_i = to; tr.exit_px = x.c[to]; tr.signal_i = to; tr.reason = "open".into(); tr.open = true; out.push(tr);
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;
    fn bar(i: usize, o: f64, h: f64, l: f64, c: f64) -> Row {
        Row { ticker: "T".into(), date: format!("2000-{:04}", i), open: o, high: h, low: l, close: c, volume: None }
    }
    #[test]
    fn hand_checked_long_day_three_fill_stop_and_five_day_exit() {
        // 60-bar steady uptrend (ADX high, +DI > -DI), then a pullback day with %K < 40
        let mut r: Vec<Row> = (0..60).map(|i| { let b = 100. + 0.5 * i as f64; bar(i, b, b + 1.5, b - 0.5, b + 1.) }).collect();
        r.push(bar(60, 130., 130.5, 125.5, 126.)); // Day One: close near the 8-bar low -> %K < 40
        r.push(bar(61, 126., 130.4, 125., 128.)); // Day Two: high 130.4 < 130.5 + 1/16 -> no fill, %K 50
        r.push(bar(62, 128., 132., 127.5, 131.)); // Day Three: buy stop 130.4 + 1/16, stop at Day Two low 125
        for i in 63..70 { r.push(bar(i, 132., 134., 131., 133.)); }
        let x = Ctx::new(&r, &r, &[]);
        let (_, _, adx) = dmi3(&x, 14);
        assert!(adx[60] >= 35. && fast_k(&x, 8)[60] < 40.);
        let t = run("jc5dm_long", &x, 20, 69, &Params::new()).unwrap();
        assert_eq!(t.len(), 1);
        assert_eq!((t[0].legs[0].i, t[0].exit_i), (62, 66));
        assert!((t[0].legs[0].px - 130.4625).abs() < 1e-9 && t[0].exit_px == 133.);
        // stop hit on day 3 of the trade
        let mut r2 = r.clone(); r2[64].low = 124.;
        let t2 = run("jc5dm_long", &Ctx::new(&r2, &r2, &[]), 20, 69, &Params::new()).unwrap();
        assert_eq!((t2[0].exit_i, t2[0].exit_px), (64, 125.));
        assert!(run("jc5dm_trailing", &x, 20, 69, &Params::new()).is_none());
    }
}
