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
/// User decision 2026-10-21: non-ranking Connors single-stock rules also run on
/// an actual index/ETF as a visibly declared application. Ranking/selection rules stay stock-only.
macro_rules! connors_any {($basis:literal)=>{{
    let mut p=requirement("index_etf_or_stock", &["index","etf","individual_stock"], concat!($basis," ",CONNORS_INDEX_LABEL!()));
    p.preferred_basket=Some("nifty50");p.market_adaptation=true;p
}}}
macro_rules! CCL08_INDEX_LABEL {()=>{"Book shows these day trades on US stocks; index/ETF runs are a declared application, not the author's test. The Liquidity Filter is computed on the instrument's own close x volume against the benchmark (SPX or NIFTY 50): on a cash index it is always met (no screening); on an ETF it is the fund's actual dollar volume and can screen days (IWM, MDY, SMH fall below 3-10 at times); indices without volume (NIFTY IT) are refused for LF rules. HV100, RSI(2), 200-day and percent-move filters apply as printed."}}
macro_rules! BTP_INDEX_LABEL {()=>{"Book tested or illustrated on futures and US stocks; index/ETF runs are a declared application, not the author's test."}}
macro_rules! CWIN_INDEX_LABEL {()=>{"The book and course show Windows on US stocks, HOLDRs, indices, ETFs and futures; index/ETF runs are a declared application, not the author's test. The 100,000-share volume suggestion (Q&A p56) applies to stocks only and is not coded."}}
macro_rules! WS5_INDEX_LABEL {()=>{"Seminar method for US stocks from a nightly scan; index/ETF runs are a declared application, not the author's test. Stock-only filters (price, volume, sub-sector, range, ATR) do not apply to an index."}}
macro_rules! STOCK_TICK_LABEL {()=>{"Actual stocks are admitted (the book shows this setup on US stocks). Adaptation: the book used 1/8-dollar ticks; stock runs offset entries/stops by the stock's real tick, 0.05 INR on NSE and 0.01 USD on US stocks. Vault stock prices are split/dividend-adjusted (not independently verified), so one tick is a larger fraction of early adjusted prices. One stock at a time; no stock population or book result is reproduced."}}
macro_rules! WS5_STOCK_LABEL {()=>{"Actual stocks are admitted without the filters that cannot be applied: #5 sub-sector vs its 200-day SMA is omitted for every stock (no SUBSECTOR series); on Indian stocks the USD #2 price >= $15, #6 90-day range > 6 points and #7 ATR(90) >= 1 point are omitted (no reviewed INR version) and only #3 share volume (100,000 shares/day; weekly 500,000 shares/week) is kept as printed; on US stocks #2, #3, #6 and #7 are kept in USD as printed, applied to split/dividend-adjusted prices. Results are therefore not the author's scan."}}
macro_rules! USD_PRICE_US_LABEL {()=>{"US stocks only: the book's USD price filter is applied as printed to the US stock's own USD prices (vault prices are split/dividend-adjusted, not independently verified, so early adjusted prices sit below the historical quotes). Indian stocks stay held: the USD 1-15 price band is the strategy itself."}}
macro_rules! AFACTOR_STOCK_LABEL {()=>{"US stocks: the book's USD price filter (previous close > 10) is applied as printed to the US stock's own USD prices (split/dividend-adjusted, not independently verified). Indian stocks: USD price filter omitted on INR stocks (no reviewed INR version); the share-volume filter (20-day average > 100,000 shares) is kept as printed. Not the author's US universe."}}
macro_rules! ALPHA_TREND_LABEL {()=>{"Book tested on nine US sector ETFs (XLB..XLY, not in the vault); other ETFs and index runs are a declared application, not the author's test."}}
macro_rules! CONNORS_INDEX_LABEL {()=>{"Book tested on US stocks; index/ETF runs are a declared application, not the author's test. Stock filters in the rule (minimum price, share volume, Liquidity Filter) are still calculated on the index's own price and volume and are always met there (the R3/R4 low-price $1-$15 band is never met), so they do not screen index runs."}}
pub const CONNORS_INDEX_IDS:&[&str]=&["f1a","f3a","f4a","f4c","f4d","rsi2_5d","tps_stocks","x555","h10","r3r4_base","r3r4_base_scale","r3r4_bigcap","r3r4_bigcap_scale","r3r4_bigcap_deck","r3r4_bigcap_deck_scale","r3r4_lowprice","r3r4_lowprice_scale","ats_dvmtm","ats_cvr_equities","ats_15min_adx_breakout_equities","afactor_baseline","afactor_spx_down"];

pub fn reviewed_kestner_stock(def:&StrategyDef)->bool {
    def.book==crate::books::lkqts::BOOK && crate::books::lkqts::stock_research_profile(def.id)
}

/// Exact source-rule IDs reviewed against selected real-price ledgers.
/// Each batch retains its instrument and readiness limits; original studies are not reproduced.
pub fn reviewed_cash_rule(def:&StrategyDef)->bool {
    ((def.book=="hbqts" && member(def.id,&["hbqts_ma5_25","hbqts_ma1_36","hbqts_limit_entry","hbqts_fixed_interval","hbqts_profit_target","hbqts_entry_id1","hbqts_bb_breakout","hbqts_cgo","hbqts_stoch_rsi","hbqts_dpo","hbqts_dom","hbqts_macd_daily","hbqts_cmo"])) ||
    (def.book=="kftsm" && member(def.id,&["kftsm_ma","kftsm_exp","kftsm_lrs","kftsm_ndb","kftsm_bb_ma","kftsm_ma_pct_band","kftsm_ma_tr_band","kftsm_triple_smooth","kftsm_vol_breakout","kftsm_keltner","kftsm_hh_ma","kftsm_hh_cross","kftsm_hh_channel","kftsm_mod_3ma","kftsm_4_9_18","kftsm_month_end","kftsm_hirsch"]))) || reviewed_adaptive_rule(def)
        || (def.book=="spmt" && crate::books::spmt::reviewed_daily(def.id))
        || (def.book=="vixr" && crate::books::vixr::reviewed_daily(def.id))
        || (def.book=="ptjmt" && crate::books::ptjmt::reviewed_daily(def.id))
        || (def.book==crate::books::ss::BOOK && crate::books::ss::reviewed_daily(def.id))
        || (def.book=="btp" && crate::books::btp::reviewed_daily(def.id))
        || (def.book==crate::books::cwin::BOOK && crate::books::cwin::reviewed_daily(def.id))
        || (def.book==crate::books::ws5::BOOK && crate::books::ws5::reviewed_daily(def.id))
        || (def.book==crate::books::ats::BOOK && crate::books::ats::reviewed_daily(def.id))
        || (def.book==crate::books::ccl08::BOOK && crate::books::ccl08::reviewed_daily(def.id))
        || (def.book==crate::books::alpha::BOOK && def.id=="alpha_trend_6m")
        || newrules_misc(def)
        || hfm_otvt_reviewed(def)
        || (def.book=="chbta2" && member(def.id,&["ch_bta2_ma_stack","ch_bta2_65sma_ravi"]))
        || (def.book=="sbma" && member(def.id,&["sbma_sma50_long","sbma_x_5s20s"]))
        || (def.book=="radge" && def.id=="radge_oc_long")
        || (def.book=="bllstr" && crate::books::bllstr::reviewed_daily_index(def.id))
        || (def.book=="sbma" && crate::books::sbma::next_reviewed_stock_rule(def.id))
        || (def.book=="sbma" && crate::books::sbma::four_reviewed_stock_rule(def.id))
        || (def.book=="kfsc" && crate::books::kfsc::reviewed_stock_rule(def.id))
        || (def.book=="kfsc" && crate::books::kfsc::reviewed_momentum_rule(def.id))
        || (def.book=="kfsc" && crate::books::kfsc::reviewed_hv_rule(def.id))
        || (def.book=="ehrs" && crate::books::ehrs::reviewed_sinetrend(def.id))
        || (def.book=="elcim" && def.id==crate::books::elcim_account::ID)
        || (def.book=="hbiab" && crate::books::hbiab_reviewed::reviewed(def.id))
        || reviewed_kestner_stock(def)
}
/// Connors rules coded from the full-source readers (connors/newrules-misc, 2026-10-21): HPETF seminar RSI 14 TPS,
/// STSTW VIX RSI, ETF Sweet Spot 2nd-signal variation, Alvarez Factor S&P-down-days filter.
pub fn newrules_misc(def:&StrategyDef)->bool {
    (def.book==crate::books::hpetf::BOOK && crate::books::hpetf::reviewed_daily(def.id))
        || (def.book==crate::books::ststw::BOOK && crate::books::ststw::reviewed_daily(def.id))
        || (def.book=="etfss" && crate::connors::ETFSS_REVIEWED.contains(&def.id))
        || (def.book==crate::books::afactor::BOOK && crate::books::afactor::REVIEWED_DAILY.contains(&def.id))
}
macro_rules! NEWRULE_ANY_LABEL {()=>{"Index, ETF and stock runs: the source trades US ETFs; actual indices and actual stocks are a declared application (user rule 21 Oct 2026), not the author's test. Close-only rule: no price, volume or tick filter to adapt."}}
pub fn ststw_symbol_issue(def:&StrategyDef,ticker:&str)->Option<Issue> {
    use crate::books::ststw;
    if def.book!=ststw::BOOK || !ststw::reviewed_daily(def.id) || ststw::admitted_symbol(def.id,ticker) {return None;}
    Some(Issue{status:"needs_data",note:format!("The VIX RSI Strategy (Short Term Trading Strategies That Work p.96-98) times the S&P 500 on SPY or the SPX and reads today's VIX open. This review admits SPX and SPY with the CBOE VIX only: India VIX opens repeat the previous close, so the rule cannot be read on NIFTY. {ticker} is outside that reviewed scope; no result is published.")})
}
/// Figure A.7 is an index example, distinct from the reviewed stock/ETF batch.
pub fn reviewed_dema_index(def:&StrategyDef)->bool {
    def.book=="hbqts" && crate::books::hbqts::reviewed::is_dema_index(def.id)
}
fn dema_index_symbol_issue(def:&StrategyDef,ticker:&str,selected:&str)->Option<Issue> {
    if reviewed_dema_index(def) && !(selected=="index" && matches!(ticker,"SPX"|"NIFTY 50")) {
        return Some(Issue{status:"needs_data",note:"QTS Figure A.7 printed324 illustrates the S&P500 cash index. This review admits actual SPX and the separately declared actual NIFTY 50 cash-index adaptation only. Other indices, stocks, ETFs, derivatives and baskets do not inherit source or numerical review.".into()});
    }
    None
}
fn vixr_symbol_issue(def:&StrategyDef,ticker:&str)->Option<Issue> {
    if def.book!="vixr" || !crate::books::vixr::reviewed_daily(def.id) || crate::books::vixr::admitted_symbol(def.id,ticker) {return None;}
    Some(Issue{status:"needs_data",note:format!("Trading Connors VIX Reversals traded S&P futures on the CBOE VIX. This review admits actual SPX and SPY with the CBOE VIX, and NIFTY 50 with India VIX only for CVR 2, 3 and 6 (rules that do not read the VIX open or fixed VIX-point thresholds). {ticker} is outside that reviewed scope; no result is published.")})
}
fn spmt_symbol_issue(def:&StrategyDef,ticker:&str)->Option<Issue> {
    use crate::books::spmt;
    if def.book!=spmt::BOOK || !spmt::reviewed_daily(def.id) || spmt::admitted_symbol(def.id,ticker) {return None;}
    Some(Issue{status:"needs_data",note:format!("The S&P Market Timing Course signals on SPY with the CBOE VIX and NYSE TRIN/advancers. This review admits SPX and SPY (CBOE VIX, NYSE breadth) and NIFTY 50 (India VIX for the close-only VIX rules; the NSE TRIN / advancing-issues runs are held, thresholds not calibrated for India) as a declared application; VIX (2) and VIX (7) read the VIX open and stay on SPX/SPY. {ticker} is outside that reviewed scope; no result is published.")})
}
fn ptjmt_symbol_issue(def:&StrategyDef,ticker:&str)->Option<Issue> {
    if def.book!="ptjmt" || !crate::books::ptjmt::reviewed_daily(def.id) || crate::books::ptjmt::admitted_symbol(def.id,ticker) {return None;}
    Some(Issue{status:"needs_data",note:format!("PTJ Market Timing traded S&P 500 futures on the OEX VIX and NYSE breadth. This review admits actual SPX and SPY for the VIX and NYSE-breadth rules, NIFTY 50 with India VIX for CVR II and III only; the NSE-breadth versions are held (thresholds not calibrated for India). {ticker} is outside that reviewed scope; no result is published.")})
}
fn ss_symbol_issue(def:&StrategyDef,ticker:&str,native_stock:bool)->Option<Issue> {
    use crate::books::ss;
    if def.book!=ss::BOOK || !ss::reviewed_daily(def.id) || native_stock || ss::admitted_symbol(def.id,ticker) {return None;}
    Some(Issue{status:"needs_data",note:format!("Street Smarts traded futures (and stocks for some setups). This review admits SPX, NDX, DJI, SPY, NIFTY 50 and NIFTYBANK for the futures setups; the A/D Market Buster only on SPX/SPY with NYSE breadth (the NSE-breadth version is held, thresholds not calibrated for India). {ticker} is outside that reviewed scope; no result is published.")})
}
fn ws5_symbol_issue(def:&StrategyDef,ticker:&str)->Option<Issue> {
    use crate::books::ws5;
    if def.book!=ws5::BOOK || !ws5::TIMING.contains(&def.id) || ws5::admitted_symbol(def.id,ticker) {return None;}
    Some(Issue{status:"needs_data",note:format!("The 2003 seminar times the S&P 500 (SPX signal, SPY/S&P traded); its Non-VIX thresholds for NDX, XBD and SOX differ from the default. This review admits actual SPX and SPY (CBOE VIX) and NIFTY 50 (India VIX) as a declared application. {ticker} is outside that reviewed scope; no result is published.")})
}
/// `stock_us`: None = not an individual stock; Some(true) = a US stock (CBOE VIX); Some(false) = other or unknown market.
fn ats_symbol_issue(def:&StrategyDef,ticker:&str,stock_us:Option<bool>)->Option<Issue> {
    use crate::books::ats;
    if def.book!=ats::BOOK || !ats::reviewed_daily(def.id) {return None;}
    let ok=match stock_us {Some(us)=>def.id!="ats_cvr_equities" || us, None=>ats::admitted_symbol(def.id,ticker)};
    if ok {return None;}
    Some(Issue{status:"needs_data",note:format!("Connors on Advanced Trading Strategies traded S&P 500 futures and US stocks. This review admits SPX and SPY for the CVR I, NYSE TRIN and NYSE breadth rules, also NIFTY 50 with India VIX for CVR II and III (the NSE TRIN / NSE breadth versions are held, thresholds not calibrated for India), SPX/NDX/DJI/SPY/NIFTY 50/NIFTYBANK for the HV Thrust, and CVR I on stocks only for US stocks and US indices/ETFs with the CBOE VIX (CVR I reads the VIX open; India VIX opens repeat the previous close). {ticker} is outside that reviewed scope; no result is published.")})
}
/// Hedge Fund Manager CHADTP and the PTJ Options & Volatility CVR I sides / HV Thrust (connors/hfm-admission).
fn hfm_otvt_reviewed(def:&StrategyDef)->bool {
    (def.book==crate::books::hfm::BOOK && crate::books::hfm::reviewed_daily(def.id)) || (def.book==crate::books::otvt::BOOK && crate::books::otvt::reviewed_daily(def.id))
}
fn hfm_otvt_symbol_issue(def:&StrategyDef,ticker:&str)->Option<Issue> {
    if !hfm_otvt_reviewed(def) || crate::books::hfm::admitted_symbol(def.id,ticker) || crate::books::otvt::admitted_symbol(def.id,ticker) {return None;}
    Some(Issue{status:"needs_data",note:format!("Investment Secrets of a Hedge Fund Manager and the PTJ Options & Volatility Trading volume traded S&P 500 futures. This review admits CHADTP on NYSE breadth for SPX and SPY (the NSE-breadth CHADTP is held, thresholds not calibrated for India), CVR I buys / sells for SPX and SPY with the CBOE VIX (CVR I reads the VIX open; India VIX opens repeat the previous close), and the HV Thrust for SPX/NDX/DJI/SPY/NIFTY 50/NIFTYBANK. {ticker} is outside that reviewed scope; no result is published.")})
}
/// User decision (2026-10-21): every NSE breadth / TRIN mapping of the NYSE-calibrated Connors rules is held; US versions run.
pub fn nse_breadth_held(def:&StrategyDef,ticker:&str)->Option<Issue> {
    let held=matches!(def.id,"ats_trin_thrust_nse"|"ats_padi_nse"|"ptjmt_trin_reversals_nse"|"ptjmt_trin_reversals_20_nse"|"ptjmt_trin_thrusts_nse"|"ptjmt_padi_nse"|"ss_ad_market_buster_nse"|"hfm_chadtp_nse")
        || (ticker=="NIFTY 50" && matches!(def.id,"spmt_trin_1"|"spmt_trin_2"|"spmt_trin_3"|"spmt_trin_4"|"spmt_trin_5"|"spmt_nyse_adv"));
    held.then(||Issue{status:"needs_data",note:"Held: this rule uses the book's NYSE TRIN/advance-decline thresholds; NSE breadth behaves differently (e.g. TRIN median 0.70 vs NYSE 0.93; fires 2-3x more often), so the thresholds are not calibrated for India. Not published until recalibrated and reviewed.".into()})
}
fn reviewed_adaptive_rule(def:&StrategyDef)->bool {
    def.book=="kftsm" && member(def.id,&["kftsm_kama","kftsm_adapt_exp","kftsm_vidya","kftsm_adapt_r2","kftsm_adapt_rsi","kftsm_swg","kftsm_swing","kftsm_raschke_first_cross","kftsm_outside_day","kftsm_fc_bb_abx"])
}

