/**
 * The instrument a BTST run really trades. The index buttons (Nifty 50, Bank Nifty) only pick the series: the prices are
 * the near-month futures from the team's IEOD_FO database (AmiBroker symbols NIFTY_F1 / BANKNIFTY_F1 in the team's
 * sheets, files n/NIFTY and b/BANKNIFTY in the database), or the separate 2019-2026 futures feed (src 2).
 */
export const IEOD_DATABASE = 'DATABASE/Intraday_FO/IEOD_FO STOCKS-EODIEOD_UPDATED TILL 07-02-2023';

const FUTURES: Record<string, {symbol: string; name: string}> = {
  'NIFTY 50': {symbol: 'NIFTY_F1', name: 'Nifty near-month futures'},
  NIFTYBANK: {symbol: 'BANKNIFTY_F1', name: 'Bank Nifty near-month futures'},
};

export interface BtstInstrument {symbol: string; label: string; source: string}

/** null for anything that is not a BTST single-instrument run. */
export function btstInstrument(id: string | undefined, ticker: string | undefined, src = 1): BtstInstrument | null {
  if (!id?.startsWith('ytbtst_') || !ticker) return null;
  const f = FUTURES[ticker];
  if (!f) return null;
  if (src === 2) return {symbol: 'NIFTY_FUT_FEED', label: 'Nifty futures (2019-2026 feed)', source: 'Rolling Nifty futures feed 2019-2026, not the IEOD database'};
  return {
    symbol: f.symbol,
    label: `${f.symbol} · ${f.name}`,
    source: `${f.name}, continuous, from the IEOD_FO database (${IEOD_DATABASE})${src === 3 ? ', as the team\'s 2021 copy' : ''}`,
  };
}

/** The futures symbol a BTST index button stands for (NIFTY_F1 / BANKNIFTY_F1), or null for an index without BTST data. */
export const btstFuturesSymbol = (ticker: string | undefined): string | null => (ticker && FUTURES[ticker]?.symbol) || null;
