//! Intraday wave 3 (claude/titles-k24-1103, 2026-11-03): the last "same-day path" withheld rule converted to real
//! 1-minute bars. Reuses intraday.rs (first_touch) and intraday_w2b.rs (day, first_exit) unchanged; the minute walk
//! itself lives in the book module (books/tdperl.rs run_lines_minutes).

/// Rules converted in this wave that run on all four minute tickers (SPY, SPX, NIFTY 50, NIFTYBANK): no fixed amount.
pub const IDS: &[&str] = &["tdperl_tdline_short"];
pub fn id(id: &str) -> bool { IDS.contains(&id) }
pub fn book(b: &str) -> bool { b == "tdperl" }
/// Minute-path rules of waves 1/phase 1 where more than 5% of the trades fall in a minute that reaches two levels without
/// opening through exactly one of them (order decided by a convention, not by the data): withheld (gate row says how many).
pub const HELD: &[&str] = &["ats_wide_range_gap", "btp2_wide_range_exhaustion_gap_short", "hfm_vix", "ss_80_20"];
pub fn held(id: &str) -> bool { HELD.contains(&id) }

#[cfg(test)]
mod tests {
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
    fn wave_3_admission_and_gate() {
        use crate::instrument_policy as ip;
        let cat = crate::connors::catalogue();
        for id in IDS {
            let d = cat.iter().find(|d| d.id == *id).unwrap_or_else(|| panic!("{id}"));
            assert!(ip::intraday_p1(d) && ip::reviewed_cash_rule(d) && ip::held_reason(id).is_none(), "{id}");
            assert_eq!(ip::policy(d).requirement, "declared_index_timing", "{id}");
            for t in ip::INTRADAY_P1_TICKERS { assert!(ip::check(d, t, false).is_none(), "{id} {t}"); }
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
            assert_eq!(crate::gate::get(id).unwrap().verdict, "unverified", "{id}");
        }
    }
}
