import type {AuthorFile} from '../author-files';

export default {
  order: 500,
  author: { id: 'timers', name: "Zweig, Ned Davis, Schannep & Ryan Jones", tagline: "Breadth and Dow-theory market timing", books: [{ id: 'mzwws', title: "Winning on Wall Street (Zweig)", year: 1994, blurb: "Zweig, revised 1994 · breadth thrusts match his dated signals 14 of 14; monetary models need rate data" }, { id: 'ndbrm', title: "Being Right or Making Money (Ned Davis)", year: 2000, blurb: "NDR, 2nd ed. 2000 · % reversal model reproduces; sentiment and rate models need data" }, { id: 'jsdow', title: "Dow Theory for the 21st Century (Schannep)", year: 2008, blurb: "Schannep, 2008 · every rule needs the Dow Transports" }, { id: 'rjspr', title: "S&P Reloaded (Ryan Jones)", year: 2005, blurb: "Jones, 2005 · intraday E-mini system, needs minute bars" }, { id: 'rjmil', title: "$1 Million workbook, Advanced (Ryan Jones)", year: null, blurb: "Jones · MA-MA Divergence, FutureVideo and Big Trend from his SuperCharts code" }, { id: 'rjtg', title: "The Trading Game (Ryan Jones)", year: 1999, blurb: "Jones, 1999 · the simple method’s stop size is never given, so it is not testable" }, { id: 'ndtci', title: "The Triumph of Contrarian Investing (Ned Davis)", year: 2003, blurb: "Davis, 2003 · the sentiment rule needs Investors Intelligence data" }, { id: 'phbt', title: "Big Trends in Trading (Price Headley)", year: 2002, blurb: "Headley, 2002 · VIX Bollinger, VIX>45, acceleration bands; VIX>45 signal weeks match" }] },
  identities: ["Ned Davis","Ryan Jones"],
  profile: {
    "kind": "collection",
    "mastery": [
      "Market timing",
      "Breadth",
      "Position sizing & risk"
    ],
    "summary": "Market-timing and money-management authors. Martin Zweig was an adviser who combined growth-stock selection with timing models. Ned Davis co-founded the data-driven Ned Davis Research Group. Jack Schannep extends Dow Theory with timing indicators, Ryan Jones developed Fixed Ratio position sizing, and Price Headley founded BigTrends.",
    "known_for": [
      "Winning on Wall Street (Zweig)",
      "Being Right or Making Money (Ned Davis)",
      "Dow Theory for the 21st Century (Schannep)",
      "Fixed Ratio Trading (Ryan Jones)"
    ],
    "sources": [
      "https://en.wikipedia.org/wiki/Martin_Zweig",
      "https://en.wikipedia.org/wiki/Ned_Davis_(analyst)",
      "https://www.wiley.com/en-gb/The+Trading+Game%3A+Playing+by+the+Numbers+to+Make+Millions-p-9780471316985"
    ],
    "confidence": "high"
  },
  sourceReview: {
    "mzwws": {"type":"book","people":["Martin Zweig"]},
    "ndbrm": {"type":"book","people":["Ned Davis"]},
    "jsdow": {"type":"book","people":["Jack Schannep"]},
    "rjspr": {"type":"code_package","people":["Ryan Jones"]},
    "rjmil": {"type":"course_manual","people":["Ryan Jones"]},
    "rjtg": {"type":"book","people":["Ryan Jones"]},
    "ndtci": {"type":"book","people":["Ned Davis"]},
    "phbt": {"type":"book","people":["Price Headley"]},
  },
} satisfies AuthorFile;
