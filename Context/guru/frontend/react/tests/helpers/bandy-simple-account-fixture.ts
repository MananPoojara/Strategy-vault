/** Arithmetic-only account fixture for the frozen Simple wire; not rule/market results. */
import {bandyAccountFixture} from './bandy-source-account-fixture';
export const simpleBasis = "Bandy Introduction to AmiBroker (2008), printed527-528, SimpleSystem.afl interpretation/commentary teaching example shown on actual SPY; no printed performance study. MA25=MA(C,25), raw Buy=Close>MA25, Sell=BarsSince(raw Buy)>=5; every true raw Buy resets the clock even while held, and equality is false. Twenty-five genuine observations mature MA; no actionable sell before the first observed Buy and no native BarsSince sentinel claim. Daily finalized same-close fills, one long without pyramiding, 100% fractional current-equity target with entry-cost cash reserve are declared research conventions. Normalized initial equity1, full supplied signal history and flat selected account start; terminal entries remain open and pay actual entry cost only. Gross removes paid costs from identical accepted quantities; liquidation estimate separate. Actual stocks/ordinary equity ETFs only; no cash-index, basket portfolio or derivative substitution. No source performance, funded capital, native AmiBroker binary parity or broker-execution claim.";
export function simpleAccountFixture(bps=5) {
 const base=bandyAccountFixture(false,bps);
 return {...base,id:'hbiab_simple_ma25',accounting_basis:simpleBasis,model_initial_equity:1,
  provenance:{qualified_ticker:'basket:us_sp500:AAPL',symbol:'AAPL',asset_type:'individual_stock',trusted_market:'us'},
  source_account:{...base.source_account,basis:simpleBasis}};
}
export function simpleAdmissionFixture() {
 return {definition:{id:'hbiab_simple_ma25',book:'hbiab',instrument_policy:{allowed_types:['individual_stock','etf'],execution_scope:'single_symbol_source_account',
  source_basis:simpleBasis,source_convention:'bandy_simple_raw_buy_clock_fractional_cash_v1',account_model:'bandy_fixed_fraction_fractional_cash_v1',
  source_rule_reviewed:true,independent_selected_ledger_checked:true,source_performance_reproduced:false}},parameters:{ma:25,hold:5},instrument:{kind:"individual_stock",classification_basis:"native_stock_metadata",metadata:{ticker:"basket:us_sp500:AAPL",symbol:"AAPL",asset_type:"individual_stock"}}};
}
