//! Intraday wave 2b (claude/intraday-w2b, 2026-10-29): more same-day rules on real 1-minute bars.
//! Shared by the book modules converted in this wave; reuses src/intraday.rs (first_touch, stop_entry_day) unchanged.
use crate::connors::Ctx;
use crate::intraday::{fill_px, touched, Bar1, Level};

/// NIFTY 50 minutes from this date have bad closing minutes in the source (frozen 15:15-15:19 bars, then a spike outside
/// NSE's official day range; data-verification-20261027): such days are not used, so they make no minute-path trade.
pub const NIFTY_BAD_FROM: &str = "2026-08-11";

/// The day's 1-minute bars for bar i, or None (no minutes loaded, day excluded by the cleaner, or a rejected NIFTY 50 day).
pub fn day(x: &Ctx, i: usize) -> Option<Vec<Bar1>> {
    let r = x.r.get(i)?;
    if r.ticker == "NIFTY 50" && r.date.as_str() >= NIFTY_BAD_FROM { return None; }
    x.minutes?.day(&r.date)
}

/// Rules converted to the minute path in this wave that run on all four minute tickers (SPY, SPX, NIFTY 50, NIFTYBANK).
pub const IDS: &[&str] = &["hfm_news", "hfm_news_long", "hfm_news_short", "lgvp_volpat", "mhp_ftse_12_20041015",
    "mhp_ftse_3_20040804", "putg_inside_day"];
/// Rules whose levels are the book's fixed S&P points: SPX only (the same SPX-literal reading as wave 1).
pub const SPX: &[&str] = &["bab80_daytrade", "jbcdt_gap_go", "jkbwts_saidenberg30", "upg18_traders_dream"];
pub fn id(id: &str) -> bool { IDS.contains(&id) || SPX.contains(&id) }
pub fn spx_id(id: &str) -> bool { SPX.contains(&id) }
pub fn book(b: &str) -> bool { matches!(b, "hfm" | "lgvp" | "mhpst" | "putg" | "bab80" | "jbcdt" | "jkbwts" | "upg18") }

