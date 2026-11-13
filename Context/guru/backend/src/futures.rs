//! Norgate continuous futures (R4 #293, lane F1, 13 Nov 2026).
//!
//! Data: the user's share DATABASE/GLOBAL DATABASE/norgate data 8SEP2020/Continuous Futures, mirrored read-only
//! (SOURCES.csv norgate-continuous-futures-20200908, 222/222 sha256); the engine reads the copy `futures/norgate-20200908`
//! next to its price file (us_data.csv), one `&SYM.csv` (unadjusted, spliced at the roll) and one `&SYM_CCB.csv`
//! (back-adjusted) per market. Only the markets in `MARKETS` are loaded, and only by the US engine.
//!
//! #293 (2): a futures context's bars are the BACK-ADJUSTED series (signals, indicators and point P&L: point differences
//! survive Norgate's additive roll adjustment); the unadjusted series is attached as RAW.Open/High/Low/Close for rules that
//! compare absolute price levels. Days without an unadjusted bar (6J _CCB 1976-08..1979-04) are not loaded.
//! Returns: each trade is priced in the contract held at entry (back-adjusted price + that day's roll offset, i.e. the
//! unadjusted price), so its % = point P&L / unadjusted entry price (= the raw close on the entry date for close entries).
//! Labelled on every card: "Norgate continuous futures, ends 2020-09-11; roll method not stated by the vendor" (#293 (5)).
use crate::{connors::{Ctx, Trade}, Row};
use std::collections::BTreeMap;

/// One continuous-futures market. `usd` = (first date in force, US$ per 1.00 of the quoted price), oldest first.
/// `tick` = the minimum price move in the quote (TradeStation "points" in entry/stop offsets are ticks, #57).
pub struct Market { pub sym: &'static str, pub name: &'static str, pub tick: f64, pub usd: &'static [(&'static str, f64)], pub source: &'static str }

/// Contract multipliers (#293 (3)): exchange contract specifications (reference data, not market data). Norgate quotes
/// currencies in US$ per unit (yen per 100 yen), metals/energy in the exchange's quote unit, T-bonds in points of par.
pub const MARKETS: &[Market] = &[
    Market { sym: "&6J", name: "Japanese Yen", tick: 0.0001, usd: &[("1972-05-16", 125_000.)],
        source: "CME Japanese yen futures: 12,500,000 yen per contract; Norgate quotes US$ per 100 yen, so 1.00 = US$125,000 (0.0001 = US$12.50, the tick of the book era)" },
    Market { sym: "&6A", name: "Australian Dollar", tick: 0.0001, usd: &[("1987-01-13", 100_000.)],
        source: "CME Australian dollar futures: 100,000 Australian dollars per contract (1.00 = US$100,000; 0.0001 = US$10)" },
    Market { sym: "&6B", name: "British Pound", tick: 0.0002, usd: &[("1975-01-02", 62_500.)],
        source: "CME British pound futures: 62,500 pounds per contract (1.00 = US$62,500; 0.0002 = US$12.50, the tick of the book era); no earlier contract size was found, so 62,500 applies to the whole series (labelled)" },
    Market { sym: "&GC", name: "Gold", tick: 0.1, usd: &[("1974-12-31", 100.)],
        source: "COMEX gold futures: 100 troy ounces per contract, quoted in US$ per ounce (US$1.00 = US$100; 0.10 = US$10)" },
    Market { sym: "&SP", name: "S&P 500", tick: 0.05, usd: &[("1982-04-21", 500.), ("1997-11-03", 250.)],
        source: "CME S&P 500 futures: US$500 x the index to 1997-11-02, US$250 x the index from 1997-11-03 (R4 #293 (3)); tick 0.05 of the book era" },
    Market { sym: "&6S", name: "Swiss Franc", tick: 0.0001, usd: &[("1972-05-16", 125_000.)],
        source: "CME Swiss franc futures: 125,000 francs per contract (1.00 = US$125,000; 0.0001 = US$12.50)" },
    Market { sym: "&CL", name: "Crude Oil", tick: 0.01, usd: &[("1983-03-30", 1_000.)],
        source: "NYMEX light sweet crude oil futures: 1,000 barrels per contract (US$1.00 = US$1,000; 0.01 = US$10)" },
    Market { sym: "&ZB", name: "US T-Bond", tick: 1. / 32., usd: &[("1977-08-22", 1_000.)],
        source: "CBOT U.S. Treasury bond futures: US$100,000 face value, quoted in points of par (1 point = US$1,000; 1/32 = US$31.25)" },
    // lane F3 (R4 #294 (1)): Harris's COMEX silver patterns
    Market { sym: "&SI", name: "Silver", tick: 0.5, usd: &[("1978-04-27", 50.)],
        source: "COMEX silver futures: 5,000 troy ounces per contract; Norgate quotes US cents per ounce, so 1.00 (one cent) = US$50; tick 0.5 cent = US$25 per the current CME contract specification (not used by the Harris patterns)" },
    // lane F1B (R4 #293 (3)): the rest of Chande's Table 5.1 / 5.15 markets (Beyond Technical Analysis ch.5)
    Market { sym: "&KC", name: "Coffee", tick: 0.05, usd: &[("1978-12-14", 375.)],
        source: "ICE (CSCE) Coffee \"C\" futures: 37,500 lb per contract, quoted in US cents per lb (1.00 cent = US$375; tick 0.05 cent = US$18.75)" },
    Market { sym: "&CT", name: "Cotton", tick: 0.01, usd: &[("1978-09-19", 500.)],
        source: "ICE (NYCE) Cotton No. 2 futures: 50,000 lb per contract, quoted in US cents per lb (1.00 cent = US$500; tick 0.01 cent = US$5)" },
    Market { sym: "&GE", name: "Eurodollar", tick: 0.01, usd: &[("1982-02-01", 2_500.)],
        source: "CME three-month Eurodollar futures: US$1,000,000 face value, quoted as an IMM index (100 - rate); 1.00 = US$2,500 (0.01 = one basis point = US$25)" },
    Market { sym: "&ZW", name: "Wheat", tick: 0.25, usd: &[("1978-01-05", 50.)],
        source: "CBOT soft red winter wheat futures: 5,000 bushels per contract, quoted in US cents per bushel (1.00 cent = US$50; tick 1/4 cent = US$12.50)" },
    Market { sym: "&HO", name: "Heating Oil", tick: 0.0001, usd: &[("1978-11-15", 42_000.)],
        source: "NYMEX No. 2 heating oil (now NY Harbor ULSD) futures: 42,000 US gallons per contract, quoted in US$ per gallon (1.00 = US$42,000; tick 0.0001 = US$4.20)" },
    Market { sym: "&HG", name: "Copper", tick: 0.05, usd: &[("1978-04-27", 250.)],
        source: "COMEX high grade copper futures: 25,000 lb per contract, quoted in US cents per lb (1.00 cent = US$250; tick 0.05 cent = US$12.50)" },
    Market { sym: "&ZS", name: "Soybeans", tick: 0.25, usd: &[("1977-11-21", 50.)],
        source: "CBOT soybean futures: 5,000 bushels per contract, quoted in US cents per bushel (1.00 cent = US$50; tick 1/4 cent = US$12.50)" },
];

