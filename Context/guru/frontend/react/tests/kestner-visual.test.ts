import {describe,expect,it} from 'vitest';
import {buildStrategyIndicators,type IndicatorRequest} from '../src/services/strategy-indicators';
import {kestnerReportLogicalRange} from '../src/services/kestner-visual';

// Synthetic response-shape fixtures exercise display/admission only. These are
// not a market backtest, indicator oracle or original-performance evidence.
type Json=Record<string,any>;
const profiles={lkqts_volbo:'volbo_current_inclusive_population100_k2',lkqts_rsi14:'rsi_wilder_rma14_level35_65_seed14_flat50',lkqts_macd:'macd_period_alpha_first_close',lkqts_macd_literal_alpha:'macd_literal_alpha_first_close'};
type Id=keyof typeof profiles;
function fixture(id:Id='lkqts_macd',start=100,length=3):IndicatorRequest {
  const dates:string[]=[];const day=new Date('2020-01-01T00:00:00Z');
  while(dates.length<120){if(day.getUTCDay()!==0&&day.getUTCDay()!==6)dates.push(day.toISOString().slice(0,10));day.setUTCDate(day.getUTCDate()+1);}
  const ticker='basket:us_sp500:AAPL';
  const provenance={qualified_ticker:ticker,symbol:'AAPL',basket:'us_sp500',asset_type:'individual_stock',timeframe:'D',trusted_market:'us',currency:'USD',source_sha256:'a'.repeat(64),csv_sha256:'b'.repeat(64)};
  const prices={ticker:'AAPL',full_history:true,provenance:{...provenance},d:dates,o:dates.map(()=>100.1),h:dates.map(()=>101),l:dates.map(()=>99),c:dates.map(()=>100.1)};
  const selected={d:dates.slice(start,start+length),o:prices.o.slice(start,start+length),h:prices.h.slice(start,start+length),l:prices.l.slice(start,start+length),c:prices.c.slice(start,start+length)};
  const rule=(index:number):Json|null=>{
    if(id==='lkqts_volbo')return index<100?null:{profile:profiles[id],delta:0,mean_change_100:0,population_variance_100:0,four_variance:0,delta_squared:0,long_condition:false,short_condition:false};
    if(id==='lkqts_rsi14')return index<14?null:{profile:profiles[id],rma_up_14:0,rma_down_14:0,rsi_14:50,long_comparison_13_up:0,long_comparison_7_down:0,short_comparison_7_up:0,short_comparison_13_down:0,long_condition:false,short_condition:false};
    const literal=id==='lkqts_macd_literal_alpha';
    return {profile:profiles[id],fast_ema:100.1,slow_ema:100.1,macd:0,signal_ema:0,histogram:0,previous_histogram:index===0?null:0,fast_alpha:literal ? .15 : 2/13,slow_alpha:literal ? .075 : 2/27,signal_alpha:.2,long_condition:false,short_condition:false};
  };
  const report={dates:selected.d,close:selected.c,equity:selected.d.map(()=>1),stats:{trades:0,net_profit:0},ohlc:selected,provenance:{...provenance},
    kestner_research:{id,book:'lkqts',qualified_ticker:ticker,ticker:'AAPL',provenance:{...provenance},profile:profiles[id],source_convention:'Synthetic fixed declared profile; display fixture only.',states:selected.d.map((date,i)=>({source_index:start+i,date,phase:'completed_close',close:selected.c[i],rule_state:rule(start+i)}))}};
  return {id,book:'lkqts',ticker,prices,fullHistory:true,parameters:id==='lkqts_volbo'?{sd_n:100,k:2}:id==='lkqts_rsi14'?{n:14,lo:35,hi:65}:{fast:12,slow:26,sig:9},sourceReport:report};
}
const source=(request:IndicatorRequest)=>request.sourceReport as Json;
const state=(request:IndicatorRequest,index=0)=>source(request).kestner_research.states[index].rule_state as Json;
const result=(request:IndicatorRequest)=>buildStrategyIndicators(request);
const rejects=(request:IndicatorRequest)=>{expect(result(request)).toMatchObject({status:'unavailable',series:[]});};

