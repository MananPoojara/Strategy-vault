/** Figure A.7 visual arithmetic only. No orders, account sizing or book/native parity claim. */
import type {IndicatorRequest,PriceHistory,StrategyIndicator,StrategyIndicators} from './strategy-indicators';
export const DEMA_METHOD='hbqts_dema16_8';
export interface DemaSourceArrays {
 first:number[];second:number[];rawBuy:boolean[];reverseCross:boolean[];
 rawBarsSinceBuy:(number|null)[];rawSell:boolean[];filteredSell:boolean[];
}
/** Ordered binary64 recurrence: seed both EMA states at the first actual close.
 * This is deliberately e1+(e1-e2), not an algebraically rearranged2*e1-e2. */
export function firstCloseDema(closes:readonly number[],period:16|8):number[] {
 if(!closes.length||![16,8].includes(period)||closes.some(v=>!Number.isFinite(v)||v<=0))throw Error('Complete finite positive source closes and fixed DEMA16/8 lengths are required.');
 const alpha=2/(period+1);let e1=closes[0],e2=closes[0];const out=[e1];
 for(let i=1;i<closes.length;i++){e1=e1+alpha*(closes[i]-e1);e2=e2+alpha*(e1-e2);const value=e1+(e1-e2);if(!Number.isFinite(value))throw Error('The visual DEMA recurrence is outside finite bounds.');out.push(value);}
 return out;
}
/** Raw Buy always resets the full-source clock. ExRem applies to Sell only. */
export function demaExitClock(rawBuy:readonly boolean[],reverse:readonly boolean[]) {
 if(rawBuy.length!==reverse.length||[...rawBuy,...reverse].some(v=>typeof v!=='boolean'))throw Error('Ordered complete raw signal arrays are required.');
 let last:number|null=null,armed=true;const rawBarsSinceBuy:(number|null)[]=[],rawSell:boolean[]=[],filteredSell:boolean[]=[];
 for(let i=0;i<rawBuy.length;i++){
  if(rawBuy[i])last=i;const clock=last===null?null:i-last;
  const sell=reverse[i]||(clock!==null&&clock>=1),emit=armed&&sell;
  if(emit)armed=false;if(rawBuy[i])armed=true;
  rawBarsSinceBuy.push(clock);rawSell.push(sell);filteredSell.push(emit);
 }
 return {rawBarsSinceBuy,rawSell,filteredSell};
}
export function demaSourceArrays(closes:readonly number[]):DemaSourceArrays {
 const first=firstCloseDema(closes,16),second=firstCloseDema(closes,8);
 const rawBuy=first.map((v,i)=>i>0&&v>second[i]&&first[i-1]<=second[i-1]);
 const reverseCross=first.map((v,i)=>i>0&&second[i]>v&&second[i-1]<=first[i-1]);
 return {first,second,rawBuy,reverseCross,...demaExitClock(rawBuy,reverseCross)};
}
export function demaParameters(p:Readonly<Record<string,number>>):boolean {
 return !!p&&typeof p==='object'&&!Array.isArray(p)&&Object.keys(p).length===3&&p.fast===16&&p.slow===8&&p.hold===1;
}
export function demaPriceIdentity(px:PriceHistory|null,ticker?:string):px is PriceHistory&{ticker:string;full_history:true} {
 const raw=px as (PriceHistory&{ticker?:string;full_history?:boolean;provenance?:{asset_type?:string}})|null;
 return !!raw&&raw.full_history===true&&['SPX','NIFTY 50'].includes(raw.ticker||'')&&(!ticker||raw.ticker===ticker)
  &&(!raw.provenance?.asset_type||raw.provenance.asset_type==='index')&&Array.isArray(raw.d)&&raw.d.length>0
  &&[raw.o,raw.h,raw.l,raw.c].every(Array.isArray)
  &&[raw.o,raw.h,raw.l,raw.c].every(v=>v.length===raw.d.length&&v.every(n=>Number.isFinite(n)&&n>0))
  &&raw.d.every((d,i)=>/^\d{4}-\d{2}-\d{2}$/.test(d)&&Number.isFinite(Date.parse(d))&&new Date(d).toISOString().slice(0,10)===d&&(!i||d>raw.d[i-1])
    &&raw.l[i]<=Math.min(raw.o[i],raw.c[i])&&raw.h[i]>=Math.max(raw.o[i],raw.c[i]))
  &&(raw.ticker!=='NIFTY 50'||raw.d[0]>='2000-01-01');
}
export function demaSeries(px:PriceHistory,arrays:DemaSourceArrays,displayDates:readonly string[]=px.d):StrategyIndicator[] {
 const byDate=new Map(px.d.map((d,i)=>[d,i]));
 if(displayDates.some((d,i)=>!byDate.has(d)||i>0&&d<=displayDates[i-1]))throw Error('Displayed dates must be actual ordered full-source sessions.');
 const add=(key:string,label:string,kind:StrategyIndicator['kind'],placement:'price'|'pane',values:readonly(number|null)[],thresholds:StrategyIndicator['thresholds']=[]):StrategyIndicator=>({key,label,kind,placement,thresholds,points:displayDates.map(time=>({time,value:values[byDate.get(time)!]}))});
 return [add('dema-first','DEMA(16) · first-close seed','dema','price',arrays.first),add('dema-second','DEMA(8) · first-close seed','dema','price',arrays.second),
  add('dema-raw-clock','Bars since raw Buy','sum','pane',arrays.rawBarsSinceBuy,[{value:1,label:'Raw clock exit threshold',kind:'exit'}])];
}
export function buildDemaIndicators(request:IndicatorRequest):StrategyIndicators {
 const unavailable=(note:string):StrategyIndicators=>({status:'unavailable',series:[],note});
 if(request.id!==DEMA_METHOD||request.book!=='hbqts'||!request.fullHistory||!request.ticker||!demaPriceIdentity(request.prices,request.ticker)||!demaParameters(request.parameters))
  return unavailable('This DEMA16/8 overlay requires the exact reviewed book, fixed16/8/1 parameters and complete actual SPX or NIFTY 50 source history. Sliced prices or other instruments are not substituted.');
 try{return {status:'ready',series:demaSeries(request.prices,demaSourceArrays(request.prices.c),request.displayDates),note:'Figure A.7 DEMA first length16, second length8: both EMA states seed at the first actual source close; DEMA = e1 + (e1 − e2). Full-prefix raw Buy resets BarsSince; Sell alone is filtered by ExRem. Weekly/monthly views sample the daily value at their actual final session. These calculated rule inputs do not establish book performance or native AmiBroker parity.'};}
 catch(error){return unavailable(error instanceof Error?error.message:'The exact DEMA visual inputs are unavailable.');}
}
