import {useEffect, useId, useMemo, useRef, useState} from 'react';
import {createPortal} from 'react-dom';
import {buildRuleEvidence, chartablePrices, loadBasketRulePrices, type RuleEvidence, type RuleVisualModel, type VisualTrade} from '../services/rule-evidence';
import {mountStrategyIndicators, type IndicatorChart, type IndicatorChartLibrary} from '../services/strategy-indicator-charts';
import {StrategyIndicatorControls} from './StrategyIndicatorControls';
import './RulesVisualGuide.css';

interface CandleSeries {
  setData(data: {time:string;open:number;high:number;low:number;close:number}[]): void;
  setMarkers(markers: {time:string;position:string;color:string;shape:string;text:string}[]): void;
}
interface RuleChart extends IndicatorChart {
  addCandlestickSeries(options: Record<string,unknown>): CandleSeries;
  subscribeClick(callback:(event:{time?: unknown})=>void):void;
  unsubscribeClick(callback:(event:{time?: unknown})=>void):void;
}
interface RuleChartLibrary extends IndicatorChartLibrary {createChart(element:HTMLElement,options:Record<string,unknown>):RuleChart}
const format = (value:number|null|undefined) => value==null?'Not available':value.toLocaleString('en-IN',{maximumFractionDigits:6});
const timeString=(time:unknown):string|null=>typeof time==='string'?time:time&&typeof time==='object'&&'year' in time&&'month' in time&&'day' in time?`${time.year}-${String(time.month).padStart(2,'0')}-${String(time.day).padStart(2,'0')}`:null;

export function recordedTradeEvents(trade:VisualTrade):{date:string;price:number|null;label:string;kind:'entry'|'exit'|'add'|'mark'}[]{
  const entry=trade.legs.map((leg,i)=>({date:leg.date,price:leg.price,label:i?(trade.direction>0?'Buy more · scale-in fill':'Add short · scale-in fill'):(trade.direction>0?'Buy · entry fill':'Sell short · entry fill'),kind:i?'add' as const:'entry' as const}));
  const exits:ReturnType<typeof recordedTradeEvents>=trade.exits.length?trade.exits.map(leg=>({date:leg.date,price:leg.price,label:trade.direction>0?'Sell · partial exit fill':'Buy back · partial exit fill',kind:'exit'})):[];
  if(trade.open)exits.push({date:trade.exit,price:trade.exitPrice,label:'Open position mark · not an exit',kind:'mark'});
  else if(!exits.length||exits.at(-1)?.date!==trade.exit)exits.push({date:trade.exit,price:trade.exitPrice,label:trade.direction>0?'Sell · exit fill':'Buy back · exit fill',kind:'exit'});
  return [...entry,...exits];
}

