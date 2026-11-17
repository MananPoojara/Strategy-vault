//! Provisional Narrow Range / ORB AFL. Daily future references, two rounds of ExRem/Flip,
//! trailing stops and same-bar execution remain to be reconciled before results are published.
#![allow(dead_code)]
use crate::intraday::Candles;
use crate::connors::{Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "yt_orb_intraday";
pub const ID: &str = "ytorb_narrow_range";
pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef { id:ID, name:"Intraday Narrow Range and Opening Range Breakout", group:"YT Strategies · Narrow Range ORB", session:0,
        dir:"Long/Short", status:Status::NotTestable("Held: the AFL builds long and short positions independently, so a long and a short can be open on the symbol at the same time; the engine's single-position trade list cannot hold both and AmiBroker's per-symbol arbitration of the two (the backtester keeps one position per symbol) is not yet reproduced. Its interval (5 or 15 minutes) is also not fixed by the AFL."), needs_volume:false,
        needs_vix:false, needs_series:&[], book:BOOK, rules:&["F|Narrow daily ranges followed by a gap or lap", "E|Enter on a close beyond the opening range", "X|Trailing percentage stop or end-of-day exit"],
        notes:"INTRADAY NARROW RANGE , ORB ENTRY. EXIT - %SWING AND EOD Final Version.afl. The source uses future references and two rounds of ExRem/Flip. Its description names 5- or 15-minute bars; 5 minutes is provisional. Default hard stop is off. Long and short are constructed independently, so overlapping positions may be produced; the single-position accounting engine cannot report them. Execution, data expansion and source equivalence need review; results are held.", claim:None }]
}
pub fn params(id:&str)->Option<Vec<ParamDef>> {
    if id!=ID {return None;}
    let p=|key,label,def,min,max,step|ParamDef{key,label,def,min,max,step};
    Some(vec![p("tf_min","Candle minutes (the AFL description: 5 or 15 minute)",5.,1.,60.,1.),p("narrow_days","Narrow day range",2.,1.,5.,1.),p("candles","Opening range candles",3.,1.,4.,1.),p("trail_pct","Trailing percentage",2.,0.,100.,0.5),p("hard_pct","Hard stop percentage (0 off)",0.,0.,50.,1.),p("gap_up_lap","Use lap-up (1) instead of gap-up",0.,0.,1.,1.),p("gap_down_lap","Use lap-down (1) instead of gap-down",0.,0.,1.,1.),
        p("rank_mode","Ranking mode (0 none, 1 +ROC, 2 -ROC, 3 RSI, 4 STD, 5 +ROC/STD, 6 -ROC/STD, 7 positive close, 8 swing, 9 +ROC + positive close, 10 positive close + swing)",0.,0.,10.,1.),
        p("roc_period","Ranking ROC/positive close period",10.,0.,400.,1.),p("rsi_period","Ranking RSI period",14.,10.,1000.,1.),p("std_period","Ranking standard-deviation period",14.,10.,1000.,1.),p("swing_period","Ranking swing lookback",180.,10.,1000.,1.),crate::intraday::align_param()])
}

/// Source breakout candidates given already-expanded long and short daily conditions.
/// The AFL makes its opening range on the Nth candle after `fc`, then carries it while
/// the corresponding condition remains true; `fc` is the DateNum change array.
fn opening_range_candidates(b: &Candles, long: &[bool], short: &[bool], count: usize)
    -> Result<Vec<(bool,bool)>, &'static str> {
    let n=b.len();
    if count==0 || [b.day.len(),b.h.len(),b.l.len(),long.len(),short.len()].iter().any(|&v|v!=n) {
        return Err("ORB candles and expanded conditions must align and count must be positive");
    }
    let mut fc=vec![false;n];
    for i in 1..n {fc[i]=b.day[i]!=b.day[i-1];}
    let mut upper: Option<f64>=None; let mut lower: Option<f64>=None;
    let mut out=Vec::with_capacity(n);
    for i in 0..n {
        let final_bar=i.checked_sub(count-1).is_some_and(|j|fc[j]);
        let new_upper=if long[i] && final_bar {Some(b.h[i+1-count..=i].iter().copied().fold(f64::NEG_INFINITY,f64::max))} else {None};
        let new_lower=if short[i] && final_bar {Some(b.l[i+1-count..=i].iter().copied().fold(f64::INFINITY,f64::min))} else {None};
        upper=if !fc[i] && long[i] && upper.is_some_and(|v|v>0.) {upper} else {new_upper};
        lower=if !fc[i] && short[i] && lower.is_some_and(|v|v>0.) {lower} else {new_lower};
        let last=i+1<n && b.day[i]!=b.day[i+1];
        out.push((long[i] && !last && upper.is_some_and(|v|v>0. && b.c[i]>v),
                  short[i] && lower.is_some_and(|v|v>0. && b.c[i]<v)));
    }
    Ok(out)
}

