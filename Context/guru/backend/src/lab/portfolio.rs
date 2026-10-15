//! Shared-cash portfolio simulation; book strategy calculations remain untouched.
use super::{ledger::{self, Fill as MoneyFill}, rules::{Fill, LabError, Mode, MoneyCosts, Sizing}, series::Compiled, universe::Member};
use crate::connors::{Leg, Trade};
use std::{sync::atomic::{AtomicBool,Ordering},time::Instant};

pub struct Lane<'a> { pub m: &'a Member, pub c: Compiled, pub score: Option<Vec<f64>> }
pub struct Book {
    pub capital: f64, pub mode: Mode, pub max_open: usize, pub worst_rank_held: usize,
    pub rebalance: Option<usize>, pub lowest: bool, pub fill: Fill, pub exit_fill: Fill,
    pub size: Sizing, pub costs: MoneyCosts,
}
pub struct PTrade {
    pub lane: usize, pub signal_i: usize, pub exit_signal_i: usize, pub entry_i: usize, pub exit_i: usize,
    pub entry_px: f64, pub exit_px: f64, pub qty: f64, pub entry_cost: f64, pub exit_cost: f64,
    pub equity_at_signal: f64, pub rank: usize, pub score: f64, pub reason: String, pub open: bool, pub kind: &'static str,
    pub entry: MoneyFill, pub exit: Option<MoneyFill>,
}
impl PTrade {
    pub fn pnl(&self) -> f64 { self.qty*(self.exit_px-self.entry_px)-self.entry_cost-self.exit_cost }
    /// Statistics use effective prices; API fills remain observed prices. Both use the SAME actual quantities.
    pub fn as_trade(&self, net: bool) -> Trade {
        let entry=self.entry_px+if net {self.entry_cost/self.qty} else {0.};
        let exit=self.exit_px-if net {self.exit_cost/self.qty} else {0.};
        Trade {dir:1.,legs:vec![Leg {i:self.entry_i,px:entry,w:self.qty*entry/self.equity_at_signal}],exit_i:self.exit_i,exit_px:exit,reason:self.reason.clone(),entry_kind:self.kind,signal_i:self.exit_signal_i,open:self.open,exits:vec![]}
    }
}
#[derive(Default,Debug,PartialEq)]
pub struct Skipped { pub no_slot: usize, pub no_cash: usize, pub no_bar: usize, pub data_end: usize, pub final_bar: usize }
pub struct Ledger {
    pub equity: Vec<f64>, pub gross_equity: Vec<f64>, pub cash: Vec<f64>, pub invested: Vec<f64>, pub positions: Vec<usize>,
    pub trades: Vec<PTrade>, pub skipped: Skipped, pub events: Vec<String>, pub costs_total: f64,
}

