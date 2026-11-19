//! Intraday wave 3 (claude/titles-k24-1103, 2026-11-03): the last "same-day path" withheld rule converted to real
//! 1-minute bars. Reuses intraday.rs (first_touch) and intraday_w2b.rs (day, first_exit) unchanged; the minute walk
//! itself lives in the book module (books/tdperl.rs run_lines_minutes).

/// Rules converted in this wave that run on all four minute tickers (SPY, SPX, NIFTY 50, NIFTYBANK): no fixed amount.
pub const IDS: &[&str] = &["tdperl_tdline_short", "tdperl_rebo" /* K25 code, re-applied by lane K53 (decision #229) */];
pub fn id(id: &str) -> bool { IDS.contains(&id) || K38_SPX.contains(&id) || K38_US.contains(&id) }
pub fn book(b: &str) -> bool { matches!(b, "tdperl" | "cpgap" | "acbff") }
// ---- lane K38 block (4 Nov 2026, decision #202; authors/titles-1030/K38.tsv, K38w/check177.out)
/// Capstone gap systems at the book's E-mini S&P points: SPX only (intraday_w1_spx_id), like intraday_w2b::SPX.
pub const K38_SPX: &[&str] = &["cpgap_gap_fill_1", "cpgap_gap_fill_2", "cpgap_gap_fill_combo", "cpgap_gap_continue_1",
    "cpgap_gap_continue_2", "cpgap_gap_continue_2b"];
pub fn k38_spx(id: &str) -> bool { K38_SPX.contains(&id) }
/// #177b on tradable SPY with the points scaled by the trailing 250-day SPX/SPY ratio (2014-01-02..2025-10-21): the average
/// trade changes sign, so the SPX minute result is withheld (nothing is published).
pub const K38_SPX_HELD: &[&str] = &["cpgap_gap_fill_1", "cpgap_gap_fill_2", "cpgap_gap_continue_2", "cpgap_gap_continue_2b"];
/// The e-mini S&P 1-minute rule with no fixed amount: the US minute series only (its clock window is the US session); SPY and SPX
/// passed the #177 tests (same sign, no 3x / 10x gap).
pub const K38_US: &[&str] = &["acbff_mega_1min"];
pub fn k38_us(id: &str) -> bool { K38_US.contains(&id) }
// ---- end lane K38 block
/// Minute-path rules of waves 1/phase 1 where more than 5% of the trades fall in a minute that reaches two levels without
/// opening through exactly one of them (order decided by a convention, not by the data): withheld (gate row says how many).
pub const HELD: &[&str] = &["ats_wide_range_gap", "btp2_wide_range_exhaustion_gap_short", "hfm_vix", "ss_80_20"];
pub fn held(id: &str) -> bool { HELD.contains(&id) }
/// Decision #177a (lane M1, 3 Nov 2026): on the SPY/SPX overlap these minute rules lose on the tradable SPY and win on the
/// cash index (stale component prints). The minute result is published on SPY only, the mirror image of intraday_w2b::SPX.
pub const SPY: &[&str] = &["ats_15min_adx_breakout", "ptjdt_adx15_breakout", "hfm_news", "hfm_news_long", "hfm_news_short",
    "jb30d_gap_pct", "tdnmtt_trap_short", "tdperl_tdline_short", "putg_inside_day", "mhp_ftse_3_20040804",
    "jmls_bar_breakout" /* lane K39 (EP2 15-minute bars): SPX compounded > 10x SPY's (test c) */,
    "gfstsb_ema_cross" /* lane K42 (EP2 60-minute bars): average trade SPY +0.0122% vs SPX -0.0215% (test a) */,
    "vdso_supreme_order" /* lane K48 (daily bars on the minute path, 2014-2025): SPX avg +0.163% vs SPY +0.009% (test b) */,
    "jbcdt_mac" /* lane K55 (5-minute, 2014-2025): average trade SPY +0.0161% vs SPX -0.0111% (test a; SPY the better series, as gfstsb_ema_cross) */,
    "stad2_bullbeater" /* lane K55 (5-minute, 2014-2025): SPY -0.0247% vs SPX +0.0537% (test a) */,
    "stad2_opengap" /* lane K55 (10-minute, 2014-2025): compounded SPX +33.1% vs SPY +2.6% (test c) */,
    "tdperl_rebo" /* K25 code re-applied by lane K53 (decisions #176/#229, 2014-2025): SPY avg -0.0291% vs SPX +0.0882% (test a) */,
    "rwmts_60m_rsi200h" /* lane K53 (60-minute bars, 2014-2025): SPY avg -0.0166% vs SPX +0.0321% (test a) */,
    "rwmts_60m_seven" /* lane K53 (60-minute bars, 2014-2025): SPY avg +0.0247% vs SPX -0.0152% (test a) */,
    "rwmts_30m_rsi_cross" /* lane K53 (30-minute bars, 2014-2025): SPY avg +0.0027% vs SPX -0.0152% (test a) */,
    "rwmts_15m_5m_rsi" /* lane K53 (15-minute bars, 2014-2025): SPY avg -0.0029% vs SPX +0.0228% (test a) */,
    "as51_double_rsi" /* lane K70 (5-minute, 2014-2025): SPX avg +0.0311% vs SPY +0.0053% (test b, floor 0.003 pp per #234) */,
    "as51_ha_psar" /* lane K70 (3-minute Heikin Ashi, 2014-2025): SPY avg -0.0005% vs SPX +0.0174% (test a); lane K71 (#271(2) no-wick candle): SPY +0.0011% vs SPX +0.0170%, comp +6.6% vs +351.9% (tests b, c) */,
    "as51_ema_scalp" /* lane K70 (5-minute, 2014-2025): SPY avg -0.0039% vs SPX +0.0163% (test a) */,
    "jr_tbtm_shrl" /* lane K80 (5-minute, 2014-2025): SPY avg -0.0026% vs SPX +0.0162% (test a) */,
    "jr_tbtm_3day" /* lane K80 (5-minute, 2014-2025): SPY avg +0.0024% vs SPX +0.0527%, comp +2.7% vs +107.0% (tests b, c) */];
