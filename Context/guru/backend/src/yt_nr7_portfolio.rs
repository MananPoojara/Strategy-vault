//! Internal NR7 stock portfolio research. Not an admitted/public AmiBroker reproduction.
//! Source signals run independently, as in the AFL, even when the portfolio rejects an entry.
//! Equity-percent sizing uses the previous calendar bar's closing equity, source lot size is one,
//! and insufficient cash shrinks orders. Missing held-stock marks fail closed.
#![allow(dead_code)]
use std::collections::{BTreeMap, BTreeSet};
use crate::connors::{Ctx, Params};
use crate::books::ytnr7;

#[derive(Clone, Copy, Debug)]
pub(crate) enum Size { Shares(u32), EquityPercent(f64), FixedValue(f64) }
#[derive(Clone, Copy, Debug)]
pub(crate) struct Settings { pub capital: f64, pub max_positions: usize, pub size: Size, pub commission_pct: f64 }
#[derive(Clone, Debug)]
pub(crate) struct Bar { pub date: String, pub close: f64, pub buy: bool, pub sell: bool }
#[derive(Debug)]
pub(crate) struct Series { pub symbol: String, pub bars: Vec<Bar> }
#[derive(Debug)]
pub(crate) struct Fill { pub date: String, pub symbol: String, pub shares: u64, pub price: f64, pub entry: bool }
#[derive(Debug)]
pub(crate) struct ResultLedger { pub equity: Vec<(String, f64)>, pub fills: Vec<Fill>, pub cash: f64 }

/// Preserve raw Buy/Sell arrays rather than regenerating source state from accepted trades.
pub(crate) fn observe(symbol: &str, x: &Ctx<'_>, params: &Params) -> Result<Series, &'static str> {
    if symbol.is_empty() || x.r.len()!=x.c.len() { return Err("invalid source identity or series"); }
    let s=ytnr7::source_signals(ytnr7::NR7_ST,x,params).ok_or("NR7 signal unavailable")?;
    let bars=x.r.iter().enumerate().map(|(i,r)| Bar {date:r.date.clone(),close:x.c[i],buy:s.entry[i].is_some(),sell:s.exit[i].is_some()}).collect();
    Ok(Series {symbol:symbol.into(),bars})
}

/// Use source-hash-checked, dated AUX values as a research entry filter. The
/// original NR7 AFL lacks this condition; existing unfiltered runs are unchanged.
pub(crate) fn observe_with_aux(catalogue:&crate::baskets::Catalogue,key:&str,native_root:&std::path::Path,x:&Ctx<'_>,params:&Params,rule:crate::yt_aux::Rule)->Result<Series,String> {
    let symbol=x.r.first().ok_or("empty stock series")?.ticker.as_str();
    if x.r.len()!=x.c.len() {return Err("invalid source series".into());}
    let aux=catalogue.yt_aux(key,native_root,x.r)?;
    let eligible:Vec<_>=aux.into_iter().map(|a|rule.allows(a)).collect();
    let s=ytnr7::source_signals_with_eligibility(ytnr7::NR7_ST,x,params,Some(&eligible)).ok_or("NR7 signal unavailable")?;
    let bars=x.r.iter().enumerate().map(|(i,r)|Bar{date:r.date.clone(),close:x.c[i],buy:s.entry[i].is_some(),sell:s.exit[i].is_some()}).collect();
    Ok(Series{symbol:symbol.into(),bars})
}

/// `members` identifies the supplied symbol inventory; a dated AUX rule can be
/// applied independently to each entry by `observe_with_aux`.
/// The chosen calendar and account assumptions are research inputs, not source verification.
pub(crate) fn run(series: &[Series], members: &BTreeSet<String>, from: &str, to: &str, cfg: Settings) -> Result<ResultLedger, &'static str> {
    if from>to || members.is_empty() || members.len()>5000 || !cfg.capital.is_finite() || cfg.capital<=0. || cfg.max_positions==0 || cfg.max_positions>5000 || !cfg.commission_pct.is_finite() || !(0.0..=100.0).contains(&cfg.commission_pct) {return Err("invalid portfolio settings");}
    match cfg.size { Size::Shares(n) if n>0 => (), Size::EquityPercent(n) if n.is_finite() && n>0. && n<=100. => (), Size::FixedValue(n) if n.is_finite() && n>0. => (), _=>return Err("invalid position size") }
    let mut sources=BTreeMap::new();
    let mut dates=BTreeSet::new();
    for s in series {
        if !members.contains(&s.symbol) {continue;}
        if sources.contains_key(s.symbol.as_str()) || s.bars.windows(2).any(|w|w[0].date>=w[1].date) {return Err("duplicate symbol or unordered data");}
        let mut bars=BTreeMap::new();
        for bar in &s.bars {
            if !bar.close.is_finite() || bar.close<=0. {return Err("invalid stock close");}
            if bar.date.as_str()>=from && bar.date.as_str()<=to {dates.insert(bar.date.clone());}
            bars.insert(bar.date.as_str(),bar);
        }
        sources.insert(s.symbol.as_str(),bars);
    }
    if sources.len()!=members.len() || dates.is_empty() {return Err("watchlist member or selected observations unavailable");}
    let fee=cfg.commission_pct/100.;
    let (mut cash,mut prior_equity)=(cfg.capital,cfg.capital);
    let mut positions:BTreeMap<&str,u64>=BTreeMap::new();
    let mut out=ResultLedger {equity:Vec::new(),fills:Vec::new(),cash};
    for date in dates {
        let day=date.as_str();
        for (&symbol,&qty) in &positions {
            let bar=sources[symbol].get(day).ok_or("held stock is missing a close mark")?;
            if bar.buy && bar.sell {return Err("ambiguous same-bar buy and sell");}
            if qty==0 {return Err("invalid holding");}
        }
        let exits:Vec<_>=positions.keys().copied().filter(|symbol|sources[symbol].get(day).is_some_and(|b|b.sell)).collect();
        for symbol in exits {
            let qty=positions.remove(symbol).unwrap(); let price=sources[symbol][day].close;
            cash+=qty as f64*price*(1.-fee);
            out.fills.push(Fill {date:date.clone(),symbol:symbol.into(),shares:qty,price,entry:false});
        }
        for (&symbol,bars) in &sources {
            let Some(bar)=bars.get(day) else {continue};
            if !bar.buy || positions.contains_key(symbol) || positions.len()>=cfg.max_positions {continue;}
            if bar.sell {return Err("ambiguous same-bar buy and sell");}
            let desired=match cfg.size {Size::Shares(n)=>n as f64,Size::EquityPercent(p)=>prior_equity*p/100./bar.close,Size::FixedValue(v)=>v/bar.close};
            let affordable=cash/(bar.close*(1.+fee));
            let shares=desired.min(affordable).floor();
            if !shares.is_finite() || shares>u64::MAX as f64 {return Err("position exceeds supported size");}
            let shares=shares as u64;
            if shares==0 {continue;}
            cash-=shares as f64*bar.close*(1.+fee);
            positions.insert(symbol,shares);
            out.fills.push(Fill {date:date.clone(),symbol:symbol.into(),shares,price:bar.close,entry:true});
        }
        let mut equity=cash;
        for (&symbol,&qty) in &positions {equity+=qty as f64*sources[symbol].get(day).ok_or("held stock is missing a close mark")?.close;}
        if !cash.is_finite() || cash < -1e-8 || !equity.is_finite() || equity<=0. {return Err("unsupported account state");}
        prior_equity=equity;
        out.equity.push((date,equity));
    }
    out.cash=cash; Ok(out)
}

