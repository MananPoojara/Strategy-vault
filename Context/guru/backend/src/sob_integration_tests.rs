//! SOB integration tests use synthetic observations only; no market files or external services.
use super::*;
const ID:&str="lkqts_sob";
fn rows(c:&[f64],symbol:&str)->Vec<Row>{
    c.iter().enumerate().map(|(i,&close)|Row{ticker:symbol.into(),date:format!("2000-{:02}-{:02}",i/28+1,i%28+1),open:close,high:close*1.01,low:close*0.99,close,volume:Some(1.)}).collect()
}
fn empty()->State {State{ctxs:BTreeMap::new(),baskets:baskets::Catalogue::default(),vix:vec![],market_extra:BTreeMap::new()}}
fn owned(raw:&str)->BTreeMap<String,String>{owned_query(&query(&format!("/api/guru/connors/run?{raw}")).1,"connors")}
fn sob_def()->connors::StrategyDef{books::lkqts::catalogue().into_iter().find(|d|d.id==ID).unwrap()}

#[test]
fn exact_sob_profile_dispatch_does_not_expand_the_prior_seven_lists_or_other_books(){
    assert_eq!(books::lkqts::STOCK_CANDIDATE_IDS,&["lkqts_cb4020","lkqts_ma1040","lkqts_mom80"]);
    assert_eq!(books::lkqts::NEXT_CANDIDATE_IDS,&["lkqts_volbo","lkqts_rsi14","lkqts_macd","lkqts_macd_literal_alpha"]);
    assert!(books::lkqts::stock_research_profile(ID));assert!(books::lkqts::sob_candidate(ID));
    for id in ["lkqts_mom20","lkqts_sob_inclusive","lkqts_sob2","lkqts_macdhr","sob"]{assert!(!books::lkqts::stock_research_profile(id));}
    let d=sob_def();assert!(matches!(d.status,Status::Live));assert!(instrument_policy::reviewed_kestner_stock(&d));
    let p=instrument_policy::policy(&d);assert_eq!(p.allowed_types,&["index","etf","individual_stock"]);assert_eq!(p.execution_scope,"single_symbol_stock_price_research");
    let metadata=instrument_policy::metadata_json(&d);assert!(metadata.contains(books::lkqts::SOB_PROPOSED_PROFILE));assert!(metadata.contains("\"source_performance_reproduced\":false"));
    assert!(metadata.contains("\"normalized_account_model\":\"sequential_full_allocation_price_returns\""));assert!(metadata.contains("Intel"));
    assert!(instrument_policy::check_native(&d,"AAPL",&baskets::NativeEvidence::us_test_fixture()).is_none());
    for ticker in ["NIFTY 50","SPX","SPY","QQQ"] {assert!(instrument_policy::check(&d,ticker,false).is_none());}
    assert!(instrument_policy::check(&d,"AAPL",false).is_some());
    let mut wrong=d;wrong.book="other";assert!(!instrument_policy::reviewed_kestner_stock(&wrong));assert_eq!(instrument_policy::policy(&wrong).requirement,"source_review_required");
    assert_eq!(crate::gate::get(ID).unwrap().verdict,"unverified");assert_eq!(crate::gate::get(ID).unwrap().ours,"");
}

