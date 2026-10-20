//! Lars N. Kestner, "Quantitative Trading Strategies" (McGraw-Hill, 2003). Book id `lkqts`.
//! Source authority: the published book; method-specific page references and declared
//! research conventions are recorded in docs/kestner-sob-stock-research.md.
//!
//! Execution context: the Chapter 7 study states (p.139), "Signals are generated at the close of each
//! trading day and executed on the next trading day's opening price", with no commissions or slippage.
//! Applying that clock to another section, including Chapter 8 SOB/TIAR, is a declared research convention unless
//! that method's source independently specifies it; p.139 does not establish every chapter's execution rules.
//! Standard deviation is the population form, divided by N (p.41, Fig. 2.2). "Average true range" = simple average
//! of the true range (p.64). Current-inclusive channel windows are a declared interpretation of the prose,
//! not parity with a published indexed implementation. See docs/kestner-stock-candidates.md.
//! Entry-only long/short systems use a declared stop-and-reverse implementation convention. Chapter 7 explicitly
//! gives an opposite-entry exit ("no rules for exiting positions, except for an entry in the other direction",
//! pp.145, 155, 160, 165); applying that convention to Chapter 8 SOB/TIAR remains declared. Aggregate table occupancy
//! cannot establish a unique trade ledger or certify the original exit implementation; see README.
//! Chapter 10 (S&P timing) measures close-to-close returns, so those rules fill at the close of the signal day.
//! The original mixed-instrument study used volatility-scaled dollar sizing. Selected-stock
//! normalized percentage research is a separate application and cannot reproduce those dollar figures.
use crate::connors::{sma, true_range, Claim, Ctx, Leg, ParamDef, Params, Status, StrategyDef, Trade};

pub const BOOK: &str = "lkqts";

/// Exact RAW-native MACS research; separate from original lists/SOB/TIAR.
pub const MACS_SOURCE_PROFILE: &str =
    "macs_close_sma_n1_20_ratio4_full80_longcount20_inclusive60_40_levels_flatstart_nextopen_reverse";
/// Native MACS consumes exact RAW decimal values from its typed same-byte source binding.
pub const MACS_RAW_NUMERIC_CONVENTION: &str = "raw_native_decimal_bounded_window_sums_v4";
pub const MACS_STRICT_DIAGNOSTIC_PROFILE: &str =
    "macs_close_sma_n1_20_ratio4_full80_longcount20_strict60_40_levels_flatstart_nextopen_reverse";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MacsThresholdProfile {
    DetailedProseInclusive,
    SummaryStrictDiagnostic,
}
impl MacsThresholdProfile {
    pub fn source_profile(self) -> &'static str {
        match self {
            Self::DetailedProseInclusive => MACS_SOURCE_PROFILE,
            Self::SummaryStrictDiagnostic => MACS_STRICT_DIAGNOSTIC_PROFILE,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct MacsSourcePair {
    pub short_length: usize,
    pub long_length: usize,
    // Approximate binary64 diagnostics only. These means do NOT decide votes;
    // Exact complete sums in the report's declared domain decide ordering;
    // native research uses the same-byte bound RAW decimal closes.
    pub comparison_anchor: f64,
    pub centered_short_mean: f64,
    pub centered_long_mean: f64,
    pub pair_long: bool,
    pub pair_short: bool,
    pub pair_tie: bool,
}
#[derive(Clone, Debug, PartialEq)]
pub struct MacsSourceState {
    pub pairs: Vec<MacsSourcePair>,
    pub long_pairs: u8,
    pub short_pairs: u8,
    pub tied_pairs: u8,
    pub macs: u8,
    pub long_condition: bool,
    pub short_condition: bool,
}
#[derive(Clone, Debug)]
pub struct MacsSourceDiagnostics {
    pub profile: &'static str,
    pub arithmetic_convention: &'static str,
    pub states: Vec<Option<MacsSourceState>>,
}


pub fn macs_candidate(id:&str)->bool {id=="lkqts_macs"}
pub fn validate_macs_candidate(p:&Params)->Option<String> {
    (!p.iter().all(|(key,value)|value.is_finite() && match key.as_str(){"long"=>*value==60.,"short"=>*value==40.,_=>false}))
        .then(||"MACS raw-native stock research requires exact long60/short40 under the named inclusive source profile. Unknown, changed, fractional or nonfinite settings are not rounded into admission.".into())
}

fn macs_exact_window_sums(values: &[crate::unit_exposure_arithmetic::ExactValue])
    -> Option<Vec<crate::unit_exposure_arithmetic::ExactValue>> {
    use crate::unit_exposure_arithmetic::ExactValue;
    if values.is_empty() || values.len()>80 { return None; }
    let mut sums=Vec::with_capacity(values.len()+1);
    sums.push(ExactValue::canonical(0.).ok()?);
    for value in values.iter().rev() {
        sums.push(sums.last()?.add(value));
    }
    Some(sums)
}


fn macs_source_pair_from_sums(closes: &[f64], n: usize,
    sums: &[crate::unit_exposure_arithmetic::ExactValue]) -> Option<MacsSourcePair> {
    use std::cmp::Ordering;
    if !(1..=20).contains(&n) || closes.len()!=4*n || sums.len()<=4*n
        || closes.iter().any(|v| !v.is_finite() || *v<=0.) { return None; }
    let ordering=sums[n].scale(4).signed_cmp(&sums[4*n]);
    let anchor=*closes.last()?;
    // Retained approximate chart/audit diagnostics, never a predicate source.
    let long_sum=closes.iter().map(|v|*v-anchor).sum::<f64>();
    let short_sum=closes[3*n..].iter().map(|v|*v-anchor).sum::<f64>();
    let short_mean=short_sum/n as f64;
    let long_mean=long_sum/(4*n) as f64;
    if ![short_mean,long_mean].iter().all(|v|v.is_finite()) { return None; }
    Some(MacsSourcePair {
        short_length:n, long_length:4*n, comparison_anchor:anchor,
        centered_short_mean:short_mean, centered_long_mean:long_mean,
        pair_long:ordering==Ordering::Greater, pair_short:ordering==Ordering::Less,
        pair_tie:ordering==Ordering::Equal,
    })
}


fn macs_source_thresholds(long_pairs: u8, profile: MacsThresholdProfile) -> Option<(u8, bool, bool)> {
    if long_pairs > 20 { return None; }
    let score = 5*long_pairs;
    let (long, short) = match profile {
        MacsThresholdProfile::DetailedProseInclusive => (score >= 60, score <= 40),
        MacsThresholdProfile::SummaryStrictDiagnostic => (score > 60, score < 40),
    };
    Some((score, long, short))
}


fn macs_source_diagnostics_with_exact(x:&Ctx,to:usize,p:&Params,profile:MacsThresholdProfile,
    exact:&[crate::unit_exposure_arithmetic::ExactValue],convention:&'static str)->Option<MacsSourceDiagnostics> {
    if validate_macs_candidate(p).is_some()
        || following_two_source_history_issue(x,0,to).is_some() || exact.len()!=to+1
        || exact.iter().any(|v|!v.positive()) {return None;}
    let closes=&x.c[..=to];
    let mut states = vec![None; closes.len()];
    for i in 79..closes.len() {
        let sums=macs_exact_window_sums(&exact[i-79..=i])?;
        let pairs: Vec<MacsSourcePair> = (1..=20)
            .map(|n| macs_source_pair_from_sums(&closes[i+1-4*n..=i], n, &sums)).collect::<Option<_>>()?;
        let long_pairs = pairs.iter().filter(|p| p.pair_long).count() as u8;
        let short_pairs = pairs.iter().filter(|p| p.pair_short).count() as u8;
        let tied_pairs = pairs.iter().filter(|p| p.pair_tie).count() as u8;
        let (macs, long_condition, short_condition) = macs_source_thresholds(long_pairs, profile)?;
        states[i] = Some(MacsSourceState { pairs, long_pairs, short_pairs, tied_pairs, macs, long_condition, short_condition });
    }
    Some(MacsSourceDiagnostics { profile: profile.source_profile(), arithmetic_convention: convention, states })
}


pub(crate) fn macs_raw_source_diagnostics(series:&crate::baskets::macs_source_data::ValidatedSeries,
    to:usize,p:&Params,profile:MacsThresholdProfile)->Option<MacsSourceDiagnostics> {
    let report=series.report();
    if report.arithmetic_convention!=MACS_RAW_NUMERIC_CONVENTION || report.through_source_i!=to
        || report.checked_close_rows!=to+1 || report.closes.len()!=to+1
        || series.exact_raw_closes().len()!=to+1 {return None;}
    let x=Ctx::new(series.rows(),&[],&[]);
    macs_source_diagnostics_with_exact(&x,to,p,profile,series.exact_raw_closes(),MACS_RAW_NUMERIC_CONVENTION)
}


pub(crate) fn macs_raw_source_run(series:&crate::baskets::macs_source_data::ValidatedSeries,
    from:usize,to:usize,p:&Params,profile:MacsThresholdProfile)->Option<Vec<Trade>> {
    let x=Ctx::new(series.rows(),&[],&[]);
    if validate_macs_candidate(p).is_some()
        || following_two_source_history_issue(&x,from,to).is_some() {return None;}
    let diagnostics=macs_raw_source_diagnostics(series,to,p,profile)?;
    let signals:Vec<i8>=diagnostics.states.iter().map(|s|s.as_ref()
        .map_or(0,|v|if v.long_condition {1} else if v.short_condition {-1} else {0})).collect();
    Some(sar(&x,from,to,&signals,"Private TIAR/MACS source long","Private TIAR/MACS source short"))
}


pub fn macs_source_convention()->&'static str {
    "Printed207 detailed prose uses inclusive score>=60/<=40; strict summary alternatives remain a disclosed same-family ambiguity, not a fitted winner. Declared close-SMA n/4n for n1..20, complete80 actual closes; each pair votes long only when its short SMA is strictly above the long SMA. Pair equality contributes zero long votes without a neutral veto; the printed systems-on-long-trades wording does not uniquely specify equality state, so daily relationship/tie treatment is declared. Native votes compare exact RAW decimal sums through same-byte SHA/row/date/parsed-price binding, not rounded Rust Display prices; approximate centered binary64 means are diagnostics only. First complete state79/earliest next-open80, genuine prefix, flat selected start, repeated levels/no pyramiding, opposite next-actual-open reversal and terminal open marks are declared Chapter8 research conventions. Fixed AAPL1990–2001/RELIANCE2001–2010 full-prefix source comparisons matched unchanged exact references, including all20 pairs, with zero mismatches. Shared-author mechanical corroboration is not blind independence. Fixed AAPL1990-01-01..2001-12-31 and RELIANCE2001-01-01..2010-12-31: MACS and TIAR each matched four ordinary normalized-account cases (fees/slippage0/0 and3/2bps per side, rollover0) and16 direct production-function responses (detail, readiness, selected summary, singleton optimizer) against unchanged source/account references with zero mismatches (combined32responses/eightaccounts). Selected API states and earlier full-prefix source evidence have separate scope. HTTP transport/request-matrix, browser and deployment are separate acceptance gates. Every requested run still needs its own source/data/account checks; data readiness alone does not certify account support. Source agreement alone does not establish a financially supported account. Original mixed-instrument volatility-dollar sizing and book performance are not reproduced."
}



/// Fixed TIAR research profile; separate from earlier admission lists and MACS.
pub const TIAR_SOURCE_PROFILE: &str =
    "tiar_close_lags5_10_15_strict_levels_flatstart_nextopen_reverse";
pub fn tiar_candidate(id: &str) -> bool { id=="lkqts_tiar" }
/// Retained diagnostic helper name; this integration accepts TIAR only.
pub fn validate_following_two_source(id: &str, p: &Params) -> Option<String> {
    if !tiar_candidate(id) {return Some("Unknown TIAR stock research profile".into());}
    (!p.iter().all(|(key,value)| key=="step" && value.is_finite() && *value==5.))
        .then(|| "TIAR stock research requires exact step5 (close lags5/10/15). Changed, unknown, fractional and nonfinite parameters are rejected without rounding; source, data, account and instrument checks remain mandatory.".into())
}

pub fn following_two_source_history_issue(x: &Ctx, from: usize, to: usize) -> Option<&'static str> {
    if from > to || to >= x.r.len() {
        return Some("TIAR/MACS selected source window is unavailable");
    }
    if x.r[..=to].iter().any(|r| r.ticker.is_empty() || !crate::baskets::valid_date(&r.date)
        || ![r.open, r.high, r.low, r.close].iter().all(|v| v.is_finite() && *v > 0.)
        || r.low > r.high || r.open < r.low || r.open > r.high || r.close < r.low || r.close > r.high)
        || x.r[..=to].windows(2).any(|w| w[0].date >= w[1].date || w[0].ticker != w[1].ticker)
    {
        return Some("TIAR/MACS source proposals require one genuine symbol, finite positive consistent OHLC and ordered distinct valid dates through the selected end. Earlier rows are not dropped or reseeded.");
    }
    None
}

#[derive(Clone, Debug, PartialEq)]
pub struct TiarSourceState {
    pub close: f64,
    pub close_lag5: f64,
    pub close_lag10: f64,
    pub close_lag15: f64,
    pub long_condition: bool,
    pub short_condition: bool,
}
#[derive(Clone, Debug)]
pub struct TiarSourceDiagnostics {
    pub profile: &'static str,
    pub states: Vec<Option<TiarSourceState>>,
}

/// Printed222/227 strict three-step close chain, not three consecutive daily
/// rises or endpoint momentum. Equality supplies no new direction.
pub fn tiar_source_diagnostics(x: &Ctx, to: usize, p: &Params) -> Option<TiarSourceDiagnostics> {
    if validate_following_two_source("lkqts_tiar", p).is_some()
        || following_two_source_history_issue(x, 0, to).is_some() { return None; }
    let closes = &x.c[..=to];
    let mut states = vec![None; closes.len()];
    for i in 15..closes.len() {
        let (c0, c5, c10, c15) = (closes[i], closes[i-5], closes[i-10], closes[i-15]);
        states[i] = Some(TiarSourceState {
            close: c0, close_lag5: c5, close_lag10: c10, close_lag15: c15,
            long_condition: c0 > c5 && c5 > c10 && c10 > c15,
            short_condition: c0 < c5 && c5 < c10 && c10 < c15,
        });
    }
    Some(TiarSourceDiagnostics { profile: TIAR_SOURCE_PROFILE, states })
}

/// TIAR-only extraction of the frozen source runner. No MACS code is dispatched.
pub fn tiar_source_run(x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if validate_following_two_source("lkqts_tiar", p).is_some()
        || following_two_source_history_issue(x, from, to).is_some() { return None; }
    let signals: Vec<i8> = tiar_source_diagnostics(x, to, p)?.states.iter().map(|s| s.as_ref()
                .map_or(0, |v| if v.long_condition { 1 } else if v.short_condition { -1 } else { 0 })).collect();
    Some(sar(x, from, to, &signals, "Private TIAR/MACS source long", "Private TIAR/MACS source short"))
}

pub fn tiar_source_convention() -> &'static str {
    "Printed222/227: long close>close5>close10>close15; short strict reverse chain. These are recurring levels, not observed crossings or three consecutive daily changes. Exact fixed step5, genuine prefix, first defined source state15/earliest next actual open16, equality supplies no new direction. Flat selected start, repeated same-side level does not add exposure; completed-close/next-actual-open opposite reversal and terminal open close mark are declared Chapter8 research conventions. Fixed AAPL1990–2001/RELIANCE2001–2010 source mechanics, four ordinary normalized-account cost cases (fees/slippage0/0 and3/2bps per side, rollover0) and16 production-function responses matched unchanged references with zero comparison mismatches. API states cover selected dates; earlier full-prefix source evidence is separately qualified. Shared-author mechanical corroboration is not blind independence. HTTP transport/request-matrix, browser and deployment are separate acceptance gates. Original mixed-instrument volatility-dollar sizing and published performance are not reproduced; no index/ETF/basket substitution, membership, adjustment, calendar, borrow, margin, financing or dividend certification."
}



/// Fixed SOB source configuration. Kept separate from the original three and
/// next-four lists; stock-only request/history admission is dispatched explicitly.
pub const SOB_PROPOSED_PROFILE: &str =
    "sob_close_max80_current_inclusive_channel40_tie_inclusive_strict20";

