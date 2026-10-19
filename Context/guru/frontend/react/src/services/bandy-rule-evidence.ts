/** Bounded visual trace of reviewed QTS companion formulas. This reads existing
 * prices and accepted positions only; it never executes orders or certifies a
 * book, native AmiBroker parity, source population, or reported performance. */
import type {RuleCheck,RuleEvidence,RuleVisualModel,VisualTrade} from './rule-evidence';
import type {PriceHistory,StrategyIndicator} from './strategy-indicators';

type Value=number|null;
type Truth=boolean|null;
type Context=RuleVisualModel&{book?:string;ledgerAvailable?:boolean;focusTicker?:string};
const ids=new Set(['hbqts_stoch_rsi','hbqts_ma5_25','hbqts_ma1_36','hbqts_fixed_interval','hbqts_entry_id1']);
const finite=(v:unknown):v is number=>typeof v==='number'&&Number.isFinite(v);
const value=(v:unknown):Value=>finite(v)?v:null;
const object=(v:unknown):Record<string,unknown>=>v!==null&&typeof v==='object'&&!Array.isArray(v)?v as Record<string,unknown>:{};
const day=(v:unknown):v is string=>typeof v==='string'&&/^\d{4}-\d{2}-\d{2}$/.test(v);
const unavailable=(note:string):RuleEvidence=>({scope:'unavailable',title:'Bandy rule values unavailable',note,series:[],at:()=>[]});
const numeric=(label:string,explanation:string,v:Value,comparison?:string):RuleCheck=>({label,explanation,value:v,comparison,passed:null});
const condition=(label:string,explanation:string,passed:Truth,comparison?:string):RuleCheck=>({label,explanation,value:passed===null?null:passed?1:0,comparison,passed});

/** QTS owner's fresh-window MA, not the shared running-sum indicator helper. */
function average(c:readonly number[],n:number):Value[]{return c.map((_,i)=>i+1<n?null:c.slice(i+1-n,i+1).reduce((a,b)=>a+b,0)/n);}
function cross(a:readonly Value[],b:readonly Value[]):Truth[]{return a.map((v,i)=>i===0||v===null||b[i]===null||a[i-1]===null||b[i-1]===null?null:v>b[i]!&&a[i-1]!<=b[i-1]!);}
/** Missing output stays missing. The last finite recursion state is retained. */
function smooth(v:readonly Value[],alpha:number):Value[]{let state:Value=null;return v.map(x=>{if(x===null)return null;state=state===null?x:state+alpha*(x-state);return value(state);});}
function dema(v:readonly Value[],n:number):Value[]{const a=smooth(v,2/(n+1)),b=smooth(a,2/(n+1));return a.map((x,i)=>x===null||b[i]===null?null:value(x+(x-b[i]!)));}
/** Published RSIa recurrence: P=N=0 at bar1; first ratio at period. */
function rsi(c:readonly number[],n:number):Value[]{let up=0,down=0;const out:Value[]=Array(c.length).fill(null);for(let i=1;i<c.length;i++){const change=c[i]-c[i-1];up=((n-1)*up+Math.max(change,0))/n;down=((n-1)*down+Math.max(-change,0))/n;if(i>=n&&up+down>0)out[i]=value(100*up/(up+down));}return out;}
function extreme(v:readonly Value[],n:number,high:boolean):Value[]{return v.map((_,i)=>{if(i+1<n)return null;const window=v.slice(i+1-n,i+1);return window.some(x=>x===null)?null:(high?Math.max(...window as number[]):Math.min(...window as number[]));});}
function barsSince(raw:readonly Truth[]):Value[]{let last:number|null=null;return raw.map((hit,i)=>{if(hit===true)last=i;return last===null?null:i-last;});}
function exrem(set:readonly Truth[],reset:readonly Truth[]):boolean[]{let armed=true;return set.map((hit,i)=>{const emit=armed&&hit===true;if(emit)armed=false;if(reset[i]===true)armed=true;return emit;});}