fn exrem(a:&[bool],reset:&[bool])->Vec<bool> {
    let mut armed=true;
    a.iter().zip(reset).map(|(&a,&b)| {let out=a&&armed;if out {armed=false;}if b {armed=true;}out}).collect()
}
fn flip(set:&[bool],reset:&[bool])->Vec<bool> {
    let mut on=false;
    set.iter().zip(reset).map(|(&a,&b)| {if a {on=true;}if b {on=false;}on}).collect()
}

/// Execute the source's two ExRem/Flip passes and close-based trails. The external
/// daily conditions must already be expanded onto the intraday clock.
fn orders(b:&Candles,long:&[bool],short:&[bool],count:usize,trail:f64)->Result<(Vec<bool>,Vec<bool>,Vec<bool>,Vec<bool>),&'static str> {
    if b.hhmm.len()!=b.len(){return Err("ORB timestamps must align");}
    let raw=opening_range_candidates(b,long,short,count)?;
    let n=b.len();
    let end:Vec<bool>=(0..n).map(|i|i+1<n && b.day[i]!=b.day[i+1]).collect();
    let buy0:Vec<bool>=raw.iter().map(|v|v.0).collect();
    let short0:Vec<bool>=raw.iter().map(|v|v.1).collect();
    let b1=exrem(&buy0,&end);let s1=exrem(&end,&b1);let long_open=flip(&b1,&s1);
    let sh1=exrem(&short0,&end);let c1=exrem(&end,&sh1);let short_open=flip(&sh1,&c1);
    let mut long_stop=None::<f64>;let mut short_stop=None::<f64>;
    let mut s2=Vec::with_capacity(n);let mut c2=Vec::with_capacity(n);
    for i in 0..n {
        let next_long=long_open[i].then_some(b.c[i]*(1.-trail/100.));
        let next_short=short_open[i].then_some(b.c[i]*(1.+trail/100.));
        long_stop=next_long.map(|v|long_stop.map_or(v,|p|p.max(v)));
        short_stop=next_short.map(|v|short_stop.map_or(v,|p|p.min(v)));
        s2.push(s1[i] || long_stop.is_some_and(|s|b.c[i]<=s));
        c2.push(c1[i] || short_stop.is_some_and(|s|b.c[i]>=s));
    }
    let buy=exrem(&b1,&s2);let sell1=exrem(&s2,&buy);
    let sell0:Vec<bool>=(0..n).map(|i|sell1[i] || (b.hhmm[i]<1515 && b.hhmm[i]>=920)).collect();
    let sell=exrem(&sell0,&buy);
    let short=exrem(&sh1,&c2);let cover1=exrem(&c2,&short);let cover=exrem(&cover1,&short);
    Ok((buy,sell,short,cover))
}

