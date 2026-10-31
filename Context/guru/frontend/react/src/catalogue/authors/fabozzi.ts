import type {AuthorFile} from '../author-files';

export default {
  order: 590,
  author: { id: "fabozzi", name: "Frank J. Fabozzi", tagline: "Finance professor and editor of investment handbooks · the commodity trend rules collected in his commodity handbook", books: [
    { id: "fzhci", sourceIds: ["title-3cfe53be49203ef5"], title: "The Handbook of Commodity Investing", year: 2008, blurb: "Fabozzi, Füss & Kaiser (eds.), 2008 · price/average cross, 12-month momentum, dual average, outside channel and filter rules; term-structure strategies need futures curves" }] },
  identities: ["Frank J. Fabozzi"],
  profile: {
    "kind": "person",
    "mastery": [
      "Fixed income",
      "Quantitative finance",
      "Commodities"
    ],
    "summary": "Frank J. Fabozzi is a finance academic and editor of many investment handbooks. The rules on this card come from chapters by contributors to The Handbook of Commodity Investing (Lhabitant, Park and Irwin, and the Erb-Harvey study they cite); his other books in the library are textbooks without trading rules. This profile rests on the books' title pages.",
    "known_for": [
      "Mathematics of Financial Modeling and Investment Management",
      "The Handbook of Commodity Investing",
      "Quantitative Equity Investing"
    ],
    "sources": [],
    "confidence": "low"
  },
  sourceReview: {
    "fzhci": {"type":"book","people":["Frank J. Fabozzi"]},
  },
} satisfies AuthorFile;