describe('Kestner chart viewport stays anchored to actual selected report bars',()=>{
  it('anchors tail presets at report end instead of the later full-source tail',()=>{
    const r=fixture(),dates=r.prices!.d,report=source(r).dates;
    expect(kestnerReportLogicalRange(dates,report,2)).toEqual({from:101,to:102});
    expect(kestnerReportLogicalRange(dates,report,1)).toEqual({from:102,to:102});
    expect(kestnerReportLogicalRange(dates,report,756)).toEqual({from:100,to:102});
    expect(kestnerReportLogicalRange(dates,report,0)).toEqual({from:100,to:102});
    expect(dates.length).toBe(120);expect(dates[119]>report[report.length-1]).toBe(true);
  });
  it('distinguishes 3y,1y and All over a long report with a later source tail',()=>{
    const dates=Array.from({length:2000},(_,i)=>new Date(Date.UTC(2000,0,1)+i*86400000).toISOString().slice(0,10));
    const report=dates.slice(100,1501);
    expect(kestnerReportLogicalRange(dates,report,756)).toEqual({from:745,to:1500});
    expect(kestnerReportLogicalRange(dates,report,252)).toEqual({from:1249,to:1500});
    expect(kestnerReportLogicalRange(dates,report,0)).toEqual({from:100,to:1500});
    expect(dates[1999]>report[1400]).toBe(true);
  });
  it('uses only actual dated weekly bars inside the daily report interval',()=>{
    const weekly=['2019-12-27','2020-01-03','2020-01-10','2020-01-17','2020-01-24','2020-01-31'];
    const report=['2020-01-06','2020-01-10','2020-01-17','2020-01-21'];
    expect(kestnerReportLogicalRange(weekly,report,0)).toEqual({from:2,to:3});
    expect(kestnerReportLogicalRange(weekly,report,1)).toEqual({from:3,to:3});
    expect(kestnerReportLogicalRange(weekly,['2020-01-06','2020-01-09'],52)).toBeNull();
    expect(kestnerReportLogicalRange(weekly,['2021-01-04','2021-01-05'],0)).toBeNull();
  });
  it('handles monthly endpoints and clamps a larger preset to actual report coverage',()=>{
    const monthly=['2019-12-31','2020-01-31','2020-02-28','2020-03-31','2020-04-30'];
    expect(kestnerReportLogicalRange(monthly,['2020-01-06','2020-03-17'],36)).toEqual({from:1,to:2});
    expect(kestnerReportLogicalRange(monthly,['2020-01-06','2020-03-17'],1)).toEqual({from:2,to:2});
  });
  it('rejects malformed ordering, dates, empty or non-overlapping bars and invalid presets',()=>{
    const d=['2020-01-03','2020-01-06','2020-01-07'];
    for(const value of [-1,.5,NaN,Infinity])expect(kestnerReportLogicalRange(d,d,value)).toBeNull();
    for(const invalid of [[],['2020-02-30'],[d[1],d[0]],[d[0],d[0]]]){
      expect(kestnerReportLogicalRange(invalid,d,0)).toBeNull();expect(kestnerReportLogicalRange(d,invalid,0)).toBeNull();
    }
    expect(kestnerReportLogicalRange(d,['2019-01-01','2019-01-02'],0)).toBeNull();
  });
});

