//! Bounded QTS companion-code translation. See library/bandy/REVIEW-2026-10-13.md.
//! These are individual-symbol research ledgers, without source portfolio accounting.
use super::*;

pub const IDS: &[&str] = &[
    "hbqts_ma5_25", "hbqts_ma1_36", "hbqts_limit_entry", "hbqts_fixed_interval",
    "hbqts_profit_target", "hbqts_entry_id1", "hbqts_bb_breakout", "hbqts_cgo",
    "hbqts_stoch_rsi", "hbqts_dpo", "hbqts_dom", "hbqts_macd_daily", "hbqts_cmo",
    "hbqts_dema16_8",
];

pub const DEMA_INDEX_ID: &str = "hbqts_dema16_8";
pub const DEMA_INDEX_CONVENTION: &str = "qts_dema16_8_first_close_index_unit_exposure_v1";
pub const DEMA_INDEX_BASIS: &str = "Bandy Quantitative Trading Systems (2007), Figure A.7 printed322-323; printed324 illustrates the S&P500 cash index. Fixed ordered DEMA16/8 crossing, reverse crossing or one observed bar since raw Buy, then ExRem(Sell,Buy). Both internal DEMA states start at the first supplied close, following the official developer's dated2003 loop; no standalone-EMA SMA seed or native binary parity claim. Daily finalized same-close fills, zero delay and one normalized long unit-exposure position are declared research conventions for omitted settings. Full genuine prefix, flat selected start and terminal open marks retained. Actual SPX is the primary index application; actual NIFTY50 is a separately declared cash-index adaptation. Stocks, ETFs and derivatives are not substitutes. Book optimization, objective-function rankings and reported equity are not reproduced. Return-only unit compounding uses declared entry-allocation cost deductions and estimated terminal liquidation costs, without funded capital, actual cash/share quantities or statutory taxes.";

pub fn is_dema_index(id: &str) -> bool { id == DEMA_INDEX_ID }
pub fn validate_dema_index(p: &Params) -> Option<String> {
    let defaults = [("fast",16.),("slow",8.),("hold",1.)];
    p.iter().find(|(key,value)| !defaults.iter().any(|(k,v)| key.as_str()==*k && value.is_finite() && **value==*v))
        .map(|(key,_)|format!("QTS Figure A.7 supports only exact reviewed fast16/slow8/hold1; {key} is changed, unknown or invalid. Alternate grids do not inherit review."))
}

#[derive(Clone, Debug)]
pub struct DemaIndexStates {
    pub dema16: Vec<f64>, pub dema8: Vec<f64>, pub raw_buy: Vec<bool>,
    pub reverse_cross: Vec<bool>, pub raw_bars_since_buy: Vec<Option<usize>>,
    pub raw_sell: Vec<bool>, pub filtered_sell: Vec<bool>,
}

/// Exact source ordering; no execution or accounting state feeds these arrays.
pub fn dema_index_states(x: &Ctx, to: usize) -> Option<DemaIndexStates> {
    if to>=x.r.len() || x.r[..=to].iter().any(|r|
        !crate::baskets::valid_date(&r.date) || ![r.open,r.high,r.low,r.close].iter().all(|v|v.is_finite()&&*v>0.) ||
        r.low>r.high || r.open<r.low || r.open>r.high || r.close<r.low || r.close>r.high
    ) || x.r[..=to].windows(2).any(|w|w[0].date>=w[1].date || w[0].ticker!=w[1].ticker) { return None; }
    let dema16=double(&x.c[..=to],16); let dema8=double(&x.c[..=to],8);
    if dema16.iter().chain(&dema8).any(|v|!v.is_finite()) { return None; }
    let raw_buy=cross(&dema16,&dema8);
    let reverse_cross=cross(&dema8,&dema16);
    let raw_bars_since_buy=bars_since(&raw_buy);
    let raw_sell=(0..=to).map(|i|reverse_cross[i]||raw_bars_since_buy[i].is_some_and(|n|n>=1)).collect::<Vec<_>>();
    let filtered_sell=exrem(&raw_sell,&raw_buy);
    Some(DemaIndexStates{dema16,dema8,raw_buy,reverse_cross,raw_bars_since_buy,raw_sell,filtered_sell})
}

/// Formula arrays are calculated over actual loaded history BEFORE selecting a test range.
/// Raw crossover and eligible order are different events for the one-day limit example.
#[derive(Clone, Debug)]
pub struct Signal {
    pub raw_buy: bool, pub buy: bool, pub sell: bool, pub short: bool, pub cover: bool,
    pub buy_px: f64, pub sell_px: f64,
}