/// Local direct-run defaults only; never round a changed value into review.
/// Empty/missing fields use the exact fixed source defaults80/40/20.
pub fn validate_sob_candidate(p: &Params) -> Option<String> {
    (!p.iter().all(|(key,value)| value.is_finite() && match key.as_str() {
        "search" => *value==80., "chan" => *value==40., "min" => *value==20., _ => false,
    })).then(|| "SOB stock research requires exact search80/chan40/min20. Changed, unknown, fractional or nonfinite parameters are rejected without rounding; all source/data/account and instrument checks remain mandatory.".into())
}

/// At the Ctx boundary, validate only the genuine causal prefix and required opens.
/// SOB state/fill calculations ignore observations after selectedto. The native
/// loader separately requires whole-file whitelist/hash/format integrity before
/// constructing a Ctx; this does not waive rejection of an invalid native suffix.
pub fn sob_history_issue(x: &Ctx, from: usize, to: usize) -> Option<&'static str> {
    if from>to || to>=x.r.len() {return Some("SOB selected source window is unavailable");}
    if x.r[..=to].iter().any(|r| r.ticker.is_empty() || !crate::baskets::valid_date(&r.date)
        || ![r.open,r.high,r.low,r.close].iter().all(|v|v.is_finite() && *v>0.)
        || r.low>r.high || r.open<r.low || r.open>r.high || r.close<r.low || r.close>r.high)
        || x.r[..=to].windows(2).any(|w|w[0].date>=w[1].date || w[0].ticker!=w[1].ticker) {
        return Some("SOB requires genuine finite positive consistent OHLC, ordered distinct valid dates and one source identity through the selected end. No observations, opens or earlier indicator history are repaired or invented.");
    }
    None
}

/// Longest high/low horizons over exactly80 current-inclusive actual closes.
/// Equality is included; both horizons80 on a flat window resolve to neutral0.
fn sob_close_statistic(window: &[f64]) -> Option<(u8,u8,i16)> {
    if window.len()!=80 || window.iter().any(|v|!v.is_finite() || *v<=0.) {return None;}
    let current=window[79];
    let high=window.iter().rev().take_while(|&&v|v<=current).count() as u8;
    let low=window.iter().rev().take_while(|&&v|v>=current).count() as u8;
    let statistic=if high>low {i16::from(high)} else if low>high {-i16::from(low)} else {0};
    Some((high,low,statistic))
}

/// Exactly40 defined statistics includingtoday. Ties at the outer extreme can
/// qualify again; the strict formal20 threshold differs from the Intel example.
fn sob_channel_predicate(window: &[i16]) -> Option<(i16,i16,i8)> {
    if window.len()!=40 || window.iter().any(|s|!(-80..=80).contains(s)) {return None;}
    let upper=*window.iter().max()?;let lower=*window.iter().min()?;let current=window[39];
    let side=if current>20 && current==upper {1} else if current< -20 && current==lower {-1} else {0};
    Some((upper,lower,side))
}

pub struct SobRuleState {
    pub high_horizon: u8,
    pub low_horizon: u8,
    pub statistic: i16,
    pub upper_40: Option<i16>,
    pub lower_40: Option<i16>,
    pub long_condition: bool,
    pub short_condition: bool,
}
pub struct SobDiagnostics {
    pub profile: &'static str,
    pub states: Vec<Option<SobRuleState>>,
}

/// Causal source state for the exact fixed SOB stock-price research application.
/// First statistic79; first complete predicate118; first possible nextopen119.
pub fn sob_diagnostics(x: &Ctx, to: usize, p: &Params) -> Option<SobDiagnostics> {
    if validate_sob_candidate(p).is_some() || sob_history_issue(x,0,to).is_some() {return None;}
    let closes=&x.c[..=to];
    let mut states:Vec<Option<SobRuleState>>=(0..closes.len()).map(|_|None).collect();
    let mut statistics=Vec::with_capacity(closes.len().saturating_sub(79));
    for i in 79..closes.len() {
        let (high,low,statistic)=sob_close_statistic(&closes[i-79..=i])?;
        statistics.push(statistic);
        let channel=if statistics.len()>=40 {Some(sob_channel_predicate(&statistics[statistics.len()-40..])?)} else {None};
        let (upper,lower,side)=channel.map_or((None,None,0),|(u,l,s)|(Some(u),Some(l),s));
        states[i]=Some(SobRuleState{high_horizon:high,low_horizon:low,statistic,upper_40:upper,lower_40:lower,long_condition:side==1,short_condition:side== -1});
    }
    Some(SobDiagnostics{profile:SOB_PROPOSED_PROFILE,states})
}

fn sob_proposed_run(x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if validate_sob_candidate(p).is_some() || sob_history_issue(x,from,to).is_some() {return None;}
    let diagnostics=sob_diagnostics(x,to,p)?;
    let signals:Vec<i8>=diagnostics.states.iter().map(|state|state.as_ref().map_or(0,|s|if s.long_condition {1} else if s.short_condition {-1} else {0})).collect();
    // The existing source-clock runner startsflat at selectedfrom, ignores
    // same-side repeated levels, reverses at the next actualopen, never fills
    // a terminalsignal outside the period and marks an existing holdingclose.
    Some(sar(x,from,to,&signals,"Proposed SOB strict20: tied40-statistic high","Proposed SOB strict20: tied40-statistic low"))
}


/// Exact bounded source configurations. Instrument admission is owned separately
/// by instrument_policy::reviewed_kestner_stock; other Kestner IDs inherit none.
pub const STOCK_CANDIDATE_IDS: &[&str] = &["lkqts_cb4020", "lkqts_ma1040", "lkqts_mom80"];

pub fn stock_candidate(id: &str) -> bool {
    STOCK_CANDIDATE_IDS.contains(&id)
}

pub fn validate_stock_candidate(id: &str, p: &Params) -> Option<String> {
    if !stock_candidate(id) { return Some("Not a bounded Kestner stock candidate".into()); }
    let schema = params(id)?;
    (!p.iter().all(|(key, value)| value.is_finite() && schema.iter().any(|s| s.key == key && *value == s.def)))
        .then(|| "Kestner stock research covers only CB entry40/exit20, SMA fast10/slow40 and momentum n80. Changed, unknown or nonfinite parameters need separate review; no rounding or clamping is admitted.".into())
}

pub fn stock_history_issue(id: &str, x: &Ctx, from: usize, to: usize) -> Option<&'static str> {
    let first = match id { "lkqts_cb4020" => 39, "lkqts_ma1040" => 40, "lkqts_mom80" => 80, _ => return Some("Unknown Kestner stock candidate") };
    if from < first || from > to || to >= x.r.len() {
        return Some("Kestner stock research needs genuine preceding source observations: current-inclusive channel first signal39, observed SMA crossover40, momentum80. Select a start with actual warmup; no history is invented.");
    }
    if x.r[..=to].iter().any(|r| !crate::baskets::valid_date(&r.date)
        || ![r.open,r.high,r.low,r.close].iter().all(|v|v.is_finite() && *v>0.)
        || r.low>r.high || r.open<r.low || r.open>r.high || r.close<r.low || r.close>r.high)
        || x.r[..=to].windows(2).any(|w|w[0].date>=w[1].date || w[0].ticker!=w[1].ticker) {
        return Some("Full supplied Kestner indicator history contains invalid OHLC, dates, chronology or mixed symbols. Earlier observations are not dropped, filled or reseeded to manufacture results.");
    }
    None
}

/// Additional fixed research profiles, deliberately separate from STOCK_CANDIDATE_IDS.
/// Source/indicator declarations are recorded in docs/kestner-next-stock-candidates.md.
pub const NEXT_CANDIDATE_IDS: &[&str] = &[
    "lkqts_volbo", "lkqts_rsi14", "lkqts_macd", "lkqts_macd_literal_alpha",
];

pub fn next_candidate(id: &str) -> bool { NEXT_CANDIDATE_IDS.contains(&id) }

/// Shared request/history dispatch; the original exact-three list stays stable.
pub fn sob_candidate(id: &str) -> bool { id=="lkqts_sob" }

pub fn sob_source_convention() -> &'static str {
    "Printed185/190: closing-price maximum signed breakout horizon80 and strict20 entries at the40-statistic extreme. Printed191 Intel exactly+20/-20 conflicts with that formal paragraph; this named profile follows strict20 without fitting returns. Current-inclusive80/40 windows, inclusive ties, equal high/low horizons neutral0, complete initialization79/118/119, flat selected start and Chapter7 completed-close/next-actual-open opposite reversal are declared interpretations. Genuine prefix, no pyramiding, independent stop or invented terminal exit. Original volatility-dollar sizing and published performance are not reproduced."
}

pub fn stock_research_profile(id: &str) -> bool { stock_candidate(id) || next_candidate(id) || sob_candidate(id) || tiar_candidate(id) || macs_candidate(id) }

pub fn validate_stock_research_profile(id: &str,p: &Params) -> Option<String> {
    if stock_candidate(id) {validate_stock_candidate(id,p)}
    else if sob_candidate(id) {validate_sob_candidate(p)}
    else if tiar_candidate(id) {validate_following_two_source(id,p)}
    else if macs_candidate(id) {validate_macs_candidate(p)}
    else {validate_next_candidate(id,p)}
}

pub fn stock_research_history_issue(id: &str,x: &Ctx,from: usize,to: usize) -> Option<&'static str> {
    if stock_candidate(id) {stock_history_issue(id,x,from,to)}
    else if next_candidate(id) {next_history_issue(x,from,to)}
    else if sob_candidate(id) {sob_history_issue(x,from,to)}
    else if tiar_candidate(id) {following_two_source_history_issue(x,from,to)}
    else if macs_candidate(id) {following_two_source_history_issue(x,from,to)}
    else {Some("Unknown Kestner stock research profile")}
}

pub fn next_source_convention(id: &str) -> Option<&'static str> {
    Some(match id {
        "lkqts_volbo"=>"Printed155/41: strict signed2-sigma price-change entries with opposite-entry reversal; declared current-inclusive100 changes, population variance and finite squared comparison without epsilon. Genuine prefix, flat selected start, next actual stock open; original10000/sigma quantities are unsupported, including zero variance.",
        "lkqts_rsi14"=>"Printed165/170 thresholds with declared Wilder RMA14, arithmetic first14-change seed, zero-filled gains/losses, neutral flat50 and strict level35/65 startup. Printed66 does not uniquely prescribe smoothing or seed. Genuine prefix, flat selected start and next actual stock open; nonpositive ordinary accounts remain unavailable.",
        "lkqts_macd"=>"Declared period-alpha interpretation of printed57/170:2/13,2/27,1/5. Printed68's .15/.075 conflicts and is a separate named variant. First-close price seeds, MACD/signal0, local state+=alpha*(value-state), observed signal-line crossings, equality holds, full genuine prefix and next actual stock open.",
        "lkqts_macd_literal_alpha"=>"Declared literal interpretation of printed68: .15/.075, with .2 signal from printed57's9-day rule. Distinct from2/13,2/27. First-close price seeds, MACD/signal0, local state+=alpha*(value-state), observed signal-line crossings, equality holds, full genuine prefix and next actual stock open.",
        _=>return None,
    })
}

/// A completed-close state computed once per selected report, never per trade.
pub struct NextRuleState {
    pub values: Vec<(&'static str,f64)>,
    pub long_condition: bool,
    pub short_condition: bool,
}
pub struct NextDiagnostics {
    pub profile: &'static str,
    pub states: Vec<Option<NextRuleState>>,
}

pub fn next_diagnostics(id: &str,x: &Ctx,to: usize,p: &Params) -> Option<NextDiagnostics> {
    if !next_candidate(id) || validate_next_candidate(id,p).is_some() || next_history_issue(x,0,to).is_some() {return None;}
    let c=&x.c[..=to];let mut states:Vec<Option<NextRuleState>>=(0..c.len()).map(|_|None).collect();
    let profile=match id {
        "lkqts_volbo"=>{
            for i in 100..c.len() {
                let changes:Vec<_>=(i-99..=i).map(|k|c[k]-c[k-1]).collect();
                let mean=changes.iter().sum::<f64>()/100.;let variance=next_population_variance(&changes)?;
                let delta=changes[99];let square=delta*delta;let four=4.*variance;
                if ![mean,square,four].iter().all(|v|v.is_finite()) || (delta!=0. && square==0.) {return None;}
                states[i]=Some(NextRuleState{values:vec![("delta",delta),("mean_change_100",mean),("population_variance_100",variance),("four_variance",four),("delta_squared",square)],long_condition:delta>0. && square>four,short_condition:delta<0. && square>four});
            }
            "volbo_current_inclusive_population100_k2"
        },
        "lkqts_rsi14"=>{
            for (i,state) in next_rsi_states(c)?.into_iter().enumerate() {
                if let Some((up,down))=state {
                    let side=next_rsi_side(up,down);
                    let rsi=next_rsi_value(up,down);if !rsi.is_finite() {return None;}
                    states[i]=Some(NextRuleState{values:vec![("rma_up_14",up),("rma_down_14",down),("rsi_14",rsi),("long_comparison_13_up",13.*up),("long_comparison_7_down",7.*down),("short_comparison_7_up",7.*up),("short_comparison_13_down",13.*down)],long_condition:side==1,short_condition:side== -1});
                }
            }
            "rsi_wilder_rma14_level35_65_seed14_flat50"
        },
        "lkqts_macd" | "lkqts_macd_literal_alpha"=>{
            let literal=id=="lkqts_macd_literal_alpha";let (f,s,m,signal,h)=next_macd_components(c,literal)?;
            let (fa,sa)=if literal {(0.15,0.075)} else {(2./13.,2./27.)};
            for i in 0..c.len() {
                states[i]=Some(NextRuleState{values:vec![("fast_ema",f[i]),("slow_ema",s[i]),("macd",m[i]),("signal_ema",signal[i]),("histogram",h[i]),("previous_histogram",if i>0 {h[i-1]}else{f64::NAN}),("fast_alpha",fa),("slow_alpha",sa),("signal_alpha",0.2)],long_condition:cross_up(&h,i,0.),short_condition:cross_dn(&h,i,0.)});
            }
            if literal {"macd_literal_alpha_first_close"} else {"macd_period_alpha_first_close"}
        },
        _=>return None,
    };
    Some(NextDiagnostics{profile,states})
}

pub fn validate_next_candidate(id: &str, p: &Params) -> Option<String> {
    if !next_candidate(id) { return Some("Not a next Kestner conversion candidate".into()); }
    let schema=params(id)?;
    (!p.iter().all(|(key,value)| value.is_finite() && schema.iter().any(|s|s.key==key && *value==s.def)))
        .then(|| "Next Kestner profiles require exact volbo100/2, declared RSI14/35/65 or the named fixed MACD12/26/9 alpha profile. Changed, unknown, fractional or nonfinite settings are not rounded into this contract or allowed to inherit reviewed admission.".into())
}

/// Validate the genuine prefix used by indicators. Early selected bars can stay
/// flat until the declared indicator exists; requested dates are not shortened.
pub fn next_history_issue(x: &Ctx, from: usize, to: usize) -> Option<&'static str> {
    if from>to || to>=x.r.len() { return Some("Next Kestner candidate window is unavailable"); }
    if x.r[..=to].iter().any(|r| !crate::baskets::valid_date(&r.date)
        || ![r.open,r.high,r.low,r.close].iter().all(|v|v.is_finite() && *v>0.)
        || r.low>r.high || r.open<r.low || r.open>r.high || r.close<r.low || r.close>r.high)
        || x.r[..=to].windows(2).any(|w|w[0].date>=w[1].date || w[0].ticker!=w[1].ticker) {
        return Some("Next Kestner full source prefix requires genuine finite positive OHLC, strictly ordered dates and one identity. No observations are dropped, filled or reseeded; original quantities, borrow and financing are unsupported.");
    }
    None
}

fn next_population_variance(v: &[f64]) -> Option<f64> {
    if v.is_empty() || v.iter().any(|v|!v.is_finite()) { return None; }
    // Exact equal binary64 observations have genuine zero dispersion. A varying
    // window whose squared deviations disappear is an unsupported underflow.
    if v.iter().all(|value|*value==v[0]) {return Some(0.);}
    let mean=v.iter().sum::<f64>()/v.len() as f64;
    let variance=v.iter().map(|v|(v-mean).powi(2)).sum::<f64>()/v.len() as f64;
    (mean.is_finite() && variance.is_finite() && variance>0.).then_some(variance)
}

