//! Held futures-accounting research model. Requires explicit observed contract identity,
//! contract count, point value, dated margin, fees and timestamped contract marks.
//! It models variation settlement at each supplied mark; this is a declared adaptation,
//! not a claim about an exchange's actual settlement schedule or an AmiBroker result.
//! AmiBroker describes the distinct margin-deposit and point-value requirements at
//! https://www.amibroker.com/guide/h_futbacktest.html .
//! No public endpoint admits this model without source and instrument review.
#![allow(dead_code)]
use std::collections::{BTreeMap,BTreeSet};
use crate::yt_intraday_rank::{Candidate,rank_batch};

#[derive(Clone,Debug)]
pub(crate) struct Signal {
    pub id:u64,pub symbol:String,pub contract:String,pub entry_time:i64,pub entry_price:f64,
    pub direction:i8,pub score:f64,pub contracts:u32,pub point_value:f64,pub margin_per_contract:f64,
    pub entry_fee:f64,pub expiry:i64,pub exit:Option<(i64,f64,f64)>, // timestamp, price, observed/modelled fee
}
#[derive(Clone,Debug)]
pub(crate) struct Mark {pub time:i64,pub symbol:String,pub contract:String,pub price:f64,pub margin_per_contract:f64}
/// Explicit contract metadata; the caller remains responsible for establishing provenance.
#[derive(Clone,Debug)]
pub(crate) struct ContractSpec {pub symbol:String,pub contract:String,pub expiry:i64,pub contracts:u32,pub point_value:f64,pub entry_fee:f64,pub exit_fee:f64}
#[derive(Debug)]
pub(crate) struct Closed {pub id:u64,pub gross:f64,pub fees:f64,pub net:f64}
#[derive(Debug)]
pub(crate) struct Ledger {pub accepted:Vec<u64>,pub rejected:Vec<u64>,pub closed:Vec<Closed>,pub equity:Vec<(i64,f64)>,pub cash:f64,pub open:Vec<u64>}
struct Position {signal:Signal,last:f64,margin:f64}
fn positive(v:f64)->bool {v.is_finite()&&v>0.}
fn fee(v:f64)->bool {v.is_finite()&&v>=0.}

/// Convert strategy observations only when independent contract-labelled quotes match
/// every source mark. Contract metadata and quote provenance are still subject to
/// admission review. A trade spanning contracts is rejected: no roll is invented.
pub(crate) fn run_observed(signals:&[crate::yt_intraday_portfolio::Signal],source_marks:&[crate::yt_intraday_portfolio::Mark],
    contract_marks:&[Mark],specs:&[ContractSpec],capital:f64,capacity:usize)->Result<Ledger,String> {
    let mut quote_map=BTreeMap::new();
    for m in contract_marks {
        if quote_map.insert((m.time,m.symbol.as_str()),m).is_some(){return Err("duplicate contract-labelled quote".into());}
    }
    let mut spec_map=BTreeMap::new();
    for s in specs {
        if spec_map.insert((s.symbol.as_str(),s.contract.as_str()),s).is_some(){return Err("duplicate futures contract specification".into());}
    }
    let mut admitted_marks=Vec::with_capacity(source_marks.len());let mut used=BTreeSet::new();
    for source in source_marks {
        if !used.insert((source.time,source.symbol.as_str())){return Err("duplicate source minute mark".into());}
        let contract=quote_map.get(&(source.time,source.symbol.as_str())).ok_or_else(||format!("missing contract-labelled quote for {} at {}",source.symbol,source.time))?;
        if !positive(source.price) || !positive(contract.price) || (source.price-contract.price).abs()>0.00011 {
            return Err(format!("contract quote does not match source minute for {} at {}",source.symbol,source.time));
        }
        admitted_marks.push((**contract).clone());
    }
    let mut converted=Vec::with_capacity(signals.len());
    for source in signals {
        if !used.contains(&(source.entry_time,source.symbol.as_str())) {return Err("entry is missing a source minute mark".into());}
        let entry=quote_map.get(&(source.entry_time,source.symbol.as_str())).ok_or("entry lacks contract identity")?;
        let spec=spec_map.get(&(source.symbol.as_str(),entry.contract.as_str())).ok_or("entry lacks contract specification")?;
        let exit=if let Some((time,price))=source.exit {
            if !used.contains(&(time,source.symbol.as_str())){return Err("exit is missing a source minute mark".into());}
            let quote=quote_map.get(&(time,source.symbol.as_str())).ok_or("exit lacks contract identity")?;
            if quote.contract!=entry.contract{return Err("position crosses contracts; explicit roll execution is required".into());}
            Some((time,price,spec.exit_fee))
        } else {None};
        converted.push(Signal{id:source.id,symbol:source.symbol.clone(),contract:entry.contract.clone(),entry_time:source.entry_time,
            entry_price:source.entry_price,direction:source.direction,score:source.score,contracts:spec.contracts,point_value:spec.point_value,
            margin_per_contract:entry.margin_per_contract,entry_fee:spec.entry_fee,expiry:spec.expiry,exit});
    }
    run(&converted,&admitted_marks,capital,capacity)
}

