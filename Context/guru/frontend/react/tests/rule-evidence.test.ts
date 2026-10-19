import {afterEach,describe,expect,it,vi} from 'vitest';
import {buildRuleEvidence,chartablePrices,loadBasketRulePrices,makeRuleVisualModel,smoothedAverage,type RuleVisualModel} from '../src/services/rule-evidence';
afterEach(()=>vi.restoreAllMocks());

const days=(n:number)=>Array.from({length:n},(_,i)=>new Date(Date.UTC(2020,0,i+1)).toISOString().slice(0,10));
function model(closes:number[],id='kftsm_exp',parameters:Record<string,number>={period:2,filter:0,lag:0,kind:0}):RuleVisualModel {
  const d=days(closes.length);
  return makeRuleVisualModel({strategy:{id,name:id,rules:['E|Enter on the declared signal','X|Reverse on the opposite signal']},run:{dates:d,trades:[]},prices:{ticker:'TEST',full_history:true,d,o:closes,h:closes.map(v=>v+1),l:closes.map(v=>v*.9),c:closes},parameters,ticker:'TEST',symbol:'TEST'});
}
const values=(m:RuleVisualModel,key:string)=>buildRuleEvidence(m).series.find(s=>s.key===key)!.points.map(p=>p.value);
describe('method-specific visual rule arithmetic',()=>{
  it('uses mean-seeded SmoothedAverage, not a generic first-close EMA',()=>{
    expect(smoothedAverage([10,20,30,10],2)).toEqual([null,15,22.5,16.25]);
    const m=model([10,20,30,10]);
    expect(values(m,'trendline')).toEqual([null,15,22.5,16.25]);
    expect(values(m,'trend-change')).toEqual([null,null,37.5,-100*6.25/30]);
    expect(buildRuleEvidence(m).at(2).map(c=>c.passed)).toEqual([null,true,false,null]);
  });
  it('keeps all three distinct smoothing kinds and full-history seeds',()=>{
    expect(values(model([10,20,30],undefined,{period:2,filter:0,lag:0,kind:1}),'trendline')[2]).toBeCloseTo(230/9,12);
    expect(values(model([10,20,30],undefined,{period:2,filter:0,lag:0,kind:2}),'trendline')[1]).toBeCloseTo(40/3,12);
    const m=model([10,20,30,10,15]);m.dates=m.dates.slice(3);
    expect(values(m,'trendline')[3]).toBe(16.25);
    expect(buildRuleEvidence({...m,prices:{...m.prices!,full_history:false}}).scope).toBe('unavailable');
  });
  it('applies the lag to normalized change and keeps latched direction on a tie',()=>{
    const m=model([10,20,30,30,30,10],undefined,{period:2,filter:10,lag:1,kind:0});
    const e=buildRuleEvidence(m);
    expect(e.at(2)[1].value).toBeNull();
    expect(e.at(3)[1].value).toBe(37.5);
    expect(e.at(3)[3].value).toBe(1);
    // Last lagged change is 6.25%, inside the ±10% band: retain long.
    expect(e.at(5)[1].passed).toBe(false);
    expect(e.at(5)[2].passed).toBe(false);
    expect(e.at(5)[3].value).toBe(1);
    const equality=model([10,20,30],undefined,{period:2,filter:37.5,lag:0,kind:0});
    expect(buildRuleEvidence(equality).at(2)[3].value).toBe(0);
  });
  it('uses literal .0476/.0243/.10 MACD, not a period-based EMA substitution',()=>{
    const m=model([100,110,90],'kfsc_macd',{alpha_fast:.0476,alpha_slow:.0243,alpha_signal:.1});
    const e=buildRuleEvidence(m);
    expect(values(m,'macd')[1]).toBeCloseTo(.233,12);
    expect(values(m,'macd-signal')[1]).toBeCloseTo(.0233,12);
    expect(e.at(1)[1].passed).toBe(true);
    expect(e.at(2)[2].passed).toBe(true);
    expect(buildRuleEvidence({...m,parameters:{...m.parameters,alpha_fast:1/21}}).scope).toBe('unavailable');
  });
  it('does not invent a DSM turn at the first observed slope and carries equality',()=>{
    const closes=[...Array(10).fill(100),110,120,120,90,50];
    const e=buildRuleEvidence(model(closes,'kfsc_dsm',{mom:10,alpha:.0476}));
    expect(e.at(10)[2].value).toBeNull();
    expect(e.at(11)[2].passed).toBe(false);
    expect(e.at(13)[3].passed).toBe(true);
  });
  it('retains the asymmetric percent lower band and sample rather than population SD',()=>{
    const m=model([100,100,100],'kfsc_pct_band',{period:2,pct:5});
    expect(values(m,'lower')[1]).toBe(100/1.05);
    const b=model([10,20,15],'kfsc_bollinger',{period:2,nsd:2});
    expect(values(b,'upper')[1]).toBeCloseTo(15+2*Math.sqrt(50),12);
    expect(buildRuleEvidence(b).at(1)[1].passed).toBe(false);
  });
  it('uses exact SMA directions for the two-average rule, not an average crossover',()=>{
    const e=buildRuleEvidence(model([10,11,12,13],'kfsc_two_ma',{fast:1,slow:2}));
    expect(e.at(2).map(v=>v.value)).toEqual([1,2]);
    expect(e.at(2).every(v=>v.passed)).toBe(true);
  });
  it('matches Burns fresh-window SMA ties and refuses missing execution parameters',()=>{
    const m=model([.1,.2,.3,.3,.3],'sbma_x_10s200s',{fast:2,slow:3,fill:1,always_in:0});
    const e=buildRuleEvidence(m);
    expect(e.series[0].points[4].value).toBe((.3+.3)/2);
    expect(e.note).toContain('next actual open');
    expect(buildRuleEvidence({...m,parameters:{fast:2,slow:3}}).scope).toBe('unavailable');
  });
  it('keeps generic indicator inputs partial and unknown methods unproved',()=>{
    const m=model([10,20,30,10],'eom_rsi',{trend:2,rsi_n:2,exit_rsi:65,rsi_lt:10});
    expect(buildRuleEvidence(m).scope).toBe('indicators');
    expect(buildRuleEvidence(m).at(3).every(v=>v.passed===null)).toBe(true);
    expect(buildRuleEvidence({...m,id:'unreviewed'}).scope).toBe('unavailable');
  });
});
describe('actual source and position mapping',()=>{
  it('keeps source-review flags and full source qualifications separate from arithmetic evidence',()=>{
    const make=(source_rule_reviewed:boolean)=>makeRuleVisualModel({strategy:{id:'x',instrument_policy:{source_rule_reviewed,source_basis:'Individual stock scope',source_convention:'Literal printed constants',limitations:'Hypothetical short execution'},gate:{note:'Original study not reproduced'}}});
    expect(make(true).sourceReview).toContain('Selected source rules reviewed');
    expect(make(false).sourceReview).toContain('not established');
    expect(make(true).sourceQualifications).toEqual(['Individual stock scope','Literal printed constants','Hypothetical short execution']);
    expect(make(true).sourceReview).toContain('Original study not reproduced');
  });
  it('requests only one source-qualified basket constituent and caches only matched full history',async()=>{
    const original=model([10,11,12]),ticker='basket:test:ONE';
    const m=makeRuleVisualModel({strategy:{id:'kftsm_exp'},run:{kind:'basket',dates:original.dates,trades:[{ticker,symbol:'ONE',dir:1,entry:original.dates[0],exit:original.dates[2],legs:[[0,10,1]],exit_px:12}]},basketPriceApi:'/api/in',sourceTickers:[{ticker,provenance:{qualified_ticker:ticker,source_sha256:'source-one',csv_sha256:'csv-one'}}]});
    const body={...original.prices,provenance:{qualified_ticker:ticker,source_sha256:'source-one',csv_sha256:'csv-one'}};
    const fetch=vi.spyOn(window,'fetch').mockResolvedValue(new Response(JSON.stringify(body),{status:200}));
    const first=await loadBasketRulePrices(m,m.trades[0],new AbortController().signal);
    expect(first).toEqual(body);expect(fetch).toHaveBeenCalledTimes(1);
    expect(fetch.mock.calls[0][0]).toBe('/api/in/ohlc?ticker=basket%3Atest%3AONE');
    expect(await loadBasketRulePrices(m,m.trades[0],new AbortController().signal)).toBe(first);
    expect(fetch).toHaveBeenCalledTimes(1);
  });
  it('rejects a mismatched basket source and honors cancellation before a request',async()=>{
    const original=model([10,11,12]),ticker='basket:test:TWO';
    const m=makeRuleVisualModel({strategy:{id:'x'},run:{kind:'basket',dates:original.dates,trades:[{ticker,symbol:'TWO',dir:1,legs:[[0,10,1]]}]},basketPriceApi:'/api/in',sourceTickers:[{ticker,provenance:{qualified_ticker:ticker,source_sha256:'source-two',csv_sha256:'csv-two'}}]});
    const fetch=vi.spyOn(window,'fetch').mockResolvedValue(new Response(JSON.stringify({...original.prices,provenance:{qualified_ticker:ticker,source_sha256:'wrong',csv_sha256:'csv-two'}}),{status:200}));
    await expect(loadBasketRulePrices(m,m.trades[0],new AbortController().signal)).rejects.toThrow('does not match');
    const controller=new AbortController();controller.abort();
    await expect(loadBasketRulePrices(m,m.trades[0],controller.signal)).rejects.toThrow('Cancelled');
    expect(fetch).toHaveBeenCalledTimes(1);
  });
  it('binds native source hashes and ticker; rejects a benchmark masquerading as stock data',()=>{
    const original=model([10,11,12]);
    const make=(prices:unknown)=>makeRuleVisualModel({strategy:{id:'kftsm_exp'},run:{dates:original.dates,provenance:{qualified_ticker:'basket:nifty50:ABC',source_sha256:'pdf',csv_sha256:'csv'}},prices,ticker:'basket:nifty50:ABC'});
    expect(chartablePrices(make({...original.prices,ticker:'NIFTY 50'}))).toBeNull();
    const native={...original.prices,ticker:'ABC',provenance:{qualified_ticker:'basket:nifty50:ABC',source_sha256:'pdf',csv_sha256:'csv'}};
    expect(chartablePrices(make(native))).not.toBeNull();
    expect(chartablePrices(make({...native,provenance:{...native.provenance,csv_sha256:'different'}}))).toBeNull();
  });
  it('maps scale-in legs using selected dates and keeps exit signal separate',()=>{
    const m=makeRuleVisualModel({strategy:{id:'x'},run:{dates:days(4),trades:[{dir:1,entry:'2020-01-01',exit:'2020-01-04',signal:'2020-01-03',legs:[[0,10,.5],[1,11,.5]],exits:[[3,12,1]],exit_px:12,entry_kind:'open',open:false,reason:'Exit'}]}});
    expect(m.trades[0].legs.map(v=>v.date)).toEqual(['2020-01-01','2020-01-02']);
    expect(m.trades[0].signal).toBe('2020-01-03');
  });
  it('does not chart held runs, baskets or invalid candles even if arrays are present',()=>{
    const m=model([10,11,12]);
    expect(chartablePrices({...m,status:'needs_data'})).toBeNull();
    expect(chartablePrices({...m,basket:true})).toBeNull();
    expect(chartablePrices({...m,prices:{...m.prices!,h:[1,2,3]}})).toBeNull();
    expect(buildRuleEvidence({...m,status:'needs_data',blocked:'Source gap'}).note).toBe('Source gap');
  });
});
