/** Gurus page filters. Every option comes from the catalogue, library progress and the library records;
 * a filter only narrows the list and never certifies a rule or a result. */
export type AuthorStatus = "backtested" | "rules" | "reading";
export interface AuthorFacts {
  id: string;
  status: AuthorStatus;
  type: "person" | "collection";
}
export const FACETS = ["author_status", "author_type"] as const;
export type FacetKey = (typeof FACETS)[number];
export type Counts = Record<string, number>;

export const STATUS_LABELS: Record<AuthorStatus, string> = {
  backtested: "Backtested",
  rules: "Rules found, not yet backtested",
  reading: "Reading only",
};
export const BOOK_STATUS_LABELS: Record<string, string> = {
  backtested: "Backtested",
  in_progress: "In progress",
  not_backtested: "Not Backtested",
};
export const TYPE_LABELS: Record<string, string> = { person: "Person", collection: "Collection" };
export const TOPIC_LABELS: Record<string, string> = {
  "rule-based": "Rule-based",
  psychology: "Psychology",
  "money-management": "Money management",
};
export const SCOPE_LABELS: Record<string, string> = {
  trading: "Trading library",
  technical: "Supporting technical material",
  pending: "Relevance review pending",
  all: "Full source inventory",
};

/** Same reading as the card: any measured backtest, else any catalogued strategy, else reading material only. */
export function authorStatus(counts: { strategies: number; backtested: number }): AuthorStatus {
  return counts.backtested > 0 ? "backtested" : counts.strategies > 0 ? "rules" : "reading";
}

const valueOf = (facts: AuthorFacts, key: FacetKey) => (key === "author_status" ? facts.status : facts.type);

export function passes(facts: AuthorFacts, state: Record<string, string>, skip?: FacetKey) {
  return FACETS.every((key) => key === skip || !state[key] || valueOf(facts, key) === state[key]);
}

/** The authors left by the selected facets, and per option how many there would be if that option were chosen
 * (each facet is counted with every other filter applied, as on the strategies page). */
export function facetAuthors(facts: readonly AuthorFacts[], state: Record<string, string>) {
  const counts = Object.fromEntries(FACETS.map((key) => [key, {} as Counts])) as Record<FacetKey, Counts>;
  for (const item of facts)
    for (const key of FACETS)
      if (passes(item, state, key)) counts[key][valueOf(item, key)] = (counts[key][valueOf(item, key)] || 0) + 1;
  return { ids: new Set(facts.filter((item) => passes(item, state)).map((item) => item.id)), counts };
}

/** Clear all, and the URL defaults of these keys (the runtime adds them to its hash state, so links and Back keep filters). */
export const CLEAR_ALL: Record<string, string> = {
  library_q: "",
  book_category: "",
  author_status: "",
  author_type: "",
  book_progress: "",
  library_scope: "trading",
};

/** The chips under the bar: one per narrowing filter, each with the patch that removes it. */
export function activeFilters(state: Record<string, string>, view: string) {
  const out: { key: string; label: string; clear: Record<string, string> }[] = [];
  const add = (key: string, name: string, labels: Record<string, string>, reset = "") => {
    const value = state[key] || reset;
    if (value !== reset) out.push({ key, label: `${name}: ${labels[value] || value}`, clear: { [key]: reset } });
  };
  if (view === "books") add("book_progress", "Status", BOOK_STATUS_LABELS);
  else {
    add("author_status", "Status", STATUS_LABELS);
    add("author_type", "Type", TYPE_LABELS);
  }
  add("book_category", "Topic", TOPIC_LABELS);
  add("library_scope", "Library", SCOPE_LABELS, "trading");
  if (state.library_q) out.push({ key: "library_q", label: `“${state.library_q}”`, clear: { library_q: "" } });
  return out;
}
