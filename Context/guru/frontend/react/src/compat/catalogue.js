// Curated application catalogue; source data and book PDFs remain external.
// Each author's card lives in src/catalogue/authors/<id>.ts; LIB keeps their `order`.
import {AUTHOR_FILES} from '../catalogue/author-files';
export const LIB = AUTHOR_FILES.map(f => f.author);
