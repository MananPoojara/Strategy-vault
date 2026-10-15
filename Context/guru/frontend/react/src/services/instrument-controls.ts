export interface InstrumentPolicy { allowed_types?: string[]; execution_scope?: string }

/** A visible ETF button must remain a valid route choice alongside supported indices. */
export function instrumentTickers<T extends {ticker: string}>(policy: InstrumentPolicy | undefined, indices: T[], etfs: T[]): T[] {
  const controls = instrumentControls(policy);
  return [...(controls.index ? indices : []), ...(controls.etf ? etfs : [])];
}

/** Navigation follows reviewed types; an unresolved policy still uses the API's admission gate. */
export function instrumentControls(policy?: InstrumentPolicy) {
  const types = policy?.allowed_types || [];
  if (!types.length) return { index: true, stock: true, basket: true, etf: false, note: '' };
  const index = types.includes('index'), stock = types.includes('individual_stock'), etf = types.includes('etf');
  const basket = stock && policy?.execution_scope !== 'single_symbol_source_account';
  const note = stock && !basket
    ? 'This method uses an individual stock risk account. Choose a Stock above; equal-slot basket execution and position limits are unavailable for this source account.'
    : index && !basket && !etf
    ? 'This method uses an index. Choose an index above; stock baskets apply to stock-compatible methods.'
    : basket && !index && !etf
    ? 'This method uses individual stocks. Choose a Stock for an individual result, or a Basket for the equal-slot portfolio backtest.'
    : etf && !index && !basket
    ? 'This method uses actual ETF prices. Select an available ETF in the US market; stock baskets and cash indices cannot supply ETF prices.'
    : !basket
    ? 'This market-timing method uses index or ETF prices. Stock baskets apply to stock-compatible methods.'
    : '';
  return {index, stock, basket, etf, note};
}
