import {describe,it,expect} from 'vitest';
import {firstCloseDema,demaSourceArrays,demaExitClock} from '../src/services/dema-visual';
import {buildStrategyIndicators} from '../src/services/strategy-indicators';
import {buildRuleEvidence,makeRuleVisualModel} from '../src/services/rule-evidence';
import {UNIT_EXPOSURE_CONVENTION} from '../src/services/unit-exposure-report';
const dates=(n:number)=>Array.from({length:n},(_,i)=>new Date(Date.UTC(2024,0,2+i)).toISOString().slice(0,10));
const prices=(c:number[])=>({ticker:'SPX',full_history:true,d:dates(c.length),o:c,h:c,l:c,c});
const params={fast:16,slow:8,hold:1};
function model(c=[100,99,98,100,97,99,96,100]) {
 const px=prices(c);return makeRuleVisualModel({strategy:{id:'hbqts_dema16_8',book:'hbqts',instrument_policy:{source_rule_reviewed:true,source_convention:UNIT_EXPOSURE_CONVENTION,source_basis:'Explicit synthetic unit-test source context only.'}},run:{dates:px.d,trades:[]},prices:px,parameters:params,ticker:'SPX',symbol:'SPX'});
}
const item=(m:ReturnType<typeof model>,i:number,label:string)=>buildRuleEvidence(m).at(i).find(x=>x.label===label)!;
function actualTestTrade(m:ReturnType<typeof model>,entry:number,exit:number,open=false){
 return {key:0,symbol:'SPX',ticker:'SPX',direction:1,entry:m.prices!.d[entry],exit:m.prices!.d[exit],open,reason:open?'open':'formula exit',entryKind:'close',signal:m.prices!.d[exit],legs:[{date:m.prices!.d[entry],price:m.prices!.c[entry],weight:1}],exits:[],exitPrice:m.prices!.c[exit]};
}
describe('exact ordered first-close DEMA visual inputs',()=>{
 it('seeds at the first actual close and preserves printed16/8 order',()=>{
  const a=demaSourceArrays([100,90,95]);expect(a.first[0]).toBe(100);expect(a.second[0]).toBe(100);
  expect(a.first[1]).toBe(97.78546712802769);expect(a.first[1]).toBeGreaterThan(a.second[1]);
  expect(a.rawBuy).toEqual([false,true,false]);expect(a.rawBarsSinceBuy).toEqual([null,0,1]);expect(a.filteredSell).toEqual([false,false,true]);
 });
 it('keeps flat equality non-crossing and raw Sell before any Buy distinct from a filled exit',()=>{
  const flat=demaSourceArrays([100,100,100,100]);expect(flat.first).toEqual([100,100,100,100]);expect(flat.rawBuy).toEqual([false,false,false,false]);expect(flat.rawSell).toEqual([false,false,false,false]);
  const rising=model([100,101,102]);expect(item(rising,1,'Reverse crossover').passed).toBe(true);expect(item(rising,1,'Formula Sell after Sell-only ExRem').passed).toBe(true);expect(item(rising,1,'Source exit permitted').passed).toBe(false);expect(item(rising,1,'Recorded exit fill').passed).toBe(false);
 });
 it('resets on every raw Buy and filters Sell only, not the Buy clock',()=>{
  const buy=[false,true,true,false,false,true,false],reverse=buy.map(()=>false),copy=[...buy];const a=demaExitClock(buy,reverse);
  expect(buy).toEqual(copy);expect(a.rawBarsSinceBuy).toEqual([null,0,0,1,2,0,1]);expect(a.rawSell).toEqual([false,false,false,true,true,false,true]);expect(a.filteredSell).toEqual([false,false,false,true,false,false,true]);
 });
 it('retains full-prefix seeds and formula state when selected dates change',()=>{
  const m=model(),full=buildRuleEvidence(m);expect(full.scope).toBe('exact');m.dates=m.dates.slice(2);
  const selected=buildRuleEvidence(m);expect(selected.series).toEqual(full.series);expect(item(m,1,'Source position before close').value).toBeNull();expect(item(m,2,'Formula Sell after Sell-only ExRem').passed).toBe(true);expect(item(m,2,'Source exit permitted').passed).toBe(false);
  const c=m.prices!.c;for(let end=1;end<c.length;end++){const prefix=demaSourceArrays(c.slice(0,end+1)),whole=demaSourceArrays(c);for(const key of Object.keys(prefix) as (keyof typeof prefix)[])expect(prefix[key]).toEqual(whole[key].slice(0,end+1));}
 });
 it('separates source conditions, accepted position permission, filled exits and open marks',()=>{
  const m=model([100,99,98]);m.trades=[actualTestTrade(m,1,2)];expect(item(m,1,'Raw Buy crossover').passed).toBe(true);expect(item(m,1,'Source entry permitted').passed).toBe(true);expect(item(m,1,'Recorded entry fill').passed).toBe(true);expect(item(m,2,'Source exit permitted').passed).toBe(true);expect(item(m,2,'Recorded exit fill').passed).toBe(true);
  const open=model([100,99]);open.trades=[actualTestTrade(open,1,1,true)];expect(item(open,1,'Recorded entry fill').passed).toBe(true);expect(item(open,1,'Recorded exit fill').passed).toBe(false);
  m.trades[0].legs[0].price=123;expect(item(m,1,'Source entry permitted').passed).toBeNull();expect(item(m,1,'Recorded entry fill').passed).toBeNull();
 });
 it('plots only the exact fixed source/context and samples real weekly candle dates',()=>{
  const px=prices([100,99,98,100,97,99]),request={id:'hbqts_dema16_8',book:'hbqts',ticker:'SPX',prices:px,fullHistory:true,parameters:params};
  const a=buildStrategyIndicators(request);expect(a.status).toBe('ready');expect(a.series.map(x=>x.label)).toEqual(['DEMA(16) · first-close seed','DEMA(8) · first-close seed','Bars since raw Buy']);expect(a.series[0].kind).toBe('dema');
  const sampled=buildStrategyIndicators({...request,displayDates:[px.d[2],px.d[5]]});expect(sampled.series[0].points).toEqual([a.series[0].points[2],a.series[0].points[5]]);expect(a.note).toContain('native AmiBroker parity');
  for(const changed of [{book:'other'},{ticker:'NDX'},{fullHistory:false},{parameters:{fast:8,slow:16,hold:1}},{parameters:{...params,extra:1}},{prices:{...px,ticker:'SPY'}},{prices:{...px,full_history:false}}])expect(buildStrategyIndicators({...request,...changed} as any).status).toBe('unavailable');
 });
 it('withholds mismatched book/convention/instrument or source prices without an SMA fallback',()=>{
  const m=model();for(const changed of [{book:'other'},{sourceQualifications:[]},{basket:true},{parameters:{fast:8,slow:16,hold:1}},{prices:{...m.prices!,full_history:false}}])expect(buildRuleEvidence({...m,...changed}).scope).toBe('unavailable');
  expect(buildStrategyIndicators({id:'hbqts_dema16_8',book:'hbqts',ticker:'SPY',prices:{...prices([100,99]),ticker:'SPY'},fullHistory:true,parameters:params}).series).toEqual([]);
  expect(()=>firstCloseDema([100,NaN],16)).toThrow();expect(()=>demaExitClock([true],[false,true])).toThrow();
 });
});
