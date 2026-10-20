//! Synthetic request/clock evidence for the next Kestner profiles. No market data.
use super::*;

fn observations(closes:&[f64])->Vec<Row> {
    closes.iter().enumerate().map(|(i,&c)|Row{ticker:"AAPL".into(),date:format!("2020-{:02}-{:02}",i/28+1,i%28+1),open:c,high:c+1.,low:c-1.,close:c,volume:Some(1.)}).collect()
}
fn empty_state()->State {State{ctxs:BTreeMap::new(),baskets:baskets::Catalogue::default(),vix:vec![],market_extra:BTreeMap::new()}}
fn value(state:&books::lkqts::NextRuleState,key:&str)->f64 {
    state.values.iter().find(|(k,_)|*k==key).unwrap().1
}

#[test]
fn all_seven_profiles_keep_exact_typed_and_raw_settings() {
    assert_eq!(books::lkqts::STOCK_CANDIDATE_IDS.len(),3);
    for &id in books::lkqts::STOCK_CANDIDATE_IDS.iter().chain(books::lkqts::NEXT_CANDIDATE_IDS) {
        assert!(books::lkqts::stock_research_profile(id));
        let base=BTreeMap::from([("id".into(),id.into())]);
        assert!(checked_params_of(&base).is_ok());
        for s in books::lkqts::params(id).unwrap() {
            for prefix in ["p.","g."] {
                let mut q=base.clone();let key=format!("{prefix}{}",s.key);
                for bad in ["NaN".into(),"inf".into(),"garbage".into(),"".into(),"1e-400".into(),format!("{}.00000000000000000001",s.def as u64)] {
                    q.insert(key.clone(),bad);assert!(checked_params_of(&q).is_err());
                    assert!(summary(&empty_state(),&q).unwrap_err().contains("Kestner"));
                }
                q.insert(key,format!("{}e-1",s.def*10.));assert!(checked_params_of(&q).is_ok());
            }
            for bad in [format!("{0}:{0}:0",s.def),format!("{0}:{0}:inf",s.def),format!("{}:{}:1",s.def,s.def+1.)] {
                let mut q=base.clone();q.insert(format!("g.{}",s.key),bad);assert!(checked_params_of(&q).is_err());
            }
        }
        for key in ["p.fill","p.unknown","g.unknown","p.source_convention"] {
            let mut q=base.clone();q.insert(key.into(),"0".into());assert!(checked_params_of(&q).is_err());
        }
    }
    assert!(!books::lkqts::stock_research_profile("lkqts_mom20"));
}

#[test]
fn raw_query_duplicates_and_bare_fields_cannot_be_collapsed_into_defaults() {
    for raw in [
        "id=lkqts_macd&id=lkqts_macd", "id=lkqts_macd&id=f1a", "id=f1a&id=lkqts_macd",
        "id=lkqts_macd&p.fast=bad&p.fast=12", "id=lkqts_macd&g.fast=bad&g.fast=12",
        "id=lkqts_macd&from=bad&from=2020-01-01", "id=lkqts_macd&p.fast",
        "id=lkqts_macd&cost", "id=lkqts_macd&p.%66ast=12",
        "id=lkqts_macd&p.fast&p.__raw_query_error=clear",
    ] {
        let (_,q)=query(&format!("/api/guru/connors/run?{raw}"));
        assert!(checked_params_of(&q).is_err(),"duplicate/bare query must be rejected");
        assert!(summary(&empty_state(),&q).unwrap_err().contains("Kestner"));
    }
    let (_,q)=query("/api/guru/connors/run?id=lkqts_macd&p.fast=120e-1&p.slow=2.6e1&g.sig=9:9:0.1");
    assert!(checked_params_of(&q).is_ok());
    let (_,legacy)=query("/legacy?id=f1a&p.fast=bad&p.fast=12&flag");
    assert_eq!(legacy.get("p.fast").unwrap(),"12");assert!(!legacy.contains_key("p.__raw_query_error"));
}

