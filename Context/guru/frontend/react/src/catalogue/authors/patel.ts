import type {AuthorFile} from '../author-files';

export default {
  order: 630,
  author: { id: "patel", name: "Alpesh B. Patel", tagline: "UK online-trading author and broadcaster · the 50-day moving-average example from his trading course; the rest is chart reading and interviews", books: [
    { id: "apitc", sourceIds: ["title-445e010c1da20c14"], title: "The Internet Trading Course", year: null, blurb: "Patel · long above the 50-day average, out below (the course's example of a mechanical system); indicator chapters give no settings" }] },
  identities: ["Alpesh B. Patel"],
  profile: {
    "kind": "person",
    "mastery": [
      "Online trading",
      "Trading psychology"
    ],
    "summary": "Alpesh B. Patel writes guides to online trading and investing and interviewed traders for The Mind of a Trader. His books in the library teach chart reading and trading plans; only the Internet Trading Course states a complete mechanical rule, given as an example. This profile rests on the books themselves.",
    "known_for": [
      "Trading Online",
      "The Mind of a Trader",
      "The Internet Trading Course"
    ],
    "sources": [],
    "confidence": "low"
  },
  sourceReview: {
    "apitc": {"type":"book","people":["Alpesh B. Patel"]},
  },
} satisfies AuthorFile;
