import type { InstrumentsResponse, Market, SavedMarket, TickerRow } from "./types";

export interface InstrumentOption {
  ticker: string; label: string; disabled: boolean; reason?: string; type?: string; hasVolume: boolean;
  market?: SavedMarket; instrumentLabel?: string; first?: string; last?: string;
}

const notAdmitted = (type: string | undefined, allowed?: string[]) =>
  allowed && type && !allowed.includes(type) ? `Not admitted for this method (needs ${allowed.join(", ")})` : undefined;

/** in/us rows. Index rows carry no client-side type, so they stay selectable (spec §3.1). */
export function optionsFromTickers(rows: TickerRow[], allowedTypes?: string[], metadata: readonly {ticker:string;type:string;market:Market}[] = []): InstrumentOption[] {
  return rows.map((r) => {
    const declared=metadata.find(m=>m.ticker===r.ticker);
    const type = r.native ? r.asset_type : declared?.type;
    const reason = notAdmitted(type, allowedTypes);
    return {
      ticker: r.ticker, label: r.native ? [r.symbol ?? r.ticker, r.basket].filter(Boolean).join(" · ") : r.ticker,
      disabled: !!reason, reason, type, hasVolume: r.has_volume, market: r.trusted_market ?? r.market ?? declared?.market, first: r.first, last: r.last,
    };
  });
}

export function optionsFromGl(res: InstrumentsResponse, allowedTypes?: string[]): InstrumentOption[] {
  return res.instruments.map((row) => {
    const i = row.instrument;
    const reason = row.status === "needs_data" ? row.reason : notAdmitted(i.type, allowedTypes);
    return {
      ticker: i.ticker, label: i.name ?? i.ticker, disabled: !!reason, reason, type: i.type, hasVolume: false,
      market: i.market ?? undefined, instrumentLabel: i.label,
      ...(row.status === "ready" ? { first: row.first, last: row.last } : {}),
    };
  });
}

export function defaultTicker(market: Market, options: InstrumentOption[]): string | undefined {
  if (market === "in" && options.some((o) => o.ticker === "NIFTY 50" && !o.disabled)) return "NIFTY 50";
  return options.find((o) => !o.disabled)?.ticker;
}

export const savedMarket = (market: Market, option?: InstrumentOption): SavedMarket =>
  market === "gl" && option?.market === "crypto" ? "crypto" : market;
