//! Bounded source/transport checks. Synthetic fixtures are not market results.
use super::*;
const ID:&str="hbqts_dema16_8";
fn state()->State {
    let rows:&'static [Row]=Box::leak([100.,99.,101.,101.].iter().enumerate().map(|(i,&c)|Row{ticker:"SPX".into(),date:format!("2001-01-{:02}",i+1),open:c,high:c+1.,low:c-1.,close:c,volume:None}).collect::<Vec<_>>().into_boxed_slice());
    State{ctxs:BTreeMap::from([("SPX".into(),Ctx::new(rows,rows,&[]))]),baskets:Default::default(),vix:vec![],market_extra:Default::default()}
}
#[test]
fn dema_raw_settings_require_exact_singleton_source_defaults() {
    let base=BTreeMap::from([("id".into(),ID.into())]);
    assert!(checked_params_of(&base).is_ok());
    for (key,value) in [("p.fast","16.00000000000000001"),("p.slow","8.00000000000000001"),("p.hold","1e-400"),("p.unknown","0"),("p.fast","NaN"),("p.fast","garbage"),("g.fast","16:17:100"),("g.slow","8:8:inf"),("g.hold","1:1:0")] {
        let mut q=base.clone();q.insert(key.into(),value.into());assert!(checked_params_of(&q).is_err(),"{key}={value}");
    }
    for (key,value) in [("p.fast","1.6e1"),("p.slow","8.00"),("g.fast","16:16:1e-300"),("g.hold","1")] {
        let mut q=base.clone();q.insert(key.into(),value.into());assert!(checked_params_of(&q).is_ok(),"{key}={value}");
    }
}
#[test]
fn dema_cash_index_scope_is_exact_without_stock_or_etf_inheritance() {
    let d=books::hbqts::catalogue().into_iter().find(|d|d.id==ID).unwrap();
    assert!(!instrument_policy::reviewed_cash_rule(&d));assert!(instrument_policy::reviewed_dema_index(&d));
    assert_eq!(instrument_policy::policy(&d).allowed_types,&["index"]);
    for t in ["SPX","NIFTY 50"] {assert!(instrument_policy::check(&d,t,false).is_none());}
    for t in ["SPY","QQQ","AAPL","RELIANCE","NDX","DJI","NIFTY BANK"] {assert!(instrument_policy::check(&d,t,false).is_some(),"{t}");}
    assert!(instrument_policy::check_native(&d,"AAPL",&baskets::NativeEvidence::us_test_fixture()).is_some());
    assert!(d.notes.contains("S&P500"));assert!(!d.notes.contains("Actual individual stocks only"));
    let mut other=d;other.book="other";assert!(!instrument_policy::reviewed_dema_index(&other));
    assert_eq!(instrument_policy::policy(&other).requirement,"source_review_required");
    let g=gate::get(ID).unwrap();assert_eq!(g.verdict,"unverified");assert!(g.reason.contains("printed324"));
}
#[test]
fn dema_single_session_detail_and_public_summary_share_range_contract() {
    let st=state();
    let q=BTreeMap::from([("id".into(),ID.into()),("ticker".into(),"SPX".into()),("from".into(),"2001-01-02".into()),("to".into(),"2001-01-02".into()),("cost".into(),"3".into()),("slippage".into(),"2".into())]);
    let body=detail(&st,&q).unwrap();assert!(body.contains("\"open\":true"));assert!(body.contains("open_liquidation_estimate"));
    let mut public=q.clone();public.remove("id");let public=owned_query(&public,"connors");
    let all=summary(&st,&public).unwrap();let row=all.split(&format!("{{\"id\":\"{ID}\"")).nth(1).unwrap().split("},{\"id\":").next().unwrap();
    assert!(row.contains("\"stats\":"));
    let summary_stats=row.split("\"stats\":").nth(1).unwrap().split(",\"spark\":").next().unwrap();
    let detail_stats=body.split("\"stats\":").nth(1).unwrap().split(",\"gross_stats\":").next().unwrap();assert_eq!(summary_stats,detail_stats);
    let mut legacy=q.clone();legacy.insert("id".into(),"hbqts_ma11_50".into());assert_eq!(detail(&st,&legacy).unwrap_err(),"empty date range");
    let mut bad=q.clone();bad.insert("to".into(),"2000-12-31".into());assert!(detail(&st,&bad).is_err());
}
#[test]
fn dema_full_prefix_validation_is_causal_and_does_not_reseed() {
    let st=state();let ctx=&st.ctxs["SPX"];let d=books::hbqts::catalogue().into_iter().find(|d|d.id==ID).unwrap();
    assert!(readiness::check(ctx,&d,0,0,&Params::new()).is_none());
    let mut rows=ctx.r.to_vec();rows[3].close=f64::NAN;let ctx=Ctx::new(&rows,&rows,&[]);
    assert!(readiness::check(&ctx,&d,0,2,&Params::new()).is_none());assert!(readiness::check(&ctx,&d,0,3,&Params::new()).is_some());
    rows[0].close=f64::NAN;let ctx=Ctx::new(&rows,&rows,&[]);assert!(readiness::check(&ctx,&d,1,2,&Params::new()).is_some());
}