pub fn spy_id(id: &str) -> bool { SPY.contains(&id) }
/// Decision #232 (1): the SPY-only rules whose #177a flag has the tradable SPY as the BETTER series (gate `ours` numbers);
/// their withdrawal note says the cash index understates the rule. Every other SPY id was flagged with SPX better.
pub const SPY_BETTER: &[&str] = &["gfstsb_ema_cross", "jbcdt_mac", "rwmts_60m_seven", "rwmts_30m_rsi_cross"]; // #232 (1): SPY the better series
pub fn spy_better(id: &str) -> bool { SPY_BETTER.contains(&id) }
/// Decision #177b: SPX-point minute rules that fail the tradable check on SPY with the points scaled by SPX/SPY (sign flip:
/// 23; SPX average trade >3x and >0.10 pt, or SPX compounded >10x: 13). SPX is their only ticker, so nothing is published.
pub const SPX_HELD: &[&str] = &["baboops_oops", "jb30d_gap_tick", "tcdt_2bnr_m160_h0", "tcdt_2bnr_m80_h0", "tcdt_3bnr_m160_h0",
    "tcdt_3bnr_m80_h0", "tcdt_bearhook_p160", "tcdt_bearhook_p80", "tcdt_bullhook_m80", "tcdt_ctl_m160", "tcdt_ctl_m300",
    "tcdt_ctl_m600", "tcdt_ctl_m80", "tcdt_doji_p80", "tcdt_dojinr4_p160", "tcdt_id_p160", "tcdt_id_p80", "tcdt_nr7_p160",
    "tcdt_nr7_p80", "tcdt_ws4_m160", "tcdt_ws4_m80", "tcdt_ws7_m160", "tcdt_ws7_m80",
    "jkbwts_saidenberg30", "tcdt_ctl_p160", "tcdt_ctl_p80", "tcdt_doji_m160", "tcdt_doji_m80", "tcdt_gap_orb", "tcdt_id_m80",
    "tcdt_idnr4_p160", "tcdt_idnr4_p80", "tcdt_nr4_m160", "tcdt_nr4_m80", "tcdt_nr4_p80", "tcdt_quad_orb",
    "kftsm_first_hour" /* lane K42 (EP2 10-minute bars, 0.20 S&P points): scaled SPY avg -0.0074% vs SPX +0.0285% (test a) */,
    "rsrl_rlevels" /* lane K48 (5-minute bars, S&P points, 2014-2025): scaled SPY avg -0.0026% vs SPX +0.0332% (test a) */];