function RulePriceChart({model,evidence,trade,onDate,focusDate,containerRef}: {
  model:RuleVisualModel;evidence:RuleEvidence;trade?:VisualTrade;onDate:(date:string,pinned:boolean)=>void;focusDate:string|null;containerRef:React.RefObject<HTMLDivElement|null>;
}){
  const host=useRef<HTMLDivElement>(null), chart=useRef<RuleChart|null>(null),candles=useRef<CandleSeries|null>(null),callbacks=useRef(onDate);
  const [error,setError]=useState('');callbacks.current=onDate;
  const px=chartablePrices(model);
  // Warmup is retained for arithmetic, but only the admitted run's actual dates
  // are painted. Earlier index rows may be close-only warmup, not valid candles.
  const visibleIndices=useMemo(()=>{const dates=new Set(model.dates);return px?.d.flatMap((d,i)=>dates.has(d)?[i]:[])||[];},[model,px]);
  useEffect(()=>{
    const element=host.current,root=containerRef.current;
    const library=(window as unknown as {LightweightCharts?:RuleChartLibrary}).LightweightCharts;
    if(!element||!root||!px)return;
    if(!library){setError('The chart library could not load. Recorded fills and rule details are still available below.');return;}
    let instance:RuleChart|undefined,indicators:ReturnType<typeof mountStrategyIndicators>|undefined,observer:ResizeObserver|undefined;
    const css=getComputedStyle(element),color=(key:string,fallback:string)=>css.getPropertyValue(key).trim()||fallback;
    const theme={layout:{attributionLogo:false,background:{color:color('--panel','#f6f6f6')},textColor:color('--ink2','#5f6166'),fontFamily:getComputedStyle(document.body).fontFamily,fontSize:11},grid:{vertLines:{visible:false},horzLines:{color:color('--rule','#dcdcdd')}},timeScale:{borderColor:color('--steel','#b9b9ba'),minBarSpacing:.1},rightPriceScale:{borderColor:color('--steel','#b9b9ba')},crosshair:{mode:0}};
    const hover=(event:{time?:unknown})=>{const date=timeString(event.time);if(date)callbacks.current(date,false);};
    const click=(event:{time?:unknown})=>{const date=timeString(event.time);if(date)callbacks.current(date,true);};
    try{
      instance=library.createChart(element,{...theme,width:element.clientWidth||600,height:320});chart.current=instance;
      const series=instance.addCandlestickSeries({upColor:color('--green','#2e7d4f'),downColor:color('--red','#c43c3c'),borderVisible:false,wickUpColor:color('--green','#2e7d4f'),wickDownColor:color('--red','#c43c3c'),priceLineVisible:false});candles.current=series;
      series.setData(visibleIndices.map(i=>({time:px.d[i],open:px.o[i],high:px.h[i],low:px.l[i],close:px.c[i]})));
      instance.subscribeCrosshairMove(hover);instance.subscribeClick(click);
      const visibleDates=new Set(model.dates);
      indicators=mountStrategyIndicators({root,chart:instance,priceSeries:series,result:{status:evidence.series.length?'ready':'unsupported',series:evidence.series.map(s=>({...s,points:s.points.filter(point=>visibleDates.has(point.time))})),note:evidence.note},library,chartOptions:theme,pricePoints:visibleIndices.map(i=>({time:px.d[i],value:px.c[i]}))});
      observer=typeof ResizeObserver==='function'?new ResizeObserver(()=>{if(element.isConnected)instance?.resize(element.clientWidth||600,320);}):undefined;observer?.observe(element);
      instance.timeScale().setVisibleLogicalRange({from:0,to:Math.min(visibleIndices.length-1,100)});
    }catch{setError('This chart could not be displayed. No prices or condition matches have been invented.');}
    return()=>{observer?.disconnect();indicators?.dispose();if(instance){instance.unsubscribeCrosshairMove(hover);instance.unsubscribeClick(click);instance.remove();}chart.current=null;candles.current=null;};
  },[model,evidence,px,containerRef,visibleIndices]);
  useEffect(()=>{
    if(!chart.current||!candles.current||!px)return;
    const css=getComputedStyle(host.current!),green=css.getPropertyValue('--green').trim()||'#2e7d4f',red=css.getPropertyValue('--red').trim()||'#c43c3c';
    const markerEvents=trade?recordedTradeEvents(trade).filter(e=>e.kind!=='mark'&&px.d.includes(e.date)):[];
    candles.current.setMarkers(markerEvents.map(event=>{
      const buy=event.kind==='exit'?trade!.direction<0:trade!.direction>0;
      return {time:event.date,position:buy?'belowBar':'aboveBar',color:buy?green:red,shape:buy?'arrowUp':'arrowDown',text:event.kind==='add'?'Add':event.kind==='exit'?'Exit':'Entry'};
    }).sort((a,b)=>a.time.localeCompare(b.time)));
    if(trade){const first=model.dates.indexOf(trade.entry),last=model.dates.indexOf(trade.exit);if(first>=0)chart.current.timeScale().setVisibleLogicalRange({from:Math.max(0,first-18),to:Math.min(visibleIndices.length-1,Math.max(first+24,last+12))});}
  },[trade,px,model,evidence]);
  useEffect(()=>{
    if(!focusDate||!px||!chart.current)return;const i=model.dates.indexOf(focusDate);if(i<0)return;
    chart.current.timeScale().setVisibleLogicalRange({from:Math.max(0,i-25),to:Math.min(model.dates.length-1,i+25)});
  },[focusDate,px,model]);
  return <><div className="rule-guide-price" ref={host} aria-label="Actual daily price candles and selected trade fill arrows" data-rule-price-chart />{error&&<p className="rule-guide-warning" role="status">{error}</p>}</>;
}

