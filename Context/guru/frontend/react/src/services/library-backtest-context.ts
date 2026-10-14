export interface LibraryBacktestContext {
  region: string;
  tk: string;
  basket: string;
  label: string;
  kind: string;
  from: string;
  to: string;
}
interface MeasuredContext {
  availability: {availableFamilies?: number};
}
const hasClosedResults = (value?: number) => Number.isInteger(value) && (value ?? 0) > 0;

/** A fallback must have measured results. Retain a usable selected context. */
export function chooseBacktestContext<T extends MeasuredContext>(selected: T, alternatives: readonly T[]): T {
  if (hasClosedResults(selected.availability.availableFamilies)) return selected;
  return alternatives.find(item => hasClosedResults(item.availability.availableFamilies)) || selected;
}

/** Bounded discovery samples, never an aggregate basket or an original-book study. */
export function referenceBacktestContexts(
  region: string,
  baskets: readonly {id: string; symbols: string[]}[],
  prices: readonly {ticker: string}[],
  etfs: readonly {ticker: string}[],
): LibraryBacktestContext[] {
  const result: LibraryBacktestContext[] = [];
  const keys = new Set(prices.map(item => item.ticker));
  const basket = baskets.find(item => item.id === (region === 'us' ? 'us_sp500' : 'nifty50'));
  const preferred = region === 'us' ? 'AAPL' : 'RELIANCE';
  const stock = basket?.symbols.find(symbol => keys.has(symbol) && symbol.split(':').at(-1) === preferred)
    || basket?.symbols.find(symbol => keys.has(symbol));
  const period = {from: '2010-01-01', to: '2020-09-11'};
  if (basket && stock) result.push({region, tk: stock, basket: basket.id, label: stock.split(':').at(-1) || stock, kind: 'stock', ...period});
  const etf = region === 'us' && etfs.find(item => item.ticker === 'SPY' && keys.has(item.ticker));
  if (etf) result.push({region, tk: etf.ticker, basket: '', label: etf.ticker, kind: 'ETF', ...period});
  return result;
}