/// Position exits on the minutes from `from`: the first of `levels` touched (stop or limit), at the level or the minute's
/// open when it opened through it. Two levels in one minute without opening through exactly one [convention]: the level
/// nearer the minute's open came first (the TradeStation bar path, applied to the minute). Returns (level k, bar, fill, ambiguous).
pub fn first_exit(b: &[Bar1], from: usize, levels: &[Level]) -> Option<(usize, usize, f64, bool)> {
    use crate::intraday::{first_touch, Touch};
    match first_touch(b, from, levels) {
        Touch::Hit { k, bar, px } => Some((k, bar, px, false)),
        Touch::Ambiguous { bar } => {
            let z = &b[bar];
            let k = (0..levels.len()).filter(|&k| touched(z, levels[k]))
                .min_by(|&a, &c| (levels[a].px - z.o).abs().total_cmp(&(levels[c].px - z.o).abs()))?;
            Some((k, bar, fill_px(z, levels[k]), true))
        }
        Touch::None => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::connors::Ctx;
    fn b(m: u16, o: f64, h: f64, l: f64, c: f64) -> Bar1 { Bar1 { m, o, h, l, c } }
    #[test]
    fn first_exit_nearer_level_in_one_minute() {
        let lv = [Level { up: true, px: 102.0 }, Level { up: false, px: 99.0 }];
        let v = [b(0, 100.0, 101.0, 99.5, 100.5), b(1, 100.5, 102.5, 98.0, 100.0)];
        assert_eq!(first_exit(&v, 0, &lv), Some((0, 1, 102.0, true))); // 102 is 1.5 from the open, 99 is 1.5 too -> first listed
        let w = [b(0, 101.5, 102.5, 98.0, 100.0)];
        assert_eq!(first_exit(&w, 0, &lv), Some((0, 0, 102.0, true)));
        let z = [b(0, 99.2, 102.5, 98.0, 100.0)];
        assert_eq!(first_exit(&z, 0, &lv), Some((1, 0, 99.0, true)));
        assert_eq!(first_exit(&z, 1, &lv), None);
    }
    fn minutes(name: &str, days: &[(&str, &[&str])]) -> &'static crate::intraday::Minutes {
        let dir = std::env::temp_dir().join(format!("w2b{name}{}", std::process::id()));
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
    fn row(d: &str, o: f64, h: f64, l: f64, c: f64) -> crate::Row { crate::Row { ticker: "T".into(), date: d.into(), open: o, high: h, low: l, close: c, volume: None } }
    #[test]
    fn news_reversal_minute_order_decides_the_stop() {
        // gap below yesterday's low 100: buy stop 100.01; stop at the open 99. Daily bar reaches both; the minutes say the
        // low (98.5) came before the fill, so the trade is not stopped and exits at the close.
        let r = vec![row("2020-01-01", 102., 104., 100., 101.), row("2020-01-02", 99., 103., 98.5, 102.)];
        let mut x = Ctx::new(&r, &r, &[]);
        x.minutes = Some(minutes("news", &[("2020-01-02", &["09:30,99,99.2,98.5,98.8", "09:31,98.8,100.5,98.8,100.4", "09:32,100.4,103,100.2,102"])]));
        let t = crate::books::hfm::run("hfm_news_long", &x, 0, 1, &crate::connors::Params::new()).unwrap();
        assert_eq!(t.len(), 1);
        assert!((t[0].legs[0].px - 100.01).abs() < 1e-9 && t[0].exit_px == 102.0, "{:?}", t[0]);
        // no minutes for the day: no trade (no daily substitute)
        x.minutes = Some(minutes("news2", &[("2020-01-03", &["09:30,1,1,1,1"])]));
        assert!(crate::books::hfm::run("hfm_news_long", &x, 0, 1, &crate::connors::Params::new()).unwrap().is_empty());
    }
    #[test]
    fn collins_5c_day_buys_the_up_close_after_five_down_closes() { // Table 36.1 printed code (code-vs-book fix)
        let c = [10., 9., 8., 7., 6., 5., 6., 6.5];
        let r: Vec<crate::Row> = c.iter().enumerate().map(|(k, &v)| row(&format!("2020-01-{:02}", k + 1), v, v + 0.5, v - 0.5, v)).collect();
        let x = Ctx::new(&r, &r, &[]);
        let t = crate::books::acbff::run("acbff_5c_day", &x, 0, 7, &crate::connors::Params::new()).unwrap();
        assert_eq!((t.len(), t[0].dir, t[0].legs[0].i), (1, 1.0, 7));
        let t = crate::books::acbff::run("acbff_5c_day", &x, 0, 7, &crate::connors::Params::from([("swap".to_string(), 1.0)])).unwrap();
        assert!(t.is_empty() || t[0].dir < 0.0);
    }
    #[test]
    fn nifty_bad_closing_minutes_are_not_used() {
        let r = vec![crate::Row { ticker: "NIFTY 50".into(), ..row("2026-08-11", 1., 1., 1., 1.) }];
        let mut x = Ctx::new(&r, &r, &[]);
        x.minutes = Some(minutes("nifty", &[("2026-08-11", &["09:15,1,1,1,1"])]));
        assert!(day(&x, 0).is_none());
    }
    #[test]
    fn wave_2b_admission_and_gate() {
        use crate::instrument_policy as ip;
        let cat = crate::connors::catalogue();
        for id in IDS.iter().chain(SPX) {
            let d = cat.iter().find(|d| d.id == *id).unwrap_or_else(|| panic!("{id}"));
            assert!(ip::intraday_p1(d) && ip::reviewed_cash_rule(d) && ip::held_reason(id).is_none(), "{id}");
            assert_eq!(ip::policy(d).requirement, "declared_index_timing", "{id}");
            for t in ip::INTRADAY_P1_TICKERS { assert_eq!(ip::check(d, t, false).is_none(), !spx_id(id) || *t == "SPX", "{id} {t}"); }
            assert!(ip::check(d, "QQQ", false).is_some() && ip::check(d, "AAPL", true).is_some(), "{id}");
            assert_eq!(crate::gate::get(id).unwrap().verdict, "unverified", "{id}");
            assert!(crate::books::params(id).unwrap().iter().any(|p| p.key == "path" && p.def == 1.0), "{id}");
        }
        for id in ["acbff_c40", "acbff_six_q66_v2", "rgc_dow_oc", "ftm2_dm_skew", "tdperl_clop_short", "dgmf_vbo_taylor", "jk36k_jk1_buy_monday"] {
            let d = cat.iter().find(|d| d.id == id).unwrap();
            assert!(ip::reviewed_cash_rule(d) && !ip::intraday_p1(d), "{id}");
            assert_eq!(crate::gate::get(id).unwrap().verdict, "unverified", "{id}");
        }
        assert_eq!(crate::gate::get("jkbwts_buymon1400").unwrap().allowed, Some(&["SPX"][..]));
        for id in ["atm04_qqq_crash", "radge_oc_long_stop", "jchr1_lizard_short", "tdperl_tdline_short"] { assert_eq!(crate::gate::get(id).unwrap().verdict, "withheld", "{id}"); }
    }

}
