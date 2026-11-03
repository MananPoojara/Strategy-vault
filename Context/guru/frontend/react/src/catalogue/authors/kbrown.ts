import type {AuthorFile} from '../author-files';

export default {
  order: 1250,
  author: { id: "kbrown", name: "Kedrick F. Brown", tagline: "Equity market maker and trend trader · the 32-day close Donchian and the all-time-high breakout run here", books: [
    { id: "kbttm", sourceIds: ["title-3a876c2b43f2c9cc"], title: "Trend Trading: Timing Market Tides", year: 2006, blurb: "Brown · buy when the close beats the highest close of the past 32 days and sell below the lowest, on the next close; buy a new all-time high with a 96% stop and a 32-day-low exit; NASDAQ pilot signals and basket averages held" }] },
  identities: ["Kedrick Brown"],
  profile: {
    "kind": "person",
    "mastery": [
      "Equity trend following"
    ],
    "summary": "Kedrick F. Brown is a former equity market maker. Trend Trading: Timing Market Tides (Wiley, 2006) adapts futures trend-following discipline to stocks: Donchian, stochastic and peak breakouts, index 'pilot' signals, stop placement and position sizing, with a ten-year test of a 32-day close Donchian rule on three US indices. This profile rests on the book itself.",
    "known_for": [
      "Trend Trading: Timing Market Tides"
    ],
    "sources": [],
    "confidence": "low"
  },
  sourceReview: {
    "kbttm": {"type":"book","people":["Kedrick Brown"]},
  },
} satisfies AuthorFile;
