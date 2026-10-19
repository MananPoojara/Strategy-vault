import {afterEach, expect, it, vi} from "vitest";
import {act} from "react";
import {createRoot, type Root} from "react-dom/client";
import {StrategyPage} from "../src/pages/ResearchPages";
import type {ViewSnapshot} from "../src/state/store";

const markup = (signals = true) => `
  <header><h1>Reviewed method</h1></header><section class="box" data-rules>Source rules</section>
  <section class="box"><div class="box-h">Equity curve</div><label><input id="cmp" type="checkbox">Compare</label><div id="c-eq"></div></section>
  ${signals ? '<section class="box"><div class="box-h">Price &amp; signals</div><div id="c-px"></div><details data-indicators><summary>Indicators</summary><input type="checkbox" checked></details></section>' : ''}
  <div class="two"><section class="box"><div class="box-h">Drawdown</div><div id="c-dd"></div></section>
  <section class="box"><div class="box-h">Yearly returns</div><div id="c-yr"></div></section></div>
  <section class="box" data-benchmark>Book vs benchmark</section><section class="box" data-detailed>Detailed statistics</section>
  <section class="box"><div class="box-h">Monthly returns</div><select id="heat-palette"><option value="green">Green / red</option><option value="blue">Blue</option></select><div id="heatbox"><table><tbody><tr><td>4.12</td></tr></tbody></table></div></section>
  <section class="box" data-tradesheet><div class="box-h">Tradesheet</div><button data-export="trades">Excel</button><div class="tlog"><table><tbody><tr><td>RELIANCE</td><td>2026-01-02</td><td>1.27%</td></tr></tbody></table></div></section>`;
let root: Root | undefined, host: HTMLDivElement;
afterEach(async () => {
  if(root) await act(async () => root!.unmount());
  host?.remove(); root=undefined; vi.restoreAllMocks();
});
async function mount(html=markup()) {
  Object.assign(globalThis,{IS_REACT_ACT_ENVIRONMENT:true});
  const patch=vi.fn(),activateReportTab=vi.fn();
  window.VaultRuntime={patch,activateReportTab} as unknown as Window['VaultRuntime'];
  const snapshot:ViewSnapshot={revision:1,html,loading:false,error:null,route:null,status:'Strategy',state:{},home:null,reading:null};
  host=document.createElement('div');document.body.append(host);root=createRoot(host);
  await act(async () => root!.render(<StrategyPage snapshot={snapshot}/>));
  return {patch,activateReportTab};
}
const tab=(id:string)=>host.querySelector<HTMLButtonElement>(`#report-tab-${id}`)!;
const panel=(id:string)=>host.querySelector<HTMLElement>(`#report-panel-${id}`)!;
const click=async(id:string)=>act(async()=>tab(id).click());

it('groups the six existing views in the equity card and keeps rules, benchmark and statistics outside',async()=>{
  await mount();
  const report=host.querySelector('[data-strategy-report]')!;
  expect([...host.querySelectorAll('[role="tab"]')].map(n=>n.textContent)).toEqual(['Equity Curve','Drawdown','Monthly Returns','Yearly Returns','Price & Signals','Positions']);
  expect(report.tagName).toBe('SECTION');expect(report.classList.contains('box')).toBe(true);
  expect(report.querySelectorAll('[role="tabpanel"]')).toHaveLength(6);
  expect(host.querySelector('.two')).toBeNull();
  for(const id of ['c-eq','c-dd','c-yr','c-px','heatbox'])expect(host.querySelectorAll(`#${id}`)).toHaveLength(1);
  for(const selector of ['[data-rules]','[data-benchmark]','[data-detailed]'])expect(report.contains(host.querySelector(selector))).toBe(false);
  expect(panel('positions').hasAttribute('data-tradesheet')).toBe(true);
  expect(panel('positions').textContent).toContain('1.27%');expect(panel('monthly').textContent).toContain('4.12');
  expect([...host.querySelectorAll<HTMLElement>('[role="tabpanel"]')].filter(p=>!p.hidden)).toEqual([panel('equity')]);
});

it('keeps mounted chart children, inputs, heatmap choices and indicator disclosure through switches without requesting a result',async()=>{
  const {patch,activateReportTab}=await mount();const fetch=vi.spyOn(window,'fetch');
  const equity=host.querySelector('#c-eq')!,canvas=document.createElement('canvas');equity.append(canvas);
  const compare=host.querySelector<HTMLInputElement>('#cmp')!;compare.checked=true;
  await click('signals');const disclosure=host.querySelector<HTMLDetailsElement>('[data-indicators]')!;disclosure.open=true;
  const indicator=disclosure.querySelector<HTMLInputElement>('input')!;indicator.checked=false;
  await click('monthly');const palette=host.querySelector<HTMLSelectElement>('#heat-palette')!;palette.value='blue';
  await click('positions');await click('equity');
  expect(host.querySelector('#c-eq')).toBe(equity);expect(equity.firstElementChild).toBe(canvas);
  expect(compare.checked).toBe(true);expect(palette.value).toBe('blue');expect(disclosure.open).toBe(true);expect(indicator.checked).toBe(false);
  expect(activateReportTab).toHaveBeenCalledTimes(5);expect(patch).not.toHaveBeenCalled();expect(fetch).not.toHaveBeenCalled();
});

it('activates and focuses tabs with arrows/Home/End and provides linked accessible panels',async()=>{
  await mount();
  const key=async(id:string,key:string)=>act(async()=>tab(id).dispatchEvent(new KeyboardEvent('keydown',{key,bubbles:true})));
  await key('equity','ArrowRight');expect(document.activeElement).toBe(tab('drawdown'));expect(panel('drawdown').hidden).toBe(false);
  await key('drawdown','End');expect(document.activeElement).toBe(tab('positions'));
  await key('positions','ArrowRight');expect(document.activeElement).toBe(tab('equity'));
  await key('equity','ArrowLeft');expect(document.activeElement).toBe(tab('positions'));
  await key('positions','Home');expect(document.activeElement).toBe(tab('equity'));
  expect(host.querySelectorAll('[role="tab"][tabindex="0"]')).toHaveLength(1);
  for(const button of host.querySelectorAll('[role="tab"]')){
    const content=host.querySelector(`#${button.getAttribute('aria-controls')}`)!;
    expect(content.getAttribute('aria-labelledby')).toBe(button.id);
    expect(button.getAttribute('aria-selected')).toBe(String(!(content as HTMLElement).hidden));
  }
});

it('offers only supported basket views and leaves withheld results without tabs',async()=>{
  await mount(markup(false));expect(host.querySelectorAll('[role="tab"]')).toHaveLength(5);expect(tab('signals')).toBeNull();
  await act(async()=>root!.render(<StrategyPage snapshot={{revision:2,html:'<section class="box" data-withheld>Data missing: membership dates unresolved</section>',loading:false,error:null,route:null,status:'Strategy',state:{},home:null,reading:null}}/>));
  expect(host.querySelector('[role="tablist"]')).toBeNull();expect(host.querySelector('[data-strategy-report]')).toBeNull();
  expect(host.textContent).toContain('membership dates unresolved');
});
