interface ResultRow {
  v: {book: string; family?: string; id: string};
  s?: {trades?: number; net_profit?: number; total_return?: number} | null;
}
/** Count the actual available strategy rows rendered after selection and filters. */
export function visibleBacktestCount(rows: ResultRow[]) {
  return new Set(rows.filter(row => row.s && Number.isInteger(row.s.trades) && row.s.trades! > 0
    && Number.isFinite(row.s.net_profit ?? row.s.total_return))
    .map(row => `${row.v.book}:${row.v.family || row.v.id}`)).size;
}
interface MeasuredRow {
  book_id: string; method_id: string; market?: string; instrument?: string;
  evidence: unknown;
  execution?: {cost?: string; slippage?: string; rollover?: string; research_mode?: string};
}
/** Book cards use exactly the instrument/window/settings their link opens. */
export function linkedBookBacktestCount(rows: MeasuredRow[], book: string, link: string) {
  const q = new URLSearchParams(link.split('?')[1] || '');
  const value = (key: string, fallback = '') => q.get(key) || fallback;
  return new Set(rows.filter(row => {
    const evidence = row.evidence as {from?: string; to?: string};
    return row.book_id === book && row.instrument === value('tk','NIFTY 50')
      && (row.market === 'us' ? 'us' : 'in') === value('region','in')
      && (evidence?.from || '') === value('from') && (evidence?.to || '') === value('to')
      && (row.execution?.cost || '0') === value('cost','0')
      && (row.execution?.slippage || '0') === value('slippage','0')
      && (row.execution?.rollover || '0') === value('rollover','0')
      && (row.execution?.research_mode || '') === value('research_mode');
  }).map(row => row.method_id)).size;
}
