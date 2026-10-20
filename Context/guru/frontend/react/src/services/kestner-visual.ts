/** Selected Kestner overlays display causal backend states, never new indicator calculations. */
import type {IndicatorRequest,PriceHistory,StrategyIndicator,StrategyIndicators} from './strategy-indicators';

const profiles = {
  lkqts_sob: 'sob_close_max80_current_inclusive_channel40_tie_inclusive_strict20',
  lkqts_volbo: 'volbo_current_inclusive_population100_k2',
  lkqts_rsi14: 'rsi_wilder_rma14_level35_65_seed14_flat50',
  lkqts_macd: 'macd_period_alpha_first_close',
  lkqts_macd_literal_alpha: 'macd_literal_alpha_first_close',
} as const;
type Method = keyof typeof profiles;
type ObjectValue = Record<string,unknown>;
const defaults: Record<Method,Readonly<Record<string,number>>> = {
  lkqts_sob:{search:80,chan:40,min:20},
  lkqts_volbo: {sd_n:100,k:2}, lkqts_rsi14:{n:14,lo:35,hi:65},
  lkqts_macd:{fast:12,slow:26,sig:9},lkqts_macd_literal_alpha:{fast:12,slow:26,sig:9},
};
export const isKestnerVisualMethod = (id:string): id is Method => Object.hasOwn(profiles,id);
export interface KestnerLogicalRange {from:number;to:number}
/** Choose visible observed bars within an already validated selected report.
 * All source history remains loaded. Zero means All selected bars; positive
 * counts end at the report's final available daily/weekly/monthly observation.
 * This changes only viewport coordinates, never prices, states or calculations. */
