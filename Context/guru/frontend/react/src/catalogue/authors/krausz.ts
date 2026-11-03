import type {AuthorFile} from '../author-files';

export default {
  order: 880,
  author: { id: "krausz", name: "Robert Krausz", tagline: "Gann swing trader (New Market Wizards) · the Basic Gann Swing Plan runs here; the Professional Plan needs 10/20-minute T-bond bars", books: [
    { id: "krgann", sourceIds: ["title-d7c844f5d172b872"], title: "W.D. Gann Treasure Discovered: Simple Trading Plans for Stocks & Commodities", year: 1998, blurb: "Krausz · 2-bar Gann swing trend + 3-day HiLo Activator entry and trailing exit, 38% swing retracement exit (scanned)" },
    { id: "krftj", sourceIds: [], readerPaths: ["Trading Books/Robert Krausz - Fibonacci Traders Journals, No 1-14/FTJ03.pdf"], title: "Fibonacci Trader Journal 3: The Professional Gann Swing Plan", year: null, blurb: "Krausz \u00b7 daily plan with the weekly HiLo: trend, contra-trend and special entries, 38.2% and HiLo profit protection; one position (targets and pyramids not applied)" }] },
  identities: ["Robert Krausz"],
  profile: {
    "kind": "person",
    "mastery": [
      "Gann swing charts",
      "Multiple time frames",
      "T-bond futures"
    ],
    "summary": "Robert Krausz, interviewed in Jack Schwager's The New Market Wizards, wrote W.D. Gann Treasure Discovered (1998): Gann swing charts, the HiLo Activator and mechanical swing plans tested trade by trade on T-bond futures and five US stocks. This profile rests on the book itself.",
    "known_for": [
      "W.D. Gann Treasure Discovered",
      "Fibonacci Trader"
    ],
    "sources": [],
    "confidence": "low"
  },
  sourceReview: {
    "krgann": {"type":"book","people":["Robert Krausz"]},
    "krftj": {"type":"magazine_volume","people":["Robert Krausz"]},
  },
} satisfies AuthorFile;