function RuleGuideBody({model,onClose,id}: {model:RuleVisualModel;onClose:()=>void;id:string}){
  const root=useRef<HTMLDivElement>(null);
  const [tradeIndex,setTradeIndex]=useState(0),[selectedDate,setSelectedDate]=useState(model.trades[0]?.entry||model.dates[0]||''),[pinned,setPinned]=useState(false),[focusDate,setFocusDate]=useState<string|null>(null);
  const [basketPrice,setBasketPrice]=useState<{ticker:string;prices:RuleVisualModel['prices'];note:string;loading:boolean}>({ticker:'',prices:null,note:'',loading:false});
  const selectedTrade=model.trades[tradeIndex];
  useEffect(()=>{
    if(!model.basket||model.status!=='ready'||!selectedTrade)return;
    const controller=new AbortController(),ticker=selectedTrade.ticker,timer=setTimeout(()=>{controller.abort();setBasketPrice({ticker,prices:null,note:'The one-stock price request reached its time limit. Recorded fills remain readable.',loading:false});},30000);
    setBasketPrice({ticker,prices:null,note:'Loading one actual constituent price history…',loading:true});
    loadBasketRulePrices(model,selectedTrade,controller.signal).then(prices=>{if(!controller.signal.aborted)setBasketPrice({ticker,prices,note:'',loading:false});}).catch(error=>{if(!controller.signal.aborted)setBasketPrice({ticker,prices:null,note:error instanceof Error?error.message:'The constituent price history is unavailable.',loading:false});}).finally(()=>clearTimeout(timer));
    return()=>{clearTimeout(timer);controller.abort();};
  },[model,selectedTrade?.ticker]);
  const chartModel=useMemo(()=>!model.basket?model:{...model,symbol:selectedTrade?.symbol||model.symbol,basket:false,prices:basketPrice.ticker===selectedTrade?.ticker?basketPrice.prices:null,blocked:basketPrice.ticker===selectedTrade?.ticker?basketPrice.note:'Choose a recorded position to inspect its own stock prices.'},[model,selectedTrade?.ticker,selectedTrade?.symbol,basketPrice]);
  const evidence=useMemo(()=>buildRuleEvidence(chartModel),[chartModel]),px=chartablePrices(chartModel);
  const trade=model.trades[tradeIndex],sourceIndex=model.dates.includes(selectedDate)?px?.d.indexOf(selectedDate)??-1:-1,checks=evidence.at(sourceIndex);
  // Keep very large basket trade lists accessible without creating thousands of
  // option DOM nodes on each candle hover. Numeric jump reaches every position.
  const choiceStart=Math.max(0,tradeIndex-20),tradeChoices=model.trades.slice(choiceStart,tradeIndex+21);
  const isReady=model.status==='ready';
  const selectDate=(date:string,pin:boolean)=>{if(pin||!pinned){setSelectedDate(date);if(pin)setPinned(true);}};
  const jump=(date:string)=>{setSelectedDate(date);setPinned(true);setFocusDate(date);};
  const chooseTrade=(next:number)=>{const t=model.trades[next];setTradeIndex(next);if(t){setSelectedDate(t.entry);setPinned(true);setFocusDate(null);}};
  return <div className="rule-guide-body" ref={root}>
    <header className="rule-guide-header"><div><span className="rule-guide-eyebrow">RULES, EXPLAINED</span><h2 id={`${id}-title`}>{model.name}</h2><p id={`${id}-description`}>Follow a recorded trade, then inspect the rule inputs on each candle.</p></div><button className="rule-guide-close" aria-label="Close visual rules" onClick={onClose}>×</button></header>
    <div className="rule-guide-context"><span>{model.basket&&selectedTrade?`${model.symbol} · ${selectedTrade.symbol}`:model.symbol||'Selected instrument'}</span><span>{isReady?'Actual local data':'Results withheld'}</span><span data-rule-evidence-scope={evidence.scope}>{evidence.scope==='exact'?'Method-specific calculations':evidence.scope==='indicators'?'Partial indicator inputs':'Rule trace unavailable'}</span></div>
    <p className="rule-guide-review-notice">{model.sourceReview}</p>
    {!isReady?<section className="rule-guide-empty"><strong>No backtest chart for this selection</strong><p>{model.blocked||'Required rules or market data are unresolved.'}</p><p>The book rules below remain readable. A sample chart would imply a trade that has not been calculated, so none is shown.</p></section>:<>
      <div className="rule-guide-navigation"><div className="rule-guide-trade-select"><label htmlFor={`${id}-trade`}>{model.basket?'Recorded basket position':'Follow a trade'}</label><select id={`${id}-trade`} value={tradeIndex} disabled={!model.trades.length} onChange={e=>chooseTrade(Number(e.target.value))}>{model.trades.length?tradeChoices.map((t,i)=><option value={choiceStart+i} key={t.key}>{choiceStart+i+1}. {model.basket?t.symbol+' · ':''}{t.direction>0?'Long':'Short'} · {t.entry} → {t.open?'Open':t.exit}</option>):<option>No trades in the selected period</option>}</select></div><div className="rule-guide-paging"><button disabled={tradeIndex===0||!trade} onClick={()=>chooseTrade(tradeIndex-1)} aria-label="Previous recorded trade">←</button><span>{model.trades.length?`${tradeIndex+1} / ${model.trades.length}`:'0 trades'}</span><button disabled={tradeIndex>=model.trades.length-1} onClick={()=>chooseTrade(tradeIndex+1)} aria-label="Next recorded trade">→</button></div>{model.trades.length>41&&<form className="rule-guide-jump" key={tradeIndex} onSubmit={event=>{event.preventDefault();const n=Number(new FormData(event.currentTarget).get('trade'));if(Number.isInteger(n)&&n>=1&&n<=model.trades.length)chooseTrade(n-1);}}><label>Trade number <input type="number" name="trade" min="1" max={model.trades.length} step="1" defaultValue={tradeIndex+1} required/></label><button type="submit">Go</button></form>}</div>
      <div className="rule-guide-workbench"><div className="rule-guide-chart-column">
        {px?<><div className="rule-guide-chart-heading"><strong>Read the price, then the condition</strong><span>↑ Buy / cover &nbsp; ↓ Sell / short</span></div><div className="rule-guide-chart-frame"><RulePriceChart model={chartModel} evidence={evidence} trade={trade} onDate={selectDate} focusDate={focusDate} containerRef={root}/>{sourceIndex>=0&&<div className="rule-guide-chart-tooltip" role="tooltip"><strong>{selectedDate}</strong><span>Close {format(px.c[sourceIndex])}</span>{trade&&recordedTradeEvents(trade).filter(e=>e.date===selectedDate).map((e,i)=><b key={i}>{e.label} · {format(e.price)}</b>)}{checks.filter(c=>c.passed===true).map((c,i)=><small key={i}>✓ {c.label}</small>)}</div>}</div><p className="rule-guide-chart-help">Drag or scroll to explore · hover a candle to inspect · click to pin its values. Arrows mark recorded fills, not proof that all book rules are verified.</p><StrategyIndicatorControls result={{status:evidence.series.length?'ready':'unsupported',series:evidence.series,note:''}}/>
          <label className="rule-guide-date">Inspect candle <input type="date" min={model.dates[0]} max={model.dates.at(-1)} value={selectedDate} onChange={e=>jump(e.target.value)}/><button type="button" onClick={()=>setPinned(false)} disabled={!pinned}>Resume hover</button></label>
        </>:<div className="rule-guide-empty" role="status"><strong>{basketPrice.loading?'Loading selected stock':model.basket?'Selected stock price chart unavailable':'Actual candle data unavailable'}</strong><p>{evidence.note}</p><p>Recorded fills below remain available. A benchmark or another stock is never substituted for the traded instrument.</p></div>}
        {trade?<div className="rule-guide-events" aria-label="Recorded trade fills"><div className="rule-guide-event-title"><strong>{trade.direction>0?'Long':'Short'} position{model.basket?' · '+trade.symbol:''}</strong><small>{trade.entryKind==='open'?'Entry recorded at an open. Entry signal time is not supplied in this trade payload.':trade.entryKind==='close'?'Entry recorded at the close.':'Entry execution: '+(trade.entryKind||'not specified')}</small></div>{recordedTradeEvents(trade).map((event,i)=><button key={`${event.date}-${i}`} className={`rule-guide-event ${event.kind}`} disabled={!px||!px.d.includes(event.date)} onClick={()=>jump(event.date)}><span className="rule-guide-event-number">{event.kind==='entry'?'↗':event.kind==='add'?'+':event.kind==='mark'?'○':'↘'}</span><span><strong>{event.label}</strong><small>{event.date} · {format(event.price)}</small></span></button>)}{!trade.open&&trade.signal&&<p className="rule-guide-fill-note">Exit signal: <button className="rule-guide-text-button" disabled={!px||!px.d.includes(trade.signal)} onClick={()=>jump(trade.signal)}>{trade.signal}</button> · Recorded exit reason: {trade.reason||'not supplied'}. Signal and fill can occur on different candles.</p>}{trade.open&&<p className="rule-guide-fill-note">This position is still open at the last supplied mark. No exit arrow or completed exit condition is inferred.</p>}</div>:<p className="rule-guide-warning">No positions were recorded for this window. You can still inspect available price and indicator values; no example trade is invented.</p>}
      </div><aside className="rule-guide-inspector" aria-label="Selected candle rule inputs"><div className="rule-guide-inspector-heading"><span>{pinned?'PINNED CANDLE':'CANDLE INSPECTOR'}</span><strong>{selectedDate||'Select a candle'}</strong></div>{px&&sourceIndex>=0?<dl className="rule-guide-ohlc">{(['o','h','l','c'] as const).map((key,i)=><div key={key}><dt>{['Open','High','Low','Close'][i]}</dt><dd>{format(px[key][sourceIndex])}</dd></div>)}</dl>:px?<p>No actual source candle on this date. Missing sessions are not interpolated.</p>:null}
        <h3>{evidence.title}</h3>{checks.map((check,i)=><div className="rule-guide-condition" key={i} data-condition-result={check.passed===null?'context':check.passed?'met':'not-met'}><div><strong>{check.label}</strong><span className="rule-guide-check-state">{check.value===null?'Warmup / unavailable':check.passed===null?'Value only':check.passed?'Met':'Not met'}</span></div><b>{format(check.value)}</b>{check.comparison&&<code>{check.comparison}</code>}<p>{check.explanation}</p></div>)}{!checks.length&&<p className="rule-guide-warning">No condition values are certified for this selected candle. Read the source rules below; an arrow alone does not certify their interpretation.</p>}
      </aside></div>
      <p className="rule-guide-evidence-note"><strong>What this view establishes.</strong> {evidence.note} Condition results describe individual predicates; position state, fill timing, sizing and the complete Rules still govern each recorded trade.</p>
    </>}
    <section className="rule-guide-source"><div><span className="rule-guide-eyebrow">THE FULL RULE PATH</span><h3>Keep the complete trading plan in view</h3><p>Source rules and declared implementation choices. These are separate from a book’s reported performance.</p></div><ol>{model.rules.map((rule,i)=><li key={i}><span className="rule-guide-stage">{rule.stage}</span><p>{rule.text}</p></li>)}</ol>{!model.rules.length&&<p>No source rules are registered for this method.</p>}{(model.notes||model.sourceQualifications.length>0)&&<details><summary>Source interpretation, adaptations &amp; implementation notes</summary>{model.sourceQualifications.map((note,i)=><p key={i}>{note}</p>)}{model.notes&&<p>{model.notes}</p>}</details>}</section>
  </div>;
}

