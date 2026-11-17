//! Default-path port of the hourly RSI Short EntryExit AFL. Hourly candles are AmiBroker's 60-minute compression
//! (`intraday::compress_aligned`, the `bar_align` setting); the daily filter is expanded with expandLast; the stop is
//! close-based and the target fills at the target price (at the open + buffer on a gap), so no intra-candle order of
//! two price levels is needed.
//!
//! Also coded (16-11-2026), "Old Topics AFL/RSI/Fast Slow RSI" (Suraj Mehra, 31-12-2021), as written with their
//! default Params, one id per AFL (the two differ in more than the filter, so they are not one id with a switch):
//! - `ytrsi_ieod_fast_slow_st` — "Slow and Fast Hourly RSI Crossover based entry and RSI Trail SL EXIT.afl": long
//!   only; buy at the hourly close when the daily ST(5,4) (mid point) is up and the fast RSI(7) crosses above the
//!   slow RSI(20) (`Fast > Slow` now, `Fast < Slow` on the previous bar, both strict); the RSI trail is the highest
//!   fast RSI since entry less 20 points; sell at the close when the fast RSI falls below the trail or below the slow
//!   RSI or the daily ST is down, or at the 5 % target (entry close × 1.05) when the high reaches it — at the open
//!   plus the 0.25 % "Open Buffer Prcnt" when the candle opens above it (as written: the buffer is added).
//! - `ytrsi_ieod_fast_slow` — "..._Wo filter.afl": the same entry without the daily ST and exit on the trail or the
//!   fast RSI below the slow RSI only, at the close; no target (its unused "Open Prcnt" Param is not modelled).
//! Instruments: neither AFL names a symbol or watchlist (no Nifty / Bank Nifty reference, unlike the "RSI - Index"
//! AFLs); they use the team's stock template (dead-stock toggle, commented-out OpenInt turnover table) and need
//! hourly bars, so they target F&O stocks of the IEOD F&O database; index futures in the same database are the
//! same kind of series and are admitted too (any ieodfo symbol, as the other intraday ids).
//! AmiBroker behaviour followed: the RSIs are AmiBroker's RSI (Wilder) in TimeFrameSet(hourly) — run here on
//! 60-minute candles anchored at 09:15 as the base periodicity (the AFL's base periodicity is not stated; its other
//! RSI timeframes, daily / weekly, are not offered); the loop starts at bar RSI_S_Prd (20) and runs over all loaded
//! candles, continuously across days (the header says "Intraday" but the AFL has no square-off, so positions carry
//! overnight); the daily ST is the daily ieodfo carrier's ST (`ytrsi::st_trend`) expanded with expandLast (the
//! day's value from its last candle, earlier candles the previous day's, as the short ids); a trade counts when its
//! entry date is in the selected period (state runs from the data's first candle); a target fill outside the
//! candle's range is clamped into it. Not modelled: equity, position size, brokerage, max positions.
#![allow(dead_code)]
use crate::connors::{Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};
pub const BOOK: &str = "yt_rsi_ieod";
pub const ID: &str = "ytrsi_ieod_short";
pub const DELAYED: &str = "ytrsi_ieod_short_delayed";
pub const FS_ST: &str = "ytrsi_ieod_fast_slow_st";
pub const FS_WO: &str = "ytrsi_ieod_fast_slow";
pub const FS_IDS: [&str;2] = [FS_ST,FS_WO];
pub const IDS: [&str;4] = [ID,DELAYED,FS_ST,FS_WO];
pub fn catalogue() -> Vec<StrategyDef> {
    let mut v=short_catalogue();v.extend(fs_catalogue());v
}
fn fs_catalogue() -> Vec<StrategyDef> {
    let d=|id,name,rules,notes|StrategyDef { id, name, group: "YT Strategies · RSI (Intraday)", session: 0, dir: "Long",
        status: Status::Live, needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK, rules, notes, claim: None };
    vec![
        d(FS_ST,"Hourly Fast RSI(7) / Slow RSI(20) Crossover Long with Daily Supertrend, RSI Trail and 5% Target",&[
            "F|Data: 1-minute F&O futures (team AmiBroker IEOD database) compressed to 60-minute candles (AmiBroker's hourly grid: the day's first candle is 09:15-09:59, stamped 09:00)",
            "F|Daily Supertrend (5,4) on the daily mid point, applied from each day's last candle (earlier candles use the previous day's value)",
            "E|Buy at the hourly close when the daily ST is up and RSI(7) crosses strictly above RSI(20)",
            "X|RSI trail: highest RSI(7) since entry less 20 points; sell at the close when RSI(7) falls below the trail or below RSI(20), or the daily ST turns down",
            "X|Target 5 % above the entry close: sell at the target when the high reaches it, or at the open + 0.25 % when the candle opens above it",
            "X|No time square-off: positions carry overnight, as the AFL is written",
        ][..],"RSI/Fast Slow RSI/Slow and Fast Hourly RSI Crossover based entry and RSI Trail SL EXIT.afl, coded as written with its default settings on 60-minute candles (AmiBroker's hourly grid; setting: aligned to 09:15). The AFL names no symbol; its stock template suggests F&O stocks, and any ieodfo F&O futures series is admitted. The AFL's equity, position-size, brokerage and max-position settings are not modelled. Not compared with the team's trade sheets."),
        d(FS_WO,"Hourly Fast RSI(7) / Slow RSI(20) Crossover Long with RSI Trail (No Filter)",&[
            "F|Data: 1-minute F&O futures (team AmiBroker IEOD database) compressed to 60-minute candles (AmiBroker's hourly grid: the day's first candle is 09:15-09:59, stamped 09:00)",
            "E|Buy at the hourly close when RSI(7) crosses strictly above RSI(20)",
            "X|RSI trail: highest RSI(7) since entry less 20 points; sell at the close when RSI(7) falls below the trail or below RSI(20)",
            "X|No target and no time square-off: positions carry overnight, as the AFL is written",
        ][..],"RSI/Fast Slow RSI/Slow and Fast Hourly RSI Crossover based entry and RSI Trail SL EXIT_Wo filter.afl, coded as written with its default settings on 60-minute candles (AmiBroker's hourly grid; setting: aligned to 09:15). Differs from the filtered AFL by having no daily Supertrend and no 5 % target. The AFL names no symbol; any ieodfo F&O futures series is admitted. Equity, position-size, brokerage and max-position settings are not modelled. Not compared with the team's trade sheets."),
    ]
}
fn short_catalogue() -> Vec<StrategyDef> {
    [(ID,"Hourly RSI Short with Daily Supertrend"),(DELAYED,"Hourly RSI Delayed Short with Daily Supertrend")].into_iter().map(|(id,name)|StrategyDef { id, name, group: "YT Strategies · RSI (Intraday)", session: 0,
        dir: "Short", status: Status::Live, needs_volume: false,
        needs_vix: false, needs_series: &[], book: BOOK,
        rules: &["F|Daily Supertrend (5,4) bearish", "E|Short when hourly RSI crosses strictly above its threshold", "X|Cover when RSI crosses strictly below its threshold or the daily filter turns bullish"],
        notes: "RSI/RSI - Index: corresponding RSI Short EntryExit or RSI Delayed Short EntryExit AFL. RSI period 5 is the AFL's stated default despite its Param minimum 10. ST and swing filters and percentage and ATR stop/target paths follow the AFL, including its percentage-based ATR target expression. Hourly candles on AmiBroker's grid (setting: aligned to 09:15), filter expanded with expandLast. Not compared with the team's trade sheets.", claim: None }).collect()
}
pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    if !IDS.contains(&id) { return None; }
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    if FS_IDS.contains(&id) {
        let mut v=vec![p("fast_period","Fast RSI period",7.,1.,100.,1.),p("slow_period","Slow RSI period",20.,1.,100.,1.),p("rsi_trail","RSI trail points (0 off)",20.,0.,100.,1.)];
        if id==FS_ST {
            v.extend([p("pt_pct","Profit target percent",5.,0.,20.,0.05),p("buffer_pct","Open buffer percent (target gap fill)",0.25,0.,20.,0.001),
                p("filter_tf","Filter timeframe (0 daily, 1 weekly, 2 monthly)",0.,0.,2.,1.),p("st_period","ST filter period",5.,1.,20.,1.),p("st_factor","ST filter factor",4.,0.,20.,1.),
                p("st_point","ST calculation point (1 mid point, 2 close, 3 pivot)",1.,1.,3.,1.)]);
        }
        v.push(crate::intraday::align_param());
        return Some(v);
    }
    Some(vec![p("rsi_period", "RSI period", 5., 1., 26., 1.), p("short_value", "Short RSI threshold", 10., 0., 100., 1.), p("cover_value", "Cover RSI threshold", 10., 0., 100., 1.),
        p("stop_type","Stop type (0 percentage, 1 ATR)",0.,0.,1.,1.),p("hsl_pct","Hard stop percent",0.,0.,6.,1.),p("pt_pct","Profit target percent",0.,0.,20.,0.001),
        p("atr_period","ATR period",1.,1.,100.,1.),p("atr_stop","ATR stop multiplier",0.,0.,10.,0.001),p("atr_target","ATR target parameter",0.,0.,10.,0.001),p("buffer_pct","Target gap buffer percent",0.25,0.,10.,0.05),
        p("filter","Bear filter (0 ST, 1 swing)",0.,0.,1.,1.),p("filter_tf","Filter timeframe (0 daily, 1 weekly, 2 monthly)",0.,0.,2.,1.),p("st_period","ST period",5.,1.,20.,1.),p("st_factor","ST factor",4.,1.,20.,1.),p("st_point","ST calculation point",1.,1.,6.,1.),p("bear_pct","Bear swing percent",23.,10.,50.,1.),p("bull_pct","Bull swing percent",44.,10.,50.,1.),crate::intraday::align_param()])
}

