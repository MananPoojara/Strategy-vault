//! Admission checks for publishing backtests. Coding a rule is not proof that its data or P&L is ready.
use crate::{connors::{Ctx, Params, Status, StrategyDef}, books};

#[derive(Debug)]
pub struct Issue {
    pub status: &'static str,
    pub note: String,
}
fn issue(status: &'static str, note: impl Into<String>) -> Option<Issue> {
    Some(Issue { status, note: note.into() })
}

/// A fill/stop column is intentionally empty when that event did not happen.
/// Its raw opening-range fields are still checked on every bar below.
fn event_column(name: &str) -> bool {
    ["LONG_FILLS", "SHORT_FILLS", "LONG_STOPS", "SHORT_STOPS", "VIXFILL_", "VIXUP_", "VIXDN_"].iter().any(|prefix| name.starts_with(prefix))
}

#[cfg(test)]
mod short_course_readiness_tests {
    use super::*;
    #[test]
    fn momentum_uses_actual_preceding_states_and_does_not_reseed_bad_history() {
        let mut rows:Vec<crate::Row>=(0..100).map(|i|crate::Row {ticker:"RELIANCE".into(),date:format!("2020-{:02}-{:02}",i/28+1,i%28+1),open:100.,high:101.,low:99.,close:100.,volume:None}).collect();
        for &id in books::kfsc::MOMENTUM_STOCK_IDS {
            let def=books::kfsc::catalogue().into_iter().find(|d|d.id==id).unwrap();let ctx=Ctx::new(&rows,&rows,&[]);let required=if id=="kfsc_macd" {1} else {12};
            assert_eq!(check(&ctx,&def,required-1,99,&Params::new()).unwrap().status,"needs_data");assert!(check(&ctx,&def,required,99,&Params::new()).is_none());
            assert_eq!(check(&ctx,&def,20,99,&Params::from([("kind".into(),0.)])).unwrap().status,"not_testable");
        }
        rows[0].close=f64::NAN;let ctx=Ctx::new(&rows,&rows,&[]);let def=books::kfsc::catalogue().into_iter().find(|d|d.id=="kfsc_macd").unwrap();
        assert!(check(&ctx,&def,20,99,&Params::new()).unwrap().note.contains("invalid closing prices"));
    }
    #[test]
    fn short_course_requires_real_warmup_and_exact_source_parameters() {
        let mut rows:Vec<crate::Row>=(0..100).map(|i|crate::Row {ticker:"RELIANCE".into(),date:format!("2020-{:02}-{:02}",i/28+1,i%28+1),open:100.,high:101.,low:99.,close:100.,volume:None}).collect();
        for &id in books::kfsc::REVIEWED_STOCK_IDS {
            let def=books::kfsc::catalogue().into_iter().find(|d|d.id==id).unwrap();let ctx=Ctx::new(&rows,&rows,&[]);
            let warmup=if id=="kfsc_two_ma" {80} else {19};
            assert_eq!(check(&ctx,&def,warmup-1,99,&Params::new()).unwrap().status,"needs_data");
            assert!(check(&ctx,&def,warmup,99,&Params::new()).is_none());
            assert_eq!(check(&ctx,&def,80,99,&Params::from([("source_convention".into(),1.)])).unwrap().status,"not_testable");
        }
        rows[2].close=f64::NAN;let ctx=Ctx::new(&rows,&rows,&[]);
        let def=books::kfsc::catalogue().into_iter().find(|d|d.id=="kfsc_two_ma").unwrap();
        assert!(check(&ctx,&def,80,99,&Params::new()).unwrap().note.contains("warmup contains invalid"));
    }
}

