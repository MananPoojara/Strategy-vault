/* YT Strategies page: the current trade-sheet status per collection, read from data/research-tradesheets.json
   (the same file as Research > Tradesheet verified). Each strategy goes to the collection whose book the
   frontend catalogue gives it; a strategy missing from the catalogue falls back to its workbook category. */
import { checkTradesheets, type Tradesheets, type TsStrategy } from "./research-tradesheets";

export const YT_STATUS_LINK = "#/research?v=tradesheets";

export interface YtSetRef { key: string; book: string; name: string }
export interface YtSetStatus {
  matching: number; partly: number; notMatching: number; pending: number; none: number;
  /** Strategies with a trade-sheet verdict (matching, partly or not matching). */
  compared: number; total: number;
  /** Team trade sheets compared and matched, summed over the strategies that report them. */
  sheets: number; sheetsMatched: number;
  /** Partly and not matching strategies, for the panel list. */
  problems: TsStrategy[];
}
export interface YtStatusIndex { sets: Record<string, YtSetStatus>; generatedAt: string; made: string; file: string; unmapped: TsStrategy[] }

const norm = (s: string) => s.toLowerCase().replace(/[^a-z0-9]+/g, " ").trim();
const empty = (): YtSetStatus => ({ matching: 0, partly: 0, notMatching: 0, pending: 0, none: 0, compared: 0, total: 0, sheets: 0, sheetsMatched: 0, problems: [] });

/** Group the strategies of research-tradesheets.json by YT collection. Returns null when the file is unusable. */
export function ytStatusBySet(body: unknown, catalogue: { id: string; book: string }[], sets: YtSetRef[]): YtStatusIndex | null {
  const load = checkTradesheets(body);
  if (!load.ok) return null;
  const d: Tradesheets = load.data;
  const bookOf = new Map(catalogue.map(v => [v.id, v.book]));
  const setOfBook = new Map(sets.map(s => [s.book, s.key]));
  const setOfName = new Map(sets.map(s => [norm(s.name), s.key]));
  const out: Record<string, YtSetStatus> = {}, unmapped: TsStrategy[] = [];
  for (const s of d.strategies) {
    const book = bookOf.get(s.id);
    const key = (book && setOfBook.get(book)) || setOfName.get(norm(s.category));
    if (!key) { unmapped.push(s); continue; }
    const t = (out[key] ||= empty());
    t.total++;
    if (s.status === "Matching") t.matching++;
    else if (s.status === "Partly matching") t.partly++;
    else if (s.status === "Not matching") t.notMatching++;
    else if (s.status === "Not compared yet") t.pending++;
    else t.none++;
    if (s.status === "Partly matching" || s.status === "Not matching") t.problems.push(s);
    if (s.sheets > 0) { t.sheets += s.sheets; t.sheetsMatched += Math.min(s.matched, s.sheets); }
  }
  for (const t of Object.values(out)) t.compared = t.matching + t.partly + t.notMatching;
  return { sets: out, generatedAt: d.generated_at || "", made: d.source?.made || "", file: d.source?.file || "", unmapped };
}

/** "20-11-2026 18:31" in the viewer's time zone; the raw text when it is not a date. */
export function statusTime(iso: string): string {
  const t = new Date(iso);
  if (!iso || Number.isNaN(t.getTime())) return iso || "";
  const p = (n: number) => String(n).padStart(2, "0");
  return `${p(t.getDate())}-${p(t.getMonth() + 1)}-${t.getFullYear()} ${p(t.getHours())}:${p(t.getMinutes())}`;
}

const esc = (s: unknown) => String(s ?? "").replace(/[&<>"']/g, c => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;" })[c]!);
const hue = (t: YtSetStatus) => Math.round(120 * t.matching / t.compared);

