import {afterEach,describe,it,expect,vi} from 'vitest';
import {act} from 'react';
import {createRoot,type Root} from 'react-dom/client';
import {validatedUnitExposureReport,isValidatedUnitExposureView} from '../src/services/unit-exposure-report';
import {closedUnitEquity,unitExposureTables,unitExposureMetadata,UNIT_VISUAL_BASIS} from '../src/services/unit-exposure-presenter';
import {returnDisplay} from '../src/services/return-display';
import {instrumentControls,instrumentTickers} from '../src/services/instrument-controls';
import {unitRuntimeFixture} from './helpers/unit-runtime-fixture';
import {StrategyPage} from '../src/pages/ResearchPages';
import * as rounding from '../src/services/unit-exposure-rounding';
let root:Root|undefined,host:HTMLDivElement|undefined;
afterEach(async()=>{if(root)await act(async()=>root!.unmount());host?.remove();root=undefined;host=undefined;vi.restoreAllMocks();});
describe('one validated unit report reused without mutable financial bypass',()=>{
 it('runs the full factor audit once, protects every financial subtree and leaves diagnostics extensible',()=>{
  const {data,admission}=unitRuntimeFixture(),spy=vi.spyOn(rounding,'unitCurveConsistent');
  const a=validatedUnitExposureReport(data,admission),first=spy.mock.calls.length;expect(first).toBeGreaterThan(0);
  expect(validatedUnitExposureReport(data,admission)).toBe(a);expect(spy).toHaveBeenCalledTimes(first);
  expect(isValidatedUnitExposureView(a)).toBe(true);expect(isValidatedUnitExposureView({...a})).toBe(false);
  expect(()=>{data.stats.win_rate=1;}).toThrow();expect(()=>{data.trades[0].at_entry.changed=1;}).toThrow();
  expect(()=>{data.unit_exposure_report.days[1].equity=1;}).toThrow();expect(()=>{data.ohlc.c[1]=1;}).toThrow();
  expect(()=>{data.costs.slippage_bps=0;}).toThrow();expect(()=>{data.trades=[];}).toThrow();expect(()=>{data.provenance={symbol:'fake'};}).toThrow();
  data._ms=12;data.nonfinancialDiagnostic='available';expect(data._ms).toBe(12);expect(Object.isExtensible(data)).toBe(true);
  expect(validatedUnitExposureReport(data,admission)).toBe(a);expect(spy).toHaveBeenCalledTimes(first);
 });
 it.each([
  ['ticker',(a:any)=>{a.ticker='NDX';}],['region',(a:any)=>{a.region='in';}],['parameters',(a:any)=>{a.parameters.fast=17;}],
  ['costs',(a:any)=>{a.costs.slippage_bps=0;}],['policy',(a:any)=>{a.definition.instrument_policy.source_rule_reviewed=false;}],
  ['basis',(a:any)=>{a.definition.instrument_policy.source_basis='other';}],['metadata bounds',(a:any)=>{a.instrument.metadata.first='2024-01-04';}],
  ['instrument identity',(a:any)=>{a.instrument.metadata.ticker='SPY';}],['requested window',(a:any)=>{a.window.from='2024-01-04';}],
 ])('rejects changed %s on a cached response',(_name,mutate)=>{
  const {data,admission}=unitRuntimeFixture();validatedUnitExposureReport(data,admission);mutate(admission);
  expect(()=>validatedUnitExposureReport(data,admission)).toThrow(/unit-exposure/);
 });
 it('rejects newly added cash/basket flags and missing reports without legacy financial fallback',()=>{
  const {data,admission}=unitRuntimeFixture();validatedUnitExposureReport(data,admission);data.source_account=null;
  expect(()=>validatedUnitExposureReport(data,admission)).toThrow();delete data.source_account;data.kind='basket';expect(()=>validatedUnitExposureReport(data,admission)).toThrow();
  const absent=unitRuntimeFixture();delete absent.data.unit_exposure_report;
  expect(()=>validatedUnitExposureReport(absent.data,absent.admission)).toThrow();
  expect(returnDisplay(absent.data).value).toBeNull();expect(returnDisplay(absent.data).unavailableReason).toContain('validated');
  expect(returnDisplay(unitRuntimeFixture().data).value).toBeNull();
 });
 it('revalidates a clone instead of inheriting another object’s acceptance',()=>{
  const {data,admission}=unitRuntimeFixture();const first=validatedUnitExposureReport(data,admission),clone=structuredClone(data);
  expect(returnDisplay({...data},'compound',null,first).value).toBeNull();
  expect(returnDisplay({...data},'compound',null,first).unavailableReason).toContain('validated');
  expect(validatedUnitExposureReport(clone,admission)).not.toBe(first);const bad=structuredClone(data);bad.stats.win_rate=1;expect(()=>validatedUnitExposureReport(bad,admission)).toThrow();
 });
 it.each(['basket','cash','portfolio','unknown',null,undefined])('rejects any own kind field cold or hot: %s',kind=>{
  const cold=unitRuntimeFixture();cold.data.kind=kind;expect(()=>validatedUnitExposureReport(cold.data,cold.admission)).toThrow();
  const hot=unitRuntimeFixture();validatedUnitExposureReport(hot.data,hot.admission);hot.data.kind=kind;
  expect(()=>validatedUnitExposureReport(hot.data,hot.admission)).toThrow();
 });
});
describe('model-specific closed display and exports',()=>{
 it('uses the engine’s last closed exit equity and excludes a terminal-open cost mark',()=>{
  const {data,admission}=unitRuntimeFixture(true),view=validatedUnitExposureReport(data,admission),last=view.report.trade_returns.filter(t=>!t.open).at(-1)!;
  expect(view.report.trade_returns.at(-1)?.open).toBe(true);
  const closedEquity=view.report.days[last.exit_source_index-view.report.days[0].source_index].equity;
  expect(returnDisplay(data,'compound',null,view).value).toBe(closedEquity-1);
  expect(closedEquity).not.toBe(view.report.days.at(-1)!.equity);
  expect(returnDisplay(data,'fixed',null,view).value).toBe(view.closedReturnSum);
  expect(returnDisplay(data,'compound',null,view).description).toContain('terminal-open marks are excluded');
 });
 it('does not reconstruct a positive tiny factor using1+rounded return, and handles no closed trades',()=>{
  const {data}=unitRuntimeFixture();const report=data.unit_exposure_report;
  report.trade_returns[0].net_return=-1;report.days[2].equity=Number.MIN_VALUE;
  expect(1+report.trade_returns[0].net_return).toBe(0);expect(closedUnitEquity(report)).toBe(Number.MIN_VALUE);
  report.trade_returns[0].open=true;expect(closedUnitEquity(report)).toBe(1);
 });
 it('keeps all13 audit and8 daily engine fields with no cash-risk or money columns',()=>{
  const {data,admission}=unitRuntimeFixture(true),view=validatedUnitExposureReport(data,admission),tables=unitExposureTables(view);
  expect(tables.audit.headers).toHaveLength(13);expect(tables.daily.headers).toHaveLength(8);
  view.report.trade_returns.forEach((t,i)=>expect(tables.audit.rows[i]).toEqual([t.trade_index,t.entry_source_index,t.exit_source_index,t.open,t.entry_allocation,t.gross_return,t.net_return,t.fee_return,t.slippage_return,t.gross_contribution,t.net_contribution,t.fee_contribution,t.slippage_contribution]));
  view.report.days.forEach((d,i)=>expect(tables.daily.rows[i]).toEqual([d.source_index,d.date,d.equity,d.gross_equity,d.benchmark_equity,d.net_daily_return,d.gross_daily_return,d.benchmark_daily_return]));
  expect(tables.audit.headers.join(' ')+tables.daily.headers.join(' ')).not.toMatch(/cash|shares|monthly|risk budget|₹/i);
  const metadata=unitExposureMetadata(view);expect(metadata).toContainEqual(['Chart/calendar presentation',UNIT_VISUAL_BASIS]);expect(metadata).toContainEqual(['Original performance reproduced',false]);
  expect(()=>unitExposureTables({...view})).toThrow();
 });
 it('restricts only the exact method to its reviewed SPX or literal Nifty index choices',()=>{
  const {admission}=unitRuntimeFixture();const policy=admission.definition.instrument_policy;
  expect(instrumentTickers(policy,[{ticker:'SPX'},{ticker:'NDX'},{ticker:'NIFTY 50'}],[{ticker:'SPY'}],admission.id,admission.book).map(t=>t.ticker)).toEqual(['SPX','NIFTY 50']);
  expect(instrumentControls(policy,admission.id,admission.book)).toMatchObject({index:true,stock:false,basket:false,etf:false});
  expect(instrumentTickers(policy,[{ticker:'SPX'},{ticker:'NDX'}],[]).map(t=>t.ticker)).toEqual(['SPX','NDX']);
 });
});
it('shows the separate unit notice, six tabs and display-only settings without cash-account claims',async()=>{
 Object.assign(globalThis,{IS_REACT_ACT_ENVIRONMENT:true});const {data,admission}=unitRuntimeFixture(),view=validatedUnitExposureReport(data,admission),patch=vi.fn();
 window.VaultRuntime={patch,activateReportTab:vi.fn()} as unknown as Window['VaultRuntime'];host=document.createElement('div');document.body.append(host);root=createRoot(host);
 const html='<header><span data-react-slot="strategy-settings"></span></header><div data-react-slot="unit-exposure-notice"></div>'+['c-eq','c-dd','heatbox','c-yr','c-px'].map(id=>`<section class="box"><div id="${id}"></div></section>`).join('')+'<section class="box" data-tradesheet></section>';
 await act(async()=>root!.render(<StrategyPage snapshot={{revision:1,loading:false,html,error:null,route:null,status:'Strategy',state:{report_mode:'fixed'},home:null,reading:null,strategyUnit:view}}/>));
 expect(host.querySelectorAll('[role=tab]')).toHaveLength(6);expect(host.querySelector('[data-unit-exposure-notice]')?.textContent).toContain(UNIT_VISUAL_BASIS);expect(host.textContent).not.toMatch(/2%|6%|SafeZone|₹/);
 await act(async()=>host!.querySelector<HTMLButtonElement>('[aria-label="Strategy settings"]')!.click());const dialog=document.querySelector('[role=dialog]')!;
 expect(dialog.textContent).toContain('Compounded closed model');expect(dialog.textContent).not.toContain('Fixed quantity');expect(dialog.textContent).toContain('independently rounded factors');
 await act(async()=>dialog.querySelector<HTMLInputElement>('input[value=compound]')!.click());await act(async()=>[...dialog.querySelectorAll<HTMLButtonElement>('button')].find(b=>b.textContent==='Apply')!.click());expect(patch).toHaveBeenCalledExactlyOnceWith({report_mode:'compound'});
});
