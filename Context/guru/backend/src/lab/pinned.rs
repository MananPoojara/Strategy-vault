//! Compatibility pin (Phase 7 spec §8): the trade lists of every Customize template and of four plain v1 specs, hashed on
//! fixed synthetic data at Phase 6 HEAD. Any change to what an existing spec means changes the hash.
use super::bars::Bars;
use super::rules::parse_spec;
use super::series::{compile, Cache, Frames};
use super::simulate::simulate;
use super::templates::{parsed, TEMPLATES};
use super::testdata::walk;

/// FNV-1a over every field of every trade.
fn fold(h: &mut u64, bytes: &[u8]) { for b in bytes { *h ^= u64::from(*b); *h = h.wrapping_mul(0x100_0000_01b3); } }
fn digest(specs: &[super::rules::Spec]) -> (u64, usize) {
    let (mut h, mut n) = (0xcbf2_9ce4_8422_2325u64, 0);
    for seed in 1..=4u64 {
        let b: Bars = walk(1500, seed, true);
        let fr = Frames::new(b.clone());
        for s in specs {
            for t in simulate(&compile(s, &fr, &mut Cache::new()).unwrap(), &b, 0, b.len() - 1) {
                for l in &t.legs { fold(&mut h, &(l.i as u64).to_le_bytes()); fold(&mut h, &l.px.to_bits().to_le_bytes()); fold(&mut h, &l.w.to_bits().to_le_bytes()); }
                fold(&mut h, &(t.exit_i as u64).to_le_bytes()); fold(&mut h, &t.exit_px.to_bits().to_le_bytes());
                fold(&mut h, &t.dir.to_bits().to_le_bytes()); fold(&mut h, &[t.open as u8]); fold(&mut h, t.reason.as_bytes());
                fold(&mut h, t.entry_kind.as_bytes()); fold(&mut h, &(t.signal_i as u64).to_le_bytes());
                n += 1;
            }
        }
    }
    (h, n)
}
pub const PLAIN: [&str; 4] = [
    r#"{"v":1,"entry":{"all":[{"lhs":{"s":"rsi","p":{"length":2}},"op":"crosses_below","rhs":{"value":10}},{"lhs":{"s":"close"},"op":">","rhs":{"s":"sma","p":{"length":40},"tf":"W"}}]},"exit":{"any":[{"lhs":{"s":"rsi","p":{"length":2}},"op":"crosses_above","rhs":{"value":70}},{"lhs":{"s":"bars_held"},"op":">=","rhs":{"value":10}}]}}"#,
    r#"{"v":1,"entry":{"all":[{"lhs":{"s":"rsi","p":{"length":2}},"op":"<","rhs":{"value":20}}]},"exit":{"any":[],"stop_loss":{"pct":2},"target":{"pct":3}},"execution":{"fill":"close"}}"#,
    r#"{"v":1,"entry":{"all":[{"lhs":{"s":"close"},"op":">","rhs":{"s":"sma","p":{"length":5}}}]},"exit":{"any":[{"lhs":{"s":"bars_held"},"op":">=","rhs":{"value":8}}],"trailing":{"atr":2,"atr_length":10}}}"#,
    r#"{"v":1,"direction":"short","entry":{"all":[{"lhs":{"s":"macd","out":"signal","tf":"M","offset":1},"op":"<","rhs":{"s":"macd","tf":"M"}},{"lhs":{"s":"rsi","p":{"length":2}},"op":">","rhs":{"value":80}}],"limit":{"atr":0.4,"atr_length":10}},"exit":{"any":[{"lhs":{"s":"rsi","p":{"length":2}},"op":"<","rhs":{"value":40}}],"stop_loss":{"atr":3}},"execution":{"exit_fill":"close"}}"#,
];

#[test] fn existing_specs_keep_their_exact_trades() {
    let templates: Vec<_> = TEMPLATES.iter().map(parsed).collect();
    let plain: Vec<_> = PLAIN.iter().map(|t| parse_spec(t).unwrap()).collect();
    assert_eq!((digest(&templates), digest(&plain)), ((8551371247103554118, 4422), (14900454072103790734, 1366)), "pinned at Phase 6 HEAD 53d7308; a change here means an existing spec changed meaning");
}

/// AmiBroker trade prices and delays at today's settings give today's trades: each PLAIN spec in the legacy form and
/// in `execution.prices` / `delays` (the pin above shows the legacy form itself is unchanged).
#[test] fn amibroker_prices_at_todays_settings_reproduce_todays_trades() {
    let same = |legacy: &str, prices: &str| {
        let (a, b) = (parse_spec(legacy).unwrap(), parse_spec(prices).unwrap());
        assert!(b.exec.market.is_some() && a.exec.market.is_none());
        let (x, y) = (digest(std::slice::from_ref(&a)), digest(std::slice::from_ref(&b)));
        assert!(x.1 > 0 && x == y, "{prices}");
    };
    let next_open = r#""execution":{"prices":{"buy":"open","sell":"open"},"delays":{"buy":1,"sell":1}}"#;
    let with_execution = |spec: &str, execution: &str| format!("{},{execution}}}", spec.strip_suffix('}').unwrap());
    same(PLAIN[0], &with_execution(PLAIN[0], next_open));
    same(PLAIN[1], &PLAIN[1].replace(r#""execution":{"fill":"close"}"#, r#""execution":{"prices":{"buy":"close","sell":"close"},"delays":{"buy":0,"sell":0}}"#));
    let delayed = with_execution(PLAIN[2], r#""execution":{"delay_bars":3}"#);
    same(&delayed, &delayed.replace(r#""delay_bars":3"#, r#""delays":{"buy":3,"sell":3}"#));
    let both = r#"{"v":1,"direction":"both","entry":{"all":[{"lhs":{"s":"rsi","p":{"length":2}},"op":"<","rhs":{"value":20}}]},"exit":{"any":[{"lhs":{"s":"bars_held"},"op":">=","rhs":{"value":6}}],"stop_loss":{"pct":3}},"short_side":{"entry":{"all":[{"lhs":{"s":"rsi","p":{"length":2}},"op":">","rhs":{"value":85}}]},"exit":{"any":[{"lhs":{"s":"bars_held"},"op":">=","rhs":{"value":4}}]}},"execution":{"fill":"close"}}"#;
    same(both, &both.replace(r#""fill":"close""#, r#""prices":{"buy":"close","sell":"close","short":"close","cover":"close"},"delays":{"buy":0,"sell":0,"short":0,"cover":0}"#));
    same(&both.replace(r#""fill":"close""#, r#""delay_bars":2"#), &both.replace(r#""fill":"close""#, r#""delays":{"buy":2,"sell":2,"short":2,"cover":2}"#));
    same(PLAIN[3], &PLAIN[3].replace(r#""execution":{"exit_fill":"close"}"#, r#""execution":{"prices":{"cover":"close"},"delays":{"cover":0}}"#));
}
