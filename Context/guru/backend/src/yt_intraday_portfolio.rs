//! Timestamped intraday portfolio research ledger. This is a declared fixed-slot,
//! normalized-notional model, not a futures margin, contract or AmiBroker reproduction.
//! It is not exposed as a published backtest: source-qualified candidate generation,
//! scores, instrument admission and complete actual marks must be reviewed first.
#![allow(dead_code)]
use std::collections::{BTreeMap, BTreeSet};
use crate::yt_intraday_rank::{rank_batch,Candidate};

#[derive(Clone, Debug)]
pub(crate) struct Signal {
    pub id:u64,
    pub symbol:String,
    pub entry_time:i64,
    pub entry_price:f64,
    pub score:f64,
    pub direction:i8,
    /// An actual observed exit; None represents a position still open at the end.
    pub exit:Option<(i64,f64)>,
}
#[derive(Clone, Debug)]
pub(crate) struct Mark {pub time:i64,pub symbol:String,pub price:f64}
#[derive(Clone, Debug)]
pub(crate) struct Closed {pub id:u64,pub symbol:String,pub entry_time:i64,pub exit_time:i64,pub allocation:f64,pub gross:f64,pub fees:f64,pub net:f64}
#[derive(Debug)]
pub(crate) struct Ledger {pub closed:Vec<Closed>,pub accepted:Vec<u64>,pub rejected:Vec<u64>,pub equity:Vec<(i64,f64)>,pub cash:f64,pub open:Vec<u64>}
struct Position {signal:Signal,allocation:f64,entry_fee:f64}

/// Build observed marks and close-phase fills without parsing a display string.
/// Used only by source AFLs whose default PositionScore is undefined and whose
/// default position sizes are equal; score zero selects alphabetical tie-breaking.
pub(crate) fn close_observations(x:&crate::connors::Ctx,from:usize,to:usize,m:&crate::intraday::Minutes,b:&crate::intraday::Candles,
    timed:Vec<(crate::connors::Trade,usize,usize)>,first_id:u64)->Result<(Vec<Signal>,Vec<Mark>),String> {
    if x.r.is_empty() || from>to || to>=x.r.len(){return Err("invalid selected daily coverage".into());}
    crate::intraday::validate_daily_carrier(m,x.r,to)?;
    let stamp=|date:&str,hm:u16,phase:i64|->Result<i64,String>{
        if !crate::baskets::valid_date(date) || hm/100>23 || hm%100>59 {return Err("invalid source date or minute".into());}
        Ok(crate::day_number(date)*2880+i64::from(hm/100*60+hm%100)*2+phase)
    };
    let mut marks=Vec::new();
    for i in 0..m.len() {
        let date=m.date(i);if date<x.r[from].date.as_str() || date>x.r[to].date.as_str(){continue;}
        if marks.len()>=2_000_000 {return Err("selected source exceeds the bounded intraday mark limit".into());}
        marks.push(Mark{time:stamp(date,m.hhmm[i],0)?,symbol:x.r[0].ticker.clone(),price:crate::intraday::price(m.o[i])});
        marks.push(Mark{time:stamp(date,m.hhmm[i],1)?,symbol:x.r[0].ticker.clone(),price:crate::intraday::price(m.c[i])});
    }
    let mut out=Vec::with_capacity(timed.len());
    for (i,(t,entry,exit)) in timed.into_iter().enumerate() {
        let when=|k:usize|->Result<i64,String>{
            let mindex=*b.end.get(k).ok_or("source candle index is invalid")?;
            if mindex>=m.len(){return Err("source minute index is invalid".into());}
            stamp(m.date(mindex),m.hhmm[mindex],1)
        };
        let direction=if t.dir==1. {1}else if t.dir == -1. {-1}else{return Err("source direction is invalid".into());};
        if t.legs.len()!=1 || !t.exits.is_empty() {return Err("multi-leg portfolio execution is not supported".into());}
        let entry_close=*b.c.get(entry).ok_or("source entry candle is invalid")?;
        let exit_close=*b.c.get(exit).ok_or("source exit candle is invalid")?;
        if (t.legs[0].px-entry_close).abs()>0.00011 || (t.exit_px-exit_close).abs()>0.00011 {
            return Err("source fill is not the observed candle close; event timing requires review".into());
        }
        out.push(Signal{id:first_id.checked_add(i as u64).ok_or("portfolio signal ID overflow")?,symbol:x.r[0].ticker.clone(),
            entry_time:when(entry)?,entry_price:t.legs[0].px,score:0.,direction,exit:if t.open {None}else{Some((when(exit)?,t.exit_px))}});
    }
    Ok((out,marks))
}