/// Internal strategy-to-futures bridge. It does not bypass the public readiness hold.
pub(crate) fn run_strategy_research(id:&str,contexts:&[(&crate::connors::Ctx,usize,usize)],params:&crate::connors::Params,
    contract_marks:&[Mark],specs:&[ContractSpec],capital:f64,capacity:usize)->Result<Ledger,String> {
    let (signals,marks)=crate::yt_intraday_portfolio::strategy_observations(id,contexts,params)?;
    run_observed(&signals,&marks,contract_marks,specs,capital,capacity)
}

/// All mark times and fills are supplied by the caller. Positions are marked and
/// variation is settled at each supplied timestamp, with margin rechecked afterward.
/// Existing exits settle before ranking new entries. Rejected signals do not close a
/// different accepted trade. Missing marks, contract mismatches, expiry, unsupported
/// funding and same-timestamp entry/exit with no phase ordering all fail closed.
pub(crate) fn run(signals:&[Signal],marks:&[Mark],capital:f64,capacity:usize)->Result<Ledger,String> {
    if !positive(capital) || capacity==0 || capacity>100 {return Err("invalid futures capital or position capacity".into());}
    if signals.len()>100_000 || marks.len()>2_000_000 {return Err("futures research input exceeds bounded limits".into());}
    let mut entries:BTreeMap<i64,Vec<&Signal>>=BTreeMap::new();let mut exits:BTreeMap<i64,Vec<&Signal>>=BTreeMap::new();
    let mut ids=BTreeSet::new();let mut unique=BTreeSet::new();
    for s in signals {
        if !ids.insert(s.id) || s.symbol.is_empty() || s.contract.is_empty() || !unique.insert((s.entry_time,s.symbol.as_str()))
            || !positive(s.entry_price) || !s.score.is_finite() || ![-1,1].contains(&s.direction) || s.contracts==0
            || !positive(s.point_value) || !positive(s.margin_per_contract) || !fee(s.entry_fee) || s.entry_time>s.expiry {
            return Err("invalid or duplicate futures signal or contract specification".into());
        }
        if let Some((t,px,cost))=s.exit {
            if t<=s.entry_time || t>s.expiry || !positive(px) || !fee(cost) {return Err("invalid futures exit, expiry or ordering".into());}
            exits.entry(t).or_default().push(s);
        }
        entries.entry(s.entry_time).or_default().push(s);
    }
    let mut quotes=BTreeMap::new();
    for m in marks {
        if m.symbol.is_empty() || m.contract.is_empty() || !positive(m.price) || !positive(m.margin_per_contract)
            || quotes.insert((m.time,m.symbol.as_str()),m).is_some() {return Err("invalid or duplicate futures mark".into());}
    }
    let times:BTreeSet<i64>=entries.keys().chain(exits.keys()).chain(marks.iter().map(|m|&m.time)).copied().collect();
    let mut positions=BTreeMap::<u64,Position>::new();let mut cash=capital;
    let mut out=Ledger{accepted:Vec::new(),rejected:Vec::new(),closed:Vec::new(),equity:Vec::new(),cash:capital,open:Vec::new()};
    for time in times {
        if let Some(xs)=exits.get(&time) {
            for s in xs {
                if let Some(pos)=positions.remove(&s.id) {
                    let (_,px,exit_fee)=s.exit.unwrap();
                    let factor=f64::from(s.direction)*f64::from(s.contracts)*s.point_value;
                    cash+=(px-pos.last)*factor-exit_fee;
                    let gross=(px-s.entry_price)*factor;let fees=s.entry_fee+exit_fee;
                    out.closed.push(Closed{id:s.id,gross,fees,net:gross-fees});
                }
            }
        }
        // Settle the observed variation on positions already held before sizing
        // new entries at this timestamp. Stale equity cannot fund a new position.
        for pos in positions.values_mut() {
            if time>pos.signal.expiry {return Err("open futures position exceeds supplied contract expiry".into());}
            let mark=quotes.get(&(time,pos.signal.symbol.as_str())).ok_or_else(||format!("missing actual futures mark for {} at {time}",pos.signal.symbol))?;
            if mark.contract!=pos.signal.contract {return Err("futures mark contract identity differs from held contract".into());}
            cash+=(mark.price-pos.last)*f64::from(pos.signal.direction)*f64::from(pos.signal.contracts)*pos.signal.point_value;
            pos.last=mark.price;pos.margin=mark.margin_per_contract*f64::from(pos.signal.contracts);
        }
        let held_margin:f64=positions.values().map(|p|p.margin).sum();
        if !cash.is_finite() || cash<=0. || !held_margin.is_finite() || cash<held_margin {return Err("futures variation or margin requires unsupported additional funding".into());}
        let mut entered=BTreeSet::new();
        if let Some(xs)=entries.get(&time) {
            let candidates:Vec<Candidate>=xs.iter().map(|s|Candidate{timestamp:time,symbol:s.symbol.clone(),score:s.score,priority_size:f64::from(s.contracts)}).collect();
            let map:BTreeMap<&str,&Signal>=xs.iter().map(|s|(s.symbol.as_str(),*s)).collect();
            let mut ranked:Vec<&Candidate>=candidates.iter().collect();rank_batch(&mut ranked);
            for candidate in ranked {
                let s=map[candidate.symbol.as_str()];
                let margin=s.margin_per_contract*f64::from(s.contracts);
                let reserved: f64=positions.values().map(|p|p.margin).sum();
                if !margin.is_finite(){return Err("invalid total margin".into());}
                if positions.len()>=capacity || positions.values().any(|p|p.signal.symbol==s.symbol) || cash-reserved<s.entry_fee+margin {
                    out.rejected.push(s.id);continue;
                }
                cash-=s.entry_fee;out.accepted.push(s.id);
                positions.insert(s.id,Position{signal:s.clone(),last:s.entry_price,margin});
                entered.insert(s.id);
            }
        }
        for (id,pos) in positions.iter_mut() {
            if !entered.contains(id) {continue;}
            if time>pos.signal.expiry {return Err("open futures position exceeds supplied contract expiry".into());}
            let mark=quotes.get(&(time,pos.signal.symbol.as_str())).ok_or_else(||format!("missing actual futures mark for {} at {time}",pos.signal.symbol))?;
            if mark.contract!=pos.signal.contract {return Err("futures mark contract identity differs from held contract".into());}
            cash+=(mark.price-pos.last)*f64::from(pos.signal.direction)*f64::from(pos.signal.contracts)*pos.signal.point_value;
            pos.last=mark.price;pos.margin=mark.margin_per_contract*f64::from(pos.signal.contracts);
        }
        let reserved:f64=positions.values().map(|p|p.margin).sum();
        if !cash.is_finite() || cash<=0. || !reserved.is_finite() || cash<reserved {return Err("futures variation or margin requires unsupported additional funding".into());}
        out.equity.push((time,cash));
    }
    out.cash=cash;out.open=positions.keys().copied().collect();Ok(out)
}

