import type {ReactNode} from "react";
import {ForeignProvider,useForeignScope} from "./ForeignFields";
import {NumField,OperandEditor} from "./OperandEditor";
import {removeAt,setExecution} from "./spec-model";
import {budget,limits} from "./rule-tree";
import {NodeEditor} from "./NodeEditor";
import {SettingsTabs} from "./RuleBuilder";
import {Segmented} from "./ui";
import {Note,SettingGroup} from "./settings/common";
import {memberOptions,portfolioEnabled,portfolioShape,withPortfolioMode} from "./portfolio-model";
import type {Catalogue,Market,PortfolioSpec,Spec} from "./types";
import type {InstrumentOption} from "./instruments";
import "./portfolio.css";
function ScoreScope({children}:{children:ReactNode}){const scope=useForeignScope();return scope?<ForeignProvider value={{...scope,allowSymbol:false}}>{children}</ForeignProvider>:<>{children}</>;}
type Mode="off"|"signals"|"rotation";
const VAULT="Vault lists (C&P members by date, today's lists, your own)";
/** Strategy settings › Portfolio: what the backtest runs on (one instrument by default), and for a portfolio its watchlist, ranking and positions as tabs. */
export function PortfolioPanel({spec,catalogue,market,options,onChange}:{spec:Spec;catalogue:Catalogue;market:Market;options:readonly InstrumentOption[];onChange(s:Spec):void}){
 const p=spec.portfolio,cap=catalogue.portfolio;if(!Object.hasOwn(spec,"portfolio")&&!portfolioEnabled(catalogue))return null;
 if(Object.hasOwn(spec,"portfolio")&&!portfolioShape(p))return <section className="box lab-portfolio"><p className="lab-kept">Portfolio settings kept as is: <code>{JSON.stringify(p)}</code></p><button type="button" className="dbtn sm" onClick={()=>onChange(removeAt(spec,"portfolio"))}>Turn portfolio off</button></section>;
 const patch=(x:Partial<PortfolioSpec>)=>p&&onChange({...spec,portfolio:{...p,...x}});const members=memberOptions(options,market);const b=budget(spec),lim=limits(catalogue);
 const modes:Mode[]=["off",...(cap?.modes??(p?[p.mode??"signals"]:[])) as Mode[]];
 const name=(m:Mode)=>m==="off"?"Single instrument":m==="rotation"?"Portfolio · rank rotation":"Portfolio · entry and exit signals";
 const picked=cap?.baskets.find(x=>x.id===p?.universe.basket);
 // AmiBroker databases (their watchlists), then Vault's own lists: dated C&P lists first, then today's lists
 const all=cap?.baskets??[],dbs=[...new Set(all.flatMap(x=>x.database?[x.database]:[]))];
 // the team's scratch lists stay hidden (user, 25-11-2026), unless this strategy already uses one
 const watchlists=(d:string)=>all.filter(x=>x.database===d&&(!scratchList(x.label??x.id)||x.id===p?.universe.basket)).sort((x,y)=>(x.label??x.id).localeCompare(y.label??y.id));
 const vault=all.filter(x=>!x.database).sort((x,y)=>Number(y.membership_kind==="dated_aux")-Number(x.membership_kind==="dated_aux"));
 const db=picked?.database??VAULT;
 const off=(x:NonNullable<typeof cap>["baskets"][number])=>x.supported===false||x.membership_kind!=="dated_aux"&&x.members>(cap?.max_universe??120);
 const aux=auxMeaning(db);
 const rankWhy=p?.mode==="rotation"&&!p.score?"Rotation holds the best-ranked names, so it needs a rank score.":null;
 return <section className="lab-portfolio" data-lab-portfolio>
  <div className="lab-portfolio-mode"><span className="lab-portfolio-mode-h">Backtest on</span>
   <Segmented label="Strategy mode" value={(p?.mode??(p?"signals":"off")) as Mode} onChange={m=>onChange(withPortfolioMode(spec,m))}
    options={modes.map(m=>({value:m,label:name(m),disabled:m!=="off"&&(!cap||market==="gl"),title:m==="off"?"The instrument on the chart":m==="rotation"?"Hold the best-ranked names, swapping at each rebalance":"Every stock in the watchlist trades your entry and exit rules, sharing one account"}))}/>
  </div>
  {p&&<>
   {spec.origin&&<p className="rnote">Book origins cannot combine with portfolio execution. <button type="button" className="dbtn sm" onClick={()=>onChange(removeAt(spec,"origin"))}>Remove book origin explicitly</button></p>}
   {/* Maximum position and previous-bar equity are in Backtest parameters (user, 26-11-2026); an engine without them keeps a Positions tab */}
   <SettingsTabs ariaLabel="Portfolio settings" labels={catalogue.execution?["Watchlist","Ranking"]:["Watchlist","Ranking","Positions"]}>
    <>
     <SettingGroup title="Database">
      <label className="dfield">Database<select aria-label="Database" value={db} onChange={e=>{const w=watchlists(e.target.value);patch({universe:e.target.value===VAULT?{basket:vault[0]?.id}:{basket:(w.find(x=>/nifty 50 c&p 141/i.test(x.label??""))??w[0]).id}});}}>
       {dbs.map(d=><option key={d} value={d}>{dbName(d)}</option>)}<option value={VAULT}>{VAULT}</option></select></label>
      {db!==VAULT&&<label className="dfield">Watchlist<select aria-label="Watchlist" value={p.universe.basket??""} onChange={e=>patch({universe:{basket:e.target.value}})}>
       {watchlists(db).map(x=><option key={x.id} value={x.id}>{x.label??x.id} · {x.members} names</option>)}</select></label>}
      {db===VAULT&&<div className="lab-watchlist" role="radiogroup" aria-label="Portfolio universe">
       {vault.map(x=><label key={x.id} className={`lab-watch-row${off(x)?" off":""}`}><input type="radio" name="lab-watchlist" value={x.id} checked={p.universe.basket===x.id} disabled={off(x)} onChange={()=>patch({universe:{basket:x.id}})}/>
        <span>{x.label??x.id}</span><span className="cap">{x.membership_kind==="dated_aux"?"by date":"today's list"}</span><span className="n">{x.members} names{x.supported===false?" · unavailable":""}</span></label>)}
       <label className="lab-watch-row"><input type="radio" name="lab-watchlist" value="list" checked={!p.universe.basket} onChange={()=>patch({universe:{tickers:[]}})}/><span>My own stocks or ETFs…</span><span className="cap">today's list</span><span className="n">{p.universe.tickers?.length??0} chosen</span></label>
      </div>}
      {db===VAULT&&!p.universe.basket&&<label className="dfield">Members<select multiple aria-label="Portfolio members" value={p.universe.tickers??[]} onChange={e=>patch({universe:{tickers:[...e.target.selectedOptions].map(o=>o.value)}})}>{(p.universe.tickers??[]).filter(t=>!members.some(o=>o.ticker===t)).map(t=><option key={t} value={t}>Kept: {t}</option>)}{members.map(o=><option key={o.ticker} value={o.ticker}>{o.label}</option>)}</select></label>}
      {db!==VAULT?<Note gist="Watchlists are today's lists; the Aux filters add the dates.">AmiBroker's Apply to › Filter: the database's own watchlist, read as it is saved in AmiBroker. Without an Aux filter every stock can be bought on every bar.</Note>
       :<Note gist={picked?.membership_kind==="dated_aux"?"Members by date: bought only on days in the list.":"Today's list: carries hindsight and survivorship bias."}>{picked?.membership_kind==="dated_aux"
        ?"Members by date (the C&P database's Aux flag): every stock that was ever in the list, bought only on days it was in it; a stock that leaves keeps its own exit. An index is a signal or benchmark only."
        :"A universe chosen today carries hindsight and survivorship bias. Historical membership dates are not verified. An index is a signal or benchmark only."}</Note>}
      {/* the dated C&P lists and AmiBroker watchlists are bounded by the engine's memory instead (api.rs dated_limits) */}
      {cap?.max_source_bars&&!["dated_aux","ami_watchlist"].includes(picked?.membership_kind??"")&&<p className="cap">Source history budget: {cap.max_source_bars.toLocaleString()} bars across the universe; {cap.max_member_bars?.toLocaleString()} per member.</p>}
     </SettingGroup>
     {db!==VAULT&&<SettingGroup title="Buy only when">
      {(["aux1","aux2"] as const).map((k,i)=>{const on=spec.execution?.[k]===true;return <label key={k} className="lab-check"><input type="checkbox" aria-label={`Aux${i+1}`} checked={on} disabled={!on&&!aux[k]}
       onChange={e=>onChange(setExecution(spec,{[k]:e.target.checked?true:undefined}))}/> Aux{i+1} = 1 <span className="cap">{aux[k]??"not filled in this database"}</span></label>;})}
      <Note gist="Each stock checks its own Aux column on the signal bar.">As the AFLs' Buy AND Aux1 == 1: a stock is bought only on bars where its Aux value is 1, and a held stock keeps its own exit. These are the same switches as Aux1 / Aux2 under Backtest parameters.</Note>
     </SettingGroup>}
    </>
    <>
     <SettingGroup title="Ranking">
      {rankWhy&&<p className="cap" role="note">{rankWhy}</p>}
      <label className="dfield">Rank by<select aria-label="Rank by" value={p.score?"score":"none"} onChange={e=>patch({score:e.target.value==="score"?p.score??{s:"close"}:undefined})}><option value="none">No ranking (A to Z)</option><option value="score">An indicator or price</option></select></label>
      <label className="dfield">Prefer<select aria-label="Portfolio ranking" value={p.rank??"highest"} onChange={e=>patch({rank:e.target.value as "highest"|"lowest"})}><option value="highest">Highest score (TopBottom)</option><option value="lowest">Lowest score (BottomUp)</option></select></label>
      {p.score&&<div className="lab-portfolio-score"><span className="cap">Rank score</span><ScoreScope><OperandEditor operand={p.score} side="entry" catalogue={catalogue} heldOk={false} label="Portfolio score" onChange={score=>patch({score})} renderNode={(node,change,label)=><NodeEditor node={node} onChange={change} label={label} catalogue={catalogue} side="entry" top={false} caps={{leaves:b.leaves>=lim.leaves,nodes:b.nodes>=lim.nodes,top:false}}/>}/></ScoreScope></div>}
      <Note gist="More signals than free positions: the best-ranked buy first.">AmiBroker's PositionScore. With no score, ties go to the larger order, then A to Z.</Note>
     </SettingGroup>
     {p.mode==="rotation"&&<SettingGroup title="Rotation">
      <NumField label="Worst rank held" value={p.worst_rank_held} min={p.max_open??1} max={cap?.max_rank??cap?.max_universe??120} step={1} onChange={n=>patch({worst_rank_held:n})}/>
      <label className="dfield">Rebalance<select aria-label="Portfolio rebalance" value={p.rebalance??"bar"} onChange={e=>patch({rebalance:e.target.value as PortfolioSpec["rebalance"]})}>{(cap?.rebalance??["bar"]).map(v=><option key={v} value={v}>{v==="bar"?"Every session":v}</option>)}</select></label>
     </SettingGroup>}
    </>
    {!catalogue.execution&&<SettingGroup title="Positions"><NumField label="Max positions" value={p.max_open??1} min={1} max={cap?.max_open??50} step={1} onChange={n=>n!==undefined&&patch({max_open:n})}/></SettingGroup>}
   </SettingsTabs>
  </>}
 </section>;
}

