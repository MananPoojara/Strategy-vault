import type {AuthorFile} from '../author-files';

export default {
  order: 1100,
  author: { id: "csatchwell", name: "Chris Satchwell", tagline: "Technical analyst and system designer · the Trend2Noise filtered-line system runs here", books: [
    { id: "cspr", sourceIds: [], readerPaths: ["Trading Books/Chris Satchwell - Pattern Recognition & Trading Decisions/Chris Satchwell - Pattern Recognition & Trading Decisions.pdf"], title: "Pattern Recognition and Trading Decisions", year: 2005, blurb: "Satchwell · weekly: trade only when the 21-week Trend2Noise (trend slope against the noise around it) is beyond plus or minus 0.1, enter on the side of the quadratic filtered line's stop lines and trail that stop line until it is hit" }] },
  identities: ["Chris Satchwell"],
  profile: {
    "kind": "person",
    "mastery": [
      "Technical indicators",
      "Trading system design"
    ],
    "summary": "Chris Satchwell wrote Pattern Recognition and Trading Decisions (McGraw-Hill, 2005), which covers market behaviour, chart patterns, indicators with their formulas, exits and stops, and closes with a tested weekly trading system built from his Trend2Noise indicator and quadratic filtered-line stops. This profile rests on the book itself.",
    "known_for": [
      "Pattern Recognition and Trading Decisions",
      "Trend2Noise indicator"
    ],
    "sources": [],
    "confidence": "low"
  },
  sourceReview: {
    "cspr": {"type":"book","people":["Chris Satchwell"]},
  },
} satisfies AuthorFile;
