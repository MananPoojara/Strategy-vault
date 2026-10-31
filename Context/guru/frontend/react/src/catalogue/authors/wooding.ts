import type {AuthorFile} from '../author-files';

export default {
  order: 1070,
  author: { id: "wooding", name: "Dave Wooding", tagline: "Stock trader · the Stock Rocket breakout system runs here on US stocks", books: [
    { id: "dwsr", sourceIds: ["title-209cf424fa79e012"], title: "The Stock Rocket Stock Trading System", year: 2001, blurb: "Wooding · a 52-week-high day on ten times the 50-day volume (over 500,000 shares, close over $10, open low and close high in the widest range of two weeks); buy stop one tick over its high, stop one tick under its low, half sold at one times the risk, the rest trailed under higher lows. US stocks and ETFs only" }] },
  identities: ["Dave Wooding"],
  profile: {
    "kind": "person",
    "mastery": [
      "Stock breakout trading"
    ],
    "summary": "Dave Wooding (The Wooding Trading Company) wrote The Stock Rocket Stock Trading System (2001), a short manual for buying US stocks that break to new 52-week highs on very heavy volume, with exact entry, stop, half-profit and trailing-stop rules and seven 1999-2000 case studies. This profile rests on the manual itself.",
    "known_for": [
      "The Stock Rocket Stock Trading System"
    ],
    "sources": [],
    "confidence": "low"
  },
  sourceReview: {
    "dwsr": {"type":"book","people":["Dave Wooding"]},
  },
} satisfies AuthorFile;
