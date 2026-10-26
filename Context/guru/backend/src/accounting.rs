//! Explicit execution-cost and normalized-allocation ledger. Test fixtures are arithmetic, not market claims.
use crate::{connors::{Ctx,Trade},day_number,esc,num};
use std::collections::BTreeMap;

#[derive(Clone,Copy,Debug,Default)]
pub struct Costs { pub fees_bps:f64,pub slippage_bps:f64,pub rollover_bps:f64 }
impl Costs {
    pub fn parse(q:&BTreeMap<String,String>)->Result<Self,String>{
        if q.contains_key("capital") {return Err("Capital sizing is not supported by this percentage-based report".into());}
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

/// Excursion status / plain-English basis shared by every report that publishes MAE/MFE.
pub const EXCURSION_STATUS:&str="fill_aware_ohlc_path";
pub const EXCURSION_BASIS:&str="MAE = lowest price the position lived through / entry - 1, MFE = highest / entry - 1 (shorts mirrored; gross, on the trade's allocation; not clamped, so MAE can be above 0 and MFE below 0). Window: an entry at the close starts with the next bar; an entry at the open counts its whole bar; a stop/limit entry inside a bar counts only its fill price on that bar (a gap fill counts as the open). An exit at the close or an end-of-data mark counts its whole bar; an exit at the open or by a stop/limit counts only its fill price on that bar. When 1-minute bars exist for a stop/limit fill day, the minutes after the fill (before the exit) are used instead.";

#[derive(Clone,Copy,PartialEq,Debug)]
enum Fill { Close, Open, At(f64) }

/// The one MAE/MFE rule for every strategy (research-team verified 2026-10-26): the price points the position lived
/// through, as (bar, price). See EXCURSION_BASIS for the window.
pub fn path_points(ctx:&Ctx,t:&Trade)->Vec<(usize,f64)> {
    let (e,x,epx)=(t.legs[0].i,t.exit_i,t.legs[0].px);
    let xpx=t.exits.last().map_or(t.exit_px,|p|p.1);
    let entry=if t.entry_kind=="open"&&epx==ctx.o[e] {Fill::Open} else if t.entry_kind=="close"&&epx==ctx.c[e] {Fill::Close} else {Fill::At(epx)};
    let exit=if t.open||(xpx==ctx.c[x]&&t.signal_i==x) {Fill::Close} else {Fill::At(xpx)};
    let mut pts=Vec::new();
    for k in e..=x {
        let start=if k==e {entry} else {Fill::Open};
        let end=if k==x {exit} else {Fill::Close};
        if start==Fill::Close {if k==x {pts.push((k,ctx.c[k]));} continue;} // entered at this close: nothing after it on this bar
        if start==Fill::Open&&end==Fill::Close {pts.push((k,ctx.l[k]));pts.push((k,ctx.h[k]));continue;}
        // part of a bar: 1-minute bars between the fill(s) when they exist, else only the fill prices
        let inside=|m:&[crate::intraday::Bar1],from:usize,px:f64|m.iter().skip(from).position(|b|b.l<=px&&px<=b.h).map(|j|j+from);
        let minute=ctx.minutes.and_then(|m|m.day(&ctx.r[k].date)).and_then(|m|{
            let (lo,mut p)=match start {Fill::At(px)=>(inside(&m,0,px)?+1,vec![px]),_=>(0,vec![])};
            let hi=match end {Fill::At(px)=>{let j=inside(&m,lo.saturating_sub(1),px)?;p.push(px);j.max(lo)},_=>m.len()};
            for b in &m[lo..hi] {p.push(b.l);p.push(b.h);}
            Some(p)
        });
        let daily=||{
            let mut p=vec![];
            match start {Fill::At(px)=>p.push(px),_ if k==e=>p.push(ctx.o[k]),_=>{}}
            if k==x {p.push(match end {Fill::At(px)=>px,_=>ctx.c[k]});}
            p
        };
        pts.extend(minute.unwrap_or_else(daily).into_iter().map(|px|(k,px)));
    }
    pts
}

/// (MAE, MFE, bars counted) as fractions of the trade's allocation, gross; None when no finite point exists.
/// One leg of weight 1: MAE = min(price) / entry - 1 and MFE = max(price) / entry - 1 (shorts mirrored).
/// Scale-ins count from their fill bar on (the whole bar for an open fill, else the next bar); scale-out pieces
/// stay banked at their fill price from their bar on.
pub fn excursion(ctx:&Ctx,t:&Trade)->Option<(f64,f64,usize)> {
    let pts=path_points(ctx,t);
    let mut out:Option<(f64,f64)>=None;
    for &(k,px) in &pts {
        let (mut mark,mut done)=(0.,0.);
        for &(i,q,f) in &t.exits {if i<k||(i==k&&k<t.exit_i) {mark+=f*q;done+=f;}}
        let mark=mark+(1.-done)*px;
        let r:f64=t.legs.iter().enumerate().filter(|(n,l)|l.i<k||l.i==k&&(*n==0||t.entry_kind=="open"&&l.px==ctx.o[k])).map(|(_,l)|l.w*t.dir*(mark/l.px-1.)).sum();
        if !r.is_finite() {return None;}
        out=Some(out.map_or((r,r),|(a,b)|(a.min(r),b.max(r))));
    }
    let bars=pts.iter().map(|p|p.0).collect::<std::collections::BTreeSet<_>>().len();
    out.map(|(a,b)|(a,b,bars))
}

pub struct TradeLedger {
    symbol:String,side:&'static str,open:bool,
    pub gross_pnl:f64,pub fees:f64,pub slippage:f64,pub net_pnl:f64,
    pub gross_ret:f64,pub net_ret:f64,pub fee_ret:f64,pub slippage_ret:f64,pub pts:f64,pub days_held:i64,
    pub observed_mae:Option<f64>,pub observed_mfe:Option<f64>,pub excursion_bars:usize,allocation:f64,entry_only_open:bool
}
impl TradeLedger {
    pub fn new(ctx:&Ctx,t:&Trade,allocation:f64,costs:Costs)->Self{
        Self::with_open_cost_basis(ctx,t,allocation,costs,false)
    }
    pub fn source_account(ctx:&Ctx,t:&Trade,allocation:f64,costs:Costs)->Self{
        Self::with_open_cost_basis(ctx,t,allocation,costs,true)
    }
    fn with_open_cost_basis(ctx:&Ctx,t:&Trade,allocation:f64,costs:Costs,entry_only_open:bool)->Self{
        let (gross_ret,net_ret)=(t.ret(0.),t.ret(costs.side_rate()));
        let sides=if entry_only_open && t.open {1.}else{2.};
        let fee_ret=sides*t.deployed()*costs.fees_bps/10000.;let slippage_ret=sides*t.deployed()*costs.slippage_bps/10000.;
        let net_ret=if entry_only_open && t.open {gross_ret-fee_ret-slippage_ret}else{net_ret};
        let (observed_mae,observed_mfe,excursion_bars)=match excursion(ctx,t) {
            Some((mae,mfe,bars))=>(Some(mae*allocation),Some(mfe*allocation),bars),
            None=>(None,None,0),
        };
        Self{symbol:ctx.r[t.legs[0].i].ticker.clone(),side:if t.dir>0.{"Long"}else{"Short"},open:t.open,gross_pnl:allocation*gross_ret,fees:allocation*fee_ret,slippage:allocation*slippage_ret,net_pnl:allocation*net_ret,
            gross_ret,net_ret,fee_ret,slippage_ret,pts:t.dir*(t.exit_px-t.avg_px()),days_held:day_number(&ctx.r[t.exit_i].date)-day_number(&ctx.r[t.legs[0].i].date),observed_mae,observed_mfe,excursion_bars,allocation,entry_only_open}
    }
    pub fn json(&self)->String{
        let pct=|v:Option<f64>|v.map_or("null".into(),|n|crate::full_precision_trade_return(n/self.allocation));
        format!("{{\"symbol\":\"{}\",\"signal_side\":\"{}\",\"pts\":{},\"days_held\":{},\"gross_ret\":{},\"net_ret\":{},\"fee_ret\":{},\"slippage_ret\":{},\"gross_pnl\":null,\"fees\":null,\"slippage\":null,\"taxes\":null,\"net_pnl\":null,\"money_status\":\"No position quantities or monetary sizing supplied\",\"return_basis\":\"Existing normalized allocation and actual filled leg weights; not contract quantities\",\"pts_basis\":\"Direction-adjusted effective exit price minus weighted average entry price; not monetary profit\",\"cost_status\":\"{}\",\"mae\":null,\"mfe\":null,\"mae_ret_observed\":{},\"mfe_ret_observed\":{},\"excursion_bars\":{},\"excursion_status\":\"{}\",\"excursion_basis\":\"{}\"}}",
            esc(&self.symbol),self.side,num(self.pts),self.days_held,num(self.gross_ret),num(self.net_ret),num(self.fee_ret),num(self.slippage_ret),if self.open&&self.entry_only_open{"actual_entry_cost_only_open_mark"}else if self.open{"open_liquidation_estimate"}else{"modeled_round_trip_cost"},pct(self.observed_mae),pct(self.observed_mfe),self.excursion_bars,if self.excursion_bars>0{EXCURSION_STATUS}else{"unavailable"},EXCURSION_BASIS)
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
    fn excursions_weight_scale_in_legs_and_reverse_for_short(){
        // close entry: bar 0 is not counted; the add-on leg (close fill on bar 1) counts from bar 2; exit at the close counts bar 3
        let r=rows();let ctx=Ctx::new(&r,&r,&[]);let t=trade();let a=TradeLedger::new(&ctx,&t,10000.,inputs(&[]));
        near(a.observed_mae.unwrap(),-1687.5);near(a.observed_mfe.unwrap(),4250.);assert_eq!(a.excursion_bars,3);
        let s=Trade{dir:-1.,..t};let b=TradeLedger::new(&ctx,&s,10000.,inputs(&[]));near(b.observed_mae.unwrap(),-4250.);near(b.observed_mfe.unwrap(),1687.5);
        assert!(a.json().contains("\"mae\":null"));assert!(a.json().contains(EXCURSION_STATUS));
    }
    #[test]
    fn scale_out_bank_is_retained_and_same_close_trade_is_flat(){
        let mut r=rows();r[2].high=120.;r[2].low=80.;let ctx=Ctx::new(&r,&r,&[]);
        let t=Trade{legs:vec![Leg{i:0,px:100.,w:1.}],exits:vec![(1,110.,0.5),(3,90.,0.5)],..trade()};
        let a=TradeLedger::new(&ctx,&t,10000.,inputs(&[]));near(a.observed_mae.unwrap(),-1500.);near(a.observed_mfe.unwrap(),1500.);
        let same=Trade{exit_i:0,signal_i:0,exit_px:100.,exits:vec![],..t};let b=TradeLedger::new(&ctx,&same,10000.,inputs(&[]));near(b.observed_mae.unwrap(),0.);near(b.observed_mfe.unwrap(),0.);assert_eq!(b.days_held,0);
    }
    // distinct open/high/low/close so every fill type is visible
    fn path_rows()->Vec<Row>{
        [(100.,110.,90.,105.),(104.,112.,95.,100.),(101.,108.,100.5,103.),(102.,115.,99.,110.)].iter().enumerate()
            .map(|(i,&(o,h,l,c))|Row{ticker:"TEST".into(),date:format!("2020-01-0{}",i+1),open:o,high:h,low:l,close:c,volume:None}).collect()
    }
    fn one(dir:f64,i:usize,px:f64,kind:&'static str,xi:usize,xpx:f64,signal:usize)->Trade{
        Trade{dir,legs:vec![Leg{i,px,w:1.}],exit_i:xi,exit_px:xpx,reason:"fixture".into(),entry_kind:kind,signal_i:signal,open:false,exits:vec![]}
    }
    fn ex(ctx:&Ctx,t:&Trade)->(f64,f64){let e=excursion(ctx,t).unwrap();(e.0,e.1)}
    fn pair(a:(f64,f64),b:(f64,f64)){near(a.0,b.0);near(a.1,b.1);}
    #[test]
    fn close_entry_starts_next_bar_and_close_exit_counts_its_whole_bar(){
        let r=path_rows();let x=Ctx::new(&r,&r,&[]);
        pair(ex(&x,&one(1.,0,105.,"close",3,110.,3)),(95./105.-1.,115./105.-1.));
        // 1-day trade: only the exit bar (was blank)
        pair(ex(&x,&one(1.,0,105.,"close",1,100.,1)),(95./105.-1.,112./105.-1.));
        // no clamp: the lowest low stayed above the entry, so MAE is positive
        pair(ex(&x,&one(1.,1,100.,"close",2,103.,2)),(100.5/100.-1.,108./100.-1.));
    }
    #[test]
    fn open_entry_counts_its_bar_and_open_exit_counts_only_the_open(){
        let r=path_rows();let x=Ctx::new(&r,&r,&[]);
        pair(ex(&x,&one(1.,1,104.,"open",3,102.,2)),(95./104.-1.,112./104.-1.));
        // same-bar open entry, close exit: the whole bar
        pair(ex(&x,&one(1.,3,102.,"open",3,110.,3)),(99./102.-1.,115./102.-1.));
    }
    #[test]
    fn stop_fills_count_only_their_price_and_gap_fills_count_as_the_open(){
        let r=path_rows();let x=Ctx::new(&r,&r,&[]);
        // stop entry 106 inside bar 1, stop exit 100 inside bar 3: bar 2 whole, the fill bars only their fills
        pair(ex(&x,&one(1.,1,106.,"stop",3,100.,3)),(100./106.-1.,108./106.-1.));
        // gap-through stop entry at bar 1's open, limit exit gapping at bar 3's open
        pair(ex(&x,&one(1.,1,104.,"stop",3,102.,3)),(100.5/104.-1.,108./104.-1.));
        // same-bar stop entry and stop exit: just the two fills
        pair(ex(&x,&one(1.,1,106.,"stop",1,96.,1)),(96./106.-1.,0.));
    }
    #[test]
    fn shorts_take_mae_from_highs_and_mfe_from_lows(){
        let r=path_rows();let x=Ctx::new(&r,&r,&[]);
        pair(ex(&x,&one(-1.,0,105.,"close",3,110.,3)),(1.-115./105.,1.-95./105.));
        pair(ex(&x,&one(-1.,1,106.,"stop",3,100.,3)),(1.-108./106.,1.-100./106.));
    }
    #[test]
    fn stop_fill_days_with_minute_bars_use_the_minutes_between_fill_and_exit(){
        let dir=std::env::temp_dir().join(format!("mae-minutes-{}",std::process::id()));std::fs::create_dir_all(&dir).unwrap();
        let day="2020-01-02,09:30,104,105,103,104.5\n2020-01-02,09:31,104.5,106.5,104,106\n2020-01-02,09:32,106,107,105,105.5\n2020-01-02,09:33,105.5,105.6,99.5,100\n2020-01-02,09:34,100,111,100,110\n";
        std::fs::write(dir.join("TEST.m1.csv"),day).unwrap();
        std::fs::write(dir.join("TEST.m1idx.csv"),format!("Date,offset,bytes\n2020-01-02,0,{}\n",day.len())).unwrap();
        let r=path_rows();let mut x=Ctx::new(&r,&r,&[]);
        x.minutes=crate::intraday::Minutes::load(dir.to_str().unwrap(),"TEST").map(|m|&*Box::leak(Box::new(m)));
        // fill at 106 in 09:31, stop at 101 in 09:33: only 09:32 lies between
        pair(ex(&x,&one(1.,1,106.,"stop",1,101.,1)),(101./106.-1.,107./106.-1.));
        // held to bar 3's close: every minute after the fill, bar 2 and bar 3 whole
        pair(ex(&x,&one(1.,1,106.,"stop",3,110.,3)),(99./106.-1.,115./106.-1.));
        // a day without minutes (bar 3) falls back to the daily rule
        pair(ex(&x,&one(1.,2,103.,"close",3,107.,3)),(107./103.-1.,107./103.-1.));
        let _=std::fs::remove_dir_all(&dir);
    }
    #[test]
    fn zero_slippage_retains_legacy_cost_and_open_trade_is_labeled_estimate(){
        let r=rows();let ctx=Ctx::new(&r,&r,&[]);let c=inputs(&[("cost","5")]);let mut t=trade();t.open=true;
        let a=TradeLedger::new(&ctx,&t,1.,c);near(a.net_ret,t.ret(0.0005));assert!(a.json().contains("open_liquidation_estimate"));
    }
}
