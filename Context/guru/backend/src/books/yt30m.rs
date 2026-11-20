//! YT Strategies · 30 Min Breakout: the team's AmiBroker AFLs in "Old Topics/30 Min Breakout"
//! (30 MIN Breakout ANALYSIS/FINAL_06.12.2021: INTRADAY, POSITIONAL and TEMP, 23 AFLs). The normal intraday
//! variants NORMAL, VERSION1 and VERSION2 have intraday and positional implementations including bull/bear and TEMP
//! variants. Candles: `intraday::compress_aligned` (AmiBroker's grid; the `bar_align` setting); the daily filter is
//! expanded with expandLast (`intraday` module docs); intraday trade timings are recorded (`intraday::record`).
//!
//! The rules read the day's first (and, in the VER 1 / VER 2 "WITH LAST BAR" AFLs, also the previous day's last)
//! 30-minute candle: `STARTBAR = Day() != Ref(Day(), -1)`, `ENDBAR = Day() != Ref(Day(), 1)`,
//! `BARHIGH = ValueWhen(STARTBAR, H)` (VER 1: Max of first / last bar highs and Min of their lows; VER 2: Max of
//! both), buy / cover when a 30-minute close breaks above it and sell / short below the low, filtered by a daily
//! SuperTrend(5, 4) computed with `TimeFrameSet(86400)`; the INTRADAY AFLs also square off on the day's last bar.
//! On daily bars every bar is both STARTBAR and ENDBAR, so the breakout level is the bar's own high / low (the
//! normal AFLs could never trade: C > H is never true) or the previous day's (VER 1 / VER 2 positional): a
//! different strategy from the one written, so nothing is run on daily data.
//!
//! TEMP variants use unlagged filter entries; the plain drafts do not use filter exits.
#![allow(dead_code)]
use crate::connors::{Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};
use crate::intraday::Candles;

/// Raw signals from the NORMAL intraday AFL, before ExRem and portfolio execution.
/// `filter` contains the already-expanded daily filter state for each candle:
/// +1 bullish, -1 bearish, 0 unavailable. The caller must establish the source
/// timeframe and expansion semantics; this function does not infer them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct NormalSignals { buy: bool, sell: bool, short: bool, cover: bool }

fn normal_intraday_signals(b: &Candles, day_of_month: &[u8], filter: &[i8]) -> Result<Vec<NormalSignals>, &'static str> {
    if filter.len() != b.len() || day_of_month.len() != b.len() || b.h.len() != b.len() || b.l.len() != b.len() {
        return Err("30-minute candles and expanded filter must have equal lengths");
    }
    let mut level: Option<(f64, f64)> = None;
    let mut result = Vec::with_capacity(b.len());
    for i in 0..b.len() {
        // AFL STARTBAR is Day() != Ref(Day(), -1), with no BarIndex()==0 override.
        if i > 0 && day_of_month[i] != day_of_month[i - 1] { level = Some((b.h[i], b.l[i])); }
        let end = i + 1 < b.len() && day_of_month[i] != day_of_month[i + 1];
        let last = i + 1 == b.len();
        let (above, below) = level.map_or((false, false), |(high, low)| (b.c[i] > high, b.c[i] < low));
        let bull = filter[i] == 1 || filter[i] == 2;
        let bear = filter[i] == -1 || filter[i] == 2;
        result.push(NormalSignals {
            buy: above && bull && !end && !last,
            sell: below || bear || end || last,
            short: below && bear && !end && !last,
            cover: above || end || last,
        });
    }
    Ok(result)
}

