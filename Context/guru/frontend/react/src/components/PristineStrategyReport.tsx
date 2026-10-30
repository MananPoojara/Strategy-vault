import {flushSync} from 'react-dom';
import {useEffect, useMemo, useRef, useState, type KeyboardEvent} from 'react';
import {StrategySettings} from './StrategySettings';
import {PristineMonthlyHeatmap} from './PristineMonthlyHeatmap';
import {ResearchMarkup} from './ResearchMarkup';
import {REPORT_TABS,type ReportTabId} from '../services/strategy-report-layout';
import {PRISTINE_RESEARCH_MODE,isValidatedPristinePair} from '../services/pristine-report';
import {pristinePresentation,pristineTradeTable,pristineNumber,PRISTINE_CLOSED_BASIS,PRISTINE_VISUAL_BASIS,PRISTINE_COST_BASIS,type PristineStrategySnapshot,type PristinePresentation,type PristineCell} from '../services/pristine-presenter';
import './PristineStrategyReport.css';

interface Series {
  setData(points: readonly Record<string,unknown>[]):void;
  setMarkers?(points: readonly Record<string,unknown>[]):void;
  applyOptions(options: Record<string,unknown>):void;
}
interface Chart {
  addLineSeries(options:Record<string,unknown>):Series;
  addCandlestickSeries(options:Record<string,unknown>):Series;
  timeScale():{fitContent():void;setVisibleLogicalRange(range:{from:number;to:number}):void};
  priceScale(id:string):{applyOptions(options:Record<string,unknown>):void};
  subscribeCrosshairMove(callback:(event:{time?:unknown})=>void):void;
  resize(width:number,height:number):void;
  takeScreenshot():HTMLCanvasElement;
  remove():void;
}
interface ChartLibrary {createChart(element:HTMLElement,options:Record<string,unknown>):Chart}
function download(blob:Blob,name:string){const url=URL.createObjectURL(blob),a=document.createElement('a');a.href=url;a.download=name;document.body.append(a);a.click();a.remove();setTimeout(()=>URL.revokeObjectURL(url),4000);}
function assertCurrent(snapshot:PristineStrategySnapshot){
  if(!snapshot.view||!isValidatedPristinePair(snapshot.data,snapshot.view)||!window.VaultRuntime.pristineCurrent?.(snapshot.data,snapshot.view))throw Error('The selected report or controls changed. Run the selected stock again before exporting.');
}
function ChartPanel({snapshot,model,kind,visible}:{snapshot:PristineStrategySnapshot;model:PristinePresentation;kind:'equity'|'drawdown'|'signals';visible:boolean}){
  const host=useRef<HTMLDivElement>(null),instance=useRef<Chart|null>(null),series=useRef<Record<string,Series>>({});
  const [error,setError]=useState(''),[hover,setHover]=useState(''),[compare,setCompare]=useState(false),[gross,setGross]=useState(false),[log,setLog]=useState(true);
  const [range,setRange]=useState<readonly[number,number]>([0,model.dates.length-1]);
  useEffect(()=>()=>{instance.current?.remove();instance.current=null;series.current={};},[model,kind]);
  useEffect(()=>{
    const element=host.current;
    if(!visible||!element)return;
    const library=(window as unknown as {LightweightCharts?:ChartLibrary}).LightweightCharts;
    if(!library){setError('The chart library is unavailable. Recorded values and downloads remain available.');return;}
    if(!instance.current){
      const style=getComputedStyle(element),color=(key:string,fallback:string)=>style.getPropertyValue(key).trim()||fallback;
      try{
        const chart=library.createChart(element,{width:element.clientWidth||600,height:340,layout:{attributionLogo:false,background:{color:color('--panel','#fff')},textColor:color('--ink3','#555'),fontFamily:getComputedStyle(document.body).fontFamily,fontSize:11},grid:{vertLines:{visible:false},horzLines:{color:color('--rule','#ddd')}},timeScale:{borderColor:color('--steel','#aaa'),minBarSpacing:.01},rightPriceScale:{borderColor:color('--steel','#aaa'),mode:kind==='equity'&&log?1:0},crosshair:{mode:0}});
        instance.current=chart;
        if(kind==='signals'){
          const candles=chart.addCandlestickSeries({upColor:color('--green','#268256'),downColor:color('--red','#b13e3e'),borderVisible:false,wickUpColor:color('--green','#268256'),wickDownColor:color('--red','#b13e3e'),priceLineVisible:false});
          candles.setData(model.report.rows.map(row=>({time:row.date,open:row.open,high:row.high,low:row.low,close:row.close})));
          candles.setMarkers?.(model.report.source.events.filter(e=>e.kind==='entry'||e.kind==='exit'||e.kind==='terminal_mark').map(event=>({time:model.report.rows[event.i-model.report.from_source_index].date,position:event.kind==='entry'?'belowBar':'aboveBar',shape:event.kind==='terminal_mark'?'circle':event.kind==='entry'?'arrowUp':'arrowDown',color:event.kind==='terminal_mark'?color('--ink3','#777'):event.kind==='entry'?color('--green','#268256'):color('--red','#b13e3e'),text:event.kind==='terminal_mark'?'Open mark':`${event.kind==='entry'?'Buy':'Sell'} · ${event.phase}`})));
          series.current.price=candles;
        }else{
          const line=chart.addLineSeries({color:color(kind==='drawdown'?'--red':'--blue',kind==='drawdown'?'#b13e3e':'#275d99'),lineWidth:2,priceLineVisible:false,priceFormat:{type:'custom',formatter:(value:number)=>kind==='drawdown'?`${(value*100).toFixed(2)}%`:`${value.toFixed(3)}×`}});
          line.setData(model.dates.map((time,i)=>({time,value:kind==='drawdown'?model.drawdown[i]:model.equity[i]})));series.current.net=line;
          if(kind==='equity')for(const [key,values,colour]of [['benchmark',model.benchmark,color('--bh','#888')],['gross',model.gross,color('--green','#268256')]]as const){const item=chart.addLineSeries({color:colour,lineWidth:1,visible:key==='benchmark'?compare:gross,priceLineVisible:false,lastValueVisible:false});item.setData(model.dates.map((time,i)=>({time,value:values[i]})));series.current[key]=item;}
        }
        chart.subscribeCrosshairMove(event=>{if(typeof event.time!=='string')return;const index=model.dates.indexOf(event.time);if(index<0)return;const row=model.report.rows[index];setHover(kind==='signals'?`${row.date} · O ${pristineNumber(row.open)} · H ${pristineNumber(row.high)} · L ${pristineNumber(row.low)} · C ${pristineNumber(row.close)}`:`${row.date} · ${kind==='drawdown'?pristineNumber(model.drawdown[index],true):`${pristineNumber(model.equity[index])}× · ${model.benchmarkLabel} ${pristineNumber(model.benchmark[index])}×`}`);});
        chart.timeScale().fitContent();setError('');
      }catch{instance.current?.remove();instance.current=null;setError('The chart could not render. No replacement prices or signals were generated.');}
    }
    const observer=typeof ResizeObserver==='function'?new ResizeObserver(()=>{if(visible&&element.clientWidth)instance.current?.resize(element.clientWidth,340);}):null;observer?.observe(element);instance.current?.resize(element.clientWidth||600,340);
    return()=>observer?.disconnect();
  },[visible,model,kind,compare,gross,log]);
  useEffect(()=>{series.current.benchmark?.applyOptions({visible:compare});series.current.gross?.applyOptions({visible:gross});instance.current?.priceScale('right').applyOptions({mode:kind==='equity'&&log?1:0});},[compare,gross,log,kind]);
  const applyRange=(start:number,end:number)=>{const last=model.dates.length-1,from=Math.max(0,Math.min(start,last)),to=Math.max(from,Math.min(end,last));setRange([from,to]);instance.current?.timeScale().setVisibleLogicalRange({from:from-.5,to:to+.5});};
  const png=()=>{try{assertCurrent(snapshot);if(!instance.current)throw Error('No visible chart is available.');const canvas=instance.current.takeScreenshot();canvas.toBlob(blob=>{try{assertCurrent(snapshot);if(!blob)throw Error('The chart image is unavailable.');download(blob,`${model.report.id}-${model.report.symbol}-${kind}.png`);}catch(e){setError((e as Error).message);}},'image/png');}catch(e){setError((e as Error).message);}};
  return <>
    <div className="chart-tools"><button type="button" className="dbtn sm" onClick={png}>Download PNG</button>{kind==='equity'&&<><label className="dcheck"><input type="checkbox" checked={compare} onChange={e=>setCompare(e.target.checked)}/>Compare with {model.benchmarkLabel}</label><label className="dcheck"><input type="checkbox" checked={gross} onChange={e=>setGross(e.target.checked)}/>Independent gross</label><label className="dcheck"><input type="checkbox" checked={log} onChange={e=>setLog(e.target.checked)}/>Log scale</label></>}</div>
    <div className="pxread" role="status">{hover||'Hover to inspect recorded values. Drag or scroll to zoom.'}</div>
    <div ref={host} className="lwc" style={{height:340,minWidth:0}} data-pristine-chart={kind}/>
    {error&&<p className="rnote" role="alert">{error}</p>}
    <div className="equity-navigator"><div className="year-shortcuts"><button className="dbtn sm" onClick={()=>applyRange(0,model.dates.length-1)}>All sessions</button>{[...new Set(model.dates.map(d=>d.slice(0,4)))].map(year=><button className="dbtn sm" key={year} onClick={()=>{const indices=model.dates.flatMap((d,i)=>d.startsWith(year)?[i]:[]);applyRange(indices[0],indices.at(-1)!);}}>{year}</button>)}</div><label>Start <input type="range" aria-label="Chart range start" min={0} max={model.dates.length-1} value={range[0]} disabled={model.dates.length===1} onChange={e=>applyRange(+e.target.value,range[1])}/></label><label>End <input type="range" aria-label="Chart range end" min={0} max={model.dates.length-1} value={range[1]} disabled={model.dates.length===1} onChange={e=>applyRange(range[0],+e.target.value)}/></label><output>{model.dates[range[0]]} — {model.dates[range[1]]}</output></div>
    <p className="rnote">{PRISTINE_VISUAL_BASIS} {kind==='signals'?'Source signals use daily sessions. Arrows identify actual filled entry/exit events; circles mark open positions. OHLC ambiguity remains explicit in source events.':'The model starts at1. This is not funded capital or a source-book performance reproduction.'}</p>
  </>;
}
function DataTable({headers,rows,percentColumns=[]}:{headers:readonly string[];rows:readonly(readonly PristineCell[])[];percentColumns?:readonly number[]}){
  const [limit,setLimit]=useState(100);
  return <><div className="tlog" data-pristine-table><table className="tbl"><thead><tr>{headers.map((header,i)=><th key={i}>{header}</th>)}</tr></thead><tbody>{rows.slice(0,limit).map((row,i)=><tr key={i}>{row.map((value,j)=><td key={j} className={typeof value==='number'?'n':''}>{typeof value==='number'?pristineNumber(value,percentColumns.includes(j)):value===null?'—':typeof value==='boolean'?String(value):value}</td>)}</tr>)}</tbody></table></div>{limit<rows.length&&<p className="rnote">Showing {limit} of {rows.length} rows. Excel includes every recorded row.</p>}{limit<rows.length&&<button className="dbtn sm" onClick={()=>setLimit(limit+100)}>Show 100 more ({rows.length} total; full export available)</button>}</>;
}
function Controls({snapshot}:{snapshot:PristineStrategySnapshot}){
  const selection=snapshot.selection;
  return <form className="period-panel" onSubmit={event=>{event.preventDefault();const form=new FormData(event.currentTarget);window.VaultRuntime.patch({region:'us',run_scope:'single',basket:'',tk:String(form.get('ticker')||selection.tk),research_mode:String(form.get('research_mode')||''),from:String(form.get('from')||''),to:String(form.get('to')||''),cost:selection.cost??'0',slippage:String(form.get('slippage')||'0'),rollover:'0',p:''});}}>
    <div className="sw research-controls"><label className="dfield">Research basis<select name="research_mode" defaultValue={selection.research_mode||''} required><option value="">Choose research basis</option><option value={PRISTINE_RESEARCH_MODE}>Supplied USD-labelled series units</option></select></label><label className="dfield stock-picker">Actual US stock<select name="ticker" defaultValue={selection.tk} required>{!snapshot.stocks.some(stock=>stock.ticker===selection.tk)&&<option value={selection.tk}>{selection.tk||'Select a stock'}</option>}{snapshot.stocks.map(stock=><option key={stock.ticker} value={stock.ticker}>{stock.symbol}</option>)}</select></label><label className="dfield">From<input type="date" name="from" defaultValue={selection.from||''}/></label><label className="dfield">To<input type="date" name="to" defaultValue={selection.to||''}/></label><label className="dfield">Slippage (bps / side)<input type="number" name="slippage" defaultValue={selection.slippage||'0'} min="0" max="5000" step="any" required/></label><button className="dbtn primary" type="submit">Run selected stock</button></div>
    {Number(selection.cost??'0')!==0&&<p className="rnote" data-retained-fee-setting>Supplied execution-fee setting: {selection.cost??'0'} bps per side, retained unchanged. This form does not edit execution fees.</p>}
    <p className="rnote">This explicit research basis applies source constants to the unchanged supplied series units. Historical-dollar equivalence, quote accuracy, corporate-action conventions and dated universe membership remain unverified. Fixed source rules; no parameter grid. Basket results require their separately reviewed coordinator and are unavailable here.</p>
  </form>;
}
/** Existing strategy route, separate protected model; no legacy Run is created. */
export function PristineStrategyReport({snapshot,chrome}:{snapshot:PristineStrategySnapshot;chrome:string}){
  const model=useMemo(()=>snapshot.view?pristinePresentation(snapshot.data,snapshot.view):null,[snapshot]);
  const [tab,setTab]=useState<ReportTabId>('equity'),[message,setMessage]=useState(''),[printing,setPrinting]=useState(false);
  useEffect(()=>{const before=()=>flushSync(()=>setPrinting(true)),after=()=>setPrinting(false);window.addEventListener('beforeprint',before);window.addEventListener('afterprint',after);const observer=new MutationObserver(()=>setPrinting(document.body.classList.contains('pdf-capture')||document.body.classList.contains('report-print')));observer.observe(document.body,{attributes:true,attributeFilter:['class']});return()=>{window.removeEventListener('beforeprint',before);window.removeEventListener('afterprint',after);observer.disconnect();};},[]);
  const report=model?.report,mode=snapshot.selection.report_mode==='compound'?'compound':'fixed';
  const exportReport=async(kind:'workbook'|'csv')=>{try{assertCurrent(snapshot);setMessage('Preparing download…');if(!window.VaultRuntime.exportPristine)throw Error('The Pristine export service is unavailable.');await window.VaultRuntime.exportPristine(snapshot.data,snapshot.view!,kind);setMessage('Download prepared.');}catch(e){setMessage((e as Error).message);}};
  const tabKey=(event:KeyboardEvent<HTMLButtonElement>,index:number)=>{const next=event.key==='ArrowRight'?(index+1)%REPORT_TABS.length:event.key==='ArrowLeft'?(index+REPORT_TABS.length-1)%REPORT_TABS.length:event.key==='Home'?0:event.key==='End'?REPORT_TABS.length-1:null;if(next!==null){event.preventDefault();setTab(REPORT_TABS[next].id);document.getElementById(`pristine-tab-${REPORT_TABS[next].id}`)?.focus();}};
  const title=snapshot.definition.name||snapshot.definition.family_name||snapshot.definition.id;
  return <>
    <ResearchMarkup html={chrome} slots={{"strategy-settings":model?<StrategySettings pristine mode={mode} onApply={report_mode=>window.VaultRuntime.patch({report_mode})}/>:null}}/>
    <section className="box"><div className="st-head"><h1 className="st">{title}</h1><span className="olabel">Daily stock research</span></div><p className="rnote">Trading the Pristine Method — Part1 · Edition year unconfirmed. Selected source branches are reviewed; simulated application, declared execution conventions and book-reported performance remain separate.</p><Controls snapshot={snapshot}/>{snapshot.availability&&snapshot.availability.status!=='ready'&&<p className="rnote" role="status">{snapshot.availability.note}</p>}{snapshot.optimization&&<p className="rnote">Fixed source rules; no parameter grid, optimizer search or fitted configuration is available. Open the strategy report for the selected stock.</p>}</section>
    {!model||!report?null:<>
      <section className="box" data-pristine-notice><p><strong>Source-reviewed daily branch · declared supplied-series research.</strong> {report.basis}</p><p className="rnote">{PRISTINE_COST_BASIS} Exposure counts observed entry-through-exit sessions; Days Held uses calendar days. Exact-zero closed returns join non-wins. No monetary P&amp;L, share quantities, taxes or exact intraday excursions are asserted.</p><div className="chart-tools"><button className="dbtn sm" onClick={()=>exportReport('workbook')}>Download Excel</button><button className="dbtn sm" onClick={()=>exportReport('csv')}>Positions CSV</button><button className="dbtn sm" onClick={()=>{try{assertCurrent(snapshot);window.print();}catch(e){setMessage((e as Error).message);}}}>Print / save PDF</button></div><p role="status">{message}</p></section>
      <section className="box"><div className="box-h">Computed backtest · {report.symbol} · US</div><p className="rnote">{model.dates[0]} to {model.dates.at(-1)} · {report.costs.fees_bps} execution-fee bps and {report.costs.slippage_bps} slippage bps per side. No native-platform or original-book performance claim.</p><div className="metrics">{[['Net Profit',mode==='compound'?model.closedCompound:model.closedSum,true],['Avg net pnl',report.statistics.financial.values.avg_net_pnl,true],['No. of trades',report.closed_stats.net.closed_count,false],['Win Rate',report.closed_stats.net.closed_win_rate,true],['Expectancy',report.statistics.net.values.expectancy_ratio,false],['MaxDD',report.closed_stats.net_max_drawdown,true],['Sharpe',report.statistics.net.values.sharpe,false]].map(([label,value,percent])=><div className="metric" key={String(label)}><span>{String(label)}</span><b>{(label==='Expectancy'||label==='Sharpe')&&typeof value==='number'&&Number.isFinite(value)?value.toFixed(2):pristineNumber(value as number|null,Boolean(percent))}</b></div>)}</div><p className="rnote" data-return-definition><b>Net Profit · {mode==='compound'?'Compounded closed model':'Closed return sum'}.</b> {PRISTINE_CLOSED_BASIS} The separate net model curve includes held-position marks.</p></section>
      <section className="box strategy-report" data-strategy-report data-pristine-report>
        <div className="report-tabs" role="tablist" aria-label="Backtest report views">{REPORT_TABS.map((item,index)=><button key={item.id} type="button" role="tab" id={`pristine-tab-${item.id}`} aria-controls={`pristine-panel-${item.id}`} aria-selected={tab===item.id} tabIndex={tab===item.id?0:-1} onClick={()=>setTab(item.id)} onKeyDown={event=>tabKey(event,index)}>{item.label}</button>)}</div>
        {REPORT_TABS.map(item=><div key={item.id} id={`pristine-panel-${item.id}`} className="report-panel" role="tabpanel" aria-labelledby={`pristine-tab-${item.id}`} hidden={!printing&&tab!==item.id} tabIndex={0}>
          <div className="box-h">{item.id==='equity'?'Normalized unit-exposure model':item.id==='signals'?`Price & signals · ${report.symbol}`:item.label}<button className="dbtn sm" onClick={()=>exportReport('workbook')}>Download Excel</button></div>
          {item.id==='equity'||item.id==='drawdown'||item.id==='signals'?<ChartPanel snapshot={snapshot} model={model} kind={item.id==='signals'?'signals':item.id} visible={printing||tab===item.id}/>:item.id==='positions'?<DataTable {...pristineTradeTable(snapshot.data,snapshot.view!)}/>:item.id==='yearly'?<DataTable headers={['Year','Net model return',`${report.symbol} buy & hold`]} rows={Object.entries(model.years).map(([year,value])=>[year,value,model.benchmarkYears[year]??null])} percentColumns={[1,2]}/>:<PristineMonthlyHeatmap model={model}/>}
          {(item.id==='monthly'||item.id==='yearly')&&Object.keys(item.id==='monthly'?model.visualPeriodReasons.months:{...model.visualPeriodReasons.years,...model.visualPeriodReasons.benchmarkYears}).length>0&&<p className="rnote" role="status">{Object.entries(item.id==='monthly'?model.visualPeriodReasons.months:{...model.visualPeriodReasons.years,...model.visualPeriodReasons.benchmarkYears}).map(([period,reason])=>`${period}: ${reason}`).join(' ')}</p>}
          {item.id==='signals'&&<details className="strategy-indicators"><summary>Indicators and source timing</summary><p className="rnote">The source signal/volume prefix can begin before this selected window. No missing pre-window indicator values are reconstructed. CBS holds the known signal SMA20 target and trails the prior completed low after two bars; BGS uses its completed setup volume and next actual open. Exact levels, source events and typed volume evidence are included in the workbook.</p></details>}
        </div>)}
      </section>
      <section className="box" data-benchmark-comparison data-benchmark-symbol={report.qualified_ticker}><div className="box-h">Strategy vs {model.benchmarkLabel}</div><p className="rnote">Actual selected-stock closes; no benchmark execution costs. No index is substituted.</p><DataTable headers={['Statistic','Net model',model.benchmarkLabel]} rows={['total_return','cagr','max_dd','sharpe','sortino','vol','mar'].map(key=>[key,(report.statistics.net.values as Record<string,number|null>)[key],(report.statistics.benchmark.values as Record<string,number|null>)[key]])}/></section>
      <details className="box detailed-stats" data-pristine-details><summary>Detailed statistics · raw net / gross</summary><DataTable headers={['Statistic','Net','Gross','Net unavailable reason','Gross unavailable reason']} rows={Object.entries(report.statistics.net.values).filter(([key])=>!['net_profit','avg_trade','trades','win_rate','expectancy_ratio','max_dd','sharpe',...(mode==='fixed'?['closed_return_sum']:[])].includes(key)).map(([key,value])=>[key,value,(report.statistics.gross.values as Record<string,number|null>)[key],(report.statistics.net.null_reasons as Record<string,string>)[key]||null,(report.statistics.gross.null_reasons as Record<string,string>)[key]||null])}/><p className="rnote">Raw values are model units, fractional returns, ratios or observed-session counts according to the named statistic. Gross compounds independently at zero costs. Undefined statistics stay unavailable; full raw/display maps are exported separately.</p></details>
      <section className="box"><div className="box-h">Source rules and declared conventions</div><ul>{snapshot.definition.rules?.map((rule,index)=><li key={index}>{rule.replace(/^[A-Z]\|/,'')}</li>)}</ul><p className="rnote">{snapshot.definition.notes}</p><details><summary>Original source provenance and limitations</summary><pre style={{whiteSpace:'pre-wrap',overflowWrap:'anywhere'}}>{report.provenance_json}</pre><p className="rnote">No historical-dollar, adjustment, corporate-action, quote-accuracy or dated membership flag is upgraded by this display.</p></details></section>
    </>}
  </>;
}
