import type {AuthorFile} from '../author-files';

export default {
  order: 930,
  author: { id: "sands", name: "Russell Sands", tagline: "One of the original Turtle traders · the Simple Turtle Breakout System runs here", books: [
    { id: "rsotr", sourceIds: ["title-d35ce795ae9f8c1c"], title: "The Original Turtle Trading Rules", year: null, blurb: "Sands · buy above the highest high of the past 20 bars (sell below the 20-bar low), exit on a 10-bar low (high), stop at 2% of the account, no profit target" }] },
  identities: ["Russell Sands"],
  profile: {
    "kind": "person",
    "mastery": [
      "Trend following"
    ],
    "summary": "Russell Sands was one of Richard Dennis's original Turtle traders. His booklet The Original Turtle Trading Rules states a Simple Turtle Breakout System: buy above the highest high of the past 20 bars, sell below the lowest low, exit on the opposite 10-bar extreme, with a stop at 2% of the account. This profile rests on the booklet itself.",
    "known_for": [
      "The Original Turtle Trading Rules"
    ],
    "sources": [],
    "confidence": "low"
  },
  sourceReview: {
    "rsotr": {"type":"book","people":["Russell Sands"]},
  },
} satisfies AuthorFile;