pub fn check(ctx: &Ctx, def: &StrategyDef, from: usize, to: usize, p: &Params) -> Option<Issue> {
    if def.book==books::lkqts::BOOK && books::lkqts::stock_research_profile(def.id) {
        if let Some(note)=books::lkqts::validate_stock_research_profile(def.id,p) {return issue("not_testable",note);}
        if let Some(note)=books::lkqts::stock_research_history_issue(def.id,ctx,from,to) {return issue("needs_data",note);}
    }
    if crate::instrument_policy::reviewed_dema_index(def) {
        if let Some(note)=books::hbqts::reviewed::validate_dema_index(p){return issue("not_testable",note);}
        if from>to || books::hbqts::reviewed::dema_index_states(ctx,to).is_none(){return issue("needs_data","Actual QTS index history is unavailable or contains invalid OHLC, chronology or nonfinite DEMA states. The full supplied prefix is retained; observations are not dropped, filled or reseeded.");}
    }
    if def.book=="hbiab" && books::hbiab_reviewed::reviewed(def.id) {
        if let Some(note)=books::hbiab_reviewed::validate(def.id,p){return issue("not_testable",note);}
        if from>to||to>=ctx.r.len(){return issue("needs_data","Actual Bandy selected window unavailable");}
        if ctx.r[..=to].iter().any(|r|!crate::baskets::valid_date(&r.date)||![r.open,r.high,r.low,r.close].iter().all(|v|v.is_finite()&&*v>0.)||r.low>r.high||r.open<r.low||r.open>r.high||r.close<r.low||r.close>r.high)||ctx.r[..=to].windows(2).any(|w|w[0].date>=w[1].date||w[0].ticker!=w[1].ticker){return issue("needs_data","Full supplied Bandy indicator history contains invalid observations or chronology; no dropping, filling or reseeding is permitted");}
    }
    if def.book=="kfsc" && books::kfsc::reviewed_hv_rule(def.id) {
        if let Some(note)=books::kfsc::validate_hv(def.id,p) {return issue("not_testable",note);}
        if from<65 || to>=ctx.r.len() || from>to {return issue("needs_data","The selected high-volatility application requires 65 genuine true-range observations including the signal day, each with an actual preceding stock close: earliest eligible source index 65 (66 rows). No initial previous close or warmup is invented.");}
        if ctx.r[from-65..=to].iter().any(|r|!crate::baskets::valid_date(&r.date) || ![r.open,r.high,r.low,r.close].iter().all(|v|v.is_finite()&&*v>0.) || r.low>r.high || r.open<r.low || r.open>r.high || r.close<r.low || r.close>r.high) || ctx.r[from-65..=to].windows(2).any(|w|w[0].date>=w[1].date) {
            return issue("needs_data","Actual high-volatility warmup/selected observations contain invalid dates, chronology or OHLC. Required observations are not dropped, repaired or filled to manufacture results.");
        }
    }
    if def.book=="kfsc" && books::kfsc::reviewed_hv_rule(def.id) && !books::kfsc::hv_exact_supported(ctx,from,to) {
        return issue("needs_data","High-volatility exact decimal threshold comparisons exceed supported observed-price precision or arithmetic capacity. No epsilon, rounded threshold or fabricated signal is substituted.");
    }
    if def.book=="kfsc" && books::kfsc::reviewed_momentum_rule(def.id) {
        if let Some(note)=books::kfsc::validate_momentum(def.id,p) {return issue("not_testable",note);}
        let required=if def.id=="kfsc_macd" {1} else {12};
        if from<required {return issue("needs_data","This selected Short Course momentum application requires genuine preceding source observations for a crossing/turn: MACD needs a previous seeded state; DSM needs10-day momentum and two observable slope comparisons. Select a later test start; no prices or indicator history are invented.");}
        if ctx.r.iter().take(from).any(|r|!r.close.is_finite() || r.close<=0.) {
            return issue("needs_data","Actual Short Course momentum warmup contains invalid closing prices. The full source history is retained; invalid earlier values are not dropped, filled or reseeded to manufacture a result.");
        }
    }
    if def.book=="kfsc" && books::kfsc::reviewed_stock_rule(def.id) {
        if let Some(note)=books::kfsc::validate_reviewed(def.id,p) {return issue("not_testable",note);}
        let warmup=if def.id=="kfsc_two_ma" {80} else {19};
        if from<warmup {return issue("needs_data","This selected Short Course configuration requires actual earlier full-period observations (including both current and previous SMA80 for two_ma). Select a later test start; no indicator history is invented.");}
        if ctx.r.iter().take(from).any(|r|!r.close.is_finite() || r.close<=0.) {
            return issue("needs_data","Actual Short Course closing-price warmup contains invalid observations. Earlier rows are not filled, discarded or reseeded to manufacture a result.");
        }
    }
    if def.book==books::bllstr::BOOK && books::bllstr::reviewed_daily_index(def.id) {
        // An admitted zero-band ID must not expose a held band/holding/direction
        // variant through query parameters. Keep the reviewed contract explicit.
        let valid=|(key,value):(&String,&f64)| value.is_finite() && match key.as_str() {
            "short_ma" => *value==1.,
            "long_ma" => *value==50.,
            "band_pct" => *value==0.,
            "sell_short" => *value==1.,
            "hold" if def.id=="bllstr_fma_1_50_0" => *value==10.,
            _ => false,
        };
        if !p.iter().all(valid) {
            return issue("not_testable","This reviewed Brock admission covers only short MA 1, long MA 50, band 0%, sell_short 1 and, for FMA, a 10-observation hold. Changed or unknown parameters await separate source and ledger review; they cannot inherit default verification.");
        }
    }
    if def.book=="sbma" && books::sbma::next_reviewed_stock_rule(def.id) {
        if let Some(note)=books::sbma::validate_next_reviewed(def.id,p) {return issue("not_testable",note);}
    }
    if def.book=="sbma" && books::sbma::four_reviewed_stock_rule(def.id) {
        if let Some(note)=books::sbma::validate_four_reviewed(def.id,p) {return issue("not_testable",note);}
        let slow=if def.id=="sbma_x_15e30e" {30} else if def.id=="sbma_x_15e150e" {150} else {200};
        if from<slow {return issue("needs_data","This reviewed crossover requires actual earlier observations for both current and previous moving averages. Choose a later start with real warmup; no indicator history is invented.");}
        if ctx.r.iter().take(from).any(|r| ![r.open,r.high,r.low,r.close].iter().all(|v|v.is_finite()&&*v>0.) || r.low>r.high || r.open<r.low || r.open>r.high || r.close<r.low || r.close>r.high) {
            return issue("needs_data","Full supplied moving-average warmup contains invalid OHLC. Earlier rows are not discarded, repaired or reseeded to manufacture a result.");
        }
    }
    if def.book=="elcim" && def.id==books::elcim_account::ID {
        if let Some(note)=books::elcim_account::validate(p){return issue("not_testable",note);}
    }
    if matches!(def.id,"elcim_ts_fi"|"elcim_fi2"|"eltfl_sroc") {
        return issue("source_review_required","The supplied Elder sources require protective stops and account risk limits. This legacy fragment omits part of that contract. Triple Screen additionally has stop-entry/protective-stop ordering ambiguities on daily bars; FI2 lacks a printed entry order; SROC lacks objective initial protection. These fragments remain held, without invented exits or risk-free source labels.");
    }
    if def.book=="sbma" && matches!(def.id,"sbma_sma50_long"|"sbma_x_5s20s") {
        let fill=p.get("fill").copied().unwrap_or(0.);
        if ![0.,1.].contains(&fill) {return issue("not_testable","Reviewed Burns fill must be 0 (idealized same-close research) or 1 (next actual open).");}
        let changed=if def.id=="sbma_sma50_long" {p.get("ma").is_some_and(|v|*v!=50.)} else {p.get("fast").is_some_and(|v|*v!=5.) || p.get("slow").is_some_and(|v|*v!=20.) || p.get("always_in").is_some_and(|v|*v!=0.)};
        if changed {return issue("not_testable","This bounded Burns admission covers only the independently checked default 50-SMA long or 5-SMA/20-SMA long/cash baseline. Other lengths/directions are separate research variants awaiting their own review; no changed result inherits default verification.");}
    }
    if def.book=="spmt" {
        if let Some(note)=books::spmt::hold_reason(def.id) {return issue("needs_data",note);}
    }
    if def.book==books::vixr::BOOK && books::vixr::reviewed_daily(def.id) && ctx.r.get(from).is_some_and(|r|r.date.as_str()<books::vixr::REAL_VIX_OHLC_FROM) {
        return issue("needs_data",format!("The CVR rules read the VIX open, high and low; the served CBOE VIX history has close-only bars before {}. Choose a later start; close-only bars are not used as highs, lows or opens.",books::vixr::REAL_VIX_OHLC_FROM));
    }
    if def.book==books::ptjmt::BOOK && books::ptjmt::reviewed_daily(def.id) {
        if let Some(r)=ctx.r.get(from) {
            if def.id.starts_with("ptjmt_cvr") && r.date.as_str()<books::ptjmt::REAL_VIX_OHLC_FROM {
                return issue("needs_data",format!("The CVR rules read the VIX high and low (CVR I and V also the open); the served CBOE VIX history has close-only bars before {}. Choose a later start.",books::ptjmt::REAL_VIX_OHLC_FROM));
            }
            if let Some(cut)=books::ptjmt::reads_open(def.id).then(||crate::gate::real_open_from(&r.ticker)).flatten().filter(|c|r.date.as_str()<*c) {
                return issue("needs_data",format!("The 1.5% stop fills at the open when a bar gaps through it; {} opens are real only from {cut}. Choose a start on or after {cut}.",r.ticker));
            }
        }
    }
    // Reviewed Street Smarts rules fill stops at a gapped open; where the gate does not already move the start to
    // the first real US index open, refuse earlier windows instead of using stale opens.
    if def.book==books::ss::BOOK && books::ss::reads_open(def.id) && !crate::gate::get(def.id).is_some_and(|g|g.clamp_open) {
        if let Some((row,cutoff))=ctx.r.get(from).and_then(|r|crate::gate::real_open_from(&r.ticker).map(|c|(r,c))) {
            if row.date.as_str()<cutoff {return issue("needs_data",format!("This rule fills stop orders at the open when the market gaps through them. {} opens in this data are real only from {cutoff}; choose that start or later. Earlier stale opens are not used.",row.ticker));}
        }
    }
    // Admitted btp rules that read the open but are not clamped by the gate: refuse windows with stale index opens.
    if def.book==books::btp::BOOK && books::btp::reads_open(def.id) && !crate::gate::get(def.id).is_some_and(|g|g.clamp_open) {
        if let Some((r,cut))=ctx.r.get(from).and_then(|r|crate::gate::real_open_from(&r.ticker).map(|c|(r,c))).filter(|(r,c)|r.date.as_str()<*c) {
            return issue("needs_data",format!("This rule's stop entries and gap tests read the opening price; {} opens in this file are real only from {cut}. Choose a start on or after {cut}.",r.ticker));
        }
    }
    // Admitted cwin rules all read the open; the gate clamps all but the two-rule exit test, which is refused instead.
    if def.book==books::cwin::BOOK && books::cwin::reads_open(def.id) && !crate::gate::get(def.id).is_some_and(|g|g.clamp_open) {
        if let Some((r,cut))=ctx.r.get(from).and_then(|r|crate::gate::real_open_from(&r.ticker).map(|c|(r,c))).filter(|(r,c)|r.date.as_str()<*c) {
            return issue("needs_data",format!("This rule's stop entries read the opening price; {} opens in this file are real only from {cut}. Choose a start on or after {cut}.",r.ticker));
        }
    }
    // Admitted ws5 rules all read the open (stop entries, gap negation, gapped stops): refuse stale US index opens
    // where the gate does not already move the start; VIX III also needs real CBOE VIX highs and lows.
    if def.book==books::ws5::BOOK && books::ws5::reviewed_daily(def.id) {
        if let Some(r)=ctx.r.get(from) {
            if def.id=="ws5_vix_iii_reversal" && r.date.as_str()<books::ws5::REAL_VIX_OHLC_FROM {
                return issue("needs_data",format!("VIX III reads the VIX high and low; the served CBOE VIX history has close-only bars before {}. Choose a later start.",books::ws5::REAL_VIX_OHLC_FROM));
            }
            if let Some(cut)=crate::gate::real_open_from(&r.ticker).filter(|c|r.date.as_str()<*c && !crate::gate::get(def.id).is_some_and(|g|g.clamp_open)) {
                return issue("needs_data",format!("This rule's stop entries, gap tests and gapped stops read the opening price; {} opens in this file are real only from {cut}. Choose a start on or after {cut}.",r.ticker));
            }
        }
    }
    // Admitted ats rules: CVR I/III read the VIX open/high/low (close-only CBOE VIX before 1992); rules whose fills read the
    // traded instrument's open refuse windows with stale US index opens unless the gate already clamps the start.
    if def.book==books::ats::BOOK && books::ats::reviewed_daily(def.id) {
        if let Some(r)=ctx.r.get(from) {
            if matches!(def.id,"ats_cvr1"|"ats_cvr3"|"ats_cvr_equities") && r.date.as_str()<books::ats::REAL_VIX_OHLC_FROM {
                return issue("needs_data",format!("CVR I and III read the VIX open, high and low; the served CBOE VIX history has close-only bars before {}. Choose a later start.",books::ats::REAL_VIX_OHLC_FROM));
            }
            if books::ats::reads_open(def.id,p) && !crate::gate::get(def.id).is_some_and(|g|g.clamp_open) {
                if let Some(cut)=crate::gate::real_open_from(&r.ticker).filter(|c|r.date.as_str()<*c) {
                    return issue("needs_data",format!("This rule's stop or trailing-stop fills read the opening price; {} opens in this file are real only from {cut}. Choose a start on or after {cut}.",r.ticker));
                }
            }
        }
    }
    if def.book=="chbta2" && matches!(def.id,"ch_bta2_ma_stack"|"ch_bta2_65sma_ravi") {
        if let Some(note)=books::chbta2::validate_reviewed(def.id,p) {return issue("not_testable",note);}
    }
    if (def.book=="ehrs" && books::ehrs::reviewed_sinetrend(def.id)) || (def.book=="ehmd" && def.id=="ehmd_mama") {
        let invalid=if def.book=="ehrs" {books::ehrs::validate(def.id,p)} else {books::ehmd::validate(def.id,p)};
        if let Some(note)=invalid {return issue("not_testable",note);}
        let schema=if def.book=="ehrs" {books::ehrs::params(def.id)} else {books::ehmd::params(def.id)};
        let default_mbb=schema.as_ref().and_then(|s|s.iter().find(|d|d.key=="max_bars_back")).map_or(50.,|d|d.def);
        let mbb=p.get("max_bars_back").copied().unwrap_or(default_mbb) as usize;
        if from<mbb+5 {return issue("needs_data","The selected Ehlers start lacks actual MaxBarsBack plus CurrentBar>5 warmup. Earlier observed bars must remain loaded; no indicator history is invented.");}
        if ctx.r.iter().take(from).any(|r| ![r.open,r.high,r.low,r.close].iter().all(|v|v.is_finite()&&*v>0.) || r.low>r.high || r.open<r.low || r.open>r.high || r.close<r.low || r.close>r.high) {return issue("needs_data","Ehlers recursive indicator warmup contains invalid OHLC. No earlier prices are repaired or filled.");}
    }
    if def.id=="atm00_fourplay" {return issue("calculation_unavailable","Fourplay's source uses fixed100-share legs in a30-stock portfolio and stated starting equity. The current runner substitutes full unit exposure and pyramids above that supported allocation. Stock routing alone cannot establish source sizing, margin or financing; results are withheld until portfolio accounting is validated.");}
    if def.id=="tdperl_rebo" {return issue("needs_data","REBO requires source-supported stop/reversal sequencing. Daily OHLC cannot order both touched stop levels, and the current runner's one-reversal-per-day/skip-both-hit choices are not established by the reviewed source. Intraday execution evidence is required; no modeled path is published as fact.");}
    if def.id=="ss_turtle_soup" {return issue("needs_data","Turtle Soup's recovery entry must follow the new low/high break. The current daily-bar runner uses final extrema to qualify entry and set its stop without establishing that sequence. Intraday or source-supported causal execution data is required.");}
    if def.id.starts_with("ss_study_") && p.get("overlap").is_some_and(|v|*v>0.) {return issue("calculation_unavailable","Overlapping study observations are not a sequential single-position account. Portfolio sizing and overlapping mark-to-market accounting are unsupported; equity, risk statistics and optimization are withheld in overlap mode.");}
    if def.id.starts_with("r3r4_lowprice") && !matches!(ctx.r.first().map(|r|r.ticker.as_str()),Some("SPY"|"QQQ"|"MDY"|"IWM"|"SMH")) {return issue("not_testable","The source's literalUSD1–15 stock price band is not an INR1–15 rule. Actual source-currency stocks or an explicitly validated local-price specification are required; no silent currency substitution is published.");}
    if def.book==books::chande::BOOK {
        if let Some(note)=books::chande::validate(def.id,p) {return issue("not_testable",note);}
    }
    // These runners still price options with Black-Scholes. Loading IV or changing
    // a catalogue badge must never enable their modeled performance in the app.
    if matches!(def.id,"otvt_diz_short_strangle" | "otvt_cvr1_sell_puts" | "otvt_cvr1_buy_puts" | "otvt_sector_options_overpriced") {
        return issue("needs_data","This options runner uses a pricing model, not traded contract prices, and is disabled. Real prices of the exact contracts (strike, expiry and each leg) and a validated option-price implementation are required. Loading an IV or VIX series cannot enable this backtest.");
    }
    match def.status {
        Status::NeedsData(why) => return issue("needs_data", format!("Required data unavailable: {why}. No backtest results are published.")),
        Status::NotTestable(why) => return issue("not_testable", format!("Backtest unavailable: {why}. No rules or prices are invented to fill this gap.")),
        Status::Live => {}
    }
    if let Some(g) = crate::gate::get(def.id) {
        if g.verdict == "withheld" && !crate::instrument_policy::reviewed_cash_rule(def) {
            return issue("not_testable", format!("Withheld by the accuracy check: {} No results are published.", g.reason));
        }
        if let (Some(allowed), Some(row)) = (g.allowed, ctx.r.get(from)) {
            if !allowed.contains(&row.ticker.as_str()) {
                if def.book=="radge" && def.id=="radge_oc_long" {
                    return issue("not_testable",format!("This independently checked plain open-to-close research application is limited to NIFTY 50 and policy-qualified SPX prices. {} is outside that reviewed symbol scope; no result is published.",row.ticker));
                }
                return issue("not_testable", format!("Not published on {}: this rule uses fixed point or dollar amounts set for {}; on this instrument they would not mean what the book meant. {}", row.ticker, if allowed.is_empty() { "an instrument this vault does not hold".to_string() } else { allowed.join(", ") }, g.reason));
            }
        }
    }
    if from >= ctx.r.len() || to >= ctx.r.len() || from > to || (from==to && !((def.book=="hbiab" && books::hbiab_reviewed::reviewed(def.id)) || crate::instrument_policy::reviewed_dema_index(def))) {
        return issue("needs_data", "No usable trading range is covered by the required data. Choose another period.");
    }
    if ctx.r[from..=to].iter().any(|r| ![r.open,r.high,r.low,r.close].iter().all(|n| n.is_finite() && *n > 0.0) || r.low > r.high || r.open < r.low || r.open > r.high || r.close < r.low || r.close > r.high) {
        return issue("needs_data", "The selected period contains invalid or incomplete OHLC market prices. Results are withheld until the source data is corrected.");
    }
    if def.needs_volume && ctx.r[from..=to].iter().any(|r| !r.volume.is_some_and(|v| v.is_finite() && v > 0.0)) {
        return issue("needs_data", "Required volume data is missing on one or more trading days in this period. A price-only index cannot substitute for this rule's volume or stock-universe filter.");
    }
    let mut required=def.needs_series.to_vec();
    if p.get("src").is_some_and(|v| v.round()==1.0) && (def.id.starts_with("otvt_cvr1") || def.id.starts_with("ptjmt_cvr")) {
        required.extend(["VXO_OHLC.Open","VXO_OHLC.High","VXO_OHLC.Low","VXO_OHLC.Close"]);
    }
    let missing=ctx.missing(&required);
    if !missing.is_empty() {
        return issue("needs_data",format!("Required market data is missing: {}. No chart, statistics, trades or optimization are available for this instrument.",missing.join(", ")));
    }
    // A series existing somewhere in history does not mean it covers the requested dates.
    for &name in &required {
        if event_column(name) { continue; }
        let values=ctx.series(name).unwrap();
        let gaps:Vec<_>=(from..=to).filter(|&i| !values.get(i).is_some_and(|v| v.is_finite())).collect();
        if let Some(&first)=gaps.first() {
            let dates=values.iter().enumerate().filter(|(_,v)| v.is_finite()).map(|(i,_)| ctx.r[i].date.as_str()).collect::<Vec<_>>();
            return issue("needs_data",format!("Required series {name} is missing on {} trading day(s) in this period, first on {}. Loaded coverage: {} to {}. Choose a fully covered period; no missing prices are estimated.",gaps.len(),ctx.r[first].date,dates.first().unwrap_or(&"unknown"),dates.last().unwrap_or(&"unknown")));
        }
    }
    if def.needs_vix {
        let Some(start)=ctx.vix.first else { return issue("needs_data","VIX data required: this strategy needs the daily volatility index (VIX) for this market, and that history is not loaded yet, so no backtest is shown. US markets: CBOE VIX data is pending and will be added. India: India VIX is used with NIFTY 50.") };
        if from < start { return issue("needs_data",format!("Required volatility-index data begins on {}. The selected period includes earlier days with no data; choose a fully covered period.",ctx.r[start].date)); }
        if start > to { return issue("needs_data","The selected period ends before the required volatility-index data begins. Choose another period."); }
        if let Some(i)=(from.max(start)..=to).find(|&i| !ctx.vix.has_bar(i)) {
            return issue("needs_data",format!("Required volatility-index data is missing on {}. Results are withheld for this period.",ctx.r[i].date));
        }
    }
    if def.book=="oiss" {
        return issue("calculation_unavailable","Option quote series are loaded for this instrument, but this engine marks option positions using the underlying index and does not correctly aggregate overlapping option positions. Option portfolio equity, drawdown and risk statistics are not supported. The complete backtest, charts, trades, rankings and optimization are withheld until held-contract mark-to-market accounting is implemented and validated. This is not a reproduction of the book's reported performance.");
    }
    if let Some(note)=books::otvt::gap(def.id,ctx,from,to,p) {
        return issue("needs_data",note);
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Row;
    #[test]
    fn four_burns_requires_real_valid_prior_history_without_shortening_it() {
        let mut bars:Vec<Row>=(0..450).map(|i|Row{ticker:"AAPL".into(),date:format!("{i:04}"),open:100.,high:101.,low:99.,close:100.,volume:None}).collect();
        for &id in books::sbma::FOUR_REVIEWED_STOCK_IDS {
            let def=books::sbma::catalogue().into_iter().find(|d|d.id==id).unwrap();
            let ctx=Ctx::new(&bars,&bars,&[]);
            assert!(check(&ctx,&def,250,449,&Params::new()).is_none());
            assert_eq!(check(&ctx,&def,2,449,&Params::new()).unwrap().status,"needs_data");
        }
        bars[1].close=f64::NAN;
        let ctx=Ctx::new(&bars,&bars,&[]);
        for &id in books::sbma::FOUR_REVIEWED_STOCK_IDS {
            let def=books::sbma::catalogue().into_iter().find(|d|d.id==id).unwrap();
            assert!(check(&ctx,&def,250,449,&Params::new()).unwrap().note.contains("Full supplied"));
        }
    }
    fn rows() -> Vec<Row> {
        (1..=5).map(|i| Row{ticker:"TEST".into(),date:format!("2020-01-0{i}"),open:100.0,high:101.0,low:99.0,close:100.0,volume:Some(1000.0)}).collect()
    }
    fn definition(id: &str) -> StrategyDef { crate::connors::catalogue().into_iter().find(|d| d.id==id).unwrap() }
    #[test]
    fn reviewed_burns_only_accepts_default_long_cash_lengths_and_exact_fill_modes() {
        let r=rows();let ctx=Ctx::new(&r,&r,&[]);
        for id in ["sbma_sma50_long","sbma_x_5s20s"] {
            let d=definition(id);
            for fill in [0.,1.] {assert!(check(&ctx,&d,0,4,&Params::from([("fill".into(),fill)])).is_none());}
            for fill in [-1.,0.5,2.,f64::NAN,f64::INFINITY] {
                assert_eq!(check(&ctx,&d,0,4,&Params::from([("fill".into(),fill)])).unwrap().status,"not_testable");
            }
        }
        for (id,key,value) in [("sbma_sma50_long","ma",49.),("sbma_sma50_long","ma",f64::NAN),
            ("sbma_x_5s20s","fast",6.),("sbma_x_5s20s","slow",21.),("sbma_x_5s20s","always_in",1.)] {
            let d=definition(id);let p=Params::from([(key.into(),value)]);
            assert_eq!(check(&ctx,&d,0,4,&p).unwrap().status,"not_testable");
        }
    }
    #[test]
    fn reviewed_brock_accepts_only_finite_exact_source_defaults() {
        let r=rows();let ctx=Ctx::new(&r,&r,&[]);
        for id in ["bllstr_vma_1_50_0","bllstr_fma_1_50_0"] {
            let d=definition(id);assert!(check(&ctx,&d,0,4,&Params::new()).is_none());
            let defaults=crate::connors::params(id).into_iter().map(|p|(p.key.to_string(),p.def)).collect();
            assert!(check(&ctx,&d,0,4,&defaults).is_none());
        }
    }
    #[test]
    fn reviewed_brock_parameters_cannot_reach_held_source_variants() {
        let r=rows();let ctx=Ctx::new(&r,&r,&[]);
        for id in ["bllstr_vma_1_50_0","bllstr_fma_1_50_0"] {
            let d=definition(id);
            for (key,value) in [("short_ma",2.),("short_ma",1.1),("long_ma",150.),("band_pct",1.),
                ("band_pct",-0.01),("sell_short",0.),("sell_short",0.5),("unknown",1.),("fill",1.)] {
                let e=check(&ctx,&d,0,4,&Params::from([(key.into(),value)])).expect("changed or unknown parameter must not inherit default source admission");
                assert_eq!(e.status,"not_testable");assert!(e.note.contains("reviewed"));
            }
            for key in ["short_ma","long_ma","band_pct","sell_short"] {
                for value in [f64::NAN,f64::INFINITY,f64::NEG_INFINITY] {
                    assert_eq!(check(&ctx,&d,0,4,&Params::from([(key.into(),value)])).unwrap().status,"not_testable");
                }
            }
        }
        let d=definition("bllstr_fma_1_50_0");
        for hold in [9.,10.1,11.,f64::NAN,f64::INFINITY] {
            assert_eq!(check(&ctx,&d,0,4,&Params::from([("hold".into(),hold)])).unwrap().status,"not_testable");
        }
    }
    #[test]
    fn spmt_partial_auxiliary_basket_and_inferred_options_exit_stay_held() {
        let r=rows();let mut ctx=Ctx::new(&r,&r,&r);
        for name in ["TRIN","ADV","TICK"] {ctx.extra.insert(name.into(),vec![1.0;r.len()]);}
        for id in ["spmt_multi5","spmt_multi9","spmt_mts","spmt_vix200","spmt_trin_1","spmt_nyse_adv","spmt_tick","spmt_eom_und","spmt_eom_puts","spmt_dt_gapdown"] {
            assert_eq!(check(&ctx,&definition(id),0,4,&Params::new()).unwrap().status,"needs_data");
        }
        // Printed performance mismatch alone is not an executable-rule gap.
        for id in ["spmt_rsi2_4","spmt_rsi4_25","spmt_rsi_1"] {
            assert!(check(&ctx,&definition(id),0,4,&Params::new()).is_none());
        }
    }
    #[test]
    fn ss_open_reading_rules_refuse_stale_us_index_opens_unless_the_gate_clamps() {
        let at=|t:&str,y:&str|{let mut r=rows();for (i,x) in r.iter_mut().enumerate(){x.ticker=t.into();x.date=format!("{y}-01-0{}",i+2);}r};
        for (t,y,id,refused) in [("SPX","2013","ss_holy_grail",true),("SPX","2014","ss_holy_grail",false),("DJI","2015","ss_study_channel_2period_sar",true),
            ("NDX","2000","ss_id_nr4",true),("SPX","2013","ss_2_period_roc",false),("SPX","2013","ss_turtle_soup_plus_one",false),("NIFTY 50","2005","ss_holy_grail",false),("SPY","1994","ss_holy_grail",false)] {
            let r=at(t,y);let ctx=Ctx::new(&r,&r,&r);
            assert_eq!(check(&ctx,&definition(id),0,4,&Params::new()).is_some(),refused,"{id} {t} {y}");
        }
    }
    #[test]
    fn vixr_admitted_rules_refuse_close_only_vix_years() {
        let mut r=rows();for x in r.iter_mut(){x.ticker="SPX".into();}let ctx=Ctx::new(&r,&r,&r);
        let mut old=r.clone();for (i,x) in old.iter_mut().enumerate(){x.date=format!("1991-12-2{}",i+5);}
        let octx=Ctx::new(&old,&old,&old);
        for &id in books::vixr::REVIEWED_DAILY {
            assert!(check(&ctx,&definition(id),0,4,&Params::new()).is_none(),"{id}");
            assert_eq!(check(&octx,&definition(id),0,4,&Params::new()).unwrap().status,"needs_data","{id}");
        }
    }
    #[test]
    fn ptjmt_admitted_rules_refuse_close_only_vix_years_and_stale_spx_opens() {
        let mk=|t:&str,y:&str|{let mut r=rows();for (i,x) in r.iter_mut().enumerate(){x.ticker=t.into();x.date=format!("{y}-01-0{}",i+5);}r};
        let ok=|r:&Vec<crate::Row>,id:&str|{let mut ctx=Ctx::new(r,r,r);for s in definition(id).needs_series {ctx.extra.insert(s.to_string(),vec![1.0;r.len()]);}check(&ctx,&definition(id),0,4,&Params::new()).map(|i|i.status)};
        for &id in books::ptjmt::REVIEWED_DAILY {
            let cvr=id.starts_with("ptjmt_cvr");
            assert_eq!(ok(&mk("SPY","1991"),id),cvr.then_some("needs_data"),"{id}");
            assert_eq!(ok(&mk("SPX","2013"),id),books::ptjmt::reads_open(id).then_some("needs_data"),"{id}");
            assert_eq!(ok(&mk("SPX","2014"),id),None,"{id}");
            assert_eq!(ok(&mk("SPY","2013"),id),None,"{id}");
        }
    }
    #[test]
    fn cwin_unclamped_rules_refuse_stale_index_opens() {
        let mk=|t:&str,y:&str|{let mut r=rows();for (i,x) in r.iter_mut().enumerate(){x.ticker=t.into();x.date=format!("{y}-06-0{}",i+1);}r};
        for &id in books::cwin::REVIEWED_DAILY {
            let d=definition(id);
            let guarded=!crate::gate::get(id).is_some_and(|g|g.clamp_open);
            assert_eq!(guarded,id=="cwin_window_two_rule_exit_test");
            let (old,new,spy)=(mk("SPX","2013"),mk("SPX","2014"),mk("SPY","1995"));
            assert_eq!(check(&Ctx::new(&old,&old,&old),&d,0,4,&Params::new()).is_some(),guarded,"{id}");
            assert!(check(&Ctx::new(&new,&new,&new),&d,0,4,&Params::new()).is_none(),"{id}");
            assert!(check(&Ctx::new(&spy,&spy,&spy),&d,0,4,&Params::new()).is_none(),"{id}");
        }
    }
    #[test]
    fn btp_open_reading_rules_refuse_stale_index_opens() {
        let mk=|t:&str,y:&str|{let mut r=rows();for (i,x) in r.iter_mut().enumerate(){x.ticker=t.into();x.date=format!("{y}-06-0{}",i+1);}r};
        for &id in books::btp::REVIEWED_DAILY {
            let d=definition(id);
            let clamped=crate::gate::get(id).is_some_and(|g|g.clamp_open);
            let guarded=books::btp::reads_open(id) && !clamped;
            let (old,new,spy)=(mk("SPX","2013"),mk("SPX","2014"),mk("SPY","1995"));
            assert_eq!(check(&Ctx::new(&old,&old,&old),&d,0,4,&Params::new()).is_some(),guarded,"{id}");
            assert!(check(&Ctx::new(&new,&new,&new),&d,0,4,&Params::new()).is_none(),"{id}");
            assert!(check(&Ctx::new(&spy,&spy,&spy),&d,0,4,&Params::new()).is_none(),"{id}");
        }
    }
    #[test]
    fn ws5_rules_refuse_stale_index_opens_and_close_only_vix() {
        let mk=|t:&str,y:&str|{let mut r=rows();for (i,x) in r.iter_mut().enumerate(){x.ticker=t.into();x.date=format!("{y}-06-0{}",i+1);}r};
        for &id in books::ws5::REVIEWED_DAILY {
            let d=definition(id);
            let guarded=!crate::gate::get(id).is_some_and(|g|g.clamp_open);
            let (old,new,spy)=(mk("SPX","2013"),mk("SPX","2014"),mk("SPY","1991"));
            let note=|r:&Vec<Row>|check(&Ctx::new(r,r,r),&d,0,4,&Params::new()).map(|i|i.note);
            assert_eq!(note(&old).is_some_and(|n|n.contains("opens in this file are real only from 2014")),guarded,"{id}");
            assert!(!note(&new).is_some_and(|n|n.contains("opening price")||n.contains("close-only")),"{id}");
            assert_eq!(note(&spy).is_some_and(|n|n.contains("close-only")),id=="ws5_vix_iii_reversal","{id}");
            if id!="ws5_vix_iii_reversal" {assert!(note(&new).is_none(),"{id}");}
        }
    }
    #[test]
    fn ats_admitted_rules_refuse_close_only_vix_and_stale_index_opens() {
        let mk=|t:&str,y:&str|{let mut r=rows();for (i,x) in r.iter_mut().enumerate(){x.ticker=t.into();x.date=format!("{y}-06-0{}",i+1);}r};
        let refused=|r:&Vec<Row>,id:&str,p:&Params|check(&Ctx::new(r,r,r),&definition(id),0,4,p).is_some_and(|i|i.note.contains("close-only")||i.note.contains("opening price"));
        let none=Params::new();let atr=Params::from([("exit".into(),1.0)]);
        for (t,y,id,p,want) in [("SPX","1991","ats_cvr1",&none,true),("SPX","1991","ats_cvr2",&none,false),("SPY","1991","ats_cvr_equities",&none,true),
            ("SPX","2013","ats_trin_thrust",&none,true),("SPX","2014","ats_trin_thrust",&none,false),("SPY","1995","ats_padi",&none,false),
            ("NDX","2000","ats_hv_thrust",&none,false),("NDX","2000","ats_hv_thrust",&atr,true),("NDX","2001","ats_hv_thrust",&atr,false),("NIFTY 50","2005","ats_hv_thrust",&atr,false)] {
            assert_eq!(refused(&mk(t,y),id,p),want,"{id} {t} {y}");
        }
    }
    #[test]
    fn real_underlying_prices_can_be_backtested() {
        let r=rows();let ctx=Ctx::new(&r,&r,&[]);
        assert!(check(&ctx,&definition("f1a"),0,4,&Params::new()).is_none());
    }
    #[test]
    fn modeled_options_are_blocked_even_with_real_underlying_and_iv() {
        let r=rows();let mut ctx=Ctx::new(&r,&r,&[]);
        ctx.extra.insert("IV30".into(),vec![20.0;5]);
        for id in ["otvt_diz_short_strangle","otvt_cvr1_sell_puts","otvt_cvr1_buy_puts","otvt_sector_options_overpriced"] {
            assert_eq!(check(&ctx,&definition(id),0,4,&Params::new()).unwrap().status,"needs_data");
            let mut def=definition(id);def.status=Status::Live;
            assert_eq!(check(&ctx,&def,0,4,&Params::new()).unwrap().status,"needs_data");
        }
    }
    #[test]
    fn options_quotes_do_not_validate_underlying_based_equity() {
        let r=rows();
        for id in ["oiss_short_put","oiss_plus_three_put","oiss_short_call","oiss_plus_three_call"] {
            let mut ctx=Ctx::new(&r,&r,&[]);
            let def=definition(id);
            assert_eq!(check(&ctx,&def,0,4,&Params::new()).unwrap().status,"needs_data");
            for name in def.needs_series {ctx.extra.insert((*name).into(),vec![10.0;5]);}
            assert_eq!(check(&ctx,&def,0,4,&Params::new()).unwrap().status,"calculation_unavailable");
        }
    }
    #[test]
    fn external_series_are_checked_in_the_selected_window() {
        let r=rows();let mut ctx=Ctx::new(&r,&r,&[]);let def=definition("trin4");
        ctx.extra.insert("TRIN".into(),vec![1.0,1.0,f64::NAN,f64::NAN,f64::NAN]);
        let e=check(&ctx,&def,2,4,&Params::new()).unwrap();
        assert_eq!(e.status,"needs_data");assert!(e.note.contains("2020-01-03"));
        assert!(check(&ctx,&def,0,1,&Params::new()).is_none());
    }
    #[test]
    fn reviewed_chande_rejects_legacy_cash_stops_and_keeps_ntt_held() {
        let r=rows();let ctx=Ctx::new(&r,&r,&[]);
        for id in ["ch_bta2_ma_stack","ch_bta2_65sma_ravi"] {
            let def=definition(id);
            assert!(check(&ctx,&def,0,4,&Params::new()).is_none());
            let p=Params::from([("stop_pts".into(),4.)]);
            assert!(check(&ctx,&def,0,4,&p).unwrap().note.contains("Futures dollar stops"));
        }
        for id in ["ch_ntt_vidya_cmo","ch_ntt_to_buy","ch_ntt_to_buy_nse"] {
            assert!(crate::instrument_policy::check(&definition(id),"NIFTY 50",false).is_some());
        }
    }
    #[test]
    fn volume_is_required_on_each_day_and_prices_must_be_valid() {
        let mut r=rows();r[2].volume=None;let ctx=Ctx::new(&r,&r,&[]);
        let mut def=definition("f1a");def.needs_volume=true;
        assert!(check(&ctx,&def,0,4,&Params::new()).unwrap().note.contains("volume"));
        let mut r=rows();r[3].close=200.0;let ctx=Ctx::new(&r,&r,&[]);
        assert!(check(&ctx,&definition("f1a"),0,4,&Params::new()).unwrap().note.contains("OHLC"));
    }
    #[test]
    fn invalid_period_and_discretionary_rules_cannot_emit_results() {
        let r=rows();let ctx=Ctx::new(&r,&r,&[]);
        assert!(check(&ctx,&definition("f1a"),4,4,&Params::new()).is_some());
        assert!(check(&ctx,&definition("f1a"),0,9,&Params::new()).is_some());
        assert!(check(&ctx,&definition("f1a"),9,10,&Params::new()).is_some());
        assert_eq!(check(&ctx,&definition("oiss_intraday_exit_variant"),0,4,&Params::new()).unwrap().status,"not_testable");
    }
    #[test]
    fn ehlers_exact_ids_require_real_valid_warmup_and_valid_platform_settings() {
        let mut r:Vec<Row>=(0..100).map(|i|Row{ticker:"AAPL".into(),date:format!("{i:05}"),open:100.,high:101.,low:99.,close:100.,volume:None}).collect();
        for id in ["ehrs_sinetrend","ehrs_sinetrend_trend","ehrs_sinetrend_cp115","ehrs_sinetrend_cp115_trend","ehmd_mama"] {
            let ctx=Ctx::new(&r,&r,&[]);let d=definition(id);
            assert_eq!(check(&ctx,&d,50,99,&Params::new()).unwrap().status,"needs_data");
            assert!(check(&ctx,&d,65,99,&Params::new()).is_none());
            let bad=Params::from([("max_bars_back".into(),50.5)]);
            assert_eq!(check(&ctx,&d,60,99,&bad).unwrap().status,"not_testable");
        }
        for id in ["ehrs_sinetrend_cp115","ehrs_sinetrend_cp115_trend"] {
            let ctx=Ctx::new(&r,&r,&[]);let d=definition(id);
            assert_eq!(check(&ctx,&d,61,99,&Params::new()).unwrap().status,"needs_data");
            assert!(check(&ctx,&d,62,99,&Params::new()).is_none());
            assert_eq!(check(&ctx,&d,65,99,&Params::from([("max_bars_back".into(),50.)])).unwrap().status,"not_testable");
        }
        r[2].close=f64::NAN;let ctx=Ctx::new(&r,&r,&[]);
        assert!(check(&ctx,&definition("ehmd_mama"),60,99,&Params::new()).unwrap().note.contains("warmup"));
    }
    #[test]
    fn volatility_data_must_cover_the_period_and_its_internal_days() {
        let r=rows();let mut def=definition("f1a");def.needs_vix=true;
        let ctx=Ctx::new(&r,&r,&[]);
        assert!(check(&ctx,&def,0,4,&Params::new()).is_some());
        let ctx=Ctx::new(&r,&r,&r[2..]);
        assert!(check(&ctx,&def,0,4,&Params::new()).unwrap().note.contains("2020-01-03"));
        assert!(check(&ctx,&def,2,4,&Params::new()).is_none());
        let vix=vec![r[0].clone(),r[1].clone(),r[3].clone(),r[4].clone()];
        let ctx=Ctx::new(&r,&r,&vix);
        assert!(check(&ctx,&def,0,4,&Params::new()).unwrap().note.contains("2020-01-03"));
    }
    #[test]
    fn changing_volatility_source_requires_all_of_that_sources_prices() {
        let r=rows();let mut ctx=Ctx::new(&r,&r,&r);let def=definition("ptjmt_cvr1");
        let p=Params::from([("src".into(),1.0)]);
        assert!(check(&ctx,&def,0,4,&p).unwrap().note.contains("VXO_OHLC"));
        for name in ["VXO_OHLC.Open","VXO_OHLC.High","VXO_OHLC.Low","VXO_OHLC.Close"] {
            ctx.extra.insert(name.into(),vec![20.0;5]);
        }
        assert!(check(&ctx,&def,0,4,&p).is_none());
        ctx.extra.get_mut("VXO_OHLC.Open").unwrap()[3]=f64::NAN;
        assert!(check(&ctx,&def,0,4,&p).unwrap().note.contains("2020-01-04"));
    }
    #[test]
    fn empty_execution_events_are_distinct_from_missing_daily_source_bars() {
        let r=rows();let mut ctx=Ctx::new(&r,&r,&[]);let mut def=definition("f1a");
        def.needs_series=&["OPEN15_HIGH","LONG_FILLS"];
        ctx.extra.insert("OPEN15_HIGH".into(),vec![101.0;5]);
        ctx.extra.insert("LONG_FILLS".into(),vec![f64::NAN,100.0,f64::NAN,f64::NAN,f64::NAN]);
        assert!(check(&ctx,&def,0,4,&Params::new()).is_none());
        ctx.extra.get_mut("OPEN15_HIGH").unwrap()[2]=f64::NAN;
        assert!(check(&ctx,&def,0,4,&Params::new()).unwrap().note.contains("OPEN15_HIGH"));
        def.status=Status::NeedsData("stock universe");
        assert_eq!(check(&ctx,&def,0,4,&Params::new()).unwrap().status,"needs_data");
    }
}

#[cfg(test)]
mod kestner_next_readiness_tests {
    use super::*;
    fn rows()->Vec<crate::Row> {(0..112).map(|i|crate::Row{ticker:"AAPL".into(),date:format!("2000-{:02}-{:02}",1+i/28,1+i%28),open:100.,high:101.,low:99.,close:100.,volume:None}).collect()}
    #[test]
    fn next_profile_input_checks_retain_history_and_exact_settings() {
        let r=rows();let ctx=Ctx::new(&r,&[],&[]);
        for &id in books::lkqts::NEXT_CANDIDATE_IDS {
            let d=books::lkqts::catalogue().into_iter().find(|d|d.id==id).unwrap();
            assert!(check(&ctx,&d,100,110,&Params::new()).is_none(),"{id}");
            assert_eq!(check(&ctx,&d,100,110,&Params::from([("unknown".into(),0.)])).unwrap().status,"not_testable");
            let mut bad=r.clone();bad[1].date=bad[0].date.clone();let badctx=Ctx::new(&bad,&[],&[]);
            assert_eq!(check(&badctx,&d,100,110,&Params::new()).unwrap().status,"needs_data");
            let mut future=r.clone();future[111].close=f64::NAN;let futurectx=Ctx::new(&future,&[],&[]);
            assert!(check(&futurectx,&d,100,110,&Params::new()).is_none(),"unused future row {id}");
        }
    }
}
