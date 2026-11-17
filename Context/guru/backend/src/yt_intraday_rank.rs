//! Draft intraday candidate ranking and position-capacity selector.
//! This module feeds the separate research ledger; it does not publish a portfolio.
//! Source-specific scores, instrument admission and typed strategy events require integration.
#![allow(dead_code)]
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug)]
pub(crate) struct Candidate {
    pub timestamp:i64,
    pub symbol:String,
    pub score:f64,
    /// Comparable source position-size value in a common sizing mode.
    pub priority_size:f64,
}
#[derive(Clone, Debug)]
pub(crate) struct Exit {
    pub timestamp:i64,
    pub symbol:String,
}
#[derive(Debug, PartialEq)]
pub(crate) struct Selection {
    pub accepted:Vec<(i64,String)>,
    pub rejected:Vec<(i64,String)>,
}

/// Order one timestamp's already-validated candidates. The caller must supply only
/// genuinely simultaneous signals and a source-reviewed score convention.
pub(crate) fn rank_batch<'a>(events:&mut Vec<&'a Candidate>) {
    events.sort_by(|a,b|b.score.abs().total_cmp(&a.score.abs()).then_with(||b.priority_size.total_cmp(&a.priority_size)).then_with(||a.symbol.cmp(&b.symbol)));
}

/// Select candidates at each actual timestamp, processing exits before entries. Larger absolute
/// scores rank first, matching AmiBroker's documented default PositionScore ordering
/// (https://www.amibroker.com/guide/h_ranking.html). Equal scores use source position size,
/// then symbol order (https://www.amibroker.com/kb/2015/09/28/symbol-selection-when-positionscore-is-not-defined/).
/// AFLs enabling separate long/short ranking need a different contract.
pub(crate) fn select(candidates:&[Candidate], exits:&[Exit], max_positions:usize)->Result<Selection,&'static str> {
    if max_positions==0 {return Err("portfolio capacity must be positive");}
    let mut by_time:BTreeMap<i64,Vec<&Candidate>>=BTreeMap::new();
    let mut exits_by_time:BTreeMap<i64,Vec<&Exit>>=BTreeMap::new();
    let mut seen=BTreeSet::new();
    for c in candidates {
        if c.symbol.is_empty() || !c.score.is_finite() || !c.priority_size.is_finite() || c.priority_size<=0. || !seen.insert((c.timestamp,c.symbol.as_str())) {return Err("candidate must have a unique timestamp/symbol, finite score and positive size");}
        by_time.entry(c.timestamp).or_default().push(c);
    }
    for e in exits {
        if e.symbol.is_empty() {return Err("exit must name a symbol");}
        exits_by_time.entry(e.timestamp).or_default().push(e);
    }
    let times:BTreeSet<i64>=by_time.keys().chain(exits_by_time.keys()).copied().collect();
    let mut open=BTreeSet::<String>::new();let mut result=Selection{accepted:Vec::new(),rejected:Vec::new()};
    for time in times {
        if let Some(events)=exits_by_time.get(&time) {for e in events {open.remove(&e.symbol);}}
        if let Some(events)=by_time.get_mut(&time) {
            rank_batch(events);
            for c in events {
                if open.len()<max_positions && !open.contains(&c.symbol) {
                    open.insert(c.symbol.clone());result.accepted.push((time,c.symbol.clone()));
                } else {result.rejected.push((time,c.symbol.clone()));}
            }
        }
    }
    Ok(result)
}

#[cfg(test)] mod tests {
    use super::*;
    #[test] fn ranks_same_time_and_reuses_exited_capacity() {
        let c=vec![Candidate{timestamp:1,symbol:"B".into(),score:2.,priority_size:1.},Candidate{timestamp:1,symbol:"A".into(),score:-3.,priority_size:1.},Candidate{timestamp:2,symbol:"C".into(),score:1.,priority_size:1.}];
        let r=select(&c,&[Exit{timestamp:2,symbol:"A".into()}],1).unwrap();
        assert_eq!(r.accepted,vec![(1,"A".into()),(2,"C".into())]);assert_eq!(r.rejected,vec![(1,"B".into())]);
        assert!(select(&[Candidate{timestamp:1,symbol:"A".into(),score:f64::NAN,priority_size:1.}],&[],1).is_err());
    }
    #[test] fn equal_scores_prefer_larger_position_size_before_symbol() {
        let c=vec![Candidate{timestamp:1,symbol:"A".into(),score:2.,priority_size:1.},Candidate{timestamp:1,symbol:"Z".into(),score:-2.,priority_size:2.}];
        assert_eq!(select(&c,&[],1).unwrap().accepted,vec![(1,"Z".into())]);
    }
}