/// Explicit, bounded research bridge for the source-scored one-minute Gap Down AFLs.
/// The caller must first enforce the normal instrument/readiness checks; this function
/// is deliberately not registered as a public result endpoint. No symbol is silently
/// omitted, and missing cross-symbol marks cause the ledger to fail closed.
pub(crate) fn run_gap_research(id:&str,contexts:&[(&crate::connors::Ctx,usize,usize)],params:&crate::connors::Params,
    capital:f64,capacity:usize,fee_rate:f64)->Result<Ledger,String> {
    if ![crate::books::ytgd::MINUTE_TIME,crate::books::ytgd::CONFIRM_POS,crate::books::ytgd::CONFIRM_NEG,crate::books::ytgd::CONFIRM_NONE].contains(&id) {
        return Err("strategy is not a supported gap research adapter".into());
    }
    run_strategy_research(id,contexts,params,capital,capacity,fee_rate)
}

/// Internal strategy-to-ledger dispatch. Only explicitly connected source adapters are admitted.
/// Normal instrument policy and readiness gates must be applied by a caller before any public use.
pub(crate) fn run_strategy_research(id:&str,contexts:&[(&crate::connors::Ctx,usize,usize)],params:&crate::connors::Params,
    capital:f64,capacity:usize,fee_rate:f64)->Result<Ledger,String> {
    let (signals,marks)=strategy_observations(id,contexts,params)?;
    run(&signals,&marks,capital,capacity,fee_rate)
}

/// Build events without selecting positions, for use by a separately admitted ledger.
pub(crate) fn strategy_observations(id:&str,contexts:&[(&crate::connors::Ctx,usize,usize)],params:&crate::connors::Params)
    ->Result<(Vec<Signal>,Vec<Mark>),String> {
    if contexts.is_empty() || contexts.len()>8 {return Err("intraday research requires one to eight explicitly selected symbols".into());}
    let mut symbols=BTreeSet::new();let mut signals=Vec::new();let mut marks=Vec::new();let mut next_id=0u64;
    for &(ctx,from,to) in contexts {
        let symbol=ctx.r.first().ok_or("empty source context")?.ticker.as_str();
        if !symbols.insert(symbol) {return Err("duplicate source symbol".into());}
        let (mut new_signals,mut new_marks)=if crate::books::ytsti::IDS.contains(&id) {
            crate::books::ytsti::portfolio_observations(id,ctx,from,to,params,next_id)?
        } else if crate::books::yt30m::IDS.contains(&id) {
            crate::books::yt30m::portfolio_observations(id,ctx,from,to,params,next_id)?
        } else if crate::books::yt2ds::INTRADAY_IDS.contains(&id) {
            crate::books::yt2ds::intraday_portfolio_observations(id,ctx,from,to,params,next_id)?
        } else if crate::books::ytrsi_ieod::IDS.contains(&id) {
            crate::books::ytrsi_ieod::portfolio_observations(id,ctx,from,to,params,next_id)?
        } else if id==crate::books::ytnth::ID {
            crate::books::ytnth::portfolio_observations(id,ctx,from,to,params,next_id)?
        } else if id==crate::books::ytorb_draft::ID {
            crate::books::ytorb_draft::portfolio_observations(id,ctx,from,to,params,next_id)?
        } else if id==crate::books::ytgd::SHOCK_IEOD {
            crate::books::ytgd::shock_portfolio_observations(ctx,from,to,params,next_id)?
        } else {
            crate::books::ytgd::gap_portfolio_observations(id,ctx,from,to,params,next_id)?
        };
        next_id=next_id.checked_add(new_signals.len() as u64).ok_or("portfolio signal ID overflow")?;
        if signals.len()+new_signals.len()>100_000 || marks.len()+new_marks.len()>2_000_000 {return Err("intraday research exceeds bounded portfolio input limits".into());}
        signals.append(&mut new_signals);marks.append(&mut new_marks);
    }
    Ok((signals,marks))
}

