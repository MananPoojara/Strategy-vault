import {afterEach, expect, it, vi} from 'vitest';
import {act} from 'react';
import {createRoot, type Root} from 'react-dom/client';
import {StrategySettings, type StrategySettingsModel} from '../src/components/StrategySettings';

let root: Root | undefined;
let host: HTMLDivElement | undefined;
afterEach(async () => {
  if (root) await act(async () => root!.unmount());
  host?.remove(); root = undefined; host = undefined;
});
const model = (over: Partial<StrategySettingsModel> = {}): StrategySettingsModel => ({
  params: [
    {key: 'trend', label: 'Trend SMA', def: 200, min: 50, max: 300, step: 10, value: 200},
    {key: 'hold', label: 'Hold bars', def: 5, min: 1, max: 20, step: 1, value: 5},
  ],
  assumptions: [['Fills', 'Entry: Buy on the close.'], ['Costs', 'Slippage 0 bps per side · Rollover 0 bps per actual roll.']],
  ...over,
});
async function mount(settings: StrategySettingsModel, disabledReason?: string) {
  Object.assign(globalThis, {IS_REACT_ACT_ENVIRONMENT: true});
  const onApply = vi.fn();
  host = document.createElement('div'); document.body.append(host); root = createRoot(host);
  await act(async () => root!.render(<StrategySettings onApply={onApply} settings={settings} disabledReason={disabledReason} />));
  await act(async () => host!.querySelector<HTMLButtonElement>('[aria-label="Strategy settings"]')!.click());
  return onApply;
}
const dialog = () => document.querySelector<HTMLElement>('[role="dialog"]')!;
const input = (key: string) => dialog().querySelector<HTMLInputElement>(`tr[data-param="${key}"] input`)!;
const button = (text: string) => [...dialog().querySelectorAll<HTMLButtonElement>('button')].find(b => b.textContent === text)!;
const type = async (key: string, value: string) => act(async () => {
  const el = input(key);
  Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, 'value')!.set!.call(el, value);
  el.dispatchEvent(new Event('input', {bubbles: true}));
});

it('lists each setting with its book default, the value used and the schema limits, plus the assumptions', async () => {
  await mount(model());
  const rows = [...dialog().querySelectorAll('tr[data-param]')];
  expect(rows.map(r => r.querySelector('label')!.textContent)).toEqual(['Trend SMA', 'Hold bars']);
  expect(rows[0].querySelector('td')!.textContent).toBe('200');
  expect(input('trend').value).toBe('200');
  expect([input('trend').min, input('trend').max, input('trend').step]).toEqual(['50', '300', '10']);
  expect(document.getElementById(input('hold').id)!.closest('tr')!.querySelector('label')!.htmlFor).toBe(input('hold').id);
  expect(dialog().querySelector('[data-not-default]')).toBeNull();
  expect(dialog().textContent).toContain('Backtest assumptions');
  expect(dialog().textContent).toContain('Entry: Buy on the close.');
  expect(dialog().textContent).toContain('Net Profit calculation');
});

it('marks a changed value and Apply sends the optimizer encoding of every setting', async () => {
  const onApply = await mount(model());
  await type('hold', '7');
  expect(dialog().querySelector('tr[data-param="hold"] [data-not-default]')!.textContent).toBe('Not the book default');
  expect(dialog().querySelector('tr[data-param="trend"] [data-not-default]')).toBeNull();
  await act(async () => button('Apply').click());
  expect(onApply).toHaveBeenCalledExactlyOnceWith('fixed', 'trend:200,hold:7');
});

it('Reset restores book defaults and Apply clears custom settings (empty encoding)', async () => {
  const onApply = await mount(model({params: model().params.map(p => p.key === 'hold' ? {...p, value: 9} : p)}));
  expect(dialog().querySelector('tr[data-param="hold"] [data-not-default]')).not.toBeNull();
  await act(async () => button('Reset to book defaults').click());
  expect(input('hold').value).toBe('5');
  expect(dialog().querySelector('[data-not-default]')).toBeNull();
  await act(async () => button('Apply').click());
  expect(onApply).toHaveBeenCalledExactlyOnceWith('fixed', '');
});

