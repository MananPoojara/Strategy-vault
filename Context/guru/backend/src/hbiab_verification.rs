//! Bounded private-data adapter for independently frozen cases. No runtime route.
use super::*;
#[test]
fn dump_case() {
    let Ok(csv) = std::env::var("BANDY_CASE_CSV") else {
        return;
    };
    let id = std::env::var("BANDY_CASE_ID").unwrap();
    let ticker = std::env::var("BANDY_CASE_TICKER").unwrap();
    let all = load_csv(&csv).unwrap();
    let rows = all.get(&ticker).unwrap();
    let x = Ctx::new(rows, rows, &[]);
    let from = std::env::var("BANDY_CASE_FROM")
        .unwrap()
        .parse::<usize>()
        .unwrap();
    let to = std::env::var("BANDY_CASE_TO")
        .unwrap()
        .parse::<usize>()
        .unwrap();
    let costs = accounting::Costs {
        fees_bps: std::env::var("BANDY_CASE_FEES")
            .unwrap_or("0".into())
            .parse()
            .unwrap(),
        slippage_bps: std::env::var("BANDY_CASE_SLIPPAGE")
            .unwrap_or("0".into())
            .parse()
            .unwrap(),
        rollover_bps: 0.,
    };
    let run = backtest_costs(&x, &id, from, to, costs, &Params::new()).unwrap();
    let payload = bandy_detail(&x, &id, from, to, costs, &run);
    let st = books::hbiab_reviewed::states(&id, &x).unwrap();
    let indicators = (0..=to)
        .map(|i| {
            format!(
                "{{\"date\":\"{}\",\"a\":{},\"b\":{},\"buy\":{},\"sell\":{}}}",
                x.r[i].date,
                full_precision_trade_return(st.first[i]),
                full_precision_trade_return(st.second[i]),
                st.buy[i],
                st.sell[i]
            )
        })
        .collect::<Vec<_>>()
        .join(",");
    let result = format!("{{\"payload\":{payload},\"indicators\":[{indicators}]}}");
    std::fs::write(std::env::var("BANDY_CASE_OUTPUT").unwrap(), result).unwrap();
}
#[test]
fn raw_default_queries_do_not_round_into_review() {
    for &id in books::hbiab_reviewed::IDS {
        let base = BTreeMap::from([("id".into(), id.into())]);
        assert!(checked_params_of(&base).is_ok());
        for setting in books::hbiab::params(id).unwrap() {
            for prefix in ["p.", "g."] {
                let mut q = base.clone();
                q.insert(format!("{prefix}{}", setting.key), setting.def.to_string());
                assert!(checked_params_of(&q).is_ok());
                for value in [
                    "NaN".into(),
                    "inf".into(),
                    "garbage".into(),
                    "".into(),
                    format!("{}.00000000000000000001", setting.def),
                ] {
                    q.insert(format!("{prefix}{}", setting.key), value);
                    assert!(checked_params_of(&q).is_err());
                }
            }
            for raw in [
                format!("{}:{}:1", setting.def, setting.def),
                format!("{}:{}:1e-300", setting.def, setting.def),
            ] {
                let mut q = base.clone();
                q.insert(format!("g.{}", setting.key), raw);
                assert!(checked_params_of(&q).is_ok());
            }
            for raw in [
                "0:inf:1",
                "0:1:1e-300",
                "-1e308:1e308:1",
                "5:5:0",
                "5:5:NaN",
                "5:5:inf",
            ] {
                let mut q = base.clone();
                q.insert(format!("g.{}", setting.key), raw.into());
                assert!(checked_params_of(&q).is_err());
            }
        }
        for key in [
            "p.unknown",
            "g.unknown",
            "p.fill",
            "p.allocation",
            "p.source_convention",
        ] {
            let mut q = base.clone();
            q.insert(key.into(), "0".into());
            assert!(checked_params_of(&q).is_err());
        }
    }
}
#[test]
fn exact_three_account_policy_has_no_cross_book_or_legacy_inheritance() {
    for &id in books::hbiab_reviewed::IDS {
        let d = books::hbiab::catalogue()
            .into_iter()
            .find(|d| d.id == id)
            .unwrap();
        assert_eq!(
            instrument_policy::policy(&d).allowed_types,
            &["individual_stock", "etf"]
        );
        assert!(instrument_policy::check(&d, "SPY", false).is_none());
        assert!(instrument_policy::check(&d, "NIFTY 50", false).is_some());
        let mut spoof = d;
        spoof.book = "other";
        assert_eq!(
            instrument_policy::policy(&spoof).requirement,
            "source_review_required"
        );
    }
    for d in books::hbiab::catalogue()
        .iter()
        .filter(|d| !books::hbiab_reviewed::reviewed(d.id) && !books::hbiab::reviewed(d.id)) // Bandy block admits two more (25 Oct 2026)
    {
        assert_eq!(
            instrument_policy::policy(d).requirement,
            "source_review_required"
        );
    }
}

