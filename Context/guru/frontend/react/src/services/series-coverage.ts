/** First and last date with a value for a required series, as reported by the engine's ticker list. */
export type SeriesCoverage = Record<string, [string, string]>;
export interface DataWindow {from: string; to: string}

/**
 * The dates in which every series a strategy requires has values, when that window is narrower than the instrument's own
 * history (for example futures bars that start years after the cash index). Null means no preset is needed or the
 * coverage is unknown.
 */
export function strategyDataWindow(needs: string[] | undefined, coverage: SeriesCoverage | undefined, span: {first: string; last: string}): DataWindow | null {
  if (!needs?.length || !coverage) return null;
  let from = '';
  let to = '9999-12-31';
  for (const name of needs) {
    const range = coverage[name];
    if (!range) return null;
    if (range[0] > from) from = range[0];
    if (range[1] < to) to = range[1];
  }
  if (from >= to) return null;
  return from > span.first || to < span.last ? {from, to} : null;
}
