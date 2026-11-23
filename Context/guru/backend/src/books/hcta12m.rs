//! Jeffrey A. Hirsch & John L. Person, Commodity Trader's Almanac 2012 (Wiley; research drive Seasonality Top Reads/13-Commodity-Trader-s-Almanac-2012/
//! Reading.epub, OEBPS/9781118168691_c01..c12.htm): the 39 printed seasonal trade boxes other than the January S&P short already coded in `hcta12`
//! ("MONTH LONG|SHORT MARKET (CONTRACT) TRADING DAY: N-HOLD: D DAYS"). Book id `hcta12m`. Lane K3 (24 Nov 2026). Each card runs only on the
//! Norgate continuous contract of its own market (futures.rs cards). Independent check authors/conv-1123/k3w/hcta12m_ref.py.
use crate::connors::{Ctx, Leg, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "hcta12m";
const NOTE: &str = "[Printed trade box; unverified] Readings (labelled, 3): (1) 'on or about' the Nth trading day = exactly the Nth session of that calendar month in the continuous contract's own bars, at its close; (2) the HOLD of D days = covered at the close D sessions after the entry session; (3) the box names one contract month ('MARCH', 'JULY', ...) and the Norgate continuous contract (nearby, back-adjusted) stands in for it, a labelled adaptation, so a trade can cross a roll. One trade per year; no stop. A trade still open on the last bar is marked at its close.";

/// (id, month, direction, trading day, hold)
pub const ROWS: &[(&str, u32, f64, usize, usize)] = &[
    ("hcta12m_01_s_6e_3", 1, -1.0, 3, 24),
    ("hcta12m_01_s_zw_3", 1, -1.0, 3, 85),
    ("hcta12m_01_l_sp_15", 1, 1.0, 15, 7),
    ("hcta12m_02_l_zs_9", 2, 1.0, 9, 73),
    ("hcta12m_02_l_cl_10", 2, 1.0, 10, 60),
    ("hcta12m_02_s_si_13", 2, -1.0, 13, 45),
    ("hcta12m_02_l_ng_16", 2, 1.0, 16, 41),
    ("hcta12m_03_s_cc_10", 3, -1.0, 10, 23),
    ("hcta12m_03_l_6b_15", 3, 1.0, 15, 22),
    ("hcta12m_04_l_zb_18", 4, 1.0, 18, 81),
    ("hcta12m_05_s_le_2", 5, -1.0, 2, 30),
    ("hcta12m_05_s_hg_8", 5, -1.0, 8, 13),
    ("hcta12m_05_s_si_10", 5, -1.0, 10, 29),
    ("hcta12m_05_s_kc_16", 5, -1.0, 16, 54),
    ("hcta12m_06_s_zs_5", 6, -1.0, 5, 36),
    ("hcta12m_06_l_sb_11", 6, 1.0, 11, 32),
    ("hcta12m_06_l_le_14", 6, 1.0, 14, 160),
    ("hcta12m_06_s_zc_18", 6, -1.0, 18, 25),
    ("hcta12m_07_s_sp_10", 7, -1.0, 10, 7),
    ("hcta12m_07_l_ng_17", 7, 1.0, 17, 62),
    ("hcta12m_08_l_6s_6", 8, 1.0, 6, 48),
    ("hcta12m_08_l_kc_12", 8, 1.0, 12, 13),
    ("hcta12m_07_s_cc_18", 7, -1.0, 18, 70),
    ("hcta12m_08_l_cc_14", 8, 1.0, 14, 24),
    ("hcta12m_08_l_gc_18", 8, 1.0, 18, 25),
    ("hcta12m_09_s_cl_8", 9, -1.0, 8, 62),
    ("hcta12m_09_l_6b_12", 9, 1.0, 12, 33),
    ("hcta12m_10_s_si_4", 10, -1.0, 4, 17),
    ("hcta12m_10_s_6j_12", 10, -1.0, 12, 78),
    ("hcta12m_10_l_zs_16", 10, 1.0, 16, 12),
    ("hcta12m_10_l_6e_18", 10, 1.0, 18, 47),
    ("hcta12m_10_l_sp_19", 10, 1.0, 19, 41),
    ("hcta12m_10_l_zc_20", 10, 1.0, 20, 133),
    ("hcta12m_11_l_he_2", 11, 1.0, 2, 13),
    ("hcta12m_11_l_gc_13", 11, 1.0, 13, 10),
    ("hcta12m_11_s_zb_14", 11, -1.0, 14, 107),
    ("hcta12m_12_l_zw_5", 12, 1.0, 5, 20),
    ("hcta12m_12_s_6b_19", 12, -1.0, 19, 56),
    ("hcta12m_12_s_6s_19", 12, -1.0, 19, 41),
];
pub fn row(id: &str) -> Option<&'static (&'static str, u32, f64, usize, usize)> { ROWS.iter().find(|r| r.0 == id) }
pub fn reviewed(id: &str) -> bool { row(id).is_some() }