/// ExRem keeps the first set event, then suppresses repeats until reset.
/// All uses in the ready batch have disjoint set/reset events.
fn exrem(set: &[bool], reset: &[bool]) -> Vec<bool> {
    let mut armed = true;
    (0..set.len()).map(|i| {
        let emit = armed && set[i];
        if emit { armed = false; }
        if reset[i] { armed = true; }
        emit
    }).collect()
}

/// AFL DEMA initialization: first finite value, alpha recursion; missing
/// output stays missing rather than becoming the last valid indicator value.
/// Exact native AmiBroker export parity is not claimed for this translation.
fn smooth(v: &[f64], alpha: f64) -> Vec<f64> {
    let mut prior = f64::NAN;
    v.iter().map(|&x| {
        if !x.is_finite() { return f64::NAN; }
        prior = if prior.is_finite() { prior + alpha * (x-prior) } else { x };
        prior
    }).collect()
}
fn exp(v: &[f64], n: usize) -> Vec<f64> { smooth(v, 2.0/(n as f64+1.0)) }
/// AFL EMA uses a full-period SMA seed; DEMA below uses the first input value.
fn afl_ema(v: &[f64], n: usize) -> Vec<f64> {
    let mut out=vec![f64::NAN;v.len()];
    let first=v.iter().position(|x|x.is_finite()).unwrap_or(v.len());
    if first+n>v.len() {return out;}
    let seed=first+n-1;
    if v[first..=seed].iter().any(|x|!x.is_finite()) {return out;}
    let mut state=v[first..=seed].iter().sum::<f64>()/n as f64;
    out[seed]=state;let alpha=2.0/(n as f64+1.0);
    for i in seed+1..v.len() {if v[i].is_finite() {state+=alpha*(v[i]-state);out[i]=state;}}
    out
}
/// AmiBroker's published RSI recurrence starts P=N=0 at bar 1 and reveals
/// the ratio at bar period. A flat denominator stays Null.
fn afl_rsi(c: &[f64], period: usize) -> Vec<f64> {
    let mut out=vec![f64::NAN;c.len()];let mut up=0.0;let mut down=0.0;
    for i in 1..c.len() {
        let diff=c[i]-c[i-1];
        up=((period-1) as f64*up+diff.max(0.0))/period as f64;
        down=((period-1) as f64*down+(-diff).max(0.0))/period as f64;
        if i>=period && up+down>0.0 {out[i]=100.0*up/(up+down);}
    }
    out
}
fn double(v: &[f64], n: usize) -> Vec<f64> {
    let a=exp(v,n); let b=exp(&a,n);
    a.iter().zip(b).map(|(a,b)|a+(a-b)).collect()
}

