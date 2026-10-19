import {act} from 'react';
import {createRoot,type Root} from 'react-dom/client';
import {afterEach,beforeEach,expect,it,vi} from 'vitest';
import {RulesVisualGuide,recordedTradeEvents} from '../src/components/RulesVisualGuide';
import {makeRuleVisualModel,type RuleVisualModel} from '../src/services/rule-evidence';

let root:Root|undefined,host:HTMLDivElement|undefined;
const charts:{candles:{setData:ReturnType<typeof vi.fn>;setMarkers:ReturnType<typeof vi.fn>};remove:ReturnType<typeof vi.fn>;setRange:ReturnType<typeof vi.fn>;hover?:(event:{time:string})=>void;click?:(event:{time:string})=>void}[]=[];
const makeModel=():RuleVisualModel=>makeRuleVisualModel({strategy:{id:'kftsm_exp',name:'Smoothed trendline',rules:['E|Enter the latched direction at the close.','X|Reverse on the opposite direction.'],notes:'Declared SmoothedAverage seed convention.',instrument_policy:{source_rule_reviewed:false,source_basis:'Actual selected stocks or admitted index',source_convention:'Mean-seeded source convention',limitations:'No execution or original performance certification'}},ticker:'TEST',symbol:'TEST',parameters:{period:2,kind:0,lag:0,filter:0},prices:{ticker:'TEST',full_history:true,d:['2020-01-01','2020-01-02','2020-01-03','2020-01-06','2020-01-07'],o:[10,20,30,15,16],h:[11,21,31,16,17],l:[9,19,29,14,15],c:[10,20,30,15,16]},run:{dates:['2020-01-03','2020-01-06','2020-01-07'],trades:[{dir:1,entry:'2020-01-03',exit:'2020-01-06',signal:'2020-01-06',legs:[[0,30,.5],[1,15,.5]],exits:[],exit_px:15,open:false,entry_kind:'close',reason:'trendline turned'},{dir:-1,entry:'2020-01-06',exit:'2020-01-07',signal:'2020-01-07',legs:[[1,15,1]],exits:[],exit_px:16,open:true,entry_kind:'close',reason:'open'}]}});
beforeEach(()=>{
  Object.assign(globalThis,{IS_REACT_ACT_ENVIRONMENT:true});
  charts.length=0;
  (window as unknown as {LightweightCharts:unknown}).LightweightCharts={createChart:()=>{
    const state={candles:{setData:vi.fn(),setMarkers:vi.fn()},remove:vi.fn(),setRange:vi.fn(),hover:undefined as undefined|((event:{time:string})=>void),click:undefined as undefined|((event:{time:string})=>void)};charts.push(state);
    const scale={getVisibleLogicalRange:()=>({from:0,to:10}),setVisibleLogicalRange:state.setRange,subscribeVisibleLogicalRangeChange:vi.fn(),unsubscribeVisibleLogicalRangeChange:vi.fn()};
    return {addCandlestickSeries:()=>state.candles,addLineSeries:()=>({setData:vi.fn(),applyOptions:vi.fn(),createPriceLine:vi.fn()}),removeSeries:vi.fn(),timeScale:()=>scale,subscribeCrosshairMove:(cb:typeof state.hover)=>{state.hover??=cb;},unsubscribeCrosshairMove:vi.fn(),subscribeClick:(cb:typeof state.click)=>{state.click=cb;},unsubscribeClick:vi.fn(),resize:vi.fn(),remove:state.remove};
  }};
});
afterEach(async()=>{if(root)await act(async()=>root!.unmount());host?.remove();root=undefined;host=undefined;document.body.style.overflow='';vi.restoreAllMocks();});
async function mount(model=makeModel()){
  host=document.createElement('div');document.body.append(host);root=createRoot(host);
  await act(async()=>root!.render(<RulesVisualGuide model={model}/>));
  const trigger=host.querySelector<HTMLButtonElement>('button')!;
  await act(async()=>trigger.click());return trigger;
}
const dialog=()=>document.querySelector<HTMLElement>('[role="dialog"]')!;
const select=()=>dialog().querySelector<HTMLSelectElement>('select')!;

it('opens a named dialog and preserves every source rule and declared adaptation',async()=>{
  const trigger=await mount();expect(trigger.getAttribute('aria-expanded')).toBe('true');
  expect(dialog().getAttribute('aria-modal')).toBe('true');
  expect(document.getElementById(dialog().getAttribute('aria-labelledby')!)?.textContent).toBe('Smoothed trendline');
  expect(dialog().textContent).toContain('SmoothedAverage(2)');
  expect(dialog().textContent).toContain('Enter the latched direction at the close.');
  expect(dialog().textContent).toContain('Declared SmoothedAverage seed convention.');
  expect(dialog().textContent).toContain('Actual selected stocks or admitted index');
  expect(dialog().textContent).toContain('Mean-seeded source convention');
  expect(dialog().textContent).toContain('No execution or original performance certification');
  expect(document.activeElement?.getAttribute('aria-label')).toBe('Close visual rules');
});

