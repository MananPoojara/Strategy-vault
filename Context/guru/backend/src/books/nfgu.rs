//! Nelson Freeburg, "Building a Mechanical Trading System from the Ground Up" (TAG XVIII, 1996,
//! INO workbook, 49 pp. scan). Source and checks: library/nfgu/.
//! Pathfinder, Channel 25 and CQIII run on the Norgate Swiss franc, British pound and Japanese yen continuous contracts
//! (lane F1C, R4 #293; no Deutsche mark series); the rest need rates or Value Line data the vault lacks.
//! The 2000 version of the Quadram system (no PAL filter) is Live in nfhp.
use crate::books::jkbwts::{entry, exit, sim, Ty};
use crate::connors::{sma, true_range, Ctx, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "nfgu";

const METALS: &str = "silver/gold futures, Swiss franc futures, US T-bill yield and CPI inflation; none in the vault";
const PAL: &str = "the T-bill (DTB3) and bond (DGS10) yields are held (lane U2, 19 Nov 2026) and the PAL filter alone is Live as nfgu_pal_model; this system stays held on its OWN rules: 8 unstated details, more than the 4 R4 #339/#343 allow - (1) which maturity 'T-Bond yield' means, (2) the weekly bar for the PAL filter, (3) the MA definition, (4) the standard-deviation window of the CHAD bands ('10-Day Ma With 1.5 Standard Deviation Bands'), (5) what 'crosses above' means for CHAD against a band, (6) where the $2,000 trailing stop is measured from, (7) when the stops start, (8) whether a PAL flip closes an open trade (the page prints stops as the only exits, and $1,000 / $2,000 are S&P futures dollars, not index points); not coded (p.6, Figs 64-70)";
const HEINE: &str = "Lehman bond index, T-Bond / T-Bill yields, DJ Utilities, CRB (same model as nftm_heine_bond); not in the vault";

pub(crate) const PAL_SERIES: &[&str] = &["DTB3", "DGS10"];
pub(crate) const R2_SERIES: &[&str] = &["DTB3", "CPIAUCNS_yoy_release"];
pub(crate) const R2_A1: &str = "A|[Declared reading 1, lane M3 17 Nov 2026, R4 #346] CPI inflation = the 12-month percent change of the CPI-U all-items index, not seasonally adjusted (BLS via FRED CPIAUCNS), known only from the ReleaseDate of its month (ALFRED first vintage; no release date exists before 1949, those months are not used); real rate = inflation subtracted from the T-bill yield, both in percent";
pub(crate) const R2_A2: &str = "A|[Declared reading 2] the T-bill yield = the daily 3-month Treasury bill, discount basis (FRED DTB3, matched to Treasury.gov by lane DA, no print on a bar = no signal, nothing carried over); Daily FRED values are used on their own date (same-day publication assumed; the H.15 posts the next business day)";
pub(crate) const R2_A3: &str = "A|[Declared reading 3] stops as the same workbook's Channel 25 code (Fig. 27): the $2,000 protective stop from the entry fill (armed on the entry bar), the $2,500 trailing stop from the highest (lowest) close since the entry bar once open profit has reached $2,500; a stop fills at its price or at the open when it gaps through; dollars at the contract's $ per point";
pub(crate) const R2_A4: &str = "A|[Declared reading 4] entries are tested at every close and fill at the next open; an opposite entry reverses a position (a stop-and-reverse), and a stopped-out system enters again on any later bar where the conditions hold; the 12-day average is of silver's closes; the Swiss franc's close is joined by exact date (no franc bar = no signal) and its 4-day average is taken on its own bars and carried (Norgate &6S, as the Pathfinder card's Data2)";
const R2_NOTE: &str = "Book (1996, Fig. 15-17): silver 4/75-10/96, 125 trades, 54% winners; (2000, Fig. 22-27): silver 1975-2000 205 trades 48% (gold 3 lots 119 trades 47%, not run: the printed rules name silver). Runs on the Norgate silver continuous contract with the Norgate Swiss franc contract as Data2 (R4 #346). adapted: 4 labelled readings, UNVERIFIED.";

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
        StrategyDef { needs_series: R2_SERIES, ..d("nfgu_r2_metals", "R-Squared Silver/Gold System", "Formulas p.2 · R-Squared", "Long/Short", Live,
          &["F|Real interest rate = US T-Bill yield minus CPI inflation rate (TAG XVIII formulas p.2, PDF p.4)",
            "E|Long: silver > 12-day SMA, real T-Bill rate < 1.0%, Swiss franc > 4-day SMA; short: silver < 12-day SMA, real rate >= 1.0%, SF < 4-day SMA (p.2); the 2000 handout (Fig. 24) prints the entries as 'Enter on Tomorrow's Open' and the short side with <= (same trades)",
            "X|Protective stop $2,000; trailing stop $2,500 (p.2; Fig. 24: '$2,000 Initial Stop, $2,500 Trailing Stop')",
            R2_A1, R2_A2, R2_A3, R2_A4],
          R2_NOTE) },
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
        StrategyDef { needs_series: PAL_SERIES, ..d("nfgu_pal_model", "PAL stock market timing model", "PAL Timing Model", "Long", Live,
          &["F|\"PAL Model Always Bullish Except Under Following Conditions. PAL Is Bearish If S&P 500 < 42-Week MA And Either: 1) T-Bill Yield > 42-Week MA; Or 2) T-Bond Yield > 10-Week MA\" (TAG XVIII formulas p.6, the Quadram/PAL setup, PDF p.8)",
            "E|Bullish = in the market (Figs 60-62, pp.37-38: market exposure 75.4% / 78.0% < 100%, so the model is long or flat); the weekly state is known at the week's last close and the order fills at the next bar's open",
            "X|Bearish = out of the market, at the next bar's open after the week's last close",
            "A|[Declared adaptation, R4 #340 (data substitution), and declared reading 1, lane U2 19 Nov 2026, R4 #343/#346] 'T-Bond yield' is not defined in the workbook; read as the 10-year constant-maturity Treasury yield (FRED DGS10, 1962-): the 30-year series (DGS30) has no print from 2002-02-19 to 2006-02-08, and a gap would have to be filled or skipped",
            "A|[Declared reading 2] weekly bars: a week ends on its last trading day; the S&P close is that day's close, a yield is the last published daily value of that week (FRED DTB3 3-month bill, discount basis; DGS10), nothing carried from another week and no interpolation; 'weekly bar + last print of the week' is one definition (lead ruling). Timing: Treasury.gov publishes a day's rate the same evening (matched to FRED by lane DA), the H.15 copy (FRED) the next business day, so a Friday print is public before Monday's open on the Treasury.gov source but not on the H.15 one; the engine uses the Friday value for the Monday open",
            "A|[Declared reading 3] 'MA' = simple average of the weekly values, including the current week; a week without every input (including the 42-week warm-up) gives no signal and keeps the previous state",
            "A|[Declared reading 4] the model is long or flat only (no short side is printed for the timing model); the price tested is the traded index's own close (the book tests the S&P 500 and also trades the NASDAQ Composite on the same model)"],
          "Book (Figs 60-63, pp.37-38; S&P 500 total return, 'Max Peak-to-Valley Equity Dip', no costs stated): S&P 1/8/54-10/4/96 CAR 15.6% vs 12.2% buy-and-hold, DD 18.2% vs 44.7%, exposure 75.4%, Sharpe 0.22; NASDAQ Composite 1/3/64-10/4/96 15.8% vs 11.4%, DD 24.3%, exposure 78.0%. Runs on the held index price series (price only, no dividends; the vault's SPX starts in 1983 and DGS10 in 1962, so the book window is not covered). adapted: 4 labelled readings plus one declared data substitution (10-year yield for the 30-year bond, R4 #340), UNVERIFIED.") },
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
        "nfgu_pal_model" => ("nf_pal", "Freeburg PAL timing model", "1996 · US rates"),
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
            p("tight", "Tight stop factor (1 of 3)", 0.4, 0.1, 1., 0.05), stop,
            p("onchange", "[Option, not the default reading] 1 = enter only on the day the indicator count reaches 3 (no same-direction re-entry after a stop)", 0., 0., 1., 1.),
            p("ratchet", "[Option, not the default reading] 1 = the trailing stop never loosens", 0., 0., 1., 1.)],
        "nfgu_r2_metals" => vec![p("sma", "Silver simple average (days)", 12., 3., 50., 1.), p("sfma", "Swiss franc simple average (days)", 4., 2., 30., 1.),
            p("rr", "Real T-bill rate threshold (%)", 1.0, -2., 5., 0.25), stop, p("trl_usd", "Trailing stop $ per contract", 2500., 250., 10000., 250.)],
        "nfgu_pathfinder" => vec![p("fast", "Currency short MA", 9., 2., 30., 1.), p("slow", "Currency long MA", 18., 5., 60., 1.), p("bfast", "T-bond short MA", 3., 1., 20., 1.),
            p("bslow", "T-bond long MA", 25., 5., 60., 1.), p("trig", "Entry MA (rising / falling)", 6., 2., 30., 1.), stop],
        "nfgu_pal_model" => vec![p("spma", "S&P simple average (weeks)", 42., 5., 104., 1.), p("tbma", "T-bill yield simple average (weeks)", 42., 5., 104., 1.), p("bdma", "T-bond yield simple average (weeks)", 10., 2., 52., 1.)],
        _ => return None,
    })
}

