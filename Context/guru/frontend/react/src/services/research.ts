import { apiJson } from "./api";

export type Site = "verified" | "held" | "unreviewed";
export type Verdict = "not_checked" | "matches" | "mismatch" | "unclear";
export type NoteType = "rule_error" | "data_issue" | "question" | "verified_ok";
export type TargetType = "strategy" | "book" | "author" | "general";

export const VERDICTS: Record<Verdict, string> = {
  not_checked: "Not checked",
  matches: "Matches the book",
  mismatch: "Mismatch",
  unclear: "Unclear",
};
export const NOTE_TYPES: Record<NoteType, string> = {
  rule_error: "Rule error",
  data_issue: "Data issue",
  question: "Question",
  verified_ok: "Verified OK",
};
export const SITE: Record<Site, string> = {
  verified: "Backtested",
  held: "Held with a reason",
  unreviewed: "Not reviewed yet",
};

export interface Page<T> { items: T[]; total: number; page: number; size: number; pages: number }
export interface Strategy {
  key: string; author: string; author_id: string; book: string; book_id: string; name: string; site: Site;
  reason: string; rules: { kind: string; text: string }[]; section?: string; coding_notes?: string; variant: string; variants: number; url: string;
  verdict: Verdict; verdict_by: string | null; verdict_at: string | null; notes: number; rule_errors: number;
}
export interface Note {
  id: number; target_type: TargetType; target_key: string; target_label: string; note_type: NoteType; body: string;
  author_id: string | null; author_name: string; imported: boolean; created_at: string; updated_at: string; edits: number;
  deleted_at: string | null; deleted_reason: string | null; deleted_by_name: string | null; can_edit: boolean; can_delete: boolean;
}
export interface NoteHistory { action: "created" | "edited" | "deleted"; note_type: NoteType; body: string; actor_name: string; created_at: string }
export interface TimelineItem {
  kind: "note" | "verdict" | "platform"; at: string; who: string; actor_id: string | null; action: string; detail: string | null;
  text: string | null; ref: number | null; target_type: TargetType | null; target_key: string; target_label: string | null;
  backtested: number | null; proof: string; removed?: boolean;
}
export interface AuthorRow { name: string; share_books: number; site_books: number; strategies: number; verified: number; held: number; unreviewed: number; pct: string; loop: string }
export interface BookRow { author: string; title: string; year: number | string; strategies: number; verified: number; held: number; unreviewed: number; status: string; pct: string }
export interface Summary {
  as_of: string; generated_at: string; run: string;
  totals: Record<string, number>;
  reasons: { reason: string; count: number }[];
  history: { as_of: string; strategies: number; verified: number; held: number; unreviewed: number }[];
  authors: AuthorRow[]; books: BookRow[];
  todo_authors: { name: string; share_books: number; works: number; status: string }[];
  deploys: { at: string; count: number; work: string; label: string }[];
  team: { verdicts: Record<Verdict, number>; notes: Record<NoteType, number>; people: number };
  activity: { day: string; verdicts: number; notes: number }[];
  strip: [string, string, string][];
  author_index: { id: string; name: string }[];
  book_index: { id: string; title: string; author: string }[];
}
export interface StrategyDetail {
  strategy: Omit<Strategy, "verdict" | "verdict_by" | "verdict_at" | "notes" | "rule_errors"> | null;
  verdict: { verdict: Verdict; author_name?: string; updated_at?: string };
  history: { verdict: Verdict; previous: Verdict | null; actor_name: string; created_at: string }[];
  notes: Note[];
}
export interface Person {
  id: string; name: string; role: string | null; verdicts: number; matches: number; mismatches: number; unclear: number;
  notes: number; rule_errors: number; edits: number; last_at: string | null; imported: boolean;
}

const BASE = "/api/app/research";
export function query(params: Record<string, string | number | undefined>) {
  const q = new URLSearchParams();
  for (const [k, v] of Object.entries(params)) if (v !== undefined && v !== "") q.set(k, String(v));
  const s = q.toString();
  return s ? `?${s}` : "";
}
const send = <T>(path: string, method: string, body?: unknown) =>
  apiJson<T>(BASE + path, { method, headers: { "Content-Type": "application/json" }, body: JSON.stringify(body ?? {}) });

export const research = {
  summary: () => apiJson<Summary>(`${BASE}/summary`),
  strategies: (p: Record<string, string | number | undefined>) =>
    apiJson<Page<Strategy> & { facets: { authors: { id: string; name: string }[]; books: { id: string; title: string }[]; reviewers: string[] } }>(`${BASE}/strategies${query(p)}`),
  strategy: (key: string) => apiJson<StrategyDetail>(`${BASE}/strategy${query({ key })}`),
  setVerdict: (key: string, verdict: Verdict) => send<{ previous: Verdict; changed: boolean }>("/verdict", "PUT", { key, verdict }),
  notes: (p: Record<string, string | number | undefined>) => apiJson<Page<Note>>(`${BASE}/notes${query(p)}`),
  addNote: (n: { target_type: TargetType; target_key: string; note_type: NoteType; body: string }) => send<Note>("/notes", "POST", n),
  editNote: (id: number, n: { body?: string; note_type?: NoteType }) => send<Note>(`/notes/${id}`, "PATCH", n),
  removeNote: (id: number, reason: string) => send<Note>(`/notes/${id}`, "DELETE", { reason }),
  noteHistory: (id: number) => apiJson<NoteHistory[]>(`${BASE}/notes/${id}/history`),
  timeline: (p: Record<string, string | number | undefined>) => apiJson<Page<TimelineItem>>(`${BASE}/timeline${query(p)}`),
  people: () => apiJson<Person[]>(`${BASE}/people`),
};

const IST: Intl.DateTimeFormatOptions = { timeZone: "Asia/Kolkata" };
export const fmtDate = (iso: string) => new Date(iso).toLocaleDateString("en-IN", { ...IST, day: "numeric", month: "short", year: "numeric" });
export const fmtTime = (iso: string) => new Date(iso).toLocaleTimeString("en-IN", { ...IST, hour: "2-digit", minute: "2-digit", hour12: false });
export const fmtStamp = (iso: string) => `${fmtDate(iso)}, ${fmtTime(iso)}`;
export const dayKey = (iso: string) => new Date(iso).toLocaleDateString("en-CA", IST);
export function ago(iso: string, now = Date.now()) {
  const s = Math.round((now - Date.parse(iso)) / 1000);
  if (s < 60) return "just now";
  if (s < 3600) return `${Math.round(s / 60)} min ago`;
  if (s < 86400) return `${Math.round(s / 3600)} h ago`;
  if (s < 86400 * 14) return `${Math.round(s / 86400)} d ago`;
  return fmtDate(iso);
}

/** Tab and filters live in the hash (#/research?tab=review&author=connors) so views can be shared and survive reload. */
export function readHashParams(hash = location.hash) {
  return new URLSearchParams(hash.split("?")[1] || "");
}
export function hashWith(patch: Record<string, string | undefined>, hash = location.hash) {
  const p = readHashParams(hash);
  for (const [k, v] of Object.entries(patch)) if (v === undefined || v === "") p.delete(k); else p.set(k, v);
  const s = p.toString();
  return `#/research${s ? `?${s}` : ""}`;
}
