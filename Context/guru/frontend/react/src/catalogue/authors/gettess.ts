import type {AuthorFile} from '../author-files';

export default {
  order: 480,
  author: { id: 'gettess', name: "Lee Gettess & Peter Aan", tagline: "S&P reversal and weekly-breakout systems", books: [{ id: 'lg3k', title: "The $3000 Secret (Gettess)", year: 1991, blurb: "Gettess, 1991 · four S&P reversal and day systems; the bond system needs data" }, { id: 'lgvp', readerPaths: ["Trading Knowledge I/Lee E.Gettess - Volpat Trading System (leegettess.com)/volpat.pdf"], title: "Volpat (Gettess)", year: null, blurb: "Gettess · volatility-pattern S&P system; the T-Bond version needs data" }, { id: 'lgtt30', title: "TT30 with Market Mapping (Gettess)", year: 1994, blurb: "Gettess, 1994 · TT30 reproduces the book: 958 trades, 71.6% winners against 1,059 and 72%" }, { id: 'lgsc', title: "Trading in the Trenches / Scorpion S&P (Gettess)", year: null, blurb: "Gettess · day-trading material without complete rules" }, { id: 'aanwr', title: "The Weekly Rule System (Aan)", year: 1993, blurb: "Aan, 1993 · calendar-week breakout; 72 trades against the book’s 72 at 5 weeks" }, { id: 'aanto', title: "Trend One (Aan)", year: 1999, blurb: "Aan, 1999 · the only code is intraday, needs 15-minute bars" }, { id: "scaandc", sourceIds: [], readerPaths: ["Trading Knowledge I/Stocks & Commodities DVD 11.26 (1982 - 2008) (traders.com)/VOLUMES/V08/C09/DELAY.pdf"], title: "Delayed Channel Breakout System (Aan, S&C V.8:9)", year: 1990, blurb: "Aan · 8-day channel delayed 20 days, stop and reverse" }, { id: "scaankr", sourceIds: [], readerPaths: ["Trading Knowledge I/Stocks & Commodities DVD 11.26 (1982 - 2008) (traders.com)/VOLUMES/V07/C10/KEYREV.pdf"], title: "Key reversal days (Aan, S&C V.7:10)", year: 1989, blurb: "Aan · reversal day at a 30-day extreme, 10-day trailing stop" }, { id: "scaansma", sourceIds: [], readerPaths: ["Trading Knowledge I/Stocks & Commodities DVD 11.26 (1982 - 2008) (traders.com)/VOLUMES/V07/C06/SIMPLEM.pdf"], title: "Simple moving average crossover (Aan, S&C V.7:6)", year: 1989, blurb: "Aan · 3/10-day SMA crossover, stop and reverse at the next open" }, { id: "scaanrsi", sourceIds: [], readerPaths: ["Trading Knowledge I/Stocks & Commodities DVD 11.26 (1982 - 2008) (traders.com)/VOLUMES/V07/C08/RSI.pdf"], title: "Relative Strength Index (Aan, S&C V.7:8)", year: 1989, blurb: "Aan · three RSI(14) 70/30 methods, two with a 15-day channel trailing stop" }, { id: "scaancb", sourceIds: [], readerPaths: ["Trading Knowledge I/Stocks & Commodities DVD 11.26 (1982 - 2008) (traders.com)/VOLUMES/V07/C09/CHANNEL.pdf"], title: "Channel breakout (Aan, S&C V.7:9)", year: 1989, blurb: "Aan · 8-day channel stop and reverse; close-only option" }, { id: "scaanwk", sourceIds: [], readerPaths: ["Trading Knowledge I/Stocks & Commodities DVD 11.26 (1982 - 2008) (traders.com)/VOLUMES/V07/C12/WEEKLYH.pdf"], title: "Weekly high/low moving average (Aan, S&C V.7:12)", year: 1989, blurb: "Aan · stop and reverse at the 3-week average weekly high / low" }, { id: "scaanmad", sourceIds: [], readerPaths: ["Trading Knowledge I/Stocks & Commodities DVD 11.26 (1982 - 2008) (traders.com)/VOLUMES/V08/C05/SIMPVAR.pdf"], title: "A simple variation of the moving average (Aan, S&C V.8:5)", year: 1990, blurb: "Aan · 30-day SMA direction with a 1-tick range penetration entry; the dollar stop is held" }] },
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
    "scaandc": {"type":"article","people":["Peter Aan"]},
    "scaankr": {"type":"article","people":["Peter Aan"]},
    "scaansma": {"type":"article","people":["Peter Aan"]},
    "scaanrsi": {"type":"article","people":["Peter Aan"]},
    "scaancb": {"type":"article","people":["Peter Aan"]},
    "scaanwk": {"type":"article","people":["Peter Aan"]},
    "scaanmad": {"type":"article","people":["Peter Aan"]},
  },
} satisfies AuthorFile;