describe('Kestner indicators use only matched backend report states',()=>{
  it('displays the four fixed backend profiles and does not fabricate prefix/suffix values',()=>{
    for(const id of Object.keys(profiles) as Id[]){
      const request=fixture(id);const output=result(request);expect(output.status).toBe('ready');
      expect(output.series.every(s=>s.placement==='pane')).toBe(true);
      expect(output.series[0].points[99].value).toBeNull();expect(output.series[0].points[100].value).not.toBeNull();expect(output.series[0].points[103].value).toBeNull();
      expect(output.note).toContain('no indicator recurrence is recalculated');
    }
    expect(result(fixture('lkqts_macd')).series[0].label).toContain('2/13, 2/27');
    expect(result(fixture('lkqts_macd_literal_alpha')).series[0].label).toContain('.15, .075');
  });
  it('preserves supplied full-precision values rather than reseeding or recomputing MACD',()=>{
    const request=fixture();const s=state(request);s.fast_ema=100.12345678901235;s.slow_ema=100.1;s.macd=s.fast_ema-s.slow_ema;s.signal_ema=.001234567890123;s.histogram=s.macd-s.signal_ema;s.long_condition=true;
    // A one-observation selected window intentionally has no prior report state.
    const report=source(request);for(const k of ['dates','close','equity'])report[k]=report[k].slice(0,1);for(const k of ['d','o','h','l','c'])report.ohlc[k]=report.ohlc[k].slice(0,1);report.kestner_research.states=report.kestner_research.states.slice(0,1);
    const output=result(request);expect(output.status).toBe('ready');expect(output.series[0].points[100].value).toBe(s.macd);expect(output.series[1].points[100].value).toBe(s.signal_ema);expect(output.series[2].points[100].value).toBe(s.histogram);
  });
  it('shows RSI35/65 references while validating flags from backend cross-multiplied operands',()=>{
    const request=fixture('lkqts_rsi14',14,1);Object.assign(state(request),{rma_up_14:7,rma_down_14:13,rsi_14:34.99999999999999,long_comparison_13_up:91,long_comparison_7_down:91,short_comparison_7_up:49,short_comparison_13_down:169});
    const output=result(request);expect(output.status).toBe('ready');expect(output.series[0].points[14].value).toBe(34.99999999999999);expect(output.series[0].thresholds.map(t=>t.value)).toEqual([35,65]);
    state(request).long_condition=true;rejects(request);
  });
  it('labels volatility bands as visual references and retains the strict squared-predicate basis',()=>{
    const request=fixture('lkqts_volbo',100,1);Object.assign(state(request),{mean_change_100:0,population_variance_100:1,four_variance:4,delta_squared:0});
    const output=result(request);expect(output.status).toBe('ready');expect(output.series[1].points[100].value).toBe(2);expect(output.series[2].points[100].value).toBe(-2);expect(output.note).toContain('change² > 4 × population variance');
    state(request).long_condition=true;rejects(request);
  });
  it('samples only actual source observation dates for weekly/monthly candles',()=>{
    const request=fixture();const d=request.prices!.d;request.displayDates=[d[98],d[100],d[102],d[110]];
    expect(result(request).series[0].points).toEqual([{time:d[98],value:null},{time:d[100],value:0},{time:d[102],value:0},{time:d[110],value:null}]);
    for(const bad of [[d[100],d[100]],[d[102],d[100]],['2020-02-30'],['2020-01-04']])rejects({...request,displayDates:bad});
  });
  it('keeps missing, held, foreign-account and wrong profile reports unavailable',()=>{
    const r=fixture();for(const sourceReport of [undefined,null,{error:'calculation_unavailable'}, {...source(r),blocked:'account held'}, {...source(r),readiness:'needs_data'}, {...source(r),id:'wrong_method'}, {...source(r),portfolio:{}}, {...source(r),unit_exposure_report:{}}])rejects({...r,sourceReport});
    for(const [key,value] of [['id','lkqts_rsi14'],['book','other'],['profile',profiles.lkqts_macd_literal_alpha],['source_convention',''],['qualified_ticker','basket:us_sp500:MSFT']]){const request=fixture();source(request).kestner_research[key]=value;rejects(request);}
    for(const id of ['lkqts_cb4020','lkqts_ma1040','lkqts_mom80'])expect(result({...r,id}).status).toBe('unsupported');
  });
  it('rejects wrong fixed settings, thresholds, nonfinite values and extra parameters',()=>{
    const settings:Record<string,number>[]=[{n:14,lo:36,hi:65},{n:14,lo:35,hi:65,extra:1},{n:14,lo:35,hi:Infinity},{n:14.1,lo:35,hi:65}];
    for(const p of settings)rejects({...fixture('lkqts_rsi14'),parameters:p});
    rejects({...fixture(),parameters:{fast:12,slow:26,sig:9.0001}});rejects({...fixture(),book:'other'});rejects({...fixture(),fullHistory:false});
  });
  it('rejects source borrowing across instruments, markets and price identities',()=>{
    for(const [key,value] of [['qualified_ticker','basket:us_sp500:MSFT'],['asset_type','index'],['trusted_market','in'],['csv_sha256','c'.repeat(64)],['source_sha256','invalid']]){
      const request=structuredClone(fixture());(request.prices as unknown as Json).provenance[key]=value;rejects(request);
    }
    rejects({...fixture(),ticker:'SPX'});const r=structuredClone(fixture());source(r).kestner_research.provenance.symbol='MSFT';rejects(r);
  });
  it('rejects altered dates, prices, source indices and report ranges',()=>{
    const mutations:((r:IndicatorRequest)=>void)[]=[r=>source(r).kestner_research.states[0].source_index=99,r=>source(r).kestner_research.states[1].date=source(r).dates[0],r=>source(r).kestner_research.states[0].phase='open',r=>source(r).kestner_research.states[0].close+=.000000001,r=>source(r).close[0]=90,r=>source(r).ohlc.c[0]=90,r=>source(r).equity[0]=0,r=>source(r).stats.net_profit=null,r=>source(r).kestner_research.states.pop()];
    for(const mutate of mutations){const request=structuredClone(fixture());mutate(request);rejects(request);}
  });
  it('rejects contradictory, mixed, ineligible and nonfinite backend states without predicate epsilon',()=>{
    const bad:((r:IndicatorRequest)=>void)[]=[r=>state(r).profile=profiles.lkqts_macd_literal_alpha,r=>state(r).fast_alpha=.15,r=>state(r).histogram=Number.NaN,r=>state(r).previous_histogram=null,r=>state(r,1).previous_histogram=1,r=>state(r).long_condition=true,r=>state(r).macd=1,r=>state(r).unexpected=1];
    for(const mutate of bad){const r=fixture();mutate(r);rejects(r);}
    const early=fixture('lkqts_rsi14',13,1);source(early).kestner_research.states[0].rule_state=state(fixture('lkqts_rsi14'));rejects(early);
    const seed=fixture('lkqts_macd',0,2);expect(result(seed).status).toBe('ready');state(seed).previous_histogram=0;rejects(seed);
    const overflow=fixture('lkqts_volbo');state(overflow).population_variance_100=Infinity;rejects(overflow);
  });
});
