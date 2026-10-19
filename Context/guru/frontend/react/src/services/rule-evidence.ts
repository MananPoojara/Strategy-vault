import {buildStrategyIndicators, simpleMovingAverage, type PriceHistory, type StrategyIndicator} from './strategy-indicators';
import {buildConnorsRuleEvidence} from './connors-rule-evidence';

export interface RuleCheck {
  label: string;
  explanation: string;
  value: number | null;
  comparison?: string;
  passed: boolean | null;
}
export interface RuleEvidence {
  scope: 'exact' | 'indicators' | 'unavailable';
  title: string;
  note: string;
  series: StrategyIndicator[];
  /** Raw daily source index, never a re-seeded selected-window index. */
  at(index: number): RuleCheck[];
}
export interface VisualTrade {
  key: number;
  symbol: string;
  ticker: string;
  direction: number;
  entry: string;
  exit: string;
  open: boolean;
  reason: string;
  entryKind: string;
  signal: string;
  legs: {date: string; price: number; weight: number}[];
  exits: {date: string; price: number; weight: number}[];
  exitPrice: number | null;
}
export interface RuleVisualModel {
  id: string;
  name: string;
  book: string;
  symbol: string;
  status: string;
  blocked: string;
  rules: {stage: string; text: string}[];
  notes: string;
  sourceReview: string;
  sourceQualifications: string[];
  prices: (PriceHistory & {full_history?: boolean}) | null;
  parameters: Readonly<Record<string, number>>;
  trades: VisualTrade[];
  dates: string[];
  basket: boolean;
  /** The actual constituent currently charted inside a basket guide. */
  focusTicker?: string;
  ledgerAvailable: boolean;
  basketPriceApi?: string;
  basketSources?: Readonly<Record<string, {source_sha256:string;csv_sha256:string}>>;
}
const object = (v: unknown): Record<string, unknown> => v && typeof v === 'object' && !Array.isArray(v) ? v as Record<string, unknown> : {};
const finite = (v: unknown): v is number => typeof v === 'number' && Number.isFinite(v);
const date = (v: unknown): v is string => typeof v === 'string' && /^\d{4}-\d{2}-\d{2}$/.test(v);
const text = (v: unknown) => typeof v === 'string' ? v : '';
const number = (v: number | undefined | null): number | null => v != null && Number.isFinite(v) ? v : null;