/// `as_written`: the AFL files' rules (filter and Version 1 / 2 levels one bar back, `Ref(.., -1)`; filter flip also
/// exits). Otherwise, for the positional Normal / Version 1 / Version 2 ids, the rules the team's FINAL_06.12.2021 trade
/// sheets show (current bar's filter and levels; exits on the breakout level only): all 18 sheets match (20-11-2026).
fn breakout_signals(b:&Candles,day:&[u8],filter:&[i8],version:u8,positional:bool,regime:i8,temp:bool,as_written:bool)->Result<Vec<NormalSignals>,&'static str> {
    let team=positional && !temp && !as_written;
    if version==0 && !positional && regime==0 && !temp {return normal_intraday_signals(b,day,filter);}
    let n=b.len();
    if [day.len(),filter.len(),b.h.len(),b.l.len()].iter().any(|&v|v!=n) || version>2 {return Err("unaligned breakout inputs");}
    let mut start=None::<(f64,f64)>;let mut last=None::<(f64,f64)>;let mut prev_level=None::<(f64,f64)>;
    let mut out=Vec::with_capacity(n);
    for i in 0..n {
        if (i==0 && version>0) || (i>0 && day[i]!=day[i-1]) {start=Some((b.h[i],b.l[i]));}
        let end=i+1<n && day[i]!=day[i+1];
        if end {last=Some((b.h[i],b.l[i]));}
        let level=if version==0 {start} else {start.zip(last).map(|(a,z)|(a.0.max(z.0),if version==1 {a.1.min(z.1)} else {a.1.max(z.1)}))};
        let compare=if positional && version>0 && !temp && !team {prev_level} else {level};
        let (above,below)=compare.map_or((false,false),|(h,l)|(b.c[i]>h,b.c[i]<l));
        let current_bull=filter[i]==1 || filter[i]==2;let current_bear=filter[i]==-1 || filter[i]==2;
        let previous=i.checked_sub(1).map_or(0,|j|filter[j]);
        let buy_filter=if positional && !temp && !team {if regime == -1 {current_bear} else {previous==1 || previous==2}} else if regime == -1 {current_bear} else {current_bull};
        let short_filter=if positional && !temp && !team {if regime == 1 {current_bull} else {previous == -1 || previous==2}} else if regime == 1 {current_bull} else {current_bear};
        let regime_exit=if regime == -1 {current_bull} else {current_bear};
        let cover_filter=if temp && regime==0 {false} else if regime == 0 {positional&&current_bull} else if regime==1 && !positional && version==1 {false} else {regime_exit};
        let last_bar=i+1==n;
        out.push(NormalSignals {buy:above && buy_filter && (positional || (!end&&!last_bar)),
            sell:below||(!(temp && regime==0)&&regime_exit&&!team)||(!positional&&(end||last_bar)),
            short:below&&short_filter && (positional||(!end&&!last_bar)),
            cover:above || (cover_filter&&!team) || (!positional&&(end||last_bar))});
        prev_level=level;
    }
    Ok(out)
}

pub const BOOK: &str = "yt_30m";
pub const NORMAL_INTRADAY: &str = "yt30m_normal_intraday";
pub const V1_INTRADAY: &str = "yt30m_v1_intraday";
pub const V2_INTRADAY: &str = "yt30m_v2_intraday";
pub const NORMAL_POSITIONAL: &str = "yt30m_normal_positional";
pub const V1_POSITIONAL: &str = "yt30m_v1_positional";
pub const V2_POSITIONAL: &str = "yt30m_v2_positional";
pub const IDS: [&str; 23] = [NORMAL_INTRADAY,V1_INTRADAY,V2_INTRADAY,NORMAL_POSITIONAL,V1_POSITIONAL,V2_POSITIONAL,
    "yt30m_normal_intraday_bull","yt30m_v1_intraday_bull","yt30m_v2_intraday_bull","yt30m_normal_positional_bull","yt30m_v1_positional_bull","yt30m_v2_positional_bull",
    "yt30m_normal_intraday_bear","yt30m_v1_intraday_bear","yt30m_v2_intraday_bear","yt30m_normal_positional_bear","yt30m_v1_positional_bear","yt30m_v2_positional_bear",
    "yt30m_temp_normal","yt30m_temp_v1","yt30m_temp_v2","yt30m_temp_bull","yt30m_temp_bear"];

