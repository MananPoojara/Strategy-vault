//! Documented-example fixtures (Phase 7 spec §10.3): published AmiBroker and TradingView rules written as lab specs, run on
//! crafted bars, with the trades the platform's documented semantics give, computed by hand. They exercise declared lab adaptations; they verify no book and no platform output (nothing is exported from either).
//! Plus a trade-level no-lookahead test for six representative Phase 7 combinations; money/futures have separate tests.
use super::bars::Bars;
use super::rules::parse_spec;
use super::series::{compile, Cache, Frames};
use super::simulate::simulate;
use super::testdata::{closes, from_ohlc};
use crate::connors::Trade;

fn run(spec: &str, b: &Bars) -> Vec<Trade> { simulate(&compile(&parse_spec(spec).unwrap(), &Frames::new(b.clone()), &mut Cache::new()).unwrap(), b, 0, b.len() - 1) }
/// (entry bar, entry price, exit bar, exit price, direction, open)
fn rows(t: &[Trade]) -> Vec<(usize, f64, usize, f64, f64, bool)> { t.iter().map(|t| (t.legs[0].i, (t.legs[0].px * 100.).round() / 100., t.exit_i, (t.exit_px * 100.).round() / 100., t.dir, t.open)).collect() }

/// E1 — AmiBroker guide stop-and-reverse pattern (https://www.amibroker.com/guide/h_backtest.html).
/// Fixture adaptation: 3-bar SMA instead of the guide's 45-bar EMA, with one-bar delays and all fills at open.
#[test] fn e1_amibroker_stop_and_reverse_ma_cross() {
    let spec = r#"{"v":1,"direction":"both","entry":{"all":[{"lhs":{"s":"close"},"op":"crosses_above","rhs":{"s":"sma","p":{"length":3}}}]},"exit":{"any":[]},
        "short_side":{"entry":{"all":[{"lhs":{"s":"close"},"op":"crosses_below","rhs":{"s":"sma","p":{"length":3}}}]},"exit":{"any":[]}}}"#;
    // crosses above on bar 3, below on bar 5, above on bar 8; fills at the next open (open = close here)
    let t = run(spec, &closes(&[10., 10., 10., 13., 16., 13., 10., 7., 10., 13., 16.]));
    assert_eq!(rows(&t), vec![(4, 16., 6, 10., 1., false), (6, 10., 9, 13., -1., false), (9, 13., 10, 16., 1., true)]);
}
/// E2 — AmiBroker KB "Handling limit orders in the backtester": `Buy = Ref(cond, -1) AND L < limit; BuyPrice = Min(O, limit)`
/// with limit = 1% below the signal close (https://www.amibroker.com/kb/2014/11/26/handling-limit-orders-in-the-backtester/).
/// The fixture avoids L == limit (AmiBroker's `<` against the lab's touch).
#[test] fn e2_amibroker_limit_order_fills_at_the_limit_or_the_gapped_open() {
    let mut bars = vec![(100., 101., 99., 100.); 10];
    bars[3] = (99.5, 100., 98.5, 99.8); // touches the 99 limit: filled at 99
    bars[7] = (97., 98., 96., 97.5); // opens below the limit: filled at the open, 97
    let spec = r#"{"v":1,"entry":{"all":[{"any":[{"lhs":{"s":"calendar","out":"bar_index"},"op":"=","rhs":{"value":2}},{"lhs":{"s":"calendar","out":"bar_index"},"op":"=","rhs":{"value":6}}]}],"limit":{"pct":1}},
        "exit":{"any":[{"lhs":{"s":"bars_held"},"op":">=","rhs":{"value":2}}]},"execution":{"exit_fill":"close"}}"#;
    assert_eq!(rows(&run(spec, &from_ohlc(&bars))), vec![(3, 99., 5, 100., 1., false), (7, 97., 9, 100., 1., false)]);
}
/// E3 — AmiBroker ApplyStop (https://www.amibroker.com/guide/afl/applystop.html): `ApplyStop(stopTypeLoss, stopModePercent, 2,
/// 1)` checks the intraday stop; the lab fills gaps at the open. `ApplyStop(stopTypeNBar,
/// stopModeBars, 5)` motivates the time exit. Fixture convention: bars_held >= 5, close entries/exits, no stop on entry close.
/// N-bar counting and every broker setting are not established platform parity.
#[test] fn e3_amibroker_percent_stop_and_n_bar_stop() {
    let mut bars = vec![(100., 101., 99., 100.); 15];
    bars[3] = (99.5, 100., 97.5, 98.5); // touches the 98 stop
    bars[6] = (96., 97., 95., 96.5); // gaps below the 98 stop: out at the open
    let spec = r#"{"v":1,"entry":{"all":[{"any":[{"lhs":{"s":"calendar","out":"bar_index"},"op":"=","rhs":{"value":1}},{"lhs":{"s":"calendar","out":"bar_index"},"op":"=","rhs":{"value":5}},{"lhs":{"s":"calendar","out":"bar_index"},"op":"=","rhs":{"value":8}}]}]},
        "exit":{"any":[{"lhs":{"s":"bars_held"},"op":">=","rhs":{"value":5}}],"stop_loss":{"pct":2}},"execution":{"fill":"close","exit_fill":"close"}}"#;
    let t = run(spec, &from_ohlc(&bars));
    assert_eq!(rows(&t), vec![(1, 100., 3, 98., 1., false), (5, 100., 6, 96., 1., false), (8, 100., 13, 100., 1., false)]);
    assert_eq!((t[1].reason.as_str(), t[2].reason.as_str()), ("exit.stop_loss (gap at the open)", "exit.any[0]: bars_held >= 5"));
}
/// E4 — TradingView built-in "Channel BreakOut Strategy" (length 5): every bar, `strategy.entry("ChBrkLE", long, stop =
/// ta.highest(high, 5) + mintick)` once `close[5]` exists, and `strategy.entry("ChBrkSE", short, stop = ta.lowest(low, 5) -
/// mintick)`; an opposite entry reverses. Official source: https://www.tradingview.com/support/solutions/43000599828-channel-breakout-strategy/ (checked 2026-10-15); fixture mintick 0.01.
#[test] fn e4_tradingview_channel_breakout() {
    let mut bars = vec![(100., 101., 99., 100.); 6];
    bars.extend([(100., 103., 99.5, 102.5), (102.5, 104., 101., 103.), (103., 103.5, 100., 100.5), (100.5, 101., 98., 98.5), (98.5, 99., 97., 97.5),
                 (97.5, 98., 96., 96.5), (96.5, 97., 95., 95.5), (95.5, 96., 94., 95.), (95., 102., 94.5, 101.5), (101.5, 102., 101., 101.8)]);
    let spec = r#"{"v":1,"direction":"both",
        "entry":{"all":[{"lhs":{"s":"close","offset":5},"op":">","rhs":{"value":0}}],"stop":{"at":{"add":[{"s":"highest","p":{"length":5}},{"value":0.01}]}}},"exit":{"any":[]},
        "short_side":{"entry":{"all":[{"lhs":{"s":"close"},"op":">","rhs":{"value":0}}],"stop":{"at":{"sub":[{"s":"lowest","p":{"length":5}},{"value":0.01}]}}},"exit":{"any":[]}}}"#;
    let t = run(spec, &from_ohlc(&bars));
    // long stop 101.01 fills on bar 6; the short stop 98.99 reverses on bar 9; the long stop 101.01 (from bar 13) reverses on bar 14
    assert_eq!(rows(&t), vec![(6, 101.01, 9, 98.99, 1., false), (9, 98.99, 14, 101.01, -1., false), (14, 101.01, 15, 101.8, 1., true)]);
    assert_eq!((t[0].entry_kind, t[1].entry_kind), ("stop", "reverse"));
}
/// E5 — TradingView built-in "Consecutive Up/Down Strategy" (3 and 3): long after 3 higher closes, short after 3 lower closes,
/// market orders at the next open, reversing. `ups >= 3` is a strictly rising run of 3 (lab `rising`, bars 3).
/// Official source: https://www.tradingview.com/support/solutions/43000599836-consecutive-up-down-strategy/ (checked 2026-10-15).
#[test] fn e5_tradingview_consecutive_up_down() {
    let spec = r#"{"v":1,"direction":"both","entry":{"all":[{"lhs":{"s":"close"},"op":"rising","bars":3}]},"exit":{"any":[]},
        "short_side":{"entry":{"all":[{"lhs":{"s":"close"},"op":"falling","bars":3}]},"exit":{"any":[]}}}"#;
    let t = run(spec, &closes(&[10., 11., 12., 13., 14., 13., 12., 11., 10., 11., 12., 13.]));
    assert_eq!(rows(&t), vec![(4, 14., 8, 10., 1., false), (8, 10., 11, 13., -1., true)]); // the last bar's long signal has no fill
}
/// E6 — Pine User Manual, Strategies: `strategy.exit(profit = 100, loss = 50)` in ticks, and `trail_points` / `trail_offset`
/// (https://www.tradingview.com/pine-script-docs/concepts/strategies/), on a futures fixture with tick 0.05. TradingView's
/// broker emulator takes the open's nearer extreme first; the lab's default takes the stop first (spec §6.3).
#[test] fn e6_tradingview_bracket_and_trailing_in_ticks() {
    use crate::lab::futures::{run as fut, Contract};
    let contract = |rows: &[(f64, f64, f64, f64)]| {
        let b = from_ohlc(rows);
        Contract { symbol: "F".into(), expiry: b.date[b.len() - 1].clone(), lot: 1., tick: 0.05, point_value: 1., margin_pct: 10., scheduled_sessions: b.date.clone(), bars: b, oi: None }
    };
    let bracket = |intrabar: &str| format!(r#"{{"v":1,"entry":{{"all":[{{"lhs":{{"s":"calendar","out":"bar_index"}},"op":"=","rhs":{{"value":1}}}}]}},"exit":{{"any":[],"target":{{"ticks":100}},"stop_loss":{{"ticks":50}}}},
        "execution":{{"fill":"close","intrabar":"{intrabar}","capital":100000,"sizing":{{"units":1}},"instrument":{{"kind":"futures"}}}}}}"#);
    let c = contract(&[(100., 100., 100., 100.), (100., 100., 100., 100.), (104., 106., 97., 102.), (102., 102., 102., 102.)]);
    // entry 100: target 105, stop 97.5; bar 2 opens nearer its high (O → H → L → C)
    let tv = fut(&[c.clone()], &parse_spec(&bracket("ohlc_path")).unwrap(), 1e5).unwrap();
    assert_eq!((tv.rows[0].0.exit_px, tv.rows[0].0.reason.as_str()), (105., "exit.target"));
    let lab = fut(&[c], &parse_spec(&bracket("stop_first")).unwrap(), 1e5).unwrap();
    assert_eq!((lab.rows[0].0.exit_px, lab.rows[0].0.reason.as_str()), (97.5, "exit.stop_loss"));
    // trail_points 60 ticks (3 points) arms it, trail_offset 40 ticks (2 points) trails; bar 3 opens nearer its low, so
    // TradingView meets the low (101) before a new high, against the trail from bar 2's high: 103.5 − 2 = 101.5
    let trail = r#"{"v":1,"entry":{"all":[{"lhs":{"s":"calendar","out":"bar_index"},"op":"=","rhs":{"value":1}}]},"exit":{"any":[],"trailing":{"ticks":40,"activate":{"ticks":60}}},
        "execution":{"fill":"close","capital":100000,"sizing":{"units":1},"instrument":{"kind":"futures"}}}"#;
    let c = contract(&[(100., 100., 100., 100.), (100., 100., 100., 100.), (100., 103.5, 99.5, 103.), (102., 104., 101., 103.), (103., 103., 103., 103.)]);
    let r = fut(&[c], &parse_spec(trail).unwrap(), 1e5).unwrap();
    assert_eq!((r.rows[0].0.exit_i, r.rows[0].0.exit_px, r.rows[0].0.reason.as_str()), (3, 101.5, "exit.trailing"));
}

