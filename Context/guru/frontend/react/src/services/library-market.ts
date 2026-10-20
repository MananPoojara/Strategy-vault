export type LibraryMarket = 'all' | 'in' | 'us';

/** Catalogue coverage is separate from the actual market used for a calculation. */
export function libraryMarket(state: Record<string, string>, route = 'home'): LibraryMarket {
  const region = state.region === 'us' ? 'us' : 'in';
  if (!['home', 'guru', 'all'].includes(route)) return region;
  return ['all', 'in', 'us'].includes(state.library_market)
    ? state.library_market as LibraryMarket
    : region;
}

export function libraryMarketVariants<T extends {needs_series?: string[];instrument_policy?: {allowed_markets?: string[]}}>(variants: T[], market: LibraryMarket): T[] {
  return market === 'all' ? [...variants] : variants.filter(variant =>
    (!variant.instrument_policy?.allowed_markets || variant.instrument_policy.allowed_markets.includes(market)) &&
    !(variant.needs_series || []).some(name => market === 'us' ? /^NSE_/.test(name) : /^NYSE_/.test(name)));
}
