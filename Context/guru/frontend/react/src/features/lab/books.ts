import { LIB } from "../../compat/catalogue.js";

type Guru = { name: string; books?: { id: string; title: string }[] };
const BOOKS = new Map<string, { title: string; author: string }>();
for (const g of LIB as Guru[]) for (const b of g.books ?? []) BOOKS.set(b.id, { title: b.title, author: g.name });

/** Book title from the app's own research catalogue; the id itself when the catalogue does not know it. */
export const bookTitle = (id: string) => BOOKS.get(id)?.title ?? id;
/** "Title — Author", or the id when unknown. */
export function bookLabel(id: string): string {
  const b = BOOKS.get(id);
  return b ? `${b.title} — ${b.author}` : id;
}

/** Engine requirement codes in plain words: [label, what to pick]. */
const REQUIREMENTS: Record<string, [string, string]> = {
  individual_stock: ["Stocks only", "a stock"],
  index_or_etf: ["Index or ETF", "an index or ETF"],
  declared_index_timing: ["Index timing (index or ETF)", "an index or ETF"],
  index: ["Index only", "an index"],
  etf: ["ETF only", "an ETF"],
};
export const requirementText = (code: string) => REQUIREMENTS[code]?.[0] ?? code.replace(/_/g, " ");

/**
 * Why a method cannot run on the selected instrument, or undefined when it may.
 * shortcut: untyped in/us rows are an index or an ETF (spec §3.1) and the client cannot tell which,
 * so on them only methods admitting neither are greyed; the engine stays the authority.
 */
export function notAdmittedReason(requirement: string, allowed: string[], type: string | undefined): string | undefined {
  const ok = type ? allowed.includes(type) : allowed.some((t) => t === "index" || t === "etf");
  if (ok) return undefined;
  const [label, pick] = REQUIREMENTS[requirement] ?? [requirementText(requirement), `an instrument of type ${allowed.join(" or ")}`];
  return `${label} — pick ${pick} to run this`;
}