/// All fills, scores and marks must be observed inputs. Allocation is a fixed share of
/// initial capital per slot including its entry fee; notional is reserved for long and short positions. Costs
/// are proportional to actual entry and exit notional. Simultaneous existing exits are
/// settled before ranked entries. A same-timestamp entry/exit is rejected because the
/// event phase is not supplied. Rejected entries cannot later close an accepted position.
pub(crate) fn run(signals:&[Signal],marks:&[Mark],capital:f64,capacity:usize,fee_rate:f64)->Result<Ledger,String> {
    if !capital.is_finite() || capital<=0. || capacity==0 || capacity>100 || !fee_rate.is_finite() || !(0. ..=0.5).contains(&fee_rate) {
        return Err("invalid portfolio capital, capacity or fee rate".into());
    }
    if signals.len()>100_000 || marks.len()>2_000_000 {return Err("intraday portfolio input exceeds bounded research limits".into());}
    let mut entries:BTreeMap<i64,Vec<&Signal>>=BTreeMap::new();let mut exits:BTreeMap<i64,Vec<&Signal>>=BTreeMap::new();
    let mut seen=BTreeSet::new();let mut same_symbol=BTreeSet::new();
    for s in signals {
        if !seen.insert(s.id) || s.symbol.is_empty() || !same_symbol.insert((s.entry_time,s.symbol.as_str())) || !s.entry_price.is_finite() || s.entry_price<=0. || !s.score.is_finite() || ![-1,1].contains(&s.direction) {
            return Err("invalid or duplicate timestamped portfolio signal".into());
        }
        if let Some((time,price))=s.exit {
            if time<=s.entry_time || !price.is_finite() || price<=0. {return Err("exit requires a later observed timestamp and positive fill".into());}
            exits.entry(time).or_default().push(s);
        }
        entries.entry(s.entry_time).or_default().push(s);
    }
    let mut quotes=BTreeMap::new();
    for m in marks {
        if m.symbol.is_empty() || !m.price.is_finite() || m.price<=0. || quotes.insert((m.time,m.symbol.as_str()),m.price).is_some() {return Err("invalid or duplicate timestamped portfolio mark".into());}
    }
    let times:BTreeSet<i64>=entries.keys().chain(exits.keys()).chain(marks.iter().map(|m|&m.time)).copied().collect();
    let slot=capital/capacity as f64;let allocation=slot/(1.+fee_rate);let mut cash=capital;let mut positions=BTreeMap::<u64,Position>::new();
    let mut result=Ledger{closed:Vec::new(),accepted:Vec::new(),rejected:Vec::new(),equity:Vec::new(),cash:capital,open:Vec::new()};
    for time in times {
        if let Some(events)=exits.get(&time) {
            for s in events {
                if let Some(pos)=positions.remove(&s.id) {
                    let px=s.exit.unwrap().1;
                    let gross=pos.allocation*f64::from(s.direction)*(px/s.entry_price-1.);
                    let exit_fee=pos.allocation*(px/s.entry_price)*fee_rate;
                    let released=pos.allocation+gross-exit_fee;
                    if !released.is_finite() || released<0. {return Err("position requires unsupported additional margin or funding".into());}
                    cash+=released;
                    result.closed.push(Closed{id:s.id,symbol:s.symbol.clone(),entry_time:s.entry_time,exit_time:time,allocation:pos.allocation,gross,fees:pos.entry_fee+exit_fee,net:gross-pos.entry_fee-exit_fee});
                }
            }
        }
        if let Some(events)=entries.get(&time) {
            let candidates:Vec<Candidate>=events.iter().map(|s|Candidate{timestamp:time,symbol:s.symbol.clone(),score:s.score,priority_size:1.}).collect();
            let source:BTreeMap<&str,&Signal>=events.iter().map(|s|(s.symbol.as_str(),*s)).collect();
            let mut ranked:Vec<&Candidate>=candidates.iter().collect();rank_batch(&mut ranked);
            for candidate in ranked {
                let s=source[candidate.symbol.as_str()];
                let fee=slot-allocation;
                if positions.len()>=capacity || positions.values().any(|p|p.signal.symbol==s.symbol) || cash<slot {
                    result.rejected.push(s.id);continue;
                }
                cash-=slot;result.accepted.push(s.id);
                positions.insert(s.id,Position{signal:s.clone(),allocation,entry_fee:fee});
            }
        }
        let mut equity=cash;
        for pos in positions.values() {
            let price=quotes.get(&(time,pos.signal.symbol.as_str())).ok_or_else(||format!("missing actual mark for {} at {time}",pos.signal.symbol))?;
            equity+=pos.allocation*(1.+f64::from(pos.signal.direction)*(price/pos.signal.entry_price-1.));
        }
        if !equity.is_finite() || equity<=0. {return Err("portfolio equity requires unsupported insolvency or funding treatment".into());}
        result.equity.push((time,equity));
    }
    result.cash=cash;result.open=positions.keys().copied().collect();Ok(result)
}