/// Population/current-inclusive100 changes, strict2-sigma comparison without a
/// square-root or epsilon. Finite IEEE754 arithmetic is an explicit convention.
fn next_volbo_signals(c: &[f64]) -> Option<Vec<i8>> {
    let mut out=vec![0;c.len()];
    for i in 100..c.len() {
        let changes:Vec<_>=(i-99..=i).map(|k|c[k]-c[k-1]).collect();
        let variance=next_population_variance(&changes)?;
        let delta=changes[99];let square=delta*delta;let threshold=4.*variance;
        if !square.is_finite() || !threshold.is_finite() || (delta!=0. && square==0.) { return None; }
        if square>threshold {out[i]=if delta>0. {1} else if delta<0. {-1} else {0};}
    }
    Some(out)
}

/// Declared Wilder RMA14, seeded with the first14 actual changes. The source
/// gives average up/down moves, not this recursion/seed; no shared helper changes.
fn next_rsi_average(numerator: f64) -> Option<f64> {
    let average=numerator/14.;
    // A genuine nonzero move must not disappear into an invented flat state.
    // Multiplication by13 cannot underflow a positive stored state, but a new
    // subnormal move divided by14 can. No epsilon or replacement value is used.
    (numerator.is_finite() && numerator>=0. && average.is_finite()
        && !(numerator>0. && average==0.)).then_some(average)
}

fn next_rsi_states(c: &[f64]) -> Option<Vec<Option<(f64,f64)>>> {
    let mut out=vec![None;c.len()];
    if c.len()<=14 { return Some(out); }
    let (mut up,mut down)=(0.,0.);
    for i in 1..=14 {let delta=c[i]-c[i-1];up+=delta.max(0.);down+=(-delta).max(0.);}
    up=next_rsi_average(up)?;down=next_rsi_average(down)?;
    for i in 14..c.len() {
        if i>14 {let delta=c[i]-c[i-1];up=next_rsi_average(13.*up+delta.max(0.))?;down=next_rsi_average(13.*down+(-delta).max(0.))?;}
        if ![up,down,13.*up,13.*down,up+down].iter().all(|v|v.is_finite()) {return None;}
        out[i]=Some((up,down));
    }
    Some(out)
}

fn next_rsi_side(up: f64, down: f64) -> i8 {
    // RSI<35 iff13U<7D; RSI>65 iff7U>13D. Flat U=D=0
    // is the declared neutral50 state. Equality never supplies an entry.
    if 13.*up<7.*down {1} else if 7.*up>13.*down {-1} else {0}
}

fn next_rsi_value(up: f64, down: f64) -> f64 {
    if up+down==0. {50.} else {100.*(up/(up+down))}
}

fn next_ema_alpha(v: &[f64], alpha: f64) -> Option<Vec<f64>> {
    let mut out=Vec::with_capacity(v.len());
    let Some(&first)=v.first() else {return Some(out)};
    if !first.is_finite() {return None;}
    let mut state=first;out.push(state);
    for &value in &v[1..] {
        // Declared local machine-operation convention: a genuine constant input
        // preserves its exact seeded value, without an epsilon/phantom cross.
        state+=alpha*(value-state);
        if !value.is_finite() || !state.is_finite() {return None;}
        out.push(state);
    }
    Some(out)
}

/// Two named source interpretations, same first-close seed and signal-line clock.
/// Literal .15/.075 (p68) is not numerically equal to2/13,2/27 (p57).
fn next_macd_histogram(c: &[f64], literal: bool) -> Option<Vec<f64>> {
    Some(next_macd_components(c,literal)?.4)
}

fn next_macd_components(c: &[f64], literal: bool) -> Option<(Vec<f64>,Vec<f64>,Vec<f64>,Vec<f64>,Vec<f64>)> {
    let (fast,slow)=if literal {(0.15,0.075)} else {(2./13.,2./27.)};
    let f=next_ema_alpha(c,fast)?;let s=next_ema_alpha(c,slow)?;
    let macd:Vec<_>=f.iter().zip(&s).map(|(f,s)|f-s).collect();
    let signal=next_ema_alpha(&macd,0.2)?;
    let histogram:Vec<_>=macd.iter().zip(&signal).map(|(m,s)|m-s).collect();
    histogram.iter().all(|v|v.is_finite()).then_some((f,s,macd,signal,histogram))
}

// ------------------------------------------------------------------ indicators

/// EMA with alpha = 2/(1+n) (p.57), seeded with the first value (TradeStation XAverage).
fn ema(v: &[f64], n: usize) -> Vec<f64> {
    let a = 2.0 / (1.0 + n as f64);
    let mut out = vec![f64::NAN; v.len()];
    let mut prev = f64::NAN;
    for i in 0..v.len() {
        if v[i].is_nan() {
            continue;
        }
        prev = if prev.is_nan() { v[i] } else { a * v[i] + (1.0 - a) * prev };
        out[i] = prev;
    }
    out
}

/// Population standard deviation of the last n one-day changes (c[k]-c[k-1], k = i-n+1..=i).
fn sd_chg(c: &[f64], n: usize) -> Vec<f64> {
    let mut out = vec![f64::NAN; c.len()];
    for i in n..c.len() {
        let d: Vec<f64> = (i + 1 - n..=i).map(|k| c[k] - c[k - 1]).collect();
        if d.iter().any(|x| !x.is_finite()) {
            continue;
        }
        let m = d.iter().sum::<f64>() / n as f64;
        out[i] = (d.iter().map(|x| (x - m).powi(2)).sum::<f64>() / n as f64).sqrt();
    }
    out
}

fn hh(v: &[f64], i: usize, n: usize) -> f64 {
    v[i + 1 - n..=i].iter().cloned().fold(f64::NEG_INFINITY, f64::max)
}
fn ll(v: &[f64], i: usize, n: usize) -> f64 {
    v[i + 1 - n..=i].iter().cloned().fold(f64::INFINITY, f64::min)
}

/// 14-day Slow %K = 3-day SMA of raw %K (raw = (C - LL)/(HH - LL), pp.66-67; TradeStation SlowK).
fn slow_k(x: &Ctx, n: usize) -> Vec<f64> {
    let len = x.c.len();
    let mut raw = vec![f64::NAN; len];
    for i in n - 1..len {
        let (h, l) = (hh(&x.h, i, n), ll(&x.l, i, n));
        if h > l {
            raw[i] = 100.0 * (x.c[i] - l) / (h - l);
        }
    }
    sma(&raw, 3)
}

/// Simple average true range (p.64).
fn atr_s(x: &Ctx, n: usize) -> Vec<f64> {
    sma(&true_range(x.r), n)
}

// ------------------------------------------------------------------ simulator

#[derive(Clone, Copy, PartialEq)]
enum Fill {
    NextOpen,
    Close,
}

fn new_trade(dir: f64, i: usize, px: f64, units: i32, kind: &'static str, sig: usize) -> Trade {
    Trade {
        dir,
        legs: (0..units.max(1)).map(|_| Leg { i, px, w: 1.0 }).collect(),
        exit_i: i,
        exit_px: px,
        reason: String::new(),
        entry_kind: kind,
        signal_i: sig,
        open: false,
        exits: Vec::new(),
    }
}

