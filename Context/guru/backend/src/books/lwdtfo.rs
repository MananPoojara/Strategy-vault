//! Larry Williams, "Day Trade Futures Online" (Wiley 2000). Book id `lwdtfo`.
//! Source copy: Books/Larry Williams/Day Trade Futures Online.pdf (image scan, 118 PDF pages = two printed pages
//! each). Page images: library/lwdtfo/pages/d-NNN.png. Page refs below are printed pages.
//!
//! The copyright page says "Portions of this book have been reproduced from Long-Term Secrets to Short-Term
//! Trading". Chapters 4-10 reprint that book's text and printouts, so every system already coded in `lwlts` is NOT
//! repeated here (see library/lwdtfo/README.md for the duplicate map). This module holds only what `lwlts` does
//! not catalogue: the three S&P 500 rows of Tables 4.1-4.3 (p.76-77), "no stop was used and we were always
//! long/short", i.e. stop-and-reverse volatility breakouts.
//!
//! Simulator: `lwlts::simulate` (TradeStation default bar path, stops fill at their price or at the open on a gap,
//! one fill per direction per bar, an opposite entry exits and reverses). Not jkbwts::sim.
use crate::books::lwlts::{simulate, Cfg, Ent};
use crate::connors::{claim, Ctx, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "lwdtfo";

/// Larry Williams block: rules admitted after the 23 Oct 2026 reading and independent ledgers
/// (authors/lwilliams/admission/SOURCE-CONTRACT.md). SPX_POINTS = stops in S&P points: actual SPX only.
pub const REVIEWED: &[&str] = &["lwdtfo_vb_close", "lwdtfo_vb_hilo", "lwdtfo_vb_open"];

/// Tables 4.1-4.3 commodity rows (p.76-77), one card per market and table: (id, market name, Norgate symbol, table 0/1/2 = close/high-low/open,
/// buy %, sell %). Lane K3 (25 Nov 2026, R4 #414/#418). Pork bellies (&PB) is not held.
const CM: &[(&str, &str, &str, usize, f64, f64)] = &[
    ("lwdtfo_vb_close_cattle", "Live cattle", "&LE", 0, 0.7, 0.5),
    ("lwdtfo_vb_hilo_cattle", "Live cattle", "&LE", 1, 0.7, 0.7),
    ("lwdtfo_vb_open_cattle", "Live cattle", "&LE", 2, 1.4, 1.4),
    ("lwdtfo_vb_close_cotton", "Cotton", "&CT", 0, 0.5, 1.5),
    ("lwdtfo_vb_hilo_cotton", "Cotton", "&CT", 1, 0.9, 0.9),
    ("lwdtfo_vb_open_cotton", "Cotton", "&CT", 2, 0.6, 0.6),
    ("lwdtfo_vb_close_coffee", "Coffee", "&KC", 0, 0.7, 0.5),
    ("lwdtfo_vb_hilo_coffee", "Coffee", "&KC", 1, 0.6, 0.6),
    ("lwdtfo_vb_open_coffee", "Coffee", "&KC", 2, 1.3, 1.3),
    ("lwdtfo_vb_close_oj", "Orange juice", "&OJ", 0, 0.7, 0.5),
    ("lwdtfo_vb_hilo_oj", "Orange juice", "&OJ", 1, 1.1, 1.1),
    ("lwdtfo_vb_open_oj", "Orange juice", "&OJ", 2, 0.5, 0.5),
    ("lwdtfo_vb_close_soy", "Soybeans", "&ZS", 0, 0.7, 0.5),
    ("lwdtfo_vb_hilo_soy", "Soybeans", "&ZS", 1, 0.8, 0.8),
    ("lwdtfo_vb_open_soy", "Soybeans", "&ZS", 2, 1.0, 1.0),
    ("lwdtfo_vb_close_gbp", "British pound", "&6B", 0, 0.7, 0.5),
    ("lwdtfo_vb_hilo_gbp", "British pound", "&6B", 1, 1.2, 1.2),
    ("lwdtfo_vb_open_gbp", "British pound", "&6B", 2, 1.3, 1.3),
    ("lwdtfo_vb_close_gold", "Gold", "&GC", 0, 1.9, 0.7),
    ("lwdtfo_vb_hilo_gold", "Gold", "&GC", 1, 1.3, 1.3),
    ("lwdtfo_vb_open_gold", "Gold", "&GC", 2, 1.3, 1.3),
    ("lwdtfo_vb_close_ho", "Heating oil", "&HO", 0, 0.5, 1.3),
    ("lwdtfo_vb_hilo_ho", "Heating oil", "&HO", 1, 0.6, 0.6),
    ("lwdtfo_vb_open_ho", "Heating oil", "&HO", 2, 1.4, 1.4),
    ("lwdtfo_vb_close_bonds", "T-bonds", "&ZB", 0, 1.1, 1.1),
    ("lwdtfo_vb_hilo_bonds", "T-bonds", "&ZB", 1, 0.9, 0.9),
    ("lwdtfo_vb_open_bonds", "T-bonds", "&ZB", 2, 1.0, 1.0),
];

pub const SPX_POINTS: &[&str] = &[];

const NOTE: &str = "Book: S&P 500 futures, row of a multi-commodity table; the test window is not printed (the S&P printouts in the same chapter run 07/02/82-08/25/98). The percentages are the book's 'best-fit' values for the S&P. Always in the market: each fill of the opposite stop exits and reverses; no protective stop, no other exit. India: same rule on NIFTY 50 (instrument swap only). Bars with zero range give no orders [Not from the book: needed so buy and sell stops do not coincide].";
const CM_NOTE: &str = "Book: one commodity row of Tables 4.1-4.3 (p.76-77), the book's 'best-fit' percentages for that market; the test window is not printed. Run on the Norgate continuous contract (back-adjusted daily bars), no costs. Always in the market: each fill of the opposite stop exits and reverses; no protective stop. Bars with zero range give no orders [Not from the book: needed so buy and sell stops do not coincide]. Daily bars cannot order the high and the low of a day: see the card's gate text for the share of trades affected (#240).";
const ALWAYS: &str = "X|\"In this, and all data shown, no stop was used and we were always long/short\" (p.76): exit only by reversal on the opposite stop";

macro_rules! live {
    ($id:expr, $name:expr, $rules:expr, $claim:expr) => {
        StrategyDef { id: $id, name: $name, group: "Ch 4 · Volatility breakouts (Tables 4.1-4.3)", session: 0, dir: "Long/Short",
            status: Status::Live, needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK, rules: $rules, notes: NOTE, claim: $claim }
    };
}

pub fn catalogue() -> Vec<StrategyDef> {
    let mut v = vec![
        live!("lwdtfo_vb_close", "S&P breakout from the close · buy 100% / sell 190% (Table 4.1)",
            &["E|\"buying tomorrow at a percentage of today's range added to today's close\" (p.75): buy stop = Close + 100% x (High - Low) (S&P row: 100/190)",
              "E|\"and to subtract for a sell\" (p.76): sell stop = Close - 190% x (High - Low)", ALWAYS],
            claim("330", "40%", "$258", "", "Table 4.1 (p.76), $85,350, window not printed")),
        live!("lwdtfo_vb_hilo", "S&P breakout from the high/low · 100% (Table 4.2)",
            &["E|\"buying tomorrow at a percentage of yesterday's range added to yesterday's high\" (p.76): buy stop = High + 100% x (High - Low)",
              "E|\"or subtracting that same amount from yesterday's low for a sell signal\" (p.76): sell stop = Low - 100% x (High - Low)", ALWAYS],
            claim("569", "40%", "$141", "", "Table 4.2 (p.77), $80,787, window not printed")),
        live!("lwdtfo_vb_open", "S&P breakout from the open · 50% (Table 4.3)",
            &["E|\"adds a percentage of today's range to tomorrow's open and buys there for a long entry\" (p.77): buy stop = Open + 50% x yesterday's (High - Low)",
              "E|\"or subtracts a percentage of today's range from the opening for a sell\" (p.77): sell stop = Open - 50% x yesterday's (High - Low)", ALWAYS],
            claim("1,727", "44%", "$143", "", "Table 4.3 (p.77), $247,850, window not printed")),
        StrategyDef { id: "lwdtfo_vb_commodities", name: "Tables 4.1-4.3: pork bellies row", group: "Ch 4 · Volatility breakouts (Tables 4.1-4.3)",
            session: 0, dir: "Long/Short", status: Status::NeedsData("Pork bellies (no Norgate &PB series is held). The other nine commodity rows are the cards lwdtfo_vb_{close,hilo,open}_{cattle,cotton,coffee,oj,soy,gbp,gold,ho,bonds}."),
            needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK,
            rules: &["E|Same three rules with each commodity's own best-fit percentages (p.76-77)", ALWAYS], notes: "", claim: None },
    ];
    // ponytail: leaked once per call for the 27 formatted names/rules; catalogue() is called at start-up only.
    let leak = |s: String| -> &'static str { Box::leak(s.into_boxed_str()) };
    for &(id, mkt, sym, k, b, s) in CM {
        let (tn, tt) = [("4.1", "close"), ("4.2", "high/low"), ("4.3", "open")][k];
        let (bp, sp) = ((b * 100.0).round(), (s * 100.0).round());
        let e = match k {
            0 => [format!("E|\"buying tomorrow at a percentage of today's range added to today's close\" (p.75): buy stop = Close + {bp}% x (High - Low) ({mkt} row of Table 4.1: {bp}/{sp})"), format!("E|\"and to subtract for a sell\" (p.76): sell stop = Close - {sp}% x (High - Low)")],
            1 => [format!("E|\"buying tomorrow at a percentage of yesterday's range added to yesterday's high\" (p.76): buy stop = High + {bp}% x (High - Low) ({mkt} row of Table 4.2)"), format!("E|\"or subtracting that same amount from yesterday's low for a sell signal\" (p.76): sell stop = Low - {sp}% x (High - Low)")],
            _ => [format!("E|\"adds a percentage of today's range to tomorrow's open and buys there for a long entry\" (p.77): buy stop = Open + {bp}% x yesterday's (High - Low) ({mkt} row of Table 4.3)"), format!("E|\"or subtracts a percentage of today's range from the opening for a sell\" (p.77): sell stop = Open - {sp}% x yesterday's (High - Low)")],
        };
        let rules: &'static [&'static str] = Box::leak(vec![leak(e[0].clone()), leak(e[1].clone()), ALWAYS].into_boxed_slice());
        v.push(StrategyDef { id, name: leak(format!("{mkt} breakout from the {tt} · buy {bp}% / sell {sp}% (Table {tn})")), group: "Ch 4 · Volatility breakouts (Tables 4.1-4.3)", session: 0, dir: "Long/Short",
            status: Status::Live, needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK, rules, notes: CM_NOTE, claim: None });
        let _ = sym;
    }
    v
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    let f = |v| Some(("lwdtfo_vb", "Always-in volatility breakout (Tables 4.1-4.3)", v));
    match id {
        "lwdtfo_vb_close" => f("From the close"),
        "lwdtfo_vb_hilo" => f("From the high / low"),
        "lwdtfo_vb_open" => f("From the open"),
        "lwdtfo_vb_commodities" => f("Pork bellies (needs data)"),
        _ => CM.iter().find(|r| r.0 == id).map(|r| f(["From the close, commodities", "From the high / low, commodities", "From the open, commodities"][r.3])).flatten(),
    }
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let mut v = params_base(id)?;
    crate::intraday_ep6::add_param(id, &mut v); // EP6 (#243)
    Some(v)
}