pub fn spx_held(id: &str) -> bool { SPX_HELD.contains(&id) || K38_SPX_HELD.contains(&id) } // + lane K38
/// Decision #231 (lane M4, 5 Nov 2026; authors/titles-1030/M4.tsv): India minute results that are India-better than SPY over the
/// dates both minute files cover on BOTH tests (average trade > 3x SPY's and > 0.003 pp above it, #234; compounded > 10x SPY's, or
/// > +100% when SPY's <= 0). Those (rule, India ticker) results are withdrawn; the SPY/SPX results stay.
pub const INDIA_HELD: &[(&str, &str)] = &[
    ("jb30d_or30", "NIFTY 50") /* avg +0.0622% vs SPY +0.0041%, compounded +1184.1% vs +8.6% */,
    ("jmls_ma_cross_30m", "NIFTY 50") /* avg +0.1091% vs SPY +0.0302%, compounded +2526.7% vs +128.4% */,
    ("stad7_idbo", "NIFTY 50") /* avg +0.0772% vs SPY +0.0095%, compounded +2501.5% vs +36.1% */,
    ("mlhpt_ma3", "NIFTY 50") /* avg +0.0634% vs SPY +0.0135%, compounded +19698.9% vs +184.9% */,
    ("atm04_60min", "NIFTY 50") /* avg +0.0603% vs SPY +0.0135%, compounded +803.9% vs +54.0% */,
    ("sgbic_breakout", "NIFTY 50") /* avg +0.0194% vs SPY +0.0023%, compounded +82.8% vs +7.1% */,
    ("jb30d_or30", "NIFTYBANK") /* avg +0.0715% vs SPY +0.0022%, compounded +1185.9% vs +1.8% */,
    ("stad7_idbo", "NIFTYBANK") /* avg +0.0875% vs SPY +0.0077%, compounded +2239.9% vs +26.2% */,
    ("lgvp_volpat", "NIFTYBANK") /* avg +0.1243% vs SPY +0.0189%, compounded +532.7% vs +30.6% */,
    ("acbff_open_rel", "NIFTYBANK") /* avg +0.0312% vs SPY +0.0030%, compounded +165.3% vs +1.8% */,
    ("acbff_9oc", "NIFTYBANK") /* avg +0.0056% vs SPY +0.0008%, compounded +689.5% vs +21.7% */,
    ("mlhpt_ma3", "NIFTYBANK") /* avg +0.0753% vs SPY +0.0116%, compounded +28227.8% vs +124.0% */,
    ("atm04_60min", "NIFTYBANK") /* avg +0.0686% vs SPY +0.0066%, compounded +768.4% vs +18.2% */,
    ("as51_st_pivot", "NIFTY 50") /* lane K70: avg +0.0708% vs SPY +0.0217%, compounded +2323.0% vs +147.7% (2008-2025) */,
    ("as51_st_pivot", "NIFTYBANK") /* lane K70: avg +0.0856% vs SPY +0.0161%, compounded +2301.6% vs +82.3% (2010-2025) */];