export function kestnerReportLogicalRange(priceDates:readonly string[],reportDates:readonly string[],bars:number):KestnerLogicalRange|null {
  const ordered=(values:readonly string[])=>Array.isArray(values)&&values.length>0&&values.every((d,i)=>date(d)&&(!i||d>values[i-1]));
  if(!ordered(priceDates)||!ordered(reportDates)||!Number.isSafeInteger(bars)||bars<0)return null;
  const firstDate=reportDates[0],lastDate=reportDates[reportDates.length-1];
  const bound=(target:string,after:boolean)=>{
    let lo=0,hi=priceDates.length;
    while(lo<hi){const mid=lo+Math.floor((hi-lo)/2);if(priceDates[mid]<target||(after&&priceDates[mid]===target))lo=mid+1;else hi=mid;}
    return lo;
  };
  const first=bound(firstDate,false),last=bound(lastDate,true)-1;
  if(first>last||first>=priceDates.length||last<0)return null;
  return {from:bars===0?first:Math.max(first,last-bars+1),to:last};
}
function requireValue(condition:unknown,message:string): asserts condition {if(!condition)throw Error(message);}
function object(value:unknown):ObjectValue {
  requireValue(!!value&&typeof value==='object'&&!Array.isArray(value),'A complete backend Kestner report is required.');
  return value as ObjectValue;
}
function finite(value:unknown):value is number {return typeof value==='number'&&Number.isFinite(value);}
function date(value:unknown):value is string {
  return typeof value==='string'&&/^\d{4}-\d{2}-\d{2}$/.test(value)&&Number.isFinite(Date.parse(value))&&new Date(value).toISOString().slice(0,10)===value;
}
function keys(value:ObjectValue,expected:readonly string[]) {
  requireValue(Object.keys(value).length===expected.length&&expected.every(key=>Object.hasOwn(value,key)),'Backend Kestner state fields do not match the fixed profile.');
}
function prices(value:unknown):PriceHistory {
  const p=object(value);
  requireValue(Array.isArray(p.d)&&p.d.length>0,'Actual observed report dates are required.');
  const ds=p.d;
  requireValue(ds.every((d,i)=>date(d)&&(!i||d>ds[i-1])),'Observed dates must be valid, unique and increasing.');
  for(const key of ['o','h','l','c'])requireValue(Array.isArray(p[key])&&p[key].length===ds.length&&(p[key] as unknown[]).every(v=>finite(v)&&v>0),'Actual finite positive OHLC is required.');
  const px=p as unknown as PriceHistory;
  requireValue(ds.every((_,i)=>px.l[i]<=Math.min(px.o[i],px.c[i])&&px.h[i]>=Math.max(px.o[i],px.c[i])),'Report OHLC bounds are inconsistent.');
  return px;
}
function provenance(value:unknown,ticker:string):ObjectValue {
  const p=object(value),parts=ticker.split(':');
  requireValue(parts.length===3&&parts[0]==='basket'&&parts[1]&&parts[2]&&!/[\x00-\x20\x7f]/.test(ticker),'An actual source-qualified single stock is required.');
  requireValue(p.qualified_ticker===ticker&&p.basket===parts[1]&&p.symbol===parts[2]&&p.asset_type==='individual_stock'&&p.timeframe==='D','Kestner report does not identify the selected actual stock.');
  requireValue((p.trusted_market==='in'&&p.currency==='INR')||(p.trusted_market==='us'&&p.currency==='USD'),'Trusted stock market/currency identity is unavailable.');
  for(const key of ['source_sha256','csv_sha256'])requireValue(typeof p[key]==='string'&&/^[a-f0-9]{64}$/.test(p[key] as string),'Source identity hashes are unavailable.');
  return p;
}
function stateValues(id:Method,raw:unknown,index:number,previous:ObjectValue|null):ObjectValue|null {
  const warmup=id==='lkqts_sob'?79:id==='lkqts_volbo'?100:id==='lkqts_rsi14'?14:0;
  if(index<warmup){requireValue(raw===null,'A backend state was supplied before its declared eligibility.');return null;}
  const s=object(raw);
  if(id==='lkqts_sob'){
    keys(s,['profile','high_horizon','low_horizon','statistic','upper_40','lower_40','predicate_ready','long_condition','short_condition']);
    const high=s.high_horizon,low=s.low_horizon,statistic=s.statistic;
    requireValue(s.profile===profiles[id]&&finite(high)&&Number.isSafeInteger(high)&&high>=1&&high<=80&&finite(low)&&Number.isSafeInteger(low)&&low>=1&&low<=80,'SOB horizons must be exact integers in 1..80 for the fixed profile.');
    requireValue(finite(statistic)&&Number.isSafeInteger(statistic)&&statistic>=-80&&statistic<=80&&statistic===(high>low?high:low>high?-low:0)&&(high!==low||high===80),'SOB signed statistic or equal-horizon neutral state is inconsistent.');
    requireValue(typeof s.long_condition==='boolean'&&typeof s.short_condition==='boolean'&&s.predicate_ready===(index>=118),'SOB predicate phase or decision flags are missing or inconsistent.');
    if(index<118){
      requireValue(s.upper_40===null&&s.lower_40===null&&s.long_condition===false&&s.short_condition===false,'SOB channel and predicates must remain unavailable until 40 defined statistics.');
    }else{
      const upper=s.upper_40,lower=s.lower_40;
      requireValue(finite(upper)&&finite(lower)&&Number.isSafeInteger(upper)&&Number.isSafeInteger(lower)&&upper<=80&&lower>=-80&&lower<=statistic&&statistic<=upper,'SOB supplied channel bounds must be exact supported integers containing the statistic.');
      requireValue(s.long_condition===(statistic>20&&statistic===upper)&&s.short_condition===(statistic< -20&&statistic===lower),'SOB strict threshold or inclusive channel-tie predicate disagrees with the supplied state.');
    }
    return s;
  }
  const names=id==='lkqts_volbo'?['delta','mean_change_100','population_variance_100','four_variance','delta_squared']
    :id==='lkqts_rsi14'?['rma_up_14','rma_down_14','rsi_14','long_comparison_13_up','long_comparison_7_down','short_comparison_7_up','short_comparison_13_down']
    :['fast_ema','slow_ema','macd','signal_ema','histogram','previous_histogram','fast_alpha','slow_alpha','signal_alpha'];
  keys(s,['profile',...names,'long_condition','short_condition']);
  requireValue(s.profile===profiles[id]&&typeof s.long_condition==='boolean'&&typeof s.short_condition==='boolean'&&!(s.long_condition&&s.short_condition),'Mixed or contradictory backend profile state.');
  for(const key of names)requireValue(key==='previous_histogram'&&index===0?s[key]===null:finite(s[key]),'Backend indicator state is nonfinite or missing.');
  if(id==='lkqts_volbo'){
    requireValue((s.population_variance_100 as number)>=0&&(s.four_variance as number)>=0&&(s.delta_squared as number)>=0,'Volatility state is outside its supported domain.');
    requireValue(s.four_variance===4*(s.population_variance_100 as number)&&s.delta_squared===(s.delta as number)*(s.delta as number),'Volatility diagnostic fields disagree.');
    requireValue(s.long_condition===((s.delta as number)>0&&(s.delta_squared as number)>(s.four_variance as number))&&s.short_condition===((s.delta as number)<0&&(s.delta_squared as number)>(s.four_variance as number)),'Volatility predicate does not match its backend comparison fields.');
  }else if(id==='lkqts_rsi14'){
    requireValue((s.rma_up_14 as number)>=0&&(s.rma_down_14 as number)>=0&&(s.rsi_14 as number)>=0&&(s.rsi_14 as number)<=100,'RSI state is outside its supported domain.');
    for(const [key,multiple,field] of [['long_comparison_13_up',13,'rma_up_14'],['long_comparison_7_down',7,'rma_down_14'],['short_comparison_7_up',7,'rma_up_14'],['short_comparison_13_down',13,'rma_down_14']] as const)
      requireValue(s[key]===multiple*(s[field] as number),'RSI diagnostic comparison fields disagree.');
    requireValue(s.long_condition===((s.long_comparison_13_up as number)<(s.long_comparison_7_down as number))&&s.short_condition===((s.short_comparison_7_up as number)>(s.short_comparison_13_down as number)),'RSI predicate does not match its backend comparison fields.');
  }else{
    const literal=id==='lkqts_macd_literal_alpha';
    requireValue(s.fast_alpha===(literal ? .15 : 2/13)&&s.slow_alpha===(literal ? .075 : 2/27)&&s.signal_alpha===.2&&(s.fast_ema as number)>0&&(s.slow_ema as number)>0,'MACD alpha/price state does not match the named profile.');
    requireValue(s.macd===(s.fast_ema as number)-(s.slow_ema as number)&&s.histogram===(s.macd as number)-(s.signal_ema as number),'MACD diagnostic components disagree.');
    if(previous)requireValue(s.previous_histogram===previous.histogram,'Previous MACD state does not match the preceding observed report day.');
    if(index===0)requireValue(s.fast_ema===s.slow_ema&&s.macd===0&&s.signal_ema===0&&s.histogram===0&&s.long_condition===false&&s.short_condition===false,'Initial MACD state does not match its declared seed.');
    else requireValue(s.long_condition===((s.previous_histogram as number)<=0&&(s.histogram as number)>0)&&s.short_condition===((s.previous_histogram as number)>=0&&(s.histogram as number)<0),'MACD crossing does not match its backend diagnostic fields.');
  }
  return s;
}

