//! Explicit execution-cost and normalized-allocation ledger. Test fixtures are arithmetic, not market claims.
use crate::{connors::{Ctx,Trade},day_number,esc,num};
use std::collections::BTreeMap;

#[derive(Clone,Copy,Debug,Default)]
pub struct Costs { pub fees_bps:f64,pub slippage_bps:f64,pub rollover_bps:f64 }
impl Costs {
    pub fn parse(q:&BTreeMap<String,String>)->Result<Self,String>{
        // Only the YT (AmiBroker-port) catalogue has a whole-unit money account; see main::yt_capital.
        if q.contains_key("capital") && q.get("_guru").map(String::as_str)!=Some("yt") {return Err("Capital sizing is not supported by this percentage-based report".into());}
        let value=|key:&str|->Result<f64,String>{
            match q.get(key) {
                None=>Ok(0.),
                Some(raw)=>raw.parse::<f64>().ok().filter(|n|n.is_finite()&&*n>=0.&&*n<=5000.).ok_or_else(||format!("{key} must be a finite number from 0 to 5000 bps"))
            }
        };
        let c=Self{fees_bps:value("cost")?,slippage_bps:value("slippage")?,rollover_bps:value("rollover")?};
        if c.fees_bps+c.slippage_bps>5000. {return Err("Combined cost and slippage cannot exceed 5000 bps per side".into());}
        Ok(c)
    }
    pub fn side_rate(self)->f64{(self.fees_bps+self.slippage_bps)/10000.}
    pub fn rollover_issue(self)->Option<&'static str>{
        (self.rollover_bps>0.).then_some("Rollover costs require verified futures contract identities, expiry/roll dates, held quantities and traded roll prices. The loaded cash/index OHLC does not provide a supported roll-event mapping; no roll dates or charges are invented.")
    }
    pub fn ensure_rollover(self)->Result<(),String>{self.rollover_issue().map_or(Ok(()),|note|Err(format!("needs_data: {note}")))}
    pub fn json(self)->String{
        format!("{{\"fees_bps\":{},\"slippage_bps\":{},\"rollover_bps\":{},\"total_side_bps\":{},\"taxes\":null,\"rolls\":null,\"basis\":\"User-entered execution fees and slippage, each in bps per side of deployed normalized allocation. Fees are not a verified statutory tax schedule.\",\"rollover_status\":\"No verified contract roll-event mapping\",\"slippage_status\":\"User-entered return deduction; fill prices are not rewritten\",\"money_status\":\"No position quantities or monetary sizing supplied; monetary P&L and taxes remain unavailable\",\"open_trade_cost_basis\":\"Legacy equity includes estimated exit costs on marked-open trades\"}}",num(self.fees_bps),num(self.slippage_bps),num(self.rollover_bps),num(self.fees_bps+self.slippage_bps))
    }
    pub fn source_account_json(self)->String {
        self.json().replace("Legacy equity includes estimated exit costs on marked-open trades","Protected source account charges actual entry costs on open marks; no exit transaction or liquidation fee is invented")
    }
}