/// Target-position simulator. `tgt(i, pos, trade)` is evaluated on the close of bar i (from..=to) and returns the
/// wanted signed unit count plus the rule that fired; a change is filled at the next open (Fill::NextOpen) or at
/// this close (Fill::Close). Units > 1 in the same direction add legs (pyramiding); a smaller same-sign target is
/// never requested by this module.
fn run_target(
    x: &Ctx,
    from: usize,
    to: usize,
    fill: Fill,
    mut tgt: impl FnMut(usize, i32, Option<&Trade>) -> Option<(i32, &'static str)>,
) -> Vec<Trade> {
    let mut out = Vec::new();
    let mut cur: Option<Trade> = None;
    let mut pos: i32 = 0;
    for i in from..=to {
        if fill == Fill::NextOpen && i == to {
            break;
        }
        let Some((want, why)) = tgt(i, pos, cur.as_ref()) else { continue };
        if want == pos {
            continue;
        }
        let (k, px, kind) = match fill {
            Fill::NextOpen => (i + 1, x.o[i + 1], "open"),
            Fill::Close => (i, x.c[i], "close"),
        };
        if fill == Fill::Close && i == to && (pos == 0 || want.signum() == pos.signum()) {
            continue; // no new exposure on the last bar
        }
        if pos != 0 && (want == 0 || want.signum() != pos.signum()) {
            let mut t = cur.take().unwrap();
            t.exit_i = k;
            t.exit_px = px;
            t.signal_i = i;
            t.reason = why.into();
            out.push(t);
            pos = 0;
        }
        if want != 0 && !(fill == Fill::Close && i == to) {
            if pos == 0 {
                cur = Some(new_trade(want.signum() as f64, k, px, want.abs(), kind, i));
            } else if want.abs() > pos.abs() {
                let t = cur.as_mut().unwrap();
                for _ in 0..(want.abs() - pos.abs()) {
                    t.legs.push(Leg { i: k, px, w: 1.0 });
                }
            }
            pos = want;
        }
    }
    if let Some(mut t) = cur {
        t.exit_i = to;
        t.exit_px = x.c[to];
        t.reason = "open".into();
        t.signal_i = to;
        t.open = true;
        out.push(t);
    }
    out
}

/// Stop-and-reverse on entry signals only: sig[i] = +1 long entry, -1 short entry, 0 none.
fn sar(x: &Ctx, from: usize, to: usize, sig: &[i8], lw: &'static str, sw: &'static str) -> Vec<Trade> {
    run_target(x, from, to, Fill::NextOpen, |i, _, _| match sig[i] {
        1 => Some((1, lw)),
        -1 => Some((-1, sw)),
        _ => None,
    })
}

// ------------------------------------------------------------------ catalogue

macro_rules! sd {
    ($id:expr, $name:expr, $group:expr, $dir:expr, $status:expr, [$($r:expr),* $(,)?], $notes:expr, $claim:expr) => {
        StrategyDef { id: $id, name: $name, group: $group, session: 0, dir: $dir, status: $status,
            needs_volume: false, needs_vix: false, needs_series: &[], book: BOOK, rules: &[$($r),*], notes: $notes, claim: $claim }
    };
}

fn cl(trades: &'static str, win: &'static str, avg: &'static str, hold: &'static str, period: &'static str) -> Option<Claim> {
    Some(Claim { trades, win, avg, hold, period })
}

const IDX: &str = "Original book research used multiple instruments and volatility-scaled dollar sizing. This legacy catalogue entry still requires instrument-specific source, data and accounting review. Registration does not reproduce the source study or authorize an index to substitute for stocks, ETFs or futures.";
const STOCK_RESEARCH: &str = "Reviewed single-stock price research at fixed source settings; release and measured availability are separate from catalogue registration. Signals use completed closes and next actual session opens. Current-inclusive channel extrema/ties, exit-first collision handling, flat selected-window startup and SMA equality edges are declared conventions. Normalized single-stock long/short returns do not reproduce the book's mixed-instrument volatility-scaled dollar study. No stock basket, index substitution, borrowing, financing, complete exchange calendar or corporate-action correctness is certified by these rules.";
const NEXT_VOLBO: &str = "Selected-stock research profile. Printed155 supplies strict2-sigma price-change entries and opposite-entry reversals; printed41 supports population variance. Current-inclusive100 actual close changes, strict squared IEEE754 comparison without epsilon, flat selected start and next actual open are declared conventions. Zero-variance signed constant changes can signal; original10000/sigma sizing is undefined there and is not supplied. Actual stocks and per-request data/account checks remain required; no original performance, historical stock population, quote-adjustment, index-substitution, borrow or financing certification. See docs/kestner-next-stock-candidates.md.";
const NEXT_RSI: &str = "Selected-stock research profile. Printed165/170 supplies long below35 and short above65; contradictory sample prose and copied stochastic table labels do not invert those rules. Declared Wilder RMA14 with arithmetic first14-change seed, zero-filled up/down changes, flat RSI50 and strict LEVEL entries. Printed66 does not specify smoothing, seed or flat handling; naming Wilder is not source certification of this implementation. Full genuine prefix, flat selected start, next actual open and opposite reversals. Nonpositive ordinary normalized accounts remain unavailable without clipping, resets or resized positions. Source-event checks do not reproduce original sizing, performance or borrowing feasibility. See docs/kestner-next-stock-candidates.md.";
const NEXT_MACD_PERIOD: &str = "Named period-alpha selected-stock research profile. Printed57 gives2/(N+1), while printed68 MACD-specific .15/.075 conflicts with2/13,2/27. This profile retains2/13,2/27 and signal1/5; first actual close seeds both price EMAs, MACD/signal start0. Local finite arithmetic uses state+=alpha*(value-state) to preserve constant prices without epsilon. Observed signal-line crossing (previous histogram<=0/current>0, reverse>=0/<0), not MACD-zero crossing; equality holds, full genuine prefix, flat selected start, next actual open. No unique original alpha/default or original performance is claimed; actual-stock data/account checks and qualified adjustments/borrow remain separate. See docs/kestner-next-stock-candidates.md.";
const NEXT_MACD_LITERAL: &str = "Named literal-alpha selected-stock research profile in the same MACD family. Printed68 constants .15/.075 are implemented literally, with declared signal .2 from printed57's9-day rule. They differ from the period2/13,2/27 profile. First-close price seeds, initial MACD/signal0 and local state+=alpha*(value-state) arithmetic preserve constant inputs without epsilon. Observed signal-line crosses and next actual open; no source-original seed or performance certification. Actual stocks and per-request data/account checks remain required; original sizing, qualified adjustments and borrow are not certified. See docs/kestner-next-stock-candidates.md.";
const P90: &str = "SPX 1/1/1990-12/31/2001 (stock table)";

pub fn catalogue() -> Vec<StrategyDef> {
    use Status::*;
    const G7: &str = "Ch.7 Popular strategies (pp.139-178)";
    const G8: &str = "Ch.8 New entries, exits, filters (pp.181-245)";
    const G6: &str = "Ch.6 Filtering (pp.117-124)";
    const G9: &str = "Ch.9 Relative-value strategies (pp.247-287)";
    const G10: &str = "Ch.10 Investing in the S&P 500 (pp.289-303)";
    const G12: &str = "Ch.12 New volatility breakout (pp.321-328)";
    let mut v = vec![
        // ---------------- Chapter 7
        sd!("lkqts_cb4020", "40-day entry / 20-day exit channel breakout", G7, "Long/Short", Live,
            ["E|\"enters long when a market's close is the highest close of the past 40 days\" (p.139)",
             "E|\"enters short when a market's close is the lowest close of the past 40 days\" (pp.139-140)",
             "X|\"Longs are exited if today's close is the lowest close of the past 20 days. Shorts are exited if today's close is the highest close of the past 20 days.\" (p.140)",
             "F|Signals at the close, executed at the next day's open (p.139)"],
            STOCK_RESEARCH, cl("71", "34%", "-$1/share", "53 / 19 bars (win / loss)", P90)),
        sd!("lkqts_ma1040", "10-day / 40-day dual moving average crossover", G7, "Long/Short", Live,
            ["E|\"Long entries are established if the 10-day average crosses above the 40-day average\" (simple averages, p.145)",
             "E|\"Short entries are established if the 10-day average crosses below the 40-day average\" (p.145)",
             "X|\"Positions are exited on an entry signal in the opposite direction\" (p.145)"],
            STOCK_RESEARCH, cl("97", "30%", "$1/share", "59 / 19 bars", P90)),
        sd!("lkqts_mom20", "20-day momentum", G7, "Long/Short", Live,
            ["E|\"long entries are established if today's close is greater than the close 20 days ago\" (p.150)",
             "E|\"Short entries are established if today's close is less than the close 20 days ago\" (p.150)",
             "X|Reverse on the opposite entry (table: \"exit if today's close < close of 20 days ago\", p.151)"],
            IDX, cl("298", "31%", "$0/share", "20 / 6 bars", P90)),
        sd!("lkqts_mom80", "80-day momentum", G7, "Long/Short", Live,
            ["E|\"long entries are established if today's close is greater than the close 80 days ago\" (p.150)",
             "E|\"Short entries are established if today's close is less than the close 80 days ago\" (p.150)",
             "X|Reverse on the opposite entry (p.157 table)"],
            STOCK_RESEARCH, cl("149", "31%", "$1/share", "47 / 8 bars", P90)),
        sd!("lkqts_volbo", "Volatility breakout (2 sigma of 100 price changes)", G7, "Long/Short", Live,
            ["E|\"enter long if today's price change is greater than twice the standard deviation of price changes over the past 100 days\" (p.155)",
             "E|\"enter short if today's price change is less than negative two times the standard deviation of price changes over the past 100 days\" (p.155)",
             "X|\"no rules for exiting positions, except for an entry in the other direction\" (p.155)"],
            NEXT_VOLBO, cl("89", "30%", "-$3/share", "44 / 28 bars", P90)),
        sd!("lkqts_stoch14", "14-day Slow %K stochastic 20/80", G7, "Long/Short", Live,
            ["E|\"Long entries are established when today's 14-day Slow %K stochastic falls below 20 and then crosses back above 20\" (p.155)",
             "E|\"Short entries are established when today's 14-day Slow %K stochastic rises above 80 and then crosses below 80\" (pp.155, 160)",
             "X|\"no rules for exiting positions, except for an entry in the other direction\" (p.160)",
             "F|Slow %K = 3-day SMA of raw %K = (C - lowest low 14)/(highest high 14 - lowest low 14) (pp.66-67)"],
            IDX, cl("90", "64%", "-$1/share", "18 / 61 bars", P90)),
        sd!("lkqts_rsi14", "14-day RSI 35/65", G7, "Long/Short", Live,
            ["E|\"Long entries are established if today's 14-day RSI falls below 35\" (p.165)",
             "E|\"Short entries are established if today's 14-day RSI rises above 65\" (p.165)",
             "X|\"no rules for exiting positions, except for an entry in the other direction\" (p.165)",
             "F|Source p.66 gives average up/down moves; RMA14 smoothing, first14-change seed, flat50 and level startup are declared research conventions, not printed executable instructions"],
            NEXT_RSI, cl("36", "69%", "$0/share", "49 / 160 bars", P90)),
        sd!("lkqts_macd", "MACD 12/26/9 crossover (period alphas)", G7, "Long/Short", Live,
            ["F|Declared period profile: fast2/13, slow2/27, signal1/5 (general EMA rule p.57); p.68 instead specifies .15/.075, retained as a separate named variant",
             "E|\"Long entries are established when the MACD rises above the signal line\" (p.170)",
             "E|\"Short entries are established when the MACD falls below the signal line\" (p.170)",
             "X|Reverse on the opposite entry (GE example: \"our short positions are exited and a long position is established\", p.175)"],
            NEXT_MACD_PERIOD, cl("260", "30%", "-$2/share", "21 / 7 bars", P90)),
        sd!("lkqts_macd_literal_alpha", "MACD 12/26/9 crossover (literal .15/.075 alphas)", G7, "Long/Short", Live,
            ["F|Printed p.68 MACD-specific fast .15 and slow .075; signal .2 is a declared application of p.57's9-day EMA rule",
             "E|Long on an observed MACD/signal upward cross; short on an observed downward cross (p.170)",
             "X|Opposite entry exits and reverses at the next actual session open (pp.139,175)",
             "F|First actual close seeds both price EMAs; MACD/signal start0; equality does not itself enter. These are declared conventions"],
            NEXT_MACD_LITERAL, None),
        // ---------------- Chapter 8
        sd!("lkqts_kma", "Kestner's moving average system (20 & 80 day)", G8, "Long/Short", Live,
            ["F|Sub-system n (n = 20 and 80): long when the close > n-day SMA of highs and the n-day SMA of closes > its value x days ago; short when the close < n-day SMA of lows and the n-day SMA of closes < its value x days ago (p.183)",
             "F|x = 1: the GE example compares \"the 20-day moving average is greater than the prior day's value\" (p.183); the prose says x is \"typically\" the MA length. x = 1 reproduces the book's SPX/NDX rows (80/52 trades vs 73/52 printed; x = n gives 97/58) [fitted reading, p.187 table]",
             "E|\"if the new moving average strategy using a 20-day moving average is long and the new strategy using an 80-day moving average is also long, we will enter a long position\"; short when both are short (p.183)",
             "X|\"Long positions are exited if today's close falls below the 80-day simple moving average of lows. Short positions are exited if today's close rises above the 80-day simple moving average of highs.\" (p.183)"],
            "x_one=0 gives the prose's 'typically x = MA length' reading (non-default).",
            cl("73", "23%", "-$1/share", "105 / 16 bars", P90)),
        sd!("lkqts_sob", "Second order breakout", G8, "Long/Short", Live,
            ["F|Closing-price signed maximum high/low breakout horizon through80 (printed185); declared current-inclusive80 closes, inclusive price ties and neutral equal high/low horizons",
             "E|Formal paragraph printed190: statistic strictly>20 and at its40-observation high; strictly<−20 and at its40-observation low. Declared current-inclusive40 complete statistics and tied extremes qualify",
             "X|Declared research convention: completed-close signals fill next actual open; opposite eligible signal exits/reverses, repeated same-side signal does not add exposure; no independent stop or fabricated terminal exit"],
            "Fixed selected-stock research profile sob_close_max80_current_inclusive_channel40_tie_inclusive_strict20. The Intel narrative uses exactly+20/−20, conflicting with the formal strict threshold; no choice is fitted to performance. First statistic79, first complete predicate118, earliest next-open119; genuine full prefix, selected account starts flat. Chapter7 next-open execution is a declared Chapter8 adaptation. Selected AAPL/RELIANCE source ledgers and four zero/nonzero-cost normalized accounts matched independent references. Every requested run still requires source/data/account and actual-stock admission checks. Original mixed-instrument volatility-dollar sizing, quoted performance, price adjustments, borrow and historical population are not certified.",
            cl("93", "31%", "-$3/share", "51 / 19 bars", P90)),
        sd!("lkqts_macdhr", "MACD histogram retracement", G8, "Long/Short", Live,
            ["F|Histogram = MACD(12,26) - 9-day EMA of MACD (p.191)",
             "E|Long: the histogram has been below -0.5 x the 20-day standard deviation of price changes since its last cross below zero, and \"retraces 25 percent of its minimum value since its last cross below zero\" (p.196)",
             "E|Short: the histogram has exceeded +0.5 x sigma(20) since its last cross above zero and retraces 25% of its maximum since that cross (p.196)",
             "X|Reverse on the opposite entry (KO example, p.196)"],
            "Retrace test: histogram > 0.75 x the minimum (long) or < 0.75 x the maximum (short), evaluated each close, once per swing.",
            cl("53", "62%", "$6/share", "46 / 71 bars", P90)),
        sd!("lkqts_dividx", "Divergence Index", G8, "Long/Short", NotTestable("the book gives entries only (DI = mom10 x mom40 / sigma40^2 < -10, direction of mom40) and no exit; its own SPX/NDX tables (36 trades x ~29 bars) show positions were closed by an unstated rule, so stop-and-reverse would be invented"),
            ["F|Divergence Index = 10-day momentum x 40-day momentum / (40-day standard deviation of price changes)^2 (p.197)",
             "E|\"Long entries are established when the Divergence Index is less than -10 and 40-day momentum is greater than zero. Sell signals are generated when the Divergence Index is less than -10 and 40-day momentum is less than zero.\" (p.197)",
             "X|No exit stated"],
            "Book SPX: 36 trades, 31% win, 63 / 14 bars - only ~1/3 of bars held, so not stop-and-reverse.", cl("36", "31%", "$6/share", "63 / 14 bars", P90)),
        sd!("lkqts_macs", "Moving average confluence method (MACS)", G8, "Long/Short", Live,
            ["F|Printed207:20 MA systems, n versus4n for n1..20. Declared close-SMA and complete80 actual closes; score=5 times the daily long-vote count",
             "E|Primary detailed prose inclusive score>=60 long/<=40 short; the strict summary alternative remains disclosed source ambiguity",
             "F|Pair equality contributes zero long votes under a declared daily-relationship convention; systems-on-long-trades does not uniquely establish equality state",
             "X|Declared flat selected start and completed-close/next actual stock open opposite reversal; no pyramiding or fabricated terminal exit"],
            "Named raw_native_decimal_bounded_window_sums_v4 profile. Exact native RAW closes, SHA/ordinal/date/ingested-price binding, full genuine prefix; first complete state79/fill80. Fixed stock full-prefix source comparisons matched unchanged exact references across all20 pairs with zero mismatches. Shared-author mechanical checks are not blind replication. Fixed AAPL1990-01-01..2001-12-31 and RELIANCE2001-01-01..2010-12-31: MACS and TIAR each matched four ordinary normalized-account cases (fees/slippage0/0 and3/2bps per side, rollover0) and16 direct production-function responses (detail, readiness, selected summary, singleton optimizer) against unchanged source/account references with zero mismatches (combined32responses/eightaccounts). Selected API states and earlier full-prefix source evidence have separate scope. HTTP transport/request-matrix, browser and deployment are separate acceptance gates. Every requested run still needs its own source/data/account checks; data readiness alone does not certify account support. Original mixed-instrument volatility-dollar sizing and book performance are not reproduced. No index/ETF/derivative/basket, quote-adjustment, complete-calendar, borrow or historical-membership certification. See docs/kestner-macs-raw-stock-research.md.",
            cl("58", "29%", "$1/share", "107 / 29 bars", P90)),
        sd!("lkqts_nei", "Normalized envelope indicator (NEI)", G8, "Long/Short", Live,
            ["F|NEI = (Close - average of past 50 closes)/standard deviation of past 50 price changes; past NEIs use today's sigma (p.213)",
             "F|Sort the last 50 NEIs: upper band = today's 50-day MA + 10th-ranked NEI, lower band = 50-day MA + 40th-ranked NEI (p.213); the NEI is in sigma units, so it is converted back to price (x today's sigma) - this reading gives SPX 98 trades / 65% win vs the book's 98 / 66% (as printed, unscaled: 157 / 39%) [fitted reading]",
             "E|\"I enter short when prices rise above and then fall below the upper NEI band. Long entries are initiated when prices fall below and then rise above the lower NEI band.\" (p.213)",
             "X|Reverse on the opposite entry (XOM example: \"Profits are taken and shorts entered\", p.213)"],
            "Option scale=0 adds the unitless NEI to the price MA exactly as printed (non-default; does not reproduce the book).",
            cl("98", "66%", "$5/share", "25 / 41 bars", P90)),
        sd!("lkqts_meo", "Multiple Entry Oscillator system", G8, "Long/Short", NotTestable("unit changes are listed only for upward/downward crosses of 55/70/85 and 45/30/15; the book's IP example also covers one short unit on a re-cross of 15, a reduction rule the text never states"),
            ["F|20-day Slow %K stochastic (p.213)",
             "E|crosses above 55/70/85: long 1/2/3 units; crosses below 45/30/15: short 1/2/3 units (p.213)"],
            "Only portfolio equity curves are printed (Figs. 8.21-8.22).", None),
        sd!("lkqts_adjsto", "Adjusted stochastic", G8, "Long/Short", Live,
            ["F|New Stochastic = (14-day Slow %K - 50) x (HH14 - LL14)/(HH100 - LL100) + 50 (p.219)",
             "E|\"We will take countertrend short positions when the adjusted stochastic rises above 65 and then falls back below 65. Countertrend long positions are entered when the adjusted stochastic declines below and then rises back above 35.\" (p.219)",
             "X|Reverse on the opposite entry (TXN example, p.219)"],
            IDX, cl("51", "65%", "-$7/share", "29 / 113 bars", P90)),
        sd!("lkqts_tiar", "Three in a row", G8, "Long/Short", Live,
            ["E|Printed222/227: long close>close5>close10>close15; short strict reverse chain at each completed close. Exact fixed step5; equality supplies no new direction",
             "F|Genuine full-prefix actual closes; first complete state15, earliest next-open16. Strict close levels, not observed crossings or three consecutive daily returns",
             "X|Declared Chapter8 research convention: flat selected start, next actual stock open, opposite entry exits/reverses; same-side repeats do not add; terminal holding is marked, not fabricated as a closed exit"],
            "Fixed selected-stock research profile tiar_close_lags5_10_15_strict_levels_flatstart_nextopen_reverse. Fixed AAPL1990–2001/RELIANCE2001–2010 source events, four zero/nonzero-cost ordinary accounts and16 detail/readiness/selected-summary/singleton-optimizer production-function responses matched unchanged references. Shared-author mechanical checks are not blind independent replication. Selected API states and earlier full-prefix source proof are separate evidence; HTTP transport/request-matrix, browser and deployment are separate acceptance gates. Original mixed-instrument volatility-dollar sizing, book dollar performance, borrow, adjustment correctness, complete exchange calendars and historical populations are not certified. See docs/kestner-tiar-stock-research.md.",
            cl("78", "33%", "$0/share", "67 / 24 bars", P90)),
        StrategyDef { needs_volume: true, ..sd!("lkqts_volrev", "Volume reversal strategy", G8, "Long/Short", Live,
            ["F|Five-day absolute price change > 100-day standard deviation of price changes (p.227)",
             "F|Five-day average volume < 75% of \"the five-day average volume beginning 10 days prior\" (days t-10..t-6) (p.227)",
             "E|\"We enter long if the most recent five-day price change is negative while we enter short if the five-day price change is positive.\" (p.227)",
             "X|\"All entries are exited on the fifth day of the trade.\" - at the open of the fifth day (entry day = day 1), the book's only execution price (p.139)"],
            "Needs daily volume (index volume on SPX/NIFTY). Option exit_close=1 exits at the fifth day's close instead.",
            cl("26", "69%", "$6/share", "4 / 5 bars", P90)) },
        sd!("lkqts_saitta", "Saitta's support and resistance strategy", G8, "Long/Short", Live,
            ["F|Phase turns positive when the close rises above the 20-day SMA of highs, negative when it falls below the 20-day SMA of lows (p.233)",
             "F|When an uptrend starts, the lowest close of the prior downtrend is a bottom; when a downtrend starts, the highest close of the prior uptrend is a top (p.233)",
             "E|\"Long positions ... are entered when prices rise above a previous market top, while short positions are entered when prices fall below a previous market bottom\" (close vs the latest top/bottom, p.233)",
             "X|Reverse on the opposite entry"],
            "Book: \"lowest closing price (or low)\" - closes by default; option hl=1 uses highs/lows.",
            cl("62", "34%", "$1/share", "85 / 29 bars", P90)),
        sd!("lkqts_cb_atrstop", "Channel breakout with 2-ATR stop loss", G8, "Long/Short", Live,
            ["E|40-day entry / 20-day exit channel breakout (p.236)",
             "X|\"exits longs if prices fall below two times the 20-day average true range from the entry price. Short positions are exited if prices rise more than two times the 20-day average true range above the entry point.\" (p.237) - close vs entry -/+ 2 x ATR20 at the signal day, exit next open"],
            "Book prints portfolio totals only (Fig. 8.34).", None),
        sd!("lkqts_cb_ptake", "Channel breakout with profit taking (Strategy 2)", G8, "Long/Short", Live,
            ["E|40/20 channel breakout, but \"No new long entries are allowed unless five days have passed since the last 40-day high\" (no 40-day high on the 5 previous days); shorts likewise (p.242)",
             "X|20-day opposite extreme, plus \"exit if open profit is greater than three times the average true range of the past 20 days\" (close-based, p.242)"],
            "Portfolio Sharpe/K only (Fig. 8.35).", None),
        sd!("lkqts_cb_pyr", "Channel breakout with pyramiding (Strategy 3)", G8, "Long/Short", Live,
            ["E|40/20 channel breakout (p.242)",
             "A|\"we add another unit long if today is a 40-day high and there have been no new 40-day highs over the five previous days\"; shorts likewise (p.242)",
             "X|20-day opposite extreme"],
            "Each unit = one full position (returns add up per unit, no cap stated).", None),
        sd!("lkqts_cb_trendstat", "Channel breakout with overlap trend statistic", G8, "Long/Short", NotTestable("the overlap statistic's look-back is stated as both \"the past 100 days\" and \"the past 20 days\" (p.243); the book does not resolve which"),
            ["E|40-day breakout only when the trend statistic < 0.15; exits on 20-day extremes (p.244)"], "", None),
        sd!("lkqts_trend_filters", "Other new trend filters (40-day high recency; 10-day range expansion)", G8, "Long/Short", NotTestable("filters only - the book suggests combining them with other systems but specifies no complete system or exit (p.243)"),
            ["F|\"take long entries only if a 40-day high was made more recently than a 40-day low\" (p.243)", "F|range of past 10 days > range of the 10 days beginning 20 days ago (p.243)"], "", None),
        // ---------------- Chapter 6
        sd!("lkqts_cb_afterloss", "Channel breakout - only after a losing trade", G6, "Long/Short", Live,
            ["E|40/20 channel breakout entries taken \"only ... when ... the last trade was a loser\" (p.123)",
             "X|20-day opposite extreme"],
            "Last trade = the last closed trade of the unfiltered breakout (shadow trades); taking only real trades would stop all trading after one winner. Book portfolio: futures $2,971,124 vs $5,971,279 for all trades (Fig. 6.14).", None),
        sd!("lkqts_cb_afterwin", "Channel breakout - only after a winning trade", G6, "Long/Short", Live,
            ["E|40/20 channel breakout entries taken only when the last trade was a winner (p.123)", "X|20-day opposite extreme"],
            "Shadow-trade reading as for lkqts_cb_afterloss.", None),
        sd!("lkqts_regime", "ADX regime switching (channel breakout / RSI)", G6, "Long/Short", NotTestable("the book gives regimes (14-day ADX < 20 or > 30: 40/20 breakout; 20-30: 14-day RSI) but no rule for positions open when the regime changes; its test only bins daily P&L by ADX"),
            ["F|Fig. 6.13 regimes (p.123)"], "", None),
        sd!("lkqts_adx_bins", "Channel breakout with ADX band filter", G6, "Long/Short", NotTestable("a parameter sweep (ADX between x and x+5, x = 15..30, optionally rising) with results only as bar charts; no single rule is recommended"),
            ["F|Take breakout signals only when the 14-day ADX is between x and x+5 (p.118)"], "", None),
        // ---------------- Chapter 9
        sd!("lkqts_d100", "Difference from 100-day moving average", G9, "Long/Short", Live,
            ["F|100-day stat = (Close - 100-day SMA of closes)/100-day standard deviation of price changes (p.277)",
             "E|\"Long entries are entered when the 100-day stat falls below -2.5, while short positions are established when this statistic rises above 2.5\" (p.277)",
             "X|\"Long entries are exited when the 100-day stat rises above zero, and short entries are exited when the 100-day stat falls below zero.\" (p.277)"],
            "Book applied it to 30 relative-value spreads; here on an index (instrument swap).", None),
        sd!("lkqts_d1040", "Difference between 10- and 40-day moving averages", G9, "Long/Short", Live,
            ["F|10/40 stat = (10-day SMA - 40-day SMA)/100-day standard deviation of price changes (p.280)",
             "E|\"Long entries are established when the 10/40 stat falls below -2 ... Short entries are established when the 10/40 stat rises above 2\" (p.280)",
             "X|longs \"are exited if the 10/40 stat rises above zero\"; shorts \"are exited if the 10/40 stat falls below zero\" (p.280)"],
            "Book applied it to relative-value spreads; here on an index.", None),
        sd!("lkqts_rv_markets", "Relative-value markets (yield curve, credit, VIX, index ratios, stock pairs, commodity ratios)", G9, "Long/Short", NeedsData("30 spread series (Treasury 2/5/10/30y yields, 5y swap spreads, Merrill Lynch HY yields, spot commodities, 15 US stock pairs, XAU/XOI) and two-leg spread execution; the engine trades one instrument"),
            ["F|ln(SPX/NDX), ln(SPX/RUT), yield spreads, ... (Fig. 9.17, p.268); position size $10,000/sigma(100)"], "", None),
        // ---------------- Chapter 10
        sd!("lkqts_dom1", "Day-of-month strategy 1 (days 1-5 and 27-31)", G10, "Long", Live,
            ["E|\"We are long the market during Days one through five and Days 27 through 31\" (calendar days, p.297)",
             "X|\"Assets are invested in cash for the remainder of the month\" (p.297)",
             "F|In the market for a day = hold from the previous close to that day's close (book uses daily close-to-close returns)"],
            "", cl("-", "-", "$100 -> $454 (B&H $419)", "<30% of the time", "S&P 500 1/1990-2001")),
        sd!("lkqts_dom2", "Day-of-month strategy 2 (days 1-5, 12-18, 27-31)", G10, "Long", Live,
            ["E|\"We are long the market during Days one through five, 12 through 18, and 27 through 31\" (p.297)",
             "X|In cash the remainder of the month (p.297)"],
            "", cl("-", "-", "$100 -> $758 (B&H $419)", "54% of the time", "S&P 500 1/1990-2001")),
        StrategyDef { needs_vix: true, ..sd!("lkqts_vixscore", "Kestner VIX score strategy", G10, "Long", Live,
            ["F|VIX score = (VIX today - 120-day average VIX)/standard deviation of daily VIX changes over 120 days (p.301)",
             "E|\"Our new VIX strategy buys the market when today's VIX score is greater than +2\" (p.301)",
             "X|\"We stay long until the current day's score falls below -2.\" (p.301)",
             "F|Fills at the signal day's close (Ch.10 measures close-to-close returns)"],
            "US: CBOE VIX; India: India VIX (from 2009; window starts at the first VIX bar).", cl("-", "-", "beats buy & hold", "53% of the time", "S&P 500 1990-2001"))},
        StrategyDef { needs_vix: true, ..sd!("lkqts_gross_vix", "Leon Gross VIX 30/20 (reported by Kestner)", G10, "Long", Live,
            ["E|\"entering long positions in the S&P 500 on VIX crosses above 30\" (p.299)", "X|\"exiting into cash on drops below 20\" (p.299)"],
            "Not Kestner's own rule; reported as a fixed-level method. Fills at the close.", None)},
        sd!("lkqts_headley_vix", "Price Headley VIX Bollinger method (reported)", G10, "Long", NotTestable("the book quotes only the entry (VIX crossed below its upper 21-day 2-sd band five days ago and S&P 100 close > 10-day MA); no exit is given"),
            ["E|p.299"], "", None),
        sd!("lkqts_rate_timing", "Monthly timing with 1-year / 10-year Treasury yields (19 rules of Fig. 10.3)", G10, "Long", NeedsData("monthly 1-year and 10-year US Treasury yields (India: RBI/NSE G-sec yields) and the S&P 500 total-return index with T-bill cash returns"),
            ["E|e.g. \"10 year yields<3 month average\" -> $6,819 from $100 (1970-2001), 50% invested (Fig. 10.3, p.296)"], "", None),
    ];
    for (id, name, rule, claim) in [
        ("lkqts_m_sma12", "Monthly: SPX > 12-month average", "E|Invested when the month-end close > the average of the last 12 month-end closes (Fig. 10.3)", ("$4,123", "76%")),
        ("lkqts_m_sma6", "Monthly: SPX > 6-month average", "E|Invested when the month-end close > the average of the last 6 month-end closes", ("$1,879", "71%")),
        ("lkqts_m_sma3", "Monthly: SPX > 3-month average", "E|Invested when the month-end close > the average of the last 3 month-end closes", ("$1,310", "63%")),
        ("lkqts_m_mom12", "Monthly: SPX > 12 months ago", "E|Invested when the month-end close > the month-end close 12 months earlier", ("$4,211", "81%")),
        ("lkqts_m_mom6", "Monthly: SPX > 6 months ago", "E|Invested when the month-end close > the month-end close 6 months earlier", ("$2,132", "72%")),
        ("lkqts_m_mom3", "Monthly: SPX > 3 months ago", "E|Invested when the month-end close > the month-end close 3 months earlier", ("$2,006", "71%")),
    ] {
        let rules: &'static [&'static str] = Box::leak(vec![rule, "X|Otherwise in cash; decided at each month-end close (p.294-296)", "F|Book used the S&P 500 total-return index and paid T-bill interest in cash; here the price index, no cash interest"].into_boxed_slice());
        v.push(StrategyDef { id, name, group: G10, session: 0, dir: "Long", status: Live, needs_volume: false, needs_vix: false,
            needs_series: &[], book: BOOK, rules, notes: "Fills at the close of the last trading day of the month.",
            claim: cl("-", "-", Box::leak(format!("$100 -> {}", claim.0).into_boxed_str()), Box::leak(format!("{} invested", claim.1).into_boxed_str()), "S&P 500 TR 1970-2001") });
    }
    v.push(sd!("lkqts_nvb_entry", "New volatility breakout - entry only (Version 1)", G12, "Long/Short", Live,
        ["E|For every x from 5 to 20: buy if close - close x days ago > num x sqrt(x) x sigma(100 price changes); sell if < -num x sqrt(x) x sigma (p.322)",
         "F|\"the longer term signal always overrides the shorter term signal\" (p.323); num = 1.0 (p.325)",
         "X|Version #1 \"includes only the entry mechanism\" - reverse on the opposite signal (p.324)"],
        "Book tested 19 trend-prone futures only.", None));
    v.push(sd!("lkqts_nvb_trail", "New volatility breakout with 2-sigma trailing stop (Version 2)", G12, "Long/Short", Live,
        ["E|Version 1 entries, num = 1.0 (p.325)",
         "X|\"If we're long and prices fall from the highest high seen during the trade by twice the 100-day standard deviation of price returns, we'll exit our long position. If we're short and prices rise from the lowest low seen during the trade by twice ... we'll exit our short position.\" (p.324) - close-based, exit next open"],
        "Book (19 futures): $8,741,153, Sharpe 0.74, K 0.42 (Fig. 12.4a).", None));
    v
}