#[test]
fn history_dispatch_preserves_old_eligibility_without_inventing_macd_cutoff() {
    let r=observations(&vec![100.;110]);let ctx=Ctx::new(&r,&r,&[]);
    for (&id,first) in books::lkqts::STOCK_CANDIDATE_IDS.iter().zip([39,40,80]) {
        assert!(books::lkqts::stock_research_history_issue(id,&ctx,first-1,109).is_some());
        assert!(books::lkqts::stock_research_history_issue(id,&ctx,first,109).is_none());
    }
    for &id in books::lkqts::NEXT_CANDIDATE_IDS {
        assert!(books::lkqts::stock_research_history_issue(id,&ctx,0,109).is_none());
        for kind in 0..4 {
            let mut bad=r.clone();match kind {0=>bad[0].date="bad".into(),1=>bad[1].date=bad[0].date.clone(),2=>bad[0].close=f64::NAN,_=>bad[0].ticker="OTHER".into()}
            let ctx=Ctx::new(&bad,&bad,&[]);assert!(books::lkqts::stock_research_history_issue(id,&ctx,100,109).is_some());
        }
    }
}

#[test]
fn next_diagnostics_expose_actual_predicates_and_named_profiles() {
    let mut c=vec![100.;100];c.extend([113.,92.54,92.54]);
    let r=observations(&c);let ctx=Ctx::new(&r,&r,&[]);
    for (id,profile) in [("lkqts_macd","macd_period_alpha_first_close"),("lkqts_macd_literal_alpha","macd_literal_alpha_first_close")] {
        let ds=diagnostic_series_through(&ctx,id,&Params::new(),102);
        let d=ds.kestner_next.as_ref().unwrap();assert_eq!(d.profile,profile);assert_eq!(d.states.len(),103);
        assert!(d.states[100].as_ref().unwrap().long_condition);
        assert_eq!(d.states[101].as_ref().unwrap().short_condition,id=="lkqts_macd");
        assert!(value(d.states[100].as_ref().unwrap(),"histogram")>0.);
        assert!(value(d.states[0].as_ref().unwrap(),"previous_histogram").is_nan());
    }
    let mut c:Vec<_>=(0..15).map(|i|100.-i as f64).collect();c.extend([85.,200.,200.]);
    let r=observations(&c);let ctx=Ctx::new(&r,&r,&[]);
    let ds=diagnostic_series_through(&ctx,"lkqts_rsi14",&Params::new(),17);
    let d=ds.kestner_next.as_ref().unwrap();assert!(d.states[13].is_none());
    assert!(d.states[14].as_ref().unwrap().long_condition);assert_eq!(value(d.states[14].as_ref().unwrap(),"rsi_14"),0.);
    assert!(d.states[16].as_ref().unwrap().short_condition);
    let mut c=vec![1000.;100];c.extend([1010.,1005.,1005.,1005.]);
    let r=observations(&c);let ctx=Ctx::new(&r,&r,&[]);
    let ds=diagnostic_series_through(&ctx,"lkqts_volbo",&Params::new(),103);
    let d=ds.kestner_next.as_ref().unwrap();assert!(d.states[99].is_none());
    let state=d.states[100].as_ref().unwrap();assert_eq!(value(state,"delta"),10.);assert_eq!(value(state,"delta_squared"),100.);assert!(state.long_condition);
}

#[test]
fn causal_states_use_prefix_once_and_ignore_poisoned_future_observations() {
    let mut c=vec![100.;100];c.extend([113.,92.54,92.54,92.54,92.54]);
    let r=observations(&c);let mut poisoned=r.clone();poisoned[104].close=f64::NAN;poisoned[104].date="invalid_future".into();
    let a=Ctx::new(&r,&r,&[]);let b=Ctx::new(&poisoned,&poisoned,&[]);
    for &id in books::lkqts::NEXT_CANDIDATE_IDS {
        let da=diagnostic_series_through(&a,id,&Params::new(),103);let db=diagnostic_series_through(&b,id,&Params::new(),103);
        assert_eq!(da.kestner_next.as_ref().unwrap().states.len(),104);
        for i in 0..=103 {assert_eq!(kestner_next_rule_state(i,&da),kestner_next_rule_state(i,&db));}
        let trades=books::lkqts::run(id,&a,100,103,&Params::new()).unwrap();
        for t in trades {assert_eq!(kestner_execution_clock(&a,id,&t,&da),kestner_execution_clock(&b,id,&t,&db));}
    }
}