#[cfg(test)] mod tests {
    use super::*;
    fn signal()->Signal {Signal{id:1,symbol:"IDX".into(),contract:"IDX-EXP".into(),entry_time:1,entry_price:100.,direction:1,score:2.,contracts:2,point_value:10.,margin_per_contract:100.,entry_fee:3.,expiry:9,exit:Some((3,110.,4.))}}
    fn mark(t:i64,price:f64)->Mark {Mark{time:t,symbol:"IDX".into(),contract:"IDX-EXP".into(),price,margin_per_contract:100.}}
    #[test] fn futures_variation_margin_and_fee_reconcile() {
        let r=run(&[signal()],&[mark(1,100.),mark(2,105.)],1000.,1).unwrap();
        assert!((r.equity[1].1-1097.).abs()<1e-9);
        assert!((r.closed[0].gross-200.).abs()<1e-9 && (r.closed[0].fees-7.).abs()<1e-9);
        assert!((r.cash-1193.).abs()<1e-9);
    }
    #[test] fn missing_contract_mark_or_expiry_fails_closed() {
        assert!(run(&[signal()],&[],1000.,1).unwrap_err().contains("missing actual futures mark"));
        let mut m=mark(1,100.);m.contract="OTHER".into();
        assert!(run(&[signal()],&[m],1000.,1).unwrap_err().contains("identity"));
        let mut s=signal();s.expiry=2;
        assert!(run(&[s],&[mark(1,100.)],1000.,1).unwrap_err().contains("expiry"));
    }
    #[test] fn variation_is_settled_before_new_margin_is_admitted() {
        let mut a=signal();a.exit=Some((3,65.,0.));
        let b=Signal{id:2,symbol:"OTHER".into(),contract:"OTHER-EXP".into(),entry_time:2,entry_price:100.,direction:1,score:1.,contracts:2,point_value:10.,margin_per_contract:100.,entry_fee:0.,expiry:9,exit:Some((3,100.,0.))};
        let r=run(&[a,b],&[mark(1,100.),mark(2,65.)],1000.,2).unwrap();
        assert_eq!(r.accepted,vec![1]);assert_eq!(r.rejected,vec![2]);
        assert!((r.cash-297.).abs()<1e-9);
    }
    #[test] fn strategy_bridge_requires_matching_contract_quotes_and_no_inferred_roll() {
        use crate::yt_intraday_portfolio::{Signal as SourceSignal,Mark as SourceMark};
        let s=SourceSignal{id:1,symbol:"IDX".into(),entry_time:1,entry_price:100.,score:2.,direction:1,exit:Some((3,110.))};
        let source=[SourceMark{time:1,symbol:"IDX".into(),price:100.},SourceMark{time:2,symbol:"IDX".into(),price:105.},SourceMark{time:3,symbol:"IDX".into(),price:110.}];
        let mut quotes=[mark(1,100.),mark(2,105.),mark(3,110.)];
        let specs=[ContractSpec{symbol:"IDX".into(),contract:"IDX-EXP".into(),expiry:9,contracts:2,point_value:10.,entry_fee:3.,exit_fee:4.}];
        let result=run_observed(&[s.clone()],&source,&quotes,&specs,1000.,1).unwrap();
        assert!((result.cash-1193.).abs()<1e-9);
        quotes[1].price=106.;
        assert!(run_observed(&[s.clone()],&source,&quotes,&specs,1000.,1).unwrap_err().contains("does not match"));
        quotes[1].price=105.;quotes[2].contract="NEXT".into();
        assert!(run_observed(&[s],&source,&quotes,&specs,1000.,1).unwrap_err().contains("crosses contracts"));
    }
}
