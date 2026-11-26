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

export interface RecordedRuleEvent {date:string;price:number|null;label:string;kind:'entry'|'exit'|'add'|'mark';arrow?:boolean;note?:string}
export function recordedTradeEvents(trade:VisualTrade):RecordedRuleEvent[]{
  const entry=trade.legs.map((leg,i)=>({date:leg.date,price:leg.price,label:i?(trade.direction>0?'Buy more · scale-in fill':'Add short · scale-in fill'):(trade.direction>0?'Buy · entry fill':'Sell short · entry fill'),kind:i?'add' as const:'entry' as const}));
  const pieces=[...trade.exits];
  // Trade.exits includes the final piece. For an open scale-out position, several
  // engines append a synthetic terminal mark tuple for the remaining fraction;
  // exit_px is then a weighted average, not an actual final fill/mark price.
  const terminalMark=trade.open&&pieces.at(-1)?.date===trade.exit?pieces.pop():undefined;
  const exits:RecordedRuleEvent[]=pieces.map((leg,i)=>{
    const final=!trade.open&&i===pieces.length-1&&leg.date===trade.exit;
    const ambiguousFinalDate=trade.open&&leg.date===trade.exit;
    return {date:leg.date,price:leg.price,label:`${trade.direction>0?'Sell':'Buy back'} · ${final?'final':'partial'} exit fill`,kind:'exit',arrow:!ambiguousFinalDate,note:ambiguousFinalDate?'Reported scale-out on the terminal date. Intraday ordering is not established; no terminal exit arrow is inferred.':undefined};
  });
  if(trade.open)exits.push({date:trade.exit,price:terminalMark?.price??(trade.exits.length?null:trade.exitPrice),label:'Open position mark · not an exit',kind:'mark'});
  else if(!trade.exits.length)exits.push({date:trade.exit,price:trade.exitPrice,label:trade.direction>0?'Sell · exit fill':'Buy back · exit fill',kind:'exit'});
  else if(pieces.at(-1)?.date!==trade.exit)exits.push({date:trade.exit,price:null,label:'Final exit piece unavailable',kind:'mark',note:'The aggregate exit price is not a final fill price.'});
  return [...entry,...exits];
}

function RulePriceChart({model,evidence,trade,onDate,focusDate,containerRef,callouts}: {
  model:RuleVisualModel;evidence:RuleEvidence;trade?:VisualTrade;onDate:(date:string,pinned:boolean)=>void;focusDate:string|null;containerRef:React.RefObject<HTMLDivElement|null>;callouts?:{entry:string;exit:string};
}){
  const [boxes,setBoxes]=useState<{kind:'entry'|'exit';x:number;y:number;text:string}[]>([]);
  const host=useRef<HTMLDivElement>(null), chart=useRef<RuleChart|null>(null),candles=useRef<CandleSeries|null>(null),callbacks=useRef(onDate);
  const [error,setError]=useState('');callbacks.current=onDate;
  const px=chartablePrices(model);
  // Warmup is retained for arithmetic, but only the admitted run's actual dates
  // are painted. Earlier index rows may be close-only warmup, not valid candles.
  const visibleIndices=useMemo(()=>{const dates=new Set(model.dates);return px?.d.flatMap((d,i)=>dates.has(d)?[i]:[])||[];},[model,px]);
  const visibleDates=useMemo(()=>visibleIndices.map(i=>px!.d[i]),[visibleIndices,px]);
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
      instance=library.createChart(element,{...theme,width:element.clientWidth||600,height:320,handleScroll:false,handleScale:false});/* a fixed picture, not a movable chart (user 20 Nov 2026) */chart.current=instance;
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
    const markerEvents=trade?recordedTradeEvents(trade).filter(e=>e.kind!=='mark'&&e.arrow!==false&&px.d.includes(e.date)):[];
    candles.current.setMarkers(markerEvents.map(event=>{
      const buy=event.kind==='exit'?trade!.direction<0:trade!.direction>0;
      return {time:event.date,position:buy?'belowBar':'aboveBar',color:buy?green:red,shape:buy?'arrowUp':'arrowDown',text:event.kind==='add'?'Add':event.kind==='exit'?'Exit':'Entry'};
    }).sort((a,b)=>a.time.localeCompare(b.time)));
    if(trade){const first=visibleDates.indexOf(trade.entry),last=visibleDates.indexOf(trade.exit);if(first>=0)chart.current.timeScale().setVisibleLogicalRange({from:Math.max(0,first-18),to:Math.min(visibleDates.length-1,Math.max(first+24,last+12))});}
    // Rule callouts joined to the recorded entry and exit fills (positions follow pan/zoom; skipped when the chart cannot map them).
    const scale=chart.current.timeScale() as unknown as {timeToCoordinate?:(t:string)=>number|null;subscribeVisibleLogicalRangeChange?:(cb:()=>void)=>void;unsubscribeVisibleLogicalRangeChange?:(cb:()=>void)=>void};
    const series=candles.current as unknown as {priceToCoordinate?:(p:number)=>number|null};
    const place=()=>{
      if(!trade||!callouts||typeof scale.timeToCoordinate!=='function'||typeof series.priceToCoordinate!=='function'){setBoxes([]);return;}
      const exitPrice=trade.exits.at(-1)?.price??trade.exitPrice;
      const pts=[{kind:'entry' as const,date:trade.entry,price:trade.legs[0]?.price??null,text:callouts.entry},...(trade.open?[]:[{kind:'exit' as const,date:trade.exit,price:exitPrice,text:callouts.exit}])];
      setBoxes(pts.flatMap(p=>{const x=scale.timeToCoordinate!(p.date),y=p.price==null?null:series.priceToCoordinate!(p.price);return x==null||y==null||!p.text?[]:[{kind:p.kind,x,y,text:p.text}];}));
    };
    const frame=requestAnimationFrame(place);scale.subscribeVisibleLogicalRangeChange?.(place);
    return()=>{cancelAnimationFrame(frame);scale.unsubscribeVisibleLogicalRangeChange?.(place);};
  },[trade,px,model,evidence,visibleDates,callouts]);
  useEffect(()=>{
    if(!focusDate||!px||!chart.current)return;const i=visibleDates.indexOf(focusDate);if(i<0)return;
    chart.current.timeScale().setVisibleLogicalRange({from:Math.max(0,i-25),to:Math.min(visibleDates.length-1,i+25)});
  },[focusDate,px,visibleDates]);
  const W=host.current?.clientWidth||600;
  return <div className="rg-chart-wrap"><div className="rule-guide-price" ref={host} aria-label="Actual daily price candles and selected trade fill arrows" data-rule-price-chart />
    {boxes.length>0&&<svg className="rg-links" aria-hidden="true">{boxes.map(b=>{const bx=Math.min(Math.max(b.x,130),W-130),by=b.kind==='entry'?44:270;return <line key={b.kind} x1={bx} y1={by} x2={b.x} y2={b.y} className={'rg-link '+b.kind}/>;})}</svg>}
    {boxes.map(b=><div key={b.kind} className={'rg-callout '+b.kind} style={{left:Math.min(Math.max(b.x,130),W-130),top:b.kind==='entry'?8:270}}><b>{b.kind==='entry'?'Entry':'Exit'}</b> {b.text}</div>)}
    {error&&<p className="rule-guide-warning" role="status">{error}</p>}</div>;
}