it('computes with warmup but charts only actual selected dates',async()=>{
  await mount();expect(charts[0].candles.setData).toHaveBeenCalledWith([
    {time:'2020-01-03',open:30,high:31,low:29,close:30},
    {time:'2020-01-06',open:15,high:16,low:14,close:15},
    {time:'2020-01-07',open:16,high:17,low:15,close:16},
  ]);
  expect(dialog().querySelector('[data-condition-result="met"]')?.textContent).toContain('37.5');
  expect(dialog().textContent).toContain('22.5'); // Full-history SmoothedAverage, not selected-start seed.
});

it('shows true entry/scale-in/exit arrows but never an exit arrow for a terminal open mark',async()=>{
  await mount();const markers=charts[0].candles.setMarkers.mock.calls.at(-1)![0];
  expect(markers.map((m:{text:string})=>m.text)).toEqual(['Entry','Add','Exit']);
  await act(async()=>{select().value='1';select().dispatchEvent(new Event('change',{bubbles:true}));});
  expect(charts[0].candles.setMarkers.mock.calls.at(-1)![0].map((m:{text:string})=>m.text)).toEqual(['Entry']);
  expect(dialog().textContent).toContain('not an exit');
  expect(recordedTradeEvents(makeModel().trades[1]).at(-1)?.kind).toBe('mark');
});

it('updates candle values on hover, pins on click and keeps a single main chart alive',async()=>{
  await mount();const count=charts.length;
  await act(async()=>charts[0].hover?.({time:'2020-01-06'}));
  expect(dialog().querySelector('.rule-guide-inspector-heading')?.textContent).toContain('2020-01-06');
  await act(async()=>charts[0].click?.({time:'2020-01-03'}));
  await act(async()=>charts[0].hover?.({time:'2020-01-06'}));
  expect(dialog().querySelector('.rule-guide-inspector-heading')?.textContent).toContain('2020-01-03');
  expect(dialog().querySelector('[role="tooltip"]')?.textContent).toContain('Buy · entry fill');
  expect(charts.length).toBe(count);
});

it('cleans charts, restores scrolling and restores trigger focus on Escape and unmount',async()=>{
  document.body.style.overflow='auto';const trigger=await mount();expect(document.body.style.overflow).toBe('hidden');
  await act(async()=>document.dispatchEvent(new KeyboardEvent('keydown',{key:'Escape',bubbles:true,cancelable:true})));
  expect(document.querySelector('[role="dialog"]')).toBeNull();expect(document.activeElement).toBe(trigger);expect(document.body.style.overflow).toBe('auto');
  expect(charts.every(c=>c.remove.mock.calls.length===1)).toBe(true);
  await act(async()=>trigger.click());await act(async()=>root!.unmount());root=undefined;
  expect(document.body.style.overflow).toBe('auto');expect(charts.every(c=>c.remove.mock.calls.length===1)).toBe(true);
});

it('traps Tab, closes on backdrop and does not close for an inside click',async()=>{
  await mount();const close=dialog().querySelector<HTMLButtonElement>('[aria-label="Close visual rules"]')!;
  close.focus();await act(async()=>close.dispatchEvent(new KeyboardEvent('keydown',{key:'Tab',shiftKey:true,bubbles:true,cancelable:true})));
  expect(dialog().contains(document.activeElement)).toBe(true);expect(document.activeElement).not.toBe(close);
  await act(async()=>dialog().querySelector('h2')!.click());expect(dialog()).not.toBeNull();
  await act(async()=>document.querySelector<HTMLElement>('[data-rule-guide-backdrop]')!.click());expect(document.querySelector('[role="dialog"]')).toBeNull();
});

it('exposes rules for held selections without fetching, plotting, or inventing example results',async()=>{
  const fetch=vi.spyOn(window,'fetch');const m=makeModel();await mount({...m,status:'needs_data',blocked:'Required futures data is missing.'});
  expect(dialog().textContent).toContain('Required futures data is missing.');expect(dialog().textContent).toContain('Reverse on the opposite direction.');
  expect(dialog().querySelector('[data-rule-price-chart]')).toBeNull();expect(charts).toHaveLength(0);expect(fetch).not.toHaveBeenCalled();fetch.mockRestore();
});

it('keeps basket trades readable without substituting a benchmark chart',async()=>{
  const m=makeModel();await mount({...m,basket:true,symbol:'NSE 500'});
  expect(dialog().textContent).toContain('no source-qualified constituent price identity');
  expect(dialog().querySelector('select')).not.toBeNull();expect(charts).toHaveLength(0);
});

it('cancels the single-constituent history request when the guide closes',async()=>{
  const m=makeModel(),ticker='basket:test:CANCEL';
  m.basket=true;m.trades[0].ticker=ticker;m.basketPriceApi='/api/in';m.basketSources={[ticker]:{source_sha256:'cancel-source',csv_sha256:'cancel-csv'}};
  const fetch=vi.spyOn(window,'fetch').mockImplementation((_url,options)=>new Promise((_resolve,reject)=>options?.signal?.addEventListener('abort',()=>reject(new DOMException('Cancelled','AbortError')))));
  await mount(m);expect(fetch).toHaveBeenCalledTimes(1);const signal=fetch.mock.calls[0][1]?.signal;
  expect(signal?.aborted).toBe(false);
  await act(async()=>dialog().querySelector<HTMLButtonElement>('[aria-label="Close visual rules"]')!.click());
  expect(signal?.aborted).toBe(true);expect(charts).toHaveLength(0);
});