#[cfg(test)]
mod short_course_policy_tests {
    use super::*;
    #[test]
    fn literal_momentum_applications_admit_only_exact_stock_ids() {
        for &id in crate::books::kfsc::MOMENTUM_STOCK_IDS {
            let d=crate::books::kfsc::catalogue().into_iter().find(|d|d.id==id).unwrap();assert!(reviewed_cash_rule(&d));
            assert_eq!(policy(&d).allowed_types,&["individual_stock"]);assert!(check_native(&d,"AAPL",&NativeEvidence::us_test_fixture()).is_none());
            assert!(metadata_json(&d).contains("source_convention"));
            for symbol in ["NIFTY 50","SPX","SPY","QQQ"] {assert!(check(&d,symbol,false).is_some());}
            let mut other=d;other.book="kftsm";assert!(!reviewed_cash_rule(&other));
        }
        let d=crate::books::kfsc::catalogue().into_iter().find(|d|d.id=="kfsc_stoch").unwrap();assert!(!reviewed_cash_rule(&d));assert_eq!(policy(&d).requirement,"source_review_required");
    }
    #[test]
    fn exact_short_course_rules_require_actual_stocks_and_do_not_widen_other_sources() {
        for &id in crate::books::kfsc::REVIEWED_STOCK_IDS {
            let d=crate::books::kfsc::catalogue().into_iter().find(|d|d.id==id).unwrap();
            assert!(reviewed_cash_rule(&d));let p=policy(&d);assert_eq!(p.allowed_types,&["individual_stock"]);
            assert!(p.source_basis.contains("source convention"));assert!(p.source_basis.contains("idealized"));
            assert!(check_native(&d,"AAPL",&NativeEvidence::us_test_fixture()).is_none());
            for symbol in ["NIFTY 50","SPX","SPY","QQQ"] {assert!(check(&d,symbol,false).is_some());}
            let mut wrong=d;wrong.book="kftsm";assert!(!reviewed_cash_rule(&wrong));
        }
        for id in ["kfsc_regression_band","kfsc_stoch"] {
            let d=crate::books::kfsc::catalogue().into_iter().find(|d|d.id==id).unwrap();
            assert!(!reviewed_cash_rule(&d));assert_eq!(policy(&d).requirement,"source_review_required");
        }
    }
}

