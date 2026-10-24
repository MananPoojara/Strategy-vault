//! Data-independent TIAR integration fixtures; no market data or external services.
use super::*;
const ID:&str="lkqts_tiar";
fn rows(c:&[f64],symbol:&str)->Vec<Row>{
    c.iter().enumerate().map(|(i,&close)|Row{ticker:symbol.into(),date:format!("2000-{:02}-{:02}",i/28+1,i%28+1),open:close,high:close*1.01,low:close*0.99,close,volume:Some(1.)}).collect()
}
fn empty()->State {State{ctxs:BTreeMap::new(),baskets:baskets::Catalogue::default(),vix:vec![],market_extra:BTreeMap::new()}}
fn owned(raw:&str)->BTreeMap<String,String>{owned_query(&query(&format!("/api/guru/connors/run?{raw}")).1,"connors")}
fn def()->connors::StrategyDef{books::lkqts::catalogue().into_iter().find(|d|d.id==ID).unwrap()}
#[test]
fn exact_tiar_dispatch_leaves_sob_prior_lists_and_macs_admission_unchanged(){
    assert_eq!(books::lkqts::STOCK_CANDIDATE_IDS,&["lkqts_cb4020","lkqts_ma1040","lkqts_mom80"]);
    assert_eq!(books::lkqts::NEXT_CANDIDATE_IDS,&["lkqts_volbo","lkqts_rsi14","lkqts_macd","lkqts_macd_literal_alpha"]);
    assert!(books::lkqts::tiar_candidate(ID));assert!(books::lkqts::stock_research_profile(ID));assert!(books::lkqts::stock_research_profile("lkqts_sob"));
    for id in ["lkqts_macs_strict","lkqts_tiar2","lkqts_mom20","lkqts_volrev"]{assert!(!books::lkqts::stock_research_profile(id));}
    // Exact RAW MACS is a separately reviewed source integration; strict/other ids above stay held.
    assert!(books::lkqts::stock_research_profile("lkqts_macs"));
    let macs=books::lkqts::catalogue().into_iter().find(|d|d.id=="lkqts_macs").unwrap();assert!(instrument_policy::reviewed_kestner_stock(&macs));
    assert_eq!(instrument_policy::policy(&macs).allowed_types,&["individual_stock"]);assert_eq!(instrument_policy::policy(&macs).execution_scope,"single_symbol_stock_price_research");
    assert!(instrument_policy::metadata_json(&macs).contains(books::lkqts::MACS_SOURCE_PROFILE));assert!(instrument_policy::metadata_json(&macs).contains(books::lkqts::MACS_RAW_NUMERIC_CONVENTION));
    for p in books::lkqts::params("lkqts_macs").unwrap(){assert_eq!((p.def,p.min,p.max),(p.def,p.def,p.def));}
    for ticker in ["NIFTY 50","SPX","SPY","AAPL"]{assert!(instrument_policy::check(&macs,ticker,false).is_some());}
    let d=def();let p=instrument_policy::policy(&d);assert!(instrument_policy::reviewed_kestner_stock(&d));assert_eq!(p.allowed_types,&["index","etf","individual_stock"]);assert_eq!(p.execution_scope,"single_symbol_stock_price_research");
    let metadata=instrument_policy::metadata_json(&d);assert!(metadata.contains(books::lkqts::TIAR_SOURCE_PROFILE));assert!(metadata.contains("comparison"));assert!(metadata.contains("separate acceptance gates"));assert!(metadata.contains("other runs need their own source/data/account checks"));assert!(metadata.contains("Data readiness alone does not prove a financially supported account"));assert!(metadata.contains("\"source_performance_reproduced\":false"));
    assert!(instrument_policy::check_native(&d,"AAPL",&baskets::NativeEvidence::us_test_fixture()).is_none());
    for ticker in ["NIFTY 50","SPX","SPY","QQQ"]{assert!(instrument_policy::check(&d,ticker,false).is_none());}assert!(instrument_policy::check(&d,"AAPL",false).is_some());
    let mut wrong=d;wrong.book="other";assert!(!instrument_policy::reviewed_kestner_stock(&wrong));assert_eq!(instrument_policy::policy(&wrong).requirement,"source_review_required");assert_eq!(gate::get(ID).unwrap().verdict,"unverified");assert_eq!(gate::get(ID).unwrap().ours,"");
}
#[test]
fn all_four_tiar_handlers_reject_raw_changed_duplicate_or_forged_settings(){
    for tail in ["p.step=4","p.step=5.5","p.step=5.000000000000000001","p.step=NaN","p.step=inf","p.step=1e-400","p.step=garbage","p.step=","p.unknown=5","g.step=4:5:1","g.step=5:5:0","g.step=5:5:inf","g.step=5:5:1e-400","g.unknown=1","p.profile=1","profile=wrong","source_profile=wrong","source_convention=wrong","execution_scope=portfolio"]{
        let q=owned(&format!("id={ID}&ticker=missing&{tail}"));assert!(checked_params_of(&q).is_err(),"{tail}");
        for response in [detail(&empty(),&q),readiness_json(&empty(),&q),summary(&empty(),&q),optimize(&empty(),&q)]{let note=response.unwrap_err();assert!(!note.starts_with("unknown ticker"),"settings discarded: {tail}");}
    }
    for raw in ["id=lkqts_tiar&id=lkqts_tiar","id=lkqts_tiar&id=f1a","id=f1a&id=lkqts_tiar","id=lkqts_tiar&p.step=bad&p.step=5","id=lkqts_tiar&g.step=bad&g.step=5","id=lkqts_tiar&p.step","id=lkqts_tiar&cost","id=lkqts_tiar&p.%73tep=5","%69d=lkqts_tiar","id=lkqts_tiar&p.step&p.__raw_query_error=clear"]{assert!(checked_params_of(&owned(raw)).is_err(),"{raw}");}
    for raw in ["id=lkqts_tiar","id=lkqts_tiar&p.step=5","id=lkqts_tiar&p.step=50e-1","id=lkqts_tiar&g.step=5","id=lkqts_tiar&g.step=5:5:0.01","id=lkqts_%74iar&p.step=%35"]{assert!(checked_params_of(&owned(raw)).is_ok(),"{raw}");}
    for key in ["profile","source_profile","source_convention","execution_scope"] {
        assert_eq!(checked_params_of(&owned(&format!("id=lkqts_sob&{key}=wrong"))).unwrap_err(),"Kestner SOB profile and source convention are server-owned; request overrides are unsupported");
        assert_eq!(checked_params_of(&owned(&format!("id=lkqts_tiar&{key}=wrong"))).unwrap_err(),"Kestner TIAR profile and source convention are server-owned; request overrides are unsupported");
    }
    let p=books::lkqts::params(ID).unwrap();assert_eq!(p.len(),1);assert_eq!((p[0].def,p[0].min,p[0].max),(5.,5.,5.));assert!(checked_params_of(&owned("id=lkqts_mom80&profile=legacy_ignored")).is_ok());
}
#[test]
fn tiar_backend_state_first15_fill16_flat_selected_start_and_future_scope(){
    let c:Vec<_>=(1..=22).map(|i|i as f64).collect();let r=rows(&c,"AAPL");let x=Ctx::new(&r,&r,&[]);let p=Params::new();let d=diagnostic_series_through(&x,ID,&p,20);
    assert_eq!(kestner_tiar_rule_state(14,&d),"null");let state=kestner_tiar_rule_state(15,&d);assert!(state.contains("\"close\":16"));assert!(state.contains("\"close_lag5\":11"));assert!(state.contains("\"close_lag10\":6"));assert!(state.contains("\"close_lag15\":1"));assert!(state.contains("\"long_condition\":true,\"short_condition\":false"));
    let first=backtest_costs(&x,ID,14,16,accounting::Costs::default(),&p).unwrap();assert_eq!(first.trades.len(),1);assert_eq!((first.trades[0].legs[0].i,first.trades[0].legs[0].px),(16,r[16].open));let first_d=diagnostic_series_through(&x,ID,&p,16);validate_tiar_execution_clocks(&x,&first,&first_d).unwrap();assert!(kestner_execution_clock(&x,ID,&first.trades[0],&first_d).contains(&format!("\"entry_signal\":{{\"date\":\"{}\"",r[15].date)));
    let run=backtest_costs(&x,ID,18,20,accounting::Costs::default(),&p).unwrap();assert_eq!(run.trades.len(),1);assert_eq!(run.trades[0].legs[0].i,19);validate_tiar_execution_clocks(&x,&run,&d).unwrap();
    let clock=kestner_execution_clock(&x,ID,&run.trades[0],&d);assert!(clock.contains(&format!("\"entry_signal\":{{\"date\":\"{}\"",r[18].date)));assert!(clock.contains("\"exit_signal\":null,\"exit_fill\":null"));assert!(clock.contains("\"exit_filled\":false"));
    let mut changed=r.clone();changed[20].close=1000.;changed[20].high=1010.;let y=Ctx::new(&changed,&changed,&[]);assert_eq!(kestner_tiar_rule_state(18,&diagnostic_series_through(&y,ID,&p,20)),kestner_tiar_rule_state(18,&d));
    let mut future=r.clone();future.push(Row{ticker:"BROKEN".into(),date:"bad".into(),open:0.,high:0.,low:0.,close:f64::NAN,volume:None});let y=Ctx::new(&future,&future,&[]);assert_eq!(kestner_tiar_rule_state(15,&diagnostic_series_through(&y,ID,&p,20)),state);assert!(books::lkqts::stock_research_history_issue(ID,&y,18,22).is_some());
}
#[test]
fn strict_tiar_chain_equality_and_endpoint_momentum_do_not_manufacture_direction(){
    let mut c=vec![100.;17];c[0]=1.;c[5]=6.;c[10]=6.;c[15]=16.;let r=rows(&c,"AAPL");let x=Ctx::new(&r,&r,&[]);let d=books::lkqts::tiar_source_diagnostics(&x,16,&Params::new()).unwrap();let s=d.states[15].as_ref().unwrap();assert!(!s.long_condition&&!s.short_condition);assert!(books::lkqts::run(ID,&x,15,16,&Params::new()).unwrap().is_empty());
    c[5]=12.;c[10]=6.;let r=rows(&c,"AAPL");let x=Ctx::new(&r,&r,&[]);let d=books::lkqts::tiar_source_diagnostics(&x,16,&Params::new()).unwrap();let s=d.states[15].as_ref().unwrap();assert!(!s.long_condition&&!s.short_condition);
}
#[test]
fn tiar_clocks_reversal_and_forged_account_trade_are_guarded(){
    let c:Vec<f64>=(0..38).map(|i|if i<=20 {(i+1) as f64}else{(41-i) as f64}).collect();let r=rows(&c,"AAPL");let x=Ctx::new(&r,&r,&[]);let p=Params::new();let d=diagnostic_series_through(&x,ID,&p,37);let mut run=backtest_costs(&x,ID,15,37,accounting::Costs::default(),&p).unwrap();assert_eq!(run.trades.len(),2);assert!(!run.trades[0].open);assert!(run.trades[1].open);assert_eq!(run.trades[0].exit_i,run.trades[1].legs[0].i);validate_tiar_execution_clocks(&x,&run,&d).unwrap();
    for kind in 0..5{let old=run.trades.clone();match kind{0=>run.trades[0].legs[0].px+=1.,1=>run.trades[0].legs[0].w=0.5,2=>run.trades[0].signal_i=15,3=>run.trades[0].exit_px+=1.,_=>run.trades[1].open=false};assert!(validate_tiar_execution_clocks(&x,&run,&d).is_err());run.trades=old;}
}
#[test]
fn tiar_data_readiness_cannot_certify_nonpositive_account_and_basket_stays_held(){
    let mut c:Vec<_>=(0..18).map(|i|300.-i as f64).collect();c[16]=1000.;c[17]=1000.;let mut r=rows(&c,"AAPL");r[16].open=284.;r[16].low=280.;let x=Ctx::new(&r,&r,&[]);let p=Params::new();assert!(readiness::check(&x,&def(),15,17,&p).is_none());assert_eq!(books::lkqts::run(ID,&x,15,17,&p).unwrap().len(),1);assert_ruined(&x,&backtest_costs(&x,ID,15,17,accounting::Costs::default(),&p).unwrap());
    let q=owned("id=lkqts_tiar&basket=nifty50&max_positions=10&membership=current");let body=basket_portfolio::run(&empty(),&q).unwrap();assert!(body.contains("Kestner"));assert!(body.contains("\"status\":\"not_testable\""));assert!(body.contains("\"stats\":null,\"trades\":[],\"dates\":[],\"equity\":[]"));
}
#[test]
fn tiar_loaded_index_and_etf_cannot_forge_native_stock_policy(){
    // SPX/SPY admitted since 22 Oct 2026; a loaded unqualified stock ticker still cannot forge native identity.
    for ticker in ["AAPL"]{let r=Box::leak(rows(&vec![100.;35],ticker).into_boxed_slice());let mut st=empty();st.ctxs.insert(ticker.into(),Ctx::new(r,r,&[]));let q=owned(&format!("id={ID}&ticker={ticker}&from=2000-01-01&to=2000-02-07&asset_type=individual_stock&native_stock=true"));let issue=instrument_policy::check(&def(),ticker,false).unwrap();assert_eq!(issue.status,"needs_data");let ready=readiness_json(&st,&q).unwrap();assert!(ready.contains("\"status\":\"needs_data\""));assert!(ready.contains(&esc(&issue.note)));let expected=format!("[{{\"id\":\"{ID}\",\"blocked\":\"{}\",\"readiness\":\"needs_data\"}}]",esc(&issue.note));assert_eq!(summary(&st,&q).unwrap(),expected);for result in [detail(&st,&q),optimize(&st,&q)]{assert_eq!(result.unwrap_err(),format!("needs_data: {}",issue.note));}}
}
#[test]
fn tiar_causal_history_rejects_invalid_identity_dates_ohlc_and_parameters(){
    let r=rows(&(1..=20).map(|i|i as f64).collect::<Vec<_>>(),"AAPL");
    for kind in 0..7{let mut bad=r.clone();match kind{0=>bad[2].open=0.,1=>bad[2].close=f64::NAN,2=>bad[2].high=bad[2].low-1.,3=>bad[2].date=bad[1].date.clone(),4=>bad[2].date="2000-02-31".into(),5=>bad[2].ticker="OTHER".into(),_=>bad[2].ticker.clear()};let x=Ctx::new(&bad,&bad,&[]);assert!(books::lkqts::stock_research_history_issue(ID,&x,15,19).is_some());assert!(books::lkqts::run(ID,&x,15,19,&Params::new()).is_none());}
    let x=Ctx::new(&r,&r,&[]);for (name,value) in [("step",4.),("step",5.5),("step",f64::NAN),("unknown",5.)]{let p=Params::from([(name.into(),value)]);assert!(books::lkqts::run(ID,&x,15,19,&p).is_none());}
}