#[cfg(test)] mod tests {
    use super::*;
    fn b(date:&str,close:f64,buy:bool,sell:bool)->Bar {Bar{date:date.into(),close,buy,sell}}
    fn config(size:Size,max_positions:usize)->Settings {Settings{capital:1000.,max_positions,size,commission_pct:0.}}
    #[test] fn prior_equity_sizes_new_entries_and_watchlist_filters() {
        let src=vec![Series{symbol:"A".into(),bars:vec![b("1",10.,true,false),b("2",20.,false,false),b("3",20.,false,true)]},Series{symbol:"B".into(),bars:vec![b("1",10.,false,false),b("2",10.,true,false),b("3",10.,false,false)]},Series{symbol:"C".into(),bars:vec![b("1",10.,true,false),b("2",10.,false,false),b("3",10.,false,false)]}];
        let members=["A".into(),"B".into()].into_iter().collect();
        let r=run(&src,&members,"1","3",config(Size::EquityPercent(25.),2)).unwrap();
        assert_eq!(r.fills.iter().filter(|f|f.entry).map(|f|(&*f.symbol,f.shares)).collect::<Vec<_>>(),vec![("A",25),("B",25)]);
        assert_eq!(r.equity[1].1,1250.);
        assert!(!r.fills.iter().any(|f|f.symbol=="C"));
    }
    #[test] fn shares_cash_shrinking_fees_and_capacity() {
        let src=vec![Series{symbol:"A".into(),bars:vec![b("1",100.,true,false),b("2",100.,false,true)]},Series{symbol:"B".into(),bars:vec![b("1",100.,true,false),b("2",100.,false,false)]}];
        let m=["A".into(),"B".into()].into_iter().collect();
        let r=run(&src,&m,"1","2",Settings{commission_pct:1.,..config(Size::Shares(20),1)}).unwrap();
        assert_eq!(r.fills.len(),2); assert_eq!(r.fills[0].shares,9); assert!((r.cash-982.).abs()<1e-8);
    }
    #[test] fn missing_member_or_held_mark_fails() {
        let src=vec![Series{symbol:"A".into(),bars:vec![b("1",10.,true,false)]},Series{symbol:"B".into(),bars:vec![b("2",10.,false,false)]}];
        let m=["A".into(),"B".into()].into_iter().collect();
        assert!(run(&src,&m,"1","2",config(Size::Shares(1),2)).is_err());
    }
    #[test] fn nr7_source_observations_drive_the_ledger() {
        let rows=crate::books::ytnr7::tests::wave(420);
        let ctx=Ctx::new(&rows,&rows,&[]);
        let observed=observe("ACTUAL",&ctx,&Params::new()).unwrap();
        let expected=ytnr7::run(ytnr7::NR7_ST,&ctx,0,rows.len()-1,&Params::new()).unwrap();
        assert!(!expected.is_empty());
        let members=["ACTUAL".into()].into_iter().collect();
        let r=run(&[observed],&members,&rows[0].date,&rows[rows.len()-1].date,config(Size::FixedValue(1000.),1)).unwrap();
        let entries:Vec<_>=r.fills.iter().filter(|f|f.entry).collect();
        assert_eq!(entries.len(),expected.len());
        for (fill,trade) in entries.iter().zip(expected) {assert_eq!(fill.date,rows[trade.legs[0].i].date); assert_eq!(fill.price,trade.legs[0].px);}
    }
}
