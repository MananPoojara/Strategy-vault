import {directionalPlus,simpleMovingAverage,wilderRsi,type PriceHistory,type StrategyIndicator} from './strategy-indicators';
import type {RuleCheck,RuleEvidence,RuleVisualModel,VisualTrade} from './rule-evidence';

const ids=new Set(['r3','rsi2_4','rsi4_25','spmt_rsi2_4','spmt_rsi4_25',...Array.from({length:5},(_,i)=>`dmi_${i+1}`),...Array.from({length:5},(_,i)=>`spmt_dmi_${i+1}`)]);
const finite=(v:number|null|undefined):v is number=>v!=null&&Number.isFinite(v);
const value=(v:number|null|undefined)=>finite(v)?v:null;
const condition=(label:string,explanation:string,v:number|null|undefined,comparison:string,test:(v:number)=>boolean):RuleCheck=>({label,explanation,value:value(v),comparison,passed:finite(v)?test(v):null});
const context=(label:string,explanation:string,v:number|null|undefined):RuleCheck=>({label,explanation,value:value(v),passed:null});
const unavailable=(note:string):RuleEvidence=>({scope:'unavailable',title:'Selected Connors rule inputs unavailable',note,series:[],at:()=>[]});

/** Arithmetic/predicate explanation of the existing runners, not a source-book
 * re-review or an alternative order generator. Recorded fills remain authoritative. */
