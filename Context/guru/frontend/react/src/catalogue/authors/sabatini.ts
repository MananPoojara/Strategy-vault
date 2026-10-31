import type {AuthorFile} from '../author-files';

export default {
  order: 870,
  author: { id: "sabatini", name: "Alberto Sabatini", tagline: "Italian trader and author · the Gann one-bar swing chart trend method runs here; outside-bar orders and Gann angles held", books: [
    { id: "astk3", sourceIds: ["title-d13dcf241ff9a9ab"], title: "Trading Toolkit Collection Vol.3: Gli Swing Chart di W.D. Gann", year: 2001, blurb: "Sabatini · buy above the last swing top, out below the last swing bottom (Italian, scanned)" }] },
  identities: ["Alberto Sabatini"],
  profile: {
    "kind": "person",
    "mastery": [
      "Gann swing charts",
      "Italian stocks"
    ],
    "summary": "Alberto Sabatini wrote the Trading Toolkit Collection; volume 3 explains W.D. Gann's one-bar swing charts, outside bars and Gann angles on Italian stocks. This profile rests on the book itself.",
    "known_for": [
      "Trading Toolkit Collection Vol.3"
    ],
    "sources": [],
    "confidence": "low"
  },
  sourceReview: {
    "astk3": {"type":"book","people":["Alberto Sabatini"]},
  },
} satisfies AuthorFile;
