//! Nelson Freeburg, "Building a Mechanical Trading System from the Ground Up" (TAG XVIII, 1996,
//! INO workbook, 49 pp. scan). Source and checks: library/nfgu/.
//! Pathfinder, Channel 25 and CQIII run on the Norgate Swiss franc, British pound and Japanese yen continuous contracts
//! (lane F1C, R4 #293; no Deutsche mark series); the rest need rates or Value Line data the vault lacks.
//! The 2000 version of the Quadram system (no PAL filter) is Live in nfhp.
use crate::books::jkbwts::{entry, exit, sim, Ty};
use crate::connors::{sma, true_range, Ctx, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "nfgu";

const METALS: &str = "silver/gold futures, Swiss franc futures, US T-bill yield and CPI inflation; none in the vault";
const PAL: &str = "weekly US T-Bill yield and T-Bond yield (PAL model filter: S&P < 42-week MA and T-Bill yield > 42-week MA or T-Bond yield > 10-week MA); not in the vault";
const HEINE: &str = "Lehman bond index, T-Bond / T-Bill yields, DJ Utilities, CRB (same model as nftm_heine_bond); not in the vault";

pub fn catalogue() -> Vec<StrategyDef> {
    use Status::*;
    let d = |id, name, group, dir, status, rules, notes| StrategyDef {
        id, name, group, session: 0, dir, status, needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK, rules, notes, claim: None,
    };
    vec![
        d("nfgu_pathfinder", "Pathfinder Currency Trading System", "Formulas p.1 · Pathfinder", "Long/Short", Live,
          &["F|\"A. Setup: Target Currency 1. Long: 9-Day Ma > 18-Day Ma 2. Short: 9-Day Ma < 18-Day Ma B. Setup: US T-Bonds 1. Long: 3-Day Ma > 25-Day Ma 2. Short: 3-Day Ma < 25-Day Ma\" (TAG XVIII formulas p.1); the T-bond averages on the Norgate T-bond contract's own bars, carried to the currency's dates (TradeStation Data2, labelled reading, R4 #314 (4))",
            "E|\"C. Entry Point If Setup Ok 1. Long: If 6-Day Ma > 6-Day Ma[1] Then Buy On Next Day Open 2. Short: If 6-Day Ma < 6-Day Ma[1] Then Sell On Next Day Open\" (p.1); a short signal while long reverses",
            "X|\"A. Protective Stop 1. Long: $2,000 Below Entry Price 2. Short: $2,000 Above Entry Price\" (p.1), at the contract's $ per point",
            "X|\"B. Price Reversal Exit 1. Long: 9-Day Ma Of Currency < 18-Day Ma 2. Short: 9-Day Ma Of Currency > 18-Day Ma\" (p.1)",
            "A|Unstated in the book: (1) when the price reversal exit is executed; vault reading: the next day's open, like the entries; (2) when the protective stop starts; vault reading: in the market from the entry day (the entry fills at the open). Ma = simple average (TradeStation Average; the same formula sheet writes \"Simple MA\", p.2)"],
          "Book 1/78-10/96 (Figs 9-14, p.10-11; no costs stated): DM 181 trades 46%, SF 185 49% avg $542.97, BP 192 40% avg $444.79, JY 167 48% avg $988.02; composite 725 trades 46%, avg $576. Runs on the Norgate Swiss franc, British pound and Japanese yen continuous contracts with the Norgate T-bond contract as Data2 (lane F1C, R4 #293; no Deutsche mark series): signals and points on the back-adjusted series, % per trade on the unadjusted entry fill price, $2,000 at the contract's $ per point (futures::MARKETS). Norgate continuous futures, ends 2020-09-11; roll method not stated by the vendor. adapted: 2 unstated details (R4 #297). Krutsinger's jkbwts_pathfinder prints a different reading of the same idea with an unnamed data2 (stays held)."),
        d("nfgu_r2_metals", "R-Squared Silver/Gold System", "Formulas p.2 · R-Squared", "Long/Short", NeedsData(METALS),
          &["F|Real interest rate = US T-Bill yield minus CPI inflation rate (p.2)",
            "E|Long: silver > 12-day SMA, real T-Bill rate < 1.0%, Swiss franc > 4-day SMA; short: silver < 12-day SMA, real rate >= 1.0%, SF < 4-day SMA (p.2)",
            "X|Protective stop $2,000; trailing stop $2,500 (p.2)"],
          "Book: silver 4/75-10/96 125 trades 54% (Fig. 15-17)."),
        d("nfgu_channel25", "Channel 25 Currency System (TradeStation code)", "Formulas p.3 · Channel 25", "Long/Short", Live,
          &["E|\"Buy tomorrow at highest(H, Length) stop; Sell tomorrow at lowest(L, Length) stop;\" Length 25 (TradeStation code, Fig. 27, p.18); the 25 bars include today; an opposite stop entry reverses; an entry on the side already held is not sent",
            "X|\"If MarketPosition = 1 then ExitLong (\"Initial LStop\") tomorrow at EntryPrice - InitStop/BigPointValue stop;\" InitStop 2000; short mirror (Fig. 27): from the bar after the entry",
            "X|\"Value1 = BarsSinceEntry; If Value1 > MaxBarsBack then Value1 = MaxBarsBack; If (Highest(c, Value1) - EntryPrice) * BigPointValue >= TrlStop then ExitLong (\"Trailing LStop\") tomorrow at Highest(c, Value1) - TrlStop/BigPointValue stop;\" TrlStop 2000; short mirror with Lowest(c, Value1) (Fig. 27)",
            "A|Unstated in the book: (1) MaxBarsBack, a TradeStation setting; vault reading: 50, TradeStation's default (p.mbb); (2) Highest(c, 0) on the entry bar; vault reading: no trailing stop until one bar after the entry. Stops fill at the stop, or at the open when it gaps through; inside a bar the order nearer the open is met first (TradeStation bar path, books::jkbwts::sim)"],
          "Book 1/78-9/96 (Figs 28-33, p.19-20; no costs stated): DM 128 trades 49%, SF 152 56% avg $599.26, BP 163 53% avg $706.98, JY 133 52% avg $960.81; composite 576 trades 53%. Runs on the Norgate Swiss franc, British pound and Japanese yen continuous contracts (lane F1C, R4 #293; no Deutsche mark series): signals and points on the back-adjusted series, % per trade on the unadjusted entry fill price, $2,000 at the contract's $ per point (futures::MARKETS). Norgate continuous futures, ends 2020-09-11; roll method not stated by the vendor. adapted: 2 unstated details (R4 #297). The 2000 handout's nfhp_channel25 prints the same rules (duplicate, counted once)."),
        d("nfgu_cqiii", "CQIII Currency System with adaptive trailing stop", "Formulas pp.4-5 · CQIII", "Long/Short", Live,
          &["E|\"A. Enter Long Next Day Open If: Close > 10-Day MA, 10-Day Ma Today > 10-Day MA Of Ten Days Ago, Close > Close Of 40 Days Ago. B. Enter Short Next Day Open If: Close < 10-Day MA, 10-Day Ma Today < 10-Day MA Of Ten Days Ago, Close < Close 40 Days Ago\" (TAG XVIII formulas p.4)",
            "X|\"A. Reversing Signal: Reverse From Long To Short If The Three Indicators Turn Negative\"; \"B. Protective Stop Long: $2,000 Below Entry Price\" (p.4), at the contract's $ per point",
            "X|\"C. Adaptive Trailing Stop ... If All 3 Indicators Support The Trade, Stop Is Very Wide ... B) Default Stop 1) Take 49-Day Average True Range i) Add Up Each Day's True Range For Past 49 Days And Divide By 49 2) Multiply 49-Day Average True Range By 10 For Default Stop ... C) Medium Stop 1) Multiply Default Stop By .60 ... Use When Two Of Three ... D) Tight Stop 1) Multiply Default Stop By .40 ... When Just One Price Trend Indicator Supports The Trade\" (p.4-5)",
            "A|Unstated in the book: (1) where the trailing stop is measured from; vault reading: the highest close (short: lowest close) since entry, as the author's own Channel 25 TradeStation code (Fig. 27: Highest(c, BarsSinceEntry), from the bar after the entry), recomputed each day with that day's width (no indicator supporting: the tight width); (2) when the protective stop starts; vault reading: in the market from the entry day (the entry fills at the open). MA = simple average (the sheet's p.2 \"Simple MA\")"],
          "Book 1/78-9/96 (Figs 46-51, p.29-30; no costs stated): DM 82 trades 46%, SF 74 47% avg $1,751.35, BP 93 32% avg $1,209.41, JY 75 54% avg $2,285.83; composite 323 trades 44%. Runs on the Norgate Swiss franc, British pound and Japanese yen continuous contracts (lane F1C, R4 #293; no Deutsche mark series): signals and points on the back-adjusted series, % per trade on the unadjusted entry fill price, $2,000 at the contract's $ per point (futures::MARKETS). Norgate continuous futures, ends 2020-09-11; roll method not stated by the vendor. adapted: 2 unstated details (R4 #297). The 2000 handout's nfhp_cqiii (Fig. 41) is the same system with an abbreviated rule list (duplicate, counted once)."),
        d("nfgu_quadram_pal", "Quadram/PAL Stock Index System", "Formulas p.6 · Quadram/PAL", "Long/Short", NeedsData(PAL),
          &["F|PAL bullish except when S&P 500 < 42-week MA and either T-Bill yield > 42-week MA or T-Bond yield > 10-week MA; bullish PAL -> longs only, bearish -> shorts only (p.6)",
            "F|CHAD = 5-day total NYSE advances minus 5-day total declines; countertrend bands = 10-day MA of CHAD +/- 1.5 SD; trend-following bands = 25-day MA +/- 1.0 SD (p.6)",
            "E|Next day open. Countertrend: long PAL bullish and CHAD crosses above lower band; short PAL bearish and CHAD crosses below upper band. Trend-following: long crosses above upper band; short crosses below lower band (p.6)",
            "X|Protective stop $1,000; trailing stop $2,000 (p.6)"],
          "Book 5/82-10/96 (Figs 68-70): 789 trades 43%, net $214,200, avg $266.25; long 682 41%, short 107 53%. The PAL filter needs T-Bill / T-Bond yields. The 2000 version without PAL is Live as nfhp_quadram_*."),
        d("nfgu_pal_model", "PAL stock market timing model", "PAL Timing Model", "Long", NeedsData(PAL),
          &["E|PAL bullish (S&P not below its 42-week MA with rising rates) = in stocks; bearish = out (p.6, Figs 59-62)"],
          "Book: S&P 1/8/54-10/4/96 CAR 15.6% vs 12.2%, DD 18.2%, exposure 75.4%; NASDAQ 1/3/64-10/4/96 15.8% vs 11.4% (pp.37-38). The workbook never states the PAL exit as a separate rule - read from the filter."),
        d("nfgu_heine", "Heine bond model (cash & futures)", "Heine Bond Model", "Long", NeedsData(HEINE),
          &["E|Same five-indicator point model as nftm_heine_bond (buy >= 3 points, sell <= 2)"],
          "Book: Heine cash 3/1/57-10/3/96 CAR 12.5% vs DJ20 bonds 8.0%, DD 3.2%, exposure 50% (p.43)."),
        d("nfgu_vl_swing", "Value Line Percent Swing System", "Value Line Percent Swing", "Long", NotTestable("Only the table labels 'Buy on N-Percent Rally / Sell on N-Percent Pullback' (Figs 2-3): no reference point, frequency or price basis; needs the Value Line Geometric index."),
          &["E|Buy on an N-percent rally, sell on an N-percent pullback (Fig. 2)"], "Book: 2% swing model 8/9/63-9/30/96 CAR 18.5% vs NASDAQ B&H 11.5%, DD 19.6%, exposure 63% (Fig. 4)."),
        d("nfgu_results_only", "Momentum, Kestner Hi/Lo MA, dual WMA T-Bond and range-breakout bond systems", "Bond systems filtered by Heine", "Long/Short", NotTestable("Result tables only (pp.44-46); rules not printed; also need T-Bond futures."),
          &["E|No rules printed"], "Dual WMA T-Bond 9/77-10/96: 122 trades 41%."),
    ]
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    Some(match id {
        "nfgu_pathfinder" => ("nf_pathfinder", "Freeburg Pathfinder currency", "1996 · currency futures"),
        "nfgu_r2_metals" => ("nf_r2", "Freeburg R-Squared metals", "1996 · Data missing"),
        "nfgu_channel25" => ("nf_channel25", "Freeburg Channel 25", "1996 code · currency futures"),
        "nfgu_cqiii" => ("nf_cqiii", "Freeburg CQIII", "1996 adaptive stop · currency futures"),
        "nfgu_quadram_pal" => ("nf_quadram", "Freeburg Quadram S&P system", "PAL-filtered (1996) · Data missing"),
        "nfgu_pal_model" => ("nf_pal", "Freeburg PAL timing model", "Data missing"),
        "nfgu_heine" => ("nf_heine", "Heine bond timing model", "1996 · Data missing"),
        "nfgu_vl_swing" => ("nf_vlswing", "Value Line percent swing", "1996 · not testable"),
        "nfgu_results_only" => ("nf_results", "Freeburg results-only examples", "1996"),
        _ => return None,
    })
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    let stop = p("init_usd", "Protective stop $ per contract", 2000., 250., 10000., 250.);
    Some(match id {
        "nfgu_channel25" => vec![p("len", "Channel length (Highest(H) / Lowest(L))", 25., 5., 100., 1.), stop,
            p("trl_usd", "Trailing stop $ per contract (TrlStop)", 2000., 250., 10000., 250.), p("mbb", "MaxBarsBack (caps the trailing window; TradeStation default 50)", 50., 10., 500., 1.)],
        "nfgu_cqiii" => vec![p("ma", "Moving average days", 10., 3., 50., 1.), p("lb", "MA compared with N days ago", 10., 1., 50., 1.), p("roc", "Close compared with N days ago", 40., 5., 100., 1.),
            p("atr", "Average true range days", 49., 5., 100., 1.), p("mult", "Default stop (x ATR, all 3 support)", 10., 1., 20., 0.5), p("med", "Medium stop factor (2 of 3)", 0.6, 0.1, 1., 0.05),
            p("tight", "Tight stop factor (1 of 3)", 0.4, 0.1, 1., 0.05), stop],
        "nfgu_pathfinder" => vec![p("fast", "Currency short MA", 9., 2., 30., 1.), p("slow", "Currency long MA", 18., 5., 60., 1.), p("bfast", "T-bond short MA", 3., 1., 20., 1.),
            p("bslow", "T-bond long MA", 25., 5., 60., 1.), p("trig", "Entry MA (rising / falling)", 6., 2., 30., 1.), stop],
        _ => return None,
    })
}

/// Lane F1C (R4 #293): the three currency systems on the Norgate continuous contracts (futures::CARDS_F1B), orders walked on
/// the TradeStation bar path (books::jkbwts::sim: stop and market orders for the next bar, exits tied to the position held).
pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    let schema = params(id)?;
    if from > to || to >= x.c.len() { return None; }
    let g = |k: &str| crate::books::chbta2::getp(&schema, p, k);
    let u = |k: &str| g(k).round().max(1.) as usize;
    let (h, l, c) = (&x.h, &x.l, &x.c);
    // off the Norgate contracts the card does not run (readiness refuses those tickers with futures::home)
    let Some(init) = crate::futures::usd_points(x, from, g("init_usd")) else { return Some(vec![]) };
    let best = |a: usize, b: usize, d: i8| { let w = &c[a..=b]; if d > 0 { w.iter().cloned().fold(f64::MIN, f64::max) } else { w.iter().cloned().fold(f64::MAX, f64::min) } };
    Some(match id {
        "nfgu_channel25" => {
            let (len, mbb) = (u("len"), u("mbb"));
            let trl = crate::futures::usd_points(x, from, g("trl_usd"))?;
            sim(x, from, to, 0., false, |i, pos, _| {
                if i + 1 < len { return vec![]; }
                let mut v = vec![entry(1, Ty::Stop, h[i + 1 - len..=i].iter().cloned().fold(f64::MIN, f64::max), "Channel 25 buy stop"),
                                 entry(-1, Ty::Stop, l[i + 1 - len..=i].iter().cloned().fold(f64::MAX, f64::min), "Channel 25 sell stop")];
                if let Some(q) = pos {
                    let d = q.dir as f64;
                    v.push(exit(q.dir, Ty::Stop, q.px - d * init, "Initial stop"));
                    let k = (i - q.i).min(mbb); // Value1 = BarsSinceEntry, capped at MaxBarsBack
                    if k > 0 {
                        let b = best(i + 1 - k, i, q.dir);
                        if (b - q.px) * d >= trl - 1e-9 { v.push(exit(q.dir, Ty::Stop, b - d * trl, "Trailing stop")); }
                    }
                }
                v
            })
        }
        "nfgu_cqiii" => {
            let (lb, roc) = (u("lb"), u("roc"));
            let (ma, atr) = (sma(c, u("ma")), sma(&true_range(x.r), u("atr")));
            let (mult, med, tight) = (g("mult"), g("med"), g("tight"));
            // how many of the three trend indicators support direction d (strict, as printed)
            let n = |i: usize, d: f64| [(c[i] - ma[i]) * d > 0., (ma[i] - ma[i - lb]) * d > 0., (c[i] - c[i - roc]) * d > 0.].iter().filter(|b| **b).count();
            sim(x, from, to, init, false, |i, pos, _| {
                if i < lb.max(roc) || !ma[i - lb].is_finite() || !atr[i].is_finite() { return vec![]; }
                let cur = pos.map_or(0, |q| q.dir);
                let mut v = vec![];
                if n(i, 1.) == 3 && cur != 1 { v.push(entry(1, Ty::Mkt, 0., "CQIII long")); }
                if n(i, -1.) == 3 && cur != -1 { v.push(entry(-1, Ty::Mkt, 0., "CQIII short")); }
                if let Some(q) = pos.filter(|q| i > q.i) {
                    let d = q.dir as f64;
                    let f = match n(i, d) { 3 => 1., 2 => med, _ => tight };
                    v.push(exit(q.dir, Ty::Stop, best(q.i + 1, i, q.dir) - d * f * mult * atr[i], "Adaptive trailing stop"));
                }
                v
            })
        }
        "nfgu_pathfinder" => {
            let m = crate::futures::MARKETS.iter().find(|m| m.sym == "&ZB")?;
            let Some(rows) = crate::books::rotation::PRELOADED.get().and_then(|d| d.get(&crate::futures::ticker(m))) else { return Some(vec![]) };
            let zb = crate::second::align(m.sym, x.r, rows);
            let bma = |k: usize| zb.own(|r| sma(&r.iter().map(|b| b.close).collect::<Vec<_>>(), k));
            let (bf, bs) = (bma(u("bfast")), bma(u("bslow")));
            let (f, s, t) = (sma(c, u("fast")), sma(c, u("slow")), sma(c, u("trig")));
            let side = |a: f64, b: f64| if a > b { 1 } else if a < b { -1 } else { 0 };
            sim(x, from, to, init, false, |i, pos, _| {
                if i == 0 { return vec![]; }
                let (cs, bd, tr) = (side(f[i], s[i]), side(bf[i], bs[i]), side(t[i], t[i - 1]));
                let cur = pos.map_or(0, |q| q.dir);
                let mut v = vec![];
                if cur != 0 && cs == -cur { v.push(exit(cur, Ty::Mkt, 0., "Price reversal exit")); }
                if cs != 0 && cs == bd && cs == tr && cs != cur { v.push(entry(cs, Ty::Mkt, 0., if cs > 0 { "Pathfinder long" } else { "Pathfinder short" })); }
                v
            })
        }
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;
    fn ctx_rows(v: &[(f64, f64, f64, f64)]) -> Vec<Row> {
        v.iter().enumerate().map(|(i, &(o, h, l, c))| Row { ticker: "Swiss Franc (Norgate &6S)".into(), date: format!("1990-{:02}-{:02}", 1 + i / 28, 1 + i % 28), open: o, high: h, low: l, close: c, volume: None }).collect()
    }
    #[test]
    fn f1c_channel25_follows_the_printed_code() {
        // bar 0 wide (1.002 / 0.994), 25 flat bars (25-day high 1.001, low 0.995); bar 26 trades through the buy stop; bar 27
        // falls through the 25-day low (0.995) before the initial stop (1.001 - $2,000 / $125,000 = 0.985): the sell entry reverses
        let mut v = vec![(1.0, 1.002, 0.994, 1.0)];
        v.extend(vec![(1.0, 1.001, 0.995, 1.0); 25]);
        v.push((1.0, 1.003, 0.998, 1.002));
        v.push((1.0, 1.001, 0.97, 0.975));
        let r = ctx_rows(&v);
        let x = Ctx::new(&r, &r, &[]);
        let t = run("nfgu_channel25", &x, 0, r.len() - 1, &Params::new()).unwrap();
        assert_eq!((t[0].dir, t[0].legs[0].i, t[0].exit_i), (1., 26, 27));
        assert!((t[0].legs[0].px - 1.001).abs() < 1e-12 && (t[0].exit_px - 0.995).abs() < 1e-12, "{:?}", t[0]);
        assert!(t.len() == 2 && t[1].dir < 0. && t[1].open && (t[1].legs[0].px - 0.995).abs() < 1e-12);
        // the published markets are the book's currencies Norgate carries; elsewhere the module does not run (no contract)
        assert!(crate::futures::home("nfgu_cqiii", "Japanese Yen (Norgate &6J)").is_none() && crate::futures::home("nfgu_pathfinder", "SPX").is_some());
        assert!(run("nfgu_cqiii", &x, 0, r.len() - 1, &Params::new()).is_some_and(|t| t.is_empty()));
        let plain: Vec<Row> = r.iter().map(|b| Row { ticker: "SPX".into(), ..b.clone() }).collect();
        assert!(run("nfgu_channel25", &Ctx::new(&plain, &plain, &[]), 0, 27, &Params::new()).is_some_and(|t| t.is_empty()));
    }
}
