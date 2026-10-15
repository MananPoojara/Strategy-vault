export interface StockPrice {
  ticker: string;
  symbol?: string;
  native?: boolean;
  first?: string;
  last?: string;
  price_status?: string;
}

/** Qualified vendor keys are retained: a stock symbol cannot alias an index or
 * an ETF. Duplicate universe listings are one choice, with the selected source
 * kept when an existing research link names it explicitly. */
export function stockChoices<T extends StockPrice>(prices: T[], selected = ""): T[] {
  const choices = new Map<string, T>();
  for (const price of prices) {
    if (!price.ticker.startsWith("basket:") || price.price_status && price.price_status !== "ready") continue;
    const symbol = price.symbol || price.ticker.split(":").at(-1) || "";
    if (!symbol) continue;
    const old = choices.get(symbol);
    if (!old || price.ticker === selected) choices.set(symbol, price);
  }
  return [...choices.values()].sort((a, b) => {
    const symbolA = a.symbol || a.ticker.split(":").at(-1) || "";
    const symbolB = b.symbol || b.ticker.split(":").at(-1) || "";
    return symbolA < symbolB ? -1 : symbolA > symbolB ? 1 : 0;
  });
}

/** Union coverage is a date selector, not a promise that every constituent has
 * all those prices. The basket calculation reports constituent coverage. */
export function basketSpan(prices: StockPrice[]): {first: string; last: string} | null {
  const usable = prices.filter(p => /^\d{4}-\d{2}-\d{2}$/.test(p.first || "")
    && /^\d{4}-\d{2}-\d{2}$/.test(p.last || "") && p.first! <= p.last!);
  if (!usable.length) return null;
  return {first: usable.reduce((d, p) => p.first! < d ? p.first! : d, usable[0].first!),
    last: usable.reduce((d, p) => p.last! > d ? p.last! : d, usable[0].last!)};
}

export function positionLimit(value: unknown): number | null {
  if (typeof value !== "string" || !/^[1-9]\d{0,2}$/.test(value)) return null;
  const count = Number(value);
  return count <= 100 ? count : null;
}

/** An explicit alternative window, never an automatic change to a request.
 * Observation coverage does not certify historical membership or execution. */
export function priceRecoveryPeriod(coverage: unknown, from: string, to: string): {from: string; to: string} | null {
  const validDate = (value: unknown): value is string => typeof value === "string"
    && /^\d{4}-\d{2}-\d{2}$/.test(value) && Number.isFinite(Date.parse(value))
    && new Date(value).toISOString().slice(0, 10) === value;
  if (!coverage || typeof coverage !== "object" || !validDate(from) || !validDate(to) || from > to) return null;
  const c = coverage as Record<string, unknown>;
  if (c.scope !== "supplied_daily_observations" || !validDate(c.from) || !validDate(c.to) || c.from > c.to
    || !/^[a-f0-9]{64}$/.test(String(c.source_index_sha256 || ""))
    || !/^[a-f0-9]{64}$/.test(String(c.benchmark_sha256 || ""))) return null;
  const first = from > c.from ? from : c.from, last = to < c.to ? to : c.to;
  return first > last || first === from && last === to ? null : {from: first, to: last};
}