function stockIdentity(model:Context,px:PriceHistory):string|null {
 const raw=object(px),source=object(raw.provenance),qualified=source.qualified_ticker;
 if(source.asset_type!=='individual_stock'||typeof qualified!=='string'||!/^basket:[a-zA-Z0-9_]+:[^\s/]+$/.test(qualified)
  ||typeof source.symbol!=='string'||raw.ticker!==source.symbol
  ||![source.source_sha256,source.csv_sha256].every(v=>typeof v==='string'&&/^[a-f0-9]{64}$/i.test(v)))return null;
 if(model.focusTicker&&model.focusTicker!==qualified)return null;
 if(!model.focusTicker&&!model.basket){const tickers=new Set(model.trades.map(t=>t.ticker).filter(Boolean));if(tickers.size>1||tickers.size===1&&!tickers.has(qualified))return null;}
 return qualified;
}
function validPrices(px:PriceHistory):boolean{return px.d.length>0&&[px.o,px.h,px.l,px.c].every(v=>v.length===px.d.length&&v.every(x=>finite(x)&&x>0))&&px.d.every((d,i)=>day(d)&&(!i||d>px.d[i-1])&&px.h[i]>=Math.max(px.o[i],px.l[i],px.c[i])&&px.l[i]<=Math.min(px.o[i],px.h[i],px.c[i]));}

/** The central dispatcher validates full source history first. These local checks
 * also make direct callers fail closed on sliced or substituted instruments. */
