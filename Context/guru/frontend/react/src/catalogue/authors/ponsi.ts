import type {AuthorFile} from '../author-files';

export default {
  order: 800,
  author: { id: "ponsi", name: "Ed Ponsi", tagline: "Forex trader and educator (FXEducator) · the FX-Ed Trend Technique runs here; Fibonacci, pattern, pip and carry trades held", books: [
    { id: "epfpp", sourceIds: ["title-360d41abd111aa32", "title-10a678f7dce48921"], title: "Forex Patterns & Probabilities (and The Ed Ponsi Forex Playbook)", year: 2007, blurb: "Ponsi, 2007 · FX-Ed Trend Technique (10-day EMA pullback, half-ATR trailing stop) runs on daily bars; the Playbook repeats it with carry trades" }] },
  identities: ["Ed Ponsi"],
  profile: {
    "kind": "person",
    "mastery": [
      "Currency trading",
      "Trend following"
    ],
    "summary": "Ed Ponsi is a forex trader and educator (FXEducator). Forex Patterns & Probabilities sets out his trend and range setups for currency pairs; the FX-Ed Trend Technique buys pullbacks to the 10-day EMA in a 10/20/50/200 EMA uptrend with a half-ATR trailing stop. This profile rests on the books themselves.",
    "known_for": [
      "Forex Patterns & Probabilities",
      "The Ed Ponsi Forex Playbook"
    ],
    "sources": [],
    "confidence": "low"
  },
  sourceReview: {
    "epfpp": {"type":"book","people":["Ed Ponsi"]},
  },
} satisfies AuthorFile;
