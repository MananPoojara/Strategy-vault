//! Synthetic arithmetic/report-boundary regressions, not market performance.
use super::*;
use crate::{
    connors::{Leg, Params},
    Row,
};
fn rows(values: &[f64]) -> Vec<Row> {
    values
        .iter()
        .enumerate()
        .map(|(i, &c)| Row {
            ticker: "SPX".into(),
            date: format!("2001-01-{:02}", i + 1),
            open: c,
            high: c,
            low: c,
            close: c,
            volume: None,
        })
        .collect()
}
fn trade(entry: usize, exit: usize, e: f64, x: f64, open: bool) -> Trade {
    Trade {
        dir: 1.,
        legs: vec![Leg {
            i: entry,
            px: e,
            w: 1.,
        }],
        exit_i: exit,
        exit_px: x,
        reason: "synthetic close".into(),
        entry_kind: "close",
        signal_i: exit,
        open,
        exits: vec![],
    }
}
#[test]
fn dema_exact_economic_zero_is_flat_nonwinning_and_keeps_gross_distinct() {
    let r = rows(&[100., 99., 99.099]);
    let x = Ctx::new(&r, &r, &[]);
    let costs = Costs {
        fees_bps: 0.,
        slippage_bps: 5.,
        rollover_bps: 0.,
    };
    let run = crate::backtest_costs(&x, ID, 0, 2, costs, &Params::new()).unwrap();
    let a = run.unit_exposure_report.as_ref().unwrap();
    assert_eq!(run.trades.len(), 1);
    assert_eq!(a.trade_returns[0].net_return.to_bits(), 0);
    assert_eq!(a.trade_returns[0].net_contribution.to_bits(), 0);
    assert_eq!(a.days[2].equity, 1.);
    assert_eq!(a.days[2].gross_equity, 1.001);
    let s = crate::financial_stats(&x, &run, costs);
    assert_eq!(s["trades"], 1.);
    assert_eq!(s["win_rate"], 0.);
    assert_eq!(s["closed_return_sum"], 0.);
    assert_eq!(s["max_consec_losses"], 1.);
    assert!(run.source_account.is_none() && run.bandy_account.is_none());
    assert!(
        crate::backtest(&x, ID, 0, 2, costs.side_rate(), &Params::new())
            .unwrap_err()
            .contains("separate fee/slippage")
    );
    assert!(
        run.trades[0].ret(costs.side_rate()) > 0.,
        "The preserved legacy helper must still expose the known discrepancy"
    );
}
#[test]
fn dema_subnormal_nav_retains_all_event_checks_without_legacy_return_veto() {
    let tiny = f64::from_bits(1);
    let r = rows(&[1., tiny]);
    let x = Ctx::new(&r, &r, &[]);
    let t = trade(0, 1, 1., tiny, false);
    assert!(crate::trade_validation::check(std::slice::from_ref(&t), 0, 1, 0.).is_err());
    crate::trade_validation::check_structure(std::slice::from_ref(&t), 0, 1).unwrap();
    let report = build(&x, &[t.clone()], 0, 1, Costs::default()).unwrap();
    assert_eq!(report.days[1].equity.to_bits(), 1);
    assert_eq!(report.trade_returns[0].net_return, -1.);
    let mut invalid = t.clone();
    invalid.legs[0].w = 2.;
    assert!(crate::trade_validation::check_structure(&[invalid], 0, 1).is_err());
    let mut invalid = t;
    invalid.open = true;
    assert!(crate::trade_validation::check_structure(&[invalid.clone(), invalid], 0, 1).is_err());
}
#[test]
fn dema_terminal_estimate_and_positive_factor_domain_are_explicit() {
    let r = rows(&[100., 99.]);
    let x = Ctx::new(&r, &r, &[]);
    let c = Costs {
        fees_bps: 2.,
        slippage_bps: 3.,
        rollover_bps: 0.,
    };
    let run = crate::backtest_costs(&x, ID, 1, 1, c, &Params::new()).unwrap();
    let a = run.unit_exposure_report.as_ref().unwrap();
    assert!(a.trade_returns[0].open);
    assert_eq!(a.trade_returns[0].net_return, -0.001);
    assert_eq!(run.equity[0], 0.999);
    assert_eq!(a.days[0].gross_equity, 1.);
    assert_eq!(crate::financial_stats(&x, &run, c)["trades"], 0.);
    assert!(a
        .trade_accounting_json(&x, &run.trades[0], 0)
        .unwrap()
        .contains("open_liquidation_estimate"));
    let domain = Costs {
        fees_bps: 5000.,
        slippage_bps: 0.,
        rollover_bps: 0.,
    };
    assert!(crate::backtest_costs(&x, ID, 1, 1, domain, &Params::new())
        .unwrap_err()
        .starts_with("unit_exposure_equity_domain"));
}
#[test]
fn dema_unit_detail_metadata_and_all_statistics_share_one_policy() {
    let r = rows(&[100., 99., 99.099]);
    let x = Ctx::new(&r, &r, &[]);
    let c = Costs {
        fees_bps: 0.,
        slippage_bps: 5.,
        rollover_bps: 0.,
    };
    let run = crate::backtest_costs(&x, ID, 0, 2, c, &Params::new()).unwrap();
    let report = run.unit_exposure_report.as_ref().unwrap();
    let output = crate::unit_exposure_detail::render(&x, &run, c).unwrap();
    assert!(output.contains("\"unit_exposure_report\":"));
    assert!(!output.contains("\"source_account\":"));
    assert!(output.contains("\"ret\":0,\"ret_full_precision\":0"));
    assert!(output.contains(&crate::esc(BASIS)));
    assert!(report.json().contains(&crate::esc(BASIS)));
    assert_eq!(
        report
            .common_stats(&x, &run.trades, 0, 2, View::Gross)
            .len(),
        36
    );
    assert_eq!(crate::financial_stats(&x, &run, c).len(), 45);
    assert_eq!(crate::benchmark_stats(&x, &run).len(), 7);
    let d = crate::books::hbqts::catalogue()
        .into_iter()
        .find(|d| d.id == ID)
        .unwrap();
    assert_eq!(crate::instrument_policy::policy(&d).source_basis, BASIS);
    assert_eq!(
        crate::books::hbqts::reviewed::DEMA_INDEX_CONVENTION,
        CONVENTION
    );
}

