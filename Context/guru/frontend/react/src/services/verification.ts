/* Verification tracker: Yash Darji's workbook, published every 5 minutes as frontend-data/verification-tracker.json by
   authors/v1/sync_tracker.py. The tracker counts VERSIONS (one testable variant each), not the site's strategies. */

export type StatusKey = "done" | "done_generic" | "done_earlier" | "todo" | "cant_test" | "needs_data" | "other";
export interface AuditReport { file: string; path: string }
export interface TrackerRow {
  n?: number; id: string; author: string; book: string; strategy: string; version: string; status: StatusKey;
  good?: string; result?: string; checked?: string; type?: string; report?: AuditReport;
}
export interface TrackerAuthor { author: string; books: number; versions: number; testable: number; done: number; done_earlier: number; todo: number; cant_test: number; progress: number }
export interface TrackerBook { book: string; versions: number; testable: number; done: number; todo: number; progress: number }
export interface TrackerResult {
  author: string; book: string; strategy: string; id: string | null; check?: string; trades: number; right: number; wrong: number;
  to_check: number; result: string; notes?: string; checked: string; report?: AuditReport | null;
}
export interface TrackerGood {
  strategy: string; book?: string; index?: string; period?: string; settings?: string; params?: string; trades?: number;
  cagr?: number; maxdd?: number; car_mdd?: number; sharpe?: number; net?: number; id?: string | null; link?: string | null; report?: AuditReport | null;
}
export interface TrackerBug { n?: number; found?: string; where?: string; problem: string; evidence?: string; status: string }
export interface Tracker {
  schema: number; source_mtime: string; synced_at: string; snapshot?: string;
  overview: {
    kpi: { versions: number; testable: number; done: number; todo: number; cant_test: number };
    progress: number | null; progress_text?: string; notes: string[]; updated_text?: string; focus: string | null;
  };
  authors: TrackerAuthor[]; books: TrackerBook[];
  statusCounts: { key: StatusKey; label: string; count: number }[];
  results: TrackerResult[]; good: TrackerGood[]; good_rule?: string; bugs: TrackerBug[];
  legend: { label: string; key: StatusKey; text: string }[];
  howto?: { term: string; text: string }[];
  strategies: TrackerRow[];
  runs: { runs: number; strategies: number; trades: number; right: number; wrong: number; last_verified: string | null };
}

/** Status display order and colour class; the label comes from the workbook when it has one. */
export const STATUS_ORDER: StatusKey[] = ["done", "done_generic", "done_earlier", "todo", "needs_data", "cant_test", "other"];
export const STATUS_LABEL: Record<StatusKey, string> = {
  done: "Done", done_generic: "Done (generic checks)", done_earlier: "Done earlier (trades only)", todo: "To do",
  needs_data: "Site needs data", cant_test: "Can't test on site", other: "Other",
};
export const isDone = (s: StatusKey) => s === "done" || s === "done_generic" || s === "done_earlier";
export const isTestable = (s: StatusKey) => s !== "cant_test" && s !== "needs_data";

export async function loadTracker(fetcher: typeof fetch = fetch): Promise<Tracker> {
  const r = await fetcher("data/verification-tracker.json", { cache: "no-cache", credentials: "same-origin" });
  if (!r.ok) throw new Error(r.status === 404 ? "The verification tracker has not been synced yet." : `Could not load the verification tracker (${r.status}).`);
  const d = (await r.json()) as Tracker;
  if (d.schema !== 1 || !Array.isArray(d.strategies) || !d.overview?.kpi) throw new Error("The verification tracker file has an unknown format.");
  return d;
}

/** Status slices in display order, with the workbook's own label (symbol stripped). Empty statuses are dropped. */
export function statusSlices(t: Tracker) {
  const by = new Map<StatusKey, { count: number; label: string }>();
  for (const c of t.statusCounts) {
    const cur = by.get(c.key) || { count: 0, label: c.label.replace(/^[^\p{L}\d]+/u, "") || STATUS_LABEL[c.key] };
    by.set(c.key, { ...cur, count: cur.count + c.count });
  }
  return STATUS_ORDER.filter(k => by.get(k)?.count).map(key => ({ key, ...by.get(key)! }));
}

export interface Filters { q: string; author: string; book: string; status: StatusKey | "" | "done_any" }
const fold = (s: string) => s.toLocaleLowerCase("en").normalize("NFKD").replace(/[̀-ͯ]/g, "");
const searchCache = new WeakMap<TrackerRow, string>();
const haystack = (r: TrackerRow) => {
  let h = searchCache.get(r);
  if (h == null) searchCache.set(r, (h = fold([r.strategy, r.version, r.book, r.author, r.id, r.result || ""].join(" "))));
  return h;
};
export function filterRows(rows: TrackerRow[], f: Filters) {
  const words = fold(f.q).split(/\s+/).filter(Boolean);
  return rows.filter(r => (!f.author || r.author === f.author) && (!f.book || r.book === f.book)
    && (!f.status || (f.status === "done_any" ? isDone(r.status) : r.status === f.status))
    && (!words.length || words.every(w => haystack(r).includes(w))));
}

/** Books of one author. The focus author uses the tracker's own Overview table; any other author is counted from the
    Strategy list (testable = not "can't test" / "needs data"). */
export function booksFor(t: Tracker, author: string): { rows: TrackerBook[]; fromOverview: boolean } {
  if (author && author === t.overview.focus && t.books.length) return { rows: t.books, fromOverview: true };
  const by = new Map<string, TrackerBook>();
  for (const r of t.strategies) {
    if (r.author !== author) continue;
    const b = by.get(r.book) || { book: r.book, versions: 0, testable: 0, done: 0, todo: 0, progress: 0 };
    b.versions++;
    if (isTestable(r.status)) b.testable++;
    if (isDone(r.status)) b.done++;
    if (r.status === "todo") b.todo++;
    by.set(r.book, b);
  }
  const rows = [...by.values()].map(b => ({ ...b, progress: b.testable ? b.done / b.testable : 0 }));
  return { rows: rows.sort((a, b) => b.progress - a.progress || b.testable - a.testable || a.book.localeCompare(b.book)), fromOverview: false };
}

/** Our strategy page for a tracker Strategy ID (the site's route is #/s/<id>; ?tk= keeps the index the result used). */
export function strategyHref(id: string, link?: string | null) {
  const tk = link && /[?&]tk=([^&#]+)/.exec(link)?.[1];
  return `#/s/${encodeURIComponent(id)}${tk ? `?tk=${tk}` : ""}`;
}

export const pct = (x: number | null | undefined, digits = 0) => (x == null ? "–" : `${(100 * x).toFixed(digits)}%`);
/** The audit report's folder on the share (the file name is shown separately). */
export const reportFolder = (r: AuditReport) => r.path.slice(0, Math.max(0, r.path.length - r.file.length - 1));
