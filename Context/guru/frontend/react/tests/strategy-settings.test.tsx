import {afterEach, expect, it, vi} from 'vitest';
import {act} from 'react';
import {createRoot, type Root} from 'react-dom/client';
import {StrategyParameters, StrategySettings, stepValue} from '../src/components/StrategySettings';
import type {ReturnDisplayMode} from '../src/services/return-display';

let root: Root | undefined;
let host: HTMLDivElement | undefined;
afterEach(async () => {
  if (root) await act(async () => root!.unmount());
  host?.remove(); root = undefined; host = undefined;
});
async function mount(mode?: ReturnDisplayMode) {
  Object.assign(globalThis, {IS_REACT_ACT_ENVIRONMENT: true});
  const onApply = vi.fn();
  host = document.createElement('div'); document.body.append(host); root = createRoot(host);
  await act(async () => root!.render(<StrategySettings mode={mode} onApply={onApply} />));
  const trigger = host.querySelector<HTMLButtonElement>('[aria-label="Strategy settings"]')!;
  await act(async () => trigger.click());
  return {onApply, trigger};
}
const dialog = () => document.querySelector<HTMLElement>('[role="dialog"]')!;
const radio = (value: string) => dialog().querySelector<HTMLInputElement>(`input[value="${value}"]`)!;
const button = (text: string) => [...dialog().querySelectorAll<HTMLButtonElement>('button')].find(b => b.textContent === text)!;

it('opens an accessible settings dialog with the default fixed choice focused and explanatory sizing basis', async () => {
  const {trigger, onApply} = await mount();
  expect(trigger.getAttribute('aria-haspopup')).toBe('dialog');
  expect(trigger.getAttribute('aria-expanded')).toBe('true');
  expect(dialog().getAttribute('aria-modal')).toBe('true');
  expect(document.getElementById(dialog().getAttribute('aria-labelledby')!)?.textContent).toBe('Strategy settings');
  expect(radio('fixed').checked).toBe(true);
  expect(document.activeElement).toBe(radio('fixed'));
  expect(dialog().textContent).toContain('No share quantity supplied.');
  expect(onApply).not.toHaveBeenCalled();
});

it('Cancel discards a draft choice, restores trigger focus and performs no apply', async () => {
  const {trigger, onApply} = await mount();
  await act(async () => radio('compound').click());
  expect(radio('compound').checked).toBe(true);
  await act(async () => button('Cancel').click());
  expect(document.querySelector('[role="dialog"]')).toBeNull();
  expect(onApply).not.toHaveBeenCalled();
  expect(document.activeElement).toBe(trigger);
  expect(trigger.getAttribute('aria-expanded')).toBe('false');
  await act(async () => trigger.click());
  expect(radio('fixed').checked).toBe(true);
});

it('Apply sends the chosen report mode exactly once and closes the dialog', async () => {
  const {trigger, onApply} = await mount();
  await act(async () => radio('compound').click());
  await act(async () => button('Apply').click());
  expect(onApply).toHaveBeenCalledTimes(1);
  expect(onApply).toHaveBeenCalledWith('compound');
  expect(document.querySelector('[role="dialog"]')).toBeNull();
  expect(document.activeElement).toBe(trigger);
});

it('reads the current controlled mode when opening', async () => {
  await mount('compound');
  expect(radio('compound').checked).toBe(true);
  expect(document.activeElement).toBe(radio('compound'));
});

it('Escape dismisses the draft without applying and restores focus', async () => {
  const {trigger, onApply} = await mount();
  await act(async () => radio('compound').click());
  await act(async () => document.dispatchEvent(new KeyboardEvent('keydown', {key: 'Escape', bubbles: true})));
  expect(document.querySelector('[role="dialog"]')).toBeNull();
  expect(onApply).not.toHaveBeenCalled();
  expect(document.activeElement).toBe(trigger);
});