/** A scratch watchlist in the team's AmiBroker databases: Temp…, …Check…, …(IGNORE), "List 62", "123". */
export const scratchList=(name:string)=>/temp|check|ignore|^list\s*\d+$|^\d+$/i.test(name.trim());
/** "C&P DATABASE DATABASE UPDATED TILL 16-10-2026" -> "C&P DATABASE · till 16-10-2026": the folder name without its date suffix. */
export function dbName(folder:string){
 const till=folder.match(/(\d{2}-\d{2}-\d{4})\s*$/)?.[1];
 const name=folder.replace(/[\s_]*updated\s+till[\s_]*\d{2}-\d{2}-\d{4}\s*$/i,"").replace(/\b(DATABASE)(\s+\1)+\b/i,"$1").trim();
 return till?`${name} · till ${till}`:name;
}
/** What Aux1 / Aux2 hold in an AmiBroker database (convert_cp.py): the broad C&P DATABASE flags NSE 500 and F&O; an index
 *  database flags only its index (its Aux2 is never 1, so turning it on would block every buy). */
function auxMeaning(db:string):{aux1?:string;aux2?:string}{
 if(db===VAULT)return {};
 if(/^C&P DATABASE/i.test(db))return {aux1:"in NSE 500 that day",aux2:"in F&O that day"};
 const index=dbName(db).replace(/\s*·.*$/,"").replace(/\s*C&P\b.*$/i,"").trim();
 return {aux1:`in ${index||"the database's index"} that day`};
}
