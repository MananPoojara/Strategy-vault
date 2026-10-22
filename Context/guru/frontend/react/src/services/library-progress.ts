export type ProgressStatus = 'backtested' | 'in_progress' | 'not_backtested';
export const progressLabels: Record<ProgressStatus, string> = {
  backtested: 'Backtested', in_progress: 'In progress', not_backtested: 'Not Backtested',
};
interface Definition {
  id: string; book: string; family?: string; status?: string; status_note?: string; needs_series?: string[]; needs_vix?: boolean;
  instrument_policy?: {requirement?: string; allowed_types?: string[]};
}
interface Measured {
  variant_id: string; book_id: string; method_id: string; author_id?: string;
  result_status: string; evidence: unknown; market?: string; instrument?: string;
}
interface Snapshot {
  schema_version: number; as_of: string; catalogue_variant_ids: string[];
  measured_backtests: Measured[];
  catalogue_rule_signatures: Record<string, string>;
  source_reviews?: {source_id: string; author_id: string; progress_status: string; reason?: string}[];
}
export const methodKey = (v: Definition) => `${v.book}:${v.family || v.id}`;
/** Plain-language headline for a hold note. Display wording only; it never changes a status or admission. */
const reasonLabels: [RegExp, string][] = [
  [/accuracy check|does not reproduce the results printed/i, 'Held by the accuracy check'],
  [/NSE breadth/i, 'NSE breadth held'],
  [/discretionary|subjective|not fully defined|not quantified|no (?:objective )?entry|no exit|no direction|event study|not disclosed|hindsight|names no system|indicator reading/i, 'Rules not fully defined'],
  [/\boptions?\b|option chains|\bOPT_/i, 'Needs option prices'],
  [/intraday|same day|\d+-minute|half-hour|OPEN60|OPEN15/i, 'Needs intraday data'],
  [/\bTICK\b|_TICK\b/, 'Needs NYSE TICK data'],
  [/\bVIX|volatility-index/i, 'VIX data required'],
  [/fixed point or dollar|USD|price band/i, 'Fixed amounts set for another market'],
  [/ranking|lowest-volatility|stock universe|portfolio|most liquid/i, 'Needs a stock universe or portfolio'],
  [/required data|data (?:is )?required|missing|unavailable|not loaded|not in the vault/i, 'Required data missing'],
];
export const reasonLabel = (note?: string) => reasonLabels.find(([pattern]) => pattern.test(note || ''))?.[1];
const signatureFields = ['id','book','family','dir','status','status_note','needs_volume','needs_vix','needs_series','rules','notes','instrument_policy','gate'];
function canonical(value: unknown): unknown {
  if (Array.isArray(value)) return value.map(canonical);
  if (value && typeof value === 'object') return Object.fromEntries(Object.entries(value).sort(([a],[b]) => a.localeCompare(b, 'en')).map(([k,v]) => [k,canonical(v)]));
  return value;
}
export function definitionSignature(def: Definition) {
  const input = def as unknown as Record<string, unknown>;
  return JSON.stringify(canonical(Object.fromEntries(signatureFields.filter(k => input[k] !== undefined).map(k => [k,input[k]]))));
}
/** Availability is an audited measured snapshot, never inferred from registration. */
export function libraryProgress(definitions: Definition[], input: unknown) {
  const defs = new Map(definitions.map(v => [v.id, v]));
  const snapshot = input as Partial<Snapshot> | null;
  const ids = [...defs.keys()].sort();
  const sourceIds = Array.isArray(snapshot?.catalogue_variant_ids) ? [...snapshot.catalogue_variant_ids].sort() : [];
  const current = snapshot?.schema_version === 1 && typeof snapshot.as_of === 'string'
    && ids.length === sourceIds.length && ids.every((id, i) => id === sourceIds[i])
    && definitions.every(def => snapshot.catalogue_rule_signatures?.[def.id] === definitionSignature(def));
  const measured = current && Array.isArray(snapshot?.measured_backtests)
    ? snapshot.measured_backtests.filter(row => {
      const def = defs.get(row.variant_id);
      return def && def.book === row.book_id && methodKey(def) === row.method_id
        && row.result_status === 'available' && !!row.evidence
        && def.status === 'live' && !!def.instrument_policy?.allowed_types?.length;
    }) : [];
  const available = new Set(measured.map(row => row.method_id));
  const byBook = new Map<string, Definition[]>();
  for (const def of definitions) { const rows = byBook.get(def.book) || []; rows.push(def); byBook.set(def.book, rows); }
  const found = new Set(definitions.map(methodKey));
  function counts(bookIds: string[]) {
    const books = new Set(bookIds);
    const methods = new Set([...books].flatMap(id => (byBook.get(id) || []).map(methodKey)));
    return {strategies: methods.size, backtested: [...methods].filter(key => available.has(key)).length};
  }
  /** Why a catalogued book has no published backtest, counted per strategy (book method). */
  function heldSummary(own: Definition[]) {
    const methods = new Map<string, Definition[]>();
    for (const def of own) { const rows = methods.get(methodKey(def)) || []; rows.push(def); methods.set(methodKey(def), rows); }
    const tally = new Map<string, number>();
    for (const rows of methods.values()) {
      // A rule still awaiting admission is described by the market series it reads; held siblings by their notes.
      const pending = rows.filter(v => /not completed admission review/i.test(v.status_note || ''));
      const coded = rows.filter(v => v.status === 'live' && v.instrument_policy?.allowed_types?.length);
      const series = coded.flatMap(v => v.needs_series || []).join(' ');
      const label = coded.length ? (coded.some(v => v.needs_vix) || /VIX/i.test(series) ? 'VIX data required'
          : /TRIN|ADV|DEC/i.test(series) ? 'Breadth data required (TRIN / advance-decline)' : 'Coded, not yet measured in the published results')
        : reasonLabel(pending.flatMap(v => v.needs_series || []).join(' ')) || (pending.some(v => v.needs_series?.length) ? 'Required data missing' : '')
        || reasonLabel(rows.filter(v => !pending.includes(v)).map(v => v.status_note).join(' '))
        || (pending.length ? 'Held until instrument admission review' : 'Held');
      tally.set(label, (tally.get(label) || 0) + 1);
    }
    const parts = [...tally.entries()].sort((a, b) => b[1] - a[1]).map(([label, n]) => `${label} (${n})`);
    return `No backtest is published yet for its ${methods.size} ${methods.size === 1 ? 'strategy' : 'strategies'}: ${parts.join('; ')}. Each strategy page gives the exact reason.`;
  }
  function book(bookId: string, review?: {status?: string; reason?: string}) {
    const own = byBook.get(bookId) || [];
    const n = counts([bookId]);
    // Coded books are never "in progress": without a result they are not backtested, with per-strategy reasons.
    let status: ProgressStatus = own.length ? 'not_backtested' : 'in_progress';
    let reason = own.length
      ? heldSummary(own)
      : 'Source review and strategy extraction are pending. This is queued work, not a claim of active conversion.';
    if (n.backtested) {
      status = 'backtested';
      reason = 'At least one strategy has an available computed result. This does not certify every strategy or reproduce the original book study.';
    } else if (review?.status === 'not_backtested' || own.length && own.every(v =>
      ['not_testable', 'needs_data'].includes(v.status || '')
      && v.instrument_policy?.requirement !== 'source_review_required'
      && !/not completed admission review|unreviewed|review pending|not yet converted/i.test(v.status_note || ''))) {
      status = 'not_backtested';
      reason = review?.reason || own.find(v => v.status_note)?.status_note
        || 'Registered rules currently lack required data or a complete supported execution rule. No performance is published.';
    }
    return {...n, status, label: progressLabels[status], reason};
  }
  function source(sourceId: string, authorIds: string[]) {
    const review = snapshot?.source_reviews?.find(row => row.source_id === sourceId && authorIds.includes(row.author_id));
    return book('', review?.progress_status === 'NOT BACKTESTED'
      ? {status:'not_backtested',reason:review.reason} : undefined);
  }
  return {current, asOf: current ? snapshot?.as_of : undefined, strategiesFound: found.size,
    backtestedStrategies: available.size, measured, counts, book, source};
}
