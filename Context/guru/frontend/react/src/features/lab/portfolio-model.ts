import { comboCount, removeAt, setExecution } from "./spec-model";
import { foreignProblems, type ForeignScope } from "./foreign-model";
import type {Catalogue,Market,PortfolioSpec,Spec} from "./types";
import type {InstrumentOption} from "./instruments";
const object=(v:unknown):v is Record<string,unknown>=>!!v&&typeof v==="object"&&!Array.isArray(v);
export const portfolioShape=(v:unknown):v is PortfolioSpec=>object(v)&&object(v.universe)&&((typeof v.universe.basket==="string"&&!Object.hasOwn(v.universe,"tickers"))||(Array.isArray(v.universe.tickers)&&v.universe.tickers.every(t=>typeof t==="string")&&!Object.hasOwn(v.universe,"basket")));
export const portfolioEnabled=(cat:Catalogue)=>!!cat.portfolio?.modes?.length;
export function withPortfolioMode(s:Spec,mode:"off"|"signals"|"rotation"):Spec {
 if(mode==="off")return removeAt(s,"portfolio");
 // single-instrument AmiBroker settings the portfolio engine refuses (rules.rs portfolio_rules); their controls are hidden there
 const single=s.portfolio?s:setExecution(s,{prices:undefined,delays:undefined,reentry:undefined,limits:undefined,tick_size:undefined,pad_align:undefined});
 return {...single,portfolio:{...s.portfolio,universe:s.portfolio?.universe??{tickers:[]},mode}};
}
export const portfolioIdentity=(s:Spec)=>portfolioShape(s.portfolio)&&s.portfolio.universe.basket?`portfolio:${s.portfolio.universe.basket}`:"portfolio:list";
export const portfolioMembers=(s:Spec,cat:Catalogue)=>(portfolioShape(s.portfolio)?s.portfolio.universe.tickers?.length:undefined)??cat.portfolio?.baskets.find(b=>b.id===(portfolioShape(s.portfolio)?s.portfolio.universe.basket:undefined))?.members;
export const memberOptions=(options:readonly InstrumentOption[],market:Market)=>options.filter(o=>!o.disabled&&["individual_stock","etf"].includes(o.type??"")&&(!o.market||o.market===market));
export function portfolioProblems(s:Spec,cat:Catalogue,market:Market,options?:readonly InstrumentOption[]):string[]{
 const p=s.portfolio;if(!Object.hasOwn(s,"portfolio"))return[];if(!portfolioShape(p))return["Portfolio settings are kept as is; choose exactly one valid basket or ticker list."];const out:string[]=[];const cap=cat.portfolio;
 if(p.mode!==undefined&&!["signals","rotation"].includes(p.mode))out.push("Portfolio mode must be signals or rotation.");
 if(!cap)out.push("Portfolio mode is not advertised by this engine yet; saved settings are kept.");
 if(market==="gl")out.push("Portfolios require admitted India or US stocks and ETFs.");
 if(s.origin)out.push("portfolio + origin is unsupported; keep this book variant single-instrument or explicitly remove its origin.");
 if((s.direction??"long")!=="long"||s.short_side)out.push("Portfolio execution is long-only; change Direction and remove the short side explicitly.");
 if(!(Number.isFinite(s.execution?.capital)&&s.execution!.capital!>0))out.push("Portfolio money mode requires capital above zero in Position size, re-entry and risk.");
 const ex=s.execution??{};
 for(const [name,value]of Object.entries({"entry.limit":s.entry.limit,"entry.stop":s.entry.stop,"entry.scale_in":s.entry.scale_in,"exit.take_profits":s.exit.take_profits?.length?true:undefined,"execution.cooldown":ex.cooldown,"execution.max_trades":ex.max_trades,"execution.risk":ex.risk&&Object.keys(ex.risk).length?ex.risk:undefined,"execution.instrument":(ex.instrument as {kind?:string}|undefined)?.kind==="cash"?undefined:ex.instrument,"execution.cost_bps":ex.cost_bps,"execution.prices":ex.prices,"execution.delays":ex.delays,"execution.reentry":ex.reentry,"execution.limits":ex.limits,"execution.tick_size":ex.tick_size||undefined,"execution.pad_align":ex.pad_align||undefined}))if(value!==undefined)out.push(`${name} is not supported in a portfolio; settings are kept until you edit them.`);
 if((ex.delay_bars??1)!==1)out.push("Portfolio delay_bars must be 1.");
 const n=p.max_open??1;if(!Number.isInteger(n)||n<1||n>(cap?.max_open??50))out.push("max_open is outside the advertised whole-number limit.");
 if(p.worst_rank_held!==undefined&&(p.mode!=="rotation"||!Number.isInteger(p.worst_rank_held)||p.worst_rank_held<n||p.worst_rank_held>(cap?.max_universe??120)))out.push("worst_rank_held is rotation-only and must cover max_open within the universe limit.");
 if(p.rebalance&&p.rebalance!=="bar"&&p.mode!=="rotation")out.push("Rebalance periods apply to rotation only.");
 const names=p.universe?.tickers;
 if(names){if(names.length<2||names.length>(cap?.max_universe_list??50)||new Set(names).size!==names.length)out.push("Choose 2 to the advertised list limit of distinct actual stocks or ETFs.");
  if(options)for(const t of names)if(!memberOptions(options,market).some(o=>o.ticker===t))out.push(`${t} is not an admitted stock or ETF listed by this market.`);
 }else{const b=cap?.baskets.find(b=>b.id===p.universe?.basket);if(!b||b.supported===false||b.members>(cap?.max_universe??120))out.push("Choose an available basket within the advertised universe limit.");}
 if(p.mode==="rotation"){
  if(!p.score)out.push("Rotation needs a score series or formula.");
  if(s.exit.any.length||[s.exit.stop_loss,s.exit.target,s.exit.trailing,s.exit.breakeven].some(x=>x!==undefined))out.push("Rotation uses rank exits: clear exit rules and price exits explicitly.");
 }
 if(p.score){const scope:ForeignScope={catalogue:cat,ticker:"",market,options:options??[]};out.push(...foreignProblems({...s,portfolio:p},scope).filter(x=>x.startsWith("portfolio.score")&&!x.includes("not listed")));}
 return out;
}
export function portfolioGridProblems(s:Spec,cat:Catalogue):string[]{
 if(!s.portfolio)return[];const dims=s.optimize??[];const combos=comboCount(dims);const out:string[]=[];const cap=cat.portfolio;
 if(combos>(cap?.max_combos??200))out.push(`Portfolio optimization allows at most ${cap?.max_combos??200} combinations.`);
 const members=portfolioMembers(s,cat);if(members===undefined)out.push("Choose a verified universe size before optimizing.");else if(members*combos>(cap?.max_work??12000))out.push(`Members × combinations must be at most ${cap?.max_work??12000}.`);
 const dim=(path:string)=>dims.find(d=>d.path===path);const high=dim("portfolio.max_open")?.to??s.portfolio.max_open??1;const low=dim("portfolio.worst_rank_held")?.from??s.portfolio.worst_rank_held;
 if(low!==undefined&&high>low)out.push("Every optimized worst_rank_held must cover every max_open value.");return out;
}