struct Held {
    qty:f64, entry:MoneyFill, entry_k:usize, signal_i:usize, equity:f64, rank:usize, score:f64,
    kind:&'static str, position:super::simulate::PortfolioPosition,
}
struct PendingExit { signal_i:usize, entry_i:usize, reason:String }
#[derive(Clone)]
struct Candidate {
    lane:usize, signal_i:usize, signal_k:usize, equity:f64, qty:f64, requested:f64, score:f64, key:f64, rank:usize,
}
struct Account {
    cash:f64, paid:f64, held:Vec<Option<Held>>, pending:Vec<Option<PendingExit>>, marks:Vec<f64>, count:usize,
}
fn invalid(message:impl Into<String>)->LabError {LabError::Invalid(message.into())}
fn pulse(deadline:Instant,stop:Option<&AtomicBool>)->Result<(),LabError> {
    if stop.is_some_and(|s|s.load(Ordering::Relaxed)) {return Err(invalid("portfolio computation cancelled after another combination failed; no result is published"));}
    if Instant::now()>=deadline {return Err(invalid("portfolio run stopped after 90 s; use fewer members or a shorter window"));} Ok(())
}
fn finite(x:f64,at:&str)->Result<f64,LabError> {if x.is_finite(){Ok(x)}else{Err(invalid(format!("portfolio {at} is not finite; no result is published")))}}
impl Account {
    fn value(&self,order:&[usize])->Result<(f64,f64),LabError> {
        let mut invested=0.;for &m in order {if let Some(h)=&self.held[m] {invested=finite(invested+h.qty*self.marks[m],"invested value")?;}}
        Ok((invested,finite(self.cash+invested,"equity")?))
    }
    fn open(&mut self,out:&mut Ledger,b:&Book,ln:&Lane,c:&Candidate,i:usize,k:usize,raw:f64,kind:&'static str)->Result<(),LabError> {
        if self.held[c.lane].is_some(){return Ok(());}
        if self.count>=b.max_open {out.skipped.no_slot+=1;return Ok(());}
        if c.qty<1. {out.skipped.no_cash+=1;return Ok(());}
        let unit=ledger::fill(&b.costs,&ledger::CASH,i,raw,1.,true,true);
        let unit_cost=finite(unit.costs()-b.costs.per_order,"per-unit entry costs")?;
        let room=finite((self.cash-b.costs.per_order).max(0.)/(raw+unit_cost),"affordable quantity")?.floor();
        let mut qty=c.qty.min(room).floor();
        if qty<1. {out.skipped.no_cash+=1;return Ok(());}
        let mut entry=ledger::fill(&b.costs,&ledger::CASH,i,raw,qty,true,true);
        let mut debit=finite(qty*raw+entry.costs(),"entry debit")?;
        // Affine affordability solves once. At a floating-point boundary, reduce once rather than loop per share.
        if debit>self.cash {
            let reduced=qty-1.;if reduced==qty {return Err(invalid("portfolio whole-share affordability exceeds numeric precision"));}
            qty=reduced;
            if qty<1. {out.skipped.no_cash+=1;return Ok(());}
            entry=ledger::fill(&b.costs,&ledger::CASH,i,raw,qty,true,true);debit=finite(qty*raw+entry.costs(),"entry debit")?;
        }
        if debit>self.cash {return Err(invalid("portfolio entry would exceed available cash; no result is published"));}
        self.cash-=debit;self.paid=finite(self.paid+entry.costs(),"paid costs")?;
        self.held[c.lane]=Some(Held {qty,entry,entry_k:k,signal_i:c.signal_i,equity:c.equity,rank:c.rank,score:c.score,kind,
            position:super::simulate::PortfolioPosition::new(&ln.c,k,raw,c.signal_k,kind)});
        self.count+=1;Ok(())
    }
    fn close(&mut self,out:&mut Ledger,b:&Book,m:usize,i:usize,raw:f64,signal_i:usize,reason:String)->Result<(),LabError> {
        let Some(h)=self.held[m].take() else {self.pending[m]=None;return Ok(());};
        let exit=ledger::fill(&b.costs,&ledger::CASH,i,raw,h.qty,false,!reason.starts_with("exit.target"));
        let costs=finite(exit.costs(),"exit costs")?;
        self.cash=finite(self.cash+h.qty*raw-costs,"cash after exit")?;
        if self.cash<0. {return Err(invalid("portfolio declared exit costs exhaust available cash; no result is published"));}
        self.paid=finite(self.paid+costs,"paid costs")?;
        self.pending[m]=None;self.count-=1;out.trades.push(finish(m,h,i,raw,signal_i,reason,Some(exit)));Ok(())
    }
}
fn finish(lane:usize,h:Held,i:usize,raw:f64,signal_i:usize,reason:String,exit:Option<MoneyFill>)->PTrade {
    PTrade {lane,signal_i:h.signal_i,exit_signal_i:signal_i,entry_i:h.entry.i,exit_i:i,entry_px:h.entry.px,exit_px:raw,qty:h.qty,
        entry_cost:h.entry.costs(),exit_cost:exit.as_ref().map_or(0.,MoneyFill::costs),equity_at_signal:h.equity,rank:h.rank,score:h.score,
        reason,open:exit.is_none(),kind:h.kind,entry:h.entry,exit}
}
fn level(ln:&Lane,k:usize)->bool {ln.c.entry.iter().all(|r|r.holds(k,None))}
fn fires(ln:&Lane,k:usize)->bool {level(ln,k)&&(!ln.c.exec.edge||k==0||!level(ln,k-1))}
fn quantity(b:&Book,ln:&Lane,k:usize,equity:f64)->Result<(f64,f64),LabError> {
    let px=ln.m.frames.d.close[k];
    let stop=ln.c.exec.stop_loss.map_or(f64::NAN,|d|ln.c.dist_at(d,px,k,k));
    let (qty,requested)=match b.size {
        Sizing::Full => {let p=100./b.max_open as f64;(equity*p/100./px,p)},
        Sizing::PctEquity(p)=>(equity*p/100./px,p),Sizing::Value(v)=>(v/px,v),Sizing::Units(n)=>(n,n),
        Sizing::RiskPct(_) if !stop.is_finite()||stop<=0. => (0.,0.),
        Sizing::RiskPct(r)=>{let q=equity*r/100./stop;(q,q*px)},
    };
    Ok((finite(qty,"requested quantity")?.floor().max(0.),finite(requested,"requested ranking size")?))
}
fn candidate(ln:&Lane,m:usize,i:usize,k:usize,equity:f64,b:&Book)->Result<Option<Candidate>,LabError> {
    let score=ln.score.as_ref().map_or(0.,|s|s[k]);if !score.is_finite(){return Ok(None);}
    let (qty,requested)=quantity(b,ln,k,equity)?;
    Ok(Some(Candidate {lane:m,signal_i:i,signal_k:k,equity,qty,requested,score,key:if b.lowest {-score}else{score},rank:0}))
}
fn rank(candidates:&mut[Candidate],lanes:&[Lane]) {
    candidates.sort_by(|a,b|b.key.total_cmp(&a.key).then(b.requested.total_cmp(&a.requested)).then(lanes[a.lane].m.ticker.cmp(&lanes[b.lane].m.ticker)));
    for (i,c) in candidates.iter_mut().enumerate(){c.rank=i+1;}
}
fn check_inputs(lanes:&[Lane],b:&Book,dates:&[String],from:usize,to:usize,deadline:Instant,stop:Option<&AtomicBool>)->Result<Vec<usize>,LabError> {
    pulse(deadline,stop)?;
    if lanes.is_empty()||lanes.len()>super::rules::MAX_UNIVERSE||from>to||to>=dates.len(){return Err(invalid("portfolio members/window are outside their bounds"));}
    if !b.capital.is_finite()||b.capital<=0.||b.capital>super::rules::MAX_CAPITAL||b.max_open==0||b.max_open>super::rules::MAX_OPEN||b.worst_rank_held<b.max_open||b.worst_rank_held>super::rules::MAX_UNIVERSE||b.rebalance.is_some_and(|r|!(1..=4).contains(&r)){return Err(invalid("portfolio capital/slots/rebalance are outside their bounds"));}
    super::foreign::validate_dates(dates,"portfolio reference calendar")?;
    let c=&b.costs;
    if [c.per_order,c.per_unit,c.pct,c.slip_bps,c.slip_ticks].iter().any(|v|!v.is_finite()||*v<0.)||c.taxes.iter().any(|t|!t.1.is_finite()||t.1<0.)||c.borrow_pct_year.is_some_and(|v|!v.is_finite()||v<0.) {return Err(invalid("portfolio costs must be finite and nonnegative"));}
    if c.slip_ticks!=0. {return Err(LabError::Unsupported("portfolio cash fills do not have admitted tick slippage".into()));}
    match b.size {Sizing::Full=>{},Sizing::PctEquity(p)|Sizing::RiskPct(p) if !p.is_finite()||p<=0.||p>100.=>return Err(invalid("portfolio percentage sizing must be in (0,100]")),Sizing::Value(v)|Sizing::Units(v) if !v.is_finite()||v<=0.=>return Err(invalid("portfolio quantity/value sizing must be positive and finite")),_=>{}}
    let mut order:Vec<_>=(0..lanes.len()).collect();order.sort_by(|&a,&z|lanes[a].m.ticker.cmp(&lanes[z].m.ticker));
    if order.windows(2).any(|w|lanes[w[0]].m.ticker==lanes[w[1]].m.ticker){return Err(invalid("portfolio members must have distinct identities"));}
    for &m in &order {
        pulse(deadline,stop)?;let ln=&lanes[m];let d=&ln.m.frames.d;let n=d.len();
        if !matches!(ln.m.kind,"individual_stock"|"etf"){return Err(invalid(format!("{} is not an actual stock or ETF",ln.m.ticker)));}
        if n==0||d.date.len()!=n||[d.open.len(),d.high.len(),d.low.len()].iter().any(|&len|len!=n)||ln.m.at.len()!=dates.len()||ln.score.as_ref().is_some_and(|s|s.len()!=n){return Err(invalid(format!("{}: portfolio series/calendar lengths disagree",ln.m.ticker)));}
        super::foreign::validate_dates(&d.date,&ln.m.ticker)?;
        for r in ln.c.entry.iter().chain(ln.c.exit.iter()){for val in [&r.lhs,&r.rhs] {if matches!(val,super::series::Val::Arr(a) if a.len()!=n){return Err(invalid(format!("{}: compiled rule length disagrees with member bars",ln.m.ticker)));}}}
        let x=&ln.c.exec;
        if x.short||ln.c.other.is_some()||ln.c.has_order()||x.scale_in.is_some()||!x.take_profits.is_empty()||x.cooldown.is_some()||x.max_trades.is_some()||x.risk!=super::rules::Risk::default()||x.futures.is_some()||x.delay!=1 {return Err(LabError::Unsupported("portfolio execution contains a setting not supported by its long cash model".into()));}
        if matches!(b.size,Sizing::RiskPct(_))&&x.stop_loss.is_none(){return Err(invalid("portfolio risk sizing requires exit.stop_loss"));}
        if b.mode==Mode::Rotation&&(ln.score.is_none()||!ln.c.exit.is_empty()||x.stop_loss.is_some()||x.target.is_some()||x.trailing.is_some()||x.trail_activate.is_some()||x.breakeven.is_some()){return Err(LabError::Unsupported("rotation requires score and rank exits only".into()));}
        for (i,at) in ln.m.at.iter().enumerate() {
            if i%1024==0{pulse(deadline,stop)?;}
            if let Some(k)=at {
                if *k>=n||d.date[*k]!=dates[i]{return Err(invalid(format!("{}: reference/member alignment points to a different or missing date",ln.m.ticker)));}
                if i>=from&&i<=to && (![d.open[*k],d.high[*k],d.low[*k],d.close[*k]].iter().all(|x|x.is_finite()&&*x>0.)||d.low[*k]>d.high[*k]||d.open[*k]<d.low[*k]||d.open[*k]>d.high[*k]||d.close[*k]<d.low[*k]||d.close[*k]>d.high[*k]){return Err(LabError::NeedsData(format!("{}: invalid bar on {}; no portfolio result is published",ln.m.ticker,dates[i])));}
            }
        }
    }
    Ok(order)
}