/// Source PositionScore array on requested entry candles. Positive-close modes execute
/// the AFL's descending nested loop exactly; a hard operation bound avoids a corpus-wide job.
fn ranking_scores(c:&[f64],entries:&[usize],mode:usize,roc_period:usize,rsi_period:usize,std_period:usize,swing_period:usize)->Result<Vec<f64>,String> {
    if mode>10 || entries.iter().any(|&i|i>=c.len()) {return Err("invalid ORB ranking mode or entry index".into());}
    if mode==0 {return Ok(vec![0.;entries.len()]);}
    let uses_roc=[1,2,5,6,9].contains(&mode);
    let uses_positive=[7,9,10].contains(&mode);
    if (uses_roc || uses_positive) && roc_period==0 {return Err("zero-period ORB ROC/ranking is unresolved".into());}
    let positive=if uses_positive {
        if c.len().saturating_mul(roc_period)>20_000_000 {return Err("ORB positive-close ranking exceeds the bounded work limit".into());}
        let mut values=vec![0.;c.len()];
        if c.len()>roc_period+1 {
            for m in (1..=c.len()-(roc_period+1)).rev() {
                for j in 1..=roc_period {let k=m+j;values[k]=values[k-1]+if c[k]>c[k-1] {1.}else{0.};}
            }
        }
        values
    }else{Vec::new()};
    let rsi=if mode==3 {crate::books::yteng::rsi(c,rsi_period)}else{Vec::new()};
    entries.iter().map(|&i| {
        let roc=||->Result<f64,String>{let base=i.checked_sub(roc_period).ok_or("ORB ROC ranking needs more history")?;let v=100.*(c[i]/c[base]-1.);if v.is_finite(){Ok(v)}else{Err("ORB ROC ranking is not finite".into())}};
        let std=||->Result<f64,String>{let start=(i+1).checked_sub(std_period).filter(|_|std_period>0).ok_or("ORB standard deviation needs more history")?;let w=&c[start..=i];let mean=w.iter().sum::<f64>()/w.len() as f64;let v=(w.iter().map(|x|(x-mean).powi(2)).sum::<f64>()/w.len() as f64).sqrt();if v.is_finite(){Ok(v)}else{Err("ORB standard deviation is not finite".into())}};
        let swing=||->Result<f64,String>{let start=(i+1).checked_sub(swing_period).filter(|_|swing_period>0).ok_or("ORB swing ranking needs more history")?;let high=c[start..=i].iter().copied().fold(f64::NEG_INFINITY,f64::max);if high>0. {Ok(100.-(high-c[i])*100./high)}else{Err("ORB swing ranking has invalid prices".into())}};
        let value=match mode {
            1=>100.+roc()?,2=>100.-roc()?,3=>rsi[i].ok_or("ORB RSI ranking needs more history")?,4=>std()?,
            5|6=>{let sd=std()?;if sd<=0. {return Err("ORB ROC/STD ranking has zero deviation".into());}let r=roc()?/sd;if mode==5 {100.+r}else{100.-r}},
            7=>positive[i],8=>swing()?,9=>100.+roc()?+positive[i],10=>swing()?+positive[i],_=>return Err("invalid ORB ranking mode".into()),
        };
        if value.is_finite(){Ok(value)}else{Err("ORB ranking score is not finite".into())}
    }).collect()
}

