import { apiJson } from "./api";

/* Research YTS: QA issues on the YT Strategies pages, filed per collection and strategy (auth service, research-yts.mjs). */

export type Category = "wrong_trades" | "wrong_stats" | "data" | "page" | "rules" | "other";
export type Severity = "low" | "normal" | "high";
export type Status = "open" | "in_progress" | "fixed" | "not_issue";

export const CATEGORIES: Record<Category, string> = {
  wrong_trades: "Wrong trades (entry / exit)",
  wrong_stats: "Wrong statistics or report numbers",
  data: "Data problem",
  page: "Page or display problem",
  rules: "Rule or settings question",
  other: "Other",
};
export const SEVERITIES: Record<Severity, string> = { low: "Low", normal: "Normal", high: "High" };
export const STATUSES: Record<Status, string> = { open: "Open", in_progress: "In progress", fixed: "Fixed", not_issue: "Not an issue" };

export interface Issue {
  id: number; collection: string; strategy_id: string; category: Category; severity: Severity; status: Status;
  title: string; body: string; instrument: string; period_from: string | null; period_to: string | null; page_link: string;
  reporter_name: string; reporter_id: number | null; created_at: string; updated_at: string; deleted_at: string | null; deleted_reason: string | null;
  comments: number; files: number; can_edit: boolean; can_delete: boolean;
  /** A general issue (collection "general"): the strategies it is about and their collections; empty means all YT strategies. */
  strategies: string[]; collections: string[];
}
/** The collection of a general issue: one issue about several strategies, possibly in different collections. */
export const GENERAL = "general";
export interface IssueFile { id: number; name: string; content_type: string; size: number; image: boolean; uploader_name: string; created_at: string; can_delete: boolean }
export const MAX_FILE_BYTES = 10 * 1024 * 1024;
export const fileUrl = (issue: number, file: number, download = false) => `${BASE}/issues/${issue}/files/${file}${download ? "?download=1" : ""}`;
export const fmtSize = (n: number) => n < 1024 ? `${n} B` : n < 1024 * 1024 ? `${Math.round(n / 1024)} KB` : `${(n / 1024 / 1024).toFixed(1)} MB`;
export interface IssueEvent { id: number; action: "created" | "edited" | "status" | "comment" | "deleted" | "file"; status: Status | null; body: string; actor_name: string; created_at: string }
export interface Page<T> { items: T[]; total: number; page: number; size: number; pages: number }
export type Counts = Record<Status, number>;
export interface Summary { totals: Counts; collections: Record<string, Counts>; strategies: Record<string, Counts> }
export interface NewIssue {
  collection: string; strategy_id: string; category: Category; severity: Severity; title: string; body: string;
  instrument?: string; period_from?: string; period_to?: string; page_link?: string;
  /** General issues only. */
  strategies?: { collection: string; id: string }[];
}
/** The person signed in to Research YTS (its own login, separate from the app account). */
export interface Qa { id: number; username: string; name: string; admin: boolean }
export interface QaAccount extends Qa { created_at: string; last_login_at: string | null; deactivated_at: string | null; issues: number; open: number }
/** One YT strategy from the engine catalogue (only the fields this page reads). */
export interface YtStrategy { id: string; name: string; book: string; variant?: string; family_name?: string }
export interface YtCollection { key: string; name: string; book: string }

const BASE = "/api/app/research-yts";
export function query(params: Record<string, string | number | undefined>) {
  const q = new URLSearchParams();
  for (const [k, v] of Object.entries(params)) if (v !== undefined && v !== "") q.set(k, String(v));
  const s = q.toString();
  return s ? `?${s}` : "";
}
const send = <T>(path: string, method: string, body?: unknown) =>
  apiJson<T>(BASE + path, { method, headers: { "Content-Type": "application/json" }, body: JSON.stringify(body ?? {}) });

export const yts = {
  me: () => apiJson<{ qa: Qa | null }>(`${BASE}/me`),
  login: (username: string, password: string) => send<{ qa: Qa }>("/login", "POST", { username, password }),
  logout: () => send<{ ok: boolean }>("/logout", "POST"),
  changePassword: (current: string, next: string) => send<{ ok: boolean }>("/password", "POST", { current, next }),
  accounts: () => apiJson<QaAccount[]>(`${BASE}/qa-users`),
  addAccount: (a: { username: string; name: string; password: string; admin: boolean }) => send<QaAccount>("/qa-users", "POST", a),
  resetPassword: (id: number, password: string) => send<{ ok: boolean }>(`/qa-users/${id}/password`, "POST", { password }),
  setActive: (id: number, active: boolean) => send<QaAccount>(`/qa-users/${id}/${active ? "reactivate" : "deactivate"}`, "POST"),
  summary: () => apiJson<Summary>(`${BASE}/summary`),
  issues: (p: Record<string, string | number | undefined>) => apiJson<Page<Issue>>(`${BASE}/issues${query(p)}`),
  issue: (id: number) => apiJson<{ issue: Issue; events: IssueEvent[]; files: IssueFile[] }>(`${BASE}/issues/${id}`),
  /** Uploads one file (the request body is the file itself). */
  upload: (id: number, file: File) => apiJson<{ issue: Issue; files: IssueFile[] }>(`${BASE}/issues/${id}/files${query({ name: file.name || "pasted-image.png" })}`,
    { method: "POST", headers: { "Content-Type": file.type || "application/octet-stream" }, body: file }),
  removeFile: (id: number, file: number) => send<{ issue: Issue; files: IssueFile[] }>(`/issues/${id}/files/${file}`, "DELETE"),
  create: (n: NewIssue) => send<Issue>("/issues", "POST", n),
  edit: (id: number, n: Partial<Pick<Issue, "title" | "body" | "category" | "severity" | "instrument">> & { strategies?: { collection: string; id: string }[] }) => send<Issue>(`/issues/${id}`, "PATCH", n),
  setStatus: (id: number, status: Status, note: string) => send<Issue>(`/issues/${id}/status`, "PUT", { status, note }),
  comment: (id: number, body: string) => send<Issue>(`/issues/${id}/comments`, "POST", { body }),
  remove: (id: number, reason: string) => send<Issue>(`/issues/${id}`, "DELETE", { reason }),
  catalogue: () => apiJson<YtStrategy[] | { strategies: YtStrategy[] }>("/api/in/guru/yt/strategies")
    .then(d => (Array.isArray(d) ? d : d.strategies || [])),
};

/** The YT collections as the YT Strategies page groups them (book id → collection name). */
export const ytCollections = (): YtCollection[] => window.VaultRuntime?.ytCollections?.() ?? [];

const IST: Intl.DateTimeFormatOptions = { timeZone: "Asia/Kolkata" };
export const fmtDate = (iso: string) => new Date(iso).toLocaleDateString("en-IN", { ...IST, day: "numeric", month: "short", year: "numeric" });
export const fmtStamp = (iso: string) => `${fmtDate(iso)}, ${new Date(iso).toLocaleTimeString("en-IN", { ...IST, hour: "2-digit", minute: "2-digit", hour12: false })}`;

/** Filters and the open issue live in the hash (#/research-yts?c=yt_bb&issue=12) so links can be shared. */
export const readHash = (hash = location.hash) => new URLSearchParams(hash.split("?")[1] || "");
export function hashWith(patch: Record<string, string | undefined>, hash = location.hash) {
  const p = readHash(hash);
  for (const [k, v] of Object.entries(patch)) if (v === undefined || v === "") p.delete(k); else p.set(k, v);
  const s = p.toString();
  return `#/research-yts${s ? `?${s}` : ""}`;
}