/// Lane U2 (Q26, R4 #343/#346): the PAL timing state per bar (1 = bullish/long, 0 = bearish or not yet known), set on each week's last
/// trading day and carried until the next week's; the weekly yield is the last print of its week.
fn pal_targets(x: &Ctx, spma: usize, tbma: usize, bdma: usize) -> Option<Vec<i8>> {
    let (tb, bd) = (x.extra.get("DTB3")?, x.extra.get("DGS10")?);
    let n = x.c.len();
    let wk = |i: usize| (crate::books::ftm::days(&x.r[i].date) + 3).div_euclid(7);
    let ends: Vec<usize> = (0..n.saturating_sub(1)).filter(|&i| wk(i + 1) != wk(i)).collect();
    let last = |v: &[f64], a: usize, b: usize| (a..=b).rev().map(|i| v[i]).find(|q| q.is_finite()).unwrap_or(f64::NAN);
    let (mut sp, mut t, mut b) = (vec![], vec![], vec![]);
    for (w, &li) in ends.iter().enumerate() {
        let a = if w == 0 { 0 } else { ends[w - 1] + 1 };
        sp.push(x.c[li]); t.push(last(tb, a, li)); b.push(last(bd, a, li));
    }
    let avg = |v: &[f64], w: usize, k: usize| if w + 1 < k { f64::NAN } else { v[w + 1 - k..=w].iter().sum::<f64>() / k as f64 };
    let (mut tgt, mut state, mut nxt) = (vec![0i8; n], 0i8, 0usize);
    for i in 0..n {
        if nxt < ends.len() && ends[nxt] == i {
            let (sa, ta, ba) = (avg(&sp, nxt, spma), avg(&t, nxt, tbma), avg(&b, nxt, bdma));
            if [sp[nxt], sa, t[nxt], ta, b[nxt], ba].iter().all(|v| v.is_finite()) {
                state = if sp[nxt] < sa && (t[nxt] > ta || b[nxt] > ba) { 0 } else { 1 };
            }
            nxt += 1;
        }
        tgt[i] = state;
    }
    Some(tgt)
}