pub fn family_of(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    if !id.starts_with("lkqts_") {
        return None;
    }
    Some(match id {
        "lkqts_cb4020" | "lkqts_cb_atrstop" | "lkqts_cb_ptake" | "lkqts_cb_pyr" | "lkqts_cb_trendstat" | "lkqts_cb_afterloss" | "lkqts_cb_afterwin" | "lkqts_adx_bins" | "lkqts_regime" | "lkqts_trend_filters" =>
            ("lkqts_cb", "Kestner 40/20 channel breakout", match id {
                "lkqts_cb4020" => "Plain", "lkqts_cb_atrstop" => "2-ATR stop", "lkqts_cb_ptake" => "Profit taking", "lkqts_cb_pyr" => "Pyramiding",
                "lkqts_cb_afterloss" => "After a loser", "lkqts_cb_afterwin" => "After a winner", "lkqts_cb_trendstat" => "Trend statistic filter",
                "lkqts_adx_bins" => "ADX band filter", "lkqts_regime" => "ADX regime switch", _ => "Trend filters" }),
        "lkqts_mom20" | "lkqts_mom80" => ("lkqts_mom", "Kestner momentum", if id == "lkqts_mom20" { "20-day" } else { "80-day" }),
        "lkqts_macd" | "lkqts_macd_literal_alpha" => ("lkqts_macd", "Kestner MACD", if id=="lkqts_macd" {"Period-derived alphas"} else {"Literal .15/.075 alphas"}),
        "lkqts_d100" | "lkqts_d1040" | "lkqts_rv_markets" => ("lkqts_rv", "Kestner relative-value exhaustion", match id { "lkqts_d100" => "100-day stat", "lkqts_d1040" => "10/40 stat", _ => "Spread markets" }),
        "lkqts_dom1" | "lkqts_dom2" => ("lkqts_dom", "Kestner day of month", if id == "lkqts_dom1" { "Strategy 1" } else { "Strategy 2" }),
        "lkqts_vixscore" | "lkqts_gross_vix" | "lkqts_headley_vix" => ("lkqts_vix", "VIX timing of the S&P 500", match id { "lkqts_vixscore" => "Kestner VIX score", "lkqts_gross_vix" => "Gross 30/20", _ => "Headley bands" }),
        "lkqts_nvb_entry" | "lkqts_nvb_trail" => ("lkqts_nvb", "Kestner new volatility breakout", if id == "lkqts_nvb_entry" { "Entry only" } else { "Trailing stop" }),
        s if s.starts_with("lkqts_m_") || s == "lkqts_rate_timing" => ("lkqts_monthly", "Kestner monthly S&P timing", match s {
            "lkqts_m_sma12" => "> 12-month average", "lkqts_m_sma6" => "> 6-month average", "lkqts_m_sma3" => "> 3-month average",
            "lkqts_m_mom12" => "> 12 months ago", "lkqts_m_mom6" => "> 6 months ago", "lkqts_m_mom3" => "> 3 months ago", _ => "Interest-rate rules" }),
        _ => (id_static(id)?, "Kestner single system", "Book rules"),
    })
}

fn id_static(id: &str) -> Option<&'static str> {
    catalogue().into_iter().find(|d| d.id == id).map(|d| d.id)
}

pub fn params(id: &str) -> Option<Vec<ParamDef>> {
    let p = |key, label, def, min, max, step| ParamDef { key, label, def, min, max, step };
    let cb = || vec![p("entry", "Entry channel (closes)", 40., 10., 100., 5.), p("exit", "Exit channel (closes)", 20., 5., 50., 5.)];
    Some(match id {
        "lkqts_cb4020" => vec![p("entry","Entry channel (closes)",40.,40.,40.,1.),p("exit","Exit channel (closes)",20.,20.,20.,1.)],
        "lkqts_cb_afterloss" | "lkqts_cb_afterwin" => cb(),
        "lkqts_cb_atrstop" => { let mut v = cb(); v.push(p("atr_n", "ATR length", 20., 5., 50., 5.)); v.push(p("stop", "Stop, ATR multiples from entry", 2., 0.5, 5., 0.5)); v }
        "lkqts_cb_ptake" => { let mut v = cb(); v.push(p("atr_n", "ATR length", 20., 5., 50., 5.)); v.push(p("target", "Profit exit, ATR multiples", 3., 1., 6., 0.5)); v.push(p("gap", "Days without a 40-day extreme before a new entry", 5., 1., 10., 1.)); v }
        "lkqts_cb_pyr" => { let mut v = cb(); v.push(p("gap", "Days without a 40-day extreme before adding", 5., 1., 10., 1.)); v }
        "lkqts_ma1040" => vec![p("fast", "Fast SMA", 10., 10., 10., 1.), p("slow", "Slow SMA", 40., 40., 40., 1.)],
        "lkqts_mom20" => vec![p("n", "Momentum days", 20., 5., 120., 5.)],
        "lkqts_mom80" => vec![p("n", "Momentum days", 80., 80., 80., 1.)],
        "lkqts_volbo" => vec![p("k", "Sigma multiple", 2., 2., 2., 1.), p("sd_n", "Sigma look-back (price changes)", 100., 100., 100., 1.)],
        "lkqts_stoch14" => vec![p("n", "Stochastic days", 14., 5., 30., 1.), p("lo", "Oversold level", 20., 5., 40., 5.), p("hi", "Overbought level", 80., 60., 95., 5.)],
        "lkqts_rsi14" => vec![p("n", "Declared RMA days", 14., 14., 14., 1.), p("lo", "Level long below", 35., 35., 35., 1.), p("hi", "Level short above", 65., 65., 65., 1.)],
        "lkqts_macd" | "lkqts_macd_literal_alpha" => vec![p("fast", "Fast EMA label", 12., 12., 12., 1.), p("slow", "Slow EMA label", 26., 26., 26., 1.), p("sig", "Signal EMA days", 9., 9., 9., 1.)],
        "lkqts_kma" => vec![p("n1", "Short MA length", 20., 5., 50., 5.), p("n2", "Long MA length", 80., 40., 200., 10.), p("x_one", "Filter: compare with yesterday (1, book's GE example and results) or n days ago (0, p.183 'typically')", 1., 0., 1., 1.)],
        "lkqts_sob" => vec![p("search", "Maximum current-inclusive close horizon", 80., 80., 80., 1.), p("chan", "Current-inclusive statistic channel", 40., 40., 40., 1.), p("min", "Strict absolute statistic threshold", 20., 20., 20., 1.)],
        "lkqts_macdhr" => vec![p("retrace", "Retracement of the extreme", 0.25, 0.1, 0.5, 0.05), p("arm", "Arming level, x sigma(20)", 0.5, 0.0, 2.0, 0.25), p("sd_n", "Sigma look-back", 20., 10., 50., 5.)],
        "lkqts_macs" => vec![p("long", "Fixed inclusive long score", 60., 60., 60., 1.), p("short", "Fixed inclusive short score", 40., 40., 40., 1.)],
        "lkqts_nei" => vec![p("n", "Look-back days", 50., 20., 100., 10.), p("up", "Upper band rank", 10., 1., 25., 1.), p("dn", "Lower band rank", 40., 25., 50., 1.), p("scale", "Band = MA + ranked NEI x sigma (1, reproduces the book's SPX table) or + NEI as printed (0)", 1., 0., 1., 1.)],
        "lkqts_adjsto" => vec![p("n", "Stochastic days", 14., 5., 30., 1.), p("long_n", "Range normaliser days", 100., 50., 200., 10.), p("lo", "Long cross above", 35., 20., 45., 1.), p("hi", "Short cross below", 65., 55., 80., 1.)],
        "lkqts_tiar" => vec![p("step", "Fixed close lag unit (5/10/15 actual observations)", 5., 5., 5., 1.)],
        "lkqts_volrev" => vec![p("sd_n", "Sigma look-back", 100., 20., 250., 10.), p("vol_pct", "Volume ratio below", 0.75, 0.5, 1.0, 0.05), p("hold", "Exit on day N of the trade", 5., 2., 10., 1.), p("exit_close", "Exit at that day's close (1) instead of open (0)", 0., 0., 1., 1.)],
        "lkqts_saitta" => vec![p("n", "SMA of highs/lows", 20., 5., 50., 5.), p("hl", "Tops/bottoms from highs/lows (1) instead of closes (0)", 0., 0., 1., 1.)],
        "lkqts_d100" => vec![p("n", "MA / sigma days", 100., 20., 250., 10.), p("k", "Entry threshold", 2.5, 1., 4., 0.25)],
        "lkqts_d1040" => vec![p("fast", "Fast SMA", 10., 5., 20., 1.), p("slow", "Slow SMA", 40., 20., 100., 5.), p("sd_n", "Sigma days", 100., 20., 250., 10.), p("k", "Entry threshold", 2., 1., 4., 0.25)],
        "lkqts_dom1" | "lkqts_dom2" => vec![],
        "lkqts_vixscore" => vec![p("n", "Average / sigma days", 120., 40., 250., 10.), p("hi", "Buy above score", 2., 0.5, 4., 0.25), p("lo", "Exit below score", -2., -4., 0., 0.25)],
        "lkqts_gross_vix" => vec![p("hi", "Buy on VIX cross above", 30., 15., 50., 1.), p("lo", "Exit on VIX drop below", 20., 10., 30., 1.)],
        "lkqts_m_sma12" | "lkqts_m_sma6" | "lkqts_m_sma3" | "lkqts_m_mom12" | "lkqts_m_mom6" | "lkqts_m_mom3" => vec![],
        "lkqts_nvb_entry" => vec![p("num", "num (sigma multiple)", 1.0, 0.5, 3., 0.25), p("x_min", "Shortest change (days)", 5., 2., 10., 1.), p("x_max", "Longest change (days)", 20., 10., 40., 1.)],
        "lkqts_nvb_trail" => vec![p("num", "num (sigma multiple)", 1.0, 0.5, 3., 0.25), p("x_min", "Shortest change (days)", 5., 2., 10., 1.), p("x_max", "Longest change (days)", 20., 10., 40., 1.), p("trail", "Trailing stop, sigma multiples", 2., 0.5, 5., 0.5)],
        _ => return None,
    })
}