/// Representative Phase 7 combinations: perturbing every bar after k never changes a trade closed by k.
#[test] fn phase7_features_never_use_later_bars() {
    let rsi_lo = r#"{"lhs":{"s":"rsi","p":{"length":2}},"op":"<","rhs":{"value":25}}"#;
    let rsi_hi = r#"{"lhs":{"s":"rsi","p":{"length":2}},"op":">","rhs":{"value":75}}"#;
    let held = |n: u32| format!(r#"{{"lhs":{{"s":"bars_held"}},"op":">=","rhs":{{"value":{n}}}}}"#);
    let specs = [
        format!(r#"{{"v":1,"direction":"both","entry":{{"all":[{rsi_lo}]}},"exit":{{"any":[]}},"short_side":{{"entry":{{"all":[{rsi_hi}]}},"exit":{{"any":[]}}}}}}"#),
        format!(r#"{{"v":1,"direction":"both","entry":{{"all":[{rsi_lo}],"stop":{{"at":{{"s":"highest","p":{{"length":3}}}}}},"valid_bars":3}},"exit":{{"any":[{}]}},"short_side":{{"entry":{{"all":[{rsi_hi}],"stop":{{"pct":0.5}},"limit":{{"pct":0}}}},"exit":{{"any":[{}]}}}}}}"#, held(6), held(6)),
        format!(r#"{{"v":1,"entry":{{"all":[{rsi_lo}]}},"exit":{{"any":[{}]}},"execution":{{"delay_bars":2,"entry_on":"edge"}}}}"#, held(4)),
        format!(r#"{{"v":1,"entry":{{"all":[{rsi_lo}]}},"exit":{{"any":[{}],"stop_loss":{{"pct":3}},"trailing":{{"pct":2,"activate":{{"pct":1}}}},"breakeven":{{"after":{{"pct":1.5}}}},"take_profits":[{{"pct":1,"size":0.3}},{{"atr":2,"size":0.3}}]}},"execution":{{"fill":"close","intrabar":"ohlc_path"}}}}"#, held(12)),
        format!(r#"{{"v":1,"entry":{{"all":[{rsi_lo}],"scale_in":{{"legs":[0.4,0.3,0.3],"price":"better"}}}},"exit":{{"any":[{rsi_hi}],"stop_loss":{{"pct":6}}}},"execution":{{"sizing":{{"risk_pct":3}},"cooldown":{{"bars":3,"after":"loss"}},"max_trades":{{"count":2,"per":"M"}},"risk":{{"max_consecutive_losses":2,"pause_bars":10}}}}}}"#),
        format!(r#"{{"v":1,"entry":{{"all":[{{"then":[{rsi_lo},{{"lhs":{{"s":"close"}},"op":">","rhs":{{"s":"close","tf":"Q","offset":1,"expand":"first"}}}}],"within":10}},{{"lhs":{{"bars_since":{rsi_hi}}},"op":">","rhs":{{"value":2}}}}]}},"exit":{{"any":[{{"lhs":{{"sum":{{"s":"rsi","p":{{"length":2}}}},"bars":2}},"op":">","rhs":{{"value":140}}}},{}]}}}}"#, held(15)),
    ];
    let full = super::testdata::walk(400, 7, false);
    let close_by = |t: &[Trade], k: usize| t.iter().filter(|t| !t.open && t.exit_i <= k).map(|t| (t.legs.iter().map(|l| (l.i, l.px.to_bits(), l.w.to_bits())).collect::<Vec<_>>(), t.exit_i, t.exit_px.to_bits(), t.dir.to_bits(), t.exits.iter().map(|e|(e.0,e.1.to_bits(),e.2.to_bits())).collect::<Vec<_>>(), t.signal_i, t.entry_kind, t.reason.clone())).collect::<Vec<_>>();
    for text in &specs {
        let want = run(text, &full);
        assert!(want.len() > 3, "{text}: only {} trades", want.len());
        for k in (30usize..398).step_by(3) {
            let mut alt = full.clone();
            for i in k + 1..alt.len() { for v in [&mut alt.open, &mut alt.high, &mut alt.low, &mut alt.close] { v[i] *= 0.7; } }
            assert_eq!(close_by(&run(text, &alt), k), close_by(&want, k), "{text}: bars after {k} changed earlier trades");
        }
    }
}