/// Lane F1C (R4 #293): the three currency systems on the Norgate continuous contracts (futures::CARDS_F1B), orders walked on
/// the TradeStation bar path (books::jkbwts::sim: stop and market orders for the next bar, exits tied to the position held).
pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    let schema = params(id)?;
    if from > to || to >= x.c.len() { return None; }
    let g = |k: &str| crate::books::chbta2::getp(&schema, p, k);
    let u = |k: &str| g(k).round().max(1.) as usize;
    let (h, l, c) = (&x.h, &x.l, &x.c);
    if id == "nfgu_pal_model" {
        let u = |k: &str| g(k).round().max(1.) as usize;
        let Some(tgt) = pal_targets(x, u("spma"), u("tbma"), u("bdma")) else { return Some(vec![]) };
        return Some(crate::books::rbtie::target_trades(x, &tgt, from, to, true, "PAL bearish"));
    }
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
            // REV-F1B (3aff014d) M1: two labelled options; the defaults keep the declared readings
            let (onchange, ratchet) = (g("onchange") >= 0.5, g("ratchet") >= 0.5);
            let mut held: (usize, f64) = (usize::MAX, f64::NAN); // (entry bar, tightest trailing level so far)
            sim(x, from, to, init, false, |i, pos, _| {
                if i < lb.max(roc) || !ma[i - lb].is_finite() || !atr[i].is_finite() { return vec![]; }
                let cur = pos.map_or(0, |q| q.dir);
                let fresh = |d: f64| n(i, d) == 3 && !(onchange && i > lb.max(roc) && n(i - 1, d) == 3);
                let mut v = vec![];
                if fresh(1.) && cur != 1 { v.push(entry(1, Ty::Mkt, 0., "CQIII long")); }
                if fresh(-1.) && cur != -1 { v.push(entry(-1, Ty::Mkt, 0., "CQIII short")); }
                if let Some(q) = pos.filter(|q| i > q.i) {
                    let d = q.dir as f64;
                    let f = match n(i, d) { 3 => 1., 2 => med, _ => tight };
                    let mut lv = best(q.i + 1, i, q.dir) - d * f * mult * atr[i];
                    if ratchet { if held.0 == q.i && (held.1 - lv) * d > 0. { lv = held.1; } held = (q.i, lv); }
                    v.push(exit(q.dir, Ty::Stop, lv, "Adaptive trailing stop"));
                }
                v
            })
        }
        "nfgu_r2_metals" => {
            // lane M3 (R4 #346): DTB3 and the CPI 12-month change (released series) are extra_data; the Swiss franc is Data2
            let (Some(tb), Some(inf)) = (x.extra.get("DTB3"), x.extra.get("CPIAUCNS_yoy_release")) else { return Some(vec![]) };
            let Some(rows) = crate::futures::rows("&6S") else { return Some(vec![]) }; // readiness refuses this case (futures::data2_issue)
            let sf = crate::second::align("&6S", x.r, rows);
            let (sfc, sfa) = (sf.exact(crate::second::Field::C), sf.own(|r| sma(&r.iter().map(|b| b.close).collect::<Vec<_>>(), u("sfma"))));
            let sa = sma(c, u("sma"));
            let (trl, rr) = (crate::futures::usd_points(x, from, g("trl_usd"))?, g("rr"));
            sim(x, from, to, init, false, |i, pos, _| {
                if ![tb[i], inf[i], sfc[i], sfa[i], sa[i]].iter().all(|v| v.is_finite()) { return vec![]; }
                let real = tb[i] - inf[i];
                let long = c[i] > sa[i] && real < rr && sfc[i] > sfa[i];
                let short = c[i] < sa[i] && real >= rr && sfc[i] < sfa[i]; // the 2000 handout prints <=: the same 492 trades (r2_ref.py)
                let cur = pos.map_or(0, |q| q.dir);
                let mut v = vec![];
                if long && cur != 1 { v.push(entry(1, Ty::Mkt, 0., "R-Squared long")); }
                if short && cur != -1 { v.push(entry(-1, Ty::Mkt, 0., "R-Squared short")); }
                if let Some(q) = pos {
                    let d = q.dir as f64;
                    let k = i - q.i; // closes after the entry bar
                    if k > 0 {
                        let b = best(i + 1 - k, i, q.dir);
                        if (b - q.px) * d >= trl - 1e-9 { v.push(exit(q.dir, Ty::Stop, b - d * trl, "Trailing stop")); }
                    }
                }
                v
            })
        }
        "nfgu_pathfinder" => {
            let Some(rows) = crate::futures::rows("&ZB") else { return Some(vec![]) }; // readiness refuses this case (futures::data2_issue)
            let zb = crate::second::align("&ZB", x.r, rows);
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
    /// Lane M3: R-Squared enters long at the next open when silver > its 12-day SMA, the real T-bill rate < 1.0% and the Swiss franc
    /// > its 4-day SMA; a bar without a T-bill print gives no signal.
    #[test]
    fn m3_r2_metals_enters_at_the_next_open_and_needs_every_series() {
        let n = 40;
        let tk = |s: &str| crate::futures::ticker(crate::futures::MARKETS.iter().find(|m| m.sym == s).unwrap());
        let mk = |t: String, base: f64, st: f64| -> Vec<crate::Row> { (0..n).map(|i| { let c = base + st * i as f64;
            crate::Row { ticker: t.clone(), date: format!("1990-{:02}-{:02}", 1 + i / 28, 1 + i % 28), open: c, high: c + st, low: c - st, close: c, volume: None } }).collect() };
        let (si, sf): (Vec<crate::Row>, &'static [crate::Row]) = (mk(tk("&SI"), 5., 0.01), Box::leak(mk(tk("&6S"), 0.7, 0.001).into_boxed_slice()));
        crate::futures::TEST_ROWS.with(|t| t.borrow_mut().insert("&6S", sf));
        let mut x = Ctx::new(&si, &si, &[]);
        let mut tb = vec![3.0; n];
        x.extra.insert("CPIAUCNS_yoy_release".into(), vec![4.0; n]);
        x.extra.insert("DTB3".into(), tb.clone());
        let t = run("nfgu_r2_metals", &x, 0, n - 1, &Params::new()).unwrap();
        assert_eq!((t[0].dir, t[0].legs[0].i), (1., 12)); // first full 12-day average on bar 11, filled at the open of bar 12
        tb[11] = f64::NAN; x.extra.insert("DTB3".into(), tb);
        assert_eq!(run("nfgu_r2_metals", &x, 0, n - 1, &Params::new()).unwrap()[0].legs[0].i, 13);
        assert!(run("nfgu_r2_metals", &Ctx::new(&si, &si, &[]), 0, n - 1, &Params::new()).unwrap().is_empty());
        crate::futures::TEST_ROWS.with(|t| t.borrow_mut().clear());
    }
    /// Lane M3 reference feed: `R2_DUMP=out R2_ID=nfgu_r2_metals R2_DIR=<norgate dir> R2_DTB3=DTB3.csv R2_CPI=CPIAUCNS_yoy_release.csv cargo test --release r2_dump -- --ignored`
    #[test]
    #[ignore]
    fn r2_dump() {
        let (Ok(out), Ok(id), Ok(dir), Ok(tbf), Ok(cpf)) = (std::env::var("R2_DUMP"), std::env::var("R2_ID"), std::env::var("R2_DIR"), std::env::var("R2_DTB3"), std::env::var("R2_CPI")) else { return };
        let q = |s: &str| s.trim_matches('"').to_string();
        let load = |sym: &str| -> Vec<crate::Row> { let t = crate::futures::ticker(crate::futures::MARKETS.iter().find(|m| m.sym == sym).unwrap());
            std::fs::read_to_string(format!("{dir}/{sym}.csv")).unwrap().lines().skip(1).filter_map(|l| { let f: Vec<&str> = l.split(',').collect();
                Some(crate::Row { ticker: t.clone(), date: q(f[0]), open: q(f[3]).parse().ok()?, high: q(f[4]).parse().ok()?, low: q(f[5]).parse().ok()?, close: q(f[6]).parse().ok()?, volume: None }) }).collect() };
        let (si, sf): (Vec<crate::Row>, &'static [crate::Row]) = (load("&SI"), Box::leak(load("&6S").into_boxed_slice()));
        crate::futures::TEST_ROWS.with(|t| t.borrow_mut().insert("&6S", sf));
        let tb: std::collections::BTreeMap<String, f64> = std::fs::read_to_string(tbf).unwrap().lines().skip(1).filter_map(|l| { let f: Vec<&str> = l.split(',').collect(); Some((f[0].to_string(), f[1].parse().ok()?)) }).collect();
        let cp: std::collections::BTreeMap<String, f64> = std::fs::read_to_string(cpf).unwrap().lines().skip(1).filter_map(|l| { let f: Vec<&str> = l.split(',').collect(); Some((f[1].to_string(), f[2].parse().ok()?)) }).collect();
        crate::release::mark("CPIAUCNS_yoy_release");
        let mut x = Ctx::new(&si, &si, &[]);
        x.extra.insert("DTB3".into(), si.iter().map(|r| tb.get(&r.date).copied().unwrap_or(f64::NAN)).collect());
        x.extra.insert("CPIAUCNS_yoy_release".into(), crate::release::column("CPIAUCNS_yoy_release", si.iter().map(|r| r.date.as_str()), &cp));
        let t = run(&id, &x, 0, si.len() - 1, &Params::new()).unwrap();
        let mut o = String::new();
        for z in &t { o += &format!("{}\t{}\t{}\t{}\t{}\n", si[z.legs[0].i].date, z.legs[0].px, si[z.exit_i].date, z.exit_px, z.open); }
        std::fs::write(out, o).unwrap();
    }
    /// Lane U2 reference feed: `PAL_DUMP=out PAL_CSV=us_data.csv PAL_DTB3=DTB3.csv PAL_DGS10=DGS10.csv cargo test --release pal_dump -- --ignored`
    #[test]
    #[ignore]
    fn pal_dump() {
        let (Ok(out), Ok(csv), Ok(tbf), Ok(bdf)) = (std::env::var("PAL_DUMP"), std::env::var("PAL_CSV"), std::env::var("PAL_DTB3"), std::env::var("PAL_DGS10")) else { return };
        let iso = |d: &str| format!("{}-{}-{}", &d[6..10], &d[3..5], &d[0..2]);
        let rows: Vec<crate::Row> = std::fs::read_to_string(csv).unwrap().lines().skip(1).filter_map(|l| { let f: Vec<&str> = l.split(',').collect();
            if f[0] != "SPX" { return None; }
            Some(crate::Row { ticker: "SPX".into(), date: iso(f[1]), open: f[2].parse().ok()?, high: f[3].parse().ok()?, low: f[4].parse().ok()?, close: f[5].parse().ok()?, volume: None }) }).collect();
        let ld = |p: String| -> std::collections::BTreeMap<String, f64> { std::fs::read_to_string(p).unwrap().lines().skip(1).filter_map(|l| { let f: Vec<&str> = l.split(',').collect(); Some((f[0].to_string(), f[1].parse().ok()?)) }).collect() };
        let (tb, bd) = (ld(tbf), ld(bdf));
        let mut x = Ctx::new(&rows, &rows, &[]);
        x.extra.insert("DTB3".into(), rows.iter().map(|r| tb.get(&r.date).copied().unwrap_or(f64::NAN)).collect());
        x.extra.insert("DGS10".into(), rows.iter().map(|r| bd.get(&r.date).copied().unwrap_or(f64::NAN)).collect());
        let t = run("nfgu_pal_model", &x, 0, rows.len() - 1, &Params::new()).unwrap();
        let mut o = String::new();
        for z in &t { o += &format!("{}\t{}\t{}\t{}\t{}\n", rows[z.legs[0].i].date, z.legs[0].px, rows[z.exit_i].date, z.exit_px, z.open); }
        std::fs::write(out, o).unwrap();
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