pub struct TradeLedger {
    symbol:String,side:&'static str,open:bool,
    pub gross_pnl:f64,pub fees:f64,pub slippage:f64,pub net_pnl:f64,
    pub gross_ret:f64,pub net_ret:f64,pub fee_ret:f64,pub slippage_ret:f64,pub pts:f64,pub days_held:i64,
    pub observed_mae:Option<f64>,pub observed_mfe:Option<f64>,pub excursion_bars:usize,allocation:f64,entry_only_open:bool,amibroker_excursions:bool
}
impl TradeLedger {
    pub fn new(ctx:&Ctx,t:&Trade,allocation:f64,costs:Costs)->Self{
        Self::with_open_cost_basis(ctx,t,allocation,costs,false)
    }
    /// AmiBroker trade-list MAE/MFE for books ported from AmiBroker AFLs: the high/low of every bar from the
    /// entry bar up to the bar before the exit (the entry bar's full range counts even for a close fill, as
    /// AmiBroker does), plus the exit fill itself on the exit bar.
    pub fn amibroker(ctx:&Ctx,t:&Trade,allocation:f64,costs:Costs)->Self{
        let mut s=Self::new(ctx,t,allocation,costs);
        let (mut mae,mut mfe,mut bars)=(0f64,0f64,0);
        // Every bar from entry up to the bar before the exit counts its high and low; a same-day trade's only bar is
        // the entry bar, so its high and low count too (AmiBroker, checked on the team's Outside Day trade lists).
        for k in t.legs[0].i..t.exit_i.max(t.legs[0].i+1) {
            let bounds=[ctx.l[k],ctx.h[k]].map(|px|{
                let mark=t.mark(k,px);
                t.legs.iter().filter(|l|l.i<=k).map(|l|l.w*t.dir*(mark/l.px-1.)).sum::<f64>()*allocation
            });
            if !bounds.iter().all(|n|n.is_finite()){continue;}
            bars+=1;
            mae=mae.min(bounds[0]).min(bounds[1]);mfe=mfe.max(bounds[0]).max(bounds[1]);
        }
        // On the exit bar only its open and the exit fill are known to have traded while the trade was open.
        let open_mark=if !t.open && t.exit_i>t.legs[0].i {
            let mark=t.mark(t.exit_i,ctx.o[t.exit_i]);
            t.legs.iter().map(|l|l.w*t.dir*(mark/l.px-1.)).sum::<f64>()*allocation
        }else{f64::NAN};
        for n in [s.gross_ret*allocation,open_mark] {if n.is_finite() {mae=mae.min(n);mfe=mfe.max(n);}}
        s.observed_mae=Some(mae);s.observed_mfe=Some(mfe);s.excursion_bars=bars;s.amibroker_excursions=true;
        s
    }
    pub fn source_account(ctx:&Ctx,t:&Trade,allocation:f64,costs:Costs)->Self{
        Self::with_open_cost_basis(ctx,t,allocation,costs,true)
    }
    fn with_open_cost_basis(ctx:&Ctx,t:&Trade,allocation:f64,costs:Costs,entry_only_open:bool)->Self{
        let (gross_ret,net_ret)=(t.ret(0.),t.ret(costs.side_rate()));
        let sides=if entry_only_open && t.open {1.}else{2.};
        let fee_ret=sides*t.deployed()*costs.fees_bps/10000.;let slippage_ret=sides*t.deployed()*costs.slippage_bps/10000.;
        let net_ret=if entry_only_open && t.open {gross_ret-fee_ret-slippage_ret}else{net_ret};
        let mut observed_mae:Option<f64>=None;let mut observed_mfe:Option<f64>=None;let mut excursion_bars=0;
        // Event-day daily ranges do not establish whether extremes happened before
        // entry or after exit. Only complete event-free held bars are eligible.
        for k in t.legs[0].i+1..t.exit_i {
            if t.legs.iter().any(|l|l.i==k)||t.exits.iter().any(|e|e.0==k){continue;}
            let bounds=[ctx.l[k],ctx.h[k]].map(|px|{
                let mark=t.mark(k,px);
                t.legs.iter().filter(|l|l.i<k).map(|l|l.w*t.dir*(mark/l.px-1.)).sum::<f64>()*allocation
            });
            if !bounds.iter().all(|n|n.is_finite()){continue;}
            excursion_bars+=1;
            observed_mae=Some(observed_mae.unwrap_or(0.).min(bounds[0]).min(bounds[1]));
            observed_mfe=Some(observed_mfe.unwrap_or(0.).max(bounds[0]).max(bounds[1]));
        }
        Self{symbol:ctx.r[t.legs[0].i].ticker.clone(),side:if t.dir>0.{"Long"}else{"Short"},open:t.open,gross_pnl:allocation*gross_ret,fees:allocation*fee_ret,slippage:allocation*slippage_ret,net_pnl:allocation*net_ret,
            gross_ret,net_ret,fee_ret,slippage_ret,pts:t.dir*(t.exit_px-t.avg_px()),days_held:day_number(&ctx.r[t.exit_i].date)-day_number(&ctx.r[t.legs[0].i].date),observed_mae,observed_mfe,excursion_bars,allocation,entry_only_open,amibroker_excursions:false}
    }
    pub fn json(&self)->String{
        let pct=|v:Option<f64>|v.map_or("null".into(),|n|num(n/self.allocation));
        format!("{{\"symbol\":\"{}\",\"signal_side\":\"{}\",\"pts\":{},\"days_held\":{},\"gross_ret\":{},\"net_ret\":{},\"fee_ret\":{},\"slippage_ret\":{},\"gross_pnl\":null,\"fees\":null,\"slippage\":null,\"taxes\":null,\"net_pnl\":null,\"money_status\":\"No position quantities or monetary sizing supplied\",\"return_basis\":\"Existing normalized allocation and actual filled leg weights; not contract quantities\",\"pts_basis\":\"Direction-adjusted effective exit price minus weighted average entry price; not monetary profit\",\"cost_status\":\"{}\",\"mae\":null,\"mfe\":null,\"mae_ret_observed\":{},\"mfe_ret_observed\":{},\"excursion_bars\":{},\"excursion_status\":\"{}\",\"excursion_basis\":\"{}\"}}",
            esc(&self.symbol),self.side,num(self.pts),self.days_held,num(self.gross_ret),num(self.net_ret),num(self.fee_ret),num(self.slippage_ret),if self.open&&self.entry_only_open{"actual_entry_cost_only_open_mark"}else if self.open{"open_liquidation_estimate"}else{"modeled_round_trip_cost"},pct(self.observed_mae),pct(self.observed_mfe),self.excursion_bars,
            if self.amibroker_excursions{"amibroker_daily_ohlc"}else if self.excursion_bars>0{"partial_daily_ohlc_bounds"}else{"unavailable"},
            if self.amibroker_excursions{"AmiBroker trade-list convention on gross allocation returns: high/low of the entry bar through the bar before the exit, plus the exit fill. The entry bar's full range is counted even when the fill is at its close."}else{"Gross allocation returns on event-free full held OHLC bars only, preserving banked scale-out fills. Execution-day ordering is unknown; exact full-trade MAE/MFE remains unavailable."})
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::{connors::{Ctx,Leg,Trade},Row};
    use std::collections::BTreeMap;
    #[test]
    fn protected_source_open_mark_has_one_actual_cost_side_and_preserves_legacy_estimate(){
        let r=rows();let x=Ctx::new(&r,&r,&[]);let t=Trade{dir:1.,legs:vec![Leg{i:0,px:100.,w:0.4}],exit_i:2,exit_px:110.,reason:"actual_open_holding_mark".into(),entry_kind:"open",signal_i:2,open:true,exits:vec![]};
        let c=Costs{fees_bps:3.,slippage_bps:2.,rollover_bps:0.};let source=TradeLedger::source_account(&x,&t,1.,c);let legacy=TradeLedger::new(&x,&t,1.,c);
        assert!((source.net_ret-(0.04-0.4*0.0005)).abs()<1e-12);assert!((legacy.net_ret-(0.04-0.4*0.001)).abs()<1e-12);assert!(source.json().contains("actual_entry_cost_only_open_mark"));assert!(legacy.json().contains("open_liquidation_estimate"));
        assert!((source.gross_pnl-source.fees-source.slippage-source.net_pnl).abs()<1e-12);
    }
    fn inputs(values:&[(&str,&str)]) -> Costs { Costs::parse(&values.iter().map(|(k,v)|(k.to_string(),v.to_string())).collect()).unwrap() }
    fn rows()->Vec<Row>{
        [(100.,120.,70.),(80.,120.,60.),(90.,110.,70.),(100.,120.,80.)].iter().enumerate().map(|(i,&(c,h,l))|Row{ticker:"TEST".into(),date:format!("2020-01-0{}",i+1),open:c,high:h,low:l,close:c,volume:Some(100.)}).collect()
    }
    fn trade()->Trade{Trade{dir:1.,legs:vec![Leg{i:0,px:100.,w:0.25},Leg{i:1,px:80.,w:0.75}],exit_i:3,exit_px:100.,reason:"arithmetic exit".into(),entry_kind:"close",signal_i:3,open:false,exits:vec![]}}
    fn near(a:f64,b:f64){assert!((a-b).abs()<1e-8,"{a} != {b}");}
    #[test]
    fn costs_are_explicit_finite_and_added_exactly_once(){
        let c=inputs(&[("cost","3"),("slippage","2")]);near(c.side_rate(),0.0005);
        for (key,value) in [("cost","NaN"),("slippage","-1"),("rollover","inf"),("cost","5001")]{
            assert!(Costs::parse(&BTreeMap::from([(key.to_string(),value.to_string())])).is_err(),"{key} {value}");
        }
        assert!(Costs::parse(&BTreeMap::from([("cost".into(),"4000".into()),("slippage".into(),"2000".into())])).is_err());
    }
    #[test]
    fn nonzero_rollover_has_no_invented_roll_events(){
        let c=inputs(&[("rollover","4")]);assert!(c.rollover_issue().unwrap().contains("contract"));assert!(inputs(&[]).rollover_issue().is_none());
    }
    #[test]
    fn weighted_leg_ledger_reconciles_actual_net_allocation_and_costs(){
        let r=rows();let ctx=Ctx::new(&r,&r,&[]);let t=trade();let c=inputs(&[("cost","3"),("slippage","2")]);
        let a=TradeLedger::new(&ctx,&t,10000.,c);
        near(a.gross_pnl,1875.);near(a.fees,6.);near(a.slippage,4.);near(a.net_pnl,1865.);near(a.gross_pnl-a.fees-a.slippage,a.net_pnl);
        near(a.net_ret,t.ret(c.side_rate()));near(a.pts,15.);assert_eq!(a.days_held,3);
    }
    #[test]
    fn excursions_use_only_known_full_held_bars_and_reverse_for_short(){
        let r=rows();let ctx=Ctx::new(&r,&r,&[]);let t=trade();let a=TradeLedger::new(&ctx,&t,10000.,inputs(&[]));
        near(a.observed_mae.unwrap(),-1687.5);near(a.observed_mfe.unwrap(),3062.5);assert_eq!(a.excursion_bars,1);
        let s=Trade{dir:-1.,..t};let b=TradeLedger::new(&ctx,&s,10000.,inputs(&[]));near(b.observed_mae.unwrap(),-3062.5);near(b.observed_mfe.unwrap(),1687.5);
        assert!(a.json().contains("\"mae\":null"));assert!(a.json().contains("partial_daily_ohlc_bounds"));
    }
    #[test]
    fn amibroker_excursions_count_entry_bar_range_and_exit_fill(){
        // Entry at the close of bar 0 (100; bar range 70-120), held bars 1-2, exit at 95 on bar 3 (range 80-120).
        let r=rows();let ctx=Ctx::new(&r,&r,&[]);
        let t=Trade{legs:vec![Leg{i:0,px:100.,w:1.}],exit_px:95.,..trade()};
        let a=TradeLedger::amibroker(&ctx,&t,1.,inputs(&[]));
        near(a.observed_mae.unwrap(),-0.4);near(a.observed_mfe.unwrap(),0.2);assert_eq!(a.excursion_bars,3);
        assert!(a.json().contains("amibroker_daily_ohlc"));
        // The exit bar's own high/low is not used, only the exit fill.
        let r2:Vec<Row>=[(100.,100.,100.),(100.,100.,100.),(95.,200.,10.)].iter().enumerate().map(|(i,&(c,h,l))|Row{ticker:"TEST".into(),date:format!("2020-01-0{}",i+1),open:c,high:h,low:l,close:c,volume:None}).collect();
        let x=Ctx::new(&r2,&r2,&[]);let u=Trade{legs:vec![Leg{i:0,px:100.,w:1.}],exit_i:2,signal_i:2,exit_px:95.,..trade()};
        let b=TradeLedger::amibroker(&x,&u,1.,inputs(&[]));near(b.observed_mae.unwrap(),-0.05);near(b.observed_mfe.unwrap(),0.);
        let s=TradeLedger::amibroker(&x,&Trade{dir:-1.,..u.clone()},1.,inputs(&[]));near(s.observed_mae.unwrap(),0.);near(s.observed_mfe.unwrap(),0.05);
        // The exit bar's open does count: a short exiting at the close after opening 3% higher saw a 3% adverse move.
        let mut r3=r2.clone();r3[2].open=103.;let y=Ctx::new(&r3,&r3,&[]);
        let o=TradeLedger::amibroker(&y,&Trade{dir:-1.,..u.clone()},1.,inputs(&[]));near(o.observed_mae.unwrap(),-0.03);near(o.observed_mfe.unwrap(),0.05);
        // A same-day trade (entry and exit on bar 2, range 10-200) counts that day's high and low.
        let d=TradeLedger::amibroker(&x,&Trade{legs:vec![Leg{i:2,px:100.,w:1.}],exit_i:2,signal_i:2,exit_px:95.,..u},1.,inputs(&[]));
        near(d.observed_mae.unwrap(),-0.9);near(d.observed_mfe.unwrap(),1.0);
    }
    #[test]
    fn scale_out_bank_is_retained_and_same_day_excursions_remain_unavailable(){
        let mut r=rows();r[2].high=120.;r[2].low=80.;let ctx=Ctx::new(&r,&r,&[]);
        let t=Trade{legs:vec![Leg{i:0,px:100.,w:1.}],exits:vec![(1,110.,0.5),(3,90.,0.5)],..trade()};
        let a=TradeLedger::new(&ctx,&t,10000.,inputs(&[]));near(a.observed_mae.unwrap(),-500.);near(a.observed_mfe.unwrap(),1500.);
        let same=Trade{exit_i:0,signal_i:0,exit_px:100.,exits:vec![],..t};let b=TradeLedger::new(&ctx,&same,10000.,inputs(&[]));assert!(b.observed_mae.is_none());assert!(b.observed_mfe.is_none());assert_eq!(b.days_held,0);
    }
    #[test]
    fn zero_slippage_retains_legacy_cost_and_open_trade_is_labeled_estimate(){
        let r=rows();let ctx=Ctx::new(&r,&r,&[]);let c=inputs(&[("cost","5")]);let mut t=trade();t.open=true;
        let a=TradeLedger::new(&ctx,&t,1.,c);near(a.net_ret,t.ret(0.0005));assert!(a.json().contains("open_liquidation_estimate"));
    }
}
