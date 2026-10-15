/** Phase 8 foreign leaves. Discovery follows the existing rule walker; the engine decides admission/alignment. */
import { isSeries } from "./spec-model";
import { walkSpec } from "./rule-tree";
import type { Catalogue, ForeignDisclosure, Market, Operand, SeriesOperand, Spec } from "./types";
import type { InstrumentOption } from "./instruments";

export const PAIR_IDS = ["correlation", "correlation_log", "ratio", "spread"] as const;
export type ForeignField = "symbol" | "vs" | "fixup";
export interface ForeignOperand extends SeriesOperand { symbol?: string; vs?: string; fixup?: "none" | "previous"; [extra: string]: unknown }
export interface ForeignCatalogue extends Catalogue { max_foreign?: number; foreign_volatility?: string | null }
export interface ForeignScope {
  catalogue: ForeignCatalogue; ticker: string; market: Market; options: readonly InstrumentOption[];
  /** Market-specific volatility ticker advertised by the API, never guessed from a filename. */
  volatility?: string | null; spec?: Spec; allowSymbol?: boolean; portfolio?: boolean;
}
export interface ForeignReference { path: string; field: "symbol" | "vs"; ticker: string }
const object = (v: unknown): v is Record<string, unknown> => !!v && typeof v === "object" && !Array.isArray(v);
const fields: ForeignField[] = ["symbol", "vs", "fixup"];
const hasFields = (v: Record<string, unknown>) => fields.some((k) => Object.hasOwn(v, k));
export const isPair = (id: string) => (PAIR_IDS as readonly string[]).includes(id);
export const foreignAdvertised = (catalogue: Pick<Catalogue, "max_foreign">) => Number.isInteger(catalogue.max_foreign) && (catalogue.max_foreign ?? 0) > 0;
export const foreignEnabled = (scope: ForeignScope) => foreignAdvertised(scope.catalogue);
export const foreignLimit = (scope: ForeignScope) => Math.min(4, foreignEnabled(scope) ? scope.catalogue.max_foreign! : 4);
/** An explicit edit changes only that field; incompatible saved fields are left for the user to resolve. */
export function withForeignField(o: Operand, field: ForeignField, value: string | undefined): Operand {
  const next = { ...o } as Record<string, unknown>;
  if (value === undefined) delete next[field]; else next[field] = value;
  return next as Operand;
}
export function foreignOptions(scope: ForeignScope): InstrumentOption[] {
  const options = new Map<string, InstrumentOption>();
  for (const o of scope.options) {
    const market = o.market === "crypto" ? "gl" : o.market;
    if (!market || market === scope.market) options.set(o.ticker, o);
  }
  const vix = scope.volatility ?? scope.catalogue.foreign_volatility;
  if (vix) options.set(vix, { ticker: vix, label: vix, disabled: false, hasVolume: false });
  return [...options.values()].sort((a, b) => a.ticker < b.ticker ? -1 : a.ticker > b.ticker ? 1 : 0);
}
/** Score uses the same expression grammar. The wrapper is only a traversal adapter until walkSpec includes portfolio.score. */
function operands(spec: Spec, visit: (o: Operand, path: string) => void) {
  const seen = new Set<string>();
  const emit = (o: Operand, path: string) => { if (!seen.has(path)) { seen.add(path); visit(o, path); } };
  walkSpec(spec, (item) => { if (item.t === "operand") emit(item.operand, item.path); });
  const p = object(spec.portfolio) ? spec.portfolio : undefined;
  if (p?.score !== undefined) {
    const wrapper: Spec = { v: 1, entry: { all: [{ lhs: p.score as Operand, op: ">", rhs: { value: 0 } }] }, exit: { any: [] } };
    walkSpec(wrapper, (item) => { if (item.t === "operand" && item.path.startsWith("entry.all[0].lhs")) emit(item.operand, item.path.replace("entry.all[0].lhs", "portfolio.score")); });
  }
}
export function foreignReferences(spec: Spec): ForeignReference[] {
  const result: ForeignReference[] = [];
  operands(spec, (o, path) => {
    if (!isSeries(o)) return;
    const v = o as unknown as Record<string, unknown>;
    for (const field of ["symbol", "vs"] as const) if (typeof v[field] === "string" && v[field] !== "") result.push({ path, field, ticker: v[field] as string });
  });
  return result;
}
const validName = (v: unknown): v is string => typeof v === "string" && v.trim().length > 0 && new TextEncoder().encode(v).length <= 80 && !/\p{Cc}/u.test(v);
export function foreignOperandProblems(o: Operand, scope: ForeignScope): string[] {
  if (!object(o)) return [];
  const out: string[] = [], active = hasFields(o);
  if (!isSeries(o)) return active ? ["Foreign fields belong on a series leaf, not on a formula or number."] : [];
  if (o.s === "bars_held") return active ? ["Bars held takes no symbol, vs or fixup fields."] : [];
  const pair = isPair(o.s);
  if ((active || pair) && !foreignEnabled(scope)) out.push("Foreign values are not advertised by this engine yet; saved fields are kept.");
  for (const field of ["symbol", "vs"] as const) {
    const value = o[field];
    if (value === undefined) continue;
    if (!validName(value)) { out.push(`${field}: use an instrument name of 1 to 80 bytes without control characters.`); continue; }
    const option = scope.options.find((x) => x.ticker === value);
    const market = option?.market === "crypto" ? "gl" : option?.market;
    if (market && market !== scope.market) out.push(`${field}: choose an instrument from the same market.`);
    else if (value !== scope.ticker && !foreignOptions(scope).some((x) => x.ticker === value)) out.push(`${field}: ${value} is not listed by this market's API; kept until corrected.`);
    else if (option?.disabled) out.push(`${field}: ${option.reason ?? "this instrument is unavailable"}.`);
  }
  if (Object.hasOwn(o, "fixup")) {
    if (o.fixup !== "none" && o.fixup !== "previous") out.push("fixup must be none or previous.");
    if (!validName(o.symbol)) out.push("fixup requires a symbol operand; remove fixup or choose Read from.");
  }
  if (pair) {
    if (!validName(o.vs)) out.push("Compared with: choose a second instrument for this pair indicator.");
    if (o.tf !== undefined && o.tf !== "D") out.push("Pair indicators use daily (D) values only.");
  } else if (Object.hasOwn(o, "vs")) out.push("Compared with (vs) applies to the four pair indicators only.");
  return out;
}
export function foreignProblems(spec: Spec, scope: ForeignScope): string[] {
  const out: string[] = []; let scoreSeries = 0;
  operands(spec, (o, path) => {
    for (const message of foreignOperandProblems(o, scope)) out.push(`${path}: ${message}`);
    if (path.startsWith("portfolio.score") && isSeries(o)) {
      if (o.s === "bars_held") out.push(`${path}: Bars held is not available in portfolio.score.`); else scoreSeries++;
      if (Object.hasOwn(o, "symbol")) out.push(`${path}: portfolio.score cannot read another symbol; vs is allowed.`);
    }
  });
  const names = [...new Set(foreignReferences(spec).map((r) => r.ticker))].sort();
  if (names.length > foreignLimit(scope)) out.push(`The spec reads ${names.length} instruments; the limit is ${foreignLimit(scope)} (${names.join(", ")}).`);
  if (object(spec.portfolio) && spec.portfolio.score !== undefined && !scoreSeries) out.push("portfolio.score needs at least one member series; constants and Bars held are not scores.");
  return out;
}

/** Per-use metadata is authoritative when a symbol has mixed policies or is a pair's comparison. */
export function foreignDescription(f:ForeignDisclosure):string {
 const coverage=`${f.first} to ${f.last}`;
 if(f.uses?.length)return f.uses.map(u=>`${u.source_scope==="portfolio_member"?"each portfolio member":u.source} → ${f.ticker} · ${u.path} · ${u.alignment==="prior_date"?"prior date":"same date"} · ${u.fixup?"previous available date":"gaps kept"} · ${coverage}`).join("; ");
 return `${f.ticker} · ${f.alignment==="prior_date"?"prior date":"same date"} · ${f.fixup?"previous available date":"gaps kept"} · ${coverage}`;
}
