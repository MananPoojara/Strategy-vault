#![allow(dead_code)]
// The verifier replaces path markers in a private copy. Calculations are
// imported from the actual repository sources, never copied into this harness.
#[derive(Clone, Debug)]
pub struct Row {
    pub ticker: String, pub date: String, pub open: f64, pub high: f64,
    pub low: f64, pub close: f64, pub volume: Option<f64>,
}
#[path = "@CONNORS@"] mod connors;
#[path = "@SPMT@"] mod spmt;
mod books {
    use crate::connors::{Ctx, Params, ParamDef, StrategyDef, Trade};
    pub fn catalogue() -> Vec<StrategyDef> { crate::spmt::catalogue() }
    pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> { crate::spmt::family_of(id) }
    pub fn params(id: &str) -> Option<Vec<ParamDef>> { crate::spmt::params(id) }
    pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> { crate::spmt::run(id, x, from, to, p) }
}
fn main() {
    let a: Vec<String> = std::env::args().collect();
    if a[1] == "--contracts" {
        for d in spmt::catalogue() {
            let family = spmt::family_of(d.id).unwrap().0;
            let params = spmt::params(d.id).unwrap_or_default().iter()
                .map(|p| format!("{}={}", p.key, p.def)).collect::<Vec<_>>().join(";");
            println!("{}\t{}\t{}\t{}\t{}", d.id, spmt::reviewed_daily(d.id), family,
                params, spmt::hold_reason(d.id).unwrap_or(""));
        }
        return;
    }
    let data = std::fs::read_to_string(&a[1]).unwrap();
    let rows: Vec<Row> = data.lines().skip(1).map(|line| {
        let f: Vec<&str> = line.split(',').collect();
        Row { ticker: "AUDIT".into(), date: f[0].into(), open: f[1].parse().unwrap(), high: f[2].parse().unwrap(),
            low: f[3].parse().unwrap(), close: f[4].parse().unwrap(), volume: None }
    }).collect();
    let x = connors::Ctx::new(&rows, &rows, &[]);
    let params: connors::Params = a[5].split(';').filter(|s| !s.is_empty()).map(|s| {
        let (k, v) = s.split_once('=').unwrap(); (k.into(), v.parse().unwrap())
    }).collect();
    let from: usize = a[3].parse().unwrap(); let to: usize = a[4].parse().unwrap();
    let signals = spmt::daily_signals(&a[2], &x, &params).expect("unreviewed method");
    let mut log = String::from("bar,date,signal\n");
    for i in from..=to { log.push_str(&format!("{},{},{}\n", i, rows[i].date, signals[i])); }
    std::fs::write(&a[6], log).unwrap();
    let trades = spmt::run(&a[2], &x, from, to, &params).unwrap();
    println!("entry,exit,entry_date,exit_date,direction,entry_price,exit_price,entry_fill,exit_fill,entry_signal,exit_signal,open");
    for t in trades {
        assert_eq!(t.legs.len(), 1); assert!(t.exits.is_empty());
        let l = &t.legs[0];
        println!("{},{},{},{},{},{:.17},{:.17},{},{},{},{},{}", l.i, t.exit_i,
            rows[l.i].date, rows[t.exit_i].date, t.dir, l.px, t.exit_px, t.entry_kind,
            if t.open { "mark" } else { "close" }, l.i, t.signal_i, t.open);
    }
}
