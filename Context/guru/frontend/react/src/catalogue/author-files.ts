// One file per author: src/catalogue/authors/<id>.ts. A new author is one new file there; nothing else is edited.
// `order` sets the catalogue order (LIB); equal numbers sort by id. The first 158 were numbered 10, 20 ... on 31 Oct 2026.
export type SourceReviewRow = {type: string; sameWorkAs?: string; people: string[]};
export type CatalogueBook = {id: string; title: string; [key: string]: unknown};
export type AuthorFile = {
  order: number;
  /** The LIB entry (catalogue card): id, name, books, optional tagline/photo/aliases. */
  author: {id: string; name: string; books: CatalogueBook[]; [key: string]: unknown};
  /** Explicit people credited (author-identities). Leave out when the credit is unresolved. */
  identities?: readonly string[];
  /** Author page profile (author-profiles), cited public sources. */
  profile?: {kind: string; mastery: string[]; summary: string; [key: string]: unknown};
  /** Source review row for each of this author's books, keyed by book id. */
  sourceReview?: Record<string, SourceReviewRow>;
};

const modules = import.meta.glob<{default: AuthorFile}>('./authors/*.ts', {eager: true});
export const AUTHOR_FILES: AuthorFile[] = Object.values(modules).map(m => m.default)
  .sort((a, b) => a.order - b.order || (a.author.id < b.author.id ? -1 : a.author.id > b.author.id ? 1 : 0));
