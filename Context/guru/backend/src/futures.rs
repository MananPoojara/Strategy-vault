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
    // lane P1 (vvbats_spread, Van Vliet BATS ch.46): the E-mini legs of the ES/NQ ratio spread
    Market { sym: "&ES", name: "E-mini S&P 500", tick: 0.25, usd: &[("1997-09-09", 50.)],
        source: "CME E-mini S&P 500 futures: US$50 x the S&P 500 index since launch (9 Sep 1997); tick 0.25 index point = US$12.50" },
    Market { sym: "&NQ", name: "E-mini Nasdaq-100", tick: 0.25, usd: &[("1999-06-21", 20.)],
        source: "CME E-mini Nasdaq-100 futures: US$20 x the Nasdaq-100 index since launch (21 Jun 1999); tick 0.25 index point = US$5" },
    // lane PB (Pee TDI, S&C V19:10 Figure 2 basket): contract sizes from the exchange specifications (reference data); the TDI
    // reads closes and fills at the open, so these multipliers only label the contract
    Market { sym: "&ZO", name: "Oats", tick: 0.25, usd: &[("1978-04-26", 50.)],
        source: "CBOT oats futures: 5,000 bushels per contract, quoted in US cents per bushel (1.00 = US$50; 0.25 = US$12.50)" },
    Market { sym: "&OJ", name: "Orange Juice", tick: 0.05, usd: &[("1978-08-08", 150.)],
        source: "ICE frozen concentrated orange juice futures: 15,000 lb per contract, quoted in US cents per lb (1.00 = US$150; 0.05 = US$7.50)" },
    // lane PA1 (Downs, S&C V.17:1 soybean gaps and equilateral quads)
    Market { sym: "&DX", name: "US Dollar Index", tick: 0.01, usd: &[("1985-11-20", 500.), ("1996-08-01", f64::NAN)],
        source: "FINEX/ICE US Dollar Index futures: Downs's printed P&L (V.17:1 Figure 15: +0.50 index points on 5 contracts = $875 after $375 costs) implies US$500 per index point in 1987-96 (R4 #323 (2)); the later US$1,000 contract's start date is not established, so from 1996-08-01 (after the article's last printed month) no multiplier is given and US$ amounts are refused (R4 #324 M1); tick 0.01 of the book era" },
    // lane PF (Abel, S&C V21:10 Figure 3 portfolio): contract sizes from the exchange specifications (reference data); the Abel
    // system's stops are standard-deviation distances, so these multipliers only label the contract
    // lane PF (Gibbs SPI systems, Guppy newsletters): the SPI contract (A$ per point; % returns do not depend on the currency)
    Market { sym: "&YAP", name: "ASX SPI 200", tick: 1., usd: &[("1983-02-16", 25.)],
        source: "SFE/ASX SPI 200 index futures, quoted in A$ (NOT US$: futures::NON_USD, so usd_per_point refuses it): the article's own A$25 per point ($1,500 = 60 points); the historic contract size (the 1983 All Ordinaries SPI may have been larger) is not checked (review PF2 M21); the 1983-2000 All Ordinaries SPI is spliced in by Norgate" },
    // lane PG (13 Nov 2026): Balsara's live cattle test (books::scnbps)
    Market { sym: "&LE", name: "Live Cattle", tick: 0.025, usd: &[("1978-10-23", 400.)],
        source: "CME live cattle futures: 40,000 pounds per contract, quoted in US cents per pound (1.00 = US$400; 0.025 = US$10, the tick of the book era)" },
    Market { sym: "&NG", name: "Natural Gas", tick: 0.001, usd: &[("1990-04-03", 10_000.)],
        source: "NYMEX Henry Hub natural gas futures: 10,000 MMBtu per contract, quoted in US$ per MMBtu (1.00 = US$10,000; 0.001 = US$10)" },
    Market { sym: "&ZC", name: "Corn", tick: 0.25, usd: &[("1978-01-04", 50.)],
        source: "CBOT corn futures: 5,000 bushels per contract, quoted in US cents per bushel (1.00 = US$50; 0.25 = US$12.50)" },
    Market { sym: "&CC", name: "Cocoa", tick: 1., usd: &[("1979-12-19", 10.)],
        source: "ICE cocoa futures: 10 metric tonnes per contract, quoted in US$ per tonne (1.00 = US$10)" },
    Market { sym: "&SB", name: "Sugar No. 11", tick: 0.01, usd: &[("1978-10-17", 1_120.)],
        source: "ICE Sugar No. 11 futures: 112,000 lb per contract, quoted in US cents per lb (1.00 = US$1,120; 0.01 = US$11.20)" },
    Market { sym: "&ZN", name: "10-Year T-Note", tick: 1. / 64., usd: &[("1982-05-03", 1_000.)],
        source: "CBOT 10-year U.S. Treasury note futures: US$100,000 face value, quoted in points of par (1 point = US$1,000; 1/64 = US$15.625)" },
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
/// Contracts whose `usd` row is NOT in US$ (review PF2 ca66231c M20): no US$ conversion is made for them.
pub const NON_USD: &[&str] = &["&YAP"];
/// A NaN entry ends a known period: from that date the multiplier is unknown (None), so US$ rules fail closed (R4 #324 M1).
pub fn usd_per_point(m: &Market, date: &str) -> Option<f64> {
    if NON_USD.contains(&m.sym) { return None; }
    m.usd.iter().rev().find(|(d, _)| *d <= date).map(|(_, v)| *v).filter(|v| v.is_finite())
}
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
        let rows: Vec<Row> = adj.into_iter().filter(|r| raw.contains_key(&r.date)).map(|r| Row { ticker: t.clone(), ..r }).collect();
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
        Some(Row { ticker: String::new(), date: crate::normalize_date(f.get(di)?).or_else(|| compact_date(f.get(di)?))?, open: num(oi)?, high: num(hi)?, low: num(li)?, close: num(ci)?, volume: vi.and_then(num).filter(|v| *v > 0.) })
    }).collect();
    out.sort_by(|a, b| a.date.cmp(&b.date));
    out.dedup_by(|a, b| a.date == b.date);
    Some(out)
}

/// Lane F3 (R4 #294 (2)): Norgate US indices and Nasdaq breadth, copied read-only to futures/norgate-us-indices-20200908 next to
/// us_data.csv (SOURCES.csv norgate-us-indices-20200908-f3). (file, series): each file's close becomes a series every US context
/// reads by date (NaN where the file has no bar). Other lanes add their US Indices files here.
pub const INDEX_SERIES: &[(&str, &str)] = &[("$COMP", "NASDAQ_COMP"), ("$NYA", "NYSE_COMP"), ("#NASHI", "NASDAQ_NEW_HIGHS"), ("#NASLO", "NASDAQ_NEW_LOWS"),
    // lane PB: NYSE 52-week new highs / lows 1932-2020 (SOURCES norgate-us-indices-20200908-pb)
    ("#NYSEHI", "NYSE_NEW_HIGHS"), ("#NYSELO", "NYSE_NEW_LOWS"), ("$DJU", "DJ_UTILITY"),
    // lane PF: the DJIA close for Meyers' DJstr/DJCstr (SOURCES norgate-us-indices-20200908-pf)
    ("$DJI", "DJ_INDUSTRIAL")];
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

/// Lane PF (Meyers, S&C V15:11): the CBOT T-bond continuous future, back-adjusted close (Norgate &ZB_CCB in futures/norgate-20200908,
/// SOURCES norgate-continuous-futures-20200908; R4 #293 (2): indicators on the back-adjusted series) as a series every US context reads
/// by date. A missing file loads nothing.
pub const TBOND_SERIES: &str = "TBOND_CCB";
pub fn load_tbond(dir: &std::path::Path) -> Option<BTreeMap<String, f64>> {
    let rows = read(&dir.join("&ZB_CCB.csv")).filter(|r| !r.is_empty());
    if rows.is_none() { eprintln!("warning: {TBOND_SERIES} not loaded from {}", dir.display()); }
    Some(rows?.into_iter().map(|r| (r.date, r.close)).collect())
}

