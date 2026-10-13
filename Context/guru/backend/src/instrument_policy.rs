//! Explicit instrument routing. Admission is separate from source-performance certification.
use crate::{connors::StrategyDef, readiness::Issue,baskets::{NativeEvidence,Market}};

#[derive(Clone, Copy, Debug)]
pub struct Policy {
    pub requirement: &'static str,
    pub allowed_types: &'static [&'static str],
    pub preferred_basket: Option<&'static str>,
    pub execution_scope: &'static str,
    pub source_basis: &'static str,
    pub market_adaptation: bool,
    pub source_price_unit: Option<&'static str>,
    pub native_currency_constraint: Option<&'static str>,
}

fn requirement(kind: &'static str, allowed: &'static [&'static str], basis: &'static str) -> Policy {
    Policy {requirement:kind, allowed_types:allowed,
        preferred_basket:if kind=="individual_stock" {Some("nifty50")} else {None},
        execution_scope:if kind=="portfolio" {"portfolio"} else if kind=="source_review_required" {"source_review"} else {"single_symbol"},
        source_basis:basis, market_adaptation:false,source_price_unit:None,native_currency_constraint:None}
}
fn stock(basis: &'static str)->Policy {requirement("individual_stock", &["individual_stock"], basis)}
fn index(basis: &'static str)->Policy {requirement("index", &["index"], basis)}
fn etf(basis: &'static str)->Policy {requirement("etf", &["etf"], basis)}
fn future(basis: &'static str)->Policy {requirement("futures", &[], basis)}
fn portfolio(basis: &'static str)->Policy {requirement("portfolio", &[], basis)}
fn review()->Policy {requirement("source_review_required", &[], "Instrument population and executable rule requirements have not been independently reviewed for this specific source rule. Book titles, keywords, code availability and prior verification badges are not evidence of instrument compatibility.")}
fn member(id:&str,ids:&[&str])->bool {ids.contains(&id)}

/// Reviewed keys include both the book and the exact rule identifier. Adding a
/// method to a familiar book cannot inherit admission without a source review.
pub fn policy(def:&StrategyDef)->Policy {
    let mut p=base_policy(def);
    p.native_currency_constraint=match (def.book,def.id) {
        ("afactor","afactor_baseline")=>Some("Alvarez Factor workbook p.13 filters the previous stock close above USD 10. The runner compares a literal price parameter with the INR close, which is not the same economic filter."),
        ("jc5dm","jc5dm_long")=>Some("Five Day Momentum p.10 requires the US stock price above USD 50 and pp.11-12 use a 1/16 USD stock tick. The runner's literal values do not establish an INR threshold or the actual NSE stock tick."),
        ("jc5dm","jc5dm_short")=>Some("Five Day Momentum p.10 requires the US stock price above USD 40 and pp.11-12 use a 1/16 USD stock tick. The runner's literal values do not establish an INR threshold or the actual NSE stock tick."),
        ("r3r4","r3r4_lowprice"|"r3r4_lowprice_scale")=>Some("R3/R4 Low Priced Rules require the stock close above USD 1 and below USD 15. The source module explicitly keeps that numeric band as INR 1 to INR 15; this is an unsupported currency reinterpretation."),
        ("ats","ats_cbp")=>Some("Advanced Trading Strategies Chapter 11 filters US momentum stocks above USD 20. The executable min_price parameter is compared directly with INR stock prices; a local currency specification is missing."),
        ("ats","ats_dvmtm")=>Some("Advanced Trading Strategies Chapter 13 enters one source US-stock tick (1/8 USD) below the signal low. The runner's generic local tick parameter does not establish the source-supported INR/NSE execution increment."),
        ("ats","ats_15min_adx_breakout_equities")=>Some("Advanced Trading Strategies p.164 requires a US equity price above USD 40 and stock execution ticks. Comparing a literal min_price with INR prices or substituting a generic tick does not preserve this source rule."),
        _=>None,
    };
    if p.native_currency_constraint.is_some(){p.source_price_unit=Some("USD");}
    p
}
fn base_policy(def:&StrategyDef)->Policy {
    let id=def.id;
    match def.book {
        "stc9" if member(id,&["f1a","f3a","f4a","f4c","f4d","rsi2_5d"]) => stock("STC IX Session 1 foundation studies concern a pooled US individual-stock population. A native single-stock run uses one actual constituent at a time; it does not reproduce the pooled published counts, universe or performance."),
        "stc9" if member(id,&["tps_stocks","x555","h10"]) => stock("STC IX Sessions 2-3 specify stocks with volatility, ADX and Liquidity Filter population restrictions. Actual constituent OHLC/volume is necessary; this type check does not supply missing liquidity denominators, historical membership or portfolio slots."),
        "stc9" if id=="r3" => index("STC IX Session 1 R3 explicitly uses an index; the reference is the S&P 500. A selected Indian index is a separate market application."),
        "stc9" if member(id,&["tps_long","tps_short"]) => etf("STC IX Session 4 presents ETF TPS rules. Cash-index OHLC cannot substitute for actual ETF prices or implementation of the ETF's exposure."),
        "stc9" if id=="s12345" => requirement("leveraged_etf", &[], "STC IX Session 7 specifies 2x/3x leveraged ETFs. No designated leveraged-ETF price series and exposure validation is currently admitted. An index or unleveraged ETF is not a substitute."),
        "stc9" if member(id,&["lev_calls","busted"]) => requirement("options", &[], "STC IX Session 8 requires exact traded option contracts, strikes, expiries and option-position accounting, plus earnings data where specified."),
        "stc9" if member(id,&["eom_co","eom_rsi","eom_cc","rsi2_4","rsi4_25","rsi_div8","rsi_div2","rsi_1","rsi_2","rsi_3","price","pctb_1","pctb_2","dmi_1","dmi_2","dmi_3","dmi_4","dmi_5","short_4up","short_strong","vix200","vix2","vix4","vix7","vix8","vix53_short","vix101_short","trin4","trin4_12","trin3_10","trin3_11","trin2_11"]) => {
            let mut p=requirement("index_or_etf", &["index","etf"], "STC IX Session 7 market-timing studies reference SPY. Applying the percentage/indicator timing rules to an observed cash index is an explicitly declared market adaptation, not ETF performance reproduction. Required market-specific VIX/TRIN and execution checks remain mandatory.");p.market_adaptation=true;p
        },
        "etfss" if member(id,&["ss_long","ss_short","ss_both"]) => etf("ETF Sweet Spot Part III pp.10-11 specifies ETF entries and exits; the reference instruments are SPY, QQQ, IWM, SMH and MDY. Published trade counts use units; this engine counts closed positions."),
        "rsi25" if member(id,&["r25_rsi25_basic_no_filter","r25_rsi25_official_sma200","r25_rsi25_vix","r25_rsi25_15","r25_rsi25_15_vix"]) => index("RSI 25 Explosions Part I/II studies NDX, SPX and SOX cash indices. The selected index is a new market run; scale-in reference counts may be per unit rather than per closed position."),
        "pctb" if member(id,&["pbs_percent_b_swing","pbs_percent_b_plus_one"]) => requirement("index_etf_or_stock", &["index","etf","individual_stock"], "%b Swing Part I/II tests SPX/NDX/SOX and explicitly suggests SPY/QQQ/SMH or component stocks. Only the selected actual price series is tested; no pooled portfolio or source-performance reproduction is implied."),
        "ststw" if member(id,&["ststw_10d_high_buy","ststw_10d_high_short","ststw_10d_low_buy","ststw_rsi2_under5"]) => index("Short Term Trading Strategies That Work Chapter 2 pp.11-14 and Chapter 9 pp.61-65 explicitly study the S&P500 cash index. Selected Indian-index outcomes are separate from those SPX studies."),
        "ststw" if member(id,&["ststw_overnight","ststw_intraday","ststw_cum_rsi_35","ststw_cum_rsi_50","ststw_double7"]) => etf("Short Term Trading Strategies That Work Chapter 7 pp.36-38, Chapter 9 pp.66-68 and Chapter 10 pp.75-80 reference actual SPY/QQQ or other ETFs. Actual ETF opens and closes are required; cash-index swaps are not silently admitted."),
        "ststw" if member(id,&["ststw_vix_stretch","ststw_sp_short"]) => requirement("index_or_etf", &["index","etf"], "Short Term Trading Strategies That Work Chapter 12 pp.93-95 and pp.106-108 explicitly names SPY or SPX. Market-specific VIX and all other rule/data checks remain required; a single stock is not the timed index/ETF."),
        "hpetf" if member(id,&["hpetf_3dhl_long","hpetf_3dhl_long_aggr","hpetf_3dhl_short","hpetf_3dhl_short_aggr","hpetf_rsi25_long_aggr","hpetf_rsi75_short","hpetf_rsi75_short_aggr","hpetf_r3_long","hpetf_r3_long_aggr","hpetf_r3_short","hpetf_r3_short_aggr","hpetf_pctb_long_aggr","hpetf_pctb_short_aggr","hpetf_mdd_long","hpetf_mdd_long_aggr","hpetf_mdu_short","hpetf_mdu_short_aggr","hpetf_rsi10_6_long","hpetf_rsi90_94_short","hpetf_tps_long","hpetf_tps_short"]) => etf("High Probability ETF Trading (2009), Chapters 2-8, explicitly trades 20 non-leveraged ETFs. Actual ETF prices are required. DJI is an index, not DIA; an Indian constituent or cash index cannot be silently substituted."),
        "afactor" if id=="afactor_baseline" => stock("Alvarez Factor workbook pp.13-14 and Stats.xls baseline specify one stock's gap, OHLC and share volume. A selected Indian stock is an instrument/currency adaptation; literal US dollar price thresholds, corporate-action treatment and the absent pooled US universe remain limitations."),
        "afactor" if id=="afactor_portfolio" => portfolio("Alvarez Factor Stats.xls portfolio uses simultaneous stock signals, 1/10 slots and day-level allocation. Independent-symbol returns cannot be compounded into the published portfolio without the verified universe and allocation engine."),
        "r3r4" if member(id,&["r3r4_base","r3r4_base_scale","r3r4_bigcap","r3r4_bigcap_scale","r3r4_bigcap_deck","r3r4_bigcap_deck_scale","r3r4_midcap","r3r4_midcap_scale","r3r4_lowprice","r3r4_lowprice_scale"]) => stock("R3/R4 guide pp.6-8 and appendices pp.28-29 explicitly require stocks, share-volume liquidity, HV and ADX filters. Native constituent data alone cannot establish the missing benchmark volume or reproduce the 6,000+ US-stock study."),
        "jc5dm" if member(id,&["jc5dm_long","jc5dm_short"]) => stock("Five Day Momentum pp.10-12 and p.39 specifies individual stocks, US dollar price thresholds and a 1/16-point tick. Instrument-type compatibility does not approve a currency/tick conversion or waive the existing fixed-amount accuracy gate."),
        "ats" if member(id,&["ats_cbp","ats_dvmtm","ats_cvr_equities","ats_15min_adx_breakout_equities"]) => stock("Advanced Trading Strategies Chapters 11-14 and the equities ADX variant concern individual stocks. IBD ratings, volume, intraday data, currency-specific offsets and incomplete discretionary exits remain separate source/data requirements; index prices are not stock substitutes."),
        "lwrsr" if member(id,&["lwrsr_oct_apr","lwrsr_apr_oct"]) => index("The Right Stock at the Right Time Chapter 3, Table 3.2 printed pp.32-34, explicitly times the DJIA index. Nifty is a new index application, not the Dow's historical results."),
        "lwrsr" if id=="lwrsr_stock_selection" => portfolio("The Right Stock at the Right Time Chapters 8-11 selects Dow/Utilities members using point-in-time fundamentals and portfolio rules. Present cash constituents do not supply these historical inputs."),
        "mmft" if id=="mmft_nasdaq79" => index("Market & Mutual Fund Timing Week 3 Model 1 uses the Nasdaq Composite index. Other observed index applications are separate market experiments; the absent source index and its 1963-1997 result are not reproduced."),
        "radge" if id=="radge_ma220" => future("Radge Trading Course Moving Average Breakout Example 50 reports CL crude-oil futures and uses instrument minimum ticks. Actual dated futures prices, contract identities and tick values are required; default NSE cash-index ticks cannot reproduce CL execution."),
        "radge" if id=="radge_oc_long" => index("Successful Stock Trading Chapter 1 printed p.16 studies an unnamed major stock index, not an individual company. Selected-index compatibility does not identify or reproduce that source instrument; the accuracy gate still applies."),
        "radge" if member(id,&["radge_oc_long_stop","radge_oc_short_stop"]) => review(),
        "lwdg2" if member(id,&["lwdg2_updown_daytrade","lwdg2_0105","lwdg2_vol_up_up","lwdg2_vol_dn_up","lwdg2_vol_up_dn","lwdg2_vol_dn_dn","lwdg2_weekly_entry"]) => future("Definitive Guide to Futures Trading Volume II Chapters 3/8, printed pp.34-39 and pp.124-152, trades S&P/Value Line futures. Cash OHLC does not supply the actual futures basis, fixed stops/ticks, contracts or roll history."),
        "hmrw" if id=="hmrw_sp500_rotation" => portfolio("How Markets Really Work Chapter 13 pp.159-162 requires all historical S&P500 members including removed/delisted names, a separate index regime, candidate ranking and up to ten simultaneous slots. A current snapshot and one-symbol runner cannot reproduce this portfolio."),
        "alpha" if member(id,&["alpha_rising_assets","alpha_weekly_mr","alpha_dyn_treasuries","alpha_etf_avalanches","alpha_formula_portfolio"]) => portfolio("The Alpha Formula Chapters 5-9 require specific stock/ETF universes, cross-sectional selection and simultaneous portfolio allocations. Current Indian cash snapshots do not establish these source universes or portfolio accounting."),
        "alpha" if member(id,&["alpha_spy_rsi4_mr","alpha_dyn_treasuries_sleeve","alpha_etf_avalanches_single"]) => {
            // A Treasury sleeve is not interchangeable with an equity ETF. No
            // required Treasury instrument is currently on the admitted whitelist.
            if id=="alpha_dyn_treasuries_sleeve" {requirement("treasury_etf", &[], "The Alpha Formula Chapter 7 requires actual IEI/IEF/TLH/TLT Treasury ETF prices. Equity ETFs or Indian stock prices cannot replace a Treasury sleeve.")}
            else {etf("The Alpha Formula Chapters 4/8 uses actual ETF prices and dividend-adjusted source returns. The selected real ETF is a separate price-series test; no portfolio or dividend-source reproduction is implied.")}
        },
        "ch_bta" if member(id,&["ch_65sma3cc","ch_cbpb_time"]) => {
            let mut p=requirement("declared_ohlc_adaptation", &["index","etf","individual_stock"], "Supplied partial Chande 1997 text, printed pp.75-80 and pp.101-111. These two rules are explicitly named OHLC return-based adaptations on actual selected prices; they do not reproduce the original futures contracts, dollar stops, 23-market portfolio or intraday drawdown.");p.market_adaptation=true;p
        },
        "ch_bta" if member(id,&["ch_cbpb_dollar","ch_cbpb_target","ch_cbpb_trail"]) => future("Chande Tables 4.8-4.10 require actual futures contract dollar/tick values, dated contract/roll prices and supported stop/target ordering. No cash-point or percentage substitute is authorized."),
        _ => review(),
    }
}

/// `native_stock` is a provenance assertion by the caller AFTER a successful
/// constituent catalogue lookup/load, never a client-supplied flag.
pub fn selected_type(ticker:&str,native_stock:bool)->&'static str {
    if native_stock {return "individual_stock";}
    match ticker {
        "NIFTY 50"|"NIFTY BANK"|"NIFTYBANK"|"NIFTY IT"|"NIFTYIT"|"NIFTY MIDCAP 100"|"NIFTYMIDCAP100"|"SPX"|"NDX"|"DJI" => "index",
        "SPY"|"QQQ"|"IWM"|"MDY"|"SMH"|"DIA" => "etf",
        _ => "unknown",
    }
}
pub fn check(def:&StrategyDef,ticker:&str,native_stock:bool)->Option<Issue> {
    let p=policy(def);let selected=selected_type(ticker,native_stock);
    if native_stock {
        if let Some(reason)=p.native_currency_constraint {
            return Some(Issue {status:"needs_data",note:format!("Source currency/tick data missing. {reason} USD amounts cannot silently become INR amounts. A source-supported Indian specification or dated FX treatment and verified local tick rules are required before admission; editing the numeric parameter alone does not establish those inputs. Selected actual stock: {ticker}. No results are published.")});
        }
    }
    check_selected(p,ticker,selected)
}
/// Native type/currency comes from the fixed server catalogue, never from a
/// request flag or a ticker spelling. USD compatibility does not certify the
/// source's historical adjusted-price basis or its original execution ticks.
pub fn check_native(def:&StrategyDef,ticker:&str,evidence:&NativeEvidence)->Option<Issue> {
    let p=policy(def);
    if let Some(issue)=check_selected(p,ticker,evidence.asset_type()){return Some(issue);}
    if let Some(reason)=p.native_currency_constraint {
        return Some(Issue{status:"needs_data",note:format!("Source currency/price-unit/tick review is pending. {reason} Selected actual stock: {ticker}; trusted source market {} and currency {}. Matching USD currency alone cannot establish the author's historical adjusted-price basis, absolute price thresholds or execution ticks; no reviewed source-unit proof is staged. No results are published.",evidence.market().code(),evidence.currency())});
    }
    None
}
/// Check actual context identities before coverage checks on derived arrays.
/// These inputs are server-owned loaded-series identities, not query values.
/// Even a matching identity still needs actual finite close/volume and date
/// coverage checked by the caller; this function supplies no synthetic data.
pub fn check_native_market_inputs(def:&StrategyDef,evidence:&NativeEvidence,benchmark_ticker:Option<&str>,volatility_ticker:Option<&str>)->Option<Issue> {
    if evidence.market()!=Market::UnitedStates{return None;}
    if crate::connors::params(def.id).iter().any(|p|p.key=="lf")&&benchmark_ticker!=Some("SPX") {
        return Some(Issue{status:"needs_data",note:"The US stock Liquidity Filter requires actual source S&P500 (SPX) benchmark close and volume with covered dates. Nifty close/volume, a basket-specific index, individual-stock volume and unknown auxiliary fields cannot replace that denominator. No results are published.".into()});
    }
    let requires_vix=def.needs_vix||def.needs_series.iter().any(|s|s.starts_with("VIX.")||s.starts_with("VIX_OHLC."));
    if requires_vix&&volatility_ticker!=Some("CBOE VIX") {
        return Some(Issue{status:"needs_data",note:"This US-market method requires actual CBOE VIX data and covered dates. India VIX or a generic series named VIX cannot establish the required US volatility-index context. No results are published.".into()});
    }
    None
}
fn check_selected(p:Policy,ticker:&str,selected:&str)->Option<Issue> {
    if p.allowed_types.contains(&selected) {return None;}
    let (status,explanation)=match p.requirement {
        "source_review_required" => ("not_testable","Source review required before publishing this method on any instrument."),
        "individual_stock" => ("needs_data","This method requires a validated individual stock from a source-qualified constituent basket. A cash index or ETF cannot stand in for its stock population."),
        "index" => ("needs_data","This method requires an actual index price series. A constituent stock or ETF is a different instrument."),
        "etf" => ("needs_data","This method requires real ETF prices. An index, including DJI in place of DIA, or an individual stock is not the ETF."),
        "futures" => ("needs_data","Actual dated futures contracts, contract-specific tick/point values, execution prices and roll history are required; cash-stock/index prices cannot replace them."),
        "options" => ("needs_data","Exact traded option contract/leg prices and validated option-position accounting are required; underlying prices cannot replace premiums."),
        "portfolio" => ("needs_data","Historical universe membership, required stock/asset data, cross-sectional selection and simultaneous portfolio accounting are unavailable. Independent symbol tests are not an aggregate portfolio."),
        "leveraged_etf" => ("needs_data","Actual designated leveraged-ETF prices/exposure are unavailable. Unleveraged ETFs, indices and stocks are not substitutes."),
        _ => ("needs_data","The selected instrument does not meet the explicitly reviewed source requirement; no substitute is inferred."),
    };
    Some(Issue{status,note:format!("{explanation} Selected {ticker}: {selected}. {}",p.source_basis)})
}
pub fn metadata_json(def:&StrategyDef)->String {
    let p=policy(def);
    fn escape(s:&str)->String {s.chars().flat_map(|c|match c {'"'=>"\\\"".chars().collect::<Vec<_>>(),'\\'=>"\\\\".chars().collect(),'\n'=>"\\n".chars().collect(),'\r'=>"\\r".chars().collect(),'\t'=>"\\t".chars().collect(),c if c.is_control()=>format!("\\u{:04x}",c as u32).chars().collect(),c=>vec![c]}).collect()}
    let mut json=format!("{{\"requirement\":\"{}\",\"allowed_types\":[{}],\"preferred_basket\":{},\"execution_scope\":\"{}\",\"source_basis\":\"{}\",\"market_adaptation\":{},\"source_performance_reproduced\":false,\"limitations\":\"Compatibility is only an instrument check. Required source filters, data, currency/tick units, corporate actions, fill ordering and accounting must pass separate checks. Current constituent snapshots do not establish historical membership; retrospective selection has survivorship bias.\"}}",p.requirement,p.allowed_types.iter().map(|s|format!("\"{s}\"")).collect::<Vec<_>>().join(","),p.preferred_basket.map_or("null".into(),|s|format!("\"{s}\"")),p.execution_scope,escape(p.source_basis),p.market_adaptation);
    json.pop();
    let currency_admitted=if p.native_currency_constraint.is_some(){"false"}else if p.allowed_types.contains(&"individual_stock"){"true"}else{"null"};
    json.push_str(&format!(",\"source_price_unit\":{},\"native_india_currency_admitted\":{currency_admitted},\"native_currency_constraint\":{}}}",p.source_price_unit.map_or("null".into(),|s|format!("\"{}\"",escape(s))),p.native_currency_constraint.map_or("null".into(),|s|format!("\"{}\"",escape(s)))));
    json
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::connors::{Status, StrategyDef};
    fn def(id: &'static str, book: &'static str) -> StrategyDef {
        StrategyDef { id, book, name: "Test", group: "Test", session: 0, dir: "Long", status: Status::Live,
            needs_volume: false, needs_vix: false, needs_series: &[], rules: &[], notes: "", claim: None }
    }
    #[test]
    fn stock_studies_require_validated_individual_stock_and_keep_snapshot_limits() {
        for id in ["f1a", "f3a", "f4a", "f4c", "f4d", "rsi2_5d", "tps_stocks", "x555", "h10"] {
            let d=def(id,"stc9");
            assert!(check(&d,"NIFTY 50",false).unwrap().note.contains("individual"));
            assert!(check(&d,"SPY",false).is_some());
            assert!(check(&d,"RELIANCE",true).is_none());
            assert!(check(&d,"RELIANCE",false).is_some());
            let json=metadata_json(&d);
            assert!(json.contains("\"preferred_basket\":\"nifty50\""));
            assert!(json.contains("survivorship"));
        }
    }
    #[test]
    fn index_rules_and_real_etf_rules_are_distinct() {
        let d=def("r25_rsi25_official_sma200","rsi25");
        assert!(check(&d,"NIFTY 50",false).is_none());
        assert!(check(&d,"RELIANCE",true).is_some());
        assert!(check(&d,"SPY",false).is_some());
        let etf=def("hpetf_3dhl_long","hpetf");
        assert!(check(&etf,"SPY",false).is_none());
        assert!(check(&etf,"IWM",false).is_none());
        assert!(check(&etf,"DJI",false).is_some());
        assert!(check(&etf,"NIFTY 50",false).is_some());
        assert!(check(&etf,"RELIANCE",true).is_some());
    }
    #[test]
    fn leverage_futures_options_and_portfolios_cannot_become_cash_backtests() {
        for (id,book) in [("s12345","stc9"),("lwdg2_0105","lwdg2"),("lev_calls","stc9"),("hmrw_sp500_rotation","hmrw"),("alpha_weekly_mr","alpha")] {
            let d=def(id,book);
            for (ticker,native) in [("NIFTY 50",false),("SPY",false),("RELIANCE",true)] {
                assert_eq!(check(&d,ticker,native).unwrap().status,"needs_data");
            }
        }
        assert_eq!(selected_type("FO CASH CURRENT",false),"unknown");
        assert_eq!(selected_type("RELIANCE",true),"individual_stock");
    }
    #[test]
    fn no_keyword_or_new_book_name_can_automatically_admit_a_method() {
        let d=def("future_new_method","hpetf");
        assert_eq!(policy(&d).requirement,"source_review_required");
        assert_eq!(check(&d,"SPY",false).unwrap().status,"not_testable");
        let spoof=def("f1a","new_unreviewed_book");
        assert_eq!(policy(&spoof).requirement,"source_review_required");
        assert!(check(&spoof,"RELIANCE",true).is_some());
        assert_eq!(selected_type("SPY_FAKE",false),"unknown");
        assert_eq!(selected_type("NIFTY SOMETHING",false),"unknown");
    }
    #[test]
    fn only_explicit_source_permission_or_declared_adaptation_allows_multiple_types() {
        let d=def("pbs_percent_b_swing","pctb");
        for (ticker,native) in [("NIFTY 50",false),("SPY",false),("RELIANCE",true)] {assert!(check(&d,ticker,native).is_none());}
        let d=def("ch_65sma3cc","ch_bta");
        assert!(policy(&d).market_adaptation);
        assert!(check(&d,"RELIANCE",true).is_none());
        assert!(check(&d,"DJI",false).is_none());
        assert!(metadata_json(&d).contains("\"source_performance_reproduced\":false"));
        assert!(check(&def("ch_cbpb_dollar","ch_bta"),"NIFTY 50",false).is_some());
    }
    #[test]
    fn per_rule_exceptions_avoid_a_misleading_book_title_classification() {
        assert_eq!(policy(&def("lwrsr_oct_apr","lwrsr")).requirement,"index");
        assert_eq!(policy(&def("lwrsr_stock_selection","lwrsr")).requirement,"portfolio");
        assert_eq!(policy(&def("alpha_spy_rsi4_mr","alpha")).requirement,"etf");
        assert_eq!(policy(&def("alpha_weekly_mr_single","alpha")).requirement,"source_review_required");
        assert_eq!(policy(&def("radge_ma220","radge")).requirement,"futures");
        assert_eq!(policy(&def("radge_oc_long","radge")).requirement,"index");
        assert_eq!(policy(&def("jc5dm_long","jc5dm")).requirement,"individual_stock");
        assert!(check(&def("jc5dm_long","jc5dm"),"NIFTY 50",false).is_some());
    }
    #[test]
    fn mixed_stock_etf_and_index_book_uses_the_specific_rule_population() {
        assert!(check(&def("ststw_rsi2_under5","ststw"),"NIFTY 50",false).is_none());
        assert!(check(&def("ststw_rsi2_under5","ststw"),"SPY",false).is_some());
        assert!(check(&def("ststw_double7","ststw"),"SPY",false).is_none());
        assert!(check(&def("ststw_double7","ststw"),"NIFTY 50",false).is_some());
        assert!(check(&def("ststw_sp_short","ststw"),"SPY",false).is_none());
        assert!(check(&def("ststw_sp_short","ststw"),"NIFTY 50",false).is_none());
        assert!(check(&def("ststw_sp_short","ststw"),"RELIANCE",true).is_some());
        // Current snapshot stock prices do not enable the absent original pooled universe.
        assert!(check(&def("ststw_intraday_drops_highs","ststw"),"RELIANCE",true).is_some());
    }
    #[test]
    fn source_dollar_price_and_tick_rules_are_not_reinterpreted_as_rupees() {
        for (id,book) in [("afactor_baseline","afactor"),("jc5dm_long","jc5dm"),("jc5dm_short","jc5dm"),
            ("r3r4_lowprice","r3r4"),("r3r4_lowprice_scale","r3r4"),("ats_cbp","ats"),
            ("ats_dvmtm","ats"),("ats_15min_adx_breakout_equities","ats")] {
            let d=def(id,book);
            let issue=check(&d,"RELIANCE",true).expect("Native stock type must not bypass source currency");
            assert_eq!(issue.status,"needs_data");
            assert!(issue.note.contains("USD")&&issue.note.contains("INR"));
            let json=metadata_json(&d);
            assert!(json.contains("\"native_india_currency_admitted\":false"));
            assert!(json.contains("\"source_price_unit\":\"USD\""));
            assert_eq!(policy(&d).preferred_basket,Some("nifty50"));
        }
    }
    #[test]
    fn price_independent_stock_rules_and_existing_ticker_aliases_stay_eligible() {
        for id in ["f1a","f3a","f4a","f4c","f4d","rsi2_5d","tps_stocks","x555","h10"] {
            assert!(check(&def(id,"stc9"),"RELIANCE",true).is_none());
        }
        assert!(check(&def("r3r4_base","r3r4"),"RELIANCE",true).is_none());
        for ticker in ["NIFTYBANK","NIFTYIT","NIFTYMIDCAP100"] {assert_eq!(selected_type(ticker,false),"index");}
        for ticker in ["MDY","SMH"] {assert_eq!(selected_type(ticker,false),"etf");}
        assert!(metadata_json(&def("f1a","stc9")).contains("\"native_india_currency_admitted\":true"));
    }
    #[test]
    fn trusted_us_company_stock_is_not_an_etf_or_index_and_does_not_certify_dollar_units() {
        let e=NativeEvidence::us_test_fixture();
        assert!(check_native(&def("f1a","stc9"),"AAPL",&e).is_none());
        assert!(check_native(&def("f3a","stc9"),"AABA-201910",&e).is_none());
        assert!(check_native(&def("r3","stc9"),"SPX",&e).is_some(),"Even an index-like ticker cannot change trusted stock type");
        assert!(check_native(&def("hpetf_3dhl_long","hpetf"),"SPY",&e).is_some());
        for (id,book) in [("afactor_baseline","afactor"),("jc5dm_long","jc5dm"),("r3r4_lowprice","r3r4"),("ats_dvmtm","ats")] {
            let issue=check_native(&def(id,book),"AAPL",&e).unwrap();assert_eq!(issue.status,"needs_data");assert!(issue.note.contains("USD")&&issue.note.contains("Matching USD currency alone"));
        }
        // Existing admission API has no client currency argument and preserves
        // Indian defaults even if a client adds a misleading ticker suffix.
        assert!(check(&def("afactor_baseline","afactor"),"RELIANCE?currency=USD",true).unwrap().note.contains("INR"));
    }
    #[test]
    fn us_stock_filters_require_us_source_benchmark_and_cboe_vix_context() {
        let e=NativeEvidence::us_test_fixture();let lf=def("tps_stocks","stc9");
        for benchmark in [None,Some("NIFTY 50"),Some("NDX"),Some("DJI"),Some("RUT")] {
            let issue=check_native_market_inputs(&lf,&e,benchmark,None).unwrap();assert!(issue.note.contains("SPX")&&issue.note.contains("volume"));
        }
        assert!(check_native_market_inputs(&lf,&e,Some("SPX"),None).is_none());
        let mut vix=def("f1a","stc9");vix.needs_vix=true;
        for volatility in [None,Some("INDIA VIX"),Some("VIX")] {assert!(check_native_market_inputs(&vix,&e,None,volatility).unwrap().note.contains("CBOE VIX"));}
        assert!(check_native_market_inputs(&vix,&e,None,Some("CBOE VIX")).is_none());
        assert!(check_native_market_inputs(&def("f1a","stc9"),&e,None,None).is_none());
    }
}