pub fn catalogue() -> Vec<StrategyDef> {
    vec![
        StrategyDef { id: "hcta12m_01_s_6e_3", name: "January short Euro: enter on trading day 3, hold 24 days", group: "Commodity Trader's Almanac 2012 · seasonal trades", session: 0, dir: "Short", status: Status::Live, needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK,
            rules: &["E|\"JANUARY SHORT EURO (MARCH) TRADING DAY: 3-HOLD: 24 DAYS\" (ch.1); the trade is taken on the 3th trading day of January at the close", "X|held 24 trading days, covered at that close (the box's HOLD)"], notes: NOTE, claim: None },
        StrategyDef { id: "hcta12m_01_s_zw_3", name: "January short wheat: enter on trading day 3, hold 85 days", group: "Commodity Trader's Almanac 2012 · seasonal trades", session: 0, dir: "Short", status: Status::Live, needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK,
            rules: &["E|\"JANUARY SHORT WHEAT (JULY) TRADING DAY: 3-HOLD: 85 DAYS\" (ch.1); the trade is taken on the 3th trading day of January at the close", "X|held 85 trading days, covered at that close (the box's HOLD)"], notes: NOTE, claim: None },
        StrategyDef { id: "hcta12m_01_l_sp_15", name: "January long S&P 500: enter on trading day 15, hold 7 days", group: "Commodity Trader's Almanac 2012 · seasonal trades", session: 0, dir: "Long", status: Status::Live, needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK,
            rules: &["E|\"JANUARY LONG S&P 500 (MARCH) TRADING DAY: 15-HOLD: 7 DAYS\" (ch.2); the trade is taken on the 15th trading day of January at the close", "X|held 7 trading days, covered at that close (the box's HOLD)"], notes: NOTE, claim: None },
        StrategyDef { id: "hcta12m_02_l_zs_9", name: "February long soybeans: enter on trading day 9, hold 73 days", group: "Commodity Trader's Almanac 2012 · seasonal trades", session: 0, dir: "Long", status: Status::Live, needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK,
            rules: &["E|\"FEBRUARY LONG SOYBEANS (JULY) TRADING DAY: 9-HOLD: 73 DAYS\" (ch.2); the trade is taken on the 9th trading day of February at the close", "X|held 73 trading days, covered at that close (the box's HOLD)"], notes: NOTE, claim: None },
        StrategyDef { id: "hcta12m_02_l_cl_10", name: "February long crude oil: enter on trading day 10, hold 60 days", group: "Commodity Trader's Almanac 2012 · seasonal trades", session: 0, dir: "Long", status: Status::Live, needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK,
            rules: &["E|\"FEBRUARY LONG CRUDE OIL (JULY) TRADING DAY: 10-HOLD: 60 DAYS\" (ch.2); the trade is taken on the 10th trading day of February at the close", "X|held 60 trading days, covered at that close (the box's HOLD)"], notes: NOTE, claim: None },
        StrategyDef { id: "hcta12m_02_s_si_13", name: "February short silver: enter on trading day 13, hold 45 days", group: "Commodity Trader's Almanac 2012 · seasonal trades", session: 0, dir: "Short", status: Status::Live, needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK,
            rules: &["E|\"FEBRUARY SHORT SILVER (MAY) TRADING DAY: 13-HOLD: 45 DAYS\" (ch.3); the trade is taken on the 13th trading day of February at the close", "X|held 45 trading days, covered at that close (the box's HOLD)"], notes: NOTE, claim: None },
        StrategyDef { id: "hcta12m_02_l_ng_16", name: "February long natural gas: enter on trading day 16, hold 41 days", group: "Commodity Trader's Almanac 2012 · seasonal trades", session: 0, dir: "Long", status: Status::Live, needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK,
            rules: &["E|\"FEBRUARY LONG NATURAL GAS (JULY) TRADING DAY: 16-HOLD: 41 DAYS\" (ch.3); the trade is taken on the 16th trading day of February at the close", "X|held 41 trading days, covered at that close (the box's HOLD)"], notes: NOTE, claim: None },
        StrategyDef { id: "hcta12m_03_s_cc_10", name: "March short cocoa: enter on trading day 10, hold 23 days", group: "Commodity Trader's Almanac 2012 · seasonal trades", session: 0, dir: "Short", status: Status::Live, needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK,
            rules: &["E|\"MARCH SHORT COCOA TRADING (JULY) TRADING DAY: 10-HOLD: 23 DAYS\" (ch.3); the trade is taken on the 10th trading day of March at the close", "X|held 23 trading days, covered at that close (the box's HOLD)"], notes: NOTE, claim: None },
        StrategyDef { id: "hcta12m_03_l_6b_15", name: "March long British pound: enter on trading day 15, hold 22 days", group: "Commodity Trader's Almanac 2012 · seasonal trades", session: 0, dir: "Long", status: Status::Live, needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK,
            rules: &["E|\"MARCH LONG BRITISH POUND (JUNE) TRADING DAY: 15-HOLD: 22 DAYS\" (ch.3); the trade is taken on the 15th trading day of March at the close", "X|held 22 trading days, covered at that close (the box's HOLD)"], notes: NOTE, claim: None },
        StrategyDef { id: "hcta12m_04_l_zb_18", name: "April long 30-year bond: enter on trading day 18, hold 81 days", group: "Commodity Trader's Almanac 2012 · seasonal trades", session: 0, dir: "Long", status: Status::Live, needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK,
            rules: &["E|\"APRIL LONG 30-YR BOND (SEPTEMBER) TRADING DAY: 18-HOLD: 81 DAYS\" (ch.4); the trade is taken on the 18th trading day of April at the close", "X|held 81 trading days, covered at that close (the box's HOLD)"], notes: NOTE, claim: None },
        StrategyDef { id: "hcta12m_05_s_le_2", name: "May short live cattle: enter on trading day 2, hold 30 days", group: "Commodity Trader's Almanac 2012 · seasonal trades", session: 0, dir: "Short", status: Status::Live, needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK,
            rules: &["E|\"MAY SHORT LIVE CATTLE (OCTOBER) TRADING DAY: 2-HOLD: 30 DAYS\" (ch.4); the trade is taken on the 2th trading day of May at the close", "X|held 30 trading days, covered at that close (the box's HOLD)"], notes: NOTE, claim: None },
        StrategyDef { id: "hcta12m_05_s_hg_8", name: "May short copper: enter on trading day 8, hold 13 days", group: "Commodity Trader's Almanac 2012 · seasonal trades", session: 0, dir: "Short", status: Status::Live, needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK,
            rules: &["E|\"MAY SHORT COPPER (JULY) TRADING DAY: 8-HOLD: 13 DAYS\" (ch.5); the trade is taken on the 8th trading day of May at the close", "X|held 13 trading days, covered at that close (the box's HOLD)"], notes: NOTE, claim: None },
        StrategyDef { id: "hcta12m_05_s_si_10", name: "May short silver: enter on trading day 10, hold 29 days", group: "Commodity Trader's Almanac 2012 · seasonal trades", session: 0, dir: "Short", status: Status::Live, needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK,
            rules: &["E|\"MAY SHORT SILVER (JULY) TRADING DAY: 10-HOLD: 29 DAYS\" (ch.5); the trade is taken on the 10th trading day of May at the close", "X|held 29 trading days, covered at that close (the box's HOLD)"], notes: NOTE, claim: None },
        StrategyDef { id: "hcta12m_05_s_kc_16", name: "May short coffee: enter on trading day 16, hold 54 days", group: "Commodity Trader's Almanac 2012 · seasonal trades", session: 0, dir: "Short", status: Status::Live, needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK,
            rules: &["E|\"MAY SHORT COFFEE (SEPTEMBER) TRADING DAY: 16-HOLD: 54 DAYS\" (ch.5); the trade is taken on the 16th trading day of May at the close", "X|held 54 trading days, covered at that close (the box's HOLD)"], notes: NOTE, claim: None },
        StrategyDef { id: "hcta12m_06_s_zs_5", name: "June short soybeans: enter on trading day 5, hold 36 days", group: "Commodity Trader's Almanac 2012 · seasonal trades", session: 0, dir: "Short", status: Status::Live, needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK,
            rules: &["E|\"JUNE SHORT SOYBEANS (SEPTEMBER) TRADING DAY: 5-HOLD: 36 DAYS\" (ch.6); the trade is taken on the 5th trading day of June at the close", "X|held 36 trading days, covered at that close (the box's HOLD)"], notes: NOTE, claim: None },
        StrategyDef { id: "hcta12m_06_l_sb_11", name: "June long sugar: enter on trading day 11, hold 32 days", group: "Commodity Trader's Almanac 2012 · seasonal trades", session: 0, dir: "Long", status: Status::Live, needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK,
            rules: &["E|\"JUNE LONG SUGAR (OCTOBER) TRADING DAY: 11-HOLD: 32 DAYS\" (ch.6); the trade is taken on the 11th trading day of June at the close", "X|held 32 trading days, covered at that close (the box's HOLD)"], notes: NOTE, claim: None },
        StrategyDef { id: "hcta12m_06_l_le_14", name: "June long live cattle: enter on trading day 14, hold 160 days", group: "Commodity Trader's Almanac 2012 · seasonal trades", session: 0, dir: "Long", status: Status::Live, needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK,
            rules: &["E|\"JUNE LONG LIVE CATTLE (APRIL) TRADING DAY: 14-HOLD: 160 DAYS\" (ch.6); the trade is taken on the 14th trading day of June at the close", "X|held 160 trading days, covered at that close (the box's HOLD)"], notes: NOTE, claim: None },
        StrategyDef { id: "hcta12m_06_s_zc_18", name: "June short corn: enter on trading day 18, hold 25 days", group: "Commodity Trader's Almanac 2012 · seasonal trades", session: 0, dir: "Short", status: Status::Live, needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK,
            rules: &["E|\"JUNE SHORT CORN (SEPTEMBER) TRADING DAY: 18-HOLD: 25 DAYS\" (ch.6); the trade is taken on the 18th trading day of June at the close", "X|held 25 trading days, covered at that close (the box's HOLD)"], notes: NOTE, claim: None },
        StrategyDef { id: "hcta12m_07_s_sp_10", name: "July short S&P 500: enter on trading day 10, hold 7 days", group: "Commodity Trader's Almanac 2012 · seasonal trades", session: 0, dir: "Short", status: Status::Live, needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK,
            rules: &["E|\"JULY SHORT S&P 500 (SEPTEMBER) TRADING DAY: 10-HOLD: 7 DAYS\" (ch.7); the trade is taken on the 10th trading day of July at the close", "X|held 7 trading days, covered at that close (the box's HOLD)"], notes: NOTE, claim: None },
        StrategyDef { id: "hcta12m_07_l_ng_17", name: "July long natural gas: enter on trading day 17, hold 62 days", group: "Commodity Trader's Almanac 2012 · seasonal trades", session: 0, dir: "Long", status: Status::Live, needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK,
            rules: &["E|\"JULY LONG NATURAL GAS (NOVEMBER) TRADING DAY: 17-HOLD: 62 DAYS\" (ch.7); the trade is taken on the 17th trading day of July at the close", "X|held 62 trading days, covered at that close (the box's HOLD)"], notes: NOTE, claim: None },
        StrategyDef { id: "hcta12m_08_l_6s_6", name: "August long Swiss franc: enter on trading day 6, hold 48 days", group: "Commodity Trader's Almanac 2012 · seasonal trades", session: 0, dir: "Long", status: Status::Live, needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK,
            rules: &["E|\"AUGUST LONG SWISS FRANC (DECEMBER) TRADING DAY: 6-HOLD: 48 DAYS\" (ch.7); the trade is taken on the 6th trading day of August at the close", "X|held 48 trading days, covered at that close (the box's HOLD)"], notes: NOTE, claim: None },
        StrategyDef { id: "hcta12m_08_l_kc_12", name: "August long coffee: enter on trading day 12, hold 13 days", group: "Commodity Trader's Almanac 2012 · seasonal trades", session: 0, dir: "Long", status: Status::Live, needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK,
            rules: &["E|\"AUGUST LONG COFFEE (DECEMBER) TRADING DAY: 12-HOLD: 13 DAYS\" (ch.8); the trade is taken on the 12th trading day of August at the close", "X|held 13 trading days, covered at that close (the box's HOLD)"], notes: NOTE, claim: None },
        StrategyDef { id: "hcta12m_07_s_cc_18", name: "July short cocoa: enter on trading day 18, hold 70 days", group: "Commodity Trader's Almanac 2012 · seasonal trades", session: 0, dir: "Short", status: Status::Live, needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK,
            rules: &["E|\"JULY SHORT COCOA (DECEMBER) TRADING DAY: 18-HOLD: 70 DAYS\" (ch.8); the trade is taken on the 18th trading day of July at the close", "X|held 70 trading days, covered at that close (the box's HOLD)"], notes: NOTE, claim: None },
        StrategyDef { id: "hcta12m_08_l_cc_14", name: "August long cocoa: enter on trading day 14, hold 24 days", group: "Commodity Trader's Almanac 2012 · seasonal trades", session: 0, dir: "Long", status: Status::Live, needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK,
            rules: &["E|\"AUGUST LONG COCOA (DECEMBER) TRADING DAY: 14-HOLD: 24 DAYS\" (ch.8); the trade is taken on the 14th trading day of August at the close", "X|held 24 trading days, covered at that close (the box's HOLD)"], notes: NOTE, claim: None },
        StrategyDef { id: "hcta12m_08_l_gc_18", name: "August long gold: enter on trading day 18, hold 25 days", group: "Commodity Trader's Almanac 2012 · seasonal trades", session: 0, dir: "Long", status: Status::Live, needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK,
            rules: &["E|\"AUGUST LONG GOLD (DECEMBER) TRADING DAY: 18-HOLD: 25 DAYS\" (ch.8); the trade is taken on the 18th trading day of August at the close", "X|held 25 trading days, covered at that close (the box's HOLD)"], notes: NOTE, claim: None },
        StrategyDef { id: "hcta12m_09_s_cl_8", name: "September short crude oil: enter on trading day 8, hold 62 days", group: "Commodity Trader's Almanac 2012 · seasonal trades", session: 0, dir: "Short", status: Status::Live, needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK,
            rules: &["E|\"SEPTEMBER SHORT CRUDE OIL (FEBRUARY) TRADING DAY: 8-HOLD: 62 DAYS\" (ch.9); the trade is taken on the 8th trading day of September at the close", "X|held 62 trading days, covered at that close (the box's HOLD)"], notes: NOTE, claim: None },
        StrategyDef { id: "hcta12m_09_l_6b_12", name: "September long British pound: enter on trading day 12, hold 33 days", group: "Commodity Trader's Almanac 2012 · seasonal trades", session: 0, dir: "Long", status: Status::Live, needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK,
            rules: &["E|\"SEPTEMBER LONG BRITISH POUND (DECEMBER) TRADING DAY: 12-HOLD: 33 DAYS\" (ch.9); the trade is taken on the 12th trading day of September at the close", "X|held 33 trading days, covered at that close (the box's HOLD)"], notes: NOTE, claim: None },
        StrategyDef { id: "hcta12m_10_s_si_4", name: "October short silver: enter on trading day 4, hold 17 days", group: "Commodity Trader's Almanac 2012 · seasonal trades", session: 0, dir: "Short", status: Status::Live, needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK,
            rules: &["E|\"OCTOBER SHORT SILVER (DECEMBER) TRADING DAY: 4-HOLD: 17 DAYS\" (ch.9); the trade is taken on the 4th trading day of October at the close", "X|held 17 trading days, covered at that close (the box's HOLD)"], notes: NOTE, claim: None },
        StrategyDef { id: "hcta12m_10_s_6j_12", name: "October short yen: enter on trading day 12, hold 78 days", group: "Commodity Trader's Almanac 2012 · seasonal trades", session: 0, dir: "Short", status: Status::Live, needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK,
            rules: &["E|\"OCTOBER SHORT YEN (MARCH) TRADING DAY: 12-HOLD: 78 DAYS\" (ch.9); the trade is taken on the 12th trading day of October at the close", "X|held 78 trading days, covered at that close (the box's HOLD)"], notes: NOTE, claim: None },
        StrategyDef { id: "hcta12m_10_l_zs_16", name: "October long soybeans: enter on trading day 16, hold 12 days", group: "Commodity Trader's Almanac 2012 · seasonal trades", session: 0, dir: "Long", status: Status::Live, needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK,
            rules: &["E|\"OCTOBER LONG SOYBEANS (JANUARY) TRADING DAY: 16-HOLD: 12 DAYS\" (ch.10); the trade is taken on the 16th trading day of October at the close", "X|held 12 trading days, covered at that close (the box's HOLD)"], notes: NOTE, claim: None },
        StrategyDef { id: "hcta12m_10_l_6e_18", name: "October long Euro: enter on trading day 18, hold 47 days", group: "Commodity Trader's Almanac 2012 · seasonal trades", session: 0, dir: "Long", status: Status::Live, needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK,
            rules: &["E|\"OCTOBER LONG EURO (MARCH) TRADING DAY: 18-HOLD: 47 DAYS\" (ch.10); the trade is taken on the 18th trading day of October at the close", "X|held 47 trading days, covered at that close (the box's HOLD)"], notes: NOTE, claim: None },
        StrategyDef { id: "hcta12m_10_l_sp_19", name: "October long S&P 500: enter on trading day 19, hold 41 days", group: "Commodity Trader's Almanac 2012 · seasonal trades", session: 0, dir: "Long", status: Status::Live, needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK,
            rules: &["E|\"OCTOBER LONG S&P 500 (MARCH) TRADING DAY: 19-HOLD: 41 DAYS\" (ch.10); the trade is taken on the 19th trading day of October at the close", "X|held 41 trading days, covered at that close (the box's HOLD)"], notes: NOTE, claim: None },
        StrategyDef { id: "hcta12m_10_l_zc_20", name: "October long corn: enter on trading day 20, hold 133 days", group: "Commodity Trader's Almanac 2012 · seasonal trades", session: 0, dir: "Long", status: Status::Live, needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK,
            rules: &["E|\"OCTOBER LONG CORN (JULY) TRADING DAY: 20-HOLD: 133 DAYS\" (ch.11); the trade is taken on the 20th trading day of October at the close", "X|held 133 trading days, covered at that close (the box's HOLD)"], notes: NOTE, claim: None },
        StrategyDef { id: "hcta12m_11_l_he_2", name: "November long lean hogs: enter on trading day 2, hold 13 days", group: "Commodity Trader's Almanac 2012 · seasonal trades", session: 0, dir: "Long", status: Status::Live, needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK,
            rules: &["E|\"NOVEMBER LONG LEAN HOGS (FEBRUARY) TRADING DAY: 2-HOLD: 13 DAYS\" (ch.11); the trade is taken on the 2th trading day of November at the close", "X|held 13 trading days, covered at that close (the box's HOLD)"], notes: NOTE, claim: None },
        StrategyDef { id: "hcta12m_11_l_gc_13", name: "November long gold: enter on trading day 13, hold 10 days", group: "Commodity Trader's Almanac 2012 · seasonal trades", session: 0, dir: "Long", status: Status::Live, needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK,
            rules: &["E|\"NOVEMBER LONG GOLD (FEBRUARY) TRADING DAY: 13-HOLD: 10 DAYS\" (ch.11); the trade is taken on the 13th trading day of November at the close", "X|held 10 trading days, covered at that close (the box's HOLD)"], notes: NOTE, claim: None },
        StrategyDef { id: "hcta12m_11_s_zb_14", name: "November short 30-year bond: enter on trading day 14, hold 107 days", group: "Commodity Trader's Almanac 2012 · seasonal trades", session: 0, dir: "Short", status: Status::Live, needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK,
            rules: &["E|\"NOVEMBER SHORT 30-YR BOND (JUNE) TRADING DAY: 14-HOLD: 107 DAYS\" (ch.11); the trade is taken on the 14th trading day of November at the close", "X|held 107 trading days, covered at that close (the box's HOLD)"], notes: NOTE, claim: None },
        StrategyDef { id: "hcta12m_12_l_zw_5", name: "December long wheat: enter on trading day 5, hold 20 days", group: "Commodity Trader's Almanac 2012 · seasonal trades", session: 0, dir: "Long", status: Status::Live, needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK,
            rules: &["E|\"DECEMBER LONG CBOT WHEAT (MAY) TRADING DAY: 5-HOLD: 20 DAYS\" (ch.12); the trade is taken on the 5th trading day of December at the close", "X|held 20 trading days, covered at that close (the box's HOLD)"], notes: NOTE, claim: None },
        StrategyDef { id: "hcta12m_12_s_6b_19", name: "December short British pound: enter on trading day 19, hold 56 days", group: "Commodity Trader's Almanac 2012 · seasonal trades", session: 0, dir: "Short", status: Status::Live, needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK,
            rules: &["E|\"DECEMBER SHORT BRITISH POUND (JUNE) TRADING DAY: 19-HOLD: 56 DAYS\" (ch.12); the trade is taken on the 19th trading day of December at the close", "X|held 56 trading days, covered at that close (the box's HOLD)"], notes: NOTE, claim: None },
        StrategyDef { id: "hcta12m_12_s_6s_19", name: "December short Swiss franc: enter on trading day 19, hold 41 days", group: "Commodity Trader's Almanac 2012 · seasonal trades", session: 0, dir: "Short", status: Status::Live, needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK,
            rules: &["E|\"DECEMBER SHORT SWISS FRANC (MARCH) TRADING DAY: 19-HOLD: 41 DAYS\" (ch.12); the trade is taken on the 19th trading day of December at the close", "X|held 41 trading days, covered at that close (the box's HOLD)"], notes: NOTE, claim: None },
    ]
}
pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> { reviewed(id).then_some(("hcta12m", "Commodity Trader's Almanac 2012 · seasonal trades", "calendar")) }
pub fn params(_id: &str) -> Option<Vec<crate::connors::ParamDef>> { None }

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, _p: &Params) -> Option<Vec<Trade>> {
    let &(_, mon, dir, td, hold) = row(id)?;
    if from >= to || to >= x.r.len() { return None; }
    let ym = |i: usize| &x.r[i].date[..7];
    let mut out: Vec<Trade> = vec![];
    for f in 1..x.r.len() {
        if ym(f) == ym(f - 1) || x.r[f].date[5..7].parse::<u32>().ok() != Some(mon) { continue; }
        let e = f + td - 1;
        if e >= x.r.len() || ym(e) != ym(f) || e < from || e >= to { continue; }
        let (xi, open) = if e + hold <= to { (e + hold, false) } else { (to, true) };
        out.push(Trade { dir, legs: vec![Leg { i: e, px: x.c[e], w: 1. }], exit_i: xi, exit_px: x.c[xi], reason: if open { "open".into() } else { format!("{hold} trading days held · close") },
            entry_kind: "close", signal_i: xi, open, exits: vec![] });
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn every_row_runs_on_a_long_series() {
        let r = super::super::sctraub::tests::bars(2600);
        let x = Ctx::new(&r, &r, &[]);
        assert_eq!(catalogue().len(), ROWS.len());
        for (id, mon, dir, _, hold) in ROWS {
            let t = run(id, &x, 0, r.len() - 1, &Params::new()).unwrap();
            assert!(t.iter().all(|z| z.dir == *dir && z.exit_i >= z.legs[0].i && (z.open || z.exit_i - z.legs[0].i == *hold)) && !t.is_empty() || *mon > 12);
        }
    }
    /// hand-run dump for the pandas reference: HC12M_DIR=<norgate dir> HC12M_OUT=<file> cargo test --release hcta12m_dump -- --ignored
    #[test]
    #[ignore]
    fn hcta12m_dump() {
        use std::io::Write;
        let mut data = std::collections::BTreeMap::new();
        crate::futures::load(std::path::Path::new(&std::env::var("HC12M_DIR").unwrap()), &mut data);
        let mut f = std::io::BufWriter::new(std::fs::File::create(std::env::var("HC12M_OUT").unwrap()).unwrap());
        writeln!(f, "id,ticker,dir,entry_date,entry_px,exit_date,exit_px,open").unwrap();
        for def in catalogue() {
            let sym = SYMS.iter().find(|s| s.0 == def.id).unwrap().1;
            let tk = crate::futures::MARKETS.iter().find(|m| m.sym == sym).map(crate::futures::ticker).unwrap();
            let Some(rows) = data.get(&tk) else { continue };
            let x = Ctx::new(rows, &[], &[]);
            for z in run(def.id, &x, 0, rows.len() - 1, &Params::new()).unwrap() {
                writeln!(f, "{},{tk},{},{},{},{},{},{}", def.id, z.dir, rows[z.legs[0].i].date, z.legs[0].px, rows[z.exit_i].date, z.exit_px, z.open).unwrap();
            }
        }
    }
}

/// (id, Norgate symbol)
pub const SYMS: &[(&str, &str)] = &[
    ("hcta12m_01_s_6e_3", "&6E"),
    ("hcta12m_01_s_zw_3", "&ZW"),
    ("hcta12m_01_l_sp_15", "&SP"),
    ("hcta12m_02_l_zs_9", "&ZS"),
    ("hcta12m_02_l_cl_10", "&CL"),
    ("hcta12m_02_s_si_13", "&SI"),
    ("hcta12m_02_l_ng_16", "&NG"),
    ("hcta12m_03_s_cc_10", "&CC"),
    ("hcta12m_03_l_6b_15", "&6B"),
    ("hcta12m_04_l_zb_18", "&ZB"),
    ("hcta12m_05_s_le_2", "&LE"),
    ("hcta12m_05_s_hg_8", "&HG"),
    ("hcta12m_05_s_si_10", "&SI"),
    ("hcta12m_05_s_kc_16", "&KC"),
    ("hcta12m_06_s_zs_5", "&ZS"),
    ("hcta12m_06_l_sb_11", "&SB"),
    ("hcta12m_06_l_le_14", "&LE"),
    ("hcta12m_06_s_zc_18", "&ZC"),
    ("hcta12m_07_s_sp_10", "&SP"),
    ("hcta12m_07_l_ng_17", "&NG"),
    ("hcta12m_08_l_6s_6", "&6S"),
    ("hcta12m_08_l_kc_12", "&KC"),
    ("hcta12m_07_s_cc_18", "&CC"),
    ("hcta12m_08_l_cc_14", "&CC"),
    ("hcta12m_08_l_gc_18", "&GC"),
    ("hcta12m_09_s_cl_8", "&CL"),
    ("hcta12m_09_l_6b_12", "&6B"),
    ("hcta12m_10_s_si_4", "&SI"),
    ("hcta12m_10_s_6j_12", "&6J"),
    ("hcta12m_10_l_zs_16", "&ZS"),
    ("hcta12m_10_l_6e_18", "&6E"),
    ("hcta12m_10_l_sp_19", "&SP"),
    ("hcta12m_10_l_zc_20", "&ZC"),
    ("hcta12m_11_l_he_2", "&HE"),
    ("hcta12m_11_l_gc_13", "&GC"),
    ("hcta12m_11_s_zb_14", "&ZB"),
    ("hcta12m_12_l_zw_5", "&ZW"),
    ("hcta12m_12_s_6b_19", "&6B"),
    ("hcta12m_12_s_6s_19", "&6S"),
];
