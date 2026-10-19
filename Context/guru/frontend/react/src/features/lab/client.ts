import { ApiError } from "../../services/api";
import { parseFormulaInfo } from "./formula";
import type {
  Catalogue, EngineStatus, FormulaInfo, IndicatorDraft, SavedIndicator, ForeignDisclosure, IndividualOptimizeResult, Provisional, FidelityResult, InstrumentsResponse, LabErrorFields, LabStrategy, OptimizeResult,
  PublishedStrategy, PortfolioSpec, PortfolioRunResult, PortfolioOptimizeResult, LabRunResult, LabOptimizeResult, RunResult, SavedMarket, SeriesResponse, Spec, StrategyDraft, StrategyStatus, TemplateResponse,
  TemplatesResponse, TickerRow, WalkforwardOptions, WalkforwardResult,
} from "./types";

export class LabError extends ApiError implements LabErrorFields {
  constructor(
    message: string, status: number,
    readonly kind: LabErrorFields["kind"],
    readonly engineStatus?: EngineStatus, readonly label?: string,
    readonly provisionalNote?: string, readonly retryAfter?: number, readonly read = false,
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
      (init.method ?? "GET").toUpperCase() === "GET",
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
/** A formula chip sends `formula` and `name` instead of `id`, and its Params as `<Param>=<value>` (custom indicators contract §2). */
export function series(m: SavedMarket, q: { id: string; ticker: string; tf?: string; p?: Record<string, number>; vs?: string; formula?: string; name?: string }) {
  const params: Record<string, QueryValue> = q.formula !== undefined
    ? { formula: q.formula, name: q.name, ticker: q.ticker, tf: q.tf }
    : { id: q.id, ticker: q.ticker, tf: q.tf, vs: q.vs };
  for (const [k, v] of Object.entries(q.p ?? {})) params[q.formula !== undefined ? k : `p.${k}`] = v;
  return engine<SeriesResponse>(m, "lab/series", params);
}
/** Check a formula (lab/formula): its Params, Plots and placement; a refusal is a LabError "line L, col C: …". Cached per text. */
export const formulaInfo = (m: SavedMarket, formula: string): Promise<FormulaInfo> =>
  cached(`formula:${engineBase(m)}:${formula}`, () => engine<unknown>(m, "lab/formula", { formula }).then(parseFormulaInfo));
export type PortfolioRunQuery = Omit<RunQuery,"ticker"|"cost"|"slippage">;
const executionQuery = (spec: Spec, q: (RunQuery | PortfolioRunQuery) & {split?: string | null}) => spec.portfolio ? {from:q.from,to:q.to} : q;
export function run(m:SavedMarket,spec:Spec&{portfolio:PortfolioSpec},q:PortfolioRunQuery):Promise<PortfolioRunResult>;
export function run(m:SavedMarket,spec:Spec,q:RunQuery):Promise<LabRunResult>;
export function run(m:SavedMarket,spec:Spec,q:RunQuery|PortfolioRunQuery):Promise<LabRunResult>{return engine<LabRunResult>(m,"lab/run",{spec:JSON.stringify(spec),...executionQuery(spec,q)});}
/** slim=1 always: the page reads `picked`, never `rows` (full rows can reach the 64 MB gateway cap); table=1 adds the compact all-results list. Not slot-limited: it has its own server slot. */
export function optimize(m:SavedMarket,spec:Spec&{portfolio:PortfolioSpec},q:PortfolioRunQuery):Promise<PortfolioOptimizeResult>;
export function optimize(m:SavedMarket,spec:Spec,q:RunQuery&{split?:string|null}):Promise<LabOptimizeResult>;
export function optimize(m:SavedMarket,spec:Spec,q:(RunQuery|PortfolioRunQuery)&{split?:string|null}):Promise<LabOptimizeResult>{return labRequest<LabOptimizeResult>(`${engineBase(m)}/lab/optimize?${qs({spec:JSON.stringify(spec),...executionQuery(spec,q),slim:"1",table:"1"})}`);}
/** One optimizer run per window, so it shares the optimization slot and role check; slim on the server side. */
export const walkforward = (m: SavedMarket, spec: Spec, q: RunQuery & WalkforwardOptions) =>
  labRequest<WalkforwardResult>(`${engineBase(m)}/lab/walkforward?${qs({ spec: JSON.stringify(spec), ...q, anchored: q.anchored ? "1" : "0" })}`);
/** AmiBroker Individual Optimize: each symbol of the portfolio universe optimized alone (same /lab/optimize route, mode=individual). */
export const optimizeIndividual = (m: SavedMarket, spec: Spec, q: PortfolioRunQuery) =>
  labRequest<IndividualOptimizeResult>(`${engineBase(m)}/lab/optimize?${qs({ spec: JSON.stringify(spec), from: q.from, to: q.to, mode: "individual", table: "1" })}`);
/** AmiBroker Analysis Scan / Explore (/lab/scan): raw rule signals, not filled trades. Rows are [symbol, date, trade, close]. */
export interface ScanBase extends Provisional { label: string; apply: "current" | "all"; from: string | null; to: string | null; notes: string[]; foreign?: ForeignDisclosure[] }
export interface ScanResult extends ScanBase { kind: "lab_scan"; symbols: number; rows: [string, string, "Buy" | "Sell" | "Short" | "Cover", number | null][]; refused: { ticker: string; reason: string }[]; membership?: { kind: string; note: string } }
export interface ExploreResult extends ScanBase { kind: "lab_explore"; dates: string[]; open: (number | null)[]; high: (number | null)[]; low: (number | null)[]; close: (number | null)[]; volume: (number | null)[] | null; signals: Record<"buy" | "sell" | "short" | "cover", (0 | 1)[]> }
export interface ScanQuery { ticker?: string; from?: string; to?: string; apply?: "current" | "all" }
export const scan = (m: SavedMarket, spec: Spec, q: ScanQuery) =>
  engine<ScanResult>(m, "lab/scan", { spec: JSON.stringify(spec), mode: "scan", apply: q.apply ?? "current", ticker: q.apply === "all" ? undefined : q.ticker, from: q.from, to: q.to });
export const explore = (m: SavedMarket, spec: Spec, q: Omit<ScanQuery, "apply">) =>
  engine<ExploreResult>(m, "lab/scan", { spec: JSON.stringify(spec), mode: "explore", ticker: q.ticker, from: q.from, to: q.to });
export const tickers = (m: "in" | "us") => cached(`tickers:${m}`, () => engine<TickerRow[]>(m, "tickers"));
export const labInstruments = () => cached("gl:instruments", () => engine<InstrumentsResponse>("gl", "lab/instruments"));
export const templates = (m: SavedMarket) => cached(`templates:${engineBase(m)}`, () => engine<TemplatesResponse>(m, "lab/templates"));
/** Cached: card summaries and the selected method share one answer per method. */
export const template = (m: SavedMarket, method: string) =>
  cached(`template:${engineBase(m)}:${method}`, () => engine<TemplateResponse>(m, "lab/template", { method }));
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

const IND = "/api/app/lab/indicators";
/** The user's own formulas ("My indicators"); the server keeps text only, the engine is the authority on the formula. */
export const indicators = {
  list: () => labRequest<SavedIndicator[]>(IND),
  create: (draft: IndicatorDraft) => labRequest<SavedIndicator>(IND, send("POST", draft)),
  update: (id: number, version: number, draft: IndicatorDraft) => labRequest<SavedIndicator>(`${IND}/${id}`, send("PUT", { ...draft, version })),
  remove: (id: number, version: number) => labRequest<null>(`${IND}/${id}?version=${version}`, send("DELETE")),
};
export type IndicatorsApi = typeof indicators;