fn params_base(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    let (b, s) = match id {
        "lwdtfo_vb_close" => (1.0, 1.9),
        "lwdtfo_vb_hilo" => (1.0, 1.0),
        "lwdtfo_vb_open" => (0.5, 0.5),
        _ => { let r = CM.iter().find(|r| r.0 == id)?; (r.4, r.5) }
    };
    Some(vec![p("buy_mult", "Buy stop: x previous day's range (book S&P value)", b, 0.1, 3.0, 0.1),
              p("sell_mult", "Sell stop: x previous day's range (book S&P value)", s, 0.1, 3.0, 0.1)])
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    let schema = params(id)?;
    if from > to || to >= x.r.len() { return None; }
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| schema.iter().find(|d| d.key == k).map_or(f64::NAN, |d| d.def));
    let (bm, sm) = (g("buy_mult"), g("sell_mult"));
    let (o, h, l, c) = (&x.o, &x.h, &x.l, &x.c);
    let Some((from, to)) = crate::intraday_ep6::window(id, x, from, to, p) else { return Some(vec![]) }; // EP6 #243 (c)
    let cfg = Cfg { stop: f64::NAN, stop_rng: f64::NAN, day0: true, target: f64::NAN, hold: None, bailout: false, bail_after: 0, stop_s: f64::NAN, reverse: true, order: crate::intraday_ep6::on(id, p) };
    Some(simulate(x, from, to, &cfg, |i| {
        let s = i - 1;
        let r = h[s] - l[s];
        if !(r > 0.0) { return (None, None); }
        let k = CM.iter().find(|r| r.0 == id).map_or(match id { "lwdtfo_vb_close" => 0, "lwdtfo_vb_hilo" => 1, _ => 2 }, |r| r.3);
        let (bb, sb) = match k {
            0 => (c[s], c[s]),
            1 => (h[s], l[s]),
            _ => (o[i], o[i]),
        };
        (Some(Ent::Stop(bb + bm * r)), Some(Ent::Stop(sb - sm * r)))
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;
    fn bar(d: &str, o: f64, h: f64, l: f64, c: f64) -> Row {
        Row { ticker: "T".into(), date: d.into(), open: o, high: h, low: l, close: c, volume: None }
    }
    #[test]
    fn open_breakout_reverses() {
        // range 2 -> day 2 buy stop 101+1=102 hit (sell stop 100 not); day 3 sell stop 100-0.5*2.5=98.75 -> reverse.
        let r = vec![bar("2020-01-01", 100., 101., 99., 100.), bar("2020-01-02", 101., 103., 100.5, 102.5),
            bar("2020-01-03", 100., 100.5, 97., 97.5)];
        let x = Ctx::new(&r, &r, &[]);
        let t = run("lwdtfo_vb_open", &x, 1, 2, &Params::new()).unwrap();
        assert_eq!(t.len(), 2);
        assert_eq!((t[0].dir, t[0].legs[0].px, t[0].exit_i, t[0].exit_px), (1.0, 102.0, 2, 98.75));
        assert!(t[1].open && t[1].dir == -1.0 && t[1].legs[0].px == 98.75);
    }
}
