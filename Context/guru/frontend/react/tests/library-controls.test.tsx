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
  if (root) await act(async () => root!.unmount());
  host?.remove();
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
const pill = (host: HTMLElement, name: string) =>
  [...host.querySelectorAll<HTMLDetailsElement>('details.dd')].find(d => d.querySelector('.dd-l')?.textContent === name);
const options = (host: HTMLElement, name: string) => [...pill(host, name)!.querySelectorAll<HTMLButtonElement>('.menu button')];
const choose = async (host: HTMLElement, name: string, text: string) => {
  const button = options(host, name).find(b => b.querySelector('span')?.textContent === text)!;
  await act(async () => button.click());
};
const RESET = {library_limit: '24', library_page: '1', source_page: '1'};

it('renders the strategies-page pill bar: Browse, Market, Status, Topic, Type, Sort, Library and a search box', async () => {
  const {host} = await mount('in');
  expect(host.querySelector('section.fbar .fbar-in')).not.toBeNull();
  expect([...host.querySelectorAll('.dd-l')].map(l => l.textContent)).toEqual(['Browse', 'Market', 'Status', 'Topic', 'Type', 'Sort', 'Library']);
  expect(host.querySelector('.dsearch input#library-filter-q')).not.toBeNull();
  expect(pill(host, 'Browse')!.querySelector('b')!.textContent).toBe('Authors');
  expect(pill(host, 'Market')!.querySelector('b')!.textContent).toBe('Nifty 50');
  expect(pill(host, 'Sort')!.querySelector('b')!.textContent).toBe('Backtests first');
  expect(options(host, 'Sort').map(b => b.textContent)).toEqual(['Backtests first', 'Most strategies', 'Most books', 'A–Z']);
  // An unset filter pill shows only its name, as on the strategies page.
  expect(pill(host, 'Status')!.querySelector('b')).toBeNull();
});
it('shows a count per option and disables empty options', async () => {
  const {host} = await mount('in', {model: {filterCounts: {author_status: {backtested: 140, rules: 30}, author_type: {person: 1541, collection: 131}}}});
  const status = options(host, 'Status');
  expect(status.map(b => [b.querySelector('span')!.textContent, b.querySelector('.badge')?.textContent ?? null, b.disabled]))
    .toEqual([['All authors', null, false], ['Backtested', '140', false], ['Rules found, not yet backtested', '30', false], ['Reading only', null, false]]);
  // A missing count is unknown, not zero; an explicit zero disables the option.
  expect(options(host, 'Type').map(b => b.querySelector('.badge')?.textContent)).toEqual([undefined, '1,541', '131']);
  const {host: zero} = await mount('in', {model: {filterCounts: {author_status: {backtested: 2, rules: 0, reading: 0}}}});
  expect(options(zero, 'Status').map(b => b.disabled)).toEqual([false, false, true, true]);
});
it('a filter pick patches its key and starts the list at page 1', async () => {
  const {host, patch} = await mount('in');
  await choose(host, 'Status', 'Backtested');
  expect(patch).toHaveBeenLastCalledWith({author_status: 'backtested', ...RESET});
  await choose(host, 'Topic', 'Psychology');
  expect(patch).toHaveBeenLastCalledWith({book_category: 'psychology', ...RESET});
  await choose(host, 'Type', 'Collection');
  expect(patch).toHaveBeenLastCalledWith({author_type: 'collection', ...RESET});
  await choose(host, 'Sort', 'Most strategies');
  expect(patch).toHaveBeenLastCalledWith({library_sort: 'strategies', ...RESET});
  await choose(host, 'Browse', 'Books & reading');
  expect(patch).toHaveBeenLastCalledWith({library_view: 'books', library_sort: 'default', ...RESET});
});
it('combined filters show one chip each, remove one at a time, and Clear all resets them together', async () => {
  const {host, patch} = await mount('in', {model: {category: 'psychology'}, state: {author_status: 'rules', author_type: 'person', book_category: 'psychology', library_scope: 'all', library_q: 'rsi'}});
  const chips = [...host.querySelectorAll('.chips .chip')];
  expect(chips.map(c => c.firstChild!.textContent)).toEqual(['Status: Rules found, not yet backtested', 'Type: Person', 'Topic: Psychology', 'Library: Full source inventory', '“rsi”']);
  expect(pill(host, 'Status')!.classList.contains('set')).toBe(true);
  expect(pill(host, 'Status')!.querySelector('b')!.textContent).toBe('Rules found, not yet backtested');
  await act(async () => (chips[1].querySelector('button') as HTMLButtonElement).click());
  expect(patch).toHaveBeenLastCalledWith({author_type: '', ...RESET});
  await act(async () => (host.querySelector('.chips button.link') as HTMLButtonElement).click());
  expect(patch).toHaveBeenLastCalledWith({library_q: '', book_category: '', author_status: '', author_type: '', book_progress: '', library_scope: 'trading', ...RESET});
});
it('shows no chip row when nothing narrows the list', async () => {
  const {host} = await mount('in');
  expect(host.querySelector('.chips')).toBeNull();
});
it('Market lists All, India and US instruments and sets the instrument the card stats use', async () => {
  const {host, patch} = await mount('in', {model: {marketChoices: {in: [{ticker: 'NIFTY 50', name: 'Nifty 50'}, {ticker: 'NIFTYBANK', name: 'Bank Nifty'}], us: [{ticker: 'SPX', name: 'S&P 500'}]}}});
  expect([...pill(host, 'Market')!.querySelectorAll('.menu li')].map(li => li.textContent)).toEqual(['All markets', 'India', 'Nifty 50', 'Bank Nifty', 'US', 'S&P 500']);
  await choose(host, 'Market', 'S&P 500');
  expect(patch).toHaveBeenLastCalledWith({library_market: 'us', region: 'us', basket: '', tk: 'SPX', from: '', to: '', ...RESET});
  await choose(host, 'Market', 'Bank Nifty');
  expect(patch).toHaveBeenLastCalledWith({library_market: 'in', region: 'in', basket: '', tk: 'NIFTYBANK', from: '', to: '', ...RESET});
  await choose(host, 'Market', 'All markets');
  expect(patch).toHaveBeenLastCalledWith({library_market: 'all', ...RESET});
});
it('All markets keeps naming the instrument whose results the cards show', async () => {
  const {host} = await mount('all');
  expect(pill(host, 'Market')!.querySelector('b')!.textContent).toBe('All · Nifty 50');
});
it('US ETFs and stock baskets appear only for the loaded region, and a basket opens a Stock pill', async () => {
  const ticker = 'basket:us_sp500:AAPL';
  const {host, patch} = await mount('us', {
    model: {
      marketChoices: {in: [{ticker: 'NIFTY 50', name: 'Nifty 50'}], us: [{ticker: 'SPX', name: 'S&P 500'}]},
      etfs: [{ticker: 'SPY', name: 'SPY ETF'}],
      baskets: [{id: 'us_sp500', name: 'S&P 500 current stocks', symbols: [ticker]}, {id: 'ndx', name: 'NASDAQ 100', symbols: ['basket:ndx:MSFT']}, {id: 'empty', name: 'Empty', symbols: []}],
      symbols: [{ticker, name: 'AAPL'}],
    },
    state: {region: 'us', basket: 'us_sp500', tk: ticker},
  });
  expect([...pill(host, 'Market')!.querySelectorAll('.menu li')].map(li => li.textContent))
    .toEqual(['All markets', 'India', 'Nifty 50', 'US', 'S&P 500', 'SPY ETF', 'US stock baskets', 'S&P 500 current stocks', 'NASDAQ 100 · stocks']);
  expect(pill(host, 'Market')!.querySelector('b')!.textContent).toBe('S&P 500 current stocks');
  expect(pill(host, 'Stock')!.querySelector('b')!.textContent).toBe('AAPL');
  await choose(host, 'Market', 'SPY ETF');
  expect(patch).toHaveBeenLastCalledWith({library_market: 'us', region: 'us', basket: '', tk: 'SPY', from: '', to: '', ...RESET});
  await choose(host, 'Market', 'S&P 500 current stocks');
  expect(patch).toHaveBeenLastCalledWith({library_market: 'us', region: 'us', basket: 'us_sp500', tk: ticker, from: '', to: '', ...RESET});
  const {host: india} = await mount('in', {model: {etfs: [{ticker: 'SPY', name: 'SPY ETF'}], marketChoices: {in: [{ticker: 'NIFTY 50', name: 'Nifty 50'}], us: [{ticker: 'SPX', name: 'S&P 500'}]}}});
  expect(pill(india, 'Market')!.textContent).not.toContain('SPY ETF');
});
it('the Books view swaps author filters for book status and book sorts', async () => {
  const {host, patch} = await mount('in', {model: {view: 'books', filterCounts: {book_progress: {backtested: 300, in_progress: 700}}}});
  expect([...host.querySelectorAll('.dd-l')].map(l => l.textContent)).toEqual(['Browse', 'Status', 'Topic', 'Sort', 'Library']);
  expect(options(host, 'Status').map(b => b.querySelector('span')!.textContent)).toEqual(['All statuses', 'Backtested', 'In progress', 'Not Backtested']);
  expect(pill(host, 'Sort')!.querySelector('b')!.textContent).toBe('Catalogue order');
  await choose(host, 'Status', 'In progress');
  expect(patch).toHaveBeenLastCalledWith({book_progress: 'in_progress', ...RESET});
});
it('search sends the query after a pause or on Enter', async () => {
  const {host, patch} = await mount('in');
  vi.useFakeTimers();
  try {
    const input = host.querySelector<HTMLInputElement>('#library-filter-q')!;
    const type = async (value: string) => act(async () => {
      Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, 'value')!.set!.call(input, value);
      input.dispatchEvent(new Event('input', {bubbles: true}));
    });
    await type('conn');
    expect(patch).not.toHaveBeenCalled();
    await act(async () => vi.advanceTimersByTime(300));
    expect(patch).toHaveBeenLastCalledWith({library_q: 'conn', ...RESET});
    await type('connors');
    await act(async () => input.dispatchEvent(new KeyboardEvent('keydown', {key: 'Enter', bubbles: true})));
    expect(patch).toHaveBeenLastCalledWith({library_q: 'connors', ...RESET});
    await act(async () => vi.advanceTimersByTime(300));
    expect(patch).toHaveBeenCalledTimes(2);
  } finally { vi.useRealTimers(); }
});

// A stock-only US source with no breadth dependency must not inherit India
// catalogue coverage merely because needs_series is empty.
it('keeps explicit market admission separate from breadth dependencies and All coverage', () => {
  const variants = [
    {id:'pristine-us-stock',needs_series:[],instrument_policy:{allowed_markets:['us']}},
    {id:'shared',needs_series:[]},
    {id:'india-breadth',needs_series:['NSE_ADV']},
    {id:'us-breadth',needs_series:['NYSE_ADV']},
  ];
  expect(libraryMarketVariants(variants,'in').map(v=>v.id)).toEqual(['shared','india-breadth']);
  expect(libraryMarketVariants(variants,'us').map(v=>v.id)).toEqual(['pristine-us-stock','shared','us-breadth']);
  expect(libraryMarketVariants(variants,'all')).toEqual(variants);
  expect(libraryMarketVariants(variants,'all')).not.toBe(variants);
});
