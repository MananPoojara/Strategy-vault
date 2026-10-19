import {expect,it} from 'vitest';
import {createElement} from 'react';
import {renderToStaticMarkup} from 'react-dom/server';
import {sourceAccount,sourceAccountCopy,sourceAccountView,sourceAccountTables} from '../src/services/source-account';
import {simpleInstrumentContext,parseBandyAdapterContract} from '../src/services/bandy-source-account';
import {SourceAccountNotice} from '../src/components/SourceAccountNotice';
import {instrumentControls} from '../src/services/instrument-controls';
import {simpleAccountFixture,simpleAdmissionFixture} from './helpers/bandy-simple-account-fixture';
import {bandyAccountFixture} from './helpers/bandy-source-account-fixture';
const parse=(data:unknown=simpleAccountFixture(),context:unknown=simpleAdmissionFixture())=>sourceAccount(data,'hbiab_simple_ma25','hbiab','basket:us_sp500:AAPL',context)!;
it('admits only the fourth exact Simple method with its own declared source contract and conserved unchanged ledger',()=>{
 for(const bps of [0,5,.0000003]){
  const data=simpleAccountFixture(bps),before=JSON.stringify(data),account=parse(data);
  expect(account.model).toBe('bandy_fixed_fraction_fractional_cash_v1');expect(account.trade_accounts).toHaveLength(2);expect(JSON.stringify(data)).toBe(before);
  const tables=sourceAccountTables(account,data.trades.map(t=>t.ret_full_precision));expect(tables.audit.headers).toHaveLength(20);expect(tables.daily.headers).toHaveLength(9);
 }
});
it('requires matching Simple identity, no basket kind, declared initial unit and exact per-method basis',()=>{
 const d=simpleAccountFixture();
 for(const fields of [{id:'hbiab_ma5_20'},{book:'other'},{kind:'basket'},{kind:'single'},{model_initial_equity:10000},{accounting_basis:'shared MA basis'}])expect(()=>parse({...d,...fields})).toThrow('Bandy cash account');
 expect(()=>parse({...d,source_account:{...d.source_account,basis:'Arithmetic-only shared MA account.'}})).toThrow();
 expect(()=>parse({...d,source_account:{...d.source_account,model:'unknown'}})).toThrow();
 expect(()=>sourceAccount(d,'hbiab_simple_ma25','other','basket:us_sp500:AAPL',simpleAdmissionFixture())).toThrow();
});
it('requires the exact reviewed Simple metadata, convention, defaults and independent stock/ETF policy',()=>{
 const base=simpleAdmissionFixture();
 expect(()=>sourceAccount(simpleAccountFixture(),'hbiab_simple_ma25','hbiab','basket:us_sp500:AAPL')).toThrow();
 const corrupt=[null,{}, {...base,parameters:{ma:24,hold:5}}, {...base,parameters:{ma:25,hold:5.0000000001}}, {...base,parameters:{ma:25}}, {...base,parameters:{ma:25,hold:5,fast:5}},
  {...base,definition:{...base.definition,id:'hbiab_ma5_20'}}, {...base,definition:{...base.definition,book:'other'}},
  ...[{source_basis:'old MA source'},{source_convention:'bandy_sma_seed_fractional_cash_v1'},{account_model:'unknown'},{execution_scope:'single_symbol'},{allowed_types:['index']},{allowed_types:['individual_stock','etf','index']},{source_performance_reproduced:true},{source_rule_reviewed:false}].map(change=>({...base,definition:{...base.definition,instrument_policy:{...base.definition.instrument_policy,...change}}}))];
 for(const context of corrupt)expect(()=>parse(simpleAccountFixture(),context)).toThrow('Bandy cash account');
});
it('rejects next-open timing, pending orders, sizing changes, other-instrument source and monetary invention',()=>{
 const d=simpleAccountFixture();
 const corrupt=[
  {...d,source_account:{...d.source_account,sizing:{...d.source_account.sizing,target_equity_fraction:.25,origin:'source_rule'}}},
  {...d,source_account:{...d.source_account,terminal_entry_pending:true,pending_order:{side:'buy',signal_date:d.dates.at(-1),fill_phase:'open',filled:false}}},
  {...d,source_account:{...d.source_account,terminal_exit_pending:true}},
  {...d,source_account:{...d.source_account,trade_accounts:d.source_account.trade_accounts.map((t,i)=>i? t:{...t,entry_phase:'open',entry_signal_date:d.dates[0]})}},
  {...d,source_account:{...d.source_account,trade_accounts:d.source_account.trade_accounts.map((t,i)=>i? t:{...t,exit_phase:'open',exit_signal_date:d.dates[1]})}},
  {...d,provenance:{...d.provenance,asset_type:'index'}},
  {...d,trades:d.trades.map((t,i)=>i? t:{...t,accounting:{...t.accounting,symbol:'OTHER'}})},
  {...d,trades:d.trades.map((t,i)=>i? t:{...t,accounting:{...t.accounting,net_pnl:1000}})},
 ];
 for(const data of corrupt)expect(()=>parse(data)).toThrow('Bandy cash account');
 expect(()=>sourceAccount(d,'hbiab_simple_ma25','hbiab','SPX',simpleAdmissionFixture())).toThrow();
});
it('describes the teaching example and raw-Buy clock without claiming a fixed holding period or printed execution/sizing',()=>{
 const account=parse(),copy=sourceAccountCopy(account),markup=renderToStaticMarkup(createElement(SourceAccountNotice,{account:sourceAccountView(account)}));
 expect(copy.sizing).toContain('100%');expect(copy.sizing).toContain('teaching');expect(markup).toContain('every true raw Buy resets');expect(markup).toContain('not five bars from entry');expect(markup).toContain('declared daily');expect(markup).not.toMatch(/2%|6%|₹|SafeZone/);
 const controls=instrumentControls(simpleAdmissionFixture().definition.instrument_policy,'hbiab_simple_ma25','hbiab');expect([controls.stock,controls.etf,controls.index,controls.basket]).toEqual([true,true,false,false]);expect(controls.note).toContain('Stock or ETF');
});
it('preserves all three original parser paths without requiring or inheriting Simple metadata',()=>{
 for(const id of ['hbiab_ma5_20','hbiab_ma7_4','hbiab_macd_example']){
  const d={...bandyAccountFixture(id==='hbiab_macd_example'),id},before=JSON.stringify(d);
  const a=sourceAccount(d,id,'hbiab','AAPL',{definition:{bad:true},parameters:{ma:24}})!;expect(a.model).toBe('bandy_fixed_fraction_fractional_cash_v1');expect(sourceAccountCopy(a).sizing).not.toContain('teaching');expect(JSON.stringify(d)).toBe(before);
 }
});