/// R4 #331 (audit authors/rev/ZB-OPEN.md, lane PF): the shared label for Norgate &ZB cards that read or fill at the open.
/// Text only: appended to the card notes (main.rs catalogue); no calculation reads it. Lanes adding &ZB open-readers (e.g. F1D)
/// add a row to ZB_OPEN_CARDS.
pub const ZB_OPEN_NOTE: &str = "On Norgate &ZB, from 1999-12 to 2000-10 the open sits at the previous close (median gap at most 1 tick, a third of days exactly equal), so open fills and open-based levels in those 11 months are not true opening prices (R4 #331; the whole history is kept, as for the NIFTY opens).";
/// (card id, trades affected in 1999-12..2000-10 = open fills, or trades held through a stop/target check or pattern that reads
/// the open, per ZB-OPEN.md; extra sentence when the card's SIGNAL reads the open).
pub const ZB_OPEN_CARDS: &[(&str, usize, &str)] = &[
    ("mhs_ua830406_1", 0, ""), ("mhs_ua830407_1", 2, ""), ("mhs_ua830826_1", 4, ""), ("mhs_ua830131_1", 3, ""), ("mhs_ua830304_1", 2, ""),
    ("mhs_ua830125_1", 0, ""),
    ("mhs_us800310_1", 3, " This pattern itself reads the open (O(0)-O(2)): 3 of its signals in those months are read from these opens."),
    ("mhs_us801701_1", 3, ""), ("mhs_us810220_1", 2, ""), ("mhs_us800111_1", 3, ""),
    ("mhs_hd_rt_long", 0, " This pattern itself reads the open (O(0)); none of its signals falls in those months."),
    ("mhs_sp830128_2", 0, ""), ("mhs_sp830107_2", 0, ""), ("mhs_sp830825_2", 3, ""), ("mhs_sp830405_2", 5, ""),
    ("jkbwts_ez_bonder", 11, ""), ("konwv_bond", 5, ""), ("mhptdi_tdi", 5, ""),
    ("scpee_tcf_zb", 4, ""), ("scpee_tii_zb", 4, ""), // lane PA1 (enabled at integration int-1114a)
];
/// Lane PF (F1C item 4): the same 1999-12..2000-10 open episode on Norgate &ZN (10-year T-note), measured the ZB-OPEN.md way
/// (median |open - previous close| 0.5-1.5 ticks of 1/32 and 10-48% zero gaps in those months, vs 3-4.5 ticks and 0-10% in 1995-99).
pub const ZN_OPEN_NOTE: &str = "On Norgate &ZN, from 1999-12 to 2000-10 the open sits at or near the previous close (median gap 0.5-1.5 ticks, 10-48% of days exactly equal), as on &ZB (R4 #331), so open fills in those 11 months are not true opening prices (the whole history is kept).";
pub const ZN_OPEN_CARDS: &[(&str, usize, &str)] = &[
    ("ftm_rg_intermarket", 23, " All 23 lie inside the article's test period (1988-2002); the signals read closes only."),
    // review PF2 b9ca8ce8 M16: Abel's hard stop fills at the open when gapped; 4 of its 99 &ZN trades are held through those months
    ("scabeldiv_sma", 4, " They are held through those months; none of their exits fills at an open there."),
    ("andro_andromeda", 3, " Its orders all fill at the next open; signals read closes only."),
];
/// Review PF2 e41b28c8 M17: Norgate &ZC (corn) has a longer episode of the same kind, 1999-12 to about 2006-12 (most months 15-82% of days
/// open exactly at the previous close and median gaps 0-12 bp, against 5-14% and 13-111 bp in 1999-06..11; mixed again from 2007).
pub const ZC_OPEN_NOTE: &str = "On Norgate &ZC, from 1999-12 to about 2006-12 the open often sits at the previous close (in most months 15-82% of days exactly equal, against 5-14% before), so open fills in those years are not true opening prices (the whole history is kept, as R4 #331).";
pub const ZC_OPEN_CARDS: &[(&str, usize, &str)] = &[
    ("andro_andromeda", 30, " Its orders all fill at the next open; 7 of them fall inside The Trading Plan's 1980-2001 window and #168 passes without them."),
    ("scabeldiv_sma", 26, " They are held through those years; 2 of their exits fill at an open there."),
];
/// The card notes with every matching open-episode label appended (&ZB, &ZN, &ZC; unchanged for other cards).
pub fn zb_open_notes(id: &str, notes: &str) -> String {
    let mut s = notes.to_string();
    for (table, note, sym) in [(ZB_OPEN_CARDS, ZB_OPEN_NOTE, "&ZB"), (ZN_OPEN_CARDS, ZN_OPEN_NOTE, "&ZN"), (ZC_OPEN_CARDS, ZC_OPEN_NOTE, "&ZC")] {
        if let Some((_, n, extra)) = table.iter().find(|c| c.0 == id) {
            if !s.is_empty() { s.push(' '); }
            s += &format!("{note} {n} of this card's {sym} trades are affected.{extra}");
        }
    }
    s
}

/// Norgate's compact yyyymmdd date (the US Indices files) as yyyy-mm-dd.
fn compact_date(d: &str) -> Option<String> { (d.len() == 8 && d.bytes().all(|b| b.is_ascii_digit())).then(|| format!("{}-{}-{}", &d[..4], &d[4..6], &d[6..])) }

/// Lane PE (R4 #319): the NYSE Composite as its own context, from Norgate $NYA (futures/norgate-us-indices-20200908/$NYA.csv,
/// SOURCES.csv norgate-us-indices-20200908-f3), so Holt's NYSE Composite systems run on the index they were tested on. Close and
/// volume are real from 1970-01-05 (volume 0 before = no volume); Norgate's open/high/low equal the close before 1988-12-07, so
/// only the close(-and-volume) cards in NYA_CARDS are admitted (instrument_policy::check). A missing file loads nothing.
pub const NYA_TICKER: &str = "NYSE Composite (Norgate $NYA)";
pub const NYA_CARDS: &[&str] = &["scholt_obvdi", "scholt_nvdi",
    // lane PF: Appel & Hitschler's close-only NYSE Index systems (Stock Market Trading Systems, Tables 4-5, 4-6, 5-1, 6-2, 9-1)
    "apsmts_confirm", "apsmts_shift", "apsmts_mmacd", "apsmts_lead", "apsmts_channel"];
