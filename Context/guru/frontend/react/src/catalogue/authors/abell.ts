import type {AuthorFile} from '../author-files';

export default {
  order: 1320,
  author: { id: "abell", name: "Howard Abell", tagline: "Floor trader and author · day-trading patterns and futures spreads (held)", books: [
    { id: "haddt", sourceIds: ["title-c34b63ac910c13fd"], title: "Digital Day Trading", year: 1999, blurb: "Abell, 1999 · Taylor swing days, inside/constricted-range breakouts, interviews; held: no exits, judgement" },
    { id: "haspr", sourceIds: ["title-327b1e8156c61ae5"], title: "Spread Trading", year: null, blurb: "Abell · futures spreads; held: needs futures prices (djvu copy not readable here)" }] },
  identities: ["Howard Abell"],
  sourceReview: {
    "haddt": {"type":"book","people":["Howard Abell"]},
    "haspr": {"type":"book","people":["Howard Abell"]},
  },
} satisfies AuthorFile;