#[test]
fn entry_rule_state_cannot_see_the_fill_sessions_future_close() {
    let mut c=vec![100.;100];c.extend([113.,92.54,92.54,92.54]);
    let mut r=observations(&c);r[101].open=115.;r[101].high=116.;
    let mut changed=r.clone();changed[101].close=250.;changed[101].high=251.;
    let a=Ctx::new(&r,&r,&[]);let b=Ctx::new(&changed,&changed,&[]);
    for &id in books::lkqts::NEXT_CANDIDATE_IDS {
        let trades=books::lkqts::run(id,&a,100,103,&Params::new()).unwrap();
        let t=trades.first().unwrap();assert_eq!(t.legs[0].i,101);
        let da=diagnostic_series_through(&a,id,&Params::new(),103);let db=diagnostic_series_through(&b,id,&Params::new(),103);
        let original=kestner_execution_clock(&a,id,t,&da);let changed=kestner_execution_clock(&b,id,t,&db);
        assert_eq!(original.split(",\"entry_fill\"").next(),changed.split(",\"entry_fill\"").next());
    }
}

#[test]
fn clocks_keep_next_open_reversal_and_terminal_marks_distinct() {
    let mut c=vec![100.;100];c.extend([113.,92.54,92.54,92.54]);
    let mut r=observations(&c);r[101].open=115.;r[101].high=116.;r[102].open=90.;r[102].low=89.;
    let ctx=Ctx::new(&r,&r,&[]);let ds=diagnostic_series_through(&ctx,"lkqts_macd",&Params::new(),103);
    let trades=books::lkqts::run("lkqts_macd",&ctx,100,103,&Params::new()).unwrap();
    let closed=kestner_execution_clock(&ctx,"lkqts_macd",&trades[0],&ds);
    assert!(closed.contains("\"profile\":\"macd_period_alpha_first_close\""));
    assert!(closed.contains("\"phase\":\"open\",\"price\":115"));assert!(closed.contains("\"phase\":\"open\",\"price\":90"));
    let open=kestner_execution_clock(&ctx,"lkqts_macd",&trades[1],&ds);
    assert!(open.contains("\"exit_signal\":null,\"exit_fill\":null"));assert!(open.contains("\"exit_filled\":false"));
    assert!(open.contains(&format!("\"entry_signal\":{{\"date\":\"{}\"",r[101].date)));
    assert!(open.contains(&format!("\"terminal_mark\":{{\"date\":\"{}\"",r[103].date)));
    let old=diagnostic_series_through(&ctx,"lkqts_mom80",&Params::new(),103);assert!(old.kestner_next.is_none());
}

#[test]
fn rsi_source_events_do_not_bypass_nonpositive_normalized_accounts() {
    let mut c:Vec<_>=(0..16).map(|i|100.+i as f64).collect();c.extend([400.,400.]);
    let r=observations(&c);let ctx=Ctx::new(&r,&r,&[]);
    let raw=books::lkqts::run("lkqts_rsi14",&ctx,14,17,&Params::new()).unwrap();assert_eq!(raw.len(),1);assert_eq!(raw[0].dir,-1.);
    assert!(backtest_costs(&ctx,"lkqts_rsi14",14,17,accounting::Costs::default(),&Params::new()).is_err());
}

