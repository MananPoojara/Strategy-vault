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
        .filter(|d| !books::hbiab_reviewed::reviewed(d.id))
    {
        assert_eq!(
            instrument_policy::policy(d).requirement,
            "source_review_required"
        );
    }
}
