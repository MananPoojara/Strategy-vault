import type {AuthorFile} from '../author-files';

export default {
  order: 2750,
  author: { id: "hartle", name: "Thom Hartle", tagline: "Stocks & Commodities technical editor · the Wilder DI cross, run as stop and reverse", books: [
    { id: "schdadx", sourceIds: [], readerPaths: ["Trading Knowledge I/Stocks & Commodities DVD 11.26 (1982 - 2008) (traders.com)/VOLUMES/V09/C03/ADX.pdf"], title: "The Average Directional Movement Index (ADX), S&C V.9:3", year: 1991, blurb: "Hartle, 1991 · long when +DI14 crosses above -DI14, short on the reverse cross; run on the NSE indices as a declared application, unverified" }] },
  identities: ["Thom Hartle"],
  profile: {
    "kind": "person",
    "mastery": [
      "Technical analysis writing"
    ],
    "summary": "Thom Hartle wrote for Technical Analysis of Stocks & Commodities; his 1991 ADX article states Wilder's +DI/-DI crossover as a trading rule. This profile rests on that article alone.",
    "known_for": [
      "S&C articles on directional movement"
    ],
    "sources": [],
    "confidence": "low"
  },
  sourceReview: {
    "schdadx": {"type":"article","people":["Thom Hartle"]},
  },
} satisfies AuthorFile;
