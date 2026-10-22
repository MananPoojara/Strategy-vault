import {afterEach,expect,it,vi} from 'vitest';
import {act} from 'react';
import {createRoot,type Root} from 'react-dom/client';
import {PristineMonthlyHeatmap} from '../src/components/PristineMonthlyHeatmap';
import {PristineStrategyReport} from '../src/components/PristineStrategyReport';
import {StrategySettings} from '../src/components/StrategySettings';
import {PRISTINE_RESEARCH_MODE} from '../src/services/pristine-report';
import type {PristineStrategySnapshot,PristinePresentation} from '../src/services/pristine-presenter';

let root:Root|undefined,host:HTMLDivElement;
afterEach(async()=>{if(root)await act(async()=>root!.unmount());host?.remove();root=undefined;vi.restoreAllMocks();});
async function mount(node:React.ReactNode){
  Object.assign(globalThis,{IS_REACT_ACT_ENVIRONMENT:true});
  host=document.createElement('div');document.body.append(host);root=createRoot(host);
  await act(async()=>root!.render(node));
}
const visual=():Pick<PristinePresentation,'months'|'years'|'visualPeriodReasons'>=>({
  // Literal visual-only periods, not a financial report/admission fixture.
  months:{'2023-01':.1,'2023-02':-.1,'2024-01':0,'2024-02':.2},
  years:{'2023':-.01,'2024':.2},
  visualPeriodReasons:{months:{},years:{},benchmarkYears:{}},
});
const cell=(period:string)=>host.querySelector<HTMLTableCellElement>(`[data-period="${period}"]`)!;
const click=async(text:string)=>act(async()=>[...host.querySelectorAll<HTMLButtonElement>('button')].find(button=>button.textContent===text)!.click());

