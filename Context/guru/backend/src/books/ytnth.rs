//! Port of "intraday nth cadle buy sell afl.afl", as written. The AFL does not declare its analysis timeframe (the
//! `tf_min` setting is the Analysis periodicity), uses a future daily reference (`Ref(today, 1)` expanded with
//! expandLast; see `intraday::look_ahead_notice`). With the default entry and exit candle both 1, Buy and Sell are
//! both true on the day's first candle, but `Sell = ExRem(Sell, Buy)` drops that Sell (Sell was already used up
//! before this Buy and is re-armed only by it), so the first Sell after the entry is the next candle: the trade exits
//! at the next candle's close (on 1-minute candles: the 09:15 close to the 09:16 close).
#![allow(dead_code)]
use crate::connors::{Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "yt_nth";
pub const ID: &str = "ytnth_intraday";

pub fn catalogue() -> Vec<StrategyDef> {
    vec![StrategyDef { id: ID, name: "Intraday Nth Candle", group: "YT Strategies · Intraday Nth Candle", session: 0,
        dir: "Long/Short", status: Status::Live,
        needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK,
        rules: &["F|Trading-day counter is derived from the AFL's daily calculation and expanded future reference",
            "E|Enter at the selected intraday candle close", "X|Exit at the selected candle close or the last bar of the day"],
        notes: "Intraday Buy Sell on nth Candle/intraday nth cadle buy sell afl.afl. The AFL states no candle interval (the candle minutes setting is the Analysis periodicity, default 1 minute) and uses Ref(today,1), a future daily value, kept as written. Default entry and exit candle are both 1: Buy and Sell fall on the same candle, ExRem(Sell, Buy) removes that Sell, and the trade exits at the next candle's close. Not compared with the team's trade sheets.", claim: None }]
}
pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    if id != ID { return None; }
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    Some(vec![p("tf_min", "Candle minutes (the AFL states no interval; the Analysis periodicity)", 1., 1., 60., 1.), p("entry_day", "Entry trading day", 1., 1., 19., 1.),
        p("entry_bar", "Entry candle", 1., 1., 25., 1.), p("exit_bar", "Exit candle", 1., 1., 400., 1.),
        p("side", "Side (0 long, 1 short)", 0., 0., 1., 1.),p("turnover","OpenInt turnover filter (1 on)",0.,0.,1.,1.),
        p("deadstock","Dead-stock backtest (1 on)",0.,0.,1.,1.),p("penny","Penny price threshold",0.,0.,100.,1.),crate::intraday::align_param()])
}