#[test]
fn unsupported_indicator_domains_are_held_without_hiding_genuine_zero_dispersion() {
    let tiny_rows=|c:&[f64]|observations(c).into_iter().map(|mut r|{r.low=r.close*0.99;r.high=r.close*1.01;r}).collect::<Vec<_>>();
    let mut c=vec![1e-198;102];c[1..].fill(1.000001e-198);
    let r=tiny_rows(&c);let ctx=Ctx::new(&r,&r,&[]);
    assert!(books::lkqts::stock_research_history_issue("lkqts_volbo",&ctx,100,101).is_none());
    assert!(books::lkqts::next_diagnostics("lkqts_volbo",&ctx,101,&Params::new()).is_none());
    assert!(books::lkqts::run("lkqts_volbo",&ctx,100,101,&Params::new()).is_none());
    let c:Vec<_>=(0..102).map(|i|1e-198+i as f64*1e-200).collect();
    let r=tiny_rows(&c);let ctx=Ctx::new(&r,&r,&[]);
    assert!(books::lkqts::next_diagnostics("lkqts_volbo",&ctx,101,&Params::new()).is_none());
    let r=tiny_rows(&vec![1e-198;102]);let ctx=Ctx::new(&r,&r,&[]);
    let d=books::lkqts::next_diagnostics("lkqts_volbo",&ctx,101,&Params::new()).unwrap();
    assert_eq!(value(d.states[100].as_ref().unwrap(),"population_variance_100"),0.);
    assert!(!d.states[100].as_ref().unwrap().long_condition);
    let c:Vec<_>=(0..16).map(|i|1e307+i as f64*9e306).collect();
    let r=tiny_rows(&c);let ctx=Ctx::new(&r,&r,&[]);
    let d=books::lkqts::next_diagnostics("lkqts_rsi14",&ctx,15,&Params::new()).unwrap();
    assert_eq!(value(d.states[14].as_ref().unwrap(),"rsi_14"),100.);
    assert!(d.states[14].as_ref().unwrap().short_condition);
    let c:Vec<_>=(0..16).map(|i|if i%2==0 {2.} else {1e308}).collect();
    let r=observations(&c);let ctx=Ctx::new(&r,&r,&[]);
    assert!(books::lkqts::next_diagnostics("lkqts_rsi14",&ctx,15,&Params::new()).is_none());
}

#[test]
fn rsi_nonzero_seed_or_new_move_underflow_is_not_published_as_flat_fifty() {
    let m=f64::from_bits(1);
    let exact_rows=|closes:&[f64]|observations(closes).into_iter().map(|mut r|{r.open=r.close;r.high=r.close;r.low=r.close;r}).collect::<Vec<_>>();
    for gain in [true,false] {
        let mut seed=vec![if gain {3.*m}else{2.*m};17];seed[0]=if gain {2.*m}else{3.*m};
        let mut recurrence=vec![2.*m;17];recurrence[15..].fill(if gain {3.*m}else{m});
        for closes in [seed,recurrence] {
            let r=exact_rows(&closes);let ctx=Ctx::new(&r,&r,&[]);
            assert!(books::lkqts::stock_research_history_issue("lkqts_rsi14",&ctx,14,16).is_none());
            assert!(books::lkqts::next_diagnostics("lkqts_rsi14",&ctx,16,&Params::new()).is_none());
            assert!(books::lkqts::run("lkqts_rsi14",&ctx,14,16,&Params::new()).is_none());
        }
    }
    let r=exact_rows(&vec![2.*m;17]);let ctx=Ctx::new(&r,&r,&[]);
    let d=books::lkqts::next_diagnostics("lkqts_rsi14",&ctx,16,&Params::new()).unwrap();
    assert_eq!(value(d.states[14].as_ref().unwrap(),"rsi_14"),50.);
    assert!(!d.states[14].as_ref().unwrap().long_condition);assert!(!d.states[14].as_ref().unwrap().short_condition);
    let mut retained:Vec<_>=(2..=16).map(|i|i as f64*m).collect();retained.extend([16.*m,16.*m]);
    let r=exact_rows(&retained);let ctx=Ctx::new(&r,&r,&[]);
    let d=books::lkqts::next_diagnostics("lkqts_rsi14",&ctx,16,&Params::new()).unwrap();
    assert_eq!(value(d.states[14].as_ref().unwrap(),"rma_up_14"),m);
    assert_eq!(value(d.states[15].as_ref().unwrap(),"rma_up_14"),m);
    assert_eq!(value(d.states[16].as_ref().unwrap(),"rsi_14"),100.);
}