// ------------------------------------------------------------------ strategies

fn cross_up(v: &[f64], i: usize, lvl: f64) -> bool {
    i > 0 && v[i - 1] <= lvl && v[i] > lvl
}
fn cross_dn(v: &[f64], i: usize, lvl: f64) -> bool {
    i > 0 && v[i - 1] >= lvl && v[i] < lvl
}

struct CbOpt {
    entry: usize,
    exit: usize,
    atr_n: usize,
    stop: f64,
    target: f64,
    gap: usize,  // 0 = no gap rule
    pyr: usize,  // 0 = no pyramiding, else gap for adds
}

/// Kestner 40/20 channel breakout family (pp.139-140, 236-242).
fn cb(x: &Ctx, from: usize, to: usize, o: &CbOpt) -> Vec<Trade> {
    let c = &x.c;
    let atr = atr_s(x, o.atr_n.max(1));
    let warm = o.entry.max(o.exit);
    let hi = |i: usize, n: usize| i + 1 >= n && c[i] >= hh(c, i, n);
    let lo = |i: usize, n: usize| i + 1 >= n && c[i] <= ll(c, i, n);
    let none_before = |i: usize, g: usize, f: &dyn Fn(usize) -> bool| i >= g && (i - g..i).all(|k| !f(k));
    let mut stop_lvl = f64::NAN;
    run_target(x, from.max(warm), to, Fill::NextOpen, |i, pos, t| {
        let h40 = hi(i, o.entry);
        let l40 = lo(i, o.entry);
        let mut want = pos;
        let mut why = "";
        if pos > 0 {
            let t = t.unwrap();
            let e = t.legs[0].px;
            if lo(i, o.exit) {
                want = 0; why = "20-day lowest close";
            } else if o.stop > 0.0 && c[i] < stop_lvl {
                want = 0; why = "2 x ATR(20) stop below entry";
            } else if o.target > 0.0 && c[i] - e > o.target * atr[i] {
                want = 0; why = "open profit > 3 x ATR(20)";
            }
        } else if pos < 0 {
            let t = t.unwrap();
            let e = t.legs[0].px;
            if hi(i, o.exit) {
                want = 0; why = "20-day highest close";
            } else if o.stop > 0.0 && c[i] > stop_lvl {
                want = 0; why = "2 x ATR(20) stop above entry";
            } else if o.target > 0.0 && e - c[i] > o.target * atr[i] {
                want = 0; why = "open profit > 3 x ATR(20)";
            }
        }
        let gap_ok_l = o.gap == 0 || none_before(i, o.gap, &|k| hi(k, o.entry));
        let gap_ok_s = o.gap == 0 || none_before(i, o.gap, &|k| lo(k, o.entry));
        if want <= 0 && h40 && gap_ok_l {
            want = 1; why = "40-day highest close";
        } else if want >= 0 && l40 && gap_ok_s {
            want = -1; why = "40-day lowest close";
        } else if o.pyr > 0 && want > 0 && h40 && none_before(i, o.pyr, &|k| hi(k, o.entry)) {
            want += 1; why = "add unit: new 40-day high";
        } else if o.pyr > 0 && want < 0 && l40 && none_before(i, o.pyr, &|k| lo(k, o.entry)) {
            want -= 1; why = "add unit: new 40-day low";
        }
        if want != pos && want.signum() != pos.signum() && want != 0 && o.stop > 0.0 {
            // stop distance fixed with the ATR "at the time of entry" (signal day)
            stop_lvl = x.o.get(i + 1).copied().unwrap_or(f64::NAN) - want.signum() as f64 * o.stop * atr[i];
        }
        (want != pos).then_some((want, why))
    })
}

/// Bounded plain-channel convention; legacy stop/filter/pyramiding variants keep
/// their existing runner. Ambiguous opposing entries do not choose a direction.
/// An existing position's 20-close exit takes priority over same-side re-entry;
/// an unambiguous opposite 40-close entry reverses at that same next open.
fn stock_channel(x: &Ctx, from: usize, to: usize) -> Vec<Trade> {
    run_target(x, from.max(39), to, Fill::NextOpen, |i, pos, _| {
        let c=x.c[i];
        let long=c>=hh(&x.c,i,40);
        let short=c<=ll(&x.c,i,40);
        let direction=if long && !short {1} else if short && !long {-1} else {0};
        let exit=(pos>0 && c<=ll(&x.c,i,20)) || (pos<0 && c>=hh(&x.c,i,20));
        if pos!=0 && direction==-pos {
            Some((direction,"opposite 40-close entry"))
        } else if exit {
            Some((0,"20-close exit; exit-first convention"))
        } else if pos==0 && direction!=0 {
            Some((direction,"unambiguous 40-close extreme"))
        } else { None }
    })
}

fn stoch_sar(k: &[f64], from: usize, to: usize, x: &Ctx, lo: f64, hi: f64) -> Vec<Trade> {
    let sig: Vec<i8> = (0..k.len()).map(|i| if cross_up(k, i, lo) { 1 } else if cross_dn(k, i, hi) { -1 } else { 0 }).collect();
    sar(x, from, to, &sig, "oscillator crossed back above the low level", "oscillator crossed back below the high level")
}

