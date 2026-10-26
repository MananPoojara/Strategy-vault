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
macro_rules! CCL08_INDEX_LABEL {()=>{"Book shows these day trades on US stocks; index/ETF runs are a declared application, not the author's test. The Liquidity Filter is computed on the instrument's own close x volume against the benchmark (SPX or NIFTY 50): on a cash index it is always met (no screening); on an ETF it is the fund's actual dollar volume and can screen days (IWM, MDY, SMH fall below 3-10 at times); indices without volume (NIFTY IT) are refused for LF rules. On NSE stocks the thresholds (3, 10) are the book's US numbers applied against NIFTY 50 rupee turnover (close x volume), so LF > 3 is about INR 1.7 crore/day (around USD 0.2M) versus roughly USD 20-60M on US stocks; the India filter screens far less than the US one [Not from the book]. HV100, RSI(2), 200-day and percent-move filters apply as printed."}}
macro_rules! BTP_INDEX_LABEL {()=>{"Book tested or illustrated on futures and US stocks; index/ETF runs are a declared application, not the author's test."}}
macro_rules! CWIN_INDEX_LABEL {()=>{"The book and course show Windows on US stocks, HOLDRs, indices, ETFs and futures; index/ETF runs are a declared application, not the author's test. The 100,000-share volume suggestion (Q&A p56) applies to stocks only."}}
macro_rules! WS5_INDEX_LABEL {()=>{"Seminar method for US stocks from a nightly scan; index/ETF runs are a declared application, not the author's test. Stock-only filters (price, volume, sub-sector, range, ATR) do not apply to an index."}}
macro_rules! STOCK_TICK_LABEL {()=>{"Actual stocks are admitted (the book shows this setup on US stocks). Adaptation: the book used 1/8-dollar ticks; stock runs offset entries/stops by the stock's real tick, 0.05 INR on NSE and 0.01 USD on US stocks. Vault stock prices are split/dividend-adjusted (not independently verified), so one tick is a larger fraction of early adjusted prices. One stock at a time; no stock population or book result is reproduced."}}
macro_rules! WS5_STOCK_LABEL {()=>{"Actual stocks are admitted without the filters that cannot be applied: #5 sub-sector vs its 200-day SMA is omitted for every stock (no SUBSECTOR series); on Indian stocks the USD #2 price >= $15, #6 90-day range > 6 points and #7 ATR(90) >= 1 point are omitted (no reviewed INR version) and only #3 share volume (100,000 shares/day; weekly 500,000 shares/week) is kept as printed; on US stocks #2, #3, #6 and #7 are kept in USD as printed, applied to split/dividend-adjusted prices. Results are therefore not the author's scan."}}
macro_rules! USD_PRICE_US_LABEL {()=>{"US stocks only: the book's USD price filter is applied as printed to the US stock's own USD prices (vault prices are split/dividend-adjusted, not independently verified, so early adjusted prices sit below the historical quotes). Indian stocks stay held: the USD 1-15 price band is the strategy itself."}}
macro_rules! AFACTOR_STOCK_LABEL {()=>{"US stocks: the book's USD price filter (previous close > 10) is applied as printed to the US stock's own USD prices (split/dividend-adjusted, not independently verified). Indian stocks: USD price filter omitted on INR stocks (no reviewed INR version); the share-volume filter (20-day average > 100,000 shares) is kept as printed. Not the author's US universe."}}
macro_rules! ALPHA_TREND_LABEL {()=>{"Book tested on nine US sector ETFs (XLB..XLY, not in the vault); other ETFs and index runs are a declared application, not the author's test."}}
macro_rules! CONNORS_INDEX_LABEL {()=>{"Book tested on US stocks; index/ETF runs are a declared application, not the author's test. Stock filters in the rule (minimum price, share volume, Liquidity Filter) are still calculated on the index's own price and volume and are always met there (the R3/R4 low-price $1-$15 band is never met), so they do not screen index runs."}}
pub const CONNORS_INDEX_IDS:&[&str]=&["f1a","f3a","f4a","f4c","f4d","rsi2_5d","tps_stocks","x555","h10","r3r4_base","r3r4_base_scale","r3r4_bigcap","r3r4_bigcap_scale","r3r4_bigcap_deck","r3r4_bigcap_deck_scale","r3r4_lowprice","r3r4_lowprice_scale","ats_dvmtm","ats_cvr_equities","ats_15min_adx_breakout_equities","afactor_baseline","afactor_spx_down"];

/// User decision 2026-10-22: these backtested single-instrument rules also run on actual indices, ETFs and
/// stocks (US and India) as a visibly declared application. Admission only: rules, parameters and calculations are unchanged.
pub const OPEN_ETF_RULES:&[(&str,&str)]=&[("hpetf","hpetf_3dhl_long"),("hpetf","hpetf_3dhl_long_aggr"),("hpetf","hpetf_mdd_long"),("hpetf","hpetf_mdd_long_aggr"),("hpetf","hpetf_mdu_short"),("hpetf","hpetf_mdu_short_aggr"),("hpetf","hpetf_pctb_long_aggr"),("hpetf","hpetf_pctb_short_aggr"),("hpetf","hpetf_r3_long"),("hpetf","hpetf_r3_long_aggr"),("hpetf","hpetf_rsi10_6_long"),("hpetf","hpetf_rsi90_94_short"),("hpetf","hpetf_rsi25_long_aggr"),("hpetf","hpetf_rsi75_short"),("hpetf","hpetf_tps_long"),("hpetf","hpetf_tps_short"),("stc9","tps_long"),("stc9","tps_short"),("alpha","alpha_spy_rsi4_mr"),("ststw","ststw_double7"),("ststw","ststw_overnight"),("ststw","ststw_cum_rsi_35"),("ststw","ststw_cum_rsi_50"),("hbqts","hbqts_cmo")];
pub const OPEN_STOCK_RULES:&[(&str,&str)]=&[("ehmd","ehmd_mama"),("elcim","elcim_impulse"),("hbqts","hbqts_bb_breakout"),("hbqts","hbqts_cgo"),("hbqts","hbqts_dom"),("hbqts","hbqts_dpo"),("hbqts","hbqts_entry_id1"),("hbqts","hbqts_fixed_interval"),("hbqts","hbqts_limit_entry"),("hbqts","hbqts_macd_daily"),("hbqts","hbqts_profit_target"),("hbqts","hbqts_stoch_rsi"),("kfsc","kfsc_bollinger"),("kfsc","kfsc_pct_band"),("kfsc","kfsc_hv_day"),("kfsc","kfsc_dsm"),("kfsc","kfsc_macd"),("kfsc","kfsc_two_ma"),("lkqts","lkqts_cb4020"),("lkqts","lkqts_ma1040"),("lkqts","lkqts_macd"),("lkqts","lkqts_macd_literal_alpha"),("lkqts","lkqts_mom80"),("lkqts","lkqts_rsi14"),("lkqts","lkqts_sob"),("lkqts","lkqts_tiar"),("lkqts","lkqts_volbo"),("sbma","sbma_x_10e20e"),("sbma","sbma_x_10e50e"),("sbma","sbma_x_10s200s"),("sbma","sbma_x_15e150e"),("sbma","sbma_x_15e30e"),("sbma","sbma_x_50s200s"),("sbma","sbma_x_5e50s"),("sbma","sbma_x_5s20s"),("sbma","sbma_sma50_long")];
macro_rules! OPEN_ETF_LABEL {()=>{"Book tests ETFs; index and stock runs (US and India) are a declared application (user decision 22 Oct 2026), not the author's test. Rules and parameters are unchanged."}}
macro_rules! OPEN_STOCK_LABEL {()=>{"Book tests stocks; index and ETF runs (US and India) are a declared application (user decision 22 Oct 2026), not the author's test. Rules and parameters are unchanged."}}
/// connors-leftovers (22 Oct 2026): STC IX 1-2-3-4-5 is written for 2x/3x leveraged ETFs, which the vault does not hold.
macro_rules! OPEN_LEVERAGED_LABEL {()=>{"Book trades 2x/3x leveraged equity ETFs (STC IX Session 7); the vault holds none, so runs on unleveraged ETFs, indices and stocks (US and India) are a declared application (user decision 22 Oct 2026), not the author's test. Rules and parameters are unchanged; the 21-day share-volume filter is read on the selected instrument's own volume."}}
// ---------------------------------------------------------------- VIX India (user request 24 Oct 2026: "if you don't find US VIX you can use India VIX data and do backtest")
// Admission only: every live VIX rule runs on actual indices, ETFs and stocks, US (CBOE VIX) and India (India VIX in place of
// the CBOE VIX). Rules, parameters and calculations are unchanged; VIX levels printed for the US VIX stay as printed.
// Real data limits stay: rules that read the VIX open need a real India VIX open (1-minute India VIX, readiness::reads_vix_open).
pub const VIX_INDIA_RULES:&[(&str,&str)]=&[("stc9","vix200"),("stc9","vix2"),("stc9","vix4"),("stc9","vix7"),("stc9","vix8"),("stc9","vix53_short"),("stc9","vix101_short"),
    ("spmt","spmt_vix200"),("spmt","spmt_vix2"),("spmt","spmt_vix4"),("spmt","spmt_vix7"),("spmt","spmt_vix8"),("spmt","spmt_short_vix53"),("spmt","spmt_short_vix101"),
    ("ws5","ws5_momentum_windows_any"),("ws5","ws5_single_window"),("ws5","ws5_double_window"),("ws5","ws5_hanging_window"),("ws5","ws5_gap_window_eod"),("ws5","ws5_gap_window_intraday"),("ws5","ws5_outside_window"),("ws5","ws5_vix_iii_reversal"),
    ("ats","ats_cvr1"),("ats","ats_cvr2"),("ats","ats_cvr3"),("ats","ats_cvr_equities"),("otvt","otvt_cvr1_long"),("otvt","otvt_cvr1_short"),("rsi25","r25_rsi25_vix"),("rsi25","r25_rsi25_15_vix"),
    ("ptjmt","ptjmt_cvr1"),("ptjmt","ptjmt_cvr2"),("ptjmt","ptjmt_cvr3"),("ptjmt","ptjmt_cvr5"),("ststw","ststw_vix_stretch"),("ststw","ststw_vix_rsi"),
    ("vixr","vixr_cvr1"),("vixr","vixr_cvr2"),("vixr","vixr_cvr3"),("vixr","vixr_cvr5_1d"),("vixr","vixr_cvr5_3d"),("vixr","vixr_cvr6"),("vixr","vixr_cvr7"),("vixr","vixr_cvr8"),("vixr","vixr_cvr9"),("vixr","vixr_cvr10"),("vixr","vixr_cvr11")];
pub const VIX_INDIA_LABEL:&str="The book used another instrument (futures, a cash index or an ETF); runs on actual indices, ETFs and stocks, US and India, are a declared application (user decision 23 Oct 2026; VIX India 24 Oct 2026), not the author's test. US instruments read the CBOE VIX. Indian instruments (NIFTY 50, NIFTYBANK, Indian stocks): declared application: India VIX in place of the CBOE VIX. VIX levels and VIX-point thresholds printed for the US VIX are kept as printed; India VIX runs on a different scale, so on Indian instruments they mark different market conditions. Rules that read the VIX open use the real India VIX open from 1-minute data (the first minute after NSE's 09:15 opening print), available from 2015; NSE's daily India VIX open repeats the previous close and is never used.";
pub fn vix_india(def:&StrategyDef)->bool {VIX_INDIA_RULES.contains(&(def.book,def.id))}
/// Held on Indian instruments (user decision): the independent reference and the engine differ on an exact tie.
pub fn vix_india_held(def:&StrategyDef,ticker:&str,india:bool)->Option<Issue> {
    (india && def.id=="vixr_cvr11").then(||Issue{status:"not_testable",note:format!("Held on Indian instruments ({ticker}): CVR 11 compares yesterday's VIX 3/10-day oscillator with -1 VIX point. On India VIX the oscillator is exactly -1.00 on 2012-04-03; the book's rule (at or below -1) sells, but the engine's floating-point average gives -0.99999... and does not, so the engine and the independent check differ from that trade on. Not published until the tie handling is decided. US runs (CBOE VIX) match the check.")})
}
// ---------------------------------------------------------------- end VIX India
// ---------------------------------------------------------------- connors-final (26 Oct 2026)
/// US ETFs the engine serves from us_data.csv. connors-final adds the Alpha Formula ETFs (Rising Assets, Dynamic
/// Treasuries, ETF Avalanches universes; real Yahoo daily OHLCV staged in strategy-vault-data-staging/etf-alpha-20261026).
pub const US_ETFS:&[&str]=&["SPY","QQQ","IWM","MDY","SMH","DIA","EFA","EEM","VNQ","LQD","GLD","SHY","IEF","TLT","AGG","IEI","TLH",
    "EWA","EWC","EWD","EWG","EWH","EWI","EWJ","EWK","EWL","EWM","EWN","EWO","EWP","EWQ","EWS","EWU","EWW",
    "XLB","XLE","XLF","XLI","XLK","XLP","XLU","XLV","XLY","EZU","EWZ","EWT","EWY","EPP","FXI","EZA"];
/// The Dynamic Treasuries allocation rule is written for the three duration ETFs (IEI takes the remainder).
pub const ALPHA_DT_ETFS:&[&str]=&["IEF","TLH","TLT"];
/// VIX Gap Window: India only (NIFTY 50 with India VIX), on series built from real 1-minute India VIX and NIFTY 50 bars.
pub const CWIN_VIX_GAP_LABEL:&str="The course trades SPY / S&P E-minis on the CBOE VIX. This run is NIFTY 50 with India VIX in place of the CBOE VIX, a declared application (not the author's test). The intraday moment the VIX breaks yesterday's high or low comes from the vault's cleaned 1-minute India VIX and NIFTY 50 bars (from 2015-01-09); the entry is the NIFTY 50 price at the close of that minute and the same-day stop is decided on the later minutes. The exit is the official daily close. US runs stay held: the vault has no intraday CBOE VIX.";
/// STSTW Ch 6 / 8 / 13 stock tests: actual stocks (basket runs replay them across a universe); indices/ETFs declared.
pub const STSTW_STOCK_LABEL:&str="Short Term Trading Strategies That Work tested every US stock 1995-2007 (100-day average volume >= 250,000 shares, price > $5, delisted names included) and printed pooled statistics. Vault runs are single stocks, or a basket run over the Norgate US universes (today's S&P 500 / Russell members, plus past members where the basket says so; survivorship bias), one position at a time per stock: a declared application, not the author's test. Index and ETF runs are a declared application too. Rules with the $5 price filter run on US data only.";
pub const ALPHA_DT_LABEL:&str="The Alpha Formula applies this weekly allocation rule to IEF, TLH and TLT inside a four-ETF Treasury portfolio (IEI takes the rest). This is the single-ETF sleeve on the actual ETF: the unallocated part earns nothing here instead of IEI, and vault prices are split-adjusted, not dividend-adjusted (the book used total returns). It is not the book's portfolio result.";
/// Crash, Burn and Profit (PTJ Apr 1997, in ATS Ch 11 and Best Trading Patterns I): the $20 price filter is kept as
/// printed, so it runs on US data only (user rule 21 Oct 2026); the IBD RS 98-99 scan is the author's candidate source,
/// not one of the five rules: every US stock is scanned by the rules themselves (weekly ADX above 60).
pub const CBP_LABEL:&str="The article (PTJ April 1997) shorts single high-flying US stocks; its five rules (weekly ADX above 60, price above $20, the first lower weekly ADX, a 15% stop, half covered at 20% profit) are run as printed on US stocks, and on US indices and ETFs as a declared application. The author found candidates in an IBD Relative Strength 98-99 list; that list is not one of the rules and is not available, so every stock is scanned by the rules themselves. The trailing stop for the second half is not given in the article: [Not from the book] the highest high of the prior 20 days. The $20 filter is in US dollars, so Indian instruments are not run.";
pub fn cbp_us(def:&StrategyDef)->bool {matches!((def.book,def.id),("ats","ats_cbp")|("btp","btp_cbp_short"))}
pub fn connors_final_reviewed(def:&StrategyDef)->bool {
    cbp_us(def) ||
    (def.book==crate::books::cwin::BOOK && def.id=="cwin_vix_gap_window") || (def.book==crate::books::ststw::BOOK && crate::books::ststw::stock_study(def.id))
        || (def.book==crate::books::alpha::BOOK && def.id=="alpha_dyn_treasuries_sleeve")
}
fn connors_final_label(def:&StrategyDef)->Option<&'static str> {
    if !connors_final_reviewed(def) {return None;}
    Some(match def.book {_ if cbp_us(def)=>CBP_LABEL,"cwin"=>CWIN_VIX_GAP_LABEL,"alpha"=>ALPHA_DT_LABEL,_=>STSTW_STOCK_LABEL})
}
fn connors_final_policy(def:&StrategyDef)->Option<Policy> {
    if !connors_final_reviewed(def) {return None;}
    let mut p=match def.book {
        "cwin"=>requirement("declared_index_timing", &["index"], CWIN_VIX_GAP_LABEL),
        _ if cbp_us(def)=>requirement("index_etf_or_stock", &["index","etf","individual_stock"], CBP_LABEL),
        "alpha"=>requirement("etf", &["etf"], ALPHA_DT_LABEL),
        _=>requirement("index_etf_or_stock", &["index","etf","individual_stock"], STSTW_STOCK_LABEL),
    };
    if def.book==crate::books::ststw::BOOK {p.preferred_basket=Some("us_sp500");}
    p.market_adaptation=true;Some(p)
}
/// us: whether the selected instrument is a US one (None = decide from the ticker).
fn connors_final_symbol_issue(def:&StrategyDef,ticker:&str,native_stock:bool,us:Option<bool>)->Option<Issue> {
    if !connors_final_reviewed(def) {return None;}
    if cbp_us(def) {
        let us=us.unwrap_or(!native_stock && !ticker.starts_with("NIFTY"));
        return (!us).then(||Issue{status:"needs_data",note:format!("Crash, Burn and Profit keeps only stocks priced above $20. A dollar price filter means what the article meant only on US prices, so it runs on US stocks, indices and ETFs only (user rule: $-amount rules run on US data with the literal amount). {ticker} is not a US instrument; no result is published.")});
    }
    if def.book==crate::books::ststw::BOOK {
        let us=us.unwrap_or(!native_stock && !ticker.starts_with("NIFTY"));
        return (crate::books::ststw::usd_filter(def.id) && !us).then(||Issue{status:"needs_data",note:format!("This test keeps only stocks priced above $5 (p.43). A dollar price filter means what the book meant only on US prices, so it runs on US stocks, indices and ETFs only (user rule: $-amount rules run on US data with the literal amount). {ticker} is not a US instrument; no result is published.")});
    }
    if def.book==crate::books::alpha::BOOK {
        return (native_stock || !ALPHA_DT_ETFS.contains(&ticker)).then(||Issue{status:"needs_data",note:format!("The Dynamic Treasuries rule (Alpha Formula Ch 7) allocates to the Treasury ETFs IEF, TLH and TLT. {ticker} is not one of them; an equity index, equity ETF or stock would change the asset class, so no result is published.")});
    }
    if ticker=="NIFTY 50" && !native_stock {return None;}
    Some(Issue{status:"needs_data",note:format!("The VIX Gap Window needs the moment during the day when the VIX breaks yesterday's high or low, together with the traded market's price at that minute. The vault has that only for NIFTY 50 with India VIX (1-minute bars of both). {ticker} has no matching intraday volatility-index data (there is no intraday CBOE VIX), so no result is published.")})
}
// ---------------------------------------------------------------- NYSE breadth (user request 26 Oct 2026, connors-final)
// The NYSE-breadth versions of the Connors TRIN / advance-decline rules read the vault's real NYSE breadth (extra_data/us,
// to 2020-02-10) with the book's NYSE thresholds. Admission only: they run on every US index, ETF and stock as a
// declared application; the NSE-breadth versions stay held (user decision 21 Oct 2026) and Indian instruments are refused.
pub const NYSE_BREADTH_RULES:&[(&str,&str)]=&[("ats","ats_trin_thrust"),("ats","ats_padi"),("hfm","hfm_chadtp"),
    ("ptjmt","ptjmt_trin_reversals"),("ptjmt","ptjmt_trin_reversals_20"),("ptjmt","ptjmt_trin_thrusts"),("ptjmt","ptjmt_padi"),
    ("spmt","spmt_trin_1"),("spmt","spmt_trin_2"),("spmt","spmt_trin_3"),("spmt","spmt_trin_4"),("spmt","spmt_trin_5"),("spmt","spmt_nyse_adv"),
    ("ss","ss_ad_market_buster")];
pub const NYSE_BREADTH_LABEL:&str="The book traded S&P 500 futures on NYSE breadth (NYSE TRIN / NYSE advances and declines). The signal is read from the real NYSE breadth series with the book's NYSE thresholds; runs on US indices, ETFs and stocks other than the book's market are a declared application (user request 26 Oct 2026), not the author's test. NYSE breadth in the vault ends on 2020-02-10 and is missing on a few days (for example 1999-11-26, 2002-06-20, 2019-08-20); a run uses the longest stretch with no missing day. Indian instruments are not run: the NSE-breadth version is held (thresholds not calibrated for India).";
pub fn nyse_breadth(def:&StrategyDef)->bool {NYSE_BREADTH_RULES.contains(&(def.book,def.id))}
// ---------------------------------------------------------------- end NYSE breadth
pub fn open_instrument_label(def:&StrategyDef)->Option<&'static str> {
    let key=(def.book,def.id);
    if nyse_breadth(def) {return Some(NYSE_BREADTH_LABEL);} // NYSE breadth block
    if vix_india(def) {return Some(VIX_INDIA_LABEL);} // VIX India block
    if key==("stc9","s12345") {return Some(OPEN_LEVERAGED_LABEL!());}
    if OPEN_ALL_RULES.contains(&key) {return Some(OPEN_ALL_LABEL);} // open-all block
    if OPEN_ETF_RULES.contains(&key) {Some(OPEN_ETF_LABEL!())} else if OPEN_STOCK_RULES.contains(&key) {Some(OPEN_STOCK_LABEL!())} else {None}
}
// ---------------------------------------------------------------- open-all (user request 23 Oct 2026: "remove the 'Available on X' thing so I can backtest all")
// Admission only: backtested variants that were refused on some instrument types only because the review named fewer
// instruments now run on actual indices, ETFs and stocks, US and India. Rules, parameters and calculations are unchanged.
// Real data limits stay: rules that read the VIX open or a fixed US-VIX level stay off Indian instruments.
// Not listed (held for a user decision): STC IX dmi_1-5, pctb_2, price, r3 and S&P Market Timing pctb_2 / price: the
// independent ledgers differ by one trade on an exact tie (floating-point rounding in the engine), authors/open-all/REPORT.md.
// Also not listed: QTS DEMA 16/8 (hbqts_dema16_8) - its canonical unit-exposure report (frontend) is built for SPX / NIFTY 50 only.
pub const OPEN_ALL_RULES:&[(&str,&str)]=&[("alpha","alpha_etf_avalanches_single"),("alpha","alpha_trend_6m"),("ats","ats_hv_thrust"),("bllstr","bllstr_fma_1_50_0"),("bllstr","bllstr_vma_1_50_0"),("btp","btp_triple_day_pullback_adx25"),("btp","btp_triple_day_pullback_adx30"),("btp","btp_turtle_thrust"),("ccl08","ccl08_tps_long"),("ccl08","ccl08_tps_research"),("ccl08","ccl08_tps_short"),("kftsm","kftsm_hirsch"),("kftsm","kftsm_month_end"),("lwrsr","lwrsr_apr_oct"),("lwrsr","lwrsr_oct_apr"),("mmft","mmft_nasdaq79"),("otvt","otvt_cvr1_long"),("otvt","otvt_cvr1_short"),("otvt","otvt_hv_thrust"),("ptjmt","ptjmt_cvr1"),("ptjmt","ptjmt_cvr2"),("ptjmt","ptjmt_cvr3"),("ptjmt","ptjmt_cvr5"),("radge","radge_oc_long"),("rsi25","r25_rsi25_15"),("rsi25","r25_rsi25_15_vix"),("rsi25","r25_rsi25_basic_no_filter"),("rsi25","r25_rsi25_official_sma200"),("rsi25","r25_rsi25_vix"),("spmt","spmt_dmi_1"),("spmt","spmt_dmi_2"),("spmt","spmt_dmi_3"),("spmt","spmt_dmi_4"),("spmt","spmt_dmi_5"),("spmt","spmt_eom_cc"),("spmt","spmt_eom_und"),("spmt","spmt_eom_co"),("spmt","spmt_eom_rsi"),("spmt","spmt_pctb_1"),("spmt","spmt_rsi_div2"),("spmt","spmt_rsi_div8"),("spmt","spmt_rsi2_4"),("spmt","spmt_rsi4_25"),("spmt","spmt_rsi_1"),("spmt","spmt_rsi_2"),("spmt","spmt_rsi_3"),("spmt","spmt_short_strong"),("spmt","spmt_short_4up"),("spmt","spmt_short_vix101"),("spmt","spmt_short_vix53"),("spmt","spmt_vix2"),("spmt","spmt_vix200"),("spmt","spmt_vix4"),("spmt","spmt_vix7"),("spmt","spmt_vix8"),("ss","ss_adx_gapper"),("ss","ss_study_channel_2period_sar"),("ss","ss_study_fresh_roc_buy"),("ss","ss_study_fresh_roc_sell"),("ss","ss_study_gap_failure_buy"),("ss","ss_study_gap_failure_sell"),("ss","ss_study_oops_adx_gapper_buy"),("ss","ss_study_oops_adx_gapper_sell"),("ss","ss_study_oops_buy"),("ss","ss_study_oops_sell"),("ss","ss_study_close_lower_10"),("ss","ss_study_close_lower_20"),("ss","ss_study_close_upper_80"),("ss","ss_study_close_upper_90"),("ss","ss_study_wr7_high_close"),("ss","ss_study_wr7_low_close"),("ss","ss_study_rocrsi_buy"),("ss","ss_study_rocrsi_sell"),("ss","ss_whiplash"),("stc9","eom_cc"),("stc9","eom_co"),("stc9","eom_rsi"),("stc9","short_4up"),("stc9","short_strong"),("stc9","vix101_short"),("stc9","vix53_short"),("stc9","pctb_1"),("stc9","rsi_1"),("stc9","rsi_2"),("stc9","rsi2_4"),("stc9","rsi_3"),("stc9","rsi4_25"),("stc9","rsi_div2"),("stc9","rsi_div8"),("stc9","trin2_11"),("stc9","trin3_10"),("stc9","trin3_11"),("stc9","trin4"),("stc9","trin4_12"),("stc9","vix2"),("stc9","vix200"),("stc9","vix4"),("stc9","vix7"),("stc9","vix8"),("ststw","ststw_rsi2_under5"),("ststw","ststw_10d_high_buy"),("ststw","ststw_10d_high_short"),("ststw","ststw_10d_low_buy"),("ststw","ststw_sp_short"),("ststw","ststw_vix_rsi"),("ststw","ststw_vix_stretch"),("vixr","vixr_cvr10"),("vixr","vixr_cvr1"),("vixr","vixr_cvr2"),("vixr","vixr_cvr3"),("vixr","vixr_cvr5_1d"),("vixr","vixr_cvr5_3d"),("vixr","vixr_cvr6"),("vixr","vixr_cvr7"),("vixr","vixr_cvr9"),("ws5","ws5_non_vix_market_timing"),("ws5","ws5_vix_iii_reversal")];
pub const OPEN_ALL_LABEL:&str="The book used another instrument (futures, a cash index or an ETF); runs on actual indices, ETFs and stocks, US and India, are a declared application (user decision 23 Oct 2026), not the author's test. Rules and parameters are unchanged. Rules that read the VIX use the CBOE VIX on US instruments and India VIX on Indian ones.";
pub fn open_all(def:&StrategyDef)->bool {OPEN_ALL_RULES.contains(&(def.book,def.id)) || nyse_breadth(def)}
/// Book-specific symbol lists still decide what the VIX allows: the selected instrument is checked as its market's
/// reviewed index (US -> SPX, India -> NIFTY 50). us: None = decide from the ticker (NIFTY... = India).
fn open_all_symbol_issue(def:&StrategyDef,ticker:&str,us:Option<bool>)->Option<Issue> {
    use crate::books::{vixr,spmt,ptjmt,ss,ws5,ststw,ats,hfm,otvt};
    if vix_india(def) {return None;} // VIX India block: India VIX on Indian instruments, CBOE VIX on US ones
    let us=us.unwrap_or(!ticker.starts_with("NIFTY"));
    if nyse_breadth(def) { // NYSE breadth block
        return (!us).then(||Issue{status:"needs_data",note:format!("This version reads NYSE breadth (NYSE TRIN / NYSE advances-declines), a US market series; it runs on US indices, ETFs and stocks and is not run on Indian instruments ({ticker}). The NSE-breadth version is held: it uses the book's NYSE TRIN/advance-decline thresholds; NSE breadth behaves differently (e.g. TRIN median 0.70 vs NYSE 0.93; fires 2-3x more often), so the thresholds are not calibrated for India. Not published until recalibrated and reviewed.")});
    }
    let t=if us {"SPX"} else {"NIFTY 50"};
    let ok=match def.book {
        "vixr"=>vixr::admitted_symbol(def.id,t), "spmt"=>spmt::admitted_symbol(def.id,t), "ptjmt"=>ptjmt::admitted_symbol(def.id,t),
        "ss"=>ss::admitted_symbol(def.id,t) || !ss::reviewed_daily(def.id), "ws5"=>ws5::admitted_symbol(def.id,t), "ststw"=>ststw::admitted_symbol(def.id,t) || !ststw::reviewed_daily(def.id),
        "ats"=>ats::admitted_symbol(def.id,t), "hfm"=>hfm::admitted_symbol(def.id,t), "otvt"=>otvt::admitted_symbol(def.id,t),
        _=>true,
    };
    (!ok).then(||Issue{status:"needs_data",note:format!("This rule reads the VIX open or uses VIX levels set for the US (CBOE) VIX. In the vault, India VIX opens repeat the previous close and its levels are not the CBOE VIX, so the rule is not run on Indian instruments ({ticker}). It runs on US indices, ETFs and stocks. No result is published here.")})
}
// ---------------------------------------------------------------- end open-all

pub fn reviewed_kestner_stock(def:&StrategyDef)->bool {
    def.book==crate::books::lkqts::BOOK && crate::books::lkqts::stock_research_profile(def.id)
}

// ---------------------------------------------------------------- Bernstein g1 (jb624, jbcaf, jbssf) 2026-10-22
// Admission only for the listed ids (authors/bernstein/g1-admission/SOURCE-CONTRACT.md); kept in one block to merge.
macro_rules! JB624_LABEL {()=>{"6-2-4 Winning Strategies shows these methods on futures (the MAC, 3HL and Stochastic POP also on stocks); index, ETF and stock runs (US and India) are a declared application, not the author's test."}}
macro_rules! JBCAF_LABEL {()=>{"Cyclic Analysis in Futures Trading shows these timing rules on futures charts; index, ETF and stock runs (US and India) are a declared application, not the author's test."}}
macro_rules! JBSSF_LABEL {()=>{"How to Trade the New Single Stock Futures trades single stock futures (S&P futures for the seasonal); actual stocks, indices and ETFs (US and India) are a declared application, not the author's test."}}
macro_rules! JB624_FRIMON_LABEL {()=>{"The Friday-Monday system was tested on S&P 500 futures with S&P-point offsets and stops (p.130); it runs on the SPX cash index only (real opens from 2014) as a declared application. The points are not rescaled to other instruments."}}
pub fn bernstein_g1_reviewed(def:&StrategyDef)->bool {
    use crate::books::{jb624,jbcaf,jbssf};
    (def.book==jb624::BOOK && jb624::reviewed(def.id)) || (def.book==jbcaf::BOOK && jbcaf::reviewed(def.id)) || (def.book==jbssf::BOOK && jbssf::reviewed(def.id))
}
fn bernstein_g1_label(def:&StrategyDef)->Option<&'static str> {
    if !bernstein_g1_reviewed(def) {return None;}
    Some(match def.book {_ if def.id=="jb624_fri_mon"=>JB624_FRIMON_LABEL!(), "jb624"=>JB624_LABEL!(), "jbcaf"=>JBCAF_LABEL!(), _=>JBSSF_LABEL!()})
}
fn bernstein_g1_policy(def:&StrategyDef)->Option<Policy> {
    if !bernstein_g1_reviewed(def) {return None;}
    let mut p=if def.id=="jb624_fri_mon" {
        requirement("declared_index_timing", &["index"], concat!("Bernstein, 6-2-4 Winning Strategies and Systems (2006), Trading System I pp.126-131, reviewed in authors/bernstein/g1-admission/SOURCE-CONTRACT.md with independent chronological SPX ledgers. ",JB624_FRIMON_LABEL!()," The entry-day stop uses a declared open-near-far-close bar path. No futures dollars or the book's 198-trade result are claimed."))
    } else {
        requirement("index_etf_or_stock", &["index","etf","individual_stock"], match def.book {
            "jb624"=>concat!("Bernstein, 6-2-4 Winning Strategies and Systems (2006), MAC, 5-bar MAC, MAC with AD/MA, MAC swing, Three High Low MAC, Stochastic POP and 2-Day Breakaway Gap (pp.32-50, 76, 79-81), reviewed in authors/bernstein/g1-admission/SOURCE-CONTRACT.md with independent chronological ledgers. ",JB624_LABEL!()," Order types the book leaves open (close fills, yesterday's MA levels as limits, the 2-day gap's half at the target and 75%-of-best-profit trail) are declared readings. Rules that read the open start where SPX/NDX/DJI opens are real; NIFTY opens before 2011 are close to the prior close and are disclosed. Cash-index shorts are hypothetical exposure. No futures dollars or book performance are claimed."),
            "jbcaf"=>concat!("Bernstein, Cyclic Analysis in Futures Trading (1988), stochastic 25/75 guidelines (Fig 7.1 p.177, p.182-183) and the 10-high/8-low MA channel (Fig 9.8 p.213), reviewed in authors/bernstein/g1-admission/SOURCE-CONTRACT.md with independent chronological ledgers. ",JBCAF_LABEL!()," Close fills and stop-and-reverse are declared readings; the cycle time windows the author prefers are judgement and are not used. Cash-index shorts are hypothetical exposure. No results are printed in the book."),
            _=>concat!("Bernstein, How to Trade the New Single Stock Futures (2002), three-MA 4/9/18 (p.90-92), gap trade (p.132), MOM/MA 28 (p.136, Fig 11.16) and the S&P Jan 12-18 seasonal (Fig 12.1 p.142-143), reviewed in authors/bernstein/g1-admission/SOURCE-CONTRACT.md with independent chronological ledgers. ",JBSSF_LABEL!()," The gap-down line's 'sell short' is read as a buy (p.128); no stop level is given for the gap trade or the MOM/MA timing method, so none is applied. The gap test reads the open (SPX/NDX/DJI from their real-open dates). The earlier same-day withhold does not apply to the gap trade: the open must be beyond yesterday's extreme before price can trade back to it, and the exit is the close, so daily bars fix the order. The Jan 12-18 seasonal matches the book's 19 entry and exit dates on SPX (win rate differs: cash index vs futures). Cash-index shorts are hypothetical exposure. No futures dollars or book performance are claimed."),
        })
    };
    p.preferred_basket=Some("nifty50");p.market_adaptation=true;Some(p)
}
fn bernstein_g1_symbol_issue(def:&StrategyDef,ticker:&str,native_stock:bool)->Option<Issue> {
    if def.book!=crate::books::jb624::BOOK || def.id!="jb624_fri_mon" || (!native_stock && ticker=="SPX") {return None;}
    Some(Issue{status:"needs_data",note:format!("The Friday-Monday system's offsets and stops are S&P 500 futures points (5.40 / 0.10 entry offsets, 20 / 10.2 point stops, p.130). This review admits the SPX cash index only; {ticker} would need rescaled amounts the book does not give. No result is published.")})
}
// ---------------------------------------------------------------- end Bernstein g1
/// Exact source-rule IDs reviewed against selected real-price ledgers.
/// Each batch retains its instrument and readiness limits; original studies are not reproduced.
// ---- intraday phase 1 block (claude/intraday-p1, 2026-10-24): rules admitted on real 1-minute bars
/// Day-trade rules whose same-day order is resolved minute by minute (src/intraday.rs). Admitted on the instruments
/// that have cleaned 1-minute bars: SPY (2000-2025), SPX (2008-2026), NIFTY 50 (2008-2026), NIFTYBANK (2010-2026).
/// No stock or other ETF has minute data, so those stay held. Evidence: data-survey/PHASE1-REPORT.md.
pub const INTRADAY_P1:&[&str]=&["ss_momentum_pinball","ss_80_20","ats_15min_adx_breakout","ptjdt_adx15_breakout"];
pub const INTRADAY_P1_TICKERS:&[&str]=&["SPY","SPX","NIFTY 50","NIFTYBANK"];
pub fn intraday_p1_id(id:&str)->bool {INTRADAY_P1.contains(&id) || intraday_w1_id(id)}
pub fn intraday_p1(def:&StrategyDef)->bool {intraday_p1_id(def.id) && matches!(def.book,"ss"|"ats"|"ptjdt"|"btp"|"hfm"|"tdnmtt"|"baboops"|"tcdt"|"lwdg2"|"lwlts"|"jb30d")}
// ---- intraday wave 1 (claude/intraday-w1, 2026-10-24): same block, more day-trade rules on the minute path
/// No fixed amount: run on all four minute tickers (declared application).
pub const INTRADAY_W1:&[&str]=&["ats_10pct_oops","ats_wide_range_gap","btp2_10pct_oops","btp2_10pct_oops_long","btp2_10pct_oops_short",
    "btp2_wide_range_exhaustion_gap_short","tdnmtt_trap_long","tdnmtt_trap_short","lwdg2_0105","jb30d_gap_pct"];
/// Book levels in fixed S&P points (.50/2.00 points, 15 ticks of 0.05, $3,000 = 12 points, Crabel's 80-600 "points"):
/// SPX only, the market those points belong to. No conversion to SPY or NSE prices is invented.
pub const INTRADAY_W1_SPX:&[&str]=&["hfm_vix","baboops_oops","lwlts_mon_vb","jb30d_gap_tick"];
pub fn intraday_w1_id(id:&str)->bool {INTRADAY_W1.contains(&id) || intraday_w1_spx_id(id)}
pub fn intraday_w1_spx_id(id:&str)->bool {INTRADAY_W1_SPX.contains(&id) || crate::books::tcdt::same_day_orb(id)}
pub fn intraday_tickers(id:&str)->&'static [&'static str] {if intraday_w1_spx_id(id) {&["SPX"]} else {INTRADAY_P1_TICKERS}}
const INTRADAY_W1_SPX_BASIS:&str="Intraday wave 1 (2026-10-24): the book's same-day rule is run on real SPX 1-minute bars (cash index, 2008-01..2026-08; rules that read the open start 2014-01, where the daily SPX open is real), cleaned by library/intraday/m1_normalise.py: padded, no-open and stale-open minutes are removed and never fill; excluded days make no trade. The book's levels are fixed S&P 500 points, so the rule runs only on SPX, at those printed points; on today's index levels they are much smaller relative to price than in the book's years. The book traded S&P 500 futures, so SPX runs are a declared application; cash-index shorts are hypothetical exposure. Stops and entries fill at the level, or at the minute's open when the minute opens through it; conventions for a minute that reaches two levels are stated in the notes. Independent Python ledgers matched the engine with zero mismatches. No book performance is reproduced.";
// ---- end intraday wave 1
const INTRADAY_P1_BASIS:&str="Intraday phase 1 (2026-10-24): the book's same-day rule is run on real 1-minute bars (SPY 2000-01..2025-10, SPX cash index 2008-01..2026-08, NIFTY 50 2008-01..2026-10, NIFTYBANK 2010-03..2026-04), cleaned by library/intraday/m1_normalise.py: padded, no-open and stale-open minutes are removed and never fill; outage, late-start and truncated days are excluded and make no trade. Stops fill at the level, or at the minute's open when the minute opens through it. The book traded S&P and other futures (and stocks for some setups), so SPY, SPX, NIFTY 50 and NIFTYBANK runs are a declared application, not the author's test; cash-index shorts are hypothetical exposure. Conventions for a minute that reaches two levels are stated in the notes. Licence: the SPY 1-minute and NIFTY 2008-2014 sources state no licence (internal research). Independent Python ledgers matched the engine with zero mismatches. No book performance is reproduced.";
fn intraday_p1_symbol_issue(def:&StrategyDef,ticker:&str,native_stock:bool)->Option<Issue> {
    if !intraday_p1(def) || (!native_stock && intraday_tickers(def.id).contains(&ticker)) {return None;}
    if intraday_w1_spx_id(def.id) && (native_stock || INTRADAY_P1_TICKERS.contains(&ticker)) { // intraday wave 1
        return Some(Issue{status:"needs_data",note:format!("This day-trade rule uses the book's fixed S&P 500 point levels, which only mean what the book meant on the S&P 500 itself, and it needs real 1-minute bars. It runs on SPX only; {ticker} would need an invented conversion, so no result is published.")});
    }
    Some(Issue{status:"needs_data",note:format!("This day-trade rule needs real 1-minute bars to know which price came first during the day. The vault holds cleaned 1-minute bars only for SPY, SPX, NIFTY 50 and NIFTYBANK; {ticker} has none, so no result is published.")})
}
// ---- end intraday phase 1 block
// ---- options phase 2 block (claude/options-p2, 2026-10-24): rules admitted on real EOD option prices
/// Option rules run on the real monthly-option chain of the underlying (src/options.rs): NSE index options from the
/// F&O bhavcopy (NIFTY 2001-2026, BANKNIFTY 2005-2026) and SPY/QQQ/IWM options (2008/2011-2025). Each id runs only where
/// its rule can be priced. Evidence: data-survey/PHASE2-REPORT.md.
pub fn options_p2(def:&StrategyDef)->bool {crate::options::is_option_rule(def.id) && matches!(def.book,"oiss"|"otvt"|"ccl08"|"mahrs"|"spmt")}
pub fn options_tickers(id:&str)->&'static [&'static str] {
    match id {
        "otvt_cvr1_sell_puts"=>&["SPY"],
        "ccl08_weekly_rsi_options"|"mahrs_covered_calls"=>&["SPY","QQQ","IWM","NIFTY 50","NIFTYBANK"],
        _=>&["NIFTY 50","NIFTYBANK"], // OISS, spmt_eom_puts: need the option's own open/high (India only)
    }
}
const OPTIONS_P2_BASIS:&str="Options phase 2 (2026-10-24): the rule trades real option contracts. Prices come from the NSE F&O end-of-day bhavcopy (NIFTY and BANKNIFTY monthly index options) and the SPY/QQQ/IWM 16:00 ET option snapshot (2008-2025; QQQ from 2011), built by library/options/build_{india,us}.py; monthly expiries only. A fill needs a contract that traded that day (India Contracts > 0 at its close or open; US volume > 0 at the 16:00 mid (bid+ask)/2, labelled, no bid/ask edge). Open positions are valued every day at the held contract's own price (India settlement price / US two-sided mid on untraded days, labelled). The books traded OEX, S&P, QQQ, SPY or stock options in 1997-2009, so these runs are a declared application on our market and period. Independent Python ledgers matched the engine with zero mismatches. No book performance is reproduced.";
fn options_p2_symbol_issue(def:&StrategyDef,ticker:&str,native_stock:bool)->Option<Issue> {
    if !options_p2(def) || (!native_stock && options_tickers(def.id).contains(&ticker)) {return None;}
    let us=matches!(ticker,"SPY"|"QQQ"|"IWM");
    let note=if us && options_tickers(def.id)==["NIFTY 50","NIFTYBANK"] {
        format!("This option rule needs the option's own intraday high or opening price (a 2x stop on the option's high, or a next-morning exit). The US option data hold only the 16:00 snapshot (bid, ask, last), so {ticker} cannot be priced as the book trades it; the rule runs on NIFTY 50 and NIFTY BANK index options. No result is published.")
    } else {
        format!("This rule trades real option contracts. The vault holds traded option prices only for NIFTY and BANKNIFTY index options (NSE) and SPY, QQQ and IWM options (US); this rule is admitted on {}. {ticker} has no option prices, and the underlying's own price cannot stand in for an option's, so no result is published.", options_tickers(def.id).join(", "))
    };
    Some(Issue{status:"needs_data",note})
}
// ---- end options phase 2 block
pub fn reviewed_cash_rule(def:&StrategyDef)->bool {
    if intraday_p1(def) {return true;} // intraday phase 1
    if options_p2(def) {return true;} // options phase 2
    if cbp_us(def) {return true;} // connors-final block: CBP on US data with the literal $20
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
        || bernstein_g1_reviewed(def)
        || schwager_reviewed(def) // Schwager block
        || elder_reviewed(def) // Elder block
        || pring_reviewed(def) // Pring block
        || jobman_reviewed(def) // Jobman block
        || schaap_reviewed(def) // Schaap block
        || satchell_reviewed(def) // Satchell block
        || kirkpatrick_reviewed(def) // Kirkpatrick block
        || goslin_reviewed(def) // Goslin block
        || ruggiero_reviewed(def) // Ruggiero block
        || link_reviewed(def) // Link block
        || katz_reviewed(def) // Katz block
        || crabel_reviewed(def) // Crabel block
        || stad_reviewed(def) // STAD block
        || masonson_reviewed(def) // Leslie Masonson block
        || collins_reviewed(def) // Art Collins block
        || guppy_reviewed(def) // Guppy block
        || headley_reviewed(def) // Headley block
        || jcarter_reviewed(def) // Carter block
        || bryant_reviewed(def) // Bryant block
        || gettess_reviewed(def) // Gettess block
        || primo_reviewed(def) // Primo block
        || meyers_reviewed(def) // Meyers block
        || krutsinger_reviewed(def) // Krutsinger block
        || clayburg_reviewed(def) // Clayburg block
        || stendahl_reviewed(def) // Stendahl block
        || boucher_reviewed(def) // Boucher block
        || freeburg_reviewed(def) // Freeburg block
        || rjones_reviewed(def) // RJones block
        || blau_reviewed(def) // Blau block
        || faith_reviewed(def) // Faith block
        || dunnigan_reviewed(def) // Dunnigan block
        || katsanos_reviewed(def) // Katsanos block
        || tomasini_reviewed(def) // Tomasini block
        || pardo_reviewed(def) // Pardo block
        || aan_reviewed(def) // Aan block
        || lakonishok_reviewed(def) // Lakonishok block
        || sullivan_reviewed(def) // Sullivan block
        || campbell_reviewed(def) // Campbell block
        || heitkoetter_reviewed(def) // Heitkoetter block
        || dorsey_reviewed(def) // Dorsey block
        || morris_reviewed(def) // Morris block
        || bwilliams_reviewed(def) // BWilliams block
        || stridsman_reviewed(def) // Stridsman block
        || weissman_reviewed(def) // Weissman block
        || raschke_reviewed(def) // Raschke block
        || carnold_reviewed(def) // CArnold block
        || ndavis_reviewed(def) // NDavis block
        || siegel_reviewed(def) // Siegel block
        || lydon_reviewed(def) // Lydon block
        || landry_reviewed(def) // Landry block
        || pruitt_reviewed(def) // Pruitt block
        || hirsch_reviewed(def) // Hirsch block
        || bandy_reviewed(def) // Bandy block
        || kestner_reviewed(def) // Kestner block
        || wirrick_reviewed(def) // Wirrick block
        || sperandeo_reviewed(def) // Sperandeo block
        || csmith_reviewed(def) // CSmith block
        || lien_reviewed(def) // Lien block
        || arms_reviewed(def) // Arms block
        || gregoriou_reviewed(def) // Gregoriou block
        || crisp_reviewed(def) // Crisp block
        || fabozzi_reviewed(def) // Fabozzi block
        || fontanills_reviewed(def) // Fontanills block
        || thomsett_reviewed(def) // Thomsett block
        || patel_reviewed(def) // Patel block
        || demark_reviewed(def) // DeMark block
        || kroll_reviewed(def) // Kroll block
        || kaeppel_reviewed(def) // Kaeppel block
        || hamon_reviewed(def) // Hamon block
        || toghraie_reviewed(def) // Toghraie block
        || harris_reviewed(def) // Harris block
        || etzkorn_reviewed(def) // Etzkorn block
        || lebeau_reviewed(def) // LeBeau block
        || tharp_reviewed(def) // Tharp block
        || person_reviewed(def) // Person block
        || kaufman_reviewed(def) // Kaufman block
        || appel_reviewed(def) // Appel block
        || bernstein_g2_reviewed(def)
        || jb_g3_reviewed(def)
        || lwilliams_reviewed(def) // Larry Williams block
}
// ---- Bernstein g2: 30 Days to Market Mastery (jb30d), The Ultimate Day Trader (jbudt), The Compleat Day Trader II (jbcdt2).
// Reviewed 22 Oct 2026 (authors/bernstein/agents/<work>, independent ledgers in authors/bernstein/g2-admission).
pub fn bernstein_g2_reviewed(def:&StrategyDef)->bool {
    use crate::books::{jb30d,jbudt,jbcdt2};
    (def.book==jb30d::BOOK && jb30d::reviewed_daily(def.id)) || (def.book==jbudt::BOOK && jbudt::reviewed_daily(def.id)) || (def.book==jbcdt2::BOOK && jbcdt2::reviewed_daily(def.id))
}
fn bernstein_g2_spx_points(id:&str)->bool {crate::books::jb30d::SPX_POINTS.contains(&id) || crate::books::jbcdt2::SPX_POINTS.contains(&id)}
macro_rules! BERNSTEIN_G2_ANY_LABEL {()=>{"Jake Bernstein tested these rules on S&P 500 futures (and showed some on stocks). Runs on actual indices, ETFs and stocks, US and India, are a declared application, not the author's test. The rules use no dollar or point amounts (stop-entry ticks are the instrument's real tick: 0.05 on NSE, 0.01 in the US). Cash-index shorts are hypothetical. Daily-bar conventions: stops fill at the open when the market gaps through them, otherwise at the stop price; where the order of prices inside a day matters the bar is read open, nearer extreme, other extreme, close. NIFTY opens before late 2010 lie close to the previous close (official data, kept). No futures dollars or book results are reproduced."}}
macro_rules! BERNSTEIN_G2_SPX_LABEL {()=>{"Jake Bernstein's stop (and entry offsets) are dollar amounts on one S&P 500 futures contract, which the vault converts to S&P points (30 Days: $250 a point, p.16/p.44; Compleat Day Trader II: $500 a point). Those points mean the same thing only on the S&P 500 itself, so this rule runs on actual SPX only, as a declared application of the futures rule to the cash index; other indices, ETFs and stocks are held. SPX opens are real only from 2014, so earlier start dates are refused. No futures dollars or book results are reproduced."}}
/// `stock_us`: None = not an individual stock; Some(true) = a US stock; Some(false) = Indian or unknown market.
fn bernstein_g2_symbol_issue(def:&StrategyDef,ticker:&str,stock_us:Option<bool>)->Option<Issue> {
    if !bernstein_g2_reviewed(def) {return None;}
    if bernstein_g2_spx_points(def.id) && ticker!="SPX" {
        return Some(Issue{status:"needs_data",note:format!("{} {ticker} is outside that scope; no result is published.",BERNSTEIN_G2_SPX_LABEL!())});
    }
    let us=stock_us.unwrap_or(!ticker.starts_with("NIFTY"));
    if crate::books::jbcdt2::US_ONLY.contains(&def.id) && !us {
        return Some(Issue{status:"needs_data",note:format!("The pre-holiday day trade (Compleat Day Trader II pp.68-69) buys the day before a US market holiday (Merrill's list: Good Friday, Memorial Day, July 4, Labor Day, Thanksgiving, Christmas, New Year's). Indian markets keep a different holiday calendar, so the rule runs on US indices, ETFs and stocks only. {ticker} is outside that scope; no result is published.")});
    }
    None
}
// ---- end Bernstein g2
// ---- Bernstein (bernstein-g3): Stock Market Strategies That Work, Strategies for the Electronic Futures Trader,
// Hot Stock Market Strategies. Admission only; reviewed in authors/bernstein/bernstein-g3-REPORT.md. ----
pub fn jb_g3_reviewed(def:&StrategyDef)->bool {
    (def.book==crate::books::jbsmsw::BOOK && crate::books::jbsmsw::reviewed_daily(def.id))
        || (def.book==crate::books::jbhsms::BOOK && crate::books::jbhsms::reviewed_daily(def.id))
        || (def.book==crate::books::jbeft::BOOK && crate::books::jbeft::reviewed_daily(def.id))
        || (def.book==crate::books::jbtywr::BOOK && crate::books::jbtywr::reviewed_daily(def.id))
        || (def.book==crate::books::jbtifm::BOOK && crate::books::jbtifm::reviewed_daily(def.id))
        || (def.book==crate::books::jbhpp::BOOK && crate::books::jbhpp::reviewed_daily(def.id))
}
fn jb_g3_gap(def:&StrategyDef)->bool {def.book==crate::books::jbhsms::BOOK && crate::books::jbhsms::gap_rule(def.id)}
macro_rules! JBSMSW_LABEL {()=>{"The book applies these methods to individual stocks; index and ETF runs (US and India) are a declared application, not the authors' test."}}
macro_rules! JBHSMS_KD_LABEL {()=>{"Calendar trade printed for one market (S&P futures, the Dow or one US stock); the same dates on any other index, ETF or stock (US and India) are a declared application, not the author's test."}}
const JBSMSW_BASIS:&str="Stock Market Strategies That Work (Jake & Elliott Bernstein, 2002): MOM/MA 28 (ch.10, pp.129-133) and the MAC pullback (ch.8, pp.95-112), reviewed against the book with independent chronological ledgers (authors/bernstein, bernstein-g3). Actual stocks are the book's instrument; actual index/ETF runs are a declared application. Close fills for MOM/MA; the MAC buy (sell) limit at yesterday's channel line fills at the open when price gaps through it; the 10% closing stop is the labelled alternative (off by default). The authors' discretionary overrides in the narrated trades are not modelled. No book performance is reproduced (none is printed).";
const JBHSMS_KD_BASIS:&str="Hot Stock Market Strategies (Bernstein, 2005), ch.7 key-date seasonal trades (pp.104-134): buy the close of the first trading day on or after the entry date, sell the close of the first trading day on or after the exit date, close-only stop at the printed % below the entry close. Reviewed with independent chronological ledgers (authors/bernstein, bernstein-g3). The book's records are March/June/September S&P futures, the Dow Jones Industrial Average and eight US stocks; the S&P trades run on the cash S&P 500 (SPX) or SPY, the Dow trades on DJI, the stock trades on the named stock (old Alcoa = HWM in the vault's data). Every other index, ETF or stock (US and India, same calendar dates) is a declared application. Two S&P dates (Feb 26, Mar 1) win two of 22 trades fewer on SPX than on the book's futures (small trades flip sign on the futures basis); they are admitted with that difference disclosed.";
const JBHSMS_GAP_BASIS:&str="Hot Stock Market Strategies (Bernstein, 2005), ch.6 gap trade (pp.78-95) with the printed IBM (Tables 6.1, 6.2) and EBAY (Table 6.3) inputs, reviewed with independent chronological ledgers (authors/bernstein, bernstein-g3). The gap sizes and penetrations are US cents per share, so this runs on actual US stocks only; on rupee-priced stocks, indices or ETFs the cents have no meaning and no result is published. Vault US stock prices are split/dividend-adjusted (not independently verified), so the cent amounts are a slightly different fraction of early prices than in the book's 2000-2004 test. Daily-bar convention for the entry-day stop (bar path) is declared [Not from the book].";
const JBTYWR_BASIS:&str="Bernstein seminar material: the Dow Jones key-date seasonals of Trade Your Way to Riches Now! (MBH, 1996) Figs 4-3 and 4-4 (enter and exit on the close of the printed dates, 5% closing stop) and of High Probability Patterns and Rule Based Trading (2010) slide p.101 (Oct 26 to Jan 10, 9% stop), and the crude-oil outside-day pattern of Figs 1-7/1-8 (buy the close of day 3 after an outside day when it closes above the outside day's high; sell on the close of day 60 or on a 6% closing stop). Reviewed with independent chronological ledgers (authors/bernstein, bernstein-g3). The Dow trades are the DJIA's own record (compared on DJI); every other index, ETF or stock (US and India) is a declared application. The outside-day pattern was tested on crude oil futures with overlapping positions; here one position at a time on any index, ETF or stock is a declared application, not the author's test.";
const JBTIFM_BASIS:&str="Trading the International Futures Markets (Bernstein, 2000), Appendix III TradeStation code: the 2000ST range breakout (p.298-299; stop and reverse at the close plus/minus 0.62 x the 3-bar average true range in the direction of 28-bar momentum versus its value 10 bars earlier; tested on daily FTSE-100 futures, Fig 10.2) and the 9/18 moving-average crossover (p.307; stop and reverse on the close; shown on a monthly CAC-40 chart, run here on daily bars). Reviewed with independent chronological ledgers (authors/bernstein, bernstein-g3). Index, ETF and stock runs (US and India) are a declared application, not the author's test; cash-index shorts are hypothetical exposure. The 2000ST stop orders fill at the open when the market gaps through them, so SPX/NDX/DJI windows start where those opens are real.";
const JBEFT_BASIS:&str="Strategies for the Electronic Futures Trader (Bernstein, 2000), ch.8 S&P day-of-week system (pp.126-132): buy (sell) stop a few ticks beyond the setup day's high (low), exit on the stop or the first profitable opening. The offsets and stops are S&P 500 index points of the futures contract, so the rule runs only on the S&P 500 cash index (SPX) as a declared application of the futures rule, and only from 2014-01-01, where the vault's SPX opens are real (earlier opens repeat the previous close). SPY (one tenth of the index), other indices and stocks would need rescaled amounts that the book does not give; no result is published there. The book's 1997-99 and 1982-97 records cannot be re-run on real opens and are not reproduced.";
fn jb_g3_symbol_issue(def:&StrategyDef,ticker:&str)->Option<Issue> {
    if def.book==crate::books::jbeft::BOOK && crate::books::jbeft::reviewed_daily(def.id) && ticker!="SPX" {
        return Some(Issue{status:"needs_data",note:format!("The S&P day-of-week system's entry offsets and stop are S&P 500 index points (futures contract), so it runs only on the S&P 500 index (SPX, from 2014 where opens are real). On {ticker} those point amounts would not mean what the book meant; no result is published.")});
    }
    None
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
    if open_all(def) {return open_all_symbol_issue(def,ticker,None);} // open-all block (Indian stocks are refused by check_native)
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
    Some(Issue{status:"needs_data",note:format!("Investment Secrets of a Hedge Fund Manager and the PTJ Options & Volatility Trading volume traded S&P 500 futures. This review admits CHADTP on NYSE breadth for SPX and SPY (the NSE-breadth CHADTP is held, thresholds not calibrated for India), NDX-SPX (NASDAQ 100 vs S&P 500 signal) for SPX, SPY and DJI only, CVR I buys / sells for SPX and SPY with the CBOE VIX (CVR I reads the VIX open; India VIX opens repeat the previous close), and the HV Thrust for SPX/NDX/DJI/SPY/NIFTY 50/NIFTYBANK. {ticker} is outside that reviewed scope; no result is published.")})
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
            assert_eq!(policy(&d).allowed_types,&["index","etf","individual_stock"]);assert!(check_native(&d,"AAPL",&NativeEvidence::us_test_fixture()).is_none());
            assert!(metadata_json(&d).contains("source_convention"));
            for symbol in ["NIFTY 50","SPX","SPY","QQQ"] {assert!(check(&d,symbol,false).is_none());}
            let mut other=d;other.book="kftsm";assert!(!reviewed_cash_rule(&other));
        }
        let d=crate::books::kfsc::catalogue().into_iter().find(|d|d.id=="kfsc_stoch").unwrap();assert!(!reviewed_cash_rule(&d));assert_eq!(policy(&d).requirement,"source_review_required");
    }
    #[test]
    fn exact_short_course_rules_require_actual_stocks_and_do_not_widen_other_sources() {
        for &id in crate::books::kfsc::REVIEWED_STOCK_IDS {
            let d=crate::books::kfsc::catalogue().into_iter().find(|d|d.id==id).unwrap();
            assert!(reviewed_cash_rule(&d));let p=policy(&d);assert_eq!(p.allowed_types,&["index","etf","individual_stock"]);
            assert!(p.source_basis.contains("source convention"));assert!(p.source_basis.contains("idealized"));
            assert!(check_native(&d,"AAPL",&NativeEvidence::us_test_fixture()).is_none());
            for symbol in ["NIFTY 50","SPX","SPY","QQQ"] {assert!(check(&d,symbol,false).is_none());}
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
    // Never reopens a rule that base_policy holds for source review.
    if open_instrument_label(def).is_some() && p.requirement!="source_review_required" {
        p.requirement="index_etf_or_stock";p.allowed_types=&["index","etf","individual_stock"];
        p.preferred_basket=Some("nifty50");p.market_adaptation=true;
    }
    p.native_currency_constraint=match (def.book,def.id) {
        ("afactor","afactor_baseline"|"afactor_spx_down")=>Some("Alvarez Factor workbook p.13 filters the previous stock close above USD 10. The runner compares a literal price parameter with the INR close, which is not the same economic filter."),
        ("jc5dm","jc5dm_long")=>Some("Five Day Momentum p.10 requires the US stock price above USD 50 and pp.11-12 use a 1/16 USD stock tick. The runner's literal values do not establish an INR threshold or the actual NSE stock tick."),
        ("jc5dm","jc5dm_short")=>Some("Five Day Momentum p.10 requires the US stock price above USD 40 and pp.11-12 use a 1/16 USD stock tick. The runner's literal values do not establish an INR threshold or the actual NSE stock tick."),
        ("r3r4","r3r4_lowprice"|"r3r4_lowprice_scale")=>Some(R3R4_LOWPRICE_HELD),
        ("ats","ats_cbp")=>Some("Advanced Trading Strategies Chapter 11 filters US momentum stocks above USD 20. The executable min_price parameter is compared directly with INR stock prices; a local currency specification is missing."),
        ("ats","ats_15min_adx_breakout_equities")=>Some("Advanced Trading Strategies p.164 requires a US equity price above USD 40 and stock execution ticks. Comparing a literal min_price with INR prices or substituting a generic tick does not preserve this source rule."),
        _ if jb_g3_gap(def)=>Some("Hot Stock Market Strategies Tables 6.1-6.3 give the gap sizes and penetrations in US cents per share (IBM, EBAY). Cents cannot become rupees; no INR version is given."),
        _=>None,
    };
    if p.native_currency_constraint.is_some(){p.source_price_unit=Some("USD");}
    p
}
/// User decision 2026-10-21: stock rules whose only stock hold was the book's USD price filter run on US stocks
/// (the filter is applied in USD as printed); Indian stocks stay held.
pub fn usd_price_rule_on_us_stock(def:&StrategyDef,market:Market)->bool {
    market==Market::UnitedStates && (matches!((def.book,def.id),("afactor","afactor_baseline"|"afactor_spx_down")|("r3r4","r3r4_lowprice"|"r3r4_lowprice_scale")) || jb_g3_gap(def) || cbp_us(def))
}
/// The R3/R4 Low Priced rules ARE the USD 1-15 band: they run on US stocks only (refusal text for every other instrument).
pub const R3R4_LOWPRICE_HELD:&str="R3/R4 Low Priced Rules are defined by the stock closing above USD 1 and below USD 15: the dollar price band is the strategy itself, so it runs only on actual US stocks. It stays held on Indian stocks (an INR 1-15 band would be a different strategy, not the book's) and on indices/ETFs. No results are published.";
/// User decision 2026-10-21: afactor_baseline runs on Indian stocks with the USD price (> 10) filter omitted (labelled);
/// the share-volume filter (> 100,000 shares) is kept as printed.
pub fn inr_price_filter_omitted(def:&StrategyDef,market:Market)->bool {
    market==Market::India && def.book=="afactor" && crate::books::afactor::stock_rule(def.id)
}
// ---------------------------------------------------------------- Larry Williams (lwilliams author agent) 2026-10-23
// Admission only for the ids in each module's REVIEWED list (authors/lwilliams/admission/SOURCE-CONTRACT.md); kept in one block to merge.
pub fn lwilliams_reviewed(def:&StrategyDef)->bool { crate::books::lw_admit::reviewed(def.book,def.id) }
macro_rules! LW_SPX_LABEL {()=>{"Larry Williams' stop, target or gap size is a dollar (or point) amount on one S&P 500 futures contract (the book's own reports value it at $250 or $500 an index point, as printed). The vault keeps the same number of S&P index points, which mean the same thing only on the S&P 500 itself, so this rule runs on actual SPX only, as a declared application of the futures rule to the cash index; other indices, ETFs and stocks are held. SPX opens are real only from 2014, so earlier start dates are refused. Daily bars: stops fill at their price, or at the open when the market gaps through them; inside a day the bar is read open, nearer extreme, other extreme, close. No futures dollars or book results are reproduced."}}
macro_rules! LW_NDX_LABEL {()=>{"Larry Williams' stop is a dollar amount on one Nasdaq-100 futures contract ($100 an index point in 1996). Those points mean the same thing only on the Nasdaq-100 itself, so this rule runs on actual NDX only, as a declared application of the futures rule to the cash index; other instruments are held. NDX opens are real only from 2001, so earlier start dates are refused. Daily bars: stops fill at their price, or at the open on a gap; inside a day the bar is read open, nearer extreme, other extreme, close. No futures dollars or newsletter results are reproduced."}}
macro_rules! LW_US_LABEL {()=>{"Larry Williams timed this trade around a US market holiday on S&P 500 futures. It uses no dollar or point amounts, but Indian markets keep a different holiday calendar, so it runs on US indices, ETFs and stocks only, as a declared application; Indian instruments are held. US index opens are real only from 2014 (SPX), 2001 (NDX) and 2016 (DJI), so earlier starts are refused there. No futures dollars or newsletter results are reproduced."}}
macro_rules! LW_ANY_LABEL {()=>{"Larry Williams tested this rule on futures (mostly S&P 500 futures). Runs on actual indices, ETFs and stocks, US and India, are a declared application, not the author's test. The rule uses no dollar or point amounts. Daily bars: stops fill at their price, or at the open when the market gaps through them; inside a day the bar is read open, nearer extreme, other extreme, close. US index opens are real only from 2014 (SPX), 2001 (NDX) and 2016 (DJI), so rules that read the open refuse earlier starts there; NIFTY opens before late 2010 lie close to the previous close (official data, kept). Cash-index and stock shorts are hypothetical. No futures dollars or book results are reproduced."}}
fn lwilliams_label_of(id:&str)->&'static str {
    match crate::books::lw_admit::only_ticker(id) {Some("NDX")=>LW_NDX_LABEL!(),Some(_)=>LW_SPX_LABEL!(),None if crate::books::lw_admit::us_only(id)=>LW_US_LABEL!(),None=>LW_ANY_LABEL!()}
}
fn lwilliams_policy(def:&StrategyDef)->Option<Policy> {
    if !lwilliams_reviewed(def) {return None;}
    let label=lwilliams_label_of(def.id);
    let mut p=if crate::books::lw_admit::only_ticker(def.id).is_some() {index(label)} else {requirement("index_etf_or_stock",&["index","etf","individual_stock"],label)};
    p.market_adaptation=true; Some(p)
}
/// `stock_us`: None = not an individual stock; Some(true) = a US stock; Some(false) = an Indian or unknown stock.
fn lwilliams_symbol_issue(def:&StrategyDef,ticker:&str,stock_us:Option<bool>)->Option<Issue> {
    if !lwilliams_reviewed(def) {return None;}
    if let Some(t)=crate::books::lw_admit::only_ticker(def.id).filter(|t|*t!=ticker) {
        let _=t; return Some(Issue{status:"needs_data",note:format!("{} {ticker} is outside that scope; no result is published.",lwilliams_label_of(def.id))});
    }
    let us=stock_us.unwrap_or(!(ticker.starts_with("NIFTY")));
    if crate::books::lw_admit::us_only(def.id) && !us {
        return Some(Issue{status:"needs_data",note:format!("{} {ticker} is outside that scope; no result is published.",LW_US_LABEL!())});
    }
    None
}
fn lwilliams_label(def:&StrategyDef)->Option<&'static str> {
    lwilliams_reviewed(def).then(||lwilliams_label_of(def.id))
}
// ---------------------------------------------------------------- end Larry Williams
// ---------------------------------------------------------------- Schwager (jscgf, jsgsta, jsats, sofmt, nmwbk) 2026-10-23
// Admission only for the listed ids (authors/schwager/admission/SOURCE-CONTRACT.md); kept in one block to merge.
pub fn schwager_reviewed(def:&StrategyDef)->bool {
    use crate::books::{jscgf,jsgsta,jsats,sofmt,nmwbk};
    match def.book {"jscgf"=>jscgf::reviewed(def.id),"jsgsta"=>jsgsta::reviewed(def.id),"jsats"=>jsats::reviewed(def.id),"sofmt"=>sofmt::reviewed(def.id),"nmwbk"=>nmwbk::reviewed(def.id),_=>false}
}
fn schwager_label(def:&StrategyDef)->Option<&'static str> {
    if !schwager_reviewed(def) {return None;}
    Some(match def.book {
        "jscgf"=>"A Complete Guide to the Futures Markets shows these systems on futures charts (copper, crude oil, soybeans, silver, coffee, gold and others); index, ETF and stock runs (US and India) are a declared application, not the author's test.",
        "jsgsta"=>"Getting Started in Technical Analysis shows these systems on futures and stock charts; index, ETF and stock runs (US and India) are a declared application where the book used futures.",
        "jsats"=>"The automated-trading-systems course tests its EasyLanguage systems on futures and US stocks; index, ETF and Indian stock runs are a declared application, not the course's test.",
        "sofmt"=>"The BARRA/MLM index applies this rule to 25 US futures markets with T-bill interest; single index, ETF and stock runs (US and India) are a declared application without interest.",
        _=>"Gil Blake applied the one-penny rule to a municipal bond fund's NAV; index, ETF and stock runs (US and India) are a declared application, not his trading.",
    })
}
fn schwager_policy(def:&StrategyDef)->Option<Policy> {
    if !schwager_reviewed(def) {return None;}
    let basis=match def.book {
        "jscgf"=>concat!("Schwager, A Complete Guide to the Futures Markets (2nd ed. 2017): Ch.17 original systems (p.261-277) with the Ch.9 run-day and wide-ranging-day definitions, the Ch.16 moving-average, crossover, breakout (with penetration, time-delay and thrust-day confirmation), MA-direction filter and countertrend fade systems (p.237-256), the Ch.6 moving-average turn (p.79-80), the Ch.19 six-day MA example (p.290) and the 1st edition's weekly 4-week breakout (1984, p.410), reviewed in authors/schwager/admission/SOURCE-CONTRACT.md with independent chronological ledgers. ","Close fills as the book's testing chapter assumes; next-open fills are a labelled option. Futures ticks become a labelled percentage of price. Cash-index shorts are hypothetical exposure. No futures dollars or book results are claimed."),
        "jsgsta"=>"Schwager, Getting Started in Technical Analysis (1999): Ch.14 moving-average, crossover, breakout with confirmations, MA-direction filter and countertrend fade (p.229-249), Ch.3 moving-average turn (p.47-49), Ch.6 MACD crossover (p.114) and the Ch.15 six-day MA example (p.254), reviewed in authors/schwager/admission/SOURCE-CONTRACT.md with independent chronological ledgers. Close fills; next-open fills are a labelled option. Cash-index shorts are hypothetical exposure. No book results are claimed.",
        "jsats"=>"Jack Schwager's Guide to Winning with Automated Trading Systems (course manual and lesson manuals 2-22): the EasyLanguage breakout, crossover, momentum, gap, consecutive-close, wide-ranging-day, ATR-band, confirmation and time-exit systems, run as printed with next-open market orders, reviewed in authors/schwager/admission/SOURCE-CONTRACT.md with independent chronological ledgers. Rules read the open: SPX/NDX/DJI start where their opens are real; NIFTY opens before 2011 are close to the prior close and are disclosed. Mirrored sell cases of printed buy cases are labelled. Cash-index shorts are hypothetical exposure. No futures dollars or course results are claimed.",
        "sofmt"=>"Schwager on Futures: Managed Trading (1996), BARRA/MLM index rule (p.82-84): monthly reading on the next-to-last trading day against the 12-month average, always in, reviewed in authors/schwager/admission/SOURCE-CONTRACT.md with independent chronological ledgers. Average type and ties are declared readings. Cash-index shorts are hypothetical exposure; no index returns or T-bill interest are claimed.",
        _=>"Schwager, The New Market Wizards (1992), Gil Blake's one-penny fund-switching rule (p.91-93): long after an up close, cash after a down close, reviewed in authors/schwager/admission/SOURCE-CONTRACT.md with independent chronological ledgers. No fund NAVs or reported returns are claimed.",
    };
    let mut p=requirement("index_etf_or_stock",&["index","etf","individual_stock"],basis);
    p.preferred_basket=Some("nifty50");p.market_adaptation=true;Some(p)
}
// ---------------------------------------------------------------- Elder (eltfl, elcim) 2026-10-23
// Admission only for the ids in each module's REVIEWED list (authors/elder/admission/SOURCE-CONTRACT.md); kept in one block to merge.
pub fn elder_reviewed(def:&StrategyDef)->bool {
    use crate::books::{eltfl,elcim};
    match def.book {"eltfl"=>eltfl::REVIEWED.contains(&def.id),"elcim"=>elcim::REVIEWED.contains(&def.id),_=>false}
}
const ELDER_LABEL:&str="Alexander Elder shows these rules on futures and stock charts (gold, the Nikkei, US stocks); index, ETF and stock runs (US and India) are a declared application, not the author's test. Weekly bars are calendar weeks built from the daily bars. One tick is the instrument's real tick (0.01 in the US, 0.05 on NSE). Force Index needs volume: windows without volume show a data message. Cash-index shorts are hypothetical. Elder prints no test results.";
fn elder_policy(def:&StrategyDef)->Option<Policy> {
    if !elder_reviewed(def) {return None;}
    let basis=match def.book {
        "eltfl"=>"Elder, Trading for a Living (1993): Force Index 2-day EMA with the 13-day EMA trend, trailing buy/sell stop and two-bar protective stop (p.227-232), and Triple Screen with weekly MACD-Histogram and daily Elder-ray (p.235-242), reviewed in authors/elder/admission/SOURCE-CONTRACT.md with independent chronological ledgers. Stop entries fill at the open when the market gaps through them; indicator exits at the next open. Break-even and 50%-of-profit stop moves have no exact trigger in the book and are not modelled.",
        _=>"Elder, Come Into My Trading Room (2002): Triple Screen update with the 26-week EMA, 2-day Force Index, buy (sell) order at the previous day's high (low) and the SafeZone stop (p.128-138, 173-178), reviewed in authors/elder/admission/SOURCE-CONTRACT.md with independent chronological ledgers. The 2%/6% account rules and the break-even move are not modelled (one-unit ledger).",
    };
    let mut p=requirement("index_etf_or_stock",&["index","etf","individual_stock"],basis);
    p.preferred_basket=Some("nifty50");p.market_adaptation=true;Some(p)
}
fn elder_label(def:&StrategyDef)->Option<&'static str> {elder_reviewed(def).then_some(ELDER_LABEL)}
// ---------------------------------------------------------------- end Elder
// ---------------------------------------------------------------- end Schwager
// ---------------------------------------------------------------- Pring (mpts, mpmm, mpcse, mpsel) 2026-10-23
// Admission only for the listed ids (authors/pring/admission/SOURCE-CONTRACT.md); kept in one block to merge.
pub fn pring_reviewed(def:&StrategyDef)->bool {
    use crate::books::{mpts,mpmm,mpcse,mpsel,mpasi,mptae};
    match def.book {"mpasi"=>mpasi::reviewed(def.id),"mptae"=>mptae::reviewed(def.id),"mpts"=>mpts::reviewed(def.id),"mpmm"=>mpmm::reviewed(def.id),"mpcse"=>mpcse::reviewed(def.id),"mpsel"=>mpsel::reviewed(def.id),_=>false}
}
fn pring_label(def:&StrategyDef)->Option<&'static str> {
    if !pring_reviewed(def) {return None;}
    Some(match def.book {
        "mpts"=>"Trading Systems Explained tests these systems on the S&P Composite, Nikkei, NASDAQ, Russell 2000, the British pound and single stocks and funds; index, ETF and stock runs (US and India) are a declared application, not the book's test.",
        "mpmm"=>"Market Momentum quotes these rules on the S&P Composite, NYSE and other indices (mostly Colby & Meyers' tests); index, ETF and stock runs (US and India) are a declared application. The Summation Index rules use the market's own advance/decline data (NYSE for US symbols, NSE for Indian symbols).",
        "mptae"=>"Technical Analysis Explained shows these rules on the DJIA (and a trade-weighted currency index); index, ETF and stock runs (US and India) are a declared application.",
        "mpasi"=>"The All-Season Investor applies this rule to the S&P Composite; other index, ETF and stock runs (US and India) are a declared application.",
        "mpcse"=>"Candlesticks Explained shows the three-line break on US stocks; index, ETF and stock runs (US and India) are a declared application.",
        _=>"The How to Select Stocks CD gives these MetaStock systems without a market; index, ETF and stock runs (US and India) are a declared application.",
    })
}
fn pring_policy(def:&StrategyDef)->Option<Policy> {
    if !pring_reviewed(def) {return None;}
    let basis=match def.book {
        "mpts"=>"Pring, Trading Systems Explained (2003): MA crossovers (Ch.11-12, 15), MA + CMO partial profits (Ch.13 neutral version), MA + price oscillator (Ch.14), Triple Indicator and its ROC parts (Ch.15), reviewed in authors/pring/admission/SOURCE-CONTRACT.md with independent chronological ledgers. Signal-close fills are a declared convention (MetaStock default; the book does not state fills). Cash-index shorts are hypothetical exposure. No commissions, cash interest or book results are claimed.",
        "mpmm"=>"Pring, Martin Pring on Market Momentum (1993): Colby & Meyers RSI, moving-average, 15/36-week and 39-week %K tests, Gartley's advanced average, the 26/40-week deviation systems, the monthly and weekly KST crossovers, the 12-month ROC average turns and the McClellan Summation 35-day crossover, reviewed in authors/pring/admission/SOURCE-CONTRACT.md with independent chronological ledgers. Long or cash by default where the book says only buy and sell. Signal-close fills. No book results are claimed.",
        "mptae"=>"Pring, Technical Analysis Explained (2nd ed. 1985): 40-week MA crossover (p.89-92), 10/30-week averages with the slope condition (p.95-97), Russell 13/50-week signals (p.158), 9-month MA with 12-month ROC confirmation (p.354) and Coppock's buy indicator (p.116-117) with the user's standard exits (the book gives none), reviewed in authors/pring/admission/SOURCE-CONTRACT.md with independent chronological ledgers. Long or cash by default; signal-close fills are a declared convention.",
        "mpasi"=>"Pring, The All-Season Investor (1992): the S&P Composite 12-month moving-average crossover, at or above = bullish (p.101-105, 185-190), long or cash, reviewed in authors/pring/admission/SOURCE-CONTRACT.md with independent chronological ledgers. Month-end close fills are a declared convention. The stage allocation model needs bond, commodity and yield data and is not run.",
        "mpcse"=>"Pring, Candlesticks Explained (2002), Ch.8 three-line break (p.129-142): reversal at the close of the turnaround day, reviewed in authors/pring/admission/SOURCE-CONTRACT.md with independent chronological ledgers. The first line's start is a declared convention. Cash-index shorts are hypothetical exposure.",
        _=>"Pring, How to Select Stocks Using Technical Analysis (pring.com CD): MetaStock System Tester formulas (long-term KST momentum, Wellinvested NASDAQ, MA + CMO(7), advanced MA crossover) run as printed, reviewed in authors/pring/admission/SOURCE-CONTRACT.md with independent chronological ledgers. Signal-close fills are the MetaStock default. Cash-index shorts are hypothetical exposure.",
    };
    let mut p=requirement("index_etf_or_stock",&["index","etf","individual_stock"],basis);
    p.preferred_basket=Some("nifty50");p.market_adaptation=true;Some(p)
}
// ---------------------------------------------------------------- end Pring
// ---------------------------------------------------------------- Gregoriou (ggth, ggeai, gghf) 2026-10-24
// Admission only for books::gregoriou::REVIEWED (authors/gregoriou/admission/SOURCE-CONTRACT.md); kept in one block to merge.
pub fn gregoriou_reviewed(def:&StrategyDef)->bool {matches!(def.book,"ggth"|"ggeai"|"gghf"|"ggfohf") && crate::books::gregoriou::reviewed(def.id)}
fn gregoriou_label(def:&StrategyDef)->Option<&'static str> {
    if !gregoriou_reviewed(def) {return None;}
    Some(match def.book {
        "ggth"=>"The Handbook of Trading tests these rules on the FTSE/ASE 20 (Ch.7) and on the S&P 500, NASDAQ and DJIA (Ch.11); other index, ETF and stock runs (US and India) are a declared application, not the authors' test.",
        "ggeai"=>"The Encyclopedia of Alternative Investments gives this crossover as an example of a futures CTA rule; index, ETF and stock runs (US and India) are a declared application.",
        "ggfohf"=>"Funds of Hedge Funds builds this trend-following factor from 38 futures markets; index, ETF and stock runs (US and India) are a declared application.",
        _=>"Hedge Funds (Gregoriou et al., eds., 2005) describes these as rules of simulated futures indexes and benchmarks (Calyon, Spurgin-Schneeweis-Georgiev); index, ETF and stock runs (US and India) are a declared application.",
    })
}
fn gregoriou_policy(def:&StrategyDef)->Option<Policy> {
    if !gregoriou_reviewed(def) {return None;}
    let basis=match def.book {
        "ggth"=>"Gregoriou (ed.), The Handbook of Trading (2010): Kenourgios & Papathanasiou's (1,n) moving-average rule, long above and short below (Ch.7, p.100-105), and Lento's dual-moving-average crossover, traditional and exponentially smoothed, long or cash (Ch.11, p.157-160), reviewed in authors/gregoriou/admission/SOURCE-CONTRACT.md with independent chronological ledgers. Signal-close fills are a declared convention. Commissions, spreads and Lento's savings-account interest are not modelled; no book results are claimed.",
        "ggeai"=>"Gregoriou (ed.), Encyclopedia of Alternative Investments (2009), Systematic CTA (p.468): 'perhaps 5 day' over 'perhaps 30 day' moving-average crossover, long and short, reviewed in authors/gregoriou/admission/SOURCE-CONTRACT.md with independent chronological ledgers. Simple averages and signal-close fills are declared readings. Cash-index shorts are hypothetical exposure.",
        "ggfohf"=>"Gregoriou (ed.), Funds of Hedge Funds (2006), Ch.4 Bacmann, Jeanneret & Scholz p.61: closing price vs its m-day moving average (224, 117, 61, 32), long above and short below, reviewed in authors/gregoriou/admission/SOURCE-CONTRACT.md with independent chronological ledgers. Signal-close fills; each average runs on its own (the within-market combination is not stated). Cash-index shorts are hypothetical exposure.",
        _=>"Gregoriou, Hübner, Papageorgiou and Rouah (eds.), Hedge Funds (2005), Ch.10 p.230: the 80-day range breakout and the 80-day moving-average rule, and Ch.23 p.482: the 15-day momentum rule (long if above the price 15 days ago, otherwise short), always long or short, reviewed in authors/gregoriou/admission/SOURCE-CONTRACT.md with independent chronological ledgers. Closing-price breakouts, simple averages and signal-close fills are declared readings. Cash-index shorts are hypothetical exposure.",
    };
    let mut p=requirement("index_etf_or_stock",&["index","etf","individual_stock"],basis);
    p.preferred_basket=Some("nifty50");p.market_adaptation=true;Some(p)
}
// ---------------------------------------------------------------- end Gregoriou
// ---------------------------------------------------------------- Crisp (mccic) 2026-10-24
// Admission only for books::mccic::REVIEWED (authors/crisp/admission/SOURCE-CONTRACT.md); kept in one block to merge.
pub fn crisp_reviewed(def:&StrategyDef)->bool {def.book==crate::books::mccic::BOOK && crate::books::mccic::reviewed(def.id)}
const CRISP_LABEL:&str="Cashing in with Commodities trades this breakout on commodity futures (crude oil, bean oil, sugar, cocoa); index, ETF and stock runs (US and India) are a declared application, not the author's test.";
fn crisp_policy(def:&StrategyDef)->Option<Policy> {
    if !crisp_reviewed(def) {return None;}
    let mut p=requirement("index_etf_or_stock",&["index","etf","individual_stock"],"Mark Crisp, Cashing in with Commodities (pdf p.17-43): weekly two-week breakout, buy/sell stop at the two-week high/low, 2.5 x 30-day ATR initial stop, weekly-low trail, 50% / 75% profit protection at 2R / 5R, reviewed in authors/crisp/admission/SOURCE-CONTRACT.md with independent chronological ledgers. Which level, gap fills, one-tick trail offset and ignoring breakouts while in a trade are declared readings. One unit; no 4% sizing. Cash-index shorts are hypothetical exposure.");
    p.preferred_basket=Some("nifty50");p.market_adaptation=true;Some(p)
}
// ---------------------------------------------------------------- end Crisp
// ---------------------------------------------------------------- Fabozzi 2026-10-24
// Admission only for the module REVIEWED lists (authors/fabozzi/admission/SOURCE-CONTRACT.md); kept in one block to merge.
pub fn fabozzi_reviewed(def:&StrategyDef)->bool {matches!(def.book,"fzhci") && (crate::books::fzhci::reviewed(def.id))}
const FABOZZI_LABEL:&str="The Handbook of Commodity Investing describes these rules on commodity futures (gold, crude oil, grains, the GSCI); index, ETF and stock runs (US and India) are a declared application, not the authors' test.";
fn fabozzi_policy(def:&StrategyDef)->Option<Policy> {
    if !fabozzi_reviewed(def) {return None;}
    let mut p=requirement("index_etf_or_stock",&["index","etf","individual_stock"],"Fabozzi, Füss and Kaiser (eds.), The Handbook of Commodity Investing (2008): Lhabitant's price/15-day average cross (Ch.16 p.395-398), the Erb-Harvey 12-month momentum rule (Ch.8 p.221), Park and Irwin's dual moving-average (next-open), outside-channel and Alexander filter definitions (Ch.40 p.914-921), reviewed in authors/fabozzi/admission/SOURCE-CONTRACT.md with independent chronological ledgers. Close fills where the book is silent; defaults are book-printed study settings. No excess-return (T-bill) component or costs. Cash-index shorts are hypothetical exposure.");
    p.preferred_basket=Some("nifty50");p.market_adaptation=true;Some(p)
}
// ---------------------------------------------------------------- end Fabozzi
// ---------------------------------------------------------------- Fontanills 2026-10-24
// Admission only for the module REVIEWED lists (authors/fontanills/admission/SOURCE-CONTRACT.md); kept in one block to merge.
pub fn fontanills_reviewed(def:&StrategyDef)->bool {matches!(def.book,"fgitc"|"fgitsm"|"fgsmc"|"fgtofd") && (crate::books::fontanills::reviewed(def.id))}
const FONTANILLS_LABEL:&str="Fontanills and Gentile show these rules on SPY, the S&P 500, QQQQ, DIA, XLF and US stocks; other index, ETF and stock runs (US and India) are a declared application, not the authors' test.";
fn fontanills_policy(def:&StrategyDef)->Option<Policy> {
    if !fontanills_reviewed(def) {return None;}
    let mut p=requirement("index_etf_or_stock",&["index","etf","individual_stock"],"George A. Fontanills and Tom Gentile: The Index Trading Course (2006) Volex bands, ROC/ADX momentum, QQQQ volume and 10/20 EMA systems (p.162-188, 321-323); Interactive Technical Trading Seminar manual Reverse VolEx and volume-spike systems (Ch.10, 12); The Stock Market Course 50/200 crossover (p.257); Trading Options For Dummies 20/50 EMA crossover (p.100-101); reviewed in authors/fontanills/admission/SOURCE-CONTRACT.md with independent chronological ledgers. Band-day, fill, ADX length and EMA start are declared readings; one unit, no pyramiding. Cash-index shorts are hypothetical exposure.");
    p.preferred_basket=Some("nifty50");p.market_adaptation=true;Some(p)
}
// ---------------------------------------------------------------- end Fontanills
// ---------------------------------------------------------------- Thomsett 2026-10-24
// Admission only for the module REVIEWED lists (authors/thomsett/admission/SOURCE-CONTRACT.md); kept in one block to merge.
pub fn thomsett_reviewed(def:&StrategyDef)->bool {matches!(def.book,"mtpos") && (crate::books::mtpos::reviewed(def.id))}
const THOMSETT_LABEL:&str="Put Option Strategies for Smarter Trading shows this set-up on US stocks; index, ETF and Indian stock runs are a declared application, not the author's test.";
fn thomsett_policy(def:&StrategyDef)->Option<Policy> {
    if !thomsett_reviewed(def) {return None;}
    let mut p=requirement("index_etf_or_stock",&["index","etf","individual_stock"],"Michael C. Thomsett, Put Option Strategies for Smarter Trading (2009), Ch.4 (p.65-69, 73-74): reversal day after three or more trend days, buy at the bottom and sell at the opposite set-up, reviewed in authors/thomsett/admission/SOURCE-CONTRACT.md with independent chronological ledgers. Close fills, no confirmation filters and long-only default are declared readings; no stop (none in the book).");
    p.preferred_basket=Some("nifty50");p.market_adaptation=true;Some(p)
}
// ---------------------------------------------------------------- end Thomsett
// ---------------------------------------------------------------- Patel 2026-10-24
// Admission only for the module REVIEWED lists (authors/patel/admission/SOURCE-CONTRACT.md); kept in one block to merge.
pub fn patel_reviewed(def:&StrategyDef)->bool {matches!(def.book,"apitc") && (crate::books::apitc::reviewed(def.id))}
const PATEL_LABEL:&str="The Internet Trading Course gives this as an example for US and UK shares; index, ETF and stock runs (US and India) are a declared application, not the author's test.";
fn patel_policy(def:&StrategyDef)->Option<Policy> {
    if !patel_reviewed(def) {return None;}
    let mut p=requirement("index_etf_or_stock",&["index","etf","individual_stock"],"Alpesh B. Patel, The Internet Trading Course, Module 2 (p.23): long while the price is above the 50-day moving average, out otherwise, reviewed in authors/patel/admission/SOURCE-CONTRACT.md with independent chronological ledgers. Closing prices, a simple average and close fills are declared readings. No costs.");
    p.preferred_basket=Some("nifty50");p.market_adaptation=true;Some(p)
}
// ---------------------------------------------------------------- end Patel
// ---------------------------------------------------------------- DeMark 2026-10-24
// Admission only for the module REVIEWED lists (authors/demark/admission/SOURCE-CONTRACT.md); kept in one block to merge.
pub fn demark_reviewed(def:&StrategyDef)->bool {matches!(def.book,"tdwb"|"tdnsta"|"tdnmtt") && (crate::books::tdwb::reviewed(def.id) || crate::books::tdnsta::reviewed(def.id) || crate::books::tdnmtt::reviewed(def.id))}
const DEMARK_LABEL:&str="Tom DeMark showed TD Sequential, TD Combo and TD Lines on futures, stocks and indices without printed statistics; index, ETF and stock runs (US and India) are a declared application, not the author's test. The TD Line fade is a one-day trade (entry at the line, exit at that day's close). Cash-index shorts are hypothetical exposure.";
fn demark_policy(def:&StrategyDef)->Option<Policy> {
    if !demark_reviewed(def) {return None;}
    let mut p=requirement("index_etf_or_stock",&["index","etf","individual_stock"],"Tom DeMark: TD Sequential with his 1994 entry, exit and stop (The New Science of Technical Analysis Ch.7, p.140-178), TD Sequential and TD Combo versions 1 and 2 with his 1997 profit level and stop (New Market Timing Techniques Ch.2-3), and the disqualified TD Line fade (Trading Indicators for the 21st Century, workbook 5 p.1-14). Reviewed in authors/demark/admission/SOURCE-CONTRACT.md with independent chronological ledgers; first-listed defaults where the book gives options or ranges; no costs.");
    p.preferred_basket=Some("nifty50");p.market_adaptation=true;Some(p)
}
// ---------------------------------------------------------------- end DeMark
// ---------------------------------------------------------------- Kroll 2026-10-24
// Admission only for the module REVIEWED lists (authors/kroll/admission/SOURCE-CONTRACT.md); kept in one block to merge.
pub fn kroll_reviewed(def:&StrategyDef)->bool {matches!(def.book,"skdab") && (crate::books::skdab::reviewed(def.id))}
const KROLL_LABEL:&str="Stanley Kroll illustrated this strategy on the NIKKEI and futures; runs on indices, ETFs and stocks, US and India, are a declared application, not the author's test. Cash-index shorts are hypothetical exposure.";
fn kroll_policy(def:&StrategyDef)->Option<Policy> {
    if !kroll_reviewed(def) {return None;}
    let mut p=requirement("index_etf_or_stock",&["index","etf","individual_stock"],"Stanley Kroll, Dragons and Bulls (Addison-Wesley 1994), Ch.11 Long-term versus Short-term Trading (p.59-61): 50-day average trend filter, 10/20/50-day entries, 4/9/18-day stop-outs and re-entries. Reviewed in authors/kroll/admission/SOURCE-CONTRACT.md with independent chronological ledgers; fills at the close; no costs.");
    p.preferred_basket=Some("nifty50");p.market_adaptation=true;Some(p)
}
// ---------------------------------------------------------------- end Kroll
// ---------------------------------------------------------------- Kaeppel 2026-10-24
// Admission only for the module REVIEWED lists (authors/kaeppel/admission/SOURCE-CONTRACT.md); kept in one block to merge.
pub fn kaeppel_reviewed(def:&StrategyDef)->bool {matches!(def.book,"kpss") && (crate::books::kpss::reviewed(def.id))}
const KAEPPEL_LABEL:&str="Jay Kaeppel tested every rule on the Dow Jones Industrial Average (the Nasdaq rule on the Nasdaq Composite/NASDAQ 100); runs on other indices, ETFs and stocks are a declared application, not the author's test. Rules defined by US market holidays (day before a holiday, Santa Claus rally, the holiday-based models) run on US instruments only. 2:1 positions and shorts are hypothetical exposure; the book's 1% cash interest is not modelled.";
fn kaeppel_policy(def:&StrategyDef)->Option<Policy> {
    if !kaeppel_reviewed(def) {return None;}
    let mut p=requirement("index_etf_or_stock",&["index","etf","individual_stock"],"Jay Kaeppel, Seasonal Stock Market Trends (Wiley 2009): calendar, holiday, monthly-day, decennial, time-cycle, election-cycle, season and Known Trends Index rules (Ch.2-9, p.22-284), each a daily position earning close-to-close. Reviewed in authors/kaeppel/admission/SOURCE-CONTRACT.md with independent chronological ledgers; book rules transcribed in audits/kpss/book_rules.md.");
    p.preferred_basket=Some("nifty50");p.market_adaptation=true;Some(p)
}
// ---------------------------------------------------------------- end Kaeppel
// Kaeppel US-holiday rules: US instruments only (stock_us None = not a stock).
fn kaeppel_symbol_issue(def:&StrategyDef,ticker:&str,stock_us:Option<bool>)->Option<Issue> {
    if !kaeppel_reviewed(def) || !crate::books::kpss::US_ONLY.contains(&def.id) {return None;}
    let us=stock_us.unwrap_or(!ticker.starts_with("NIFTY"));
    if us {return None;}
    Some(Issue{status:"needs_data",note:format!("This rule's days are set by US stock-market holidays (the trading days before and after a US exchange closure, or the Thanksgiving-to-January Santa Claus window). Indian exchanges keep a different holiday calendar, so the rule runs on US indices, ETFs and stocks only. {ticker} is outside that scope; no result is published.")})
}
// ---------------------------------------------------------------- Hamon 2026-10-24
// Admission only for the module REVIEWED lists (authors/hamon/admission/SOURCE-CONTRACT.md); kept in one block to merge.
pub fn hamon_reviewed(def:&StrategyDef)->bool {matches!(def.book,"jhbct") && (crate::books::jhbct::reviewed(def.id))}
const HAMON_LABEL:&str="J. D. Hamon showed the weekly rule on futures charts (Sept 82 corn, July 82 coffee); runs on indices, ETFs and stocks, US and India, are a declared application, not the author's test. Cash-index shorts are hypothetical exposure.";
fn hamon_policy(def:&StrategyDef)->Option<Policy> {
    if !hamon_reviewed(def) {return None;}
    let mut p=requirement("index_etf_or_stock",&["index","etf","individual_stock"],"J. D. Hamon, Breakthroughs in Commodity Technical Analysis (Windsor Books), Ch.7 Improving Your Charting Ability (p.119-124): the weekly rule on the two previous calendar weeks, close only (stop and reverse), and the close entry with the four-close exit. Reviewed in authors/hamon/admission/SOURCE-CONTRACT.md with independent chronological ledgers. Fills at the close; no costs.");
    p.preferred_basket=Some("nifty50");p.market_adaptation=true;Some(p)
}
// ---------------------------------------------------------------- end Hamon
// ---------------------------------------------------------------- Toghraie 2026-10-24
// Admission only for the module REVIEWED lists (authors/toghraie/admission/SOURCE-CONTRACT.md); kept in one block to merge.
pub fn toghraie_reviewed(def:&StrategyDef)->bool {matches!(def.book,"rgtsec") && (crate::books::rgtsec::reviewed(def.id))}
const TOGHRAIE_LABEL:&str="Murray Ruggiero tested these TradeStation rules in Traders' Secrets on futures (coffee, Japanese yen, crude oil, currencies); runs on indices, ETFs and stocks, US and India, are a declared application, not the author's test. '1 point' is 0.01 in the instrument's price. Cash-index shorts are hypothetical exposure.";
fn toghraie_policy(def:&StrategyDef)->Option<Policy> {
    if !toghraie_reviewed(def) {return None;}
    let mut p=requirement("index_etf_or_stock",&["index","etf","individual_stock"],"Murray A. Ruggiero, Jr. & Adrienne Laris Toghraie, Traders' Secrets (On Target Press 1999): Ruggiero's analytical-model rules - volatility-adjusted and standard channel breakouts (p.118, p.319), the 14/18 channel breakout (p.205-206) and the 10/15/25 three moving average crossover (p.269-270). Reviewed in authors/toghraie/admission/SOURCE-CONTRACT.md with independent chronological ledgers. Stops act on the next bar; no costs.");
    p.preferred_basket=Some("nifty50");p.market_adaptation=true;Some(p)
}
// ---------------------------------------------------------------- end Toghraie
// ---------------------------------------------------------------- Harris 2026-10-24
// Admission only for the module REVIEWED lists (authors/harris/admission/SOURCE-CONTRACT.md); kept in one block to merge.
pub fn harris_reviewed(def:&StrategyDef)->bool {matches!(def.book,"mhpst"|"mhart") && (crate::books::mhpst::reviewed(def.id) || crate::books::mhart::reviewed(def.id))}
const HARRIS_LABEL:&str="Michael Harris tested these price patterns on QQQQ, SPY, the DAX and the FTSE (Profitability and Systematic Trading appendix) and on Intel and Dell (Active Trader, Sep 2001); runs on other indices, ETFs and stocks, US and India, are a declared application, not the author's test. Short patterns on a cash index are hypothetical exposure.";
fn harris_policy(def:&StrategyDef)->Option<Policy> {
    if !harris_reviewed(def) {return None;}
    let mut p=requirement("index_etf_or_stock",&["index","etf","individual_stock"],"Michael Harris: Profitability and Systematic Trading (Wiley 2008) appendix price patterns with percentage targets and stops (p.137-178, EasyLanguage listings) and the 3L-R and GAP-2H patterns (Active Trader, Sep 2001, Tables 2-3). Reviewed in authors/harris/admission/SOURCE-CONTRACT.md with independent chronological ledgers. Entry at the next open, exits at the printed limit and stop; same-bar target and stop take the stop first (book silent); no costs.");
    p.preferred_basket=Some("nifty50");p.market_adaptation=true;Some(p)
}
// ---------------------------------------------------------------- end Harris
// ---------------------------------------------------------------- Etzkorn 2026-10-24
// Admission only for the module REVIEWED lists (authors/etzkorn/admission/SOURCE-CONTRACT.md); kept in one block to merge.
pub fn etzkorn_reviewed(def:&StrategyDef)->bool {matches!(def.book,"metwo"|"mewwot") && (crate::books::metwo::reviewed(def.id))}
const ETZKORN_LABEL:&str="Mark Etzkorn ran these oscillator tests on nine futures markets (1984-1994) and printed the best lengths per market; the S&P 500 row is the default. Runs on indices, ETFs and stocks, US and India, are a declared application, not the author's test. Cash-index shorts are hypothetical exposure.";
fn etzkorn_policy(def:&StrategyDef)->Option<Policy> {
    if !etzkorn_reviewed(def) {return None;}
    let mut p=requirement("index_etf_or_stock",&["index","etf","individual_stock"],"Mark Etzkorn, Trading with Oscillators (Wiley 1997), Ch.5 Simple Tests (p.62-69): MACD signal-line crossover, RSI 50-line and ROC equilibrium crossings as stop-and-reverse systems on the close (Tables 5.2-5.3, S&P 500 lengths by default), and his spike-reversal day rule in What Works in Online Trading (2001, p.125-132: 3% spike, out after eight days). Reviewed in authors/etzkorn/admission/SOURCE-CONTRACT.md with independent chronological ledgers; no costs.");
    p.preferred_basket=Some("nifty50");p.market_adaptation=true;Some(p)
}
// ---------------------------------------------------------------- end Etzkorn
// ---------------------------------------------------------------- LeBeau 2026-10-24
// Admission only for the module REVIEWED lists (authors/lebeau/admission/SOURCE-CONTRACT.md); kept in one block to merge.
pub fn lebeau_reviewed(def:&StrategyDef)->bool {matches!(def.book,"lebcafm"|"lebtc"|"lebtcb"|"lebws") && (crate::books::lebcafm::reviewed(def.id) || crate::books::lebtc::reviewed(def.id) || crate::books::lebtcb::reviewed(def.id) || crate::books::lebws::reviewed(def.id))}
const LEBEAU_LABEL:&str="Charles LeBeau and his System Traders Club tested these rules on futures (Computer Analysis of the Futures Markets: D-mark, gold, soybeans, T-bonds and crude; the club bulletins and forum: interest-rate and other futures) or on US stocks (High Beta workbook, ABC handout); index, ETF and stock runs (US and India) are a declared application, not the author's test. Cash-index shorts are hypothetical exposure.";
fn lebeau_policy(def:&StrategyDef)->Option<Policy> {
    if !lebeau_reviewed(def) {return None;}
    let mut p=requirement("index_etf_or_stock",&["index","etf","individual_stock"],"Charles LeBeau (& David W. Lucas): Computer Analysis of the Futures Markets Ch.2 ADX + Parabolic, volatility breakout, Babcock DMI and Lambert CCI rules (p.39, 61-63, 113-114, 139-146) and the Ch.3 entry, exit and stop test protocols (p.170-188); System Traders Club High Beta Stock system, Ronco adaptive channel and volatility breakouts, TBond Attack, Johnson's Pretty Good Oscillator (Bulletin #4, Traderclub Forum) and the ABC ATR handout. Reviewed in authors/lebeau/admission/SOURCE-CONTRACT.md with independent chronological ledgers. Orders follow the book or printed code; no costs; the futures results are not reproduced.");
    p.preferred_basket=Some("nifty50");p.market_adaptation=true;Some(p)
}
// ---------------------------------------------------------------- end LeBeau
// ---------------------------------------------------------------- Tharp 2026-10-24
// Admission only for the module REVIEWED lists (authors/tharp/admission/SOURCE-CONTRACT.md); kept in one block to merge.
pub fn tharp_reviewed(def:&StrategyDef)->bool {matches!(def.book,"tharp") && (crate::books::tharp::reviewed(def.id))}
const THARP_LABEL:&str="Trade Your Way to Financial Freedom tested these systems on a basket of futures; index, ETF and stock runs (US and India) are a declared application, not the author's test. Cash-index shorts are hypothetical exposure.";
fn tharp_policy(def:&StrategyDef)->Option<Policy> {
    if !tharp_reviewed(def) {return None;}
    let mut p=requirement("index_etf_or_stock",&["index","etf","individual_stock"],"Van K. Tharp, Trade Your Way to Financial Freedom (1999): random entry with a trailing 3 x volatility stop and 1% risk (p.200-201) and the 55-day breakout with a 21-day trailing stop (p.286), reviewed in authors/tharp/admission/SOURCE-CONTRACT.md with independent chronological ledgers. The coin is a seeded generator (seed option) and fills follow the card's declared readings. No costs; the book's futures results are not reproduced.");
    p.preferred_basket=Some("nifty50");p.market_adaptation=true;Some(p)
}
// ---------------------------------------------------------------- end Tharp
// ---------------------------------------------------------------- Person 2026-10-24
// Admission only for the module REVIEWED lists (authors/person/admission/SOURCE-CONTRACT.md); kept in one block to merge.
pub fn person_reviewed(def:&StrategyDef)->bool {matches!(def.book,"jpcppt"|"hcta") && (crate::books::jperson::reviewed(def.id) || crate::books::hcta::reviewed(def.id))}
const PERSON_LABEL:&str="John L. Person shows the pivot rules on forex, futures and stock charts, and the Commodity Trader's Almanac trades S&P 500 futures; index, ETF and stock runs (US and India) on daily bars are a declared application, not the author's test. Cash-index shorts are hypothetical exposure.";
fn person_policy(def:&StrategyDef)->Option<Policy> {
    if !person_reviewed(def) {return None;}
    let mut p=requirement("index_etf_or_stock",&["index","etf","individual_stock"],"John L. Person, Candlestick and Pivot Point Trading Triggers (2007) p.161-174: close or pivot crossing the three-day pivot average, stop and reverse; Hirsch & Person, Commodity Trader's Almanac 2010 p.126-127: the four S&P 500 calendar trades (Nth trading day, fixed hold). Reviewed in authors/person/admission/SOURCE-CONTRACT.md with independent chronological ledgers. Close fills; no costs; the book's futures results are not reproduced.");
    p.preferred_basket=Some("nifty50");p.market_adaptation=true;Some(p)
}
// ---------------------------------------------------------------- end Person
// ---------------------------------------------------------------- Kaufman 2026-10-24
// Admission only for the module REVIEWED lists (authors/kaufman/admission/SOURCE-CONTRACT.md); kept in one block to merge.
pub fn kaufman_reviewed(def:&StrategyDef)->bool {matches!(def.book,"kfgei"|"kfbpm"|"kfst"|"kfcd"|"kftsm") && (crate::books::kfgei::reviewed(def.id) || crate::books::kfbpm::reviewed(def.id) || crate::books::kfst::reviewed(def.id) || crate::books::kfcd::reviewed(def.id) || crate::books::kftsm::reviewed(def.id))}
const KAUFMAN_LABEL:&str="Perry Kaufman tested these rules on futures, foreign indices or the S&P 500 (workshop, Smarter Trading, Global Equity Investing, the TSM CD programs); index, ETF and stock runs (US and India) are a declared application, not the author's test. Cash-index shorts are hypothetical exposure.";
fn kaufman_policy(def:&StrategyDef)->Option<Policy> {
    if !kaufman_reviewed(def) {return None;}
    let mut p=requirement("index_etf_or_stock",&["index","etf","individual_stock"],"Perry J. Kaufman: Global Equity Investing (with Vivanti) weekly systems A, B, E and UC (p.191-192, 209); The Best (Public) Systems workshop MA countertrend, 2-Systems and TSM Divergence (slides 31-58); Smarter Trading Box 5-1, Ch 6 and Box 8-5; TSM4 CD FC Orson and volume programs; New Trading Systems and Methods Parabolic, Four-Week Rule (p.177), Moon trading and Multiple time periods. Reviewed in authors/kaufman/admission/SOURCE-CONTRACT.md with independent chronological ledgers. Orders follow the printed code; no costs; the book's futures results are not reproduced.");
    p.preferred_basket=Some("nifty50");p.market_adaptation=true;Some(p)
}
// ---------------------------------------------------------------- end Kaufman
// ---------------------------------------------------------------- Appel 2026-10-24
// Admission only for the module REVIEWED lists (authors/appel/admission/SOURCE-CONTRACT.md); kept in one block to merge.
pub fn appel_reviewed(def:&StrategyDef)->bool {matches!(def.book,"apta"|"apmacd"|"apwms"|"apsmts"|"apoi") && (crate::books::apta::reviewed(def.id) || crate::books::apmacd::reviewed(def.id) || crate::books::apwms::reviewed(def.id) || crate::books::apsmts::reviewed(def.id) || crate::books::apoi::reviewed(def.id))}
const APPEL_LABEL:&str="Gerald Appel tested these models on the Dow, the NYSE Index, the Nasdaq Composite or the S&P 500 with NYSE breadth; index, ETF and stock runs (US and India, India with NSE breadth and India VIX) are a declared application, not the author's test. Cash-index shorts are hypothetical exposure.";
fn appel_policy(def:&StrategyDef)->Option<Policy> {
    if !appel_reviewed(def) {return None;}
    let mut p=requirement("index_etf_or_stock",&["index","etf","individual_stock"],"Gerald Appel: Technical Analysis - Power Tools (2005) 200/100-day Dow crossings, Triple Momentum, 15-day ROC, best/worst six months, Daily Breadth Impulse (with the MACD filter), VIX 35/18, MACD 12-26/19-39; the 1996 MACD booklet rule sets and A-D super stay-in; Winning Market Systems TRIN, up/down volume, Shadow Line, 10-day A-D total and Haurlan indices; Stock Market Trading Systems (with Hitschler) penetration, confirming, shifting, MMACDTS, lead-time, chord, A-D overbought/oversold and 25-day channel systems; Opportunity Investing presidential strategies. Reviewed in authors/appel/admission/SOURCE-CONTRACT.md with independent chronological ledgers. Close fills; no costs; the book's statistics are not reproduced.");
    p.preferred_basket=Some("nifty50");p.market_adaptation=true;Some(p)
}
// ---------------------------------------------------------------- end Appel
// ---------------------------------------------------------------- Sperandeo 2026-10-24
// Admission only for the module REVIEWED lists (authors/sperandeo/admission/SOURCE-CONTRACT.md); kept in one block to merge.
pub fn sperandeo_reviewed(def:&StrategyDef)->bool {matches!(def.book,"svtvc"|"svmws") && (crate::books::svtvc::reviewed(def.id) || crate::books::svmws::reviewed(def.id))}
const SPERANDEO_LABEL:&str="Victor Sperandeo tested the moving-average rules on the S&P 500 cash index, the 7-month 1.6-weighted rule on futures sectors and the 10/30-week rule on stocks, indexes and commodities; runs on ETFs, indices and stocks, US and India, are a declared application, not the author's test. Shorts on a cash index are hypothetical exposure.";
fn sperandeo_policy(def:&StrategyDef)->Option<Policy> {
    if !sperandeo_reviewed(def) {return None;}
    let mut p=requirement("index_etf_or_stock",&["index","etf","individual_stock"],"Victor Sperandeo, Trader Vic on Commodities (Wiley 2008) Ch.6 Tables 6.3-6.5 notes (p.52-64) and App. A (p.155, 173-174); Methods of a Wall Street Master (Wiley 1991) Ch.8 (p.90-93). Reviewed in authors/sperandeo/admission/SOURCE-CONTRACT.md with independent chronological ledgers; fills at the close; no costs.");
    p.preferred_basket=Some("nifty50");p.market_adaptation=true;Some(p)
}
// ---------------------------------------------------------------- end Sperandeo
// ---------------------------------------------------------------- CSmith 2026-10-24
// Admission only for the module REVIEWED lists (authors/csmith/admission/SOURCE-CONTRACT.md); kept in one block to merge.
pub fn csmith_reviewed(def:&StrategyDef)->bool {matches!(def.book,"csmlt") && (crate::books::csmlt::reviewed(def.id))}
const CSMITH_LABEL:&str="Courtney Smith wrote these methods for currency pairs; runs on indices, ETFs and stocks, US and India, are a declared application, not the author's test. Cash-index shorts are hypothetical exposure.";
fn csmith_policy(def:&StrategyDef)->Option<Policy> {
    if !csmith_reviewed(def) {return None;}
    let mut p=requirement("index_etf_or_stock",&["index","etf","individual_stock"],"Courtney D. Smith, How to Make a Living Trading Foreign Exchange (Wiley 2010): Ch.2 trend analysis (p.13-30), Ch.3 channel breakouts (p.31-53), Ch.4 the Conqueror (p.55-65), Ch.5 stochastics (p.81), Ch.6 reversal days and RRR (p.97-103). Reviewed in authors/csmith/admission/SOURCE-CONTRACT.md with independent chronological ledgers; no costs.");
    p.preferred_basket=Some("nifty50");p.market_adaptation=true;Some(p)
}
// ---------------------------------------------------------------- end CSmith
// ---------------------------------------------------------------- Lien 2026-10-24
// Admission only for the module REVIEWED lists (authors/lien/admission/SOURCE-CONTRACT.md); kept in one block to merge.
pub fn lien_reviewed(def:&StrategyDef)->bool {matches!(def.book,"klhpt"|"kldst"|"kldtc") && (crate::books::kllien::reviewed(def.id))}
const LIEN_LABEL:&str="Kathy Lien and Boris Schlossberg wrote these setups for currency pairs; runs on indices, ETFs and stocks, US and India, are a declared application, not the authors' test. Cash-index shorts are hypothetical exposure.";
fn lien_policy(def:&StrategyDef)->Option<Policy> {
    if !lien_reviewed(def) {return None;}
    let mut p=requirement("index_etf_or_stock",&["index","etf","individual_stock"],"Kathy Lien and Boris Schlossberg, High Probability Trading Setups for the Currency Market (Investopedia 2006), Setup 2 Do the Right Thing (p.27-33); Kathy Lien, Day Trading and Swing Trading the Currency Market (Wiley 2008) Ch.9 Perfect Order (p.138-140), same rule in Day Trading the Currency Market (Wiley 2005) Ch.8. Reviewed in authors/lien/admission/SOURCE-CONTRACT.md with independent chronological ledgers; no costs.");
    p.preferred_basket=Some("nifty50");p.market_adaptation=true;Some(p)
}
// ---------------------------------------------------------------- end Lien
// ---------------------------------------------------------------- Arms 2026-10-24
// Admission only for the module REVIEWED lists (authors/arms/admission/SOURCE-CONTRACT.md); kept in one block to merge.
pub fn arms_reviewed(def:&StrategyDef)->bool {matches!(def.book,"ravcs"|"rasmm"|"rateq") && (crate::books::rwarms::reviewed(def.id))}
const ARMS_LABEL:&str="Richard Arms tested Ease of Movement on NYSE stocks (IBM, K mart, Ford); runs on indices, ETFs and stocks, US and India, are a declared application, not the author's test. Cash-index shorts are hypothetical exposure.";
fn arms_policy(def:&StrategyDef)->Option<Policy> {
    if !arms_reviewed(def) {return None;}
    let mut p=requirement("index_etf_or_stock",&["index","etf","individual_stock"],"Richard W. Arms Jr., Volume Cycles in the Stock Market (1983/1994) Part Three: Ch.15 Calculating Ease of Movement, Ch.16 13-day EMV on daily charts, Ch.17 four-week reversal of the 13-week EMV. Reviewed in authors/arms/admission/SOURCE-CONTRACT.md with independent chronological ledgers; fills at the close; needs volume; no costs.");
    p.preferred_basket=Some("nifty50");p.market_adaptation=true;Some(p)
}
// ---------------------------------------------------------------- end Arms
// ---------------------------------------------------------------- Wirrick 2026-10-24
// Admission only for the module REVIEWED lists (authors/wirrick/admission/SOURCE-CONTRACT.md); kept in one block to merge.
pub fn wirrick_reviewed(def:&StrategyDef)->bool {matches!(def.book,"swmfst"|"swhoi"|"swhot") && (crate::books::swirrick::reviewed(def.id))}
const WIRRICK_LABEL:&str="Steve Wirrick taught this set-up on US stocks traded with options; runs on the stock and index prices of indices, ETFs and stocks, US and India, are a declared application, not the author's test. Cash-index shorts are hypothetical exposure.";
fn wirrick_policy(def:&StrategyDef)->Option<Policy> {
    if !wirrick_reviewed(def) {return None;}
    let mut p=requirement("index_etf_or_stock",&["index","etf","individual_stock"],"Steve Wirrick, Make a Fortune Swing Trading! (2003), The Swing Trade Set-up (p.38, 43, 50-53, 58): pull-back of two closes in a row above the 50-day average, next-day break of the set-up high, stop below the set-up low, close below the 20-day average exits; bearish mirror. Reviewed in authors/wirrick/admission/SOURCE-CONTRACT.md with independent chronological ledgers; no costs.");
    p.preferred_basket=Some("nifty50");p.market_adaptation=true;Some(p)
}
// ---------------------------------------------------------------- end Wirrick
// ---------------------------------------------------------------- BWilliams 2026-10-24
// Admission only for the module REVIEWED lists (authors/bwilliams/admission/SOURCE-CONTRACT.md); kept in one block to merge.
pub fn bwilliams_reviewed(def:&StrategyDef)->bool {matches!(def.book,"bwntd"|"bwtc1"|"bwtc2") && (crate::books::bwntd::reviewed(def.id) || crate::books::bwtc1::reviewed(def.id) || crate::books::bwtc2::reviewed(def.id))}
const BWILLIAMS_LABEL:&str="Bill Williams taught these rules on futures (coffee, S&P, gold, bonds) and stocks; runs on indices, ETFs and stocks, US and India, are a declared application, not the author's test. Cash-index shorts are hypothetical exposure.";
fn bwilliams_policy(def:&StrategyDef)->Option<Policy> {
    if !bwilliams_reviewed(def) {return None;}
    let mut p=requirement("index_etf_or_stock",&["index","etf","individual_stock"],"Bill Williams: New Trading Dimensions (1998) fractal outside the Alligator (p.36-45, 72-74) and AO turn system (p.46); Trading Chaos (1995) fractal start/signal/stop (p.135-142) and 5/13/34 rhythm (p.181-186); Trading Chaos 2nd ed. (2004) Fractal + Super AO with trailing stop (p.128-156). Reviewed in authors/bwilliams/admission/SOURCE-CONTRACT.md with independent chronological ledgers; no costs.");
    p.preferred_basket=Some("nifty50");p.market_adaptation=true;Some(p)
}
// ---------------------------------------------------------------- end BWilliams
// ---------------------------------------------------------------- Stridsman 2026-10-24
// Admission only for the module REVIEWED lists (authors/stridsman/admission/SOURCE-CONTRACT.md); kept in one block to merge.
pub fn stridsman_reviewed(def:&StrategyDef)->bool {matches!(def.book,"strtw"|"strmm") && (crate::books::strtw::reviewed(def.id) || crate::books::strmm::reviewed(def.id))}
const STRIDSMAN_LABEL:&str="Thomas Stridsman tested these systems on S&P 500 futures, 16 futures markets and 65 US stocks and indexes; runs on indices, ETFs and stocks, US and India, are a declared application, not the author's test. Cash-index shorts are hypothetical exposure.";
fn stridsman_policy(def:&StrategyDef)->Option<Policy> {
    if !stridsman_reviewed(def) {return None;}
    let mut p=requirement("index_etf_or_stock",&["index","etf","individual_stock"],"Thomas Stridsman: Trading Systems That Work (2000) Gold Digger, Meander, Black Jack, directional slope, dynamic breakout and standard deviation breakout systems (printed TradeStation code p.59-245); Trading Systems and Money Management (2003) Hybrid No. 1, Meander V.1.0, Volume-weighted Average, Harris 3L-R, Expert Exits (printed code ch.8-15, exit code ch.20) and Counterpunch (p.376). Reviewed in authors/stridsman/admission/SOURCE-CONTRACT.md with independent chronological ledgers; no costs.");
    p.preferred_basket=Some("nifty50");p.market_adaptation=true;Some(p)
}
// ---------------------------------------------------------------- end Stridsman
// ---------------------------------------------------------------- Weissman 2026-10-24
// Admission only for the module REVIEWED lists (authors/weissman/admission/SOURCE-CONTRACT.md); kept in one block to merge.
pub fn weissman_reviewed(def:&StrategyDef)->bool {matches!(def.book,"rwmts"|"rwtlc") && (crate::books::rwmts::reviewed(def.id) || crate::books::rwtlc::reviewed(def.id))}
const WEISSMAN_LABEL:&str="Richard Weissman tested these systems on futures, spot currencies and the cash S&P 500; runs on indices, ETFs and stocks, US and India, are a declared application, not the author's test. Cash-index shorts are hypothetical exposure.";
fn weissman_policy(def:&StrategyDef)->Option<Policy> {
    if !weissman_reviewed(def) {return None;}
    let mut p=requirement("index_etf_or_stock",&["index","etf","individual_stock"],"Richard L. Weissman: Mechanical Trading Systems (2004) indicator, trend-following and mean-reversion systems with printed CQG code (p.4-84); Trade Like a Casino (2011) volatility, RSI Trend and Ichimoku systems (p.57-154). Signals on the close, orders at the next open. Reviewed in authors/weissman/admission/SOURCE-CONTRACT.md with independent chronological ledgers; no costs.");
    p.preferred_basket=Some("nifty50");p.market_adaptation=true;Some(p)
}
// ---------------------------------------------------------------- end Weissman
// ---------------------------------------------------------------- Raschke 2026-10-24
// Admission only for the module REVIEWED lists (authors/raschke/admission/SOURCE-CONTRACT.md); kept in one block to merge.
pub fn raschke_reviewed(def:&StrategyDef)->bool {matches!(def.book,"lbrtt") && (crate::books::lbrtt::reviewed(def.id))}
const RASCHKE_LABEL:&str="Linda Raschke tested these patterns on futures; runs on indices, ETFs and stocks, US and India, are a declared application, not the author's test. Cash-index shorts are hypothetical exposure.";
fn raschke_policy(def:&StrategyDef)->Option<Policy> {
    if !raschke_reviewed(def) {return None;}
    let mut p=requirement("index_etf_or_stock",&["index","etf","individual_stock"],"Linda Bradford Raschke, Trading Techniques 2008 one-day workshop manual: Wide Range Reversal System (PDF p.49, printed p.47) and the Extended Run retracement (slides 101-103). Reviewed in authors/raschke/admission/SOURCE-CONTRACT.md with independent chronological ledgers; no costs.");
    p.preferred_basket=Some("nifty50");p.market_adaptation=true;Some(p)
}
// ---------------------------------------------------------------- end Raschke
// ---------------------------------------------------------------- CArnold 2026-10-24
// Admission only for the module REVIEWED lists (authors/carnold/admission/SOURCE-CONTRACT.md); kept in one block to merge.
pub fn carnold_reviewed(def:&StrategyDef)->bool {matches!(def.book,"capttm") && (crate::books::capttm::reviewed(def.id))}
const CARNOLD_LABEL:&str="Curtis Arnold's moving-average examples come from gold futures (Merrill Lynch study) and general futures/stock charting; runs on indices, ETFs and stocks, US and India, are a declared application, not the author's test. Cash-index shorts are hypothetical exposure.";
fn carnold_policy(def:&StrategyDef)->Option<Policy> {
    if !carnold_reviewed(def) {return None;}
    let mut p=requirement("index_etf_or_stock",&["index","etf","individual_stock"],"Curtis M. Arnold, Timing the Market (1993), ch.13 Moving averages (p.82): the 14/50-day two-crossover model and its modified version (close above/below both averages, exit when the close crosses either). Signals and fills on the close. Reviewed in authors/carnold/admission/SOURCE-CONTRACT.md with independent chronological ledgers; no costs.");
    p.preferred_basket=Some("nifty50");p.market_adaptation=true;Some(p)
}
// ---------------------------------------------------------------- end CArnold
// ---------------------------------------------------------------- NDavis 2026-10-24
// Admission only for the module REVIEWED lists (authors/ndavis/admission/SOURCE-CONTRACT.md); kept in one block to merge.
pub fn ndavis_reviewed(def:&StrategyDef)->bool {matches!(def.book,"ndbrm") && (crate::books::ndbrm::reviewed(def.id))}
const NDAVIS_LABEL:&str="Ned Davis Research tested these indicators on the S&P 500, the Dow and NASDAQ breadth; runs on indices, ETFs and stocks, US and India, are a declared application, not the author's test. NYSE breadth rules need NYSE breadth data (US, to Feb 2020); NSE breadth rules run on Indian instruments.";
fn ndavis_policy(def:&StrategyDef)->Option<Policy> {
    if !ndavis_reviewed(def) {return None;}
    let mut p=requirement("index_etf_or_stock",&["index","etf","individual_stock"],"Ned Davis, Being Right or Making Money (2nd ed. 2000): S&P 500 8.4%/7.2% weekly reversals and the 40-day TRIN (ch.4 p.87-88), A/D line vs its 25-day average (ch.2 p.47; NSE breadth mapping). Long or cash, signals and fills on the close. Reviewed in authors/ndavis/admission/SOURCE-CONTRACT.md with independent chronological ledgers; no costs.");
    p.preferred_basket=Some("nifty50");p.market_adaptation=true;Some(p)
}
// ---------------------------------------------------------------- end NDavis
// ---------------------------------------------------------------- Siegel 2026-10-24
// Admission only for the module REVIEWED lists (authors/siegel/admission/SOURCE-CONTRACT.md); kept in one block to merge.
pub fn siegel_reviewed(def:&StrategyDef)->bool {matches!(def.book,"sglr") && (crate::books::sglr::reviewed(def.id))}
const SIEGEL_LABEL:&str="Jeremy Siegel tested this rule on the Dow (1885-2001) and the Nasdaq Composite; runs on indices, ETFs and stocks, US and India, are a declared application, not the author's test. Price-only; the book adds dividends and T-bill interest.";
fn siegel_policy(def:&StrategyDef)->Option<Policy> {
    if !siegel_reviewed(def) {return None;}
    let mut p=requirement("index_etf_or_stock",&["index","etf","individual_stock"],"Jeremy J. Siegel, Stocks for the Long Run (3rd ed. 2002) ch.17 p.289-294: buy when the index closes at least 1% above its 200-day average, sell when it closes at least 1% below; long or cash at the close. Reviewed in authors/siegel/admission/SOURCE-CONTRACT.md with independent chronological ledgers; no costs.");
    p.preferred_basket=Some("nifty50");p.market_adaptation=true;Some(p)
}
// ---------------------------------------------------------------- end Siegel
// ---------------------------------------------------------------- Lydon 2026-10-24
// Admission only for the module REVIEWED lists (authors/lydon/admission/SOURCE-CONTRACT.md); kept in one block to merge.
pub fn lydon_reviewed(def:&StrategyDef)->bool {matches!(def.book,"lydetf"|"lydpes") && (crate::books::lydetf::reviewed(def.id) || crate::books::lydpes::reviewed(def.id))}
const LYDON_LABEL:&str="Tom Lydon applied these trend rules to ETFs (asset classes, sectors, regions) and the S&P 500; runs on indices, ETFs and stocks, US and India, are a declared application, not the author's test.";
fn lydon_policy(def:&StrategyDef)->Option<Policy> {
    if !lydon_reviewed(def) {return None;}
    let mut p=requirement("index_etf_or_stock",&["index","etf","individual_stock"],"Tom Lydon: The ETF Trend Following Playbook (2010) 200-day EMA long/cash (p.25-32); iMoney / Profitable ETF Strategies (2008, with John F. Wasik) 200-day average with the 8% trailing exit and 5% re-entry (p.149-152). Signals and fills on the close. Reviewed in authors/lydon/admission/SOURCE-CONTRACT.md with independent chronological ledgers; no costs.");
    p.preferred_basket=Some("nifty50");p.market_adaptation=true;Some(p)
}
// ---------------------------------------------------------------- end Lydon
// ---------------------------------------------------------------- Landry 2026-10-24
// Admission only for the module REVIEWED lists (authors/landry/admission/SOURCE-CONTRACT.md); kept in one block to merge.
pub fn landry_reviewed(def:&StrategyDef)->bool {matches!(def.book,"dlost"|"dl10b") && (crate::books::dlost::reviewed(def.id) || crate::books::dl10b::reviewed(def.id))}
const LANDRY_LABEL:&str="Dave Landry taught these patterns on US stocks (prices in sixteenths and cents) and timed the S&P 500 futures with the 3-10 oscillator and the VIX; runs on indices, ETFs and stocks are a declared application, not the author's test. The patterns' 1/16 and cent offsets are US-dollar amounts, so the patterns run on US instruments only; OSS and CVR III-Modified also run on India (India VIX). Cash-index shorts are hypothetical exposure.";
fn landry_policy(def:&StrategyDef)->Option<Policy> {
    if !landry_reviewed(def) {return None;}
    let mut p=requirement("index_etf_or_stock",&["index","etf","individual_stock"],"Dave Landry: Dave Landry on Swing Trading (2000) pullback, knockout, trend pivot and bow tie patterns with the book's 2-for-1 money management (ch.5-7, p.12-13, 37-85, 206), the Oscillator Swing System and CVR III-Modified (ch.10, p.129-141); 10 Best Swing Trading Patterns: Explosion Gap Pivots (p.72-79) and Reversal Gap Strategy (p.104-108). Reviewed in authors/landry/admission/SOURCE-CONTRACT.md with independent chronological ledgers; no costs.");
    p.preferred_basket=Some("nifty50");p.market_adaptation=true;Some(p)
}
// Landry patterns: 1/16-dollar and cent order offsets are US-dollar amounts -> US instruments only (stock_us None = not a stock).
const LANDRY_US_ONLY:&[&str]=&["dlost_simple_pb","dlost_od_pb","dlost_tko","dlost_dtko","dlost_tpp","dlost_bow_tie","dl10b_egp","dl10b_rgs"];
fn landry_symbol_issue(def:&StrategyDef,ticker:&str,stock_us:Option<bool>)->Option<Issue> {
    if !landry_reviewed(def) || !LANDRY_US_ONLY.contains(&def.id) {return None;}
    if stock_us.unwrap_or(!ticker.starts_with("NIFTY")) {return None;}
    Some(Issue{status:"needs_data",note:format!("This pattern's order and stop offsets are US-dollar amounts (1/16 of a dollar, 10 or 25 cents on US stocks), so it runs on US indices, ETFs and stocks only. {ticker} is outside that scope; no result is published.")})
}
// ---------------------------------------------------------------- end Landry
// ---------------------------------------------------------------- Pruitt 2026-10-24
// Admission only for the module REVIEWED lists (authors/pruitt/admission/SOURCE-CONTRACT.md); kept in one block to merge.
pub fn pruitt_reviewed(def:&StrategyDef)->bool {matches!(def.book,"pbwts"|"putg") && (crate::books::pbwts::reviewed(def.id) || crate::books::putg::reviewed(def.id))}
const PRUITT_LABEL:&str="George Pruitt and John Hill tested these systems on futures (17-29 markets, one contract) and a few US stocks; runs on indices, ETFs and stocks, US and India, are a declared application, not the author's test. The soybean seasonal dates come from soybeans. Cash-index shorts are hypothetical exposure.";
fn pruitt_policy(def:&StrategyDef)->Option<Policy> {
    if !pruitt_reviewed(def) {return None;}
    let mut p=requirement("index_etf_or_stock",&["index","etf","individual_stock"],"George Pruitt & John R. Hill: Building Winning Trading Systems with TradeStation (2002) King Keltner, Bollinger Bandit, Thermostat, Dynamic Break Out II and its fade, seasonal, Ghost Trader, Money Manager, 40-day channel, ADX/MA, MA channel, %R and day-of-week studies (printed EasyLanguage ch.1-8, App. B); The Ultimate Trading Guide (2000) ch.9 turnkey systems (App. code p.285-291) and ch.3 bar-chart studies. Reviewed in authors/pruitt/admission/SOURCE-CONTRACT.md with independent chronological ledgers; no costs.");
    p.preferred_basket=Some("nifty50");p.market_adaptation=true;Some(p)
}
// ---------------------------------------------------------------- end Pruitt
// ---------------------------------------------------------------- Hirsch 2026-10-24
// Admission only for the module REVIEWED lists (authors/hirsch/admission/SOURCE-CONTRACT.md); kept in one block to merge.
pub fn hirsch_reviewed(def:&StrategyDef)->bool {matches!(def.book,"hsta") && (crate::books::hsta::reviewed(def.id))}
const HIRSCH_LABEL:&str="The Hirsch almanacs measured these calendar windows on the Dow, the S&P 500 and the NASDAQ; runs on indices, ETFs and stocks, US and India, are a declared application, not the author's test. The Thanksgiving window is set by a US exchange holiday and runs on US instruments only.";
fn hirsch_policy(def:&StrategyDef)->Option<Policy> {
    if !hirsch_reviewed(def) {return None;}
    let mut p=requirement("index_etf_or_stock",&["index","etf","individual_stock"],"Yale & Jeffrey A. Hirsch, Stock Trader's Almanac (2008 ed.; 2005 ed. MACD parameters; 2010 ed. four-year cycle): Best Six Months and its MACD and four-year-cycle versions, NASDAQ Best Eight Months, Santa Claus rally, St. Patrick's Day, mid-year NASDAQ rally, first trading day, Thanksgiving, Super-8 days, Yom Kippur to Passover, last seven months of election years. Long from the close before each window to the close of its last day. Reviewed in authors/hirsch/admission/SOURCE-CONTRACT.md with independent chronological ledgers; no costs.");
    p.preferred_basket=Some("nifty50");p.market_adaptation=true;Some(p)
}
// Hirsch: US instruments only for these ids (stock_us None = not a stock).
const HIRSCH_US_ONLY:&[&str]=&["hsta_thanksgiving"];
fn hirsch_symbol_issue(def:&StrategyDef,ticker:&str,stock_us:Option<bool>)->Option<Issue> {
    if !hirsch_reviewed(def) || !HIRSCH_US_ONLY.contains(&def.id) {return None;}
    if stock_us.unwrap_or(!ticker.starts_with("NIFTY")) {return None;}
    Some(Issue{status:"needs_data",note:format!("This window is set by the US Thanksgiving exchange holiday (the market is closed on Thursday), so it runs on US indices, ETFs and stocks only. {ticker} is outside that scope; no result is published.")})
}
// ---------------------------------------------------------------- end Hirsch
// ---------------------------------------------------------------- Bandy 2026-10-24
// Admission only for the module REVIEWED lists (authors/bandy/admission/SOURCE-CONTRACT.md); kept in one block to merge.
pub fn bandy_reviewed(def:&StrategyDef)->bool {matches!(def.book,"hbqts"|"hbiab") && (crate::books::hbqts::reviewed(def.id) || crate::books::hbiab::reviewed(def.id))}
const BANDY_LABEL:&str="Howard Bandy showed these AmiBroker examples on US stocks, sector ETFs and T-bond futures; runs on indices, ETFs and stocks, US and India, are a declared application, not the author's test. Cash-index shorts are hypothetical exposure.";
fn bandy_policy(def:&StrategyDef)->Option<Policy> {
    if !bandy_reviewed(def) {return None;}
    let mut p=requirement("index_etf_or_stock",&["index","etf","individual_stock"],"Howard B. Bandy: Quantitative Trading Systems (2007) AFL examples (entries and exits ch.5-7, trend and mean-reversion ch.9-10, seasonality and patterns ch.11-13, multi-time-frame filters ch.17, stops and allocation ch.22, App. A); Introduction to AmiBroker (2008) TrendFollowing.afl and MeanReversion.afl (p.542-543). Reviewed in authors/bandy/admission/SOURCE-CONTRACT.md with independent chronological ledgers; no costs; one position at a time.");
    p.preferred_basket=Some("nifty50");p.market_adaptation=true;Some(p)
}
// ---------------------------------------------------------------- end Bandy
// ---------------------------------------------------------------- Kestner 2026-10-24
// Admission only for the module REVIEWED lists (authors/kestner/admission/SOURCE-CONTRACT.md); kept in one block to merge.
pub fn kestner_reviewed(def:&StrategyDef)->bool {matches!(def.book,"lkqts"|"lkcpt") && (crate::books::lkqts::reviewed(def.id) || crate::books::lkcpt::reviewed(def.id))}
const KESTNER_LABEL:&str="Lars Kestner tested these systems on futures portfolios, the S&P 500 and US stocks (Quantitative Trading Strategies) and on S&P 500 futures (A Comparison of Popular Trading Systems); runs on indices, ETFs and stocks, US and India, are a declared application, not the author's test. Cash-index shorts are hypothetical exposure.";
fn kestner_policy(def:&StrategyDef)->Option<Policy> {
    if !kestner_reviewed(def) {return None;}
    let mut p=requirement("index_etf_or_stock",&["index","etf","individual_stock"],"Lars N. Kestner: Quantitative Trading Strategies (2003) momentum, stochastic, Kestner MA, MACD-histogram, NEI, adjusted stochastic, volume reversal, Saitta, channel-breakout exits/filters, relative-value statistics, S&P 500 calendar, VIX and monthly timing, new volatility breakout (ch.6-12); A Comparison of Popular Trading Systems (2nd ed. 1999), 35 systems from Kestner's printed EasyLanguage (App. p.225-244). Reviewed in authors/kestner/admission/SOURCE-CONTRACT.md with independent chronological ledgers; no costs.");
    p.preferred_basket=Some("nifty50");p.market_adaptation=true;Some(p)
}
// ---------------------------------------------------------------- end Kestner
// ---------------------------------------------------------------- Jobman 2026-10-24
// Admission only for the module REVIEWED lists (authors/jobman/admission/SOURCE-CONTRACT.md); kept in one block to merge.
pub fn jobman_reviewed(def:&StrategyDef)->bool {matches!(def.book,"djhta") && (crate::books::djhta::reviewed(def.id))}
const JOBMAN_LABEL:&str="Dennis Dunn's technique is quoted for futures (no market named, no results); runs on indices, ETFs and stocks, US and India, are a declared application, not the author's test.";
fn jobman_policy(def:&StrategyDef)->Option<Policy> {
    if !jobman_reviewed(def) {return None;}
    let mut p=requirement("index_etf_or_stock",&["index","etf","individual_stock"],"Darrell R. Jobman (ed.), The Handbook of Technical Analysis (1995), ch.8 p.125-126: Dunn's seven-week moving average, Thursday closes over/under the average plotted two weeks ahead, always in the market. Signals and fills on the Thursday close. Reviewed in authors/jobman/admission/SOURCE-CONTRACT.md with independent chronological ledgers; no costs.");
    p.preferred_basket=Some("nifty50");p.market_adaptation=true;Some(p)
}
// ---------------------------------------------------------------- end Jobman
// ---------------------------------------------------------------- Schaap 2026-10-24
// Admission only for the module REVIEWED lists (authors/schaap/admission/SOURCE-CONTRACT.md); kept in one block to merge.
pub fn schaap_reviewed(def:&StrategyDef)->bool {matches!(def.book,"cbsiws") && (crate::books::cbsiws::reviewed(def.id))}
const SCHAAP_LABEL:&str="Charles Schaap applies the 50-50 Strategy to US stocks and ETFs on weekly charts; runs on US indices, ETFs and stocks are a declared application. The $0.25 stop offset is a US-dollar amount, so Indian instruments are not run.";
fn schaap_policy(def:&StrategyDef)->Option<Policy> {
    if !schaap_reviewed(def) {return None;}
    let mut p=requirement("index_etf_or_stock",&["index","etf","individual_stock"],"Charles B. Schaap, Invest with Success (2008): 50-50 Strategy, weekly 50 EMA and RSI 20 double cross (p.96, 196), High Riser entry on the first bar entirely above the rising 50 MA (p.198), stop $0.25 under the 50 MA, breakeven and monthly trailing (p.200, 215-218). Reviewed in authors/schaap/admission/SOURCE-CONTRACT.md with independent chronological ledgers; no costs.");
    p.preferred_basket=Some("nifty50");p.market_adaptation=true;Some(p)
}
// Schaap: US instruments only for these ids (stock_us None = not a stock).
const SCHAAP_US_ONLY:&[&str]=&["cbsiws_5050"];
fn schaap_symbol_issue(def:&StrategyDef,ticker:&str,stock_us:Option<bool>)->Option<Issue> {
    if !schaap_reviewed(def) || !SCHAAP_US_ONLY.contains(&def.id) {return None;}
    if stock_us.unwrap_or(!ticker.starts_with("NIFTY")) {return None;}
    Some(Issue{status:"needs_data",note:format!("The 50-50 Strategy's stop sits $0.25 under the 50-week average, a US-dollar stock amount, so it runs on US instruments only. {ticker} is outside that scope; no result is published.")})
}
// ---------------------------------------------------------------- end Schaap
// ---------------------------------------------------------------- Satchell 2026-10-24
// Admission only for the module REVIEWED lists (authors/satchell/admission/SOURCE-CONTRACT.md); kept in one block to merge.
pub fn satchell_reviewed(def:&StrategyDef)->bool {matches!(def.book,"acatr") && (crate::books::acatr::reviewed(def.id))}
const SATCHELL_LABEL:&str="The chapter authors of Advanced Trading Rules (Acar and Satchell, eds.) studied these rules on currencies, Swiss franc futures and the FT100; runs on indices, ETFs and stocks, US and India, are a declared application, not the authors' test.";
fn satchell_policy(def:&StrategyDef)->Option<Policy> {
    if !satchell_reviewed(def) {return None;}
    let mut p=requirement("index_etf_or_stock",&["index","etf","individual_stock"],"E. Acar and S. Satchell (eds.), Advanced Trading Rules, 2nd ed. (2002): Kuo's geometric moving-average (2,1) rule, Eq. 5.3-5.4 p.156-160; Gasser's 45-bar close breakout, TradeStation code p.430; Chang and Osler's filter rules 0.5/1/2 per cent, p.265-269. Signals and fills on the close. Reviewed in authors/satchell/admission/SOURCE-CONTRACT.md with independent chronological ledgers; no costs.");
    p.preferred_basket=Some("nifty50");p.market_adaptation=true;Some(p)
}
// ---------------------------------------------------------------- end Satchell
// ---------------------------------------------------------------- Kirkpatrick 2026-10-24
// Admission only for the module REVIEWED lists (authors/kirkpatrick/admission/SOURCE-CONTRACT.md); kept in one block to merge.
pub fn kirkpatrick_reviewed(def:&StrategyDef)->bool {matches!(def.book,"kdta") && (crate::books::kdta::reviewed(def.id))}
const KIRKPATRICK_LABEL:&str="Kirkpatrick and Dahlquist cite or test these rules on the DJIA, S&P 500, NYSE breadth, 18 or 19 futures/stocks and AAPL; runs on indices, ETFs and stocks, US and India, are a declared application, not the authors' test. Breadth rules read NYSE breadth on US instruments and NSE breadth on Indian ones (thresholds unchanged).";
fn kirkpatrick_policy(def:&StrategyDef)->Option<Policy> {
    if !kirkpatrick_reviewed(def) {return None;}
    let mut p=requirement("index_etf_or_stock",&["index","etf","individual_stock"],"Kirkpatrick and Dahlquist, Technical Analysis (FT Press 2006): VIX 10-day EMA (Box 7.3 p.98), RSI is wrong (p.440), Apple-a-Day systems (Box 22.1-22.2 p.551-564), May-October (p.165), 6% pullback (p.387), one-day A/D line (p.133), 10-day A/D ratio 1.91 (p.138), Zweig breadth thrust (p.142), Haurlan 200-day (p.136), Arms Index rules (p.143-145). Reviewed in authors/kirkpatrick/admission/SOURCE-CONTRACT.md with independent chronological ledgers; no costs.");
    p.preferred_basket=Some("nifty50");p.market_adaptation=true;Some(p)
}
// ---------------------------------------------------------------- end Kirkpatrick
// ---------------------------------------------------------------- Goslin 2026-10-24
// Admission only for the module REVIEWED lists (authors/goslin/admission/SOURCE-CONTRACT.md); kept in one block to merge.
pub fn goslin_reviewed(def:&StrategyDef)->bool {matches!(def.book,"cgtdd") && (crate::books::cgtdd::reviewed(def.id))}
const GOSLIN_LABEL:&str="Chick Goslin traded the Status on futures (stock index, currency, metals, grains); runs on indices, ETFs and stocks, US and India, are a declared application, not the author's test. The author warns the Status alone is not a straight mechanical system; this tests it exactly as defined, without his judgement overrides.";
fn goslin_policy(def:&StrategyDef)->Option<Policy> {
    if !goslin_reviewed(def) {return None;}
    let mut p=requirement("index_etf_or_stock",&["index","etf","individual_stock"],"F. H. Chick Goslin, Trading Day by Day (2002): three-point Status p.60-62 (50-day trend, 3-10 short line, 16-day middle line, confirming half point), get out when the Status returns to Neutral (p.114), orders at the next open (p.109). Reviewed in authors/goslin/admission/SOURCE-CONTRACT.md with independent chronological ledgers; no costs.");
    p.preferred_basket=Some("nifty50");p.market_adaptation=true;Some(p)
}
// ---------------------------------------------------------------- end Goslin
// ---------------------------------------------------------------- Ruggiero 2026-10-24
// Admission only for the module REVIEWED lists (authors/ruggiero/admission/SOURCE-CONTRACT.md); kept in one block to merge.
pub fn ruggiero_reviewed(def:&StrategyDef)->bool {matches!(def.book,"rgcts") && (crate::books::rgcts::reviewed(def.id))}
const RUGGIERO_LABEL:&str="Murray Ruggiero tested these rules on futures (S&P 500, T-Bonds, D-Mark, Yen and others); runs on indices, ETFs and stocks, US and India, are a declared application, not the author's test.";
fn ruggiero_policy(def:&StrategyDef)->Option<Policy> {
    if !ruggiero_reviewed(def) {return None;}
    let mut p=requirement("index_etf_or_stock",&["index","etf","individual_stock"],"Murray A. Ruggiero Jr., Cybernetic Trading Strategies (1997): conditional day of week p.50, trading day of month p.51 (Table 2.5), channel and volatility breakouts p.77-78, 17/10 breakout p.79, momentum precedes price Table 4.13 p.84, Elliott Wave counter Tables 13.2-13.5 p.189-192, GA rule generation 1723 Table 20.6 p.305. TradeStation order model. Reviewed in authors/ruggiero/admission/SOURCE-CONTRACT.md with independent chronological ledgers; no costs.");
    p.preferred_basket=Some("nifty50");p.market_adaptation=true;Some(p)
}
// ---------------------------------------------------------------- end Ruggiero
// ---------------------------------------------------------------- Link 2026-10-24
// Admission only for the module REVIEWED lists (authors/link/admission/SOURCE-CONTRACT.md); kept in one block to merge.
pub fn link_reviewed(def:&StrategyDef)->bool {matches!(def.book,"mlhpt") && (crate::books::mlhpt::reviewed(def.id))}
const LINK_LABEL:&str="Marcel Link gives these systems for futures and stocks without a named market or test results; runs on indices, ETFs and stocks, US and India, are a declared application, not the author's test.";
fn link_policy(def:&StrategyDef)->Option<Policy> {
    if !link_reviewed(def) {return None;}
    let mut p=requirement("index_etf_or_stock",&["index","etf","individual_stock"],"Marcel Link, High Probability Trading (2003): 10-bar close breakout with the 3-bar-low exit (p.173); the sample system's TradeStation code with ADX-switched exits and a 2 standard deviation stop (p.263-264). Reviewed in authors/link/admission/SOURCE-CONTRACT.md with independent chronological ledgers; no costs.");
    p.preferred_basket=Some("nifty50");p.market_adaptation=true;Some(p)
}
// ---------------------------------------------------------------- end Link
// ---------------------------------------------------------------- Katz 2026-10-25
// Admission only for the module REVIEWED lists (authors/katz/admission/SOURCE-CONTRACT.md); kept in one block to merge.
pub fn katz_reviewed(def:&StrategyDef)->bool {(def.book=="kets" && crate::books::kets::reviewed(def.id))}
const KATZ_LABEL:&str="Katz and McCormick tested these entry and exit models on a 36-market futures portfolio (1985-1999); runs on indices, ETFs and single stocks (US and India) are a declared application, one unit per trade, no futures ticks or costs.";
fn katz_policy(def:&StrategyDef)->Option<Policy> {
    if !katz_reviewed(def) {return None;}
    let mut p=requirement("index_etf_or_stock",&["index","etf","individual_stock"],"The Encyclopedia of Trading Strategies (McGraw-Hill, 2000), printed C++ and the authors' C-Trader Toolkit CD: Ch.5 breakouts p.86-103, Ch.7 oscillators p.141-150, Ch.8 seasonality p.159-174, Ch.9 lunar p.183-195, Ch.10 cycles p.211-221, Ch.13-14 random entries and exits p.295-332. Seasonal and lunar averages use earlier years/cycles only (labelled adaptation). Index/ETF/stock runs are a declared application.");
    p.preferred_basket=Some("nifty50");p.market_adaptation=true;Some(p)
}
// ---------------------------------------------------------------- end Katz
// ---------------------------------------------------------------- Crabel 2026-10-25
// Admission only for the module REVIEWED lists (authors/crabel/admission/SOURCE-CONTRACT.md); kept in one block to merge.
pub fn crabel_reviewed(def:&StrategyDef)->bool {(def.book=="tcdt" && crate::books::tcdt::reviewed(def.id))}
const CRABEL_LABEL:&str="Toby Crabel tested these patterns on S&P 500 futures (open-to-close tables, 1982-88) and T-Bond futures (upthrusts, 1978-87); runs on indices, ETFs and single stocks (US and India) are a declared application.";
fn crabel_policy(def:&StrategyDef)->Option<Policy> {
    if !crabel_reviewed(def) {return None;}
    let mut p=requirement("index_etf_or_stock",&["index","etf","individual_stock"],"Day Trading with Short Term Price Patterns and Opening Range Breakout (Traders Press, 1990): Open to Close Bias Tables A and B printed p.67-69 (pattern convention p.51); Upthrust and Spring reversals printed p.261-265, pivot top p.284. Index/ETF/stock runs are a declared application.");
    p.preferred_basket=Some("nifty50");p.market_adaptation=true;Some(p)
}
// ---------------------------------------------------------------- end Crabel
// ---------------------------------------------------------------- STAD 2026-10-25
// Admission only for the module REVIEWED lists (authors/stad/admission/SOURCE-CONTRACT.md); kept in one block to merge.
pub fn stad_reviewed(def:&StrategyDef)->bool {(def.book=="stad" && crate::books::stad::reviewed(def.id)) || (def.book=="stad2" && crate::books::stad2::reviewed(def.id))}
const STAD_LABEL:&str="Omega Research printed these TradeStation systems with examples on stocks, indices and futures (1997-2000); runs on indices, ETFs and single stocks (US and India) are a declared application, one unit per trade.";
fn stad_policy(def:&StrategyDef)->Option<Policy> {
    if !stad_reviewed(def) {return None;}
    let mut p=requirement("index_etf_or_stock",&["index","etf","individual_stock"],"Omega Research System Trading and Development Club Vol. 1-13 (1997-2000): the printed EasyLanguage of each system, run with the book's bar assumptions and 10% bouncing ticks (Vol 5 ch.1). Index/ETF/stock runs are a declared application.");
    p.preferred_basket=Some("nifty50");p.market_adaptation=true;Some(p)
}
// ---------------------------------------------------------------- end STAD
// ---------------------------------------------------------------- Leslie Masonson 2026-10-25
// Admission only for the module REVIEWED lists (authors/masonson/admission/SOURCE-CONTRACT.md); kept in one block to merge.
pub fn masonson_reviewed(def:&StrategyDef)->bool {(def.book=="masmt" && crate::books::masmt::reviewed(def.id))}
const MASONSON_LABEL:&str="Masonson reports these timing rules on the S&P 500, DJIA, Nasdaq and Value Line indices; runs on other indices, ETFs and single stocks (US and India; the US presidential and calendar months kept as printed) are a declared application, one unit per trade, fills at the signal-day close.";
fn masonson_policy(def:&StrategyDef)->Option<Policy> {
    if !masonson_reviewed(def) {return None;}
    let mut p=requirement("index_etf_or_stock",&["index","etf","individual_stock"],"Leslie N. Masonson, All About Market Timing (McGraw-Hill 2004)");
    p.preferred_basket=Some("nifty50");p.market_adaptation=true;Some(p)
}
// ---------------------------------------------------------------- end Leslie Masonson
// ---------------------------------------------------------------- Art Collins 2026-10-25
// Admission only for the module REVIEWED lists (authors/collins/admission/SOURCE-CONTRACT.md); kept in one block to merge.
pub fn collins_reviewed(def:&StrategyDef)->bool {(def.book=="acbff" && crate::books::acbff::reviewed(def.id))}
const COLLINS_LABEL:&str="Art Collins tested these TradeStation systems on S&P, NASDAQ and Russell futures (and bonds/currencies); runs on indices, ETFs and single stocks (US and India) are a declared application, one unit per trade, 1 tick = 0.10 point US / 0.05 point India.";
fn collins_policy(def:&StrategyDef)->Option<Policy> {
    if !collins_reviewed(def) {return None;}
    let mut p=requirement("index_etf_or_stock",&["index","etf","individual_stock"],"Art Collins, Beating the Financial Futures Market (Wiley 2006), printed TradeStation code pp.205-238");
    p.preferred_basket=Some("nifty50");p.market_adaptation=true;Some(p)
}
// ---------------------------------------------------------------- end Art Collins
// ---------------------------------------------------------------- Guppy 2026-10-25
// Admission only for the module REVIEWED lists (authors/guppy/admission/SOURCE-CONTRACT.md); kept in one block to merge.
pub fn guppy_reviewed(def:&StrategyDef)->bool {(def.book=="dgdwb" && crate::books::dgdwb::reviewed(def.id))}
const GUPPY_LABEL:&str="Daryl Guppy's Darvas workbook applies the Modern Darvas box to shares (ASX/US stocks); runs on indices, ETFs and single stocks (US and India) are a declared application.";
fn guppy_policy(def:&StrategyDef)->Option<Policy> {
    if !guppy_reviewed(def) {return None;}
    let mut p=requirement("index_etf_or_stock",&["index","etf","individual_stock"],"Darvas Trading WorkBook (darvasbook.pdf slides 3-6, 14-19) + Modern Darvas Trading (mdarvas.pdf slides 10-19): 6-month new high, 3 lower highs set the top, a low and 3 higher lows set the bottom, buy the first trade above the box, exit the day after a close below the D_Box, ghost boxes for the stop. Index/ETF/stock runs are a declared application.");
    p.preferred_basket=Some("nifty50");p.market_adaptation=true;Some(p)
}
// ---------------------------------------------------------------- end Guppy
// ---------------------------------------------------------------- Headley 2026-10-25
// Admission only for the module REVIEWED lists (authors/headley/admission/SOURCE-CONTRACT.md); kept in one block to merge.
pub fn headley_reviewed(def:&StrategyDef)->bool {(def.book=="phbt" && crate::books::phbt::reviewed(def.id)) || (def.book=="phbttk" && crate::books::phbttk::reviewed(def.id))}
const HEADLEY_LABEL:&str="Price Headley tested the VIX systems on the S&P 100 (OEX) with the CBOE VIX and applied Acceleration Bands and Percent R to US stocks; runs on other indices, ETFs and single stocks (US and India, India VIX in India) are a declared application.";
fn headley_policy(def:&StrategyDef)->Option<Policy> {
    if !headley_reviewed(def) {return None;}
    let mut p=requirement("index_etf_or_stock",&["index","etf","individual_stock"],"Big Trends in Trading (Wiley 2002): VIX Bollinger system p.38-40 (TradeStation code), weekly VIX > 45 buy p.44-46, Acceleration Bands p.91-95 (TradeStation formula); Big Trends Toolkit 2.0 for MetaStock manual p.36-38 (Percent R confirmation). Index/ETF/stock runs are a declared application.");
    p.preferred_basket=Some("nifty50");p.market_adaptation=true;Some(p)
}
// ---------------------------------------------------------------- end Headley
// ---------------------------------------------------------------- Carter 2026-10-25
// Admission only for the module REVIEWED lists (authors/jcarter/admission/SOURCE-CONTRACT.md); kept in one block to merge.
pub fn jcarter_reviewed(def:&StrategyDef)->bool {(def.book==crate::books::jcarter::BOOK && crate::books::jcarter::reviewed(def.id))}
const JCARTER_LABEL:&str="John Carter traded the Squeeze, HOLP/LOHP and Propulsion plays on stock-index futures, forex and US stocks; runs on indices, ETFs and single stocks (US and India) are a declared application.";
fn jcarter_policy(def:&StrategyDef)->Option<Policy> {
    if !jcarter_reviewed(def) {return None;}
    let mut p=requirement("index_etf_or_stock",&["index","etf","individual_stock"],"Mastering the Trade, 3rd ed. (McGraw-Hill 2018): The Squeeze Ch.11 p.248-254, HOLP and LOHP Ch.17 p.349-358, Propulsion Plays Ch.18 p.373-378. Index/ETF/stock runs are a declared application.");
    p.preferred_basket=Some("nifty50");p.market_adaptation=true;Some(p)
}
// ---------------------------------------------------------------- end Carter
// ---------------------------------------------------------------- Bryant 2026-10-25
// Admission only for the module REVIEWED lists (authors/bryant/admission/SOURCE-CONTRACT.md); kept in one block to merge.
pub fn bryant_reviewed(def:&StrategyDef)->bool {(def.book=="bfmm" && crate::books::bfmm::reviewed(def.id))}
const BRYANT_LABEL:&str="Michael R. Bryant built and tested the MiniMax systems on daily E-mini S&P 500 and E-mini Nasdaq 100 futures; runs on indices, ETFs and single stocks (US and India) are a declared application.";
fn bryant_policy(def:&StrategyDef)->Option<Policy> {
    if !bryant_reviewed(def) {return None;}
    let mut p=requirement("index_etf_or_stock",&["index","etf","individual_stock"],"User's Guide: MiniMax Trading Systems (Breakout Futures 2001) ch.2 p.10-11 and Appendix p.37-38 (ESL, NQL, S); User's Guide: MiniMax II System v2 (2002) ch.1 p.4 and ch.2 p.10-11. Index/ETF/stock runs are a declared application.");
    p.preferred_basket=Some("nifty50");p.market_adaptation=true;Some(p)
}
// ---------------------------------------------------------------- end Bryant
// ---------------------------------------------------------------- Gettess 2026-10-25
// Admission only for the module REVIEWED lists (authors/gettess/admission/SOURCE-CONTRACT.md); kept in one block to merge.
pub fn gettess_reviewed(def:&StrategyDef)->bool {(def.book=="lg3k" && crate::books::lg3k::reviewed(def.id)) || (def.book=="lgtt30" && crate::books::lgtt30::reviewed(def.id)) || (def.book=="lgsc" && crate::books::lgsc::reviewed(def.id))}
const GETTESS_LABEL:&str="Lee E. Gettess tested these volatility systems on T-Bond and S&P 500 futures; runs on indices, ETFs and single stocks (US and India) are a declared application.";
fn gettess_policy(def:&StrategyDef)->Option<Policy> {
    if !gettess_reviewed(def) {return None;}
    let mut p=requirement("index_etf_or_stock",&["index","etf","individual_stock"],"The $3000 Secret (Windsor Books, 1991): volatility reversal p.5-11 (4-day actual range 45%, 4-day true range 55%) and S&P 2-day range 50% p.16-19; TT30 with Market Mapping (1994): TT30 System Writer Plus code p.9-10, rules p.2-6; Trading in the Trenches (1993-95): SS-TDA entry p.5 with the user's standard exits (labelled variant, not from the book). Index/ETF/stock runs are a declared application.");
    p.preferred_basket=Some("nifty50");p.market_adaptation=true;Some(p)
}
// ---------------------------------------------------------------- end Gettess
// ---------------------------------------------------------------- Primo 2026-10-25
// Admission only for the module REVIEWED lists (authors/primo/admission/SOURCE-CONTRACT.md); kept in one block to merge.
pub fn primo_reviewed(def:&StrategyDef)->bool {(def.book=="pstc" && crate::books::pstc::reviewed(def.id)) || (def.book=="pstc08" && crate::books::pstc08::reviewed(def.id)) || (def.book=="prap2" && crate::books::prap2::reviewed(def.id))}
const PRIMO_LABEL:&str="Steve Primo's courses tested these rules on $SPX/$NDX/$SOX, SPY and US stocks (S&P 100 / Nasdaq 100 / the whole US stock universe); runs on other indices, ETFs and single stocks (US and India) are a declared application.";
fn primo_policy(def:&StrategyDef)->Option<Policy> {
    if !primo_reviewed(def) {return None;}
    let mut p=requirement("index_etf_or_stock",&["index","etf","individual_stock"],"Professional Swing Trading College 2005 (Week 2-7 notes, Other version Session 1: RS2, R3, RSI 20, R4, 4/10/60); Swing Trading College 2008 (Session 3 slides 8-18 R4/R5, Session 4 slides 7-13 RS Meteors, Session 7 slides 16-21 RSI 25 scale-in 20); Raptor-II Strategy Guidebook (April 2006) p.5 rules 1-9 + Raptor2_ClassScan.afl. Index/ETF/stock runs are a declared application.");
    p.preferred_basket=Some("nifty50");p.market_adaptation=true;Some(p)
}
// ---------------------------------------------------------------- end Primo
// ---------------------------------------------------------------- Meyers 2026-10-25
// Admission only for the module REVIEWED lists (authors/meyers/admission/SOURCE-CONTRACT.md); kept in one block to merge.
pub fn meyers_reviewed(def:&StrategyDef)->bool {(def.book=="dmast" && crate::books::dmast::reviewed(def.id)) || (def.book=="dmst2" && crate::books::dmst2::reviewed(def.id))}
const MEYERS_LABEL:&str="Dennis Meyers tested these stop-and-reverse systems on single futures contracts (British pound, yen, D-mark, US bonds, gold, S&P 500); runs on indices, ETFs and single stocks are a declared application (one unit, percentage thresholds unchanged).";
fn meyers_policy(def:&StrategyDef)->Option<Policy> {
    if !meyers_reviewed(def) {return None;}
    let mut p=requirement("index_etf_or_stock",&["index","etf","individual_stock"],"Advanced Short Term Systems & Indicators v1.01 (ASTMAN10): Fixed Memory Polynomial III-11, Fading Memory Polynomial II V-8, Kalman g-h VI-9, g-h-k VII-9, Fading Memory Kalman VIII-10; Short Term Systems & Indicators v2.0 (STMANUAL): xAvTR V-10, Least Squares t+1 VII-9, T**2 VIII-9, Polychromatic Momentum IX-9, Maximum Likelihood Range X-9, %Up-%Down XI-5. Printed TradeStation 4 code; index/ETF/stock runs are a declared application.");
    p.preferred_basket=Some("nifty50");p.market_adaptation=true;Some(p)
}
// ---------------------------------------------------------------- end Meyers
// ---------------------------------------------------------------- Krutsinger 2026-10-25
// Admission only for the module REVIEWED lists (authors/krutsinger/admission/SOURCE-CONTRACT.md); kept in one block to merge.
pub fn krutsinger_reviewed(def:&StrategyDef)->bool {(def.book=="jkbwts" && crate::books::jkbwts::reviewed(def.id)) || (def.book=="jkbk" && crate::books::jkbk::reviewed(def.id)) || (def.book=="upg18" && crate::books::upg18::reviewed(def.id))}
const KRUTSINGER_LABEL:&str="Joe Krutsinger built and reported these systems on S&P 500 futures (Dinapoli5, RainMaker, JK-Call Buyer), stock-index futures (SAFE) and QQQ / any market (Prince, English Channel, Parrot); runs on other indices, ETFs and single stocks (US and India) are a declared application.";
fn krutsinger_policy(def:&StrategyDef)->Option<Policy> {
    if !krutsinger_reviewed(def) {return None;}
    let mut p=requirement("index_etf_or_stock",&["index","etf","individual_stock"],"Building Winning Trading Systems (1997): Dinapoli5 p.5, JK RainMaker p.37; joe_krut.pdf: SAFE worksheet p.22, JK-Call Buyer p.34-35 (Active Trader Sept 2000); vendor code: English Channel (EnglishChannel.pdf, English_Channel.txt), Prince 2002 (Open Code.txt), Parrot (Parrot.txt). Index/ETF/stock runs are a declared application.");
    p.preferred_basket=Some("nifty50");p.market_adaptation=true;Some(p)
}
// ---------------------------------------------------------------- end Krutsinger
// ---------------------------------------------------------------- Clayburg 2026-10-25
// Admission only for the module REVIEWED lists (authors/clayburg/admission/SOURCE-CONTRACT.md); kept in one block to merge.
pub fn clayburg_reviewed(def:&StrategyDef)->bool {(def.book=="jcfeed" && crate::books::jcfeed::reviewed(def.id))}
const CLAYBURG_LABEL:&str="John Clayburg built and reported Feeder Trader on Feeder Cattle futures; runs on indices, ETFs and single stocks (US and India) are a declared application (range-fraction stops, no dollar or tick constants).";
fn clayburg_policy(def:&StrategyDef)->Option<Policy> {
    if !clayburg_reviewed(def) {return None;}
    let mut p=requirement("index_etf_or_stock",&["index","etf","individual_stock"],"Feeder Trader (c) 1999, clayburg.com: Code.txt (Code.rar), Feeder_Trader_setup.txt (defaults B 3, S 1, B1 6, S1 7; end-of-day, orders at tomorrow's open). Index/ETF/stock runs are a declared application.");
    p.preferred_basket=Some("nifty50");p.market_adaptation=true;Some(p)
}
// ---------------------------------------------------------------- end Clayburg
// ---------------------------------------------------------------- Stendahl 2026-10-25
// Admission only for the module REVIEWED lists (authors/stendahl/admission/SOURCE-CONTRACT.md); kept in one block to merge.
pub fn stendahl_reviewed(def:&StrategyDef)->bool {(def.book=="rinadz" && crate::books::rinadz::reviewed(def.id)) || (def.book=="hwdti" && crate::books::hwdti::reviewed(def.id))}
const STENDAHL_LABEL:&str="Zamansky & Stendahl tested the Dynamic Zone systems on the S&P 500 (cash index and futures); Helweg & Stendahl tested the moving-average system on Treasury bond futures; runs on other indices, ETFs and single stocks (US and India) are a declared application.";
fn stendahl_policy(def:&StrategyDef)->Option<Policy> {
    if !stendahl_reviewed(def) {return None;}
    let mut p=requirement("index_etf_or_stock",&["index","etf","individual_stock"],"Zamansky & Stendahl, Dynamic Zones, S&C V15:7 (1997) p.306-310 and the RINA Dynamic Zone User's Guide pp.10-21 (DZ RSI(9), fixed RSI 30/70, smoothed RSI(14), %R, Extreme MACD); Helweg & Stendahl, Dynamic Trading Indicators (2002) Figure 6.4 p.130 (WMA20 x SMA56 stop-and-reverse). Index/ETF/stock runs are a declared application.");
    p.preferred_basket=Some("nifty50");p.market_adaptation=true;Some(p)
}
// ---------------------------------------------------------------- end Stendahl
// ---------------------------------------------------------------- Boucher 2026-10-25
// Admission only for the module REVIEWED lists (authors/boucher/admission/SOURCE-CONTRACT.md); kept in one block to merge.
pub fn boucher_reviewed(def:&StrategyDef)->bool {(def.book=="mbhfe" && crate::books::mbhfe::reviewed(def.id)) || (def.book=="mbsot" && crate::books::mbsot::reviewed(def.id))}
const BOUCHER_LABEL:&str="Mark Boucher tested these timing models on the S&P 500 (breadth thrusts, Renshaw, Appel) and the Value Line Composite (Zweig 4%) with NYSE breadth; the _nse versions read NSE breadth for India. Runs on other indices, ETFs and single stocks are a declared application.";
fn boucher_policy(def:&StrategyDef)->Option<Policy> {
    if !boucher_reviewed(def) {return None;}
    let mut p=requirement("index_etf_or_stock",&["index","etf","individual_stock"],"The Hedge Fund Edge (1999) Ch.4 p.136-139, System Spreadsheets 4.2 (up volume > 77%) and 4.3 (11-day A/D > 1.9); The Science of Trading workbook, Timing Models pp.19-22 (Zweig 4% weekly, Renshaw thrust, Appel Impulse, Appel McClellan, Appel Combo). Index/ETF/stock runs are a declared application.");
    p.preferred_basket=Some("nifty50");p.market_adaptation=true;Some(p)
}
// ---------------------------------------------------------------- end Boucher
// ---------------------------------------------------------------- Freeburg 2026-10-25
// Admission only for the module REVIEWED lists (authors/freeburg/admission/SOURCE-CONTRACT.md); kept in one block to merge.
pub fn freeburg_reviewed(def:&StrategyDef)->bool {(def.book=="nfpk" && crate::books::nfpk::reviewed(def.id))}
const FREEBURG_LABEL:&str="Nelson Freeburg illustrated these weekly moving-average confirmations on the S&P 500 and the NASDAQ Composite; runs on other indices, ETFs and single stocks (US and India) are a declared application.";
fn freeburg_policy(def:&StrategyDef)->Option<Policy> {
    if !freeburg_reviewed(def) {return None;}
    let mut p=requirement("index_etf_or_stock",&["index","etf","individual_stock"],"Trading the Pankin Strategy for 30% Annual Gains and Low Risk (TAG XIX 1997 workbook) Fig. 35 (S&P vs 20-week MA, unfiltered and +/-1%) and Fig. 36 (NASDAQ vs 15-week WMA, crossover and 0.5% MA swing), p.25. Index/ETF/stock runs are a declared application.");
    p.preferred_basket=Some("nifty50");p.market_adaptation=true;Some(p)
}
// ---------------------------------------------------------------- end Freeburg
// ---------------------------------------------------------------- RJones 2026-10-25
// Admission only for the module REVIEWED lists (authors/rjones/admission/SOURCE-CONTRACT.md); kept in one block to merge.
pub fn rjones_reviewed(def:&StrategyDef)->bool {(def.book=="rjmil" && crate::books::rjmil::reviewed(def.id))}
const RJONES_LABEL:&str="Ryan Jones tested these systems on futures (crude oil, T-bonds, wheat, soybeans, 10-year notes); runs on indices, ETFs and single stocks (US and India) are a declared application.";
fn rjones_policy(def:&StrategyDef)->Option<Policy> {
    if !rjones_reviewed(def) {return None;}
    let mut p=requirement("index_etf_or_stock",&["index","etf","individual_stock"],"The Secret to Making $1 Million in 5 Years or Less, Advanced Course workbook (Futures Learning Center 1998), Session 8: MA-MA Divergence (code p.55), FutureVideo (code p.60). Index/ETF/stock runs are a declared application.");
    p.preferred_basket=Some("nifty50");p.market_adaptation=true;Some(p)
}
// ---------------------------------------------------------------- end RJones
// ---------------------------------------------------------------- Blau 2026-10-25
// Admission only for the module REVIEWED lists (authors/blau/admission/SOURCE-CONTRACT.md); kept in one block to merge.
pub fn blau_reviewed(def:&StrategyDef)->bool {(def.book=="wbmdd" && crate::books::wbmdd::reviewed(def.id))}
const BLAU_LABEL:&str="Blau's Ergodic systems were shown on daily Deutsche mark futures charts (no printed results); runs on indices, ETFs and stocks are a declared application.";
fn blau_policy(def:&StrategyDef)->Option<Policy> {
    if !blau_reviewed(def) {return None;}
    let mut p=requirement("index_etf_or_stock",&["index","etf","individual_stock"],"Momentum, Direction, and Divergence (Wiley 1995) Ch.2 p.18-24: Ergodic = TSI(close,r,5) with a 5-day EMA Signal Line, trend = 32-day EMA of the close or slow TSI(close,64,64); Systems 1-3. Book market: Deutsche mark futures. Index/ETF/stock runs are a declared application.");
    p.preferred_basket=Some("nifty50");p.market_adaptation=true;Some(p)
}
// ---------------------------------------------------------------- end Blau
// ---------------------------------------------------------------- Faith 2026-10-25
// Admission only for the module REVIEWED lists (authors/faith/admission/SOURCE-CONTRACT.md); kept in one block to merge.
pub fn faith_reviewed(def:&StrategyDef)->bool {(def.book=="covel" && crate::books::covel::reviewed(def.id))}
const FAITH_LABEL:&str="Curtis Faith tested these six systems on a 27-market US futures portfolio (Trading Blox, 1996-2006); runs on one index, ETF or stock at a time are a declared application.";
fn faith_policy(def:&StrategyDef)->Option<Policy> {
    if !faith_reviewed(def) {return None;}
    let mut p=requirement("index_etf_or_stock",&["index","etf","individual_stock"],"Way of the Turtle (McGraw-Hill 2007) ch.10 p.131-142: ATR Channel Breakout, Bollinger Breakout, Donchian Trend, Donchian Trend with Time Exit, Dual and Triple Moving Average; 0.5% of equity per ATR (p.136). Book market: US futures portfolio. Index/ETF/stock runs are a declared application.");
    p.preferred_basket=Some("nifty50");p.market_adaptation=true;Some(p)
}
// ---------------------------------------------------------------- end Faith
// ---------------------------------------------------------------- Dunnigan 2026-10-25
// Admission only for the module REVIEWED lists (authors/dunnigan/admission/SOURCE-CONTRACT.md); kept in one block to merge.
pub fn dunnigan_reviewed(def:&StrategyDef)->bool {(def.book=="wdnbp" && crate::books::wdnbp::reviewed(def.id)) || (def.book=="wdgig" && crate::books::wdgig::reviewed(def.id))}
const DUNNIGAN_LABEL:&str="Dunnigan applied the New Dow Barometer to the Dow-Jones industrial average (1921-1956) and the 2 1/2%-3-day swings to wheat futures (1930-1942); runs on other indices, ETFs and single stocks are a declared application.";
fn dunnigan_policy(def:&StrategyDef)->Option<Policy> {
    if !dunnigan_reviewed(def) {return None;}
    let mut p=requirement("index_etf_or_stock",&["index","etf","individual_stock"],"New Blueprints for Gains in Stocks and Grains (1956): New Dow Barometer Rules 1-2 on 4% closing-price swings (Part III p.62-64); 2 1/2%-3-day swing penetration of 1% (Gains in Grains p.113-115, Fig 4.3). All thresholds are percentages. Index/ETF/stock runs are a declared application.");
    p.preferred_basket=Some("nifty50");p.market_adaptation=true;Some(p)
}
// ---------------------------------------------------------------- end Dunnigan
// ---------------------------------------------------------------- Katsanos 2026-10-25
// Admission only for the module REVIEWED lists (authors/katsanos/admission/SOURCE-CONTRACT.md); kept in one block to merge.
pub fn katsanos_reviewed(def:&StrategyDef)->bool {(def.book=="mkits" && crate::books::mkits::reviewed(def.id)) || (def.book=="mkfve" && crate::books::mkfve::reviewed(def.id))}
const KATSANOS_LABEL:&str="Katsanos tested these rules on DAX futures, EUR/USD and one US stock (XM Satellite Radio); runs on indices, ETFs and other stocks are a declared application.";
fn katsanos_policy(def:&StrategyDef)->Option<Policy> {
    if !katsanos_reviewed(def) {return None;}
    let mut p=requirement("index_etf_or_stock",&["index","etf","individual_stock"],"Intermarket Trading Strategies (Wiley 2008): DAX MA Crossover System (App. A.4, Table 13.2 p.210) and the Euro classic MA system (eq. 17.1-17.8 p.281-282, code p.355 without its CRB/TNX line); Detecting Breakouts (S&C April 2003) FVE divergence with linear-regression slopes (p.29, 36). Book markets: DAX futures, EUR/USD, XMSR stock. Index/ETF/stock runs are a declared application.");
    p.preferred_basket=Some("nifty50");p.market_adaptation=true;Some(p)
}
// ---------------------------------------------------------------- end Katsanos
// ---------------------------------------------------------------- Tomasini 2026-10-25
// Admission only for the module REVIEWED lists (authors/tomasini/admission/SOURCE-CONTRACT.md); kept in one block to merge.
pub fn tomasini_reviewed(def:&StrategyDef)->bool {(def.book=="jt" && crate::books::jt::reviewed(def.id))}
const TOMASINI_LABEL:&str="Jaekle & Tomasini tested these systems on futures, GBP/USD, stock indices (BOM) and Dow/S&P 500 stocks; runs on other indices, ETFs and single stocks are a declared application.";
fn tomasini_policy(def:&StrategyDef)->Option<Policy> {
    if !tomasini_reviewed(def) {return None;}
    let mut p=requirement("index_etf_or_stock",&["index","etf","individual_stock"],"Trading Systems, 2nd ed. (Harriman House 2019): 20-day breakout pseudo code (PDF p.24), LUXOR daily 7/26 (p.57-58, App. 3 p.375), Bollinger counter-breakout FOREX (Fig 5.4B p.129), Beginning of Month (p.159), Bollinger 60/2 futures (App. 1.2 p.358), single-stock Bollinger 100/+3/-1 (ch 9.2, p.240, 393). Index/ETF/stock runs are a declared application.");
    p.preferred_basket=Some("nifty50");p.market_adaptation=true;Some(p)
}
// ---------------------------------------------------------------- end Tomasini
// ---------------------------------------------------------------- Pardo 2026-10-25
// Admission only for the module REVIEWED lists (authors/pardo/admission/SOURCE-CONTRACT.md); kept in one block to merge.
pub fn pardo_reviewed(def:&StrategyDef)->bool {(def.book=="rpdt" && crate::books::rpdt::reviewed(def.id)) || (def.book=="rpeo" && crate::books::rpeo::reviewed(def.id))}
const PARDO_LABEL:&str="Robert Pardo gives these example systems for futures (S&P 500, Swiss franc, T-bonds and others); runs on indices, ETFs and stocks, US and India, are a declared application, not the author's test.";
fn pardo_policy(def:&StrategyDef)->Option<Policy> {
    if !pardo_reviewed(def) {return None;}
    let mut p=requirement("index_etf_or_stock",&["index","etf","individual_stock"],"Robert Pardo, Design, Testing and Optimization of Trading Systems (1992): 5/20 moving-average crossover (p.13-16), 125% of yesterday's range breakout (p.9), 3-day MA vs 12-day midpoint MA with a 2-day time filter (p.85-87); The Evaluation and Optimization of Trading Strategies (2008): EOTS-MA 10/30 code (p.151-153), 125%/145% range breakout (p.146). Reviewed in authors/pardo/admission/SOURCE-CONTRACT.md with independent chronological ledgers; no costs.");
    p.preferred_basket=Some("nifty50");p.market_adaptation=true;Some(p)
}
// ---------------------------------------------------------------- end Pardo
// ---------------------------------------------------------------- Aan 2026-10-25
// Admission only for the module REVIEWED lists (authors/aan/admission/SOURCE-CONTRACT.md); kept in one block to merge.
pub fn aan_reviewed(def:&StrategyDef)->bool {(def.book=="aanwr" && crate::books::aanwr::reviewed(def.id))}
const AAN_LABEL:&str="Peter W. Aan tested the Weekly Rule on 18 futures markets (S&P 500 among them); runs on indices, ETFs and stocks, US and India, are a declared application, not the author's test.";
fn aan_policy(def:&StrategyDef)->Option<Policy> {
    if !aan_reviewed(def) {return None;}
    let mut p=requirement("index_etf_or_stock",&["index","etf","individual_stock"],"Peter W. Aan, The Weekly Rule System (Mystery System, 1993): always in, reverse one tick beyond the highest high / lowest low of the last 4 completed Monday-Friday calendar weeks (MS1 rules 1-2 and worksheet; first trade = first fresh breakout, MS2 option a). Reviewed in authors/aan/admission/SOURCE-CONTRACT.md with independent chronological ledgers; no costs.");
    p.preferred_basket=Some("nifty50");p.market_adaptation=true;Some(p)
}
// ---------------------------------------------------------------- end Aan
// ---------------------------------------------------------------- Lakonishok 2026-10-25
// Admission only for the module REVIEWED lists (authors/lakonishok/admission/SOURCE-CONTRACT.md); kept in one block to merge.
pub fn lakonishok_reviewed(def:&StrategyDef)->bool {(def.book=="bllstr" && crate::books::bllstr::reviewed(def.id))}
const LAKONISHOK_LABEL:&str="Brock, Lakonishok and LeBaron studied these rules on the Dow Jones Industrial Average closes 1897-1986; runs on other indices, ETFs and stocks, US and India, are a declared application, not the authors' test.";
fn lakonishok_policy(def:&StrategyDef)->Option<Policy> {
    if !lakonishok_reviewed(def) {return None;}
    let mut p=requirement("index_etf_or_stock",&["index","etf","individual_stock"],"Brock, Lakonishok and LeBaron (1992), Journal of Finance 47(5): variable-length moving average (p.1735-1738), fixed-length moving average and trading range break with ten-day holding periods and blocked signals (p.1736, 1742, fn 11 p.1740), signal-day close (eq. A6-A8 p.1759). The 24 printed specifications other than the two defaults reviewed on 15 Oct. Reviewed in authors/lakonishok/admission/SOURCE-CONTRACT.md with independent chronological ledgers; no costs.");
    p.preferred_basket=Some("nifty50");p.market_adaptation=true;Some(p)
}
// ---------------------------------------------------------------- end Lakonishok
// ---------------------------------------------------------------- Sullivan 2026-10-25
// Admission only for the module REVIEWED lists (authors/sullivan/admission/SOURCE-CONTRACT.md); kept in one block to merge.
pub fn sullivan_reviewed(def:&StrategyDef)->bool {(def.book=="stwds" && crate::books::stwds::reviewed(def.id))}
const SULLIVAN_LABEL:&str="Sullivan, Timmermann and White tested these rules on the Dow Jones Industrial Average closes 1897-1996 and S&P 500 futures 1984-1996; runs on other indices, ETFs and stocks, US and India, are a declared application, not the authors' test.";
fn sullivan_policy(def:&StrategyDef)->Option<Policy> {
    if !sullivan_reviewed(def) {return None;}
    let mut p=requirement("index_etf_or_stock",&["index","etf","individual_stock"],"Sullivan, Timmermann and White, Data-Snooping, Technical Trading Rule Performance, and the Bootstrap (UCSD 97-31, 1997; J. Finance 1999): the best rules named in Tables I-II and VI from the filter, moving-average, support and resistance, channel break-out and on-balance volume families (Sec. III p.10-13, Appendix 1 p.25-27), signal-day close. Reviewed in authors/sullivan/admission/SOURCE-CONTRACT.md with independent chronological ledgers; no costs.");
    p.preferred_basket=Some("nifty50");p.market_adaptation=true;Some(p)
}
// ---------------------------------------------------------------- end Sullivan
// ---------------------------------------------------------------- Campbell 2026-10-25
// Admission only for the module REVIEWED lists (authors/campbell/admission/SOURCE-CONTRACT.md); kept in one block to merge.
pub fn campbell_reviewed(def:&StrategyDef)->bool {(def.book=="sctw" && crate::books::sctw::reviewed(def.id))}
const CAMPBELL_LABEL:&str="Shay Campbell tested Trend Weaver on a 37-market futures portfolio (1980-2006); runs on indices, ETFs and stocks, US and India, are a declared application, not the author's test.";
fn campbell_policy(def:&StrategyDef)->Option<Policy> {
    if !campbell_reviewed(def) {return None;}
    let mut p=requirement("index_etf_or_stock",&["index","etf","individual_stock"],"Shay Campbell, Trend Weaver Trading System (Timing Charts, 2006): 150-day trend with reversal stops 1% inside the 150-day range, entries at 90%/10% and exits at 40%/60% of the 20-day range above the 20-day low (Trading Rules p.3-9; the system's TradeStation code wins where the prose differs). Reviewed in authors/campbell/admission/SOURCE-CONTRACT.md with independent chronological ledgers; no costs.");
    p.preferred_basket=Some("nifty50");p.market_adaptation=true;Some(p)
}
// ---------------------------------------------------------------- end Campbell
// ---------------------------------------------------------------- Heitkoetter 2026-10-25
// Admission only for the module REVIEWED lists (authors/heitkoetter/admission/SOURCE-CONTRACT.md); kept in one block to merge.
pub fn heitkoetter_reviewed(def:&StrategyDef)->bool {(def.book=="hkmmts" && crate::books::hkmmts::reviewed(def.id))}
const HEITKOETTER_LABEL:&str="Markus Heitkoetter tested this sample system intraday on e-mini S&P, DAX, e-mini Nasdaq, EuroStoxx and FTSE futures; runs on daily indices, ETFs and stocks, US and India, are a declared application, not the author's test.";
fn heitkoetter_policy(def:&StrategyDef)->Option<Policy> {
    if !heitkoetter_reviewed(def) {return None;}
    let mut p=requirement("index_etf_or_stock",&["index","etf","individual_stock"],"Markus Heitkoetter, How to Make Money with Trading Systems (Rockwell Trading, 2005): Bollinger Band (34, 2.5) fade - sell above the upper band, buy below the lower band, exit at the close of the same day (p.9-13) or at the close of the 2nd day after entry (p.14). Reviewed in authors/heitkoetter/admission/SOURCE-CONTRACT.md with independent chronological ledgers; no costs.");
    p.preferred_basket=Some("nifty50");p.market_adaptation=true;Some(p)
}
// ---------------------------------------------------------------- end Heitkoetter
// ---------------------------------------------------------------- Dorsey 2026-10-25
// Admission only for the module REVIEWED lists (authors/dorsey/admission/SOURCE-CONTRACT.md); kept in one block to merge.
pub fn dorsey_reviewed(def:&StrategyDef)->bool {(def.book=="ddrvi" && crate::books::ddrvi::reviewed(def.id))}
const DORSEY_LABEL:&str="Donald Dorsey tested the inertia system on 13 futures markets (10-20 years of daily data to about 1994); runs on indices, ETFs and stocks, US and India, are a declared application, not the author's test.";
fn dorsey_policy(def:&StrategyDef)->Option<Policy> {
    if !dorsey_reviewed(def) {return None;}
    let mut p=requirement("index_etf_or_stock",&["index","etf","individual_stock"],"Donald Dorsey, Refining the Relative Volatility Index (Stocks & Commodities V13, 1995, p.388-391): new RVI = (RVIH + RVIL)/2 from the Fig.4 TradeStation code, inertia = 20-bar average; buy when inertia is above 50, sell when below 50 (Fig.5 text). Reviewed in authors/dorsey/admission/SOURCE-CONTRACT.md with independent chronological ledgers; no costs.");
    p.preferred_basket=Some("nifty50");p.market_adaptation=true;Some(p)
}
// ---------------------------------------------------------------- end Dorsey
// ---------------------------------------------------------------- Morris 2026-10-25
// Admission only for the module REVIEWED lists (authors/morris/admission/SOURCE-CONTRACT.md); kept in one block to merge.
pub fn morris_reviewed(def:&StrategyDef)->bool {(def.book=="msws" && crate::books::msws::reviewed(def.id))}
const MORRIS_LABEL:&str="Gregory L. Morris wrote these MetaStock systems (GM1-GM9, partly his codings of Stocks & Commodities articles) for stocks and futures without a named test market; runs on indices, ETFs and stocks, US and India, are a declared application, not the author's test.";
fn morris_policy(def:&StrategyDef)->Option<Policy> {
    if !morris_reviewed(def) {return None;}
    let mut p=requirement("index_etf_or_stock",&["index","etf","individual_stock"],"Gregory L. Morris, Indicators & Trading Systems GM1-GM9 (Wallstreet Trade Systems & Indicators for MetaStock): the printed Enter/Close Long/Short formulas with the series' own Fml() indicator definitions, MetaStock System Tester defaults (trade price = close, trade delay = 0). Reviewed in authors/morris/admission/SOURCE-CONTRACT.md with an independent MetaStock-formula reference and chronological ledgers; no costs.");
    p.preferred_basket=Some("nifty50");p.market_adaptation=true;Some(p)
}
// ---------------------------------------------------------------- end Morris
// ---- Joe Ross block (authors/ross admission, 2026-10-23) ----
/// Reviewed Joe Ross daily rules (books::joe_ross_rules::REVIEWED): actual indices, ETFs and stocks, US and India, as a
/// labelled declared application. The source references in books::joe_ross stay held (source review).
fn joe_ross_policy(def:&StrategyDef)->Option<Policy> {
    use crate::books::joe_ross_rules as jr;
    if !def.book.starts_with("jr_") || !jr::reviewed(def.id) {return None;}
    let basis=match def.book {
        jr::ZULU=>"Joe Ross, ZULU Trading Method (course package, pp.11-13): the author trades CBOT soybean futures on daily bars. Actual index, ETF and stock runs (US and India) are a declared application, not the author's test. The 32/16-tick target/stop use 1 tick = 0.04% of the previous close, the book's own soybean ratio (pp.14-16); 0.01% / the real stock tick is a labelled option [declared convention]. Cash-index shorts are hypothetical exposure. No futures dollars or book statistics are reproduced.",
        jr::AMBUSH=>"Joe Ross, AMBUSH Trading Method (course package, pp.9-11): the author trades CBOT wheat and corn futures on daily bars. Actual index, ETF and stock runs (US and India) are a declared application, not the author's test. The 30/40-tick target/stop use 1 tick = 0.1% of the previous close, the book's own wheat/corn ratio (pp.12-17); 0.01% / the real stock tick is a labelled option [declared convention]. Cash-index shorts are hypothetical exposure. No futures dollars or book statistics are reproduced.",
        jr::TBTB=>"Joe Ross, Trading by the Book Part V ledge rules (pp.238-246): the author trades US futures on daily bars. Actual index, ETF and stock runs (US and India) are a declared application, not the author's test. Tick offsets and the 1-2 tick match use 1 tick = 0.015% of the previous close, the book's S&P ratio (p.246); 0.01% / the real stock tick and a wider %-of-price match are labelled options; the swing-leg reading of Rules 6 and 10 is a declared convention. Cash-index shorts are hypothetical exposure.",
        jr::TNT1=>"Joe Ross, Electronic Trading TNT I (simple count, pp.230-232): shown on intraday stock charts and said to apply to daily charts. Daily actual stock runs and actual index/ETF runs (US and India) are a declared application, not the author's test. Ticks: tick_pct % of the previous close (auto: real tick on a stock). Cash-index shorts are hypothetical exposure.",
        jr::TNT3=>"Joe Ross, Electronic Trading TNT III (Bollinger Bands Method #2, pp.218-220): shown on an intraday Microsoft chart. Daily actual stock runs and actual index/ETF runs (US and India) are a declared application, not the author's test. Ticks: tick_pct % of the previous close (auto: real tick on a stock). Cash-index shorts are hypothetical exposure.",
        _=>"Joe Ross, The Law of Charts (1-2-3 and Ross Hook definitions) with the initial stops of Trading by the Book (pp.80, 375): the author trades futures (and stocks in the TNT series). The book gives no complete exit, so the user's standard exits are labelled [Not from the book]. Actual index, ETF and stock runs (US and India) are a declared application, not the author's test. Ticks: tick_pct % of the previous close (auto: real tick on a stock). Cash-index shorts are hypothetical exposure.",
    };
    let mut p=requirement("index_etf_or_stock",&["index","etf","individual_stock"],basis);
    p.preferred_basket=Some("nifty50");p.market_adaptation=true;
    Some(p)
}
// ---- end Joe Ross block ----
fn base_policy(def:&StrategyDef)->Policy {
    if intraday_p1(def) && intraday_w1_spx_id(def.id) {let mut p=requirement("declared_index_timing", &["index"], INTRADAY_W1_SPX_BASIS);p.market_adaptation=true;return p;} // intraday wave 1
    if intraday_p1(def) {let mut p=requirement("declared_index_timing", &["index","etf"], INTRADAY_P1_BASIS);p.market_adaptation=true;return p;} // intraday phase 1
    if options_p2(def) {let mut p=requirement("options_underlying", &["index","etf"], OPTIONS_P2_BASIS);p.market_adaptation=true;return p;} // options phase 2
    if let Some(p)=connors_final_policy(def) {return p;} // connors-final block
    if let Some(p)=lwilliams_policy(def) {return p;} // Larry Williams block
    // ---- Schwager block (2026-10-23) ----
    if let Some(p)=schwager_policy(def) {return p;}
    if let Some(p)=elder_policy(def) {return p;} // Elder block
    if let Some(p)=pring_policy(def) {return p;} // Pring block
    if let Some(p)=jobman_policy(def) {return p;} // Jobman block
    if let Some(p)=schaap_policy(def) {return p;} // Schaap block
    if let Some(p)=satchell_policy(def) {return p;} // Satchell block
    if let Some(p)=kirkpatrick_policy(def) {return p;} // Kirkpatrick block
    if let Some(p)=goslin_policy(def) {return p;} // Goslin block
    if let Some(p)=ruggiero_policy(def) {return p;} // Ruggiero block
    if let Some(p)=link_policy(def) {return p;} // Link block
    if let Some(p)=katz_policy(def) {return p;} // Katz block
    if let Some(p)=crabel_policy(def) {return p;} // Crabel block
    if let Some(p)=stad_policy(def) {return p;} // STAD block
    if let Some(p)=masonson_policy(def) {return p;} // Leslie Masonson block
    if let Some(p)=collins_policy(def) {return p;} // Art Collins block
    if let Some(p)=guppy_policy(def) {return p;} // Guppy block
    if let Some(p)=headley_policy(def) {return p;} // Headley block
    if let Some(p)=jcarter_policy(def) {return p;} // Carter block
    if let Some(p)=bryant_policy(def) {return p;} // Bryant block
    if let Some(p)=gettess_policy(def) {return p;} // Gettess block
    if let Some(p)=primo_policy(def) {return p;} // Primo block
    if let Some(p)=meyers_policy(def) {return p;} // Meyers block
    if let Some(p)=krutsinger_policy(def) {return p;} // Krutsinger block
    if let Some(p)=clayburg_policy(def) {return p;} // Clayburg block
    if let Some(p)=stendahl_policy(def) {return p;} // Stendahl block
    if let Some(p)=boucher_policy(def) {return p;} // Boucher block
    if let Some(p)=freeburg_policy(def) {return p;} // Freeburg block
    if let Some(p)=rjones_policy(def) {return p;} // RJones block
    if let Some(p)=blau_policy(def) {return p;} // Blau block
    if let Some(p)=faith_policy(def) {return p;} // Faith block
    if let Some(p)=dunnigan_policy(def) {return p;} // Dunnigan block
    if let Some(p)=katsanos_policy(def) {return p;} // Katsanos block
    if let Some(p)=tomasini_policy(def) {return p;} // Tomasini block
    if let Some(p)=pardo_policy(def) {return p;} // Pardo block
    if let Some(p)=aan_policy(def) {return p;} // Aan block
    if let Some(p)=lakonishok_policy(def) {return p;} // Lakonishok block
    if let Some(p)=sullivan_policy(def) {return p;} // Sullivan block
    if let Some(p)=campbell_policy(def) {return p;} // Campbell block
    if let Some(p)=heitkoetter_policy(def) {return p;} // Heitkoetter block
    if let Some(p)=dorsey_policy(def) {return p;} // Dorsey block
    if let Some(p)=morris_policy(def) {return p;} // Morris block
    if let Some(p)=bwilliams_policy(def) {return p;} // BWilliams block
    if let Some(p)=stridsman_policy(def) {return p;} // Stridsman block
    if let Some(p)=weissman_policy(def) {return p;} // Weissman block
    if let Some(p)=raschke_policy(def) {return p;} // Raschke block
    if let Some(p)=carnold_policy(def) {return p;} // CArnold block
    if let Some(p)=ndavis_policy(def) {return p;} // NDavis block
    if let Some(p)=siegel_policy(def) {return p;} // Siegel block
    if let Some(p)=lydon_policy(def) {return p;} // Lydon block
    if let Some(p)=landry_policy(def) {return p;} // Landry block
    if let Some(p)=pruitt_policy(def) {return p;} // Pruitt block
    if let Some(p)=hirsch_policy(def) {return p;} // Hirsch block
    if let Some(p)=bandy_policy(def) {return p;} // Bandy block
    if let Some(p)=kestner_policy(def) {return p;} // Kestner block
    if let Some(p)=wirrick_policy(def) {return p;} // Wirrick block
    if let Some(p)=sperandeo_policy(def) {return p;} // Sperandeo block
    if let Some(p)=csmith_policy(def) {return p;} // CSmith block
    if let Some(p)=lien_policy(def) {return p;} // Lien block
    if let Some(p)=arms_policy(def) {return p;} // Arms block
    if let Some(p)=gregoriou_policy(def) {return p;} // Gregoriou block
    if let Some(p)=crisp_policy(def) {return p;} // Crisp block
    if let Some(p)=fabozzi_policy(def) {return p;} // Fabozzi block
    if let Some(p)=fontanills_policy(def) {return p;} // Fontanills block
    if let Some(p)=thomsett_policy(def) {return p;} // Thomsett block
    if let Some(p)=patel_policy(def) {return p;} // Patel block
    if let Some(p)=demark_policy(def) {return p;} // DeMark block
    if let Some(p)=kroll_policy(def) {return p;} // Kroll block
    if let Some(p)=kaeppel_policy(def) {return p;} // Kaeppel block
    if let Some(p)=hamon_policy(def) {return p;} // Hamon block
    if let Some(p)=toghraie_policy(def) {return p;} // Toghraie block
    if let Some(p)=harris_policy(def) {return p;} // Harris block
    if let Some(p)=etzkorn_policy(def) {return p;} // Etzkorn block
    if let Some(p)=lebeau_policy(def) {return p;} // LeBeau block
    if let Some(p)=tharp_policy(def) {return p;} // Tharp block
    if let Some(p)=person_policy(def) {return p;} // Person block
    if let Some(p)=kaufman_policy(def) {return p;} // Kaufman block
    if let Some(p)=appel_policy(def) {return p;} // Appel block
    // ---- Joe Ross block (authors/ross admission, 2026-10-23) ----
    if let Some(p)=joe_ross_policy(def) {return p;}
    // ---- end Joe Ross block ----
    if jb_g3_reviewed(def) {
        if jb_g3_gap(def) {let mut p=stock(JBHSMS_GAP_BASIS);p.preferred_basket=Some("us_sp500");p.market_adaptation=true;return p;}
        if def.book==crate::books::jbeft::BOOK {let mut p=index(JBEFT_BASIS);p.market_adaptation=true;return p;}
        let mut p=requirement("index_etf_or_stock", &["index","etf","individual_stock"], if def.book==crate::books::jbsmsw::BOOK {JBSMSW_BASIS} else if def.book==crate::books::jbtywr::BOOK || def.book==crate::books::jbhpp::BOOK {JBTYWR_BASIS} else if def.book==crate::books::jbtifm::BOOK {JBTIFM_BASIS} else {JBHSMS_KD_BASIS});
        p.preferred_basket=Some("nifty50");p.market_adaptation=true;return p;
    }
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
    // ---- Bernstein g2 (30 Days / Ultimate Day Trader / Compleat Day Trader II), 22 Oct 2026
    if bernstein_g2_reviewed(def) {
        let mut p=if bernstein_g2_spx_points(def.id) {index(BERNSTEIN_G2_SPX_LABEL!())} else {requirement("index_etf_or_stock", &["index","etf","individual_stock"], BERNSTEIN_G2_ANY_LABEL!())};
        p.market_adaptation=true; return p;
    }
    // ---- end Bernstein g2
    if def.book==crate::books::ss::BOOK && crate::books::ss::reviewed_daily(def.id) {
        let stock=crate::books::ss::STOCK_SHOWN.contains(&def.id);
        let mut p=if stock {
            requirement("index_etf_or_stock", &["index","etf","individual_stock"], "Street Smarts (Raschke & Connors, 1995), daily rules reviewed in connors/ss-admission/SOURCE-CONTRACT.md with independent chronological ledgers. The book shows this setup on futures and on individual stocks (2-period ROC: Micron p.64; The Anti: 'all equities and futures markets' p.77; Three-Day Unfilled Gap: Motorola pp.99-101). Actual stocks are admitted; actual index/ETF runs are a declared application, not the authors' test. 1 tick = 0.01% of price on an index/ETF; on an actual stock the stock's real tick, 0.05 INR on NSE and 0.01 USD on US stocks (adaptation: the book used 1/8-dollar ticks). Exits the chapter leaves to judgement ('trail a stop') use the labelled 2-bar channel from Appendix p.232. No futures dollars or book performance is reproduced.")
        } else {
            requirement("declared_index_timing", &["index","etf"], "Street Smarts (Raschke & Connors, 1995), daily rules reviewed in connors/ss-admission/SOURCE-CONTRACT.md with independent chronological SPY/SPX/NIFTY ledgers. The book traded S&P, bond, currency and grain futures (Moore studies: 25 US futures 1986-1995). Actual SPX, NDX, DJI, NIFTY 50 and NIFTYBANK cash-index runs and actual SPY are a declared application; cash-index shorts are hypothetical exposure. 1 tick = 0.01% of price. Daily-bar fill conventions, the labelled 2-bar channel trail (Appendix p.232) and every [Not from the book] option are disclosed; NIFTY opens before late 2010 lie close to the previous close (official NSE data, kept). No futures dollars, contract rolls or book performance is reproduced.")
        };
        p.market_adaptation=true; return p;
    }
    if let Some(p)=bernstein_g1_policy(def) {return p;}
    if def.book==crate::books::cwin::BOOK && crate::books::cwin::reviewed_daily(def.id) {
        let mut p=requirement("index_etf_or_stock", &["index","etf","individual_stock"], concat!("The TradingWindows Strategy (Connors, 2002) and its course pages, rules reviewed in connors/cwin-admission/SOURCE-CONTRACT.md with independent chronological stock/index/ETF ledgers. ",CWIN_INDEX_LABEL!()," Profit taking is discretionary in the book (p31, p41 'there is no mechanical science to this'): the coded exit (1/3 at the first close beyond SMA4 then breakeven stop, 1/3 on a profitable day-2 close, rest at the day-3 close) is a declared mechanical reading of p31/p38/p56 and the course, not the author's exact exit; trailing stops beyond breakeven are not coded. The two-rule exit test is the book's own mechanical exit. On actual stocks the p56 Q&A volume suggestion (\"at least 100,000 shares traded on average each day\") is a default filter, averaged over 20 days [Not from the book] (min_avg_vol 0 = off); index/ETF runs have no volume filter. Rules that read the open start where SPX/NDX/DJI opens are real (2014/2001/2016); NIFTY opens before 2011 are close to the prior close and are disclosed. Cash-index shorts are hypothetical exposure. 1 unit, prices in points; no futures dollars or book performance are claimed."));
        p.market_adaptation=true; return p;
    }
    if def.book==crate::books::ccl08::BOOK && crate::books::ccl08::reviewed_daily(def.id) {
        let mut p=if crate::books::ccl08::day_trade(def.id) {
            requirement("index_etf_or_stock", &["index","etf","individual_stock"], concat!("Chairman's Club 2008 (TradingMarkets) CR Equity / CRDT Equity day trades, slide rules re-read and checked with independent chronological ledgers (connors/newrules-cc/SOURCE-CONTRACT.md). Limit entry X% beyond the setup close, exit at a 10% stop beyond the fill or at that day's close: exact on daily OHLC. The decks show US stocks (Bloomberg/TradeStation examples) and print only seminar-reported results (S2 s14: 15-25 Sep 2008), which are not reproduced. Actual US and NSE stocks run the rule as printed; the Liquidity Filter and HV formulas are vault definitions [Not from the book]. ",CCL08_INDEX_LABEL!()))
        } else {
            requirement("index_or_etf", &["index","etf"], "Chairman's Club 2008 (TradingMarkets) Session 2 TPS rules (s93, s95-s96), re-read from the slides and checked with independent chronological ledgers (connors/newrules-cc/SOURCE-CONTRACT.md). The deck lists S&P 500 TPS trades (s99); SPX and SPY are the deck's market, other indices/ETFs (NIFTY 50 included) are a declared application. S2 s100 adds 'This can be used high liquidity stocks'; stock runs are not admitted yet. Scale-in weights are shares of the full 1-2-3-4 (or 2-4-6-8) position; cash-index shorts are hypothetical exposure. No book performance is claimed.")
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
        let mut p=requirement("declared_index_timing", &["index","etf"], "Investment Secrets of a Hedge Fund Manager (Connors & Hayward, 1995) CHADTP (Ch.5) and The Best of the Professional Traders Journal: Options Trading and Volatility Trading (Connors, 1999) CVR I buys / sells and HV Thrusts, daily rules reviewed in connors/hfm-admission/SOURCE-CONTRACT.md with independent chronological ledgers; NDX-SPX (hfm Ch.6) reviewed in authors/connors-leftovers with an independent SPX/SPY/DJI ledger check (signal = NASDAQ 100 vs S&P 500 cash closes, traded at the next open on SPX, SPY or DJI; the book showed the DJIA too). The books traded S&P 500 futures (CHADTP on NYSE advancing/declining issues; CVR I on the OEX VIX, today's VXO). SPY uses actual ETF prices and the CBOE VIX; SPX cash-index timing is a declared application; the NSE-breadth CHADTP on NIFTY 50 (book's fixed +/-400 issues) and the HV Thrust on SPX/NDX/DJI/SPY/NIFTY 50/NIFTYBANK are declared adaptations. The CHADTP .10-point trigger is coded as 0.03% of price. The HV Thrust has no book exit, so the user's standard exits are labelled options [Not from the book]. Cash-index shorts are hypothetical exposure. No futures dollars or book performance is claimed.");
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
        t if US_ETFS.contains(&t) => "etf",
        _ => "unknown",
    }
}
pub fn check(def:&StrategyDef,ticker:&str,native_stock:bool)->Option<Issue> {
    let p=policy(def);let selected=selected_type(ticker,native_stock);
    if let Some(issue)=intraday_p1_symbol_issue(def,ticker,native_stock){return Some(issue);} // intraday phase 1
    if let Some(issue)=options_p2_symbol_issue(def,ticker,native_stock){return Some(issue);} // options phase 2
    if let Some(issue)=connors_final_symbol_issue(def,ticker,native_stock,native_stock.then_some(false)){return Some(issue);} // connors-final block
    if vix_india(def) {return vix_india_held(def,ticker,ticker.starts_with("NIFTY")).or_else(||check_selected(p,def.id,ticker,selected));} // VIX India block
    if open_all(def) {return open_all_symbol_issue(def,ticker,native_stock.then_some(false)).or_else(||check_selected(p,def.id,ticker,selected));} // open-all block
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
    if let Some(issue)=bernstein_g1_symbol_issue(def,ticker,native_stock){return Some(issue);}
    if let Some(issue)=bernstein_g2_symbol_issue(def,ticker,native_stock.then_some(false)){return Some(issue);}
    if let Some(issue)=jb_g3_symbol_issue(def,ticker){return Some(issue);}
    if let Some(issue)=lwilliams_symbol_issue(def,ticker,native_stock.then_some(false)){return Some(issue);} // Larry Williams block
    if let Some(issue)=kaeppel_symbol_issue(def,ticker,native_stock.then_some(false)){return Some(issue);} // Kaeppel block
    if let Some(issue)=schaap_symbol_issue(def,ticker,native_stock.then_some(false)){return Some(issue);} // Schaap block
    if let Some(issue)=landry_symbol_issue(def,ticker,native_stock.then_some(false)){return Some(issue);} // Landry block
    if let Some(issue)=hirsch_symbol_issue(def,ticker,native_stock.then_some(false)){return Some(issue);} // Hirsch block
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
    if let Some(issue)=intraday_p1_symbol_issue(def,ticker,true){return Some(issue);} // intraday phase 1
    if let Some(issue)=options_p2_symbol_issue(def,ticker,true){return Some(issue);} // options phase 2
    if let Some(issue)=connors_final_symbol_issue(def,ticker,true,Some(evidence.market()==Market::UnitedStates)){return Some(issue);} // connors-final block
    if vix_india(def) {return vix_india_held(def,ticker,evidence.market()==Market::India).or_else(||check_selected(p,def.id,ticker,evidence.asset_type()));} // VIX India block
    if open_all(def) {return open_all_symbol_issue(def,ticker,Some(evidence.market()==Market::UnitedStates)).or_else(||check_selected(p,def.id,ticker,evidence.asset_type()));} // open-all block
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
    if let Some(issue)=bernstein_g1_symbol_issue(def,ticker,evidence.asset_type()=="individual_stock"){return Some(issue);}
    if let Some(issue)=bernstein_g2_symbol_issue(def,ticker,Some(evidence.market()==Market::UnitedStates)){return Some(issue);}
    if let Some(issue)=jb_g3_symbol_issue(def,ticker){return Some(issue);}
    if let Some(issue)=lwilliams_symbol_issue(def,ticker,Some(evidence.market()==Market::UnitedStates)){return Some(issue);} // Larry Williams block
    if let Some(issue)=kaeppel_symbol_issue(def,ticker,Some(evidence.market()==Market::UnitedStates)){return Some(issue);} // Kaeppel block
    if let Some(issue)=schaap_symbol_issue(def,ticker,Some(evidence.market()==Market::UnitedStates)){return Some(issue);} // Schaap block
    if let Some(issue)=landry_symbol_issue(def,ticker,Some(evidence.market()==Market::UnitedStates)){return Some(issue);} // Landry block
    if let Some(issue)=hirsch_symbol_issue(def,ticker,Some(evidence.market()==Market::UnitedStates)){return Some(issue);} // Hirsch block
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
pub fn held_reason(id:&str)->Option<&'static str> {
    if intraday_p1_id(id) {return None;} // intraday phase 1: admitted on 1-minute bars
    if crate::options::is_option_rule(id) {return None;} // options phase 2: admitted on real option prices
    // intraday phase 1: minute data now exists for these, so the hold says what still blocks them
    match id {
        "spmt_dt_gapdown"|"spmt_dt_unch"|"spmt_dt_gapup"=>return Some("Held: needs NYSE TICK data. Real 15-minute bars are now built from 1-minute data (SPY 2000-2025, SPX 2008-2026, NIFTY 50 2008-2026), but these day trades are taken only on days the course's 30-signal Market Timing count gives a buy bias, and one of the 30 signals is the NYSE TICK, which the vault does not have; counting only the other signals changes the strategy."),
        "ss_smart_money_index"=>return Some("Held: rules not fully defined. The first-hour and last-hour prices are now built from real 1-minute bars, but the book uses the Smart Money Index as an indicator reading and gives no entry price, exit or stop for it."),
        "cwin_vix_gap_window"=>return Some("Held: waiting for the rebuilt India VIX daily file. India VIX and NIFTY 50 1-minute bars are now cleaned (2015-2026), but the rule compares today's VIX minutes with yesterday's VIX high and low, which must come from the same rebuilt daily India VIX that a separate review is producing. There is no intraday CBOE VIX, so US runs stay held."),
        _=>{}
    }
    Some(match id {
        // ---- Larry Williams block (23 Oct 2026): held Live rules
        "lwnl_july4_sell"|"lwnl_keyrev_sell"=>"Held: no stop in the newsletter. The only exit is the first profitable opening, so a short that never gets one stays open: on every index, ETF and stock tested the short ran for years and lost more than the whole account, which no account can carry.",
        "lwlts_mon_vb"=>"Held: needs intraday data. The trade enters on a stop above Monday's open and is stopped out or closed the same day; daily bars cannot show whether the stop or the entry came first, and the accuracy check found most of the gain comes from such same-day trades.",
        "lwdg2_0105"=>"Held: needs intraday data. The day trade enters on whichever of two stops (the close plus or minus 1.05%) is hit first and exits on the close; when a day reaches both, daily bars cannot show which came first, and the accuracy check found its whole gain comes from such same-day trades.",
        // ---- end Larry Williams block
        // ---- Crabel block (26 Oct 2026): held Live rules
        "tcdt_ut1"|"tcdt_ut2"|"tcdt_ut3"|"tcdt_ut4"|"tcdt_ut5"=>"Held: the book counts every upthrust, even when a new one comes while the last three-day short is still open, so its trades overlap; one account cannot hold overlapping trades of the same rule, and on several markets the vault therefore refuses the run. A version that skips signals while a trade is open would be a change to the book's test.",
        // ---- end Crabel block
        // ---- STAD block (26 Oct 2026): held Live rules
        "stad2_obv"=>"Held: the printed code buys 100 shares on a new OBV high and 200 shares on a new OBV low, so the second entry is twice the opening size; the vault trades one unit and refuses the larger position on every market tested, so no result can be published.",
        "stad2_stoch"=>"Held: the printed code opens with 200 shares and reverses with 400 shares, so every reversal is twice the opening size; the vault trades one unit and refuses the larger position on every market tested, so no result can be published.",
        "stad8_stampede"|"stad9_luxor"|"stad13_luxor"=>"Held: the strategy adds to a winning position (pyramiding up to several entries in the same direction); the vault carries one unit per strategy and refuses the added entries, so no result can be published.",
        // ---- end STAD block
        // ---- Masonson block (26 Oct 2026): held Live rules
        "masmt_vakkur_lev_seasonal"|"masmt_vakkur_lev_all"|"masmt_pres_lev2"|"masmt_pres_lbm"|"masmt_pres_optimal"=>"Held: the printed strategy invests with 2:1 margin (twice the account) in some months; the vault trades at most one unit without margin financing, so no result can be published.",
        // ---- end Masonson block
        // ---- Art Collins block (26 Oct 2026): held Live rules
        "acbff_forever_long"=>"Held: the profit target is a fixed $1,250 per S&P futures contract (5 S&P index points); a fixed point amount has no equivalent on other indices, ETFs or stocks, and the book's test window starts at the 1982 contract launch, before the vault's data, so no reviewed result can be published.",
        // ---- end Art Collins block
        // ---- Bernstein g2 held Live rules
        "jb30d_rev_sell_fpo"=>"Held: no stop in the book. The only exit is the first profitable opening, so a trade that never gets one stays open: on SPY a short entered in 1995 is still open in 2026 and has lost more than the whole account, which no account can carry. The book's own table shows losing trades this exit cannot produce, so its test used an exit it does not print.",
        "jbudt_gap_fpo"|"jbudt_gap_split"=>"Held: no stop in the book. The kept position leaves only on the first opening beyond the entry price, so a short that never gets one stays open: on SPY a short entered in January 1995 is still open in 2026 and has lost more than the whole account, which no account can carry.",
        "jb30d_gap_tick"|"jb30d_gap_pct"|"jbcdt_gap_go"=>"Held: needs intraday data. The trade enters on a stop and is stopped out or closed on the same day; daily bars cannot show whether the stop or the entry came first, and the accuracy check found all of its gain comes from such same-day trades.",
        // ---- end Bernstein g2
        // ---- Faith block (25 Oct 2026): held Live rules
        "cov_turtle_s1"|"cov_turtle_s2"=>"Held: needs leverage. The Original Turtle Rules size each unit at 1% of the account per N and add up to 4 units every 1/2 N; on one index, ETF or stock a unit is about half to almost all of the account (N is 1-2% of the price), so the second or third unit takes the position above 100% of the account, which the vault does not model (the Turtles traded futures on margin across many markets).",
        // ---- end Faith block
        // ---- Dunnigan block (25 Oct 2026): held Live rules
        "wdowf_plan"=>"Held: the engine cannot hold the plan's four lots together. The One-Way Formula Operating Plan buys up to four separate lots (Fund L and Fund S) at different prices, each with its own stop, and sizes the stops from a dollar square-root table that stops at a $150 stock price; index levels are far outside that table.",
        "wdowf_reversal"=>"Held: not the book's plan. Reversal signals with no stop are Plan #1, which Dunnigan tested and rejected (\"It is not good practice to buy and then sit back and do nothing until a sell signal appears\", p.224); the recommended Operating Plan is held for its four lots and dollar table.",
        // ---- end Dunnigan block
        // ---- Katsanos block (25 Oct 2026): held Live rules
        "mkits_rs_fut"|"mkits_rs_etf"=>"Held: needs the S&P 500 benchmark series. The relative-strength systems compare each market with the S&P 500 close day by day (Appendix A.7); that benchmark series is not installed with the vault's market data, so the engine would find no benchmark and trade nothing.",
        // ---- end Katsanos block
        "ss_adx_gapper_no_gap"=>"Held: rules not fully defined. Street Smarts (p.93) presents this as Larry's discussion and calls it \"more subjective\"; it gives no objective entry, so any coded entry would be the vault's, not the book's.",
        "hfm_hv"=>"Held: rules not fully defined. The book (Ch. 3) gives a volatility warning with no direction, entry, exit or stop (\"a personal choice\"); the coded breakout entry and 20-day exit would be vault choices.",
        "hfm_und"|"hfm_und_long"|"hfm_und_short"=>"Held: needs option prices. The book (Ch. 4) trades Undeniables by selling out-of-the-money index puts or calls; a long or short trade in the index itself has a different payoff, and the vault holds no traded option prices.",
        "btp_bottom_reversal_long"=>"Held: rules not fully defined. \"Bottom\", \"near the bottom of the range\" and \"a fairly tight stop\" are not quantified in the book, so the entry and stop would be the vault's.",
        "btp_momentum_gap"=>"Held: rules not fully defined. The book does not say what counts as a gap (open above yesterday's close, or above yesterday's high) and gives no stop or trailing-stop level, so both the entry and the exit would be the vault's choice.",
        // connors-leftovers (22 Oct 2026): plain card reasons for the remaining held Connors rules
        "ss_momentum_pinball"=>"Held: needs intraday data. Momentum Pinball buys or sells a break of the first trading hour's high or low; the vault holds daily bars only, so the first-hour range of each day is unknown.",
        "ss_smart_money_index"=>"Held: needs intraday data. The Smart Money Index is built from the first and the last hour of each trading day, and the book does not give an entry price or exit for it; the vault holds daily bars only.",
        "ss_arms_index_5ma"|"ss_arms_index_5ma_nse"=>"Held: rules not fully defined. The book shows the 5-day average of the Arms Index (TRIN) as an indicator reading and leaves the trade to the reader (\"determine for yourself\", pp.181-182); there is no entry or exit to test.",
        "oiss_plus_three_put"|"oiss_plus_three_call"=>"Held: needs option prices. OISS Plus Three sells index options; the vault has no US option prices, and for India the engine cannot yet value open option positions, so the result would not be the option trade the book describes.",
        "spmt_tick"=>"Held: needs NYSE TICK data. This rule reads the NYSE TICK at the close each day; the vault has no TICK history for the US or India.",
        "spmt_multi5"|"spmt_multi9"|"spmt_mts"=>"Held: needs NYSE TICK data. The Market Timing System counts how many of the course's 30 daily signals fire, and one of those signals is the NYSE TICK, which the vault does not have; counting only the other signals would be a different system.",
        "spmt_dt_gapdown"|"spmt_dt_unch"|"spmt_dt_gapup"=>"Held: needs intraday data. These day trades are built on 15-minute bars of the trading day; the vault holds daily bars only.",
        // connors-final 26 Oct 2026: cwin_vix_gap_window is admitted on NIFTY 50 (India VIX 1-minute series); this text is no longer reached.
        "ss_morning_news_reversal"=>"Held: needs data. Morning News Reversals trades the first move after an 8:30 ET economic report (the book trades bond and currency futures, which react before the stock market opens): the vault has no dated list of report days and no futures prices, and on stock indices the report comes an hour before the open, so the book's \"lifts the market four ticks above yesterday's high\" cannot be seen as it happened. Reviewed 26 Oct 2026; options for you in authors/connors-final/USER-DECISIONS.md.",
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
    if let Some(label)=bernstein_g1_label(def) {json.push_str(&format!(",\"declared_application\":\"{label}\""));}
    if let Some(label)=schwager_label(def) {json.push_str(&format!(",\"declared_application\":\"{label}\""));} // Schwager block
    if let Some(label)=elder_label(def) {json.push_str(&format!(",\"declared_application\":\"{label}\""));} // Elder block
    if let Some(label)=lwilliams_label(def) {json.push_str(&format!(",\"declared_application\":\"{label}\""));} // Larry Williams block
    if let Some(label)=pring_label(def) {json.push_str(&format!(",\"declared_application\":\"{label}\""));} // Pring block
    if jobman_reviewed(def) {json.push_str(&format!(",\"declared_application\":\"{}\"",JOBMAN_LABEL));} // Jobman block
    if schaap_reviewed(def) {json.push_str(&format!(",\"declared_application\":\"{}\"",SCHAAP_LABEL));} // Schaap block
    if satchell_reviewed(def) {json.push_str(&format!(",\"declared_application\":\"{}\"",SATCHELL_LABEL));} // Satchell block
    if kirkpatrick_reviewed(def) {json.push_str(&format!(",\"declared_application\":\"{}\"",KIRKPATRICK_LABEL));} // Kirkpatrick block
    if goslin_reviewed(def) {json.push_str(&format!(",\"declared_application\":\"{}\"",GOSLIN_LABEL));} // Goslin block
    if ruggiero_reviewed(def) {json.push_str(&format!(",\"declared_application\":\"{}\"",RUGGIERO_LABEL));} // Ruggiero block
    if link_reviewed(def) {json.push_str(&format!(",\"declared_application\":\"{}\"",LINK_LABEL));} // Link block
    if katz_reviewed(def) {json.push_str(&format!(",\"declared_application\":\"{}\"",KATZ_LABEL));} // Katz block
    if crabel_reviewed(def) {json.push_str(&format!(",\"declared_application\":\"{}\"",CRABEL_LABEL));} // Crabel block
    if stad_reviewed(def) {json.push_str(&format!(",\"declared_application\":\"{}\"",STAD_LABEL));} // STAD block
    if masonson_reviewed(def) {json.push_str(&format!(",\"declared_application\":\"{}\"",MASONSON_LABEL));} // Leslie Masonson block
    if collins_reviewed(def) {json.push_str(&format!(",\"declared_application\":\"{}\"",COLLINS_LABEL));} // Art Collins block
    if guppy_reviewed(def) {json.push_str(&format!(",\"declared_application\":\"{}\"",GUPPY_LABEL));} // Guppy block
    if headley_reviewed(def) {json.push_str(&format!(",\"declared_application\":\"{}\"",HEADLEY_LABEL));} // Headley block
    if jcarter_reviewed(def) {json.push_str(&format!(",\"declared_application\":\"{}\"",JCARTER_LABEL));} // Carter block
    if bryant_reviewed(def) {json.push_str(&format!(",\"declared_application\":\"{}\"",BRYANT_LABEL));} // Bryant block
    if gettess_reviewed(def) {json.push_str(&format!(",\"declared_application\":\"{}\"",GETTESS_LABEL));} // Gettess block
    if primo_reviewed(def) {json.push_str(&format!(",\"declared_application\":\"{}\"",PRIMO_LABEL));} // Primo block
    if meyers_reviewed(def) {json.push_str(&format!(",\"declared_application\":\"{}\"",MEYERS_LABEL));} // Meyers block
    if krutsinger_reviewed(def) {json.push_str(&format!(",\"declared_application\":\"{}\"",KRUTSINGER_LABEL));} // Krutsinger block
    if clayburg_reviewed(def) {json.push_str(&format!(",\"declared_application\":\"{}\"",CLAYBURG_LABEL));} // Clayburg block
    if stendahl_reviewed(def) {json.push_str(&format!(",\"declared_application\":\"{}\"",STENDAHL_LABEL));} // Stendahl block
    if boucher_reviewed(def) {json.push_str(&format!(",\"declared_application\":\"{}\"",BOUCHER_LABEL));} // Boucher block
    if freeburg_reviewed(def) {json.push_str(&format!(",\"declared_application\":\"{}\"",FREEBURG_LABEL));} // Freeburg block
    if rjones_reviewed(def) {json.push_str(&format!(",\"declared_application\":\"{}\"",RJONES_LABEL));} // RJones block
    if blau_reviewed(def) {json.push_str(&format!(",\"declared_application\":\"{}\"",BLAU_LABEL));} // Blau block
    if faith_reviewed(def) {json.push_str(&format!(",\"declared_application\":\"{}\"",FAITH_LABEL));} // Faith block
    if dunnigan_reviewed(def) {json.push_str(&format!(",\"declared_application\":\"{}\"",DUNNIGAN_LABEL));} // Dunnigan block
    if katsanos_reviewed(def) {json.push_str(&format!(",\"declared_application\":\"{}\"",KATSANOS_LABEL));} // Katsanos block
    if tomasini_reviewed(def) {json.push_str(&format!(",\"declared_application\":\"{}\"",TOMASINI_LABEL));} // Tomasini block
    if pardo_reviewed(def) {json.push_str(&format!(",\"declared_application\":\"{}\"",PARDO_LABEL));} // Pardo block
    if aan_reviewed(def) {json.push_str(&format!(",\"declared_application\":\"{}\"",AAN_LABEL));} // Aan block
    if lakonishok_reviewed(def) {json.push_str(&format!(",\"declared_application\":\"{}\"",LAKONISHOK_LABEL));} // Lakonishok block
    if sullivan_reviewed(def) {json.push_str(&format!(",\"declared_application\":\"{}\"",SULLIVAN_LABEL));} // Sullivan block
    if campbell_reviewed(def) {json.push_str(&format!(",\"declared_application\":\"{}\"",CAMPBELL_LABEL));} // Campbell block
    if heitkoetter_reviewed(def) {json.push_str(&format!(",\"declared_application\":\"{}\"",HEITKOETTER_LABEL));} // Heitkoetter block
    if dorsey_reviewed(def) {json.push_str(&format!(",\"declared_application\":\"{}\"",DORSEY_LABEL));} // Dorsey block
    if morris_reviewed(def) {json.push_str(&format!(",\"declared_application\":\"{}\"",MORRIS_LABEL));} // Morris block
    if bwilliams_reviewed(def) {json.push_str(&format!(",\"declared_application\":\"{}\"",BWILLIAMS_LABEL));} // BWilliams block
    if stridsman_reviewed(def) {json.push_str(&format!(",\"declared_application\":\"{}\"",STRIDSMAN_LABEL));} // Stridsman block
    if weissman_reviewed(def) {json.push_str(&format!(",\"declared_application\":\"{}\"",WEISSMAN_LABEL));} // Weissman block
    if raschke_reviewed(def) {json.push_str(&format!(",\"declared_application\":\"{}\"",RASCHKE_LABEL));} // Raschke block
    if carnold_reviewed(def) {json.push_str(&format!(",\"declared_application\":\"{}\"",CARNOLD_LABEL));} // CArnold block
    if ndavis_reviewed(def) {json.push_str(&format!(",\"declared_application\":\"{}\"",NDAVIS_LABEL));} // NDavis block
    if siegel_reviewed(def) {json.push_str(&format!(",\"declared_application\":\"{}\"",SIEGEL_LABEL));} // Siegel block
    if lydon_reviewed(def) {json.push_str(&format!(",\"declared_application\":\"{}\"",LYDON_LABEL));} // Lydon block
    if landry_reviewed(def) {json.push_str(&format!(",\"declared_application\":\"{}\"",LANDRY_LABEL));} // Landry block
    if pruitt_reviewed(def) {json.push_str(&format!(",\"declared_application\":\"{}\"",PRUITT_LABEL));} // Pruitt block
    if hirsch_reviewed(def) {json.push_str(&format!(",\"declared_application\":\"{}\"",HIRSCH_LABEL));} // Hirsch block
    if bandy_reviewed(def) {json.push_str(&format!(",\"declared_application\":\"{}\"",BANDY_LABEL));} // Bandy block
    if kestner_reviewed(def) {json.push_str(&format!(",\"declared_application\":\"{}\"",KESTNER_LABEL));} // Kestner block
    if wirrick_reviewed(def) {json.push_str(&format!(",\"declared_application\":\"{}\"",WIRRICK_LABEL));} // Wirrick block
    if sperandeo_reviewed(def) {json.push_str(&format!(",\"declared_application\":\"{}\"",SPERANDEO_LABEL));} // Sperandeo block
    if csmith_reviewed(def) {json.push_str(&format!(",\"declared_application\":\"{}\"",CSMITH_LABEL));} // CSmith block
    if lien_reviewed(def) {json.push_str(&format!(",\"declared_application\":\"{}\"",LIEN_LABEL));} // Lien block
    if arms_reviewed(def) {json.push_str(&format!(",\"declared_application\":\"{}\"",ARMS_LABEL));} // Arms block
    if let Some(label)=gregoriou_label(def) {json.push_str(&format!(",\"declared_application\":\"{label}\""));} // Gregoriou block
    if crisp_reviewed(def) {json.push_str(&format!(",\"declared_application\":\"{CRISP_LABEL}\""));} // Crisp block
    if fabozzi_reviewed(def) {json.push_str(&format!(",\"declared_application\":\"{}\"",FABOZZI_LABEL));} // Fabozzi block
    if fontanills_reviewed(def) {json.push_str(&format!(",\"declared_application\":\"{}\"",FONTANILLS_LABEL));} // Fontanills block
    if thomsett_reviewed(def) {json.push_str(&format!(",\"declared_application\":\"{}\"",THOMSETT_LABEL));} // Thomsett block
    if patel_reviewed(def) {json.push_str(&format!(",\"declared_application\":\"{}\"",PATEL_LABEL));} // Patel block
    if demark_reviewed(def) {json.push_str(&format!(",\"declared_application\":\"{}\"",DEMARK_LABEL));} // DeMark block
    if kroll_reviewed(def) {json.push_str(&format!(",\"declared_application\":\"{}\"",KROLL_LABEL));} // Kroll block
    if kaeppel_reviewed(def) {json.push_str(&format!(",\"declared_application\":\"{}\"",KAEPPEL_LABEL));} // Kaeppel block
    if hamon_reviewed(def) {json.push_str(&format!(",\"declared_application\":\"{}\"",HAMON_LABEL));} // Hamon block
    if toghraie_reviewed(def) {json.push_str(&format!(",\"declared_application\":\"{}\"",TOGHRAIE_LABEL));} // Toghraie block
    if harris_reviewed(def) {json.push_str(&format!(",\"declared_application\":\"{}\"",HARRIS_LABEL));} // Harris block
    if etzkorn_reviewed(def) {json.push_str(&format!(",\"declared_application\":\"{}\"",ETZKORN_LABEL));} // Etzkorn block
    if lebeau_reviewed(def) {json.push_str(&format!(",\"declared_application\":\"{}\"",LEBEAU_LABEL));} // LeBeau block
    if tharp_reviewed(def) {json.push_str(&format!(",\"declared_application\":\"{}\"",THARP_LABEL));} // Tharp block
    if person_reviewed(def) {json.push_str(&format!(",\"declared_application\":\"{}\"",PERSON_LABEL));} // Person block
    if kaufman_reviewed(def) {json.push_str(&format!(",\"declared_application\":\"{}\"",KAUFMAN_LABEL));} // Kaufman block
    if appel_reviewed(def) {json.push_str(&format!(",\"declared_application\":\"{}\"",APPEL_LABEL));} // Appel block
    if def.book==crate::books::cwin::BOOK && crate::books::cwin::reviewed_daily(def.id) {json.push_str(concat!(",\"declared_application\":\"",CWIN_INDEX_LABEL!(),"\""));}
    if let Some(label)=connors_final_label(def) {json.push_str(&format!(",\"declared_application\":\"{label}\""));} // connors-final block
    if def.book==crate::books::ccl08::BOOK && crate::books::ccl08::day_trade(def.id) {json.push_str(concat!(",\"declared_application\":\"",CCL08_INDEX_LABEL!(),"\""));}
    if def.book==crate::books::ats::BOOK && crate::books::ats::reviewed_daily(def.id) && (p.requirement=="index_or_etf" || (crate::books::ats::STOCK_TICK_IDS.contains(&def.id) && !CONNORS_INDEX_IDS.contains(&def.id))) {json.push_str(concat!(",\"declared_application\":\"",BTP_INDEX_LABEL!(),"\""));}
    if def.book==crate::books::alpha::BOOK && def.id=="alpha_trend_6m" {json.push_str(concat!(",\"declared_application\":\"",ALPHA_TREND_LABEL!(),"\""));}
    if def.book=="btp" && crate::books::btp::reviewed_daily(def.id) {json.push_str(concat!(",\"declared_application\":\"",BTP_INDEX_LABEL!(),"\""));}
    if (def.book=="btp" && crate::books::btp::STOCK_ADMITTED.contains(&def.id)) || (def.book==crate::books::ats::BOOK && crate::books::ats::STOCK_TICK_IDS.contains(&def.id)) {json.push_str(concat!(",\"stock_adaptation\":\"",STOCK_TICK_LABEL!(),"\""));}
    if def.book==crate::books::ws5::BOOK && crate::books::ws5::reviewed_daily(def.id) && !crate::books::ws5::TIMING.contains(&def.id) {json.push_str(concat!(",\"stock_adaptation\":\"",WS5_STOCK_LABEL!(),"\""));}
    if def.book=="afactor" && crate::books::afactor::stock_rule(def.id) {json.push_str(concat!(",\"stock_adaptation\":\"",AFACTOR_STOCK_LABEL!(),"\""));}
    if matches!((def.book,def.id),("r3r4","r3r4_lowprice"|"r3r4_lowprice_scale")) {json.push_str(concat!(",\"stock_adaptation\":\"",USD_PRICE_US_LABEL!(),"\""));}
    if let Some(label)=open_instrument_label(def) {
        // open-all block: a book label already present is kept and the 23 Oct scope sentence is appended to it (one key).
        let key="\"declared_application\":\"";
        match json.find(key).filter(|_|open_all(def)) {
            Some(i)=>{let start=i+key.len();let end=start+json[start..].find('"').unwrap();json.insert_str(end,&format!(" {label}"));}
            None=>json.push_str(&format!(",\"declared_application\":\"{label}\"")),
        }
    }
    if jb_g3_reviewed(def) && !jb_g3_gap(def) && def.book!=crate::books::jbeft::BOOK {json.push_str(if def.book==crate::books::jbsmsw::BOOK {concat!(",\"declared_application\":\"",JBSMSW_LABEL!(),"\"")} else if def.book==crate::books::jbtifm::BOOK {",\"declared_application\":\"Systems tested on FTSE-100 futures (2000ST) or shown on a monthly CAC-40 chart (9/18 crossover); index, ETF and stock runs (US and India) on daily bars are a declared application, not the author's test.\""} else if def.id=="jbtywr_crude_outside" {",\"declared_application\":\"Pattern tested on crude oil futures; index, ETF and stock runs (US and India) are a declared application, not the author's test.\""} else {concat!(",\"declared_application\":\"",JBHSMS_KD_LABEL!(),"\"")});}
    if jb_g3_reviewed(def) && def.book==crate::books::jbeft::BOOK {json.push_str(",\"declared_application\":\"S&P 500 futures rule run on the S&P 500 cash index (SPX) from 2014, not the author's test.\"");}
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
    // ---- Larry Williams block (2026-10-23) ----
    #[test]
    fn lwilliams_review_admits_exact_ids_with_their_instrument_scope() {
        use crate::books::*;
        let all:Vec<StrategyDef>=[lwlts::catalogue(),lwicw::catalogue(),lwmdc::catalogue(),lwpro::catalogue(),lwcot::catalogue(),lwdtfo::catalogue(),lwdg2::catalogue(),
            lwcmc::catalogue(),lwnl::catalogue(),lwfc10::catalogue(),lwic6::catalogue(),lwinb::catalogue()].into_iter().flatten().collect();
        let want:Vec<&str>=[lwlts::REVIEWED,lwicw::REVIEWED,lwmdc::REVIEWED,lwpro::REVIEWED,lwcot::REVIEWED,lwdtfo::REVIEWED,lwdg2::REVIEWED,lwcmc::REVIEWED,
            lwnl::REVIEWED,lwfc10::REVIEWED,lwic6::REVIEWED,lwinb::REVIEWED].concat();
        let mut admitted:Vec<&str>=all.iter().filter(|d|lwilliams_reviewed(d)).map(|d|d.id).collect();
        let mut w=want.clone();admitted.sort();w.sort();assert_eq!(admitted,w);
        for d in all.iter().filter(|d|want.contains(&d.id)) {
            assert!(reviewed_cash_rule(d),"{}",d.id);assert!(policy(d).market_adaptation,"{}",d.id);
            assert!(metadata_json(d).contains("\"declared_application\""),"{}",d.id);
            match crate::books::lw_admit::only_ticker(d.id) {
                Some(t) => { assert!(check(d,t,false).is_none(),"{} {t}",d.id);
                    for o in ["SPY","QQQ","NIFTY 50","NIFTYBANK",if t=="SPX" {"NDX"} else {"SPX"}] {assert!(check(d,o,false).is_some(),"{} {o}",d.id);}
                    assert!(check(d,"AAPL",true).is_some() && check(d,"RELIANCE",true).is_some(),"{}",d.id); }
                None if crate::books::lw_admit::us_only(d.id) => {
                    for t in ["SPY","QQQ","SPX","NDX","DJI"] {assert!(check(d,t,false).is_none(),"{} {t}",d.id);}
                    for t in ["NIFTY 50","NIFTYBANK"] {assert!(check(d,t,false).is_some(),"{} {t}",d.id);} }
                None => { for t in ["SPY","QQQ","SPX","NDX","DJI","NIFTY 50","NIFTYBANK"] {assert!(check(d,t,false).is_none(),"{} {t}",d.id);}
                    assert!(check(d,"AAPL",true).is_none() && check(d,"RELIANCE",true).is_none(),"{}",d.id); }
            }
        }
        for id in ["lwnl_july4_sell","lwnl_keyrev_sell"/* lwlts_mon_vb, lwdg2_0105: intraday wave 1 admission */] {assert!(!want.contains(&id) && held_reason(id).is_some_and(|r|r.starts_with("Held: ")),"{id}");}
        assert!(!lwilliams_reviewed(&def("lwlts_hh5","jbudt")));
    }
    // ---- end Larry Williams block
    // ---- Elder block (2026-10-23) ----
    #[test]
    fn elder_review_admits_exact_ids_on_indices_etfs_and_stocks() {
        use crate::books::{eltfl,elcim};
        let all:Vec<StrategyDef>=eltfl::catalogue().into_iter().chain(elcim::catalogue()).collect();
        let mut want:Vec<&str>=eltfl::REVIEWED.iter().chain(elcim::REVIEWED).copied().collect();
        let mut admitted:Vec<&str>=all.iter().filter(|d|elder_reviewed(d)).map(|d|d.id).collect();
        want.sort();admitted.sort();assert_eq!(admitted,want);
        for d in all.iter().filter(|d|want.contains(&d.id)) {
            assert!(reviewed_cash_rule(d));assert_eq!(policy(d).requirement,"index_etf_or_stock","{}",d.id);assert!(policy(d).market_adaptation);
            assert!(metadata_json(d).contains("\"declared_application\""),"{}",d.id);
            for t in ["SPY","QQQ","SPX","NDX","DJI","NIFTY 50","NIFTYBANK"] {assert!(check(d,t,false).is_none(),"{} {t}",d.id);}
            assert!(check(d,"AAPL",true).is_none() && check(d,"RELIANCE",true).is_none(),"{}",d.id);
        }
        assert!(!elder_reviewed(&def("eltfl_fi2","elcim")));
    }
    // ---- end Elder block ----
    // ---- Schwager block (2026-10-23) ----
    #[test]
    fn schwager_review_admits_exact_ids_on_indices_etfs_and_stocks() {
        use crate::books::{jscgf,jsgsta,jsats,sofmt,nmwbk};
        let all:Vec<StrategyDef>=jscgf::catalogue().into_iter().chain(jsgsta::catalogue()).chain(jsats::catalogue()).chain(sofmt::catalogue()).chain(nmwbk::catalogue()).collect();
        let want:Vec<&str>=jscgf::REVIEWED.iter().chain(jsgsta::REVIEWED).chain(jsats::REVIEWED).chain(sofmt::REVIEWED).chain(nmwbk::REVIEWED).copied().collect();
        let mut admitted:Vec<&str>=all.iter().filter(|d|reviewed_cash_rule(d)).map(|d|d.id).collect();
        let mut w=want.clone();admitted.sort();w.sort();assert_eq!(admitted,w);
        for d in all.iter().filter(|d|want.contains(&d.id)) {
            assert_eq!(policy(d).requirement,"index_etf_or_stock","{}",d.id);assert!(policy(d).market_adaptation);
            assert!(metadata_json(d).contains("\"declared_application\""),"{}",d.id);
            for t in ["SPY","QQQ","SPX","NDX","DJI","NIFTY 50","NIFTYBANK"] {assert!(check(d,t,false).is_none(),"{} {t}",d.id);}
            assert!(check(d,"AAPL",true).is_none() && check(d,"RELIANCE",true).is_none(),"{}",d.id);
        }
        for d in all.iter().filter(|d|!want.contains(&d.id)) {assert!(!reviewed_cash_rule(d));assert!(d.status!=crate::connors::Status::Live,"{}",d.id);}
        assert!(!reviewed_cash_rule(&def("jscgf_ma","jbudt")));
    }
    // ---- end Schwager block ----
    // ---- open-all block (23 Oct 2026) ----
    #[test]
    fn open_all_runs_listed_ids_on_every_instrument_type_and_keeps_vix_open_limits() {
        let us=NativeEvidence::us_test_fixture();let india=NativeEvidence::india_test_fixture();
        let vix_open=["spmt_vix2","spmt_vix7","vixr_cvr1","vixr_cvr5_1d","vixr_cvr5_3d","vixr_cvr7","vixr_cvr9","vixr_cvr10","ptjmt_cvr1","ptjmt_cvr5","ststw_vix_rsi","otvt_cvr1_long","otvt_cvr1_short"];
        // (STC IX vix2/vix7 already ran on NIFTY 50 before 23 Oct; that existing route is reported, not changed here.)
        for &(book,id) in OPEN_ALL_RULES {
            let d=def(id,book);
            assert!(open_all(&d) && open_instrument_label(&d)==Some(if vix_india(&d) {VIX_INDIA_LABEL} else {OPEN_ALL_LABEL}),"{id}");
            assert_ne!(policy(&d).requirement,"source_review_required","{id}");
            let m=metadata_json(&d);assert!(m.contains("user decision 23 Oct 2026"),"{id}");assert_eq!(m.matches("\"declared_application\"").count(),1,"{id}");
            for t in ["SPY","QQQ","SPX","NDX","DJI"] {assert!(check(&d,t,false).is_none(),"{id} {t}");}
            assert!(check_native(&d,"AAPL",&us).is_none(),"{id} AAPL");
            // VIX India block (24 Oct 2026): the VIX-open rules also run on Indian instruments (real India VIX opens)
            assert!(!vix_open.contains(&id) || vix_india(&d),"{id}");
            for t in ["NIFTY 50","NIFTYBANK"] {assert!(check(&d,t,false).is_none(),"{id} {t}");}
            assert!(check_native(&d,"RELIANCE",&india).is_none(),"{id} RELIANCE");
        }
        // An unlisted sibling keeps its old route; a source-review hold is never opened.
        assert!(!open_all(&def("vixr_cvr8","vixr")) && vix_india(&def("vixr_cvr8","vixr")) && check(&def("vixr_cvr8","vixr"),"QQQ",false).is_none());
        assert!(check(&def("radge_oc_long_stop","radge"),"SPY",false).is_some());
    }
    // ---- end open-all block ----
    // ---- Pring block (2026-10-23) ----
    #[test]
    fn pring_review_admits_exact_ids_on_indices_etfs_and_stocks() {
        use crate::books::{mpts,mpmm,mpcse,mpsel,mpasi,mptae};
        let all:Vec<StrategyDef>=mptae::catalogue().into_iter().chain(mpts::catalogue()).into_iter().chain(mpmm::catalogue()).chain(mpcse::catalogue()).chain(mpsel::catalogue()).chain(mpasi::catalogue()).collect();
        let want:Vec<&str>=mptae::REVIEWED.iter().chain(mpasi::REVIEWED).chain(mpts::REVIEWED).chain(mpmm::REVIEWED).chain(mpcse::REVIEWED).chain(mpsel::REVIEWED).copied().collect();
        let mut admitted:Vec<&str>=all.iter().filter(|d|reviewed_cash_rule(d)).map(|d|d.id).collect();
        let mut w=want.clone();admitted.sort();w.sort();assert_eq!(admitted,w);
        for d in all.iter().filter(|d|want.contains(&d.id)) {
            assert!(d.status==crate::connors::Status::Live,"{}",d.id);
            assert_eq!(policy(d).requirement,"index_etf_or_stock","{}",d.id);assert!(policy(d).market_adaptation);
            assert!(metadata_json(d).contains("\"declared_application\""),"{}",d.id);
            for t in ["SPY","QQQ","SPX","NDX","DJI","NIFTY 50","NIFTYBANK"] {assert!(check(d,t,false).is_none(),"{} {t}",d.id);}
            assert!(check(d,"AAPL",true).is_none() && check(d,"RELIANCE",true).is_none(),"{}",d.id);
        }
        for d in all.iter().filter(|d|!want.contains(&d.id)) {assert!(!reviewed_cash_rule(d),"{}",d.id);}
        assert!(!reviewed_cash_rule(&def("mpts_triple","jbudt")));
    }
    // ---- end Pring block ----
    // ---- Jobman block (2026-10-24) ----
    #[test]
    fn jobman_review_admits_exact_ids_on_indices_etfs_and_stocks() {
        use crate::books::{djhta};
        let all:Vec<StrategyDef>=djhta::catalogue().into_iter().collect();
        let mut want:Vec<&str>=djhta::REVIEWED.iter().copied().collect();
        let mut admitted:Vec<&str>=all.iter().filter(|d|reviewed_cash_rule(d)).map(|d|d.id).collect();
        admitted.sort();want.sort();assert_eq!(admitted,want);
        for d in all.iter().filter(|d|want.contains(&d.id)) {
            assert_eq!(policy(d).requirement,"index_etf_or_stock","{}",d.id);assert!(policy(d).market_adaptation);
            assert!(metadata_json(d).contains("\"declared_application\""),"{}",d.id);
            for t in ["SPY","QQQ","SPX","NDX","DJI","NIFTY 50","NIFTYBANK"] {assert!(check(d,t,false).is_none(),"{} {t}",d.id);}
            assert!(check(d,"AAPL",true).is_none() && check(d,"RELIANCE",true).is_none(),"{}",d.id);
        }
    }
    // ---- end Jobman block ----
    // ---- Schaap block (2026-10-24) ----
    #[test]
    fn schaap_review_admits_exact_ids_on_indices_etfs_and_stocks() {
        use crate::books::{cbsiws};
        let all:Vec<StrategyDef>=cbsiws::catalogue().into_iter().collect();
        let mut want:Vec<&str>=cbsiws::REVIEWED.iter().copied().collect();
        let mut admitted:Vec<&str>=all.iter().filter(|d|reviewed_cash_rule(d)).map(|d|d.id).collect();
        admitted.sort();want.sort();assert_eq!(admitted,want);
        for d in all.iter().filter(|d|want.contains(&d.id)) {
            assert_eq!(policy(d).requirement,"index_etf_or_stock","{}",d.id);assert!(policy(d).market_adaptation);
            assert!(metadata_json(d).contains("\"declared_application\""),"{}",d.id);
            let us_only=SCHAAP_US_ONLY.contains(&d.id);
            for t in ["SPY","QQQ","SPX","NDX","DJI"] {assert!(check(d,t,false).is_none(),"{} {t}",d.id);}
            for t in ["NIFTY 50","NIFTYBANK"] {assert_eq!(check(d,t,false).is_some(),us_only,"{} {t}",d.id);}
            // check() has no market for a native stock (treated as non-US); the served path uses the stock's real market
            if !us_only {assert!(check(d,"AAPL",true).is_none() && check(d,"RELIANCE",true).is_none(),"{}",d.id);}
        }
    }
    // ---- end Schaap block ----
    // ---- Satchell block (2026-10-24) ----
    #[test]
    fn satchell_review_admits_exact_ids_on_indices_etfs_and_stocks() {
        use crate::books::{acatr};
        let all:Vec<StrategyDef>=acatr::catalogue().into_iter().collect();
        let mut want:Vec<&str>=acatr::REVIEWED.iter().copied().collect();
        let mut admitted:Vec<&str>=all.iter().filter(|d|reviewed_cash_rule(d)).map(|d|d.id).collect();
        admitted.sort();want.sort();assert_eq!(admitted,want);
        for d in all.iter().filter(|d|want.contains(&d.id)) {
            assert_eq!(policy(d).requirement,"index_etf_or_stock","{}",d.id);assert!(policy(d).market_adaptation);
            assert!(metadata_json(d).contains("\"declared_application\""),"{}",d.id);
            for t in ["SPY","QQQ","SPX","NDX","DJI","NIFTY 50","NIFTYBANK"] {assert!(check(d,t,false).is_none(),"{} {t}",d.id);}
            assert!(check(d,"AAPL",true).is_none() && check(d,"RELIANCE",true).is_none(),"{}",d.id);
        }
    }
    // ---- end Satchell block ----
    // ---- Kirkpatrick block (2026-10-24) ----
    #[test]
    fn kirkpatrick_review_admits_exact_ids_on_indices_etfs_and_stocks() {
        use crate::books::{kdta};
        let all:Vec<StrategyDef>=kdta::catalogue().into_iter().collect();
        let mut want:Vec<&str>=kdta::REVIEWED.iter().copied().collect();
        let mut admitted:Vec<&str>=all.iter().filter(|d|reviewed_cash_rule(d)).map(|d|d.id).collect();
        admitted.sort();want.sort();assert_eq!(admitted,want);
        for d in all.iter().filter(|d|want.contains(&d.id)) {
            assert_eq!(policy(d).requirement,"index_etf_or_stock","{}",d.id);assert!(policy(d).market_adaptation);
            assert!(metadata_json(d).contains("\"declared_application\""),"{}",d.id);
            for t in ["SPY","QQQ","SPX","NDX","DJI","NIFTY 50","NIFTYBANK"] {assert!(check(d,t,false).is_none(),"{} {t}",d.id);}
            assert!(check(d,"AAPL",true).is_none() && check(d,"RELIANCE",true).is_none(),"{}",d.id);
        }
    }
    // ---- end Kirkpatrick block ----
    // ---- Goslin block (2026-10-24) ----
    #[test]
    fn goslin_review_admits_exact_ids_on_indices_etfs_and_stocks() {
        use crate::books::{cgtdd};
        let all:Vec<StrategyDef>=cgtdd::catalogue().into_iter().collect();
        let mut want:Vec<&str>=cgtdd::REVIEWED.iter().copied().collect();
        let mut admitted:Vec<&str>=all.iter().filter(|d|reviewed_cash_rule(d)).map(|d|d.id).collect();
        admitted.sort();want.sort();assert_eq!(admitted,want);
        for d in all.iter().filter(|d|want.contains(&d.id)) {
            assert_eq!(policy(d).requirement,"index_etf_or_stock","{}",d.id);assert!(policy(d).market_adaptation);
            assert!(metadata_json(d).contains("\"declared_application\""),"{}",d.id);
            for t in ["SPY","QQQ","SPX","NDX","DJI","NIFTY 50","NIFTYBANK"] {assert!(check(d,t,false).is_none(),"{} {t}",d.id);}
            assert!(check(d,"AAPL",true).is_none() && check(d,"RELIANCE",true).is_none(),"{}",d.id);
        }
    }
    // ---- end Goslin block ----
    // ---- Ruggiero block (2026-10-24) ----
    #[test]
    fn ruggiero_review_admits_exact_ids_on_indices_etfs_and_stocks() {
        use crate::books::{rgcts};
        let all:Vec<StrategyDef>=rgcts::catalogue().into_iter().collect();
        let mut want:Vec<&str>=rgcts::REVIEWED.iter().copied().collect();
        let mut admitted:Vec<&str>=all.iter().filter(|d|reviewed_cash_rule(d)).map(|d|d.id).collect();
        admitted.sort();want.sort();assert_eq!(admitted,want);
        for d in all.iter().filter(|d|want.contains(&d.id)) {
            assert_eq!(policy(d).requirement,"index_etf_or_stock","{}",d.id);assert!(policy(d).market_adaptation);
            assert!(metadata_json(d).contains("\"declared_application\""),"{}",d.id);
            for t in ["SPY","QQQ","SPX","NDX","DJI","NIFTY 50","NIFTYBANK"] {assert!(check(d,t,false).is_none(),"{} {t}",d.id);}
            assert!(check(d,"AAPL",true).is_none() && check(d,"RELIANCE",true).is_none(),"{}",d.id);
        }
    }
    // ---- end Ruggiero block ----
    // ---- Link block (2026-10-24) ----
    #[test]
    fn link_review_admits_exact_ids_on_indices_etfs_and_stocks() {
        use crate::books::{mlhpt};
        let all:Vec<StrategyDef>=mlhpt::catalogue().into_iter().collect();
        let mut want:Vec<&str>=mlhpt::REVIEWED.iter().copied().collect();
        let mut admitted:Vec<&str>=all.iter().filter(|d|reviewed_cash_rule(d)).map(|d|d.id).collect();
        admitted.sort();want.sort();assert_eq!(admitted,want);
        for d in all.iter().filter(|d|want.contains(&d.id)) {
            assert_eq!(policy(d).requirement,"index_etf_or_stock","{}",d.id);assert!(policy(d).market_adaptation);
            assert!(metadata_json(d).contains("\"declared_application\""),"{}",d.id);
            for t in ["SPY","QQQ","SPX","NDX","DJI","NIFTY 50","NIFTYBANK"] {assert!(check(d,t,false).is_none(),"{} {t}",d.id);}
            assert!(check(d,"AAPL",true).is_none() && check(d,"RELIANCE",true).is_none(),"{}",d.id);
        }
    }
    // ---- end Link block ----
    // ---- Katz block (2026-10-25) ----
    #[test]
    fn katz_review_admits_exact_ids_on_indices_etfs_and_stocks() {
        use crate::books::{kets};
        let all:Vec<StrategyDef>=kets::catalogue().into_iter().collect();
        let mut want:Vec<&str>=kets::REVIEWED.iter().copied().collect();
        let mut admitted:Vec<&str>=all.iter().filter(|d|reviewed_cash_rule(d)).map(|d|d.id).collect();
        admitted.sort();want.sort();assert_eq!(admitted,want);
        for d in all.iter().filter(|d|want.contains(&d.id)) {
            assert_eq!(policy(d).requirement,"index_etf_or_stock","{}",d.id);assert!(policy(d).market_adaptation);
            assert!(metadata_json(d).contains("\"declared_application\""),"{}",d.id);
            for t in ["SPY","QQQ","SPX","NDX","DJI","NIFTY 50","NIFTYBANK"] {assert!(check(d,t,false).is_none(),"{} {t}",d.id);}
            assert!(check(d,"AAPL",true).is_none() && check(d,"RELIANCE",true).is_none(),"{}",d.id);
        }
    }
    // ---- end Katz block ----
    // ---- Crabel block (2026-10-25) ----
    #[test]
    fn crabel_review_admits_exact_ids_on_indices_etfs_and_stocks() {
        use crate::books::{tcdt};
        let all:Vec<StrategyDef>=tcdt::catalogue().into_iter().collect();
        let mut want:Vec<&str>=tcdt::REVIEWED.iter().copied().collect();
        let mut admitted:Vec<&str>=all.iter().filter(|d|reviewed_cash_rule(d)).map(|d|d.id).collect();
        // tcdt also holds intraday-admitted ids (intraday phase 1); every reviewed id must be admitted
        admitted.sort();want.sort();for w in &want {assert!(admitted.contains(w),"{w}");}
        for d in all.iter().filter(|d|want.contains(&d.id)) {
            assert_eq!(policy(d).requirement,"index_etf_or_stock","{}",d.id);assert!(policy(d).market_adaptation);
            assert!(metadata_json(d).contains("\"declared_application\""),"{}",d.id);
            for t in ["SPY","QQQ","SPX","NDX","DJI","NIFTY 50","NIFTYBANK"] {assert!(check(d,t,false).is_none(),"{} {t}",d.id);}
            assert!(check(d,"AAPL",true).is_none() && check(d,"RELIANCE",true).is_none(),"{}",d.id);
        }
    }
    // ---- end Crabel block ----
    // ---- STAD block (2026-10-25) ----
    #[test]
    fn stad_review_admits_exact_ids_on_indices_etfs_and_stocks() {
        use crate::books::{stad, stad2};
        let all:Vec<StrategyDef>=stad::catalogue().into_iter().chain(stad2::catalogue()).collect();
        let mut want:Vec<&str>=stad::REVIEWED.iter().chain(stad2::REVIEWED.iter()).copied().collect();
        let mut admitted:Vec<&str>=all.iter().filter(|d|reviewed_cash_rule(d)).map(|d|d.id).collect();
        admitted.sort();want.sort();assert_eq!(admitted,want);
        for d in all.iter().filter(|d|want.contains(&d.id)) {
            assert_eq!(policy(d).requirement,"index_etf_or_stock","{}",d.id);assert!(policy(d).market_adaptation);
            assert!(metadata_json(d).contains("\"declared_application\""),"{}",d.id);
            for t in ["SPY","QQQ","SPX","NDX","DJI","NIFTY 50","NIFTYBANK"] {assert!(check(d,t,false).is_none(),"{} {t}",d.id);}
            assert!(check(d,"AAPL",true).is_none() && check(d,"RELIANCE",true).is_none(),"{}",d.id);
        }
    }
    // ---- end STAD block ----
    // ---- Leslie Masonson block (2026-10-25) ----
    #[test]
    fn masonson_review_admits_exact_ids_on_indices_etfs_and_stocks() {
        use crate::books::{masmt};
        let all:Vec<StrategyDef>=masmt::catalogue().into_iter().collect();
        let mut want:Vec<&str>=masmt::REVIEWED.iter().copied().collect();
        for id in ["masmt_vakkur_lev_seasonal","masmt_vakkur_lev_all","masmt_pres_lev2","masmt_pres_lbm","masmt_pres_optimal"] {assert!(!want.contains(&id) && held_reason(id).is_some_and(|r|r.starts_with("Held: ")),"{id}");}
        let mut admitted:Vec<&str>=all.iter().filter(|d|reviewed_cash_rule(d)).map(|d|d.id).collect();
        admitted.sort();want.sort();assert_eq!(admitted,want);
        for d in all.iter().filter(|d|want.contains(&d.id)) {
            assert_eq!(policy(d).requirement,"index_etf_or_stock","{}",d.id);assert!(policy(d).market_adaptation);
            assert!(metadata_json(d).contains("\"declared_application\""),"{}",d.id);
            for t in ["SPY","QQQ","SPX","NDX","DJI","NIFTY 50","NIFTYBANK"] {assert!(check(d,t,false).is_none(),"{} {t}",d.id);}
            assert!(check(d,"AAPL",true).is_none() && check(d,"RELIANCE",true).is_none(),"{}",d.id);
        }
    }
    // ---- end Leslie Masonson block ----
    // ---- Art Collins block (2026-10-25) ----
    #[test]
    fn collins_review_admits_exact_ids_on_indices_etfs_and_stocks() {
        use crate::books::{acbff};
        let all:Vec<StrategyDef>=acbff::catalogue().into_iter().collect();
        let mut want:Vec<&str>=acbff::REVIEWED.iter().copied().collect();
        assert!(!want.contains(&"acbff_forever_long") && held_reason("acbff_forever_long").is_some_and(|r|r.starts_with("Held: ")));
        let mut admitted:Vec<&str>=all.iter().filter(|d|reviewed_cash_rule(d)).map(|d|d.id).collect();
        admitted.sort();want.sort();assert_eq!(admitted,want);
        for d in all.iter().filter(|d|want.contains(&d.id)) {
            assert_eq!(policy(d).requirement,"index_etf_or_stock","{}",d.id);assert!(policy(d).market_adaptation);
            assert!(metadata_json(d).contains("\"declared_application\""),"{}",d.id);
            for t in ["SPY","QQQ","SPX","NDX","DJI","NIFTY 50","NIFTYBANK"] {assert!(check(d,t,false).is_none(),"{} {t}",d.id);}
            assert!(check(d,"AAPL",true).is_none() && check(d,"RELIANCE",true).is_none(),"{}",d.id);
        }
    }
    // ---- end Art Collins block ----
    // ---- Guppy block (2026-10-25) ----
    #[test]
    fn guppy_review_admits_exact_ids_on_indices_etfs_and_stocks() {
        use crate::books::{dgdwb};
        let all:Vec<StrategyDef>=dgdwb::catalogue().into_iter().collect();
        let mut want:Vec<&str>=dgdwb::REVIEWED.iter().copied().collect();
        let mut admitted:Vec<&str>=all.iter().filter(|d|reviewed_cash_rule(d)).map(|d|d.id).collect();
        admitted.sort();want.sort();assert_eq!(admitted,want);
        for d in all.iter().filter(|d|want.contains(&d.id)) {
            assert_eq!(policy(d).requirement,"index_etf_or_stock","{}",d.id);assert!(policy(d).market_adaptation);
            assert!(metadata_json(d).contains("\"declared_application\""),"{}",d.id);
            for t in ["SPY","QQQ","SPX","NDX","DJI","NIFTY 50","NIFTYBANK"] {assert!(check(d,t,false).is_none(),"{} {t}",d.id);}
            assert!(check(d,"AAPL",true).is_none() && check(d,"RELIANCE",true).is_none(),"{}",d.id);
        }
    }
    // ---- end Guppy block ----
    // ---- Headley block (2026-10-25) ----
    #[test]
    fn headley_review_admits_exact_ids_on_indices_etfs_and_stocks() {
        use crate::books::{phbt, phbttk};
        let all:Vec<StrategyDef>=phbt::catalogue().into_iter().chain(phbttk::catalogue()).collect();
        let mut want:Vec<&str>=phbt::REVIEWED.iter().chain(phbttk::REVIEWED.iter()).copied().collect();
        let mut admitted:Vec<&str>=all.iter().filter(|d|reviewed_cash_rule(d)).map(|d|d.id).collect();
        admitted.sort();want.sort();assert_eq!(admitted,want);
        for d in all.iter().filter(|d|want.contains(&d.id)) {
            assert_eq!(policy(d).requirement,"index_etf_or_stock","{}",d.id);assert!(policy(d).market_adaptation);
            assert!(metadata_json(d).contains("\"declared_application\""),"{}",d.id);
            for t in ["SPY","QQQ","SPX","NDX","DJI","NIFTY 50","NIFTYBANK"] {assert!(check(d,t,false).is_none(),"{} {t}",d.id);}
            assert!(check(d,"AAPL",true).is_none() && check(d,"RELIANCE",true).is_none(),"{}",d.id);
        }
    }
    // ---- end Headley block ----
    // ---- Carter block (2026-10-25) ----
    #[test]
    fn jcarter_review_admits_exact_ids_on_indices_etfs_and_stocks() {
        use crate::books::{jcarter};
        let all:Vec<StrategyDef>=jcarter::catalogue().into_iter().collect();
        let mut want:Vec<&str>=jcarter::REVIEWED.iter().copied().collect();
        let mut admitted:Vec<&str>=all.iter().filter(|d|reviewed_cash_rule(d)).map(|d|d.id).collect();
        admitted.sort();want.sort();assert_eq!(admitted,want);
        for d in all.iter().filter(|d|want.contains(&d.id)) {
            assert_eq!(policy(d).requirement,"index_etf_or_stock","{}",d.id);assert!(policy(d).market_adaptation);
            assert!(metadata_json(d).contains("\"declared_application\""),"{}",d.id);
            for t in ["SPY","QQQ","SPX","NDX","DJI","NIFTY 50","NIFTYBANK"] {assert!(check(d,t,false).is_none(),"{} {t}",d.id);}
            assert!(check(d,"AAPL",true).is_none() && check(d,"RELIANCE",true).is_none(),"{}",d.id);
        }
    }
    // ---- end Carter block ----
    // ---- Bryant block (2026-10-25) ----
    #[test]
    fn bryant_review_admits_exact_ids_on_indices_etfs_and_stocks() {
        use crate::books::{bfmm};
        let all:Vec<StrategyDef>=bfmm::catalogue().into_iter().collect();
        let mut want:Vec<&str>=bfmm::REVIEWED.iter().copied().collect();
        let mut admitted:Vec<&str>=all.iter().filter(|d|reviewed_cash_rule(d)).map(|d|d.id).collect();
        admitted.sort();want.sort();assert_eq!(admitted,want);
        for d in all.iter().filter(|d|want.contains(&d.id)) {
            assert_eq!(policy(d).requirement,"index_etf_or_stock","{}",d.id);assert!(policy(d).market_adaptation);
            assert!(metadata_json(d).contains("\"declared_application\""),"{}",d.id);
            for t in ["SPY","QQQ","SPX","NDX","DJI","NIFTY 50","NIFTYBANK"] {assert!(check(d,t,false).is_none(),"{} {t}",d.id);}
            assert!(check(d,"AAPL",true).is_none() && check(d,"RELIANCE",true).is_none(),"{}",d.id);
        }
    }
    // ---- end Bryant block ----
    // ---- Gettess block (2026-10-25) ----
    #[test]
    fn gettess_review_admits_exact_ids_on_indices_etfs_and_stocks() {
        use crate::books::{lg3k, lgtt30, lgsc};
        let all:Vec<StrategyDef>=lg3k::catalogue().into_iter().chain(lgtt30::catalogue()).chain(lgsc::catalogue()).collect();
        let mut want:Vec<&str>=lg3k::REVIEWED.iter().chain(lgtt30::REVIEWED.iter()).chain(lgsc::REVIEWED.iter()).copied().collect();
        let mut admitted:Vec<&str>=all.iter().filter(|d|reviewed_cash_rule(d)).map(|d|d.id).collect();
        admitted.sort();want.sort();assert_eq!(admitted,want);
        for d in all.iter().filter(|d|want.contains(&d.id)) {
            assert_eq!(policy(d).requirement,"index_etf_or_stock","{}",d.id);assert!(policy(d).market_adaptation);
            assert!(metadata_json(d).contains("\"declared_application\""),"{}",d.id);
            for t in ["SPY","QQQ","SPX","NDX","DJI","NIFTY 50","NIFTYBANK"] {assert!(check(d,t,false).is_none(),"{} {t}",d.id);}
            assert!(check(d,"AAPL",true).is_none() && check(d,"RELIANCE",true).is_none(),"{}",d.id);
        }
    }
    // ---- end Gettess block ----
    // ---- Primo block (2026-10-25) ----
    #[test]
    fn primo_review_admits_exact_ids_on_indices_etfs_and_stocks() {
        use crate::books::{pstc, pstc08, prap2};
        let all:Vec<StrategyDef>=pstc::catalogue().into_iter().chain(pstc08::catalogue()).chain(prap2::catalogue()).collect();
        let mut want:Vec<&str>=pstc::REVIEWED.iter().chain(pstc08::REVIEWED.iter()).chain(prap2::REVIEWED.iter()).copied().collect();
        let mut admitted:Vec<&str>=all.iter().filter(|d|reviewed_cash_rule(d)).map(|d|d.id).collect();
        admitted.sort();want.sort();assert_eq!(admitted,want);
        for d in all.iter().filter(|d|want.contains(&d.id)) {
            assert_eq!(policy(d).requirement,"index_etf_or_stock","{}",d.id);assert!(policy(d).market_adaptation);
            assert!(metadata_json(d).contains("\"declared_application\""),"{}",d.id);
            for t in ["SPY","QQQ","SPX","NDX","DJI","NIFTY 50","NIFTYBANK"] {assert!(check(d,t,false).is_none(),"{} {t}",d.id);}
            assert!(check(d,"AAPL",true).is_none() && check(d,"RELIANCE",true).is_none(),"{}",d.id);
        }
    }
    // ---- end Primo block ----
    // ---- Meyers block (2026-10-25) ----
    #[test]
    fn meyers_review_admits_exact_ids_on_indices_etfs_and_stocks() {
        use crate::books::{dmast, dmst2};
        let all:Vec<StrategyDef>=dmast::catalogue().into_iter().chain(dmst2::catalogue()).collect();
        let mut want:Vec<&str>=dmast::REVIEWED.iter().chain(dmst2::REVIEWED.iter()).copied().collect();
        let mut admitted:Vec<&str>=all.iter().filter(|d|reviewed_cash_rule(d)).map(|d|d.id).collect();
        admitted.sort();want.sort();assert_eq!(admitted,want);
        for d in all.iter().filter(|d|want.contains(&d.id)) {
            assert_eq!(policy(d).requirement,"index_etf_or_stock","{}",d.id);assert!(policy(d).market_adaptation);
            assert!(metadata_json(d).contains("\"declared_application\""),"{}",d.id);
            for t in ["SPY","QQQ","SPX","NDX","DJI","NIFTY 50","NIFTYBANK"] {assert!(check(d,t,false).is_none(),"{} {t}",d.id);}
            assert!(check(d,"AAPL",true).is_none() && check(d,"RELIANCE",true).is_none(),"{}",d.id);
        }
    }
    // ---- end Meyers block ----
    // ---- Krutsinger block (2026-10-25) ----
    #[test]
    fn krutsinger_review_admits_exact_ids_on_indices_etfs_and_stocks() {
        use crate::books::{jkbwts, jkbk, upg18};
        let all:Vec<StrategyDef>=jkbwts::catalogue().into_iter().chain(jkbk::catalogue()).chain(upg18::catalogue()).collect();
        let mut want:Vec<&str>=jkbwts::REVIEWED.iter().chain(jkbk::REVIEWED.iter()).chain(upg18::REVIEWED.iter()).copied().collect();
        let mut admitted:Vec<&str>=all.iter().filter(|d|reviewed_cash_rule(d)).map(|d|d.id).collect();
        admitted.sort();want.sort();assert_eq!(admitted,want);
        for d in all.iter().filter(|d|want.contains(&d.id)) {
            assert_eq!(policy(d).requirement,"index_etf_or_stock","{}",d.id);assert!(policy(d).market_adaptation);
            assert!(metadata_json(d).contains("\"declared_application\""),"{}",d.id);
            for t in ["SPY","QQQ","SPX","NDX","DJI","NIFTY 50","NIFTYBANK"] {assert!(check(d,t,false).is_none(),"{} {t}",d.id);}
            assert!(check(d,"AAPL",true).is_none() && check(d,"RELIANCE",true).is_none(),"{}",d.id);
        }
    }
    // ---- end Krutsinger block ----
    // ---- Clayburg block (2026-10-25) ----
    #[test]
    fn clayburg_review_admits_exact_ids_on_indices_etfs_and_stocks() {
        use crate::books::{jcfeed};
        let all:Vec<StrategyDef>=jcfeed::catalogue().into_iter().collect();
        let mut want:Vec<&str>=jcfeed::REVIEWED.iter().copied().collect();
        let mut admitted:Vec<&str>=all.iter().filter(|d|reviewed_cash_rule(d)).map(|d|d.id).collect();
        admitted.sort();want.sort();assert_eq!(admitted,want);
        for d in all.iter().filter(|d|want.contains(&d.id)) {
            assert_eq!(policy(d).requirement,"index_etf_or_stock","{}",d.id);assert!(policy(d).market_adaptation);
            assert!(metadata_json(d).contains("\"declared_application\""),"{}",d.id);
            for t in ["SPY","QQQ","SPX","NDX","DJI","NIFTY 50","NIFTYBANK"] {assert!(check(d,t,false).is_none(),"{} {t}",d.id);}
            assert!(check(d,"AAPL",true).is_none() && check(d,"RELIANCE",true).is_none(),"{}",d.id);
        }
    }
    // ---- end Clayburg block ----
    // ---- Stendahl block (2026-10-25) ----
    #[test]
    fn stendahl_review_admits_exact_ids_on_indices_etfs_and_stocks() {
        use crate::books::{rinadz, hwdti};
        let all:Vec<StrategyDef>=rinadz::catalogue().into_iter().chain(hwdti::catalogue()).collect();
        let mut want:Vec<&str>=rinadz::REVIEWED.iter().chain(hwdti::REVIEWED.iter()).copied().collect();
        let mut admitted:Vec<&str>=all.iter().filter(|d|reviewed_cash_rule(d)).map(|d|d.id).collect();
        admitted.sort();want.sort();assert_eq!(admitted,want);
        for d in all.iter().filter(|d|want.contains(&d.id)) {
            assert_eq!(policy(d).requirement,"index_etf_or_stock","{}",d.id);assert!(policy(d).market_adaptation);
            assert!(metadata_json(d).contains("\"declared_application\""),"{}",d.id);
            for t in ["SPY","QQQ","SPX","NDX","DJI","NIFTY 50","NIFTYBANK"] {assert!(check(d,t,false).is_none(),"{} {t}",d.id);}
            assert!(check(d,"AAPL",true).is_none() && check(d,"RELIANCE",true).is_none(),"{}",d.id);
        }
    }
    // ---- end Stendahl block ----
    // ---- Boucher block (2026-10-25) ----
    #[test]
    fn boucher_review_admits_exact_ids_on_indices_etfs_and_stocks() {
        use crate::books::{mbhfe, mbsot};
        let all:Vec<StrategyDef>=mbhfe::catalogue().into_iter().chain(mbsot::catalogue()).collect();
        let mut want:Vec<&str>=mbhfe::REVIEWED.iter().chain(mbsot::REVIEWED.iter()).copied().collect();
        let mut admitted:Vec<&str>=all.iter().filter(|d|reviewed_cash_rule(d)).map(|d|d.id).collect();
        admitted.sort();want.sort();assert_eq!(admitted,want);
        for d in all.iter().filter(|d|want.contains(&d.id)) {
            assert_eq!(policy(d).requirement,"index_etf_or_stock","{}",d.id);assert!(policy(d).market_adaptation);
            assert!(metadata_json(d).contains("\"declared_application\""),"{}",d.id);
            for t in ["SPY","QQQ","SPX","NDX","DJI","NIFTY 50","NIFTYBANK"] {assert!(check(d,t,false).is_none(),"{} {t}",d.id);}
            assert!(check(d,"AAPL",true).is_none() && check(d,"RELIANCE",true).is_none(),"{}",d.id);
        }
    }
    // ---- end Boucher block ----
    // ---- Freeburg block (2026-10-25) ----
    #[test]
    fn freeburg_review_admits_exact_ids_on_indices_etfs_and_stocks() {
        use crate::books::{nfpk};
        let all:Vec<StrategyDef>=nfpk::catalogue().into_iter().collect();
        let mut want:Vec<&str>=nfpk::REVIEWED.iter().copied().collect();
        let mut admitted:Vec<&str>=all.iter().filter(|d|reviewed_cash_rule(d)).map(|d|d.id).collect();
        admitted.sort();want.sort();assert_eq!(admitted,want);
        for d in all.iter().filter(|d|want.contains(&d.id)) {
            assert_eq!(policy(d).requirement,"index_etf_or_stock","{}",d.id);assert!(policy(d).market_adaptation);
            assert!(metadata_json(d).contains("\"declared_application\""),"{}",d.id);
            for t in ["SPY","QQQ","SPX","NDX","DJI","NIFTY 50","NIFTYBANK"] {assert!(check(d,t,false).is_none(),"{} {t}",d.id);}
            assert!(check(d,"AAPL",true).is_none() && check(d,"RELIANCE",true).is_none(),"{}",d.id);
        }
    }
    // ---- end Freeburg block ----
    // ---- RJones block (2026-10-25) ----
    #[test]
    fn rjones_review_admits_exact_ids_on_indices_etfs_and_stocks() {
        use crate::books::{rjmil};
        let all:Vec<StrategyDef>=rjmil::catalogue().into_iter().collect();
        let mut want:Vec<&str>=rjmil::REVIEWED.iter().copied().collect();
        let mut admitted:Vec<&str>=all.iter().filter(|d|reviewed_cash_rule(d)).map(|d|d.id).collect();
        admitted.sort();want.sort();assert_eq!(admitted,want);
        for d in all.iter().filter(|d|want.contains(&d.id)) {
            assert_eq!(policy(d).requirement,"index_etf_or_stock","{}",d.id);assert!(policy(d).market_adaptation);
            assert!(metadata_json(d).contains("\"declared_application\""),"{}",d.id);
            for t in ["SPY","QQQ","SPX","NDX","DJI","NIFTY 50","NIFTYBANK"] {assert!(check(d,t,false).is_none(),"{} {t}",d.id);}
            assert!(check(d,"AAPL",true).is_none() && check(d,"RELIANCE",true).is_none(),"{}",d.id);
        }
    }
    // ---- end RJones block ----
    // ---- Blau block (2026-10-25) ----
    #[test]
    fn blau_review_admits_exact_ids_on_indices_etfs_and_stocks() {
        use crate::books::{wbmdd};
        let all:Vec<StrategyDef>=wbmdd::catalogue().into_iter().collect();
        let mut want:Vec<&str>=wbmdd::REVIEWED.iter().copied().collect();
        let mut admitted:Vec<&str>=all.iter().filter(|d|reviewed_cash_rule(d)).map(|d|d.id).collect();
        admitted.sort();want.sort();assert_eq!(admitted,want);
        for d in all.iter().filter(|d|want.contains(&d.id)) {
            assert_eq!(policy(d).requirement,"index_etf_or_stock","{}",d.id);assert!(policy(d).market_adaptation);
            assert!(metadata_json(d).contains("\"declared_application\""),"{}",d.id);
            for t in ["SPY","QQQ","SPX","NDX","DJI","NIFTY 50","NIFTYBANK"] {assert!(check(d,t,false).is_none(),"{} {t}",d.id);}
            assert!(check(d,"AAPL",true).is_none() && check(d,"RELIANCE",true).is_none(),"{}",d.id);
        }
    }
    // ---- end Blau block ----
    // ---- Faith block (2026-10-25) ----
    #[test]
    fn faith_review_admits_exact_ids_on_indices_etfs_and_stocks() {
        use crate::books::{covel};
        let all:Vec<StrategyDef>=covel::catalogue().into_iter().collect();
        let mut want:Vec<&str>=covel::REVIEWED.iter().copied().collect();
        let mut admitted:Vec<&str>=all.iter().filter(|d|reviewed_cash_rule(d)).map(|d|d.id).collect();
        admitted.sort();want.sort();assert_eq!(admitted,want);
        for d in all.iter().filter(|d|want.contains(&d.id)) {
            assert_eq!(policy(d).requirement,"index_etf_or_stock","{}",d.id);assert!(policy(d).market_adaptation);
            assert!(metadata_json(d).contains("\"declared_application\""),"{}",d.id);
            for t in ["SPY","QQQ","SPX","NDX","DJI","NIFTY 50","NIFTYBANK"] {assert!(check(d,t,false).is_none(),"{} {t}",d.id);}
            assert!(check(d,"AAPL",true).is_none() && check(d,"RELIANCE",true).is_none(),"{}",d.id);
        }
    }
    // ---- end Faith block ----
    // ---- Dunnigan block (2026-10-25) ----
    #[test]
    fn dunnigan_review_admits_exact_ids_on_indices_etfs_and_stocks() {
        use crate::books::{wdnbp, wdgig};
        let all:Vec<StrategyDef>=wdnbp::catalogue().into_iter().chain(wdgig::catalogue()).collect();
        let mut want:Vec<&str>=wdnbp::REVIEWED.iter().chain(wdgig::REVIEWED.iter()).copied().collect();
        let mut admitted:Vec<&str>=all.iter().filter(|d|reviewed_cash_rule(d)).map(|d|d.id).collect();
        admitted.sort();want.sort();assert_eq!(admitted,want);
        for d in all.iter().filter(|d|want.contains(&d.id)) {
            assert_eq!(policy(d).requirement,"index_etf_or_stock","{}",d.id);assert!(policy(d).market_adaptation);
            assert!(metadata_json(d).contains("\"declared_application\""),"{}",d.id);
            for t in ["SPY","QQQ","SPX","NDX","DJI","NIFTY 50","NIFTYBANK"] {assert!(check(d,t,false).is_none(),"{} {t}",d.id);}
            assert!(check(d,"AAPL",true).is_none() && check(d,"RELIANCE",true).is_none(),"{}",d.id);
        }
    }
    // ---- end Dunnigan block ----
    // ---- Katsanos block (2026-10-25) ----
    #[test]
    fn katsanos_review_admits_exact_ids_on_indices_etfs_and_stocks() {
        use crate::books::{mkits, mkfve};
        let all:Vec<StrategyDef>=mkits::catalogue().into_iter().chain(mkfve::catalogue()).collect();
        let mut want:Vec<&str>=mkits::REVIEWED.iter().chain(mkfve::REVIEWED.iter()).copied().collect();
        let mut admitted:Vec<&str>=all.iter().filter(|d|reviewed_cash_rule(d)).map(|d|d.id).collect();
        admitted.sort();want.sort();assert_eq!(admitted,want);
        for d in all.iter().filter(|d|want.contains(&d.id)) {
            assert_eq!(policy(d).requirement,"index_etf_or_stock","{}",d.id);assert!(policy(d).market_adaptation);
            assert!(metadata_json(d).contains("\"declared_application\""),"{}",d.id);
            for t in ["SPY","QQQ","SPX","NDX","DJI","NIFTY 50","NIFTYBANK"] {assert!(check(d,t,false).is_none(),"{} {t}",d.id);}
            assert!(check(d,"AAPL",true).is_none() && check(d,"RELIANCE",true).is_none(),"{}",d.id);
        }
    }
    // ---- end Katsanos block ----
    // ---- Tomasini block (2026-10-25) ----
    #[test]
    fn tomasini_review_admits_exact_ids_on_indices_etfs_and_stocks() {
        use crate::books::{jt};
        let all:Vec<StrategyDef>=jt::catalogue().into_iter().collect();
        let mut want:Vec<&str>=jt::REVIEWED.iter().copied().collect();
        let mut admitted:Vec<&str>=all.iter().filter(|d|reviewed_cash_rule(d)).map(|d|d.id).collect();
        admitted.sort();want.sort();assert_eq!(admitted,want);
        for d in all.iter().filter(|d|want.contains(&d.id)) {
            assert_eq!(policy(d).requirement,"index_etf_or_stock","{}",d.id);assert!(policy(d).market_adaptation);
            assert!(metadata_json(d).contains("\"declared_application\""),"{}",d.id);
            for t in ["SPY","QQQ","SPX","NDX","DJI","NIFTY 50","NIFTYBANK"] {assert!(check(d,t,false).is_none(),"{} {t}",d.id);}
            assert!(check(d,"AAPL",true).is_none() && check(d,"RELIANCE",true).is_none(),"{}",d.id);
        }
    }
    // ---- end Tomasini block ----
    // ---- Pardo block (2026-10-25) ----
    #[test]
    fn pardo_review_admits_exact_ids_on_indices_etfs_and_stocks() {
        use crate::books::{rpdt, rpeo};
        let all:Vec<StrategyDef>=rpdt::catalogue().into_iter().chain(rpeo::catalogue()).collect();
        let mut want:Vec<&str>=rpdt::REVIEWED.iter().chain(rpeo::REVIEWED.iter()).copied().collect();
        let mut admitted:Vec<&str>=all.iter().filter(|d|reviewed_cash_rule(d)).map(|d|d.id).collect();
        admitted.sort();want.sort();assert_eq!(admitted,want);
        for d in all.iter().filter(|d|want.contains(&d.id)) {
            assert_eq!(policy(d).requirement,"index_etf_or_stock","{}",d.id);assert!(policy(d).market_adaptation);
            assert!(metadata_json(d).contains("\"declared_application\""),"{}",d.id);
            for t in ["SPY","QQQ","SPX","NDX","DJI","NIFTY 50","NIFTYBANK"] {assert!(check(d,t,false).is_none(),"{} {t}",d.id);}
            assert!(check(d,"AAPL",true).is_none() && check(d,"RELIANCE",true).is_none(),"{}",d.id);
        }
    }
    // ---- end Pardo block ----
    // ---- Aan block (2026-10-25) ----
    #[test]
    fn aan_review_admits_exact_ids_on_indices_etfs_and_stocks() {
        use crate::books::{aanwr};
        let all:Vec<StrategyDef>=aanwr::catalogue().into_iter().collect();
        let mut want:Vec<&str>=aanwr::REVIEWED.iter().copied().collect();
        let mut admitted:Vec<&str>=all.iter().filter(|d|reviewed_cash_rule(d)).map(|d|d.id).collect();
        admitted.sort();want.sort();assert_eq!(admitted,want);
        for d in all.iter().filter(|d|want.contains(&d.id)) {
            assert_eq!(policy(d).requirement,"index_etf_or_stock","{}",d.id);assert!(policy(d).market_adaptation);
            assert!(metadata_json(d).contains("\"declared_application\""),"{}",d.id);
            for t in ["SPY","QQQ","SPX","NDX","DJI","NIFTY 50","NIFTYBANK"] {assert!(check(d,t,false).is_none(),"{} {t}",d.id);}
            assert!(check(d,"AAPL",true).is_none() && check(d,"RELIANCE",true).is_none(),"{}",d.id);
        }
    }
    // ---- end Aan block ----
    // ---- Lakonishok block (2026-10-25) ----
    #[test]
    fn lakonishok_review_admits_exact_ids_on_indices_etfs_and_stocks() {
        use crate::books::{bllstr};
        let all:Vec<StrategyDef>=bllstr::catalogue().into_iter().collect();
        let mut want:Vec<&str>=bllstr::REVIEWED.iter().copied().chain(["bllstr_vma_1_50_0","bllstr_fma_1_50_0"]).collect(); // + the 15 Oct pair
        let mut admitted:Vec<&str>=all.iter().filter(|d|reviewed_cash_rule(d)).map(|d|d.id).collect();
        admitted.sort();want.sort();assert_eq!(admitted,want);
        for d in all.iter().filter(|d|bllstr::reviewed(d.id)) {
            assert_eq!(policy(d).requirement,"index_etf_or_stock","{}",d.id);assert!(policy(d).market_adaptation);
            assert!(metadata_json(d).contains("\"declared_application\""),"{}",d.id);
            for t in ["SPY","QQQ","SPX","NDX","DJI","NIFTY 50","NIFTYBANK"] {assert!(check(d,t,false).is_none(),"{} {t}",d.id);}
            assert!(check(d,"AAPL",true).is_none() && check(d,"RELIANCE",true).is_none(),"{}",d.id);
        }
    }
    // ---- end Lakonishok block ----
    // ---- Sullivan block (2026-10-25) ----
    #[test]
    fn sullivan_review_admits_exact_ids_on_indices_etfs_and_stocks() {
        use crate::books::{stwds};
        let all:Vec<StrategyDef>=stwds::catalogue().into_iter().collect();
        let mut want:Vec<&str>=stwds::REVIEWED.iter().copied().collect();
        let mut admitted:Vec<&str>=all.iter().filter(|d|reviewed_cash_rule(d)).map(|d|d.id).collect();
        admitted.sort();want.sort();assert_eq!(admitted,want);
        for d in all.iter().filter(|d|want.contains(&d.id)) {
            assert_eq!(policy(d).requirement,"index_etf_or_stock","{}",d.id);assert!(policy(d).market_adaptation);
            assert!(metadata_json(d).contains("\"declared_application\""),"{}",d.id);
            for t in ["SPY","QQQ","SPX","NDX","DJI","NIFTY 50","NIFTYBANK"] {assert!(check(d,t,false).is_none(),"{} {t}",d.id);}
            assert!(check(d,"AAPL",true).is_none() && check(d,"RELIANCE",true).is_none(),"{}",d.id);
        }
    }
    // ---- end Sullivan block ----
    // ---- Campbell block (2026-10-25) ----
    #[test]
    fn campbell_review_admits_exact_ids_on_indices_etfs_and_stocks() {
        use crate::books::{sctw};
        let all:Vec<StrategyDef>=sctw::catalogue().into_iter().collect();
        let mut want:Vec<&str>=sctw::REVIEWED.iter().copied().collect();
        let mut admitted:Vec<&str>=all.iter().filter(|d|reviewed_cash_rule(d)).map(|d|d.id).collect();
        admitted.sort();want.sort();assert_eq!(admitted,want);
        for d in all.iter().filter(|d|want.contains(&d.id)) {
            assert_eq!(policy(d).requirement,"index_etf_or_stock","{}",d.id);assert!(policy(d).market_adaptation);
            assert!(metadata_json(d).contains("\"declared_application\""),"{}",d.id);
            for t in ["SPY","QQQ","SPX","NDX","DJI","NIFTY 50","NIFTYBANK"] {assert!(check(d,t,false).is_none(),"{} {t}",d.id);}
            assert!(check(d,"AAPL",true).is_none() && check(d,"RELIANCE",true).is_none(),"{}",d.id);
        }
    }
    // ---- end Campbell block ----
    // ---- Heitkoetter block (2026-10-25) ----
    #[test]
    fn heitkoetter_review_admits_exact_ids_on_indices_etfs_and_stocks() {
        use crate::books::{hkmmts};
        let all:Vec<StrategyDef>=hkmmts::catalogue().into_iter().collect();
        let mut want:Vec<&str>=hkmmts::REVIEWED.iter().copied().collect();
        let mut admitted:Vec<&str>=all.iter().filter(|d|reviewed_cash_rule(d)).map(|d|d.id).collect();
        admitted.sort();want.sort();assert_eq!(admitted,want);
        for d in all.iter().filter(|d|want.contains(&d.id)) {
            assert_eq!(policy(d).requirement,"index_etf_or_stock","{}",d.id);assert!(policy(d).market_adaptation);
            assert!(metadata_json(d).contains("\"declared_application\""),"{}",d.id);
            for t in ["SPY","QQQ","SPX","NDX","DJI","NIFTY 50","NIFTYBANK"] {assert!(check(d,t,false).is_none(),"{} {t}",d.id);}
            assert!(check(d,"AAPL",true).is_none() && check(d,"RELIANCE",true).is_none(),"{}",d.id);
        }
    }
    // ---- end Heitkoetter block ----
    // ---- Dorsey block (2026-10-25) ----
    #[test]
    fn dorsey_review_admits_exact_ids_on_indices_etfs_and_stocks() {
        use crate::books::{ddrvi};
        let all:Vec<StrategyDef>=ddrvi::catalogue().into_iter().collect();
        let mut want:Vec<&str>=ddrvi::REVIEWED.iter().copied().collect();
        let mut admitted:Vec<&str>=all.iter().filter(|d|reviewed_cash_rule(d)).map(|d|d.id).collect();
        admitted.sort();want.sort();assert_eq!(admitted,want);
        for d in all.iter().filter(|d|want.contains(&d.id)) {
            assert_eq!(policy(d).requirement,"index_etf_or_stock","{}",d.id);assert!(policy(d).market_adaptation);
            assert!(metadata_json(d).contains("\"declared_application\""),"{}",d.id);
            for t in ["SPY","QQQ","SPX","NDX","DJI","NIFTY 50","NIFTYBANK"] {assert!(check(d,t,false).is_none(),"{} {t}",d.id);}
            assert!(check(d,"AAPL",true).is_none() && check(d,"RELIANCE",true).is_none(),"{}",d.id);
        }
    }
    // ---- end Dorsey block ----
    // ---- Morris block (2026-10-25) ----
    #[test]
    fn morris_review_admits_exact_ids_on_indices_etfs_and_stocks() {
        use crate::books::{msws};
        let all:Vec<StrategyDef>=msws::catalogue().into_iter().collect();
        let mut want:Vec<&str>=msws::REVIEWED.iter().copied().collect();
        let mut admitted:Vec<&str>=all.iter().filter(|d|reviewed_cash_rule(d)).map(|d|d.id).collect();
        admitted.sort();want.sort();assert_eq!(admitted,want);
        for d in all.iter().filter(|d|want.contains(&d.id)) {
            assert_eq!(policy(d).requirement,"index_etf_or_stock","{}",d.id);assert!(policy(d).market_adaptation);
            assert!(metadata_json(d).contains("\"declared_application\""),"{}",d.id);
            for t in ["SPY","QQQ","SPX","NDX","DJI","NIFTY 50","NIFTYBANK"] {assert!(check(d,t,false).is_none(),"{} {t}",d.id);}
            assert!(check(d,"AAPL",true).is_none() && check(d,"RELIANCE",true).is_none(),"{}",d.id);
        }
    }
    // ---- end Morris block ----
    // ---- BWilliams block (2026-10-24) ----
    #[test]
    fn bwilliams_review_admits_exact_ids_on_indices_etfs_and_stocks() {
        use crate::books::{bwntd,bwtc1,bwtc2};
        let all:Vec<StrategyDef>=bwntd::catalogue().into_iter().chain(bwtc1::catalogue()).chain(bwtc2::catalogue()).collect();
        let mut want:Vec<&str>=bwntd::REVIEWED.iter().chain(bwtc1::REVIEWED).chain(bwtc2::REVIEWED).copied().collect();
        let mut admitted:Vec<&str>=all.iter().filter(|d|reviewed_cash_rule(d)).map(|d|d.id).collect();
        admitted.sort();want.sort();assert_eq!(admitted,want);
        for d in all.iter().filter(|d|want.contains(&d.id)) {
            assert_eq!(policy(d).requirement,"index_etf_or_stock","{}",d.id);assert!(policy(d).market_adaptation);
            assert!(metadata_json(d).contains("\"declared_application\""),"{}",d.id);
            for t in ["SPY","QQQ","SPX","NDX","DJI","NIFTY 50","NIFTYBANK"] {assert!(check(d,t,false).is_none(),"{} {t}",d.id);}
            assert!(check(d,"AAPL",true).is_none() && check(d,"RELIANCE",true).is_none(),"{}",d.id);
        }
    }
    // ---- end BWilliams block ----
    // ---- Stridsman block (2026-10-24) ----
    #[test]
    fn stridsman_review_admits_exact_ids_on_indices_etfs_and_stocks() {
        use crate::books::{strtw,strmm};
        let all:Vec<StrategyDef>=strtw::catalogue().into_iter().chain(strmm::catalogue()).collect();
        let mut want:Vec<&str>=strtw::REVIEWED.iter().chain(strmm::REVIEWED).copied().collect();
        let mut admitted:Vec<&str>=all.iter().filter(|d|reviewed_cash_rule(d)).map(|d|d.id).collect();
        admitted.sort();want.sort();assert_eq!(admitted,want);
        for d in all.iter().filter(|d|want.contains(&d.id)) {
            assert_eq!(policy(d).requirement,"index_etf_or_stock","{}",d.id);assert!(policy(d).market_adaptation);
            assert!(metadata_json(d).contains("\"declared_application\""),"{}",d.id);
            for t in ["SPY","QQQ","SPX","NDX","DJI","NIFTY 50","NIFTYBANK"] {assert!(check(d,t,false).is_none(),"{} {t}",d.id);}
            assert!(check(d,"AAPL",true).is_none() && check(d,"RELIANCE",true).is_none(),"{}",d.id);
        }
    }
    // ---- end Stridsman block ----
    // ---- Weissman block (2026-10-24) ----
    #[test]
    fn weissman_review_admits_exact_ids_on_indices_etfs_and_stocks() {
        use crate::books::{rwmts,rwtlc};
        let all:Vec<StrategyDef>=rwmts::catalogue().into_iter().chain(rwtlc::catalogue()).collect();
        let mut want:Vec<&str>=rwmts::REVIEWED.iter().chain(rwtlc::REVIEWED).copied().collect();
        let mut admitted:Vec<&str>=all.iter().filter(|d|reviewed_cash_rule(d)).map(|d|d.id).collect();
        admitted.sort();want.sort();assert_eq!(admitted,want);
        for d in all.iter().filter(|d|want.contains(&d.id)) {
            assert_eq!(policy(d).requirement,"index_etf_or_stock","{}",d.id);assert!(policy(d).market_adaptation);
            assert!(metadata_json(d).contains("\"declared_application\""),"{}",d.id);
            for t in ["SPY","QQQ","SPX","NDX","DJI","NIFTY 50","NIFTYBANK"] {assert!(check(d,t,false).is_none(),"{} {t}",d.id);}
            assert!(check(d,"AAPL",true).is_none() && check(d,"RELIANCE",true).is_none(),"{}",d.id);
        }
    }
    // ---- end Weissman block ----
    // ---- Raschke block (2026-10-24) ----
    #[test]
    fn raschke_review_admits_exact_ids_on_indices_etfs_and_stocks() {
        use crate::books::{lbrtt};
        let all:Vec<StrategyDef>=lbrtt::catalogue().into_iter().collect();
        let mut want:Vec<&str>=lbrtt::REVIEWED.iter().copied().collect();
        let mut admitted:Vec<&str>=all.iter().filter(|d|reviewed_cash_rule(d)).map(|d|d.id).collect();
        admitted.sort();want.sort();assert_eq!(admitted,want);
        for d in all.iter().filter(|d|want.contains(&d.id)) {
            assert_eq!(policy(d).requirement,"index_etf_or_stock","{}",d.id);assert!(policy(d).market_adaptation);
            assert!(metadata_json(d).contains("\"declared_application\""),"{}",d.id);
            for t in ["SPY","QQQ","SPX","NDX","DJI","NIFTY 50","NIFTYBANK"] {assert!(check(d,t,false).is_none(),"{} {t}",d.id);}
            assert!(check(d,"AAPL",true).is_none() && check(d,"RELIANCE",true).is_none(),"{}",d.id);
        }
    }
    // ---- end Raschke block ----
    // ---- CArnold block (2026-10-24) ----
    #[test]
    fn carnold_review_admits_exact_ids_on_indices_etfs_and_stocks() {
        use crate::books::{capttm};
        let all:Vec<StrategyDef>=capttm::catalogue().into_iter().collect();
        let mut want:Vec<&str>=capttm::REVIEWED.iter().copied().collect();
        let mut admitted:Vec<&str>=all.iter().filter(|d|reviewed_cash_rule(d)).map(|d|d.id).collect();
        admitted.sort();want.sort();assert_eq!(admitted,want);
        for d in all.iter().filter(|d|want.contains(&d.id)) {
            assert_eq!(policy(d).requirement,"index_etf_or_stock","{}",d.id);assert!(policy(d).market_adaptation);
            assert!(metadata_json(d).contains("\"declared_application\""),"{}",d.id);
            for t in ["SPY","QQQ","SPX","NDX","DJI","NIFTY 50","NIFTYBANK"] {assert!(check(d,t,false).is_none(),"{} {t}",d.id);}
            assert!(check(d,"AAPL",true).is_none() && check(d,"RELIANCE",true).is_none(),"{}",d.id);
        }
    }
    // ---- end CArnold block ----
    // ---- NDavis block (2026-10-24) ----
    #[test]
    fn ndavis_review_admits_exact_ids_on_indices_etfs_and_stocks() {
        use crate::books::{ndbrm};
        let all:Vec<StrategyDef>=ndbrm::catalogue().into_iter().collect();
        let mut want:Vec<&str>=ndbrm::REVIEWED.iter().copied().collect();
        let mut admitted:Vec<&str>=all.iter().filter(|d|reviewed_cash_rule(d)).map(|d|d.id).collect();
        admitted.sort();want.sort();assert_eq!(admitted,want);
        for d in all.iter().filter(|d|want.contains(&d.id)) {
            assert_eq!(policy(d).requirement,"index_etf_or_stock","{}",d.id);assert!(policy(d).market_adaptation);
            assert!(metadata_json(d).contains("\"declared_application\""),"{}",d.id);
            for t in ["SPY","QQQ","SPX","NDX","DJI","NIFTY 50","NIFTYBANK"] {assert!(check(d,t,false).is_none(),"{} {t}",d.id);}
            assert!(check(d,"AAPL",true).is_none() && check(d,"RELIANCE",true).is_none(),"{}",d.id);
        }
    }
    // ---- end NDavis block ----
    // ---- Siegel block (2026-10-24) ----
    #[test]
    fn siegel_review_admits_exact_ids_on_indices_etfs_and_stocks() {
        use crate::books::{sglr};
        let all:Vec<StrategyDef>=sglr::catalogue().into_iter().collect();
        let mut want:Vec<&str>=sglr::REVIEWED.iter().copied().collect();
        let mut admitted:Vec<&str>=all.iter().filter(|d|reviewed_cash_rule(d)).map(|d|d.id).collect();
        admitted.sort();want.sort();assert_eq!(admitted,want);
        for d in all.iter().filter(|d|want.contains(&d.id)) {
            assert_eq!(policy(d).requirement,"index_etf_or_stock","{}",d.id);assert!(policy(d).market_adaptation);
            assert!(metadata_json(d).contains("\"declared_application\""),"{}",d.id);
            for t in ["SPY","QQQ","SPX","NDX","DJI","NIFTY 50","NIFTYBANK"] {assert!(check(d,t,false).is_none(),"{} {t}",d.id);}
            assert!(check(d,"AAPL",true).is_none() && check(d,"RELIANCE",true).is_none(),"{}",d.id);
        }
    }
    // ---- end Siegel block ----
    // ---- Lydon block (2026-10-24) ----
    #[test]
    fn lydon_review_admits_exact_ids_on_indices_etfs_and_stocks() {
        use crate::books::{lydetf,lydpes};
        let all:Vec<StrategyDef>=lydetf::catalogue().into_iter().chain(lydpes::catalogue()).collect();
        let mut want:Vec<&str>=lydetf::REVIEWED.iter().chain(lydpes::REVIEWED).copied().collect();
        let mut admitted:Vec<&str>=all.iter().filter(|d|reviewed_cash_rule(d)).map(|d|d.id).collect();
        admitted.sort();want.sort();assert_eq!(admitted,want);
        for d in all.iter().filter(|d|want.contains(&d.id)) {
            assert_eq!(policy(d).requirement,"index_etf_or_stock","{}",d.id);assert!(policy(d).market_adaptation);
            assert!(metadata_json(d).contains("\"declared_application\""),"{}",d.id);
            for t in ["SPY","QQQ","SPX","NDX","DJI","NIFTY 50","NIFTYBANK"] {assert!(check(d,t,false).is_none(),"{} {t}",d.id);}
            assert!(check(d,"AAPL",true).is_none() && check(d,"RELIANCE",true).is_none(),"{}",d.id);
        }
    }
    // ---- end Lydon block ----
    // ---- Landry block (2026-10-24) ----
    #[test]
    fn landry_review_admits_exact_ids_on_indices_etfs_and_stocks() {
        use crate::books::{dlost,dl10b};
        let all:Vec<StrategyDef>=dlost::catalogue().into_iter().chain(dl10b::catalogue()).collect();
        let mut want:Vec<&str>=dlost::REVIEWED.iter().chain(dl10b::REVIEWED).copied().collect();
        let mut admitted:Vec<&str>=all.iter().filter(|d|reviewed_cash_rule(d)).map(|d|d.id).collect();
        admitted.sort();want.sort();assert_eq!(admitted,want);
        for d in all.iter().filter(|d|want.contains(&d.id)) {
            assert_eq!(policy(d).requirement,"index_etf_or_stock","{}",d.id);assert!(policy(d).market_adaptation);
            assert!(metadata_json(d).contains("\"declared_application\""),"{}",d.id);
            let us_only=LANDRY_US_ONLY.contains(&d.id);
            for t in ["SPY","QQQ","SPX","NDX","DJI"] {assert!(check(d,t,false).is_none(),"{} {t}",d.id);}
            for t in ["NIFTY 50","NIFTYBANK"] {assert_eq!(check(d,t,false).is_some(),us_only,"{} {t}",d.id);}
            // check() has no market for a native stock (treated as non-US); the served path uses the stock's real market
            if !us_only {assert!(check(d,"AAPL",true).is_none() && check(d,"RELIANCE",true).is_none(),"{}",d.id);}
        }
    }
    // ---- end Landry block ----
    // ---- Pruitt block (2026-10-24) ----
    #[test]
    fn pruitt_review_admits_exact_ids_on_indices_etfs_and_stocks() {
        use crate::books::{pbwts,putg};
        let all:Vec<StrategyDef>=pbwts::catalogue().into_iter().chain(putg::catalogue()).collect();
        let mut want:Vec<&str>=pbwts::REVIEWED.iter().chain(putg::REVIEWED).copied().collect();
        let mut admitted:Vec<&str>=all.iter().filter(|d|reviewed_cash_rule(d)).map(|d|d.id).collect();
        admitted.sort();want.sort();assert_eq!(admitted,want);
        for d in all.iter().filter(|d|want.contains(&d.id)) {
            assert_eq!(policy(d).requirement,"index_etf_or_stock","{}",d.id);assert!(policy(d).market_adaptation);
            assert!(metadata_json(d).contains("\"declared_application\""),"{}",d.id);
            for t in ["SPY","QQQ","SPX","NDX","DJI","NIFTY 50","NIFTYBANK"] {assert!(check(d,t,false).is_none(),"{} {t}",d.id);}
            assert!(check(d,"AAPL",true).is_none() && check(d,"RELIANCE",true).is_none(),"{}",d.id);
        }
    }
    // ---- end Pruitt block ----
    // ---- Hirsch block (2026-10-24) ----
    #[test]
    fn hirsch_review_admits_exact_ids_on_indices_etfs_and_stocks() {
        use crate::books::{hsta};
        let all:Vec<StrategyDef>=hsta::catalogue().into_iter().collect();
        let mut want:Vec<&str>=hsta::REVIEWED.iter().copied().collect();
        let mut admitted:Vec<&str>=all.iter().filter(|d|reviewed_cash_rule(d)).map(|d|d.id).collect();
        admitted.sort();want.sort();assert_eq!(admitted,want);
        for d in all.iter().filter(|d|want.contains(&d.id)) {
            assert_eq!(policy(d).requirement,"index_etf_or_stock","{}",d.id);assert!(policy(d).market_adaptation);
            assert!(metadata_json(d).contains("\"declared_application\""),"{}",d.id);
            let us_only=HIRSCH_US_ONLY.contains(&d.id);
            for t in ["SPY","QQQ","SPX","NDX","DJI"] {assert!(check(d,t,false).is_none(),"{} {t}",d.id);}
            for t in ["NIFTY 50","NIFTYBANK"] {assert_eq!(check(d,t,false).is_some(),us_only,"{} {t}",d.id);}
            // check() has no market for a native stock (treated as non-US); the served path uses the stock's real market
            if !us_only {assert!(check(d,"AAPL",true).is_none() && check(d,"RELIANCE",true).is_none(),"{}",d.id);}
        }
    }
    // ---- end Hirsch block ----
    // ---- Bandy block (2026-10-24) ----
    #[test]
    fn bandy_review_admits_exact_ids_on_indices_etfs_and_stocks() {
        use crate::books::{hbqts,hbiab};
        let all:Vec<StrategyDef>=hbqts::catalogue().into_iter().chain(hbiab::catalogue()).collect();
        let mut want:Vec<&str>=hbqts::REVIEWED.iter().chain(hbiab::REVIEWED).copied().collect();
        let mut admitted:Vec<&str>=all.iter().filter(|d|bandy_reviewed(d)).map(|d|d.id).collect();
        admitted.sort();want.sort();assert_eq!(admitted,want);
        for d in all.iter().filter(|d|want.contains(&d.id)) {
            assert!(reviewed_cash_rule(d),"{}",d.id);
            assert_eq!(policy(d).requirement,"index_etf_or_stock","{}",d.id);assert!(policy(d).market_adaptation);
            assert!(metadata_json(d).contains("\"declared_application\""),"{}",d.id);
            for t in ["SPY","QQQ","SPX","NDX","DJI","NIFTY 50","NIFTYBANK"] {assert!(check(d,t,false).is_none(),"{} {t}",d.id);}
            assert!(check(d,"AAPL",true).is_none() && check(d,"RELIANCE",true).is_none(),"{}",d.id);
        }
    }
    // ---- end Bandy block ----
    // ---- Kestner block (2026-10-24) ----
    #[test]
    fn kestner_review_admits_exact_ids_on_indices_etfs_and_stocks() {
        use crate::books::{lkqts,lkcpt};
        let all:Vec<StrategyDef>=lkqts::catalogue().into_iter().chain(lkcpt::catalogue()).collect();
        let mut want:Vec<&str>=lkqts::REVIEWED.iter().chain(lkcpt::REVIEWED).copied().collect();
        let mut admitted:Vec<&str>=all.iter().filter(|d|kestner_reviewed(d)).map(|d|d.id).collect();
        admitted.sort();want.sort();assert_eq!(admitted,want);
        for d in all.iter().filter(|d|want.contains(&d.id)) {
            assert!(reviewed_cash_rule(d),"{}",d.id);
            assert_eq!(policy(d).requirement,"index_etf_or_stock","{}",d.id);assert!(policy(d).market_adaptation);
            assert!(metadata_json(d).contains("\"declared_application\""),"{}",d.id);
            for t in ["SPY","QQQ","SPX","NDX","DJI","NIFTY 50","NIFTYBANK"] {assert!(check(d,t,false).is_none(),"{} {t}",d.id);}
            assert!(check(d,"AAPL",true).is_none() && check(d,"RELIANCE",true).is_none(),"{}",d.id);
        }
    }
    // ---- end Kestner block ----
    // ---- Wirrick block (2026-10-24) ----
    #[test]
    fn wirrick_review_admits_exact_ids_on_indices_etfs_and_stocks() {
        use crate::books::{swirrick};
        let all:Vec<StrategyDef>=swirrick::catalogue().into_iter().collect();
        let mut want:Vec<&str>=swirrick::REVIEWED.iter().copied().collect();
        let mut admitted:Vec<&str>=all.iter().filter(|d|reviewed_cash_rule(d)).map(|d|d.id).collect();
        admitted.sort();want.sort();assert_eq!(admitted,want);
        for d in all.iter().filter(|d|want.contains(&d.id)) {
            assert_eq!(policy(d).requirement,"index_etf_or_stock","{}",d.id);assert!(policy(d).market_adaptation);
            assert!(metadata_json(d).contains("\"declared_application\""),"{}",d.id);
            for t in ["SPY","QQQ","SPX","NDX","DJI","NIFTY 50","NIFTYBANK"] {assert!(check(d,t,false).is_none(),"{} {t}",d.id);}
            assert!(check(d,"AAPL",true).is_none() && check(d,"RELIANCE",true).is_none(),"{}",d.id);
        }
    }
    // ---- end Wirrick block ----
    // ---- Sperandeo block (2026-10-24) ----
    #[test]
    fn sperandeo_review_admits_exact_ids_on_indices_etfs_and_stocks() {
        use crate::books::{svtvc,svmws};
        let all:Vec<StrategyDef>=svtvc::catalogue().into_iter().chain(svmws::catalogue()).collect();
        let mut want:Vec<&str>=svtvc::REVIEWED.iter().chain(svmws::REVIEWED).copied().collect();
        let mut admitted:Vec<&str>=all.iter().filter(|d|reviewed_cash_rule(d)).map(|d|d.id).collect();
        admitted.sort();want.sort();assert_eq!(admitted,want);
        for d in all.iter().filter(|d|want.contains(&d.id)) {
            assert_eq!(policy(d).requirement,"index_etf_or_stock","{}",d.id);assert!(policy(d).market_adaptation);
            assert!(metadata_json(d).contains("\"declared_application\""),"{}",d.id);
            for t in ["SPY","QQQ","SPX","NDX","DJI","NIFTY 50","NIFTYBANK"] {assert!(check(d,t,false).is_none(),"{} {t}",d.id);}
            assert!(check(d,"AAPL",true).is_none() && check(d,"RELIANCE",true).is_none(),"{}",d.id);
        }
    }
    // ---- end Sperandeo block ----
    // ---- CSmith block (2026-10-24) ----
    #[test]
    fn csmith_review_admits_exact_ids_on_indices_etfs_and_stocks() {
        use crate::books::{csmlt};
        let all:Vec<StrategyDef>=csmlt::catalogue().into_iter().collect();
        let mut want:Vec<&str>=csmlt::REVIEWED.iter().copied().collect();
        let mut admitted:Vec<&str>=all.iter().filter(|d|reviewed_cash_rule(d)).map(|d|d.id).collect();
        admitted.sort();want.sort();assert_eq!(admitted,want);
        for d in all.iter().filter(|d|want.contains(&d.id)) {
            assert_eq!(policy(d).requirement,"index_etf_or_stock","{}",d.id);assert!(policy(d).market_adaptation);
            assert!(metadata_json(d).contains("\"declared_application\""),"{}",d.id);
            for t in ["SPY","QQQ","SPX","NDX","DJI","NIFTY 50","NIFTYBANK"] {assert!(check(d,t,false).is_none(),"{} {t}",d.id);}
            assert!(check(d,"AAPL",true).is_none() && check(d,"RELIANCE",true).is_none(),"{}",d.id);
        }
    }
    // ---- end CSmith block ----
    // ---- Lien block (2026-10-24) ----
    #[test]
    fn lien_review_admits_exact_ids_on_indices_etfs_and_stocks() {
        use crate::books::{kllien};
        let all:Vec<StrategyDef>=kllien::catalogue().into_iter().collect();
        let mut want:Vec<&str>=kllien::REVIEWED.iter().copied().collect();
        let mut admitted:Vec<&str>=all.iter().filter(|d|reviewed_cash_rule(d)).map(|d|d.id).collect();
        admitted.sort();want.sort();assert_eq!(admitted,want);
        for d in all.iter().filter(|d|want.contains(&d.id)) {
            assert_eq!(policy(d).requirement,"index_etf_or_stock","{}",d.id);assert!(policy(d).market_adaptation);
            assert!(metadata_json(d).contains("\"declared_application\""),"{}",d.id);
            for t in ["SPY","QQQ","SPX","NDX","DJI","NIFTY 50","NIFTYBANK"] {assert!(check(d,t,false).is_none(),"{} {t}",d.id);}
            assert!(check(d,"AAPL",true).is_none() && check(d,"RELIANCE",true).is_none(),"{}",d.id);
        }
    }
    // ---- end Lien block ----
    // ---- Arms block (2026-10-24) ----
    #[test]
    fn arms_review_admits_exact_ids_on_indices_etfs_and_stocks() {
        use crate::books::{rwarms};
        let all:Vec<StrategyDef>=rwarms::catalogue().into_iter().collect();
        let mut want:Vec<&str>=rwarms::REVIEWED.iter().copied().collect();
        let mut admitted:Vec<&str>=all.iter().filter(|d|reviewed_cash_rule(d)).map(|d|d.id).collect();
        admitted.sort();want.sort();assert_eq!(admitted,want);
        for d in all.iter().filter(|d|want.contains(&d.id)) {
            assert_eq!(policy(d).requirement,"index_etf_or_stock","{}",d.id);assert!(policy(d).market_adaptation);
            assert!(metadata_json(d).contains("\"declared_application\""),"{}",d.id);
            for t in ["SPY","QQQ","SPX","NDX","DJI","NIFTY 50","NIFTYBANK"] {assert!(check(d,t,false).is_none(),"{} {t}",d.id);}
            assert!(check(d,"AAPL",true).is_none() && check(d,"RELIANCE",true).is_none(),"{}",d.id);
        }
    }
    // ---- end Arms block ----
    // ---- Gregoriou block (2026-10-24) ----
    #[test]
    fn gregoriou_review_admits_exact_ids_on_indices_etfs_and_stocks() {
        use crate::books::gregoriou as g;
        let all=g::catalogue();
        let mut admitted:Vec<&str>=all.iter().filter(|d|reviewed_cash_rule(d)).map(|d|d.id).collect();
        let mut w=g::REVIEWED.to_vec();admitted.sort();w.sort();assert_eq!(admitted,w);
        for d in all.iter().filter(|d|g::reviewed(d.id)) {
            assert_eq!(policy(d).requirement,"index_etf_or_stock","{}",d.id);assert!(policy(d).market_adaptation);
            assert!(metadata_json(d).contains("\"declared_application\""),"{}",d.id);
            for t in ["SPY","QQQ","SPX","NDX","DJI","NIFTY 50","NIFTYBANK"] {assert!(check(d,t,false).is_none(),"{} {t}",d.id);}
            assert!(check(d,"AAPL",true).is_none() && check(d,"RELIANCE",true).is_none(),"{}",d.id);
        }
        assert!(!reviewed_cash_rule(&def("ggth_dmaco","mpts")));
    }
    // ---- end Gregoriou block ----
    // ---- Crisp block (2026-10-24) ----
    #[test]
    fn crisp_review_admits_exact_ids_on_indices_etfs_and_stocks() {
        use crate::books::{mccic,mcmts,mcstis};
        let all:Vec<StrategyDef>=mccic::catalogue().into_iter().chain(mcmts::catalogue()).chain(mcstis::catalogue()).collect();
        let mut admitted:Vec<&str>=all.iter().filter(|d|reviewed_cash_rule(d)).map(|d|d.id).collect();
        admitted.sort();assert_eq!(admitted,mccic::REVIEWED);
        for d in all.iter().filter(|d|mccic::reviewed(d.id)) {
            assert_eq!(policy(d).requirement,"index_etf_or_stock","{}",d.id);assert!(policy(d).market_adaptation);
            assert!(metadata_json(d).contains("\"declared_application\""),"{}",d.id);
            for t in ["SPY","QQQ","SPX","NDX","DJI","NIFTY 50","NIFTYBANK"] {assert!(check(d,t,false).is_none(),"{} {t}",d.id);}
            assert!(check(d,"AAPL",true).is_none() && check(d,"RELIANCE",true).is_none(),"{}",d.id);
        }
    }
    // ---- end Crisp block ----
    // ---- Fabozzi block (2026-10-24) ----
    #[test]
    fn fabozzi_review_admits_exact_ids_on_indices_etfs_and_stocks() {
        use crate::books::{fzhci};
        let all:Vec<StrategyDef>=fzhci::catalogue().into_iter().collect();
        let mut want:Vec<&str>=fzhci::REVIEWED.iter().copied().collect();
        let mut admitted:Vec<&str>=all.iter().filter(|d|reviewed_cash_rule(d)).map(|d|d.id).collect();
        admitted.sort();want.sort();assert_eq!(admitted,want);
        for d in all.iter().filter(|d|want.contains(&d.id)) {
            assert_eq!(policy(d).requirement,"index_etf_or_stock","{}",d.id);assert!(policy(d).market_adaptation);
            assert!(metadata_json(d).contains("\"declared_application\""),"{}",d.id);
            for t in ["SPY","QQQ","SPX","NDX","DJI","NIFTY 50","NIFTYBANK"] {assert!(check(d,t,false).is_none(),"{} {t}",d.id);}
            assert!(check(d,"AAPL",true).is_none() && check(d,"RELIANCE",true).is_none(),"{}",d.id);
        }
    }
    // ---- end Fabozzi block ----
    // ---- Fontanills block (2026-10-24) ----
    #[test]
    fn fontanills_review_admits_exact_ids_on_indices_etfs_and_stocks() {
        use crate::books::{fontanills};
        let all:Vec<StrategyDef>=fontanills::catalogue().into_iter().collect();
        let mut want:Vec<&str>=fontanills::REVIEWED.iter().copied().collect();
        let mut admitted:Vec<&str>=all.iter().filter(|d|reviewed_cash_rule(d)).map(|d|d.id).collect();
        admitted.sort();want.sort();assert_eq!(admitted,want);
        for d in all.iter().filter(|d|want.contains(&d.id)) {
            assert_eq!(policy(d).requirement,"index_etf_or_stock","{}",d.id);assert!(policy(d).market_adaptation);
            assert!(metadata_json(d).contains("\"declared_application\""),"{}",d.id);
            for t in ["SPY","QQQ","SPX","NDX","DJI","NIFTY 50","NIFTYBANK"] {assert!(check(d,t,false).is_none(),"{} {t}",d.id);}
            assert!(check(d,"AAPL",true).is_none() && check(d,"RELIANCE",true).is_none(),"{}",d.id);
        }
    }
    // ---- end Fontanills block ----
    // ---- Thomsett block (2026-10-24) ----
    #[test]
    fn thomsett_review_admits_exact_ids_on_indices_etfs_and_stocks() {
        use crate::books::{mtpos};
        let all:Vec<StrategyDef>=mtpos::catalogue().into_iter().collect();
        let mut want:Vec<&str>=mtpos::REVIEWED.iter().copied().collect();
        let mut admitted:Vec<&str>=all.iter().filter(|d|reviewed_cash_rule(d)).map(|d|d.id).collect();
        admitted.sort();want.sort();assert_eq!(admitted,want);
        for d in all.iter().filter(|d|want.contains(&d.id)) {
            assert_eq!(policy(d).requirement,"index_etf_or_stock","{}",d.id);assert!(policy(d).market_adaptation);
            assert!(metadata_json(d).contains("\"declared_application\""),"{}",d.id);
            for t in ["SPY","QQQ","SPX","NDX","DJI","NIFTY 50","NIFTYBANK"] {assert!(check(d,t,false).is_none(),"{} {t}",d.id);}
            assert!(check(d,"AAPL",true).is_none() && check(d,"RELIANCE",true).is_none(),"{}",d.id);
        }
    }
    // ---- end Thomsett block ----
    // ---- Patel block (2026-10-24) ----
    #[test]
    fn patel_review_admits_exact_ids_on_indices_etfs_and_stocks() {
        use crate::books::{apitc};
        let all:Vec<StrategyDef>=apitc::catalogue().into_iter().collect();
        let mut want:Vec<&str>=apitc::REVIEWED.iter().copied().collect();
        let mut admitted:Vec<&str>=all.iter().filter(|d|reviewed_cash_rule(d)).map(|d|d.id).collect();
        admitted.sort();want.sort();assert_eq!(admitted,want);
        for d in all.iter().filter(|d|want.contains(&d.id)) {
            assert_eq!(policy(d).requirement,"index_etf_or_stock","{}",d.id);assert!(policy(d).market_adaptation);
            assert!(metadata_json(d).contains("\"declared_application\""),"{}",d.id);
            for t in ["SPY","QQQ","SPX","NDX","DJI","NIFTY 50","NIFTYBANK"] {assert!(check(d,t,false).is_none(),"{} {t}",d.id);}
            assert!(check(d,"AAPL",true).is_none() && check(d,"RELIANCE",true).is_none(),"{}",d.id);
        }
    }
    // ---- end Patel block ----
    // ---- DeMark block (2026-10-24) ----
    #[test]
    fn demark_review_admits_exact_ids_on_indices_etfs_and_stocks() {
        use crate::books::{tdwb,tdnsta,tdnmtt};
        let all:Vec<StrategyDef>=tdwb::catalogue().into_iter().chain(tdnsta::catalogue()).chain(tdnmtt::catalogue()).collect();
        let mut want:Vec<&str>=tdwb::REVIEWED.iter().chain(tdnsta::REVIEWED).chain(tdnmtt::REVIEWED).copied().collect();
        let mut admitted:Vec<&str>=all.iter().filter(|d|demark_reviewed(d)).map(|d|d.id).collect();
        admitted.sort();want.sort();assert_eq!(admitted,want);
        for d in all.iter().filter(|d|want.contains(&d.id)) {
            assert!(reviewed_cash_rule(d),"{}",d.id);assert_eq!(policy(d).requirement,"index_etf_or_stock","{}",d.id);assert!(policy(d).market_adaptation);
            assert!(metadata_json(d).contains("\"declared_application\""),"{}",d.id);
            for t in ["SPY","QQQ","SPX","NDX","DJI","NIFTY 50","NIFTYBANK"] {assert!(check(d,t,false).is_none(),"{} {t}",d.id);}
            assert!(check(d,"AAPL",true).is_none() && check(d,"RELIANCE",true).is_none(),"{}",d.id);
        }
    }
    // ---- end DeMark block ----
    // ---- Kroll block (2026-10-24) ----
    #[test]
    fn kroll_review_admits_exact_ids_on_indices_etfs_and_stocks() {
        use crate::books::{skdab};
        let all:Vec<StrategyDef>=skdab::catalogue().into_iter().collect();
        let mut want:Vec<&str>=skdab::REVIEWED.iter().copied().collect();
        let mut admitted:Vec<&str>=all.iter().filter(|d|kroll_reviewed(d)).map(|d|d.id).collect();
        admitted.sort();want.sort();assert_eq!(admitted,want);
        for d in all.iter().filter(|d|want.contains(&d.id)) {
            assert!(reviewed_cash_rule(d),"{}",d.id);assert_eq!(policy(d).requirement,"index_etf_or_stock","{}",d.id);assert!(policy(d).market_adaptation);
            assert!(metadata_json(d).contains("\"declared_application\""),"{}",d.id);
            for t in ["SPY","QQQ","SPX","NDX","DJI","NIFTY 50","NIFTYBANK"] {assert!(check(d,t,false).is_none(),"{} {t}",d.id);}
            assert!(check(d,"AAPL",true).is_none() && check(d,"RELIANCE",true).is_none(),"{}",d.id);
        }
    }
    // ---- end Kroll block ----
    // ---- Kaeppel block (2026-10-24) ----
    #[test]
    fn kaeppel_review_admits_exact_ids_on_indices_etfs_and_stocks() {
        use crate::books::{kpss};
        let all:Vec<StrategyDef>=kpss::catalogue().into_iter().collect();
        let mut want:Vec<&str>=kpss::REVIEWED.iter().copied().collect();
        let mut admitted:Vec<&str>=all.iter().filter(|d|kaeppel_reviewed(d)).map(|d|d.id).collect();
        admitted.sort();want.sort();assert_eq!(admitted,want);
        for d in all.iter().filter(|d|want.contains(&d.id)) {
            assert!(reviewed_cash_rule(d),"{}",d.id);assert_eq!(policy(d).requirement,"index_etf_or_stock","{}",d.id);assert!(policy(d).market_adaptation);
            assert!(metadata_json(d).contains("\"declared_application\""),"{}",d.id);
            let us_only=kpss::US_ONLY.contains(&d.id);
            for t in ["SPY","QQQ","SPX","NDX","DJI"] {assert!(check(d,t,false).is_none(),"{} {t}",d.id);}
            for t in ["NIFTY 50","NIFTYBANK"] {assert_eq!(check(d,t,false).is_some(),us_only,"{} {t}",d.id);}
            assert_eq!(check(d,"RELIANCE",true).is_some(),us_only,"{}",d.id);
        }
    }
    // ---- end Kaeppel block ----
    // ---- Hamon block (2026-10-24) ----
    #[test]
    fn hamon_review_admits_exact_ids_on_indices_etfs_and_stocks() {
        use crate::books::{jhbct};
        let all:Vec<StrategyDef>=jhbct::catalogue().into_iter().collect();
        let mut want:Vec<&str>=jhbct::REVIEWED.iter().copied().collect();
        let mut admitted:Vec<&str>=all.iter().filter(|d|hamon_reviewed(d)).map(|d|d.id).collect();
        admitted.sort();want.sort();assert_eq!(admitted,want);
        for d in all.iter().filter(|d|want.contains(&d.id)) {
            assert!(reviewed_cash_rule(d),"{}",d.id);assert_eq!(policy(d).requirement,"index_etf_or_stock","{}",d.id);assert!(policy(d).market_adaptation);
            assert!(metadata_json(d).contains("\"declared_application\""),"{}",d.id);
            for t in ["SPY","QQQ","SPX","NDX","DJI","NIFTY 50","NIFTYBANK"] {assert!(check(d,t,false).is_none(),"{} {t}",d.id);}
            assert!(check(d,"AAPL",true).is_none() && check(d,"RELIANCE",true).is_none(),"{}",d.id);
        }
    }
    // ---- end Hamon block ----
    // ---- Toghraie block (2026-10-24) ----
    #[test]
    fn toghraie_review_admits_exact_ids_on_indices_etfs_and_stocks() {
        use crate::books::{rgtsec};
        let all:Vec<StrategyDef>=rgtsec::catalogue().into_iter().collect();
        let mut want:Vec<&str>=rgtsec::REVIEWED.iter().copied().collect();
        let mut admitted:Vec<&str>=all.iter().filter(|d|toghraie_reviewed(d)).map(|d|d.id).collect();
        admitted.sort();want.sort();assert_eq!(admitted,want);
        for d in all.iter().filter(|d|want.contains(&d.id)) {
            assert!(reviewed_cash_rule(d),"{}",d.id);assert_eq!(policy(d).requirement,"index_etf_or_stock","{}",d.id);assert!(policy(d).market_adaptation);
            assert!(metadata_json(d).contains("\"declared_application\""),"{}",d.id);
            for t in ["SPY","QQQ","SPX","NDX","DJI","NIFTY 50","NIFTYBANK"] {assert!(check(d,t,false).is_none(),"{} {t}",d.id);}
            assert!(check(d,"AAPL",true).is_none() && check(d,"RELIANCE",true).is_none(),"{}",d.id);
        }
    }
    // ---- end Toghraie block ----
    // ---- Harris block (2026-10-24) ----
    #[test]
    fn harris_review_admits_exact_ids_on_indices_etfs_and_stocks() {
        use crate::books::{mhpst,mhart};
        let all:Vec<StrategyDef>=mhpst::catalogue().into_iter().chain(mhart::catalogue()).collect();
        let mut want:Vec<&str>=mhpst::REVIEWED.iter().chain(mhart::REVIEWED).copied().collect();
        let mut admitted:Vec<&str>=all.iter().filter(|d|harris_reviewed(d)).map(|d|d.id).collect();
        admitted.sort();want.sort();assert_eq!(admitted,want);
        for d in all.iter().filter(|d|want.contains(&d.id)) {
            assert!(reviewed_cash_rule(d),"{}",d.id);assert_eq!(policy(d).requirement,"index_etf_or_stock","{}",d.id);assert!(policy(d).market_adaptation);
            assert!(metadata_json(d).contains("\"declared_application\""),"{}",d.id);
            for t in ["SPY","QQQ","SPX","NDX","DJI","NIFTY 50","NIFTYBANK"] {assert!(check(d,t,false).is_none(),"{} {t}",d.id);}
            assert!(check(d,"AAPL",true).is_none() && check(d,"RELIANCE",true).is_none(),"{}",d.id);
        }
    }
    // ---- end Harris block ----
    // ---- Etzkorn block (2026-10-24) ----
    #[test]
    fn etzkorn_review_admits_exact_ids_on_indices_etfs_and_stocks() {
        use crate::books::{metwo};
        let all:Vec<StrategyDef>=metwo::catalogue().into_iter().collect();
        let mut want:Vec<&str>=metwo::REVIEWED.iter().copied().collect();
        let mut admitted:Vec<&str>=all.iter().filter(|d|etzkorn_reviewed(d)).map(|d|d.id).collect();
        admitted.sort();want.sort();assert_eq!(admitted,want);
        for d in all.iter().filter(|d|want.contains(&d.id)) {
            assert!(reviewed_cash_rule(d),"{}",d.id);assert_eq!(policy(d).requirement,"index_etf_or_stock","{}",d.id);assert!(policy(d).market_adaptation);
            assert!(metadata_json(d).contains("\"declared_application\""),"{}",d.id);
            for t in ["SPY","QQQ","SPX","NDX","DJI","NIFTY 50","NIFTYBANK"] {assert!(check(d,t,false).is_none(),"{} {t}",d.id);}
            assert!(check(d,"AAPL",true).is_none() && check(d,"RELIANCE",true).is_none(),"{}",d.id);
        }
    }
    // ---- end Etzkorn block ----
    // ---- LeBeau block (2026-10-24) ----
    #[test]
    fn lebeau_review_admits_exact_ids_on_indices_etfs_and_stocks() {
        use crate::books::{lebcafm,lebtc,lebtcb,lebws};
        let all:Vec<StrategyDef>=lebcafm::catalogue().into_iter().chain(lebtc::catalogue()).chain(lebtcb::catalogue()).chain(lebws::catalogue()).collect();
        let mut want:Vec<&str>=lebcafm::REVIEWED.iter().chain(lebtc::REVIEWED).chain(lebtcb::REVIEWED).chain(lebws::REVIEWED).copied().collect();
        let mut admitted:Vec<&str>=all.iter().filter(|d|lebeau_reviewed(d)).map(|d|d.id).collect();
        admitted.sort();want.sort();assert_eq!(admitted,want);
        for d in all.iter().filter(|d|want.contains(&d.id)) {
            assert!(reviewed_cash_rule(d),"{}",d.id);assert_eq!(policy(d).requirement,"index_etf_or_stock","{}",d.id);assert!(policy(d).market_adaptation);
            assert!(metadata_json(d).contains("\"declared_application\""),"{}",d.id);
            for t in ["SPY","QQQ","SPX","NDX","DJI","NIFTY 50","NIFTYBANK"] {assert!(check(d,t,false).is_none(),"{} {t}",d.id);}
            assert!(check(d,"AAPL",true).is_none() && check(d,"RELIANCE",true).is_none(),"{}",d.id);
        }
    }
    // ---- end LeBeau block ----
    // ---- Tharp block (2026-10-24) ----
    #[test]
    fn tharp_review_admits_exact_ids_on_indices_etfs_and_stocks() {
        use crate::books::{tharp};
        let all:Vec<StrategyDef>=tharp::catalogue().into_iter().collect();
        let mut want:Vec<&str>=tharp::REVIEWED.iter().copied().collect();
        let mut admitted:Vec<&str>=all.iter().filter(|d|reviewed_cash_rule(d)).map(|d|d.id).collect();
        admitted.sort();want.sort();assert_eq!(admitted,want);
        for d in all.iter().filter(|d|want.contains(&d.id)) {
            assert_eq!(policy(d).requirement,"index_etf_or_stock","{}",d.id);assert!(policy(d).market_adaptation);
            assert!(metadata_json(d).contains("\"declared_application\""),"{}",d.id);
            for t in ["SPY","QQQ","SPX","NDX","DJI","NIFTY 50","NIFTYBANK"] {assert!(check(d,t,false).is_none(),"{} {t}",d.id);}
            assert!(check(d,"AAPL",true).is_none() && check(d,"RELIANCE",true).is_none(),"{}",d.id);
        }
    }
    // ---- end Tharp block ----
    // ---- Person block (2026-10-24) ----
    #[test]
    fn person_review_admits_exact_ids_on_indices_etfs_and_stocks() {
        use crate::books::{jperson,hcta};
        let all:Vec<StrategyDef>=jperson::catalogue().into_iter().chain(hcta::catalogue()).collect();
        let mut want:Vec<&str>=jperson::REVIEWED.iter().chain(hcta::REVIEWED).copied().collect();
        let mut admitted:Vec<&str>=all.iter().filter(|d|reviewed_cash_rule(d)).map(|d|d.id).collect();
        admitted.sort();want.sort();assert_eq!(admitted,want);
        for d in all.iter().filter(|d|want.contains(&d.id)) {
            assert_eq!(policy(d).requirement,"index_etf_or_stock","{}",d.id);assert!(policy(d).market_adaptation);
            assert!(metadata_json(d).contains("\"declared_application\""),"{}",d.id);
            for t in ["SPY","QQQ","SPX","NDX","DJI","NIFTY 50","NIFTYBANK"] {assert!(check(d,t,false).is_none(),"{} {t}",d.id);}
            assert!(check(d,"AAPL",true).is_none() && check(d,"RELIANCE",true).is_none(),"{}",d.id);
        }
    }
    // ---- end Person block ----
    // ---- Kaufman block (2026-10-24) ----
    #[test]
    fn kaufman_review_admits_exact_ids_on_indices_etfs_and_stocks() {
        use crate::books::{kfgei,kfbpm,kfst,kfcd,kftsm};
        let all:Vec<StrategyDef>=kfgei::catalogue().into_iter().chain(kfbpm::catalogue()).chain(kfst::catalogue()).chain(kfcd::catalogue()).chain(kftsm::catalogue()).collect();
        let mut want:Vec<&str>=kfgei::REVIEWED.iter().chain(kfbpm::REVIEWED).chain(kfst::REVIEWED).chain(kfcd::REVIEWED).chain(kftsm::REVIEWED).copied().collect();
        let mut admitted:Vec<&str>=all.iter().filter(|d|kaufman_reviewed(d)).map(|d|d.id).collect();
        admitted.sort();want.sort();assert_eq!(admitted,want);
        for d in all.iter().filter(|d|want.contains(&d.id)) {
            assert!(reviewed_cash_rule(d),"{}",d.id);assert_eq!(policy(d).requirement,"index_etf_or_stock","{}",d.id);assert!(policy(d).market_adaptation);
            assert!(metadata_json(d).contains("\"declared_application\""),"{}",d.id);
            for t in ["SPY","QQQ","SPX","NDX","DJI","NIFTY 50","NIFTYBANK"] {assert!(check(d,t,false).is_none(),"{} {t}",d.id);}
            assert!(check(d,"AAPL",true).is_none() && check(d,"RELIANCE",true).is_none(),"{}",d.id);
        }
    }
    // ---- end Kaufman block ----
    // ---- Appel block (2026-10-24) ----
    #[test]
    fn appel_review_admits_exact_ids_on_indices_etfs_and_stocks() {
        use crate::books::{apta,apmacd,apwms,apsmts,apoi};
        let all:Vec<StrategyDef>=apta::catalogue().into_iter().chain(apmacd::catalogue()).chain(apwms::catalogue()).chain(apsmts::catalogue()).chain(apoi::catalogue()).collect();
        let mut want:Vec<&str>=apta::REVIEWED.iter().chain(apmacd::REVIEWED).chain(apwms::REVIEWED).chain(apsmts::REVIEWED).chain(apoi::REVIEWED).copied().collect();
        let mut admitted:Vec<&str>=all.iter().filter(|d|appel_reviewed(d)).map(|d|d.id).collect();
        admitted.sort();want.sort();assert_eq!(admitted,want);
        for d in all.iter().filter(|d|want.contains(&d.id)) {
            assert!(reviewed_cash_rule(d),"{}",d.id);assert_eq!(policy(d).requirement,"index_etf_or_stock","{}",d.id);assert!(policy(d).market_adaptation);
            assert!(metadata_json(d).contains("\"declared_application\""),"{}",d.id);
            for t in ["SPY","QQQ","SPX","NDX","DJI","NIFTY 50","NIFTYBANK"] {assert!(check(d,t,false).is_none(),"{} {t}",d.id);}
            assert!(check(d,"AAPL",true).is_none() && check(d,"RELIANCE",true).is_none(),"{}",d.id);
        }
    }
    // ---- end Appel block ----
    // ---- Joe Ross block (authors/ross admission, 2026-10-23) ----
    #[test]
    fn joe_ross_reviewed_rules_run_on_indices_etfs_and_stocks_and_references_stay_held() {
        use crate::books::{joe_ross,joe_ross_rules as jr};
        let cat=jr::catalogue();
        assert_eq!(cat.len(),jr::REVIEWED.len());
        for d in &cat {
            let p=policy(d);
            assert_eq!((p.requirement,p.allowed_types,p.market_adaptation),("index_etf_or_stock",&["index","etf","individual_stock"][..],true),"{}",d.id);
            assert!(p.source_basis.contains("declared application"),"{}",d.id);
            for t in ["SPX","NDX","DJI","SPY","QQQ","NIFTY 50","NIFTYBANK"] {assert!(check(d,t,false).is_none(),"{} {t}",d.id);}
            for t in ["AAPL","RELIANCE"] {assert!(check(d,t,true).is_none(),"{} {t}",d.id);}
        }
        for d in joe_ross::catalogue() {
            assert_eq!(policy(&d).requirement,"source_review_required","{}",d.id);
            assert!(check(&d,"SPX",false).is_some_and(|i|i.status=="not_testable"),"{}",d.id);
        }
    }
    // ---- end Joe Ross block ----
    #[test]
    fn exact_protected_elder_account_requires_actual_stocks_and_never_inherits(){
        let d=def("elcim_impulse","elcim");assert!(reviewed_cash_rule(&d));assert_eq!(policy(&d).allowed_types,&["index","etf","individual_stock"]);assert_eq!(policy(&d).execution_scope,"single_symbol_source_account");
        assert!(check(&d,"RELIANCE",true).is_none());assert!(check_native(&d,"AAPL",&NativeEvidence::us_test_fixture()).is_none());
        for tk in ["NIFTY 50","SPX","SPY","QQQ"]{assert!(check(&d,tk,false).is_none());}
        assert!(check(&d,"RELIANCE",false).is_some());
        assert!(!reviewed_cash_rule(&def("elcim_impulse","other")));
        for id in ["elcim_fi2","elcim_new"]{let d=def(id,"elcim");assert!(!reviewed_cash_rule(&d));assert_eq!(policy(&d).requirement,"source_review_required");}
    }
    #[test]
    fn reviewed_burns_keys_require_real_stocks_without_opening_other_methods() {
        for id in ["sbma_sma50_long","sbma_x_5s20s"] {
            let d=def(id,"sbma");assert!(reviewed_cash_rule(&d));
            assert!(check(&d,"RELIANCE",true).is_none());
            assert!(check_native(&d,"AAPL",&NativeEvidence::us_test_fixture()).is_none());
            for symbol in ["NIFTY 50","SPY"] {assert!(check(&d,symbol,false).is_none());}
            assert!(check(&d,"RELIANCE",false).is_some());
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
            let opened=open_instrument_label(&d).is_some();assert_eq!(opened,!matches!(id,"hbqts_ma5_25"|"hbqts_ma1_36"));
            assert_eq!(check(&d,"NIFTY 50",false).is_none(),opened);assert_eq!(check(&d,"SPY",false).is_none(),opened);
        }
        let cmo=def("hbqts_cmo","hbqts");assert!(reviewed_cash_rule(&cmo));assert!(check(&cmo,"SPY",false).is_none());
        assert!(check(&cmo,"NIFTY 50",false).is_none());assert!(check(&cmo,"AAPL",true).is_none());
        // apz, stop_entry and common_signal were admitted in the Bandy block (25 Oct 2026); rotation stays held
        for id in ["hbqts_rotation"] {
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
            let open=open_all(&d); // open-all 23 Oct 2026: live ids run on stocks and every ETF too
            assert_eq!(check(&d,"AAPL",true).is_none(),open);
            assert_eq!(check(&d,"RELIANCE",true).is_none(),open);
            assert_eq!(check(&d,"QQQ",false).is_none(),open);
            assert!(!reviewed_cash_rule(&def(id,"stc9")));
        }
        for id in ["spmt_multi5","spmt_multi9","spmt_mts","spmt_tick","spmt_dt_gapdown","spmt_dt_unch","spmt_dt_gapup","spmt_future_method"] { // spmt_eom_puts: options phase 2
            assert!(!reviewed_cash_rule(&def(id,"spmt")),"{id}");
            assert!(check(&def(id,"spmt"),"SPY",false).is_some());
        }
    }
    #[test]
    fn spmt_second_admission_is_exact_and_symbol_scoped() {
        use crate::books::spmt;
        let cat=spmt::catalogue();
        let admitted:Vec<&str>=cat.iter().filter(|d|reviewed_cash_rule(d)&&!options_p2(d)).map(|d|d.id).collect(); // spmt_eom_puts: options phase 2
        assert_eq!(admitted.len(),spmt::REVIEWED_DAILY.len()+spmt::REVIEWED_AUX.len());
        for &id in spmt::REVIEWED_AUX {
            let d=def(id,"spmt");
            assert_eq!(policy(&d).requirement,if open_all(&d) {"index_etf_or_stock"} else {"declared_index_timing"});assert!(policy(&d).market_adaptation);
            assert!(policy(&d).source_basis.contains("spmt2-admission"));
            for t in ["SPX","SPY"] {assert!(check(&d,t,false).is_none(),"{id} {t}");}
            // NIFTY 50 TRIN / advancers runs are held (user decision 2026-10-21: NYSE thresholds not calibrated for NSE)
            assert_eq!(check(&d,"NIFTY 50",false).is_none(),!matches!(id,"spmt_trin_1"|"spmt_trin_2"|"spmt_trin_3"|"spmt_trin_4"|"spmt_trin_5"|"spmt_nyse_adv"),"{id}");
            let open=open_all(&d); // open-all: VIX-open ids (vix2, vix7) stay off Indian instruments
            let india_ok=open && !nyse_breadth(&d); // VIX India block (24 Oct 2026): vix2 / vix7 read real India VIX opens; NYSE breadth: US only
            assert_eq!(check(&d,"NIFTYBANK",false).is_none(),id=="spmt_eom_und" || india_ok,"{id}");
            assert_eq!(check(&d,"NDX",false).is_none(),id=="spmt_eom_und" || open,"{id}");
            assert_eq!(check(&d,"QQQ",false).is_none(),open,"{id}");
            assert_eq!(check_native(&d,"AAPL",&NativeEvidence::us_test_fixture()).is_none(),open,"{id}");
            assert_eq!(check(&d,"RELIANCE",true).is_none(),india_ok,"{id}");
        }
        // The first batch keeps its policy text and any-index scope.
        assert!(policy(&def("spmt_rsi_2","spmt")).source_basis.contains("STC IX duplicates"));
        assert!(check(&def("spmt_rsi_2","spmt"),"NDX",false).is_none());
    }
    #[test]
    fn bernstein_g3_admits_exact_ids_on_reviewed_instruments() {
        use crate::books::{jbeft,jbhsms,jbsmsw,jbtifm,jbtywr};
        let cat:Vec<StrategyDef>=jbsmsw::catalogue().into_iter().chain(jbhsms::catalogue()).chain(jbeft::catalogue()).chain(jbtywr::catalogue()).chain(jbtifm::catalogue()).chain(crate::books::jbhpp::catalogue()).collect();
        let admitted:Vec<&str>=cat.iter().filter(|d|reviewed_cash_rule(d)).map(|d|d.id).collect();
        assert_eq!(admitted.len(),2+21+3+2+3+2+1,"{admitted:?}");
        assert!(cat.iter().filter(|d|reviewed_cash_rule(d)).all(|d|d.status==Status::Live));
        let us=NativeEvidence::us_test_fixture();let ind=NativeEvidence::india_test_fixture();
        for d in cat.iter().filter(|d|reviewed_cash_rule(d)) {
            let any=!jb_g3_gap(d) && d.book!=jbeft::BOOK;
            for t in ["SPY","QQQ","NDX","DJI","NIFTY 50","NIFTYBANK"] {assert_eq!(check(d,t,false).is_none(),any,"{} {t}",d.id);}
            assert_eq!(check(d,"SPX",false).is_none(),!jb_g3_gap(d),"{} SPX",d.id);
            assert_eq!(check_native(d,"AAPL",&us).is_none(),d.book!=jbeft::BOOK,"{} AAPL",d.id);
            assert_eq!(check_native(d,"RELIANCE",&ind).is_none(),any,"{} RELIANCE",d.id);
            assert!(metadata_json(d).contains("\"declared_application\"")||jb_g3_gap(d),"{}",d.id);
            assert!(!reviewed_cash_rule(&def(d.id,"jb624")));
        }
        // Held: not a complete rule, needs data we do not have, or a duplicate of another module.
        for id in ["jbsmsw_add","jbsmsw_3610_parabolic","jbsmsw_dca_mom42","jbsmsw_dca_biq","jbhsms_sp_1027","jbhsms_stk_sunw","jbhsms_stk_nem","jbhsms_fri_mon","jbhsms_mac","jbhsms_ma200",
            "jbtywr_wheat_0901","jbtywr_tbond_0418","jbtywr_hogs_1030","jbtywr_lumber","jbtifm_jb_gap","jbtifm_dual_orbit","jbtifm_add","jbeft_dow_fm82","jbeft_gold_kd","jbeft_parabolic","jbeft_mom_ma","jbeft_mom_rise_fall","jbeft_oc_osc","jbeft_channel_break","jbeft_range_breakout","jbeft_3_3_channel","jbeft_dsi","jbeft_add"] {
            let d=cat.iter().find(|d|d.id==id).unwrap_or_else(||panic!("{id}"));
            assert!(!reviewed_cash_rule(d),"{id}");assert!(d.status!=Status::Live,"{id}");
        }
    }
    #[test]
    fn options_p2_admits_option_tickers_only() {
        let book=|id:&str|match &id[..4] {"oiss"=>"oiss","otvt"=>"otvt","ccl0"=>"ccl08","mahr"=>"mahrs",_=>"spmt"};
        for &id in crate::options::RULES {
            let d=def(id,book(id));
            assert!(options_p2(&d)&&reviewed_cash_rule(&d)&&held_reason(id).is_none(),"{id}");
            for t in options_tickers(id) {assert!(check(&d,t,false).is_none(),"{id} {t}");}
            for t in ["SPX","NDX","DJI","MDY","SMH","NIFTYIT"] {assert!(check(&d,t,false).is_some(),"{id} {t}");}
            assert!(check(&d,"RELIANCE",true).is_some()&&check(&d,"AAPL",true).is_some(),"{id}: no stock option prices");
        }
        // the US file has no option high/open: OISS (2x stop on the high) and spmt_eom_puts (next-open exit) stay India-only
        assert!(check(&def("oiss_short_put","oiss"),"SPY",false).unwrap().note.contains("16:00 snapshot"));
        assert!(check(&def("spmt_eom_puts","spmt"),"SPY",false).is_some());
        assert!(check(&def("otvt_cvr1_sell_puts","otvt"),"NIFTY 50",false).is_some());
    }
    #[test]
    fn intraday_p1_admits_minute_tickers_only() {
        let cats:Vec<StrategyDef>=crate::books::ss::catalogue().into_iter().chain(crate::books::ats::catalogue()).chain(crate::books::ptjdt::catalogue()).collect();
        for id in INTRADAY_P1 {
            let d=cats.iter().find(|d|d.id==*id).unwrap_or_else(||panic!("{id}"));
            assert!(reviewed_cash_rule(d)&&held_reason(id).is_none(),"{id}");
            assert_eq!(policy(d).requirement,"declared_index_timing");
            for t in INTRADAY_P1_TICKERS {assert!(check(d,t,false).is_none(),"{id} {t}");}
            for t in ["NDX","DJI","QQQ"] {assert_eq!(check(d,t,false).unwrap().status,"needs_data","{id} {t}");}
            assert!(check(d,"AAPL",true).is_some());
        }
        // the equities card and the other ATS day trades stay held
        for id in ["ats_15min_adx_breakout_equities","ptjdt_adx15_equities","ats_morning_call_reversal"] { // ats_10pct_oops: intraday wave 1
            let d=cats.iter().find(|d|d.id==id).unwrap();
            assert!(!intraday_p1(d),"{id}");
        }
    }
    #[test]
    fn intraday_w1_admits_minute_tickers_and_spx_points_only() { // intraday wave 1
        use crate::books as b;
        let cats:Vec<StrategyDef>=[b::ats::catalogue(),b::btp::catalogue(),b::hfm::catalogue(),b::tdnmtt::catalogue(),b::baboops::catalogue(),
            b::tcdt::catalogue(),b::lwdg2::catalogue(),b::lwlts::catalogue(),b::jb30d::catalogue()].into_iter().flatten().collect();
        let w1:Vec<&StrategyDef>=cats.iter().filter(|d|intraday_w1_id(d.id)).collect();
        assert_eq!(w1.len(),INTRADAY_W1.len()+INTRADAY_W1_SPX.len()+68);
        for d in &w1 {
            assert!(intraday_p1(d)&&reviewed_cash_rule(d)&&held_reason(d.id).is_none(),"{}",d.id);
            assert_eq!(policy(d).requirement,"declared_index_timing");
            let spx=intraday_w1_spx_id(d.id);
            for t in INTRADAY_P1_TICKERS {assert_eq!(check(d,t,false).is_none(),!spx||*t=="SPX","{} {t}",d.id);}
            for t in ["NDX","QQQ"] {assert_eq!(check(d,t,false).unwrap().status,"needs_data","{} {t}",d.id);}
            assert!(check(d,"AAPL",true).is_some());
        }
        // not converted: multi-day ORB rows, open-entry tables, the long WREG (admitted daily), STOOPS
        for id in ["tcdt_2bnr_p80_h1","tcdt_idp1","tcdt_oc_nr_re","btp2_wide_range_exhaustion_gap_long","baboops_stoops","ststw_intraday"] {assert!(!intraday_w1_id(id),"{id}");}
    }
    #[test]
    fn ss_review_admits_exact_ids_on_reviewed_symbols_only() {
        use crate::books::ss;
        let cat=ss::catalogue();
        let admitted:Vec<&str>=cat.iter().filter(|d|reviewed_cash_rule(d)&&!intraday_p1(d)).map(|d|d.id).collect(); // intraday phase 1 ids: own test
        assert_eq!(admitted.len(),30);
        assert!(admitted.iter().all(|id|ss::REVIEWED_DAILY.contains(id)));
        for d in cat.iter().filter(|d|reviewed_cash_rule(d)&&!intraday_p1(d)) {
            let stock=ss::STOCK_SHOWN.contains(&d.id);let also=ss::STOCK_ALSO.contains(&d.id);
            let open=open_all(d); // open-all 23 Oct 2026
            assert_eq!(policy(d).requirement,if stock||also||open {"index_etf_or_stock"} else {"declared_index_timing"},"{}",d.id);
            assert_eq!(check(d,"AAPL",true).is_none(),stock||also||(open&&!nyse_breadth(d)),"{}",d.id); // NYSE breadth: native US stocks go through check_native
            assert_eq!(check(d,"QQQ",false).is_none(),stock||open,"{}",d.id);
            let futures=["SPX","NDX","DJI","SPY","NIFTY 50","NIFTYBANK"];
            match d.id {
                "ss_ad_market_buster" => for t in futures {assert_eq!(check(d,t,false).is_none(),!t.starts_with("NIFTY"),"{t}");}, // NYSE breadth: every US instrument
                "ss_ad_market_buster_nse" => for t in futures {assert!(check(d,t,false).is_some(),"{t}");}, // held (NSE breadth)
                _ => for t in futures {assert!(check(d,t,false).is_none(),"{} {t}",d.id);},
            }
            assert!(!reviewed_cash_rule(&def(d.id,"vixr")));
        }
        // Held: gate-withheld, data-missing, discretionary or no trade rule in the book.
        for id in ["ss_turtle_soup","ss_adx_gapper_no_gap","ss_spike_and_ledge","ss_fakeout_shakeout","ss_three_little_indians","ss_wolfe_wave","ss_morning_news_reversal","ss_big_picture_news_reversal","ss_smart_money_index","ss_tick_divergence","ss_tick_extreme_next_day","ss_arms_index_5ma","ss_arms_index_5ma_nse","ss_study_fresh_adx_buy","ss_study_fresh_adx_sell"] {
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
            let open=open_all(d) || vix_india(d); // open-all 23 Oct 2026 + VIX India 24 Oct 2026
            let india=d.id!="vixr_cvr11"; // VIX India: CVR 11 held on Indian instruments (exact-tie difference)
            assert_eq!(policy(d).requirement,if open {"index_etf_or_stock"} else {"declared_index_timing"});
            assert!(check(d,"SPY",false).is_none() && check(d,"SPX",false).is_none(),"{}",d.id);
            assert_eq!(check(d,"NIFTY 50",false).is_none(),india,"{}",d.id);
            for t in ["NDX","DJI","QQQ","DIA"] {assert_eq!(check(d,t,false).is_none(),open,"{} {t}",d.id);}
            assert_eq!(check(d,"NIFTY BANK",false).is_none(),open && india,"{}",d.id);
            assert_eq!(check_native(d,"AAPL",&NativeEvidence::us_test_fixture()).is_none(),open,"{}",d.id);
            assert_eq!(check_native(d,"RELIANCE",&NativeEvidence::india_test_fixture()).is_none(),open && india,"{}",d.id);
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
            assert_eq!(policy(d).requirement,if open_all(d) {"index_etf_or_stock"} else {"declared_index_timing"});
            let nse=d.id.ends_with("_nse");
            assert_eq!(check(d,"SPY",false).is_none(),!nse,"{}",d.id);
            assert_eq!(check(d,"SPX",false).is_none(),!nse,"{}",d.id);
            assert_eq!(check(d,"NIFTY 50",false).is_none(),d.id.starts_with("ptjmt_cvr"),"{}",d.id); // *_nse held; CVR rules: VIX India block
            let open=open_all(d); // open-all 23 Oct 2026
            for t in ["NDX","DJI","QQQ","DIA"] {assert_eq!(check(d,t,false).is_none(),open,"{} {t}",d.id);}
            assert_eq!(check(d,"NIFTY BANK",false).is_none(),open && d.id.starts_with("ptjmt_cvr"),"{}",d.id);
            assert_eq!(check_native(d,"AAPL",&NativeEvidence::us_test_fixture()).is_none(),open,"{}",d.id);
            assert_eq!(check(d,"RELIANCE",true).is_none(),open && d.id.starts_with("ptjmt_cvr"),"{}",d.id);
        }
        for id in ["ptjmt_cvr4","ptjmt_cvr4_friday"] {
            let d=cat.iter().find(|d|d.id==id).unwrap();
            assert!(!reviewed_cash_rule(d));
            assert_eq!(policy(d).requirement,"source_review_required");
            assert!(check(d,"SPY",false).is_some());
        }
    }
    #[test]
    fn bernstein_g1_review_admits_exact_ids_and_routes() {
        use crate::books::{jb624,jbcaf,jbssf};
        let all:Vec<StrategyDef>=jb624::catalogue().into_iter().chain(jbcaf::catalogue()).chain(jbssf::catalogue()).collect();
        let admitted:Vec<&str>=all.iter().filter(|d|reviewed_cash_rule(d)).map(|d|d.id).collect();
        let want:Vec<&str>=jb624::REVIEWED.iter().chain(jbcaf::REVIEWED).chain(jbssf::REVIEWED).copied().collect();
        assert_eq!(admitted,want);
        for d in all.iter().filter(|d|reviewed_cash_rule(d)) {
            assert!(metadata_json(d).contains("\"declared_application\""),"{}",d.id);assert!(policy(d).market_adaptation);
            if d.id=="jb624_fri_mon" {
                assert!(check(d,"SPX",false).is_none());
                for t in ["SPY","NDX","DJI","NIFTY 50"] {assert!(check(d,t,false).is_some(),"{t}");}
                assert!(check(d,"AAPL",true).is_some());continue;
            }
            assert_eq!(policy(d).requirement,"index_etf_or_stock");
            for t in ["SPY","QQQ","SPX","NDX","DJI","NIFTY 50","NIFTYBANK"] {assert!(check(d,t,false).is_none(),"{} {t}",d.id);}
            assert!(check(d,"AAPL",true).is_none() && check(d,"RELIANCE",true).is_none(),"{}",d.id);
        }
        // Held: discretionary, intraday, futures/COT data, or incomplete in the book.
        for d in all.iter().filter(|d|!want.contains(&d.id)) {
            assert!(!reviewed_cash_rule(d));assert_eq!(policy(d).requirement,"source_review_required","{}",d.id);
        }
        assert!(!reviewed_cash_rule(&def("jb624_pop","jbudt")));
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
        // connors-final 26 Oct 2026: admitted on NIFTY 50 only (India VIX 1-minute series); US / other instruments refused.
        let d=def("cwin_vix_gap_window","cwin");
        assert!(!reviewed_cash_rule(&d));assert_eq!(policy(&d).requirement,"declared_index_timing");
        assert!(check(&d,"NIFTY 50",false).is_none());
        for t in ["SPY","SPX","NIFTYBANK","QQQ"] {assert!(check(&d,t,false).is_some(),"{t}");}
        assert!(check_native(&d,"AAPL",&NativeEvidence::us_test_fixture()).is_some() && check_native(&d,"RELIANCE",&NativeEvidence::india_test_fixture()).is_some());
        assert!(metadata_json(&d).contains("India VIX in place of the CBOE VIX"));
    }
    #[test]
    fn ws5_review_admits_exact_ids_on_reviewed_symbols_only() {
        use crate::books::ws5;
        let cat=ws5::catalogue();
        let admitted:Vec<&str>=cat.iter().filter(|d|reviewed_cash_rule(d)).map(|d|d.id).collect();
        assert_eq!(admitted,ws5::REVIEWED_DAILY);
        for d in cat.iter().filter(|d|reviewed_cash_rule(d)) {
            let timing=ws5::TIMING.contains(&d.id);let p=policy(d);
            let held=timing && !open_all(d); // open-all 23 Oct 2026: both timing rules run everywhere
            assert_eq!(p.requirement,if held {"declared_index_timing"} else {"index_etf_or_stock"},"{}",d.id);assert!(p.market_adaptation);
            for t in ["SPX","SPY","NIFTY 50"] {assert!(check(d,t,false).is_none(),"{} {t}",d.id);}
            for t in ["NDX","DJI","QQQ","NIFTYBANK"] {assert_eq!(check(d,t,false).is_none(),!held,"{} {t}",d.id);}
            // User decision 2026-10-21: Windows also run on actual stocks without the filters that cannot be applied.
            assert_eq!(check(d,"AAPL",true).is_none() && check(d,"RELIANCE",true).is_none(),!held,"{}",d.id);
            assert_eq!(check_native(d,"AAPL",&NativeEvidence::us_test_fixture()).is_none(),!held,"{}",d.id);
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
        let admitted:Vec<&str>=cat.iter().filter(|d|reviewed_cash_rule(d)&&!options_p2(d)).map(|d|d.id).collect(); // options phase 2 apart
        assert_eq!(admitted.len(),ccl08::REVIEWED_DAILY.len());
        let ok=|id:&'static str,t:&str,stock:bool|check(&def(id,"ccl08"),t,stock).is_none();
        for id in ccl08::REVIEWED_DAILY {
            let dt=ccl08::day_trade(id);
            for t in ["SPX","SPY","NIFTY 50","NIFTYBANK"] {assert!(ok(id,t,false),"{id} {t}");}
            let open=dt || open_all(&def(id,"ccl08")); // open-all 23 Oct 2026: TPS also on stocks
            assert_eq!(ok(id,"RELIANCE",true),open,"{id}");
            assert!(check_native(&def(id,"ccl08"),"AAPL",&NativeEvidence::us_test_fixture()).is_none()==open,"{id}");
            assert_eq!(policy(&def(id,"ccl08")).requirement,if open {"index_etf_or_stock"} else {"index_or_etf"});
            assert_eq!(metadata_json(&def(id,"ccl08")).contains("\"declared_application\":\"Book shows these day trades on US stocks"),dt);
        }
        for id in ["ccl08_crdt_emini",/* ccl08_weekly_rsi_options: options phase 2 */"ccl08_quick_scalp","ccl08_adx_quiet","ccl08_busted_options"] {
            assert!(!reviewed_cash_rule(&def(id,"ccl08")),"{id}");
        }
    }
    #[test]
    fn ats_review_admits_exact_ids_on_reviewed_symbols_only() {
        use crate::books::ats;
        let cat=ats::catalogue();
        let admitted:Vec<&str>=cat.iter().filter(|d|reviewed_cash_rule(d)&&!intraday_p1(d)&&!cbp_us(d)).map(|d|d.id).collect(); // intraday phase 1 ids: own test; CBP: connors-final
        assert_eq!(admitted.len(),ats::REVIEWED_DAILY.len());
        assert!(admitted.iter().all(|id|ats::REVIEWED_DAILY.contains(id)));
        let ok=|id:&'static str,t:&str|check(&def(id,"ats"),t,false).is_none();
        for t in ["SPX","SPY"] {for id in ["ats_cvr1","ats_cvr2","ats_cvr3","ats_trin_thrust","ats_padi","ats_hv_thrust","ats_cvr_equities"] {assert!(ok(id,t),"{id} {t}");}}
        for id in ["ats_cvr2","ats_cvr3","ats_hv_thrust"] {assert!(ok(id,"NIFTY 50"),"{id}");}
        for id in ["ats_trin_thrust_nse","ats_padi_nse"] {assert!(!ok(id,"NIFTY 50"),"{id}");} // held (NSE breadth)
        for id in ["ats_trin_thrust","ats_padi"] {assert!(!ok(id,"NIFTY 50"),"{id}");}
        for id in ["ats_cvr1","ats_cvr_equities"] {assert!(ok(id,"NIFTY 50") && ok(id,"NIFTYBANK"),"{id}");} // VIX India block
        for id in ["ats_trin_thrust_nse","ats_padi_nse"] {assert!(!ok(id,"SPX") && !ok(id,"SPY"),"{id}");}
        for id in ["ats_cvr1","ats_cvr2"] {assert!(ok(id,"QQQ") && ok(id,"NIFTYMIDCAP100"),"{id}");} // VIX India block
        assert!(ok("ats_hv_thrust","QQQ") && ok("ats_hv_thrust","NIFTYMIDCAP100") && check(&def("ats_hv_thrust","ats"),"RELIANCE",true).is_none()); // open-all
        for id in ["ats_1234","ats_8day_hl_reversal","ats_smtp","ats_two_for_one","ats_gipsons"] {
            assert_eq!(policy(&def(id,"ats")).requirement,"index_etf_or_stock");
            assert!(ok(id,"NIFTY 50") && ok(id,"NIFTYMIDCAP100") && !ok(id,"AAPL"),"{id}");
            assert!(check(&def(id,"ats"),"RELIANCE",true).is_none() && check_native(&def(id,"ats"),"AAPL",&NativeEvidence::us_test_fixture()).is_none(),"{id}");
            assert!(metadata_json(&def(id,"ats")).contains("\"stock_adaptation\":\"Actual stocks are admitted"));
            assert!(metadata_json(&def(id,"ats")).contains("\"declared_application\""));
        }
        // CVR I on stocks: US stocks (CBOE VIX) and, VIX India block (24 Oct 2026), Indian stocks (real India VIX opens)
        assert!(check_native(&def("ats_cvr_equities","ats"),"AAPL",&NativeEvidence::us_test_fixture()).is_none());
        assert!(check_native(&def("ats_cvr_equities","ats"),"RELIANCE",&NativeEvidence::india_test_fixture()).is_none());
        assert_eq!(policy(&def("ats_dvmtm","ats")).requirement,"index_etf_or_stock");
        for id in ["ats_hv_ratio_setup","ats_late_earnings","ats_torpedoes","ats_hv_ratio_short_strangle","ats_cvr_options","ats_split_call_selling","ats_sector_options_overpriced",/* ats_15min_adx_breakout: intraday phase 1 admission */"ats_15min_adx_breakout_equities",/* ats_10pct_oops, ats_wide_range_gap: intraday wave 1 */"ats_morning_call_reversal","ats_flag_within_a_flag","ats_short_term_decline_playbook","ats_large_range_day_combo","ats_after_losses"] {
            assert!(!reviewed_cash_rule(&def(id,"ats")),"{id}");
        }
    }
    #[test]
    fn hfm_otvt_review_admits_exact_ids_on_reviewed_symbols_only() {
        use crate::books::{hfm,otvt};
        let admitted:Vec<&str>=hfm::catalogue().iter().chain(otvt::catalogue().iter()).filter(|d|reviewed_cash_rule(d)&&!intraday_p1(d)&&!options_p2(d)).map(|d|d.id).collect(); // hfm_vix: intraday wave 1; otvt_cvr1_sell_puts: options phase 2
        assert_eq!(admitted,["hfm_chadtp","hfm_chadtp_nse","hfm_ndxspx","otvt_cvr1_long","otvt_cvr1_short","otvt_hv_thrust"]);
        let ok=|id:&'static str,b:&'static str,t:&str|check(&def(id,b),t,false).is_none();
        for t in ["SPX","SPY"] {for (id,b) in [("hfm_chadtp","hfm"),("otvt_cvr1_long","otvt"),("otvt_cvr1_short","otvt"),("otvt_hv_thrust","otvt")] {assert!(ok(id,b,t),"{id} {t}");}}
        for t in ["NDX","DJI","NIFTY 50","NIFTYBANK"] {assert!(ok("otvt_hv_thrust","otvt",t),"{t}");}
        assert!(!ok("hfm_chadtp_nse","hfm","NIFTY 50") && !ok("hfm_chadtp_nse","hfm","SPX") && !ok("hfm_chadtp","hfm","NIFTY 50"));
        // open-all 23 Oct 2026: CVR I sides run on every US instrument (CBOE VIX), never on Indian ones (VIX open)
        // VIX India block (24 Oct 2026): also on Indian instruments (real India VIX opens from 1-minute data)
        for id in ["otvt_cvr1_long","otvt_cvr1_short"] {assert!(ok(id,"otvt","NIFTY 50") && ok(id,"otvt","QQQ") && check(&def(id,"otvt"),"RELIANCE",true).is_none(),"{id}");}
        assert!(ok("otvt_hv_thrust","otvt","QQQ") && ok("otvt_hv_thrust","otvt","NIFTYMIDCAP100") && ok("hfm_chadtp","hfm","QQQ")); // NYSE breadth block (26 Oct 2026): every US instrument
        assert!(check(&def("otvt_hv_thrust","otvt"),"RELIANCE",true).is_none());
        // NDX-SPX: the NASDAQ 100 vs S&P 500 signal, traded on SPX, SPY and DJI only (connors-leftovers)
        for t in ["SPX","SPY","DJI"] {assert!(ok("hfm_ndxspx","hfm",t),"{t}");}
        for t in ["NDX","QQQ","NIFTY 50","NIFTYBANK"] {assert!(!ok("hfm_ndxspx","hfm",t),"{t}");}
        assert!(check(&def("hfm_ndxspx","hfm"),"AAPL",true).is_some());
        for (id,b) in [("hfm_hv","hfm"),("hfm_und","hfm"),("hfm_und_long","hfm"),("hfm_und_short","hfm"),("hfm_news","hfm"),/* hfm_vix: intraday wave 1 */("otvt_cvr1","otvt")] { // otvt_cvr1_sell_puts: options phase 2 // ptjdt_adx15_breakout: intraday phase 1 admission
            assert!(!reviewed_cash_rule(&def(id,b)),"{id}");assert_eq!(policy(&def(id,b)).requirement,"source_review_required","{id}");
        }
    }
    #[test]
    fn connors_leftovers_open_exact_ids_and_hold_the_rest_with_card_reasons() {
        for (id,b) in [("s12345","stc9"),("ststw_cum_rsi_35","ststw"),("ststw_cum_rsi_50","ststw")] {
            let d=def(id,b);assert!(open_instrument_label(&d).is_some(),"{id}");
            assert_eq!(policy(&d).allowed_types,&["index","etf","individual_stock"],"{id}");
            for (t,native) in [("SPY",false),("SPX",false),("NIFTY 50",false),("AAPL",true),("RELIANCE",true)] {assert!(check(&d,t,native).is_none(),"{id} {t}");}
            assert!(metadata_json(&d).contains("declared_application"),"{id}");
        }
        assert!(open_instrument_label(&def("s12345","stc9")).unwrap().contains("leveraged"));
        assert!(open_instrument_label(&def("s12345","other")).is_none());
        for id in ["ss_smart_money_index", /* ss_momentum_pinball: intraday phase 1 admission */"ss_arms_index_5ma","ss_arms_index_5ma_nse",/* oiss_plus_three_*: options phase 2 admission */"spmt_tick","spmt_multi5","spmt_multi9","spmt_mts","spmt_dt_gapdown","spmt_dt_unch","spmt_dt_gapup","cwin_vix_gap_window","hfm_hv","hfm_und","btp_bottom_reversal_long","btp_momentum_gap"] {
            assert!(held_reason(id).is_some_and(|r|r.starts_with("Held: ")),"{id}");
        }
        for id in ["hfm_ndxspx","s12345","price","ststw_cum_rsi_35","ststw_cum_rsi_50","ststw_vix_stretch"] {assert!(held_reason(id).is_none(),"{id}");}
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
        // open-all 23 Oct 2026: every US instrument (CBOE VIX open); never Indian ones (India VIX open repeats the close)
        for t in ["QQQ","NDX"] {assert!(ok("ststw_vix_rsi","ststw",t,false),"{t}");}
        assert!(ok("ststw_vix_rsi","ststw","NIFTY 50",false)); // VIX India block (24 Oct 2026)
        assert!(ok("ststw_vix_rsi","ststw","RELIANCE",true));
        assert_eq!(policy(&def("ststw_vix_rsi","ststw")).requirement,"index_etf_or_stock");
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
        let admitted:Vec<&str>=cat.iter().filter(|d|reviewed_cash_rule(d)&&!intraday_p1(d)&&!cbp_us(d)).map(|d|d.id).collect(); // intraday wave 1 ids: own test; CBP: connors-final
        assert_eq!(admitted,crate::books::btp::REVIEWED_DAILY);
        for d in cat.iter().filter(|d|reviewed_cash_rule(d)&&!intraday_p1(d)&&!cbp_us(d)) {
            let stock=crate::books::btp::STOCK_ADMITTED.contains(&d.id) || open_all(d); // open-all 23 Oct 2026
            let p=policy(d);assert_eq!(p.requirement,if stock {"index_etf_or_stock"} else {"index_or_etf"});assert!(p.market_adaptation);
            for t in ["SPY","QQQ","MDY","SPX","NDX","DJI","NIFTY 50","NIFTYBANK"] {assert!(check(d,t,false).is_none(),"{} {t}",d.id);}
            assert_eq!(check(d,"AAPL",true).is_none() && check(d,"RELIANCE",true).is_none(),stock,"{}",d.id);
            assert!(check(d,"NIFTY FUTURE",false).is_some());
            assert_eq!(metadata_json(d).contains("1/8-dollar ticks"),crate::books::btp::STOCK_ADMITTED.contains(&d.id),"{}",d.id);
            assert!(metadata_json(d).contains("\"declared_application\":\"Book tested or illustrated on futures and US stocks"));
            assert!(!reviewed_cash_rule(&def(d.id,"ats")));
        }
        for id in ["btp_bottom_reversal_long","btp_momentum_gap","btp_large_range_day_strangle","btp_large_range_day_study"/* btp2 10% Oops x3, WREG sells: intraday wave 1 */] {
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
        assert_eq!(policy(&def("kftsm_pf","kftsm")).requirement,"source_review_required"); // kftsm_parabolic admitted 2026-10-24 (Kaufman block)
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
        // kftsm_parabolic, _donchian_4wk, _moon, _multi_tf admitted 2026-10-24 (Kaufman block test)
        for id in ["kftsm_pf","kftsm_new_adaptive_method"] {
            let d=def(id,"kftsm");
            assert_eq!(policy(&d).requirement,"source_review_required");
            assert!(metadata_json(&d).contains("\"source_rule_reviewed\":false"));
        }
        // open-all 23 Oct 2026: the calendar rules also run on actual stocks
        assert!(check(&def("kftsm_hirsch","kftsm"),"RELIANCE",true).is_none());
        assert!(check(&def("kftsm_month_end","kftsm"),"RELIANCE",true).is_none());
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
        assert!(check(&d,"RELIANCE",true).is_none() && check(&d,"SPY",false).is_none()); // open-all 23 Oct 2026
        assert!(check(&def("r25_unlisted","rsi25"),"SPY",false).is_some());
        let etf=def("hpetf_3dhl_long","hpetf");
        assert!(check(&etf,"SPY",false).is_none());
        assert!(check(&etf,"IWM",false).is_none());
        assert!(check(&etf,"DJI",false).is_none());
        assert!(check(&etf,"NIFTY 50",false).is_none());
        assert!(check(&etf,"RELIANCE",true).is_none());
        // Only listed ids are opened: an unlisted rule of the same book keeps its old admission.
        let unlisted=def("hpetf_3dhl_short","hpetf");assert!(open_instrument_label(&unlisted).is_none());
        assert_eq!(policy(&unlisted).allowed_types,base_policy(&unlisted).allowed_types);
        let macs=def("lkqts_macs","lkqts");assert!(open_instrument_label(&macs).is_none());
    }
    #[test]
    fn radge_plain_review_does_not_admit_stops_or_other_instruments() {
        let d=def("radge_oc_long","radge");
        assert!(reviewed_cash_rule(&d));
        let p=policy(&d);
        assert_eq!(base_policy(&d).requirement,"index");
        assert_eq!(p.allowed_types,&["index","etf","individual_stock"]); // open-all 23 Oct 2026
        assert!(p.market_adaptation);
        for t in ["NIFTY 50","SPX","SPY","NIFTYBANK"] {assert!(check(&d,t,false).is_none(),"{t}");}
        assert!(check(&d,"RELIANCE",true).is_none());
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
                assert_eq!(base_policy(&d).requirement,"index");
                assert_eq!(p.allowed_types,&["index","etf","individual_stock"]); // open-all 23 Oct 2026
                assert!(p.market_adaptation);
                for ticker in ["NIFTY 50","SPX","SPY","QQQ"] {assert!(check(&d,ticker,false).is_none());}
                for ticker in ["RELIANCE","AAPL"] {assert!(check(&d,ticker,true).is_none());}
                assert!(metadata_json(&d).contains("\"source_rule_reviewed\":true"));
                assert!(metadata_json(&d).contains("\"source_performance_reproduced\":false"));
                assert!(!reviewed_cash_rule(&def(d.id,"kftsm")));
                assert!(!reviewed_cash_rule(&def(d.id,"radge")));
            } else {
                // 25 Oct 2026: the other 24 specifications have their own review (Lakonishok block), not an inherited one
                assert!(crate::books::bllstr::reviewed(d.id) && lakonishok_reviewed(&d),"{} inherited a review",d.id);
                assert_eq!(policy(&d).requirement,"index_etf_or_stock");
            }
        }
    }
    #[test]
    fn leverage_futures_options_and_portfolios_cannot_become_cash_backtests() {
        // s12345 is opened as a labelled declared application (connors-leftovers), see its own test
        for (id,book) in [/* lwdg2_0105: intraday wave 1 */("lev_calls","stc9"),("hmrw_sp500_rotation","hmrw"),("alpha_weekly_mr","alpha")] {
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
        assert_eq!(base_policy(&def("lwrsr_oct_apr","lwrsr")).requirement,"index");
        assert_eq!(policy(&def("lwrsr_stock_selection","lwrsr")).requirement,"portfolio");
        assert_eq!(policy(&def("alpha_spy_rsi4_mr","alpha")).requirement,"index_etf_or_stock");
        assert_eq!(policy(&def("alpha_weekly_mr_single","alpha")).requirement,"source_review_required");
        // Appendix A1 trend rule: single-instrument, no ranking -> actual ETF, index as a declared application; never a stock.
        let tf=def("alpha_trend_6m","alpha");
        assert!(reviewed_cash_rule(&tf));assert_eq!(base_policy(&tf).requirement,"index_or_etf");assert!(policy(&tf).market_adaptation);
        for t in ["SPY","QQQ","SMH","SPX","NIFTY 50","NIFTYBANK"] {assert!(check(&tf,t,false).is_none(),"{t}");}
        assert!(check(&tf,"RELIANCE",true).is_none()); // open-all 23 Oct 2026
        assert!(metadata_json(&tf).contains("\"declared_application\":\"Book tested on nine US sector ETFs"));
        for id in ["alpha_weekly_mr_single","alpha_weekly_mr"] {assert!(!reviewed_cash_rule(&def(id,"alpha")),"{id}");}
        assert_eq!(policy(&def("radge_ma220","radge")).requirement,"futures");
        assert_eq!(base_policy(&def("radge_oc_long","radge")).requirement,"index");
        assert_eq!(policy(&def("jc5dm_long","jc5dm")).requirement,"individual_stock");
        assert!(check(&def("jc5dm_long","jc5dm"),"NIFTY 50",false).is_some());
    }
    #[test]
    fn mixed_stock_etf_and_index_book_uses_the_specific_rule_population() {
        assert!(check(&def("ststw_rsi2_under5","ststw"),"NIFTY 50",false).is_none());
        assert!(check(&def("ststw_rsi2_under5","ststw"),"SPY",false).is_none()); // open-all 23 Oct 2026
        assert!(check(&def("ststw_double7","ststw"),"SPY",false).is_none());
        assert!(check(&def("ststw_double7","ststw"),"NIFTY 50",false).is_none());
        assert!(check(&def("ststw_sp_short","ststw"),"SPY",false).is_none());
        assert!(check(&def("ststw_sp_short","ststw"),"NIFTY 50",false).is_none());
        assert!(check(&def("ststw_sp_short","ststw"),"RELIANCE",true).is_none()); // open-all 23 Oct 2026
        // Current snapshot stock prices do not enable the absent original pooled universe.
        assert!(check(&def("ststw_intraday_drops_highs","ststw"),"RELIANCE",true).is_some());
    }
    #[test]
    fn source_dollar_price_and_tick_rules_are_not_reinterpreted_as_rupees() {
        for (id,book) in [("afactor_baseline","afactor"),("jc5dm_long","jc5dm"),("jc5dm_short","jc5dm"),
            ("r3r4_lowprice","r3r4"),("r3r4_lowprice_scale","r3r4"),
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
        assert!(check_native(&def("hpetf_3dhl_long","hpetf"),"SPY",&e).is_none(),"Opened 22 Oct 2026: a trusted stock is admitted");
        assert!(check_native(&def("ats_cbp","ats"),"AAPL",&e).is_none(),"connors-final: CBP runs on US stocks with the literal $20");
        for (id,book) in [("jc5dm_long","jc5dm")] {
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
        for ticker in ["NIFTY 50","SPY"] {assert!(check(&mama,ticker,false).is_none());}
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
            let p=policy(&d);assert!(reviewed_cash_rule(&d));assert_eq!(p.allowed_types,&["index","etf","individual_stock"]);
            assert!(p.market_adaptation);assert!(p.source_basis.contains("FOUR-LONG"));
            assert!(check_native(&d,"AAPL",&NativeEvidence::us_test_fixture()).is_none());
            for ticker in ["NIFTY 50","SPX","SPY","QQQ"] {assert!(check(&d,ticker,false).is_none());}
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
            assert!(reviewed_cash_rule(def)); assert_eq!(p.allowed_types,&["index","etf","individual_stock"]);
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
            assert_eq!(p.allowed_types,&["index","etf","individual_stock"]);assert_eq!(p.execution_scope,"single_symbol_stock_price_research");
            assert!(p.source_basis.contains("declared conventions"));assert!(p.source_basis.contains("nonpositive"));
            for symbol in ["SPX","NDX","SPY","QQQ","NIFTY 50"] {assert!(check(&d,symbol,false).is_none());}
            for symbol in ["AAPL","NIFTY31JAN2008FUT"] {assert!(check(&d,symbol,false).is_some());}
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
            // VIX India block (24 Oct 2026): CVR I on stocks also runs on Indian instruments (real India VIX opens from 2015).
            for tk in ["NIFTY 50","NIFTYBANK","SPX","SPY","QQQ"] {assert!(check(&d,tk,false).is_none(),"{id} {tk}");}
            assert!(check(&d,"NIFTY FUTURE",false).is_some());
            let json=metadata_json(&d);assert!(json.contains("\"declared_application\":\"Book tested on US stocks"));assert!(json.contains("\"market_adaptation\":true")&&json.contains("\"source_performance_reproduced\":false"));
            // Native stocks keep the previous result, including the USD-unit holds.
            assert_eq!(check(&d,"RELIANCE",true).is_none(),p.native_currency_constraint.is_none(),"{id}"); // VIX India block: ats_cvr_equities too
        }
        for id in ["r3r4_midcap","r3r4_midcap_scale"] {
            let d=find(id);assert_eq!(policy(&d).allowed_types,&["individual_stock"]);
            for tk in ["NIFTY 50","SPX","SPY"] {assert!(check(&d,tk,false).unwrap().note.contains("upper bound of 30 is never met"),"{id} {tk}");}
            assert!(check(&d,"RELIANCE",true).is_none());
        }
    }
    #[test]
    fn ranking_and_portfolio_methods_stay_stock_or_portfolio_only() {
        // connors-final 26 Oct 2026: ats_cbp now runs on US stocks, indices and ETFs ($20 kept, US only), see cbp_us
        assert!(policy(&find("ats_cbp")).allowed_types.contains(&"individual_stock") && check(&find("ats_cbp"),"NIFTY 50",false).is_some() && check(&find("ats_cbp"),"SPY",false).is_none());
        for id in ["alpha_rising_assets","alpha_weekly_mr","alpha_etf_avalanches","alpha_dyn_treasuries","alpha_formula_portfolio","hmrw_sp500_rotation","afactor_portfolio"] {
            let d=find(id);let p=policy(&d);
            assert!(!p.allowed_types.contains(&"index")&&!p.allowed_types.contains(&"etf"),"{id}");
            assert!(!CONNORS_INDEX_IDS.contains(&id));
            for tk in ["NIFTY 50","SPX","SPY"] {assert!(check(&d,tk,false).is_some(),"{id} {tk}");}
        }
    }
}
#[cfg(test)]
mod bernstein_g2_tests {
    use super::*;
    #[test]
    fn bernstein_g2_admits_exact_ids_on_their_instruments() {
        use crate::books::{jb30d,jbudt,jbcdt2};
        let cat:Vec<_>=jb30d::catalogue().into_iter().chain(jbudt::catalogue()).chain(jbcdt2::catalogue()).collect();
        let admitted:Vec<&str>=cat.iter().filter(|d|reviewed_cash_rule(d)&&!intraday_p1(d)).map(|d|d.id).collect(); // jb30d_gap_pct: intraday wave 1
        assert_eq!(admitted.len(),14);
        let (us,india)=(NativeEvidence::us_test_fixture(),NativeEvidence::india_test_fixture());
        for d in cat.iter().filter(|d|reviewed_cash_rule(d)&&!intraday_p1(d)) {
            assert!(matches!(d.status,crate::connors::Status::Live),"{}",d.id);
            if matches!(d.id,"jb30d_rev_buy_stop"|"jb30d_rev_buy_3pc"|"jbcdt2_dbo_mon"|"jbcdt2_dbo_thu") {
                assert!(check(d,"SPX",false).is_none(),"{}",d.id);
                for t in ["SPY","NDX","NIFTY 50"] {assert!(check(d,t,false).is_some(),"{} {t}",d.id);}
                assert!(check_native(d,"AAPL",&us).is_some());
            } else if d.id=="jbcdt2_preholiday_oc" {
                for t in ["SPX","SPY","QQQ","DJI"] {assert!(check(d,t,false).is_none(),"{t}");}
                assert!(check_native(d,"AAPL",&us).is_none());
                assert!(check(d,"NIFTY 50",false).is_some()&&check_native(d,"RELIANCE",&india).is_some());
            } else {
                for t in ["SPX","SPY","QQQ","NIFTY 50","NIFTYBANK"] {assert!(check(d,t,false).is_none(),"{} {t}",d.id);}
                assert!(check_native(d,"AAPL",&us).is_none()&&check_native(d,"RELIANCE",&india).is_none(),"{}",d.id);
            }
        }
        // Held: same-day gap stops, intraday, missing numbers, chart judgement.
        for d in cat.iter().filter(|d|matches!(d.id,"jb30d_rev_sell_fpo"|"jbudt_gap_fpo"|"jbudt_gap_split"/* jb30d_gap_tick, jb30d_gap_pct: intraday wave 1 */|"jb30d_rev_simple"|"jbudt_dow"|"jbudt_pop"|"jbcdt2_inside_day"|"jbcdt2_or30")) {
            assert!(!reviewed_cash_rule(d),"{}",d.id);
            assert_eq!(policy(d).requirement,"source_review_required");
        }
        assert!(crate::books::jbcdt::catalogue().iter().all(|d|!reviewed_cash_rule(d)));
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

#[cfg(test)]
mod connors_final_tests {
    use super::*;
    fn d(id:&str,book:&str)->StrategyDef {crate::guru_catalogue("connors").into_iter().find(|x|x.id==id&&x.book==book).unwrap()}
    #[test]
    fn nyse_breadth_rules_run_on_every_us_instrument_and_never_on_indian_ones() {
        let (us,india)=(NativeEvidence::us_test_fixture(),NativeEvidence::india_test_fixture());
        for &(book,id) in NYSE_BREADTH_RULES {
            let x=d(id,book);
            assert_eq!(policy(&x).requirement,"index_etf_or_stock","{id}");assert!(metadata_json(&x).contains("NYSE breadth"),"{id}");
            for t in ["SPX","SPY","NDX","DJI","QQQ","IWM","EFA"] {assert!(check(&x,t,false).is_none(),"{id} {t}");}
            assert!(check_native(&x,"AAPL",&us).is_none(),"{id} AAPL");
            for t in ["NIFTY 50","NIFTYBANK"] {assert!(check(&x,t,false).is_some(),"{id} {t}");}
            assert!(check_native(&x,"RELIANCE",&india).is_some(),"{id} RELIANCE");
        }
        for (id,book) in [("ats_padi_nse","ats"),("ptjmt_padi_nse","ptjmt"),("hfm_chadtp_nse","hfm"),("ss_ad_market_buster_nse","ss")] {assert!(nse_breadth_held(&d(id,book),"NIFTY 50").is_some(),"{id}");}
    }
    #[test]
    fn ststw_stock_tests_cbp_and_treasury_sleeve_scopes() {
        let (us,india)=(NativeEvidence::us_test_fixture(),NativeEvidence::india_test_fixture());
        for id in crate::books::ststw::STOCK_STUDIES {
            let x=d(id,"ststw");assert!(check_native(&x,"AAPL",&us).is_none() && check(&x,"SPY",false).is_none(),"{id}");
            assert_eq!(check_native(&x,"RELIANCE",&india).is_none(),!crate::books::ststw::usd_filter(id),"{id}");
            assert_eq!(check(&x,"NIFTY 50",false).is_none(),!crate::books::ststw::usd_filter(id),"{id}");
        }
        for (id,book) in [("ats_cbp","ats"),("btp_cbp_short","btp")] {
            let x=d(id,book);assert!(reviewed_cash_rule(&x) && check_native(&x,"AAPL",&us).is_none() && check(&x,"SPY",false).is_none(),"{id}");
            assert!(check_native(&x,"RELIANCE",&india).unwrap().note.contains("$20") && check(&x,"NIFTY 50",false).is_some(),"{id}");
        }
        let dt=d("alpha_dyn_treasuries_sleeve","alpha");
        for t in ALPHA_DT_ETFS {assert!(check(&dt,t,false).is_none(),"{t}");}
        for t in ["SPY","IEI","SPX","NIFTY 50"] {assert!(check(&dt,t,false).is_some(),"{t}");}
        assert!(check_native(&dt,"AAPL",&us).is_some());
        for t in US_ETFS {assert_eq!(selected_type(t,false),"etf","{t}");}
    }
}