/** Snapshot only. Opening the guide never launches a backtest or requests data. */
export function makeRuleVisualModel(input: {strategy: unknown; run?: unknown; prices?: unknown; parameters?: unknown; symbol?: string; ticker?: string; status?: string; blocked?: string; basketPriceApi?:string; sourceTickers?:unknown}): RuleVisualModel {
  const v = object(input.strategy), d = object(input.run), p = object(input.prices);
  const dates = Array.isArray(d.dates) ? d.dates.filter(date) : [];
  const runSource=object(d.provenance),priceSource=object(p.provenance);
  const sourceMatches=!input.ticker || (input.prices===d.ohlc && p.full_history!==true)
    || (p.ticker===input.ticker && !input.ticker.startsWith('basket:'))
    || (priceSource.qualified_ticker===input.ticker && runSource.qualified_ticker===input.ticker
      && typeof priceSource.csv_sha256==='string' && priceSource.csv_sha256===runSource.csv_sha256
      && typeof priceSource.source_sha256==='string' && priceSource.source_sha256===runSource.source_sha256);
  const sourcePrices = sourceMatches && ['d','o','h','l','c'].every(k => Array.isArray(p[k])) ? p as unknown as RuleVisualModel['prices'] : null;
  const basket = d.kind === 'basket';
  const rows = Array.isArray(d.trades) ? d.trades : [];
  // Validate the raw contract before sanitizing display rows. A dropped leg must
  // never turn a malformed two-unit holding into a valid one-unit position gate.
  const dateSet=new Set(dates);
  const rawLedgerComplete=Array.isArray(d.trades)&&Array.isArray(d.dates)&&dates.length===d.dates.length
    && dates.every((day,i)=>!i||day>dates[i-1])&&rows.every(raw=>{
      const t=object(raw),entry=text(t.entry),exit=text(t.exit);
      const pieces=(rawPieces:unknown,required:boolean)=>{
        if(rawPieces===undefined)return !required;
        if(!Array.isArray(rawPieces)||required&&!rawPieces.length)return false;
        let previous=-1;
        return rawPieces.every(piece=>{
          if(!Array.isArray(piece)||piece.length!==3||!Number.isInteger(piece[0])||piece[0]<0||piece[0]>=dates.length
            ||!finite(piece[1])||piece[1]<=0||!finite(piece[2])||piece[2]<=0||piece[0]<previous
            ||dates[piece[0]]<entry||dates[piece[0]]>exit)return false;
          previous=piece[0];return true;
        });
      };
      return dateSet.has(entry)&&dateSet.has(exit)&&entry<=exit&&finite(t.dir)&&[1,-1].includes(t.dir)
        &&pieces(t.legs,true)&&pieces(t.exits,false)&&Array.isArray(t.legs)&&dates[t.legs[0][0]]===entry
        &&(t.exit_i===undefined||Number.isInteger(t.exit_i)&&dates[Number(t.exit_i)]===exit);
    });
  const trades = rows.map((raw, key): VisualTrade => {
    const t = object(raw), accounting = object(t.accounting);
    const legs = (value: unknown) => !Array.isArray(value) ? [] : value.flatMap(row => {
      if (!Array.isArray(row) || !Number.isInteger(row[0]) || !date(dates[row[0]]) || !finite(row[1]) || !finite(row[2])) return [];
      return [{date: dates[row[0]], price: row[1], weight: row[2]}];
    });
    return {key, symbol: text(t.symbol) || text(accounting.symbol) || input.symbol || '', ticker:text(t.ticker)||input.ticker||'',direction: finite(t.dir) ? t.dir : 0,
      entry: text(t.entry), exit: text(t.exit), open: t.open === true, reason: text(t.reason), entryKind: text(t.entry_kind), signal: text(t.signal),
      legs: legs(t.legs), exits: legs(t.exits), exitPrice: finite(t.exit_px) ? t.exit_px : null};
  });
  const parameters = Object.fromEntries(Object.entries(object(input.parameters)).filter((entry): entry is [string, number] => finite(entry[1])));
  const requested=new Set(trades.map(t=>t.ticker));
  const basketSources=Object.fromEntries((Array.isArray(input.sourceTickers)?input.sourceTickers:[]).flatMap(raw=>{const row=object(raw),source=object(row.provenance),ticker=text(row.ticker);return requested.has(ticker)&&source.qualified_ticker===ticker&&typeof source.source_sha256==='string'&&typeof source.csv_sha256==='string'?[[ticker,{source_sha256:source.source_sha256,csv_sha256:source.csv_sha256}]]:[];}));
  return {id: text(v.id), name: text(v.family_name) || text(v.name), book:text(v.book), symbol: input.symbol || '', status: input.status || (input.run ? 'ready' : 'unavailable'), blocked: input.blocked || (sourceMatches?'':'The chart source does not match the selected run’s instrument and source identity. No substitute prices are displayed.'),
    rules: (Array.isArray(v.rules) ? v.rules : []).filter((x): x is string => typeof x === 'string').map(rule => {const [tag, ...rest] = rule.split('|'); return {stage: ({F:'Conditions',E:'Entry',A:'Scale-in',X:'Exit'} as Record<string,string>)[tag] || 'Rule', text: rest.length ? rest.join('|') : rule};}),
    notes: text(v.notes), sourceReview:(object(v.instrument_policy).source_rule_reviewed===true?'Selected source rules reviewed. Declared adaptations, instrument admission and data limits still apply.':'Source-rule review is not established for this method. The chart explains recorded engine behavior; it does not independently verify the book.')+' '+text(object(v.gate).note),
    sourceQualifications:[text(object(v.instrument_policy).source_basis),text(object(v.instrument_policy).source_convention),text(object(v.instrument_policy).limitations)].filter(Boolean),prices: sourcePrices, parameters, trades, dates, basket,ledgerAvailable:rawLedgerComplete,basketPriceApi:input.basketPriceApi,basketSources};
}

const basketPriceCache=new Map<string, NonNullable<RuleVisualModel['prices']>>();
/** One requested constituent, at most four cached histories. No strategy execution,
 * no universe fetch, no stock/index substitution. The caller owns cancellation. */