#[cfg(test)] mod tests {
    use super::*;
    #[test] fn local_research_dispatch_bridges_strategy_events_when_configured() {
        let Ok(csv)=std::env::var("TEST_INTRADAY_DAILY") else{return};
        if std::env::var_os("INTRADAY_DATA_DIR").is_none(){return;}
        let text=std::fs::read_to_string(csv).unwrap();
        let rows:Vec<crate::Row>=text.lines().skip(1).map(|line|{let v:Vec<_>=line.split(',').collect();crate::Row{
            ticker:"BANKNIFTY".into(),date:v[0].into(),open:v[1].parse().unwrap(),high:v[2].parse().unwrap(),low:v[3].parse().unwrap(),close:v[4].parse().unwrap(),volume:v[5].parse().ok()}}).collect();
        let from=rows.iter().position(|r|r.date.as_str()>="2019-01-01").unwrap();
        let to=rows.iter().rposition(|r|r.date.as_str()<="2019-01-31").unwrap();
        let x=crate::connors::Ctx::new(&rows,&rows,&[]);
        for id in [crate::books::yt30m::NORMAL_INTRADAY,crate::books::yt2ds::INTRADAY_NTH,crate::books::ytrsi_ieod::ID,crate::books::ytnth::ID,crate::books::ytorb_draft::ID,crate::books::ytgd::SHOCK_IEOD] {
            let result=run_strategy_research(id,&[(&x,from,to)],&crate::connors::Params::new(),1000.,1,0.);
            assert!(result.is_ok(),"{id}: {}",result.unwrap_err());
        }
    }
    fn s(id:u64,symbol:&str,score:f64,exit:Option<(i64,f64)>)->Signal {Signal{id,symbol:symbol.into(),entry_time:1,entry_price:100.,score,direction:1,exit}}
    #[test] fn ranks_and_accounts_only_accepted_positions() {
        let a=s(1,"A",3.,Some((3,110.)));let b=s(2,"B",2.,Some((2,80.)));
        let marks=[Mark{time:1,symbol:"A".into(),price:100.},Mark{time:2,symbol:"A".into(),price:105.}];
        let r=run(&[a,b],&marks,1000.,1,0.).unwrap();
        assert_eq!(r.accepted,vec![1]);assert_eq!(r.rejected,vec![2]);assert_eq!(r.closed.len(),1);
        assert!((r.cash-1100.).abs()<1e-9);assert!((r.equity[1].1-1050.).abs()<1e-9);
    }
    #[test] fn refuses_missing_marks_and_ambiguous_same_time_exits() {
        assert!(run(&[s(1,"A",1.,Some((2,110.)))],&[],1000.,1,0.).unwrap_err().contains("missing actual mark"));
        assert!(run(&[s(1,"A",1.,Some((1,110.)))],&[],1000.,1,0.).unwrap_err().contains("later observed"));
    }
    #[test] fn stop_paths_without_observed_execution_phase_remain_held() {
        let rows=vec![crate::Row{ticker:"X".into(),date:"2022-01-03".into(),open:100.,high:101.,low:99.,close:100.,volume:None}];
        let x=crate::connors::Ctx::new(&rows,&rows,&[]);
        let p=crate::connors::Params::from([("hard_pct".into(),1.)]);
        assert!(crate::books::ytorb_draft::portfolio_observations(crate::books::ytorb_draft::ID,&x,0,0,&p,0).unwrap_err().contains("timing"));
        let p=crate::connors::Params::from([("hsl_pct".into(),1.)]);
        assert!(crate::books::ytrsi_ieod::portfolio_observations(crate::books::ytrsi_ieod::ID,&x,0,0,&p,0).unwrap_err().contains("timing"));
        let p=crate::connors::Params::from([("trail_pct".into(),1.)]);
        assert!(crate::books::ytgd::shock_portfolio_observations(&x,0,0,&p,0).unwrap_err().contains("timing"));
    }
    #[test] fn short_profit_and_both_fee_sides_reconcile() {
        let mut a=s(1,"A",1.,Some((2,90.)));a.direction=-1;
        let short=run(&[a],&[Mark{time:1,symbol:"A".into(),price:100.}],1000.,1,0.01).unwrap();
        let unit=1000./1.01;
        assert!((short.closed[0].gross-unit*0.1).abs()<1e-9);assert!((short.closed[0].fees-unit*0.019).abs()<1e-9);
        assert!((short.cash-(1000.+unit*0.081)).abs()<1e-9);
        let r=run(&[s(2,"A",1.,Some((2,110.)))],&[Mark{time:1,symbol:"A".into(),price:100.}],1000.,2,0.01).unwrap();
        let unit=500./1.01;
        assert!((r.closed[0].gross-unit*0.1).abs()<1e-9);assert!((r.closed[0].fees-unit*0.021).abs()<1e-9);
        assert!((r.cash-(1000.+unit*0.079)).abs()<1e-9);
    }
    #[test] fn exited_slot_is_reused_at_same_timestamp_by_ranked_candidate() {
        let a=s(1,"A",1.,Some((2,100.)));
        let b=Signal{id:2,symbol:"B".into(),entry_time:2,entry_price:50.,score:3.,direction:1,exit:Some((3,55.))};
        let c=Signal{id:3,symbol:"C".into(),entry_time:2,entry_price:50.,score:2.,direction:1,exit:Some((3,45.))};
        let marks=[Mark{time:1,symbol:"A".into(),price:100.},Mark{time:2,symbol:"B".into(),price:50.}];
        let r=run(&[a,b,c],&marks,1000.,1,0.).unwrap();
        assert_eq!(r.accepted,vec![1,2]);assert_eq!(r.rejected,vec![3]);
        assert_eq!(r.closed.iter().map(|x|x.id).collect::<Vec<_>>(),vec![1,2]);
        assert!((r.cash-1100.).abs()<1e-9);
    }
}
