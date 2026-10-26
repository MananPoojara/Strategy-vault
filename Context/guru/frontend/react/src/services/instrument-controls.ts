import {bandyAccountMethod} from "./bandy-source-account";
import {unitExposureMethod} from './unit-exposure-report';
export interface InstrumentPolicy { allowed_types?: string[]; execution_scope?: string; declared_application?: string }

/** A visible ETF button must remain a valid route choice alongside supported indices. */
export function instrumentTickers<T extends {ticker: string}>(policy: InstrumentPolicy | undefined, indices: T[], etfs: T[], id?:string, book?:string): T[] {
  const controls = instrumentControls(policy,id,book);
  if (unitExposureMethod(id,book)) return controls.index ? indices.filter(t=>['SPX','NIFTY 50'].includes(t.ticker)) : [];
  return [...(controls.index ? indices : []), ...(controls.etf ? etfs : [])];
}

/** Navigation follows reviewed types; an unresolved policy still uses the API's admission gate. */
export function instrumentControls(policy?: InstrumentPolicy, id?: string, book?: string) {
  const types = policy?.allowed_types || [];
  // A stock-research scope routes to an index/ETF only when the API declares that application (user decision 22 Oct 2026).
  const declared = !!policy?.declared_application && types.includes('individual_stock');
  if(id==='lkqts_sob'&&book==='lkqts'){
    const stock=policy?.execution_scope==='single_symbol_stock_price_research'&&(declared||(types.length===1&&types[0]==='individual_stock'));
    return {index:declared&&types.includes('index'),stock,basket:false,etf:declared&&types.includes('etf'),note:`Second Order Breakout uses declared single-stock percentage research on actual stock prices. ${stock?'Choose an admitted Stock above':'The current policy does not admit a Stock'}; baskets and position limits are unavailable. Original performance and volatility-based quantities are not certified.`};
  }
  if (unitExposureMethod(id,book)) return {index:types.length===1 && types[0]==='index'
    && policy?.execution_scope==='single_symbol_index_timing_research',stock:false,basket:false,etf:false,
    note:'This reviewed rule uses actual SPX cash-index prices, or NIFTY 50 as a declared adaptation. Other indices, stocks, ETFs and baskets do not inherit this review.'};
  if (policy?.execution_scope === 'single_symbol_stock_price_research') {
    const stock = types.includes('individual_stock');
    return {index:declared&&types.includes('index'),stock,basket:false,etf:declared&&types.includes('etf'),
      note:`This method uses percentage research on actual individual stock prices. ${stock ? 'Choose an admitted Stock above' : 'The current policy does not admit a Stock'}; basket execution and position limits are unavailable because no basket execution has been reviewed.`};
  }
  if (!types.length) return { index: true, stock: true, basket: true, etf: false, note: '' };
  const index = types.includes('index'), stock = types.includes('individual_stock'), etf = types.includes('etf');
  const basket = stock && policy?.execution_scope !== 'single_symbol_source_account';
  const note = policy?.execution_scope === 'single_symbol_source_account' && bandyAccountMethod(id,book) && (stock || etf)
    ? `This reviewed application uses an individual-instrument cash account. Choose an admitted ${stock && etf ? 'Stock or ETF' : etf ? 'ETF' : 'Stock'} above; equal-slot baskets and position limits are unavailable.`
    : stock && !basket
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
