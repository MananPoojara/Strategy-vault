import {readFileSync} from 'node:fs';
import {expect, it, vi} from 'vitest';

// Exercise the production basket renderer with a supplied engine response.
const runtime = readFileSync('src/compat/runtime.js', 'utf8');
const source = runtime.slice(runtime.indexOf('const BASKET_RUNS=new Map();'), runtime.indexOf('\nfunction rulesPanel(v)'));
const esc = (value: unknown) => String(value ?? '').replaceAll('&', '&amp;').replaceAll('<', '&lt;').replaceAll('>', '&gt;').replaceAll('"', '&quot;');
async function render(disclosure: string | null, extra: Record<string, unknown> = {}) {
 const data = {status:'ready', stats:{trades:1}, dates:['2020-01-01','2020-01-02'], equity:[1,1.1], portfolio:{timing_disclosure:disclosure}, notices:[], coverage:[], ...extra};
 const deps = {
  R:{optim:false}, instrumentControls:()=>({basket:true}), BASKETS:[{id:'nifty50',name:'NIFTY 50',symbols:['A'],source_counts:{expected:500}}], VaultUI:{costState:()=>({}),toolbar:()=>'',benchmarkHTML:()=>'',detailHTML:()=>'',tradeHTML:()=>''},
  decodeP:()=>({}), enginePath:()=>'/basket-run', busy:1, fetch:vi.fn(async()=>({ok:true,json:async()=>data})), optSchema:async()=>({}),
  cls:()=>'',pct:()=>'',fx:()=>'',makeRuleVisualModel:()=>null,TICK_ALL_US:[],TICK_ALL_IN:[],ohlc:async()=>[],esc,
  familyDisplayName:()=> 'Fixture rule', gateNotice:()=>'',periodPanel:()=>'',optimBanner:()=>'',calculatedPeriodHTML:()=>'',rulesPanel:()=>'',LABEL:{},heatTable:()=>'',
 };
 const make = new Function('deps', `const {${Object.keys(deps).join(',')}}=deps;let STRAT,RULE_VISUAL;${source};return viewBasket;`);
 const html = await make(deps)({id:'fixture',instrument_policy:{}},'A','','',{basket:'nifty50',region:'in',from:'',to:'',max_positions:'10',membership:'current'},'',1);
 const host = document.createElement('div');host.innerHTML = html;return host;
}
it('shows the supplied conservative timing disclosure beside basket run notes and escapes it',async()=>{
 const text="Portfolio timing: cash is reserved from entry-day open through exit-day close (conservative). Single-stock results are unchanged. <img src=x>";
 const host=await render(text);
 expect(host.querySelector('[data-basket-timing-disclosure]')?.textContent).toBe(text);
 expect(host.querySelector('[data-basket-timing-disclosure] img')).toBeNull();
 expect(host.querySelector('[data-basket-result] [data-basket-timing-disclosure]')).not.toBeNull();
});
it('does not attach a conservative disclosure to reviewed timing',async()=>{
 expect((await render(null)).querySelector('[data-basket-timing-disclosure]')).toBeNull();
});

it('publishes incomplete source inventory with a compact escaped member report and used count', async()=>{
 const host=await render(null, {member_report:{used_members:499,expected_members:500,adjusted_members:1,excluded_members:1,summary:'499 of 500 members used; 1 adjusted; 1 excluded.'},coverage:[
  {symbol:'READY',status:'ready'},
  {symbol:'<LATE>',status:'adjusted',first:'2009-05-19',last:'2020-12-31',used_from:'2009-05-19',used_to:'2020-12-31',note:'Actual first bar'},
  {symbol:'MISSING',status:'excluded',note:'No actual price file',used_from:null,used_to:null},
 ]});
 expect(host.querySelector('[data-basket-member-summary]')?.textContent).toContain('499 of 500');
 expect(host.querySelector('[data-results-provenance]')?.textContent).toContain('499 members');
 const details=host.querySelector<HTMLDetailsElement>('[data-basket-coverage]')!;
 expect(details.open).toBe(false);
 expect(details.textContent).toContain('<LATE>');
 expect(details.querySelector('late')).toBeNull();
 expect(details.textContent).not.toContain('READY');
 expect(details.textContent).toContain('No actual price file');
 expect(details.textContent).toContain('2009-05-19');
});

it('shows and escapes the approved free-cash settlement disclosure',async()=>{
 const text="losses beyond a position's slot are paid from free cash; no borrowing <img src=x>";
 const host=await render(null,{portfolio:{funding_disclosure:text}});
 expect(host.querySelector('[data-basket-funding-disclosure]')?.textContent).toBe(text);
 expect(host.querySelector('[data-basket-funding-disclosure] img')).toBeNull();
});
