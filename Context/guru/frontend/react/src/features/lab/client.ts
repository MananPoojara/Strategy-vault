import { ApiError } from "../../services/api";
import type {
  Catalogue, EngineStatus, FidelityResult, InstrumentsResponse, LabErrorFields, LabStrategy, OptimizeResult,
  PublishedStrategy, RunResult, SavedMarket, SeriesResponse, Spec, StrategyDraft, StrategyStatus, TemplateResponse,
  TemplatesResponse, TickerRow,
} from "./types";

export class LabError extends ApiError implements LabErrorFields {
  constructor(
    message: string, status: number,
    readonly kind: LabErrorFields["kind"],
    readonly engineStatus?: EngineStatus, readonly label?: string,
    readonly provisionalNote?: string, readonly retryAfter?: number,
  ) {
    super(message, status);
    this.name = "LabError";
  }
}

export const engineBase = (m: SavedMarket) => (m === "in" ? "/api/in" : m === "us" ? "/api/us" : "/api/gl");

/** Same rules as apiJson (same-origin cookie, /api/ only) plus engine error fields, Retry-After and 204. */
export async function labRequest<T>(path: string, init: RequestInit = {}): Promise<T> {
  if (!path.startsWith("/api/")) throw new Error("Only same-origin application API requests are supported.");
  const response = await fetch(path, { ...init, credentials: "same-origin", headers: { Accept: "application/json", ...init.headers } });
  if (response.status === 204) return null as T;
  let body: Record<string, unknown> | null = null;
  try { body = (await response.json()) as Record<string, unknown>; } catch { /* non-JSON: keep the status */ }
  if (!response.ok) {
    const engineStatus = typeof body?.status === "string" ? (body.status as EngineStatus) : undefined;
    const header = Number(response.headers.get("Retry-After"));
    const retryAfter = header > 0 ? header : response.status === 429 ? 5 : undefined;
    throw new LabError(
      typeof body?.error === "string" ? body.error : `Request failed (${response.status}).`, response.status,
      path.startsWith("/api/app/") ? "auth" : engineStatus ? "engine" : "gateway",
      engineStatus, typeof body?.label === "string" ? body.label : undefined,
      typeof body?.provisional_note === "string" ? body.provisional_note : undefined, retryAfter,
    );
  }
  if (body === null) throw new LabError("The server returned an unreadable response.", response.status, "gateway");
  return body as T;
}

// The gateway shares 4 read slots; never queue more than that from one page.
let active = 0;
const waiting: (() => void)[] = [];
async function slot<T>(task: () => Promise<T>): Promise<T> {
  while (active >= 4) await new Promise<void>((resolve) => waiting.push(resolve));
  active++;
  try { return await task(); } finally { active--; waiting.shift()?.(); }
}

type QueryValue = string | number | null | undefined;
const qs = (params: Record<string, QueryValue>) =>
  new URLSearchParams(Object.entries(params).filter(([, v]) => v !== undefined && v !== null && v !== "").map(([k, v]) => [k, String(v)])).toString();
const engine = <T>(m: SavedMarket, path: string, params: Record<string, QueryValue> = {}) => {
  const query = qs(params);
  return slot(() => labRequest<T>(`${engineBase(m)}/${path}${query ? `?${query}` : ""}`));
};

const cache = new Map<string, Promise<unknown>>();
function cached<T>(key: string, load: () => Promise<T>): Promise<T> {
  if (!cache.has(key)) cache.set(key, load().catch((error) => { cache.delete(key); throw error; }));
  return cache.get(key) as Promise<T>;
}
export const clearLabCache = () => cache.clear();

export interface RunQuery { ticker: string; from?: string; to?: string; cost?: number; slippage?: number }

export const catalogue = (m: SavedMarket) => cached(`cat:${engineBase(m)}`, () => engine<Catalogue>(m, "lab/catalogue"));
export function series(m: SavedMarket, q: { id: string; ticker: string; tf?: string; p?: Record<string, number> }) {
  const params: Record<string, QueryValue> = { id: q.id, ticker: q.ticker, tf: q.tf };
  for (const [k, v] of Object.entries(q.p ?? {})) params[`p.${k}`] = v;
  return engine<SeriesResponse>(m, "lab/series", params);
}
export const run = (m: SavedMarket, spec: Spec, q: RunQuery) => engine<RunResult>(m, "lab/run", { spec: JSON.stringify(spec), ...q });
/** slim=1 always: the page reads `picked`, never `rows` (full rows can reach the 64 MB gateway cap). Not slot-limited: it has its own server slot. */
export const optimize = (m: SavedMarket, spec: Spec, q: RunQuery & { split?: string | null }) =>
  labRequest<OptimizeResult>(`${engineBase(m)}/lab/optimize?${qs({ spec: JSON.stringify(spec), ...q, slim: "1" })}`);
export const tickers = (m: "in" | "us") => cached(`tickers:${m}`, () => engine<TickerRow[]>(m, "tickers"));
export const labInstruments = () => cached("gl:instruments", () => engine<InstrumentsResponse>("gl", "lab/instruments"));
export const templates = (m: SavedMarket) => cached(`templates:${engineBase(m)}`, () => engine<TemplatesResponse>(m, "lab/templates"));
export const template = (m: SavedMarket, method: string) => engine<TemplateResponse>(m, "lab/template", { method });
export const fidelity = (m: SavedMarket, method: string, ticker: string, from?: string, to?: string) =>
  engine<FidelityResult>(m, "lab/fidelity", { method, ticker, from, to });

const BASE = "/api/app/lab/strategies";
const send = (method: string, body?: unknown): RequestInit =>
  body === undefined ? { method } : { method, headers: { "Content-Type": "application/json" }, body: JSON.stringify(body) };
export const strategies = {
  list: (status?: StrategyStatus) => labRequest<LabStrategy[]>(BASE + (status ? `?status=${status}` : "")),
  published: () => labRequest<PublishedStrategy[]>(`${BASE}/published`),
  review: () => labRequest<LabStrategy[]>(`${BASE}/review`),
  get: (id: number) => labRequest<LabStrategy>(`${BASE}/${id}`),
  create: (draft: StrategyDraft) => labRequest<LabStrategy>(BASE, send("POST", draft)),
  update: (id: number, version: number, draft: StrategyDraft) => labRequest<LabStrategy>(`${BASE}/${id}`, send("PUT", { ...draft, version })),
  remove: (id: number, version: number) => labRequest<null>(`${BASE}/${id}?version=${version}`, send("DELETE")),
  duplicate: (id: number) => labRequest<LabStrategy>(`${BASE}/${id}/duplicate`, send("POST")),
  action: (id: number, action: "submit" | "withdraw" | "publish" | "reject", version: number, note?: string) =>
    labRequest<LabStrategy>(`${BASE}/${id}/${action}`, send("POST", note === undefined ? { version } : { version, note })),
};
export type StrategiesApi = typeof strategies;