pub fn load_nya(dir: &std::path::Path, data: &mut BTreeMap<String, Vec<Row>>) {
    let Some(rows) = read(&dir.join("$NYA.csv")).filter(|r| !r.is_empty()) else { eprintln!("warning: index $NYA context not loaded from {}", dir.display()); return };
    println!("context {NYA_TICKER}: {} bars {} -> {}", rows.len(), rows[0].date, rows[rows.len() - 1].date);
    data.insert(NYA_TICKER.into(), rows.into_iter().map(|r| Row { ticker: NYA_TICKER.into(), ..r }).collect());
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

/// R4 #316 (lane X4): cards whose published run ends the day before the first non-positive unadjusted price (NYMEX crude
/// settled at -37.63 on 2020-04-20, where a % return has no base); readiness::auto_end / check apply it. The series itself
/// stays whole for every other card (raw_basis refuses a trade whose unadjusted entry price is not positive).
/// Lane F1C (R4 #326): six Chande ch.5 systems on crude enter at the 2020-04-20 close (unadjusted -37.63), so on &CL their run
/// ends 2020-04-17 (ch_bta2_adx30 does not trade there and keeps the whole series).
const POSITIVE_RAW_ONLY: &[&str] = &["jkbwts_ehrlich17", "ch_bta2_cbo_trail5", "ch_bta2_cbo_volexit", "ch_bta2_cbo_inside", "ch_bta2_cbo_ref",
    "ch_bta2_adx_rising", "ch_bta2_pullback",
    "scpee_tcf_cl", "scpee_tii_cl" /* lane PA1, R4 #324 review 9af03996 B1 */,
    "scnbps_cross" /* lane PG: Balsara crossover on &CL (R4 #326) */];
/// The first bar whose unadjusted open/high/low/close is <= 0, for a POSITIVE_RAW_ONLY card on a futures context.
pub fn first_nonpositive(id: &str, x: &Ctx) -> Option<usize> {
    if !POSITIVE_RAW_ONLY.contains(&id) { return None; }
    let s = ["RAW.Open", "RAW.High", "RAW.Low", "RAW.Close"].map(|k| x.series(k));
    if s.iter().any(|v| v.is_none()) { return None; }
    (0..x.c.len()).find(|&i| s.iter().flatten().any(|v| v.get(i).is_some_and(|p| *p <= 0.)))
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
const CHANDE5: &str = "Chande, Beyond Technical Analysis (Wiley 1997) ch.5 p.148-173, Tables 5.1-5.13: tested on British pound, coffee, cotton, crude oil, Deutsche mark, Eurodollar, gold, Japanese yen, U.S. bond and wheat futures (1/75-7/95 or the market's first data) with a $1,500 initial stop and $100 slippage and commission; run on the Norgate continuous contract of each market Norgate carries (no Deutsche mark series), the $1,500 at the contract's $ per point (R4 #293 (1), (3)), lane F1B; on crude oil every system but the ADX crossover runs to 2020-04-17, the day before the unadjusted price turned negative (R4 #316, #326).";
const CHANDE_LB: &str = "Chande, Beyond Technical Analysis (Wiley 1997) p.173-177, Table 5.15 (the Long Bomb TradeStation code, 1/1/82-7/10/95): tested on British pound, coffee, cotton, Deutsche mark, Eurodollar, heating oil, Japanese yen, soybeans, Swiss franc and U.S. bond futures with a $1,500 stop; run on the Norgate continuous contract of each market Norgate carries (no Deutsche mark series) except coffee (12.2% of trades in the book window same-day order-dependent, #240); on heating oil and T-bond only inside the book's 1/1/82-7/10/95 window (R4 #314 (3)), one contract per signal (the book's NumC sizing is not modelled), \"1 point\" = one tick of the contract (#57), lane F1B.";
const CHANDE_IM: &str = "Chande, Beyond Technical Analysis (Wiley 1997) p.126-129, Tables 4.15 and 4.17: the gold-bond intermarket systems (gold, and soybeans or copper, signal the U.S. bond futures trade; continuous contracts 8/21/77-7/10/95, $5,000 initial stop, $100 slippage and commission); run on the Norgate T-bond continuous contract with the Norgate gold / soybean / copper contracts as Data2 joined by date (R4 #293 (1)), the $5,000 = 5.00 bond points, lane F1B.";
const LEB_25: &str = "LeBeau & Tan, 25 x 25 Bond System v2.0 manual (System Traders Club 1998): +DI(14) > -DI(14), ADX(14) > 20, RSI(4) < 50, buy stop 18/32 above the close, $2,500 money stop (2.5 points), 25-day-low stop that becomes the 2-day low after 25 days or once open profit exceeds 5 x the 45-day ATR (the module reproduces the manual's own Us.txt trade list exactly); run on the Norgate T-bond continuous contract (R4 #293 (1)); Norgate does not say which session its T-bond series is (the manual used day-session bonds), lane F1B.";
const LEB_CB: &str = "LeBeau & Elden, Crossbow Swiss Franc System v1.0 manual (System Traders Club 1999): ADX/ATR setups, buy stop at the 2-day high / sell stop at the prior low, ATR trailing stops, $2,500 money stop, stop and reverse (the module reproduces the manual's own SF.LNG trade list, 67/67); run on the Norgate Swiss franc continuous contract with the $2,500 at $125,000 per 1.00 (= 0.02) and the contract tick 0.0001 (R4 #293 (1), (3)), lane F1B.";
const STAD5: &str = "Omega Research, System/Strategy Trading and Development Club (STAD) Vol 5 (1998): the printed TradeStation code with the Format dialog stops in the book's contract dollars (T-bonds $1,000 per point: $1,000 money stop = 1.0, $200 breakeven = 0.2 for Channel Breakout with Pullback, ch.4 p.77-84; gold $100 per point: $200 money stop = 2.0, $400 $ risk trailing = 4.0 for Fibonacci .382, ch.3 p.34-41); run on the Norgate continuous contract of the book's test market (R4 #293 (1), (3)), lane F1B.";
const NFGU: &str = "Nelson Freeburg, Building a Mechanical Trading System from the Ground Up (TAG XVIII workbook, 1996): Pathfinder (formulas p.1, T-bond setup), Channel 25 (Fig. 27 TradeStation code) and CQIII (formulas p.4-5), tested on Deutsche mark, Swiss franc, British pound and Japanese yen futures 1/78-9/96 (10/96 for Pathfinder) with $2,000 stops; run on the Norgate continuous contract of each currency Norgate carries (no Deutsche mark series), the $2,000 at the contract's $ per point (R4 #293 (1), (3)); Pathfinder reads the Norgate T-bond contract as Data2, lane F1C.";
const LWTDM: &str = "Larry Williams, Long-Term Secrets to Short-Term Trading (Wiley 1999) p.88-89, Table 6.7: buy the open of the trading day of the month shown, $1,500 stop from the entry day, exit on the close of the third day after entry, T-bond futures 1977-1998; run on the Norgate T-bond continuous contract ($1,500 = 1.5 points at $1,000 per point; R4 #293 (1), (3)), lane F1C.";
const CARDS_F1B: &[(&str, &str, &str)] = &[
    ("nfgu_channel25", "&6S", NFGU), ("nfgu_channel25", "&6B", NFGU), ("nfgu_channel25", "&6J", NFGU),
    ("nfgu_cqiii", "&6S", NFGU), ("nfgu_cqiii", "&6B", NFGU), ("nfgu_cqiii", "&6J", NFGU),
    ("nfgu_pathfinder", "&6S", NFGU), ("nfgu_pathfinder", "&6B", NFGU), ("nfgu_pathfinder", "&6J", NFGU),
    ("stad5_chbopb", "&ZB", STAD5), ("stad5_fib382", "&GC", STAD5),
    ("lwlts_tdm8_bonds", "&ZB", LWTDM),
    ("lwlts_tdm18_bonds", "&ZB", LWTDM),
    ("lwlts_tdm19_bonds", "&ZB", LWTDM),
    ("lwlts_tdm20_bonds", "&ZB", LWTDM),
    ("lwlts_tdm21_bonds", "&ZB", LWTDM),
    ("lwlts_tdm22_bonds", "&ZB", LWTDM),
    ("stad11_miser", "&SP", "Omega Research, STAD Vol 11 (2000) ch.3 p.45-56 and Appendix A, MISER: the printed TradeStation 2000i code (S&P 500 futures with T-bond futures as Data2, long side, ATR stops; test 2/92-11/99 with $50 per trade); run on the Norgate S&P 500 continuous contract with the Norgate T-bond contract as Data2 (R4 #293 (1)), lane F1C."),
    ("stad5_curbond", "&6S", "Omega Research, STAD Vol 5 (1998) ch.2 p.12-20, Currency/Bond System: the printed TradeStation code on the Swiss franc with T-bond futures as Data2 (book test 6/93-5/98, $1,000 StopAmt and $1,200 Format money-management stop, no costs); run on the Norgate Swiss franc continuous contract with the Norgate T-bond contract as Data2 (R4 #293 (1), (3)), $ amounts at $125,000 per 1.00, lane F1C."),
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
    // STAD Vol 5 p.16 "from June, 1993, to May, 1998": 4.5% of trades touch a bar with fills on both legs of its path (7.0% 1979-2020)
    ("stad5_curbond", "&6S", "1993-06-01", "1998-05-31"),
];
/// A market the source tests but the card is held on, with the reason the refusal gives (R4 #240).
const HELD: &[(&str, &str, &str)] = &[
    // lane F1D: Larry Williams bond stop-entry cards (no T-bond minute bars; limit 5%)
    ("lwlts_vb_bonds", "&ZB", "held under R4 #240: 22.4% of its trades (27.7% in the book's 1990-98 window) have two fills on one daily bar (entry and stop, or stop and reversal), which daily bars cannot order, and no T-bond minute bars exist (limit 5%)"),
    ("lwlts_vb_bonds_tdw", "&ZB", "held under R4 #240: 16.8% of its trades (19.5% in the book's 1990-98 window) have two fills on one daily bar, which daily bars cannot order, and no T-bond minute bars exist (limit 5%)"),
    ("lwlts_gsv_sp", "&SP", "held under R4 #240: 5.1% of its trades on the published 1982-95 window have two fills on one daily bar (an entry and the reversal), which daily bars cannot order, and no S&P futures minute bars exist (limit 5%)"),
    ("lwlts_gsv_bonds", "&ZB", "held under R4 #240: 16.1% of its trades (16.3% in the book's 1990-98 window) have two fills on one daily bar (an entry and the reversal), which daily bars cannot order, and no T-bond minute bars exist (limit 5%)"),
    ("ch_bta2_long_bomb", "&KC", "held on coffee under R4 #240: 12.2% of its trades in the book's 1982-95 window (15.7% over 1978-2020) enter and hit the stop on the same day, which daily bars cannot order, and no coffee futures minute bars exist (limit 5%, R4 #314 (3))"),
];
/// Why a card is held on a market its source tests (HELD), if it is.
pub fn held(id: &str, t: &str) -> Option<&'static str> { let s = of_ticker(t)?.sym; HELD.iter().find(|h| h.0 == id && h.1 == s).map(|h| h.2) }
/// The bars of a Norgate market as main loaded them (rotation::PRELOADED); tests supply them per thread (TEST_ROWS).
pub fn rows(sym: &str) -> Option<&'static [Row]> {
    #[cfg(test)]
    if let Some(r) = TEST_ROWS.with(|t| t.borrow().get(sym).copied()) { return Some(r); }
    let m = MARKETS.iter().find(|m| m.sym == sym)?;
    crate::books::rotation::PRELOADED.get()?.get(&ticker(m)).map(|v| v.as_slice())
}
#[cfg(test)]
thread_local! { pub static TEST_ROWS: std::cell::RefCell<BTreeMap<&'static str, &'static [Row]>> = Default::default(); }
/// Cards that read a second Norgate contract (Data2) and the contracts they read (lane F1C, review REV-F1B M3).
const DATA2: &[(&str, &[&str])] = &[("ch_bta2_goldbond_ma", &["&GC"]), ("ch_bta2_goldbond_adx", &["&GC"]), ("ch_bta2_intermarket_trio", &["&GC", "&ZS", "&HG"]),
    ("nfgu_pathfinder", &["&ZB"]), ("stad5_curbond", &["&ZB"]), ("stad11_miser", &["&ZB"]),
    ("lwlts_talon_bondfilter", &["&ZB"]), ("lwlts_gsv_sp", &["&ZB"]), ("lwlts_gsv_sp_monday", &["&ZB"]), ("lwlts_month_day1_sp_bonds", &["&ZB"]),
    ("lwlts_bonds_channel_bstop", &["&ZB"]), ("lwlts_bonds_channel_sstop", &["&ZB"]),
    ("lwlts_tdm18_gold_bonds", &["&GC"]), ("lwlts_tdm22_gold_bonds", &["&GC"]), ("lwlts_tdm12_sell_gold_bonds", &["&GC"])];
/// readiness::check: a Data2 card whose second contract is not loaded is refused (needs_data), never shown as a 0-trade run.
pub fn data2_issue(id: &str, x: &Ctx) -> Option<String> {
    ctx_market(x)?;
    let (_, syms) = DATA2.iter().find(|d| d.0 == id)?;
    let miss: Vec<String> = syms.iter().filter(|s| rows(s).is_none()).filter_map(|s| MARKETS.iter().find(|m| m.sym == *s).map(ticker)).collect();
    (!miss.is_empty()).then(|| format!("This card reads {} as a second data series, and it is not loaded; no result is published.", miss.join(", ")))
}
/// R4 #334 (lane PF): a card whose US$ rules need the contract's multiplier, on a contract where it is known only for a period
/// (MARKETS NaN = unknown): only that period is published on that contract.
const USD_WINDOW: &[(&str, &str, &str, &str)] = &[("andro_andromeda", "&DX", "1985-11-20", "1996-07-31")];
const USD_WHY: &str = "the contract's US$ per point is known only to 1996-07-31 (R4 #323/#324: US$500 per index point 1985-11..1996-07; the start of the later US$1,000 contract is not established), and this card's $4,000 risk limit and $10,000 profit exit need it (R4 #334)";
/// The published run period of a card on ticker `t`, if it is limited to the book's window (BOOK_WINDOW) or to a known multiplier (USD_WINDOW).
pub fn window(id: &str, t: &str) -> Option<(&'static str, &'static str)> {
    let s = of_ticker(t)?.sym;
    BOOK_WINDOW.iter().chain(USD_WINDOW).find(|w| w.0 == id && w.1 == s).map(|w| (w.2, w.3))
}
/// Why `window` limits the run (for readiness::auto_end's label and the refusal).
pub fn window_why(id: &str, t: &str) -> &'static str {
    let usd = of_ticker(t).is_some_and(|m| USD_WINDOW.iter().any(|w| w.0 == id && w.1 == m.sym));
    if usd { USD_WHY } else { "the book's test window on this contract, R4 #314 (3)" }
}
/// readiness::check: the run must lie inside `window` (auto_start / auto_end move it there).
pub fn window_issue(id: &str, x: &Ctx, from: usize, to: usize) -> Option<String> {
    let t = &x.r.first()?.ticker;
    let (a, b) = window(id, t)?;
    let (f, e) = (x.r.get(from)?.date.as_str(), x.r.get(to)?.date.as_str());
    if f < a || e > b {
        if window_why(id, t) == USD_WHY { return Some(format!("Results are published only from {a} to {b} on {t}: {USD_WHY}.")); }
    }
    window_issue_book(id, x, from, to)
}
fn window_issue_book(id: &str, x: &Ctx, from: usize, to: usize) -> Option<String> {
    let (a, b) = window(id, &x.r.first()?.ticker)?;
    let (f, t) = (x.r.get(from)?.date.as_str(), x.r.get(to)?.date.as_str());
    (f < a || t > b).then(|| format!("Results are published only from {a} to {b} on {}: outside the book's test window more than 5% of its trades depend on the order of fills inside one daily bar, which daily bars cannot settle (R4 #240, #314 (3)).", x.r[0].ticker))
}
/// Lane PG (13 Nov 2026): Sweeney's and Saitta's S&C T-bond tests (books::scswrl, books::scsarf), on the contract they name.
const CARDS_PG: &[(&str, &str, &str)] = &[("scswrl_underlying", "&ZB", SWRL), ("scswrl_reverse", "&ZB", SWRL), ("scsarf_hs", "&ZB", SARF), /* scsarf_dt held (REV-PG B1, R4 #297 (5)): no home, so instrument_policy refuses it with its held reason */
    ("scpged_weekly", "&GE", PGED), ("tswtc_coffee", "&KC", WTC), ("scnbps_cross", "&GE", NBPS), ("scnbps_cross", "&6S", NBPS), ("scnbps_cross", "&SP", NBPS), ("scnbps_cross", "&LE", NBPS), ("scnbps_cross", "&CL", NBPS)];