it('unchanged settings apply only the display mode; Cancel discards edits', async () => {
  const onApply = await mount(model());
  await type('trend', '150');
  await act(async () => button('Cancel').click());
  expect(onApply).not.toHaveBeenCalled();
  await act(async () => host!.querySelector<HTMLButtonElement>('[aria-label="Strategy settings"]')!.click());
  expect(input('trend').value).toBe('200');
  await act(async () => button('Apply').click());
  expect(onApply).toHaveBeenCalledExactlyOnceWith('fixed');
});

it('blocks Apply for an out-of-range value', async () => {
  await mount(model());
  await type('trend', '999');
  expect(input('trend').getAttribute('aria-invalid')).toBe('true');
  expect(button('Apply').disabled).toBe(true);
});

it('says when a rule has no adjustable settings', async () => {
  await mount(model({params: []}));
  expect(dialog().textContent).toContain('This rule has no adjustable settings; it runs exactly as written in the book.');
  expect(button('Reset to book defaults')).toBeUndefined();
});

it('a fixed reviewed configuration is shown read-only without reset', async () => {
  await mount(model({fixedNote: 'Only this reviewed configuration is admitted.'}));
  expect(input('trend').readOnly).toBe(true);
  expect(button('Reset to book defaults')).toBeUndefined();
  expect(dialog().textContent).toContain('Only this reviewed configuration is admitted.');
});

it('basket: display mode stays locked but changed settings can still be applied', async () => {
  const onApply = await mount(model(), 'Basket returns use the equal-slot portfolio calculation.');
  expect(button('Apply').disabled).toBe(true);
  await type('hold', '3');
  expect(button('Apply').disabled).toBe(false);
  await act(async () => button('Apply').click());
  expect(onApply).toHaveBeenCalledExactlyOnceWith('fixed', 'trend:200,hold:3');
});

// The runtime builds the dialog model from the schema, the selection and the run result.
import {readFileSync} from 'node:fs';
const runtime = readFileSync('src/compat/runtime.js', 'utf8');
const settingsSource = runtime.slice(runtime.indexOf('function settingsModel('), runtime.indexOf('// ---------------------------------------------------------------- loader'));
const decodeP = (s: string) => Object.fromEntries((s || '').split(',').filter(Boolean).map(kv => kv.split(':')).map(([k, v]) => [k, +v]));
const settingsModel = new Function('OPT', 'decodeP', `${settingsSource};return settingsModel;`)(
  {schemaSync: {v1: [{key: 'trend', label: 'Trend SMA', def: 200, min: 50, max: 300, step: 10}, {key: 'hold', label: 'Hold bars', def: 5, min: 1, max: 20, step: 1}]}}, decodeP);

it('runtime model: values from S.p over book defaults, and assumptions from the selection and the run', () => {
  const m = settingsModel({id: 'v1', rules: ['F|Close above the 200-day SMA', 'E|Buy on the close', 'X|Sell on the close after 5 days']},
    {dates: ['2001-01-02', '2020-12-31'], auto_start: 'Started 2001-01-02, the first day with 200 bars.'},
    {p: 'trend:200,hold:7', slippage: '5', rollover: '2', from: '', to: '2020-12-31', report_mode: 'compound'}, {});
  expect(m.params.map((p: {key: string; def: number; value: number}) => [p.key, p.def, p.value])).toEqual([['trend', 200, 200], ['hold', 5, 7]]);
  const a = Object.fromEntries(m.assumptions);
  expect(a.Fills).toBe('Entry: Buy on the close · Exit: Sell on the close after 5 days');
  expect(a.Positions).toBe('One position at a time, with the full account equity in each trade.');
  expect(a.Costs).toBe('Slippage 5 bps per side · Rollover 2 bps per actual roll.');
  expect(a.Period).toBe('Requested first available bar to 2020-12-31; calculated 2001-01-02 to 2020-12-31. Started 2001-01-02, the first day with 200 bars.');
  expect(a['Net profit']).toBe('Compounding');
  expect(settingsModel({id: 'none'}, null, {p: ''}, {basket: true}).params).toEqual([]);
});