/// One money ledger, using observed fills and already-known signals. No partial ledger is returned on error.
pub fn run(lanes:&[Lane],b:&Book,dates:&[String],from:usize,to:usize,deadline:Instant,stop:Option<&AtomicBool>)->Result<Ledger,LabError> {
    let order=check_inputs(lanes,b,dates,from,to,deadline,stop)?;
    let n=to-from+1;
    let mut out=Ledger {equity:Vec::with_capacity(n),gross_equity:Vec::with_capacity(n),cash:Vec::with_capacity(n),invested:Vec::with_capacity(n),positions:Vec::with_capacity(n),trades:vec![],skipped:Skipped::default(),events:vec![],costs_total:0.};
    let mut a=Account {cash:b.capital,paid:0.,held:lanes.iter().map(|_|None).collect(),pending:lanes.iter().map(|_|None).collect(),marks:vec![f64::NAN;lanes.len()],count:0};
    // Coverage end is a declared historical adaptation, not the final quote BEFORE a selected window's holiday.
    let terminal:Vec<_>=lanes.iter().map(|ln| {
        pulse(deadline,stop)?;
        let last=ln.m.at.iter().rposition(Option::is_some);
        Ok(last.filter(|&i|i<to&&ln.m.frames.d.date.last().is_some_and(|d|d<=&dates[to])))
    }).collect::<Result<_,LabError>>()?;
    let mut queued=Vec::<Candidate>::new();
    for i in from..=to {
        pulse(deadline,stop)?;let mut exited=vec![false;lanes.len()];
        // Open exits precede ranked entries; a missing quote keeps an exit but cancels an entry.
        for &m in &order {pulse(deadline,stop)?;if let Some(k)=lanes[m].m.at[i] {
            if let Some(p)=a.pending[m].take(){if a.held[m].as_ref().is_some_and(|h|h.entry.i==p.entry_i){a.close(&mut out,b,m,i,lanes[m].m.frames.d.open[k],p.signal_i,p.reason)?;}}
        }}
        for c in std::mem::take(&mut queued) {
            pulse(deadline,stop)?;
            if i==to {out.skipped.final_bar+=1;out.events.push(format!("{}: entry skipped on the final reference bar {}",lanes[c.lane].m.ticker,dates[i]));continue;}
            if terminal[c.lane]==Some(i) {out.skipped.data_end+=1;out.events.push(format!("{}: entry skipped on coverage-ending {}",lanes[c.lane].m.ticker,dates[i]));continue;}
            let Some(k)=lanes[c.lane].m.at[i] else {out.skipped.no_bar+=1;continue;};
            a.open(&mut out,b,&lanes[c.lane],&c,i,k,lanes[c.lane].m.frames.d.open[k],"next_open")?;
        }
        for &m in &order {pulse(deadline,stop)?;let Some(k)=lanes[m].m.at[i] else {continue;};
            let hit=a.held[m].as_mut().and_then(|h|h.position.price_exit(&lanes[m].c,&lanes[m].m.frames.d,k));
            if let Some((px,reason))=hit {a.close(&mut out,b,m,i,px,i,reason)?;exited[m]=true;}
            a.marks[m]=lanes[m].m.frames.d.close[k];
        }
        for &m in &order {pulse(deadline,stop)?;if terminal[m]==Some(i)&&a.held[m].is_some(){
            let k=lanes[m].m.at[i].unwrap();a.close(&mut out,b,m,i,lanes[m].m.frames.d.close[k],i,"data_end".into())?;exited[m]=true;
            out.events.push(format!("{}: data ends {}; closed at the last close (historical coverage-end adaptation)",lanes[m].m.ticker,dates[i]));
        }}
        if b.mode==Mode::Signals {
            for &m in &order {pulse(deadline,stop)?;let (Some(k),Some(h))=(lanes[m].m.at[i],a.held[m].as_ref()) else {continue;};
                if a.pending[m].is_some()||k-h.entry_k<lanes[m].c.exec.min_bars {continue;}
                let why=lanes[m].c.exit.iter().find(|r|r.holds(k,Some(h.entry_k))).map(|r|r.label.clone());
                if let Some(reason)=why {match b.exit_fill {
                    Fill::Close=>{a.close(&mut out,b,m,i,lanes[m].m.frames.d.close[k],i,reason)?;exited[m]=true;},
                    Fill::NextOpen if i<to=>a.pending[m]=Some(PendingExit {signal_i:i,entry_i:h.entry.i,reason}),_=>{},
                }}
            }
        }
        let rebalance=b.mode==Mode::Rotation&&b.rebalance.map_or(true,|anchor|i<to&&super::resample::period_key(&dates[i],anchor)!=super::resample::period_key(&dates[i+1],anchor));
        let mut ranked=vec![];
        if rebalance {
            let (_,equity)=a.value(&order)?;
            for &m in &order {pulse(deadline,stop)?;let Some(k)=lanes[m].m.at[i] else {continue;};if level(&lanes[m],k) {if let Some(c)=candidate(&lanes[m],m,i,k,equity,b)?{ranked.push(c);}}}
            rank(&mut ranked,lanes);
            for &m in &order {pulse(deadline,stop)?;let (Some(k),Some(h))=(lanes[m].m.at[i],a.held[m].as_ref()) else {continue;};
                if a.pending[m].is_some()||k-h.entry_k<lanes[m].c.exec.min_bars {continue;}
                let reason=match ranked.iter().find(|c|c.lane==m) {None=>Some("not eligible".into()),Some(c) if c.rank>b.worst_rank_held=>Some(format!("rank exit (rank {} > worst_rank_held {})",c.rank,b.worst_rank_held)),_=>None};
                if let Some(reason)=reason {match b.exit_fill {Fill::Close=>{a.close(&mut out,b,m,i,lanes[m].m.frames.d.close[k],i,reason)?;exited[m]=true;},Fill::NextOpen if i<to=>a.pending[m]=Some(PendingExit {signal_i:i,entry_i:h.entry.i,reason}),_=>{}}}
            }
        }
        // Observe completed highs only AFTER price checks and BEFORE any close-filled new position exists.
        for &m in &order {pulse(deadline,stop)?;if let (Some(k),Some(h))=(lanes[m].m.at[i],a.held[m].as_mut()){h.position.observe_completed_bar(&lanes[m].m.frames.d,k);}}
        let (_,signal_equity)=a.value(&order)?;
        if i<to&&(b.mode==Mode::Signals||rebalance) {
            let mut entries=vec![];
            if b.mode==Mode::Signals {
                for &m in &order {pulse(deadline,stop)?;let Some(k)=lanes[m].m.at[i] else {continue;};
                    let flat=a.held[m].is_none()||(b.fill==Fill::NextOpen&&a.pending[m].is_some());
                    if !flat||(b.fill==Fill::Close&&exited[m])||!fires(&lanes[m],k){continue;}
                    if let Some(c)=candidate(&lanes[m],m,i,k,signal_equity,b)?{entries.push(c);}
                }
                rank(&mut entries,lanes);
            } else {
                // Preserve ranks in the full eligible universe, including held/non-edge names.
                for mut c in ranked {let m=c.lane;let k=c.signal_k;
                    let flat=a.held[m].is_none()||(b.fill==Fill::NextOpen&&a.pending[m].is_some());
                    if !flat||(b.fill==Fill::Close&&exited[m])||!fires(&lanes[m],k){continue;}
                    let (qty,requested)=quantity(b,&lanes[m],k,signal_equity)?;c.qty=qty;c.requested=requested;c.equity=signal_equity;entries.push(c);
                }
            }
            match b.fill {
                Fill::NextOpen=>queued=entries,
                Fill::Close=>for c in entries {pulse(deadline,stop)?;let m=c.lane;let k=c.signal_k;
                    if terminal[m]==Some(i){out.skipped.data_end+=1;out.events.push(format!("{}: close entry skipped on coverage-ending {}",lanes[m].m.ticker,dates[i]));continue;}
                    a.open(&mut out,b,&lanes[m],&c,i,k,lanes[m].m.frames.d.close[k],"close")?;
                },
            }
        }
        let (invested,equity)=a.value(&order)?;
        if equity<=0. {return Err(invalid("calculation_unavailable: nonpositive portfolio equity; no result is published"));}
        if a.cash<0.||a.count>b.max_open||invested>equity+1e-8*equity.abs().max(1.){return Err(invalid("portfolio cash/slot accounting identity failed; no result is published"));}
        out.cash.push(a.cash);out.invested.push(invested);out.equity.push(equity);out.gross_equity.push(finite(equity+a.paid,"same-fills gross equity")?);out.positions.push(a.count);
    }
    pulse(deadline,stop)?;
    for &m in &order {if let Some(h)=a.held[m].take(){out.trades.push(finish(m,h,to,a.marks[m],to,"open".into(),None));}}
    out.trades.sort_by(|a,b|a.entry_i.cmp(&b.entry_i).then(lanes[a.lane].m.ticker.cmp(&lanes[b.lane].m.ticker)).then(a.exit_i.cmp(&b.exit_i)));
    out.costs_total=a.paid;Ok(out)
}

