import type {AuthorFile} from '../author-files';

export default {
  order: 60,
  author: { id: 'lwilliams', name: 'Larry Williams', tagline: 'Short-term futures patterns · volatility breakouts, Oops, trading-day-of-month and day-of-week studies', books: [{ id: 'lwlts', title: 'Long-Term Secrets to Short-Term Trading', year: 1999, blurb: 'Williams, 1999 · Talon swing breakout, volatility breakouts, Oops, first-day-of-month · bond and gold filters need data' }, { id: 'lwdg2', title: 'The Definitive Guide to Futures Trading, Vol. II', year: 1989, blurb: 'Williams, 1989 · the .0105 day-trade system and related S&P patterns' }, { id: 'lwicw', title: 'Inner Circle Workshop 2000', year: 2000, blurb: 'Williams · workshop rules on daily bars; COT rules need data' }, { id: 'lwmdc', title: 'Million Dollar Challenge seminar', year: 2006, blurb: 'Williams, 2006 · 21 rules from the seminar studies' }, { id: 'lwpro', title: 'Protégé Program workbook', year: 2003, blurb: 'Williams, 2003' }, { id: 'lwrsr', title: 'The Right Stock at the Right Time', year: 2003, blurb: 'Williams, 2003 · October-to-April Dow trade (9.08%/yr vs the book’s 9.05%)' }, { id: 'lwmds', title: 'The Million Dollar Stock Market Idea', year: null, blurb: 'No complete rule; COT needs data' }, { id: 'lwhtb', title: 'How to Trade Better (article)', year: 2000, blurb: 'No complete rule' }, { id: 'lwcot', title: 'Secrets of the COT Report', year: 2005, blurb: 'Williams, 2005 · S&P price/volume studies and the N-down-closes day trade; COT rules need data' }, { id: 'lwic6', title: 'Inner Circle Seminar 2006', year: 2006, blurb: 'Williams · 10-day %R with the 50-day trend, long side' }, { id: 'lwstc', title: 'Sure Thing Commodity Trading', year: 2006, blurb: 'Seminar manual · commodity rules need futures data' }, { id: 'lwppt', title: 'Picture Perfect Trading', year: 2007, blurb: 'Manual + seminar book · nothing complete on index data' }, { id: 'lwsss', title: 'The Secret of Selecting Stocks', year: 1972, blurb: 'Windsor Books, 1972 · stock-selection rules need US stock data' }, { id: 'lwmil', title: 'How I Made One Million Dollars Trading Commodities', year: 1973, blurb: 'Windsor Books · commodity rules need futures data' }, { id: 'lwdtfo', title: "Day Trade Futures Online", year: 2000, blurb: "Williams, 2000 · always-in S&P volatility breakouts from Tables 4.1–4.3; no stop, so intrabar order matters" }, { id: 'lwcmc', title: "Cracking the Money Code", year: 2006, blurb: "Williams, 2006 · Turtle 20/10 as he states it; chapter-12 patterns have no exit" }, { id: 'lwad', title: "Accumulation & Distribution (TAG 1995)", year: 1995, blurb: "Williams, 1995 · patterns give entries without exits" }, { id: 'lwnl', title: "Newsletters 1991–1997", year: null, blurb: "Williams · 59 seasonal, holiday and pattern rules; open-dependent rules don’t reproduce on the cash index" }, { id: 'lwfc10', sourceIds: ['source-16a7184cce7f55da', 'source-47462338397eef87', 'source-b58812d709478d70'], title: 'Forecast 2010', year: 2009, blurb: 'Williams, 2009 · three mechanical S&P trades: trading-days-left set-ups, best days of the year, mid-term election October trade' }, { id: 'lwinb', title: 'LW Inside Bar (Inner Circle TradeStation code)', year: null, blurb: 'Seminar code · inside day after an up day, rising 80-day regression slope' }] },
  identities: ["Larry Williams"],
  profile: {
    "kind": "person",
    "mastery": [
      "Short-term swing",
      "Futures",
      "Seasonality"
    ],
    "summary": "Larry Williams is an American stock and commodity trader and author. He won the 1987 Robbins World Cup Championship of Futures Trading and created indicators including Williams %R, the Ultimate Oscillator and COT indices. His books cover short-term futures patterns, seasonal tendencies and commitments-of-traders analysis.",
    "known_for": [
      "Williams %R",
      "Ultimate Oscillator",
      "Long-Term Secrets to Short-Term Trading",
      "COT analysis"
    ],
    "sources": [
      "https://en.wikipedia.org/wiki/Larry_R._Williams"
    ],
    "confidence": "high"
  },
  sourceReview: {
    "lwlts": {"type":"book","people":["Larry Williams"]},
    "lwdg2": {"type":"book","people":["Larry Williams"]},
    "lwicw": {"type":"course_manual","people":["Larry Williams"]},
    "lwmdc": {"type":"course_manual","people":["Larry Williams"]},
    "lwpro": {"type":"course_manual","sameWorkAs":"lwicw","people":["Larry Williams"]},
    "lwrsr": {"type":"book","people":["Larry Williams"]},
    "lwmds": {"type":"book","sameWorkAs":"lwsss","people":["Larry Williams"]},
    "lwhtb": {"type":"article","people":["Larry Williams"]},
    "lwcot": {"type":"book","people":["Larry Williams"]},
    "lwic6": {"type":"course_manual","people":["Larry Williams"]},
    "lwstc": {"type":"course_manual","people":["Larry Williams"]},
    "lwppt": {"type":"course_manual","people":["Larry Williams"]},
    "lwsss": {"type":"book","people":["Larry Williams"]},
    "lwmil": {"type":"book","people":["Larry Williams"]},
    "lwdtfo": {"type":"book","people":["Larry Williams"]},
    "lwcmc": {"type":"course_manual","people":["Larry Williams"]},
    "lwad": {"type":"course_manual","people":["Larry Williams"]},
    "lwnl": {"type":"magazine_volume","people":["Larry Williams"]},
    "lwfc10": {"type":"article","people":["Larry Williams"]},
    "lwinb": {"type":"code_package","people":["Larry Williams"]},
  },
} satisfies AuthorFile;
