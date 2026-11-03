import type {AuthorFile} from '../author-files';

export default {
  order: 230,
  author: { id: 'hirsch', name: 'Yale & Jeffrey Hirsch', tagline: 'Stock Trader’s Almanac · calendar and seasonal rules', books: [
    { id: "hsta25", sourceIds: [], title: "Stock Trader's Almanac 2025", year: 2025, blurb: "Hirsch · the Thanksgiving to Santa Claus rally trade" },
    { id: "hcta12", sourceIds: [], title: "Commodity Trader's Almanac 2012", year: 2012, blurb: "Hirsch & Person · the January short S&P 500 trade: 2nd trading day, hold 12 days" },
{ id: 'hsta', title: 'Stock Trader’s Almanac (2005, 2008, 2010)', year: 2010, blurb: 'Best Six Months (with MACD), Santa Claus rally, Super-8 days, Thanksgiving, first trading day, election-year months' }, { id: 'hcta', title: 'Commodity Trader’s Almanac 2010', year: 2010, blurb: 'Hirsch & Person, 2010 · the four S&P 500 calendar trades run on indices, ETFs and stocks (declared application); commodity seasonals need futures data' }] },
  identities: ["Jeffrey Hirsch","Yale Hirsch"],
  profile: {
    "kind": "person",
    "mastery": [
      "Seasonality",
      "Market timing",
      "Cycles"
    ],
    "summary": "Yale Hirsch created the Stock Trader's Almanac, published every year since 1968, a compilation of seasonal market tendencies laid out as a calendar. His son Jeffrey Hirsch took over as editor-in-chief in 2001. The Almanac popularised the January Barometer, Santa Claus Rally, Best Six Months and Presidential Election Cycle.",
    "known_for": [
      "Stock Trader's Almanac",
      "January Barometer",
      "Santa Claus Rally",
      "Best Six Months strategy"
    ],
    "sources": [
      "https://www.stocktradersalmanac.com/aboutus.aspx",
      "https://cmtassociation.org/presenter/yale-hirsch/",
      "https://cmtassociation.org/presenter/jeffrey-hirsch/"
    ],
    "confidence": "high"
  },
  sourceReview: {
    "hsta25": {"type":"book","people":["Jeffrey A. Hirsch"]},
    "hcta12": {"type":"book","people":["Jeffrey A. Hirsch","John L. Person"]},
    "hsta": {"type":"book","people":["Jeffrey A. Hirsch","Yale Hirsch"]},
    "hcta": {"type":"book","people":["Jeffrey A. Hirsch","John L. Person"]},
  },
} satisfies AuthorFile;