pub fn signals(id: &str, x: &Ctx, to: usize, p: &Params) -> Option<Vec<Signal>> {
    if !IDS.contains(&id) || to>=x.r.len() { return None; }
    if is_dema_index(id) && validate_dema_index(p).is_some() { return None; }
    let schema=params(id)?;
    let g=|key:&str|p.get(key).copied().unwrap_or_else(||schema.iter().find(|d|d.key==key).unwrap().def);
    let u=|key:&str|g(key).round().max(1.0) as usize;
    let n=to+1; let c=&x.c[..n]; let h=&x.h[..n]; let l=&x.l[..n];
    let mut buy=vec![false;n]; let mut sell=vec![false;n];
    let mut short=vec![false;n]; let mut cover=vec![false;n];
    let mut bp=c.to_vec(); let mut sp=c.to_vec();
    let raw;
    match id {
        DEMA_INDEX_ID => {
            let state=dema_index_states(x,to)?;
            buy=state.raw_buy.clone(); raw=state.raw_buy; sell=state.filtered_sell;
        }
        "hbqts_ma5_25"|"hbqts_ma1_36"|"hbqts_limit_entry"|"hbqts_entry_id1"|"hbqts_profit_target" => {
            let a=sma(c,u("fast")); let b=sma(c,u("slow"));
            buy=cross(&a,&b); sell=cross(&b,&a); raw=buy.clone();
            if id=="hbqts_limit_entry" {
                let k=1.0-g("pct")/100.0;
                for i in 0..n { bp[i]=if i>0 {k*c[i-1]}else{f64::NAN}; buy[i]=i>0&&raw[i-1]&&l[i]<bp[i]; }
            } else if id=="hbqts_entry_id1" {
                sell=hold(&buy,u("hold")); buy=exrem(&buy,&sell); sell=exrem(&sell,&buy);
            } else if id=="hbqts_profit_target" {
                // Literal Figure7.9 scalar loop begins at bar ZERO, independently
                // of the selected backtest start; repeated raw Buy remains in its input.
                sell.fill(false); let mut entry=0.0;
                for i in 0..n {
                    if entry==0.0&&buy[i] {entry=c[i];}
                    else if entry>0.0&&h[i]>entry*(1.0+g("tgt")/100.0) {
                        sell[i]=true;sp[i]=entry*(1.0+g("tgt")/100.0);entry=0.0;
                    }
                }
                buy=exrem(&buy,&sell);sell=exrem(&sell,&buy);
            }
        }
        "hbqts_fixed_interval" => {
            buy=(0..n).map(|i|i%u("every")==0).collect();raw=buy.clone();
            sell=exrem(&hold(&buy,u("hold")),&buy);
        }
        "hbqts_bb_breakout" => {
            let m=sma(c,u("lb")); let sd=stdev(c,u("lb"));
            let top:Vec<_>=m.iter().zip(&sd).map(|(m,s)|m+g("w")*s).collect();
            let bot:Vec<_>=m.iter().zip(&sd).map(|(m,s)|m-g("w")*s).collect();
            buy=cross(c,&top);sell=cross(&top,c);short=cross(&bot,c);cover=cross(c,&bot);raw=buy.clone();
        }
        "hbqts_cgo" => {
            let len=u("len");let mut cg=vec![0.0;n];
            for i in len..n {
                let mut num=0.0;let mut den=0.0;
                for j in 0..len {let price=(h[i-j]+l[i-j])/2.0;num+=(j+1) as f64*price;den+=price;}
                if den!=0.0 {cg[i]=100.0*(-num/den+(len as f64+1.0)/2.0);}
            }
            let sm=double(&cg,u("sm"));buy=cross(&cg,&sm);raw=buy.clone();
            let dn=cross(&sm,&cg);let hd=hold(&buy,u("hold"));
            sell=exrem(&(0..n).map(|i|dn[i]||hd[i]).collect::<Vec<_>>(),&buy);
        }
        "hbqts_stoch_rsi" => {
            let r=afl_rsi(c,u("rsi"));let hi=hhv(&r,u("lb"));let lo=llv(&r,u("lb"));
            let pir:Vec<_>=(0..n).map(|i|(r[i]-lo[i])/(hi[i]-lo[i])).collect();
            let trigger=double(&pir,u("trig"));buy=cross(&pir,&trigger);sell=cross(&trigger,&pir);raw=buy.clone();
        }
        "hbqts_dpo" => {
            let m=sma(c,u("ma"));let osc:Vec<_>=(0..n).map(|i|c[i]-m[i]).collect();
            let sm=double(&osc,u("sm"));let trigger=double(&sm,3);
            buy=cross(&sm,&trigger);sell=cross(&trigger,&sm);raw=buy.clone();
        }
        "hbqts_dom" => {
            for i in 0..n {let d=ymd(x,i).2;buy[i]=(1..=5).contains(&d)||(10..=16).contains(&d)||(21..=31).contains(&d);short[i]=(6..=9).contains(&d)||(17..=20).contains(&d);}
            sell=short.clone();cover=buy.clone();raw=buy.clone();
        }
        "hbqts_macd_daily" => {
            let a=afl_ema(c,u("p"));let b=afl_ema(c,u("q"));let diff:Vec<_>=(0..n).map(|i|a[i]-b[i]).collect();
            let sm=afl_ema(&diff,u("r"));let bs=cross(&sma(c,8),&sma(c,7));let ss=cross(&sma(c,7),&sma(c,8));
            for i in 0..n {let valid=i>0&&sm[i].is_finite()&&sm[i-1].is_finite();let permit=valid&&sm[i]>sm[i-1];buy[i]=bs[i]&&permit;sell[i]=ss[i]||(valid&&!permit);}raw=buy.clone();
        }
        "hbqts_cmo" => {
            let mut up=vec![0.0;n];let mut dn=vec![0.0;n];
            for i in 1..n {up[i]=(c[i]-c[i-1]).max(0.0);dn[i]=(c[i-1]-c[i]).max(0.0);}
            let su=sma(&up,u("pds"));let sd=sma(&dn,u("pds"));
            let cm:Vec<_>=(0..n).map(|i|100.0*(su[i]-sd[i])/(su[i]+sd[i])).collect();
            let avg=double(&cm,u("avg"));let trigger=double(&avg,3);buy=cross(&avg,&trigger);raw=buy.clone();
            let dx=cross(&trigger,&avg);let hd=hold(&buy,u("hold"));sell=(0..n).map(|i|dx[i]||hd[i]).collect();
            buy=exrem(&buy,&sell);sell=exrem(&sell,&buy);
        }
        _=>return None,
    }
    Some((0..n).map(|i|Signal{raw_buy:raw[i],buy:buy[i],sell:sell[i],short:short[i],cover:cover[i],buy_px:bp[i],sell_px:sp[i]}).collect())
}