export async function loadBasketRulePrices(model:RuleVisualModel,trade:VisualTrade,signal:AbortSignal):Promise<NonNullable<RuleVisualModel['prices']>> {
  const expected=model.basketSources?.[trade.ticker],api=model.basketPriceApi;
  if(!model.basket||model.status!=='ready'||!expected||!/^\/api\/(in|us)$/.test(api||'')||!/^basket:[a-zA-Z0-9_]+:[^\s/]+$/.test(trade.ticker))throw Error('The selected position has no source-qualified constituent price identity. No replacement chart is shown.');
  const key=[api,trade.ticker,expected.source_sha256,expected.csv_sha256].join('|');
  if(signal.aborted)throw new DOMException('Cancelled','AbortError');
  const cached=basketPriceCache.get(key);if(cached)return cached;
  const response=await fetch(`${api}/ohlc?${new URLSearchParams({ticker:trade.ticker})}`,{signal});
  if(!response.ok)throw Error('Actual constituent price history is unavailable. Recorded basket fills remain readable.');
  const raw:unknown=await response.json(),prices=object(raw),source=object(prices.provenance);
  if(signal.aborted)throw new DOMException('Cancelled','AbortError');
  if(prices.full_history!==true||source.qualified_ticker!==trade.ticker||source.source_sha256!==expected.source_sha256||source.csv_sha256!==expected.csv_sha256||!validPrices(prices as unknown as PriceHistory))throw Error('Constituent price history does not match the selected engine catalogue identity. No chart or rule values are substituted.');
  const result=prices as unknown as NonNullable<RuleVisualModel['prices']>;basketPriceCache.set(key,result);
  while(basketPriceCache.size>4)basketPriceCache.delete(basketPriceCache.keys().next().value!);
  return result;
}

function validPrices(p: PriceHistory | null): p is PriceHistory {
  return !!p && p.d.length > 0 && [p.o,p.h,p.l,p.c].every(values => values.length === p.d.length && values.every(finite))
    && p.d.every((day,i) => date(day) && (!i || day > p.d[i-1]) && p.c[i]>0 && p.l[i]>0 && p.h[i]>=Math.max(p.o[i],p.l[i],p.c[i]) && p.l[i]<=Math.min(p.o[i],p.h[i],p.c[i]));
}
export function chartablePrices(model: RuleVisualModel): PriceHistory | null {
  return model.status === 'ready' && !model.basket && validPrices(model.prices) ? model.prices : null;
}
function exponential(values: readonly (number | null)[], alpha: number): (number | null)[] {
  let previous: number | null = null;
  return values.map(value => {if(value === null) return null; previous = previous === null ? value : previous + alpha*(value-previous); return number(previous);});
}
/** TradeStation SmoothedAverage convention declared by this engine, not a generic EMA. */
export function smoothedAverage(values: readonly number[], period: number): (number | null)[] {
  const result: (number | null)[] = Array(values.length).fill(null);
  if (!Number.isInteger(period) || period<1 || period>10000) throw Error('Invalid smoothing period.');
  if(values.length<period)return result;
  let previous = values.slice(0,period).reduce((a,b)=>a+b,0)/period;
  result[period-1]=previous;
  for(let i=period;i<values.length;i++){previous=(previous*(period-1)+values[i])/period;result[i]=number(previous);}
  return result;
}
function directAverage(values: readonly number[], period: number): (number|null)[] {
  return values.map((_,i)=>i+1<period?null:number(values.slice(i+1-period,i+1).reduce((a,b)=>a+b,0)/period));
}
const unavailable = (note: string): RuleEvidence => ({scope:'unavailable',title:'Rule calculation unavailable',note,series:[],at:()=>[]});
/** Exact providers trace declared engine arithmetic; they do not certify the book,
 * its ambiguous choices, fills, sizing, or the success of an entire trading plan. */
