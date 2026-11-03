import type {AuthorFile} from '../author-files';

export default {
  order: 1980,
  author: { id: "clindsay", name: "Charles L. Lindsay", tagline: "Trident trading strategy · the minor-swing Trident trade runs here", books: [
    { id: "lcttr", sourceIds: [], readerPaths: ["USB HDD 2/tRADING KNO-2/Charles L.Lindsay - Trident. A Trading Strategy/Lindsay, Charles - Trident A Trading Strategy.pdf"], title: "Trident: A Trading Strategy", year: null, blurb: "Lindsay · minor swings: enter 25% up from a qualified P3, target P2 + P3 - P1, breakeven at the critical price, trailing stop in the window; higher swing levels held" }] },
  identities: ["Charles L. Lindsay"],
  sourceReview: {
    "lcttr": {"type":"book","people":["Charles L. Lindsay"]},
  },
} satisfies AuthorFile;
