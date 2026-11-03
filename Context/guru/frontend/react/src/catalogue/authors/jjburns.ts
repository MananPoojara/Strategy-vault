import type {AuthorFile} from '../author-files';

export default {
  order: 1020,
  author: { id: "jjburns", name: "Jeremy J. Burns", tagline: "TradeCashSystem stock trader · the inside-day breakout runs here", books: [
    { id: "jbid", sourceIds: [], title: "The One Trading Secret That Could Make You Rich: Inside Days", year: 2009, blurb: "Burns · after an inside day, buy one cent over its high or sell one cent under its low the next day; stop at the other side; out at target 1 (the inside day's range) or the close of day 3" }] },
  identities: ["Jeremy J. Burns"],
  profile: {
    "kind": "person",
    "mastery": [
      "Short-term stock trading",
      "Price patterns"
    ],
    "summary": "Jeremy J. Burns wrote Inside Days (TradeCashSystem.com, 2009), a short guide to trading the breakout of an inside day in stocks with fixed stop, range-based targets and a three-to-four-day time exit. This profile rests on the guide itself.",
    "known_for": [
      "The One Trading Secret That Could Make You Rich: Inside Days"
    ],
    "sources": [],
    "confidence": "low"
  },
  sourceReview: {
    "jbid": {"type":"book","people":["Jeremy J. Burns"]},
  },
} satisfies AuthorFile;