pub fn run(id: &str, x: &Ctx, from: usize, to: usize, p: &Params) -> Option<Vec<Trade>> {
    if !id.starts_with("lkqts_") {
        return None;
    }
    if id=="lkqts_sob" && (validate_sob_candidate(p).is_some() || sob_history_issue(x,from,to).is_some()) {return None;}
    if tiar_candidate(id) {return tiar_source_run(x,from,to,p);}
    // RAW-native MACS requires a typed verified series; generic Ctx dispatch must not fall back.
    if macs_candidate(id) {return None;}
    if stock_candidate(id) && validate_stock_candidate(id,p).is_some() { return None; }
    if next_candidate(id) && (validate_next_candidate(id,p).is_some() || next_history_issue(x,from,to).is_some()) {return None;}
    let defs = params(id)?;
    let g = |k: &str| p.get(k).copied().unwrap_or_else(|| defs.iter().find(|d| d.key == k).map_or(f64::NAN, |d| d.def));
    let u = |k: &str| g(k).round().max(1.0) as usize;
    let c = &x.c;
    let n = c.len();
    if from >= to || to >= n {
        return Some(vec![]);
    }
    let cbo = |gap, pyr, stop, target| CbOpt { entry: u("entry"), exit: u("exit"), atr_n: if stop > 0.0 || target > 0.0 { u("atr_n") } else { 20 }, stop, target, gap, pyr };
    let t = match id {
        "lkqts_cb4020" => stock_channel(x, from, to),
        "lkqts_cb_atrstop" => cb(x, from, to, &cbo(0, 0, g("stop"), 0.0)),
        "lkqts_cb_ptake" => cb(x, from, to, &cbo(u("gap"), 0, 0.0, g("target"))),
        "lkqts_cb_pyr" => cb(x, from, to, &cbo(0, u("gap"), 0.0, 0.0)),
        "lkqts_cb_afterloss" | "lkqts_cb_afterwin" => {
            let want_win = id == "lkqts_cb_afterwin";
            let shadow = cb(x, 0, to, &cbo(0, 0, 0.0, 0.0));
            let mut out = Vec::new();
            for (k, t) in shadow.iter().enumerate() {
                let sig_i = t.legs[0].i - 1; // entry signal bar (fills are next-open)
                if sig_i < from {
                    continue;
                }
                // last shadow trade already closed at the signal bar (no peeking at a same-open exit fill)
                let prev = shadow[..k].iter().rev().find(|q| !q.open && q.exit_i <= sig_i);
                let Some(prev) = prev else { continue };
                let won = prev.dir * (prev.exit_px - prev.legs[0].px) > 0.0;
                if won == want_win {
                    out.push(t.clone());
                }
            }
            out
        }
        "lkqts_ma1040" => {
            let (f, s) = (x.sma_n(u("fast")), x.sma_n(u("slow")));
            let d: Vec<f64> = (0..n).map(|i| f[i] - s[i]).collect();
            let sig: Vec<i8> = (0..n).map(|i| if cross_up(&d, i, 0.0) { 1 } else if cross_dn(&d, i, 0.0) { -1 } else { 0 }).collect();
            sar(x, from, to, &sig, "10-day SMA crossed above 40-day SMA", "10-day SMA crossed below 40-day SMA")
        }
        "lkqts_mom20" | "lkqts_mom80" => {
            let m = u("n");
            let sig: Vec<i8> = (0..n).map(|i| if i < m { 0 } else if c[i] > c[i - m] { 1 } else if c[i] < c[i - m] { -1 } else { 0 }).collect();
            sar(x, from, to, &sig, "close > close n days ago", "close < close n days ago")
        }
        "lkqts_volbo" => {
            let sig=next_volbo_signals(&c[..=to])?;
            sar(x, from, to, &sig, "price change > 2 sigma(100)", "price change < -2 sigma(100)")
        }
        "lkqts_stoch14" => stoch_sar(&slow_k(x, u("n")), from, to, x, g("lo"), g("hi")),
        "lkqts_rsi14" => {
            let states=next_rsi_states(&c[..=to])?;
            let sig:Vec<_>=states.iter().map(|v|v.map_or(0,|(up,down)|next_rsi_side(up,down))).collect();
            sar(x, from, to, &sig, "RSI(14) below 35", "RSI(14) above 65")
        }
        "lkqts_macd" | "lkqts_macd_literal_alpha" => {
            let d=next_macd_histogram(&c[..=to],id=="lkqts_macd_literal_alpha")?;
            let sig: Vec<i8> = (0..d.len()).map(|i| if cross_up(&d, i, 0.0) { 1 } else if cross_dn(&d, i, 0.0) { -1 } else { 0 }).collect();
            sar(x, from, to, &sig, "MACD crossed above signal", "MACD crossed below signal")
        }
        "lkqts_kma" => {
            let (n1, n2) = (u("n1"), u("n2"));
            let one = g("x_one") >= 0.5;
            let hs = |m: usize| (sma(&x.h, m), sma(&x.l, m), sma(c, m));
            let state = |m: usize| -> Vec<i8> {
                let (ah, al, ac) = hs(m);
                let lag = if one { 1 } else { m };
                let mut st = vec![0i8; n];
                let mut s = 0i8;
                for i in 0..n {
                    if i >= lag && ac[i].is_finite() && ac[i - lag].is_finite() {
                        if c[i] > ah[i] && ac[i] > ac[i - lag] { s = 1 } else if c[i] < al[i] && ac[i] < ac[i - lag] { s = -1 }
                    }
                    st[i] = s;
                }
                st
            };
            let (s1, s2) = (state(n1), state(n2));
            let (ah2, al2) = (sma(&x.h, n2), sma(&x.l, n2));
            run_target(x, from, to, Fill::NextOpen, |i, pos, _| {
                let agree = if s1[i] == 1 && s2[i] == 1 { 1 } else if s1[i] == -1 && s2[i] == -1 { -1 } else { 0 };
                if pos > 0 && c[i] < al2[i] {
                    return Some(if agree == -1 { (-1, "both systems short") } else { (0, "close below 80-day SMA of lows") });
                }
                if pos < 0 && c[i] > ah2[i] {
                    return Some(if agree == 1 { (1, "both systems long") } else { (0, "close above 80-day SMA of highs") });
                }
                if agree != 0 && agree as i32 != pos {
                    return Some((agree as i32, if agree > 0 { "20- and 80-day systems both long" } else { "20- and 80-day systems both short" }));
                }
                None
            })
        }
        "lkqts_sob" => sob_proposed_run(x,from,to,p)?,
        "lkqts_macdhr" => {
            let (ef, es) = (ema(c, 12), ema(c, 26));
            let m: Vec<f64> = (0..n).map(|i| ef[i] - es[i]).collect();
            let s = ema(&m, 9);
            let h: Vec<f64> = (0..n).map(|i| m[i] - s[i]).collect();
            let sd = sd_chg(c, u("sd_n"));
            let (rt, arm) = (g("retrace"), g("arm"));
            let mut sig = vec![0i8; n];
            let (mut reg, mut ext, mut armed, mut done) = (0i8, 0.0f64, false, false);
            for i in 1..n {
                if reg < 0 && armed && !done && h[i] > (1.0 - rt) * ext { sig[i] = 1; done = true; }
                if reg > 0 && armed && !done && h[i] < (1.0 - rt) * ext { sig[i] = -1; done = true; }
                if h[i - 1] >= 0.0 && h[i] < 0.0 { reg = -1; ext = h[i]; armed = false; done = false; }
                else if h[i - 1] <= 0.0 && h[i] > 0.0 { reg = 1; ext = h[i]; armed = false; done = false; }
                else if reg < 0 { ext = ext.min(h[i]) } else if reg > 0 { ext = ext.max(h[i]) }
                if reg < 0 && sd[i].is_finite() && h[i] < -arm * sd[i] { armed = true }
                if reg > 0 && sd[i].is_finite() && h[i] > arm * sd[i] { armed = true }
            }
            sar(x, from, to, &sig, "histogram retraced 25% of its trough", "histogram retraced 25% of its peak")
        }

        "lkqts_nei" => {
            let m = u("n");
            let (up, dn) = (u("up").min(m), u("dn").min(m));
            let scale = g("scale") >= 0.5;
            let ma = sma(c, m);
            let sd = sd_chg(c, m);
            let mut ub = vec![f64::NAN; n];
            let mut lb = vec![f64::NAN; n];
            for i in 0..n {
                if i + 1 < 2 * m || !sd[i].is_finite() || sd[i] <= 0.0 { continue }
                let mut v: Vec<f64> = (i + 1 - m..=i).map(|k| (c[k] - ma[k]) / sd[i]).collect();
                v.sort_by(|a, b| b.partial_cmp(a).unwrap());
                let f = if scale { sd[i] } else { 1.0 };
                ub[i] = ma[i] + v[up - 1] * f;
                lb[i] = ma[i] + v[dn - 1] * f;
            }
            let sig: Vec<i8> = (0..n).map(|i| {
                if i == 0 || !ub[i - 1].is_finite() || !ub[i].is_finite() { return 0 }
                if c[i - 1] <= lb[i - 1] && c[i] > lb[i] { 1 } else if c[i - 1] >= ub[i - 1] && c[i] < ub[i] { -1 } else { 0 }
            }).collect();
            sar(x, from, to, &sig, "close back above the lower NEI band", "close back below the upper NEI band")
        }
        "lkqts_adjsto" => {
            let (m, lm) = (u("n"), u("long_n"));
            let k = slow_k(x, m);
            let a: Vec<f64> = (0..n).map(|i| {
                if i + 1 < lm { return f64::NAN }
                let (r1, r2) = (hh(&x.h, i, m) - ll(&x.l, i, m), hh(&x.h, i, lm) - ll(&x.l, i, lm));
                if r2 > 0.0 { (k[i] - 50.0) * r1 / r2 + 50.0 } else { f64::NAN }
            }).collect();
            stoch_sar(&a, from, to, x, g("lo"), g("hi"))
        }
        "lkqts_volrev" => {
            let sd = sd_chg(c, u("sd_n"));
            let vol: Vec<f64> = x.r.iter().map(|r| r.volume.unwrap_or(f64::NAN)).collect();
            let (vp, hold, at_close) = (g("vol_pct"), u("hold"), g("exit_close") >= 0.5);
            let avg = |a: usize, b: usize| (a..=b).map(|k| vol[k]).sum::<f64>() / (b + 1 - a) as f64;
            let mut out = Vec::new();
            let mut i = from.max(10);
            while i < to {
                let ch5 = c[i] - c[i - 5];
                let setup = sd[i].is_finite() && ch5.abs() > sd[i] && avg(i - 4, i) < vp * avg(i - 10, i - 6);
                if setup && ch5 != 0.0 {
                    let e = i + 1;
                    let d = if ch5 < 0.0 { 1.0 } else { -1.0 };
                    let mut t = new_trade(d, e, x.o[e], 1, "open", i);
                    let xi = e + hold - 1;
                    if xi > to {
                        t.exit_i = to; t.exit_px = c[to]; t.signal_i = to; t.reason = "open".into(); t.open = true;
                        out.push(t);
                        break;
                    }
                    t.exit_i = xi;
                    t.exit_px = if at_close { c[xi] } else { x.o[xi] };
                    t.signal_i = xi - 1;
                    t.reason = format!("day {hold} of the trade");
                    out.push(t);
                    i = xi; // the next signal can come on the exit bar's close
                    continue;
                }
                i += 1;
            }
            out
        }
        "lkqts_saitta" => {
            let m = u("n");
            let use_hl = g("hl") >= 0.5;
            let (ah, al) = (sma(&x.h, m), sma(&x.l, m));
            let mut sig = vec![0i8; n];
            let (mut phase, mut start) = (0i8, 0usize);
            let (mut top, mut bot) = (f64::NAN, f64::NAN);
            for i in 0..n {
                if !ah[i].is_finite() { continue }
                if phase <= 0 && c[i] > ah[i] {
                    if phase < 0 { bot = (start..=i).map(|k| if use_hl { x.l[k] } else { c[k] }).fold(f64::INFINITY, f64::min) }
                    phase = 1; start = i;
                } else if phase >= 0 && c[i] < al[i] {
                    if phase > 0 { top = (start..=i).map(|k| if use_hl { x.h[k] } else { c[k] }).fold(f64::NEG_INFINITY, f64::max) }
                    phase = -1; start = i;
                }
                if top.is_finite() && c[i] > top { sig[i] = 1 } else if bot.is_finite() && c[i] < bot { sig[i] = -1 }
            }
            sar(x, from, to, &sig, "close above the last Saitta top", "close below the last Saitta bottom")
        }
        "lkqts_d100" | "lkqts_d1040" => {
            let (st, k): (Vec<f64>, f64) = if id == "lkqts_d100" {
                let m = u("n");
                let (ma, sd) = (sma(c, m), sd_chg(c, m));
                ((0..n).map(|i| (c[i] - ma[i]) / sd[i]).collect(), g("k"))
            } else {
                let (f, s, sd) = (sma(c, u("fast")), sma(c, u("slow")), sd_chg(c, u("sd_n")));
                ((0..n).map(|i| (f[i] - s[i]) / sd[i]).collect(), g("k"))
            };
            run_target(x, from, to, Fill::NextOpen, |i, pos, _| {
                let v = st[i];
                if !v.is_finite() { return None }
                if pos > 0 && v > 0.0 { return Some((if v > k { -1 } else { 0 }, "statistic rose above zero")) }
                if pos < 0 && v < 0.0 { return Some((if v < -k { 1 } else { 0 }, "statistic fell below zero")) }
                if pos <= 0 && v < -k { return Some((1, "statistic below -threshold")) }
                if pos >= 0 && v > k { return Some((-1, "statistic above +threshold")) }
                None
            })
        }
        "lkqts_dom1" | "lkqts_dom2" => {
            let set = |d: u32| (1..=5).contains(&d) || (27..=31).contains(&d) || (id == "lkqts_dom2" && (12..=18).contains(&d));
            let dom = |i: usize| x.r[i].date[8..10].parse::<u32>().unwrap_or(0);
            run_target(x, from, to, Fill::Close, |i, _, _| {
                if i + 1 >= n { return None }
                Some(if set(dom(i + 1)) { (1, "next trading day is in the window") } else { (0, "window over") })
            })
        }
        "lkqts_vixscore" | "lkqts_gross_vix" => {
            let v = x.series("VIX.Close")?;
            if id == "lkqts_vixscore" {
                let m = u("n");
                let ma = sma(v, m);
                let sd = sd_chg(v, m);
                let (hi, lo) = (g("hi"), g("lo"));
                run_target(x, from, to, Fill::Close, |i, pos, _| {
                    let s = (v[i] - ma[i]) / sd[i];
                    if !s.is_finite() { return None }
                    if pos == 0 && s > hi { Some((1, "VIX score > +2")) } else if pos > 0 && s < lo { Some((0, "VIX score < -2")) } else { None }
                })
            } else {
                let (hi, lo) = (g("hi"), g("lo"));
                run_target(x, from, to, Fill::Close, |i, pos, _| {
                    if i == 0 || !v[i].is_finite() || !v[i - 1].is_finite() { return None }
                    if pos == 0 && v[i - 1] <= hi && v[i] > hi { Some((1, "VIX crossed above 30")) }
                    else if pos > 0 && v[i - 1] >= lo && v[i] < lo { Some((0, "VIX dropped below 20")) } else { None }
                })
            }
        }
        s if s.starts_with("lkqts_m_") => {
            let k: usize = if s.ends_with("12") { 12 } else if s.ends_with('6') { 6 } else { 3 };
            let is_ma = s.contains("_sma");
            let month = |i: usize| &x.r[i].date[..7];
            // month-end bars (calendar: the next trading day is in another month)
            let me: Vec<usize> = (0..n.saturating_sub(1)).filter(|&i| month(i) != month(i + 1)).collect();
            let mut want = vec![None::<bool>; n];
            for (j, &i) in me.iter().enumerate() {
                if j < k { continue }
                want[i] = Some(if is_ma {
                    let a = me[j + 1 - k..=j].iter().map(|&q| c[q]).sum::<f64>() / k as f64;
                    c[i] > a
                } else {
                    c[i] > c[me[j - k]]
                });
            }
            run_target(x, from, to, Fill::Close, |i, _, _| want[i].map(|w| if w { (1, "month-end signal on") } else { (0, "month-end signal off") }))
        }
        "lkqts_nvb_entry" | "lkqts_nvb_trail" => {
            let sd = sd_chg(c, 100);
            let (num, x0, x1) = (g("num"), u("x_min"), u("x_max"));
            let sig: Vec<i8> = (0..n).map(|i| {
                if !sd[i].is_finite() || i < x1 { return 0 }
                for xx in (x0..=x1).rev() {
                    let d = c[i] - c[i - xx];
                    let th = num * (xx as f64).sqrt() * sd[i];
                    if d > th { return 1 }
                    if d < -th { return -1 }
                }
                0
            }).collect();
            if id == "lkqts_nvb_entry" {
                sar(x, from, to, &sig, "x-day change > num sqrt(x) sigma", "x-day change < -num sqrt(x) sigma")
            } else {
                let tr = g("trail");
                run_target(x, from, to, Fill::NextOpen, |i, pos, t| {
                    if pos != 0 {
                        let t = t.unwrap();
                        let e = t.legs[0].i;
                        if pos > 0 {
                            let best = (e..=i).map(|k| x.h[k]).fold(f64::NEG_INFINITY, f64::max);
                            if sig[i] == -1 { return Some((-1, "opposite breakout")) }
                            if c[i] < best - tr * sd[i] { return Some((0, "2-sigma trailing stop from the highest high")) }
                        } else {
                            let best = (e..=i).map(|k| x.l[k]).fold(f64::INFINITY, f64::min);
                            if sig[i] == 1 { return Some((1, "opposite breakout")) }
                            if c[i] > best + tr * sd[i] { return Some((0, "2-sigma trailing stop from the lowest low")) }
                        }
                        return None;
                    }
                    match sig[i] { 1 => Some((1, "x-day change > num sqrt(x) sigma")), -1 => Some((-1, "x-day change < -num sqrt(x) sigma")), _ => None }
                })
            }
        }
        _ => return None,
    };
    Some(t)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;
    fn rows(cl: &[f64]) -> Vec<Row> {
        cl.iter().enumerate().map(|(i, &c)| Row { ticker: "T".into(), date: format!("2000-{:02}-{:02}", 1 + i / 28, 1 + i % 28), open: c, high: c + 1.0, low: c - 1.0, close: c, volume: Some(1.0) }).collect()
    }
    #[test]
    fn channel_breakout_enters_on_40_day_high_and_exits_on_20_day_low() {
        // 50 flat-ish bars, a 40-day closing high on bar 50, a plateau, then a slide to a 20-day closing low
        let cl: Vec<f64> = (0..80).map(|i| match i { 0..=49 => 150.0 - i as f64, 50 => 200.0, 51..=60 => 199.0, _ => 99.0 - i as f64 }).collect();
        let r = rows(&cl);
        let x = Ctx::new(&r, &r, &[]);
        let all = run("lkqts_cb4020", &x, 0, r.len() - 1, &Params::new()).unwrap();
        let t = all.iter().find(|t| t.legs[0].i == 51).expect("bought at the open after the bar-50 signal");
        assert_eq!(t.dir, 1.0);
        assert_eq!(t.legs[0].px, cl[51]);
        assert_eq!(t.exit_i, 62); // bar 61 is the first close below every close of the past 20 days
        assert_eq!(t.signal_i, 61);
    }
    #[test]
    fn population_sd_of_changes() {
        let s = sd_chg(&[1.0, 2.0, 4.0, 7.0], 3); // changes 1,2,3 -> pop sd sqrt(2/3)
        assert!((s[3] - (2.0f64 / 3.0).sqrt()).abs() < 1e-12);
    }

    // Independent arithmetic fixtures frozen before observing this candidate.
    // Synthetic observations are not market history or original performance.
    fn next_closes(changes: &[f64]) -> Vec<f64> {
        let mut c = vec![1000.];
        for d in changes { c.push(c.last().unwrap() + d); }
        c
    }
    fn next_rows(c: &[f64]) -> Vec<Row> {
        rows(c).into_iter().map(|mut r| { r.ticker="AAPL".into(); r }).collect()
    }
    #[test]
    fn next_volatility_population_strict_ties_and_opposite_sign() {
        assert!((next_population_variance(&[5.,3.,7.,8.,4.]).unwrap()-3.44).abs()<1e-12);
        let mut d=vec![1.;47];d.extend(vec![-1.;49]);d.extend([0.,0.,0.,2.,3.,0.]);
        let s=next_volbo_signals(&next_closes(&d)).unwrap();
        assert!(s[..=100].iter().all(|v|*v==0));assert_eq!(s[101],1);
        let negative:Vec<_>=d.iter().map(|v|-v).collect();
        let s=next_volbo_signals(&next_closes(&negative)).unwrap();
        assert_eq!(s[100],0);assert_eq!(s[101],-1);
        let r=next_rows(&next_closes(&d));let x=Ctx::new(&r,&r,&[]);
        let t=run("lkqts_volbo",&x,100,102,&Params::new()).unwrap();
        assert_eq!(t.len(),1);assert_eq!(t[0].legs[0].i,102);assert!(t[0].open);
    }
    #[test]
    fn next_volatility_population_not_sample_and_inclusive_not_prior() {
        let mut d=vec![1.;47];d.extend(vec![-1.;49]);d.extend([0.,0.,0.,2.005]);
        let v=next_population_variance(&d).unwrap();
        assert!(2.005f64.powi(2)>4.*v);assert!(2.005f64.powi(2)<4.*v*100./99.);
        assert_eq!(next_volbo_signals(&next_closes(&d)).unwrap()[100],1);
        let mut prior=Vec::new();for _ in 0..50 {prior.extend([1.,-1.]);}
        prior.push(2.001);
        assert!(2.001f64.powi(2)>4.); // prior-only population variance is one
        assert_eq!(next_volbo_signals(&next_closes(&prior)).unwrap()[101],0);
    }
    #[test]
    fn next_volatility_zero_variance_is_not_an_invented_quantity_cap() {
        for (d,want) in [(0.,0),(1.,1),(-1.,-1)] {
            let s=next_volbo_signals(&next_closes(&vec![d;101])).unwrap();
            assert_eq!(s[100],want);
        }
        assert!(next_population_variance(&[f64::MAX,-f64::MAX]).is_none());
    }
    #[test]
    fn next_rsi_declared_seed_strict_levels_and_flat_state() {
        for (u,d,want) in [(7.,13.,0),(13.,7.,0),(1.,3.,1),(3.,1.,-1),(0.,0.,0),(1.,0.,-1),(0.,1.,1)] {
            assert_eq!(next_rsi_side(u,d),want);
        }
        let mut d=vec![35.,-65.];d.extend(vec![0.;12]);d.push(-1.);
        let states=next_rsi_states(&next_closes(&d)).unwrap();
        assert!(states[..14].iter().all(Option::is_none));
        let (u,v)=states[14].unwrap();assert_eq!(next_rsi_side(u,v),0);
        let (u,v)=states[15].unwrap();assert_eq!(next_rsi_side(u,v),1);
        assert!((u-65./28.).abs()<1e-12);assert!((v-859./196.).abs()<1e-12);
        assert!((next_rsi_value(u,v)-22750./657.).abs()<1e-12);
        d.push(0.);let r=next_rows(&next_closes(&d));let x=Ctx::new(&r,&r,&[]);
        let t=run("lkqts_rsi14",&x,15,16,&Params::new()).unwrap();
        assert_eq!(t.len(),1);assert_eq!(t[0].legs[0].i,16);assert_eq!(t[0].dir,1.);
        let mut d=vec![1.,1.,-1.];d.extend(vec![0.;11]);
        let states=next_rsi_states(&next_closes(&d)).unwrap();
        let (u,v)=states[14].unwrap();assert_eq!(next_rsi_side(u,v),-1);
        // Conditional nonzero-day means would be 1/1 =>50; that is not this profile.
        assert_eq!(next_rsi_value(1.,1.),50.);assert_eq!(next_rsi_value(0.,0.),50.);
    }
    #[test]
    fn next_rsi_level_start_reversals_and_terminal_mark_clocks() {
        let mut c:Vec<_>=(0..15).map(|i|100.-i as f64).collect();c.extend([85.,200.,200.,200.]);
        let r=next_rows(&c);let x=Ctx::new(&r,&r,&[]);
        let t=run("lkqts_rsi14",&x,14,18,&Params::new()).unwrap();
        assert_eq!(t.len(),2);assert_eq!(t[0].dir,1.);assert_eq!(t[0].legs[0].i,15);
        assert_eq!(t[0].signal_i,16);assert_eq!(t[0].exit_i,17);
        assert_eq!(t[1].dir,-1.);assert_eq!(t[1].legs[0].i,17);
        assert!(t[1].open);assert_eq!(t[1].signal_i,18);assert_eq!(t[1].exit_i,18);
        assert_eq!(t[0].exit_px,t[1].legs[0].px);
        let t=run("lkqts_rsi14",&x,15,16,&Params::new()).unwrap();
        assert_eq!(t.len(),1);assert_eq!(t[0].legs[0].i,16); // level, no observed cross required
        assert!(run("lkqts_rsi14",&x,14,14,&Params::new()).unwrap().is_empty());
    }
    #[test]
    fn next_macd_named_alpha_profiles_have_frozen_different_orders() {
        let mut c=vec![100.;100];c.extend([113.,92.54,92.54,92.54]);
        let period=next_macd_histogram(&c,false).unwrap();let literal=next_macd_histogram(&c,true).unwrap();
        assert!((literal[100]-39./50.).abs()<1e-12);
        assert!((literal[101]-9./10000.).abs()<1e-12);
        assert!((period[101]+1736./1184625.).abs()<1e-12);
        let mut r=next_rows(&c);r[101].open=115.;r[101].high=116.;r[102].open=90.;r[102].low=89.;
        let x=Ctx::new(&r,&r,&[]);
        let p=run("lkqts_macd",&x,100,102,&Params::new()).unwrap();
        let l=run("lkqts_macd_literal_alpha",&x,100,102,&Params::new()).unwrap();
        assert_eq!(p.len(),2);assert_eq!(l.len(),1);
        assert_eq!(p[0].legs[0].px,115.);assert_eq!(p[0].exit_px,90.);
        assert_eq!(p[1].dir,-1.);assert_eq!(p[1].legs[0].px,90.);assert!(p[1].open);
        assert_eq!(l[0].dir,1.);assert!(l[0].open);
        let selected=run("lkqts_macd",&x,101,102,&Params::new()).unwrap();
        assert_eq!(selected.len(),1);assert_eq!(selected[0].dir,-1.);
        assert_eq!(selected[0].legs[0].i,102); // earlier positive histogram retained; flat selected start
        let marked=run("lkqts_macd",&x,100,103,&Params::new()).unwrap();
        let last=marked.last().unwrap();assert!(last.open);assert_eq!(last.signal_i,103);
        assert_eq!(last.legs[0].i,102); // terminal legacy signal is not entry signal101
        let mirror:Vec<_>=c.iter().map(|c|200.-c).collect();
        let mut r=next_rows(&mirror);r[101].open=85.;r[101].low=84.;r[102].open=110.;r[102].high=111.;
        let x=Ctx::new(&r,&r,&[]);
        let t=run("lkqts_macd",&x,100,102,&Params::new()).unwrap();
        assert_eq!(t.len(),2);assert_eq!(t[0].dir,-1.);assert_eq!(t[1].dir,1.);
        assert_eq!(t[0].exit_i,102);assert_eq!(t[1].legs[0].i,102);
        assert_eq!(t[0].exit_px,110.);assert_eq!(t[1].legs[0].px,110.);
    }
    #[test]
    fn next_macd_crosses_signal_not_zero_and_preserves_full_seed_prefix() {
        assert!(cross_dn(&[1.,-1.],1,0.)); // M3/S2 -> M1/S2, M stays positive
        assert!(cross_up(&[-1.,1.],1,0.)); // M-3/S-2 -> M-1/S-2
        assert!(!cross_up(&[0.,0.],1,0.));assert!(!cross_dn(&[0.,0.],1,0.));
        assert!(cross_up(&[0.,1.],1,0.));assert!(cross_dn(&[0.,-1.],1,0.));
        let mut c=vec![100.;11];c.push(112.);
        assert!((next_ema_alpha(&c,2./13.).unwrap()[11]-1324./13.).abs()<1e-12);
        assert_ne!(next_ema_alpha(&c,2./13.).unwrap()[11],101.); // SMA12 seed differs
        let r=next_rows(&[100.,113.,113.]);let x=Ctx::new(&r,&r,&[]);
        for id in ["lkqts_macd","lkqts_macd_literal_alpha"] {
            assert!(run(id,&x,1,1,&Params::new()).unwrap().is_empty());
            let t=run(id,&x,1,2,&Params::new()).unwrap();assert_eq!(t.len(),1);assert_eq!(t[0].legs[0].i,2);assert!(t[0].open);
            assert!(run(id,&x,2,2,&Params::new()).unwrap().is_empty());
        }
    }
    #[test]
    fn next_candidates_do_not_inherit_admission_or_parameter_rounding() {
        for id in NEXT_CANDIDATE_IDS {
            assert!(!stock_candidate(id));assert!(validate_next_candidate(id,&Params::new()).is_none());
            for p in [Params::from([("unknown".into(),1.)]),Params::from([("n".into(),f64::NAN)]),Params::from([("fast".into(),12.1)])] {
                assert!(validate_next_candidate(id,&p).is_some());
            }
            let d=catalogue().into_iter().find(|d|d.id==*id).unwrap();
            if *id=="lkqts_macd_literal_alpha" {assert!(matches!(d.status,Status::Live));}
            let schema=params(id).unwrap();
            let defaults:Params=schema.iter().map(|s|(s.key.to_string(),s.def)).collect();
            assert!(validate_next_candidate(id,&defaults).is_none());
            for s in schema {
                assert_eq!(s.min,s.def);assert_eq!(s.max,s.def);
                for value in [s.def+0.1,f64::NAN,f64::INFINITY] {
                    assert!(validate_next_candidate(id,&Params::from([(s.key.to_string(),value)])).is_some());
                }
            }
        }
        assert_eq!(family_of("lkqts_macd").unwrap().0,family_of("lkqts_macd_literal_alpha").unwrap().0);
        let c=vec![100.;110];let mut r=next_rows(&c);r[0].close=f64::NAN;
        let x=Ctx::new(&r,&r,&[]);for id in NEXT_CANDIDATE_IDS {assert!(run(id,&x,100,109,&Params::new()).is_none());}
    }
    #[test]
    fn next_macd_flat_noninteger_inputs_have_no_phantom_cross_or_order() {
        for price in [100.1,1001.13] {
            let c=vec![price;120];let r=next_rows(&c);let x=Ctx::new(&r,&r,&[]);
            for (id,literal) in [("lkqts_macd",false),("lkqts_macd_literal_alpha",true)] {
                let h=next_macd_histogram(&c,literal).unwrap();
                assert!(h.iter().all(|v|*v==0.));
                for from in [0,1,50,118] {assert!(run(id,&x,from,119,&Params::new()).unwrap().is_empty());}
            }
        }
    }
    #[test]
    fn next_prefix_admission_rejects_mixed_identity_dates_and_nonpositive_prices() {
        let base=next_rows(&vec![100.;105]);
        for kind in 0..4 {
            let mut r=base.clone();
            match kind {0=>r[0].close=0.,1=>r[0].open=f64::INFINITY,2=>r[1].date=r[0].date.clone(),_=>r[0].ticker="MSFT".into()}
            let x=Ctx::new(&r,&r,&[]);
            assert!(next_history_issue(&x,100,104).is_some());
        }
        let x=Ctx::new(&base,&base,&[]);
        assert!(next_history_issue(&x,100,105).is_some());
        assert!(next_history_issue(&x,104,100).is_some());
    }
}