#[test]
fn one_session_summary_uses_bandy_cash_costs_and_matches_detail() {
    // Synthetic prices test report wiring, not market/source performance.
    let rows: Vec<Row> = (0..22)
        .map(|i| {
            let c = if i == 20 {
                99.
            } else if i == 21 {
                110.
            } else {
                100.
            };
            Row {
                ticker: "SPY".into(),
                date: format!("2001-01-{:02}", i + 1),
                open: c,
                high: c + 1.,
                low: c - 1.,
                close: c,
                volume: None,
            }
        })
        .collect();
    let rows: &'static [Row] = Box::leak(rows.into_boxed_slice());
    let st = State {
        ctxs: BTreeMap::from([("SPY".into(), Ctx::new(rows, rows, &[]))]),
        baskets: baskets::Catalogue::default(),
        vix: vec![],
        market_extra: BTreeMap::new(),
    };
    for &id in books::hbiab_reviewed::IDS {
        let q = BTreeMap::from([
            ("id".into(), id.into()),
            ("ticker".into(), "SPY".into()),
            ("from".into(), "2001-01-22".into()),
            ("to".into(), "2001-01-22".into()),
            ("cost".into(), "3".into()),
            ("slippage".into(), "2".into()),
        ]);
        let summary = summary(&st, &q).unwrap();
        let begin = summary.find(&format!("{{\"id\":\"{id}\"")).unwrap();
        let row = &summary[begin..];
        let row = row.split("},{\"id\":").next().unwrap();
        assert!(row.contains("\"stats\":"), "{id}: {row}");
        let actual = detail(&st, &q).unwrap();
        let detail_stats = actual
            .split("\"stats\":")
            .nth(1)
            .unwrap()
            .split(",\"gross_stats\":")
            .next()
            .unwrap();
        let summary_stats = row
            .split("\"stats\":")
            .nth(1)
            .unwrap()
            .split(",\"spark\":")
            .next()
            .unwrap();
        assert_eq!(summary_stats, detail_stats);
        let costs = bandy_costs_json(accounting::Costs::parse(&q).unwrap());
        assert!(row.contains(&costs));
        assert!(actual.contains(&costs));
        assert!(!row.contains("Legacy equity includes estimated exit costs"));

        // The public catalogue request has an owner but no strategy ID. An ID
        // on the detailed request must not accidentally authorize the list.
        let mut public_query = q.clone();
        public_query.remove("id");
        let public_query = owned_query(&public_query, "connors");
        let public_summary = crate::summary(&st, &public_query).unwrap();
        let begin = public_summary.find(&format!("{{\"id\":\"{id}\"")).unwrap();
        let public_row = public_summary[begin..].split("},{\"id\":").next().unwrap();
        assert_eq!(public_row, row);
        for item in public_summary.split("{\"id\":\"").skip(1) {
            let method = item.split('"').next().unwrap();
            if !books::hbiab_reviewed::reviewed(method) {
                assert!(!item.contains("\"stats\":"), "Unreviewed single-session result: {method}");
            }
        }
        for guru in ["chande", "ross"] {
            assert_eq!(crate::summary(&st, &owned_query(&public_query, guru)).unwrap_err(), "empty date range");
        }
        for (from, to) in [
            ("2000-12-20", "2000-12-21"),
            ("2001-01-22", "2001-01-21"),
            ("2001-02-01", "2001-02-02"),
        ] {
            let mut outside = public_query.clone();
            outside.insert("from".into(), from.into());
            outside.insert("to".into(), to.into());
            assert_eq!(crate::summary(&st, &outside).unwrap_err(), "empty date range");
        }
    }
    let legacy = BTreeMap::from([
        ("id".into(), "f1a".into()),
        ("ticker".into(), "SPY".into()),
        ("from".into(), "2001-01-22".into()),
        ("to".into(), "2001-01-22".into()),
    ]);
    assert_eq!(summary(&st, &legacy).unwrap_err(), "empty date range");
    assert_eq!(detail(&st, &legacy).unwrap_err(), "empty date range");

    // A calendar interval between observations is empty, even though the
    // resolver's upper bound uses saturating_sub for ordinary date lookup.
    let mut gap_rows = rows.to_vec();
    gap_rows.last_mut().unwrap().date = "2001-01-24".into();
    let gap_rows: &'static [Row] = Box::leak(gap_rows.into_boxed_slice());
    let gap_state = State {
        ctxs: BTreeMap::from([("SPY".into(), Ctx::new(gap_rows, gap_rows, &[]))]),
        baskets: baskets::Catalogue::default(),
        vix: vec![],
        market_extra: BTreeMap::new(),
    };
    let empty_gap = owned_query(&BTreeMap::from([
        ("ticker".into(), "SPY".into()),
        ("from".into(), "2001-01-22".into()),
        ("to".into(), "2001-01-23".into()),
    ]), "connors");
    assert_eq!(summary(&gap_state, &empty_gap).unwrap_err(), "empty date range");
}