pub fn run(id:&str,x:&Ctx,from:usize,to:usize,p:&Params)->Option<Vec<Trade>> {
    run_capture(id,x,from,to,p,&mut Vec::new())
}
fn run_capture(id:&str,x:&Ctx,from:usize,to:usize,p:&Params,capture:&mut Vec<(usize,usize)>)->Option<Vec<Trade>> {
    if id!=ID{return None;}
    if x.r.is_empty() || from>to || to>=x.r.len(){return Some(Vec::new());}
    let ticker=format!("{}{}",crate::intraday::BASKET_PREFIX,x.r[0].ticker);
    let Some(minutes)=crate::intraday::minutes(&ticker) else{return Some(Vec::new());};
    let val=|k:&str,default:f64,min:f64,max:f64|p.get(k).copied().filter(|v|v.is_finite()).unwrap_or(default).clamp(min,max);
    let tf=val("tf_min",5.,1.,60.).round() as u16;
    let (b,di)=crate::books::ytsti::candles_for(&minutes,x,tf,crate::intraday::align_of(p));
    let narrow=val("narrow_days",2.,1.,5.).round() as usize;
    let count=val("candles",3.,1.,4.).round() as usize;
    let mut daily_nr=vec![true;x.r.len()];
    for i in narrow..x.r.len() {
        for a in 1..narrow {if x.h[i-a-1]-x.l[i-a-1]<=x.h[i-a]-x.l[i-a] {daily_nr[i]=false;}}
    }
    let mut long_daily=vec![false;x.r.len()];let mut short_daily=vec![false;x.r.len()];
    for i in 1..x.r.len() {
        long_daily[i]=if val("gap_up_lap",0.,0.,1.)>=0.5 {x.o[i]>x.c[i-1] && x.o[i]>x.o[i-1]} else {x.o[i]>x.h[i-1]};
        short_daily[i]=if val("gap_down_lap",0.,0.,1.)>=0.5 {x.o[i]<x.c[i-1] && x.o[i]<x.o[i-1]} else {x.o[i]<x.l[i-1]};
    }
    // Provisional interpretation of the source's positive Ref followed by expandLast.
    // The readiness hold prevents this interpretation from being presented as verified.
    let expanded:Vec<Option<usize>>=(0..b.len()).map(|i|{
        let d=if b.last_of_day(i){Some(di[i])}else{di[i].checked_sub(1)};
        d.and_then(|d|d.checked_add(1)).filter(|&d|d<x.r.len())
    }).collect();
    let long:Vec<bool>=expanded.iter().map(|d|d.is_some_and(|i|daily_nr[i]&&long_daily[i])).collect();
    let short:Vec<bool>=expanded.iter().map(|d|d.is_some_and(|i|daily_nr[i]&&short_daily[i])).collect();
    let Ok((buy,sell,short,cover))=orders(&b,&long,&short,count,val("trail_pct",2.,0.,100.)) else{return Some(Vec::new());};
    let mut out=Vec::new();
    for (dir,entries,exits) in [(1.,&buy,&sell),(-1.,&short,&cover)] {
        let mut pos:Option<usize>=None;
        for i in 0..b.len() {
            if di[i]>to {break;}
            if let Some(e)=pos {
                let hard=val("hard_pct",0.,0.,50.);
                let level=if dir>0. {b.c[e]*(1.-hard/100.)}else{b.c[e]*(1.+hard/100.)};
                let stop=hard>0. && if dir>0. {b.l[i]<=level}else{b.h[i]>=level};
                if exits[i] || stop {
                    let px=if stop {if dir>0. {b.o[i].min(level)}else{b.o[i].max(level)}}else{b.c[i]}.clamp(b.l[i],b.h[i]);
                    out.push((Trade{dir,legs:vec![Leg{i:di[e],px:b.c[e],w:1.}],exit_i:di[i],exit_px:px,reason:format!("ORB {:04} to {:04}; {tf}-minute candles{}",b.hhmm[e],b.hhmm[i],if stop {"; hard-stop fill is provisional"}else{""}),entry_kind:"close",signal_i:di[i],open:false,exits:Vec::new()},e,i));pos=None;
                }
            } else if di[i]>=from && entries[i] {pos=Some(i);}
        }
        if let Some(e)=pos {if let Some(i)=(e..b.len()).rev().find(|&i|di[i]<=to) {
            out.push((Trade{dir,legs:vec![Leg{i:di[e],px:b.c[e],w:1.}],exit_i:di[i],exit_px:b.c[i],reason:"Open ORB position at end of selected period".into(),entry_kind:"close",signal_i:di[i],open:true,exits:Vec::new()},e,i));
        }}
    }
    out.sort_by_key(|(t,_,_)|(t.legs[0].i,t.exit_i));
    let trades:Vec<Trade>=out.into_iter().map(|(t,e,z)|{capture.push((e,z));t}).collect();
    crate::intraday::record(&minutes,&b,tf,crate::intraday::align_of(p),&trades,capture);
    Some(trades)
}
pub(crate) fn portfolio_observations(id:&str,x:&Ctx,from:usize,to:usize,p:&Params,first_id:u64)
    ->Result<(Vec<crate::yt_intraday_portfolio::Signal>,Vec<crate::yt_intraday_portfolio::Mark>),String> {
    let schema=params(id).ok_or("unknown ORB strategy")?;
    crate::intraday::validate_research_params(&schema,p)?;
    if p.get("hard_pct").is_some_and(|v|*v>0.) {return Err("needs_data: ORB hard-stop event timing within a candle is not established".into());}
    if x.r.is_empty() || from>to || to>=x.r.len(){return Err("invalid selected daily coverage".into());}
    let ticker=format!("{}{}",crate::intraday::BASKET_PREFIX,x.r[0].ticker);
    crate::intraday::available(&ticker)?;
    let minutes=crate::intraday::minutes(&ticker).ok_or("intraday bars are unavailable")?;
    let tf=p.get("tf_min").copied().unwrap_or(5.).round().clamp(1.,60.) as u16;
    let (bars,_)=crate::books::ytsti::candles_for(&minutes,x,tf,crate::intraday::align_of(p));
    let mut indices=Vec::new();let trades=run_capture(id,x,from,to,p,&mut indices).ok_or("unknown ORB strategy")?;
    if indices.len()!=trades.len(){return Err("timestamped source trade count mismatch".into());}
    let entry_indices:Vec<usize>=indices.iter().map(|&(e,_)|e).collect();
    let number=|key:&str,default:f64|p.get(key).copied().unwrap_or(default).round() as usize;
    let scores=ranking_scores(&bars.c,&entry_indices,number("rank_mode",0.),number("roc_period",10.),number("rsi_period",14.),number("std_period",14.),number("swing_period",180.))?;
    let timed=trades.into_iter().zip(indices).map(|(t,(e,z))|(t,e,z)).collect();
    let (mut events,marks)=crate::yt_intraday_portfolio::close_observations(x,from,to,&minutes,&bars,timed,first_id)?;
    for (event,score) in events.iter_mut().zip(scores){event.score=score;}
    Ok((events,marks))
}
pub fn family_of(id:&str)->Option<(&'static str,&'static str,&'static str)> {
    (id==ID).then_some((ID,"Intraday Narrow Range and Opening Range Breakout","Narrow Range ORB"))
}

