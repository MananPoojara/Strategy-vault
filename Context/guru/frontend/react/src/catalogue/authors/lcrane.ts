import type {AuthorFile} from '../author-files';

export default {
  order: 970,
  author: { id: "lcrane", name: "Les Crane", tagline: "Stock trader and author of WaveSurfer · the Impulse Ratio rule runs here", books: [
    { id: "lcws", sourceIds: [], title: "WaveSurfer", year: 2002, blurb: "Crane · the daily change of the 10-day average of closes (the Impulse Ratio): buy the next day once it closes at +0.010 or more after 10 or more negative days, sell when it falls below +0.010; the short mirror is an option" }] },
  identities: ["Les Crane"],
  profile: {
    "kind": "person",
    "mastery": [
      "Stock trading",
      "Moving-average momentum"
    ],
    "summary": "Les Crane wrote WaveSurfer (2002), a short e-book for stock traders built on one spreadsheet number, the Impulse Ratio: the day-to-day change of the 10-day average of closing prices. The book gives fixed buy and sell levels for it and a Ford Motor example. This profile rests on the book itself.",
    "known_for": [
      "WaveSurfer"
    ],
    "sources": [],
    "confidence": "low"
  },
  sourceReview: {
    "lcws": {"type":"book","people":["Les Crane"]},
  },
} satisfies AuthorFile;