it('binds unchanged native stock metadata and existing eligible ETF rows without creating source provenance',()=>{
 const stock={ticker:'basket:us_sp500:AAPL',symbol:'AAPL',asset_type:'individual_stock',provenance:{symbol:'AAPL',qualified_ticker:'basket:us_sp500:AAPL',asset_type:'individual_stock',source_sha256:'a'.repeat(64),csv_sha256:'b'.repeat(64)}};
 const spy={ticker:'SPY',first:'1993-01-29',last:'2026-10-05',has_volume:true,series:[]};const unknown={ticker:'UNKNOWN'};const index={ticker:'SPX'};const before=JSON.stringify([stock,spy,unknown,index]);
 const stockContext=simpleInstrumentContext(stock.ticker,[stock,spy],[spy]);expect(stockContext?.metadata).toBe(stock);expect(stockContext?.classification_basis).toBe('native_stock_metadata');
 const etf=simpleInstrumentContext('SPY',[stock,spy],[spy]);expect(etf?.metadata).toBe(spy);expect(etf?.classification_basis).toBe('existing_runtime_etf_selector');expect(spy).not.toHaveProperty('asset_type');expect(spy).not.toHaveProperty('provenance');
 expect(simpleInstrumentContext('UNKNOWN',[stock,spy,unknown],[spy])).toBeNull();expect(simpleInstrumentContext('SPX',[index],[index])).toBeNull();expect(simpleInstrumentContext('SPY',[spy],[{...spy}])).toBeNull();
 const d=simpleAccountFixture(),withSource={...d,provenance:{...d.provenance,...stock.provenance}},context={...simpleAdmissionFixture(),instrument:stockContext};expect(parse(withSource,context).model).toBe('bandy_fixed_fraction_fractional_cash_v1');
 for(const provenance of [{...withSource.provenance,source_sha256:'c'.repeat(64)},{...withSource.provenance,csv_sha256:undefined},{...withSource.provenance,symbol:'OTHER'}])expect(()=>parse({...withSource,provenance},context)).toThrow();
 expect(()=>parse({...withSource,ticker:'OTHER'},context)).toThrow();expect(()=>parse({...withSource,trades:withSource.trades.map((t,i)=>i?t:{...t,ticker:'OTHER'})},context)).toThrow();
 const {provenance:discard,...bare}=d;const etfData={...bare,trades:d.trades.map(t=>({...t,accounting:{...t.accounting,symbol:'SPY'}}))};
 expect(sourceAccount(etfData,'hbiab_simple_ma25','hbiab','SPY',{...simpleAdmissionFixture(),instrument:etf})!.model).toBe('bandy_fixed_fraction_fractional_cash_v1');
 for(const instrument of [null,{}, {...etf,kind:'unknown'}, {...etf,classification_basis:'invented'}, {...etf,eligible_etf_tickers:[]}, {...etf,metadata:{...spy,asset_type:'index'}}, {...etf,metadata:{ticker:'SPX'},eligible_etf_tickers:['SPX']}])expect(()=>sourceAccount(etfData,'hbiab_simple_ma25','hbiab','SPY',{...simpleAdmissionFixture(),instrument})).toThrow();
 expect(JSON.stringify([stock,spy,unknown,index])).toBe(before);
});
it('keeps synthetic adapter checks separate from public sourceAccount admission',()=>{
 const d=simpleAccountFixture(),{provenance:discard,...bare}=d;const fixture={...bare,trades:d.trades.map(t=>({...t,accounting:{...t.accounting,symbol:'SYNTHETIC'}}))};
 const context={...simpleAdmissionFixture(),instrument:{kind:'synthetic_test_only',classification_basis:'independent_synthetic_fixture',metadata:{ticker:'SYNTHETIC',symbol:'SYNTHETIC'}},adapter_contract:{scope:'independently_bound_reporting_fixture',case_id:'arithmetic-only-test',input_sha256:'a'.repeat(64),request_sha256:'b'.repeat(64),capture_sha256:'c'.repeat(64)}};
 expect(parseBandyAdapterContract(fixture,'hbiab_simple_ma25','SYNTHETIC',context).model).toBe('bandy_fixed_fraction_fractional_cash_v1');
 expect(()=>sourceAccount(fixture,'hbiab_simple_ma25','hbiab','SYNTHETIC',context)).toThrow();
 const publicNativeContext={...context,instrument:simpleAdmissionFixture().instrument};expect(()=>sourceAccount(bare,'hbiab_simple_ma25','hbiab','basket:us_sp500:AAPL',publicNativeContext)).toThrow();expect(()=>parseBandyAdapterContract(fixture,'hbiab_simple_ma25','SYNTHETIC',{...context,adapter_contract:{...context.adapter_contract,input_sha256:''}})).toThrow();
 expect(()=>parseBandyAdapterContract(fixture,'hbiab_ma5_20','SYNTHETIC',context)).toThrow();
});
