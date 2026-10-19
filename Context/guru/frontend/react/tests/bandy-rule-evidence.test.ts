import {describe,expect,it} from 'vitest';
import {makeRuleVisualModel,type RuleCheck,type RuleVisualModel} from '../src/services/rule-evidence';
import {buildBandyRuleEvidence} from '../src/services/bandy-rule-evidence';
type BandyModel=RuleVisualModel&{book?:string;ledgerAvailable?:boolean;focusTicker?:string};
const days=(n:number)=>Array.from({length:n},(_,i)=>new Date(Date.UTC(2020,0,i+1)).toISOString().slice(0,10));
function model(c:number[],id='hbqts_stoch_rsi',parameters:Record<string,number>={rsi:2,lb:2,trig:2}):BandyModel {
 const d=days(c.length),provenance={qualified_ticker:'basket:us_sp500:AAPL',symbol:'AAPL',asset_type:'individual_stock',source_sha256:'a'.repeat(64),csv_sha256:'b'.repeat(64)};
 return {...makeRuleVisualModel({strategy:{id,book:'hbqts',name:id,instrument_policy:{source_rule_reviewed:true}},run:{dates:d,trades:[],provenance},prices:{ticker:'AAPL',full_history:true,provenance,d,o:c,h:c.map(v=>v+1),l:c.map(v=>v*.9),c},parameters,ticker:provenance.qualified_ticker,symbol:'AAPL'}),book:'hbqts',ledgerAvailable:true};
}
function evidence(m:BandyModel){const e=buildBandyRuleEvidence(m,m.prices!);expect(e?.scope).toBe('exact');return e!;}
const check=(m:BandyModel,i:number,label:string):RuleCheck=>evidence(m).at(i).find(c=>c.label===label)!;
const point=(m:BandyModel,key:string,i:number)=>evidence(m).series.find(s=>s.key===key)!.points[i].value;
function trade(m:BandyModel,entry:number,exit:number,open=false,ticker='basket:us_sp500:AAPL') {
 return {key:0,symbol:ticker.endsWith(':AAPL')?'AAPL':'OTHER',ticker,direction:1,entry:m.prices!.d[entry],exit:m.prices!.d[exit],open,reason:open?'open':'formula exit',entryKind:'close',signal:m.prices!.d[exit],legs:[{date:m.prices!.d[entry],price:m.prices!.c[entry],weight:1}],exits:[],exitPrice:m.prices!.c[exit]};
}
describe('Bandy reviewed source arithmetic',()=>{
 it('uses P=N=0 RSI, first-finite DEMA and strict opposite Cross',()=>{
  const m=model([10,12,11,13,9]);expect(point(m,'bandy-rsi',1)).toBeNull();expect(point(m,'bandy-rsi',2)).toBe(50);expect(point(m,'bandy-rsi',3)).toBeCloseTo(100*1.25/1.5,12);
  expect(point(m,'bandy-pir',2)).toBeNull();expect(point(m,'bandy-pir',3)).toBe(1);expect(point(m,'bandy-trigger',3)).toBe(1);expect(point(m,'bandy-trigger',4)).toBeCloseTo(1/9,12);
  expect(check(m,3,'Raw buy crossover').passed).toBeNull();expect(check(m,4,'Raw sell condition').passed).toBe(true);expect(check(m,4,'Raw buy crossover').passed).toBe(false);
 });
 it('keeps zero ranges Null and resumes retained smoothing after a gap without bridging Cross',()=>{
  expect(evidence(model([10,10,10,10,10,10])).series.every(s=>s.points.every(p=>p.value===null))).toBe(true);
  const m=model([10,12,11,13,9,9,12]);expect(point(m,'bandy-pir',5)).toBeNull();expect(point(m,'bandy-trigger',5)).toBeNull();expect(point(m,'bandy-trigger',6)).toBeCloseTo(23/27,12);expect(check(m,6,'Raw buy crossover').passed).toBeNull();
 });
 it('uses fresh-window SMA and equality-aware strict Cross for both reviewed pairs',()=>{
  for(const id of ['hbqts_ma5_25','hbqts_ma1_36']){const m=model([10,9,8,10,9],id,{fast:1,slow:2});expect(point(m,'bandy-slow',2)).toBe(8.5);expect(check(m,3,'Raw buy crossover').passed).toBe(true);expect(check(m,4,'Raw sell condition').passed).toBe(true);expect(check(model([10,10,10],id,{fast:1,slow:2}),2,'Raw buy crossover').passed).toBe(false);}
  expect(point(model([.1,.2,.3,.3,.3],'hbqts_ma5_25',{fast:2,slow:3}),'bandy-fast',4)).toBe((.3+.3)/2);
 });
 it('does not restart BarIndex, BarsSince or ExRem at selected From',()=>{
  const m=model(Array(14).fill(100),'hbqts_fixed_interval',{every:6,hold:3});m.dates=m.dates.slice(4);
  expect(check(m,4,'Raw interval entry').passed).toBe(false);expect(check(m,6,'Raw interval entry').passed).toBe(true);expect(check(m,9,'Formula sell after ExRem').passed).toBe(true);expect(check(m,10,'Raw sell condition').passed).toBe(true);expect(check(m,10,'Formula sell after ExRem').passed).toBe(false);expect(check(m,3,'Source position before close').value).toBeNull();
  expect(check(model(Array(14).fill(100),'hbqts_fixed_interval',{every:3,hold:4}),10,'Raw sell condition').passed).toBe(false);
 });
 it('uses repeated raw crosses to restart the clock before sequential ExRem',()=>{
  const m=model([10,9,8,10,9,11,10,9,8,7],'hbqts_entry_id1',{fast:1,slow:2,hold:3});m.trades=[trade(m,3,8)];
  expect(check(m,3,'Formula buy after ExRem').passed).toBe(true);expect(check(m,5,'Raw buy crossover').passed).toBe(true);expect(check(m,5,'Formula buy after ExRem').passed).toBe(false);expect(check(m,6,'Raw sell condition').passed).toBe(false);expect(check(m,8,'Formula sell after ExRem').passed).toBe(true);expect(point(m,'bandy-bars-since',8)).toBe(3);
 });
 it('is causal and retains full-history seeds when selected dates change',()=>{
  const c=[10,12,11,13,9,9,12,8,14,13,15],m=model(c),full=evidence(m);m.dates=m.dates.slice(7);expect(evidence(m).series.map(s=>s.points)).toEqual(full.series.map(s=>s.points));
  for(let end=5;end<c.length;end++){const prefix=evidence(model(c.slice(0,end+1)));for(const s of prefix.series)expect(s.points).toEqual(full.series.find(f=>f.key===s.key)!.points.slice(0,end+1));}
 });
});
describe('accepted positions and qualified stocks',()=>{
 it('separates raw signal, source position permission and actual accepted fills',()=>{
  const m=model([10,9,8,10,9,11,10,9,8],'hbqts_entry_id1',{fast:1,slow:2,hold:3});m.trades=[trade(m,3,8)];
  expect(check(m,3,'Source position before close').value).toBe(0);expect(check(m,3,'Source entry permitted').passed).toBe(true);expect(check(m,3,'Recorded entry fill').passed).toBe(true);expect(check(m,5,'Source position before close').value).toBe(1);expect(check(m,5,'Source entry permitted').passed).toBe(false);expect(check(m,5,'Recorded entry fill').passed).toBe(false);expect(check(m,8,'Source exit permitted').passed).toBe(true);expect(check(m,8,'Recorded exit fill').passed).toBe(true);
  expect(check({...m,trades:[trade(m,3,8,true)]},8,'Recorded exit fill').passed).toBe(false);
 });
 it('does not invent ledger availability or an exit for a terminal open mark',()=>{
  const m=model([10,9,8,10,9],'hbqts_ma5_25',{fast:1,slow:2});m.trades=[trade(m,3,4,true)];m.ledgerAvailable=false;expect(check(m,3,'Source entry permitted').passed).toBeNull();expect(check(m,3,'Recorded entry fill').passed).toBeNull();m.ledgerAvailable=true;expect(check(m,4,'Recorded exit fill').passed).toBe(false);
 });
 it('shows accepted basket exposure without inferring source-flat or controller permission',()=>{
  const m=model([10,9,8,10,9],'hbqts_ma5_25',{fast:1,slow:2});m.focusTicker='basket:us_sp500:AAPL';m.trades=[trade(m,1,4,true,'basket:us_sp500:OTHER')];expect(check(m,3,'Accepted exposure before close').value).toBe(0);expect(check(m,3,'Source entry permitted').passed).toBeNull();expect(check(m,3,'Recorded entry fill').passed).toBe(false);m.trades=[trade(m,1,4,true)];expect(check(m,3,'Accepted exposure before close').value).toBe(1);expect(check(m,3,'Source entry permitted').passed).toBe(false);
 });
 it('refuses instrument substitution, stale identity, sliced prices and unresolved runs',()=>{
  const m=model([10,12,11,13,9]);for(const id of ['hbqts_cmo','hbqts_apz','unknown'])expect(buildBandyRuleEvidence({...m,id},m.prices!)).toBeNull();
  const prices=m.prices as typeof m.prices&{provenance:Record<string,unknown>};for(const asset_type of ['index','etf',undefined])expect(buildBandyRuleEvidence(m,{...prices,provenance:{...prices.provenance,asset_type}} as typeof prices)?.scope).toBe('unavailable');
  expect(buildBandyRuleEvidence({...m,focusTicker:'basket:us_sp500:MSFT'},m.prices!)?.scope).toBe('unavailable');expect(buildBandyRuleEvidence({...m,prices:{...m.prices!,full_history:false}},m.prices!)?.scope).toBe('unavailable');expect(buildBandyRuleEvidence({...m,status:'needs_data'},m.prices!)?.scope).toBe('unavailable');
 });
 it('requires exact book ownership and refuses unknown close-position state',()=>{
  const m=model([10,9,8,10,9],'hbqts_ma5_25',{fast:1,slow:2});
  for(const book of ['stc9','',undefined])expect(buildBandyRuleEvidence({...m,book},m.prices!)?.scope).toBe('unavailable');
  for(const bad of [{...trade(m,3,4),entryKind:'limit'},{...trade(m,3,4),exitPrice:null},{...trade(m,3,4),exits:[{date:m.prices!.d[4],price:9,weight:.5}]},{...trade(m,3,4),legs:[{date:m.prices!.d[3],price:10,weight:.5}]}]){m.trades=[bad];expect(check(m,3,'Source entry permitted').passed).toBeNull();expect(check(m,3,'Recorded entry fill').passed).toBeNull();}
  const original=model([10,9,8,10,9,11,12,13],'hbqts_ma5_25',{fast:1,slow:2});original.trades=[trade(original,1,6),{...trade(original,3,7),key:1}];expect(check(original,3,'Source position before close').value).toBeNull();
 });
 it('withholds missing/nonfinite/excessive parameters or invalid OHLC and handles out-of-range inspection',()=>{
  const m=model([10,12,11,13,9]);for(const parameters of [{rsi:2,lb:2},{rsi:NaN,lb:2,trig:2},{rsi:10001,lb:2,trig:2}] as Record<string,number>[])expect(buildBandyRuleEvidence({...m,parameters},m.prices!)?.scope).toBe('unavailable');expect(buildBandyRuleEvidence(m,{...m.prices!,c:[1]})?.scope).toBe('unavailable');expect(evidence(m).at(-1)).toEqual([]);expect(evidence(m).at(1000)).toEqual([]);
 });
});
