import type {AuthorFile} from '../author-files';

export default {
  order: 680,
  author: { id: "maclean", name: "George A. MacLean", tagline: "Fixed-income technical analyst · Fibonacci and Gann tools drawn by the analyst, nothing testable", books: [
    { id: "gmfg", sourceIds: ["title-b7c5b1cbfeb2e923", "title-938bbaea33f3d1ba"], title: "Fibonacci and Gann Applications in Financial Markets", year: 2005, blurb: "MacLean, 2005 · retracements, fanlines, timelines and Gann swing charts from chosen pivots; held" }] },
  identities: ["George A. MacLean"],
  profile: {
    "kind": "person",
    "mastery": [
      "Fibonacci analysis",
      "Gann analysis",
      "Fixed income"
    ],
    "summary": "George A. MacLean is a technical analyst of fixed-income and futures markets. His book applies Fibonacci retracements, fanlines, timelines and Gann angles and swing charts; every level starts from a high or low the analyst chooses, so the methods cannot be run mechanically. This profile rests on the book itself.",
    "known_for": [
      "Fibonacci and Gann Applications in Financial Markets"
    ],
    "sources": [],
    "confidence": "low"
  },
  sourceReview: {
    "gmfg": {"type":"book","people":["George A. MacLean"]},
  },
} satisfies AuthorFile;
