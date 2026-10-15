export interface SearchAuthor {
  id: string;
  name: string;
  photo: string;
  initials: string;
  href: string;
  curated?: boolean;
}

const normalize = (value: string) => value.normalize("NFKD")
  .replace(/\p{M}/gu, "").toLocaleLowerCase("en").replace(/\s+/g, " ").trim();

/** Local name matching; suggestions never request backtests or certify an author. */
export function matchingSearchAuthors(authors: SearchAuthor[], query: string): SearchAuthor[] {
  const term = normalize(query);
  if (!term) return [];
  return authors.filter(author => normalize(author.name).includes(term))
    .sort((a, b) => Number(!!b.curated) - Number(!!a.curated)
      || Number(normalize(b.name).startsWith(term)) - Number(normalize(a.name).startsWith(term))
      || a.name.localeCompare(b.name, "en", {sensitivity: "base"}))
    .slice(0, 6);
}
