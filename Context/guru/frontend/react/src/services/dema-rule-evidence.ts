import type {RuleCheck,RuleEvidence,RuleVisualModel} from './rule-evidence';
import type {PriceHistory} from './strategy-indicators';
import {DEMA_METHOD,demaParameters,demaPriceIdentity,demaSeries,demaSourceArrays} from './dema-visual';
import {UNIT_EXPOSURE_CONVENTION} from './unit-exposure-report';
const unavailable=(note:string):RuleEvidence=>({scope:'unavailable',title:'DEMA rule calculation unavailable',note,series:[],at:()=>[]});
const numeric=(label:string,explanation:string,value:number|null):RuleCheck=>({label,explanation,value,passed:null});
const condition=(label:string,explanation:string,passed:boolean|null):RuleCheck=>({label,explanation,value:passed===null?null:passed?1:0,passed});
export function buildDemaRuleEvidence(model:RuleVisualModel,px:PriceHistory):RuleEvidence|null {
 if(model.id!==DEMA_METHOD)return null;
 if(model.book!=='hbqts'||model.status!=='ready'||model.blocked||model.basket||model.focusTicker
  ||model.prices!==px||model.prices?.full_history!==true||!demaPriceIdentity(px)||!demaParameters(model.parameters)
  ||!model.sourceQualifications.includes(UNIT_EXPOSURE_CONVENTION))
  return unavailable('The exact reviewed DEMA16/8 index context, fixed16/8/1 parameters and complete source history are required. Stock, ETF, sliced-price and other-source traces cannot substitute.');
 let arrays;try{arrays=demaSourceArrays(px.c);}catch(error){return unavailable(error instanceof Error?error.message:'The source recurrence is unavailable.');}
 const byDate=new Map(px.d.map((d,i)=>[d,i])),selected=new Set(model.dates),start=byDate.get(model.dates[0]);
 const datesValid=start!==undefined&&model.dates.every((d,i)=>byDate.get(d)===start+i);
 const ledger=model.ledgerAvailable&&datesValid&&model.trades.every((t,i)=>{
  const entry=byDate.get(t.entry),exit=byDate.get(t.exit);
  return t.ticker===px.ticker&&t.direction===1&&t.entryKind==='close'&&t.exits.length===0&&t.legs.length===1
   &&entry!==undefined&&exit!==undefined&&selected.has(t.entry)&&selected.has(t.exit)
   &&t.legs[0].date===t.entry&&t.legs[0].weight===1&&t.legs[0].price===px.c[entry]&&t.exitPrice===px.c[exit]
   &&(t.open?i===model.trades.length-1&&exit===entry&&t.exit===model.dates.at(-1):exit===entry+1)
   &&(!i||!model.trades[i-1].open&&model.trades[i-1].exit<t.entry);
 });
 const note='Figure A.7: Buy = Cross(DEMA(C,16), DEMA(C,8)). Raw Sell = reverse Cross OR BarsSince(raw Buy) ≥1; Sell = ExRem(raw Sell, raw Buy). Buy is not de-duplicated. Both EMA states seed at the first source close; ordered DEMA = e1 + (e1 − e2). Full source formula state warms before the selected account starts flat. Every raw Buy resets the clock; this is not an entry-relative stop. The chart uses declared daily same-close fills. Source conditions, permission and actual recorded arrows remain separate; a terminal-open mark is not a Sell fill. SPX is the source cash-index illustration; NIFTY 50 is a declared adaptation. Book performance and native AmiBroker parity are not reproduced.'
  +(ledger?'':' The accepted position ledger could not bind this trace; position permission and recorded-fill checks remain unknown.');
 return {scope:'exact',title:'Follow the DEMA crossing and raw-Buy clock',note,series:demaSeries(px,arrays),at:i=>{
  if(!Number.isInteger(i)||i<0||i>=px.d.length)return [];
  const known=ledger&&selected.has(px.d[i]),before=known?model.trades.some(t=>t.entry<px.d[i]&&(t.open||t.exit>=px.d[i])):null;
  return [numeric('First DEMA(16)','First printed length16; first-close seed, not a mean seed or reordered pair.',arrays.first[i]),numeric('Second DEMA(8)','Second printed length8, retaining its printed order in Cross.',arrays.second[i]),
   condition('Raw Buy crossover','First DEMA today > second today, and first yesterday ≤ second yesterday. Equality alone does not cross.',arrays.rawBuy[i]),
   condition('Reverse crossover','Second DEMA today > first today, and second yesterday ≤ first yesterday.',arrays.reverseCross[i]),
   numeric('Bars since raw Buy','Clock starts at each raw Buy over the entire source prefix, even before the selected flat account begins.',arrays.rawBarsSinceBuy[i]),
   condition('Raw Sell condition','Reverse Cross OR raw-Buy clock ≥1; no invented entry-relative holding clock.',arrays.rawSell[i]),
   condition('Formula Sell after Sell-only ExRem','First raw Sell since the relevant raw Buy reset. The Buy array remains raw.',arrays.filteredSell[i]),
   numeric('Source position before close','Accepted earlier long holding that has not exited before this close. Selected period starts flat.',before===null?null:before?1:0),
   condition('Source entry permitted','Requires raw Buy and an existing accepted ledger proving flat before the close.',!arrays.rawBuy[i]?false:before===null?null:!before),
   condition('Source exit permitted','The filtered formula Sell can close only an existing long position.',!arrays.filteredSell[i]?false:before===null?null:before),
   condition('Recorded entry fill','Read from the actual accepted close-entry leg; never manufactured from a raw condition.',known?model.trades.some(t=>t.entry===px.d[i]):null),
   condition('Recorded exit fill','Completed accepted exit only; a terminal-open mark stays unfilled.',known?model.trades.some(t=>!t.open&&t.exit===px.d[i]):null)];
 }};
}
