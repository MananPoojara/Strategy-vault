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

pub fn check(ctx: &Ctx, def: &StrategyDef, from: usize, to: usize, p: &Params) -> Option<Issue> {
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
        if g.verdict == "withheld" {
            return issue("not_testable", format!("Withheld by the accuracy check: {} No results are published.", g.reason));
        }
        if let (Some(allowed), Some(row)) = (g.allowed, ctx.r.get(from)) {
            if !allowed.contains(&row.ticker.as_str()) {
                return issue("not_testable", format!("Not published on {}: this rule uses fixed point or dollar amounts set for {}; on this instrument they would not mean what the book meant. {}", row.ticker, if allowed.is_empty() { "an instrument this vault does not hold".to_string() } else { allowed.join(", ") }, g.reason));
            }
        }
    }
    if from >= ctx.r.len() || to >= ctx.r.len() || from >= to {
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
        let Some(start)=ctx.vix.first else { return issue("needs_data","The required volatility-index history is not loaded for this market.") };
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
    fn rows() -> Vec<Row> {
        (1..=5).map(|i| Row{ticker:"TEST".into(),date:format!("2020-01-0{i}"),open:100.0,high:101.0,low:99.0,close:100.0,volume:Some(1000.0)}).collect()
    }
    fn definition(id: &str) -> StrategyDef { crate::connors::catalogue().into_iter().find(|d| d.id==id).unwrap() }
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