#[cfg(test)]
mod sob_proposed_tests {
    use super::*;
    use crate::Row;
    // Synthetic observed sessions only, not real market or source trade data.
    fn sob_rows(c: &[f64]) -> Vec<Row> {
        c.iter().enumerate().map(|(i,&close)|Row{ticker:"AAPL".into(),date:format!("2000-{:02}-{:02}",1+i/28,1+i%28),open:close,high:close+1.,low:close*0.9,close,volume:Some(1.)}).collect()
    }
    fn set_open(rows:&mut [Row],i:usize,open:f64) {
        rows[i].open=open;rows[i].high=rows[i].close.max(open)+1.;rows[i].low=rows[i].close.min(open)*0.9;
    }
    #[test]
    fn sob_s01_s03_full_eighty_statistic_and_flat_neutral() {
        let up:Vec<_>=(1..=80).map(|n|n as f64).collect();
        let down:Vec<_>=(101..=180).rev().map(|n|n as f64).collect();
        assert_eq!(sob_close_statistic(&up),Some((80,1,80)));
        assert_eq!(sob_close_statistic(&down),Some((1,80,-80)));
        assert_eq!(sob_close_statistic(&vec![100.;80]),Some((80,80,0)));
        assert!(sob_close_statistic(&up[..79]).is_none());
    }
    #[test]
    fn sob_s04_s08_horizon_boundaries_and_plateau_signs() {
        let mut c=vec![98.;80];c[39]=101.;for v in &mut c[40..79] {*v=99.;}c[79]=100.;
        assert_eq!(sob_close_statistic(&c),Some((40,1,40))); // lag40 blocks41, not40
        let mut c=vec![102.;80];c[44]=99.;for v in &mut c[45..79] {*v=101.;}c[79]=100.;
        assert_eq!(sob_close_statistic(&c),Some((1,35,-35)));
        let mut c=vec![99.;80];c[78]=100.;c[79]=100.;
        assert_eq!(sob_close_statistic(&c),Some((80,2,80)));
        let mut c=vec![101.;80];c[78]=100.;c[79]=100.;
        assert_eq!(sob_close_statistic(&c),Some((2,80,-80)));
        let mut c=vec![99.;81];c[0]=1000.;c[80]=100.;
        assert_eq!(sob_close_statistic(&c[1..]),Some((80,1,80))); // excludeslag80
    }
    #[test]
    fn sob_p09_p12_strict_threshold_outer_ties_and_current_window() {
        // Outer-indicator fixtures only; no claim of a full price reconstruction.
        let mut s=vec![19;40];s[39]=20;assert_eq!(sob_channel_predicate(&s),Some((20,19,0)));
        let mirrored:Vec<_>=s.iter().map(|v|-v).collect();assert_eq!(sob_channel_predicate(&mirrored),Some((-19,-20,0)));
        let mut s=vec![0;40];s[0]=40;s[39]=21;assert_eq!(sob_channel_predicate(&s),Some((40,0,0)));
        s[39]=40;assert_eq!(sob_channel_predicate(&s),Some((40,0,1)));
        let mirrored:Vec<_>=s.iter().map(|v|-v).collect();assert_eq!(sob_channel_predicate(&mirrored),Some((0,-40,-1)));
        let mut s=vec![30;41];s[0]=50;s[40]=40;
        assert_eq!(sob_channel_predicate(&s[1..]),Some((40,30,1))); // lag40excluded
        assert_eq!(sob_channel_predicate(&s[..40]),Some((50,30,0))); // prior40different
        assert!(sob_channel_predicate(&[21;39]).is_none());
    }
    #[test]
    fn sob_c13_first_statistic79_predicate118_actual_open119() {
        let c:Vec<_>=(1..=120).map(|n|n as f64).collect();let mut r=sob_rows(&c);set_open(&mut r,119,200.);let x=Ctx::new(&r,&r,&[]);
        let d=sob_diagnostics(&x,119,&Params::new()).unwrap();assert_eq!(d.profile,SOB_PROPOSED_PROFILE);
        assert!(d.states[..79].iter().all(Option::is_none));let first=d.states[79].as_ref().unwrap();assert_eq!((first.high_horizon,first.low_horizon,first.statistic),(80,1,80));
        assert!(d.states[117].as_ref().unwrap().upper_40.is_none());assert!(!d.states[117].as_ref().unwrap().long_condition);assert!(d.states[118].as_ref().unwrap().long_condition);
        let t=run("lkqts_sob",&x,117,119,&Params::new()).unwrap();assert_eq!(t.len(),1);assert_eq!(t[0].legs.len(),1);assert_eq!((t[0].legs[0].i,t[0].legs[0].px),(119,200.));assert_eq!(t[0].entry_kind,"open");assert_eq!((t[0].exit_i,t[0].exit_px,t[0].open),(119,120.,true));
        assert!(run("lkqts_sob",&x,118,118,&Params::new()).unwrap().is_empty());
    }
    #[test]
    fn sob_c14_full_prefix_flat_selected_start_and_future_stability() {
        let c:Vec<_>=(1..=131).map(|n|n as f64).collect();let mut r=sob_rows(&c);set_open(&mut r,130,500.);
        let x=Ctx::new(&r,&r,&[]);let base=run("lkqts_sob",&x,129,130,&Params::new()).unwrap();assert_eq!(base.len(),1);assert_eq!((base[0].legs[0].i,base[0].legs[0].px),(130,500.));
        let mut suffix=r.clone();let mut bad=suffix[130].clone();bad.date="2000-05-20".into();bad.open=f64::NAN;bad.close=f64::NAN;suffix.push(bad);let x=Ctx::new(&suffix,&suffix,&[]);
        let prior=run("lkqts_sob",&x,129,130,&Params::new()).unwrap();assert_eq!((prior[0].legs[0].i,prior[0].legs[0].px,prior[0].exit_px),(130,500.,131.));assert!(run("lkqts_sob",&x,129,131,&Params::new()).is_none());
        let mut changed=r.clone();changed[130].close=1.;changed[130].low=0.5;let x=Ctx::new(&changed,&changed,&[]);let changed=run("lkqts_sob",&x,129,130,&Params::new()).unwrap();assert_eq!((changed[0].legs[0].i,changed[0].legs[0].px),(130,500.)); // signal129unchanged, terminalmark differs
    }
    #[test]
    fn sob_c15_supplied_predicates_no_pyramid_nextopen_reverse_terminal_mark() {
        // Supplied eligible predicate fixture, not a claimed price-derived SOBseries.
        let mut r=sob_rows(&vec![100.;123]);set_open(&mut r,119,101.);set_open(&mut r,120,102.);set_open(&mut r,121,95.);let x=Ctx::new(&r,&r,&[]);let mut s=vec![0;123];s[118]=1;s[119]=1;s[120]=-1;s[122]=1;
        let t=sar(&x,118,122,&s,"long fixture","short fixture");assert_eq!(t.len(),2);assert_eq!(t[0].legs.len(),1);assert_eq!((t[0].dir,t[0].legs[0].i,t[0].legs[0].px),(1.,119,101.));assert_eq!((t[0].signal_i,t[0].exit_i,t[0].exit_px,t[0].open),(120,121,95.,false));assert_eq!((t[1].dir,t[1].legs[0].i,t[1].legs[0].px),(-1.,121,95.));assert_eq!((t[1].exit_i,t[1].exit_px,t[1].open,t[1].signal_i),(122,100.,true,122));
    }
    #[test]
    fn sob_exact_fixed_parameters_and_causal_quote_identity_guards() {
        assert!(validate_sob_candidate(&Params::new()).is_none());
        let fixed=Params::from([("search".into(),80.),("chan".into(),40.),("min".into(),20.)]);assert!(validate_sob_candidate(&fixed).is_none());
        let r=sob_rows(&vec![100.;120]);
        for (key,value) in [("search",79.),("search",80.5),("chan",41.),("min",20.5),("unknown",0.),("search",f64::NAN),("min",f64::INFINITY)] {
            let p=Params::from([(key.into(),value)]);assert!(validate_sob_candidate(&p).is_some());let x=Ctx::new(&r,&r,&[]);assert!(run("lkqts_sob",&x,118,119,&p).is_none());
        }
        for kind in 0..7 {
            let mut bad=r.clone();match kind {0=>bad[2].open=0.,1=>bad[2].close=f64::NAN,2=>bad[2].high=bad[2].low-1.,3=>bad[2].date=bad[1].date.clone(),4=>bad[2].date="2000-02-31".into(),5=>bad[2].ticker="MSFT".into(),_=>bad[2].ticker=String::new()};let x=Ctx::new(&bad,&bad,&[]);assert!(sob_history_issue(&x,118,119).is_some());assert!(run("lkqts_sob",&x,118,119,&Params::new()).is_none());
        }
        let x=Ctx::new(&r,&r,&[]);assert!(sob_history_issue(&x,119,118).is_some());assert!(sob_history_issue(&x,118,120).is_some());
        assert!(sob_candidate("lkqts_sob"));assert!(stock_research_profile("lkqts_sob"));
        assert_eq!(STOCK_CANDIDATE_IDS.len(),3);assert_eq!(NEXT_CANDIDATE_IDS.len(),4);
        let d=catalogue().into_iter().find(|d|d.id=="lkqts_sob").unwrap();assert!(matches!(d.status,Status::Live));
    }
}
