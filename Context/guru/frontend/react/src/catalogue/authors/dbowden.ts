import type {AuthorFile} from '../author-files';

export default {
  order: 1110,
  author: { id: "dbowden", name: "David Bowden", tagline: "Safety in the Market (Gann-based swing trading) · the ABC swing trade runs here in four exit styles", books: [
    { id: "dbnotp", sourceIds: [], readerPaths: ["Trading Books/David Bowden - The Number One Trading Plan (sitm.com.au)/David Bowden - The Number One Trading Plan.pdf"], title: "The Number One Trading Plan", year: null, blurb: "Bowden · on the daily swing chart, after a higher top B and a higher bottom C, buy one tick over C's high within 25-33% of the A-B range; stop under C, then entry at 50%, profit at 75% (Stock), 100% (Currency), a stop trailing each 25% milestone (Stock Index) or each new swing bottom" }] },
  identities: ["David Bowden"],
  profile: {
    "kind": "person",
    "mastery": [
      "Gann swing charts",
      "Swing trading"
    ],
    "summary": "David Bowden, of Safety in the Market (Australia), teaches Gann-based swing trading; The Number One Trading Plan sets out his ABC swing trade on stocks, stock index futures and currencies with three exit styles and a trailing swing stop, worked through MSFT and YHOO exercises. This profile rests on the course manual itself.",
    "known_for": [
      "The Number One Trading Plan",
      "Safety in the Market"
    ],
    "sources": [],
    "confidence": "low"
  },
  sourceReview: {
    "dbnotp": {"type":"book","people":["David Bowden"]},
  },
} satisfies AuthorFile;
