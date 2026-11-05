//! Walter T. Downs, "Combining Statistical And Pattern Analysis", Technical Analysis of Stocks & Commodities V.16:10 (October
//! 1998) pp.447-456, sidebar "TradeStation code" (research share Trading Knowledge I/Stocks & Commodities DVD 11.26 (1982 - 2008)
//! (traders.com)/VOLUMES/V16/C10/073COMB.pdf). Book id `scdshark`: the SHRK-32 system. Lane K16 (titles loop, 2 Nov 2026),
//! to-code row 249, decisions 19, 58, 82; independent check authors/titles-1030/K16w/k16_ref.py.
use crate::connors::{Ctx, ParamDef, Params, Status, StrategyDef, Trade};
use super::lc3::high_first;
use super::tselhs::{india, Book};

pub const REVIEWED: &[&str] = &["scdshark_32"];
pub fn reviewed(id: &str) -> bool { REVIEWED.contains(&id) }

const NOTE: &str = "[Declared readings] The printed TradeStation system (sidebar, PDF p.9), read as TradeStation runs it: the code runs at each bar's close with the position held at the start of the bar; 'on the Close' orders fill at that close; when several orders fire on one bar the last one in code order sets the position (an ExitLong / ExitShort only cancels the side held at the start of the bar); 'Buy / Sell on the Open' after the 78-bar exit = the next bar's open. SYM_SHARK = two nested inside bars whose apex (H+L)/2 lies within the base bar's range shrunk by Symmetry x its width on each side. Symmetry 0.28 is the code's input default (decision 82); the article's best, 0.37 (+/-13%), is an option (tested 0.28-0.48). The C1 / RevB / RevS counters (waiting inside the base, provisional reversals through the pattern) are coded literally, including C1 counting on after a signal. Money management from the text (not in the code): US instruments use the $2.00 initial stop and the $5.00 profit target per share (intraday, from the bar after entry; stop or target at its level or the open beyond it; both touched: the extreme nearer the open first); on Indian instruments they are held ('dollar exit held', decisions 19, 58). HELD everywhere: the $2.50 trailing profit stop 'with TradeStation bouncing ticks set to 50%' (that tester setting is not defined in the article). Article: Harley-Davidson (HDI) tests by symmetry value (Figure 11), 0.37 the most profitable. Lead decision 195: the $2 stop and $5 target are part of the rule, so the card is published only on US index and ETF prices (amounts literal); results without them on other instruments are withdrawn, and US single stocks (HDI included) are withheld while only split-adjusted prices exist (decision 166).";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef { id: "scdshark_32", name: "Downs SHARK-32: trade the close beyond a symmetrical two-inside-bar triangle, provisional reversals, 78-bar exit", group: "S&C: Combining Statistical And Pattern Analysis (Downs, V.16:10)", session: 0, dir: "Long/Short", status: Status::Live,
        needs_volume: false, needs_vix: false, needs_series: &[], book: "scdshark", rules: &[
            "F|\"Condition1 = H<H[1] and L>L[1] and H[1]<H[2] and L[1]>L[2]; Condition2 = value3 <= H[2]-(value4*Symmetry) AND value3 >= L[2]+(value4*Symmetry);\" with value3 = (H+L)/2, value4 = H[2]-L[2] (V.16:10, PDF p.9)",
            "E|\"If Sym_Shark(symmetry)[1] = True then begin If C > H[3] then Buy on the Close; If C < L[3] then Sell on the Close; C1 = C1 + 1;\" and while flat \"If C > H[4+C1] then begin Buy on the Close; RevB = C1+3;\" (PDF p.9)",
            "X|\"If RevB <> 0 then begin If C > L[RevB] then RevB = RevB + 1 Else if C < L[RevB] then begin Sell on the Close;\" (provisional reversal, shorts mirrored); \"If BarsSinceEntry = 78 then ... ExitLong on the Close;\" (PDF p.9)",
            "X|\"Initial money-management stop: $2.00, or reversal exit due to provisional signal. Trailing profit stop: $2.50 ... Profit target: $5.00\" (PDF p.8): $2 / $5 on US instruments only, trailing stop held"],
        notes: NOTE, claim: None }]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    reviewed(id).then_some(("scdshark_32", "Downs SHARK-32 (S&C)", "Symmetry 0.28"))
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    reviewed(id).then(|| vec![p("sym", "Symmetry (code default 0.28; article best 0.37)", 0.28, 0.0, 0.48, 0.01), p("usd", "1 = $2 stop / $5 target on US instruments (text); 0 = code only", 1., 0., 1., 1.)])
}