#[cfg(test)]
pub(crate) mod tests {
    use super::{run, Book, Lane, Ledger};
    use crate::lab::{rules::{Dist, Fill, Mode, MoneyCosts, Op, Sizing, TaxSide},series::{Compiled,Frames,RCond,Val},testdata::from_ohlc,universe::{Member,calendar_map}};
    use std::{sync::atomic::AtomicBool,time::{Duration,Instant}};
    pub(crate) fn dates(n:usize)->Vec<String> {(0..n).map(|i|crate::lab::resample::civil(crate::lab::resample::day_number("2024-01-01")+i as i64)).collect()}
    pub(crate) fn member(t:&str,base:f64,n:usize,gaps:&[usize])->Member {
        let all=dates(n);let keep:Vec<_>=(0..n).filter(|i|!gaps.contains(i)).collect();
        let mut d=from_ohlc(&keep.iter().map(|&i|{let o=base+i as f64;(o,o+1.,o-1.,o+0.5)}).collect::<Vec<_>>());
        d.date=keep.iter().map(|&i|all[i].clone()).collect();let at=calendar_map(&all,&d.date);
        Member {ticker:t.into(),kind:"individual_stock",frames:Frames::new(d),at}
    }
    pub(crate) fn lane<'a>(m:&'a Member,on:&[usize],off:&[usize],score:Option<Vec<f64>>)->Lane<'a> {
        let sig=|r:&[usize]| {let mut v=vec![0.;m.frames.d.len()];for &i in r {if let Some(k)=m.at[i] {v[k]=1.;}} v};
        let rule=|v:Vec<f64>|RCond {lhs:Val::Arr(v),op:Op::Gt,rhs:Val::Const(0.5),label:"rule".into()};
        Lane {m,c:Compiled::new(vec![rule(sig(on))],if off.is_empty(){vec![]}else{vec![rule(sig(off))]}),score}
    }
    pub(crate) fn book(max_open:usize,size:Sizing)->Book {
        Book {capital:10_000.,mode:Mode::Signals,max_open,worst_rank_held:max_open,rebalance:None,lowest:false,fill:Fill::NextOpen,exit_fill:Fill::NextOpen,size,costs:MoneyCosts::default()}
    }
    pub(crate) fn go(lanes:&[Lane],b:&Book,n:usize)->Ledger {let l=run(lanes,b,&dates(n),0,n-1,Instant::now()+Duration::from_secs(30),None).unwrap();invariants(&l,b);l}
    pub(crate) fn invariants(l:&Ledger,b:&Book) {
        for i in 0..l.equity.len() {let tol=1e-8*l.equity[i].abs().max(1.);assert!((l.equity[i]-l.cash[i]-l.invested[i]).abs()<tol);assert!(l.cash[i]>=-tol);assert!(l.positions[i]<=b.max_open);assert!(l.gross_equity[i]>=l.equity[i]-tol);}
        for t in &l.trades {assert!(t.qty>=1.&&t.qty.fract()==0.);assert!(t.entry_px.is_finite()&&t.exit_px.is_finite());}
    }
    fn close_book(max:usize,size:Sizing)->Book {Book {fill:Fill::Close,exit_fill:Fill::Close,..book(max,size)}}
    fn rotation(max:usize,worst:usize,period:Option<usize>)->Book {Book {mode:Mode::Rotation,worst_rank_held:worst,rebalance:period,..book(max,Sizing::PctEquity(100./max as f64))}}
    fn constant(t:&str,price:f64,n:usize)->Member {let mut m=member(t,price,n,&[]);m.frames.d=from_ohlc(&vec![(price,price+1.,price-1.,price);n]);m.frames.d.date=dates(n);m.frames=Frames::new(m.frames.d);m}

    #[test] fn one_member_uses_identifiable_fills_and_signal_close_quantity() {
        let m=member("A",100.,10,&[]);let lanes=[lane(&m,&[2],&[5],None)];let l=go(&lanes,&book(1,Sizing::PctEquity(50.)),10);let t=&l.trades[0];
        assert_eq!((t.signal_i,t.entry_i,t.entry_px,t.exit_signal_i,t.exit_i,t.exit_px,t.qty,t.open),(2,3,103.,5,6,106.,48.,false));assert_eq!(l.equity[9],10_144.);
    }
    #[test] fn slots_and_cash_are_shared_and_open_exits_free_them_before_entries() {
        let (a,b,c)=(member("A",100.,12,&[]),member("B",200.,12,&[]),member("C",300.,12,&[]));
        let lanes=[lane(&a,&[2],&[5],None),lane(&b,&[2],&[],None),lane(&c,&[4,5],&[],None)];let l=go(&lanes,&book(2,Sizing::PctEquity(50.)),12);
        assert_eq!(l.trades.iter().map(|t|(t.lane,t.entry_i)).collect::<Vec<_>>(),[(0,3),(1,3),(2,6)]);assert_eq!(l.skipped.no_slot,1);
    }
    #[test] fn close_fills_do_not_reenter_the_close_that_exited() {
        let m=member("A",100.,10,&[]);let l=go(&[lane(&m,&[2,4],&[4],None)],&close_book(1,Sizing::Full),10);
        assert_eq!(l.trades.iter().map(|t|(t.entry_i,t.entry_px,t.exit_i)).collect::<Vec<_>>(),[(2,102.5,4)]);
    }
    #[test] fn fixed_value_units_and_risk_quantities_are_not_the_ranking_measure() {
        let m=member("A",100.,10,&[]);
        for (size,qty) in [(Sizing::Value(2000.),19.),(Sizing::Units(5.),5.),(Sizing::RiskPct(1.),19.)] {
            let mut ln=lane(&m,&[2],&[],None);ln.c.exec.stop_loss=Some(Dist::Pct(5.));let l=go(&[ln],&book(1,size),10);assert_eq!(l.trades[0].qty,qty);
        }
    }
    #[test] fn costs_shrink_whole_shares_and_gross_uses_the_same_booked_quantities() {
        let (a,b)=(constant("A",100.,6),constant("B",100.,6));let lanes=[lane(&a,&[1],&[],None),lane(&b,&[1],&[],None)];
        let cfg=Book {capital:1000.,costs:MoneyCosts {per_order:20.,..MoneyCosts::default()},..book(2,Sizing::Units(5.))};let l=go(&lanes,&cfg,6);
        assert_eq!(l.trades.iter().map(|t|t.qty).collect::<Vec<_>>(),[5.,4.]);assert_eq!((l.cash[2],l.invested[2],l.equity[2],l.gross_equity[2]),(60.,900.,960.,1000.));assert_eq!(l.costs_total,40.);
    }
    #[test] fn effective_trades_reconcile_declared_costs_without_changing_observed_fill_prices() {
        let a=constant("A",100.,6);let cfg=Book {costs:MoneyCosts {per_order:2.,slip_bps:10.,taxes:vec![("Declared".into(),0.1,TaxSide::Sell)],..MoneyCosts::default()},..book(1,Sizing::Units(10.))};
        let l=go(&[lane(&a,&[1],&[3],None)],&cfg,6);let t=&l.trades[0];assert_eq!((t.entry_px,t.exit_px),(100.,100.));assert!((t.pnl()+7.).abs()<1e-9);assert!((t.as_trade(true).ret(0.)-t.pnl()/t.equity_at_signal).abs()<1e-12);assert_eq!(t.as_trade(false).ret(0.),0.);assert_eq!(t.as_trade(true).signal_i,3);
    }
    #[test] fn score_ranks_then_ticker_ties_independent_of_price_and_input_order() {
        let (a,z)=(member("AAA",100.,10,&[]),member("ZZZ",10.,10,&[]));
        let l=go(&[lane(&z,&[2],&[],Some(vec![1.;10])),lane(&a,&[2],&[],Some(vec![1.;10]))],&book(1,Sizing::Value(2000.)),10);
        assert_eq!(l.trades[0].lane,1);assert_eq!((l.trades[0].qty,l.trades[0].rank),(19.,1));
        let l=go(&[lane(&a,&[2],&[],Some(vec![40.;10])),lane(&z,&[2],&[],Some(vec![20.;10]))],&Book {lowest:true,..book(1,Sizing::PctEquity(25.))},10);assert_eq!(l.trades[0].lane,1);assert_eq!(l.trades[0].qty,200.);
    }
    #[test] fn p1_lower_rsi_prefers_b_then_c_with_real_percentage_quantities() {
        let members:Vec<_>=["A","B","C","D"].iter().enumerate().map(|(i,t)|member(t,100.+i as f64*50.,10,&[])).collect();
        let lanes:Vec<_>=members.iter().zip([40.,20.,35.,60.]).map(|(m,score)|lane(m,&[2],&[],Some(vec![score;10]))).collect();
        let l=go(&lanes,&Book {lowest:true,..book(2,Sizing::PctEquity(25.))},10);
        assert_eq!(l.trades.iter().map(|t|lanes[t.lane].m.ticker.as_str()).collect::<Vec<_>>(),["B","C"]);
        assert_eq!(l.trades.iter().map(|t|(t.rank,t.qty)).collect::<Vec<_>>(),[(1,16.),(2,12.)]);assert_eq!(l.skipped.no_slot,2);
    }
    #[test] fn nan_and_infinite_scores_never_replace_finite_candidates() {
        let members:Vec<_>=["A","B","C","D"].iter().map(|t|constant(t,100.,6)).collect();
        for lowest in [false,true] {let lanes:Vec<_>=members.iter().zip([f64::NAN,f64::INFINITY,f64::NEG_INFINITY,0.]).map(|(m,s)|lane(m,&[1],&[],Some(vec![s;6]))).collect();let l=go(&lanes,&Book {lowest,..book(1,Sizing::Units(1.))},6);assert_eq!(l.trades.len(),1);assert_eq!(l.trades[0].lane,3);}
    }
    #[test] fn rotation_tries_lower_ranks_when_the_best_name_has_no_cash_or_no_bar() {
        for fill in [Fill::Close,Fill::NextOpen] {
            let (a,b)=(constant("A",2000.,6),constant("B",100.,6));let lanes=[lane(&a,&[1],&[],Some(vec![2.;6])),lane(&b,&[1],&[],Some(vec![1.;6]))];
            let l=go(&lanes,&Book {capital:1000.,fill,exit_fill:fill,..rotation(1,1,None)},6);assert_eq!(l.trades[0].lane,1);assert!(l.skipped.no_cash>=1);
        }
        let (a,b)=(member("A",100.,6,&[2]),member("B",100.,6,&[]));let lanes=[lane(&a,&[1],&[],Some(vec![2.;5])),lane(&b,&[1],&[],Some(vec![1.;6]))];let l=go(&lanes,&rotation(1,1,None),6);assert_eq!(l.trades[0].lane,1);assert_eq!(l.skipped.no_bar,1);
    }
    #[test] fn rotation_keeps_holdings_until_worst_rank_and_preserves_full_universe_rank() {
        let members:Vec<_>=["A","B","C","D"].iter().map(|t|member(t,100.,12,&[])).collect();let mut scores=vec![vec![4.;12],vec![3.;12],vec![2.;12],vec![1.;12]];
        for i in 5..12 {scores[0][i]=2.;scores[1][i]=1.;scores[2][i]=4.;scores[3][i]=3.;}for i in 8..12 {scores[0][i]=0.5;}
        let all:Vec<_>=(0..12).collect();let lanes:Vec<_>=members.iter().zip(scores).map(|(m,s)|lane(m,&all,&[],Some(s))).collect();let l=go(&lanes,&rotation(2,3,None),12);
        let held=|i| {let mut out:Vec<_>=l.trades.iter().filter(|t|t.entry_i<=i&&(t.open||t.exit_i>i)).map(|t|lanes[t.lane].m.ticker.as_str()).collect();out.sort();out};
        assert_eq!(held(1),["A","B"]);assert_eq!(held(6),["A","C"]);assert_eq!(held(9),["C","D"]);assert!(l.trades.iter().any(|t|t.reason.starts_with("rank exit (rank 4 > worst_rank_held 3)")));
    }
    #[test] fn rotation_edge_gates_new_entries_without_making_held_levels_ineligible() {
        let (a,b)=(constant("A",100.,8),constant("B",100.,8));let all:Vec<_>=(0..8).collect();let mut sa=vec![1.;8];for v in &mut sa[3..] {*v=3.;}
        let mut x=lane(&a,&all,&[],Some(sa));x.c.exec.edge=true;let mut y=lane(&b,&all,&[],Some(vec![2.;8]));y.c.exec.edge=true;
        let l=go(&[x,y],&rotation(1,1,None),8);assert_eq!(l.trades.len(),1);assert_eq!(l.trades[0].lane,1);assert_eq!((l.trades[0].entry_i,l.trades[0].exit_i),(1,4));assert!(l.trades[0].reason.starts_with("rank exit"));assert_eq!(l.positions[5],0,"a flat non-edge name still ranks but cannot replace the ranked-out holding");
    }
    #[test] fn monthly_rotation_waits_for_completed_reference_months() {
        let (a,b)=(member("A",100.,70,&[]),member("B",100.,70,&[]));let all:Vec<_>=(0..70).collect();let first:Vec<_>=(0..40).collect();
        let l=go(&[lane(&a,&first,&[],Some(vec![2.;70])),lane(&b,&all,&[],Some(vec![1.;70]))],&rotation(1,1,Some(2)),70);
        assert_eq!((l.trades[0].signal_i,l.trades[0].entry_i,l.trades[0].exit_signal_i,l.trades[0].exit_i),(30,31,59,60));assert_eq!(l.trades[1].lane,1);
    }
    #[test] fn daily_rotation_realizes_final_close_rank_exits_but_never_final_entries() {
        let (a,b)=(member("A",100.,6,&[]),member("B",100.,6,&[]));let all:Vec<_>=(0..6).collect();let mut score=vec![2.;6];score[5]=0.5;
        let l=go(&[lane(&a,&all,&[],Some(score)),lane(&b,&all,&[],Some(vec![1.;6]))],&Book {exit_fill:Fill::Close,..rotation(1,1,None)},6);
        assert_eq!(l.trades.len(),1);assert!(!l.trades[0].open);assert_eq!((l.trades[0].exit_i,l.trades[0].exit_px),(5,105.5));assert_eq!(l.positions[5],0);
    }
    #[test] fn unfinished_final_calendar_periods_do_not_rotate() {
        for (anchor,n,entry) in [(1,12,7),(2,40,31),(3,100,91),(4,380,366)] {
            let (a,b)=(member("A",100.,n,&[]),member("B",100.,n,&[]));let all:Vec<_>=(0..n).collect();let mut score=vec![2.;n];score[n-1]=0.5;
            let l=go(&[lane(&a,&all,&[],Some(score)),lane(&b,&all,&[],Some(vec![1.;n]))],&Book {exit_fill:Fill::Close,..rotation(1,1,Some(anchor))},n);
            assert_eq!(l.trades.len(),1);assert!(l.trades[0].open);assert_eq!(l.trades[0].entry_i,entry);
        }
    }
    #[test] fn rotation_min_bars_counts_member_sessions_without_freezing_other_lanes() {
        let (a,b,c)=(member("A",100.,10,&[2,3,5,6]),member("B",100.,10,&[]),member("C",100.,10,&[]));
        let all:Vec<_>=(1..10).collect();let until:Vec<_>=(0..4).collect();let mut a_lane=lane(&a,&[1],&[],Some(vec![3.,3.,0.,0.,0.,0.]));a_lane.c.exec.min_bars=2;
        let b_lane=lane(&b,&until,&[],Some(vec![2.;10]));let c_lane=lane(&c,&all,&[],Some(vec![1.;10]));
        let l=go(&[a_lane,b_lane,c_lane],&Book {fill:Fill::Close,exit_fill:Fill::Close,..rotation(2,2,None)},10);
        assert_eq!(l.trades.iter().find(|t|t.lane==0).unwrap().exit_i,7,"entry Jan2 ownbar1: min2 bars reached Jan8 ownbar3, not Jan5 ownbar2");assert_eq!(l.trades.iter().find(|t|t.lane==1).unwrap().exit_i,4);assert_eq!(l.trades.iter().find(|t|t.lane==2).unwrap().entry_i,4);
    }
    #[test] fn replacement_sizing_uses_equity_after_same_close_exit_taxes() {
        let (a,b)=(constant("A",100.,6),constant("B",100.,6));let lanes=[lane(&a,&[1],&[3],None),lane(&b,&[3],&[],None)];
        let cfg=Book {capital:1000.,costs:MoneyCosts {taxes:vec![("Declared".into(),10.,TaxSide::Sell)],..MoneyCosts::default()},..close_book(1,Sizing::PctEquity(100.))};let l=go(&lanes,&cfg,6);assert_eq!(l.trades[1].qty,9.);assert_eq!(l.trades[0].exit_cost,100.);
    }
    #[test] fn missing_member_days_keep_the_last_mark_cancel_entries_and_retain_exits() {
        let a=member("A",100.,10,&[5]);let l=go(&[lane(&a,&[2],&[4],None)],&book(1,Sizing::Units(10.)),10);
        assert_eq!((l.trades[0].exit_signal_i,l.trades[0].exit_i),(4,6));assert_eq!(l.invested[5],1045.);
        let l=go(&[lane(&a,&[4],&[],None)],&book(1,Sizing::Units(10.)),10);assert!(l.trades.is_empty());assert_eq!(l.skipped.no_bar,1);
    }
    #[test] fn price_exit_levels_anchor_raw_fills_and_targets_pay_no_exit_slippage() {
        let mut a=constant("A",100.,8);a.frames.d.low[4]=94.;let mut ln=lane(&a,&[1],&[],None);ln.c.exec.stop_loss=Some(Dist::Pct(5.));
        let b=Book {costs:MoneyCosts {slip_bps:50.,..MoneyCosts::default()},..book(1,Sizing::Units(10.))};let l=go(&[ln],&b,8);assert_eq!((l.trades[0].entry_px,l.trades[0].exit_px),(100.,95.));assert_eq!(l.trades[0].entry.slippage,5.);
        a.frames.d.low[4]=99.;a.frames.d.high[4]=106.;let mut ln=lane(&a,&[1],&[],None);ln.c.exec.target=Some(Dist::Pct(5.));let l=go(&[ln],&b,8);assert_eq!(l.trades[0].exit_px,105.);assert_eq!(l.trades[0].exit.as_ref().unwrap().slippage,0.);
    }
    #[test] fn price_exit_uses_completed_highs_and_never_checks_a_close_entry_preentry_range() {
        let mut a=constant("A",100.,8);a.frames.d.high[2]=150.;a.frames.d.low[3]=99.;let mut ln=lane(&a,&[2],&[],None);ln.c.exec.trailing=Some(Dist::Pct(2.));
        let l=go(&[ln],&close_book(1,Sizing::Units(1.)),8);assert!(l.trades[0].open,"entryclose must not inherit that day's high beforeentry");
        a.frames.d.high[3]=106.;a.frames.d.low[3]=99.;let mut ln=lane(&a,&[2],&[],None);ln.c.exec.trailing=Some(Dist::Pct(2.));ln.c.exec.trail_activate=Some(Dist::Pct(3.));let l=go(&[ln],&close_book(1,Sizing::Units(1.)),8);assert!(l.trades[0].exit_i>=4);
    }
    #[test] fn data_end_closes_positions_and_never_opens_a_terminal_close_entry() {
        let gone={let mut m=member("GONE",100.,6,&[]);m.at.extend([None;6]);m};let next=member("NEXT",100.,12,&[]);
        let l=go(&[lane(&gone,&[1],&[],None),lane(&next,&[7],&[],None)],&book(1,Sizing::Full),12);assert_eq!((l.trades[0].exit_i,l.trades[0].exit_px,l.trades[0].reason.as_str()),(5,105.5,"data_end"));assert_eq!(l.trades[1].entry_i,8);assert!(l.events.iter().any(|e|e.contains("GONE: data ends")));
        let l=go(&[lane(&gone,&[5],&[],None),lane(&next,&[5],&[],None)],&close_book(1,Sizing::Full),12);assert_eq!(l.trades.len(),1);assert_eq!(l.trades[0].lane,1);assert_eq!(l.skipped.data_end,1);
    }
    #[test] fn a_window_ending_on_a_member_holiday_is_not_a_data_end_liquidation() {
        let a=member("A",100.,10,&[5]);let lanes=[lane(&a,&[1],&[],None)];let cfg=book(1,Sizing::Units(1.));
        let l=run(&lanes,&cfg,&dates(10),0,5,Instant::now()+Duration::from_secs(30),None).unwrap();
        assert!(l.trades[0].open);assert_eq!(l.trades[0].exit_i,5);assert_eq!(l.trades[0].exit_px,104.5);assert!(l.events.is_empty());assert_eq!(l.positions[5],1);
    }
    #[test] fn queued_entries_never_open_on_actual_member_coverage_end() {
        let gone={let mut m=member("GONE",100.,6,&[]);m.at.extend([None;6]);m};let next=member("NEXT",100.,12,&[]);
        let l=go(&[lane(&gone,&[4],&[],None),lane(&next,&[4],&[],None)],&book(1,Sizing::Full),12);
        assert_eq!(l.trades.len(),1);assert_eq!((l.trades[0].lane,l.trades[0].entry_i),(1,5));assert_eq!(l.skipped.data_end,1);
    }
    #[test] fn queued_entries_never_open_on_the_final_reference_bar() {
        let a=member("A",100.,6,&[]);let l=go(&[lane(&a,&[4],&[],None)],&book(1,Sizing::Full),6);
        assert!(l.trades.is_empty());assert_eq!(l.skipped.final_bar,1);assert!(l.events.iter().any(|e|e.contains("final reference bar")));
    }
    #[test] fn risk_quantity_and_stop_levels_use_signal_atr_not_fill_bar_atr() {
        let mut a=member("A",100.,8,&[]);a.frames.d.low[4]=90.;let mut ln=lane(&a,&[2],&[],None);
        ln.c.exec.stop_loss=Some(Dist::Atr {mult:1.,length:3});ln.c.atr.insert(3,vec![f64::NAN,f64::NAN,5.,50.,50.,50.,50.,50.]);
        let l=go(&[ln],&book(1,Sizing::RiskPct(1.)),8);assert_eq!((l.trades[0].qty,l.trades[0].entry_i,l.trades[0].exit_i,l.trades[0].exit_px),(20.,3,4,98.));
    }
    #[test] fn next_open_may_reenter_after_a_price_exit_but_close_fill_may_not() {
        let mut a=constant("A",100.,8);a.frames.d.low[4]=90.;
        for (fill,count) in [(Fill::NextOpen,2),(Fill::Close,1)] {let mut ln=lane(&a,&[1,4],&[],None);ln.c.exec.stop_loss=Some(Dist::Pct(5.));let l=go(&[ln],&Book {fill,..book(1,Sizing::Units(1.))},8);assert_eq!(l.trades.len(),count);}
    }
    #[test] fn edge_only_looks_at_the_members_previous_session() {
        let a=member("A",100.,10,&[3]);let mut ln=lane(&a,&[1,2,4,5,6,7,8],&[4],None);ln.c.exec.edge=true;
        let l=go(&[ln],&book(1,Sizing::Units(1.)),10);assert_eq!(l.trades.len(),1);
    }
    #[test] fn stopped_or_expired_requests_publish_no_partial_result() {
        let m=member("A",100.,10,&[]);let lanes=[lane(&m,&[2],&[],None)];let b=book(1,Sizing::Full);let stop=AtomicBool::new(true);
        for (deadline,cancel) in [(Instant::now()-Duration::from_secs(1),None),(Instant::now()+Duration::from_secs(10),Some(&stop))] {assert_eq!(run(&lanes,&b,&dates(10),0,9,deadline,cancel).err().unwrap().status(),"invalid");}
    }
    #[test] fn malformed_alignment_and_score_lengths_refuse_before_indexing() {
        let mut m=member("A",100.,10,&[]);m.at[2]=Some(99);assert!(run(&[lane(&m,&[1],&[],None)],&book(1,Sizing::Full),&dates(10),0,9,Instant::now()+Duration::from_secs(10),None).is_err());
        let m=member("A",100.,10,&[]);assert!(run(&[lane(&m,&[1],&[],Some(vec![1.;2]))],&book(1,Sizing::Full),&dates(10),0,9,Instant::now()+Duration::from_secs(10),None).is_err());
    }
    #[test] fn shuffled_universes_have_identical_named_trades_and_curves() {
        let (a,b)=(member("A",100.,10,&[]),member("B",200.,10,&[]));let cfg=book(2,Sizing::PctEquity(40.));
        let x=[lane(&a,&[2],&[5],None),lane(&b,&[2],&[6],None)];let y=[lane(&b,&[2],&[6],None),lane(&a,&[2],&[5],None)];let l=go(&x,&cfg,10);let r=go(&y,&cfg,10);
        let named=|l:&Ledger,lanes:&[Lane]|l.trades.iter().map(|t|(lanes[t.lane].m.ticker.clone(),t.entry_i,t.entry_px,t.qty,t.exit_i,t.exit_px)).collect::<Vec<_>>();assert_eq!(named(&l,&x),named(&r,&y));assert_eq!(l.equity,r.equity);assert_eq!(l.cash,r.cash);
    }
    #[test] fn future_prices_and_scores_never_change_already_closed_trades() {
        let n=50;let all:Vec<_>=(1..40).collect();let exits:Vec<_>=(4..45).step_by(5).collect();let a=member("A",100.,n,&[]);let b=member("B",120.,n,&[]);let cfg=book(1,Sizing::PctEquity(25.));
        let lanes=[lane(&a,&all,&exits,Some(vec![1.;n])),lane(&b,&all,&exits,Some(vec![0.;n]))];let base=go(&lanes,&cfg,n);assert!(base.trades.iter().filter(|t|!t.open).count()>=3,"nonvacuous");
        for cut in [10,20,30] {let mut a=member("A",100.,n,&[]);let mut b=member("B",120.,n,&[]);let mut sa=vec![1.;n];let mut sb=vec![0.;n];for i in cut+1..n {for m in [&mut a,&mut b] {m.frames.d.open[i]*=2.;m.frames.d.high[i]*=2.;m.frames.d.low[i]*=2.;m.frames.d.close[i]*=2.;}sa[i]=-1.;sb[i]=5.;}
            let changed=go(&[lane(&a,&all,&exits,Some(sa)),lane(&b,&all,&exits,Some(sb))],&cfg,n);let closed=|l:&Ledger|l.trades.iter().filter(|t|!t.open&&t.exit_i<=cut).map(|t|(t.lane,t.entry_i,t.entry_px,t.qty,t.exit_i,t.exit_px)).collect::<Vec<_>>();assert_eq!(closed(&base),closed(&changed));}
    }
    fn named_closed(l:&Ledger,lanes:&[Lane],cut:usize)->Vec<String> {
        l.trades.iter().filter(|t|!t.open&&t.exit_i<=cut).map(|t|format!("{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}",lanes[t.lane].m.ticker,t.signal_i,t.entry_i,t.exit_signal_i,t.exit_i,t.entry_px.to_bits(),t.exit_px.to_bits(),t.qty.to_bits(),t.entry_cost.to_bits(),t.exit_cost.to_bits(),t.rank,t.score.to_bits(),t.equity_at_signal.to_bits(),t.reason)).collect()
    }
    fn causal_members(n:usize)->Vec<Member> {
        (0..5).map(|j| {
            let gaps:Vec<_>=(0..n).filter(|i|i%13==j+2).collect();
            let mut m=member(&format!("M{j}"),100.+j as f64*7.,n,&gaps);
            for k in 0..m.frames.d.len() {
                let d=&mut m.frames.d;let wave=((k*(j+3))%11) as f64-5.;d.close[k]+=wave;
                d.high[k]=d.high[k].max(d.close[k])+1.;d.low[k]=d.low[k].min(d.close[k])-1.;
            }
            m.frames=Frames::new(m.frames.d);m
        }).collect()
    }
    fn causal_lanes<'a>(members:&'a [Member],mode:Mode,n:usize)->Vec<Lane<'a>> {
        members.iter().map(|m| {
            let d=&m.frames.d;
            let on:Vec<_>=(0..n).filter(|&i|m.at[i].is_some_and(|k|d.close[k]>d.open[k])).collect();
            let off:Vec<_>=(0..n).filter(|i|i%5==0).collect();
            let mut ln=lane(m,&on,if mode==Mode::Signals {&off}else{&[]},Some(d.close.clone()));
            ln.c.exec.min_bars=2;ln
        }).collect()
    }
    #[test] fn every_mode_and_fill_keeps_the_full_ledger_prefix_when_future_member_prices_change() {
        let n=80;let members=causal_members(n);
        for (mode,fill) in [(Mode::Signals,Fill::NextOpen),(Mode::Signals,Fill::Close),(Mode::Rotation,Fill::NextOpen),(Mode::Rotation,Fill::Close)] {
            let cfg=Book {mode,fill,exit_fill:fill,worst_rank_held:3,costs:MoneyCosts {per_order:0.75,per_unit:0.01,pct:0.03,slip_bps:7.,taxes:vec![("Buy".into(),0.01,TaxSide::Buy),("Sell".into(),0.02,TaxSide::Sell)],..MoneyCosts::default()},..book(2,Sizing::PctEquity(40.))};
            let lanes=causal_lanes(&members,mode,n);let full=go(&lanes,&cfg,n);
            assert!(full.trades.iter().filter(|t|!t.open).count()>=4,"{mode:?}/{fill:?} fixture must actually close positions");
            for t in full.trades.iter().filter(|t|!t.open) {
                let at=&lanes[t.lane].m.at;
                assert!(at[t.exit_signal_i].unwrap()-at[t.entry_i].unwrap()>=2,"minimum hold uses member sessions, not reference holes");
            }
            for cut in [20,47,63] {
                let changed:Vec<_>=members.iter().enumerate().map(|(j,m)| {
                    let mut d=m.frames.d.clone();
                    for (r,at) in m.at.iter().enumerate().skip(cut+1) {if let Some(k)=at {
                        let scale=1.3+((r+j*3)%7) as f64*0.05;
                        let o=d.open[*k]*scale;let c=(d.close[*k]+((r*(j+2))%9) as f64-4.)*scale;
                        d.open[*k]=o;d.close[*k]=c;d.high[*k]=d.high[*k].mul_add(scale,3.).max(o).max(c)+1.;d.low[*k]=(d.low[*k]*scale-3.).min(o).min(c)-1.;
                    }}
                    Member {ticker:m.ticker.clone(),kind:m.kind,frames:Frames::new(d),at:m.at.clone()}
                }).collect();
                let changed_lanes=causal_lanes(&changed,mode,n);let part=go(&changed_lanes,&cfg,n);
                assert_eq!(&full.equity[..=cut],&part.equity[..=cut],"{mode:?}/{fill:?} net through{cut}");
                assert_eq!(&full.gross_equity[..=cut],&part.gross_equity[..=cut],"{mode:?}/{fill:?} gross through{cut}");
                assert_eq!(&full.cash[..=cut],&part.cash[..=cut],"{mode:?}/{fill:?} cash through{cut}");
                assert_eq!(&full.invested[..=cut],&part.invested[..=cut],"{mode:?}/{fill:?} marks through{cut}");
                assert_eq!(&full.positions[..=cut],&part.positions[..=cut],"{mode:?}/{fill:?} holdings through{cut}");
                assert!(!named_closed(&full,&lanes,cut).is_empty(),"nonvacuous closed-trade prefix");
                assert_eq!(named_closed(&full,&lanes,cut),named_closed(&part,&changed_lanes,cut),"{mode:?}/{fill:?} trades through{cut}");
            }
        }
    }
    #[test] fn all_modes_emit_the_same_named_trade_order_and_ledger_for_shuffled_universes() {
        let n=40;let members:Vec<_>=["B","A","C"].iter().enumerate().map(|(j,t)|member(t,100.,n,&[j*3+1])).collect();
        for (mode,fill) in [(Mode::Signals,Fill::NextOpen),(Mode::Signals,Fill::Close),(Mode::Rotation,Fill::NextOpen),(Mode::Rotation,Fill::Close)] {
            let cfg=Book {mode,fill,exit_fill:fill,costs:MoneyCosts {per_order:1.,pct:0.01,..MoneyCosts::default()},..book(2,Sizing::PctEquity(40.))};
            let make=|order:&[usize]|order.iter().map(|&j|lane(&members[j],&[2,9,15,23],if mode==Mode::Signals {&[6,12,19,28]}else{&[]},Some(vec![1.;members[j].frames.d.len()]))).collect::<Vec<_>>();
            let base_lanes=make(&[0,1,2]);let base=go(&base_lanes,&cfg,n);
            assert!(base.trades.iter().any(|t|!t.open));
            for order in [[2,1,0],[1,0,2],[1,2,0]] {
                let lanes=make(&order);let part=go(&lanes,&cfg,n);
                // Do not sort the projection: the emitted ordering itself is part of determinism.
                assert_eq!(named_closed(&base,&base_lanes,n-1),named_closed(&part,&lanes,n-1));
                let all=|l:&Ledger,ls:&[Lane]|l.trades.iter().map(|t|(ls[t.lane].m.ticker.clone(),t.entry_i,t.exit_i,t.qty.to_bits(),t.open)).collect::<Vec<_>>();
                assert_eq!(all(&base,&base_lanes),all(&part,&lanes));
                assert_eq!(base.equity,part.equity);assert_eq!(base.gross_equity,part.gross_equity);assert_eq!(base.cash,part.cash);assert_eq!(base.invested,part.invested);assert_eq!(base.positions,part.positions);assert_eq!(base.skipped,part.skipped);assert_eq!(base.events,part.events);assert_eq!(base.costs_total,part.costs_total);
            }
        }
    }
    #[test] fn queued_entry_existence_rank_and_signal_quantity_do_not_read_fill_bar_prices_when_cash_is_ample() {
        for missing in [false,true] {
            let n=10;let original=[member("A",100.,n,&[]),member("B",200.,n,if missing {&[3]}else{&[]})];
            let cfg=Book {capital:1_000_000.,..book(2,Sizing::PctEquity(1.))};
            let ls:Vec<_>=original.iter().map(|m|lane(m,&[2],&[6],Some(vec![1.;m.frames.d.len()]))).collect();let full=go(&ls,&cfg,n);
            let altered:Vec<_>=original.iter().map(|m| {let mut d=m.frames.d.clone();if let Some(k)=m.at[3] {d.open[k]*=8.;d.close[k]*=7.;d.high[k]*=9.;d.low[k]*=6.;}Member {ticker:m.ticker.clone(),kind:m.kind,frames:Frames::new(d),at:m.at.clone()}}).collect();
            let ls2:Vec<_>=altered.iter().map(|m|lane(m,&[2],&[6],Some(vec![1.;m.frames.d.len()]))).collect();let part=go(&ls2,&cfg,n);
            let entries=|l:&Ledger,ls:&[Lane]|l.trades.iter().map(|t|(ls[t.lane].m.ticker.clone(),t.signal_i,t.entry_i,t.qty,t.rank)).collect::<Vec<_>>();
            let mut expected=vec![("A".into(),2,3,97.,1)];if !missing {expected.push(("B".into(),2,3,49.,2));}
            assert_eq!(entries(&full,&ls),expected);assert_eq!(entries(&part,&ls2),expected);
            assert_eq!(full.skipped.no_bar,usize::from(missing));assert_eq!(part.skipped.no_bar,usize::from(missing));
            assert_eq!(&full.equity[..=2],&part.equity[..=2]);assert_eq!(&full.cash[..=2],&part.cash[..=2]);
            assert_ne!(full.trades[0].entry_px,part.trades[0].entry_px,"actual observed fill price is allowed to change");
        }
    }

    #[test] fn exhausted_equity_is_a_typed_refusal_before_statistics() {
        let m=constant("A",100.,8);
        let lanes=[lane(&m,&[1],&[3],None)];
        let cfg=Book {capital:200.,costs:MoneyCosts {per_order:100.,..MoneyCosts::default()},..book(1,Sizing::Units(1.))};
        let result=run(&lanes,&cfg,&dates(8),0,7,Instant::now()+Duration::from_secs(10),None);
        assert!(matches!(result,Err(ref error) if error.status()=="invalid"&&error.message().contains("nonpositive")), "zero equity cannot produce ratio statistics");
    }

}
