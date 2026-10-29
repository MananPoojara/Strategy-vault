//! Robert Krausz, "W.D. Gann Treasure Discovered: Simple Trading Plans for Stocks & Commodities" (1998; scanned; book id
//! `krgann`). Reading: claude-continuation-20261020/authors/krausz (OCR authors/lanec/q241/text/krgann.ocr.txt checked against
//! the page images; independent reference authors/lanec/admission4/krausz_ref.py). Admitted: the Basic Gann Swing Plan (rules
//! p.29 = PDF 18, illustrated p.30-39, stock notes p.103-104 = PDF 55-56). Held: the Professional Plan (T-bonds, 10/20-minute
//! opening rules = intraday data, weekly HiLo targets, pyramiding). Book: 30-year T-bond futures and five US stocks.
use crate::books::lc3::{high_first, Pos};
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};

pub const REVIEWED: &[&str] = &["krgann_basic"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

pub fn catalogue() -> Vec<StrategyDef> {
    vec![
        StrategyDef { id: "krgann_basic", name: "Basic Gann Swing Plan: 2-bar swing trend + 3-bar HiLo Activator, out on a close through the HiLo or a 38% retracement", group: "Basic Gann Swing Plan (p.29-39, stocks p.103)",
            session: 0, dir: "Long/Short", status: Status::Live, needs_volume: false, needs_vix: false, needs_series: &[], book: "krgann",
            rules: &["F|Swing: \"The Swing Direction can change to UP ONLY if you get TWO CONSECUTIVE HIGHER HIGHS ... to DOWN ONLY if you get TWO CONSECUTIVE LOWER LOWS\" (p.13). Trend: \"Prices must take out the nearest Peak and the Trend was previously Down\" / \"... the nearest Valley\" (p.14)",
                "F|HiLo Activator: \"we add the last three period's Highs (or Lows) and divide the sum by three ... For End of Day it is plotted one period forward\" (p.26); \"the market is up until you get a Close below the HiLo Activator Sell Stop, the Buy Stop will then appear\" (p.26)",
                "E|\"Use the one hit first. RULE 1 Buy when bar closes above HiLo Activator Buy Stop ... RULE 2 Buy when Gann Swing Chart changes from Downtrend to Uptrend. Must have HiLo Activator Sell Stop below the bar. RULE 3 Buy when nearest Peak is passed upwards ... You must also have HiLo Activator Sell Stop below the bar\"; qualifier \"Gann Swing Chart must show an Uptrend\"; shorts mirror (p.29)",
                "X|\"1 Take profit on all contracts if price closes above/below HiLo Activator. 2. 38% retracement rule ... on any 38% retracement of the current Gann Swing. Do not wait for the close. Must be past the HiLo Activator\" (p.29, p.38-39); stocks: \"do not use Profit Protection Rule #2 (.382 Retracement Rule) UNLESS the swing is approximately 8-10% of the current stock price\" (p.104)"],
            notes: "[Declared readings] End of Day form (p.104 note 9): the HiLo lines for a day are the 3-day averages of the highs / lows up to the day before. The swing turns up on the second consecutive higher high and down on the second consecutive lower low; the Valley is the lowest low from the last Peak bar to the bar before the swing turned up, the Peak the highest high from the last Valley bar to the bar before it turned down. Trend, Peaks and Valleys are those known at the previous close. Rules 2 and 3 are one stop order at the nearest Peak (Valley), live while flat with the HiLo in buy (sell) mode; it fills at the Peak or at the open when the day opens beyond it (the book's tables fill at the Peak even on a gap). Rule 1 and Profit Protection 1 fill at the close. Profit Protection 2: 38.2% back from the highest high since the last Valley (lowest low since the last Peak for shorts), used only when that level is beyond the HiLo line and the swing is at least 8% of the previous close (p.pp2min; 0 = the T-bond form, always), not on the entry day; filled at the level or a worse open. When both the Peak and the Valley are crossed in one day, the one nearer the open first. 'by 2 ticks' is the T-bond setting: 0 by default (the stock tables fill at the Peak itself), p.ticks = 2 adds 2 ticks (0.01 US, 0.05 NSE). No pyramiding; one unit, no costs. Long and short as in the book (p.long = 1: long only).",
            claim: None },
        StrategyDef { id: "krgann_pro", name: "Professional Gann Swing Plan (held: needs 10/20-minute bars)", group: "Professional Gann Swing Plan (p.258-262)", session: 0, dir: "Long/Short",
            status: Status::NotTestable("The Professional Plan adds counter-trend entries, opening-range rules on 10- and 20-minute bars, weekly HiLo targets on part of the position and pyramiding; it was tested on T-bond futures in real time. We hold no intraday T-bond data, and the partial targets and adds are not a single-unit rule."),
            needs_volume: false, needs_vix: false, needs_series: &[], book: "krgann",
            rules: &["F|\"This plan trades against the trend as well as with the trend ... We take profits at Target points on part of our total position ... You can add (Pyramid)\" (p.259)"],
            notes: "Held.", claim: None },
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    match id { "krgann_basic" => Some(("krgann_basic", "Krausz Gann swing plan", "Basic plan")), "krgann_pro" => Some(("krgann_pro", "Krausz Gann swing plan", "Professional plan")), _ => None }
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    match id {
        "krgann_basic" => Some(vec![
            ParamDef { key: "pp2min", label: "Use the 38% retracement exit only when the swing is at least this % of the price (book, stocks: about 8-10%; 0 = always, the T-bond form)", def: 8., min: 0., max: 20., step: 1. },
            ParamDef { key: "ticks", label: "Ticks beyond the HiLo line / Peak / Valley (book: 2 for T-bonds; the stock tables use 0)", def: 0., min: 0., max: 5., step: 1. },
            ParamDef { key: "long", label: "Long only (1) [Not from the book: the book also sells short]", def: 0., min: 0., max: 1., step: 1. },
        ]),
        "krgann_pro" => Some(vec![]),
        _ => None,
    }
}

/// Peak / Valley known at the close of each bar: (peak, valley, valley bar, peak bar); NaN before the first.
fn levels(h: &[f64], l: &[f64], to: usize) -> Vec<(f64, f64, usize, usize)> {
    let (mut sw, mut pk, mut vl, mut pki, mut vli) = (0i8, f64::NAN, f64::NAN, 0usize, 0usize);
    let mut out = Vec::with_capacity(to + 1);
    for i in 0..=to {
        if i >= 2 && h[i] > h[i - 1] && h[i - 1] > h[i - 2] && sw != 1 {
            if sw == -1 { let mut j = pki; for k in pki..i { if l[k] < l[j] { j = k; } } vl = l[j]; vli = j; }
            sw = 1;
        } else if i >= 2 && l[i] < l[i - 1] && l[i - 1] < l[i - 2] && sw != -1 {
            if sw == 1 { let mut j = vli; for k in vli..i { if h[k] > h[j] { j = k; } } pk = h[j]; pki = j; }
            sw = -1;
        }
        out.push((pk, vl, vli, pki));
    }
    out
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.r.len() { return None; }
    let g = |k: &str, d: f64| p.get(k).copied().unwrap_or(d);
    let off = g("ticks", 0.) * x.market_tick();
    let ppmin = g("pp2min", 8.) / 100.;
    let shorts = g("long", 0.) < 0.5;
    let (o, h, l, c) = (&x.o, &x.h, &x.l, &x.c);
    let lv = levels(h, l, to);
    let (mut hs, mut tr) = (0i8, 0i8);
    let mut pos = Pos::new();
    for i in 3..=to {
        let (pk, vl, vli, pki) = lv[i - 1];
        let hb = (h[i - 3] + h[i - 2] + h[i - 1]) / 3.;
        let hl = (l[i - 3] + l[i - 2] + l[i - 1]) / 3.;
        let trade = i >= from;
        let (up, dn) = (pk + off, vl - off);
        let (cu, cd) = (pk.is_finite() && h[i] > up, vl.is_finite() && l[i] < dn);
        let order: &[i8] = match (cu, cd) { (true, true) => if high_first(o[i], h[i], l[i]) { &[1, -1] } else { &[-1, 1] }, (true, false) => &[1], (false, true) => &[-1], _ => &[] };
        for &d in order {
            tr = d;
            if !trade || pos.side() != 0. { continue; }
            if d == 1 && hs == 1 { pos.enter(1., i, o[i].max(up), "stop"); }
            if d == -1 && hs == -1 && shorts { pos.enter(-1., i, o[i].min(dn), "stop"); }
        }
        if trade && pos.entry_bar().is_some_and(|e| e < i) {
            if pos.side() > 0. && vl.is_finite() {
                let top = h[vli..i].iter().copied().fold(f64::NEG_INFINITY, f64::max);
                let r = top - 0.382 * (top - vl);
                if top - vl >= ppmin * c[i - 1] && r < hl - off && l[i] <= r { pos.exit(i, o[i].min(r), "38% retracement (PP2)"); }
            } else if pos.side() < 0. && pk.is_finite() {
                let bot = l[pki..i].iter().copied().fold(f64::INFINITY, f64::min);
                let r = bot + 0.382 * (pk - bot);
                if pk - bot >= ppmin * c[i - 1] && r > hb + off && h[i] >= r { pos.exit(i, o[i].max(r), "38% retracement (PP2)"); }
            }
        }
        if hs != 1 && c[i] > hb + off {
            hs = 1;
            if trade {
                if pos.side() < 0. { pos.exit(i, c[i], "close above HiLo (PP1)"); }
                if tr == 1 && pos.side() == 0. { pos.enter(1., i, c[i], "close"); }
            }
        } else if hs != -1 && c[i] < hl - off {
            hs = -1;
            if trade {
                if pos.side() > 0. { pos.exit(i, c[i], "close below HiLo (PP1)"); }
                if tr == -1 && pos.side() == 0. && shorts { pos.enter(-1., i, c[i], "close"); }
            }
        }
    }
    Some(pos.done(to, c[to]))
}

#[cfg(test)]
mod tests {
    #[test]
    fn swing_levels() {
        // up 0-3, two lower lows (4,5) -> peak = high of bar 3; two higher highs (7,8) -> valley = lowest low 3..6
        let h = [10., 11., 12., 13., 12.5, 12., 11.8, 12.2, 12.6];
        let l = [9., 10., 11., 12., 11., 10., 10.5, 11., 11.5];
        let v = super::levels(&h, &l, 8);
        assert!(v[4].0.is_nan());
        assert_eq!((v[5].0, v[5].3), (13., 3));
        assert_eq!((v[8].1, v[8].2), (10., 5));
    }
}
