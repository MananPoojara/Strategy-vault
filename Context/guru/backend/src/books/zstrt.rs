//! Zone Strategy System for TS ("ZoneStrat - the Zone Strategy", version 4.29, Jun/08/2006). Source (research drive, read in
//! place, read-only): `USB HDD 1/TradeStation Systems & Indicators/Zone Strategy System for TS/ZoneStrat code.rar` ->
//! `ZoneStrat code.txt` (extracted as data). Book id `zstrt`. Notes: library/zstrt/README.md. Engine: books::rrsim.
use crate::books::rrsim::{g, o, pd, run_ts, sdef, A, P, POINT};
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "zstrt";
const ID: &str = "zstrt_zone";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![sdef(BOOK, ID, "ZoneStrat: fade a close beyond the prior bar's zone, out next close", "Zone Strategy (ZoneStrat code.txt)", "Long/Short", Status::Live, &[
        "F|\"prcUpperZoneBoundary = high - (high - low)*iZonePercent/100; prcLowerZoneBoundary = low + (high - low)*iZonePercent/100\" iZonePercent -119",
        "F|close > prcUpperZoneBoundary[1]: nZoneSide = 1, ptsMinDistToStay = (prcUpperZoneBoundary + 1 point) - close, ptsMinDistToAbandon = close - prcUpperZoneBoundary; close < prcLowerZoneBoundary[1]: nZoneSide = -1 (mirror); between: nZoneSide = 0",
        "F|\"bMinDistOK = ptsMinDistToStay >= iptsMinDistToStay points AND ptsMinDistToAbandon >= iptsMinDistToAbandon points\" (-73 / -98 points)",
        "F|same side as the bar before: nZoneStayCounter + 1, else bZoneAbandoned and counter = 1; bRunCalc turns true (and stays) at the first abandoned non-zero side after bar 1",
        "E|\"if bRunCalc[1] AND nZoneStayCounter = iBarsInZone (2) AND bMinDistOK\": side 1 -> \"if iDirection = 1 then Buy this bar on close else if iDirection = 0 then SellShort this bar on close\"; side -1 -> \"if iDirection = 1 then SellShort ... else if iDirection = 0 then Buy this bar on close\" (iDirection 0 = trade against the Zone)",
        "X|\"if MarketPosition = -1 then BuyToCover this bar on close else if MarketPosition = 1 then Sell this bar on close\"",
    ], "Literal code: a position entered on a close is exited on the next close (MarketPosition read in the code is the value before that bar's close orders); an entry and that exit on one close are processed in code order (entry first). \
Dangling 'else' binds to the nearest 'if' (EasyLanguage), which matches the input comment '0 = trade against the Zone'. 1 point = 1/PriceScale = 0.01 (2-decimal index quotes), so -73 / -98 points = -0.73 / -0.98 price units. \
Timeframe and instrument not stated (equity-curve png only). India: instrument swap; index shorts hypothetical.")]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    (id == ID).then_some(("zstrt", "Zone Strategy (ZoneStrat 4.29)", "code literal"))
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    (id == ID).then(|| vec![pd("zone_pct", "iZonePercent", -119., -200., 50., 1.), pd("bars", "iBarsInZone", 2., 1., 20., 1.),
        pd("direction", "iDirection (1 = with the Zone, 0 = against)", 0., 0., 1., 1.),
        pd("stay", "iptsMinDistToStay (points)", -73., -100., 100., 1.), pd("abandon", "iptsMinDistToAbandon (points)", -98., -100., 100., 1.),
        pd("point", "Price of 1 point (1/PriceScale)", POINT, 0.0001, 1., 0.01)])
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if id != ID || from >= to || to >= x.r.len() { return None; }
    let zp = g(p, "zone_pct", -119.);
    let nb = g(p, "bars", 2.).round() as i64;
    let dir = g(p, "direction", 0.).round() as i64;
    let pt = g(p, "point", POINT);
    let (stay_min, ab_min) = (g(p, "stay", -73.) * pt, g(p, "abandon", -98.) * pt);
    let (h, l, c) = (&x.h, &x.l, &x.c);
    let up: Vec<f64> = (0..c.len()).map(|i| h[i] - (h[i] - l[i]) * zp / 100.).collect();
    let lo: Vec<f64> = (0..c.len()).map(|i| l[i] + (h[i] - l[i]) * zp / 100.).collect();
    let (mut side, mut side1, mut cnt, mut run_calc, mut run_calc1) = (0i8, 0i8, 0i64, false, false);
    let (mut d_stay, mut d_ab) = (0f64, 0f64);
    Some(run_ts(x, from, to, |i, st, now, _next| {
        side1 = side; run_calc1 = run_calc;
        if i >= 1 {
            if c[i] > up[i - 1] { side = 1; d_stay = (up[i] + pt) - c[i]; d_ab = c[i] - up[i]; }
            else if c[i] < lo[i - 1] { side = -1; d_stay = c[i] - (lo[i] - pt); d_ab = lo[i] - c[i]; }
            else if c[i] >= lo[i - 1] && c[i] <= up[i - 1] { side = 0; }
        }
        let ok = d_stay >= stay_min && d_ab >= ab_min;
        let abandoned = if side == side1 { cnt += 1; false } else { cnt = 1; true };
        if !run_calc && abandoned && side != 0 && i >= 1 { run_calc = true; }
        if run_calc1 && cnt == nb && ok {
            if side == 1 {
                if dir == 1 { now.push(o(A::Buy, P::Mkt, "Buy (stay)")); } else if dir == 0 { now.push(o(A::Sell, P::Mkt, "SellShort (abandon)")); }
            }
            if side == -1 {
                if dir == 1 { now.push(o(A::Sell, P::Mkt, "SellShort (stay)")); } else if dir == 0 { now.push(o(A::Buy, P::Mkt, "Buy (abandon)")); }
            }
        }
        if st.pos == -1 { now.push(o(A::Xs, P::Mkt, "BuyToCover this bar on close")); } else if st.pos == 1 { now.push(o(A::Xl, P::Mkt, "Sell this bar on close")); }
    }))
}