// Private preparation fragment for the existing kestner_next_verification module.
// Root may append after freezing the complete candidate. This is never a startup job.
#[test]
#[ignore = "Needs owned numeric gate, exact frozen request contract, private roots and a fresh private output"]
fn actual_next_four_api_capture() {
    use std::path::{Path,PathBuf};
    fn file_sha(path:&Path)->String {
        let output=std::process::Command::new("sha256sum").arg(path).output().expect("sha256sum unavailable");
        assert!(output.status.success(),"Could not bind captured source/body bytes");
        let value=String::from_utf8(output.stdout).unwrap().split_whitespace().next().unwrap().to_owned();
        assert!(value.len()==64 && value.bytes().all(|b|b.is_ascii_hexdigit()));value
    }
    assert_eq!(std::env::var("VAULT_KESTNER_NEXT_API_GATE_OWNER").as_deref(),Ok("root_frozen_serial_capture"));
    let contract=PathBuf::from(std::env::var("VAULT_KESTNER_NEXT_API_CONTRACT").expect("Frozen contract path required"));
    let expected_contract=std::env::var("VAULT_KESTNER_NEXT_API_CONTRACT_SHA256").expect("Frozen contract SHA required");
    assert_eq!(file_sha(&contract),expected_contract,"Source/response contract changed");
    let request_path=PathBuf::from(std::env::var("VAULT_KESTNER_NEXT_API_REQUESTS").expect("Frozen raw request corpus required"));
    let expected_requests=std::env::var("VAULT_KESTNER_NEXT_API_REQUESTS_SHA256").expect("Frozen request SHA required");
    assert_eq!(file_sha(&request_path),expected_requests,"Raw request corpus changed");
    let output=PathBuf::from(std::env::var("VAULT_KESTNER_NEXT_API_CAPTURE_DIR").expect("Fresh private output required"));
    assert!(output.is_absolute() && !output.exists(),"No relative path or overwrite");
    let parent=output.parent().unwrap().canonicalize().expect("Prepare private parent first");
    assert!(parent.starts_with("/home/server/.cache") || parent.starts_with(std::env::temp_dir()),"Keep captures outside public checkout and original shares");
    let checkout=std::path::Path::new(env!("CARGO_MANIFEST_DIR")).ancestors().nth(3).unwrap().canonicalize().unwrap();
    assert!(!parent.starts_with(checkout),"Generated captures must stay outside the public checkout");
    std::fs::create_dir(&output).unwrap();
    let mut states=BTreeMap::new();
    for (market,variable) in [("us","VAULT_KESTNER_US_DATA_DIR"),("in","VAULT_KESTNER_IN_DATA_DIR")] {
        let root=PathBuf::from(std::env::var(variable).expect("Explicit read-only native whitelist root required"));
        let catalogue=baskets::Catalogue::load(&root).unwrap();
        states.insert(market,State{ctxs:BTreeMap::new(),baskets:catalogue,vix:vec![],market_extra:BTreeMap::new()});
    }
    // Additive private chart-source evidence, inside actual_next_four_api_capture
    // after the two states exist and before the480 strategy-handler requests.
    // These are2source-history captures, not strategy runs or extra admissions.
    let mut chart_history_records=Vec::new();
    for (market,symbol,qualified,raw) in [
        ("us","AAPL","basket:us_sp500:AAPL","/api/ohlc?ticker=basket%3Aus_sp500%3AAAPL"),
        ("in","RELIANCE","basket:nifty50:RELIANCE","/api/ohlc?ticker=basket%3Anifty50%3ARELIANCE"),
    ] {
        let (path,q)=query(raw);assert_eq!(path,"/api/ohlc");
        let body=chart_ohlc_json(states.get(market).unwrap(),&q).expect("Exact qualified chart source must be available");
        let name=format!("chart-history-{symbol}.json");std::fs::write(output.join(&name),body.as_bytes()).unwrap();
        chart_history_records.push(format!("{{\"symbol\":\"{}\",\"market_key\":\"{}\",\"qualified_ticker\":\"{}\",\"raw_path\":\"{}\",\"file\":\"{}\",\"sha256\":\"{}\"}}",symbol,market,qualified,esc(raw),name,file_sha(&output.join(&name))));
    }
    std::fs::write(output.join("CHART-HISTORY-CAPTURE.json"),format!("{{\"complete\":true,\"strategy_request_count_unchanged\":480,\"captures\":[{}]}}",chart_history_records.join(","))).unwrap();

    // Metadata serialization only. This does not backtest the catalogue.
    let catalogue=catalogue_json("connors");let catalogue_file=output.join("catalogue.json");std::fs::write(&catalogue_file,catalogue).unwrap();
    let requests=std::fs::read_to_string(&request_path).unwrap();assert!(requests.len()<1024*1024);
    let mut records=Vec::new();let mut seen=std::collections::BTreeSet::new();
    for line in requests.lines().filter(|line|!line.is_empty()) {
        let columns:Vec<_>=line.split('\t').collect();assert_eq!(columns.len(),5,"Malformed capture instruction");
        let (name,market,endpoint,expected,raw_path)=(columns[0],columns[1],columns[2],columns[3],columns[4]);
        assert!(seen.insert(name.to_owned()),"Duplicate capture identity");
        assert!(name.bytes().all(|b|b.is_ascii_alphanumeric()||b"_-".contains(&b)),"Unsafe output filename");
        assert!(matches!(endpoint,"run"|"readiness"|"summary"|"optimize"));
        let (path,parsed)=query(raw_path);assert_eq!(path,format!("/api/guru/connors/{endpoint}"));
        let q=owned_query(&parsed,"connors");let state=states.get(market).expect("Unapproved market root");
        let started=std::time::Instant::now();
        let response=match endpoint {"run"=>detail(state,&q),"readiness"=>readiness_json(state,&q),"summary"=>summary(state,&q),"optimize"=>optimize(state,&q),_=>unreachable!()};
        let elapsed=started.elapsed().as_millis();
        let (kind,file,note)=match &response {
            Ok(body)=>{let file=format!("{name}.body.json");std::fs::write(output.join(&file),body.as_bytes()).unwrap();("ok_json",file,"".to_owned())},
            Err(note)=>{let file=format!("{name}.error.txt");std::fs::write(output.join(&file),note.as_bytes()).unwrap();("err",file,note.clone())}
        };
        records.push(format!("{{\"request_id\":\"{}\",\"market_key\":\"{}\",\"endpoint\":\"{}\",\"raw_path\":\"{}\",\"expectation\":\"{}\",\"kind\":\"{}\",\"file\":\"{}\",\"sha256\":\"{}\",\"error\":\"{}\",\"elapsed_ms\":{}}}",esc(name),esc(market),endpoint,esc(raw_path),expected,kind,esc(&file),file_sha(&output.join(&file)),esc(&note),elapsed));
        // Save the observed bytes before checking; failures remain inspectable.
        match expected {
            "err"=>assert!(response.is_err(),"{name}: negative raw request unexpectedly produced a result"),
            "err_calculation"=>assert!(matches!(&response,Err(note) if note.contains("calculation_unavailable")),"{name}: account hold lost"),
            "ok_result"=>assert!(response.is_ok(),"{name}: expected account-ready response held"),
            "ok_ready"=>assert!(matches!(&response,Ok(body) if body.contains("\"status\":\"ready\"")),"{name}: expected data-ready input"),
            "ok_needs_data"=>assert!(matches!(&response,Ok(body) if body.contains("\"status\":\"needs_data\"")),"{name}: missing roll-event mapping was not held"),
            "ok_blocked_calculation"=>assert!(matches!(&response,Ok(body) if body.contains("\"readiness\":\"calculation_unavailable\"") && body.contains("\"blocked\"") && !body.contains("\"stats\"") && !body.contains("\"spark\"")),"{name}: summary invented performance for an account hold"),
            _=>panic!("Unknown frozen expectation")
        }
        std::fs::write(output.join("PARTIAL-OBSERVED.json"),format!("{{\"complete\":false,\"responses\":[{}]}}",records.join(","))).unwrap();
    }
    let manifest=format!("{{\"schema_version\":1,\"complete\":true,\"contract_sha256\":\"{}\",\"requests_sha256\":\"{}\",\"catalogue_file\":\"catalogue.json\",\"catalogue_sha256\":\"{}\",\"request_count\":{},\"capture_scope\":\"Production raw query parsing, owner restriction and API handlers/serialization. No network or HTTP status is claimed. Four independent RSI account holds remain errors, not zero performance.\",\"responses\":[{}]}}",expected_contract,expected_requests,file_sha(&catalogue_file),records.len(),records.join(","));
    std::fs::write(output.join("CAPTURE-MANIFEST.json"),manifest).unwrap();
}