fn shark(x: &Ctx, t: usize, sym: f64) -> bool {
    if t < 2 { return false; }
    let (h, l) = (&x.h, &x.l);
    let (v3, v4) = ((h[t] + l[t]) / 2.0, h[t - 2] - l[t - 2]);
    h[t] < h[t - 1] && l[t] > l[t - 1] && h[t - 1] < h[t - 2] && l[t - 1] > l[t - 2] && v3 <= h[t - 2] - v4 * sym && v3 >= l[t - 2] + v4 * sym
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !reviewed(id) || from >= to || to >= x.r.len() { return None; }
    let sym = p.get("sym").copied().unwrap_or(0.28);
    let usd = p.get("usd").copied().unwrap_or(1.) > 0.5 && !india(x);
    let (h, l, c) = (&x.h, &x.l, &x.c);
    let (mut c1, mut revb, mut revs) = (0usize, 0usize, 0usize);
    let mut pos = Book::new();
    let mut at_open: Option<f64> = None;
    for t in from..=to {
        if let Some(d) = at_open.take() { pos.enter(d, t, x.o[t], "open"); }
        // US money management: intraday from the bar after entry
        if usd { if let Some(e) = pos.entry_bar() { if t > e {
            let (d, ep, o) = (pos.side(), pos.entry_px(), x.o[t]);
            let (st, tg) = (ep - 2.0 * d, ep + 5.0 * d);
            let (hs, ht) = if d > 0.0 { (l[t] <= st, h[t] >= tg) } else { (h[t] >= st, l[t] <= tg) };
            let stop_first = if hs && ht { if d > 0.0 { o <= st || (o < tg && !high_first(o, h[t], l[t])) } else { o >= st || (o > tg && high_first(o, h[t], l[t])) } } else { hs };
            if hs && stop_first { pos.exit(t, if d > 0.0 { o.min(st) } else { o.max(st) }, "$2 stop"); }
            else if ht { pos.exit(t, if d > 0.0 { o.max(tg) } else { o.min(tg) }, "$5 target"); }
        } } }
        let mp = pos.side();
        let mut tgt = mp;
        let mut next_open: Option<f64> = None;
        if c1 != 0 && mp == 0.0 {
            if t >= 3 + c1 && c[t] < h[t - 3 - c1] && c[t] > l[t - 3 - c1] { c1 += 1; }
            if t >= 4 + c1 && c[t] > h[t - 4 - c1] { tgt = 1.0; revb = c1 + 3; c1 = 0; }
            if t >= 3 + c1 && c[t] < l[t - 3 - c1] { tgt = -1.0; revs = c1 + 3; c1 = 0; }
        }
        if revb != 0 && t >= revb { if c[t] > l[t - revb] { revb += 1 } else if c[t] < l[t - revb] { tgt = -1.0; revb = 0; } }
        if revs != 0 && t >= revs { if c[t] < h[t - revs] { revs += 1 } else if c[t] > h[t - revs] { tgt = 1.0; revs = 0; } }
        if t >= 1 && shark(x, t - 1, sym) {
            if t >= 3 && c[t] > h[t - 3] { tgt = 1.0; }
            if t >= 3 && c[t] < l[t - 3] { tgt = -1.0; }
            c1 += 1;
        }
        if shark(x, t, sym) && mp != 0.0 { if mp == tgt { tgt = 0.0; } }
        if let Some(e) = pos.entry_bar() { if t - e == 78 && mp != 0.0 {
            if tgt == mp { tgt = 0.0; }
            next_open = Some(mp);
        } }
        if revb > 100 { revb = 0; }
        if revs > 100 { revs = 0; }
        if tgt != pos.side() { if tgt == 0.0 { pos.exit(t, c[t], "exit on the close") } else { pos.enter(tgt, t, c[t], "close") } }
        if t < to { at_open = next_open.filter(|&d| d != tgt); }
    }
    Some(pos.done(to, x.c[to]))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn shark_pattern_and_run() {
        use crate::Row;
        let mk = |h: f64, l: f64| Row { ticker: "T".into(), date: "2001-01-01".into(), open: (h + l) / 2., high: h, low: l, close: (h + l) / 2., volume: None };
        let r = vec![mk(110., 90.), mk(105., 95.), mk(102., 98.)];
        let x = Ctx::new(&r, &r, &[]);
        assert!(shark(&x, 2, 0.28) && !shark(&x, 2, 0.51));
        let b = super::super::sctraub::tests::bars(3000);
        let x = Ctx::new(&b, &b, &[]);
        let mut p = Params::new();
        p.insert("sym".into(), 0.0);
        assert!(run("scdshark_32", &x, 0, b.len() - 1, &p).is_some());
    }
}

// Instrument admission: S&C Downs SHARK-32 2026-11-02 (titles lane K16).
pub const ADMISSION: crate::instrument_policy::Admission = crate::instrument_policy::Admission {
    reviewed: |def: &crate::connors::StrategyDef| def.book == "scdshark" && reviewed(def.id),
    label: "Walter Downs tested SHARK-32 on Harley-Davidson stock (dollar stops per share). Published only on US indices and ETFs, where the $2 stop and $5 target run literally (lead decision 195); a declared application, not the author's test.",
    basis: "Technical Analysis of Stocks & Commodities V.16:10 pp.447-456 (Combining Statistical And Pattern Analysis), sidebar TradeStation code; independent check authors/titles-1030/K16w/k16_ref.py",
};
