import type {AuthorFile} from '../author-files';

export default {
  order: 2741,
  author: { id: "busby", name: "Thomas Busby", tagline: "DTI · day trading around key numbers and time of day · the yearly-open index rule runs here on NSE indices", books: [
    { id: "bbyo", sourceIds: [], readerPaths: ["Trading Books/Thomas Busby -  Winning the DayTrading Game/Thomas Busby -  Winning the DayTrading Game.pdf"], title: "Winning the DayTrading Game", year: 2005, blurb: "Busby, 2005 · ch.9 index rule: long or short at 2.5% above or below the quarter open, out at 2.5% against it, no re-entry until the next quarter; run on daily NSE indices as a declared adaptation, unverified. The bond checklist, fund rotation and key-number day trading are held (bond ticks, fund returns, judged levels)" },
    { id: "bbmns", sourceIds: [], readerPaths: ["Trading Books/Thomas Busby - The Markets Never Sleep/Thomas Busby - The Market Never Sleeps.pdf"], title: "The Markets Never Sleep", year: 2007, blurb: "Busby, 2007 · first-30-minute stock breakout and gold reference-bar stops; held: no stop distance, sector and Dax agreement judged" },
    { id: "bbttw", sourceIds: [], readerPaths: ["Trading Books/Thomas Busby - Trade to Win/Thomas Busby - Trade to Win.pdf"], title: "Trade to Win", year: 2008, blurb: "Busby · the same key-number and yearly-open method as Winning the DayTrading Game; no extra complete rule" }] },
  identities: ["Thomas Busby"],
  profile: {
    "kind": "person",
    "mastery": [
      "Day trading",
      "Futures trading"
    ],
    "summary": "Thomas Busby founded DTI, a trading school in Mobile, Alabama. His books describe day trading the S&P 500 index futures around time of day, key numbers (yearly, monthly and daily opens) and market indicators; one printed index rule, a 2.5% deviation from the year's and quarter's open, is coded here.",
    "known_for": [
      "Winning the DayTrading Game",
      "The Markets Never Sleep",
      "Trade to Win"
    ],
    "sources": [],
    "confidence": "low"
  },
  sourceReview: {
    "bbyo": {"type":"book","people":["Thomas Busby"]},
    "bbmns": {"type":"book","people":["Thomas Busby"]},
    "bbttw": {"type":"book","people":["Thomas Busby"]},
  },
} satisfies AuthorFile;