/** Card badge: "<matching> of <compared> match"; a dash when nothing in the collection has been compared yet. */
export function ytStatusBadge(t: YtSetStatus | undefined): string {
  if (!t || !t.compared) {
    const why = !t ? "No strategies of this collection in the trade-sheet status" : t.pending ? `${t.pending} waiting for a trade-sheet comparison` : "No trade sheets to compare";
    return `<span class="yt-match none" data-yt-match data-yt-status title="${esc(why)}">${t?.pending ? `${t.pending} pending` : "–"}</span>`;
  }
  const title = `${t.matching} of ${t.compared} compared strategies match their trade sheets (${t.partly} partly, ${t.notMatching} not matching; ${t.pending} pending, ${t.none} without trade sheets)`;
  return `<span class="yt-match" data-yt-match data-yt-status style="--yt-h:${hue(t)}" title="${esc(title)}">${t.matching} of ${t.compared} match</span>`;
}

/** Short text for the side toggle button. */
export const ytStatusToggle = (t: YtSetStatus | undefined) => (t && t.compared ? `${t.matching}/${t.compared}` : "");

/** Panel body: current per-strategy status as the main figures. `earlier` is the older sheet tally (HTML), shown folded. */
export function ytStatusPanel(t: YtSetStatus | undefined, idx: YtStatusIndex, earlier = "", earlierMade = ""): string {
  const asOf = `<p class="ref-sub" data-yt-status-asof>Status as of ${esc(statusTime(idx.generatedAt))}${idx.file ? ` · from ${esc(idx.file)}${idx.made ? ` (made ${esc(idx.made)})` : ""}` : ""}. <a href="${YT_STATUS_LINK}">All strategies in Research → Tradesheet verified</a></p>`;
  let body: string;
  if (!t) body = '<p class="muted">No strategy of this collection is listed in the trade-sheet status workbook.</p>';
  else {
    const cell = (n: number, label: string, cls = "") => `<div><b${cls ? ` class="${cls}"` : ""}>${n}</b><span>${label}</span></div>`;
    const pct = t.compared ? Math.round(100 * t.matching / t.compared) : null;
    body = `<p class="yt-sheets-pct"><b>${t.total}</b> strategies in the status workbook</p>
    <div class="yt-sheets-counts yt-status-counts" data-yt-status-counts>${cell(t.matching, "matching", "pos")}${cell(t.partly, "partly matching", "partly")}${cell(t.notMatching, "not matching", "neg")}${cell(t.pending, "pending")}${cell(t.none, "no trade sheets")}</div>
    ${pct === null ? `<p class="yt-sheets-pct">No strategy compared with trade sheets yet.</p>` : `<div class="yt-sheets-bar" style="--yt-h:${hue(t)}" role="img" aria-label="${t.matching} of ${t.compared} strategies match"><i style="width:${pct}%"></i></div><p class="yt-sheets-pct">${t.matching} of ${t.compared} compared strategies match</p>`}
    ${t.sheets ? `<p class="yt-sheets-pct" data-yt-status-sheets>Trade sheets: <b>${t.sheetsMatched}</b> of <b>${t.sheets}</b> compared sheets match</p>` : ""}
    ${t.problems.length ? `<h4>Partly or not matching</h4><ol class="yt-sheets-list">${t.problems.map(s => `<li><div><b class="${s.status === "Not matching" ? "neg" : "partly"}">${esc(s.status)}</b> · ${esc(s.name)}${s.sheets ? ` · ${Math.min(s.matched, s.sheets)} of ${s.sheets} sheets` : ""}</div><div class="ref-path">${esc(s.id)}</div>${s.detail ? `<div class="ref-sub">${esc(s.detail)}</div>` : ""}</li>`).join("")}</ol>` : ""}`;
  }
  const old = earlier ? `<details class="yt-sheets-earlier" data-yt-sheets-earlier><summary>Earlier sheet comparison${earlierMade ? ` (${esc(earlierMade)})` : ""}</summary>${earlier}</details>` : "";
  return `${body}${asOf}${old}`;
}