#[test]
fn dema_frozen_twenty_five_safe_statistic_wire_tokens() {
    for(bits,expected)in[
        (0x0000000000000000, "0"),
        (0x8000000000000000, "-0"),
        (0x3e7ad7f29abcaf48, "0"),
        (0xbe7ad7f29abcaf48, "-0"),
        (0x3ea0c6f7a0b5ed8d, "0.000001"),
        (0xbea0c6f7a0b5ed8d, "-0.000001"),
        (0x3ea0c6f7a0b5ed8c, "0"),
        (0x3ea0c6f7a0b5ed8e, "0.000001"),
        (0xbea0c6f7a0b5ed8c, "-0"),
        (0xbea0c6f7a0b5ed8e, "-0.000001"),
        (0x3ff3c0c9d9d3458d, "1.234568"),
        (0xbff3c0c9d9d3458d, "-1.234568"),
        (0x0000000000000001, "0"),
        (0x8000000000000001, "-0"),
        (0x0010000000000000, "0"),
        (0x8010000000000000, "-0"),
        (0x7fe1ccf385ebc8a0, "100000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000"),
        (0xffe1ccf385ebc8a0, "-100000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000"),
        (0x7fefffffffffffff, "179769313486231570000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000"),
        (0xffefffffffffffff, "-179769313486231570000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000"),
        (0x7eb0c6f7a0b5ed8c, "179769313486231540000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000"),
        (0x7eb0c6f7a0b5ed8d, "179769313486231600000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000"),
        (0x7ff8000000000000, "null"),
        (0x7ff0000000000000, "null"),
        (0xfff0000000000000, "null"),
    ]{assert_eq!(statistic_number(f64::from_bits(bits)),expected);}
}