#[test]
fn all_four_handlers_fail_raw_bad_settings_before_unknown_ticker_resolution(){
    let base="id=lkqts_sob&ticker=missing";
    for tail in ["p.search=79","p.chan=40.5","p.min=20.000000000000000001","p.search=NaN","p.search=inf","p.search=1e-400","p.search=garbage","p.search=","p.unknown=80","g.search=79:80:1","g.chan=40:40:0","g.min=20:20:inf","g.search=80:80:1e-400","g.unknown=1","p.profile=1","profile=wrong","source_profile=wrong","source_convention=wrong","execution_scope=portfolio"]{
        let q=owned(&format!("{base}&{tail}"));assert!(checked_params_of(&q).is_err(),"{tail}");
        for response in [detail(&empty(),&q),readiness_json(&empty(),&q),summary(&empty(),&q),optimize(&empty(),&q)] {let note=response.unwrap_err();assert!(!note.starts_with("unknown ticker"),"settings discarded: {tail}");}
    }
    for raw in ["id=lkqts_sob&id=lkqts_sob","id=lkqts_sob&id=f1a","id=f1a&id=lkqts_sob","id=lkqts_sob&p.search=bad&p.search=80","id=lkqts_sob&g.search=bad&g.search=80","id=lkqts_sob&p.search","id=lkqts_sob&cost","id=lkqts_sob&p.%73earch=80","%69d=lkqts_sob","id=lkqts_sob&p.search&p.__raw_query_error=clear"]{
        let q=owned(raw);assert!(checked_params_of(&q).is_err(),"{raw}");
        assert!(summary(&empty(),&q).unwrap_err().contains("Kestner"));
    }
    for raw in ["id=lkqts_sob","id=lkqts_sob&p.search=80&p.chan=40&p.min=20","id=lkqts_sob&p.search=8e1&p.chan=400e-1&p.min=2e1","id=lkqts_sob&g.search=80&g.chan=40:40:0.01&g.min=20:20:1","id=lkqts_%73ob&p.search=%38%30&p.chan=%34%30&p.min=%32%30"] {assert!(checked_params_of(&owned(raw)).is_ok(),"{raw}");}
    for d in books::lkqts::params(ID).unwrap(){assert_eq!((d.def,d.min,d.max),(d.def,d.def,d.def));}
    // The new override guard is scoped to SOB. No prior-profile parsing change.
    assert!(checked_params_of(&owned("id=lkqts_mom80&profile=legacy_ignored")).is_ok());
    let legacy=query("/legacy?id=f1a&p.fast=bad&p.fast=12&flag").1;assert_eq!(legacy.get("p.fast").unwrap(),"12");assert!(!legacy.contains_key("p.__raw_query_error"));
}

#[test]
fn backend_sob_states_distinguish_statistic_warmup_channel_warmup_and_next_open(){
    let c:Vec<_>=(1..=120).map(|v|v as f64).collect();let r=rows(&c,"AAPL");let x=Ctx::new(&r,&r,&[]);
    let d=diagnostic_series_through(&x,ID,&Params::new(),119);assert!(d.kestner_next.is_none());
    assert_eq!(kestner_sob_rule_state(78,&d),"null");
    let initial=kestner_sob_rule_state(79,&d);assert!(initial.contains("\"statistic\":80"));assert!(initial.contains("\"upper_40\":null,\"lower_40\":null,\"predicate_ready\":false,\"long_condition\":false"));
    assert!(kestner_sob_rule_state(117,&d).contains("\"predicate_ready\":false"));
    let first=kestner_sob_rule_state(118,&d);assert!(first.contains("\"upper_40\":80,\"lower_40\":80,\"predicate_ready\":true,\"long_condition\":true"));
    let run=backtest_costs(&x,ID,117,119,accounting::Costs::default(),&Params::new()).unwrap();assert_eq!(run.trades.len(),1);assert_eq!(run.trades[0].legs[0].i,119);validate_sob_execution_clocks(&x,&run,&d).unwrap();
    let clock=kestner_execution_clock(&x,ID,&run.trades[0],&d);assert!(clock.contains(&format!("\"entry_signal\":{{\"date\":\"{}\",\"phase\":\"completed_close\"",r[118].date)));assert!(clock.contains("\"entry_fill\":"));assert!(clock.contains("\"exit_signal\":null,\"exit_fill\":null"));assert!(clock.contains("\"exit_filled\":false"));
    let mut poisoned=r.clone();poisoned.push(Row{ticker:"BROKEN".into(),date:"bad".into(),open:f64::NAN,high:0.,low:0.,close:0.,volume:None});let y=Ctx::new(&poisoned,&poisoned,&[]);
    let prior=diagnostic_series_through(&y,ID,&Params::new(),119);assert_eq!(kestner_sob_rule_state(118,&prior),first);assert!(books::lkqts::stock_research_history_issue(ID,&y,117,120).is_some());
    let mut changed=r.clone();changed[119].close=1000.;changed[119].high=1010.;let y=Ctx::new(&changed,&changed,&[]);let changed=diagnostic_series_through(&y,ID,&Params::new(),119);assert_eq!(kestner_sob_rule_state(118,&changed),first);
}