pub fn india_held(id: &str, ticker: &str) -> bool { INDIA_HELD.iter().any(|&(i, t)| i == id && (t == ticker || (t == "NIFTYBANK" && ticker == "NIFTY BANK"))) }
pub const INDIA_HELD_NOTE: &str = "NSE cash-index minutes are dominated by stale component prices; there is no tradable India minute series and this result is more than 10x the tradable SPY result over the same dates (decision #231, 5 Nov 2026).";

#[cfg(test)]
mod tests {
    #[test]
    fn decision_231_india_withdrawals() {
        use crate::instrument_policy as ip;
        let cat = crate::connors::catalogue();
        for &(id, t) in INDIA_HELD {
            let d = cat.iter().find(|d| d.id == id).unwrap_or_else(|| panic!("{id}"));
            assert!(ip::intraday_p1(d), "{id}");
            assert_eq!(ip::check(d, t, false).unwrap().note, INDIA_HELD_NOTE, "{id} {t}");
            assert!(!ip::intraday_tickers(id).contains(&t), "{id} {t}");
            assert!(ip::check(d, "SPY", false).is_none() && !crate::intraday_w3::spy_id(id), "{id}");
            for o in ["NIFTY 50", "NIFTYBANK"] { assert_eq!(ip::check(d, o, false).is_none(), !india_held(id, o), "{id} {o}"); }
            assert!(crate::gate::get(id).unwrap().reason.contains("#231"), "{id}");
        }
    }
    use super::*;
    use crate::connors::{Ctx, Params};
    fn minutes(name: &str, days: &[(&str, &[&str])]) -> &'static crate::intraday::Minutes {
        let dir = std::env::temp_dir().join(format!("w3{name}{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let (mut body, mut idx) = (String::from("Date,Time,Open,High,Low,Close\n"), String::from("Date,offset,bytes\n"));
        for (d, lines) in days {
            let start = body.len();
            for l in *lines { body.push_str(&format!("{d},{l}\n")); }
            idx.push_str(&format!("{d},{start},{}\n", body.len() - start));
        }
        std::fs::write(dir.join("T.m1.csv"), &body).unwrap();
        std::fs::write(dir.join("T.m1idx.csv"), &idx).unwrap();
        Box::leak(Box::new(crate::intraday::Minutes::load(dir.to_str().unwrap(), "T").unwrap()))
    }
    fn rows() -> Vec<crate::Row> {
        // demand points: bar 1 (low 95) and bar 4 (low 96) -> line 96 + (j-4)/3, 96.667 on bar 6; supply points bar 3 (103)
        // and bar 5 (102.5) -> line 102 on bar 7. Bar 6 breaks the demand line (Q1: close 5 > close 4); objective = 96.667 -
        // (true high 103 of bar 3 - line 95.667 there) = 89.333. Bar 7 reaches both the objective and a qualified supply
        // line break (Q1: close 6 < close 5).
        let b = [(100., 101., 99., 100.), (100., 101., 95., 100.), (100., 102., 97., 101.), (101., 103., 98., 102.),
            (99., 100., 96., 97.), (99.5, 102.5, 99., 102.), (100., 100.5, 90., 91.), (92., 103., 89., 95.)];
        b.iter().enumerate().map(|(k, &(o, h, l, c))| crate::Row { ticker: "T".into(), date: format!("2020-01-{:02}", k + 1), open: o, high: h, low: l, close: c, volume: None }).collect()
    }
    #[test]
    fn td_demand_line_break_minutes_decide_objective_before_adverse_break() {
        let r = rows();
        let mut x = Ctx::new(&r, &r, &[]);
        let run = |x: &Ctx, path: f64| crate::books::tdperl::run("tdperl_tdline_short", x, 0, 7, &Params::from([("path".to_string(), path)])).unwrap();
        // daily path: the adverse break is taken first -> covered at the supply line 102
        let d = run(&x, 0.0);
        assert_eq!(d.len(), 1);
        assert!((d[0].legs[0].px - (96. + 2. / 3.)).abs() < 1e-9 && d[0].exit_i == 7 && (d[0].exit_px - 102.).abs() < 1e-9, "{:?}", d[0]);
        x.minutes = Some(minutes("tdl", &[
            ("2020-01-07", &["09:30,100,100.5,99,99.5", "09:31,99.5,99.6,96,96.2", "09:32,96.2,96.3,90,91"]),
            ("2020-01-08", &["09:30,92,92.5,89,90", "09:31,90,103,90,95"]),
        ]));
        // minutes: filled at the line in the 09:31 minute, objective reached at 09:30 the next day before the 103 high
        let t = run(&x, 1.0);
        assert_eq!(t.len(), 1, "{t:?}");
        assert!((t[0].legs[0].px - (96. + 2. / 3.)).abs() < 1e-9 && t[0].legs[0].i == 6 && t[0].entry_kind == "stop", "{:?}", t[0]);
        assert!(t[0].exit_i == 7 && (t[0].exit_px - (89. + 1. / 3.)).abs() < 1e-9 && t[0].reason.starts_with("TD Line objective"), "{:?}", t[0]);
        // the later minute reaching both levels: nearer the open first, flagged ambiguous
        x.minutes = Some(minutes("tdl2", &[
            ("2020-01-07", &["09:30,100,100.5,99,99.5", "09:31,99.5,99.6,96,96.2", "09:32,96.2,96.3,90,91"]),
            ("2020-01-08", &["09:30,92,103,89,95"]),
        ]));
        let t = run(&x, 1.0);
        assert!((t[0].exit_px - (89. + 1. / 3.)).abs() < 1e-9 && t[0].reason.contains("ambiguous"), "{:?}", t[0]); // 89.33 is 2.67 from 92, 102 is 10
        // the second day has no minutes and its daily bar reaches both exits: order unknown, trade not taken
        x.minutes = Some(minutes("tdl3", &[("2020-01-07", &["09:30,100,100.5,99,99.5", "09:31,99.5,99.6,96,96.2", "09:32,96.2,96.3,90,91"])]));
        assert!(run(&x, 1.0).is_empty());
        // no minutes on the entry day: no trade (no daily substitute)
        x.minutes = Some(minutes("tdl4", &[("2020-01-08", &["09:30,92,92.5,89,90"])]));
        assert!(run(&x, 1.0).is_empty());
    }
    #[test]
    fn rebo_minutes_reverse_in_time_order() {
        // K25. Bar 2: open 100, yesterday's true range 10 -> buy stop 103.82, sell stop 96.18; the daily bar reaches both.
        let b = [(100., 105., 95., 100.), (100., 105., 95., 100.), (100., 110., 90., 100.), (100., 101., 99., 100.)];
        let r: Vec<crate::Row> = b.iter().enumerate().map(|(k, &(o, h, l, c))| crate::Row { ticker: "T".into(), date: format!("2020-01-{:02}", k + 1), open: o, high: h, low: l, close: c, volume: None }).collect();
        let mut x = Ctx::new(&r, &r, &[]);
        let run = |x: &Ctx, path: f64| crate::books::tdperl::run("tdperl_rebo", x, 0, 3, &Params::from([("path".to_string(), path)])).unwrap();
        assert!(run(&x, 0.0).is_empty()); // daily path: both levels on a flat day -> no entry
        x.minutes = Some(minutes("rebo", &[("2020-01-03", &["09:30,100,104,99,103", "09:31,103,103.5,96,97", "09:32,97,98,97,98"]), ("2020-01-04", &["09:30,100,101,99,100"])]));
        let t = run(&x, 1.0);
        assert_eq!(t.len(), 2, "{t:?}");
        assert!(t[0].dir > 0. && (t[0].legs[0].px - 103.82).abs() < 1e-9 && t[0].exit_i == 2 && (t[0].exit_px - 96.18).abs() < 1e-9, "{:?}", t[0]);
        assert!(t[1].dir < 0. && (t[1].legs[0].px - 96.18).abs() < 1e-9 && t[1].open && t[1].exit_px == 100., "{:?}", t[1]);
        // one minute reaching both, opened nearer the sell stop: short first [convention, flagged], then reversed long at 103.82
        x.minutes = Some(minutes("rebo2", &[("2020-01-03", &["09:30,97,105,95,100"])]));
        let t = run(&x, 1.0);
        assert!(t.len() == 2 && t[0].dir < 0. && (t[0].legs[0].px - 96.18).abs() < 1e-9 && t[0].reason.contains("ambiguous") && t[1].dir > 0. && t[1].open, "{t:?}");
        // no minutes on bar 2: no position opened
        x.minutes = Some(minutes("rebo3", &[("2020-01-04", &["09:30,100,101,99,100"])]));
        assert!(run(&x, 1.0).is_empty());
    }
    #[test]
    fn wave_3_admission_and_gate() {
        use crate::instrument_policy as ip;
        let cat = crate::connors::catalogue();
        for id in IDS {
            let d = cat.iter().find(|d| d.id == *id).unwrap_or_else(|| panic!("{id}"));
            assert!(ip::intraday_p1(d) && ip::reviewed_cash_rule(d) && ip::held_reason(id).is_none(), "{id}");
            assert_eq!(ip::policy(d).requirement, "declared_index_timing", "{id}");
            for t in ip::INTRADAY_P1_TICKERS { assert_eq!(ip::check(d, t, false).is_none(), *t == "SPY", "{id} {t}"); } // #177a: SPY only
            assert!(ip::check(d, "QQQ", false).is_some() && ip::check(d, "AAPL", true).is_some(), "{id}");
            assert_eq!(crate::gate::get(id).unwrap().verdict, "unverified", "{id}");
            assert!(crate::books::params(id).unwrap().iter().any(|p| p.key == "path" && p.def == 1.0), "{id}");
        }
        // more than 5% ambiguous minutes: withheld for real (no result, the gate reason on the card)
        for id in HELD {
            let d = cat.iter().find(|d| d.id == *id).unwrap_or_else(|| panic!("{id}"));
            assert!(!ip::intraday_p1(d) && !ip::reviewed_cash_rule(d), "{id}");
            assert_eq!(crate::gate::get(id).unwrap().verdict, "withheld", "{id}");
            assert!(crate::gate::get(id).unwrap().reason.contains("5% limit"), "{id}");
            for t in ip::INTRADAY_P1_TICKERS { assert!(ip::check(d, t, false).is_some(), "{id} {t}"); }
        }
        // the long side stays the reviewed daily rule; the other wave-1 ids are no longer withheld by the gate
        let long = cat.iter().find(|d| d.id == "tdperl_tdline_long").unwrap();
        assert!(!ip::intraday_p1(long) && ip::reviewed_cash_rule(long));
        for id in ["ats_10pct_oops", "baboops_oops", "btp2_10pct_oops", "btp2_10pct_oops_long", "btp2_10pct_oops_short",
            "jb30d_gap_pct", "jb30d_gap_tick", "tdnmtt_trap_long", "tdnmtt_trap_short"] {
            assert_eq!(crate::gate::get(id).unwrap().verdict, if spx_held(id) { "withheld" } else { "unverified" }, "{id}"); // #177b
        }
    }
    #[test]
    fn decision_177_tradable_check_limits_minute_tickers() {
        use crate::instrument_policy as ip;
        let cat = crate::connors::catalogue();
        fn find<'a>(cat: &'a [crate::connors::StrategyDef], id: &str) -> &'a crate::connors::StrategyDef { cat.iter().find(|d| d.id == id).unwrap_or_else(|| panic!("{id}")) }
        assert_eq!((SPY.len(), SPX_HELD.len()), (26, 38)); // + jr_tbtm_shrl, jr_tbtm_3day (K80); + as51_double_rsi, as51_ha_psar, as51_ema_scalp (K70); // + tdperl_rebo, rwmts_60m_rsi200h, rwmts_60m_seven, rwmts_30m_rsi_cross, rwmts_15m_5m_rsi (K53); + jbcdt_mac, stad2_bullbeater, stad2_opengap (K55); // + jmls_bar_breakout (K39); + gfstsb_ema_cross, kftsm_first_hour (K42); + vdso_supreme_order, rsrl_rlevels (K48); - kftsm_zones (K50: lag=1 default passes #177a)
        assert!(SPY_BETTER.iter().all(|id| SPY.contains(id))); // #232 (1)
        for id in SPY { // #177a: SPY only; the cash-index minute results are withdrawn with the card reason
            let d = find(&cat, id);
            assert!(ip::intraday_p1(d) && !ip::intraday_w1_spx_id(id), "{id}");
            assert_eq!(ip::intraday_tickers(id), &["SPY"][..]);
            assert!(ip::check(d, "SPY", false).is_none(), "{id}");
            if spy_better(id) { // decision #232 (1): both wordings, the true one per direction
                for t in ["SPX", "NIFTY 50", "NIFTYBANK"] { assert!(ip::check(d, t, false).unwrap().note.contains("cash-index minutes are stale and distort this rule (here the index understates it); only the tradable SPY result is published"), "{id} {t}"); }
            } else {
            assert!(ip::check(d, "SPX", false).unwrap().note.contains("cash-index minutes are stale; on the tradable SPY this rule loses"), "{id}");
            for t in ["NIFTY 50", "NIFTYBANK"] { assert!(ip::check(d, t, false).unwrap().note.contains("no tradable India minute series, and the US tradable check failed"), "{id} {t}"); }
            }
            let g = crate::gate::get(id).unwrap();
            // verified only on a #168 pass of the book's own table inside the minute history (rwmts_15m_5m_rsi, K61 #248 (3))
            assert!((g.verdict == "unverified" || (g.verdict == "verified" && g.reason.contains("#168"))) && g.reason.contains("#177a") && g.ours.contains("SPX (withdrawn)"), "{id}");
        }
        for id in SPX_HELD { // #177b: the SPX-point rule's only ticker is withheld
            let d = find(&cat, id);
            assert!(ip::intraday_p1(d) && ip::intraday_w1_spx_id(id) && ip::intraday_tickers(id).is_empty(), "{id}");
            assert!(ip::check(d, "SPX", false).unwrap().note.contains("on tradable SPY with the points scaled, the result fails"), "{id}");
            for t in ip::INTRADAY_P1_TICKERS { assert!(ip::check(d, t, false).is_some(), "{id} {t}"); }
            let g = crate::gate::get(id).unwrap();
            assert!(g.verdict == "withheld" && g.reason.contains("#177b") && g.ours.starts_with("scaled-SPY check"), "{id}");
        }
        // #177c: unflagged rules keep every minute ticker, India with the note; the near misses carry both numbers (#177d)
        for id in ["ats_10pct_oops", "ss_momentum_pinball", "btp2_10pct_oops", "btp2_10pct_oops_long", "btp2_10pct_oops_short",
            "lwdg2_0105", "mhp_ftse_12_20041015", "tdnmtt_trap_long", "lgvp_volpat"] {
            let d = find(&cat, id);
            for t in ip::INTRADAY_P1_TICKERS { assert_eq!(ip::check(d, t, false).is_none(), !india_held(id, t), "{id} {t}"); } // #231: India pairs withdrawn separately
            assert!(crate::gate::get(id).unwrap().reason.contains("NIFTY cash-index minutes; the same rule passed the tradable-vs-cash check on SPY/SPX"), "{id}");
        }
        for id in ["lwdg2_0105", "lgvp_volpat", "tdnmtt_trap_long"] { assert!(crate::gate::get(id).unwrap().ours.contains("#177d near miss"), "{id}"); }
        // unflagged SPX-point rules keep SPX
        for id in ["tcdt_ctl_p300", "bab80_daytrade", "upg18_traders_dream", "lwlts_mon_vb"] { assert!(ip::check(find(&cat, id), "SPX", false).is_none(), "{id}"); }
    }
    // ---- lane K38 tests
    #[test]
    fn k38_minute_rules_admission_tickers_and_gate() {
        use crate::instrument_policy as ip;
        let cat = crate::connors::catalogue();
        for id in K38_SPX.iter().chain(K38_US) {
            let d = cat.iter().find(|d| d.id == *id).unwrap_or_else(|| panic!("{id}"));
            assert!(ip::intraday_p1(d) && ip::reviewed_cash_rule(d) && ip::held_reason(id).is_none(), "{id}");
            assert_eq!(ip::policy(d).requirement, "declared_index_timing", "{id}");
            let want: &[&str] = if spx_held(id) { &[] } else if k38_us(id) { &["SPY", "SPX"] } else { &["SPX"] };
            assert_eq!(ip::intraday_tickers(id), want, "{id}");
            for t in ip::INTRADAY_P1_TICKERS { assert_eq!(ip::check(d, t, false).is_none(), want.contains(t), "{id} {t}"); }
            assert!(ip::check(d, "QQQ", false).is_some() && ip::check(d, "AAPL", true).is_some(), "{id}");
            let g = crate::gate::get(id).unwrap();
            assert_eq!(g.verdict, if spx_held(id) { "withheld" } else { "unverified" }, "{id}"); // E6: mega back (#208), unverified
            assert!((g.reason.contains("#202") || g.reason.contains("#208")) && (!spx_held(id) || g.reason.contains("#177b")), "{id}");
        }
        for id in K38_SPX_HELD { assert!(ip::check(cat.iter().find(|d| d.id == *id).unwrap(), "SPX", false).unwrap().note.contains("#177b"), "{id}"); }
        let mega = cat.iter().find(|d| d.id == "acbff_mega_1min").unwrap();
        assert!(ip::check(mega, "NIFTY 50", false).unwrap().note.contains("US e-mini S&P session"));
        assert!(matches!(mega.status, crate::connors::Status::Live)); // E6: re-admitted after #208
        assert!(cat.iter().find(|d| d.id == "cpgap_gap_fill_rank").is_some_and(|d| !ip::intraday_p1(d)));
    }
    #[test]
    fn k38_mega_system_reverses_on_the_limits() {
        // minutes 09:30-09:51; wicks 0.5 until 09:44 -> limits +-0.5 at 09:45; long at 99.5, reversed short at the 09:46 open
        // + g (0.44), out at the day's last close
        let mut v: Vec<String> = (570..585).map(|m| format!("{:02}:{:02},100,100.5,99.5,100", m / 60, m % 60)).collect();
        v.push("09:45,100,100.2,99.4,100".into());
        v.push("09:46,100,100.6,99.9,100".into());
        for m in 587..592 { v.push(format!("{:02}:{:02},100,100,100,100", m / 60, m % 60)); }
        let lines: Vec<&str> = v.iter().map(|s| s.as_str()).collect();
        let r = vec![crate::Row { ticker: "T".into(), date: "2020-01-02".into(), open: 100., high: 100.6, low: 99.4, close: 100., volume: None }];
        let mut x = Ctx::new(&r, &r, &[]);
        x.minutes = Some(minutes("mega", &[("2020-01-02", &lines)]));
        let t = crate::books::acbff::run("acbff_mega_1min", &x, 0, 0, &Params::new()).unwrap(); // E6: published again
        let got: Vec<(f64, f64, f64)> = t.iter().map(|t| (t.dir, t.legs[0].px, t.exit_px)).collect();
        assert_eq!(got.len(), 2, "{got:?}");
        assert!(got[0].0 == 1. && (got[0].1 - 99.5).abs() < 1e-9 && (got[0].2 - 100.44).abs() < 1e-9, "{got:?}");
        assert!(got[1].0 == -1. && (got[1].1 - 100.44).abs() < 1e-9 && got[1].2 == 100., "{got:?}");
        assert!(!t[0].reason.contains("ambiguous"));
    }
    // ---- end lane K38 tests

}
