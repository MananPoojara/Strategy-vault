import type {AuthorFile} from '../author-files';

export default {
  order: 170,
  author: { id: 'katz', name: 'Katz & McCormick', tagline: 'Systematic tests of entry and exit models with one standard exit', books: [{ id: 'kets', title: 'The Encyclopedia of Trading Strategies', year: 2000, blurb: 'Katz & McCormick, 2000 · breakouts, oscillators, seasonality, lunar and cycle entries with the book’s ATR stop/target exit' }, { id: "sckzlun", sourceIds: [], readerPaths: ["Trading Knowledge I/Stocks & Commodities DVD 11.26 (1982 - 2008) (traders.com)/VOLUMES/V15/C06/LUNARCY.pdf"], title: "Lunar Cycles And Trading (Katz, S&C V.15:6)", year: 1997, blurb: "Katz & McCormick · buy 1 bar after a full moon, short 1 bar after a new moon" }] },
  identities: [],
  profile: {
    "kind": "person",
    "mastery": [
      "System design & testing",
      "Quantitative research"
    ],
    "summary": "Jeffrey Owen Katz, PhD, and Donna McCormick wrote The Encyclopedia of Trading Strategies (2000), which backtests many entry types on the same futures markets: breakouts, moving averages, oscillators, seasonality, lunar and solar rhythms, cycles, neural networks and genetic algorithms. It also studies standard and improved exits.",
    "known_for": [
      "The Encyclopedia of Trading Strategies"
    ],
    "sources": [
      "https://books.google.com/books/about/The_Encyclopedia_of_Trading_Strategies.html?id=sx4NXPe4_qUC",
      "http://loc.gov/catdir/description/mh023/99045943.html",
      "http://www.scientific-consultants.com/books.html"
    ],
    "confidence": "high"
  },
  sourceReview: {
    "kets": {"type":"book","people":["Donna L. McCormick","Jeffrey Owen Katz"]},
    "sckzlun": {"type":"article","people":["Jeffrey Owen Katz", "Donna L. McCormick"]},
  },
} satisfies AuthorFile;
