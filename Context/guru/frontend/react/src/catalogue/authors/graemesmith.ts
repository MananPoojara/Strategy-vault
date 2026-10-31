import type {AuthorFile} from '../author-files';

export default {
  order: 920,
  author: { id: "graemesmith", name: "David Graeme-Smith", tagline: "UK share swing trader (shortswingtrading.com) · the six-rule Short Swing Trading strategy runs here", books: [
    { id: "dgsst", sourceIds: ["title-7f5a3a12b1c58816"], title: "Short Swing Trading v6.0", year: 2004, blurb: "Graeme-Smith, 2004 · with the 50-day average, an 8-day +DI/-DI cross, buy above the last significant (non-inside) day's high, stop 0.5% under its low, trailed under each new significant day" }] },
  identities: ["David Smith"],
  profile: {
    "kind": "person",
    "mastery": [
      "Share swing trading"
    ],
    "summary": "David Graeme-Smith (catalogued as David Smith) sold Short Swing Trading, a six-rule method for UK shares: trade with the 50-day average, wait for the 8-day +DI and -DI lines to cross, buy above the last significant day's high, and trail a stop 0.5% under each new significant day's low. This profile rests on the e-book itself.",
    "known_for": [
      "Short Swing Trading"
    ],
    "sources": [],
    "confidence": "low"
  },
  sourceReview: {
    "dgsst": {"type":"book","people":["David Smith"]},
  },
} satisfies AuthorFile;