export function buildKestnerIndicators(request:IndicatorRequest):StrategyIndicators {
  const unavailable=(note:string):StrategyIndicators=>({status:'unavailable',series:[],note});
  try{
    requireValue(isKestnerVisualMethod(request.id)&&request.book==='lkqts'&&request.fullHistory&&typeof request.ticker==='string','Exact Kestner book/profile, single stock and complete source prices are required.');
    const id=request.id,settings=object(request.parameters),expected=defaults[id];keys(settings,Object.keys(expected));
    requireValue(Object.entries(expected).every(([k,v])=>settings[k]===v),'Kestner indicators require the exact fixed profile parameters and thresholds.');
    const full=prices(request.prices),fullRaw=object(request.prices);
    requireValue(fullRaw.full_history===true,'Complete source prices are required for exact state identity.');
    const source=object(request.sourceReport);
    requireValue(!source.error&&!source.blocked&&(!Object.hasOwn(source,'readiness')||source.readiness==='ready')&&(!Object.hasOwn(source,'status')||source.status==='ready'),'The selected report is held; no indicator values are invented.');
    requireValue((!Object.hasOwn(source,'id')||source.id===id)&&(!Object.hasOwn(source,'book')||source.book==='lkqts'),'Outer report method identity disagrees with the selected Kestner method.');
    requireValue(source.kind!=='basket'&&!source.portfolio&&!source.source_account&&!source.unit_exposure_report&&!source.pristine_summary,'Other account or portfolio reports cannot supply Kestner indicator states.');
    const report=object(source.kestner_research);
    requireValue(report.id===id&&report.book==='lkqts'&&report.profile===profiles[id]&&typeof report.source_convention==='string'&&report.source_convention.trim().length>0,'Backend report belongs to another method or convention.');
    const reportSource=provenance(source.provenance,request.ticker),fullSource=provenance(fullRaw.provenance,request.ticker);
    const stateSource=provenance(report.provenance,request.ticker);
    requireValue(report.qualified_ticker===request.ticker&&report.ticker===reportSource.symbol,'Backend indicator report belongs to another selected stock.');
    for(const key of ['qualified_ticker','symbol','basket','asset_type','timeframe','trusted_market','currency','source_sha256','csv_sha256'])requireValue(fullSource[key]===reportSource[key]&&stateSource[key]===reportSource[key],'Price history and report belong to different native sources.');
    requireValue(fullRaw.ticker===reportSource.symbol,'Full source ticker disagrees with the report.');
    const selected=prices(source.ohlc);
    requireValue(Array.isArray(source.dates)&&source.dates.length===selected.d.length&&source.dates.every((d,i)=>d===selected.d[i]),'Report OHLC and selected dates disagree.');
    requireValue(Array.isArray(source.close)&&source.close.length===selected.d.length&&source.close.every((v,i)=>v===selected.c[i]),'Report close and OHLC prices disagree.');
    requireValue(Array.isArray(source.equity)&&source.equity.length===selected.d.length&&source.equity.every(v=>finite(v)&&v>0),'A supported positive-equity report is required; no account-held indicators are published.');
    const stats=object(source.stats);requireValue(finite(stats.trades)&&Number.isInteger(stats.trades)&&stats.trades>=0&&finite(stats.net_profit),'A finite completed backend report is required.');
    requireValue(Array.isArray(report.states)&&report.states.length===selected.d.length,'Backend states must cover every selected actual observation.');
    const byDate=new Map(full.d.map((d,i)=>[d,i])),values=new Map<string,ObjectValue|null>();let prior:ObjectValue|null=null,previousIndex:number|null=null;
    for(let i=0;i<selected.d.length;i++){
      const row=object(report.states[i]),index=byDate.get(selected.d[i]);
      requireValue(index!==undefined&&row.source_index===index&&(previousIndex===null||index===previousIndex+1)&&row.date===selected.d[i]&&row.phase==='completed_close'&&row.close===selected.c[i],'State date, close or absolute source index differs from the actual selected observations.');
      for(const key of ['o','h','l','c'] as const)requireValue(selected[key][i]===full[key][index],'Report prices differ from the complete source history.');
      const s=stateValues(id,row.rule_state,index,prior);values.set(selected.d[i],s);prior=s;previousIndex=index;
      if(s&&id==='lkqts_volbo')requireValue(s.delta===full.c[index]-full.c[index-1],'Backend change does not match the actual observed close change.');
      if(s&&index===0&&(id==='lkqts_macd'||id==='lkqts_macd_literal_alpha'))requireValue(s.fast_ema===row.close&&s.slow_ema===row.close,'MACD seed does not match the first actual source close.');
    }
    const display=request.displayDates||full.d;
    requireValue(display.length>0&&display.every((d,i)=>date(d)&&byDate.has(d)&&(!i||d>display[i-1])),'Displayed dates must be ordered actual supplied observations.');
    const add=(key:string,label:string,field:string,kind:StrategyIndicator['kind']='sum',thresholds:StrategyIndicator['thresholds']=[],transform?:(v:number)=>number):StrategyIndicator=>({key,label,kind,placement:'pane',thresholds,points:display.map(time=>{
      const value=values.get(time)?.[field];return {time,value:finite(value)?transform?transform(value):value:null};
    })});
    const note='Daily values are supplied by the actual backend report with its full genuine indicator prefix; no indicator recurrence is recalculated in the page. Values outside the selected report are unavailable. Weekly/monthly candles sample the daily state at their actual final observation. These are declared rule inputs, not original book-performance, portfolio, capital or borrowing certification.';
    if(id==='lkqts_sob')return {status:'ready',series:[add('kestner-sob','Second Order Breakout · supplied signed horizon','statistic','sum',[{value:20,label:'Strict long threshold >20',kind:'reference'},{value:-20,label:'Strict short threshold <−20',kind:'reference'}]),add('kestner-sob-upper','Actual supplied 40-statistic upper bound','upper_40'),add('kestner-sob-lower','Actual supplied 40-statistic lower bound','lower_40')],note:note+' The declared profile uses 80 current-inclusive closes with price ties included; equal high/low horizons are neutral. Its current-inclusive 40-statistic channel permits tied extremes. Signals require strictly >20 or <−20; the book’s Intel example at exactly ±20 conflicts with that formal rule. Statistic values begin at absolute source observation 79, complete channel predicates at 118, and a first possible actual next-open fill at 119. This display supplies no orders, original performance, volatility sizing or borrow certification.'};
    if(id==='lkqts_rsi14')return {status:'ready',series:[add('kestner-rsi','RSI(14) · declared RMA/level profile','rsi_14','rsi',[{value:35,label:'Long below',kind:'entry'},{value:65,label:'Short above',kind:'entry'}])],note:note+' The engine uses strict cross-multiplied comparisons; threshold equality supplies no new entry.'};
    if(id==='lkqts_volbo')return {status:'ready',series:[add('kestner-change','Actual close change · price units','delta'),add('kestner-upper-band','Positive 2σ reference band','population_variance_100','sum',[],v=>2*Math.sqrt(v)),add('kestner-lower-band','Negative 2σ reference band','population_variance_100','sum',[],v=>-2*Math.sqrt(v))],note:note+' Displayed ±2√variance bands are visual references. The exact engine predicate uses signed change² > 4 × population variance, without a comparison epsilon; rounded plotted bands do not decide trades. Original 10000/σ quantities are unsupported.'};
    return {status:'ready',series:[add('kestner-macd',`MACD · ${id==='lkqts_macd'?'period alphas 2/13, 2/27':'literal alphas .15, .075'}`,'macd'),add('macd-signal','Signal · backend .2 profile','signal_ema'),add('kestner-histogram','MACD minus signal · backend state','histogram','sum',[{value:0,label:'Signal-line crossing reference',kind:'reference'}])],note:note+' Both named alpha profiles use the backend first-close seed and local increment convention. A histogram departure across zero is a signal-line crossing, not a MACD-zero crossing.'};
  }catch(error){return unavailable(error instanceof Error?error.message:'The exact backend Kestner states are unavailable.');}
}
