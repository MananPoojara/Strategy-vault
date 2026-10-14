import {afterEach, expect, it, vi} from 'vitest';
import {act} from 'react';
import {createRoot, type Root} from 'react-dom/client';
import {LibraryControls} from '../src/components/LibraryControls';
import type {HomeModel} from '../src/state/store';
import {libraryMarket, libraryMarketVariants} from '../src/services/library-market';

let root: Root | undefined;
let host: HTMLDivElement | undefined;
it('catalogue coverage changes without changing the execution market or strategy status', () => {
  expect(libraryMarket({region: 'us'})).toBe('us');
  expect(libraryMarket({region: 'in', library_market: 'invalid'})).toBe('in');
  expect(libraryMarket({region: 'us', library_market: 'all'})).toBe('all');
  expect(libraryMarket({region: 'us', library_market: 'all'}, 'strategy')).toBe('us');
  const variants = [{id: 'india', needs_series: ['NSE_ADVANCES']}, {id: 'us', needs_series: ['NYSE_ADVANCES']}, {id: 'shared', status: 'not_testable'}];
  expect(libraryMarketVariants(variants, 'all')).toEqual(variants);
  expect(libraryMarketVariants(variants, 'in').map(v => v.id)).toEqual(['india', 'shared']);
  expect(libraryMarketVariants(variants, 'us').map(v => v.id)).toEqual(['us', 'shared']);
});
afterEach(async () => {
  if (root) await act(async () => root!.unmount());
  host?.remove();
  root = undefined;
  host = undefined;
});
async function mount(market: 'all' | 'in' | 'us') {
  Object.assign(globalThis, {IS_REACT_ACT_ENVIRONMENT: true});
  const patch = vi.fn();
  window.VaultRuntime = {patch} as unknown as Window['VaultRuntime'];
  host = document.createElement('div'); document.body.append(host);
  root = createRoot(host);
  const model = {market, view: 'authors', category: '', usAvailable: true,
    indices: [{ticker: 'NIFTY 50', name: 'Nifty 50'}], baskets: [], symbols: []} as unknown as HomeModel;
  await act(async () => root!.render(<LibraryControls model={model} state={{region: 'in', tk: 'NIFTY 50', library_market: market}} />));
  return {host, patch};
}
it('shows All / India / US and selects exactly one library market', async () => {
  const {host} = await mount('all');
  const buttons = [...host.querySelectorAll('[aria-label="Market"] button')];
  expect(buttons.map(button => button.textContent)).toEqual(['All', 'India', 'US']);
  expect(buttons.filter(button => button.getAttribute('aria-pressed') === 'true').map(button => button.textContent)).toEqual(['All']);
  expect(host.textContent).toContain('Test data (India)');
});
it('All resets catalogue pagination while preserving the actual backtest context', async () => {
  const {host, patch} = await mount('in');
  await act(async () => (host.querySelector('[aria-label="Market"] button') as HTMLButtonElement).click());
  expect(patch).toHaveBeenCalledWith({library_market: 'all', library_limit: '24', library_page: '1', source_page: '1'});
});
it('selecting a specific market retains its existing matching dataset behavior', async () => {
  const {host, patch} = await mount('all');
  await act(async () => (host.querySelectorAll('[aria-label="Market"] button')[2] as HTMLButtonElement).click());
  expect(patch).toHaveBeenCalledWith({library_market: 'us', region: 'us', basket: '', tk: 'SPX', from: '', to: '', library_limit: '24', library_page: '1', source_page: '1'});
});
