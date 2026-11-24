import type {AuthorFile} from '../author-files';

export default {
  order: 2751,
  author: { id: "raftopoulos", name: "Spyros Raftopoulos", tagline: "Stocks & Commodities author · the zigzag trend indicator as a stop-and-reverse system", books: [
    { id: "scrzzt", sourceIds: [], readerPaths: ["Trading Knowledge I/Stocks & Commodities DVD 11.26 (1982 - 2008) (traders.com)/VOLUMES/V21/C11/233RAFT.pdf"], title: "The Zigzag Trend Indicator, S&C V.21:11", year: 2003, blurb: "Raftopoulos, 2003 · 10% zigzag trend on the close: long when it turns +1, short when it turns -1, computed without look-ahead; run on the NSE indices, readings declared, unverified" }] },
  identities: ["Spyros Raftopoulos"],
  profile: {
    "kind": "person",
    "mastery": [
      "Technical indicators"
    ],
    "summary": "Spyros Raftopoulos wrote for Technical Analysis of Stocks & Commodities; his 2003 article gives the MetaStock code of a non-revisable zigzag trend indicator. This profile rests on that article alone.",
    "known_for": [
      "Zigzag trend indicator"
    ],
    "sources": [],
    "confidence": "low"
  },
  sourceReview: {
    "scrzzt": {"type":"article","people":["Spyros Raftopoulos"]},
  },
} satisfies AuthorFile;
