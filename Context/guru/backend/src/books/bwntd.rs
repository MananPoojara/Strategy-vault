//! Bill Williams, New Trading Dimensions (Wiley, 1998). The "Bill Williams Systems & Indicators Pack"
//! .doc chapters (THE ALLIGATOR, THE FRACTAL, THE AWESOME OSCILLATOR, THE MARKET ACCELERATOR, TRADING IN
//! THE ZONES, THE BALANCE LINE TRADES, TECHNIQUES FOR TAKING PROFITS) are the same chapter text and add
//! no rules. Indicator code: the author's own EasyLanguage in Trading Chaos 2nd ed. App. 7 (Alligator =
//! smoothed average of (H+L)/2, AO = SMA5 - SMA34 of (H+L)/2). Sources and verification: library/bwntd/.
//! Page refs "p.N" are PDF pages of Books/Bill Williams/Bill Williams - New Trading Dimensions.pdf.
use crate::connors::{sma, wilder, Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "bwntd";
/// Admitted ids (authors/bwilliams/admission/SOURCE-CONTRACT.md). bwntd_dims is held: its 5th dimension (Balance Line) is not
/// testable and its AC formula is garbled in the book (p.56), so the five-dimension system cannot be run as written.
pub const REVIEWED: &[&str] = &["bwntd_fractal", "bwntd_ao_flip"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const BL_WHY: &str = "not testable: the Balance Line buy/sell needs 'the bar with the lowest high' as base with no stated look-back, and 'the signal bar is above the Balance Line' does not say which price (high, low, whole bar) or which bar's Blue-line value decides 1 vs 2 higher highs (p.68-71). The 1998 Profitability-Test had to invent a 10-bar look-back.";

pub fn catalogue() -> Vec<StrategyDef> {
    let make = |id, name, group, dir, status, rules, notes| StrategyDef {
        id, name, group, session: 0, dir, status, needs_volume: false, needs_vix: false,
        needs_series: &[], book: BOOK, rules, notes, claim: None,
    };
    const ALLIGATOR: &str = "F|Alligator (p.37): \"Blue line—a 13-bar smoothed moving average offset 8 bars into the future. Red line—an 8-bar smoothed moving average offset 5 bars into the future. Green line—a 5-bar smoothed moving average offset 3 bars into the future.\" Price = (H+L)/2, smoothing = (prev*(n-1)+price)/n (author's EasyLanguage, Trading Chaos 2nd ed. p.217). Red line on bar i = 8-bar value of bar i-5.";
    const FRACTAL_DEF: &str = "F|Fractal (p.40-41): \"a series of a minimum of five consecutive bars where the highest high is preceded by two lower highs and is followed by two lower highs\"; \"If the current bars' high is the same height as the middle or high bar, it does not count as one of the five bars\" (equal highs are skipped; sell fractal = lows, mirrored).";
    const FRACTAL_E: &str = "E|\"If the buy signal is above the Red Balance Line (the Alligator's Teeth), we would place a buy stop one tick above the high of the up fractal\"; \"we would not take a fractal buy signal if, at the time it is hit, the price is below the Red Balance Line\"; \"it remains a signal until it is hit or until a more recent fractal signal is formed\" (p.41, 43). Valid when the stop price is above the Red line of the bar it is hit; a hit invalid fractal is used up. Sell: one tick below the down fractal, below the Red line.";
    const ADD: &str = "A|\"Notice that there are five more fractal buy signals that were hit later\"; \"based on trading one contract for each signal\" (p.38-39; Table 4-1 adds a contract at every fractal hit). Each signal adds one unit (leg weight 1 = one contract per signal, no cap stated). p.pyramid=0: first signal only [non-default].";
    const EXIT_RED: &str = "X|\"We place our first Stop to Exit (not a SAR—Stop and Reverse) just inside the Alligator's Teeth (Red line) on a Stop Close Only order for the dailies\" (p.37; p.72 \"your best overall stop is the Red line ... a Stop Close Only order\"): long exits at the close when close < Red line (short: close > Red line), checked from the entry bar.";
    const EXIT_FIVE: &str = "X|\"If the market goes our way, we trail a stop after we have had five consecutive bars of the same color\" (p.37). \"place your stop one tick below the fifth green bar ... raise the stop up to just below the most recently completed bar ... no matter what color the bars are after that fifth green bar\" (p.73; p.64-65 Red Zone mirror). Bar colour (p.63): green = AO and AC both higher than the previous bar, red = both lower, else gray. Stop only moves in favour; fill at the stop or the open if gapped.";
    const EXIT_OPT: &str = "X|Exit options (p.72-74, the book's own list): p.exit=0 Red line + five-bar trail together (p.37, default); 1 Red line only (\"best overall stop\"); 2 five-bar trail only (\"our most used stop\"). The Green-line stop is only for a \"Runaway market\" judged by angle (not testable).";
    const REV: &str = "X|An opposite valid fractal hit while in a position closes it and enters the other way at the stop price (\"4. A signal in the opposite direction\", p.72-74; netting = TradeStation default). p.reverse=0 ignores opposite fractals while in a position [non-default].";
    const ORDER: &str = "X|Same-bar order of stop fills [book silent]: the bar is assumed to go open -> nearer extreme -> other extreme -> close; stops fill in that path order (gaps fill at the open). Close-only orders (Red line, zone) act at the close.";
    vec![
        make("bwntd_fractal", "Fractal breakout outside the Alligator (fractal dimension only)", "NTD ch. 3-4 & 9 · Alligator + Fractal", "Long/Short", Status::Live,
            &[ALLIGATOR, FRACTAL_DEF, FRACTAL_E, ADD, EXIT_RED, EXIT_FIVE, EXIT_OPT, REV, ORDER],
            "PDF p.36-45, 72-74. Book's fractal-only result is Table 4-1 (coffee futures Dec 16-Mar 10, 10 fractal entries, 29,705 points, $111,393.75); coffee is not in the vault, so it is not compared. One tick = 0.01 on US indices, ETFs and stocks, 0.05 on NSE. Run on the index (instrument swap)."),
        make("bwntd_dims", "Five-dimension system: Fractal, AO, AC, Zone (Balance Line not testable)", "NTD ch. 3-9 · Tying it all together", "Long/Short", Status::Live,
            &[ALLIGATOR, FRACTAL_DEF, FRACTAL_E,
              "F|\"After the first fractal breakout has been triggered, we take any and every signal from all five dimensions in that direction ... if the price is above the Alligator's mouth, we would only take buy signals\" (p.38; p.47 \"we do not take any AO signals until the first fractal buy or sell signal is triggered outside the Alligator's mouth\"). The last valid fractal hit sets the direction; AO/AC/zone entries need their order price beyond the Red line (the fractal chapter's mouth test). After a stop-out: \"Reenter on any of the five dimensional signals going your way, provided the current price is still outside the Alligator's mouth\" (p.73).",
              "F|AO (p.46, author's code TC2 p.218): SMA5((H+L)/2) - SMA34((H+L)/2); bar green if above the previous bar, red if below (p.47). The book's printed '(H - L)/2' is corrected by the word 'midpoints' and the author's EasyLanguage.",
              "E|AO Saucer buy (p.47-48): all three bars above zero, bar b lower than a, bar c higher than b; buy stop one tick above the high of the price bar of c. Cross buy (p.48): histogram from negative to positive; buy stop above that bar. Twin Peaks buy (p.48-49): below zero, a trough higher than the previous trough with no zero cross between, trigger bar green; buy stop above it. Sells mirrored (p.49-52).",
              "E|AO/AC signal management (p.49, 52, 61): \"if a buy or sell signal is generated but not hit on the current bar and the next histogram bar changes color, that signal is canceled\"; while the colour holds the stop moves to the latest bar (\"blue light special\": \"finally sold at the first bar that made a lower low\", p.52); \"we only take the most recently generated signals of that particular dimension\" (p.48).",
              "F|AC (p.56): \"a 5-bar simple moving average of the difference between the 5/34 momentum histogram and a 5-bar simple moving average of that difference\" - garbled; the next sentence (\"If the current difference between the 5/34 AO oscillator and the 5-bar moving average ...\") supports AC = AO - SMA5(AO). Default p.ac_form=1 = AO - SMA5(AO): with it the S&P 1997 zone add-ons fall on the fill dates of Table 7-1 (10 of 10) and AC entries on 3 of 5 fill dates of Table 6-1, versus 8/10 and 0/5 for the literal SMA5(AO - SMA5(AO)) (p.ac_form=0, non-default). [Fitted reading, see README]",
              "E|AC buy (p.56-57): after a red bar, two consecutive green bars when the current bar is above zero, three when below (\"what matters is where the current bar is\", p.57); one signal per green run; buy stop one tick above the high of the signal bar. AC sell mirrored (p.58-60).",
              "A|Zone add-on (p.63-65): green bar preceded by a green bar and close higher than the previous close -> buy on the close (\"Stop Close Only\"); \"After the fifth red or green bar, we stop adding on based on the zones\" (zone add only while the run is 5 bars or fewer). Red Zone mirrored. Table 7-1 shows next-day fills; the text's close fill is used.",
              ADD, EXIT_RED, EXIT_FIVE, EXIT_OPT, REV, ORDER,
              "F|Dimension switches p.ao, p.ac, p.zone (1 = on, default) reproduce the book's one-dimension tables (Tables 5-1, 6-1, 7-1). Balance Line (5th dimension): not testable (see bwntd_balance_line)."],
            "PDF p.36-76; S&P futures Apr 17-Jul 31 tables (Russian edition images): AO 7 entries (Table 5-1), AC 5 (Table 6-1), Zone 10 (Table 7-1), all marked to the Jul 31 close; compared on SPX 1997 in library/bwntd/README.md. Run on the index (instrument swap)."),
        make("bwntd_ao_flip", "AO only: sell when it turns down, buy when it turns up", "NTD ch. 5 · Awesome Oscillator", "Long/Short", Status::Live,
            &["F|AO = SMA5((H+L)/2) - SMA34((H+L)/2) (p.46; author's code TC2 p.218).",
              "E|\"when the oscillator turns down, you simply call your broker and say, 'Sell at the market.' You stay short until it turns up, and then call and say, 'Buy at the market.'\" (p.46; same text TC2 p.126-127). Turn = histogram bar lower (higher) than the previous bar after the last turn the other way; equal bars do not turn it.",
              "X|\"You stay short until it turns up\" (p.46): the opposite turn closes the position and reverses it. Market order after the daily close shows the turn = next open [book silent on the fill]. Always in the market."],
            "PDF p.46. The book adds \"I do not recommend this because you can get much more precise\" - it is the book's own stated all-AO system. Run on the index."),
        make("bwntd_balance_line", "Balance Line trades (5th dimension)", "NTD ch. 8 · Balance Line", "Long/Short", Status::NotTestable(BL_WHY),
            &["E|\"If we are going toward the Balance Line, we place our signal at base + two higher highs (lower lows). If we are going away from the Balance Line, we use base + one higher high (lower low).\" (p.68)",
              "F|\"The base bar is either the current bar or the most recent lowest high.\" (p.68) \"The buy stop stays in place unless (a) it is triggered or (b) another bar with a lower high is formed\" (p.69)",
              "F|\"If the current bar is in the Red Zone, you double the number of new higher highs to create a buy signal. If the current bar is in the Green Zone, you double the number of lower lows\" (p.70-71)"],
            "Book result: Table 8-1 S&P Apr 17-Jul 31, 18 entries (Russian edition image). Not coded: look-back and side test undefined."),
        make("bwntd_green_line", "Green-line stop in a Runaway market", "NTD ch. 9 · Taking profits", "Long/Short",
            Status::NotTestable("the Green-line close-only stop applies only in a \"Runaway market ... when the angle of the price movement is greater than the angle of the Green line\" (p.74): chart judgement"),
            &["X|\"In a Runaway market, we use the Green line as a stop close only stop\" (p.74)"], "Exit option only; trigger is judgement."),
        make("bwntd_divergence", "Divergence and the five magic bullets", "NTD ch. 9 · End of a trend", "Long/Short",
            Status::NotTestable("needs an Elliott-wave target zone, price/AO peaks chosen by eye and squat bars (MFI with volume, Trading Chaos); no entry or exit order is given (p.75-76)"),
            &["F|\"Most markets tend to turn on a double divergence\" (p.75)", "F|Five magic bullets: target zone from Elliott Wave count, divergence, a fractal, a squat bar, a change in momentum (p.75-76)"],
            "Early-warning description only."),
        make("bwntd_mouth", "Take profits when the Alligator closes its mouth", "NTD ch. 3 · Alligator", "Long/Short",
            Status::NotTestable("\"Whenever the Alligator has had enough, he starts to close his mouth ... This is the time you want to start taking profits\" (p.38): no measurable definition of sleeping/closing"),
            &["X|\"Notice that the mouth started to close in early September, telling us to take profit\" (p.38)"], "Judgement."),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    Some(match id {
        "bwntd_fractal" => ("bwntd_system", "Profitunity dimensions (NTD)", "Fractal only"),
        "bwntd_dims" => ("bwntd_system", "Profitunity dimensions (NTD)", "Fractal + AO + AC + Zone"),
        "bwntd_balance_line" => ("bwntd_system", "Profitunity dimensions (NTD)", "Balance Line · not testable"),
        "bwntd_green_line" => ("bwntd_system", "Profitunity dimensions (NTD)", "Green-line stop · not testable"),
        "bwntd_mouth" => ("bwntd_system", "Profitunity dimensions (NTD)", "Mouth-closing exit · not testable"),
        "bwntd_ao_flip" => ("bwntd_ao", "Awesome Oscillator alone", "Turn up/down, always in"),
        "bwntd_divergence" => ("bwntd_ao", "Awesome Oscillator alone", "Divergence · not testable"),
        _ => return None,
    })
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    let common = || vec![
        p("teeth_n", "Red line (Teeth): smoothed bars", 8., 3., 21., 1.),
        p("teeth_off", "Red line offset (bars into the future)", 5., 0., 13., 1.),
        p("ao_fast", "AO fast SMA", 5., 2., 13., 1.),
        p("ao_slow", "AO slow SMA", 34., 13., 89., 1.),
        p("ac_n", "AC average length", 5., 2., 13., 1.),
        p("ac_form", "AC: 1 = AO - SMA5(AO) (fits Tables 6-1/7-1), 0 = SMA5(AO - SMA5(AO)) literal first sentence", 1., 0., 1., 1.),
        p("five", "Same-colour bars before the trailing stop", 5., 3., 8., 1.),
        p("exit", "Exit: 0 Red line + five-bar trail, 1 Red line only, 2 five-bar trail only", 0., 0., 2., 1.),
        p("reverse", "Opposite valid fractal reverses the position (1) or is ignored (0)", 1., 0., 1., 1.),
        p("pyramid", "Add one unit per further signal (1) or first signal only (0)", 1., 0., 1., 1.),
    ];
    match id {
        "bwntd_fractal" => Some(common()),
        "bwntd_dims" => {
            let mut v = common();
            v.extend([
                p("ao", "AO signals on (1) / off (0)", 1., 0., 1., 1.),
                p("ac", "AC signals on (1) / off (0)", 1., 0., 1., 1.),
                p("zone", "Zone add-ons on (1) / off (0)", 1., 0., 1., 1.),
                p("zone_max", "Stop zone adds after this many same-colour bars", 5., 3., 8., 1.),
            ]);
            Some(v)
        }
        "bwntd_ao_flip" => Some(vec![p("ao_fast", "AO fast SMA", 5., 2., 13., 1.), p("ao_slow", "AO slow SMA", 34., 13., 89., 1.)]),
        _ => None,
    }
}

// ---------------------------------------------------------------- indicators (shared with bwtc2)

/// Tick for "one tick above/below": the instrument's real tick - 0.01 on US indices, ETFs and stocks, 0.05 on NSE.
/// (Fix 24 Oct 2026: US stocks used to get 0.05 because only index/ETF tickers were listed.)
pub fn tick_of(x: &Ctx) -> f64 {
    x.market_tick()
}

fn shift(v: &[f64], k: usize) -> Vec<f64> {
    (0..v.len()).map(|i| if i >= k { v[i - k] } else { f64::NAN }).collect()
}

/// +1 higher than the previous bar, -1 lower, 0 equal or missing.
pub fn colour(v: &[f64]) -> Vec<i8> {
    (0..v.len()).map(|i| if i == 0 { 0 } else if v[i] > v[i - 1] { 1 } else if v[i] < v[i - 1] { -1 } else { 0 }).collect()
}

pub struct Ind {
    pub teeth: Vec<f64>,
    pub ao: Vec<f64>,
    pub ac: Vec<f64>,
    pub aoc: Vec<i8>,
    pub acc: Vec<i8>,
    pub zone: Vec<i8>,  // +1 green, -1 red, 0 gray
    pub run: Vec<usize>, // consecutive bars of the current non-gray zone colour, incl. this bar
}

pub fn ind(x: &Ctx, teeth_n: usize, teeth_off: usize, fast: usize, slow: usize, ac_n: usize, ac_form: u8) -> Ind {
    let med: Vec<f64> = (0..x.r.len()).map(|i| (x.h[i] + x.l[i]) / 2.0).collect();
    let teeth = shift(&wilder(&med, teeth_n), teeth_off);
    let (f, s) = (sma(&med, fast), sma(&med, slow));
    let ao: Vec<f64> = f.iter().zip(&s).map(|(a, b)| a - b).collect();
    let m = sma(&ao, ac_n);
    let diff: Vec<f64> = ao.iter().zip(&m).map(|(a, b)| a - b).collect();
    let ac = if ac_form == 0 { sma(&diff, ac_n) } else { diff };
    let (aoc, acc) = (colour(&ao), colour(&ac));
    let zone: Vec<i8> = aoc.iter().zip(&acc).map(|(&a, &b)| if a == b { a } else { 0 }).collect();
    let mut run = vec![0usize; zone.len()];
    for i in 0..zone.len() {
        run[i] = if zone[i] == 0 { 0 } else if i > 0 && zone[i - 1] == zone[i] { run[i - 1] + 1 } else { 1 };
    }
    Ind { teeth, ao, ac, aoc, acc, zone, run }
}

/// Bar k on which the fractal centred on m is confirmed (2 strictly lower highs - higher lows for `up=false` -
/// on each side, equal bars skipped, any bar beyond the middle cancels).
fn confirm(v: &[f64], m: usize, up: bool) -> Option<usize> {
    let beyond = |a: f64, b: f64| if up { a > b } else { a < b };
    let (mut n, mut j) = (0, m);
    while n < 2 {
        if j == 0 { return None; }
        j -= 1;
        if beyond(v[j], v[m]) { return None; }
        if beyond(v[m], v[j]) { n += 1; }
    }
    let (mut n, mut k) = (0, m);
    while n < 2 {
        k += 1;
        if k >= v.len() || beyond(v[k], v[m]) { return None; }
        if beyond(v[m], v[k]) { n += 1; }
    }
    Some(k)
}

/// For each bar k: the latest middle bar of an up (down) fractal confirmed on k.
pub fn fractals(x: &Ctx) -> (Vec<Option<usize>>, Vec<Option<usize>>) {
    let n = x.r.len();
    let (mut up, mut dn) = (vec![None; n], vec![None; n]);
    for m in 0..n {
        if let Some(k) = confirm(&x.h, m, true) { up[k] = Some(m); }
        if let Some(k) = confirm(&x.l, m, false) { dn[k] = Some(m); }
    }
    (up, dn)
}

/// AO signal on bar i: +1 buy (saucer, cross or twin peaks), -1 sell, 0 none (p.47-52).
pub fn ao_signal(ao: &[f64], i: usize) -> i8 {
    if i < 3 { return 0; }
    let (a, b, c) = (ao[i - 2], ao[i - 1], ao[i]);
    // twin peaks: b is a trough (peak) and the previous trough on the same side of zero is more extreme
    let twin = |s: f64| {
        // s = -1 buy side (troughs below zero), +1 sell side (peaks above zero); w = s*AO turns both into peaks
        if !(s * c > 0.0 && s * b > s * a) { return false; }
        let mut j = i - 2;
        while j >= 1 {
            if !(s * ao[j] > 0.0) { return false; }
            if s * ao[j] > s * ao[j - 1] && s * ao[j] > s * ao[j + 1] { return s * ao[j] > s * b; }
            j -= 1;
        }
        false
    };
    if c > b {
        let saucer = a > 0.0 && b > 0.0 && c > 0.0 && b < a;
        let cross = b < 0.0 && c > 0.0;
        if saucer || cross || twin(-1.0) { return 1; }
    } else if c < b {
        let saucer = a < 0.0 && b < 0.0 && c < 0.0 && b > a;
        let cross = b > 0.0 && c < 0.0;
        if saucer || cross || twin(1.0) { return -1; }
    }
    0
}

/// AC signals (p.56-60): after a red bar, the first bar of the green run with (2+ green and AC > 0) or 3+ green.
pub fn ac_signals(ac: &[f64], acc: &[i8]) -> Vec<i8> {
    let mut out = vec![0i8; ac.len()];
    let (mut k, mut fired, mut col) = (0usize, false, 0i8);
    for i in 1..ac.len() {
        let c = acc[i];
        if c == 0 { k = 0; col = 0; continue; }
        if c != col {
            if acc[i - 1] != -c { k = 0; col = 0; continue; } // run must start right after an opposite bar
            col = c; k = 0; fired = false;
        }
        k += 1;
        if !fired && ((k >= 2 && c as f64 * ac[i] > 0.0) || k >= 3) { out[i] = c; fired = true; }
    }
    out
}

// ---------------------------------------------------------------- order engine

/// A resting stop order: dir +1 buy stop, -1 sell stop.
#[derive(Clone, Copy)]
pub struct Stop { pub px: f64, pub dir: i8 }

/// When a stop at `px` fills on bar i: (time, fill). time 0 = open gap, 1/2 = first/second leg of the bar path
/// open -> nearer extreme -> other extreme.
pub fn fill_time(x: &Ctx, i: usize, s: Stop) -> Option<(u8, f64)> {
    let (o, h, l) = (x.o[i], x.h[i], x.l[i]);
    let up_first = h - o <= o - l;
    if s.dir > 0 {
        if o >= s.px { Some((0, o)) } else if h >= s.px { Some((if up_first { 1 } else { 2 }, s.px)) } else { None }
    } else if o <= s.px { Some((0, o)) } else if l <= s.px { Some((if up_first { 2 } else { 1 }, s.px)) } else { None }
}

/// Sort key so events happen in path order: time, then along the leg (rising legs by price, falling legs reversed).
pub fn path_key(t: u8, px: f64, dir: i8) -> (u8, f64) { (t, if dir > 0 { px } else { -px }) }

pub struct Pos { pub t: Trade, pub stop: Option<f64>, pub units: usize }

pub fn open_pos(dir: f64, i: usize, px: f64, w: f64, kind: &'static str) -> Pos {
    Pos { t: Trade { dir, legs: vec![Leg { i, px, w }], exit_i: i, exit_px: px, reason: String::new(), entry_kind: kind,
        signal_i: i, open: false, exits: vec![] }, stop: None, units: 1 }
}

pub fn close_pos(p: Pos, i: usize, px: f64, why: &str, out: &mut Vec<Trade>) {
    let mut t = p.t;
    t.exit_i = i; t.exit_px = px; t.signal_i = i; t.reason = why.into();
    out.push(t);
}

/// Pending AO/AC order of one dimension: cancelled when the histogram colour changes, otherwise moved to the
/// latest bar ("blue light special"), replaced by a newer signal.
pub fn manage(p: &mut Option<Stop>, col: i8, sig: i8, x: &Ctx, i: usize, tick: f64) {
    if let Some(s) = *p {
        *p = if col == s.dir { Some(Stop { px: if s.dir > 0 { x.h[i] + tick } else { x.l[i] - tick }, dir: s.dir }) } else { None };
    }
    if sig != 0 { *p = Some(Stop { px: if sig > 0 { x.h[i] + tick } else { x.l[i] - tick }, dir: sig }); }
}

struct Cfg { ao: bool, ac: bool, zone: bool, exit: u8, reverse: bool, pyramid: bool, five: usize, zone_max: usize }

#[derive(Clone, Copy, PartialEq)]
enum Src { FracUp, FracDn, Ao, Ac, Trail }

fn system(x: &Ctx, from: usize, to: usize, d: &Ind, cfg: &Cfg) -> Vec<Trade> {
    let tick = tick_of(x);
    let (fup, fdn) = fractals(x);
    let acs = ac_signals(&d.ac, &d.acc);
    let mut out = vec![];
    let mut pos: Option<Pos> = None;
    let (mut pu, mut pd): (Option<(usize, f64)>, Option<(usize, f64)>) = (None, None); // pending fractal (middle, stop price)
    let (mut last_u, mut last_d) = (None::<usize>, None::<usize>);
    let (mut pao, mut pac): (Option<Stop>, Option<Stop>) = (None, None);
    let mut armed = 0i8;
    let red = cfg.exit != 2;
    let trail = cfg.exit != 1;
    for i in 0..=to {
        let live = i >= from;
        // ---- intraday stop events in bar-path order
        let mut ev: Vec<((u8, f64), Src, Stop, f64)> = vec![]; // (path key, source, fill, order price)
        let mut push = |s: Option<Stop>, src: Src| if let Some(s) = s { if let Some((t, f)) = fill_time(x, i, s) { ev.push((path_key(t, f, s.dir), src, Stop { px: f, dir: s.dir }, s.px)) } };
        push(pu.map(|(_, p)| Stop { px: p, dir: 1 }), Src::FracUp);
        push(pd.map(|(_, p)| Stop { px: p, dir: -1 }), Src::FracDn);
        if cfg.ao { push(pao, Src::Ao); }
        if cfg.ac { push(pac, Src::Ac); }
        if let Some(p) = &pos { push(p.stop.map(|s| Stop { px: s, dir: -(p.t.dir as i8) }), Src::Trail); }
        ev.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap().then((a.1 != Src::Trail).cmp(&(b.1 != Src::Trail))));
        for (_, src, s, order) in ev {
            let dir = s.dir;
            let outside = |px: f64| if dir > 0 { px > d.teeth[i] } else { px < d.teeth[i] };
            let mut take: Option<&'static str> = None;
            match src {
                Src::Trail => {
                    if pos.as_ref().is_some_and(|p| p.t.dir as i8 == -dir) {
                        close_pos(pos.take().unwrap(), i, s.px, &format!("{}-bar same-colour trailing stop", cfg.five), &mut out);
                    }
                    continue;
                }
                Src::FracUp | Src::FracDn => {
                    if src == Src::FracUp { pu = None } else { pd = None }
                    if outside(order) { armed = dir; take = Some("fractal"); }
                }
                Src::Ao | Src::Ac => {
                    let (st, kind) = if src == Src::Ao { (&mut pao, "AO") } else { (&mut pac, "AC") };
                    *st = None;
                    if armed == dir && outside(order) { take = Some(kind); }
                }
            }
            let Some(kind) = take else { continue };
            if !live { continue; }
            match pos.as_mut() {
                Some(p) if p.t.dir as i8 == dir => {
                    if cfg.pyramid { p.t.legs.push(Leg { i, px: s.px, w: 1.0 }); p.units += 1; }
                }
                Some(_) => {
                    if kind == "fractal" && cfg.reverse {
                        close_pos(pos.take().unwrap(), i, s.px, "Opposite fractal · reverse", &mut out);
                        pos = Some(open_pos(dir as f64, i, s.px, 1.0, kind));
                    }
                }
                None => pos = Some(open_pos(dir as f64, i, s.px, 1.0, kind)),
            }
        }
        // ---- close: Red-line stop-close-only, then zone add-on on the close
        if red {
            if let Some(p) = &pos {
                let below = if p.t.dir > 0.0 { x.c[i] < d.teeth[i] } else { x.c[i] > d.teeth[i] };
                if below { close_pos(pos.take().unwrap(), i, x.c[i], "Close beyond the Red line (stop close only)", &mut out); }
            }
        }
        if cfg.zone && live && i > 0 && armed != 0 {
            let z = d.zone[i];
            let ok = z == armed && d.zone[i - 1] == z && (x.c[i] - x.c[i - 1]) * z as f64 > 0.0 && d.run[i] <= cfg.zone_max
                && (x.c[i] - d.teeth[i]) * z as f64 > 0.0;
            if ok {
                match pos.as_mut() {
                    Some(p) if p.t.dir as i8 == z => if cfg.pyramid { p.t.legs.push(Leg { i, px: x.c[i], w: 1.0 }); p.units += 1; },
                    Some(_) => {}
                    None => pos = Some(open_pos(z as f64, i, x.c[i], 1.0, "zone")),
                }
            }
        }
        if i == to { break; }
        // ---- end of bar: orders for the next bar
        if let Some(p) = pos.as_mut() {
            if trail {
                let dir = p.t.dir as i8;
                let lvl = if dir > 0 { x.l[i] - tick } else { x.h[i] + tick };
                match p.stop {
                    Some(s) => p.stop = Some(if dir > 0 { s.max(lvl) } else { s.min(lvl) }),
                    None => if d.zone[i] == dir && d.run[i] >= cfg.five { p.stop = Some(lvl) },
                }
            }
        }
        if let Some(m) = fup[i] { if last_u.map_or(true, |u| m > u) { last_u = Some(m); pu = Some((m, x.h[m] + tick)); } }
        if let Some(m) = fdn[i] { if last_d.map_or(true, |u| m > u) { last_d = Some(m); pd = Some((m, x.l[m] - tick)); } }
        manage(&mut pao, d.aoc[i], ao_signal(&d.ao, i), x, i, tick);
        manage(&mut pac, d.acc[i], acs[i], x, i, tick);
    }
    if let Some(p) = pos {
        let mut t = p.t;
        t.exit_i = to; t.exit_px = x.c[to]; t.signal_i = to; t.reason = "open".into(); t.open = true;
        out.push(t);
    }
    out
}

fn ao_flip(x: &Ctx, from: usize, to: usize, ao: &[f64]) -> Vec<Trade> {
    let col = colour(ao);
    let mut out = vec![];
    let mut pos: Option<Pos> = None;
    let mut last = 0i8;
    for i in 0..to {
        let c = col[i];
        if c == 0 || c == last { continue; }
        let first = last == 0;
        last = c;
        if first || i < from { continue; } // the first colour is not a turn
        let (k, px) = (i + 1, x.o[i + 1]);
        if let Some(p) = pos.take() {
            let mut t = p.t;
            t.exit_i = k; t.exit_px = px; t.signal_i = i; t.reason = "AO turned · reverse at the next open".into();
            out.push(t);
        }
        let mut p = open_pos(c as f64, k, px, 1.0, "open");
        p.t.signal_i = i;
        pos = Some(p);
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
    let u = |k: &str| g(k).max(0.0).round() as usize;
    if id == "bwntd_ao_flip" {
        let med: Vec<f64> = (0..x.r.len()).map(|i| (x.h[i] + x.l[i]) / 2.0).collect();
        let ao: Vec<f64> = sma(&med, u("ao_fast").max(1)).iter().zip(sma(&med, u("ao_slow").max(1))).map(|(a, b)| a - b).collect();
        return Some(ao_flip(x, from, to, &ao));
    }
    let d = ind(x, u("teeth_n").max(1), u("teeth_off"), u("ao_fast").max(1), u("ao_slow").max(1), u("ac_n").max(1), u("ac_form") as u8);
    let dims = id == "bwntd_dims";
    let cfg = Cfg { ao: dims && u("ao") == 1, ac: dims && u("ac") == 1, zone: dims && u("zone") == 1, exit: u("exit") as u8,
        reverse: u("reverse") == 1, pyramid: u("pyramid") == 1, five: u("five").max(1), zone_max: if dims { u("zone_max") } else { 5 } };
    Some(system(x, from, to, &d, &cfg))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;
    fn rows(hl: &[(f64, f64)]) -> Vec<Row> {
        hl.iter().enumerate().map(|(i, &(h, l))| Row { ticker: "SPX".into(), date: format!("{:04}-01-01", 2000 + i),
            open: (h + l) / 2.0, high: h, low: l, close: (h + l) / 2.0, volume: None }).collect()
    }
    #[test]
    fn fractal_needs_two_lower_highs_each_side_and_skips_equal_bars() {
        // pattern D (p.41): middle at 2, bar 4 equals it, confirmed on bar 5
        let r = rows(&[(10., 9.), (11., 9.), (13., 9.), (12., 9.), (13., 9.), (11., 9.), (10., 9.)]);
        let x = Ctx::new(&r, &r, &[]);
        assert_eq!(confirm(&x.h, 2, true), Some(5));
        // a higher bar after the middle cancels
        let r = rows(&[(10., 9.), (11., 9.), (13., 9.), (14., 9.), (11., 9.), (10., 9.)]);
        let x = Ctx::new(&r, &r, &[]);
        assert_eq!(confirm(&x.h, 2, true), None);
    }
    #[test]
    fn ao_saucer_cross_and_twin_peaks() {
        assert_eq!(ao_signal(&[1., 3., 2., 2.5], 3), 1); // saucer above zero
        assert_eq!(ao_signal(&[0., -1., -0.5, 0.2], 3), 1); // cross
        assert_eq!(ao_signal(&[-1., -5., -3., -4., -3.5], 4), 1); // twin peaks: -4 above -5, no zero cross
        assert_eq!(ao_signal(&[-1., -3., -2., -4., -3.5], 4), 0); // second trough lower: no signal
        assert_eq!(ao_signal(&[-1., -3., -2.5, -2., -2.4], 4), -1); // saucer sell below zero
    }
    #[test]
    fn ac_needs_two_green_above_zero_three_below() {
        let ac = [0., -1., -2., -1.5, -1.0, -0.5, 1., 0.5, 0.8, 0.9];
        let acc = colour(&ac);
        let s = ac_signals(&ac, &acc);
        assert_eq!(s[4], 0); assert_eq!(s[5], 1); // third green bar below zero
        assert_eq!(s[9], 1); // after a red bar, two green above zero
    }
    #[test]
    fn gap_through_a_buy_stop_fills_at_the_open() {
        let mut r = rows(&[(10., 9.); 3]);
        r[1].open = 12.; r[1].high = 13.; r[1].low = 11.5;
        let x = Ctx::new(&r, &r, &[]);
        assert_eq!(fill_time(&x, 1, Stop { px: 10.5, dir: 1 }), Some((0, 12.)));
        assert_eq!(fill_time(&x, 1, Stop { px: 12.5, dir: 1 }).map(|v| v.1), Some(12.5));
    }
}

// Instrument admission: BWilliams 2026-10-24 block, moved from instrument_policy.rs on 31 Oct 2026 (lane ST).
// Admission only for the module REVIEWED lists (authors/bwilliams/admission/SOURCE-CONTRACT.md); kept in one block to merge.
// Runs on actual indices, ETFs and stocks, US and India (index_etf_or_stock, nifty50 basket, declared application).
pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| matches!(def.book,"bwntd"|"bwtc1"|"bwtc2") && (crate::books::bwntd::reviewed(def.id) || crate::books::bwtc1::reviewed(def.id) || crate::books::bwtc2::reviewed(def.id)),
    label: "Bill Williams taught these rules on futures (coffee, S&P, gold, bonds) and stocks; runs on indices, ETFs and stocks, US and India, are a declared application, not the author's test. Cash-index shorts are hypothetical exposure.",
    basis: "Bill Williams: New Trading Dimensions (1998) fractal outside the Alligator (p.36-45, 72-74) and AO turn system (p.46); Trading Chaos (1995) fractal start/signal/stop (p.135-142) and 5/13/34 rhythm (p.181-186); Trading Chaos 2nd ed. (2004) Fractal + Super AO with trailing stop (p.128-156). Reviewed in authors/bwilliams/admission/SOURCE-CONTRACT.md with independent chronological ledgers; no costs.",
};
