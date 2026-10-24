//! Synthetic integration guards only; market account/handler acceptance stays separate.
use super::*;
const ID:&str="lkqts_macs";
fn empty()->State{State{ctxs:BTreeMap::new(),baskets:baskets::Catalogue::default(),vix:vec![],market_extra:BTreeMap::new()}}
fn owned(raw:&str)->BTreeMap<String,String>{owned_query(&query(&format!("/api/guru/connors/run?{raw}")).1,"connors")}
fn definition()->connors::StrategyDef{books::lkqts::catalogue().into_iter().find(|d|d.id==ID).unwrap()}
fn source(to:usize,future_change:bool)->baskets::macs_source_data::ValidatedSeries{
    let mut csv=String::from("date,open,high,low,close,volume\n");for i in 0..101{let c=if i==79{"0.20000000000000001"}else if future_change&&i>80{"0.25"}else if i%2==0{"0.1"}else{"0.2"};csv.push_str(&format!("2000-{:02}-{:02},0.15,0.3,0.05,{c},0\n",i/28+1,i%28+1));}baskets::macs_source_data::synthetic_raw_fixture(&csv,to)
}
#[test]
fn exact_macs_native_scope_preserves_sob_tiar_and_all_prior_profile_lists(){
    assert_eq!(books::lkqts::STOCK_CANDIDATE_IDS,&["lkqts_cb4020","lkqts_ma1040","lkqts_mom80"]);assert_eq!(books::lkqts::NEXT_CANDIDATE_IDS,&["lkqts_volbo","lkqts_rsi14","lkqts_macd","lkqts_macd_literal_alpha"]);
    for id in [ID,"lkqts_sob","lkqts_tiar"]{assert!(books::lkqts::stock_research_profile(id));}for id in ["lkqts_macs_strict","lkqts_macs2","lkqts_mom20","lkqts_volrev"]{assert!(!books::lkqts::stock_research_profile(id));}
    let d=definition();let p=instrument_policy::policy(&d);assert_eq!(p.allowed_types,&["individual_stock"]);assert_eq!(p.execution_scope,"single_symbol_stock_price_research");assert!(instrument_policy::metadata_json(&d).contains(books::lkqts::MACS_RAW_NUMERIC_CONVENTION));assert_eq!(gate::get(ID).unwrap().verdict,"unverified");assert_eq!(gate::get(ID).unwrap().ours,"");let mut wrong=d;wrong.book="other";assert!(!instrument_policy::reviewed_kestner_stock(&wrong));
    // The selected-account bridge must delegate unchanged for existing TIAR/SOB.
    let rows:Vec<Row>=(0..131).map(|i|{let close=100.+i as f64;Row{ticker:"AAPL".into(),date:format!("2000-{:02}-{:02}",i/28+1,i%28+1),open:close,high:close+1.,low:close-1.,close,volume:None}}).collect();
    let rows=Box::leak(rows.into_boxed_slice());let ctx=Ctx::new(rows,rows,&[]);let selected=SelectedContext::Preloaded(&ctx);let st=empty();let q=BTreeMap::new();let costs=accounting::Costs{fees_bps:3.,slippage_bps:2.,rollover_bps:0.};
    for (id,from) in [("lkqts_tiar",15),("lkqts_sob",118)]{let before=backtest_costs(&ctx,id,from,130,costs,&Params::new()).unwrap();let after=selected_backtest_costs(&selected,&st,&q,&ctx,id,from,130,costs,&Params::new()).unwrap();assert_eq!(before.equity,after.equity);assert_eq!(before.bh,after.bh);assert_eq!(before.trades.len(),after.trades.len());for (a,b) in before.trades.iter().zip(&after.trades){assert_eq!((a.dir,a.legs[0].i,a.legs[0].px,a.exit_i,a.exit_px,a.open),(b.dir,b.legs[0].i,b.legs[0].px,b.exit_i,b.exit_px,b.open));}}
    let p=books::lkqts::params(ID).unwrap();assert_eq!(p.len(),2);for s in p{assert_eq!((s.def,s.min,s.max),(s.def,s.def,s.def));}
}
#[test]
fn exact_raw_query_defaults_and_prior_error_contracts_are_preserved(){
    for tail in ["p.long=59","p.short=40.5","p.long=60.00000000000000001","p.long=NaN","p.short=1e-400","p.long=","p.unknown=1","g.long=59:60:1","g.short=40:40:0","profile=wrong","source_profile=wrong","source_convention=wrong","execution_scope=portfolio","numeric_convention=rounded"]{let q=owned(&format!("id={ID}&ticker=missing&{tail}"));assert!(checked_params_of(&q).is_err(),"{tail}");for r in [detail(&empty(),&q),readiness_json(&empty(),&q),summary(&empty(),&q),optimize(&empty(),&q)]{assert!(!r.unwrap_err().starts_with("unknown ticker"));}}
    for raw in ["id=lkqts_macs&id=lkqts_macs","id=lkqts_macs&p.long=bad&p.long=60","id=lkqts_macs&g.long=bad&g.long=60","id=lkqts_macs&p.long","%69d=lkqts_macs"]{assert!(checked_params_of(&owned(raw)).is_err());}
    for raw in ["id=lkqts_macs","id=lkqts_macs&p.long=60&p.short=40","id=lkqts_macs&p.long=6e1&p.short=400e-1&g.long=60:60:1&g.short=40","id=lkqts_macs&split=2000-02-01"]{assert!(checked_params_of(&owned(raw)).is_ok());}
    for key in ["profile","source_profile","source_convention","execution_scope"]{assert_eq!(checked_params_of(&owned(&format!("id=lkqts_sob&{key}=x"))).unwrap_err(),"Kestner SOB profile and source convention are server-owned; request overrides are unsupported");assert_eq!(checked_params_of(&owned(&format!("id=lkqts_tiar&{key}=x"))).unwrap_err(),"Kestner TIAR profile and source convention are server-owned; request overrides are unsupported");}
}
#[test]
fn raw_prefix_states_are_causal_across_source_end_and_future_mutation(){
    let short=source(80,false);let long=source(100,false);let changed=source(80,true);let p=Params::new();let profile=books::lkqts::MacsThresholdProfile::DetailedProseInclusive;
    let a=books::lkqts::macs_raw_source_diagnostics(&short,80,&p,profile).unwrap();let b=books::lkqts::macs_raw_source_diagnostics(&long,100,&p,profile).unwrap();let c=books::lkqts::macs_raw_source_diagnostics(&changed,80,&p,profile).unwrap();assert_eq!(a.states.as_slice(),&b.states[..=80]);assert_eq!(a.states,c.states);assert!(a.states[..79].iter().all(Option::is_none));assert_eq!(a.states[79].as_ref().unwrap().macs,100);
    let a=books::lkqts::macs_raw_source_run(&short,79,80,&p,profile).unwrap();let b=books::lkqts::macs_raw_source_run(&changed,79,80,&p,profile).unwrap();assert_eq!(a.len(),1);assert_eq!((a[0].legs[0].i,a[0].legs[0].px,a[0].exit_px,a[0].open),(b[0].legs[0].i,b[0].legs[0].px,b[0].exit_px,b[0].open));
}
#[test]
fn generic_canonical_fallback_is_held_and_native_raw_clocks_are_guarded(){
    let series=source(80,false);let x=Ctx::new(series.rows(),&[],&[]);let p=Params::new();assert!(books::lkqts::run(ID,&x,79,80,&p).is_none());assert!(backtest_costs(&x,ID,79,80,accounting::Costs::default(),&p).err().unwrap().starts_with("needs_data"));
    let raw=books::lkqts::macs_raw_source_run(&series,79,80,&p,books::lkqts::MacsThresholdProfile::DetailedProseInclusive).unwrap();let mut run=normalized_run(&x,raw,79,80,0.).unwrap();let mut d=diagnostic_series_through(&x,ID,&p,80);d.kestner_macs=books::lkqts::macs_raw_source_diagnostics(&series,80,&p,books::lkqts::MacsThresholdProfile::DetailedProseInclusive);validate_macs_execution_clocks(&x,&run,&d).unwrap();assert!(kestner_macs_rule_state(78,&d)=="null");assert!(kestner_macs_rule_state(79,&d).contains("\"macs\":100"));let clock=kestner_execution_clock(&x,ID,&run.trades[0],&d);assert!(clock.contains("\"exit_filled\":false"));assert!(clock.contains("\"exit_signal\":null,\"exit_fill\":null"));run.trades[0].legs[0].px=0.2;assert!(validate_macs_execution_clocks(&x,&run,&d).is_err());
}
#[test]
fn basket_and_preloaded_index_etf_native_spoofing_stay_held(){
    let body=basket_portfolio::run(&empty(),&owned("id=lkqts_macs&basket=nifty50&membership=current&max_positions=10")).unwrap();assert!(body.contains("Kestner")&&body.contains("\"status\":\"not_testable\""));
    for ticker in ["SPX","SPY","AAPL"]{assert!(instrument_policy::check(&definition(),ticker,false).is_some());}
    let series=source(80,false);let r=Box::leak(series.rows().to_vec().into_boxed_slice());let x=Ctx::new(r,r,&[]);let selected=SelectedContext::Preloaded(&x);assert!(selected.macs_raw_series(&empty(),&owned("id=lkqts_macs&ticker=basket:us_sp500:AAPL&native_stock=true"),&x,80).is_err());
}
#[test]
fn raw_account_domain_stays_separate_from_data_readiness(){
    let series=source(80,false);let mut rows=series.rows().to_vec();rows[80].close=0.05;rows[80].low=0.01;let x=Ctx::new(&rows,&[],&[]);assert!(readiness::check(&x,&definition(),79,80,&Params::new()).is_none());
    let t=Trade{dir:-1.,legs:vec![connors::Leg{i:80,px:0.01,w:1.}],exit_i:80,exit_px:0.05,signal_i:80,open:true,entry_kind:"open",reason:"synthetic unsupported short mark".into(),exits:vec![]};assert_ruined(&x,&normalized_run(&x,vec![t],79,80,0.).unwrap());
}