#[cfg(test)] mod tests {
    use super::*;
    #[test] fn opening_range_starts_after_three_candles_and_long_skips_last_bar() {
        let b=Candles {day:vec![0,1,1,1,1,1,2],h:vec![10.,11.,12.,13.,14.,15.,10.],
            l:vec![9.,8.,8.,8.,8.,8.,9.],c:vec![9.5,10.,10.,10.,14.,15.,9.5],..Candles::default()};
        let s=opening_range_candidates(&b,&[true;7],&[false;7],3).unwrap();
        assert!(!s[1].0 && !s[3].0 && s[4].0 && !s[5].0);
        assert!(opening_range_candidates(&b,&[],&[],0).is_err());
    }
    #[test] fn order_pipeline_rejects_unaligned_data_and_keeps_direction_arrays_aligned() {
        let b=Candles {day:vec![0,1,1,1,1,1,2],hhmm:vec![1525,915,920,925,930,935,915],
            h:vec![10.,11.,12.,13.,14.,15.,10.],l:vec![9.,8.,8.,8.,8.,8.,9.],
            c:vec![9.5,10.,10.,10.,14.,15.,9.5],..Candles::default()};
        let (buy,sell,short,cover)=orders(&b,&[true;7],&[false;7],3,2.).unwrap();
        assert_eq!((buy.len(),sell.len(),short.len(),cover.len()),(7,7,7,7));
        assert!(buy[4]);assert!(!short.iter().any(|&s|s));
        assert!(orders(&b,&[],&[],3,2.).is_err());
    }
    #[test] fn source_ranking_modes_use_entry_candle_history() {
        let c=[100.,101.,102.,103.];
        let pos=ranking_scores(&c,&[3],1,1,2,2,2).unwrap()[0];
        let neg=ranking_scores(&c,&[3],2,1,2,2,2).unwrap()[0];
        assert!((pos-(100.+100./102.)).abs()<1e-9);assert!((neg-(100.-100./102.)).abs()<1e-9);
        assert_eq!(ranking_scores(&c,&[3],4,1,2,2,2).unwrap(),vec![0.5]);
        assert_eq!(ranking_scores(&c,&[3],8,1,2,2,2).unwrap(),vec![100.]);
        assert!(ranking_scores(&c,&[0],1,1,2,2,2).is_err());
    }
}