/// Reviewed keys include both the book and the exact rule identifier. Adding a
/// method to a familiar book cannot inherit admission without a source review.
pub fn policy(def:&StrategyDef)->Policy {
    let mut p=base_policy(def);
    p.native_currency_constraint=match (def.book,def.id) {
        ("afactor","afactor_baseline"|"afactor_spx_down")=>Some("Alvarez Factor workbook p.13 filters the previous stock close above USD 10. The runner compares a literal price parameter with the INR close, which is not the same economic filter."),
        ("jc5dm","jc5dm_long")=>Some("Five Day Momentum p.10 requires the US stock price above USD 50 and pp.11-12 use a 1/16 USD stock tick. The runner's literal values do not establish an INR threshold or the actual NSE stock tick."),
        ("jc5dm","jc5dm_short")=>Some("Five Day Momentum p.10 requires the US stock price above USD 40 and pp.11-12 use a 1/16 USD stock tick. The runner's literal values do not establish an INR threshold or the actual NSE stock tick."),
        ("r3r4","r3r4_lowprice"|"r3r4_lowprice_scale")=>Some(R3R4_LOWPRICE_HELD),
        ("ats","ats_cbp")=>Some("Advanced Trading Strategies Chapter 11 filters US momentum stocks above USD 20. The executable min_price parameter is compared directly with INR stock prices; a local currency specification is missing."),
        ("ats","ats_15min_adx_breakout_equities")=>Some("Advanced Trading Strategies p.164 requires a US equity price above USD 40 and stock execution ticks. Comparing a literal min_price with INR prices or substituting a generic tick does not preserve this source rule."),
        _=>None,
    };
    if p.native_currency_constraint.is_some(){p.source_price_unit=Some("USD");}
    p
}
/// User decision 2026-10-21: stock rules whose only stock hold was the book's USD price filter run on US stocks
/// (the filter is applied in USD as printed); Indian stocks stay held.
pub fn usd_price_rule_on_us_stock(def:&StrategyDef,market:Market)->bool {
    market==Market::UnitedStates && matches!((def.book,def.id),("afactor","afactor_baseline"|"afactor_spx_down")|("r3r4","r3r4_lowprice"|"r3r4_lowprice_scale"))
}
/// The R3/R4 Low Priced rules ARE the USD 1-15 band: they run on US stocks only (refusal text for every other instrument).
pub const R3R4_LOWPRICE_HELD:&str="R3/R4 Low Priced Rules are defined by the stock closing above USD 1 and below USD 15: the dollar price band is the strategy itself, so it runs only on actual US stocks. It stays held on Indian stocks (an INR 1-15 band would be a different strategy, not the book's) and on indices/ETFs. No results are published.";
/// User decision 2026-10-21: afactor_baseline runs on Indian stocks with the USD price (> 10) filter omitted (labelled);
/// the share-volume filter (> 100,000 shares) is kept as printed.
pub fn inr_price_filter_omitted(def:&StrategyDef,market:Market)->bool {
    market==Market::India && def.book=="afactor" && crate::books::afactor::stock_rule(def.id)
}
fn base_policy(def:&StrategyDef)->Policy {
    if reviewed_kestner_stock(def) && crate::books::lkqts::macs_candidate(def.id) {
        let mut p=stock("Kestner printed207 MACS, exact close-SMA n/4n for n1..20 and RAW native-decimal vote comparisons. Detailed-prose inclusive score>=60/<=40 is the named primary; strict summary and systems-on-long-trades equality ambiguity remain disclosed. Daily pair ties contribute zero long votes as a declared convention. Complete80 actual closes/first state79/fill80, genuine prefix, flat selected start and next actual open opposite reversal are declared research conventions. Both fixed AAPL1990–2001/RELIANCE2001–2010 full-prefix source comparisons matched unchanged exact references across all20 pairs with zero mismatches. Shared-author mechanical corroboration is not blind independence. Fixed AAPL1990-01-01..2001-12-31 and RELIANCE2001-01-01..2010-12-31: MACS and TIAR each matched four ordinary normalized-account cases (fees/slippage0/0 and3/2bps per side, rollover0) and16 direct production-function responses (detail, readiness, selected summary, singleton optimizer) against unchanged source/account references with zero mismatches (combined32responses/eightaccounts). Selected API states and earlier full-prefix source evidence have separate scope. HTTP transport/request-matrix, browser and deployment are separate acceptance gates. Every requested run still needs its own source/data/account checks; data readiness alone does not certify account support. Source agreement and these stock checks do not reproduce original mixed-instrument volatility-dollar sizing or book performance. Actual server-qualified single stocks only: no index/ETF/derivative/basket substitution, quote-unit/adjustment/calendar/borrow/margin/financing/dividend/historical-membership certification. Terminal open marks stay outside closed trade stats and retain separately labelled legacy liquidation-cost estimates.");
        p.execution_scope="single_symbol_stock_price_research";p.market_adaptation=true;return p;
    }

    if reviewed_kestner_stock(def) && crate::books::lkqts::tiar_candidate(def.id) {
        let mut p=stock("Kestner, Quantitative Trading Strategies (2003), printed222/227. Exact fixed TIAR step5 means strict close>close5>close10>close15 for long, reverse for short; equality has no new direction. Source levels recur; no crossover requirement. Genuine full prefix, first state15/earliest next actual open16, flat selected start and Chapter7 completed-close/next-actual-open opposite reversal are declared Chapter8 research conventions. Fixed AAPL1990–2001/RELIANCE2001–2010 source states/ledgers, four ordinary accounts at fees/slippage0/0 and3/2bps per side (rollover0), and16 production-function responses matched unchanged source/account references with zero comparison mismatches. Selected API states are checked; prior genuine-prefix source evidence remains separate. This shared-author mechanical corroboration is not blind independence. HTTP transport/request-matrix, browser and deployment are separate acceptance gates. Every requested run still requires source/data/account checks. This is single actual-stock price research, not original mixed-instrument volatility-dollar sizing or book dollar-performance reproduction. No index, ETF, derivative or basket substitution; complete calendars, quote adjustments, historical membership, borrow, margin, financing and dividends remain unverified. Terminal open marks stay outside closed-trade statistics; ordinary accounting retains separately disclosed estimated liquidation costs.");
        p.execution_scope="single_symbol_stock_price_research";p.market_adaptation=true;return p;
    }

    if reviewed_kestner_stock(def) && crate::books::lkqts::sob_candidate(def.id) {
        let mut p=stock("Kestner, Quantitative Trading Strategies (2003), printed185/190/191. Exact fixed SOB80/40/20, profile sob_close_max80_current_inclusive_channel40_tie_inclusive_strict20. The formal strict20 entry paragraph conflicts with the Intel exactly+20/-20 narrative; this named interpretation retains that conflict. Current-inclusive complete80-close horizons and40-statistic channel, inclusive ties, equal high/low horizons neutral0, initialization79/118/119, genuine prefix, flat selected start, Chapter7 completed-close/next-actual-open adaptation and opposite-entry reversal are declared conventions. No same-side pyramiding, independent stop or fabricated terminal exit. Selected actual AAPL/RELIANCE source events and four zero/nonzero-cost ordinary normalized accounts matched separate source/account references. Every requested run still needs its own source/data/account checks. This is individual-stock percentage research, not original mixed-instrument volatility-dollar sizing, a funded/risk account or published book-performance reproduction. No index, ETF, derivative or basket substitution; quote adjustments, complete calendars, historical membership, borrow, margin, financing and dividends remain unverified. Terminal open marks stay outside closed-trade statistics; retained ordinary accounting discloses an estimated liquidation-cost side.");
        p.execution_scope="single_symbol_stock_price_research";p.market_adaptation=true;return p;
    }
    if reviewed_kestner_stock(def) && crate::books::lkqts::next_candidate(def.id) {
        let mut p=stock("Kestner, Quantitative Trading Strategies (2003), printed41,57,66,68,139,155,165,170,175. Only fixed volatility100/2, declared Wilder-RMA14 level35/65 and the two named MACD coefficient profiles are reviewed here; source rules and declared conventions are separated in docs/kestner-next-stock-candidates.md. Completed-close signals fill at the next actual stock open, with genuine full-prefix indicators and a flat selected start. Volatility uses current-inclusive100-change population variance and strict squared thresholds. RSI smoothing, arithmetic14-change seed, flat50 and level startup are declared conventions. Period MACD2/13,2/27 conflicts with the printed literal .15/.075; both named profiles retain a declared .2 signal, first-close seed and increment-form EMA, without choosing a winner from returns. Selected AAPL/RELIANCE source-event ledgers matched separate references. Volatility/MACD normalized accounts matched at zero and nonzero costs; the checked RSI accounts remain withheld because the model becomes nonpositive. Every requested account still requires its own calculation checks. These are individual-stock percentage research applications, not original volatility-dollar sizing or published performance reproduction. No index, ETF, derivative, basket, historical membership, complete exchange calendar, quote adjustment, borrow, margin, financing or dividend certification. Terminal open marks are not closed trades and retain separately disclosed estimated liquidation costs.");
        p.execution_scope="single_symbol_stock_price_research";p.market_adaptation=true;return p;
    }
    if reviewed_kestner_stock(def) {
        let mut p=stock("Kestner, Quantitative Trading Strategies (2003), printed139–140,145,150: only channel40/20, SMA10/40 and momentum80 at fixed source settings. Source and declared conventions: docs/kestner-stock-candidates.md. Completed-close signals fill at the next actual stock open; selected start is flat with genuine earlier indicator history. Inclusive channel bounds/ties, exit-first collision priority and SMA equality edges are declared interpretations. Independent selected AAPL/MSFT/RELIANCE/HDFCBANK source-event and normalized percentage-account comparisons do not reproduce original mixed-instrument volatility-scaled dollar sizing, source populations or book performance. Actual individual stocks only; no index, ETF, derivative or basket substitution. Price adjustments, complete exchange calendars and historical membership remain qualified. Shorts are hypothetical without certified borrow, margin, financing or dividends. Terminal open marks stay outside closed-trade statistics and include an explicitly estimated liquidation-cost side in the retained ordinary model.");
        p.execution_scope="single_symbol_stock_price_research";p.market_adaptation=true;return p;
    }
    if reviewed_dema_index(def) {
        let mut p=index(crate::books::hbqts::reviewed::DEMA_INDEX_BASIS);
        p.execution_scope="single_symbol_index_timing_research";p.market_adaptation=true;return p;
    }
    if def.book=="hbiab" && crate::books::hbiab_reviewed::reviewed(def.id) {
        let mut p=requirement("stock_or_equity_etf", &["individual_stock","etf"], crate::books::hbiab_reviewed::basis(def.id));
        p.execution_scope="single_symbol_source_account";p.market_adaptation=true;return p;
    }
    if def.book=="kfsc" && crate::books::kfsc::reviewed_hv_rule(def.id) {
        let mut p=stock("Kaufman, A Short Course in Technical Trading, printed 70, 83, 243–255: one selected four-rule high-volatility stock continuation subsystem. Source and declared reconstruction: library/kfsc/REVIEW-2026-10-19-HV.md. Fixed 65 genuine inclusive simple true ranges, strict factor 2, upper/lower deciles (explicit correction of printed upper 90 percent), actual Friday ban, idealized finalized signal-close entry and next genuine session OPEN exit chosen from Figure 17.4 caption despite shortly-after prose. MOC/MOO first/last 60 seconds do not certify OHLC fills. Required warmup and selected native-session calendar coverage must match actual stock observations; missing sessions are held. Terminal entries stay open. Actual individual stocks only; no index, ETF, basket, target/stop or source capital/sizing inheritance. Normalized unit-allocation price-return research with modeled per-side costs; hypothetical shorts lack certified borrow/margin/financing. Selected AAPL/RELIANCE ledgers do not reproduce AOL figures, original stock population, reliability or book performance; quotes/adjustments and historical membership remain qualified.");
        p.market_adaptation=true;return p;
    }
    if def.book=="kfsc" && crate::books::kfsc::reviewed_momentum_rule(def.id) {
        let mut p=stock("Kaufman, A Short Course in Technical Trading, printed57-58,70,201-205,230-233: only the exact literal .0476/.0243/.10 MACD crossing or10-day momentum double-smoothed with literal .0476. These constants are not exact1/(N+1) and conflict with the MACD general2/(N+1) formula. Source and chosen conventions: library/kfsc/REVIEW-2026-10-15-MOMENTUM.md. Actual individual stocks only; selected AAPL/RELIANCE research applications, not the AOL/Microsoft charts, historical populations or source performance. All genuine earlier source history is retained; first actual close seeds MACD price smoothings, first actual10-day momentum seeds both DSM passes. Flat selected start waits for a subsequent crossing/turn; DSM equality holds the last nonzero slope, first slope initializes direction. Completed-close same-close fills and opposite-signal reversals are declared idealized conventions; terminal entries remain open. Shorts are hypothetical without certified borrow, margin or financing. Natural signal exits do not reproduce the discretionary combined methods or full game capital/risk programme. Quote adjustments and historical membership remain qualified. No index, ETF or basket substitution; stochastic regime selection remains held.");
        p.market_adaptation=true;return p;
    }
    if def.book=="kfsc" && crate::books::kfsc::reviewed_stock_rule(def.id) {
        let mut p=stock("Kaufman, A Short Course in Technical Trading: only the exact 80/10 SMA agreement, 20/5 divided percentage band and 20/2 literal price-level Excel sample-SD interpretation. Printed52,70,97-102,123-124; source/hash and selected-stock evidence scope: library/kfsc/REVIEW-2026-10-15-THREE.md. Actual individual stocks only; these are declared selected-stock research applications, not original AMR/Amazon charts, historical populations or book-performance reproduction. The fixed source convention explicitly chooses completed-close signals with idealized same-close fills, flat start, full observed warmup, one position, equality and opposite-band reversal conventions. The Bollinger prose/formula conflict is disclosed rather than silently resolved. Terminal-close entries remain open; estimated open liquidation costs are separate from filled exits. Band shorts are hypothetical without certified borrow, margin or financing. Adjusted quote sources and historical membership remain qualified; no index/ETF or basket portfolio admission is inherited.");
        p.market_adaptation=true;return p;
    }
    if def.book=="sbma" && crate::books::sbma::four_reviewed_stock_rule(def.id) {
        let mut p=stock("Burns Moving Averages 101, physical PDF58-60/64/67: four exact long/cash moving-average pairs. Source rules and full chronological selected AAPL/RELIANCE ledgers independently reviewed; native host/API integration checked separately in library/sbma/INDEPENDENT-FOUR-LONG-REVIEW-2026-10-15.md. Actual individual stocks only. Full supplied earlier history is retained; EMA first-n SMA seed, strict equality, flat test start and idealized signal-close/next-actual-open fills are declared conventions. Truncating EMA150 warmup changes selected signals. Original charts, historical stock population, capital sizing and book performance are not reproduced; quote adjustments, membership and execution remain qualified. Final open liquidation estimates are separate from actual filled exit orders.");
        p.market_adaptation=true;return p;
    }
    // Exact source/account composition only, before generic multi-instrument
    // review handling. Basket execution has no corresponding event contract.
    if def.book=="elcim" && def.id==crate::books::elcim_account::ID {
        let mut p=stock("Elder Come Into My Trading Room pp158-161,171-180,220-227 and primary Study Guide answers55/77-80. Independently accepted protected daily Impulse research composition on actual selected AAPL/RELIANCE: next actual open, completed-week EMA26, daily EMA13/MACD12-26-9, unrounded fixed SafeZone10/2 and source2%/6% cost-aware normalized cash/risk account. Fixed worksheet lookback omits p179 discretionary turning-point resets. Source-account-aware page/export basis and exact default guards remain mandatory. Actual stocks only; no index/ETF or basket inheritance. Hypothetical shorts, fractional units, continuous stops, supplied adjustment qualification and flat selected start are declared conventions. Gross retains the same quantities; open marks pay actual entry costs only. No source-study reproduction, monetary capital, real lot/borrow or tax certification. See library/elcim/INDEPENDENT-BACKEND-REVIEW-2026-10-15.md and API-FIELD-CONTRACT.md.");
        p.execution_scope="single_symbol_source_account";p.market_adaptation=true;return p;
    }
    if def.book=="sbma" && crate::books::sbma::next_reviewed_stock_rule(def.id) {
        let mut p=stock("Burns Moving Averages 101, physical PDF54-60 and67: exact source-default selected-stock research configurations. See library/sbma/INDEPENDENT-NEXT-REVIEW-2026-10-15.md. EMA SMA seeding, flat test start, strict ties and idealized same-close/next-observed-open fills are declared implementation conventions. Original chart populations/performance are not reproduced. Actual stocks are required; current snapshots do not establish unbiased historical membership. The 200-SMA short is a hypothetical price-return study without certified borrowing/margin/overnight feasibility. Platform final open-mark liquidation estimates remain separate from filled orders.");
        p.market_adaptation=true; return p;
    }
    if def.book=="sbma" && member(def.id,&["sbma_sma50_long","sbma_x_5s20s"]) {
        let mut p=stock("Burns Moving Averages 101 (2015), physical PDF pages 10/16/42/58/60/63/67. Default 50-SMA long and 5-SMA/20-SMA long/cash contracts independently source-checked and compared on selected actual AAPL/RELIANCE data at 0/5 bps and declared idealized-close/next-actual-open fills. These are deliberate individual-stock research applications, not original Monster-stock selection, ETF/index chart or book portfolio/performance reproduction. Flat test start, full actual SMA warmup and terminal open/pending orders retained. Quote adjustments and historical population remain qualified. See library/sbma/INDEPENDENT-REVIEW-2026-10-15.md.");
        p.market_adaptation=true;return p;
    }
    // Keep this exact rule before the generic reviewed multi-instrument branch.
    if def.book=="radge" && def.id=="radge_oc_long" {
        let mut p=index("Successful Stock Trading Chapter1, PDF17 / printed16: buy each session open and exit that session close. Review scope: library/radge/REVIEW-2026-10-15.md. Only the plain stop-free method was independently checked on actual selected NIFTY50 and policy-qualified SPX prices. This is a declared cash-index price-return research application, with modeled per-side return deductions. The source index/history and original performance are not reproduced; vendor opening-print authenticity and executable trading are not certified. Open-equals-close remains an observed flat session, not proof of missing data. Existing price validation, symbol limits and open cutoffs remain required; no stock, ETF, futures, stop or money-sizing permission is inherited.");
        p.market_adaptation=true;return p;
    }
    if def.book=="bllstr" && crate::books::bllstr::reviewed_daily_index(def.id) {
        let mut p=index("Brock, Lakonishok and LeBaron (1992), exact zero-band 1/50 VMA/FMA defaults. Source and complete independent Nifty 2000-2026 ledgers reviewed in library/bllstr/REVIEW-2026-10-15.md. Actual cash-index close prices are a declared research application; other instruments cannot substitute. Signal-close fills, flat test start, fixed units, equality/terminal conventions and normalized-allocation costs with open liquidation estimates are explicit model assumptions. Paper conditional log-return observations are separate from hypothetical account results; original Dow study, financing/dividends and executable shorting are not reproduced. Other IDs inherit no admission.");
        p.market_adaptation=true;return p;
    }
    if def.book=="hbqts" && reviewed_cash_rule(def) {
        if def.id=="hbqts_cmo" {
            let mut p=etf("QTS Figure20.2, printed pp263-264, was used on XLB, an actual Materials ETF. The bounded reviewed ledger uses actual SPY ETF prices as a declared ETF research application with literal companion defaults, not the source XLB walk-forward study or its optimized results. Individual stocks and cash indices cannot substitute for ETF prices. See library/bandy/REVIEW-2026-10-13.md.");
            p.market_adaptation=true;return p;
        }
        let mut p=stock("Bandy QTS (2007), exact companion AFL and both local errata reviewed in library/bandy/REVIEW-2026-10-13.md. Bounded individual-stock research on actual AAPL and RELIANCE; these samples do not establish historical source-population membership. Literal signal timing and defaults with declared indicator seeds; full-history arrays and chronological ledgers independently checked. No source portfolio sizing or reported performance is reproduced. Independent source review and final compiled-ledger evidence are recorded there; publisher acceptance remains required.");
        p.market_adaptation=true;return p;
    }
    if def.book=="spmt" && crate::books::spmt::REVIEWED_AUX.contains(&def.id) {
        let mut p=requirement("declared_index_timing", &["index","etf"], "S&P Market Timing Course (2007), VIX, TRIN, NYSE advancers and EOM put-sale underlying rules (pp.50-72, 87-111, 133-141, 151-154, 168-169) reviewed in connors/spmt2-admission/SOURCE-CONTRACT.md with independent chronological SPY/SPX/NIFTY 50 ledgers. The book signalled on SPY with the ^VIX and NYSE TRIN/advancers and traded the E-mini. SPY uses actual ETF prices and the CBOE VIX; SPX cash-index timing is a declared application; NIFTY 50 with India VIX (close-only VIX rules; VIX (2) and VIX (7) read the VIX open, which on India VIX repeats the previous close, and stay US-only) and with NSE TRIN / NSE advancing issues at the book's unchanged thresholds is a declared adaptation, not book data. NYSE breadth in the vault ends 2020-02-10. The EOM underlying exit (RSI(2) > 65, out at the next open) is read from the p.169 example. Close fills, one position, no entry on the last bar and the open terminal position are explicit conventions. Cash-index shorts are hypothetical exposure. No E-mini dollars, option P&L or book performance is claimed.");
        p.market_adaptation=true; return p;
    }
    if def.book=="spmt" && crate::books::spmt::reviewed_daily(def.id) {
        let mut p=requirement("declared_index_timing", &["index","etf"], "S&P Market Timing Course (2007), exact daily rules reviewed in library/connors-spmt/REVIEW.md and source-based independent SPY/Nifty ledgers. SPY uses actual ETF prices; cash-index timing is a declared research application and cash-index shorts are hypothetical exposure. Indicator initialization, one sequential position and flat test start are explicit research conventions. These retained STC IX duplicates do not add new algorithms. No E-mini contract dollars, option P&L, portfolio allocation or book-performance reproduction is claimed.");
        p.market_adaptation=true; return p;
    }
    if def.book=="vixr" && crate::books::vixr::reviewed_daily(def.id) {
        let mut p=requirement("declared_index_timing", &["index","etf"], "Trading Connors VIX Reversals (2001), CVR 1-11 daily rules (book pp.19-81) reviewed in pilot-vixr/SOURCE-CONTRACT.md with independent chronological SPY/SPX/NIFTY 50 ledgers. The book traded one S&P 500 futures contract on the OEX-based VIX (today's VXO), 1993-2001. SPY uses actual ETF prices and the CBOE VIX; SPX cash-index timing is a declared research application; NIFTY 50 with India VIX is admitted only for CVR 2, 3 and 6. Cash-index shorts are hypothetical exposure. Close fills, one position with stop-and-reverse, the restart of the hold count on a same-direction exit-bar signal and the open terminal position are explicit conventions. Book exit ranges (2-3 days) and source inconsistencies are disclosed open items, not resolved. No futures contract dollars, VXO, option P&L, multi-contract portfolio or book-performance reproduction is claimed.");
        p.market_adaptation=true; return p;
    }
    if def.book=="ptjmt" && crate::books::ptjmt::reviewed_daily(def.id) {
        let mut p=requirement("declared_index_timing", &["index","etf"], "The Best of the Professional Traders Journal: Market Timing (Connors, 1997-98), CVR I/II/III/V, TRIN Reversals (30/20-day), TRIN Thrusts and PADI (book pp.3-42) reviewed in connors/ptjmt-admission/SOURCE-CONTRACT.md with independent chronological SPY/SPX/NIFTY 50 ledgers. The book traded one S&P 500 futures contract on the OEX VIX (today's VXO) and NYSE TRIN / advance-decline data. SPY uses actual ETF prices and the CBOE VIX (VXO only with the labelled p.src=1 option); SPX cash-index timing is a declared research application; NIFTY 50 with India VIX is admitted for CVR II and III only; the NSE TRIN / NSE advance-decline versions on NIFTY 50 are declared adaptations, not book data. Cash-index shorts are hypothetical exposure. Close fills, one position with stop-and-reverse, the restart of the hold count on a same-direction exit-bar signal and the open terminal position are explicit conventions. Book exit ranges and source inconsistencies are disclosed open items, not resolved. No futures contract dollars, option spreads or book-performance reproduction is claimed.");
        p.market_adaptation=true; return p;
    }
    if def.book==crate::books::ss::BOOK && crate::books::ss::STOCK_ALSO.contains(&def.id) {
        let mut p=requirement("index_etf_or_stock", &["index","etf","individual_stock"], "Street Smarts (Raschke & Connors, 1995), daily rules reviewed in connors/ss-admission/SOURCE-CONTRACT.md with independent chronological ledgers (stocks: stocks-admission). The book shows this setup on futures and on an individual stock (Turtle Soup Plus One: Delta/Texas Instruments p.41-42; Holy Grail: Citicorp p.81; ID/NR4: Abbott p.148; Historical Volatility: Micron p.157). Actual stocks are admitted (user decision 2026-10-21); index runs stay on SPX, NDX, DJI, SPY, NIFTY 50 and NIFTYBANK as a declared application, not the authors' test. 1 tick = 0.01% of price on an index/ETF; on an actual stock the stock's real tick, 0.05 INR on NSE and 0.01 USD on US stocks (adaptation: the book used 1/8-dollar ticks). Daily-bar fill conventions, the labelled exits and every [Not from the book] option are disclosed. Cash-index and stock shorts are hypothetical exposure. No futures dollars or book performance is reproduced.");
        p.market_adaptation=true; return p;
    }
    if def.book==crate::books::ss::BOOK && crate::books::ss::reviewed_daily(def.id) {
        let stock=crate::books::ss::STOCK_SHOWN.contains(&def.id);
        let mut p=if stock {
            requirement("index_etf_or_stock", &["index","etf","individual_stock"], "Street Smarts (Raschke & Connors, 1995), daily rules reviewed in connors/ss-admission/SOURCE-CONTRACT.md with independent chronological ledgers. The book shows this setup on futures and on individual stocks (2-period ROC: Micron p.64; The Anti: 'all equities and futures markets' p.77; Three-Day Unfilled Gap: Motorola pp.99-101). Actual stocks are admitted; actual index/ETF runs are a declared application, not the authors' test. 1 tick = 0.01% of price on an index/ETF; on an actual stock the stock's real tick, 0.05 INR on NSE and 0.01 USD on US stocks (adaptation: the book used 1/8-dollar ticks). Exits the chapter leaves to judgement ('trail a stop') use the labelled 2-bar channel from Appendix p.232. No futures dollars or book performance is reproduced.")
        } else {
            requirement("declared_index_timing", &["index","etf"], "Street Smarts (Raschke & Connors, 1995), daily rules reviewed in connors/ss-admission/SOURCE-CONTRACT.md with independent chronological SPY/SPX/NIFTY ledgers. The book traded S&P, bond, currency and grain futures (Moore studies: 25 US futures 1986-1995). Actual SPX, NDX, DJI, NIFTY 50 and NIFTYBANK cash-index runs and actual SPY are a declared application; cash-index shorts are hypothetical exposure. 1 tick = 0.01% of price. Daily-bar fill conventions, the labelled 2-bar channel trail (Appendix p.232) and every [Not from the book] option are disclosed; NIFTY opens before late 2010 lie close to the previous close (official NSE data, kept). No futures dollars, contract rolls or book performance is reproduced.")
        };
        p.market_adaptation=true; return p;
    }
    if def.book==crate::books::cwin::BOOK && crate::books::cwin::reviewed_daily(def.id) {
        let mut p=requirement("index_etf_or_stock", &["index","etf","individual_stock"], concat!("The TradingWindows Strategy (Connors, 2002) and its course pages, rules reviewed in connors/cwin-admission/SOURCE-CONTRACT.md with independent chronological stock/index/ETF ledgers. ",CWIN_INDEX_LABEL!()," Profit taking is discretionary in the book (p31, p41 'there is no mechanical science to this'): the coded exit (1/3 at the first close beyond SMA4 then breakeven stop, 1/3 on a profitable day-2 close, rest at the day-3 close) is a declared mechanical reading of p31/p38/p56 and the course, not the author's exact exit; trailing stops beyond breakeven are not coded. The two-rule exit test is the book's own mechanical exit. Rules that read the open start where SPX/NDX/DJI opens are real (2014/2001/2016); NIFTY opens before 2011 are close to the prior close and are disclosed. Cash-index shorts are hypothetical exposure. 1 unit, prices in points; no futures dollars or book performance are claimed."));
        p.market_adaptation=true; return p;
    }
    if def.book==crate::books::ccl08::BOOK && crate::books::ccl08::reviewed_daily(def.id) {
        let mut p=if crate::books::ccl08::day_trade(def.id) {
            requirement("index_etf_or_stock", &["index","etf","individual_stock"], concat!("Chairman's Club 2008 (TradingMarkets) CR Equity / CRDT Equity day trades, slide rules re-read and checked with independent chronological ledgers (connors/newrules-cc/SOURCE-CONTRACT.md). Limit entry X% beyond the setup close, exit at a 10% stop beyond the fill or at that day's close: exact on daily OHLC. The decks show US stocks (Bloomberg/TradeStation examples) and print only seminar-reported results (S2 s14: 15-25 Sep 2008), which are not reproduced. Actual US and NSE stocks run the rule as printed; the Liquidity Filter and HV formulas are vault definitions [Not from the book]. ",CCL08_INDEX_LABEL!()))
        } else {
            requirement("index_or_etf", &["index","etf"], "Chairman's Club 2008 (TradingMarkets) Session 2 TPS rules (s93, s95-s96), re-read from the slides and checked with independent chronological ledgers (connors/newrules-cc/SOURCE-CONTRACT.md). The deck lists S&P 500 TPS trades (s99); SPX and SPY are the deck's market, other indices/ETFs (NIFTY 50 included) are a declared application. Scale-in weights are shares of the full 1-2-3-4 (or 2-4-6-8) position; cash-index shorts are hypothetical exposure. No book performance is claimed.")
        };
        if crate::books::ccl08::day_trade(def.id) {p.preferred_basket=Some("nifty50");}
        p.market_adaptation=true; return p;
    }
    if def.book==crate::books::ats::BOOK && crate::books::ats::reviewed_daily(def.id) && !CONNORS_INDEX_IDS.contains(&def.id) {
        let timing=!matches!(def.id,"ats_1234"|"ats_8day_hl_reversal"|"ats_smtp"|"ats_two_for_one"|"ats_gipsons");
        let mut p=if timing {
            requirement("declared_index_timing", &["index","etf"], "Connors on Advanced Trading Strategies (1998), daily rules reviewed in connors/ats-admission/SOURCE-CONTRACT.md with independent chronological ledgers. The book traded one S&P 500 futures contract on the 1993-97 CBOE VIX (today's VXO), NYSE TRIN and NYSE advance-decline data. SPY uses actual ETF prices and the CBOE VIX; SPX cash-index timing is a declared application; NIFTY 50 with India VIX is admitted for CVR II and III only; the NSE TRIN / NSE breadth versions on NIFTY 50 and the HV Thrust on SPX/NDX/DJI/SPY/NIFTY 50/NIFTYBANK are declared adaptations. Cash-index shorts are hypothetical exposure. Book exit ranges are coded at the published test value; the HV Thrust has no book exit, so the user's standard exits are labelled options [Not from the book]. No futures dollars or book performance is claimed.")
        } else {
            requirement("index_etf_or_stock", &["index","etf","individual_stock"], concat!("Connors on Advanced Trading Strategies (1998), patterns reviewed in connors/ats-admission/SOURCE-CONTRACT.md with independent chronological index and stock ledgers (stocks-admission). ",STOCK_TICK_LABEL!()," ",BTP_INDEX_LABEL!()," On an index/ETF the one-tick offset stays a fixed 0.05 price unit, so the accuracy gate limits index runs to NSE indices. Where the book gives no exit ('trail your stops'), the user's standard exits are labelled options [Not from the book]; Gipsons keeps the book's exit (c) with a labelled trail. Cash-index shorts are hypothetical exposure. No futures dollars or book performance is claimed."))
        };
        p.market_adaptation=true; return p;
    }
    if def.book==crate::books::alpha::BOOK && def.id=="alpha_trend_6m" {
        let mut p=requirement("index_or_etf", &["index","etf"], concat!("The Alpha Formula (Connors & Cain, 2019/2020) Appendix A1 six-month trend rule, reviewed in connors/rest-admission/SOURCE-CONTRACT.md with independent chronological index/ETF ledgers. ",ALPHA_TREND_LABEL!()," The out-of-market leg (SHY in the book) is idle cash here; vault prices are not dividend-adjusted total returns. The book calls the rule a risk-reduction illustration, not a stand-alone strategy. Cash-index exposure is hypothetical; the book's sector-ETF averages are not reproduced."));
        p.market_adaptation=true; return p;
    }
    if hfm_otvt_reviewed(def) {
        let mut p=requirement("declared_index_timing", &["index","etf"], "Investment Secrets of a Hedge Fund Manager (Connors & Hayward, 1995) CHADTP (Ch.5) and The Best of the Professional Traders Journal: Options Trading and Volatility Trading (Connors, 1999) CVR I buys / sells and HV Thrusts, daily rules reviewed in connors/hfm-admission/SOURCE-CONTRACT.md with independent chronological ledgers. The books traded S&P 500 futures (CHADTP on NYSE advancing/declining issues; CVR I on the OEX VIX, today's VXO). SPY uses actual ETF prices and the CBOE VIX; SPX cash-index timing is a declared application; the NSE-breadth CHADTP on NIFTY 50 (book's fixed +/-400 issues) and the HV Thrust on SPX/NDX/DJI/SPY/NIFTY 50/NIFTYBANK are declared adaptations. The CHADTP .10-point trigger is coded as 0.03% of price. The HV Thrust has no book exit, so the user's standard exits are labelled options [Not from the book]. Cash-index shorts are hypothetical exposure. No futures dollars or book performance is claimed.");
        p.market_adaptation=true; return p;
    }
    if def.book=="btp" && crate::books::btp::STOCK_ADMITTED.contains(&def.id) {
        let mut p=requirement("index_etf_or_stock", &["index","etf","individual_stock"], concat!("Best Trading Patterns I & II (Connors, PTJ 1998/1999), exact daily rules reviewed in btp-admission/SOURCE-CONTRACT.md with independent chronological index/ETF and stock ledgers (stocks-admission). ",STOCK_TICK_LABEL!()," ",BTP_INDEX_LABEL!()," Rules that read the open start where SPX/NDX/DJI opens are real (2014/2001/2016); NIFTY opens before 2011 are close to the prior close and are disclosed, not clamped. Every 'trail your stops' exit in the book is undefined: the coded trailing stop, 2-for-1 half exits and tick offsets marked [Not from the book] are declared conventions. Cash-index and stock shorts are hypothetical exposure. No futures dollars or book performance are claimed."));
        p.market_adaptation=true; return p;
    }
    if def.book=="btp" && crate::books::btp::reviewed_daily(def.id) {
        let mut p=requirement("index_or_etf", &["index","etf"], concat!("Best Trading Patterns I & II (Connors, PTJ 1998/1999), exact daily rules reviewed in btp-admission/SOURCE-CONTRACT.md with independent chronological index/ETF ledgers. ",BTP_INDEX_LABEL!()," Rules that read the open start where SPX/NDX/DJI opens are real (2014/2001/2016); NIFTY opens before 2011 are close to the prior close and are disclosed, not clamped. Every 'trail your stops' exit in the book is undefined: the coded trailing stop, 2-for-1 half exits and tick offsets marked [Not from the book] are declared conventions. Cash-index shorts are hypothetical exposure. Single-stock runs are held: the book's 1/8-USD stock tick offsets have no reviewed INR/NSE or decimal-USD specification. No futures dollars or book performance are claimed."));
        p.market_adaptation=true; return p;
    }
    if def.book==crate::books::ws5::BOOK && crate::books::ws5::reviewed_daily(def.id) {
        let mut p=if crate::books::ws5::TIMING.contains(&def.id) {
            requirement("declared_index_timing", &["index","etf"], "Connors 5-Week Live Web Seminar (2003), Session 4 market timing (Non-VIX and VIX III, PDF pp.106-108, 129-131) reviewed in connors/ws5-admission/SOURCE-CONTRACT.md with independent chronological SPY/SPX/NIFTY 50 ledgers. The seminar times the S&P 500 and trades SPY/S&P futures. SPY uses actual ETF prices and the CBOE VIX; SPX cash-index timing is a declared research application; NIFTY 50 (India VIX for VIX III; the SPX 2.0% Non-VIX threshold unchanged) is a declared application, not the author's test. The 3-point SPY stop is 1.18% of price per point (SPY at $85 in the sizing slide). Close fills, the stop checked before the reversion exit and the open terminal position are explicit conventions; the Non-VIX sell exit 'below' and the VIX III literal 'above' exit are disclosed open items. Cash-index shorts are hypothetical exposure. No futures dollars or performance claim is reproduced.")
        } else {
            requirement("index_etf_or_stock", &["index","etf","individual_stock"], concat!("Connors 5-Week Live Web Seminar (2003), Daily Momentum Windows (Sessions 1-2) and Weekly Windows (Session 3) reviewed in connors/ws5-admission/SOURCE-CONTRACT.md with independent chronological index/ETF and stock ledgers (stocks-admission). ",WS5_STOCK_LABEL!()," ",WS5_INDEX_LABEL!()," Points scale by ATR(90) (pt_mode 2, labelled); the daily-bar path (adverse extreme first), gap fills at the open and the Intraday/Outside Window readings are declared conventions. Rules that read the open start where SPX/NDX/DJI opens are real (2014/2001/2016). Cash-index and stock shorts are hypothetical exposure. The workbook publishes no statistics; no performance claim is reproduced."))
        };
        p.market_adaptation=true; return p;
    }
    if def.book=="chbta2" && member(def.id,&["ch_bta2_ma_stack","ch_bta2_65sma_ravi"]) {
        let mut p=requirement("declared_ohlc_adaptation", &["index","etf","individual_stock"], "Chande, Beyond Technical Analysis (1997), printed pp.20-21 and pp.80,94-97,99. Only the exact stop-free SMA-agreement and RAVI-filtered reversal contracts were source-reviewed and checked against independent chronological ledgers; see library/chande/REVIEW-2026-10-13.md. Actual selected prices are a declared research application, flat at the test start with available historical warmup and one position per symbol. Cash-index short exposure is hypothetical. No futures dollar stops/costs, contract ticks/rollovers, portfolio allocation or source performance are reproduced. Other Chande IDs remain under their existing policy.");
        p.market_adaptation=true;return p;
    }
    if def.book=="ehrs" && crate::books::ehrs::reviewed_sinetrend(def.id) {
        let mut p=requirement("declared_ohlc_adaptation", &["index","etf","individual_stock"], "Ehlers Rocket Science for Traders p.119 identifies Fig.12.1 (pp.120-123) as a complete theoretical automatic signal system before adding a separate Treasury-bond dollar stop. Only the exact reviewed SineTrend configurations are admitted. Trend-only removes Cycle Mode orders without flattening holdings (p.124); CycPart1.15 uses Fig.12.4's trendline (p.126). Source review and selected real-price ledger scope: library/ehlers/REVIEW-2026-10-13.md and REVIEW-2026-10-15-VARIANTS.md. Selected actual daily OHLC is a declared research application; The three additional configurations require exact reviewed CycPart/MaxBarsBack pairs1/50 or1.15/57; alternate settings remain held. The previously reviewed base preserves its parameter contract. Zero-state startup and next-open fills are explicit conventions. No dollar stops, original futures contracts, roll history, portfolio or source-study performance are reproduced. Cash-index short exposure is hypothetical.");p.market_adaptation=true;return p;
    }
    if def.book==crate::books::hpetf::BOOK && crate::books::hpetf::reviewed_daily(def.id) {
        let mut p=requirement("index_etf_or_stock", &["index","etf","individual_stock"], concat!("High Probability ETF Trading seminar manuals (Connors, 2009), Session 2 RSI 14 TPS (pp.9, 17-20) reviewed in connors/newrules-misc/SOURCE-CONTRACT.md with independent chronological index/ETF/stock ledgers. ",NEWRULE_ANY_LABEL!()," The optional 200-day rule is off by default (the seminar shows both); 'closes lower' for the adds is read as below the previous purchase price [Not from the book]. The seminar's results are on a spreadsheet not in the PDF: no performance is claimed."));
        p.preferred_basket=Some("nifty50");p.market_adaptation=true;return p;
    }
    if def.book=="etfss" && crate::connors::ETFSS_REVIEWED.contains(&def.id) {
        let mut p=requirement("index_etf_or_stock", &["index","etf","individual_stock"], concat!("The ETF Sweet Spot Trading System (Connors Research, 2004), conservative 'trade only the 2nd signal' variation (printed p.32, Rules #5-#6 p.10-11, p.15) reviewed in connors/newrules-misc/SOURCE-CONTRACT.md with independent chronological index/ETF/stock ledgers. ",NEWRULE_ANY_LABEL!()," The book itself advises ETFs only (p.12). Book tests SPY/QQQ/IWM/SMH/MDY 1/3/00-6/16/04; only '77.5% of the second signals were profitable' is printed, so no claim is shown."));
        p.preferred_basket=Some("nifty50");p.market_adaptation=true;return p;
    }
    if def.book==crate::books::ststw::BOOK && crate::books::ststw::reviewed_daily(def.id) {
        let mut p=requirement("declared_index_timing", &["index","etf"], "Short Term Trading Strategies That Work (Connors & Alvarez, 2008) Ch.12 VIX RSI Strategy (pp.96-98) reviewed in connors/newrules-misc/SOURCE-CONTRACT.md with independent chronological SPY/SPX ledgers on the CBOE VIX. The book signals on SPY ('or use the SPX'). SPY uses actual ETF prices; SPX cash-index timing is the book's own alternative. The rule reads today's VIX open, so it runs only with the CBOE VIX from 1992 (close-only bars before); India VIX opens repeat the previous close, so NIFTY is not admitted. Close fills, one position, no entry on the last bar and the open terminal position are explicit conventions. No book performance is reproduced.");
        p.market_adaptation=true;return p;
    }
    // Kaufman catch-all; reviewed Connors stock rules keep their index/ETF/stock route below.
    if reviewed_cash_rule(def) && !CONNORS_INDEX_IDS.contains(&def.id) {
        let allowed=if matches!(def.id,"kftsm_month_end"|"kftsm_hirsch"){&["index","etf"][..]}else{&["index","etf","individual_stock"][..]};
        let basis=if reviewed_adaptive_rule(def) {
            "Kaufman, New Trading Systems and Methods Fourth Edition (2005), reviewed book sections and companion CD. Source contracts/defaults and selected-baseline-ledger-check scope are recorded in library/kaufman/REVIEW-2026-10-13-ADAPTIVE.md. Actual selected index/ETF/stock prices are a declared research application. Flat at the test start, actual warmup, one position per symbol, no portfolio sizing. Initial-history conventions, zero-division handling, causal close/next-open orders and outside-day zero-offset stop approximation remain declared assumptions. Cash-index short exposure is hypothetical; source futures dollars, financing, dividends and original study performance are not reproduced."
        } else {
            "Kaufman, New Trading Systems and Methods Fourth Edition (2005), reviewed book sections and companion CD. Source rules/defaults and selected independent trade-ledger checks are recorded in library/kaufman/REVIEW-2026-10-13.md. Actual selected cash/ETF prices are a declared research application. Flat at the test start, full source warmup, one position per symbol, no portfolio sizing. Cash-index short exposure is hypothetical; source futures dollars, financing, dividends and original study performance are not reproduced."
        };
        let mut p=requirement("declared_ohlc_adaptation", allowed, basis);p.market_adaptation=true;return p;
    }
    let id=def.id;
    match def.book {
        "ehmd" if id=="ehmd_mama" => stock("Ehlers MESA Adaptive Moving Averages, S&C19:9 pp.30-35, explicitly tests a long-only MAMA/FAMA crossover on100 stocks. Review and independent ledgers: library/ehlers/REVIEW-2026-10-13.md. Actual individual stocks are required. One selected stock is not the original unnamed100-stock population; configurable MaxBarsBack50 and next-open execution are declared research conventions, and the pooled source performance is not reproduced."),
        "stc9" if member(id,&["f1a","f3a","f4a","f4c","f4d","rsi2_5d"]) => connors_any!("STC IX Session 1 foundation studies concern a pooled US individual-stock population. A native single-stock run uses one actual constituent at a time; it does not reproduce the pooled published counts, universe or performance."),
        "stc9" if member(id,&["tps_stocks","x555","h10"]) => connors_any!("STC IX Sessions 2-3 specify stocks with volatility, ADX and Liquidity Filter population restrictions. Actual constituent OHLC/volume is necessary; this type check does not supply missing liquidity denominators, historical membership or portfolio slots."),
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
        "afactor" if id=="afactor_baseline" => connors_any!("Alvarez Factor workbook pp.13-14 and Stats.xls baseline specify one stock's gap, OHLC and share volume. A selected Indian stock is an instrument/currency adaptation; literal US dollar price thresholds, corporate-action treatment and the absent pooled US universe remain limitations."),
        "afactor" if id=="afactor_spx_down" => connors_any!("Alvarez Factor S&P-down-days market-timing filter (AF_DownSP_RSfilter_Summary.xls, Stats.xls Variations, workbook pp.52-53) on the baseline rules (workbook pp.13-14), reviewed in connors/newrules-misc/SOURCE-CONTRACT.md with independent chronological stock/index ledgers. The S&P 500 down closes come from the SPX on US instruments; on Indian stocks/indices NIFTY 50 stands in for the S&P 500 (declared adaptation). Down closes are counted through the setup day (yesterday) and 'X days' means at least X in a row; default X = 2 [Not from the book]. Windows without a benchmark close on a setup day, or (SPX/NDX/DJI) before real opens, are refused."),
        "afactor" if id=="afactor_portfolio" => portfolio("Alvarez Factor Stats.xls portfolio uses simultaneous stock signals, 1/10 slots and day-level allocation. Independent-symbol returns cannot be compounded into the published portfolio without the verified universe and allocation engine."),
        "r3r4" if member(id,&["r3r4_midcap","r3r4_midcap_scale"]) => stock("R3/R4 Mid Cap Rules (guide pp.6-8, appendices pp.28-29) require a Liquidity Filter between 3 and 30. On an index the filter compares the index with its own volume (about 1,000,000), so the upper bound of 30 is never met and an index/ETF run could only return zero trades; index/ETF runs are therefore refused. Actual stocks only; the 6,000+ US-stock study is not reproduced."),
        "r3r4" if member(id,&["r3r4_base","r3r4_base_scale","r3r4_bigcap","r3r4_bigcap_scale","r3r4_bigcap_deck","r3r4_bigcap_deck_scale","r3r4_midcap","r3r4_midcap_scale","r3r4_lowprice","r3r4_lowprice_scale"]) => connors_any!("R3/R4 guide pp.6-8 and appendices pp.28-29 explicitly require stocks, share-volume liquidity, HV and ADX filters. Native constituent data alone cannot establish the missing benchmark volume or reproduce the 6,000+ US-stock study."),
        "jc5dm" if member(id,&["jc5dm_long","jc5dm_short"]) => stock("Five Day Momentum pp.10-12 and p.39 specifies individual stocks, US dollar price thresholds and a 1/16-point tick. Instrument-type compatibility does not approve a currency/tick conversion or waive the existing fixed-amount accuracy gate."),
        "ats" if id=="ats_cbp" => stock("Advanced Trading Strategies Chapter 11 selects momentum stocks from an IBD Relative Strength 98-99 scan (a cross-sectional stock ranking). Index prices are not stock substitutes for that selected population."),
        "ats" if member(id,&["ats_dvmtm","ats_cvr_equities","ats_15min_adx_breakout_equities"]) => connors_any!("Advanced Trading Strategies Chapters 13-14 and the equities ADX variant concern individual stocks. Volume, intraday data, currency-specific offsets and incomplete discretionary exits remain separate source/data requirements."),
        "lwrsr" if member(id,&["lwrsr_oct_apr","lwrsr_apr_oct"]) => index("The Right Stock at the Right Time Chapter 3, Table 3.2 printed pp.32-34, explicitly times the DJIA index. Nifty is a new index application, not the Dow's historical results."),
        "lwrsr" if id=="lwrsr_stock_selection" => portfolio("The Right Stock at the Right Time Chapters 8-11 selects Dow/Utilities members using point-in-time fundamentals and portfolio rules. Present cash constituents do not supply these historical inputs."),
        "mmft" if id=="mmft_nasdaq79" => index("Market & Mutual Fund Timing Week 3 Model 1 uses the Nasdaq Composite index. Other observed index applications are separate market experiments; the absent source index and its 1963-1997 result are not reproduced."),
        "radge" if id=="radge_ma220" => future("Radge Trading Course Moving Average Breakout Example 50 reports CL crude-oil futures and uses instrument minimum ticks. Actual dated futures prices, contract identities and tick values are required; default NSE cash-index ticks cannot reproduce CL execution."),
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
    if let Some(issue)=dema_index_symbol_issue(def,ticker,selected){return Some(issue);}
    if let Some(issue)=nse_breadth_held(def,ticker){return Some(issue);}
    if let Some(issue)=vixr_symbol_issue(def,ticker){return Some(issue);}
    if let Some(issue)=spmt_symbol_issue(def,ticker){return Some(issue);}
    if let Some(issue)=ptjmt_symbol_issue(def,ticker){return Some(issue);}
    if let Some(issue)=ss_symbol_issue(def,ticker,native_stock){return Some(issue);}
    if let Some(issue)=ws5_symbol_issue(def,ticker){return Some(issue);}
    if let Some(issue)=ststw_symbol_issue(def,ticker){return Some(issue);}
    if let Some(issue)=ats_symbol_issue(def,ticker,native_stock.then_some(false)){return Some(issue);}
    if let Some(issue)=hfm_otvt_symbol_issue(def,ticker){return Some(issue);}
    if native_stock {
        if let Some(reason)=p.native_currency_constraint {
            return Some(Issue {status:"needs_data",note:format!("Source currency/tick data missing. {reason} USD amounts cannot silently become INR amounts. A source-supported Indian specification or dated FX treatment and verified local tick rules are required before admission; editing the numeric parameter alone does not establish those inputs. Selected actual stock: {ticker}. No results are published.")});
        }
    }
    check_selected(p,def.id,ticker,selected)
}
/// Native type/currency comes from the fixed server catalogue, never from a
/// request flag or a ticker spelling. USD compatibility does not certify the
/// source's historical adjusted-price basis or its original execution ticks.
pub fn check_native(def:&StrategyDef,ticker:&str,evidence:&NativeEvidence)->Option<Issue> {
    let p=policy(def);
    if let Some(issue)=dema_index_symbol_issue(def,ticker,evidence.asset_type()){return Some(issue);}
    if let Some(issue)=nse_breadth_held(def,ticker){return Some(issue);}
    if let Some(issue)=vixr_symbol_issue(def,ticker){return Some(issue);}
    if let Some(issue)=spmt_symbol_issue(def,ticker){return Some(issue);}
    if let Some(issue)=ptjmt_symbol_issue(def,ticker){return Some(issue);}
    if let Some(issue)=ss_symbol_issue(def,ticker,evidence.asset_type()=="individual_stock"){return Some(issue);}
    if let Some(issue)=ws5_symbol_issue(def,ticker){return Some(issue);}
    if let Some(issue)=ststw_symbol_issue(def,ticker){return Some(issue);}
    if let Some(issue)=ats_symbol_issue(def,ticker,Some(evidence.market()==crate::baskets::Market::UnitedStates)){return Some(issue);}
    if let Some(issue)=hfm_otvt_symbol_issue(def,ticker){return Some(issue);}
    if let Some(issue)=check_selected(p,def.id,ticker,evidence.asset_type()){return Some(issue);}
    if matches!((def.book,def.id),("r3r4","r3r4_lowprice"|"r3r4_lowprice_scale")) && evidence.market()!=Market::UnitedStates {
        return Some(Issue{status:"not_testable",note:format!("{R3R4_LOWPRICE_HELD} Selected actual stock: {ticker}.")});
    }
    if let Some(reason)=p.native_currency_constraint.filter(|_|!usd_price_rule_on_us_stock(def,evidence.market()) && !inr_price_filter_omitted(def,evidence.market())) {
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
/// Plain-language reason for Connors rules that stay unadmitted on every instrument (admission records, 21 Oct 2026).
fn held_reason(id:&str)->Option<&'static str> {
    Some(match id {
        "ss_adx_gapper_no_gap"=>"Held: rules not fully defined. Street Smarts (p.93) presents this as Larry's discussion and calls it \"more subjective\"; it gives no objective entry, so any coded entry would be the vault's, not the book's.",
        "hfm_hv"=>"Held: rules not fully defined. The book (Ch. 3) gives a volatility warning with no direction, entry, exit or stop (\"a personal choice\"); the coded breakout entry and 20-day exit would be vault choices.",
        "hfm_und"|"hfm_und_long"|"hfm_und_short"=>"Held: needs option prices. The book (Ch. 4) trades Undeniables by selling out-of-the-money index puts or calls; a long or short trade in the index itself has a different payoff, and the vault holds no traded option prices.",
        "btp_bottom_reversal_long"=>"Held: rules not fully defined. \"Bottom\", \"near the bottom of the range\" and \"a fairly tight stop\" are not quantified in the book, so the entry and stop would be the vault's.",
        "btp_momentum_gap"=>"Held: rules not fully defined. The book gives no gap size, stop level or exit, so the whole exit would be the vault's.",
        "alpha_weekly_mr_single"=>"Held: ranking method. The book (Ch. 6) buys the 10 lowest-volatility candidates from the 500 most liquid US stocks each week and reads the market regime from SPY; a single index, ETF or stock cannot reproduce that selection. The full method needs a stock universe (see the portfolio entry).",
        _=>return None,
    })
}
fn check_selected(p:Policy,id:&str,ticker:&str,selected:&str)->Option<Issue> {
    if p.requirement=="declared_index_timing" && selected=="etf" && ticker!="SPY" {
        return Some(Issue{status:"needs_data",note:"The reviewed SPMT ETF signal instrument is actual SPY. Other ETFs require a declared and separately reviewed adaptation; they cannot silently replace SPY.".into()});
    }
    if p.allowed_types.contains(&selected) {return None;}
    if p.requirement=="source_review_required" {
        if let Some(reason)=held_reason(id) {return Some(Issue{status:"not_testable",note:format!("{reason} Selected {ticker}: {selected}. No results are published.")});}
    }
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
    if def.book=="hbiab" && crate::books::hbiab_reviewed::reviewed(def.id) {json.push_str(&format!(",\"source_convention\":\"{}\",\"account_model\":\"{}\"",crate::books::hbiab_reviewed::convention(def.id),crate::books::hbiab_reviewed::MODEL));}
    if reviewed_dema_index(def) {json.push_str(&format!(",\"source_convention\":\"{}\"",crate::books::hbqts::reviewed::DEMA_INDEX_CONVENTION));}
    if reviewed_kestner_stock(def) && crate::books::lkqts::next_candidate(def.id) {
        let convention=crate::books::lkqts::next_source_convention(def.id).expect("Exact next-profile predicate");
        let profile=match def.id {
            "lkqts_volbo"=>"volbo_current_inclusive_population100_k2",
            "lkqts_rsi14"=>"rsi_wilder_rma14_level35_65_seed14_flat50",
            "lkqts_macd"=>"macd_period_alpha_first_close",
            "lkqts_macd_literal_alpha"=>"macd_literal_alpha_first_close",
            _=>unreachable!("Exact next-profile predicate"),
        };
        let account_checks=if def.id=="lkqts_rsi14" {"Selected AAPL/RELIANCE source events matched; the four checked normalized-account cost cases are correctly withheld for nonpositive equity. Source-ledger agreement does not certify a supported funded account."} else {"Selected AAPL/RELIANCE source events and four zero/nonzero-cost normalized-account cases matched. Other requested runs still require their own account-domain checks; no original book performance is certified."};
        json.push_str(&format!(",\"source_convention\":\"{}\",\"source_profile\":\"{}\",\"selected_account_checks\":\"{}\",\"normalized_account_model\":\"sequential_full_allocation_price_returns\"",escape(convention),profile,escape(account_checks)));
    }
    if reviewed_kestner_stock(def) && crate::books::lkqts::sob_candidate(def.id) {
        json.push_str(&format!(",\"source_convention\":\"{}\",\"source_profile\":\"{}\",\"selected_account_checks\":\"Selected AAPL/RELIANCE source events and four zero/nonzero-cost normalized-account cases matched separate source/account references. Other requested runs still need individual source/data/account checks; original book performance is not reproduced.\",\"normalized_account_model\":\"sequential_full_allocation_price_returns\"",escape(crate::books::lkqts::sob_source_convention()),crate::books::lkqts::SOB_PROPOSED_PROFILE));
    }
    if reviewed_kestner_stock(def) && crate::books::lkqts::tiar_candidate(def.id) {
        json.push_str(&format!(",\"source_convention\":\"{}\",\"source_profile\":\"{}\",\"selected_account_checks\":\"Fixed AAPL1990–2001/RELIANCE2001–2010: four ordinary accounts (fees/slippage0/0 and3/2bps per side; rollover0) and16 production-function responses (detail, readiness, selected summary, singleton optimizer) matched unchanged references with zero comparison mismatches. Selected API states and prior full-prefix source evidence are separately qualified. Shared-author mechanical checks are not blind independence. HTTP transport/request-matrix, browser and deployment are separate acceptance gates; other runs need their own source/data/account checks. Data readiness alone does not prove a financially supported account.\",\"normalized_account_model\":\"sequential_full_allocation_price_returns\"",escape(crate::books::lkqts::tiar_source_convention()),crate::books::lkqts::TIAR_SOURCE_PROFILE));
    }
    if reviewed_kestner_stock(def) && crate::books::lkqts::macs_candidate(def.id) {
        json.push_str(&format!(",\"source_convention\":\"{}\",\"source_profile\":\"{}\",\"numeric_convention\":\"{}\",\"selected_account_checks\":\"Fixed AAPL/RELIANCE full-prefix source states/events and all20 pairs matched unchanged exact references with zero mismatches. Fixed AAPL1990-01-01..2001-12-31 and RELIANCE2001-01-01..2010-12-31: MACS and TIAR each matched four ordinary normalized-account cases (fees/slippage0/0 and3/2bps per side, rollover0) and16 direct production-function responses (detail, readiness, selected summary, singleton optimizer) against unchanged source/account references with zero mismatches (combined32responses/eightaccounts). Selected API states and earlier full-prefix source evidence have separate scope. HTTP transport/request-matrix, browser and deployment are separate acceptance gates. Every requested run still needs its own source/data/account checks; data readiness alone does not certify account support. Shared-author mechanical checks are not blind independence. Data readiness alone does not prove account support.\",\"normalized_account_model\":\"sequential_full_allocation_price_returns\"",escape(crate::books::lkqts::macs_source_convention()),crate::books::lkqts::MACS_SOURCE_PROFILE,crate::books::lkqts::MACS_RAW_NUMERIC_CONVENTION));
    }
    let reviewed=reviewed_cash_rule(def)||reviewed_dema_index(def);
    json.push_str(&format!(",\"source_rule_reviewed\":{},\"independent_selected_ledger_checked\":{}",reviewed,reviewed));
    if p.requirement=="index_etf_or_stock" && CONNORS_INDEX_IDS.contains(&def.id) {json.push_str(concat!(",\"declared_application\":\"",CONNORS_INDEX_LABEL!(),"\""));}
    if def.book==crate::books::cwin::BOOK && crate::books::cwin::reviewed_daily(def.id) {json.push_str(concat!(",\"declared_application\":\"",CWIN_INDEX_LABEL!(),"\""));}
    if def.book==crate::books::ccl08::BOOK && crate::books::ccl08::day_trade(def.id) {json.push_str(concat!(",\"declared_application\":\"",CCL08_INDEX_LABEL!(),"\""));}
    if def.book==crate::books::ats::BOOK && crate::books::ats::reviewed_daily(def.id) && (p.requirement=="index_or_etf" || (crate::books::ats::STOCK_TICK_IDS.contains(&def.id) && !CONNORS_INDEX_IDS.contains(&def.id))) {json.push_str(concat!(",\"declared_application\":\"",BTP_INDEX_LABEL!(),"\""));}
    if def.book==crate::books::alpha::BOOK && def.id=="alpha_trend_6m" {json.push_str(concat!(",\"declared_application\":\"",ALPHA_TREND_LABEL!(),"\""));}
    if def.book=="btp" && crate::books::btp::reviewed_daily(def.id) {json.push_str(concat!(",\"declared_application\":\"",BTP_INDEX_LABEL!(),"\""));}
    if (def.book=="btp" && crate::books::btp::STOCK_ADMITTED.contains(&def.id)) || (def.book==crate::books::ats::BOOK && crate::books::ats::STOCK_TICK_IDS.contains(&def.id)) {json.push_str(concat!(",\"stock_adaptation\":\"",STOCK_TICK_LABEL!(),"\""));}
    if def.book==crate::books::ws5::BOOK && crate::books::ws5::reviewed_daily(def.id) && !crate::books::ws5::TIMING.contains(&def.id) {json.push_str(concat!(",\"stock_adaptation\":\"",WS5_STOCK_LABEL!(),"\""));}
    if def.book=="afactor" && crate::books::afactor::stock_rule(def.id) {json.push_str(concat!(",\"stock_adaptation\":\"",AFACTOR_STOCK_LABEL!(),"\""));}
    if matches!((def.book,def.id),("r3r4","r3r4_lowprice"|"r3r4_lowprice_scale")) {json.push_str(concat!(",\"stock_adaptation\":\"",USD_PRICE_US_LABEL!(),"\""));}
    if newrules_misc(def) && matches!(def.book,"hpetf"|"etfss") {json.push_str(concat!(",\"declared_application\":\"",NEWRULE_ANY_LABEL!(),"\""));}
    if def.book==crate::books::ws5::BOOK && crate::books::ws5::reviewed_daily(def.id) && !crate::books::ws5::TIMING.contains(&def.id) {json.push_str(concat!(",\"declared_application\":\"",WS5_INDEX_LABEL!(),"\""));}
    if def.book=="kfsc" && crate::books::kfsc::reviewed_hv_rule(def.id) {
        json.push_str(&format!(",\"source_convention\":\"{}\"",escape(crate::books::kfsc::HV_CONVENTION)));
    }
    if def.book=="kfsc" && crate::books::kfsc::reviewed_momentum_rule(def.id) {
        let convention=if def.id=="kfsc_macd" {"Fixed literal .0476/.0243/.10 despite the general formula conflict; first actual close seeds both price smoothings and MACD/signal start at zero; full observed history; idealized completed-close same-close crossings and reversals; terminal entries remain open."} else {"Fixed10-day current-minus-prior momentum double-smoothed with literal .0476; first actual momentum seeds both passes; first slope initializes direction, nonzero turns reverse and equality holds; full observed history; idealized completed-close same-close fills; terminal entries remain open."};
        json.push_str(&format!(",\"source_convention\":\"{}\"",escape(convention)));
    }
    if def.book=="kfsc" && crate::books::kfsc::reviewed_stock_rule(def.id) {
        let convention=if def.id=="kfsc_bollinger" {
            "Fixed chosen printed price-level sample-SD formula despite conflicting changes prose; completed-close signals and idealized same-close fills; center equality and same-close opposite-band reversal; terminal entries remain open."
        } else if def.id=="kfsc_pct_band" {
            "Fixed divided lower band MA/1.05; strict completed-close penetration and idealized same-close fills; center equality and same-close opposite-band reversal; terminal entries remain open."
        } else {
            "Fixed selected80/10 SMA example; both slopes up enters, either down exits, equality holds; completed-close signals and idealized same-close fills; terminal entries remain open."
        };
        json.push_str(&format!(",\"source_convention\":\"{}\"",escape(convention)));
    }
    let currency_admitted=if def.book=="afactor" && crate::books::afactor::stock_rule(def.id){"true"}else if p.native_currency_constraint.is_some(){"false"}else if p.allowed_types.contains(&"individual_stock"){"true"}else{"null"};
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
    fn exact_protected_elder_account_requires_actual_stocks_and_never_inherits(){
        let d=def("elcim_impulse","elcim");assert!(reviewed_cash_rule(&d));assert_eq!(policy(&d).allowed_types,&["individual_stock"]);assert_eq!(policy(&d).execution_scope,"single_symbol_source_account");
        assert!(check(&d,"RELIANCE",true).is_none());assert!(check_native(&d,"AAPL",&NativeEvidence::us_test_fixture()).is_none());
        for tk in ["NIFTY 50","SPX","SPY","QQQ","RELIANCE"]{assert!(check(&d,tk,false).is_some());}
        assert!(!reviewed_cash_rule(&def("elcim_impulse","other")));
        for id in ["elcim_fi2","elcim_ts_fi","eltfl_sroc","elcim_new"]{let d=def(id,"elcim");assert!(!reviewed_cash_rule(&d));assert_eq!(policy(&d).requirement,"source_review_required");}
    }
    #[test]
    fn reviewed_burns_keys_require_real_stocks_without_opening_other_methods() {
        for id in ["sbma_sma50_long","sbma_x_5s20s"] {
            let d=def(id,"sbma");assert!(reviewed_cash_rule(&d));
            assert!(check(&d,"RELIANCE",true).is_none());
            assert!(check_native(&d,"AAPL",&NativeEvidence::us_test_fixture()).is_none());
            for symbol in ["NIFTY 50","SPY","RELIANCE"] {assert!(check(&d,symbol,false).is_some());}
            assert!(metadata_json(&d).contains("\"source_performance_reproduced\":false"));
            assert!(!reviewed_cash_rule(&def(id,"other_book")));
        }
        for id in ["sbma_x_10e30e_sar","sbma_new_rule"] {
            let d=def(id,"sbma");assert!(!reviewed_cash_rule(&d));
            assert_eq!(policy(&d).requirement,"source_review_required");
        }
    }
    #[test]
    fn reviewed_bandy_ids_require_real_stocks_and_do_not_open_held_methods() {
        for id in ["hbqts_ma5_25","hbqts_ma1_36","hbqts_limit_entry","hbqts_fixed_interval","hbqts_profit_target","hbqts_entry_id1","hbqts_bb_breakout","hbqts_cgo","hbqts_stoch_rsi","hbqts_dpo","hbqts_dom","hbqts_macd_daily"] {
            let d=def(id,"hbqts");assert!(reviewed_cash_rule(&d));
            assert!(check(&d,"RELIANCE",true).is_none());assert!(check(&d,"AAPL",true).is_none());
            assert!(check(&d,"NIFTY 50",false).is_some());assert!(check(&d,"SPY",false).is_some());
        }
        let cmo=def("hbqts_cmo","hbqts");assert!(reviewed_cash_rule(&cmo));assert!(check(&cmo,"SPY",false).is_none());
        assert!(check(&cmo,"NIFTY 50",false).is_some());assert!(check(&cmo,"AAPL",true).is_some());
        for id in ["hbqts_apz","hbqts_stop_entry","hbqts_common_signal","hbqts_rotation"] {
            let d=def(id,"hbqts");assert!(!reviewed_cash_rule(&d));assert_eq!(policy(&d).requirement,"source_review_required");
        }
        assert!(!reviewed_cash_rule(&def("hbqts_ma5_25","another_book")));
    }
    #[test]
    fn spmt_review_is_exact_and_does_not_admit_stock_or_other_etf_substitutes() {
        for &id in crate::books::spmt::REVIEWED_DAILY {
            let d=def(id,"spmt");
            assert!(reviewed_cash_rule(&d));
            assert!(check(&d,"SPY",false).is_none());
            assert!(check(&d,"NIFTY 50",false).is_none());
            assert!(check(&d,"AAPL",true).is_some());
            assert!(check(&d,"RELIANCE",true).is_some());
            assert!(check(&d,"QQQ",false).is_some());
            assert!(!reviewed_cash_rule(&def(id,"stc9")));
        }
        for id in ["spmt_multi5","spmt_multi9","spmt_mts","spmt_tick","spmt_dt_gapdown","spmt_dt_unch","spmt_dt_gapup","spmt_eom_puts","spmt_future_method"] {
            assert!(!reviewed_cash_rule(&def(id,"spmt")),"{id}");
            assert!(check(&def(id,"spmt"),"SPY",false).is_some());
        }
    }
    #[test]
    fn spmt_second_admission_is_exact_and_symbol_scoped() {
        use crate::books::spmt;
        let cat=spmt::catalogue();
        let admitted:Vec<&str>=cat.iter().filter(|d|reviewed_cash_rule(d)).map(|d|d.id).collect();
        assert_eq!(admitted.len(),spmt::REVIEWED_DAILY.len()+spmt::REVIEWED_AUX.len());
        for &id in spmt::REVIEWED_AUX {
            let d=def(id,"spmt");
            assert_eq!(policy(&d).requirement,"declared_index_timing");assert!(policy(&d).market_adaptation);
            assert!(policy(&d).source_basis.contains("spmt2-admission"));
            for t in ["SPX","SPY"] {assert!(check(&d,t,false).is_none(),"{id} {t}");}
            // NIFTY 50 TRIN / advancers runs are held (user decision 2026-10-21: NYSE thresholds not calibrated for NSE)
            assert_eq!(check(&d,"NIFTY 50",false).is_none(),!matches!(id,"spmt_vix2"|"spmt_vix7"|"spmt_trin_1"|"spmt_trin_2"|"spmt_trin_3"|"spmt_trin_4"|"spmt_trin_5"|"spmt_nyse_adv"),"{id}");
            assert_eq!(check(&d,"NIFTYBANK",false).is_none(),id=="spmt_eom_und","{id}");
            assert_eq!(check(&d,"NDX",false).is_none(),id=="spmt_eom_und","{id}");
            assert!(check(&d,"QQQ",false).is_some() && check(&d,"AAPL",true).is_some() && check(&d,"RELIANCE",true).is_some(),"{id}");
        }
        // The first batch keeps its policy text and any-index scope.
        assert!(policy(&def("spmt_rsi_2","spmt")).source_basis.contains("STC IX duplicates"));
        assert!(check(&def("spmt_rsi_2","spmt"),"NDX",false).is_none());
    }
    #[test]
    fn ss_review_admits_exact_ids_on_reviewed_symbols_only() {
        use crate::books::ss;
        let cat=ss::catalogue();
        let admitted:Vec<&str>=cat.iter().filter(|d|reviewed_cash_rule(d)).map(|d|d.id).collect();
        assert_eq!(admitted.len(),30);
        assert!(admitted.iter().all(|id|ss::REVIEWED_DAILY.contains(id)));
        for d in cat.iter().filter(|d|reviewed_cash_rule(d)) {
            let stock=ss::STOCK_SHOWN.contains(&d.id);let also=ss::STOCK_ALSO.contains(&d.id);
            assert_eq!(policy(d).requirement,if stock||also {"index_etf_or_stock"} else {"declared_index_timing"},"{}",d.id);
            assert_eq!(check(d,"AAPL",true).is_none(),stock||also,"{}",d.id);
            assert_eq!(check(d,"QQQ",false).is_none(),stock,"{}",d.id);
            let futures=["SPX","NDX","DJI","SPY","NIFTY 50","NIFTYBANK"];
            match d.id {
                "ss_ad_market_buster" => for t in futures {assert_eq!(check(d,t,false).is_none(),matches!(t,"SPX"|"SPY"),"{t}");},
                "ss_ad_market_buster_nse" => for t in futures {assert!(check(d,t,false).is_some(),"{t}");}, // held (NSE breadth)
                _ => for t in futures {assert!(check(d,t,false).is_none(),"{} {t}",d.id);},
            }
            assert!(!reviewed_cash_rule(&def(d.id,"vixr")));
        }
        // Held: gate-withheld, data-missing, discretionary or no trade rule in the book.
        for id in ["ss_turtle_soup","ss_80_20","ss_momentum_pinball","ss_adx_gapper_no_gap","ss_spike_and_ledge","ss_fakeout_shakeout","ss_three_little_indians","ss_wolfe_wave","ss_morning_news_reversal","ss_big_picture_news_reversal","ss_smart_money_index","ss_tick_divergence","ss_tick_extreme_next_day","ss_arms_index_5ma","ss_arms_index_5ma_nse","ss_study_fresh_adx_buy","ss_study_fresh_adx_sell"] {
            let d=def(id,"ss");
            assert!(!reviewed_cash_rule(&d),"{id}");
            assert_eq!(policy(&d).requirement,"source_review_required");
        }
    }
    #[test]
    fn vixr_review_admits_exact_ids_on_reviewed_symbols_only() {
        let cat=crate::books::vixr::catalogue();
        let admitted:Vec<&str>=cat.iter().filter(|d|reviewed_cash_rule(d)).map(|d|d.id).collect();
        assert_eq!(admitted,["vixr_cvr1","vixr_cvr2","vixr_cvr3","vixr_cvr5_1d","vixr_cvr5_3d","vixr_cvr6","vixr_cvr7","vixr_cvr8","vixr_cvr9","vixr_cvr10","vixr_cvr11"]);
        for d in cat.iter().filter(|d|reviewed_cash_rule(d)) {
            assert_eq!(policy(d).requirement,"declared_index_timing");
            assert!(check(d,"SPY",false).is_none() && check(d,"SPX",false).is_none(),"{}",d.id);
            assert_eq!(check(d,"NIFTY 50",false).is_none(),["vixr_cvr2","vixr_cvr3","vixr_cvr6"].contains(&d.id),"{}",d.id);
            for t in ["NDX","DJI","QQQ","DIA","NIFTY BANK"] {assert!(check(d,t,false).is_some(),"{} {t}",d.id);}
            assert!(check(d,"AAPL",true).is_some() && check(d,"RELIANCE",true).is_some());
            assert!(!reviewed_cash_rule(&def(d.id,"spmt")));
        }
        for id in ["vixr_all","vixr_stocks","vixr_options","vixr_cvr4"] {
            let d=def(id,"vixr");
            assert!(!reviewed_cash_rule(&d));
            assert_eq!(policy(&d).requirement,"source_review_required");
            assert!(check(&d,"SPY",false).is_some());
        }
    }
    #[test]
    fn ptjmt_review_admits_exact_ids_on_reviewed_symbols_only() {
        let cat=crate::books::ptjmt::catalogue();
        let admitted:Vec<&str>=cat.iter().filter(|d|reviewed_cash_rule(d)).map(|d|d.id).collect();
        assert_eq!(admitted,["ptjmt_cvr1","ptjmt_cvr2","ptjmt_cvr3","ptjmt_cvr5","ptjmt_trin_reversals","ptjmt_trin_reversals_nse","ptjmt_trin_reversals_20","ptjmt_trin_reversals_20_nse","ptjmt_trin_thrusts","ptjmt_trin_thrusts_nse","ptjmt_padi","ptjmt_padi_nse"]);
        for d in cat.iter().filter(|d|reviewed_cash_rule(d)) {
            assert_eq!(policy(d).requirement,"declared_index_timing");
            let nse=d.id.ends_with("_nse");
            assert_eq!(check(d,"SPY",false).is_none(),!nse,"{}",d.id);
            assert_eq!(check(d,"SPX",false).is_none(),!nse,"{}",d.id);
            assert_eq!(check(d,"NIFTY 50",false).is_none(),["ptjmt_cvr2","ptjmt_cvr3"].contains(&d.id),"{}",d.id); // *_nse held
            for t in ["NDX","DJI","QQQ","DIA","NIFTY BANK"] {assert!(check(d,t,false).is_some(),"{} {t}",d.id);}
            assert!(check(d,"AAPL",true).is_some() && check(d,"RELIANCE",true).is_some());
        }
        for id in ["ptjmt_cvr4","ptjmt_cvr4_friday"] {
            let d=cat.iter().find(|d|d.id==id).unwrap();
            assert!(!reviewed_cash_rule(d));
            assert_eq!(policy(d).requirement,"source_review_required");
            assert!(check(d,"SPY",false).is_some());
        }
    }
    #[test]
    fn cwin_review_admits_exact_ids_on_stocks_and_index_etf() {
        let cat=crate::books::cwin::catalogue();
        let admitted:Vec<&str>=cat.iter().filter(|d|reviewed_cash_rule(d)).map(|d|d.id).collect();
        assert_eq!(admitted,crate::books::cwin::REVIEWED_DAILY);
        for d in cat.iter().filter(|d|reviewed_cash_rule(d)) {
            let p=policy(d);assert_eq!(p.requirement,"index_etf_or_stock");assert!(p.market_adaptation);
            for t in ["SPY","QQQ","SPX","NDX","DJI","NIFTY 50","NIFTYBANK"] {assert!(check(d,t,false).is_none(),"{} {t}",d.id);}
            assert!(check(d,"AAPL",true).is_none() && check(d,"RELIANCE",true).is_none() && check(d,"NIFTY FUTURE",false).is_some());
            assert!(metadata_json(d).contains("\"declared_application\":\"The book and course show Windows"));
        }
        // Held: needs intraday VIX and price series that the vault does not have.
        let d=def("cwin_vix_gap_window","cwin");
        assert!(!reviewed_cash_rule(&d));assert_eq!(policy(&d).requirement,"source_review_required");assert!(check(&d,"SPY",false).is_some());
    }
    #[test]
    fn ws5_review_admits_exact_ids_on_reviewed_symbols_only() {
        use crate::books::ws5;
        let cat=ws5::catalogue();
        let admitted:Vec<&str>=cat.iter().filter(|d|reviewed_cash_rule(d)).map(|d|d.id).collect();
        assert_eq!(admitted,ws5::REVIEWED_DAILY);
        for d in cat.iter().filter(|d|reviewed_cash_rule(d)) {
            let timing=ws5::TIMING.contains(&d.id);let p=policy(d);
            assert_eq!(p.requirement,if timing {"declared_index_timing"} else {"index_etf_or_stock"},"{}",d.id);assert!(p.market_adaptation);
            for t in ["SPX","SPY","NIFTY 50"] {assert!(check(d,t,false).is_none(),"{} {t}",d.id);}
            for t in ["NDX","DJI","QQQ","NIFTYBANK"] {assert_eq!(check(d,t,false).is_none(),!timing,"{} {t}",d.id);}
            // User decision 2026-10-21: Windows also run on actual stocks without the filters that cannot be applied.
            assert_eq!(check(d,"AAPL",true).is_none() && check(d,"RELIANCE",true).is_none(),!timing,"{}",d.id);
            assert_eq!(check_native(d,"AAPL",&NativeEvidence::us_test_fixture()).is_none(),!timing,"{}",d.id);
            assert_eq!(metadata_json(d).contains("\"declared_application\":\"Seminar method for US stocks"),!timing);
            assert_eq!(metadata_json(d).contains("\"stock_adaptation\":\"Actual stocks are admitted without the filters"),!timing);
            assert!(!reviewed_cash_rule(&def(d.id,"vixr")));
        }
        let d=def("ws5_emini_spy_daytrade","ws5");
        assert!(!reviewed_cash_rule(&d));assert_eq!(policy(&d).requirement,"source_review_required");assert!(check(&d,"SPY",false).is_some());
    }
    #[test]
    fn ccl08_review_admits_day_trades_on_stocks_and_indices_tps_on_index_etf() {
        use crate::books::ccl08;
        let cat=ccl08::catalogue();
        let admitted:Vec<&str>=cat.iter().filter(|d|reviewed_cash_rule(d)).map(|d|d.id).collect();
        assert_eq!(admitted.len(),ccl08::REVIEWED_DAILY.len());
        let ok=|id:&'static str,t:&str,stock:bool|check(&def(id,"ccl08"),t,stock).is_none();
        for id in ccl08::REVIEWED_DAILY {
            let dt=ccl08::day_trade(id);
            for t in ["SPX","SPY","NIFTY 50","NIFTYBANK"] {assert!(ok(id,t,false),"{id} {t}");}
            assert_eq!(ok(id,"RELIANCE",true),dt,"{id}");
            assert!(check_native(&def(id,"ccl08"),"AAPL",&NativeEvidence::us_test_fixture()).is_none()==dt,"{id}");
            assert_eq!(policy(&def(id,"ccl08")).requirement,if dt {"index_etf_or_stock"} else {"index_or_etf"});
            assert_eq!(metadata_json(&def(id,"ccl08")).contains("\"declared_application\":\"Book shows these day trades on US stocks"),dt);
        }
        for id in ["ccl08_crdt_emini","ccl08_weekly_rsi_options","ccl08_quick_scalp","ccl08_adx_quiet","ccl08_busted_options"] {
            assert!(!reviewed_cash_rule(&def(id,"ccl08")),"{id}");
        }
    }
    #[test]
    fn ats_review_admits_exact_ids_on_reviewed_symbols_only() {
        use crate::books::ats;
        let cat=ats::catalogue();
        let admitted:Vec<&str>=cat.iter().filter(|d|reviewed_cash_rule(d)).map(|d|d.id).collect();
        assert_eq!(admitted.len(),ats::REVIEWED_DAILY.len());
        assert!(admitted.iter().all(|id|ats::REVIEWED_DAILY.contains(id)));
        let ok=|id:&'static str,t:&str|check(&def(id,"ats"),t,false).is_none();
        for t in ["SPX","SPY"] {for id in ["ats_cvr1","ats_cvr2","ats_cvr3","ats_trin_thrust","ats_padi","ats_hv_thrust","ats_cvr_equities"] {assert!(ok(id,t),"{id} {t}");}}
        for id in ["ats_cvr2","ats_cvr3","ats_hv_thrust"] {assert!(ok(id,"NIFTY 50"),"{id}");}
        for id in ["ats_trin_thrust_nse","ats_padi_nse"] {assert!(!ok(id,"NIFTY 50"),"{id}");} // held (NSE breadth)
        for id in ["ats_cvr1","ats_trin_thrust","ats_padi","ats_cvr_equities"] {assert!(!ok(id,"NIFTY 50"),"{id}");}
        for id in ["ats_trin_thrust_nse","ats_padi_nse"] {assert!(!ok(id,"SPX") && !ok(id,"SPY"),"{id}");}
        for id in ["ats_cvr1","ats_cvr2","ats_hv_thrust"] {assert!(!ok(id,"QQQ") && !ok(id,"NIFTYMIDCAP100"),"{id}");}
        for id in ["ats_1234","ats_8day_hl_reversal","ats_smtp","ats_two_for_one","ats_gipsons"] {
            assert_eq!(policy(&def(id,"ats")).requirement,"index_etf_or_stock");
            assert!(ok(id,"NIFTY 50") && ok(id,"NIFTYMIDCAP100") && !ok(id,"AAPL"),"{id}");
            assert!(check(&def(id,"ats"),"RELIANCE",true).is_none() && check_native(&def(id,"ats"),"AAPL",&NativeEvidence::us_test_fixture()).is_none(),"{id}");
            assert!(metadata_json(&def(id,"ats")).contains("\"stock_adaptation\":\"Actual stocks are admitted"));
            assert!(metadata_json(&def(id,"ats")).contains("\"declared_application\""));
        }
        // CVR I on stocks: US stocks (CBOE VIX) yes, Indian stocks no (India VIX open repeats the previous close)
        assert!(check_native(&def("ats_cvr_equities","ats"),"AAPL",&NativeEvidence::us_test_fixture()).is_none());
        assert!(check(&def("ats_cvr_equities","ats"),"RELIANCE",true).is_some());
        assert_eq!(policy(&def("ats_dvmtm","ats")).requirement,"index_etf_or_stock");
        for id in ["ats_hv_ratio_setup","ats_cbp","ats_late_earnings","ats_torpedoes","ats_hv_ratio_short_strangle","ats_cvr_options","ats_split_call_selling","ats_sector_options_overpriced","ats_15min_adx_breakout","ats_15min_adx_breakout_equities","ats_10pct_oops","ats_morning_call_reversal","ats_wide_range_gap","ats_flag_within_a_flag","ats_short_term_decline_playbook","ats_large_range_day_combo","ats_after_losses"] {
            assert!(!reviewed_cash_rule(&def(id,"ats")),"{id}");
        }
    }
    #[test]
    fn hfm_otvt_review_admits_exact_ids_on_reviewed_symbols_only() {
        use crate::books::{hfm,otvt};
        let admitted:Vec<&str>=hfm::catalogue().iter().chain(otvt::catalogue().iter()).filter(|d|reviewed_cash_rule(d)).map(|d|d.id).collect();
        assert_eq!(admitted,["hfm_chadtp","hfm_chadtp_nse","otvt_cvr1_long","otvt_cvr1_short","otvt_hv_thrust"]);
        let ok=|id:&'static str,b:&'static str,t:&str|check(&def(id,b),t,false).is_none();
        for t in ["SPX","SPY"] {for (id,b) in [("hfm_chadtp","hfm"),("otvt_cvr1_long","otvt"),("otvt_cvr1_short","otvt"),("otvt_hv_thrust","otvt")] {assert!(ok(id,b,t),"{id} {t}");}}
        for t in ["NDX","DJI","NIFTY 50","NIFTYBANK"] {assert!(ok("otvt_hv_thrust","otvt",t),"{t}");}
        assert!(!ok("hfm_chadtp_nse","hfm","NIFTY 50") && !ok("hfm_chadtp_nse","hfm","SPX") && !ok("hfm_chadtp","hfm","NIFTY 50"));
        for id in ["otvt_cvr1_long","otvt_cvr1_short"] {assert!(!ok(id,"otvt","NIFTY 50") && !ok(id,"otvt","QQQ"),"{id}");}
        assert!(!ok("otvt_hv_thrust","otvt","QQQ") && !ok("otvt_hv_thrust","otvt","NIFTYMIDCAP100") && !ok("hfm_chadtp","hfm","QQQ"));
        assert!(check(&def("otvt_hv_thrust","otvt"),"RELIANCE",true).is_some());
        for (id,b) in [("hfm_hv","hfm"),("hfm_und","hfm"),("hfm_und_long","hfm"),("hfm_und_short","hfm"),("hfm_ndxspx","hfm"),("hfm_news","hfm"),("hfm_vix","hfm"),("otvt_cvr1","otvt"),("otvt_cvr1_sell_puts","otvt"),("ptjdt_adx15_breakout","ptjdt")] {
            assert!(!reviewed_cash_rule(&def(id,b)),"{id}");assert_eq!(policy(&def(id,b)).requirement,"source_review_required","{id}");
        }
    }
    #[test]
    fn newrules_misc_admits_exact_ids_and_symbols() {
        let cat=crate::connors::catalogue();
        let admitted:Vec<&str>=cat.iter().filter(|d|newrules_misc(d)).map(|d|d.id).collect();
        assert_eq!(admitted.len(),5,"{admitted:?}");
        for d in cat.iter().filter(|d|newrules_misc(d)) {assert!(reviewed_cash_rule(d),"{}",d.id);}
        let ok=|id:&'static str,book:&'static str,t:&str,stock:bool|check(&def(id,book),t,stock).is_none();
        for (id,book) in [("hpetf_sem_rsi14_tps","hpetf"),("ss_long_2nd","etfss"),("ss_short_2nd","etfss")] {
            assert_eq!(policy(&def(id,book)).requirement,"index_etf_or_stock");
            for t in ["SPY","QQQ","SPX","NIFTY 50","NIFTYBANK"] {assert!(ok(id,book,t,false),"{id} {t}");}
            assert!(ok(id,book,"RELIANCE",true),"{id}");
            assert!(check_native(&def(id,book),"AAPL",&NativeEvidence::us_test_fixture()).is_none());
            assert!(metadata_json(&def(id,book)).contains("\"declared_application\":\"Index, ETF and stock runs"));
        }
        for t in ["SPX","SPY"] {assert!(ok("ststw_vix_rsi","ststw",t,false));}
        for t in ["QQQ","NDX","NIFTY 50"] {assert!(!ok("ststw_vix_rsi","ststw",t,false),"{t}");}
        assert!(!ok("ststw_vix_rsi","ststw","RELIANCE",true));
        assert_eq!(policy(&def("ststw_vix_rsi","ststw")).requirement,"declared_index_timing");
        // the S&P-down-days variant shares the baseline's stock admission (US: USD price filter kept; India: omitted)
        let af=def("afactor_spx_down","afactor");
        assert_eq!(policy(&af).requirement,"index_etf_or_stock");
        assert!(check_native(&af,"AAPL",&NativeEvidence::us_test_fixture()).is_none());
        assert!(check_native(&af,"RELIANCE",&NativeEvidence::india_test_fixture()).is_none());
        assert!(inr_price_filter_omitted(&af,Market::India) && usd_price_rule_on_us_stock(&af,Market::UnitedStates));
        assert!(metadata_json(&af).contains("\"native_india_currency_admitted\":true"));
        // held / not admitted
        assert!(!reviewed_cash_rule(&def("hpetf_sem_pairs","hpetf")));
        for (id,book) in [("ss_long","etfss"),("ststw_vix_stretch","ststw"),("hpetf_tps_long","hpetf"),("afactor_portfolio","afactor")] {assert!(!newrules_misc(&def(id,book)),"{id}");}
    }
    #[test]
    fn btp_review_admits_exact_ids_on_index_and_etf_only() {
        let cat=crate::books::btp::catalogue();
        let admitted:Vec<&str>=cat.iter().filter(|d|reviewed_cash_rule(d)).map(|d|d.id).collect();
        assert_eq!(admitted,crate::books::btp::REVIEWED_DAILY);
        for d in cat.iter().filter(|d|reviewed_cash_rule(d)) {
            let stock=crate::books::btp::STOCK_ADMITTED.contains(&d.id);
            let p=policy(d);assert_eq!(p.requirement,if stock {"index_etf_or_stock"} else {"index_or_etf"});assert!(p.market_adaptation);
            for t in ["SPY","QQQ","MDY","SPX","NDX","DJI","NIFTY 50","NIFTYBANK"] {assert!(check(d,t,false).is_none(),"{} {t}",d.id);}
            assert_eq!(check(d,"AAPL",true).is_none() && check(d,"RELIANCE",true).is_none(),stock,"{}",d.id);
            assert!(check(d,"NIFTY FUTURE",false).is_some());
            assert_eq!(metadata_json(d).contains("1/8-dollar ticks"),stock,"{}",d.id);
            assert!(metadata_json(d).contains("\"declared_application\":\"Book tested or illustrated on futures and US stocks"));
            assert!(!reviewed_cash_rule(&def(d.id,"ats")));
        }
        for id in ["btp_cbp_short","btp_bottom_reversal_long","btp_momentum_gap","btp_large_range_day_strangle","btp_large_range_day_study","btp2_10pct_oops","btp2_10pct_oops_long","btp2_10pct_oops_short","btp2_wide_range_exhaustion_gap_short"] {
            let d=def(id,"btp");
            assert!(!reviewed_cash_rule(&d),"{id}");
            assert_eq!(policy(&d).requirement,"source_review_required");
            assert!(check(&d,"SPY",false).is_some());
        }
    }
    #[test]
    fn reviewed_chande_keys_are_exact_and_do_not_open_dollar_or_indicator_methods() {
        for id in ["ch_bta2_ma_stack", "ch_bta2_65sma_ravi"] {
            let d=def(id,"chbta2");
            assert!(reviewed_cash_rule(&d));
            assert!(check(&d,"NIFTY 50",false).is_none());
            assert!(check(&d,"SPY",false).is_none());
            assert!(check(&d,"RELIANCE",true).is_none());
            assert!(!reviewed_cash_rule(&def(id,"chntt")));
            assert!(policy(&d).source_basis.contains("Chande"));
        }
        for (id,book) in [("ch_bta2_65sma_ravi_x14","chbta2"),("ch_bta2_bottom_fish","chbta2"),("ch_ntt_vidya_cmo","chntt"),("ch_ntt_to_buy","chntt")] {
            let d=def(id,book);
            assert!(!reviewed_cash_rule(&d));
            assert!(check(&d,"NIFTY 50",false).is_some());
        }
        assert!(check(&def("ch_65sma3cc","ch_bta"),"NIFTY 50",false).is_none());
        assert!(check(&def("ch_cbpb_time","ch_bta"),"SPY",false).is_none());
    }
    #[test]
    fn reviewed_kaufman_rules_use_explicit_cash_adaptations_without_opening_other_methods(){
        let reviewed=def("kftsm_ma","kftsm");
        assert!(check(&reviewed,"NIFTY 50",false).is_none());
        assert!(check(&reviewed,"SPY",false).is_none());
        assert!(check(&reviewed,"RELIANCE",true).is_none());
        assert!(metadata_json(&reviewed).contains("\"source_rule_reviewed\":true"));
        assert!(metadata_json(&reviewed).contains("\"source_performance_reproduced\":false"));
        assert_eq!(policy(&def("kftsm_parabolic","kftsm")).requirement,"source_review_required");
        assert_eq!(policy(&def("kftsm_ma","another_book")).requirement,"source_review_required");
    }
    #[test]
    fn adaptive_batch_review_is_exact_id_and_keeps_intraday_and_other_source_holds() {
        for id in ["kftsm_kama","kftsm_adapt_exp","kftsm_vidya","kftsm_adapt_r2","kftsm_adapt_rsi","kftsm_swg","kftsm_swing","kftsm_raschke_first_cross","kftsm_outside_day","kftsm_fc_bb_abx"] {
            let d=def(id,"kftsm");
            for (symbol,native) in [("NIFTY 50",false),("SPY",false),("RELIANCE",true)] {
                assert!(check(&d,symbol,native).is_none());
            }
            assert!(check_native(&d,"AAPL",&NativeEvidence::us_test_fixture()).is_none());
            let json=metadata_json(&d);
            assert!(json.contains("\"source_rule_reviewed\":true"));
            assert!(json.contains("\"source_performance_reproduced\":false"));
            assert!(json.contains("selected-baseline-ledger-check"));
            assert_eq!(policy(&def(id,"another_book")).requirement,"source_review_required");
            // These price-return adaptations never admit contract or premium P&L.
            for ticker in ["NIFTY FUTURE", "SPY OPTION", "NIFTYMIDCAPSELECT"] {
                assert_eq!(check(&d,ticker,false).unwrap().status,"needs_data");
            }
        }
        for id in ["kftsm_parabolic","kftsm_donchian_4wk","kftsm_pf","kftsm_moon","kftsm_multi_tf","kftsm_new_adaptive_method"] {
            let d=def(id,"kftsm");
            assert_eq!(policy(&d).requirement,"source_review_required");
            assert!(metadata_json(&d).contains("\"source_rule_reviewed\":false"));
        }
        assert!(check(&def("kftsm_hirsch","kftsm"),"RELIANCE",true).is_some());
        assert!(check(&def("kftsm_month_end","kftsm"),"RELIANCE",true).is_some());
    }
    #[test]
    fn stock_studies_admit_stocks_and_declared_index_etf_and_keep_snapshot_limits() {
        for id in ["f1a", "f3a", "f4a", "f4c", "f4d", "rsi2_5d", "tps_stocks", "x555", "h10"] {
            let d=def(id,"stc9");
            // Declared index/ETF application (user decision 2026-10-21); unknown tickers still refused.
            assert!(check(&d,"NIFTY 50",false).is_none());
            assert!(check(&d,"SPY",false).is_none());
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
    fn radge_plain_review_does_not_admit_stops_or_other_instruments() {
        let d=def("radge_oc_long","radge");
        assert!(reviewed_cash_rule(&d));
        let p=policy(&d);
        assert_eq!(p.requirement,"index");
        assert_eq!(p.allowed_types,&["index"]);
        assert!(p.market_adaptation);
        assert!(check(&d,"NIFTY 50",false).is_none());
        assert!(check(&d,"SPX",false).is_none());
        assert!(check(&d,"SPY",false).is_some());
        assert!(check(&d,"RELIANCE",true).is_some());
        for id in ["radge_oc_long_stop","radge_oc_short_stop","radge_ma220"] {
            assert!(!reviewed_cash_rule(&def(id,"radge")));
        }
        assert!(!reviewed_cash_rule(&def("radge_oc_long","other_book")));
        assert!(metadata_json(&d).contains("\"source_performance_reproduced\":false"));
    }
    #[test]
    fn brock_review_is_exact_and_cannot_inherit_multi_asset_admission() {
        let reviewed=["bllstr_vma_1_50_0","bllstr_fma_1_50_0"];
        for d in crate::books::bllstr::catalogue() {
            if reviewed.contains(&d.id) {
                assert!(reviewed_cash_rule(&d));
                let p=policy(&d);
                assert_eq!(p.requirement,"index");
                assert_eq!(p.allowed_types,&["index"]);
                assert!(p.market_adaptation);
                for ticker in ["NIFTY 50","SPX"] {assert!(check(&d,ticker,false).is_none());}
                for ticker in ["SPY","QQQ"] {assert!(check(&d,ticker,false).is_some());}
                for ticker in ["RELIANCE","AAPL"] {assert!(check(&d,ticker,true).is_some());}
                assert!(metadata_json(&d).contains("\"source_rule_reviewed\":true"));
                assert!(metadata_json(&d).contains("\"source_performance_reproduced\":false"));
                assert!(!reviewed_cash_rule(&def(d.id,"kftsm")));
                assert!(!reviewed_cash_rule(&def(d.id,"radge")));
            } else {
                assert!(!reviewed_cash_rule(&d),"{} inherited a review",d.id);
                assert_eq!(policy(&d).requirement,"source_review_required");
            }
        }
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
        // Appendix A1 trend rule: single-instrument, no ranking -> actual ETF, index as a declared application; never a stock.
        let tf=def("alpha_trend_6m","alpha");
        assert!(reviewed_cash_rule(&tf));assert_eq!(policy(&tf).requirement,"index_or_etf");assert!(policy(&tf).market_adaptation);
        for t in ["SPY","QQQ","SMH","SPX","NIFTY 50","NIFTYBANK"] {assert!(check(&tf,t,false).is_none(),"{t}");}
        assert!(check(&tf,"RELIANCE",true).is_some());
        assert!(metadata_json(&tf).contains("\"declared_application\":\"Book tested on nine US sector ETFs"));
        for id in ["alpha_weekly_mr_single","alpha_weekly_mr"] {assert!(!reviewed_cash_rule(&def(id,"alpha")),"{id}");}
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
            ("ats_15min_adx_breakout_equities","ats")] {
            let d=def(id,book);
            let issue=check(&d,"RELIANCE",true).expect("Native stock type must not bypass source currency");
            assert_eq!(issue.status,"needs_data");
            assert!(issue.note.contains("USD")&&issue.note.contains("INR"));
            let json=metadata_json(&d);
            // afactor_baseline: Indian stocks admitted with the USD price filter omitted (user decision 2026-10-21)
            assert!(json.contains(if id=="afactor_baseline" {"\"native_india_currency_admitted\":true"} else {"\"native_india_currency_admitted\":false"}));
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
        for (id,book) in [("jc5dm_long","jc5dm"),("ats_cbp","ats")] {
            let issue=check_native(&def(id,book),"AAPL",&e).unwrap();assert_eq!(issue.status,"needs_data");assert!(issue.note.contains("USD"),"{id}");
        }
        // User decision 2026-10-21: a USD price filter is applied as printed on a US stock; the stock tick is the real one.
        for (id,book) in [("afactor_baseline","afactor"),("r3r4_lowprice","r3r4"),("r3r4_lowprice_scale","r3r4"),("ats_dvmtm","ats")] {
            assert!(check_native(&def(id,book),"AAPL",&e).is_none(),"{id}");
        }
        // Existing admission API has no client currency argument and preserves
        // Indian defaults even if a client adds a misleading ticker suffix.
        assert!(check(&def("afactor_baseline","afactor"),"RELIANCE?currency=USD",true).unwrap().note.contains("INR"));
        // 2026-10-21: afactor_baseline runs on Indian stocks (USD price filter omitted, labelled); the R3/R4 USD 1-15 band stays held.
        let inr=NativeEvidence::india_test_fixture();
        assert!(check_native(&def("afactor_baseline","afactor"),"RELIANCE",&inr).is_none());
        for id in ["r3r4_lowprice","r3r4_lowprice_scale"] {assert!(check_native(&def(id,"r3r4"),"RELIANCE",&inr).unwrap().note.contains("price band is the strategy itself"));}
    }
    #[test]
    fn ehlers_review_is_exact_and_mama_requires_actual_stock() {
        let mama=def("ehmd_mama","ehmd");
        for ticker in ["NIFTY 50","SPY"] {assert!(check(&mama,ticker,false).is_some());}
        assert!(check(&mama,"RELIANCE",true).is_none());
        assert!(check_native(&mama,"AAPL",&NativeEvidence::us_test_fixture()).is_none());
        for id in ["ehrs_sinetrend","ehrs_sinetrend_trend","ehrs_sinetrend_cp115","ehrs_sinetrend_cp115_trend"] {
            let st=def(id,"ehrs");assert!(reviewed_cash_rule(&st));
            for ticker in ["NIFTY 50","SPY"] {assert!(check(&st,ticker,false).is_none());}
            assert!(check(&st,"RELIANCE",true).is_none());
            assert!(policy(&st).source_basis.contains("Ehlers"));assert!(!policy(&st).source_basis.contains("Kaufman"));
            assert_eq!(policy(&def(id,"other_book")).requirement,"source_review_required");
        }
        for (id,book) in [("ehrs_sinetrend_cp110","ehrs"),("ehrs_zerolag","ehrs"),("ehrs_sinewave_cycle","ehrs"),("ehrs_optimum_predictor","ehrs"),("ehrs_adaptive_osc","ehrs"),("ehrs_snr_filter","ehrs"),("ehmd_lms_zerolag","ehmd"),("ehmd_new_unreviewed","ehmd")] {
            assert_eq!(policy(&def(id,book)).requirement,"source_review_required");
        }
        assert_eq!(policy(&def("ehmd_mama","other_book")).requirement,"source_review_required");
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

#[cfg(test)]
mod next_burns_policy_tests {
    use super::*;
    #[test]
    fn four_burns_long_versions_require_actual_stock_and_do_not_admit_shorts() {
        for &id in crate::books::sbma::FOUR_REVIEWED_STOCK_IDS {
            let d=crate::books::sbma::catalogue().into_iter().find(|d|d.id==id).unwrap();
            let p=policy(&d);assert!(reviewed_cash_rule(&d));assert_eq!(p.allowed_types,&["individual_stock"]);
            assert!(p.market_adaptation);assert!(p.source_basis.contains("FOUR-LONG"));
            assert!(check_native(&d,"AAPL",&NativeEvidence::us_test_fixture()).is_none());
            for ticker in ["NIFTY 50","SPX","SPY","QQQ"] {assert!(check(&d,ticker,false).is_some());}
            let mut wrong=d;wrong.book="unknown";assert!(!reviewed_cash_rule(&wrong));
        }
        for id in ["sbma_sma200_short","sbma_x_10e30e_sar"] {
            let d=crate::books::sbma::catalogue().into_iter().find(|d|d.id==id).unwrap();assert!(!reviewed_cash_rule(&d));
        }
    }
    #[test]
    fn exact_three_long_stock_ids_preserve_two_reviewed_and_short_hold() {
        let defs=crate::books::sbma::catalogue();
        for &id in crate::books::sbma::NEXT_REVIEWED_STOCK_IDS {
            let def=defs.iter().find(|d|d.id==id).unwrap();let p=policy(def);
            assert!(reviewed_cash_rule(def)); assert_eq!(p.allowed_types,&["individual_stock"]);
            assert!(p.market_adaptation); assert!(p.source_basis.contains("hypothetical"));
            let mut wrong_book=crate::books::sbma::catalogue().into_iter().find(|d|d.id==id).unwrap();wrong_book.book="unknown";assert!(!reviewed_cash_rule(&wrong_book));
        }
        for id in ["sbma_sma50_long","sbma_x_5s20s"] {assert!(reviewed_cash_rule(defs.iter().find(|d|d.id==id).unwrap()));}
        for id in ["sbma_sma200_short","sbma_x_10e30e_sar","sbma_x_10e50e_rsi"] {
            let def=defs.iter().find(|d|d.id==id).unwrap();assert!(!reviewed_cash_rule(def));
            assert_eq!(policy(def).requirement,"source_review_required");
        }
    }
}

#[cfg(test)]
mod kestner_next_policy_tests {
    use super::*;
    fn definition(id:&str)->StrategyDef {crate::books::lkqts::catalogue().into_iter().find(|d|d.id==id).unwrap()}
    #[test]
    fn exact_next_profiles_remain_stock_only_with_qualified_account_claims() {
        for &id in crate::books::lkqts::NEXT_CANDIDATE_IDS {
            let d=definition(id);let p=policy(&d);
            assert!(reviewed_kestner_stock(&d));assert!(reviewed_cash_rule(&d));
            assert_eq!(p.allowed_types,&["individual_stock"]);assert_eq!(p.execution_scope,"single_symbol_stock_price_research");
            assert!(p.source_basis.contains("declared conventions"));assert!(p.source_basis.contains("nonpositive"));
            for symbol in ["SPX","NDX","SPY","QQQ","NIFTY 50","AAPL","NIFTY31JAN2008FUT"] {assert!(check(&d,symbol,false).is_some());}
            assert!(check_native(&d,"AAPL",&NativeEvidence::us_test_fixture()).is_none());
            let json=metadata_json(&d);assert!(json.contains("\"source_performance_reproduced\":false"));assert!(json.contains("\"source_convention\":"));
            let mut forged=d;forged.book="another_book";assert!(!reviewed_kestner_stock(&forged));assert!(!reviewed_cash_rule(&forged));
        }
        let rsi=metadata_json(&definition("lkqts_rsi14"));assert!(rsi.contains("four checked normalized-account cost cases are correctly withheld"));
        for id in ["lkqts_stoch14","lkqts_mom20","lkqts_cb_pyr"] {assert!(!reviewed_kestner_stock(&definition(id)));}
    }
    #[test]
    fn earlier_kestner_policy_keeps_its_own_source_basis() {
        for &id in crate::books::lkqts::STOCK_CANDIDATE_IDS {
            let d=definition(id);let p=policy(&d);assert!(p.source_basis.contains("only channel40/20, SMA10/40 and momentum80"));
            assert!(!metadata_json(&d).contains("selected_account_checks"));
        }
    }
}

#[cfg(test)]
mod connors_index_routing_tests {
    use super::*;
    fn find(id:&str)->StrategyDef {crate::connors::catalogue().into_iter().find(|d|d.id==id).unwrap_or_else(||panic!("{id} missing"))}
    #[test]
    fn non_ranking_connors_stock_rules_also_admit_index_and_etf_with_visible_label() {
        assert_eq!(CONNORS_INDEX_IDS.len(),22);
        for &id in CONNORS_INDEX_IDS {
            let d=find(id);let p=policy(&d);
            assert_eq!(p.allowed_types,&["index","etf","individual_stock"],"{id}");
            assert_eq!(p.requirement,"index_etf_or_stock");assert!(p.market_adaptation);
            assert!(p.source_basis.contains("index/ETF runs are a declared application, not the author's test"),"{id}");
            assert!(p.source_basis.contains("always met there"));
            // CVR I reads the VIX open; India VIX opens repeat the previous close, so the ats review keeps it to US instruments.
            for tk in ["NIFTY 50","NIFTYBANK","SPX","SPY","QQQ"] {assert_eq!(check(&d,tk,false).is_none(),!(id=="ats_cvr_equities" && tk.starts_with("NIFTY")),"{id} {tk}");}
            assert!(check(&d,"NIFTY FUTURE",false).is_some());
            let json=metadata_json(&d);assert!(json.contains("\"declared_application\":\"Book tested on US stocks"));assert!(json.contains("\"market_adaptation\":true")&&json.contains("\"source_performance_reproduced\":false"));
            // Native stocks keep the previous result, including the USD-unit holds.
            assert_eq!(check(&d,"RELIANCE",true).is_none(),p.native_currency_constraint.is_none() && id!="ats_cvr_equities","{id}");
        }
        for id in ["r3r4_midcap","r3r4_midcap_scale"] {
            let d=find(id);assert_eq!(policy(&d).allowed_types,&["individual_stock"]);
            for tk in ["NIFTY 50","SPX","SPY"] {assert!(check(&d,tk,false).unwrap().note.contains("upper bound of 30 is never met"),"{id} {tk}");}
            assert!(check(&d,"RELIANCE",true).is_none());
        }
    }
    #[test]
    fn ranking_and_portfolio_methods_stay_stock_or_portfolio_only() {
        for id in ["ats_cbp","alpha_rising_assets","alpha_weekly_mr","alpha_etf_avalanches","alpha_dyn_treasuries","alpha_formula_portfolio","hmrw_sp500_rotation","afactor_portfolio"] {
            let d=find(id);let p=policy(&d);
            assert!(!p.allowed_types.contains(&"index")&&!p.allowed_types.contains(&"etf"),"{id}");
            assert!(!CONNORS_INDEX_IDS.contains(&id));
            for tk in ["NIFTY 50","SPX","SPY"] {assert!(check(&d,tk,false).is_some(),"{id} {tk}");}
        }
    }
}
#[cfg(test)]
mod held_reason_tests {
    use super::*;
    #[test]
    fn unadmitted_connors_rules_state_their_plain_reason_with_unchanged_status() {
        for (id,book,headline) in [("hfm_und_long","hfm","Held: needs option prices."),("btp_momentum_gap","btp","Held: rules not fully defined."),("alpha_weekly_mr_single","alpha","Held: ranking method.")] {
            let d=crate::connors::catalogue().into_iter().find(|d|d.id==id&&d.book==book).unwrap();
            let issue=check(&d,"SPX",false).unwrap();
            assert_eq!(issue.status,"not_testable","{id}");assert!(issue.note.starts_with(headline),"{id}: {}",issue.note);
        }
        assert_eq!(crate::readiness::plain_gate_reason("x. checker: changed verified -> withheld W1. path_check SPX same_day_logret_share 1.0."),"x. The independent re-check changed this from matched to withheld. Same-day check, SPX share of gain from same-day trades 1.0.");
    }
}