pub fn ticker(m: &Market) -> String { format!("{} (Norgate {})", m.name, m.sym) }
/// The market of a futures context ticker ("US T-Bond (Norgate &ZB)").
pub fn of_ticker(t: &str) -> Option<&'static Market> {
    let sym = t.strip_suffix(')')?.rsplit_once(" (Norgate ")?.1;
    MARKETS.iter().find(|m| m.sym == sym)
}
pub fn is_futures(t: &str) -> bool { of_ticker(t).is_some() }
pub fn ctx_market(x: &Ctx) -> Option<&'static Market> { x.r.first().and_then(|r| of_ticker(&r.ticker)) }
/// US$ per 1.00 of the quote on `date` (the multiplier in force on the trade date, #293 (3)).
pub fn usd_per_point(m: &Market, date: &str) -> Option<f64> { m.usd.iter().rev().find(|(d, _)| *d <= date).map(|(_, v)| *v) }
/// A fixed US$ amount per contract in quote points on bar `i` of a futures context (None elsewhere).
pub fn usd_points(x: &Ctx, i: usize, usd: f64) -> Option<f64> { usd_per_point(ctx_market(x)?, &x.r[i].date).map(|v| usd / v) }

/// Loads the back-adjusted bars of every market in MARKETS from `dir` into `data` (key = ticker()) and returns the
/// unadjusted series as extra_data entries (ticker, RAW.Open/High/Low/Close). A missing folder loads nothing.
pub fn load(dir: &std::path::Path, data: &mut BTreeMap<String, Vec<Row>>) -> BTreeMap<(String, String), BTreeMap<String, f64>> {
    let mut extra = BTreeMap::new();
    for m in MARKETS {
        let (Some(adj), Some(raw)) = (read(&dir.join(format!("{}_CCB.csv", m.sym))), read(&dir.join(format!("{}.csv", m.sym)))) else {
            eprintln!("warning: futures {} not loaded from {}", m.sym, dir.display()); continue };
        let t = ticker(m);
        let raw: BTreeMap<String, Row> = raw.into_iter().map(|r| (r.date.clone(), r)).collect();
        let mut rows: Vec<Row> = adj.into_iter().filter(|r| raw.contains_key(&r.date)).map(|r| Row { ticker: t.clone(), ..r }).collect();
        // lane F1: a % return needs a positive unadjusted price: the series ends the day before the first non-positive one
        // (NYMEX crude settled at -37.63 on 2020-04-20); nothing is filled or estimated
        if let Some(k) = rows.iter().position(|r| { let x = &raw[&r.date]; [x.open, x.high, x.low, x.close].iter().any(|v| *v <= 0.) }) {
            println!("futures {t}: series ends {} (unadjusted price <= 0 on {})", rows.get(k.wrapping_sub(1)).map_or("-", |r| &r.date), rows[k].date);
            rows.truncate(k);
        }
        // lead review of F3: a file that parsed to no common bars is not served (an empty context would break /api/tickers)
        if rows.is_empty() { eprintln!("warning: futures {} not loaded: 0 bars in both {}_CCB.csv and {}.csv", m.sym, m.sym, m.sym); continue; }
        for (k, f) in [("Open", 0), ("High", 1), ("Low", 2), ("Close", 3)] {
            extra.insert((t.clone(), format!("RAW.{k}")), rows.iter().map(|r| { let x = &raw[&r.date]; (r.date.clone(), [x.open, x.high, x.low, x.close][f]) }).collect());
        }
        println!("futures {t}: {} bars {} -> {}", rows.len(), rows.first().map_or("", |r| &r.date), rows.last().map_or("", |r| &r.date));
        data.insert(t, rows);
    }
    extra
}
fn read(path: &std::path::Path) -> Option<Vec<Row>> {
    let text = std::fs::read_to_string(path).ok()?;
    let mut lines = text.lines();
    let head = crate::parse_csv_fields(lines.next()?);
    let col = |n: &str| head.iter().position(|h| h == n);
    let (di, oi, hi, li, ci, vi) = (col("Date")?, col("Open")?, col("High")?, col("Low")?, col("Close")?, col("Volume"));
    let mut out: Vec<Row> = lines.filter(|l| !l.trim().is_empty()).filter_map(|l| {
        let f = crate::parse_csv_fields(l);
        let num = |i: usize| f.get(i).and_then(|v| crate::parse_csv_number(v).ok()).filter(|v| v.is_finite());
        Some(Row { ticker: String::new(), date: crate::normalize_date(f.get(di)?)?, open: num(oi)?, high: num(hi)?, low: num(li)?, close: num(ci)?, volume: vi.and_then(num).filter(|v| *v > 0.) })
    }).collect();
    out.sort_by(|a, b| a.date.cmp(&b.date));
    out.dedup_by(|a, b| a.date == b.date);
    Some(out)
}

