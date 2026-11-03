// Explicit people in curated credits and reviewed source bylines. Unresolved credits are excluded.
// Reviewed minimum, not an exhaustive census. Van K. Tharp and Van Tharp are the same person.
// Each author's list is `identities` in src/catalogue/authors/<id>.ts.
import {AUTHOR_FILES} from '../catalogue/author-files';
export const authorIdentities: Record<string, readonly string[]> = Object.fromEntries(
  AUTHOR_FILES.filter(f => f.identities).map(f => [f.author.id, f.identities!]));
export const identifiedAuthorCount = new Set(Object.values(authorIdentities).flat()).size;
