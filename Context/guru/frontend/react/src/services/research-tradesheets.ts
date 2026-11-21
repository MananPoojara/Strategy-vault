/* Research page, "Tradesheet verified" view: how many YT strategies match the team's trade sheets, read from
   data/research-tradesheets.json. That file is built by Context/guru/webapp/scripts/build_research_tradesheets.py
   from the Tradesheet Match Status and Strategy Tradesheet Status workbooks (read only). */

export const TS_URL = "data/research-tradesheets.json";
export const TS_HINT = "Run python3 Context/guru/webapp/scripts/build_research_tradesheets.py to build data/research-tradesheets.json.";

export const STATUSES = ["Matching", "Partly matching", "Not matching", "Not compared yet", "No trade sheets"] as const;
export type TsStatus = (typeof STATUSES)[number];
export type Counts = Record<string, number>;

export interface TsStrategy {
  category: string; id: string; name: string; status: string; detail: string;
  /** Team trade sheets compared (matching or not) and how many of them match. */
  sheets: number; matched: number;
  verified_by_jay: boolean; verified_as: string;
}
export interface TsChange { id: string; name: string; from: string | null; to: string | null }
export interface TsSnapshot { date: string; date_from: string; made: string; kind: "seed" | "run"; label: string; source: string; total: number; counts: Counts; changes: TsChange[] }
export interface TsCategory { category: string; counts: Counts; total: number; verified_by_jay: number }
export interface Tradesheets {
  version: number; generated_at: string;
  source: { file: string; made: string; date: string; date_from: string; strategies_note: string; caveat: string; status_file: string; status_made: string };
  statuses: string[];
  totals: Counts & { total: number; verified_by_jay: number };
  verified_rule: string;
  categories: TsCategory[];
  strategies: TsStrategy[];
  history: TsSnapshot[];
}
export type TsLoad = { ok: true; data: Tradesheets } | { ok: false; problem: string };

export async function loadTradesheets(fetchImpl: typeof fetch = fetch): Promise<TsLoad> {
  let body: unknown;
  try {
    const r = await fetchImpl(TS_URL, { cache: "no-store", credentials: "same-origin" });
    if (!r.ok) return { ok: false, problem: `The trade-sheet status file is not available (${r.status}). ${TS_HINT}` };
    body = await r.json();
  } catch {
    return { ok: false, problem: `The trade-sheet status file could not be read. ${TS_HINT}` };
  }
  return checkTradesheets(body);
}

export function checkTradesheets(body: unknown): TsLoad {
  const d = body as Partial<Tradesheets> | null;
  const okRow = (s: Partial<TsStrategy>) => s && typeof s.id === "string" && typeof s.status === "string";
  if (!d || typeof d !== "object" || !Array.isArray(d.strategies) || !d.strategies.every(okRow) || !d.totals || typeof d.totals.total !== "number")
    return { ok: false, problem: `The trade-sheet status file is not in the expected format. ${TS_HINT}` };
  return {
    ok: true,
    data: {
      ...(d as Tradesheets),
      statuses: Array.isArray(d.statuses) && d.statuses.length ? d.statuses : [...STATUSES],
      categories: Array.isArray(d.categories) ? d.categories : [],
      history: Array.isArray(d.history) ? d.history.map(h => ({ ...h, changes: h.changes || [] })) : [],
      strategies: d.strategies.map(s => ({ ...s, name: s.name || s.id, detail: s.detail || "", category: s.category || "", sheets: s.sheets || 0, matched: s.matched || 0, verified_by_jay: !!s.verified_by_jay, verified_as: s.verified_as || "" })),
    },
  };
}

/** The headline numbers: done, mismatch (not + partly), pending and without trade sheets. */
export function headline(t: Counts) {
  const n = (k: string) => t[k] || 0;
  return {
    done: n("Matching"), notMatching: n("Not matching"), partly: n("Partly matching"),
    mismatch: n("Not matching") + n("Partly matching"), pending: n("Not compared yet"), none: n("No trade sheets"),
    total: n("total") || STATUSES.reduce((a, k) => a + n(k), 0), jay: n("verified_by_jay"),
  };
}

/** CSS class suffix for a status. */
export const statusClass = (s: string | null) =>
  s === "Matching" ? "ok" : s === "Partly matching" ? "partly" : s === "Not matching" ? "bad" : s === "Not compared yet" ? "pending" : "none";

export type TsFilter = "all" | "jay" | TsStatus;
/** Strategies for a status chip and a search text (name, id, category or detail). */
export function filterStrategies(list: TsStrategy[], filter: TsFilter, q: string): TsStrategy[] {
  const t = q.trim().toLowerCase();
  return list.filter(s => (filter === "all" || (filter === "jay" ? s.verified_by_jay : s.status === filter))
    && (!t || `${s.name} ${s.id} ${s.category} ${s.detail}`.toLowerCase().includes(t)));
}