/// Lane F3 (R4 #294 (2)): Norgate US indices and Nasdaq breadth, copied read-only to futures/norgate-us-indices-20200908 next to
/// us_data.csv (SOURCES.csv norgate-us-indices-20200908-f3). (file, series): each file's close becomes a series every US context
/// reads by date (NaN where the file has no bar). Other lanes add their US Indices files here.
pub const INDEX_SERIES: &[(&str, &str)] = &[("$COMP", "NASDAQ_COMP"), ("$NYA", "NYSE_COMP"), ("#NASHI", "NASDAQ_NEW_HIGHS"), ("#NASLO", "NASDAQ_NEW_LOWS")];
pub fn load_indices(dir: &std::path::Path) -> BTreeMap<(String, String), BTreeMap<String, f64>> {
    let mut out = BTreeMap::new();
    for (file, name) in INDEX_SERIES {
        let Ok(text) = std::fs::read_to_string(dir.join(format!("{file}.csv"))) else { eprintln!("warning: index {file} not loaded from {}", dir.display()); continue };
        let mut lines = text.lines();
        let head = crate::parse_csv_fields(lines.next().unwrap_or(""));
        let (Some(di), Some(ci)) = (head.iter().position(|h| h == "Date"), head.iter().position(|h| h == "Close")) else { continue };
        let m: BTreeMap<String, f64> = lines.filter_map(|l| {
            let f = crate::parse_csv_fields(l);
            let d = f.get(di)?; // Norgate compact yyyymmdd
            let date = (d.len() == 8 && d.bytes().all(|b| b.is_ascii_digit())).then(|| format!("{}-{}-{}", &d[..4], &d[4..6], &d[6..]))?;
            Some((date, crate::parse_csv_number(f.get(ci)?).ok().filter(|v| v.is_finite())?))
        }).collect();
        println!("index {file} -> {name}: {} days", m.len());
        out.insert(("*".to_string(), name.to_string()), m);
    }
    out
}

/// The unadjusted close of a futures context (RAW.Close), aligned to its bars.
pub fn raw_close<'a>(x: &'a Ctx) -> Option<&'a [f64]> { ctx_market(x)?; x.series("RAW.Close") }

/// #293 (2) return basis. Each trade is shifted by the roll offset of its entry bar (unadjusted - back-adjusted close),
/// so its prices are those of the contract held at entry and point P&L is unchanged. Returns the shifted trades and the
/// close series the equity curve marks open trades with (back-adjusted close + the trade's offset, per trade window).
pub fn raw_basis(x: &Ctx, trades: Vec<Trade>) -> Result<(Vec<Trade>, Vec<f64>), String> {
    let raw = raw_close(x).ok_or("needs_data: the unadjusted futures series (RAW.Close) is not loaded")?;
    let mut close = x.c.clone();
    let out = trades.into_iter().map(|mut t| {
        let e = t.legs[0].i;
        let off = raw[e] - x.c[e]; // R4 #301: the % base is the unadjusted entry fill (the entry day's roll offset applied)
        if !off.is_finite() { return Err(format!("needs_data: no unadjusted futures close on {}", x.r[e].date)); }
        for l in &mut t.legs { l.px += off; }
        t.exit_px += off;
        for p in &mut t.exits { p.1 += off; }
        for k in e..=t.exit_i { close[k] = x.c[k] + off; }
        if t.legs.iter().any(|l| l.px <= 0.) { return Err(format!("calculation_unavailable: the unadjusted entry price on {} is not positive (the % return has no base); no result is published", x.r[e].date)); }
        Ok(t)
    }).collect::<Result<Vec<_>, String>>()?;
    Ok((out, close))
}
/// Lane F3 (lead review, shared-loader blocker): the roll offset a futures trade was shifted by (unadjusted - back-adjusted
/// close of its entry bar; raw_basis), so code that reads the context's back-adjusted bars against a shifted trade can put
/// them on the trade's prices (MAE/MFE path, entry-type checks). 0 elsewhere.
pub fn trade_offset(x: &Ctx, t: &Trade) -> f64 {
    let e = t.legs[0].i;
    raw_close(x).and_then(|r| r.get(e)).map_or(0., |r| r - x.c[e])
}
/// The close series an equity curve marks shifted futures trades with (back-adjusted close + each trade's offset over its
/// window), None on a non-futures context. Gross equity / gross statistics use it like normalized_run does.
pub fn marks(x: &Ctx, trades: &[Trade]) -> Option<Vec<f64>> {
    raw_close(x)?;
    let mut close = x.c.clone();
    for t in trades { let off = trade_offset(x, t); for k in t.legs[0].i..=t.exit_i.min(close.len() - 1) { close[k] = x.c[k] + off; } }
    Some(close)
}
/// Buy and hold on a futures context: point change of the back-adjusted close over the unadjusted close at the start.
pub fn buy_hold(x: &Ctx, from: usize, to: usize) -> Option<Vec<f64>> {
    let raw = raw_close(x)?;
    Some((from..=to).map(|k| 1. + (x.c[k] - x.c[from]) / raw[from]).collect())
}
/// Readiness: OHLC must be finite and ordered on the back-adjusted bars (which may be negative far back) and finite on
/// the unadjusted ones. false = valid.
pub fn invalid_bar(x: &Ctx, i: usize) -> bool {
    let r = &x.r[i];
    let ok = [r.open, r.high, r.low, r.close].iter().all(|v| v.is_finite()) && r.low <= r.high && r.open >= r.low && r.open <= r.high && r.close >= r.low && r.close <= r.high;
    // an unadjusted price may be negative (NYMEX crude settled at -37.63 on 2020-04-20); a trade entered on such a day has no
    // % base and is refused by raw_basis
    let raw_ok = ["RAW.Open", "RAW.High", "RAW.Low", "RAW.Close"].iter().all(|k| x.series(k).and_then(|s| s.get(i)).is_some_and(|v| v.is_finite()));
    !(ok && raw_ok)
}