const PGED: &str = "George Panagakis, 'Trading The Eurodollar', Technical Analysis of Stocks & Commodities V.13:3 (1995) p.102-106 (Figure 5: June/December Eurodollar contracts, fiscal years 1982-1993): run on the Norgate Eurodollar continuous contract only (R4 #293 (1))";
const NBPS: &str = "Nauzer Balsara, 'Using Probability Stops In Trading', Technical Analysis of Stocks & Commodities V.10:5 (1992) p.225-228 (Figures 1-5: Eurodollar, Swiss franc, S&P 500, live cattle and crude oil futures, 1988-1991, closing prices): run on those five Norgate continuous contracts only (R4 #293 (1))";
const SWRL: &str = "John Sweeney, 'Reversing Your Losses', Technical Analysis of Stocks & Commodities V.10:4 (1992) p.184-186 (Figure 5: December 1986 T-bond contract), with the underlying system's printed rules from V.10:1 (1992) p.43-45 Figure 1: run on the Norgate T-bond continuous contract only (R4 #293 (1))";
const WTC: &str = "Market Analytics, William Tell Coffee Trading System (WT Coffe Strategy.txt + Manual.pdf; Manual performance summaries 1/2/80-4/2/97 on coffee futures): run on the Norgate coffee continuous contract only (R4 #293 (1); lane PG moved the live card here from its declared index application)";
const SARF: &str = "Alex Saitta, 'Reversal Formations: Predictive Power?', Technical Analysis of Stocks & Commodities V.16:11 (1998) p.513-517 (Figures 4 and 7: continuous T-bond contract 1977-1998, one contract, no costs): run on the Norgate T-bond continuous contract only (R4 #293 (1))";
/// Every lane's card list (R4 #329): card(), card_markets() and home() read only this, and the refresh builds its futures contexts
/// from these tuples, so a new lane list is added here once.
fn cards() -> impl Iterator<Item = &'static (&'static str, &'static str, &'static str)> { CARDS.iter().chain(CARDS_F3).chain(CARDS_P1).chain(CARDS_F1B).chain(CARDS_PB).chain(CARDS_PA1).chain(CARDS_PA2).chain(CARDS_PF).chain(CARDS_PG).chain(CARDS_F1D) }
fn card(id: &str) -> Option<&'static (&'static str, &'static str, &'static str)> { cards().find(|c| c.0 == id) }
/// The continuous contracts a futures card runs on (one for most cards; every tested market for CARDS_F1B).
pub fn card_markets(id: &str) -> Vec<&'static str> { cards().filter(|c| c.0 == id).map(|c| c.1).collect() }
/// Lane F1D (continues F1B/F1C).
/// Lane F1D: Larry Williams's printed T-bond rules (Long-Term Secrets to Short-Term Trading ch.4, 6, 7, 8).
const LWB: &str = "Larry Williams, Long-Term Secrets to Short-Term Trading (Wiley 1999): the T-bond rules of ch.4 (p.61-65), ch.6 (p.86-87), ch.7 (p.98, 116-118) and ch.8 (p.124-126), tested on CSI \"DAY T-BONDS\" back-adjusted data with System Writer/TradeStation; run on the Norgate T-bond continuous contract with the book's dollar stops at $1,000 per point (R4 #293 (1), (3)), lane F1D.";
const LWB10: &str = "Larry Williams, Long-Term Secrets to Short-Term Trading (Wiley 1999) ch.10 p.150-157, Figs. 10.2, 10.6, 10.8-10.10: T-bond first trading day of the month, TDM 18 / 22 buys with the gold downtrend filter and TDM 12 sells (with and without the gold uptrend filter), DAY T-BONDS 01/01/86-08/28/98; run on the Norgate T-bond continuous contract with the Norgate gold contract as Data2 for the gold filters ($ stops at $1,000 per point; R4 #293 (1), (3)), lane F1D.";
const LWSPB: &str = "Larry Williams, Long-Term Secrets to Short-Term Trading (Wiley 1999): S&P 500 rules filtered or triggered by T-bonds (Fig. 4.21 p.72-73, Figs. 8.3-8.4 p.125-128, ch.10 p.148-149, Figs. 15.1-15.2 p.236-237), S&P 500 IND-9967 with $250 per point; run on the Norgate S&P 500 continuous contract with the Norgate T-bond contract as Data2, published 1982-04-21..1995-07-31 (R4 #293 (1), #330), lane F1D.";
const CARDS_F1D: &[(&str, &str, &str)] = &[
    ("lwlts_vb_bonds", "&ZB", LWB),
    ("lwlts_vb_bonds_tdw", "&ZB", LWB),
    ("lwlts_tdw_bonds", "&ZB", LWB),
    ("lwlts_outside_down_bonds", "&ZB", LWB),
    ("lwlts_outside_down_bonds_xthu", "&ZB", LWB),
    ("lwlts_3up_thu_sell_bonds", "&ZB", LWB),
    ("lwlts_oops_buy_bonds", "&ZB", LWB),
    ("lwlts_oops_sell_bonds_wed", "&ZB", LWB),
    ("lwlts_oops_buy_bonds_ma9", "&ZB", LWB),
    ("lwlts_oops_sell_bonds_ma9", "&ZB", LWB),
    ("lwlts_gsv_bonds", "&ZB", LWB),
    ("lwlts_month_day1_bonds", "&ZB", LWB10),
    ("lwlts_tdm18_gold_bonds", "&ZB", LWB10),
    ("lwlts_tdm22_gold_bonds", "&ZB", LWB10),
    ("lwlts_tdm12_sell_bonds", "&ZB", LWB10),
    ("lwlts_tdm12_sell_gold_bonds", "&ZB", LWB10),
    ("lwlts_talon_bondfilter", "&SP", LWSPB),
    ("lwlts_gsv_sp", "&SP", LWSPB),
    ("lwlts_gsv_sp_monday", "&SP", LWSPB),
    ("lwlts_month_day1_sp_bonds", "&SP", LWSPB),
    ("lwlts_bonds_channel_bstop", "&SP", LWSPB),
    ("lwlts_bonds_channel_sstop", "&SP", LWSPB),
    ("lwlts_bonds_8day", "&ZB", "Larry Williams, Long-Term Secrets to Short-Term Trading (Wiley 1999) p.212-213, the August 1997 letter: buy the close when it is above the close 8 days ago, $850 stop, $3,500 target, T-bond futures since 1977 with $50 commission and slippage; run on the Norgate T-bond continuous contract ($850 = 0.85 point, $3,500 = 3.5 points at $1,000 per point; R4 #293 (1), (3)), lane F1D."),
];
/// Lane P1: two-leg spreads on two continuous contracts, run in the first contract's context.
const CARDS_P1: &[(&str, &str, &str)] = &[("vvbats_spread", "&ES", "Benjamin Van Vliet, Building Automated Trading Systems (Academic Press 2007) ch.46 p.290-301 (Spreader_Example: spread = -1 x ES + 2 x NQ, z of the last 30 spread prices, |z| > 2 entry, target z = 0, stop 4 ES ticks): run on the Norgate E-mini S&P 500 continuous contract with the Norgate E-mini Nasdaq-100 contract as the second leg, joined by exact date, daily closes instead of bid/ask quotes (R4 #293 (1), #297), lane P1.")];
/// (traded contract, second leg): the second leg's back-adjusted and unadjusted closes are attached to the traded contract's
/// context as `<sym>.Close` / `<sym>.RAW.Close` (extra series: exact date, NaN where it has no bar).
pub const LEG2: &[(&str, &str)] = &[("&ES", "&NQ")];
pub fn leg2_series(data: &BTreeMap<String, Vec<Row>>, extra: &mut BTreeMap<(String, String), BTreeMap<String, f64>>) {
    for (a, b) in LEG2 {
        let (Some(ma), Some(mb)) = (MARKETS.iter().find(|m| m.sym == *a), MARKETS.iter().find(|m| m.sym == *b)) else { continue };
        let (ta, tb) = (ticker(ma), ticker(mb));
        let Some(rows) = data.get(&tb) else { continue };
        extra.insert((ta.clone(), format!("{b}.Close")), rows.iter().map(|r| (r.date.clone(), r.close)).collect());
        if let Some(raw) = extra.get(&(tb, "RAW.Close".to_string())).cloned() { extra.insert((ta, format!("{b}.RAW.Close")), raw); }
    }
}
/// Lane PF: Saitta (S&C V14:3) on the T-bond; Abel (S&C V21:10) on each Figure 3 contract held (the regular Nasdaq-100 contract is
/// not held: its E-mini, which the article names, stands in).
const ABEL: &str = "Phil Abel, Diversification And Your Portfolio, S&C V21:10 p.20-31 (Figure 3: 16 US futures, 1995-2002): the printed SMA 25/50/100 system with the 4-standard-deviation hard stop on this Norgate continuous contract (R4 #293 (1)), lane PF.";
const ANDRO: &str = "Andromeda (andromedafutures.com), Open Code.txt + The Trading Plan (2002): the vendor's TradeStation code on this Norgate continuous contract of the recommended 8-market portfolio (back-adjusted daily bars, BigPointValue = the contract's US$ per point on the bar's date, R4 #293 (1), (3)), lane PF.";
const CARDS_PF: &[(&str, &str, &str)] = &[
    ("scbrower_long", "&SP", "William Brower, The S&P 500 Seasonal Day Trade, S&C V14:8 p.338-342: the buy patterns with their DoWinMo filters on the Norgate continuous S&P 500 contract, back-adjusted daily bars (R4 #293 (1)), lane PF."),
    ("scbrower_short", "&SP", "William Brower, The S&P 500 Seasonal Day Trade, S&C V14:8 p.338-342: the sell patterns with their DoWinMo filters on the Norgate continuous S&P 500 contract, back-adjusted daily bars (R4 #293 (1)), lane PF."),
    ("andro_andromeda", "&6S", ANDRO), ("andro_andromeda", "&CL", ANDRO), ("andro_andromeda", "&ZN", ANDRO), ("andro_andromeda", "&ZC", ANDRO),
    ("andro_andromeda", "&6J", ANDRO), ("andro_andromeda", "&DX", ANDRO), ("andro_andromeda", "&GE", ANDRO), ("andro_andromeda", "&CT", ANDRO),
    ("dgnl_spi_12_10", "&YAP", "A. Gibbs in Daryl Guppy's newsletter 01/09/2000 p.5-6 (Testing SPI Breakouts, 12-day high / 10-day low): run on the Norgate continuous SPI contract, back-adjusted daily bars (R4 #293 (1)), lane PF."),
    ("dgnl_spi_seasonal_q1", "&YAP", "A. Gibbs in Daryl Guppy's newsletter 29/09/2000 p.13-14 (Seasonality And Trading, January-March dates): run on the Norgate continuous SPI contract, back-adjusted daily bars (R4 #293 (1)), lane PF."),
    ("dgnl_spi_dom", "&YAP", "A. Gibbs in Daryl Guppy's newsletter 08/09/2000 p.20-22 (Day of the Month SPI rules): run on the Norgate continuous SPI contract, back-adjusted daily bars (R4 #293 (1)), lane PF."),
    ("ftm_rg_intermarket", "&ZN", "Murray Ruggiero, What makes a system a winner, Futures Magazine May 2002 p.59: the silver / 10-year T-note intermarket rule on the Norgate continuous 10-year T-note contract with Norgate silver as the second market (R4 #293 (1)), lane PF."),
    ("scsaitta_volma", "&ZB", "Alex Saitta, A Price- And Volume-Based System, S&C V14:3 p.103-105: the T-bond 20-day moving average with the volume condition on the Norgate continuous T-bond contract, back-adjusted daily bars (R4 #293 (1)), lane PF."),
    ("scabeldiv_sma", "&6J", ABEL),
    ("scabeldiv_sma", "&6S", ABEL),
    ("scabeldiv_sma", "&GC", ABEL),
    ("scabeldiv_sma", "&SI", ABEL),
    ("scabeldiv_sma", "&SP", ABEL),
    ("scabeldiv_sma", "&NQ", ABEL),
    // ("scabeldiv_sma", "&CL"): not run: a crude trade held through 2020-04-20 (the short of 2020-02-27 at 47.09) is priced below zero in the contract held at entry (its mark is -38.32 on 2020-04-20, exit -15.84), so the engine's trade validation refuses the whole run
    ("scabeldiv_sma", "&NG", ABEL),
    ("scabeldiv_sma", "&ZC", ABEL),
    ("scabeldiv_sma", "&ZW", ABEL),
    ("scabeldiv_sma", "&CC", ABEL),
    ("scabeldiv_sma", "&SB", ABEL),
    ("scabeldiv_sma", "&CT", ABEL),
    ("scabeldiv_sma", "&KC", ABEL),
    ("scabeldiv_sma", "&ZN", ABEL),
    ("scabeldiv_sma", "&GE", ABEL),
];
/// Lane PB: Konstenius wave 1 (S&C V14:12), each printed parameter set on its own contract.
const CARDS_PB: &[(&str, &str, &str)] = &[
    ("mhptdi_tdi", "&ZB", "M.H. Pee, Trend Detection Index, S&C V19:10 p.54-61 and Figure 2 (15 futures, 16 years): the sidebar spreadsheet rule on this Norgate continuous contract (one of the 14 Figure 2 markets the vault holds; no Deutschemark), back-adjusted (R4 #293 (1)), lane PB."),
    ("mhptdi_tdi", "&6B", "M.H. Pee, Trend Detection Index, S&C V19:10 p.54-61 and Figure 2 (15 futures, 16 years): the sidebar spreadsheet rule on this Norgate continuous contract (one of the 14 Figure 2 markets the vault holds; no Deutschemark), back-adjusted (R4 #293 (1)), lane PB."),
    ("mhptdi_tdi", "&KC", "M.H. Pee, Trend Detection Index, S&C V19:10 p.54-61 and Figure 2 (15 futures, 16 years): the sidebar spreadsheet rule on this Norgate continuous contract (one of the 14 Figure 2 markets the vault holds; no Deutschemark), back-adjusted (R4 #293 (1)), lane PB."),
    // ("mhptdi_tdi", "&CL"): not run: the 2020-04-20 negative crude settlement leaves a TDI trade without a % base (raw_basis refuses the whole run)
    ("mhptdi_tdi", "&HG", "M.H. Pee, Trend Detection Index, S&C V19:10 p.54-61 and Figure 2 (15 futures, 16 years): the sidebar spreadsheet rule on this Norgate continuous contract (one of the 14 Figure 2 markets the vault holds; no Deutschemark), back-adjusted (R4 #293 (1)), lane PB."),
    ("mhptdi_tdi", "&CT", "M.H. Pee, Trend Detection Index, S&C V19:10 p.54-61 and Figure 2 (15 futures, 16 years): the sidebar spreadsheet rule on this Norgate continuous contract (one of the 14 Figure 2 markets the vault holds; no Deutschemark), back-adjusted (R4 #293 (1)), lane PB."),
    ("mhptdi_tdi", "&GE", "M.H. Pee, Trend Detection Index, S&C V19:10 p.54-61 and Figure 2 (15 futures, 16 years): the sidebar spreadsheet rule on this Norgate continuous contract (one of the 14 Figure 2 markets the vault holds; no Deutschemark), back-adjusted (R4 #293 (1)), lane PB."),
    ("mhptdi_tdi", "&GC", "M.H. Pee, Trend Detection Index, S&C V19:10 p.54-61 and Figure 2 (15 futures, 16 years): the sidebar spreadsheet rule on this Norgate continuous contract (one of the 14 Figure 2 markets the vault holds; no Deutschemark), back-adjusted (R4 #293 (1)), lane PB."),
    ("mhptdi_tdi", "&HO", "M.H. Pee, Trend Detection Index, S&C V19:10 p.54-61 and Figure 2 (15 futures, 16 years): the sidebar spreadsheet rule on this Norgate continuous contract (one of the 14 Figure 2 markets the vault holds; no Deutschemark), back-adjusted (R4 #293 (1)), lane PB."),
    ("mhptdi_tdi", "&6J", "M.H. Pee, Trend Detection Index, S&C V19:10 p.54-61 and Figure 2 (15 futures, 16 years): the sidebar spreadsheet rule on this Norgate continuous contract (one of the 14 Figure 2 markets the vault holds; no Deutschemark), back-adjusted (R4 #293 (1)), lane PB."),
    ("mhptdi_tdi", "&ZO", "M.H. Pee, Trend Detection Index, S&C V19:10 p.54-61 and Figure 2 (15 futures, 16 years): the sidebar spreadsheet rule on this Norgate continuous contract (one of the 14 Figure 2 markets the vault holds; no Deutschemark), back-adjusted (R4 #293 (1)), lane PB."),
    ("mhptdi_tdi", "&OJ", "M.H. Pee, Trend Detection Index, S&C V19:10 p.54-61 and Figure 2 (15 futures, 16 years): the sidebar spreadsheet rule on this Norgate continuous contract (one of the 14 Figure 2 markets the vault holds; no Deutschemark), back-adjusted (R4 #293 (1)), lane PB."),
    ("mhptdi_tdi", "&6S", "M.H. Pee, Trend Detection Index, S&C V19:10 p.54-61 and Figure 2 (15 futures, 16 years): the sidebar spreadsheet rule on this Norgate continuous contract (one of the 14 Figure 2 markets the vault holds; no Deutschemark), back-adjusted (R4 #293 (1)), lane PB."),
    ("mhptdi_tdi", "&SI", "M.H. Pee, Trend Detection Index, S&C V19:10 p.54-61 and Figure 2 (15 futures, 16 years): the sidebar spreadsheet rule on this Norgate continuous contract (one of the 14 Figure 2 markets the vault holds; no Deutschemark), back-adjusted (R4 #293 (1)), lane PB."),
    ("konwv_yen", "&6J", "Jeremy G. Konstenius, Combining Trend And Oscillator Signals, S&C V14:12 p.520-525: the wave 1 model with the yen parameters (30/5/50 weeks) on the Norgate continuous Japanese yen contract, back-adjusted weekly bars (R4 #293 (1)), lane PB."),
    ("konwv_bond", "&ZB", "Jeremy G. Konstenius, Combining Trend And Oscillator Signals, S&C V14:12 p.520-525: the wave 1 model with the T-bond parameters (20/10/30 weeks) on the Norgate continuous T-bond contract, back-adjusted weekly bars (R4 #293 (1)), lane PB."),
];
/// Lane PA1: Downs's tactical patterns (books::scdtac), each on the contract the article tests (S&C V.17:1).
const DOWNS: &str = "Walter T. Downs, The Tactical Trader, Technical Analysis of Stocks & Commodities V.17:1 (Jan 1999) p.27-37: weekly soybean gaps and monthly equilateral quads, each tested by the author on one futures market (Omega back-adjusted continuous data); run on the Norgate continuous contract of that market (R4 #293 (1)), lane PA1.";
const PEE: &str = "M.H. Pee, Trend Continuation Factor (S&C V.20:3 p.58-64) and Trend Intensity Index (S&C V.20:6 p.32-35): reversal systems tested on continuous contracts of 15 futures 1/4/1982-12/31/1998; each card runs on the Norgate continuous contract of one market the article tested and the vault holds (R4 #293 (1), #323 (1)), lane PA1.";
const CARDS_PA1: &[(&str, &str, &str)] = &[("scdtac_gaps_zs", "&ZS", DOWNS), ("scpee_tcf_zb", "&ZB", PEE), ("scpee_tcf_6b", "&6B", PEE), ("scpee_tcf_cl", "&CL", PEE), ("scpee_tcf_gc", "&GC", PEE), ("scpee_tcf_6j", "&6J", PEE), ("scpee_tcf_6s", "&6S", PEE), ("scpee_tcf_si", "&SI", PEE), ("scpee_tii_zb", "&ZB", PEE), ("scpee_tii_6b", "&6B", PEE), ("scpee_tii_cl", "&CL", PEE), ("scpee_tii_gc", "&GC", PEE), ("scpee_tii_6j", "&6J", PEE), ("scpee_tii_6s", "&6S", PEE), ("scpee_tii_si", "&SI", PEE)];
/// Lane PA2 (13 Nov 2026): Barrie's CBOT T-bond tests (books::scsbqs), on the T-bond continuous contract the article names.
const SBQS: &str = "Scott Barrie, 'Quasi-Seasonal Tendencies in Bond Futures', Technical Analysis of Stocks & Commodities V.14:1 (1996) p.19-25 (Figures 4, 7, 10: CBOT long bond futures 1979-1994, close-to-close, no costs): run on the Norgate T-bond continuous contract only (R4 #293 (1))";
const JMCT: &str = "John L. Momsen, 'Trade Crude Now', Technical Analysis of Stocks & Commodities V.19:8 (2001) p.53-56, sidebar 'Crude oil seasonal trade #2' (TradeStation code; November crude oil futures 1983-1999): run on the Norgate crude-oil continuous contract only (R4 #293 (1))";
const CARDS_PA2: &[(&str, &str, &str)] = &[("scsbqs_tuesday", "&ZB", SBQS), ("scsbqs_month_end", "&ZB", SBQS), ("scsbqs_april15", "&ZB", SBQS), ("scjmct_crude2", "&CL", JMCT), ("scasrb_breakout", "&ZB", ASRB)];
const ASRB: &str = "Alex Saitta, 'Range Breakout Trading in Treasury Bonds', Technical Analysis of Stocks & Commodities V.13:4 (1995) p.149-152 (Figure 4: one T-bond contract 1984-1994, closes, no costs; Morris's MetaStock transcription msws_gm9_range_breakout is held as NotTestable): run on the Norgate T-bond continuous contract only (R4 #293 (1))";
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
    #[test] fn non_usd_contract_has_no_usd_value() { let m = super::MARKETS.iter().find(|m| m.sym == "&YAP").unwrap(); assert!(super::usd_per_point(m, "2000-01-03").is_none()); } // review PF2 M20
    #[test] fn zb_open_label() { // R4 #331: every listed id is a live card; the label is appended only to them
        let ids: Vec<&str> = crate::books::catalogue().iter().map(|d| d.id).collect();
        for c in super::ZB_OPEN_CARDS.iter().chain(super::ZN_OPEN_CARDS).chain(super::ZC_OPEN_CARDS) { assert!(ids.contains(&c.0), "{}", c.0); }
        let two = super::zb_open_notes("andro_andromeda", "x"); // a card on several contracts carries each label
        assert!(two.contains("this card's &ZN trades") && two.contains("this card's &ZC trades"));
        assert!(super::zb_open_notes("mhs_us800310_1", "x").starts_with("x On Norgate &ZB, from 1999-12 to 2000-10") && super::zb_open_notes("mhs_us800310_1", "x").contains("3 of this card's"));
        assert_eq!(super::zb_open_notes("dmadr_daily", "x"), "x");
    }
    #[test] fn market_symbols_unique() { let mut v: Vec<&str> = super::MARKETS.iter().map(|m| m.sym).collect(); let n = v.len(); v.sort(); v.dedup(); assert_eq!(v.len(), n, "duplicate MARKETS symbol (lane merge)"); } // lane PF (review PF2 M4)
    use super::*;
    /// Lane PE (R4 #319): $NYA loads as its own context from Norgate's compact dates (volume 0 = none); only Holt's cards run on it.
    #[test]
    fn nya_context_loads_compact_dates_and_admits_only_holt() {
        let dir = std::env::temp_dir().join(format!("pe-nya-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("$NYA.csv"), "\"Date\",\"Symbol\",\"Security Name\",\"Open\",\"High\",\"Low\",\"Close\",\"Volume\"\n\"19800102\",\"$NYA\",\"NYSE Composite Index\",\"61.6\",\"61.6\",\"61.6\",\"61.6\",\"0\"\n\"19800103\",\"$NYA\",\"NYSE Composite Index\",\"61.2\",\"61.2\",\"61.2\",\"61.2\",\"50480000\"\n").unwrap();
        let mut data = BTreeMap::new();
        load_nya(&dir, &mut data);
        let r = &data[NYA_TICKER];
        assert_eq!((r.len(), r[0].date.as_str(), r[0].volume, r[1].volume), (2, "1980-01-02", None, Some(50480000.)));
        let _ = std::fs::remove_dir_all(&dir);
        let cat = crate::books::scholt::catalogue();
        assert!(crate::instrument_policy::check(&cat[0], NYA_TICKER, false).is_none());
        let other = crate::books::scweiss::catalogue();
        assert!(crate::instrument_policy::check(&other[0], NYA_TICKER, false).is_some_and(|i| i.note.contains("Holt")));
    }
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
    #[test]
    fn x4_ehrlich17_run_ends_before_a_nonpositive_unadjusted_price() {
        // R4 #316: the &CL series stays whole; only jkbwts_ehrlich17's run is capped (auto_end) / refused (check)
        let r: Vec<Row> = (0..10).map(|i| Row { ticker: "Crude Oil (Norgate &CL)".into(), date: format!("2020-04-{:02}", i + 10), open: 20., high: 20., low: 20., close: 20., volume: None }).collect();
        let mut x = Ctx::new(&r, &[], &[]);
        for k in ["RAW.Open", "RAW.High", "RAW.Low"] { x.extra.insert(k.into(), vec![30.; 10]); }
        let mut c = vec![30.; 10]; c[6] = -37.63; x.extra.insert("RAW.Close".into(), c);
        assert_eq!(first_nonpositive("jkbwts_ehrlich17", &x), Some(6));
        assert_eq!(first_nonpositive("mhs_cl850210_1", &x), None);
        let d = crate::books::jkbwts::catalogue().into_iter().find(|d| d.id == "jkbwts_ehrlich17").unwrap();
        let p = crate::connors::Params::new();
        let (end, note) = crate::readiness::auto_end(&x, &d, 0, 9, &p);
        assert!(end == 5 && note.is_some_and(|n| n.contains("2020-04-16") && n.contains("R4 #316")));
        assert!(crate::readiness::check(&x, &d, 0, 5, &p).map_or(true, |i| !i.note.contains("R4 #316")));
        assert!(crate::readiness::check(&x, &d, 6, 9, &p).is_some_and(|i| i.note.contains("R4 #316")));
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
    fn f1c_rows(t: &str, bars: &[(f64, f64, f64, f64)]) -> Vec<Row> {
        bars.iter().enumerate().map(|(i, &(o, h, l, c))| Row { ticker: t.into(), date: format!("1990-{:02}-{:02}", 1 + i / 28, 1 + i % 28), open: o, high: h, low: l, close: c, volume: None }).collect()
    }
    fn f1c_bond(closes: &[f64]) -> &'static [Row] {
        let b: Vec<_> = closes.iter().map(|&c| (c, c, c, c)).collect();
        Box::leak(f1c_rows("US T-Bond (Norgate &ZB)", &b).into_boxed_slice())
    }
    #[test]
    fn f1c_miser_entry_bar_stop_and_data2_refusal() {
        // REV-F1C b5ac727b M2: S&P below its 14-day SMA while bonds close above their 40-day SMA -> buy the next open; the
        // Trailing LX "ATR eb" stop (signal bar's high - 5 x ATR(10)) protects that entry bar
        let mut sp = vec![(100., 100.5, 99.5, 100.); 59];
        sp.extend([(99.8, 100.3, 98.9, 99.), (99., 99.5, 90., 92.), (92., 92.5, 91.5, 92.)]);
        let r = f1c_rows("S&P 500 (Norgate &SP)", &sp);
        let x = Ctx::new(&r, &r, &[]);
        assert!(data2_issue("stad11_miser", &x).is_some() && data2_issue("ch_bta2_cbo_ref", &x).is_none()); // no T-bond loaded: refused
        let mut b = vec![100.; 62]; b[59] = 101.;
        TEST_ROWS.with(|t| t.borrow_mut().insert("&ZB", f1c_bond(&b)));
        assert!(data2_issue("stad11_miser", &x).is_none());
        let t = crate::books::stad2::run("stad11_miser", &x, 0, 61, &crate::connors::Params::new()).unwrap();
        let atr = (9. * 1. + 1.4) / 10.; // true ranges 1.0 x 9 and 1.4 on the signal bar
        assert_eq!((t.len(), t[0].legs[0].i, t[0].exit_i), (1, 60, 60), "{t:?}");
        assert!((t[0].legs[0].px - 99.).abs() < 1e-9 && (t[0].exit_px - (100.3 - 5. * atr)).abs() < 1e-9 && t[0].reason.contains("eb"), "{t:?}");
        TEST_ROWS.with(|t| t.borrow_mut().clear());
    }
    #[test]
    fn f1c_curbond_entry_stop_lives_five_bars() {
        // a setup only on bar 40 (bond XAverage rises that day only): its buy stop is sent for bars 41-45, not 46
        let franc = |jump: usize| -> Vec<Row> {
            let bars: Vec<_> = (0..60).map(|i| { let c = 1. + 0.00005 * i as f64; (c, c + if i == jump { 0.01 } else { 0.0005 }, c - 0.0005, c) }).collect();
            f1c_rows("Swiss Franc (Norgate &6S)", &bars)
        };
        let mut b = vec![100.; 60]; b[40] = 101.;
        TEST_ROWS.with(|t| t.borrow_mut().insert("&ZB", f1c_bond(&b)));
        let run = |r: &Vec<Row>| crate::books::stad2::run("stad5_curbond", &Ctx::new(r, r, &[]), 20, 59, &crate::connors::Params::new()).unwrap();
        let (a, z) = (franc(45), franc(46));
        let ta = run(&a);
        assert!(ta.len() == 1 && ta[0].legs[0].i == 45 && ta[0].dir > 0., "{ta:?}");
        assert!(run(&z).is_empty());
        TEST_ROWS.with(|t| t.borrow_mut().clear());
    }
    #[test]
    fn f1d_tdm12_sell_gold_filter_reads_gold_bars() {
        // TDM 12 = bar 11 (the month starts on the 1st); gold's close on bar 10 is above its close 10 gold bars earlier -> sell
        // the open of bar 11, out on the close of bar 14; no gold loaded -> no trade (readiness refuses: data2_issue)
        let r = f1c_rows("US T-Bond (Norgate &ZB)", &vec![(100., 101., 99., 100.); 20]);
        let run = || crate::books::lwlts::run("lwlts_tdm12_sell_gold_bonds", &Ctx::new(&r, &r, &[]), 0, 19, &crate::connors::Params::new()).unwrap();
        assert!(run().is_empty());
        let gold: Vec<_> = (0..20).map(|k| (300. + k as f64, 300. + k as f64, 300. + k as f64, 300. + k as f64)).collect();
        TEST_ROWS.with(|t| t.borrow_mut().insert("&GC", Box::leak(f1c_rows("Gold (Norgate &GC)", &gold).into_boxed_slice())));
        let t = run();
        assert_eq!((t.len(), t[0].dir, t[0].legs[0].i, t[0].exit_i), (1, -1., 11, 14), "{t:?}");
        TEST_ROWS.with(|t| t.borrow_mut().clear());
    }
    #[test]
    fn f1d_bond_channel_buys_the_sp_close_and_exits_on_the_bond_low() {
        // the bond closes above its 14 prior highs on bar 15 -> buy the S&P close; its low breaks the 17 prior lows on bar 20 ->
        // out at that S&P close (Fig. 15.1); no bond data -> no trade
        let sp = f1c_rows("S&P 500 (Norgate &SP)", &vec![(400., 401., 399., 400.); 24]);
        let run = || crate::books::lwlts::run("lwlts_bonds_channel_bstop", &Ctx::new(&sp, &sp, &[]), 0, 23, &crate::connors::Params::new()).unwrap();
        assert!(run().is_empty());
        let mut b = vec![(100., 100.5, 99.5, 100.); 24];
        b[15] = (100., 101., 100., 101.); b[20] = (100., 100., 98., 98.5);
        TEST_ROWS.with(|t| t.borrow_mut().insert("&ZB", Box::leak(f1c_rows("US T-Bond (Norgate &ZB)", &b).into_boxed_slice())));
        let t = run();
        assert_eq!((t.len(), t[0].legs[0].i, t[0].exit_i, t[0].entry_kind), (1, 15, 20, "close"), "{t:?}");
        assert!(crate::books::mhstp::window("lwlts_bonds_channel_bstop").is_some_and(|w| w.1 == "1995-07-31"));
        TEST_ROWS.with(|t| t.borrow_mut().clear());
    }
    #[test]
    fn f1d_bond_cards_bail_delay_and_nearer_stop() {
        // 1990-01-03 is a Wednesday: it opens above Tuesday's high 101 and falls through it -> Oops! sell at 101; the "4-day
        // bailout" waits for the opening 4 bars after entry although bar 3 already opens profitable
        let mut b = vec![(100., 100.5, 99.5, 100.), (100., 101., 99.5, 100.5), (101.5, 101.5, 100.5, 100.7)];
        b.extend([(100.8, 101.2, 100.6, 100.8); 5]);
        let r = f1c_rows("US T-Bond (Norgate &ZB)", &b);
        let x = Ctx::new(&r, &r, &[]);
        let t = crate::books::lwlts::run("lwlts_oops_sell_bonds_wed", &x, 0, 7, &crate::connors::Params::new()).unwrap();
        assert_eq!((t.len(), t[0].legs[0].i, t[0].legs[0].px, t[0].exit_i, t[0].exit_px), (1, 2, 101., 6, 100.8), "{t:?}");
        // "$1,500 or 50 percent of the previous day's range": range 1.0 -> the nearer 0.5 stop (101.5 - 0.5 = 101) is hit on the
        // way to the close 100.9 after the buy stop 101.5 fills (open 100.5 nearer the low: O -> L -> H -> C)
        let r = f1c_rows("US T-Bond (Norgate &ZB)", &[(100., 101., 100., 100.5), (100.5, 102., 100.4, 100.9)]);
        let t = crate::books::lwlts::run("lwlts_vb_bonds", &Ctx::new(&r, &r, &[]), 0, 1, &crate::connors::Params::new()).unwrap();
        assert_eq!((t.len(), t[0].legs[0].px, t[0].exit_i, t[0].exit_px), (1, 101.5, 1, 101.), "{t:?}");
        assert!(held("lwlts_vb_bonds", "US T-Bond (Norgate &ZB)").is_some() && held("lwlts_oops_buy_bonds", "US T-Bond (Norgate &ZB)").is_none());
    }
    #[test]
    fn f1d_bonds_8day_close_entry_stop_reentry_target() {
        // close 101 > close 8 bars ago -> long on that close; $850 = 0.85 stop hit on bar 11 at 100.15, the same close (100.2 >
        // 100 eight bars back) re-enters; $3,500 = 3.5 target 103.7 hit on bar 12
        let mut b = vec![(100., 100., 100., 100.); 9];
        b.extend([(100., 101., 100., 101.), (101., 101., 100.5, 100.6), (100.6, 100.6, 100.1, 100.2), (100.3, 103.8, 100.3, 103.)]);
        let r = f1c_rows("US T-Bond (Norgate &ZB)", &b);
        let t = crate::books::lwlts::run("lwlts_bonds_8day", &Ctx::new(&r, &r, &[]), 0, 12, &crate::connors::Params::new()).unwrap();
        let v: Vec<_> = t.iter().map(|t| (t.legs[0].i, t.legs[0].px, t.exit_i, (t.exit_px * 100.).round() / 100., t.entry_kind)).collect();
        assert_eq!(v, vec![(9, 101., 11, 100.15, "close"), (11, 100.2, 12, 103.7, "close"), (12, 103., 12, 103., "close")], "{t:?}");
        assert!(t[2].open); // bar 12 closes above 8 bars back: re-entered on the target bar's close
        let sp: Vec<Row> = r.iter().map(|b| Row { ticker: "SPX".into(), ..b.clone() }).collect();
        assert!(crate::books::lwlts::run("lwlts_bonds_8day", &Ctx::new(&sp, &sp, &[]), 0, 12, &crate::connors::Params::new()).is_some_and(|t| t.is_empty()));
        assert!(home("lwlts_bonds_8day", "US T-Bond (Norgate &ZB)").is_none() && home("lwlts_bonds_8day", "SPX").is_some());
    }
    #[test]
    fn f1c_tdm_bonds_on_the_contract_only() {
        // REV-F1C 81cb0080 M2: TDM 8 = the 8th bar of a month that starts on the 1st; out at the close of the third day after
        let r = f1c_rows("US T-Bond (Norgate &ZB)", &vec![(100., 101., 99., 100.); 28]);
        let x = Ctx::new(&r, &r, &[]);
        let t = crate::books::lwlts::run("lwlts_tdm8_bonds", &x, 0, 27, &crate::connors::Params::new()).unwrap();
        assert_eq!((t.len(), t[0].legs[0].i, t[0].exit_i), (1, 7, 10), "{t:?}");
        let sp: Vec<Row> = r.iter().map(|b| Row { ticker: "SPX".into(), ..b.clone() }).collect();
        assert!(crate::books::lwlts::run("lwlts_tdm22_bonds", &Ctx::new(&sp, &sp, &[]), 0, 27, &crate::connors::Params::new()).is_some_and(|t| t.is_empty()));
        assert!(home("lwlts_tdm18_bonds", "US T-Bond (Norgate &ZB)").is_none() && home("lwlts_tdm18_bonds", "SPX").is_some());
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