pub fn catalogue() -> Vec<StrategyDef> {
    let names=["30 Minute Breakout Intraday","30 Minute Breakout Intraday Version 1","30 Minute Breakout Intraday Version 2","30 Minute Breakout Positional","30 Minute Breakout Positional Version 1","30 Minute Breakout Positional Version 2",
        "30 Minute Breakout Intraday Bull","30 Minute Breakout Intraday Version 1 Bull","30 Minute Breakout Intraday Version 2 Bull","30 Minute Breakout Positional Bull","30 Minute Breakout Positional Version 1 Bull","30 Minute Breakout Positional Version 2 Bull",
        "30 Minute Breakout Intraday Bear","30 Minute Breakout Intraday Version 1 Bear","30 Minute Breakout Intraday Version 2 Bear","30 Minute Breakout Positional Bear","30 Minute Breakout Positional Version 1 Bear","30 Minute Breakout Positional Version 2 Bear",
        "30 Minute Breakout TEMP Normal","30 Minute Breakout TEMP Version 1","30 Minute Breakout TEMP Version 2","30 Minute Breakout TEMP Bull","30 Minute Breakout TEMP Bear"];
    IDS.into_iter().zip(names).map(|(id,name)|
    StrategyDef { id, name, group: "YT Strategies · 30 Min Breakout", session: 0, dir: "Long/Short", status: Status::Live, needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK,
        rules: &["F|Daily Supertrend (5,4) filter, expanded to 30-minute bars", "E|Enter on a close beyond the source opening-range level", "X|Exit on the source opposite breakout or filter conditions"],
        notes: "FINAL_06.12.2021 NORMAL, VERSION1, VERSION2 or TEMP AFL with the corresponding intraday or positional setting and optional bull/bear regime. ST, SMA and filter-off modes are available; daily, weekly and monthly expansion are provisional. Version 1 uses the minimum of first/last lows; version 2 uses their maximum. The filter uses the selected symbol. Runs on 30-minute candles of the selected ieodfo F&O futures on AmiBroker's time-compression grid (setting: aligned to 09:15), filter expanded with TimeFrameExpand's expandLast, fills at the candle close; ENDBAR is a future reference kept as written. Compared 20-11-2026 with the team's FINAL_06.12.2021 positional trade sheets (Normal, Version 1, Version 2; NIFTY_F1 and BANKNIFTY_F1; daily ST 5,4 / 5,5 and weekly ST 5,1; 18 sheets): with the candle alignment at 09:15, every team entry matches (12,562 of 12,562) and 99.4% of exits. Declared adaptation: the AFL files in the folder read the filter and (Version 1 / 2) the breakout levels one bar back (`Ref(FLTBuy, -1)`, `Ref(BARHIGH, -1)`); the team's sheets match the current bar's filter and levels, so the team's run used a version without those Refs, and by default (setting Rules = 0) the positional ids follow the sheets; Rules = 1 runs the AFL file as written. The sheets also show that a filter flip does not close a positional trade (only the breakout level does), so positional exits are on the breakout level only. With raw minute highs / lows (as AmiBroker) all 18 sheets match completely: 12,562 of 12,562 trades, entries and exits.", claim: None }).collect()
}
pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    if !IDS.contains(&id) {return None;}
    let p=|key,label,def,min,max,step|ParamDef{key,label,def,min,max,step};
    Some(vec![p("filter","Filter (0 ST, 1 SMA, 2 off)",0.,0.,2.,1.),p("filter_tf","Filter timeframe (0 daily, 1 weekly, 2 monthly)",0.,0.,2.,1.),
        p("st_period","ST period",5.,1.,20.,1.),p("st_factor","ST factor",4.,0.5,20.,0.5),p("st_point","ST point (1 midpoint, 2 close, 3 pivot)",1.,1.,3.,1.),p("sma_period","SMA period",200.,1.,300.,1.),
        p("afl_rules","Rules (positional: 0 as the team's trade sheets, 1 as the AFL file is written)",0.,0.,1.,1.),crate::intraday::align_param()])
}
pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    run_capture(id,x,from,to,p,&mut Vec::new())
}
fn run_capture(id: &str, x: &Ctx, from: usize, to: usize, p: &Params, capture:&mut Vec<(usize,usize)>) -> Option<Vec<Trade>> {
    if !IDS.contains(&id) { return None; }
    if x.r.is_empty() || from > to || to >= x.r.len() { return Some(Vec::new()); }
    let ticker = format!("{}{}", crate::intraday::BASKET_PREFIX, x.r[0].ticker);
    let Some(minutes) = crate::intraday::minutes(&ticker) else { return Some(Vec::new()); };
    let (bars, di) = crate::books::ytsti::candles_for(&minutes, x, 30,crate::intraday::align_of(p));
    let get=|k:&str,d:f64|p.get(k).copied().filter(|v|v.is_finite()).unwrap_or(d);
    let tf=match get("filter_tf",0.).round() as i64 {1=>'W',2=>'M',_=>'D'};
    let trend=match get("filter",0.).round() as i64 {
        1=>{let (bull,bear)=crate::books::ytsw::sma_filter(x,tf,get("sma_period",200.).round().clamp(1.,300.) as usize,true);
            bull.into_iter().zip(bear).map(|(b,s)|if b {1}else if s {-1}else{0}).collect()},
        2=>vec![2;x.r.len()],
        _=>crate::books::ytsw::st_filter(x,tf,get("st_point",1.).round() as i64,get("st_period",5.).round().clamp(1.,20.) as usize,get("st_factor",4.).clamp(0.5,20.)),
    };
    let filter: Vec<i8> = (0..bars.len()).map(|i| {
        let day = di[i];
        if get("filter",0.).round() as i64==2 {2} else if bars.last_of_day(i) { trend[day] } else { day.checked_sub(1).map_or(0, |d| trend[d]) }
    }).collect();
    let days: Vec<u8> = bars.day.iter().map(|&d| minutes.dates[d as usize].get(8..10).and_then(|v| v.parse().ok()).unwrap_or(0)).collect();
    if days.contains(&0) { return Some(Vec::new()); }
    let index=IDS.iter().position(|&v|v==id).unwrap();
    let temp=index>=18;
    let version=if temp {match index {19=>1,20=>2,_=>0}}else{(index%3) as u8};
    let positional=temp || index%6>=3;
    let regime=if temp {match index {21=>1,22=>-1,_=>0}}else{match index/6 {1=>1,2=>-1,_=>0}};
    let as_written=get("afl_rules",0.).round() as i64==1;
    let Ok(raw) = breakout_signals(&bars, &days, &filter, version, positional, regime, temp, as_written) else { return Some(Vec::new()); };
    // ExRem is applied in the order of the AFL's assignments.
    fn exrem(a: &[bool], b: &[bool]) -> Vec<bool> {
        let mut armed = true;
        a.iter().zip(b).map(|(&set, &reset)| {
            let signal = set && armed;
            if signal { armed = false; }
            if reset { armed = true; }
            signal
        }).collect()
    }
    let buys: Vec<bool> = raw.iter().map(|r| r.buy).collect();
    let sells: Vec<bool> = raw.iter().map(|r| r.sell).collect();
    let shorts: Vec<bool> = raw.iter().map(|r| r.short).collect();
    let covers: Vec<bool> = raw.iter().map(|r| r.cover).collect();
    let buy = exrem(&buys, &sells); let sell = exrem(&sells, &buy);
    let short = exrem(&shorts, &covers); let cover = exrem(&covers, &short);
    let mut trades = Vec::new(); let mut pos: Option<(usize, f64)> = None;
    for i in 0..bars.len() {
        if di[i] > to { break; }
        if let Some((entry, dir)) = pos {
            if (dir > 0. && sell[i]) || (dir < 0. && cover[i]) {
                trades.push(Trade { dir, legs: vec![Leg { i: di[entry], px: bars.c[entry], w: 1. }], exit_i: di[i], exit_px: bars.c[i],
                    reason: format!("30-minute breakout {:04} to {:04}", bars.hhmm[entry], bars.hhmm[i]), entry_kind: "close", signal_i: di[i], open: false, exits: Vec::new() });
                capture.push((entry,i));
                pos = None;
            }
        }
        if pos.is_none() && di[i] >= from {
            if buy[i] { pos = Some((i, 1.)); } else if short[i] { pos = Some((i, -1.)); }
        }
    }
    if let Some((entry, dir)) = pos {
        if let Some(i) = (entry..bars.len()).rev().find(|&i| di[i] <= to) {
            trades.push(Trade { dir, legs: vec![Leg { i: di[entry], px: bars.c[entry], w: 1. }], exit_i: di[i], exit_px: bars.c[i], reason: "Open position at end of selected period".into(), entry_kind: "close", signal_i: di[i], open: true, exits: Vec::new() });
            capture.push((entry,i));
        }
    }
    crate::intraday::record(&minutes,&bars,30,crate::intraday::align_of(p),&trades,capture);
    Some(trades)
}
/// Research-only typed close events. The source AFL has no PositionScore and
/// its default position size is one share; fixed-notional ledger sizing is an adaptation.
pub(crate) fn portfolio_observations(id:&str,x:&Ctx,from:usize,to:usize,p:&Params,first_id:u64)
    ->Result<(Vec<crate::yt_intraday_portfolio::Signal>,Vec<crate::yt_intraday_portfolio::Mark>),String> {
    let schema=params(id).ok_or("unknown 30-minute breakout strategy")?;
    crate::intraday::validate_research_params(&schema,p)?;
    if x.r.is_empty() || from>to || to>=x.r.len(){return Err("invalid selected daily coverage".into());}
    let ticker=format!("{}{}",crate::intraday::BASKET_PREFIX,x.r[0].ticker);
    crate::intraday::available(&ticker)?;
    let minutes=crate::intraday::minutes(&ticker).ok_or("intraday bars are unavailable")?;
    let (bars,_)=crate::books::ytsti::candles_for(&minutes,x,30,crate::intraday::align_of(p));
    let mut indices=Vec::new();let trades=run_capture(id,x,from,to,p,&mut indices).ok_or("unknown breakout strategy")?;
    if trades.len()!=indices.len(){return Err("timestamped source trade count mismatch".into());}
    let timed=trades.into_iter().zip(indices).map(|(t,(e,z))|(t,e,z)).collect();
    crate::yt_intraday_portfolio::close_observations(x,from,to,&minutes,&bars,timed,first_id)
}
pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    let d=catalogue().into_iter().find(|d|d.id==id)?;Some((d.id,d.name,"30 Min Breakout"))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn local_breakout_portfolio_bridge_uses_close_observations_when_configured() {
        let Ok(csv)=std::env::var("TEST_INTRADAY_DAILY") else {return};
        if std::env::var_os("INTRADAY_DATA_DIR").is_none(){return;}
        let text=std::fs::read_to_string(csv).unwrap();
        let rows:Vec<crate::Row>=text.lines().skip(1).map(|line| {let v:Vec<_>=line.split(',').collect();crate::Row{
            ticker:"BANKNIFTY".into(),date:v[0].into(),open:v[1].parse().unwrap(),high:v[2].parse().unwrap(),low:v[3].parse().unwrap(),close:v[4].parse().unwrap(),volume:v[5].parse().ok()}}).collect();
        let from=rows.iter().position(|r|r.date.as_str()>="2019-01-01").unwrap();
        let to=rows.iter().rposition(|r|r.date.as_str()<="2019-01-31").unwrap();
        let x=Ctx::new(&rows,&rows,&[]);
        let (events,marks)=portfolio_observations(NORMAL_INTRADAY,&x,from,to,&Params::new(),0).unwrap();
        assert!(!marks.is_empty());
        for e in &events {assert_eq!(e.score,0.);assert!(marks.iter().any(|m|m.time==e.entry_time && (m.price-e.entry_price).abs()<0.00011));}
        let ledger=crate::yt_intraday_portfolio::run_strategy_research(NORMAL_INTRADAY,&[(&x,from,to)],&Params::new(),1000.,1,0.).unwrap();
        assert!(ledger.accepted.len()<=events.len());
    }

    #[test]
    fn all_breakout_variants_are_registered_and_live() {
        assert_eq!(catalogue().len(), 23);
        assert!(catalogue().iter().all(|d| matches!(d.status, Status::Live)));
        assert!(params("yt30m_breakout").is_none() && family_of("yt30m_breakout").is_none());
    }

    #[test]
    fn normal_intraday_breakout_uses_first_bar_and_squares_off_at_end() {
        let bars = Candles {
            day: vec![0, 0, 1, 1, 1, 1, 2],
            h: vec![12., 12., 12., 13., 13., 12., 12.],
            l: vec![8., 8., 8., 9., 7., 8., 8.],
            c: vec![10., 10., 10., 13., 7., 10., 10.],
            ..Candles::default()
        };
        let s = normal_intraday_signals(&bars, &[1, 1, 2, 2, 2, 2, 3], &[1, 1, 1, 1, -1, -1, 1]).unwrap();
        assert!(!s[0].buy && !s[1].buy); // no fabricated first-session opening range
        assert!(s[3].buy && s[3].cover);
        assert!(s[4].short && s[4].sell);
        assert!(s[5].sell && s[5].cover && !s[5].buy && !s[5].short);
        assert!(normal_intraday_signals(&bars, &[], &[]).is_err());
    }
    #[test]
    fn positional_filter_flip_exits_only_as_written() {
        // two days of 30-minute bars; the close stays inside the first candle's range, the filter turns bear on day 2
        let bars = Candles { o: vec![100.;4], h: vec![102.,101.,102.,101.], l: vec![98.,99.,98.,99.], c: vec![100.;4], ..Candles::default() };
        let day=[1u8,1,2,2]; let filter=[1i8,1,-1,-1];
        let team=breakout_signals(&bars,&day,&filter,0,true,0,false,false).unwrap();
        let written=breakout_signals(&bars,&day,&filter,0,true,0,false,true).unwrap();
        assert!(!team[2].sell && !team[3].sell, "team sheets: a filter flip does not exit");
        assert!(written[2].sell, "AFL as written: Sell = C < BARLOW OR fltshort");
    }

    #[test]
    fn previous_last_bar_changes_version_two_low() {
        let bars=Candles {day:vec![0,0,1,1,1,2],h:vec![12.,12.,12.,12.,12.,12.],
            l:vec![7.,9.,7.,7.,7.,7.],c:vec![10.,10.,10.,8.,8.,10.],..Candles::default()};
        let day=[1,1,2,2,2,3];let filter=[-1;6];
        let v1=breakout_signals(&bars,&day,&filter,1,false,0,false,true).unwrap();
        let v2=breakout_signals(&bars,&day,&filter,2,false,0,false,true).unwrap();
        assert!(!v1[3].short && v2[3].short);
    }
    #[test]
    fn bull_and_bear_intraday_use_their_own_regime_for_both_sides() {
        let bars=Candles {day:vec![0,1,1,1,1,2],h:vec![12.,12.,12.,12.,12.,12.],
            l:vec![8.,8.,8.,8.,8.,8.],c:vec![10.,10.,7.,13.,10.,10.],..Candles::default()};
        let day=[1,2,2,2,2,3];
        let bull=breakout_signals(&bars,&day,&[1;6],0,false,1,false,true).unwrap();
        let bear=breakout_signals(&bars,&day,&[-1;6],0,false,-1,false,true).unwrap();
        assert!(bull[2].short && bull[3].buy);
        assert!(bear[2].short && bear[3].buy);
        assert!(!breakout_signals(&bars,&day,&[-1;6],0,false,1,false,true).unwrap()[2].short);
    }
    #[test]
    fn temp_plain_has_no_filter_exit() {
        let bars=Candles {day:vec![0,1,1,1],h:vec![12.;4],l:vec![8.;4],c:vec![10.;4],..Candles::default()};
        let s=breakout_signals(&bars,&[1,2,2,2],&[-1;4],0,true,0,true,true).unwrap();
        assert!(!s[2].sell);
    }
}