/// Calculate raw signals from explicitly expanded daily values; preserves the AFL's clock-reset counter.
fn signals(time: &[u16], day: &[u8], high: &[f64], close: &[f64], expanded_day: &[Option<usize>],
           entry_ok:&[bool], entry_day: usize, entry_bar: usize, exit_bar: usize) -> Result<Vec<(bool, bool)>, &'static str> {
    let n = time.len();
    if [day.len(), high.len(), close.len(), expanded_day.len(),entry_ok.len()].iter().any(|&v| v != n) { return Err("unaligned nth-candle inputs"); }
    let mut count = 0usize;
    let mut out = Vec::with_capacity(n);
    for i in 0..n {
        if i > 0 { count = if time[i] < time[i - 1] { 1 } else { count + 1 }; }
        let end = i + 1 < n && day[i] != day[i + 1];
        let entry = entry_ok[i] && expanded_day[i] == Some(entry_day) && count == entry_bar && high[i] != close[i];
        out.push((entry, count >= exit_bar || end));
    }
    Ok(out)
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    run_capture(id,x,from,to,p,&mut Vec::new())
}
fn run_capture(id: &str, x: &Ctx, from: usize, to: usize, p: &Params,capture:&mut Vec<(usize,usize)>) -> Option<Vec<Trade>> {
    if id != ID { return None; }
    if x.r.is_empty() || from > to || to >= x.r.len() { return Some(Vec::new()); }
    let val = |key: &str, default: f64, max: f64| p.get(key).copied().filter(|v| v.is_finite()).unwrap_or(default).round().clamp(1., max) as usize;
    let tf = val("tf_min", 1., 60.) as u16;
    let ticker = format!("{}{}", crate::intraday::BASKET_PREFIX, x.r[0].ticker);
    let Some(minutes) = crate::intraday::minutes(&ticker) else { return Some(Vec::new()); };
    let (b, di) = crate::books::ytsti::candles_for(&minutes, x, tf,crate::intraday::align_of(p));
    let day_of_month: Vec<u8> = x.r.iter().map(|r| r.date.get(8..10).and_then(|v| v.parse().ok()).unwrap_or(0)).collect();
    if day_of_month.contains(&0) { return Some(Vec::new()); }
    let mut today = vec![0usize; x.r.len()];
    for i in 1..today.len() { today[i] = if day_of_month[i] < day_of_month[i-1] { 1 } else { today[i-1] + 1 }; }
    let day: Vec<u8> = di.iter().map(|&i| day_of_month[i]).collect();
    // Ref(today,1) first, then expandLast: prior daily reference until the last intraday bar.
    let expanded: Vec<Option<usize>> = (0..b.len()).map(|i| {
        let d = if b.last_of_day(i) { Some(di[i]) } else { di[i].checked_sub(1) };
        d.and_then(|v| today.get(v + 1).copied())
    }).collect();
    let on=|key:&str|p.get(key).is_some_and(|v|v.is_finite()&&*v>=0.5);
    let penny=p.get("penny").copied().filter(|v|v.is_finite()).unwrap_or(0.).clamp(0.,100.);
    let entry_ok:Vec<bool>=(0..b.len()).map(|i| {
        let deadstock=if on("deadstock"){i+1<b.len()}else{i>0};
        let turnover=!on("turnover") || b.oi[i].zip(crate::intraday::open_interest_threshold(&minutes.dates[b.day[i] as usize],false)).is_some_and(|(oi,level)|oi>level);
        deadstock && turnover && b.c[i]>penny
    }).collect();
    let Ok(raw) = signals(&b.hhmm, &day, &b.h, &b.c, &expanded, &entry_ok, val("entry_day", 1., 19.), val("entry_bar", 1., 25.), val("exit_bar", 1., 400.)) else { return Some(Vec::new()); };
    let dir = if p.get("side").copied().unwrap_or(0.) >= 0.5 { -1. } else { 1. };
    let mut trades = Vec::new(); let mut pos: Option<usize> = None; let mut armed = true;
    for i in 0..b.len() {
        if di[i] > to { break; }
        let (entry, exit) = raw[i];
        let new_signal = entry && armed;
        if new_signal { armed = false; }
        if exit { armed = true; }
        if let Some(e) = pos {
            if exit {
                trades.push(Trade { dir, legs: vec![Leg { i: di[e], px: b.c[e], w: 1. }], exit_i: di[i], exit_px: b.c[i],
                    reason: format!("Nth candle {:04} to {:04}; {tf}-minute candles", b.hhmm[e], b.hhmm[i]), entry_kind: "close", signal_i: di[i], open: false, exits: Vec::new() });
                capture.push((e,i));
                pos = None;
            }
        } else if new_signal && di[i] >= from { pos = Some(i); }
    }
    if let Some(e) = pos {
        if let Some(i) = (e..b.len()).rev().find(|&i| di[i] <= to) {
            trades.push(Trade { dir, legs: vec![Leg { i: di[e], px: b.c[e], w: 1. }], exit_i: di[i], exit_px: b.c[i],
                reason: "Open position at end of selected period".into(), entry_kind: "close", signal_i: di[i], open: true, exits: Vec::new() });
            capture.push((e,i));
        }
    }
    crate::intraday::record(&minutes,&b,tf,crate::intraday::align_of(p),&trades,capture);
    Some(trades)
}
pub(crate) fn portfolio_observations(id:&str,x:&Ctx,from:usize,to:usize,p:&Params,first_id:u64)
    ->Result<(Vec<crate::yt_intraday_portfolio::Signal>,Vec<crate::yt_intraday_portfolio::Mark>),String> {
    let schema=params(id).ok_or("unknown nth-candle strategy")?;
    crate::intraday::validate_research_params(&schema,p)?;
    if x.r.is_empty() || from>to || to>=x.r.len(){return Err("invalid selected daily coverage".into());}
    let ticker=format!("{}{}",crate::intraday::BASKET_PREFIX,x.r[0].ticker);
    crate::intraday::available(&ticker)?;
    let minutes=crate::intraday::minutes(&ticker).ok_or("intraday bars are unavailable")?;
    let tf=p.get("tf_min").copied().unwrap_or(1.).round().clamp(1.,60.) as u16;
    let (bars,_)=crate::books::ytsti::candles_for(&minutes,x,tf,crate::intraday::align_of(p));
    let mut indices=Vec::new();let trades=run_capture(id,x,from,to,p,&mut indices).ok_or("unknown nth-candle strategy")?;
    if indices.len()!=trades.len(){return Err("timestamped source trade count mismatch".into());}
    let timed=trades.into_iter().zip(indices).map(|(t,(e,z))|(t,e,z)).collect();
    crate::yt_intraday_portfolio::close_observations(x,from,to,&minutes,&bars,timed,first_id)
}
pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    (id == ID).then_some((ID, "Intraday Nth Candle", "Intraday Nth Candle"))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn preserves_same_bar_signal_and_clock_counter() {
        let s = signals(&[1529, 915, 916], &[1, 2, 2], &[11., 11., 11.], &[10., 10., 10.], &[Some(1); 3], &[false,true,true], 1, 1, 1).unwrap();
        assert_eq!(s[1], (true, true));
        assert!(!s[2].0 && s[2].1);
        assert!(signals(&[915], &[], &[], &[], &[], &[], 1, 1, 1).is_err());
    }
}