/** Every Rules section owns this React control, including held and basket routes. */
export function RulesVisualGuide({model}: {model:RuleVisualModel}){
  const [open,setOpen]=useState(false),id=useId(),trigger=useRef<HTMLButtonElement>(null),dialog=useRef<HTMLDivElement>(null);
  useEffect(()=>{
    if(!open)return;
    const oldOverflow=document.body.style.overflow;document.body.style.overflow='hidden';
    dialog.current?.querySelector<HTMLElement>('[aria-label="Close visual rules"]')?.focus();
    const key=(event:KeyboardEvent)=>{
      if(event.key==='Escape'){event.preventDefault();event.stopPropagation();setOpen(false);return;}
      if(event.key!=='Tab'||!dialog.current)return;
      const controls=[...dialog.current.querySelectorAll<HTMLElement>('button:not(:disabled),select:not(:disabled),input:not(:disabled),summary,[tabindex="0"]')].filter(el=>!el.closest('details:not([open])')||el.tagName==='SUMMARY');
      const first=controls[0],last=controls.at(-1);if(!first||!last)return;
      if(!dialog.current.contains(document.activeElement)||(event.shiftKey&&document.activeElement===first)||(!event.shiftKey&&document.activeElement===last)){event.preventDefault();(event.shiftKey?last:first).focus();}
    };
    document.addEventListener('keydown',key,true);
    return()=>{document.body.style.overflow=oldOverflow;document.removeEventListener('keydown',key,true);if(trigger.current?.isConnected)trigger.current.focus();};
  },[open]);
  return <><button type="button" ref={trigger} className="rule-guide-trigger" aria-label="Visual explanation of strategy rules" title="See the rules on the chart" aria-haspopup="dialog" aria-expanded={open} aria-controls={open?`${id}-dialog`:undefined} onClick={()=>setOpen(true)}><svg width="19" height="19" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.7" aria-hidden="true"><circle cx="12" cy="12" r="9"/><path d="M12 10v7"/><circle cx="12" cy="7" r=".7" fill="currentColor"/></svg><span className="rule-guide-trigger-label">Visual guide</span></button>{open&&createPortal(<div className="rule-guide-backdrop" data-rule-guide-backdrop onClick={e=>{if(e.target===e.currentTarget)setOpen(false);}}><div ref={dialog} id={`${id}-dialog`} className="rule-guide-dialog" role="dialog" aria-modal="true" aria-labelledby={`${id}-title`} aria-describedby={`${id}-description`} tabIndex={-1}><RuleGuideBody model={model} id={id} onClose={()=>setOpen(false)}/></div></div>,document.body)}</>;
}