/// Cards that run on one named continuous contract (#293 (1)): (strategy id, market symbol, instrument_policy basis).
pub const CARDS: &[(&str, &str, &str)] = &[
    ("ataa_moon", "&6A", "ATAA Journal Dec 1990 p.11-12 (Krastins, Sheer Lunacy): the moon-phase stop and reverse tested on AUD/USD IMM futures, run on the Norgate Australian dollar continuous contract (R4 #293 (1)); close fills, no amounts."),
    ("jkbwts_neeley31", "&6J", "Krutsinger, Building Winning Trading Systems (seminar manual, Dec 1997) p.22-23 Neeley31, Japanese Yen CME weekly: the printed Quick System sheet on weekly bars built from the Norgate continuous contract's daily bars (R4 #293 (1)); $ amounts at the contract's $ per point (futures::MARKETS)."),
    ("jkbwts_wagner31", "&GC", "Krutsinger, Building Winning Trading Systems (seminar manual, Dec 1997) p.30-31 Wagner31, COMEX gold weekly: the printed Quick System sheet on weekly bars built from the Norgate continuous contract's daily bars (R4 #293 (1)); $ amounts at the contract's $ per point (futures::MARKETS)."),
    ("stad10_mwt", "&SP", "STAD Vol 10 p.11-17 (Joe Krutsinger, JK Monday We Trade): the printed code with SetStopLoss $1,500 per position, run on the Norgate S&P 500 continuous contract; $1,500 at the multiplier in force on the trade date ($500 per point to 1997-11-02, $250 after, R4 #293 (3))."),
    ("ch_bta2_vol2x", "&6S", "Chande, Beyond Technical Analysis (1997) p.236-239, Table 8.4: the 2x-range volatility system tested on Swiss franc continuous futures 6/30/89-6/30/95 with a $3,000 initial stop, run on the Norgate Swiss franc continuous contract (R4 #293 (1)); the $3,000 at $125,000 per 1.00."),
    ("jkbwts_bwilliams19", "&6J", "Krutsinger, Building Winning Trading Systems (seminar manual, Dec 1997) p.32-33 B Williams19, Japanese Yen CME daily: the printed TradeStation code / Quick System sheet run on the Norgate continuous contract of the tested market (R4 #293 (1)); $ amounts at the contract's $ per point (futures::MARKETS)."),
    ("jkbwts_detrend", "&6S", "Krutsinger, Building Winning Trading Systems (seminar manual, Dec 1997) p.24-25 DeTrend (Jeff Roy DeTrend Oscillator), Swiss Franc daily: the printed TradeStation code run on the Norgate continuous contract of the tested market (R4 #293 (1)); the 0.01 threshold is in the contract's own quote."),
    ("jkbwts_ehrlich17", "&CL", "Krutsinger, Building Winning Trading Systems (seminar manual, Dec 1997) p.6-7 Ehrlich17, NY light crude oil daily: the printed TradeStation code run on the Norgate continuous contract of the tested market (R4 #293 (1)); $ amounts at the contract's $ per point (futures::MARKETS)."),
    ("jkbwts_conner19", "&6J", "Krutsinger, Building Winning Trading Systems (seminar manual, Dec 1997) p.2-3 Conner 19, Japanese Yen CME daily: the printed TradeStation code / Quick System sheet run on the Norgate continuous contract of the tested market (R4 #293 (1)); $ amounts at the contract's $ per point (futures::MARKETS)."),
    ("jkbwts_ez_bonder", "&ZB", "Krutsinger, Building Winning Trading Systems (seminar manual, Dec 1997) p.20-21 EZ Bonder, Day T-Bonds daily: the printed TradeStation code / Quick System sheet run on the Norgate continuous contract of the tested market (R4 #293 (1)); $ amounts at the contract's $ per point (futures::MARKETS)."),
    ("jkbwts_fox31", "&6J", "Krutsinger, Building Winning Trading Systems (seminar manual, Dec 1997) p.8-9 Fox31, Japanese Yen CME daily: the printed TradeStation code / Quick System sheet run on the Norgate continuous contract of the tested market (R4 #293 (1)); $ amounts at the contract's $ per point (futures::MARKETS)."),
    ("jkbwts_kase30", "&6B", "Krutsinger, Building Winning Trading Systems (seminar manual, Dec 1997) p.14-15 Kase30, British Pound daily: the printed TradeStation code / Quick System sheet run on the Norgate continuous contract of the tested market (R4 #293 (1)); $ amounts at the contract's $ per point (futures::MARKETS)."),
    ("jkbwts_one_night_stand", "&6J", "Krutsinger, Building Winning Trading Systems (seminar manual, Dec 1997) p.16-17 One Night Stand, Japanese Yen CME daily: the printed TradeStation code / Quick System sheet run on the Norgate continuous contract of the tested market (R4 #293 (1)); $ amounts at the contract's $ per point (futures::MARKETS)."),
    ("jkbwts_stuckey19", "&6J", "Krutsinger, Building Winning Trading Systems (seminar manual, Dec 1997) p.28-29 Stuckey19, Japanese Yen CME daily: the printed TradeStation code / Quick System sheet run on the Norgate continuous contract of the tested market (R4 #293 (1)); $ amounts at the contract's $ per point (futures::MARKETS)."),
    ("jkbwts_williams19", "&6J", "Krutsinger, Building Winning Trading Systems (seminar manual, Dec 1997) p.34-35 Williams 19, Japanese Yen CME daily: the printed TradeStation code / Quick System sheet run on the Norgate continuous contract of the tested market (R4 #293 (1)); $ amounts at the contract's $ per point (futures::MARKETS)."),
];
/// Lane F3 (R4 #294): Harris's single-market patterns (books::mhstp), each on the contract its Pattern Library page names.
const CARDS_F3: &[(&str, &str, &str)] = &[
    ("mhs_ua830406_1", "&ZB", MHS_TB), ("mhs_ua830407_1", "&ZB", MHS_TB), ("mhs_ua830826_1", "&ZB", MHS_TB), ("mhs_ua830131_1", "&ZB", MHS_TB),
    ("mhs_ua830304_1", "&ZB", MHS_TB), ("mhs_ua830125_1", "&ZB", MHS_TB), ("mhs_ua830418_1", "&ZB", MHS_TB), ("mhs_us800310_1", "&ZB", MHS_TB),
    ("mhs_us801701_1", "&ZB", MHS_TB), ("mhs_us810220_1", "&ZB", MHS_TB), ("mhs_us800111_1", "&ZB", MHS_TB), ("mhs_hd_rt_long", "&ZB", MHS_TB),
    ("mhs_cl840221_1", "&CL", MHS_CL), ("mhs_cl850210_1", "&CL", MHS_CL), ("mhs_cl850919_1", "&CL", MHS_CL), ("mhs_cl841206_1", "&CL", MHS_CL),
    ("mhs_sf830520_1", "&6S", MHS_SF), ("mhs_sf831011_3", "&6S", MHS_SF), ("mhs_sf830519_1", "&6S", MHS_SF),
    ("mhs_si830520_1", "&SI", MHS_SI), ("mhs_si830630_1", "&SI", MHS_SI),
    // inter-market: the card runs on the TRADED contract; the pattern is read from the other market (books::mhstp::INTER)
    ("mhs_sp830128_2", "&ZB", MHS_INTER), ("mhs_sp830107_2", "&ZB", MHS_INTER), ("mhs_sp830825_2", "&ZB", MHS_INTER), ("mhs_sp830405_2", "&ZB", MHS_INTER),
    ("mhs_us881011_2", "&SP", MHS_INTER), ("mhs_sf850419_2", "&SI", MHS_INTER),
    ("rgc_im_sp_tbond", "&SP", "Ruggiero, Cybernetic Trading Strategies (Wiley 1997) p.30-31 Table 1.4 (S&P500/T-Bond divergence, back-adjusted continuous contracts 4/21/82-2/7/96, $50 costs) and Table 7.5 p.117 (the rule as EasyLanguage, run without its MEM filter): run on the Norgate S&P 500 continuous contract with the Norgate T-bond contract as Data2 (R4 #293 (1), #294 (1)), lane F3."),
];
const MHS_INTER: &str = "Michael Harris, Short-Term Trading with Price Patterns (Traders Press 1998/2000) p.194-205 (inter-market patterns): the printed pattern logic on one contract (S&P 500, T-bond or Swiss franc futures), the position in another (T-bonds, S&P 500 or COMEX silver) at the Open of Tomorrow, fixed profit target / protective stop in the traded contract's points, good-til-cancelled (p.63); both Norgate continuous contracts, back-adjusted, joined by exact date (R4 #293 (1), #294 (1)), lane F3.";
const MHS_TB: &str = "Michael Harris, Short-Term Trading with Price Patterns (Traders Press 1998/2000) p.131-160 (T-Bond patterns), p.64-66 (H&D RT LONG): the printed pattern logic, entry (close or open of tomorrow) and fixed profit target / protective stop in T-bond points (1 point = 1.00 of par = $1,000), good-til-cancelled (p.63), one position per pattern; run on the Norgate continuous T-bond contract (R4 #293 (1), #294 (1)), lane F3. Norgate &ZB session coverage not stated.";
const MHS_CL: &str = "Michael Harris, Short-Term Trading with Price Patterns (Traders Press 1998/2000) p.162-170 (NYMEX crude oil patterns): the printed pattern logic, entry and fixed profit target / protective stop in crude points (1 point = $1.00 a barrel = $1,000), good-til-cancelled (p.63), one position per pattern; run on the Norgate continuous crude oil contract (R4 #293 (1), #294 (1)), lane F3.";
const MHS_SF: &str = "Michael Harris, Short-Term Trading with Price Patterns (Traders Press 1998/2000) p.172-178 (IMM Swiss franc patterns): the printed pattern logic, entry and fixed profit target / protective stop in franc points (1 point = 0.01 US$ per franc = $1,250), good-til-cancelled (p.63), one position per pattern; run on the Norgate continuous Swiss franc contract (R4 #293 (1), #294 (1)), lane F3.";
const MHS_SI: &str = "Michael Harris, Short-Term Trading with Price Patterns (Traders Press 1998/2000) p.188-190 (COMEX silver patterns): the printed pattern logic, entry and fixed profit target / protective stop in silver points (1 point = 1 cent an ounce = $50), good-til-cancelled (p.63), one position per pattern; run on the Norgate continuous silver contract (R4 #293 (1), #294 (1)), lane F3.";
pub const LABEL: &str = "Norgate continuous futures, ends 2020-09-11; roll method not stated by the vendor.";
/// Lane F1B (R4 #293 (1)): cards whose source tests several named markets; one row per market (the card runs on each).
const CHANDE5: &str = "Chande, Beyond Technical Analysis (Wiley 1997) ch.5 p.148-173, Tables 5.1-5.13: tested on British pound, coffee, cotton, crude oil, Deutsche mark, Eurodollar, gold, Japanese yen, U.S. bond and wheat futures (1/75-7/95 or the market's first data) with a $1,500 initial stop and $100 slippage and commission; run on the Norgate continuous contract of each market Norgate carries (no Deutsche mark series), the $1,500 at the contract's $ per point (R4 #293 (1), (3)), lane F1B.";
const CHANDE_LB: &str = "Chande, Beyond Technical Analysis (Wiley 1997) p.173-177, Table 5.15 (the Long Bomb TradeStation code, 1/1/82-7/10/95): tested on British pound, coffee, cotton, Deutsche mark, Eurodollar, heating oil, Japanese yen, soybeans, Swiss franc and U.S. bond futures with a $1,500 stop; run on the Norgate continuous contract of each market Norgate carries (no Deutsche mark series) except coffee (12.2% of trades in the book window same-day order-dependent, #240); on heating oil and T-bond only inside the book's 1/1/82-7/10/95 window (R4 #314 (3)), one contract per signal (the book's NumC sizing is not modelled), \"1 point\" = one tick of the contract (#57), lane F1B.";
const CHANDE_IM: &str = "Chande, Beyond Technical Analysis (Wiley 1997) p.126-129, Tables 4.15 and 4.17: the gold-bond intermarket systems (gold, and soybeans or copper, signal the U.S. bond futures trade; continuous contracts 8/21/77-7/10/95, $5,000 initial stop, $100 slippage and commission); run on the Norgate T-bond continuous contract with the Norgate gold / soybean / copper contracts as Data2 joined by date (R4 #293 (1)), the $5,000 = 5.00 bond points, lane F1B.";
const LEB_25: &str = "LeBeau & Tan, 25 x 25 Bond System v2.0 manual (System Traders Club 1998): +DI(14) > -DI(14), ADX(14) > 20, RSI(4) < 50, buy stop 18/32 above the close, $2,500 money stop (2.5 points), 25-day-low stop that becomes the 2-day low after 25 days or once open profit exceeds 5 x the 45-day ATR (the module reproduces the manual's own Us.txt trade list exactly); run on the Norgate T-bond continuous contract (R4 #293 (1)); Norgate does not say which session its T-bond series is (the manual used day-session bonds), lane F1B.";
const LEB_CB: &str = "LeBeau & Elden, Crossbow Swiss Franc System v1.0 manual (System Traders Club 1999): ADX/ATR setups, buy stop at the 2-day high / sell stop at the prior low, ATR trailing stops, $2,500 money stop, stop and reverse (the module reproduces the manual's own SF.LNG trade list, 67/67); run on the Norgate Swiss franc continuous contract with the $2,500 at $125,000 per 1.00 (= 0.02) and the contract tick 0.0001 (R4 #293 (1), (3)), lane F1B.";
const STAD5: &str = "Omega Research, System/Strategy Trading and Development Club (STAD) Vol 5 (1998): the printed TradeStation code with the Format dialog stops in the book's contract dollars (T-bonds $1,000 per point: $1,000 money stop = 1.0, $200 breakeven = 0.2 for Channel Breakout with Pullback, ch.4 p.77-84; gold $100 per point: $200 money stop = 2.0, $400 $ risk trailing = 4.0 for Fibonacci .382, ch.3 p.34-41); run on the Norgate continuous contract of the book's test market (R4 #293 (1), (3)), lane F1B.";
const NFGU: &str = "Nelson Freeburg, Building a Mechanical Trading System from the Ground Up (TAG XVIII workbook, 1996): Pathfinder (formulas p.1, T-bond setup), Channel 25 (Fig. 27 TradeStation code) and CQIII (formulas p.4-5), tested on Deutsche mark, Swiss franc, British pound and Japanese yen futures 1/78-9/96 (10/96 for Pathfinder) with $2,000 stops; run on the Norgate continuous contract of each currency Norgate carries (no Deutsche mark series), the $2,000 at the contract's $ per point (R4 #293 (1), (3)); Pathfinder reads the Norgate T-bond contract as Data2, lane F1C.";
const CARDS_F1B: &[(&str, &str, &str)] = &[
    ("nfgu_channel25", "&6S", NFGU), ("nfgu_channel25", "&6B", NFGU), ("nfgu_channel25", "&6J", NFGU),
    ("nfgu_cqiii", "&6S", NFGU), ("nfgu_cqiii", "&6B", NFGU), ("nfgu_cqiii", "&6J", NFGU),
    ("nfgu_pathfinder", "&6S", NFGU), ("nfgu_pathfinder", "&6B", NFGU), ("nfgu_pathfinder", "&6J", NFGU),
    ("stad5_chbopb", "&ZB", STAD5), ("stad5_fib382", "&GC", STAD5),
    ("lebtc_25x25", "&ZB", LEB_25), ("lebtc_crossbow", "&6S", LEB_CB),
    ("ch_bta2_goldbond_ma", "&ZB", CHANDE_IM), ("ch_bta2_goldbond_adx", "&ZB", CHANDE_IM), ("ch_bta2_intermarket_trio", "&ZB", CHANDE_IM),
    ("ch_bta2_cbo_trail5", "&6B", CHANDE5), ("ch_bta2_cbo_trail5", "&KC", CHANDE5), ("ch_bta2_cbo_trail5", "&CT", CHANDE5), ("ch_bta2_cbo_trail5", "&CL", CHANDE5), ("ch_bta2_cbo_trail5", "&GE", CHANDE5), ("ch_bta2_cbo_trail5", "&GC", CHANDE5), ("ch_bta2_cbo_trail5", "&6J", CHANDE5), ("ch_bta2_cbo_trail5", "&ZB", CHANDE5), ("ch_bta2_cbo_trail5", "&ZW", CHANDE5),
    ("ch_bta2_cbo_volexit", "&6B", CHANDE5), ("ch_bta2_cbo_volexit", "&KC", CHANDE5), ("ch_bta2_cbo_volexit", "&CT", CHANDE5), ("ch_bta2_cbo_volexit", "&CL", CHANDE5), ("ch_bta2_cbo_volexit", "&GE", CHANDE5), ("ch_bta2_cbo_volexit", "&GC", CHANDE5), ("ch_bta2_cbo_volexit", "&6J", CHANDE5), ("ch_bta2_cbo_volexit", "&ZB", CHANDE5), ("ch_bta2_cbo_volexit", "&ZW", CHANDE5),
    ("ch_bta2_cbo_inside", "&6B", CHANDE5), ("ch_bta2_cbo_inside", "&KC", CHANDE5), ("ch_bta2_cbo_inside", "&CT", CHANDE5), ("ch_bta2_cbo_inside", "&CL", CHANDE5), ("ch_bta2_cbo_inside", "&GE", CHANDE5), ("ch_bta2_cbo_inside", "&GC", CHANDE5), ("ch_bta2_cbo_inside", "&6J", CHANDE5), ("ch_bta2_cbo_inside", "&ZB", CHANDE5), ("ch_bta2_cbo_inside", "&ZW", CHANDE5),
    ("ch_bta2_cbo_ref", "&6B", CHANDE5), ("ch_bta2_cbo_ref", "&KC", CHANDE5), ("ch_bta2_cbo_ref", "&CT", CHANDE5), ("ch_bta2_cbo_ref", "&CL", CHANDE5), ("ch_bta2_cbo_ref", "&GE", CHANDE5), ("ch_bta2_cbo_ref", "&GC", CHANDE5), ("ch_bta2_cbo_ref", "&6J", CHANDE5), ("ch_bta2_cbo_ref", "&ZB", CHANDE5), ("ch_bta2_cbo_ref", "&ZW", CHANDE5),
    ("ch_bta2_adx30", "&6B", CHANDE5), ("ch_bta2_adx30", "&KC", CHANDE5), ("ch_bta2_adx30", "&CT", CHANDE5), ("ch_bta2_adx30", "&CL", CHANDE5), ("ch_bta2_adx30", "&GE", CHANDE5), ("ch_bta2_adx30", "&GC", CHANDE5), ("ch_bta2_adx30", "&6J", CHANDE5), ("ch_bta2_adx30", "&ZB", CHANDE5), ("ch_bta2_adx30", "&ZW", CHANDE5),
    ("ch_bta2_adx_rising", "&6B", CHANDE5), ("ch_bta2_adx_rising", "&KC", CHANDE5), ("ch_bta2_adx_rising", "&CT", CHANDE5), ("ch_bta2_adx_rising", "&CL", CHANDE5), ("ch_bta2_adx_rising", "&GE", CHANDE5), ("ch_bta2_adx_rising", "&GC", CHANDE5), ("ch_bta2_adx_rising", "&6J", CHANDE5), ("ch_bta2_adx_rising", "&ZB", CHANDE5), ("ch_bta2_adx_rising", "&ZW", CHANDE5),
    ("ch_bta2_pullback", "&6B", CHANDE5), ("ch_bta2_pullback", "&KC", CHANDE5), ("ch_bta2_pullback", "&CT", CHANDE5), ("ch_bta2_pullback", "&CL", CHANDE5), ("ch_bta2_pullback", "&GE", CHANDE5), ("ch_bta2_pullback", "&GC", CHANDE5), ("ch_bta2_pullback", "&6J", CHANDE5), ("ch_bta2_pullback", "&ZB", CHANDE5), ("ch_bta2_pullback", "&ZW", CHANDE5),
    // R4 #240/#308/#314 (3): heating oil and T-bond only inside the book window (BOOK_WINDOW); coffee held (HELD)
    ("ch_bta2_long_bomb", "&6B", CHANDE_LB), ("ch_bta2_long_bomb", "&CT", CHANDE_LB), ("ch_bta2_long_bomb", "&GE", CHANDE_LB), ("ch_bta2_long_bomb", "&6J", CHANDE_LB), ("ch_bta2_long_bomb", "&ZS", CHANDE_LB), ("ch_bta2_long_bomb", "&6S", CHANDE_LB),
    ("ch_bta2_long_bomb", "&HO", CHANDE_LB), ("ch_bta2_long_bomb", "&ZB", CHANDE_LB),
];
/// R4 #314 (3) (as #294 (1)): over 1978-2020 more than 5% of these trades enter and reach the stop on one daily bar, which daily
/// bars cannot order and no futures minutes exist for (#240); inside the book's test window at most 5% do. Only that window
/// (first and last date, inclusive) is the published run period on that contract (readiness::check + auto_start / auto_end).
const BOOK_WINDOW: &[(&str, &str, &str, &str)] = &[
    // Table 5.15 "1/1/82-7/10/95" (p.176): T-bond 2.9% (5.2% 1978-2020), heating oil 0% (18.6% 1978-2020) order-dependent
    ("ch_bta2_long_bomb", "&ZB", "1982-01-01", "1995-07-10"), ("ch_bta2_long_bomb", "&HO", "1982-01-01", "1995-07-10"),
];
/// A market the source tests but the card is held on, with the reason the refusal gives (R4 #240).
const HELD: &[(&str, &str, &str)] = &[
    ("ch_bta2_long_bomb", "&KC", "held on coffee under R4 #240: 12.2% of its trades in the book's 1982-95 window (15.7% over 1978-2020) enter and hit the stop on the same day, which daily bars cannot order, and no coffee futures minute bars exist (limit 5%, R4 #314 (3))"),
];
/// Why a card is held on a market its source tests (HELD), if it is.
pub fn held(id: &str, t: &str) -> Option<&'static str> { let s = of_ticker(t)?.sym; HELD.iter().find(|h| h.0 == id && h.1 == s).map(|h| h.2) }
/// The published run period of a card on ticker `t`, if it is limited to the book's window (BOOK_WINDOW).
pub fn window(id: &str, t: &str) -> Option<(&'static str, &'static str)> {
    let s = of_ticker(t)?.sym;
    BOOK_WINDOW.iter().find(|w| w.0 == id && w.1 == s).map(|w| (w.2, w.3))
}
/// readiness::check: the run must lie inside `window` (auto_start / auto_end move it there).
pub fn window_issue(id: &str, x: &Ctx, from: usize, to: usize) -> Option<String> {
    let (a, b) = window(id, &x.r.first()?.ticker)?;
    let (f, t) = (x.r.get(from)?.date.as_str(), x.r.get(to)?.date.as_str());
    (f < a || t > b).then(|| format!("Results are published only from {a} to {b} on {}: outside the book's test window more than 5% of its trades enter and reach the stop on one day, which daily bars cannot order (R4 #240, #314 (3)).", x.r[0].ticker))
}
fn cards() -> impl Iterator<Item = &'static (&'static str, &'static str, &'static str)> { CARDS.iter().chain(CARDS_F3).chain(CARDS_F1B) }
fn card(id: &str) -> Option<&'static (&'static str, &'static str, &'static str)> { cards().find(|c| c.0 == id) }
/// The continuous contracts a futures card runs on (one for most cards; every tested market for CARDS_F1B).
pub fn card_markets(id: &str) -> Vec<&'static str> { cards().filter(|c| c.0 == id).map(|c| c.1).collect() }
pub fn card_basis(def: &crate::connors::StrategyDef) -> Option<&'static str> { card(def.id).map(|c| c.2) }
/// Readiness: a futures card runs only on the continuous contract its source names (None = this is that contract, or not a futures card).
pub fn home(id: &str, t: &str) -> Option<String> {
    card(id)?;
    let ms: Vec<&Market> = card_markets(id).into_iter().filter_map(|s| MARKETS.iter().find(|m| m.sym == s)).collect();
    let on = of_ticker(t).is_some_and(|m| ms.iter().any(|h| h.sym == m.sym));
    let names = |f: fn(&Market) -> String| ms.iter().map(|m| f(m)).collect::<Vec<_>>().join(", ");
    if let Some(h) = held(id, t) { return Some(format!("Not published on {t}: {h}; this card runs only on {} ({LABEL})", names(ticker))); }
    (!on).then(|| format!("Not published on {t}: the source trades {} futures; this card runs only on {} ({LABEL})", names(|m| m.name.to_string()), names(ticker)))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn f3_excursion_and_gross_marks_on_the_unadjusted_contract() {
        // back-adjusted bars around -11, unadjusted close = back-adjusted + 91.5 (one roll offset)
        let mk = |d: &str, o: f64, h: f64, l: f64, c: f64| Row { ticker: "US T-Bond (Norgate &ZB)".into(), date: d.into(), open: o, high: h, low: l, close: c, volume: None };
        let r = vec![mk("1990-01-02", -11.7, -11.0, -12.7, -11.2), mk("1990-01-03", -11.2, -10.0, -11.5, -10.5)];
        let mut x = Ctx::new(&r, &[], &[]);
        x.extra.insert("RAW.Close".into(), vec![80.3, 81.0]);
        // long at the open of bar 0, exit at bar 1's close
        let t = Trade { dir: 1., legs: vec![Leg { i: 0, px: -11.7, w: 1. }], exit_i: 1, exit_px: -10.5, reason: "x".into(), entry_kind: "open", signal_i: 1, open: false, exits: vec![] };
        let (out, close) = raw_basis(&x, vec![t]).unwrap();
        assert!((trade_offset(&x, &out[0]) - 91.5).abs() < 1e-9 && (out[0].legs[0].px - 79.8).abs() < 1e-9);
        let (mae, mfe, _) = crate::accounting::excursion(&x, &out[0]).unwrap();
        assert!((mae - (78.8 / 79.8 - 1.)).abs() < 1e-12 && (mfe - (81.5 / 79.8 - 1.)).abs() < 1e-12, "{mae} {mfe}");
        assert_eq!(marks(&x, &out).unwrap(), close);
        let plain = Ctx::new(&r, &[], &[]); // no RAW series: offset 0, no marks
        assert!(trade_offset(&plain, &out[0]) == 0. && marks(&plain, &out).is_none());
    }
    #[test]
    fn f3_index_series_from_norgate_compact_dates() {
        let dir = std::env::temp_dir().join(format!("f3idx{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("$COMP.csv"), "\"Date\",\"Symbol\",\"Security Name\",\"Open\",\"High\",\"Low\",\"Close\",\"Volume\"\n\"19710205\",\"$COMP\",\"Nasdaq Composite Index\",\"100\",\"100\",\"100\",\"100.5\",\"0\"\n").unwrap();
        let m = load_indices(&dir);
        assert_eq!(m.get(&("*".to_string(), "NASDAQ_COMP".to_string())).and_then(|v| v.get("1971-02-05")), Some(&100.5));
        assert!(!m.contains_key(&("*".to_string(), "NYSE_COMP".to_string()))); // a missing file loads nothing
        let _ = std::fs::remove_dir_all(&dir);
    }
    use crate::connors::Leg;
    fn row(d: &str, c: f64) -> Row { Row { ticker: "US T-Bond (Norgate &ZB)".into(), date: d.into(), open: c, high: c, low: c, close: c, volume: None } }
    #[test]
    fn ticker_names_and_multipliers() {
        let zb = of_ticker("US T-Bond (Norgate &ZB)").unwrap();
        assert_eq!(zb.sym, "&ZB");
        assert!(of_ticker("SPX").is_none() && of_ticker("US T-Bond (Norgate &XX)").is_none());
        assert_eq!(usd_per_point(zb, "1990-01-02"), Some(1000.));
        assert_eq!(usd_per_point(zb, "1970-01-02"), None);
        assert_eq!(ticker(zb), "US T-Bond (Norgate &ZB)");
    }
    #[test]
    fn raw_basis_keeps_points_and_prices_in_the_held_contract() {
        // back-adjusted -11.7 .. -9.7 with a roll offset of +91.5: a long from -11.7 to -9.7 = +2 points on a 79.8 contract
        let r = vec![row("1978-03-10", -11.7), row("1978-03-13", -10.7), row("1978-03-14", -9.7)];
        let mut x = Ctx::new(&r, &[], &[]);
        x.extra.insert("RAW.Close".into(), vec![79.8, 80.8, 81.8]);
        let t = Trade { dir: 1., legs: vec![Leg { i: 0, px: -11.7, w: 1. }], exit_i: 2, exit_px: -9.7, reason: "x".into(), entry_kind: "close", signal_i: 2, open: false, exits: vec![] };
        let (out, close) = raw_basis(&x, vec![t]).unwrap();
        assert!((out[0].legs[0].px - 79.8).abs() < 1e-9 && (out[0].exit_px - 81.8).abs() < 1e-9);
        assert!((out[0].ret(0.) - 2. / 79.8).abs() < 1e-12);
        assert!((close[1] - 80.8).abs() < 1e-9);
        assert!((buy_hold(&x, 0, 2).unwrap()[2] - (1. + 2. / 79.8)).abs() < 1e-12);
        assert!(!invalid_bar(&x, 0) || x.series("RAW.Open").is_none());
    }
    #[test]
    fn f1b_cards_run_only_on_their_contracts() {
        // lane F1B (R4 #293): each card on the book's contract(s) only; $ amounts at the contract's $ per point
        for (id, t) in [("lebtc_25x25", "US T-Bond (Norgate &ZB)"), ("lebtc_crossbow", "Swiss Franc (Norgate &6S)"), ("stad5_chbopb", "US T-Bond (Norgate &ZB)"),
                        ("stad5_fib382", "Gold (Norgate &GC)"), ("ch_bta2_goldbond_ma", "US T-Bond (Norgate &ZB)"), ("ch_bta2_long_bomb", "Soybeans (Norgate &ZS)")] {
            assert!(home(id, t).is_none(), "{id} on {t}");
            assert!(home(id, "SPX").is_some() && home(id, "Silver (Norgate &SI)").is_some(), "{id}");
        }
        let sf = MARKETS.iter().find(|m| m.sym == "&6S").unwrap();
        assert!((2500. / usd_per_point(sf, "1990-01-02").unwrap() - 0.02).abs() < 1e-12); // Crossbow $2,500 money stop
        for s in ["&KC", "&CT", "&GE", "&ZW", "&HO", "&HG", "&ZS"] { assert!(MARKETS.iter().any(|m| m.sym == s && !m.source.is_empty())); }
    }
}
