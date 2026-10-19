//! Synthetic futures-chain research model. Published schedules select roll dates; observed rows never infer expiry.
//! Only fixture callers reach this model until actual contract data is admitted by the HTTP API.
#![allow(dead_code)] // Contract simulation is fixture-only until admitted chain data can reach the API.
use super::bars::Bars;
use super::catalogue::{IndicatorDef,Needs,Output,Params,Placement};
use super::ledger::{self,Contract as Terms,Money,Row};
use super::rules::{Adjust,LabError,Roll,Sizing,Spec};
use super::series::{self,Cache,Frames};
use crate::connors::{Leg,Trade};

#[derive(Clone,Debug)]
pub struct Contract {
    pub symbol:String,pub expiry:String,pub scheduled_sessions:Vec<String>,pub lot:f64,pub tick:f64,
    pub point_value:f64,pub margin_pct:f64,pub bars:Bars,pub oi:Option<Vec<f64>>,
}
#[derive(Debug,Default)]
pub struct Continuous {pub bars:Bars,pub held:Vec<usize>,pub off:Vec<f64>,pub factor:Vec<f64>,pub rolls:Vec<(usize,f64)>}
impl Continuous {pub fn actual(&self,i:usize,px:f64)->f64 {(px-self.off[i])/self.factor[i]}}
fn invalid(s:impl Into<String>)->LabError {LabError::Invalid(format!("futures chain: {}",s.into()))}
fn missing(s:impl Into<String>)->LabError {LabError::NeedsData(format!("futures chain: {}",s.into()))}
fn date_ok(d:&str)->bool {d.len()==10 && d.bytes().enumerate().all(|(i,c)|if i==4||i==7 {c==b'-'} else {c.is_ascii_digit()}) && super::resample::civil(super::resample::day_number(d))==d}
fn validate(chain:&[Contract])->Result<(),LabError> {
    if chain.is_empty() {return Err(missing("no contracts"));}
    if chain.windows(2).any(|w|w[0].expiry>=w[1].expiry) {return Err(invalid("contracts must be ordered by expiry"));}
    for (k,c) in chain.iter().enumerate() {
        if c.symbol.is_empty() || chain[..k].iter().any(|p|p.symbol==c.symbol) {return Err(invalid("contract symbols must be unique and nonempty"));}
        if ![c.lot,c.tick,c.point_value,c.margin_pct].iter().all(|n|n.is_finite()&&*n>0.) || c.lot.fract()!=0. || c.margin_pct>100. {return Err(invalid(format!("{} has invalid lot/tick/point value/margin",c.symbol)));}
        if !date_ok(&c.expiry) || c.scheduled_sessions.is_empty() || c.scheduled_sessions.iter().any(|d|!date_ok(d)) || c.scheduled_sessions.windows(2).any(|w|w[0]>=w[1]) || c.scheduled_sessions.binary_search(&c.expiry).is_err() {return Err(missing(format!("{} needs an ordered published session calendar containing expiry",c.symbol)));}
        let b=&c.bars;let n=b.len();
        if n==0 || [b.date.len(),b.open.len(),b.high.len(),b.low.len(),b.end.len()].iter().any(|&l|l!=n) || b.volume.as_ref().is_some_and(|v|v.len()!=n) || c.oi.as_ref().is_some_and(|v|v.len()!=n) {return Err(invalid(format!("{} has inconsistent column lengths",c.symbol)));}
        if b.date.iter().any(|d|!date_ok(d)||c.scheduled_sessions.binary_search(d).is_err()) || b.date.windows(2).any(|w|w[0]>=w[1]) {return Err(invalid(format!("{} has unsorted, duplicate or unscheduled dates",c.symbol)));}
        for i in 0..n {
            if ![b.open[i],b.high[i],b.low[i],b.close[i]].iter().all(|n|n.is_finite()&&*n>0.) || ![b.open[i],b.high[i],b.low[i],b.close[i]].iter().all(|p|(*p/c.tick).is_finite() && (*p*c.point_value).is_finite()) || b.low[i]>b.open[i].min(b.close[i]) || b.high[i]<b.open[i].max(b.close[i]) || b.low[i]>b.high[i] {return Err(invalid(format!("{} has invalid OHLC",c.symbol)));}
        }
        if b.volume.iter().chain(c.oi.iter()).any(|v|v.iter().any(|n|!n.is_finite()||*n<0.)) {return Err(invalid(format!("{} has invalid volume/open interest",c.symbol)));}
    }
    Ok(())
}
fn index(c:&Contract,date:&String)->Result<usize,LabError> {c.bars.date.binary_search(date).map_err(|_|missing(format!("{} has no price on required session {date}",c.symbol)))}
/// Forward/ratio adjustments apply only after the observed roll close and never rewrite earlier bars.
pub fn continuous(chain:&[Contract],roll:Roll,adjust:Adjust)->Result<Continuous,LabError> {
    validate(chain)?;let mut out=Continuous::default();out.bars.expiry=Some(vec![]);out.bars.expiry_sessions=Some(vec![]);
    let mut volume=Some(vec![]);let (mut off,mut factor,mut after)=(0.,1.,None::<String>);
    for (k,c) in chain.iter().enumerate() {
        let exp=c.scheduled_sessions.binary_search(&c.expiry).unwrap();
        let from=c.scheduled_sessions.partition_point(|d|after.as_ref().map_or(d<&c.bars.date[0],|a|d<=a));
        if from>exp {return Err(missing(format!("{} has no sessions after the preceding contract",c.symbol)));}
        let next=chain.get(k+1);
        let mut at=match (next,roll) {(Some(_),Roll::DaysBefore(n))=>exp.checked_sub(n).filter(|&i|i>=from).ok_or_else(||missing("roll date precedes the published calendar/contract coverage"))?,_=>exp};
        if let (Some(n),Roll::Volume|Roll::OpenInterest)=(next,roll) {
            for i in from..=exp {
                let d=&c.scheduled_sessions[i];let (a,b)=(index(c,d)?,index(n,d)?);
                let values=|x:&Contract,j| if roll==Roll::Volume {x.bars.volume.as_ref()} else {x.oi.as_ref()}.map(|v|v[j]).ok_or_else(||missing(format!("{} needs volume/open interest for the declared roll",x.symbol)));
                if values(n,b)?>values(c,a)? {at=i;break;}
            }
        }
        for si in from..=at {
            let date=&c.scheduled_sessions[si];let i=index(c,date)?;let p=|x:f64|x*factor+off;
            let values=[p(c.bars.open[i]),p(c.bars.high[i]),p(c.bars.low[i]),p(c.bars.close[i])];
            if values.iter().any(|p|!p.is_finite()||*p<=0.) {return Err(invalid("adjustment produces nonpositive/nonfinite prices"));}
            out.bars.date.push(date.clone());out.bars.open.push(values[0]);out.bars.high.push(values[1]);out.bars.low.push(values[2]);out.bars.close.push(values[3]);out.bars.end.push(out.bars.end.len());
            out.bars.expiry.as_mut().unwrap().push(c.expiry.clone());out.bars.expiry_sessions.as_mut().unwrap().push((exp-si) as f64);
            out.held.push(k);out.off.push(off);out.factor.push(factor);
            match (&mut volume,&c.bars.volume) {(Some(out),Some(v))=>out.push(v[i]),(_,None)=>volume=None,_=>{}}
        }
        if let Some(n)=next {
            let date=&c.scheduled_sessions[at];let old=c.bars.close[index(c,date)?];
            // No position rolls in none mode; forward adjustment still joins signal series at the expiry close.
            let new=if roll!=Roll::None || adjust!=Adjust::None {n.bars.close[index(n,date)?]} else {0.};
            if roll!=Roll::None {out.rolls.push((out.bars.len()-1,new));}
            match adjust {Adjust::Forward=>off+=factor*(old-new),Adjust::Ratio=>factor*=old/new,Adjust::None=>{}}
            if !off.is_finite()||!factor.is_finite()||factor<=0. {return Err(invalid("invalid adjustment denominator"));}
            after=Some(date.clone());
        }
    }
    out.bars.volume=volume;Ok(out)
}
/// Translate a single-leg simulator trade and every roll piece to the actual held contract; preserve its allocation.
pub fn to_contracts(trades:&[Trade],c:&Continuous)->Vec<(Trade,usize)> {
    let mut out=Vec::new();
    for t in trades {
        let (mut e,mut px,mut kind,mut k)=(t.legs[0].i,c.actual(t.legs[0].i,t.legs[0].px),t.entry_kind,c.held[t.legs[0].i]);
        for &(r,new) in c.rolls.iter().filter(|(r,_)|t.legs[0].i<=*r&&*r<t.exit_i) {
            out.push((Trade {legs:vec![Leg{i:e,px,w:t.legs[0].w}],exit_i:r,exit_px:c.actual(r,c.bars.close[r]),reason:"roll".into(),entry_kind:kind,signal_i:r,open:false,exits:vec![],..t.clone()},k));
            (e,px,kind,k)=(r,new,"roll",c.held[r]+1);
        }
        out.push((Trade {legs:vec![Leg{i:e,px,w:t.legs[0].w}],exit_px:c.actual(t.exit_i,t.exit_px),entry_kind:kind,..t.clone()},k));
    }
    out
}
pub struct FuturesRun {pub series:Continuous,pub rows:Vec<(Trade,usize,Row)>,pub events:Vec<String>,pub final_equity:f64}
/// Each roll pays both orders and preserves quantity, rounded down to the incoming lot and available margin.
pub fn run(chain:&[Contract],spec:&Spec,capital:f64)->Result<FuturesRun,LabError> {
    let f=spec.exec.futures.ok_or_else(||invalid("the spec is not futures"))?;
    if !capital.is_finite()||capital<=0.||capital>super::rules::MAX_CAPITAL {return Err(invalid("capital must be positive and finite"));}
    let x=&spec.exec;
    if spec.cost_bps.is_some() {return Err(LabError::Unsupported("futures money fills use execution.costs, not legacy execution.cost_bps".into()));}
    if x.risk!=super::rules::Risk::default() || x.cooldown.is_some() || x.max_trades.is_some() || matches!(x.sizing,Sizing::RiskPct(_)) {
        return Err(LabError::Unsupported("futures money-aware risk sizing/pauses/cooldowns are not integrated; unit returns cannot stand in for leveraged outcomes".into()));
    }
    let sides=std::iter::once(x).chain(spec.short_side.as_ref().map(|s|&s.exec));
    for side in sides {
        if side.scale_in.is_some()||!side.take_profits.is_empty(){return Err(LabError::Unsupported("futures scale-in/take-profits are not integrated".into()));}
        let prices=side.has_order()||side.stop_loss.is_some()||side.target.is_some()||side.trailing.is_some()||side.breakeven.is_some();
        // shortcut: price-state across ratio rolls needs state re-basing; admit it after that mapping is integrated and tested.
        if f.adjust==Adjust::Ratio&&prices{return Err(LabError::Unsupported("ratio-adjusted futures price orders/exits require state re-basing; use forward adjustment or market/rule exits".into()));}
    }
    if spec.short_side.is_some()&&x.opposite==super::rules::Opposite::Reverse&&(x.limit.is_some()||x.limit_at.is_some()||spec.short_side.as_ref().is_some_and(|s|s.exec.limit.is_some()||s.exec.limit_at.is_some())) {return Err(LabError::Unsupported("resting futures limit reversals need preserved originating fill metadata".into()));}
    let cont=continuous(chain,f.roll,f.adjust)?;let fr=Frames::new(cont.bars.clone());
    let mut c=series::compile(spec,&fr,&mut Cache::new())?;
    let grid:Vec<_>=(0..cont.bars.len()).map(|i|(chain[cont.held[i]].tick,cont.off[i],cont.factor[i])).collect();
    c.grid=grid.clone();c.expiry_bar=(0..cont.bars.len()).map(|i|cont.bars.date[i]==chain[cont.held[i]].expiry && !cont.rolls.iter().any(|r|r.0==i)).collect();
    if let Some(o)=c.other.as_mut(){o.grid=grid;o.expiry_bar=c.expiry_bar.clone();}
    let (trades,mut events)=super::simulate::simulate_events(&c,&fr.d,0,fr.d.len()-1);
    let (mut equity,mut rows)=(capital,vec![]);
    for logical in trades {
        let mut quantity=None;
        for (t,k) in to_contracts(&[logical],&cont) {
            let ct=&chain[k];let terms=Terms{lot:ct.lot,point_value:ct.point_value,tick:ct.tick,margin_pct:Some(ct.margin_pct)};
            let sizing=quantity.map_or(x.sizing,|q:f64|Sizing::Units(q/(ct.lot*t.legs[0].w)));
            let m=Money{capital:equity,sizing,costs:x.costs.clone(),contract:terms,limits:super::rules::Limits::none()};let span=ledger::Span{close:&[],volume:None,from:0,to:cont.bars.date.len()-1};
            let Some(row)=ledger::book(std::slice::from_ref(&t),&cont.bars.date,&m,&span).rows.into_iter().next() else {events.push(format!("entry/roll skipped: insufficient whole-lot margin on {}",cont.bars.date[t.legs[0].i]));break;};
            let paid: f64=row.entries.iter().map(|f|f.costs()).sum();
            let entry=row.entries[0].px;let q=row.qty*ct.point_value;
            let mut call=None;
            for j in t.legs[0].i..t.exit_i {
                let px=ct.bars.close[index(ct,&cont.bars.date[j])?];
                if equity-paid+q*t.dir*(px-entry)<q*px*ct.margin_pct/100. {call=Some(j);break;}
            }
            if let Some(j)=call {
                let px=ct.bars.open[index(ct,&cont.bars.date[j+1])?];
                let cut=Trade{exit_i:j+1,exit_px:px,reason:"margin_call".into(),signal_i:j,open:false,..t};
                let r=ledger::book(std::slice::from_ref(&cut),&cont.bars.date,&m,&span).rows.into_iter().next().unwrap();
                equity+=r.pnl;rows.push((cut,k,r));events.push(format!("margin call on {}: liquidated at the next actual contract open; trading stopped",cont.bars.date[j]));
                return Ok(FuturesRun{series:cont,rows,events,final_equity:equity});
            }
            quantity=Some(row.qty);equity+=row.pnl;rows.push((t,k,row));
            if !equity.is_finite()||equity<=0. {return Err(invalid("nonpositive/nonfinite money equity"));}
        }
    }
    Ok(FuturesRun{series:cont,rows,events,final_equity:equity})
}
fn expiry(b:&Bars,_:&Params)->Output {
    let (Some(exp),Some(sessions))=(&b.expiry,&b.expiry_sessions) else {return Err("needs contract data".into())};
    if exp.len()!=b.len()||sessions.len()!=b.len(){return Err("needs contract data".into());}
    Ok(vec![sessions.clone(),b.date.iter().zip(exp).map(|(d,e)|if super::resample::period_key(d,1)==super::resample::period_key(e,1){1.}else{0.}).collect(),b.date.iter().zip(exp).map(|(d,e)|if d==e{1.}else{0.}).collect()])
}
pub const DEFS:&[IndicatorDef]=&[IndicatorDef{id:"expiry",name:"Futures Expiry",category:"Periods & calendar",params:&[],outputs:&["days_to_expiry","expiry_week","expiry_day"],placement:Placement::Pane,needs:Needs{derivatives:true,..Needs::NONE},definition:"Scheduled exchange sessions remaining until the held contract expiry, expiry week and expiry day; contract metadata required",first:&[0,0,0],sparse:false,compute:Some(expiry),note:""}];

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lab::rules::{parse_spec, Adjust, Dist, Roll};
    use crate::lab::resample::{civil, day_number};
    fn dates() -> Vec<String> { (0..40).map(|i|civil(day_number("2024-01-01")+i)).collect() }
    fn contract(k: usize, start: usize, end: usize, base: f64) -> Contract {
        let date=dates(); let mut b=Bars::default();
        for i in start..=end { let p=base+i as f64; b.date.push(date[i].clone()); b.open.push(p); b.high.push(p); b.low.push(p); b.close.push(p); b.end.push(b.end.len()); }
        b.volume=Some(vec![1000.;b.len()]);
        Contract {symbol:format!("F{k}"),expiry:date[end].clone(),scheduled_sessions:date[..=end].to_vec(),lot:50.,tick:0.05,point_value:1.,margin_pct:10.,bars:b,oi:None}
    }
    fn chain() -> Vec<Contract> { vec![contract(0,0,9,100.),contract(1,0,19,105.),contract(2,10,29,110.)] }
    fn spec(extra: &str) -> Spec { parse_spec(&format!(r#"{{"v":1,"entry":{{"all":[{{"lhs":{{"s":"close"}},"op":"=","rhs":{{"value":102}}}}]}},"exit":{{"any":[{{"lhs":{{"s":"bars_held"}},"op":">=","rhs":{{"value":23}}}}]}},"execution":{{"fill":"close","exit_fill":"close","capital":1000000,"sizing":{{"units":1}},"instrument":{{"kind":"futures"}}{extra}}}}}"#)).unwrap() }
    #[test] fn roll_calendar_is_independent_of_observed_prices() {
        let c=continuous(&chain(),Roll::DaysBefore(1),Adjust::Forward).unwrap();
        assert_eq!(c.rolls,vec![(8,113.),(18,128.)]); assert_eq!((c.held[8],c.held[9]),(0,1));
        assert_eq!(c.bars.close,(0..30).map(|i|100.+i as f64).collect::<Vec<_>>());
        assert_eq!(c.bars.volume.as_ref().unwrap(),&vec![1000.;30]);
        for n in [7,8] { let mut cs=chain(); let b=&mut cs[0].bars;for v in [&mut b.open,&mut b.high,&mut b.low,&mut b.close] {v.truncate(n);} cs[0].bars.date.truncate(n);cs[0].bars.end.truncate(n);cs[0].bars.volume.as_mut().unwrap().truncate(n);
            assert_eq!(continuous(&cs,Roll::DaysBefore(1),Adjust::Forward).err().unwrap().status(),"needs_data"); }
        let mut cs=chain();cs[0].scheduled_sessions.clear(); assert!(continuous(&cs,Roll::DaysBefore(1),Adjust::Forward).is_err());
    }
    #[test] fn required_gaps_and_malformed_metadata_never_panic() {
        for kind in 0..10 { let mut cs=chain(); match kind {
            0=>{cs[0].bars.high.pop();}, 1=>cs[0].bars.low[2]=f64::NAN, 2=>cs[0].lot=0.,3=>cs[0].bars.date[2]=cs[0].bars.date[1].clone(),
            4=>{cs[0].scheduled_sessions.remove(9);},5=>{cs[0].bars.volume.as_mut().unwrap().pop();},6=>cs[0].margin_pct=f64::INFINITY,7=>cs[0].bars.high[2]=1.,8=>cs[0].tick=1e-320,_=>cs[0].bars.date[2]="2024-99-99".into(),}
            assert!(continuous(&cs,Roll::DaysBefore(1),Adjust::Ratio).is_err(),"case {kind}"); }
        let mut cs=chain();cs[0].bars.date[3]=dates()[35].clone();assert!(continuous(&cs,Roll::None,Adjust::None).is_err());
    }
    #[test] fn volume_and_interest_rolls_use_only_the_current_close() {
        let mut cs=chain();cs[2]=contract(2,0,29,110.);cs[1].bars.volume=Some((0..20).map(|i|if i>=5 {2000.}else{500.}).collect());
        assert_eq!(continuous(&cs,Roll::Volume,Adjust::None).unwrap().rolls[0].0,5);
        assert_eq!(continuous(&cs,Roll::OpenInterest,Adjust::None).err().unwrap().status(),"needs_data");
        for c in &mut cs {c.oi=Some(vec![100.;c.bars.len()]);} cs[1].oi.as_mut().unwrap()[6]=200.;
        assert_eq!(continuous(&cs,Roll::OpenInterest,Adjust::None).unwrap().rolls[0].0,6);
    }
    #[test] fn roll_pieces_preserve_allocation_and_actual_quantity() {
        let s=spec(r#", "costs":{"commission":{"per_order":20}}"#);
        let r=run(&chain(),&s,1e6).unwrap();
        let spans:Vec<_>=r.rows.iter().map(|(t,k,row)|(t.legs[0].i,t.legs[0].px,t.exit_i,t.exit_px,*k,row.qty,t.legs[0].w)).collect();
        assert_eq!(spans,vec![(2,102.,8,108.,0,50.,1.),(8,113.,18,123.,1,50.,1.),(18,128.,25,135.,2,50.,1.)]);
        assert!((r.final_equity-1e6-1030.).abs()<1e-7);
        let mut s=s.clone();s.exec.sizing=Sizing::PctEquity(50.);let r=run(&chain(),&s,10000.).unwrap();
        assert_eq!(r.rows.iter().map(|(_,_,r)|r.qty).collect::<Vec<_>>(),vec![450.;3]);
        assert!(r.rows.iter().all(|(t,_,_)|t.legs[0].w==0.5));
        let mut cs=chain();cs[1].lot=80.;let r=run(&cs,&s,10000.).unwrap();assert_eq!(r.rows[1].2.qty,400.);assert_eq!(r.rows[2].2.qty,400.);
    }
    #[test] fn roll_none_is_a_hard_expiry_barrier_even_with_min_bars() {
        let mut s=spec("");s.exec.futures.as_mut().unwrap().roll=Roll::None;s.exec.min_bars=100;
        let r=run(&chain(),&s,1e6).unwrap(); assert!(r.series.rolls.is_empty());
        assert_eq!((r.rows[0].0.exit_i,r.rows[0].0.reason.as_str()),(9,"expiry"));
        s.entry=vec![];s.exec.fill=crate::lab::rules::Fill::NextOpen;s.exec.delay=3;
        let r=run(&chain(),&s,1e6).unwrap();assert!(r.rows.iter().all(|(t,_,_)|!matches!(t.legs[0].i,9|19|29)));
        let mut cs=chain();cs[0].bars.close.pop();cs[0].bars.date.pop();cs[0].bars.open.pop();cs[0].bars.high.pop();cs[0].bars.low.pop();cs[0].bars.end.pop();cs[0].bars.volume.as_mut().unwrap().pop();
        assert_eq!(continuous(&cs,Roll::None,Adjust::None).err().unwrap().status(),"needs_data");
    }
    #[test] fn margin_uses_the_incoming_contract_mark_and_entry_costs() {
        let mut cs=chain();for c in &mut cs {c.margin_pct=10.;c.lot=1.;} cs[0].margin_pct=100.;cs[1].point_value=2.;
        // Incoming contract trades twice as high; using the outgoing continuous close would invent a margin call.
        let b=&mut cs[1].bars;for v in [&mut b.open,&mut b.high,&mut b.low,&mut b.close] { for p in v {*p*=2.;} }
        let mut s=spec("");s.exec.sizing=Sizing::Units(10.);let r=run(&cs,&s,1200.).unwrap();assert!(r.events.iter().all(|e|!e.contains("margin call")));assert_eq!(r.rows.len(),3);
        let mut c=contract(0,0,29,100.);for i in 0..30 {let p=100.-i as f64;for v in [&mut c.bars.open,&mut c.bars.high,&mut c.bars.low,&mut c.bars.close]{v[i]=p;}}
        let mut s=spec("");s.entry[0]=parse_spec(r#"{"v":1,"entry":{"all":[{"lhs":{"s":"close"},"op":"=","rhs":{"value":100}}]},"exit":{"any":[{"lhs":{"s":"bars_held"},"op":">=","rhs":{"value":25}}]}}"#).unwrap().entry.remove(0);
        let r=run(&[c.clone()],&s,600.).unwrap();assert_eq!((r.rows[0].0.exit_i,r.rows[0].0.exit_px,r.rows[0].0.reason.as_str()),(4,96.,"margin_call"));
        s.exec.costs.per_order=20.;let r=run(&[c],&s,600.).unwrap();assert_eq!((r.rows[0].0.exit_i,r.rows[0].0.exit_px,r.final_equity),(3,97.,410.));
    }
    #[test] fn expiry_counts_scheduled_sessions_including_exchange_holidays() {
        let mut cs=chain();cs[0].scheduled_sessions.remove(4);
        let b=&mut cs[0].bars;for v in [&mut b.open,&mut b.high,&mut b.low,&mut b.close] {v.remove(4);}cs[0].bars.date.remove(4);cs[0].bars.end=(0..9).collect();cs[0].bars.volume.as_mut().unwrap().remove(4);
        let c=continuous(&cs,Roll::None,Adjust::None).unwrap();let e=expiry(&c.bars,&Params::new()).unwrap();assert_eq!(e[0][0],8.);assert_eq!(e[0][8],0.);
    }
    #[test] fn tick_levels_use_actual_coordinates_and_ratio_orders_are_refused() {
        let mut s=spec("");s.exec.stop_loss=Some(Dist::Ticks(3.));s.exec.trailing=Some(Dist::Ticks(5.));
        let r=run(&chain(),&s,1e6).unwrap();assert!(r.rows.iter().all(|(t,k,_)| ((t.exit_px/chain()[*k].tick).round()-t.exit_px/chain()[*k].tick).abs()<1e-6));
        s.exec.futures.as_mut().unwrap().adjust=Adjust::Ratio;assert_eq!(run(&chain(),&s,1e6).err().unwrap().status(),"unsupported");
        let mut c=crate::lab::series::Compiled::new(vec![],vec![]);c.grid=vec![(0.25,7.,2.)];
        assert!((c.round_at(0,207.23)-207.).abs()<1e-10);assert_eq!(c.dist(Dist::Ticks(3.),207.,0),1.5);
        assert_eq!(c.dist(Dist::Pct(10.),207.,0),20.);
    }
    #[test] fn futures_parser_validates_ticks_adjustment_and_optimized_distances() {
        let base=r#"{"v":1,"entry":{"all":[{"lhs":{"s":"close"},"op":">","rhs":{"value":0}}]},"exit":{"any":[],"stop_loss":{"ticks":20}},"execution":{"instrument":{"kind":"futures","roll":{"days_before_expiry":2},"adjust":"ratio"}}}"#;
        let s=parse_spec(base).unwrap();assert_eq!(s.exec.futures,Some(crate::lab::rules::Futures{roll:Roll::DaysBefore(2),adjust:Adjust::Ratio}));
        assert_eq!(s.exec.stop_loss,Some(Dist::Ticks(20.)));
        for (from,to,status) in [("\"ratio\"","\"back\"","unsupported"),("\"ticks\":20","\"ticks\":2.5","invalid"),("\"kind\":\"futures\"","\"kind\":\"cash\"","invalid"),("\"adjust\":\"ratio\"","\"adjust\":false","invalid"),("\"kind\":\"futures\"","\"kind\":false","invalid")] {
            assert_eq!(parse_spec(&base.replace(from,to)).err().unwrap().status(),status);
        }
        let optimized=base.replace("\"v\":1,","\"v\":1,\"optimize\":[{\"path\":\"exit.stop_loss.ticks\",\"from\":1,\"to\":3,\"step\":1}],");assert!(parse_spec(&optimized).is_ok());
        assert!(parse_spec(&optimized.replace("\"step\":1","\"step\":0.5")).is_err());
        assert!(parse_spec(&base.replace("\"execution\":{\"instrument\":{\"kind\":\"futures\",\"roll\":{\"days_before_expiry\":2},\"adjust\":\"ratio\"}}", "\"execution\":{}")).is_err());
    }
    #[test] fn trailing_and_stored_levels_round_in_the_current_contract_grid() {
        let mut c=crate::lab::series::Compiled::new(vec![],vec![]);
        c.grid=vec![(0.05,0.,1.),(0.25,7.,1.)];
        assert_eq!(c.dist_at(Dist::Pct(10.),107.,0,1),10.);
        assert_eq!(c.dist_at(Dist::Ticks(3.),107.,0,1),0.75);
    }
    #[test] fn percent_targets_and_old_stops_are_rounded_to_actual_contract_ticks() {
        let mut s=spec("");s.exec.target=Some(Dist::Pct(1.1));let mut cs=chain();cs[0].tick=0.25;
        let r=run(&cs,&s,1e6).unwrap();assert_eq!((r.rows[0].0.exit_i,r.rows[0].0.exit_px),(3,103.));
        let mut cs=chain();cs[1].tick=1.;cs[1].bars.low[9]=105.;let mut s=spec("");s.exec.stop_loss=Some(Dist::Pct(2.));
        let r=run(&cs,&s,1e6).unwrap();assert_eq!((r.rows[1].0.exit_i,r.rows[1].0.exit_px),(9,105.));
        let mut cs=chain();cs[0].bars.high[9]=120.;let mut s=spec("");s.exec.futures.as_mut().unwrap().roll=Roll::None;
        s.entry=vec![];s.exec.stop_at=Some(crate::lab::rules::Operand::Value(110.));s.exec.valid_bars=20;
        let r=run(&cs,&s,1e6).unwrap();assert!(r.rows.iter().all(|(t,_,_)|!matches!(t.legs[0].i,9|19|29)));
    }
    #[test] fn futures_refuse_unintegrated_unit_risk_instead_of_claiming_money_risk() {
        let mut s=spec("");s.exec.risk.max_dd_pct=Some(5.);assert_eq!(run(&chain(),&s,1e6).err().unwrap().status(),"unsupported");
        s.exec.risk.max_dd_pct=None;s.exec.max_trades=Some((1,1));assert!(run(&chain(),&s,1e6).is_err());
        s.exec.max_trades=None;s.exec.cooldown=Some((2,crate::lab::rules::After::Any));assert!(run(&chain(),&s,1e6).is_err());
        s.exec.cooldown=None;s.cost_bps=Some(10.);assert!(run(&chain(),&s,1e6).is_err());s.cost_bps=None;
        s.exec.sizing=Sizing::RiskPct(1.);assert_eq!(run(&chain(),&s,1e6).err().unwrap().status(),"unsupported");
    }
    #[test] fn expiry_is_reachable_from_a_rule_only_on_contract_bars() {
        let text=r#"{"v":1,"entry":{"all":[{"lhs":{"s":"expiry","out":"days_to_expiry"},"op":"=","rhs":{"value":7}}]},"exit":{"any":[{"lhs":{"s":"bars_held"},"op":">=","rhs":{"value":2}}]},"execution":{"fill":"close","capital":1000000,"sizing":{"units":1},"instrument":{"kind":"futures"}}}"#;
        let s=parse_spec(text).unwrap();let r=run(&chain(),&s,1e6).unwrap();assert_eq!(r.rows[0].0.legs[0].i,2);
        let e=series::compile(&s,&Frames::new(crate::lab::testdata::walk(30,2,true)),&mut Cache::new()).err().unwrap();assert_eq!(e.status(),"needs_data");
    }
    #[test] fn future_price_changes_do_not_rewrite_finished_pieces() {
        let s=spec("");let cs=chain();let expected=run(&cs,&s,1e6).unwrap();
        for cut in [10,20] {let mut alt=cs.clone();for c in &mut alt {for i in 0..c.bars.len(){if c.bars.date[i]>dates()[cut]{for v in [&mut c.bars.open,&mut c.bars.high,&mut c.bars.low,&mut c.bars.close]{v[i]*=1.2;}}}}
            let got=run(&alt,&s,1e6).unwrap();assert_eq!(&got.series.bars.close[..=cut],&expected.series.bars.close[..=cut]);
            let done=|r:&FuturesRun|r.rows.iter().filter(|(t,_,_)|t.exit_i<=cut).map(|(t,_,r)|(t.exit_i,t.exit_px.to_bits(),r.pnl.to_bits())).collect::<Vec<_>>();assert_eq!(done(&got),done(&expected));}
    }
}