export function buildConnorsRuleEvidence(model:RuleVisualModel,prices:PriceHistory):RuleEvidence|null {
  if(!ids.has(model.id))return null;
  const spmt=model.id.startsWith('spmt_'),id=model.id.replace(/^spmt_/,''),c=prices.c,p=model.parameters;
  if(model.book!==(spmt?'spmt':'stc9'))return unavailable('The selected source book identity is missing or does not match this reviewed engine provider. No source-specific conditions are inferred from a strategy ID alone.');
  const parameter=(key:string)=>{if(!Object.hasOwn(p,key)||!Number.isFinite(p[key]))throw Error(`Selected parameter “${key}” is unavailable.`);return p[key];};
  const period=(key:string)=>{const n=Math.max(1,Math.round(parameter(key)));if(n>10000)throw Error(`Selected period “${key}” exceeds the bounded trace.`);return n;};
  const series:StrategyIndicator[]=[];
  const add=(key:string,label:string,values:(number|null)[],kind:StrategyIndicator['kind'],thresholds:StrategyIndicator['thresholds']=[])=>series.push({key,label,points:prices.d.map((time,i)=>({time,value:value(values[i])})),placement:kind==='sma'?'price':'pane',kind,thresholds});
  const dates=new Set(model.dates),finalDate=model.dates.at(-1);
  const relevantTrades=model.focusTicker?model.trades.filter(t=>t.ticker===model.focusTicker):model.trades;
  const sourceIndex=new Map(prices.d.map((day,i)=>[day,i])),heldBefore:Array<VisualTrade|undefined>=Array(c.length);
  // These fifteen close-only long runners have one holding at a time. Keep
  // this stricter state contract local: other engines support reversals/pieces.
  const byTicker=new Map<string,string>();
  const sourceLedgerValid=model.ledgerAvailable&&relevantTrades.every(trade=>{
    const previousExit=byTicker.get(trade.ticker);byTicker.set(trade.ticker,trade.exit);
    return trade.direction===1&&trade.entryKind==='close'&&sourceIndex.has(trade.entry)&&sourceIndex.has(trade.exit)
      &&trade.legs.every(leg=>sourceIndex.has(leg.date))&&trade.exits.length===0&&finite(trade.exitPrice)&&trade.exitPrice>0
      &&(!previousExit||trade.entry>previousExit)
      &&(id==='r3'?trade.legs.length>=1&&trade.legs.length<=2&&trade.legs.every((leg,i)=>leg.weight===.5&&(!i||leg.date>trade.legs[i-1].date))
        :trade.legs.length===1&&trade.legs[0].weight===1);
  });
  // A single-symbol source runner has at most one position. Prepare its actual
  // held-bar lookup once rather than scanning a large ledger on every hover.
  for(const trade of relevantTrades){const first=sourceIndex.get(trade.entry),last=sourceIndex.get(trade.exit);if(first!==undefined&&last!==undefined)for(let i=first+1;i<=last;i++)heldBefore[i]=trade;}
  const beforeClose=(i:number):VisualTrade|undefined=>heldBefore[i];
  const positionChecks=(i:number):RuleCheck[]=>{
    const known=sourceLedgerValid&&dates.has(prices.d[i]);
    return [condition('Position gate · no holding before this close',model.focusTicker?'Accepted basket exposure can establish that a position is already held. No accepted holding does not prove the independent source runner is flat: a capacity-rejected source trade may still be open. That source-flat permission remains unknown.':'Calculated from this run’s recorded position ledger. An existing position, including its exit close, blocks another entry. This is separate from the price setup.',known?(beforeClose(i)?0:model.focusTicker?null:1):null,'1 = source flat; 0 = already holding',v=>v===1),
      ...(model.focusTicker?[context('Accepted basket exposure','1 = this constituent has an accepted position before the close; 0 = no accepted position. This is not its complete source-runner state.',known?(beforeClose(i)?1:0):null)]:[]),
      condition('Entry-window gate',spmt?'The SPMT close-only runner allows an entry on its last selected close and retains it as open.':'The shared STC IX runner does not open a new position on its last selected close. A final-day setup alone is not a recorded trade.',dates.has(prices.d[i])?(spmt||prices.d[i]!==finalDate?1:0):null,'1 = this close can receive a new entry',v=>v===1)];
  };
  const note='This view calculates declared engine predicates from complete source history and selected parameters; it does not independently re-verify the source book or its published figures. Each “Met” label describes one test, not the whole entry or exit. Displayed values are rounded, while comparisons use unrounded calculations. Recorded position state, the run window and execution timing still govern fills. '+(spmt?'SPMT starts flat, allows a qualifying final-close entry, and does not re-enter on an exit close.':'STC IX starts flat, does not enter on the final selected close, and does not re-enter on an exit close.');
  try{
    const trendN=period('trend'),rsiN=period('rsi_n'),exitLevel=parameter('exit_rsi');
    const trend=simpleMovingAverage(c,trendN),rsi=wilderRsi(c,rsiN);
    add('trend',`SMA(${trendN})`,trend,'sma');
    add('rsi',`RSI(${rsiN}) · exit / entry cap`,rsi,'rsi',[{value:exitLevel,label:'Exit above',kind:'exit'}]);
    const trendTest=(i:number)=>condition(`Price trend · above SMA(${trendN})`,'Today’s close must be strictly above the selected trend average. Equality does not pass.',trend[i]===null?null:c[i]-trend[i]!,'close − SMA > 0',v=>v>0);
    const exitTest=(i:number)=>condition(`Exit price test · RSI(${rsiN})`, 'Strictly above the exit threshold. It closes only an already held position; it is not an entry signal.',rsi[i],`> ${exitLevel}`,v=>v>exitLevel);
    const capTest=(i:number)=>condition(`Entry cap · RSI(${rsiN})`,'RSI equal to the selected exit threshold is allowed for entry. This cap and the strict exit test are different comparisons.',rsi[i],`≤ ${exitLevel}`,v=>v<=exitLevel);
    const wrap=(title:string,extraNote:string,at:(i:number)=>RuleCheck[]):RuleEvidence=>({scope:'exact',title,note:note+' '+extraNote,series,at:i=>i<0||i>=c.length?[]:at(i)});
    if(/^dmi_[1-5]$/.test(id)){
      const diN=period('di_n'),days=period('days'),diLevel=parameter('di_lt');
      const di=directionalPlus(prices,diN,spmt);let count=0;
      const consecutive=di.map(v=>{count=finite(v)&&v<diLevel?count+1:0;return count;});
      add('plus-di',`+DI(${diN}) · ${days} consecutive bars`,di,'di',[{value:diLevel,label:'Entry below',kind:'entry'}]);
      return wrap('Look for weak upward movement inside an uptrend',`+DI uses Wilder-smoothed positive directional movement divided by smoothed true range, seeded from the first ${diN} interbar changes. ${spmt?'SPMT treats up/down movement differences within its EPSILON-scaled subtraction error bound as ties; this is a reviewed local numerical convention.':'STC IX uses its unchanged strict floating-point dominant-movement comparison; no SPMT tie correction is inherited.'} The entire last ${days}-bar window must be below ${diLevel}; one low reading is insufficient when the selected count is greater than one. This is +DI, not ADX. Entry and exit fills in these runners are at the same completed daily close.`,i=>[
        trendTest(i),
        condition(`Upward-movement test · +DI(${diN})`,'This is the current bar’s indicator input. The consecutive-bar test below decides whether the whole required window qualifies.',di[i],`< ${diLevel}`,v=>v<diLevel),
        condition(`Consecutive +DI tests · last ${days} bars`,'Count of consecutive qualifying actual source bars, ending today. Missing or warmup values break the run; sessions are not filled.',i+1>=days&&di.slice(i+1-days,i+1).every(finite)?consecutive[i]:null,`at least ${days} consecutive bars`,v=>v>=days),
        capTest(i),...positionChecks(i),exitTest(i),
      ]);
    }
    if(id==='rsi2_4'){
      const entry=parameter('rsi_lt');series[1].thresholds.push({value:entry,label:'Entry below',kind:'entry'});
      return wrap(`Buy a deep RSI(${rsiN}) pullback in an uptrend`,'Entry requires the trend test and RSI strictly below the selected entry threshold. There is no additional RSI entry-cap predicate in this runner. The exit uses its separately selected RSI threshold. Same-close fills are the declared execution convention.',i=>[
        trendTest(i),condition(`Oversold test · RSI(${rsiN})`,'Use the selected RSI period; the catalogue name describes the default. Equality to the entry level does not pass.',rsi[i],`< ${entry}`,v=>v<entry),...positionChecks(i),exitTest(i),
      ]);
    }
    if(id==='rsi4_25'){
      const slowN=period('slow_n'),slowLevel=parameter('slow_lt'),slow=wilderRsi(c,slowN);
      add('slow-rsi',`Entry RSI(${slowN})`,slow,'rsi',[{value:slowLevel,label:'Entry below',kind:'entry'}]);
      return wrap('Combine a slower oversold reading with the fast RSI cap',`The slower RSI(${slowN}) must be below ${slowLevel}; RSI(${rsiN}) must be at or below ${exitLevel} for entry. The exit uses RSI(${rsiN}), not the slower entry RSI. The method’s default name does not override selected periods. Same-close fills are the declared convention.`,i=>[
        trendTest(i),condition(`Slower entry test · RSI(${slowN})`,'This slower RSI supplies the oversold entry input; it is not the exit indicator.',slow[i],`< ${slowLevel}`,v=>v<slowLevel),capTest(i),...positionChecks(i),exitTest(i),
      ]);
    }
    if(id==='r3'){
      const fall=period('fall'),firstLevel=parameter('first_lt'),entryLevel=parameter('rsi_lt'),addFall=period('add_fall'),maN=period('ma'),ma=simpleMovingAverage(c,maN);let count=0;
      const falling=rsi.map((v,i)=>{count=i>0&&finite(v)&&finite(rsi[i-1])&&v<rsi[i-1]!?count+1:0;return count;});
      add('ma',`Exit SMA(${maN})`,ma,'sma');
      series[1].label=`RSI(${rsiN}) · course R3`;
      series[1].thresholds.push({value:entryLevel,label:'Entry below',kind:'entry'},{value:firstLevel,label:'First falling day below',kind:'reference'});
      return wrap('Enter a falling-RSI pullback, then add only if it continues',`Course R3 has two equal half-position units. The selected ${fall} consecutive RSI decreases end today; its first-day test uses RSI at today + 1 − ${fall}, the first falling day of that rolling window. HPETF has a selectable first_ref interpretation; this course runner always uses the rolling first falling day. Entry also needs RSI below ${entryLevel} and the trend test. On a later held bar, a ${addFall}-day falling streak may add the second half. Exit has priority over adding: RSI > ${exitLevel}, otherwise close > SMA(${maN}). The add predicate does not repeat the original entry tests.`,i=>{
        const streakKnown=(n:number)=>i>=n&&rsi.slice(i-n,i+1).every(finite);
        const firstIndex=i+1-fall,known=sourceLedgerValid&&!model.focusTicker&&dates.has(prices.d[i]),held=beforeClose(i),priorLegs=held?.legs.filter(l=>l.date<prices.d[i]).length||0;
        const exit=finite(rsi[i])&&rsi[i]!>exitLevel||finite(ma[i])&&c[i]>ma[i]!;
        return [trendTest(i),condition(`RSI falling streak · ${fall} days`,'Each current RSI must be strictly lower than the prior source bar. Ties reset this streak.',streakKnown(fall)?falling[i]:null,`at least ${fall} falling days`,v=>v>=fall),
          condition('First falling day in the selected window','The shared course engine checks RSI at today + 1 − fall. HPETF can select a preceding-value interpretation with first_ref; this course runner does not select that alternate index.',firstIndex>=0?rsi[firstIndex]:null,`< ${firstLevel}`,v=>v<firstLevel),
          condition(`Current RSI(${rsiN}) entry test`,'The current reading must be strictly below the selected entry level.',rsi[i],`< ${entryLevel}`,v=>v<entryLevel),
          ...positionChecks(i),
          condition('Scale-in state · first half already held','A second unit can be added only after the entry bar, while exactly one unit is already held. The complete two-unit allocation is not assumed at entry.',known?(held&&priorLegs===1?1:0):null,'1 = eligible position state',v=>v===1),
          condition(`Scale-in streak · ${addFall} falling days`,'This is the additional price test, not permission to add when flat or after the exit condition fires.',streakKnown(addFall)?falling[i]:null,`at least ${addFall} falling days`,v=>v>=addFall),
          condition('Scale-in priority · no exit condition','The engine evaluates exits before adding; either exit condition blocks a same-close add.',known&&held&&finite(rsi[i])&&finite(ma[i])?(exit?0:1):null,'1 = neither exit price test passes',v=>v===1),
          exitTest(i),condition(`Exit price test · close above SMA(${maN})`,'Checked after the RSI exit test. It acts only while a position is held and closes before any scale-in.',ma[i]===null?null:c[i]-ma[i]!,'close − SMA > 0',v=>v>0)];
      });
    }
  }catch(error){return unavailable(error instanceof Error?error.message:'Selected Connors parameters are unavailable.');}
  return null;
}
