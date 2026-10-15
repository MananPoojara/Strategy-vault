import {foreignDescription} from "./foreign-model";
import {useEffect,useRef} from "react";
import {createLabChart,themeColors,useThemeKey} from "./charts";
import {linePoints} from "./bars";
import {drawdown,ExecutionStrip,kpiItems} from "./RunResults";
import {num,pct,int} from "./format";
import {MetricTiles,ServerLabel} from "./ui";
import type {PortfolioRunResult} from "./types";
import "./results.css";
import "./portfolio.css";
export function portfolioTradeRows(r:PortfolioRunResult):string[][]{return [["Ticker","Entry","Entry signal","Order","Raw entry fill","Exit","Raw exit fill","Qty","Notional","Declared costs","Contribution / signal equity","Bars","Rank","Score","Reason"],...[...r.trades].reverse().map(t=>[t.ticker,t.entry,t.entry_signal,t.entry_kind,num(t.entry_px),t.open?"Open":t.exit,num(t.exit_px),num(t.qty),num(t.notional),num(t.costs),pct(t.ret,2),int(t.bars),int(t.rank),num(t.score),t.reason])];}
export function PortfolioResults({result:r}:{result:PortfolioRunResult}){
 const eq=useRef<HTMLDivElement>(null),dd=useRef<HTMLDivElement>(null),theme=useThemeKey();
 useEffect(()=>{
  if(!eq.current||!dd.current)return;const chart=createLabChart(eq.current,300,{rightPriceScale:{mode:1},timeScale:{minBarSpacing:.01}});
  if(chart){const{colors,chart:c}=chart;c.addAreaSeries({lineColor:colors.chart,topColor:colors.chart+"33",bottomColor:"transparent"}).setData(linePoints(r.dates,r.equity));c.addLineSeries({color:colors.bh,lineWidth:1}).setData(linePoints(r.dates,r.bh));c.timeScale().fitContent();}
  const host=dd.current;const paint=()=>{const values=drawdown(r.equity);if(values.length<2||values.length!==r.dates.length||values.some(v=>!Number.isFinite(v))){host.textContent=values.length===1?`${r.dates[0]} · ${pct(values[0],2)}`:"Drawdown data is unavailable.";return;}const render=window.VaultRuntime?.renderAreaChart;if(render)render(host,r.dates,values,{color:themeColors(host).red,fmt:v=>pct(v,2),axFmt:v=>pct(v,0),h:300,markMin:true});else host.textContent="Drawdown chart is unavailable.";};paint();
  const observer=typeof ResizeObserver!=="undefined"?new ResizeObserver(paint):null;observer?.observe(host);return()=>{observer?.disconnect();chart?.dispose();host.replaceChildren();};
 },[r,theme]);
 const table=portfolioTradeRows(r);const download=()=>{const report=window.VaultReport;if(!report)return;const url=URL.createObjectURL(new Blob([report.csv(table)],{type:"text/csv;charset=utf-8"}));const a=document.createElement("a");a.href=url;a.download=`portfolio-${r.from}-${r.to}.csv`;a.click();URL.revokeObjectURL(url);};
 return <section className="box lab-portfolio-result" data-lab-portfolio-result tabIndex={-1}>
  <h2 className="box-h">Portfolio backtest <small>{r.from} → {r.to} · {r.reference.ticker} reference</small></h2><ServerLabel text={r.label}/>
  <ExecutionStrip model={r.execution_model} money={r.money} adaptations={r.adaptations}/>
  <p className="rnote" data-lab-membership>{r.membership.note} Historical membership verified: {String(r.membership.historical_membership_verified)}.</p>
  <p className="cap">{r.portfolio.universe.admitted} admitted of {r.portfolio.universe.members} requested names · {r.portfolio.mode} · {r.portfolio.max_open} maximum positions. Gross revalues the same booked fills and quantities before declared costs.</p>
  {!!r.portfolio.universe.refused.length&&<details><summary>Unavailable members · {r.portfolio.universe.refused.length}</summary><ul>{r.portfolio.universe.refused.map(m=><li key={m.ticker}>{m.ticker}: {m.reason}</li>)}</ul></details>}
  <MetricTiles items={[{label:`Final equity · ${r.money.currency}`,value:num(r.money.final_equity)},{label:"Costs paid",value:num(r.money.costs_total)},{label:"Cash",value:num(r.cash.at(-1))},{label:"Invested exposure",value:num(r.exposure.at(-1))},{label:"Positions",value:int(r.positions.at(-1))},...kpiItems(r.stats).filter(m=>m.label!=="Buy & hold CAGR")]}/>
  <div className="lab-result-charts"><figure className="lab-result-chart"><figcaption className="lab-chart-h"><h3>Portfolio equity growth</h3></figcaption><p className="cap">Strategy and {r.reference.ticker} buy &amp; hold · logarithmic scale · capital baseline 1×</p><div className="lwc" ref={eq} aria-label="Portfolio equity chart"/></figure><figure className="lab-result-chart"><figcaption className="lab-chart-h"><h3>Drawdown from peak</h3></figcaption><p className="cap">0% is the previous peak · initial costs included</p><div className="lwc bkc" ref={dd} aria-label="Portfolio drawdown chart"/></figure></div>
  <p className="cap">Skipped: {Object.entries(r.skipped).map(([k,v])=>`${k.replaceAll("_"," ")}: ${v}`).join(" · ")}</p>
  {!!r.foreign?.length&&<p className="cap">Signal sources: {r.foreign.map(foreignDescription).join("; ")}</p>}
  <details><summary>Cash, invested exposure and position count by session</summary><p className="cap">Equity is normalized to initial capital; cash and invested exposure below are raw {r.money.currency}, not percentages. These counts are not a holdings snapshot.</p><div className="tblwrap"><table className="tbl"><thead><tr>{["Date","Equity ×","Cash","Exposure","Positions"].map(h=><th key={h}>{h}</th>)}</tr></thead><tbody>{r.dates.slice(-200).map((date,i)=>{const k=r.dates.length-Math.min(200,r.dates.length)+i;return <tr key={date}><td>{date}</td><td>{num(r.equity[k])}</td><td>{num(r.cash[k])}</td><td>{num(r.exposure[k])}</td><td>{int(r.positions[k])}</td></tr>;})}</tbody></table></div></details>
  <h3 className="box-h">Trades <small>Raw fill prices · contribution is P&amp;L / total equity at the signal</small></h3>{window.VaultReport&&<button type="button" className="dbtn sm" onClick={download}>Download portfolio CSV</button>}<div className="tblwrap"><table className="tbl"><thead><tr>{table[0].map(h=><th key={h}>{h}</th>)}</tr></thead><tbody>{table.slice(1,201).map((row,i)=><tr key={i}>{row.map((v,k)=><td key={k}>{v}</td>)}</tr>)}</tbody></table></div>{r.trades.length>200&&<p className="cap">Latest 200 trades shown; CSV includes every returned trade.</p>}
 </section>;
}