it('keeps red/green monthly cells, real zeros, absent months, Total and arithmetic Avg/Total distinct',async()=>{
  await mount(<PristineMonthlyHeatmap model={visual()}/>);
  expect([...host.querySelectorAll('thead th')].map(node=>node.textContent)).toEqual(['Year','Jan','Feb','Mar','Apr','May','Jun','Jul','Aug','Sep','Oct','Nov','Dec','Total']);
  expect(cell('2023-01').textContent).toBe('10.00');expect(cell('2023-01').dataset.sign).toBe('positive');
  expect(cell('2023-02').textContent).toBe('-10.00');expect(cell('2023-02').dataset.sign).toBe('negative');
  expect(cell('2024-01').textContent).toBe('0.00');expect(cell('2024-01').dataset.sign).toBe('zero');
  expect(cell('2023-03').textContent).toBe('');expect(cell('2023-03').dataset.sign).toBe('absent');
  expect(cell('2023-total').textContent).toBe('-1.00');expect(cell('2024-total').textContent).toBe('20.00');
  expect(cell('avg-1').textContent).toBe('5.00');expect(cell('avg-2').textContent).toBe('5.00');
  expect(cell('avg-total').textContent).toBe('9.50');expect(cell('avg-total').classList.contains('t')).toBe(true);
  expect(host.querySelector('tfoot th')?.textContent).toBe('Avg');expect(host.textContent).toContain('it is not CAGR');
});
it('changes heatmap palette and ordering locally without requesting another result',async()=>{
  const fetch=vi.spyOn(window,'fetch');await mount(<PristineMonthlyHeatmap model={visual()}/>);
  expect(cell('2023-01').style.getPropertyValue('--pristine-heat-shade')).toBe('#19855d');
  expect(cell('2023-02').style.getPropertyValue('--pristine-heat-shade')).toBe('#cf5550');
  const colors=host.querySelector<HTMLSelectElement>('[aria-label="Monthly heatmap colors"]')!;
  await act(async()=>{colors.value='ocean';colors.dispatchEvent(new Event('change',{bubbles:true}));});
  expect(cell('2023-01').style.getPropertyValue('--pristine-heat-shade')).toBe('#1689bb');
  await click('Newest first');expect(host.querySelector('tbody th')?.textContent).toBe('2024');
  await click('Oldest first');expect(host.querySelector('tbody th')?.textContent).toBe('2023');
  expect(colors.value).toBe('ocean');expect(fetch).not.toHaveBeenCalled();
});
it('propagates unavailable observed visual periods into their averages without replacing them with zero',async()=>{
  const model=visual();
  await mount(<PristineMonthlyHeatmap model={{...model,months:{...model.months,'2023-02':null},years:{...model.years,'2023':null},visualPeriodReasons:{months:{'2023-02':'Visual ratio overflow'},years:{'2023':'Visual ratio overflow'},benchmarkYears:{}}}}/>);
  expect(cell('2023-02').textContent).toBe('—');expect(cell('2023-02').title).toBe('Visual ratio overflow');
  expect(cell('avg-2').textContent).toBe('—');expect(cell('avg-2').dataset.sign).toBe('unavailable');
  expect(cell('avg-total').textContent).toBe('—');expect(cell('avg-1').textContent).toBe('5.00');
  expect(cell('2023-03').textContent).toBe('');expect(host.textContent).toContain('no average is asserted');
});
it('removes the fee input while retaining a supplied fee token when stock controls are applied',async()=>{
  const patch=vi.fn();window.VaultRuntime={patch}as unknown as Window['VaultRuntime'];
  const snapshot:PristineStrategySnapshot={data:null,view:null,definition:{id:'ptpm1_daily_cbs_cog',book:'p_tpm1',name:'Reviewed daily branch'},selection:{region:'us',run_scope:'single',tk:'basket:us_sp500:TEST',research_mode:PRISTINE_RESEARCH_MODE,cost:'5.125',slippage:'3',from:'2024-01-01',to:'2024-03-01'},stocks:[{ticker:'basket:us_sp500:TEST',symbol:'TEST'}],availability:{status:'not_testable',note:'No financial report supplied for this controls-only test.'},optimization:false};
  await mount(<PristineStrategyReport snapshot={snapshot} chrome=""/>);
  expect(host.querySelector('input[name="cost"]')).toBeNull();expect(host.querySelector('[data-retained-fee-setting]')?.textContent).toContain('5.125');
  const slip=host.querySelector<HTMLInputElement>('input[name="slippage"]')!;slip.value='4';
  await act(async()=>host.querySelector('form')!.dispatchEvent(new Event('submit',{bubbles:true,cancelable:true})));
  expect(patch).toHaveBeenCalledExactlyOnceWith({region:'us',run_scope:'single',basket:'',tk:'basket:us_sp500:TEST',research_mode:PRISTINE_RESEARCH_MODE,from:'2024-01-01',to:'2024-03-01',cost:'5.125',slippage:'4',rollover:'0',p:''});
  expect(snapshot.selection.cost).toBe('5.125');
});
it('uses the existing settings popup with Pristine audited closed-field wording and applies only the display mode',async()=>{
  const onApply=vi.fn();await mount(<StrategySettings pristine mode="fixed" onApply={onApply}/>);
  await act(async()=>host.querySelector<HTMLButtonElement>('[aria-label="Strategy settings"]')!.click());
  const dialog=document.querySelector<HTMLElement>('[role="dialog"]')!;
  expect(dialog.textContent).toContain('Closed Pristine-return display');expect(dialog.textContent).toContain('audited closed-return sum');expect(dialog.textContent).toContain('audited closed-compound return');
  expect(dialog.textContent).not.toMatch(/Fixed quantity|last closed exit|SafeZone|risk controller/);
  await act(async()=>dialog.querySelector<HTMLInputElement>('input[value="compound"]')!.click());
  await act(async()=>[...dialog.querySelectorAll<HTMLButtonElement>('button')].find(button=>button.textContent==='Apply')!.click());
  expect(onApply).toHaveBeenCalledExactlyOnceWith('compound');expect(document.querySelector('[role="dialog"]')).toBeNull();
});