/// The AFL prioritizes the profit-target fill price whenever a cover is triggered,
/// even when the RSI or stop condition also fires on that bar.
fn stop_cover(open:f64,high:f64,low:f64,close:f64,stop:Option<f64>,target:Option<f64>,signal:bool,buffer:f64)->Option<f64> {
    let trigger=signal || stop.is_some_and(|s|close>=s) || target.is_some_and(|t|low<=t);
    if !trigger {return None;}
    let px=if let Some(t)=target.filter(|t|open<=*t) {let _=t;open*(1.+buffer/100.)}
        else if let Some(t)=target.filter(|t|low<=*t) {t} else {close};
    Some(px.clamp(low,high))
}
/// AFL's default branch. Equal RSI values do not count as crossings.
fn signal(rsi: &[Option<f64>], trend: &[i8], entry: f64, cover: f64, delayed:bool) -> Result<Vec<(bool,bool)>, &'static str> {
    if rsi.len() != trend.len() { return Err("RSI and trend lengths differ"); }
    Ok((0..rsi.len()).map(|i| {
        let (enter, exit) = if i == 0 { (false,false) } else { rsi[i-1].zip(rsi[i]).map_or((false,false), |(a,b)| {
            if delayed {(a > entry && b < entry, a < cover && b > cover)} else {(a < entry && b > entry, a > cover && b < cover)}
        }) };
        (trend[i] == -1 && enter, trend[i] == 1 || exit)
    }).collect())
}
pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    run_capture(id,x,from,to,p,&mut Vec::new())
}
fn run_capture(id: &str, x: &Ctx, from: usize, to: usize, p: &Params,capture:&mut Vec<(usize,usize)>) -> Option<Vec<Trade>> {
    if !IDS.contains(&id) { return None; }
    if FS_IDS.contains(&id) { return fs_run(id,x,from,to,p,capture); }
    if x.r.is_empty() || from > to || to >= x.r.len() { return Some(Vec::new()); }
    let ticker = format!("{}{}", crate::intraday::BASKET_PREFIX, x.r[0].ticker);
    let Some(minutes) = crate::intraday::minutes(&ticker) else { return Some(Vec::new()); };
    let (b, di) = crate::books::ytsti::candles_for(&minutes, x, 60,crate::intraday::align_of(p));
    let get = |k: &str, default: f64| p.get(k).copied().filter(|v| v.is_finite()).unwrap_or(default);
    let rsi = crate::books::yteng::rsi(&b.c, get("rsi_period",5.).round().clamp(1.,26.) as usize);
    let tr:Vec<f64>=crate::books::ytsw::true_range(&b.h,&b.l,&b.c).into_iter().map(|v|v.unwrap_or(f64::NAN)).collect();
    let atr=crate::connors::wilder(&tr,get("atr_period",1.).round().clamp(1.,100.) as usize);
    let tf=match get("filter_tf",0.).round() as i64 {1=>'W',2=>'M',_=>'D'};
    let daily=if get("filter",0.)>=0.5 {
        let bars=crate::books::ytsw::compress(x,tf);
        let trend=crate::books::ytrsi::swing_filter(&bars.c,get("bear_pct",23.).clamp(10.,50.),get("bull_pct",44.).clamp(10.,50.));
        crate::books::ytsw::expand_last(&trend,&bars)
    } else {
        crate::books::ytrsi::st_trend(x,tf,get("st_point",1.).round().clamp(1.,6.) as i64,get("st_period",5.).round().clamp(1.,20.) as usize,get("st_factor",4.).clamp(1.,20.))
    };
    let trend: Vec<i8> = (0..b.len()).map(|i| if b.last_of_day(i) { daily[di[i]] } else { di[i].checked_sub(1).map_or(0,|j| daily[j]) }).collect();
    let Ok(signals) = signal(&rsi,&trend,get("short_value",10.),get("cover_value",10.),id==DELAYED) else {return Some(Vec::new());};
    let mut out = Vec::new(); let mut pos: Option<usize> = None;let mut stop=None;let mut target=None;
    for i in 1..b.len() {
        if di[i] > to {break;}
        if let Some(e) = pos {
            if let Some(px)=stop_cover(b.o[i],b.h[i],b.l[i],b.c[i],stop,target,signals[i].1,get("buffer_pct",0.25).clamp(0.,10.)) {
                out.push(Trade {dir:-1., legs:vec![Leg{i:di[e],px:b.c[e],w:1.}], exit_i:di[i],exit_px:px,
                    reason:format!("Hourly RSI short {:04} to {:04}",b.hhmm[e],b.hhmm[i]),entry_kind:"close",signal_i:di[i],open:false,exits:Vec::new()});
                capture.push((e,i));
                pos=None;
                stop=None;target=None;
            }
        } else if di[i]>=from && signals[i].0 && b.c[i]>0. {
            pos=Some(i);
            if get("stop_type",0.)>=0.5 {
                let multiple=get("atr_stop",0.).clamp(0.,10.);
                stop=(multiple>0. && atr[i].is_finite()).then_some(b.c[i]+atr[i]*multiple);
                let t=get("atr_target",0.).clamp(0.,10.);target=(t>0.).then_some(b.c[i]*(1.-t/100.));
            } else {
                let s=get("hsl_pct",0.).clamp(0.,6.);let t=get("pt_pct",0.).clamp(0.,20.);
                stop=(s>0.).then_some(b.c[i]*(1.+s/100.));target=(t>0.).then_some(b.c[i]*(1.-t/100.));
            }
        }
    }
    if let Some(e)=pos {
        if let Some(i)=(e..b.len()).rev().find(|&i|di[i]<=to) {
            out.push(Trade {dir:-1.,legs:vec![Leg{i:di[e],px:b.c[e],w:1.}],exit_i:di[i],exit_px:b.c[i],reason:"Open position at end of selected period".into(),entry_kind:"close",signal_i:di[i],open:true,exits:Vec::new()});
            capture.push((e,i));
        }
    }
    crate::intraday::record(&minutes,&b,60,crate::intraday::align_of(p),&out,capture);
    Some(out)
}
pub(crate) fn portfolio_observations(id:&str,x:&Ctx,from:usize,to:usize,p:&Params,first_id:u64)
    ->Result<(Vec<crate::yt_intraday_portfolio::Signal>,Vec<crate::yt_intraday_portfolio::Mark>),String> {
    let schema=params(id).ok_or("unknown intraday RSI strategy")?;
    crate::intraday::validate_research_params(&schema,p)?;
    let positive=|key:&str|p.get(key).is_some_and(|v|*v>0.);
    if id==FS_ST && p.get("pt_pct").copied().unwrap_or(5.)>0. {
        return Err("needs_data: RSI target event timing within a candle is not established".into());
    }
    if !FS_IDS.contains(&id) && (positive("hsl_pct") || positive("pt_pct") || positive("atr_stop") || positive("atr_target")) {
        return Err("needs_data: RSI stop/target event timing within a candle is not established".into());
    }
    if x.r.is_empty() || from>to || to>=x.r.len(){return Err("invalid selected daily coverage".into());}
    let ticker=format!("{}{}",crate::intraday::BASKET_PREFIX,x.r[0].ticker);
    crate::intraday::available(&ticker)?;
    let minutes=crate::intraday::minutes(&ticker).ok_or("intraday bars are unavailable")?;
    let (bars,_)=crate::books::ytsti::candles_for(&minutes,x,60,crate::intraday::align_of(p));
    let mut indices=Vec::new();let trades=run_capture(id,x,from,to,p,&mut indices).ok_or("unknown RSI strategy")?;
    if indices.len()!=trades.len(){return Err("timestamped source trade count mismatch".into());}
    let timed=trades.into_iter().zip(indices).map(|(t,(e,z))|(t,e,z)).collect();
    crate::yt_intraday_portfolio::close_observations(x,from,to,&minutes,&bars,timed,first_id)
}
/// Fast / slow RSI crossover long with the RSI trail (see the module header). `capture` gets each trade's
/// (entry candle, exit candle).
fn fs_run(id:&str,x:&Ctx,from:usize,to:usize,p:&Params,capture:&mut Vec<(usize,usize)>)->Option<Vec<Trade>> {
    if x.r.is_empty() || from > to || to >= x.r.len() { return Some(Vec::new()); }
    let ticker = format!("{}{}", crate::intraday::BASKET_PREFIX, x.r[0].ticker);
    let Some(minutes) = crate::intraday::minutes(&ticker) else { return Some(Vec::new()); };
    let (b, di) = crate::books::ytsti::candles_for(&minutes, x, 60,crate::intraday::align_of(p));
    let get = |k: &str, default: f64| p.get(k).copied().filter(|v| v.is_finite()).unwrap_or(default);
    let filter=(id==FS_ST).then(||{
        let tf=match get("filter_tf",0.).round() as i64 {1=>'W',2=>'M',_=>'D'};
        let daily=crate::books::ytrsi::st_trend(x,tf,get("st_point",1.).round().clamp(1.,3.) as i64,get("st_period",5.).round().clamp(1.,20.) as usize,get("st_factor",4.).clamp(0.,20.));
        crate::books::ytsti::expand_daily(&daily,&b,&di)
    });
    let target=(id==FS_ST).then(||get("pt_pct",5.).clamp(0.,20.));
    let trades=fs_trades(&b,&di,x,from,to,FsCfg{fast:get("fast_period",7.).round().clamp(1.,100.) as usize,slow:get("slow_period",20.).round().clamp(1.,100.) as usize,
        trail:get("rsi_trail",20.).clamp(0.,100.),target,buffer:get("buffer_pct",0.25).clamp(0.,20.)},filter.as_deref(),capture);
    crate::intraday::record(&minutes,&b,60,crate::intraday::align_of(p),&trades,capture);
    Some(trades)
}
#[derive(Clone,Copy,Debug)]
struct FsCfg {fast:usize,slow:usize,trail:f64,target:Option<f64>,buffer:f64}
fn fs_trades(b:&crate::intraday::Candles,di:&[usize],x:&Ctx,from:usize,to:usize,c:FsCfg,filter:Option<&[i8]>,capture:&mut Vec<(usize,usize)>)->Vec<Trade> {
    let n=b.len();
    let Some(end)=(0..n).rev().find(|&k|di[k]<=to) else {return Vec::new()};
    let fast=crate::books::yteng::rsi(&b.c,c.fast);let slow=crate::books::yteng::rsi(&b.c,c.slow);
    let lt=|a:Option<f64>,b:Option<f64>|a.zip(b).is_some_and(|(a,b)|a<b);
    let when=|e:usize,k:usize|if b.day[k]==b.day[e] {format!("{:02}:{:02}",b.hhmm[k]/100,b.hhmm[k]%100)} else {format!("{} {:02}:{:02}",x.r[di[k]].date,b.hhmm[k]/100,b.hhmm[k]%100)};
    let mut out=Vec::new();
    let mut pos:Option<(usize,Option<f64>)>=None;let (mut high_rsi,mut trail)=(0.,0.);
    let mut push=|out:&mut Vec<Trade>,e:usize,k:usize,px:f64,how:&str,open:bool| {
        if di[e]<from || di[e]>to {return;}
        out.push(Trade{dir:1.,legs:vec![Leg{i:di[e],px:b.c[e],w:1.}],exit_i:di[k],exit_px:px,
            reason:format!("Long {} (fast RSI crossed above slow RSI) → {how} {}; 60-minute candles",when(e,e),when(e,k)),entry_kind:"close",signal_i:di[k],open,exits:Vec::new()});
        capture.push((e,k));
    };
    for i in c.slow.max(1)..=end {
        let up=filter.is_none_or(|f|f[i]==1);
        if pos.is_none() {
            if up && lt(slow[i],fast[i]) && lt(fast[i-1],slow[i-1]) {
                high_rsi=fast[i].unwrap_or(0.);
                if c.trail>0. {trail=high_rsi-c.trail;}
                pos=Some((i,c.target.map(|t|(1.+t*0.01)*b.c[i])));
            }
        } else if let Some((e,pt))=pos {
            if c.trail>0. {
                if let Some(f)=fast[i] {high_rsi=f64::max(f,high_rsi);}
                trail=high_rsi-c.trail;
            }
            let hit=pt.is_some_and(|t|b.h[i]>=t);
            let below_trail=fast[i].is_some_and(|f|f<trail);
            let bear=filter.is_some_and(|f|f[i]==-1);
            if hit || below_trail || lt(fast[i],slow[i]) || bear {
                let (px,how)=match pt.filter(|_|hit) {
                    Some(t)=>((if b.o[i]>t {b.o[i]+b.o[i]*c.buffer/100.} else {t}).clamp(b.l[i],b.h[i]),"target"),
                    None=>(b.c[i],if below_trail {"RSI trail"} else if lt(fast[i],slow[i]) {"fast RSI below slow RSI"} else {"daily ST down"}),
                };
                push(&mut out,e,i,px,how,false);
                pos=None;trail=0.;high_rsi=0.;
            }
        }
    }
    if let Some((e,_))=pos {push(&mut out,e,end,b.c[end],"open",true);}
    out
}
pub fn family_of(id:&str)->Option<(&'static str,&'static str,&'static str)> {
    let d=catalogue().into_iter().find(|d|d.id==id)?;Some((d.id,d.name,"RSI (Intraday)"))
}
#[cfg(test)] mod tests {
    use super::*;
    #[test] fn strict_rsi_cross_and_filter() {
        let s=signal(&[Some(9.),Some(10.),Some(11.),Some(9.)],&[-1,-1,-1,-1],10.,10.,false).unwrap();
        assert!(!s[1].0 && !s[2].0 && s[3].1);
        assert!(signal(&[Some(9.),Some(11.)],&[-1,-1],10.,10.,false).unwrap()[1].0);
        assert!(signal(&[Some(11.),Some(9.)],&[-1,-1],10.,10.,true).unwrap()[1].0);
        assert!(signal(&[None],&[],10.,10.,false).is_err());
    }
    /// Hourly candles (7 a day from 09:15) with open = previous close, high / low = 0.5 beyond the open and close.
    fn fs_candles(c:&[f64])->(crate::intraday::Candles,Vec<usize>,Vec<crate::Row>) {
        let n=c.len();let days=n.div_ceil(7);let o:Vec<f64>=(0..n).map(|k|if k==0 {c[0]} else {c[k-1]}).collect();
        let rows:Vec<crate::Row>=(0..days).map(|d|crate::Row{ticker:"TEST".into(),date:format!("2022-03-{:02}",d+1),open:100.,high:101.,low:99.,close:100.,volume:None}).collect();
        let b=crate::intraday::Candles{day:(0..n).map(|k|(k/7) as u32).collect(),hhmm:(0..n).map(|k|915+100*(k%7) as u16).collect(),
            o:o.clone(),h:(0..n).map(|k|o[k].max(c[k])+0.5).collect(),l:(0..n).map(|k|o[k].min(c[k])-0.5).collect(),c:c.to_vec(),
            v:vec![1.;n],oi:vec![None;n],start:(0..n).collect(),end:(0..n).collect()};
        let di=(0..n).map(|k|k/7).collect();
        (b,di,rows)
    }
    const FS_PATH:[f64;16]=[100.,101.,100.,99.,98.,97.,96.,95.,97.,100.,103.,104.,103.5,101.,99.,97.];
    #[test] fn fast_slow_cross_entry_and_rsi_trail_exit() {
        // RSI(2) crosses above RSI(4) on candle 8 (67.2 > 46.3 after 1.6 < 10.5): buy at 97 (day 2, 10:15). The trail
        // (highest RSI(2) 96.6 less 20 = 76.6) is broken on candle 12 (75.7): sell at 103.5 (day 2, 14:15)
        let (b,di,rows)=fs_candles(&FS_PATH);let x=Ctx::new(&rows,&rows,&[]);
        let c=FsCfg{fast:2,slow:4,trail:20.,target:None,buffer:0.25};
        let mut cap=Vec::new();
        let t=fs_trades(&b,&di,&x,0,2,c,None,&mut cap);
        assert_eq!(t.len(),1,"{t:?}");
        assert_eq!((t[0].dir,t[0].legs[0].px,t[0].exit_px,t[0].legs[0].i,t[0].exit_i),(1.,97.,103.5,1,1));
        assert!(t[0].reason.starts_with("Long 10:15 (fast RSI crossed above slow RSI) → RSI trail 14:15"),"{}",t[0].reason);
        assert_eq!(cap,vec![(8,12)]);
        crate::trade_validation::check(&t,0,2,0.).unwrap();
        // the entry date must be inside the period
        assert!(fs_trades(&b,&di,&x,2,2,c,None,&mut Vec::new()).is_empty());
    }
    #[test] fn fast_slow_filter_and_target() {
        let (b,di,rows)=fs_candles(&FS_PATH);let x=Ctx::new(&rows,&rows,&[]);
        let c=FsCfg{fast:2,slow:4,trail:20.,target:Some(5.),buffer:0.25};
        // 5 % target 101.85: candle 10 opens at 100 and reaches 103.5, so it sells at the target
        let up=vec![1i8;FS_PATH.len()];
        let t=fs_trades(&b,&di,&x,0,2,c,Some(&up),&mut Vec::new());
        assert_eq!(t.len(),1);assert!((t[0].exit_px-101.85).abs()<1e-9,"{t:?}");assert!(t[0].reason.contains("→ target 12:15"),"{}",t[0].reason);
        // a daily ST down before the target sells at the close; no entry while the daily ST is down
        let mut f=up.clone();f[9]=-1;
        let t=fs_trades(&b,&di,&x,0,2,c,Some(&f),&mut Vec::new());
        assert_eq!(t[0].exit_px,100.);assert!(t[0].reason.contains("daily ST down"),"{}",t[0].reason);
        f[8]=-1;
        assert!(fs_trades(&b,&di,&x,0,2,c,Some(&f),&mut Vec::new()).is_empty());
    }
    #[test] fn fast_slow_registration_and_defaults() {
        let d=|id:&str,k:&str|params(id).unwrap().iter().find(|p|p.key==k).map(|p|p.def);
        for id in FS_IDS {assert_eq!((d(id,"fast_period"),d(id,"slow_period"),d(id,"rsi_trail")),(Some(7.),Some(20.),Some(20.)));}
        assert_eq!((d(FS_ST,"pt_pct"),d(FS_ST,"buffer_pct"),d(FS_ST,"st_factor"),d(FS_WO,"pt_pct")),(Some(5.),Some(0.25),Some(4.),None));
        assert_eq!(d(ID,"rsi_period"),Some(5.));
        let rows=vec![crate::Row{ticker:"X".into(),date:"2022-01-03".into(),open:100.,high:101.,low:99.,close:100.,volume:None}];
        let x=Ctx::new(&rows,&rows,&[]);
        for id in FS_IDS {
            let def=catalogue().into_iter().find(|d|d.id==id).unwrap();
            assert!(crate::instrument_policy::intraday_rule(&def));
            assert!(run(id,&x,0,0,&Params::new()).unwrap().is_empty());
            assert_eq!(family_of(id).unwrap().2,"RSI (Intraday)");
        }
        assert!(portfolio_observations(FS_ST,&x,0,0,&Params::new(),0).unwrap_err().contains("timing"));
    }
    /// Opt-in smoke run on the local data: TEST_INTRADAY_DAILY (BANKNIFTY daily ieodfo CSV) and INTRADAY_DATA_DIR.
    #[test] fn local_fast_slow_smoke_when_configured() {
        let Ok(csv)=std::env::var("TEST_INTRADAY_DAILY") else {return};
        if std::env::var_os("INTRADAY_DATA_DIR").is_none(){return;}
        let text=std::fs::read_to_string(csv).unwrap();
        let rows:Vec<crate::Row>=text.lines().skip(1).map(|line|{let v:Vec<_>=line.split(',').collect();crate::Row{
            ticker:"BANKNIFTY".into(),date:v[0].into(),open:v[1].parse().unwrap(),high:v[2].parse().unwrap(),low:v[3].parse().unwrap(),close:v[4].parse().unwrap(),volume:v[5].parse().ok()}}).collect();
        let from=rows.iter().position(|r|r.date.as_str()>="2019-01-01").unwrap();
        let to=rows.iter().rposition(|r|r.date.as_str()<="2019-12-31").unwrap();
        let x=Ctx::new(&rows,&rows,&[]);
        for id in FS_IDS {
            let t=run(id,&x,from,to,&Params::new()).unwrap();
            crate::trade_validation::check_structure(&t,from,to).unwrap();
            eprintln!("SMOKE {id} BANKNIFTY 2019: {} trades",t.len());
        }
        assert!(crate::yt_intraday_portfolio::run_strategy_research(FS_WO,&[(&x,from,to)],&Params::new(),1000.,1,0.).is_ok());
    }
    #[test] fn target_gap_and_stop_trigger_follow_source_fill_priority() {
        assert!((stop_cover(94.,103.,93.,101.,Some(100.),Some(95.),false,0.25).unwrap()-94.235).abs()<1e-10);
        assert_eq!(stop_cover(100.,103.,99.,101.,Some(100.),None,false,0.25),Some(101.));
        assert_eq!(stop_cover(100.,102.,99.,100.,None,None,false,0.25),None);
    }
}