#[test]
fn sob_causal_clock_validation_refuses_fabricated_fill_state_weight_and_exit(){
    let mut c:Vec<_>=(1..=120).map(|v|v as f64).collect();c.extend([0.5,0.6,0.6]);let r=rows(&c,"AAPL");let x=Ctx::new(&r,&r,&[]);let d=diagnostic_series_through(&x,ID,&Params::new(),122);
    let mut run=backtest_costs(&x,ID,118,122,accounting::Costs::default(),&Params::new()).unwrap();assert_eq!(run.trades.len(),2);assert!(!run.trades[0].open);assert!(run.trades[1].open);validate_sob_execution_clocks(&x,&run,&d).unwrap();
    let closed=kestner_execution_clock(&x,ID,&run.trades[0],&d);assert!(closed.contains(&format!("\"exit_signal\":{{\"date\":\"{}\"",r[120].date)));assert!(closed.contains(&format!("\"exit_fill\":{{\"date\":\"{}\"",r[121].date)));assert!(closed.contains("\"terminal_mark\":null"));
    for kind in 0..5 {let original=run.trades.clone();match kind{0=>run.trades[0].legs[0].px+=1.,1=>run.trades[0].legs[0].w=0.5,2=>run.trades[0].signal_i=119,3=>run.trades[0].exit_px+=1.,_=>run.trades[1].open=false};assert!(validate_sob_execution_clocks(&x,&run,&d).is_err());run.trades=original;}
}

#[test]
fn source_readiness_and_account_domain_remain_separate_and_basket_is_held(){
    let mut c:Vec<_>=(0..131).map(|i|300.-i as f64).collect();c[129]=1000.;c[130]=1000.;let r=rows(&c,"AAPL");let x=Ctx::new(&r,&r,&[]);let def=sob_def();
    assert!(readiness::check(&x,&def,118,130,&Params::new()).is_none());
    let raw=books::lkqts::run(ID,&x,118,130,&Params::new()).unwrap();assert_eq!(raw.len(),2);
    let ruined=backtest_costs(&x,ID,118,130,accounting::Costs::default(),&Params::new()).unwrap();assert_ruined(&x,&ruined);assert!(raw.starts_with(&ruined.trades));
    let q=owned("id=lkqts_sob&basket=nifty50&max_positions=10&membership=current");let body=basket_portfolio::run(&empty(),&q).unwrap();assert!(body.contains("Kestner"));assert!(body.contains("\"status\":\"not_testable\""));assert!(body.contains("\"stats\":null,\"trades\":[],\"dates\":[],\"equity\":[]"));
}

#[test]
fn loaded_synthetic_index_etf_public_handlers_return_exact_stock_policy_holds(){
    // Bounded synthetic observations test loaded-context routing only. They are
    // never presented as actual SPX/SPY prices or source performance evidence.
    // SPX/SPY are admitted since 22 Oct 2026; a loaded unqualified stock ticker still cannot forge native identity.
    for ticker in ["AAPL"] {
        let r=Box::leak(rows(&vec![100.;131],ticker).into_boxed_slice());let x=Ctx::new(r,r,&[]);
        let mut st=empty();st.ctxs.insert(ticker.into(),x);
        let q=owned(&format!("id=lkqts_sob&ticker={ticker}&from=2000-01-01&to=2000-05-19&asset_type=individual_stock&native_stock=true"));
        let exact=instrument_policy::check(&sob_def(),ticker,false).unwrap();assert_eq!(exact.status,"needs_data");
        let ready=readiness_json(&st,&q).unwrap();assert!(ready.contains("\"status\":\"needs_data\""));assert!(ready.contains(&esc(&exact.note)));assert!(!ready.contains("\"stats\""));
        let expected=format!("[{{\"id\":\"lkqts_sob\",\"blocked\":\"{}\",\"readiness\":\"needs_data\"}}]",esc(&exact.note));assert_eq!(summary(&st,&q).unwrap(),expected);
        for result in [detail(&st,&q),optimize(&st,&q)]{assert_eq!(result.unwrap_err(),format!("needs_data: {}",exact.note));}
    }
}
