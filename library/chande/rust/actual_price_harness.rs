//! Diagnostic driver: compiles the real source modules, does not replace their calculations.
#![allow(dead_code)]
#[derive(Clone, Debug)]
pub struct Row { pub ticker: String, pub date: String, pub open: f64, pub high: f64,
    pub low: f64, pub close: f64, pub volume: Option<f64> }
#[path="../../../Context/guru/backend/src/connors.rs"] mod connors;
#[path="../../../Context/guru/backend/src/books/chbta2.rs"] pub mod chbta2;
#[path="../../../Context/guru/backend/src/books/chntt.rs"] pub mod chntt;
mod books {
    pub use crate::{chbta2, chntt};
    use crate::connors::{Ctx, Params, ParamDef, StrategyDef, Trade};
    pub fn catalogue() -> Vec<StrategyDef> { let mut v=chbta2::catalogue(); v.extend(chntt::catalogue()); v }
    pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> { chbta2::family_of(id).or_else(|| chntt::family_of(id)) }
    pub fn params(id: &str) -> Option<Vec<ParamDef>> { chbta2::params(id).or_else(|| chntt::params(id)) }
    pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> { chbta2::run(id,x,from,to,p).or_else(|| chntt::run(id,x,from,to,p)) }
}
fn main() {
    let a: Vec<String> = std::env::args().collect();
    assert_eq!(a.len(), 6, "canonical observed CSV, method, start index, end index, key=value;...");
    let data=std::fs::read_to_string(&a[1]).unwrap();
    let rows:Vec<Row>=data.lines().skip(1).map(|line| {
        let p:Vec<&str>=line.split(',').collect();
        Row { ticker:"OBSERVED".into(), date:p[0].into(), open:p[1].parse().unwrap(),
            high:p[2].parse().unwrap(), low:p[3].parse().unwrap(), close:p[4].parse().unwrap(), volume:None }
    }).collect();
    let x=connors::Ctx::new(&rows,&rows,&[]);
    let p:connors::Params=a[5].split(';').filter(|s|!s.is_empty()).map(|s| {
        let (k,v)=s.split_once('=').unwrap(); (k.into(),v.parse().unwrap())
    }).collect();
    let trades=chbta2::run(&a[2],&x,a[3].parse().unwrap(),a[4].parse().unwrap(),&p).expect("Rejected parameters/range");
    println!("entry\texit\tdir\tentry_px\texit_px\topen\tentry_kind\tweight\tsignal_i\treason");
    for t in trades {
        assert_eq!(t.legs.len(),1); assert!(t.exits.is_empty());
        println!("{}\t{}\t{}\t{:.17}\t{:.17}\t{}\t{}\t{:.17}\t{}\t{}",t.legs[0].i,t.exit_i,t.dir,
            t.legs[0].px,t.exit_px,t.open,t.entry_kind,t.legs[0].w,t.signal_i,t.reason);
    }
}
