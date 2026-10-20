import {describe,expect,it} from 'vitest';
import {buildStrategyIndicators,type IndicatorRequest} from '../src/services/strategy-indicators';
import {instrumentControls,instrumentTickers} from '../src/services/instrument-controls';
// Synthetic response-shape/decision fixtures only; not source recurrence or market evidence.
type Json=Record<string,any>;
const profile='sob_close_max80_current_inclusive_channel40_tie_inclusive_strict20';
function fixture(start=118,length=3):IndicatorRequest{
 const d=Array.from({length:130},(_,i)=>new Date(Date.UTC(2020,0,i+1)).toISOString().slice(0,10));
 const ticker='basket:us_sp500:AAPL',provenance={qualified_ticker:ticker,symbol:'AAPL',basket:'us_sp500',asset_type:'individual_stock',timeframe:'D',trusted_market:'us',currency:'USD',source_sha256:'a'.repeat(64),csv_sha256:'b'.repeat(64)};
 const prices={ticker:'AAPL',full_history:true,provenance:{...provenance},d,o:d.map((_,i)=>100+i),h:d.map((_,i)=>101+i),l:d.map((_,i)=>99+i),c:d.map((_,i)=>100+i)};
 const selected={d:prices.d.slice(start,start+length),o:prices.o.slice(start,start+length),h:prices.h.slice(start,start+length),l:prices.l.slice(start,start+length),c:prices.c.slice(start,start+length)};
 const sourceReport={id:'lkqts_sob',book:'lkqts',dates:selected.d,close:selected.c,equity:selected.d.map(()=>1),stats:{trades:0,net_profit:0},ohlc:selected,provenance:{...provenance},kestner_research:{id:'lkqts_sob',book:'lkqts',profile,qualified_ticker:ticker,ticker:'AAPL',provenance:{...provenance},source_convention:'Synthetic declared-profile shape fixture, not book-performance evidence.',states:selected.d.map((date:string,i:number)=>{const index=start+i;return {source_index:index,date,phase:'completed_close',close:selected.c[i],rule_state:index<79?null:{profile,high_horizon:80,low_horizon:1,statistic:80,upper_40:index<118?null:80,lower_40:index<118?null:80,predicate_ready:index>=118,long_condition:index>=118,short_condition:false}};})}};
 return {id:'lkqts_sob',book:'lkqts',ticker,prices,fullHistory:true,parameters:{search:80,chan:40,min:20},sourceReport};
}
const report=(r:IndicatorRequest)=>r.sourceReport as Json;
const state=(r:IndicatorRequest,i=0)=>report(r).kestner_research.states[i].rule_state as Json;
const output=(r:IndicatorRequest)=>buildStrategyIndicators(r);
const reject=(r:IndicatorRequest)=>expect(output(r)).toMatchObject({status:'unavailable',series:[]});
describe('proposed SOB supplied-state display',()=>{
 it('routes exact SOB profile and displays supplied integer state/channel without recurrence',()=>{
  const r=fixture(),o=output(r);expect(o.status).toBe('ready');expect(o.series.map(s=>s.points[118].value)).toEqual([80,80,80]);expect(o.series[0].thresholds.map(t=>t.value)).toEqual([20,-20]);expect(o.series[0].points[117].value).toBeNull();expect(o.series[0].points[121].value).toBeNull();expect(o.note).toContain('no indicator recurrence is recalculated');expect(o.note).toContain('Intel example');
 });
 it('allows statistic79 but keeps channel null through117 and uses channel at118',()=>{
  const o=output(fixture(78,42));expect(o.status).toBe('ready');expect(o.series[0].points[78].value).toBeNull();expect(o.series[0].points[79].value).toBe(80);expect(o.series[1].points[117].value).toBeNull();expect(o.series[1].points[118].value).toBe(80);
  const early=fixture(78,1);report(early).kestner_research.states[0].rule_state=state(fixture());reject(early);
  for(const key of ['upper_40','predicate_ready','long_condition']){const r=fixture(117,1);state(r)[key]=key==='upper_40'?80:true;reject(r);}
 });
 it('keeps exactly20 neutral, accepts tied channel extremes and short/flat diagnostics',()=>{
  const r=fixture(118,1),s=state(r);Object.assign(s,{high_horizon:20,low_horizon:1,statistic:20,upper_40:20,lower_40:-5,long_condition:false});expect(output(r).status).toBe('ready');s.long_condition=true;reject(r);
  Object.assign(s,{high_horizon:1,low_horizon:80,statistic:-80,upper_40:-80,lower_40:-80,long_condition:false,short_condition:true});expect(output(r).status).toBe('ready');s.short_condition=false;reject(r);
  Object.assign(s,{high_horizon:80,low_horizon:80,statistic:0,upper_40:0,lower_40:0,short_condition:false});expect(output(r).status).toBe('ready');s.statistic=80;reject(r);
 });
 it('rejects fractional/out-of-domain horizons and channels, missing/extra fields and invented conditions',()=>{
  for(const [key,value] of [['high_horizon',80.1],['low_horizon',0],['statistic',81],['upper_40',79],['lower_40',81],['upper_40',null],['predicate_ready',false],['short_condition',true],['extra',1],['statistic',Infinity]] as const){const r=fixture();state(r)[key]=value;reject(r);}
 });
 it('rejects held/foreign/parameter/source/quote/phase mismatches and unsafe accounts',()=>{
  const mutations:((r:IndicatorRequest)=>void)[]=[r=>{report(r).error='held';},r=>{report(r).equity[0]=0;},r=>{report(r).kestner_research.id='lkqts_macd';},r=>{report(r).kestner_research.profile='other';},r=>{report(r).kestner_research.provenance.csv_sha256='c'.repeat(64);},r=>{report(r).kestner_research.states[0].source_index=117;},r=>{report(r).kestner_research.states[0].phase='open';},r=>{report(r).ohlc.c[0]+=.000000001;},r=>{report(r).kestner_research.states.pop();},r=>{r.parameters={search:80,chan:40,min:19};},r=>{r.parameters={search:80,chan:40,min:20,extra:1};},r=>{r.ticker='SPX';}];
  for(const mutate of mutations){const r=fixture();mutate(r);reject(r);}
 });
 it('scopes actual-stock controls to exactly admitted SOB policy and never enables baskets/index/ETF',()=>{
  for(const policy of [undefined,{allowed_types:['index']},{allowed_types:['individual_stock']},{allowed_types:['individual_stock','index'],execution_scope:'single_symbol_stock_price_research'}])expect(instrumentControls(policy,'lkqts_sob','lkqts')).toMatchObject({stock:false,index:false,basket:false,etf:false});
  const policy={allowed_types:['individual_stock'],execution_scope:'single_symbol_stock_price_research'};expect(instrumentControls(policy,'lkqts_sob','lkqts')).toMatchObject({stock:true,index:false,basket:false,etf:false});expect(instrumentTickers(policy,[{ticker:'SPX'}],[{ticker:'SPY'}],'lkqts_sob','lkqts')).toEqual([]);
 });
 it('keeps exactly minus20 neutral and rejects invented short threshold equality',()=>{
  const r=fixture(118,1);Object.assign(state(r),{high_horizon:1,low_horizon:20,statistic:-20,upper_40:5,lower_40:-20,long_condition:false,short_condition:false});expect(output(r).status).toBe('ready');state(r).short_condition=true;reject(r);
 });
 it('binds method/book/qualified symbol and refuses missing conventions or incomplete channels',()=>{
  for(const mutate of [(r:IndicatorRequest)=>{report(r).kestner_research.book='other';},(r:IndicatorRequest)=>{report(r).kestner_research.qualified_ticker='basket:us_sp500:MSFT';},(r:IndicatorRequest)=>{report(r).kestner_research.source_convention=' ';},(r:IndicatorRequest)=>{delete state(r).lower_40;}]){const r=fixture();mutate(r);reject(r);}
 });
 it('keeps the118 predicate state and119 supplied display separate from an invented order',()=>{
  const r=fixture(118,2),o=output(r);expect(o.status).toBe('ready');expect(o.series[0].points[118].value).toBe(80);expect(o.series[0].points[119].value).toBe(80);expect(Object.keys(o).sort()).toEqual(['note','series','status']);
 });
 it('displays actual selected dates only and rejects a borrowed display date',()=>{
  const r=fixture(118,2);r.displayDates=[report(r).dates[0]];expect(output(r).series.every(s=>s.points.length===1)).toBe(true);r.displayDates=['2099-01-01'];reject(r);
 });

});