const dmy=(d?:string|null)=>d?d.split('-').reverse().join('/'):'—';
const avgPrice=(xs:{price:number;weight:number}[])=>{const w=xs.reduce((a,x)=>a+(x.weight||0),0);return w?xs.reduce((a,x)=>a+x.price*(x.weight||0),0)/w:xs[0]?.price??null;};

// One picture of one recorded trade, like the desk's signal view (user 20 Nov 2026): the trade's facts, the candles
// around it, the entry and exit fills with the rule that fired, and the plotted indicator lines. Nothing is sampled
// or invented: a held selection shows its rules and no chart.
function RuleGuideBody({model,onClose,id}: {model:RuleVisualModel;onClose:()=>void;id:string}){
  const root=useRef<HTMLDivElement>(null);
  const latest=(()=>{for(let i=model.trades.length-1;i>=0;i--)if(!model.trades[i].open)return i;return Math.max(0,model.trades.length-1);})();
  const [tradeIndex,setTradeIndex]=useState(latest),[hoverDate,setHoverDate]=useState<string|null>(null);
  const [basketPrice,setBasketPrice]=useState<{ticker:string;prices:RuleVisualModel['prices'];note:string;loading:boolean}>({ticker:'',prices:null,note:'',loading:false});
  const trade=model.trades[tradeIndex];
  useEffect(()=>{
    if(!model.basket||model.status!=='ready'||!trade)return;
    const controller=new AbortController(),ticker=trade.ticker,timer=setTimeout(()=>{controller.abort();setBasketPrice({ticker,prices:null,note:'The one-stock price request reached its time limit.',loading:false});},30000);
    setBasketPrice({ticker,prices:null,note:'Loading one actual constituent price history…',loading:true});
    loadBasketRulePrices(model,trade,controller.signal).then(prices=>{if(!controller.signal.aborted)setBasketPrice({ticker,prices,note:'',loading:false});}).catch(error=>{if(!controller.signal.aborted)setBasketPrice({ticker,prices:null,note:error instanceof Error?error.message:'The constituent price history is unavailable.',loading:false});}).finally(()=>clearTimeout(timer));
    return()=>{clearTimeout(timer);controller.abort();};
  },[model,trade?.ticker]);
  const chartModel=useMemo(()=>!model.basket?model:{...model,symbol:trade?.symbol||model.symbol,focusTicker:trade?.ticker,basket:false,prices:basketPrice.ticker===trade?.ticker?basketPrice.prices:null,blocked:basketPrice.ticker===trade?.ticker?basketPrice.note:'Loading the traded stock.'},[model,trade?.ticker,trade?.symbol,basketPrice]);
  const evidence=useMemo(()=>buildRuleEvidence(chartModel),[chartModel]),px=chartablePrices(chartModel);
  const clip=(t:string)=>t.length>120?t.slice(0,117).replace(/\s+\S*$/,'')+'…':t;
  const callouts=useMemo(()=>({entry:clip(model.rules.filter(r=>r.stage==='Entry').map(r=>r.text).join(' · ')||model.rules.filter(r=>r.stage==='Conditions').map(r=>r.text).join(' · ')),exit:clip(model.rules.filter(r=>r.stage==='Exit').map(r=>r.text).join(' · '))}),[model]);
  const isReady=model.status==='ready';
  const entryAvg=trade?avgPrice(trade.legs):null,exitAvg=!trade||trade.open?null:trade.exits.length?avgPrice(trade.exits):trade.exitPrice;
  const move=entryAvg&&exitAvg?trade!.direction*(exitAvg/entryAvg-1):null;
  const shown=hoverDate&&px?.d.includes(hoverDate)&&model.dates.includes(hoverDate)?hoverDate:trade?.entry||'',k=px?px.d.indexOf(shown):-1;
  const go=(n:number)=>{setTradeIndex(n);setHoverDate(null);};
  return <div className="rule-guide-body rg-min" ref={root}>
    <header className="rule-guide-header"><div><h2 id={`${id}-title`}>{model.name}</h2><p id={`${id}-description`}>{model.basket&&trade?`${model.symbol} · ${trade.symbol}`:model.symbol}{trade?` · ${trade.direction>0?'Long':'Short'} trade ${tradeIndex+1} of ${model.trades.length}`:''}</p></div>
      <div className="rg-head-tools">{isReady&&model.trades.length>1&&<span className="rg-step"><button type="button" aria-label="Previous trade" disabled={tradeIndex<=0} onClick={()=>go(tradeIndex-1)}>‹</button><button type="button" aria-label="Next trade" disabled={tradeIndex>=model.trades.length-1} onClick={()=>go(tradeIndex+1)}>›</button></span>}<button className="rule-guide-close" aria-label="Close visual rules" onClick={onClose}>×</button></div></header>
    {!isReady||!trade?<section className="rule-guide-empty"><strong>{isReady?'No trades in this selection':'No backtest chart for this selection'}</strong><p>{isReady?'The rules below never fired on the selected instrument and dates.':model.blocked||'Required rules or market data are unresolved.'}</p><ol className="rg-rules">{model.rules.map((rule,i)=><li key={i}><b>{rule.stage}</b> {rule.text}</li>)}</ol></section>:<>
      <dl className="rg-facts"><div><dt>Entry Date</dt><dd>{dmy(trade.entry)}</dd></div><div><dt>Entry Price</dt><dd>{format(entryAvg)}</dd></div><div><dt>{trade.open?'Marked':'Exit Date'}</dt><dd>{dmy(trade.exit)}</dd></div><div><dt>Exit Price</dt><dd>{trade.open?'Open':format(exitAvg)}</dd></div><div><dt>Price move</dt><dd className={move==null?'':move>=0?'pos':'neg'}>{move==null?'—':`${move>=0?'+':''}${(move*100).toFixed(1)}%`}</dd></div></dl>
      <div className="rg-frame">
        {px?<><div className="rg-ohlc"><b>{chartModel.symbol}</b><span>Daily · {dmy(shown)}</span>{k>=0&&(['o','h','l','c'] as const).map((key,i)=><span key={key}>{'OHLC'[i]} {format(px[key][k])}</span>)}</div>
          <RulePriceChart model={chartModel} evidence={evidence} trade={trade} onDate={d=>setHoverDate(d)} focusDate={null} containerRef={root} callouts={callouts}/></>
        :<div className="rule-guide-empty" role="status"><strong>{basketPrice.loading?'Loading the traded stock':'Price chart unavailable'}</strong><p>{evidence.note}</p></div>}
      </div>
    </>}
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
  return <><button type="button" ref={trigger} className="rule-guide-trigger" aria-label="Visual explanation of strategy rules" title="See the rules on the chart" aria-haspopup="dialog" aria-expanded={open} aria-controls={open?`${id}-dialog`:undefined} onClick={()=>setOpen(true)}><svg width="19" height="19" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.7" aria-hidden="true"><circle cx="12" cy="12" r="9"/><path d="M12 10v7"/><circle cx="12" cy="7" r=".7" fill="currentColor"/></svg><span className="rule-guide-trigger-label">Rules</span></button>{open&&createPortal(<div className="rule-guide-backdrop" data-rule-guide-backdrop onClick={e=>{if(e.target===e.currentTarget)setOpen(false);}}><div ref={dialog} id={`${id}-dialog`} className="rule-guide-dialog" role="dialog" aria-modal="true" aria-labelledby={`${id}-title`} aria-describedby={`${id}-description`} tabIndex={-1}><RuleGuideBody model={model} id={id} onClose={()=>setOpen(false)}/></div></div>,document.body)}</>;
}
