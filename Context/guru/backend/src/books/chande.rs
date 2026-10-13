//! Chande, supplied 1997 partial Beyond Technical Analysis.
//! Executable OHLC adaptations, not reproduction of the original futures studies.
//! Source and independent checks: library/chande/. Original book figures are never stats.
use crate::connors::{Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "ch_bta";
pub const BASIS: &str = "OHLC market adaptation of Chande's supplied 1997 text. Each run uses the selected observed cash-index, ETF or stock prices, percentage returns and user-supplied per-side costs. Cash-index short exposure is hypothetical, with no borrow, funding, futures basis or roll model. This is not the original one-contract futures study or its dollar P&L/intraday drawdown. The supplied book stops at printed143; later chapters are missing. One position at a time; flat at the selected start, earlier bars warm indicators, terminal positions remain open with estimated liquidation costs. No capital sizing or original-study replication is claimed.";
const STOP_GAP: &str = "This futures variant needs dated contract point/tick values and the original dollar-stop conversion, actual contract/roll history and validated stop/gap/order precedence. Cash-index/stock OHLC cannot establish these futures dollar stops. No result is published.";

pub fn catalogue() -> Vec<StrategyDef> {
    let make=|id,name,dir,status,rules,notes| StrategyDef { id,name,dir,status,rules,notes,
        group:"Trend/pullback · OHLC research",session:0,needs_volume:false,needs_vix:false,
        needs_series:&[],book:BOOK,claim:None };
    vec![
        make("ch_65sma3cc","65sma-3cc reversal · OHLC adaptation","Long/Short",Status::Live,
            &["F|Use the simple mean of the most recent65 completed closes, including today.",
              "E|Three consecutive closes strictly above each day's own SMA signal long; three below signal short. Equality resets the streak.",
              "E|Enter at the next observed trading-day open. Do not repeat same-direction entries.",
              "X|The opposite signal exits and reverses at the next trading-day open. No initial stop in this baseline illustration."],
            "Printed75–80 / PDF91–96. Baseline65/3; parameters are explicit research variations. Source experiment trades one futures contract with100USD per-trade allowance. This run adapts the rule to actual selected OHLC and return-based costs; it does not reproduce the23-market source study, dollar P&L, intraday drawdown or tradable short index exposure. Later dollar stops, RAVI and exit variants are not merged."),
        make("ch_cbpb_time","CB-PB fixed-horizon · OHLC adaptation","Long",Status::Live,
            &["F|Evaluate the most recent highest-high occurrence in the previous20 completed bars; its age within yesterday's window must be strictly less than7 (0–6). Ties use the most recent occurrence.",
              "E|Today's low must be strictly below the lowest low of the previous5 completed bars; today's low is excluded from that comparison window.",
              "E|Buy at the next observed trading-day open, only when flat. This is the printed rolling HighestBar rule, not a substituted true-breakout detector.",
              "X|Exit at the close5 subsequent trading bars after entry (entry bar age0). The source entry-only study also examines10/15/20 horizons, without stops."],
            "Printed101–111 / PDF122–134. Entry code visually checked on PDF125. HighestBar(High,20)[1]<7 differs from prose's7-or-fewer phrasing. The optional27-bar window is a separate research parameter. The supplied BarsSinceEntry line visibly contains a minus sign; this implementation explicitly follows the stated fixed-horizon exit, with age0 on the open-entry day, rather than claiming literal code parity. Table4.7 entry-only study has no stop/cost deduction; this selected-market adaptation uses the chosen per-side fees/slippage. Dollar-stop, limit-target and trailing variants are withheld separately. No same-bar reentry after a closing fill."),
        make("ch_cbpb_dollar","CB-PB with futures dollar stop","Long",Status::NeedsData(STOP_GAP),
            &["E|CB-PB long entry at the next open.","X|Source Tables4.8–4.9 add a1500USD initial contract stop and fixed-horizon exit; no percentage or index-point substitute is assumed."],STOP_GAP),
        make("ch_cbpb_target","CB-PB recent-high target with dollar stop","Long",Status::NeedsData(STOP_GAP),
            &["E|CB-PB long entry at the next open.","X|Source exits at the prior20-bar high on a limit order and includes a1500USD initial stop. Limit/stop same-bar sequencing requires explicit support."],STOP_GAP),
        make("ch_cbpb_trail","CB-PB40-day trailing contract stop","Long",Status::NeedsData(STOP_GAP),
            &["E|CB-PB long entry at the next open.","X|Source code sets a stop below the prior40-bar low using1 Point, while prose says1 tick. Point=1/PriceScale; actual tick=MinMove/PriceScale. Contract-specific units are unresolved.","X|Table4.10 also uses1500USD initial stop and100USD per-trade allowance."],STOP_GAP),
    ]
}

pub fn family_of(id:&str)->Option<(&'static str,&'static str,&'static str)> {
    match id {
        "ch_65sma3cc"=>Some(("ch_reversal","65sma-3cc reversal","Long/Short · OHLC adaptation")),
        "ch_cbpb_time"=>Some(("ch_cbpb","Channel breakout–pullback","Fixed horizon · OHLC adaptation")),
        "ch_cbpb_dollar"=>Some(("ch_cbpb","Channel breakout–pullback","Futures dollar stop · Data missing")),
        "ch_cbpb_target"=>Some(("ch_cbpb","Channel breakout–pullback","Contract target/stop · Data missing")),
        "ch_cbpb_trail"=>Some(("ch_cbpb","Channel breakout–pullback","Contract trailing stop · Data missing")),_=>None
    }
}
pub fn params(id:&str)->Option<Vec<ParamDef>> {
    let p=|key,label,def,min,max,step|ParamDef{key,label,def,min,max,step};
    match id {
        "ch_65sma3cc"=>Some(vec![p("sma","SMA closes",65.,45.,85.,10.),p("confirm","Consecutive closes",3.,2.,4.,1.)]),
        "ch_cbpb_time"=>Some(vec![p("high_n","Previous high window",20.,20.,27.,7.),p("low_n","Previous low window",5.,3.,5.,2.),p("hold","Subsequent trading bars held",5.,5.,20.,5.)]),_=>None
    }
}
pub fn validate(id:&str,p:&Params)->Option<String> {
    let schema=params(id)?;
    for (key,&value) in p {
        let Some(def)=schema.iter().find(|d|d.key==key) else {return Some(format!("Unknown Chande parameter: {key}"))};
        if !value.is_finite() || value.fract()!=0. || value<def.min || value>def.max {
            return Some(format!("Chande parameter {key} must be an integer from{} to{}",def.min,def.max));
        }
    }
    None
}
fn trade(dir:f64,i:usize,price:f64)->Trade {
    Trade{dir,legs:vec![Leg{i,px:price,w:1.}],exit_i:i,exit_px:price,reason:String::new(),
        entry_kind:"open",signal_i:i,open:false,exits:vec![]}
}
/// HighestBar evaluated on yesterday's COMPLETE window; most recent tie wins.
fn cbpb_signal(x:&Ctx,i:usize,high_n:usize,low_n:usize)->bool {
    if i<high_n.max(low_n) {return false;}
    let end=i-1;
    let mut highest=end;
    for j in (i-high_n..i).rev() {
        if x.h[j]>x.h[highest] {highest=j;}
    }
    end-highest<7 && x.l[i]<x.l[i-low_n..i].iter().copied().fold(f64::INFINITY,f64::min)
}
pub fn run(id:&str,x:&Ctx,from:usize,to:usize,p:&Params)->Option<Vec<Trade>> {
    let schema=params(id)?;
    if validate(id,p).is_some() || from>=to || to>=x.r.len() {return None;}
    let get=|key:&str|p.get(key).copied().unwrap_or_else(||schema.iter().find(|d|d.key==key).unwrap().def) as usize;
    let mut out=vec![];let mut cur:Option<Trade>=None;
    if id=="ch_65sma3cc" {
        let ma=x.sma_n(get("sma"));let confirmations=get("confirm");
        let (mut side,mut count)=(0i8,0usize);
        for i in 0..to {
            let dir=if !ma[i].is_finite(){0}else if x.c[i]>ma[i]{1}else if x.c[i]<ma[i]{-1}else{0};
            if dir==0 {side=0;count=0;}else if dir==side {count+=1;}else{side=dir;count=1;}
            if i<from || dir==0 || count<confirmations || cur.as_ref().is_some_and(|t|t.dir==dir as f64) {continue;}
            if let Some(mut old)=cur.take() {
                old.exit_i=i+1;old.exit_px=x.o[i+1];old.signal_i=i;old.reason="Opposite close signal · next-open reversal".into();out.push(old);
            }
            let mut t=trade(dir as f64,i+1,x.o[i+1]);t.signal_i=i;cur=Some(t);
        }
    } else {
        let(high_n,low_n,hold)=(get("high_n"),get("low_n"),get("hold"));
        for i in from..=to {
            if let Some(t)=cur.as_mut() {
                if i>=t.legs[0].i+hold {
                    t.exit_i=i;t.exit_px=x.c[i];t.signal_i=i;t.reason=format!("{hold} subsequent bars held · close");
                    out.push(cur.take().unwrap());continue;
                }
            } else if i<to && cbpb_signal(x,i,high_n,low_n) {
                let mut t=trade(1.,i+1,x.o[i+1]);t.signal_i=i;cur=Some(t);
            }
        }
    }
    if let Some(mut t)=cur {t.exit_i=to;t.exit_px=x.c[to];t.signal_i=to;t.reason="open".into();t.open=true;out.push(t);}
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;
    fn rows()->Vec<Row> {(0..100).map(|i|Row{ticker:"TEST".into(),date:format!("{:04}-01-01",2000+i),
        open:100.,high:110.,low:90.,close:100.,volume:None}).collect()}
    #[test]
    fn baseline_reverses_on_the_next_open_without_closing_the_final_position() {
        let mut r=rows();
        for row in &mut r {row.high=150.;row.low=40.;}
        for (i,value) in [(65,101.),(66,102.),(67,103.),(68,104.)] {r[i].close=value;}
        for row in &mut r[69..] {row.close=80.;}
        r[68].open=123.;r[72].open=91.;
        let x=Ctx::new(&r,&r,&[]);let ts=run("ch_65sma3cc",&x,0,99,&Params::new()).unwrap();
        assert_eq!(ts.len(),2);assert_eq!(ts[0].dir,1.);assert_eq!(ts[0].legs[0].i,68);
        assert_eq!(ts[0].legs[0].px,123.);assert_eq!(ts[0].signal_i,71);
        assert_eq!(ts[0].exit_i,72);assert_eq!(ts[0].exit_px,91.);
        assert_eq!(ts[1].dir,-1.);assert_eq!(ts[1].legs[0].i,72);assert!(ts[1].open);
    }
    #[test]
    fn rolling_high_recency_does_not_invent_a_new_breakout_and_ties_are_recent() {
        let mut r=rows();r[25].low=89.;let x=Ctx::new(&r,&r,&[]);
        assert!(cbpb_signal(&x,25,20,5)); // old equal highs qualify via most recent tie
        let mut r=rows();r[17].high=120.;r[25].low=89.;let x=Ctx::new(&r,&r,&[]);
        assert!(!cbpb_signal(&x,25,20,5)); // yesterday's highest age7 is excluded
        let mut r=rows();r[18].high=120.;r[25].low=89.;let x=Ctx::new(&r,&r,&[]);
        assert!(cbpb_signal(&x,25,20,5)); // age6 qualifies
    }
    #[test]
    fn current_low_is_excluded_and_equality_is_not_a_new_low() {
        let r=rows();let x=Ctx::new(&r,&r,&[]);assert!(!cbpb_signal(&x,25,20,5));
        let mut r=rows();r[25].low=89.;let x=Ctx::new(&r,&r,&[]);assert!(cbpb_signal(&x,25,20,5));
    }
    #[test]
    fn fixed_exit_uses_open_entry_and_five_subsequent_bars_without_same_bar_reentry() {
        let mut r=rows();r[25].low=89.;r[26].open=103.;r[31].close=105.;r[31].low=88.;
        let x=Ctx::new(&r,&r,&[]);let ts=run("ch_cbpb_time",&x,20,40,&Params::new()).unwrap();
        assert_eq!(ts.len(),1);assert_eq!(ts[0].legs[0].i,26);assert_eq!(ts[0].legs[0].px,103.);
        assert_eq!(ts[0].exit_i,31);assert_eq!(ts[0].exit_px,105.);assert!(!ts[0].open);
    }
    #[test]
    fn last_bar_signal_has_no_future_fill_and_short_terminal_window_stays_open() {
        let mut r=rows();r[25].low=89.;let x=Ctx::new(&r,&r,&[]);
        assert!(run("ch_cbpb_time",&x,20,25,&Params::new()).unwrap().is_empty());
        let ts=run("ch_cbpb_time",&x,20,28,&Params::new()).unwrap();assert!(ts[0].open);
        assert_eq!(ts[0].exit_i,28);assert_eq!(ts[0].exit_px,100.);
    }
    #[test]
    fn variants_have_no_claims_and_contract_stops_never_create_a_successful_run() {
        let r=rows();let x=Ctx::new(&r,&r,&[]);let cat=catalogue();
        assert!(cat.iter().all(|d|d.claim.is_none()));assert_eq!(cat.iter().filter(|d|d.status==Status::Live).count(),2);
        for id in ["ch_cbpb_dollar","ch_cbpb_target","ch_cbpb_trail"] {assert!(run(id,&x,0,99,&Params::new()).is_none());}
    }
    #[test]
    fn parameters_cannot_silently_round_or_allocate_unbounded_windows() {
        for (key,val) in [("hold",5.5),("high_n",1_000_000.),("unknown",1.),("low_n",0.)] {
            assert!(validate("ch_cbpb_time",&Params::from([(key.into(),val)])).is_some());
        }
    }
}
