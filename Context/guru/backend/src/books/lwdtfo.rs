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
pub const SPX_POINTS: &[&str] = &[];

const NOTE: &str = "Book: S&P 500 futures, row of a multi-commodity table; the test window is not printed (the S&P printouts in the same chapter run 07/02/82-08/25/98). The percentages are the book's 'best-fit' values for the S&P. Always in the market: each fill of the opposite stop exits and reverses; no protective stop, no other exit. India: same rule on NIFTY 50 (instrument swap only). Bars with zero range give no orders [Not from the book: needed so buy and sell stops do not coincide].";
const ALWAYS: &str = "X|\"In this, and all data shown, no stop was used and we were always long/short\" (p.76): exit only by reversal on the opposite stop";

macro_rules! live {
    ($id:expr, $name:expr, $rules:expr, $claim:expr) => {
        StrategyDef { id: $id, name: $name, group: "Ch 4 · Volatility breakouts (Tables 4.1-4.3)", session: 0, dir: "Long/Short",
            status: Status::Live, needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK, rules: $rules, notes: NOTE, claim: $claim }
    };
}

pub fn catalogue() -> Vec<StrategyDef> {
    vec![
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
        StrategyDef { id: "lwdtfo_vb_commodities", name: "Tables 4.1-4.3: the other ten commodity rows", group: "Ch 4 · Volatility breakouts (Tables 4.1-4.3)",
            session: 0, dir: "Long/Short", status: Status::NeedsData("Needs daily futures prices for cattle, pork bellies, cotton, coffee, orange juice, soybeans, British pound, gold, heating oil and T-bonds; the vault has none."),
            needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK,
            rules: &["E|Same three rules with each commodity's own best-fit percentages (p.76-77)", ALWAYS], notes: "", claim: None },
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    let f = |v| Some(("lwdtfo_vb", "Always-in volatility breakout (Tables 4.1-4.3)", v));
    match id {
        "lwdtfo_vb_close" => f("From the close"),
        "lwdtfo_vb_hilo" => f("From the high / low"),
        "lwdtfo_vb_open" => f("From the open"),
        "lwdtfo_vb_commodities" => f("Commodities (needs data)"),
        _ => None,
    }
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    let (b, s) = match id {
        "lwdtfo_vb_close" => (1.0, 1.9),
        "lwdtfo_vb_hilo" => (1.0, 1.0),
        "lwdtfo_vb_open" => (0.5, 0.5),
        _ => return None,
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
    let cfg = Cfg { stop: f64::NAN, stop_rng: f64::NAN, day0: true, target: f64::NAN, hold: None, bailout: false, reverse: true };
    Some(simulate(x, from, to, &cfg, |i| {
        let s = i - 1;
        let r = h[s] - l[s];
        if !(r > 0.0) { return (None, None); }
        let (bb, sb) = match id {
            "lwdtfo_vb_close" => (c[s], c[s]),
            "lwdtfo_vb_hilo" => (h[s], l[s]),
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
