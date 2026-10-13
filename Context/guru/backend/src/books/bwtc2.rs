//! Bill & Justine Williams, Trading Chaos 2nd ed. (Wiley, 2004): the three "Wise Men". Only the Fractal
//! (3rd) and Super AO (2nd) entries with the book's trailing stop are mechanical; the bullish/bearish
//! divergent bar (1st) needs "angulation" judged by eye. Reuses the Alligator/AO/fractal/order helpers of
//! bwntd (same author, same definitions; TC2 App. 7 EasyLanguage). Sources: library/bwtc2/.
//! Page refs are printed pages (PDF page = printed + 33 in Books/Bill Williams/Bill Williams - Trading Chaos 2nd Ed.pdf).
use super::bwntd::{close_pos, fill_time, fractals, ind, manage, open_pos, path_key, tick_of, Pos, Stop};
use crate::connors::{Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "bwtc2";

pub fn catalogue() -> Vec<StrategyDef> {
    let make = |id, name, group, dir, status, rules, notes| StrategyDef {
        id, name, group, session: 0, dir, status, needs_volume: false, needs_vix: false,
        needs_series: &[], book: BOOK, rules, notes, claim: None,
    };
    vec![
        make("bwtc2_fractal_sao", "Fractal + Super AO entries, 3-5 bar trailing stop", "TC2 ch. 10-12 · Second & Third Wise Men", "Long/Short", Status::Live,
            &["F|Alligator Red line = 8-bar smoothed average of (H+L)/2 offset 5 bars (p.104-105; EasyLanguage p.216-217). AO = SMA5 - SMA34 of (H+L)/2 (p.218).",
              "E|Fractal (3rd Wise Man, p.136-139): five bars, highest high preceded and followed by two lower highs (equal bars need a sixth bar); \"If the buy signal is above the Red Balance Line ... place a buy stop one tick above the high of the up Fractal\"; \"we would not take a Fractal buy signal if, at the time it is hit, the price is below the Red Balance Line\"; it \"remains a signal until it is hit or until a more recent Fractal signal is formed\". Sell mirrored.",
              "E|Super AO (2nd Wise Man, p.128, 144, 149): \"three consecutive green [higher] or red [lower] bars on the AO\"; \"a bar that exceeds the high of the price bar corresponding to the third green AO bar\" -> buy stop one tick above it. \"What if the SAO Signal Is the First Signal Hit? ... we would enter just above the price bar that corresponds with the Super AO signal\" (p.148). No Alligator filter is stated for it. p.sao=0 turns it off [non-default].",
              "E|Order life of the Super AO stop [TC2 silent]: the author's AO rule from New Trading Dimensions p.49 - cancelled when the AO bar changes colour, otherwise moved to the latest bar (\"blue light special\").",
              "A|\"Once a signal is hit, we continue to take other Fractal signals that are triggered in the same direction\" (p.144); \"We would then add on to our long position on any other buy signals\" (p.148). One unit per signal (p.146 'Profit/Contract' column).",
              "A|p.sizing=1: reverse pyramiding (p.155-156) \"Our first entry was only one contract. Then on the second signal, we bought 5 contracts, then 4 ... 3 ... and 2 contracts on the fifth\" out of 15; leg weights 1/15, 5/15, 4/15, 3/15, 2/15, no sixth entry [non-default; p.146 shows both columns].",
              "X|\"If the market still continues up, trail a profit-protecting stop just under the lowest low of the last three to five price bars\" (p.148); \"exit bar based on the price dropping below the lowest low of the five preceding bars\" (Costco example, p.155); Apple example \"we were using a three-bar stop\" (p.150). Default p.trail_n=5 (the only worked example with prices), 3 and 4 are the book's alternatives. Stop = lowest low of the N bars before the current bar minus one tick (short: highest high plus one tick), from the bar after entry; fill at the stop or the open if gapped.",
              "X|\"We place our stop either at a reversal Fractal sell signal or just below the lowest low of the past three to five price bars\" (p.148); an opposite signal (valid fractal or Super AO) reverses the whole position (\"stop and reverse\", p.133, 146). p.exit=1: reversal signals only, no trailing stop [non-default].",
              "X|Same-bar order [book silent]: open -> nearer extreme -> other extreme; gaps fill at the open."],
            "Printed p.128-156 (PDF 161-189). Book results are single chart examples (gold 4 entries p.146, Costco 3 entries p.155, bonds $9,531.25 p.121) on futures/stocks the vault does not hold; none is on an index we have, so none is compared. One tick = 0.01 on US indices/ETFs, 0.05 on NSE. Run on the index (instrument swap). The AO-only always-in system (p.126-127) is the same text as bwntd_ao_flip (duplicate, not repeated)."),
        make("bwtc2_divergent_bar", "Bullish/bearish divergent bar (1st Wise Man)", "TC2 ch. 9 · First Wise Man", "Long/Short",
            Status::NotTestable("the divergent bar is only valid with 'angulation' - \"Draw or imagine a line that follows the Alligator's mouth ... If those two lines show clearly that they are moving away from each other, you have angulation\" (p.113) - and \"well outside the Alligator's mouth; the farther away, the better\" (p.114): chart judgement"),
            &["E|\"A bullish divergent bar is a bar that has a lower low and closes in the top half of the bar\" (p.110); buy stop just above it, protective stop just below it (p.114, 120).",
              "F|\"the bullish/bearish divergent bar signals are invalid unless the angle described previously is increasing\" (p.113)"],
            "Pristine three-wise-men sequence (p.145-149) starts with this signal, so it is not testable either."),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    Some(match id {
        "bwtc2_fractal_sao" => ("bwtc2_wise", "Three Wise Men (TC2)", "Fractal + Super AO, trailing stop"),
        "bwtc2_divergent_bar" => ("bwtc2_wise", "Three Wise Men (TC2)", "Divergent bar · not testable"),
        _ => return None,
    })
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    match id {
        "bwtc2_fractal_sao" => Some(vec![
            p("trail_n", "Trailing stop: lowest low of the last N bars (book 3-5)", 5., 3., 5., 1.),
            p("exit", "Exit: 0 trailing stop + reversal signals, 1 reversal signals only", 0., 0., 1., 1.),
            p("sao", "Super AO entries on (1) / off (0)", 1., 0., 1., 1.),
            p("sao_bars", "Super AO: consecutive AO bars of one colour", 3., 2., 5., 1.),
            p("sizing", "Sizing: 0 one unit per signal, 1 reverse pyramid 1-5-4-3-2 of 15", 0., 0., 1., 1.),
            p("teeth_n", "Red line (Teeth): smoothed bars", 8., 3., 21., 1.),
            p("teeth_off", "Red line offset (bars into the future)", 5., 0., 13., 1.),
            p("ao_fast", "AO fast SMA", 5., 2., 13., 1.),
            p("ao_slow", "AO slow SMA", 34., 13., 89., 1.),
        ]),
        _ => None,
    }
}

const REV_PYR: [f64; 5] = [1.0, 5.0, 4.0, 3.0, 2.0];

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    let schema = params(id)?;
    if from > to || to >= x.r.len() { return None; }
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).map_or(0.0, |d| d.def));
    let u = |k: &str| g(k).max(0.0).round() as usize;
    let (n, trail, sao, nsao, pyr) = (u("trail_n").max(1), u("exit") == 0, u("sao") == 1, u("sao_bars").max(1), u("sizing") == 1);
    let d = ind(x, u("teeth_n").max(1), u("teeth_off"), u("ao_fast").max(1), u("ao_slow").max(1), 5, 1);
    let tick = tick_of(x);
    let (fup, fdn) = fractals(x);
    // Super AO signal: the bar that completes exactly `nsao` consecutive same-colour AO bars
    let mut same = vec![0usize; d.aoc.len()];
    for i in 1..same.len() { same[i] = if d.aoc[i] == 0 { 0 } else if d.aoc[i] == d.aoc[i - 1] { same[i - 1] + 1 } else { 1 }; }
    let weight = |units: usize| if pyr { REV_PYR[units] / 15.0 } else { 1.0 };
    let mut out = vec![];
    let mut pos: Option<Pos> = None;
    let (mut pu, mut pd, mut last_u, mut last_d): (Option<f64>, Option<f64>, Option<usize>, Option<usize>) = (None, None, None, None);
    let mut psao: Option<Stop> = None;
    for i in 0..=to {
        let live = i >= from;
        let mut ev: Vec<((u8, f64), u8, Stop, f64)> = vec![]; // 0 trail, 1 fractal up, 2 fractal down, 3 SAO
        let mut push = |s: Option<Stop>, src: u8| if let Some(s) = s { if let Some((t, f)) = fill_time(x, i, s) { ev.push((path_key(t, f, s.dir), src, Stop { px: f, dir: s.dir }, s.px)) } };
        if let Some(p) = pos.as_ref().filter(|p| trail && i > p.t.legs[0].i && i >= n) {
            let dir = p.t.dir;
            let lvl = if dir > 0.0 { x.l[i - n..i].iter().copied().fold(f64::INFINITY, f64::min) - tick } else { x.h[i - n..i].iter().copied().fold(f64::NEG_INFINITY, f64::max) + tick };
            push(Some(Stop { px: lvl, dir: if dir > 0.0 { -1 } else { 1 } }), 0);
        }
        push(pu.map(|px| Stop { px, dir: 1 }), 1);
        push(pd.map(|px| Stop { px, dir: -1 }), 2);
        if sao { push(psao, 3); }
        ev.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap().then(a.1.cmp(&b.1)));
        for (_, src, s, order) in ev {
            let dir = s.dir;
            let kind: &'static str = match src {
                0 => {
                    if pos.as_ref().is_some_and(|p| p.t.dir as i8 == -dir) {
                        close_pos(pos.take().unwrap(), i, s.px, &format!("Below the lowest low of the last {n} bars (trailing stop)"), &mut out);
                    }
                    continue;
                }
                1 | 2 => {
                    if src == 1 { pu = None } else { pd = None }
                    if !(if dir > 0 { order > d.teeth[i] } else { order < d.teeth[i] }) { continue; }
                    "fractal"
                }
                _ => { psao = None; "super AO" }
            };
            if !live { continue; }
            match pos.as_mut() {
                Some(p) if p.t.dir as i8 == dir => {
                    if !pyr || p.units < REV_PYR.len() { let w = weight(p.units); p.t.legs.push(Leg { i, px: s.px, w }); p.units += 1; }
                }
                Some(_) => {
                    close_pos(pos.take().unwrap(), i, s.px, &format!("Opposite {kind} signal · stop and reverse"), &mut out);
                    pos = Some(open_pos(dir as f64, i, s.px, weight(0), kind));
                }
                None => pos = Some(open_pos(dir as f64, i, s.px, weight(0), kind)),
            }
        }
        if i == to { break; }
        if let Some(m) = fup[i] { if last_u.map_or(true, |v| m > v) { last_u = Some(m); pu = Some(x.h[m] + tick); } }
        if let Some(m) = fdn[i] { if last_d.map_or(true, |v| m > v) { last_d = Some(m); pd = Some(x.l[m] - tick); } }
        let sig = if same[i] == nsao { d.aoc[i] } else { 0 };
        manage(&mut psao, d.aoc[i], sig, x, i, tick);
    }
    if let Some(p) = pos {
        let mut t = p.t;
        t.exit_i = to; t.exit_px = x.c[to]; t.signal_i = to; t.reason = "open".into(); t.open = true;
        out.push(t);
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;
    #[test]
    fn trailing_stop_is_the_lowest_low_of_the_previous_n_bars() {
        // flat bars, then a fractal buy; the stop sits one tick under the 5-bar low
        let mut r: Vec<Row> = (0..80).map(|i| Row { ticker: "SPX".into(), date: format!("{:04}-01-01", 1900 + i), open: 100., high: 101., low: 99., close: 100., volume: None }).collect();
        r[60].high = 105.; // up fractal confirmed on 62, Red line ~100
        r[63].open = 104.; r[63].high = 106.; r[63].low = 103.; r[63].close = 105.5;
        for k in 64..70 { r[k].open = 106.; r[k].high = 107.; r[k].low = 105.; r[k].close = 106.; }
        r[70].open = 105.5; r[70].high = 105.6; r[70].low = 104.; r[70].close = 104.5;
        let x = Ctx::new(&r, &r, &[]);
        let p = Params::from([("sao".into(), 0.0)]);
        let t = run("bwtc2_fractal_sao", &x, 30, 79, &p).unwrap();
        assert_eq!(t[0].legs[0].i, 63);
        assert!((t[0].legs[0].px - 105.01).abs() < 1e-9);
        assert_eq!(t[0].exit_i, 70); // lowest low of bars 65-69 = 105 -> stop 104.99, open 105.5, low 104
        assert!((t[0].exit_px - 104.99).abs() < 1e-9);
    }
}
