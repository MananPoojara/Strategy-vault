import type {AuthorFile} from '../author-files';

export default {
  order: 670,
  author: { id: "langham", name: "James Mars Langham", tagline: "1930s planetary market forecaster · astrology books held: every rule needs an ephemeris and the author's reading", books: [
    { id: "jlpe", sourceIds: ["title-d697882d4cbe426d", "title-70b33cca5d0bb8b8"], title: "Planetary Effects on Stock Market Prices", year: null, blurb: "Langham · major-planet aspects and long trends; needs an ephemeris, held" },
    { id: "jlmf", sourceIds: ["title-aa63a15e57901fbe"], title: "Cyclical Market Forecasting: Stocks and Grain", year: null, blurb: "Langham, c.1938 · planet combinations, lunations, sun-spots and grain; needs an ephemeris, held" }] },
  identities: ["James Mars Langham"],
  profile: {
    "kind": "person",
    "mastery": [
      "Astrology",
      "Market forecasting"
    ],
    "summary": "James Mars Langham wrote planetary forecasting books in the 1930s that relate stock and grain trends to planetary aspects, eclipses, lunations and sun-spots. The rules need daily planetary positions and the author's interpretation, so nothing can be backtested on price data alone. This profile rests on the books themselves.",
    "known_for": [
      "Planetary Effects on Stock Market Prices",
      "Cyclical Market Forecasting: Stocks and Grain"
    ],
    "sources": [],
    "confidence": "low"
  },
  sourceReview: {
    "jlpe": {"type":"book","people":["James Mars Langham"]},
    "jlmf": {"type":"book","people":["James Mars Langham"]},
  },
} satisfies AuthorFile;