pub fn run(id: &str,x:&Ctx,from:usize,to:usize,p:&Params)->Option<Vec<Trade>> {
    let s=signals(id,x,to,p)?;
    if from>to {return Some(vec![]);}
    let side=if matches!(id,"hbqts_bb_breakout"|"hbqts_dom") {
        p.get("side").copied().unwrap_or(0.0).round().clamp(0.0,2.0) as u8
    } else {1};
    let mut out=vec![];let mut pos:Option<(f64,usize,f64)>=None;
    for i in from..=to {
        if id=="hbqts_limit_entry" {
            // A carried position may sell at today's close, but cannot refill
            // an earlier intraday limit while it was still held. When flat at
            // the start of the bar, the prior-day limit precedes a close exit.
            // No HoldMinBars lock is present in Figure7.2.
            if let Some((d,ei,ep))=pos {
                if s[i].sell {out.push(trade(d,ei,ep,i,clamp(x,i,s[i].sell_px),"formula exit","limit"));pos=None;}
            } else if s[i].buy {
                let ep=clamp(x,i,s[i].buy_px);
                if s[i].sell {out.push(trade(1.0,i,ep,i,clamp(x,i,s[i].sell_px),"formula exit","limit"));}
                else {pos=Some((1.0,i,ep));}
            }
            continue;
        }
        let mut exited=None;
        if let Some((d,ei,ep))=pos {
            if (d>0.0&&s[i].sell)||(d<0.0&&s[i].cover) {
                let xp=if d>0.0 {s[i].sell_px}else{x.c[i]};
                out.push(trade(d,ei,ep,i,clamp(x,i,xp),"formula exit",if id=="hbqts_limit_entry" {"limit"}else{"close"}));
                pos=None;exited=Some(d);
            }
        }
        if pos.is_none() {
            let d=if side!=2&&s[i].buy {1.0}else if side!=1&&s[i].short {-1.0}else{0.0};
            // A held long cannot fill another intraday limit before its close exit.
            // Opposite-side close reversals (DOM/BB) are explicit source signals.
            if d!=0.0&&exited!=Some(d) {let ep=if d>0.0{s[i].buy_px}else{x.c[i]};pos=Some((d,i,clamp(x,i,ep)));}
        }
    }
    if let Some((d,ei,ep))=pos {
        let mut t=trade(d,ei,ep,to,x.c[to],"open",if id=="hbqts_limit_entry" {"limit"}else{"close"});t.open=true;out.push(t);
    }
    Some(out)
}

