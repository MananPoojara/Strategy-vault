import type {ReactNode} from "react";
import {ForeignProvider,useForeignScope} from "./ForeignFields";
import {NumField,OperandEditor} from "./OperandEditor";
import {removeAt} from "./spec-model";
import {budget,limits} from "./rule-tree";
import {NodeEditor} from "./NodeEditor";
import {memberOptions,portfolioEnabled,portfolioShape,withPortfolioMode} from "./portfolio-model";
import type {Catalogue,Market,PortfolioSpec,Spec} from "./types";
import type {InstrumentOption} from "./instruments";
import "./portfolio.css";
function ScoreScope({children}:{children:ReactNode}){const scope=useForeignScope();return scope?<ForeignProvider value={{...scope,allowSymbol:false}}>{children}</ForeignProvider>:<>{children}</>;}
export function PortfolioPanel({spec,catalogue,market,options,onChange}:{spec:Spec;catalogue:Catalogue;market:Market;options:readonly InstrumentOption[];onChange(s:Spec):void}){
 const p=spec.portfolio,cap=catalogue.portfolio;if(!Object.hasOwn(spec,"portfolio")&&!portfolioEnabled(catalogue))return null;
 if(Object.hasOwn(spec,"portfolio")&&!portfolioShape(p))return <section className="box lab-portfolio"><p className="lab-kept">Portfolio settings kept as is: <code>{JSON.stringify(p)}</code></p><button type="button" className="dbtn sm" onClick={()=>onChange(removeAt(spec,"portfolio"))}>Turn portfolio off</button></section>;
 const patch=(x:Partial<PortfolioSpec>)=>p&&onChange({...spec,portfolio:{...p,...x}});const members=memberOptions(options,market);const b=budget(spec),lim=limits(catalogue);
 return <section className="box lab-portfolio" data-lab-portfolio>
  <label className="dfield">Strategy mode<select aria-label="Strategy mode" value={p?.mode??(p?"signals":"off")} onChange={e=>onChange(withPortfolioMode(spec,e.target.value as "off"|"signals"|"rotation"))}>
   <option value="off">Single instrument</option>{(cap?.modes??[p?.mode??"signals"]).map(m=><option key={m} value={m} disabled={!cap||market==="gl"}>{m==="rotation"?"Portfolio · rank rotation":"Portfolio · entry and exit signals"}</option>)}
  </select></label>
  {p&&<details><summary>Universe and ranking · {p.universe.basket??`${p.universe.tickers?.length??0} selected names`} · {p.max_open??1} positions</summary>
   {spec.origin&&<p className="rnote">Book origins cannot combine with portfolio execution. <button type="button" className="dbtn sm" onClick={()=>onChange(removeAt(spec,"origin"))}>Remove book origin explicitly</button></p>}
   <div className="lab-row"><label className="dfield">Universe<select aria-label="Portfolio universe" value={p.universe.basket??"list"} onChange={e=>patch({universe:e.target.value==="list"?{tickers:[]}:{basket:e.target.value}})}><option value="list">Choose stocks or ETFs…</option>{cap?.baskets.map(b=><option key={b.id} value={b.id} disabled={b.supported===false||b.members>cap.max_universe}>{b.id} · {b.members} names{b.supported===false?" · unavailable":""}</option>)}</select></label>
   {!p.universe.basket&&<label className="dfield">Members<select multiple aria-label="Portfolio members" value={p.universe.tickers??[]} onChange={e=>patch({universe:{tickers:[...e.target.selectedOptions].map(o=>o.value)}})}>{(p.universe.tickers??[]).filter(t=>!members.some(o=>o.ticker===t)).map(t=><option key={t} value={t}>Kept: {t}</option>)}{members.map(o=><option key={o.ticker} value={o.ticker}>{o.label}</option>)}</select></label>}
   <NumField label="Max positions" value={p.max_open??1} min={1} max={cap?.max_open??50} step={1} onChange={n=>n!==undefined&&patch({max_open:n})}/>
   <label className="dfield">Prefer<select aria-label="Portfolio ranking" value={p.rank??"highest"} onChange={e=>patch({rank:e.target.value as "highest"|"lowest"})}><option value="highest">Highest score</option><option value="lowest">Lowest score</option></select></label>
   {p.mode==="rotation"&&<><NumField label="Worst rank held" value={p.worst_rank_held} min={p.max_open??1} max={cap?.max_universe??120} step={1} onChange={n=>patch({worst_rank_held:n})}/><label className="dfield">Rebalance<select aria-label="Portfolio rebalance" value={p.rebalance??"bar"} onChange={e=>patch({rebalance:e.target.value as PortfolioSpec["rebalance"]})}>{(cap?.rebalance??["bar"]).map(v=><option key={v} value={v}>{v==="bar"?"Every session":v}</option>)}</select></label></>}
   </div><p className="rnote">A universe chosen today carries hindsight and survivorship bias. Historical membership dates are not verified. An index is a signal or benchmark only.</p>
   {p.score?<div className="lab-row"><span>Rank score</span><ScoreScope><OperandEditor operand={p.score} side="entry" catalogue={catalogue} heldOk={false} label="Portfolio score" onChange={score=>patch({score})} renderNode={(node,change,label)=><NodeEditor node={node} onChange={change} label={label} catalogue={catalogue} side="entry" top={false} caps={{leaves:b.leaves>=lim.leaves,nodes:b.nodes>=lim.nodes,top:false}}/>}/></ScoreScope><button type="button" className="dbtn sm" onClick={()=>patch({score:undefined})}>Remove score</button></div>:<button type="button" className="dbtn sm" onClick={()=>patch({score:{s:"close"}})}>Add rank score</button>}
   {cap?.max_source_bars&&<p className="cap">Source history budget: {cap.max_source_bars.toLocaleString()} bars across the universe; {cap.max_member_bars?.toLocaleString()} per member. Shorter display dates do not truncate indicator warm-up.</p>}
   <p className="cap">Capital and declared costs are shared across the universe. Default sizing divides equity by Max positions. Unsupported strategy settings stay visible until you resolve them.</p>
  </details>}
 </section>;
}
