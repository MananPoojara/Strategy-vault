export interface InstrumentPolicy { allowed_types?: string[] }

/** A visible ETF button must remain a valid route choice alongside supported indices. */
export function instrumentTickers<T extends {ticker: string}>(policy: InstrumentPolicy | undefined, indices: T[], etfs: T[]): T[] {
  const controls = instrumentControls(policy);
  return controls.etf ? controls.index ? [...indices, ...etfs] : etfs : indices;
}

/** Navigation follows reviewed types; an unresolved policy still uses the API's admission gate. */
export function instrumentControls(policy?: InstrumentPolicy) {
  const types = policy?.allowed_types || [];
  if (!types.length) return { index: true, basket: true, etf: false, note: '' };
  const index = types.includes('index'), basket = types.includes('individual_stock'), etf = types.includes('etf');
  const note = index && !basket && !etf
    ? 'This method uses an index. Choose an index above; stock baskets apply to stock-compatible methods.'
    : basket && !index && !etf
    ? 'This method uses individual stocks. Select a basket and a stock; each result is for that stock.'
    : etf && !index && !basket
    ? 'This method uses actual ETF prices. Select an available ETF in the US market; stock baskets and cash indices cannot supply ETF prices.'
    : !basket
    ? 'This market-timing method uses index or ETF prices. Stock baskets apply to stock-compatible methods.'
    : '';
  return {index, basket, etf, note};
}
