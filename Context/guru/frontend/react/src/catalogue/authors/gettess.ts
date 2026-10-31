import type {AuthorFile} from '../author-files';

export default {
  order: 480,
  author: { id: 'gettess', name: "Lee Gettess & Peter Aan", tagline: "S&P reversal and weekly-breakout systems", books: [{ id: 'lg3k', title: "The $3000 Secret (Gettess)", year: 1991, blurb: "Gettess, 1991 · four S&P reversal and day systems; the bond system needs data" }, { id: 'lgvp', title: "Volpat (Gettess)", year: null, blurb: "Gettess · volatility-pattern S&P system; the T-Bond version needs data" }, { id: 'lgtt30', title: "TT30 with Market Mapping (Gettess)", year: 1994, blurb: "Gettess, 1994 · TT30 reproduces the book: 958 trades, 71.6% winners against 1,059 and 72%" }, { id: 'lgsc', title: "Trading in the Trenches / Scorpion S&P (Gettess)", year: null, blurb: "Gettess · day-trading material without complete rules" }, { id: 'aanwr', title: "The Weekly Rule System (Aan)", year: 1993, blurb: "Aan, 1993 · calendar-week breakout; 72 trades against the book’s 72 at 5 weeks" }, { id: 'aanto', title: "Trend One (Aan)", year: 1999, blurb: "Aan, 1999 · the only code is intraday, needs 15-minute bars" }] },
  identities: ["Lee Gettess","Peter Aan"],
  profile: {
    "kind": "person",
    "mastery": [
      "Futures",
      "Day trading",
      "Short-term swing"
    ],
    "summary": "Lee Gettess is a futures trader and course author whose home-study products present short-term futures systems, including the Scorpion S&P method and the Gettess Turning Point Indicator. Peter Aan wrote for Stocks & Commodities and is known for the Weekly Rule, a weekly high/low channel breakout trend system.",
    "known_for": [
      "Trading in the Trenches / Scorpion (Gettess)",
      "The $3000 Secret (Gettess)",
      "The Weekly Rule (Aan)"
    ],
    "sources": [
      "https://tradewins.com/lee-gettess/trading-in-the-trenches/",
      "https://store.traders.com/-v07-c12-weeklyh-pdf.html"
    ],
    "confidence": "low"
  },
  sourceReview: {
    "lg3k": {"type":"book","people":["Lee E. Gettess"]},
    "lgvp": {"type":"code_package","people":["Lee E. Gettess"]},
    "lgtt30": {"type":"code_package","people":["Lee E. Gettess"]},
    "lgsc": {"type":"other","people":["Lee E. Gettess"]},
    "aanwr": {"type":"code_package","people":["Peter W. Aan"]},
    "aanto": {"type":"code_package","people":["Peter W. Aan"]},
  },
} satisfies AuthorFile;
