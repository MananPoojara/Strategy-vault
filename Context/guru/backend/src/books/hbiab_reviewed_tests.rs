use super::*;
use crate::Row;
fn rows(c: &[f64]) -> Vec<Row> {
    c.iter()
        .enumerate()
        .map(|(i, &v)| Row {
            ticker: "T".into(),
            date: format!("2001-{:02}-{:02}", 1 + i / 28, 1 + i % 28),
            open: v + 1.,
            high: v + 2.,
            low: v - 1.,
            close: v,
            volume: None,
        })
        .collect()
}
fn near(a: f64, b: f64) {
    assert!((a - b).abs() < 1e-11, "{a} vs {b}");
}
#[test]
fn source_period_order_and_terminal_ma_close_entry() {
    let mut c = vec![100.; 20];
    c.extend([99., 110.]);
    let r = rows(&c);
    let x = Ctx::new(&r, &r, &[]);
    let a = run("hbiab_ma5_20", &x, 20, 21, 0., &Params::new()).unwrap();
    assert_eq!(a.trades.len(), 1);
    let t = &a.trades[0];
    assert_eq!((t.legs[0].i, t.open, t.entry_kind), (21, true, "close"));
    let mut c = vec![100.; 8];
    c.extend([110., 80.]);
    let r = rows(&c);
    let x = Ctx::new(&r, &r, &[]);
    let a = run("hbiab_ma7_4", &x, 7, 9, 0., &Params::new()).unwrap();
    assert_eq!(a.trades[0].legs[0].i, 9);
    assert!(a.trades[0].open);
}
#[test]
fn ema_full_seed_warmup_and_strict_ties() {
    let v: Vec<f64> = (1..=40).map(|i| i as f64).collect();
    let e = ema(&v, 12);
    assert!(e[10].is_nan());
    near(e[11], 6.5);
    near(e[12], 7.5);
    let r = rows(&vec![100.; 40]);
    let x = Ctx::new(&r, &r, &[]);
    let states = states("hbiab_macd_example", &x).unwrap();
    assert!(states.first[24].is_nan());
    assert!(states.second[32].is_nan());
    near(states.second[33], 0.);
    assert!(!states.buy.iter().any(|b| *b));
}
#[test]
fn macd_quarter_current_equity_and_actual_cash_conservation() {
    let r = rows(&[100., 110., 120., 100., 110., 120.]);
    let x = Ctx::new(&r, &r, &[]);
    let buy = vec![true, false, false, true, false, false];
    let sell = vec![false, true, false, false, true, false];
    let a = execute(&x, 0, 5, &buy, &sell, 1, 0.25, 0.001).unwrap();
    assert_eq!(a.trades.len(), 2);
    let t = &a.trades[0];
    assert_eq!(
        (t.legs[0].i, t.exit_i, t.signal_i, t.entry_kind),
        (1, 2, 1, "open")
    );
    near(t.legs[0].w, 0.25);
    let audit = &a.trade_audit[0];
    near(audit.quantity, 0.25 / 111.);
    near(audit.entry_cost, 0.00025);
    near(audit.exit_cost, (0.25 / 111.) * 121. * 0.001);
    near(
        a.trade_audit[1].notional,
        a.trade_audit[1].entry_equity * 0.25,
    );
    for d in &a.days {
        near(d.equity, d.cash + d.quantity * x.c[d.i]);
        near(d.gross, d.equity + d.paid);
        assert!(d.cash >= -1e-12);
    }
    near(
        a.days.last().unwrap().equity,
        1. + a.trade_audit.iter().map(|t| t.net).sum::<f64>(),
    );
}
#[test]
fn ma_entry_cost_reserved_open_mark_no_exit_fee_and_pending_orders() {
    let r = rows(&[100., 110., 120.]);
    let x = Ctx::new(&r, &r, &[]);
    let a = execute(
        &x,
        0,
        2,
        &[false, false, true],
        &[false, false, false],
        0,
        1.,
        0.01,
    )
    .unwrap();
    let d = a.days.last().unwrap();
    near(d.cash, 0.);
    near(d.equity, 1. / 1.01);
    near(a.trade_audit[0].exit_cost, 0.);
    assert!(a.trades[0].open);
    let p = execute(
        &x,
        0,
        2,
        &[false, false, true],
        &[false, false, false],
        1,
        0.25,
        0.,
    )
    .unwrap();
    assert!(p.trades.is_empty());
    assert_eq!(p.terminal_pending, Some("buy"));
}
#[test]
fn prewindow_signals_do_not_create_entries_and_invalid_defaults_are_held() {
    let r = rows(&vec![100.; 40]);
    let x = Ctx::new(&r, &r, &[]);
    let mut buy = vec![false; 40];
    buy[34] = true;
    let a = execute(&x, 35, 39, &buy, &vec![false; 40], 1, 0.25, 0.).unwrap();
    assert!(a.trades.is_empty());
    assert!(validate("hbiab_ma7_4", &Params::from([("fast".into(), 4.)])).is_some());
    assert!(validate(
        "hbiab_macd_example",
        &Params::from([("allocation".into(), 1.)])
    )
    .is_some());
    assert!(run("hbiab_mean_reversion", &x, 34, 39, 0., &Params::new()).is_err());
    assert!(run("hbiab_macd_example", &x, 0, 33, 0., &Params::new())
        .unwrap()
        .trades
        .is_empty());
}
#[test]
fn invalid_actual_history_is_never_filled_or_reseeded() {
    let mut r = rows(&vec![100.; 40]);
    r[0].close = f64::NAN;
    let x = Ctx::new(&r, &r, &[]);
    assert!(run("hbiab_ma5_20", &x, 20, 39, 0., &Params::new()).is_err());
}
#[test]
fn arbitrary_constant_decimal_price_preserves_flat_macd_and_account() {
    let r = rows(&vec![0.1283481; 80]);
    let mut r = r;
    for row in &mut r {
        row.open = row.close;
        row.high = row.close;
        row.low = row.close;
    }
    let x = Ctx::new(&r, &r, &[]);
    let s = states("hbiab_macd_example", &x).unwrap();
    assert!(!s.buy.iter().any(|v| *v));
    assert!(!s.sell.iter().any(|v| *v));
    let a = run("hbiab_macd_example", &x, 0, 79, 0., &Params::new()).unwrap();
    assert!(a.trades.is_empty());
    assert!(a.days.iter().all(|d| d.equity == 1. && d.gross == 1.));
    let a = execute(&x, 0, 79, &vec![true; 80], &vec![false; 80], 0, 1., 0.).unwrap();
    assert!(a.days.iter().all(|d| d.equity == 1. && d.gross == 1.));
    let prices: Vec<_> = x.c.iter().map(|&v| decimal::Decimal::new(v)).collect();
    assert_eq!(
        decimal::mean_order(&prices, 7, 7, 4),
        std::cmp::Ordering::Equal
    );
}
#[test]
fn exact_break_even_is_nonwinning_and_flat_gross_has_no_event_noise() {
    let mut r = rows(&[439.78, 440.22, 440.22]);
    for row in &mut r {
        row.open = row.close;
    }
    let x = Ctx::new(&r, &r, &[]);
    let a = execute(
        &x,
        0,
        2,
        &[true, false, false],
        &[false, true, false],
        0,
        0.25,
        0.0005,
    )
    .unwrap();
    assert_eq!(a.trade_audit[0].net, 0.);
    assert_eq!(a.days[1].equity, 1.);
    assert_eq!(a.days[2].equity, 1.);
    let r = rows(&[0.1283481; 4]);
    let x = Ctx::new(&r, &r, &[]);
    let a = execute(
        &x,
        0,
        3,
        &[true, false, true, false],
        &[false, true, false, true],
        0,
        1.,
        0.0005,
    )
    .unwrap();
    assert!(a.days.iter().all(|d| d.gross == 1.));
}
#[test]
fn future_invalid_values_never_affect_selected_prefix() {
    let mut r = rows(&vec![100.; 50]);
    r[45].close = f64::NAN;
    let x = Ctx::new(&r, &r, &[]);
    assert!(run("hbiab_ma5_20", &x, 0, 40, 0., &Params::new()).is_ok());
}