/// Private frozen-case adapter. No production route, provider identity or data mutation.
#[test]
fn dump_case() {
    let Ok(csv)=std::env::var("DEMA_CASE_CSV") else{return;};
    let id=std::env::var("DEMA_CASE_ID").unwrap();assert_eq!(id,ID);
    let ticker=std::env::var("DEMA_CASE_TICKER").unwrap();assert!(matches!(ticker.as_str(),"SPX"|"NIFTY 50"));
    let all=load_csv(&csv).unwrap();let rows:&'static [Row]=Box::leak(all.get(&ticker).unwrap().clone().into_boxed_slice());
    let st=State{ctxs:BTreeMap::from([(ticker.clone(),Ctx::new(rows,rows,&[]))]),baskets:Default::default(),vix:vec![],market_extra:Default::default()};
    let x=&st.ctxs[&ticker];let from=std::env::var("DEMA_CASE_FROM").unwrap().parse::<usize>().unwrap();let to=std::env::var("DEMA_CASE_TO").unwrap().parse::<usize>().unwrap();
    let q=BTreeMap::from([("id".into(),id.clone()),("ticker".into(),ticker),("from".into(),rows[from].date.clone()),("to".into(),rows[to].date.clone()),("cost".into(),std::env::var("DEMA_CASE_FEES").unwrap_or("0".into())),("slippage".into(),std::env::var("DEMA_CASE_SLIPPAGE").unwrap_or("0".into()))]);
    let costs=accounting::Costs::parse(&q).unwrap();let run=backtest_costs(x,&id,from,to,costs,&Params::new()).unwrap();
    let payload=detail(&st,&q).unwrap();let s=books::hbqts::reviewed::dema_index_states(x,to).unwrap();
    let fp=full_precision_trade_return;
    let indicators=(0..=to).map(|i|format!("{{\"source_index\":{i},\"date\":\"{}\",\"close\":{},\"dema16\":{},\"dema8\":{},\"raw_buy\":{},\"reverse_cross\":{},\"raw_bars_since_buy\":{},\"raw_sell\":{},\"filtered_sell\":{}}}",rows[i].date,fp(x.c[i]),fp(s.dema16[i]),fp(s.dema8[i]),s.raw_buy[i],s.reverse_cross[i],s.raw_bars_since_buy[i].map_or("null".into(),|v|v.to_string()),s.raw_sell[i],s.filtered_sell[i])).collect::<Vec<_>>().join(",");
    let array=|v:&[f64]|v.iter().copied().map(fp).collect::<Vec<_>>().join(",");
    let report=run.unit_exposure_report.as_ref().unwrap();
    let trades=run.trades.iter().zip(&report.trade_returns).map(|(t,a)|{
        format!("{{\"entry_i\":{},\"exit_i\":{},\"signal_i\":{},\"entry_px\":{},\"exit_px\":{},\"weight\":{},\"open\":{},\"gross_ret\":{},\"net_ret\":{},\"fee_ret\":{},\"slippage_ret\":{},\"entry_allocation\":{},\"gross_contribution\":{},\"net_contribution\":{},\"fee_contribution\":{},\"slippage_contribution\":{}}}",t.legs[0].i,t.exit_i,t.signal_i,fp(t.legs[0].px),fp(t.exit_px),fp(t.legs[0].w),t.open,fp(a.gross_return),fp(a.net_return),fp(a.fee_return),fp(a.slippage_return),fp(a.entry_allocation),fp(a.gross_contribution),fp(a.net_contribution),fp(a.fee_contribution),fp(a.slippage_contribution))
    }).collect::<Vec<_>>().join(",");
    let gross=report.days.iter().map(|d|d.gross_equity).collect::<Vec<_>>();
    let raw_stats=|s:BTreeMap<&'static str,f64>|format!("{{{}}}",s.iter().map(|(k,v)|format!("\"{k}\":{}",fp(*v))).collect::<Vec<_>>().join(","));
    let result=format!("{{\"payload\":{payload},\"indicators\":[{indicators}],\"raw\":{{\"from\":{from},\"to\":{to},\"equity\":[{}],\"gross_equity\":[{}],\"bh\":[{}],\"trades\":[{trades}],\"stats\":{},\"gross_stats\":{},\"benchmark_stats\":{}}}}}",array(&run.equity),array(&gross),array(&run.bh),raw_stats(report.financial_stats(x,&run.trades,from,to)),raw_stats(report.common_stats(x,&run.trades,from,to,unit_exposure_report::View::Gross)),raw_stats(report.benchmark_stats(x,from,to)));
    std::fs::write(std::env::var("DEMA_CASE_OUTPUT").unwrap(),result).unwrap();
}
