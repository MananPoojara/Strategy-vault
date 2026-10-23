import {readFileSync} from 'node:fs';
import {expect, it, vi} from 'vitest';

// Exercise the production basket renderer with a supplied engine response.
const runtime = readFileSync('src/compat/runtime.js', 'utf8');
const source = runtime.slice(runtime.indexOf('const BASKET_RUNS=new Map();'), runtime.indexOf('\nfunction rulesPanel(v)'));
const esc = (value: unknown) => String(value ?? '').replaceAll('&', '&amp;').replaceAll('<', '&lt;').replaceAll('>', '&gt;').replaceAll('"', '&quot;');
async function render(disclosure: string | null) {
 const data = {status:'ready', stats:{trades:1}, dates:['2020-01-01','2020-01-02'], equity:[1,1.1], portfolio:{timing_disclosure:disclosure}, notices:[], coverage:[]};
 const deps = {
  R:{optim:false}, instrumentControls:()=>({basket:true}), BASKETS:[], VaultUI:{costState:()=>({}),toolbar:()=>'',benchmarkHTML:()=>'',detailHTML:()=>'',tradeHTML:()=>''},
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
