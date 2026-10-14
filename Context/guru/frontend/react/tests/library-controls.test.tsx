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
async function mount(market: 'all' | 'in' | 'us', options: {
  model?: Partial<HomeModel>;
  state?: Record<string, string>;
} = {}) {
  Object.assign(globalThis, {IS_REACT_ACT_ENVIRONMENT: true});
  const patch = vi.fn();
  window.VaultRuntime = {patch} as unknown as Window['VaultRuntime'];
  host = document.createElement('div'); document.body.append(host);
  root = createRoot(host);
  const model = {market, view: 'authors', category: '', usAvailable: true,
    indices: [{ticker: 'NIFTY 50', name: 'Nifty 50'}], baskets: [], symbols: [],
    ...options.model} as unknown as HomeModel;
  await act(async () => root!.render(<LibraryControls model={model} state={{region: 'in', tk: 'NIFTY 50', library_market: market, ...options.state}} />));
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
it('names the author default sort Backtests first while preserving its existing sort key', async () => {
  const {host} = await mount('all');
  const sort = host.querySelector<HTMLSelectElement>('#library-sort')!;
  expect(sort.value).toBe('default');
  expect(sort.selectedOptions[0].textContent).toBe('Backtests first');
});
it('keeps Catalogue order as the book default sort', async () => {
  const {host} = await mount('in', {model: {view: 'books'}});
  const sort = host.querySelector<HTMLSelectElement>('#library-sort')!;
  expect(sort.value).toBe('default');
  expect(sort.selectedOptions[0].textContent).toBe('Catalogue order');
});
it('selects a real ETF through the existing instrument route and clears the prior stock basket and dates', async () => {
  const ticker = 'basket:us_sp500:AAPL';
  const {host, patch} = await mount('all', {
    model: {
      indices: [{ticker: 'SPX', name: 'S&P 500'}],
      etfs: [{ticker: 'SPY', name: 'SPY ETF'}, {ticker: 'QQQ', name: 'QQQ ETF'}],
      baskets: [{id: 'us_sp500', name: 'S&P 500 current stocks', symbols: [ticker]}],
      symbols: [{ticker, name: 'AAPL'}],
    },
    state: {region: 'us', basket: 'us_sp500', tk: ticker, from: '2010-01-01', to: '2020-09-11'},
  });
  const select = host.querySelector<HTMLSelectElement>('[aria-label="Backtest data"]')!;
  expect([...select.querySelectorAll('optgroup[label="ETFs"] option')].map(option => option.getAttribute('value')))
    .toEqual(['i:SPY', 'i:QQQ']);
  expect(host.textContent).toContain('Test data (US)');
  await act(async () => {
    select.value = 'i:SPY';
    select.dispatchEvent(new Event('change', {bubbles: true}));
  });
  expect(patch).toHaveBeenCalledWith({basket: '', tk: 'SPY', from: '', to: ''});
});
it('does not mix US ETF choices into the actual Indian data context', async () => {
  const {host} = await mount('all', {model: {etfs: [{ticker: 'SPY', name: 'SPY ETF'}]}});
  expect(host.querySelector('optgroup[label="ETFs"]')).toBeNull();
  expect(host.querySelector('option[value="i:NIFTY 50"]')).not.toBeNull();
  expect(host.textContent).toContain('Test data (India)');
});
it('accepts older home models without an ETF field', async () => {
  const {host} = await mount('us', {
    model: {indices: [{ticker: 'SPX', name: 'S&P 500'}]},
    state: {region: 'us', tk: 'SPX'},
  });
  expect(host.querySelector('optgroup[label="ETFs"]')).toBeNull();
  expect(host.querySelector<HTMLSelectElement>('#library-instrument')!.value).toBe('i:SPX');
});