export function buildRuleEvidence(model: RuleVisualModel): RuleEvidence {
  const px = chartablePrices(model);
  if(!px)return unavailable(model.blocked || (model.basket ? 'A basket has many price series. A single index or stock chart cannot represent its rule conditions. Open an individual stock result for a candle-by-candle view.' : 'No valid actual price series is available for this selected run.'));
  if(model.prices?.full_history !== true)return unavailable('Full engine source history, including indicator warmup, is unavailable. Sliced candles can show fills, but exact rule values are withheld.');
  const connors=buildConnorsRuleEvidence(model,px);if(connors)return connors;
  const c=px.c, p=model.parameters, series:StrategyIndicator[]=[];
  const add=(key:string,label:string,values:(number|null)[],placement:'price'|'pane'='price',thresholds:StrategyIndicator['thresholds']=[])=>{
    series.push({key,label,points:px.d.map((time,i)=>({time,value:number(values[i])})),kind:placement==='price'?'sma':'sum',placement,thresholds});
  };
  const parameter=(key:string)=>{const value=p[key];if(!finite(value))throw Error(`Selected parameter “${key}” is unavailable.`);return value;};
  const period=(key:string,min=1)=>{const n=Math.max(min,Math.round(parameter(key)));if(n>10000)throw Error('Selected period is too large.');return n;};
  const check=(label:string, explanation:string, value:number|null|undefined, comparison:string, test:(v:number)=>boolean):RuleCheck=>({label,explanation,value:number(value),comparison,passed:value==null||!finite(value)?null:test(value)});
  const exact=(title:string,note:string,at:RuleEvidence['at']):RuleEvidence=>({scope:'exact',title,note,series,at:i=>i<0||i>=c.length?[]:at(i)});
  try {
    if(model.id==='kftsm_exp'||model.id==='kftsm_ma'){
      const n=period('period'),filter=parameter('filter'),lag=Math.max(0,Math.round(parameter('lag'))),kind=model.id==='kftsm_exp'?Math.max(0,Math.round(parameter('kind'))):0;
      if(filter<0||lag>10000||kind>2)throw Error('Selected smoothing, filter or lag is outside the supported trace.');
      const trend=model.id==='kftsm_ma'?simpleMovingAverage(c,n):kind===0?smoothedAverage(c,n):exponential(c,kind===1?2/(n+1):1/(n+1));
      const change=c.map((_,i)=>i>0&&trend[i]!=null&&trend[i-1]!=null?number((trend[i]!-trend[i-1]!)*100/c[i-1]):null);
      const delayed=change.map((_,i)=>i>=lag?change[i-lag]:null);let direction=0;
      const state=delayed.map(value=>{if(value!==null){if(value>filter)direction=1;else if(value < -filter)direction=-1;}return direction;});
      const label=model.id==='kftsm_ma'?`SMA(${n})`:kind===0?`SmoothedAverage(${n})`: `Exponential(${n}, α=${kind===1?'2':'1'}/(n+1))`;
      add('trendline',label,trend);add('trend-change',`Lagged trend change · ${lag} bars`,delayed,'pane',[{value:filter,label:'Long above',kind:'entry'},{value:-filter,label:'Short below',kind:'exit'}]);
      return exact('Follow the direction of the smoothed line',`${label}. ${model.id==='kftsm_exp'&&kind===0?'First full value = mean of the first '+n+' actual closes; then (previous × (n−1) + close) / n. This declared engine initialization is not certified TradeStation platform parity.':model.id==='kftsm_exp'?'Seeded at the first actual close.':'Uses the full-history running-sum SMA.'} Change = (line today − line yesterday) × 100 / yesterday’s close; use the value ${lag} bar(s) earlier. Strictly beyond ±${filter}% sets direction; inside the band or equal holds the prior direction. The run enters that latched direction at its first selected bar, then reverses at the opposite direction. Same-close execution is the engine’s declared convention.`,i=>[
        {label:label,explanation:'Trendline value on this candle, calculated before applying the lag.',value:number(trend[i]),passed:null},
        check('Long direction','The lagged line change must be strictly above the positive filter.',delayed[i],`> ${filter}%`,v=>v>filter),
        check('Short direction','The lagged line change must be strictly below the negative filter.',delayed[i],`< ${-filter}%`,v=>v < -filter),
        {label:'Direction carried forward',explanation:'1 = long, −1 = short, 0 = no direction yet. A neutral change keeps the previous direction; it does not close a position.',value:state[i],passed:null},
      ]);
    }
    if(model.id==='kfsc_macd'){
      if(parameter('alpha_fast')!==.0476||parameter('alpha_slow')!==.0243||parameter('alpha_signal')!==.1)throw Error('Only the reviewed literal MACD constants have a trace.');
      const fast=exponential(c,.0476),slow=exponential(c,.0243),macd=c.map((_,i)=>fast[i]!-slow[i]!),signal=exponential(macd,.1),difference=macd.map((v,i)=>v-signal[i]!);
      add('macd','MACD · literal .0476/.0243',macd,'pane');add('macd-signal','Signal · literal .10',signal,'pane');
      return exact('Act when MACD crosses its signal line','Both price smoothings start at the first actual close; MACD and its signal begin at zero. The printed literal constants conflict with the general period formula, so this is the declared literal interpretation, not standard 20/40/9 MACD. A cross reverses direction at the same close; equality holds. A flat selected start waits for a fresh cross.',i=>[
        {label:'MACD minus signal',explanation:'Positive means MACD is above its signal; a crossing also needs the previous candle.',value:number(difference[i]),passed:null},
        check('Cross above → long','Today > 0 and previous ≤ 0. On a genuine cross, buy / cover short.',i?difference[i]:null,'today > 0; previous ≤ 0',v=>v>0&&difference[i-1]<=0),
        check('Cross below → short','Today < 0 and previous ≥ 0. On a genuine cross, sell / open short.',i?difference[i]:null,'today < 0; previous ≥ 0',v=>v<0&&difference[i-1]>=0),
        {label:'Previous difference',explanation:'The preceding actual source candle; no selected-window reseeding.',value:i?number(difference[i-1]):null,passed:null},
      ]);
    }
    if(model.id==='kfsc_dsm'){
      if(parameter('mom')!==10||parameter('alpha')!==.0476)throw Error('Only the reviewed 10-day/.0476 momentum has a trace.');
      const momentum=c.map((v,i)=>i>=10?v-c[i-10]:null),first=exponential(momentum,.0476),second=exponential(first,.0476);let direction=0;
      const turns=c.map((_,i)=>{if(i<11)return 0;const next=second[i]!>second[i-1]!?1:second[i]!<second[i-1]!?-1:0;const turn=next&&direction&&next!==direction?next:0;if(next)direction=next;return turn;});
      add('dsm','Double-smoothed momentum',second,'pane');
      return exact('Trade a genuine turn in smoothed momentum','Momentum = today’s close − close 10 actual source bars earlier. Smooth twice with literal .0476, seeding each pass at its first finite value. The first nonzero slope establishes direction; only a later change of direction is a turn. Flat days retain slope direction. Same-close turns reverse the position.',i=>[
        {label:'10-day momentum',explanation:'A price difference, not a percentage return.',value:number(momentum[i]),passed:null},
        {label:'Double-smoothed value',explanation:'Two .0476 passes using the complete source history.',value:number(second[i]),passed:null},
        check('Turn up → long','The last nonzero slope changed from down to up.',i>=11?turns[i]:null,'turn = +1',v=>v===1),
        check('Turn down → short','The last nonzero slope changed from up to down.',i>=11?turns[i]:null,'turn = −1',v=>v===-1),
      ]);
    }
    if(model.id==='kfsc_two_ma'){
      const fast=period('fast'),slow=period('slow');add('fast',`SMA(${fast})`,directAverage(c,fast));add('slow',`SMA(${slow})`,directAverage(c,slow));
      return exact('Enter when both average lines rise','The engine compares today’s close with the close N bars ago to determine the exact SMA direction. Both must rise to enter long; either falling closes the position. Equal closes hold. This is not a crossing of the two averages.',i=>[
        check(`Fast ${fast}-bar direction`,'Today’s close minus the close at the start of the SMA change.',i>=fast?c[i]-c[i-fast]:null,'> 0 to rise; < 0 to fall',v=>v>0),
        check(`Slow ${slow}-bar direction`,'Both fast and slow directions must be positive for a new entry.',i>=slow?c[i]-c[i-slow]:null,'> 0 to rise; < 0 to fall',v=>v>0),
      ]);
    }
    if(model.id==='kfsc_pct_band'||model.id==='kfsc_bollinger'){
      const n=period('period',2),ma=directAverage(c,n),f=model.id==='kfsc_pct_band'?1+parameter('pct')/100:parameter('nsd');
      const sd=c.map((_,i)=>i+1<n?null:Math.sqrt(c.slice(i+1-n,i+1).reduce((sum,value)=>sum+(value-ma[i]!)**2,0)/(n-1)));
      const lower=ma.map((m,i)=>m===null?null:model.id==='kfsc_pct_band'?m/f:m-f*sd[i]!),upper=ma.map((m,i)=>m===null?null:model.id==='kfsc_pct_band'?m*f:m+f*sd[i]!);
      add('middle',`SMA(${n})`,ma);add('lower','Lower band',lower);add('upper','Upper band',upper);
      return exact('Buy below the band; exit at the middle',model.id==='kfsc_pct_band'?'Upper band = SMA × 1.05; lower = SMA / 1.05, not SMA × .95. A flat position buys strictly below the lower band or shorts strictly above the upper band. Long closes at or above the middle; short closes at or below it. Exit is checked first, so the opposite outer-band entry may follow at the same close.':'Bands use closing price levels and sample standard deviation (N−1), following the declared literal Excel formula. A flat position buys below the lower band or shorts above the upper band. Exit at the middle; if the opposite outer band also qualifies, a same-close reversal can follow. This differs from the book’s conflicting price-change prose.',i=>[
        check('Below lower band → long setup','A setup can open only when the strategy is flat.',lower[i]===null?null:c[i]-lower[i]!,'close − lower < 0',v=>v<0),
        check('Above upper band → short setup','A setup can open only when the strategy is flat.',upper[i]===null?null:c[i]-upper[i]!,'close − upper > 0',v=>v>0),
        check('Long exit at middle','Applies only while a long position is open.',ma[i]===null?null:c[i]-ma[i]!,'close − middle ≥ 0',v=>v>=0),
        check('Short exit at middle','Applies only while a short position is open.',ma[i]===null?null:c[i]-ma[i]!,'close − middle ≤ 0',v=>v<=0),
      ]);
    }
    const pair=model.id.match(/^sbma_x_(\d+)([se])(\d+)([se])$/);
    if(pair){
      const fastN=period('fast'),slowN=period('slow'),fill=parameter('fill'),alwaysIn=parameter('always_in');
      const average=(n:number,ema:boolean)=>{if(!ema)return directAverage(c,n);const out:(number|null)[]=Array(c.length).fill(null);if(c.length<n)return out;let previous=c.slice(0,n).reduce((a,b)=>a+b,0)/n;out[n-1]=previous;for(let i=n;i<c.length;i++){previous+=2/(n+1)*(c[i]-previous);out[i]=number(previous);}return out;};
      const fast=average(fastN,pair[2]==='e'),slow=average(slowN,pair[4]==='e');
      add('fast',`${pair[2]==='e'?'EMA':'SMA'}(${fastN})`,fast);add('slow',`${pair[4]==='e'?'EMA':'SMA'}(${slowN})`,slow);
      return exact('Follow a crossing of the two averages',`EMA values seed from the first full N-close mean; SMA sums a fresh oldest-to-newest window on each candle. Fast crosses above slow to enter long; crosses below to exit${alwaysIn>=.5?' and reverse short':''}. Fill setting ${fill>=.5?'1 uses the next actual open; a final unfilled signal is not a trade':'0 uses the idealized signal close'}. Equality alone does not trigger a cross.`,i=>[
        check('Cross above → long','Both current and preceding values must exist.',i>0&&fast[i]!=null&&slow[i]!=null&&fast[i-1]!=null&&slow[i-1]!=null?fast[i]!-slow[i]!:null,'today > 0; previous ≤ 0',v=>v>0&&fast[i-1]!<=slow[i-1]!),
        check('Cross below → exit / reverse','The printed engine position mode determines whether to stay in cash or short.',i>0&&fast[i]!=null&&slow[i]!=null&&fast[i-1]!=null&&slow[i-1]!=null?fast[i]!-slow[i]!:null,'today < 0; previous ≥ 0',v=>v<0&&fast[i-1]!>=slow[i-1]!),
      ]);
    }
  }catch(error){return unavailable(error instanceof Error?error.message:'The selected rule trace is unavailable.');}
  const indicators=buildStrategyIndicators({id:model.id,prices:px,fullHistory:true,parameters:p});
  if(indicators.status==='ready')return {scope:'indicators',title:'Inspect the strategy’s plotted inputs',note:indicators.note+' These indicator values are partial evidence: an entry arrow does not prove that every entry, scale-in, liquidity or position-management condition matched.',series:indicators.series,at:i=>indicators.series.map(s=>({label:s.label,explanation:'Calculated indicator input only; compare the complete source rules below.',value:number(s.points[i]?.value),passed:null}))};
  return unavailable('Actual recorded fills are shown on the chart. A source-specific condition calculator for this method is not available yet, so no condition is marked as matched. The complete source rules and implementation notes remain below.');
}