it('bounds a large position dropdown while allowing a direct jump to every actual trade',async()=>{
  const m=makeModel();m.trades=Array.from({length:10000},(_,key)=>({...m.trades[0],key}));
  await mount(m);expect(select().options.length).toBeLessThanOrEqual(41);
  const form=dialog().querySelector<HTMLFormElement>('.rule-guide-jump')!,input=form.querySelector<HTMLInputElement>('input')!;
  input.value='10000';await act(async()=>form.dispatchEvent(new Event('submit',{bubbles:true,cancelable:true})));
  expect(select().value).toBe('9999');expect(select().options.length).toBeLessThanOrEqual(41);
  expect(dialog().querySelector('.rule-guide-paging')?.textContent).toContain('10000 / 10000');
});

it('shows actual candles and no invented matches for unsupported rules and zero-trade windows',async()=>{
  const m=makeModel();await mount({...m,id:'unknown',trades:[]});
  expect(dialog().querySelector('[data-rule-evidence-scope]')?.getAttribute('data-rule-evidence-scope')).toBe('unavailable');
  expect(dialog().textContent).toContain('No positions were recorded');
  expect(dialog().querySelector('[data-condition-result="met"]')).toBeNull();
  expect(charts[0].candles.setData).toHaveBeenCalled();expect(charts[0].candles.setMarkers).toHaveBeenCalledWith([]);
});

it('does not treat a non-trading date as an interpolated candle or a condition match',async()=>{
  await mount();const input=dialog().querySelector<HTMLInputElement>('input[type="date"]')!;
  const setter=Object.getOwnPropertyDescriptor(HTMLInputElement.prototype,'value')!.set!;
  await act(async()=>{setter.call(input,'2020-01-04');input.dispatchEvent(new Event('change',{bubbles:true}));});
  // jsdom React date changes use the input event as well.
  await act(async()=>input.dispatchEvent(new Event('input',{bubbles:true})));
  if(input.value==='2020-01-04')expect(dialog().querySelector('.rule-guide-ohlc')).toBeNull();
});


it('focuses a late-listed constituent using only its painted candle indices',async()=>{
  const m=makeModel();
  // The basket calendar has two earlier dates with no listed-stock observations.
  m.dates=['2020-01-01','2020-01-02',...m.dates];
  m.prices={...m.prices!,d:m.prices!.d.slice(2),o:m.prices!.o.slice(2),h:m.prices!.h.slice(2),l:m.prices!.l.slice(2),c:m.prices!.c.slice(2)};
  await mount(m);
  expect(charts[0].candles.setData.mock.calls[0][0].map((c:{time:string})=>c.time)).toEqual(['2020-01-03','2020-01-06','2020-01-07']);
  expect(charts[0].setRange.mock.calls.at(-1)![0]).toEqual({from:0,to:2});
  await act(async()=>[...dialog().querySelectorAll<HTMLButtonElement>('.rule-guide-event')].at(-1)!.click());
  expect(charts[0].setRange.mock.calls.at(-1)![0]).toEqual({from:0,to:2});
});

it('uses each closed scale-out piece price, including the final piece, rather than weighted exit_px',()=>{
  const trade=makeModel().trades[0];trade.exits=[{date:'2020-01-03',price:110,weight:.25},{date:trade.exit,price:90,weight:.75}];trade.exitPrice=95;
  const events=recordedTradeEvents(trade).filter(e=>e.kind==='exit');
  expect(events.map(e=>e.price)).toEqual([110,90]);expect(events.at(-1)?.label).toContain('final exit');
  expect(events.some(e=>e.price===95)).toBe(false);
});

it('never turns an open scale-out terminal tuple into an exit arrow and uses its raw close mark',async()=>{
  const m=makeModel(),trade=m.trades[1];
  trade.exits=[{date:trade.entry,price:110,weight:.25},{date:trade.exit,price:90,weight:.25},{date:trade.exit,price:120,weight:.5}];trade.exitPrice=110;
  const events=recordedTradeEvents(trade);expect(events.at(-1)).toMatchObject({kind:'mark',price:120,label:'Open position mark · not an exit'});
  const partials=events.filter(e=>e.kind==='exit');expect(partials.map(e=>e.price)).toEqual([110,90]);expect(partials[1].arrow).toBe(false);
  m.trades=[trade];await mount(m);
  const markers=charts[0].candles.setMarkers.mock.calls.at(-1)![0];
  expect(markers.filter((e:{time:string})=>e.time===trade.exit)).toEqual([]);
  expect(dialog().textContent).toContain('Intraday ordering is not established');
});
