import type {AuthorFile} from '../author-files';

export default {
  order: 1010,
  author: { id: "meissner", name: "Gunter Meissner", tagline: "Finance professor, derivatives author · the refined MACD runs here", books: [
    { id: "gmrm", sourceIds: [], title: "A Refined MACD Indicator: Evidence against the Random Walk Hypothesis?", year: 2001, blurb: "Meissner, Alex and Nolte · enter at the close of the third day after a MACD / signal crossing if no new crossing came; close at a 3% (or 5%) gain or the next crossing; a minimum day-3 gap (MACDR2) as an option" }] },
  identities: ["Gunter Meissner"],
  profile: {
    "kind": "person",
    "mastery": [
      "Technical indicator research",
      "Derivatives"
    ],
    "summary": "Gunter Meissner, with Albin Alex and Kai Nolte, wrote A Refined MACD Indicator - Evidence against the Random Walk Hypothesis?, which tests two refinements of the MACD crossover (a three-day confirmation and profit targets, plus a minimum gap) on the NASDAQ-100 stocks over ten years. This profile rests on the article itself.",
    "known_for": [
      "A Refined MACD Indicator"
    ],
    "sources": [],
    "confidence": "low"
  },
  sourceReview: {
    "gmrm": {"type":"article","people":["Gunter Meissner"]},
  },
} satisfies AuthorFile;