export function buildBandyRuleEvidence(model:Context,px:PriceHistory):RuleEvidence|null {
 if(!ids.has(model.id))return null;
 if(model.book!=='hbqts')return unavailable('The exact Bandy QTS book identity is missing or mismatched. A familiar method identifier cannot inherit another source’s trace.');
 if(model.status!=='ready'||model.blocked||model.prices?.full_history!==true||object(px).full_history!==true||!validPrices(px))return unavailable('This stock trace requires a ready run and complete, ordered actual source OHLC, including all warmup. Missing prices are not repaired or reseeded.');
 const ticker=stockIdentity(model,px);if(!ticker)return unavailable('The reviewed stock formulas require source-qualified individual-stock prices for this exact instrument. An index, ETF, another stock or unqualified array is not substituted.');
 const series:StrategyIndicator[]=[],c=px.c;
 const add=(key:string,label:string,values:readonly Value[],placement:'price'|'pane'='price',thresholds:StrategyIndicator['thresholds']=[])=>series.push({key,label,placement,kind:placement==='price'?'sma':key==='bandy-rsi'?'rsi':'sum',thresholds,points:px.d.map((time,i)=>({time,value:value(values[i])}))});
 const period=(key:string)=>{const raw=model.parameters[key];if(!finite(raw))throw Error(`Selected Bandy parameter “${key}” is unavailable.`);const n=Math.max(1,Math.round(raw));if(n>10000)throw Error('The selected Bandy period exceeds this bounded visual trace.');return n;};
 let rawBuy:Truth[]=[],rawSell:Truth[]=[],buy:Truth[]=[],sell:Truth[]=[],details:(i:number)=>RuleCheck[]=()=>[],title='',note='';
 try {
  if(model.id==='hbqts_stoch_rsi'){
   const n=period('rsi'),lb=period('lb'),trig=period('trig'),osc=rsi(c,n),hi=extreme(osc,lb,true),lo=extreme(osc,lb,false);
   const pir=osc.map((r,i)=>r===null||hi[i]===null||lo[i]===null||hi[i]===lo[i]?null:value((r-lo[i]!)/(hi[i]!-lo[i]!))),trigger=dema(pir,trig);
   rawBuy=cross(pir,trigger);rawSell=cross(trigger,pir);buy=rawBuy;sell=rawSell;
   add('bandy-rsi',`RSIa(${n}) · declared recurrence`,osc,'pane');add('bandy-pir',`RSI position in ${lb}-bar range · fraction`,pir,'pane',[{value:0,label:'Range low',kind:'reference'},{value:1,label:'Range high',kind:'reference'}]);add('bandy-trigger',`DEMA(${trig}) of RSI range fraction`,trigger,'pane',[{value:0,label:'Fraction 0',kind:'reference'},{value:1,label:'Fraction 1',kind:'reference'}]);
   title='Trade the crossing of RSI range position and its trigger';note=`Figure 10.3: RSIa(${n}), ${lb}-bar full-window RSI high/low, then first-finite DEMA(${trig}). RSI starts P=N=0 at source bar 1 and first displays at its period. The range position is a fraction, not RSI itself or a fixed 20/80 test. Zero ranges and incomplete windows stay missing. DEMA retains its last finite state through missing values but does not fill their output; Cross requires two adjacent finite pairs. Buy is Cross(range position, trigger); Sell is the reverse Cross. No ExRem is present in this companion formula.`;
   details=i=>[numeric('RSI value','Published P=N=0 RSIa recurrence; this is not the generic mean-seeded Wilder RSI.',osc[i]),numeric('RSI range low','Full-window LLV of RSI; any missing input leaves the window missing.',lo[i]),numeric('RSI range high','Full-window HHV of RSI.',hi[i]),numeric('RSI range fraction','(RSI − low) / (high − low). A zero denominator is undefined.',pir[i]),numeric('DEMA trigger','Twice the first smoothing minus the second, seeded from the first finite range fraction.',trigger[i])];
  }else if(model.id==='hbqts_fixed_interval'){
   const every=period('every'),hold=period('hold');rawBuy=c.map((_,i)=>i%every===0);const since=barsSince(rawBuy);rawSell=since.map(v=>v===null?null:v>=hold);buy=rawBuy;sell=exrem(rawSell,rawBuy);
   add('bandy-bars-since','Bars since most recent raw interval Buy',since,'pane',[{value:hold,label:'Formula exit threshold',kind:'exit'}]);title='Follow the full-source bar clock';note=`Figure 7.6: Buy when the absolute loaded source BarIndex modulo ${every} equals zero. Sell is ExRem(BarsSince(raw Buy) ≥ ${hold}, raw Buy). The clock and Sell de-duplication start at the first genuine source row, not the selected From date. Repeated raw interval Buys reset the clock while held without adding exposure. This is a formula exit, not an ApplyStop N-bar stop.`;
   details=i=>[numeric('Source BarIndex','Zero belongs to the first actual loaded quote; selected dates do not restart it.',i),numeric('Interval remainder',`Source BarIndex modulo ${every}.`,i%every),numeric('Bars since raw Buy','Every raw interval Buy resets this clock, including while a position is held.',since[i])];
  }else{
   const fast=period('fast'),slow=period('slow'),a=average(c,fast),b=average(c,slow);rawBuy=cross(a,b);rawSell=cross(b,a);buy=rawBuy;sell=rawSell;
   add('bandy-fast',`SMA(${fast}) · fresh-window sum`,a);add('bandy-slow',`SMA(${slow}) · fresh-window sum`,b);title='Act on a crossing of the two simple averages';
   note=`The reviewed ${model.id==='hbqts_ma1_36'?'Figure 13.2 active 1/36':'Figure 7.1 5/25'} example uses simple moving averages. This view traces the resolved selected lengths ${fast}/${slow}, summed afresh oldest to newest. Buy requires fast today > slow today and fast yesterday ≤ slow yesterday; Sell is the reverse crossing. Equality alone does not cross. Native AmiBroker export parity is not certified.`;
   details=i=>[numeric('Fast average','Fresh full-window arithmetic mean of actual source closes.',a[i]),numeric('Slow average','No selected-window restart or partial-window average.',b[i]),numeric('Fast minus slow','A crossing needs both today and the preceding finite pair.',a[i]===null||b[i]===null?null:a[i]!-b[i]!)];
   if(model.id==='hbqts_entry_id1'){
    const hold=period('hold'),since=barsSince(rawBuy);rawSell=since.map(v=>v===null?null:v>=hold);buy=exrem(rawBuy,rawSell);sell=exrem(rawSell,buy);add('bandy-bars-since','Bars since most recent raw MA Buy',since,'pane',[{value:hold,label:'Formula exit threshold',kind:'exit'}]);
    title='Separate raw entry identification from the holding clock';note=`Figure 7.14: resolved SMA(${fast})/SMA(${slow}) upward Cross is raw Buy. Raw Sell is BarsSince(raw Buy) ≥ ${hold}; then assign Buy = ExRem(raw Buy, raw Sell), followed by Sell = ExRem(raw Sell, cleaned Buy). A repeated raw Cross while held restarts the clock without adding a position. Both arrays warm over the complete input before the selected account starts flat. No opposite-MA exit or ApplyStop is added.`;
    const previous=details;details=i=>[...previous(i),numeric('Bars since raw Buy','The clock follows raw Cross events, including events suppressed by ExRem.',since[i])];
   }
  }
 }catch(error){return unavailable(error instanceof Error?error.message:'The selected Bandy formula parameters are unavailable.');}
 const isBasket=!!model.focusTicker||model.basket,selected=new Set(model.dates);
 const matching=model.trades.filter(t=>t.ticker===ticker||!t.ticker&&t.symbol===object(object(px).provenance).symbol);
 const validTrade=(t:VisualTrade)=>t.direction===1&&t.entryKind==='close'&&t.exits.length===0&&finite(t.exitPrice)&&t.exitPrice>0&&day(t.entry)&&day(t.exit)&&t.entry<=t.exit&&t.legs.length===1&&t.legs[0].date===t.entry&&finite(t.legs[0].price)&&t.legs[0].price>0&&t.legs[0].weight===1&&selected.has(t.entry)&&selected.has(t.exit)&&(!t.open||t.exit===model.dates.at(-1));
 const ledger=model.ledgerAvailable===true&&matching.every((t,i)=>validTrade(t)&&(!i||!matching[i-1].open&&matching[i-1].exit<t.entry));
 note+=' These are calculated source-formula inputs, not new orders or source-performance certification. All five methods are long/cash. Same-bar close fills are the declared research convention; Figure 13.2 omits delays/prices and uses the reviewed zero-delay Close selection. Formula history is warmed, the selected single-stock account starts flat, and terminal holdings remain open. Arrows remain actual accepted fills from the existing ledger.';
 if(isBasket)note+=' In a basket constituent view the ledger establishes accepted exposure only. Capacity-rejected source positions and controller decisions are not supplied; accepted flat exposure cannot prove source-flat entry permission.';
 if(!ledger)note+=' The accepted close-position ledger is unavailable or cannot bind this trace, so position permission and recorded fill checks remain unknown.';
 return {scope:'exact',title,note,series,at:i=>{
  if(!Number.isInteger(i)||i<0||i>=c.length)return [];
  const inRun=selected.has(px.d[i]),known=ledger&&inRun,before=known?Number(matching.some(t=>t.entry<px.d[i]&&(t.open||t.exit>=px.d[i]))):null;
  const rawLabel=model.id==='hbqts_fixed_interval'?'Raw interval entry':'Raw buy crossover';
  const checks=[...details(i),condition(rawLabel,'Raw companion condition only. A true value does not itself prove entry permission or an accepted fill.',rawBuy[i]),condition('Raw sell condition','Raw reverse Cross or raw BarsSince threshold before de-duplication.',rawSell[i])];
  if(model.id==='hbqts_entry_id1')checks.push(condition('Formula buy after ExRem','First raw Buy since the raw Sell reset; formula-array state is warmed before the selected period.',rawBuy[i]===null?null:buy[i]));
  if(model.id==='hbqts_entry_id1'||model.id==='hbqts_fixed_interval')checks.push(condition('Formula sell after ExRem','Sequential source assignment retains the first Sell until the relevant Buy reset.',rawSell[i]===null?null:sell[i]));
  const enter=buy[i]===null?null:!buy[i]?false:before===null?null:before===1?false:isBasket?null:true;
  const exit=sell[i]===null?null:!sell[i]?false:before===null||isBasket?null:before===1;
  checks.push(numeric(isBasket?'Accepted exposure before close':'Source position before close',isBasket?'1 means this constituent has an accepted holding before today’s close. Zero is not proof that its underlying source runner was flat.':'1 means an accepted source holding entered earlier and has not exited before this close. Same-close entries are not pre-existing holdings.',before),condition('Source entry permitted',isBasket?'Requires the cleaned source Buy, source-flat state and portfolio/controller permission. Accepted exposure can block an entry; accepted flat cannot establish all remaining gates.':'The cleaned Buy can enter only when the accepted single-stock source account was flat before the close.',enter),condition('Source exit permitted',isBasket?'The source and portfolio controller states are not reconstructed from an accepted basket ledger.':'The formula Sell applies only to an existing long holding before the close.',exit),condition('Recorded entry fill','Read from an actual accepted close-entry leg; never manufactured from a raw condition.',known?matching.some(t=>t.legs.some(l=>l.date===px.d[i])):null),condition('Recorded exit fill','Read only from a completed accepted exit. A terminal open mark is not an exit fill.',known?matching.some(t=>!t.open&&t.exit===px.d[i]):null));
  return checks;
 }};
}