#[cfg(test)]
#[path = "dema_index_tests.rs"]
mod dema_index_tests;

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn afl_indicator_seeds_follow_published_recurrences() {
        let e=afl_ema(&[1.0,3.0,8.0,10.0],3);
        assert!(e[0].is_nan()&&e[1].is_nan());assert_eq!(e[2],4.0);assert_eq!(e[3],7.0);
        let r=afl_rsi(&[10.0,12.0,11.0,13.0],2);
        assert!(r[1].is_nan());assert_eq!(r[2],50.0);assert_eq!(r[3],100.0*1.25/1.5);
    }
    #[test] fn exrem_preserves_first_and_resets() {
        assert_eq!(exrem(&[true,true,false,true,true],&[false,false,true,false,false]),vec![true,false,false,true,false]);
    }
    #[test] fn equality_and_null_are_not_crosses() {
        assert_eq!(cross(&[f64::NAN,1.0,1.0,2.0],&[1.0;4]),vec![false,false,false,true]);
    }
    #[test] fn missing_indicator_is_not_forward_filled() {
        let v=smooth(&[1.0,f64::NAN,3.0],0.5);assert!(v[1].is_nan());assert_eq!(v[2],2.0);
    }
    #[test] fn bars_since_uses_raw_signals_including_while_held() {
        assert_eq!(hold(&[true,false,true,false,false],2),vec![false,false,false,false,true]);
    }
    #[test] fn final_bar_entry_remains_open() {
        let r:Vec<_>=(0..4).map(|i|crate::Row{ticker:"TEST".into(),date:format!("2020-01-{:02}",i+1),open:100.0,high:102.0,low:98.0,close:100.0,volume:None}).collect();
        let x=Ctx::new(&r,&r,&[]);let p=Params::from([("every".into(),3.0),("hold".into(),1.0)]);
        let t=run("hbqts_fixed_interval",&x,1,3,&p).unwrap();assert_eq!(t.len(),1);assert_eq!(t[0].legs[0].i,3);assert!(t[0].open);
    }
    fn fixture(c: &[f64]) -> Vec<crate::Row> {
        c.iter().enumerate().map(|(i,&c)|crate::Row{ticker:"UNIT_FIXTURE".into(),date:format!("2020-01-{:02}",i+1),open:c,high:c+1.0,low:c-1.0,close:c,volume:None}).collect()
    }
    #[test] fn one_day_limit_strict_touch_and_expiration() {
        let mut r=fixture(&[100.0,99.0,98.0,100.0,100.0,103.0]);r[4].low=99.0;r[5].low=98.0;
        let p=Params::from([("fast".into(),1.0),("slow".into(),2.0),("pct".into(),1.0)]);
        let x=Ctx::new(&r,&r,&[]);let s=signals("hbqts_limit_entry",&x,5,&p).unwrap();
        assert!(s[3].raw_buy);assert!(!s[4].buy);assert!(!s[5].buy); // no carry to the later touch
        assert!(run("hbqts_limit_entry",&x,4,5,&p).unwrap().is_empty());
        r[4].low=98.99;let x=Ctx::new(&r,&r,&[]);
        let t=run("hbqts_limit_entry",&x,4,5,&p).unwrap();assert_eq!(t.len(),1);assert_eq!(t[0].legs[0].i,4);assert_eq!(t[0].legs[0].px,99.0);assert_eq!(t[0].entry_kind,"limit");
    }
    #[test] fn flat_limit_entry_precedes_same_day_close_exit() {
        let mut r=fixture(&[100.0,99.0,98.0,100.0,97.0]);r[4].high=100.0;r[4].low=96.0;
        let p=Params::from([("fast".into(),1.0),("slow".into(),2.0),("pct".into(),1.0)]);
        let x=Ctx::new(&r,&r,&[]);let s=signals("hbqts_limit_entry",&x,4,&p).unwrap();
        assert!(s[4].buy&&s[4].sell);
        let t=run("hbqts_limit_entry",&x,0,4,&p).unwrap();assert_eq!(t.len(),1);
        assert_eq!(t[0].legs[0].i,4);assert_eq!(t[0].exit_i,4);assert_eq!(t[0].legs[0].px,99.0);assert_eq!(t[0].exit_px,97.0);assert!(!t[0].open);
    }
    #[test] fn profit_loop_never_checks_the_entry_bars_high() {
        let mut r=fixture(&[100.0,99.0,98.0,100.0,100.0,100.0]);r[3].high=120.0;r[4].high=110.0;r[5].high=110.01;
        let p=Params::from([("fast".into(),1.0),("slow".into(),2.0),("tgt".into(),10.0)]);
        let x=Ctx::new(&r,&r,&[]);let s=signals("hbqts_profit_target",&x,5,&p).unwrap();
        assert!(s[3].buy);assert!(!s[3].sell);assert!(!s[4].sell);assert!(s[5].sell);
        let t=run("hbqts_profit_target",&x,0,5,&p).unwrap();assert_eq!(t.len(),1);assert_eq!(t[0].exit_i,5);
        assert!(run("hbqts_profit_target",&x,4,5,&p).unwrap().is_empty()); // warm loop was not restarted at4
    }
    #[test] fn exrem_arrays_are_not_restarted_at_test_start() {
        let r=fixture(&[100.0;10]);let x=Ctx::new(&r,&r,&[]);
        let t=run("hbqts_fixed_interval",&x,4,9,&Params::new()).unwrap();
        assert_eq!(t.len(),1);assert_eq!(t[0].legs[0].i,6);assert_eq!(t[0].exit_i,9);assert!(!t[0].open);
    }
    #[test] fn identical_smoothing_input_cannot_create_a_cross() {
        let v=double(&[7.368440908640764;30],96);let trigger=double(&v,3);
        assert!(cross(&v,&trigger).iter().all(|v|!*v));
    }
    #[test] fn flat_rsi_range_produces_no_entry() {
        let r=fixture(&[100.0;80]);let x=Ctx::new(&r,&r,&[]);
        let s=signals("hbqts_stoch_rsi",&x,79,&Params::new()).unwrap();
        assert!(s.iter().all(|s|!s.buy&&!s.sell));
    }
}