it('an inside click stays open, while an outside backdrop click cancels the draft', async () => {
  const {trigger, onApply} = await mount();
  await act(async () => dialog().querySelector<HTMLElement>('h2')!.click());
  expect(document.querySelector('[role="dialog"]')).not.toBeNull();
  await act(async () => document.querySelector<HTMLElement>('[data-strategy-settings-backdrop]')!.click());
  expect(document.querySelector('[role="dialog"]')).toBeNull();
  expect(onApply).not.toHaveBeenCalled();
  expect(document.activeElement).toBe(trigger);
});

it('traps forward and backward Tab navigation at the dialog boundaries', async () => {
  await mount();
  const first = dialog().querySelector<HTMLButtonElement>('button')!;
  const last = button('Apply');
  last.focus();
  await act(async () => last.dispatchEvent(new KeyboardEvent('keydown', {key: 'Tab', bubbles: true, cancelable: true})));
  expect(document.activeElement).toBe(first);
  await act(async () => first.dispatchEvent(new KeyboardEvent('keydown', {key: 'Tab', shiftKey: true, bubbles: true, cancelable: true})));
  expect(document.activeElement).toBe(last);
});

it('the header close button dismisses without committing the draft', async () => {
  const {onApply} = await mount('compound');
  await act(async () => dialog().querySelector<HTMLButtonElement>('[aria-label="Close strategy settings"]')!.click());
  expect(document.querySelector('[role="dialog"]')).toBeNull();
  expect(onApply).not.toHaveBeenCalled();
});

it('steps a setting with the arrows by its own precision and stays inside its limits', () => {
  const p = (def: number, extra = {}) => ({key: 'k', label: 'K', def, value: def, ...extra});
  expect(stepValue(p(200), '200', 1)).toBe('201');
  expect(stepValue(p(200), '200', -1)).toBe('199');
  expect(stepValue(p(0.02), '0.02', 1)).toBe('0.03');
  expect(stepValue(p(2.5, {step: 0.5}), '2.5', 1)).toBe('3');
  expect(stepValue(p(10, {min: 10}), '10', -1)).toBe('10');
  expect(stepValue(p(10, {max: 10}), '10', 1)).toBe('10');
  expect(stepValue(p(10), '', 1)).toBe('11');
});

it('shows only the parameters in the page strip, and applies a changed value', async () => {
  Object.assign(globalThis, {IS_REACT_ACT_ENVIRONMENT: true});
  const onApply = vi.fn();
  host = document.createElement('div'); document.body.append(host); root = createRoot(host);
  const settings = {params: [{key: 'rsi', label: 'RSI under', def: 10, value: 10}], assumptions: [['Fills', 'Next open']] as [string, string][]};
  await act(async () => root!.render(<StrategyParameters settings={settings} onApply={onApply} />));
  expect(host.querySelector('[data-strategy-params]')).not.toBeNull();
  expect(host.textContent).not.toMatch(/Fixed quantity|Compounding|Next open/);
  const apply = host.querySelector<HTMLButtonElement>('.strategy-settings-apply')!;
  expect(apply.disabled).toBe(true);
  await act(async () => host!.querySelector<HTMLButtonElement>('[aria-label="Increase RSI under"]')!.click());
  expect(host.querySelector<HTMLInputElement>('input')!.value).toBe('11');
  await act(async () => apply.click());
  expect(onApply).toHaveBeenCalledExactlyOnceWith('rsi:11');
});

it('the popup leaves parameters to the page when asked, keeping the Net Profit mode and assumptions', async () => {
  Object.assign(globalThis, {IS_REACT_ACT_ENVIRONMENT: true});
  host = document.createElement('div'); document.body.append(host); root = createRoot(host);
  const settings = {params: [{key: 'rsi', label: 'RSI under', def: 10, value: 10}], assumptions: [['Fills', 'Next open']] as [string, string][]};
  await act(async () => root!.render(<StrategySettings onApply={vi.fn()} settings={settings} hideParameters />));
  await act(async () => host!.querySelector<HTMLButtonElement>('[aria-label="Strategy settings"]')!.click());
  expect(dialog().textContent).toContain('Fixed quantity');
  expect(dialog().textContent).toContain('Next open');
  expect(dialog().textContent).not.toContain('RSI under');
  expect(dialog().querySelector('input[type="number"]')).toBeNull();
});
