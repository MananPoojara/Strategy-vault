import type {AuthorFile} from '../author-files';

export default {
  order: 980,
  author: { id: "burke", name: "Edward Burke", tagline: "CNBC Million Dollar Portfolio Challenge winner · the 3-6-18 EMA continuation system runs here", books: [
    { id: "ebmst", sourceIds: [], title: "How I Won: Making Money in Stock Trends", year: 2009, blurb: "Burke · buy when the 3-day EMA crosses above the 6-day while the 6 is above the 18, sell when the 3 crosses back below the 6; short is the mirror" }] },
  identities: ["Edward Burke"],
  profile: {
    "kind": "person",
    "mastery": [
      "Equity trend following",
      "Moving averages"
    ],
    "summary": "Ed Burke (Big Wednesday Capital Management) won the 2008 CNBC Million Dollar Portfolio Challenge and wrote How I Won: Making Money in Stock Trends (2009), which sets out a 3-6-18 exponential moving average trend system for stocks with long and short rules; later tutorials add trade management. This profile rests on the e-book itself.",
    "known_for": [
      "How I Won: Making Money in Stock Trends",
      "2008 CNBC Million Dollar Portfolio Challenge"
    ],
    "sources": [],
    "confidence": "low"
  },
  sourceReview: {
    "ebmst": {"type":"book","people":["Edward Burke"]},
  },
} satisfies AuthorFile;
