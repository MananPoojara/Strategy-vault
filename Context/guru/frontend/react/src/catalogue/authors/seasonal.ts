import type {AuthorFile} from '../author-files';

export default {
  order: 460,
  author: { id: 'seasonal', name: "Jay Kaeppel & Marvin Appel", tagline: "Calendar models and fund-switching timing", books: [{ id: 'kpss', title: "Seasonal Stock Market Trends (Kaeppel)", year: 2009, blurb: "Kaeppel, 2009 · 56 calendar models; 404 of 453 printed Dow rows reproduce" }, { id: "sckjb", sourceIds: [], readerPaths: ["Trading Knowledge I/Stocks & Commodities DVD 11.26 (1982 - 2008) (traders.com)/VOLUMES/V08/C07/JANBARO.pdf"], title: "The January Barometer: Myth And Reality (Kaeppel, S&C V.8:7)", year: 1990, blurb: "Kaeppel · January pattern 4 (first five days up, January higher still) -> long February-December; patterns 3 and 5 as options" }, { id: 'maetf', title: "Investing with ETFs Made Easy (M. Appel)", year: 2007, blurb: "M. Appel, 2007 · every timing rule needs bond, style-index or fund data" }, { id: 'mahrs', title: "Higher Returns from Safe Investments (M. Appel)", year: 2010, blurb: "M. Appel, 2010 · bond-fund and yield models need data" }, { id: 'jkofc', title: "Optionetics Futures & Commodity Course (Kaeppel et al.)", year: 2007, blurb: "Kaeppel, Lofton & Zigler, 2007 · 4-day average stop-and-reverse" }, { id: "sckdjroc", sourceIds: [], readerPaths: ["Trading Knowledge I/Stocks & Commodities DVD 11.26 (1982 - 2008) (traders.com)/VOLUMES/V13/C05/ADJIA.PDF"], title: "A DJIA Long-Term Momentum Indicator (Kaeppel, S&C V.13:5)", year: 1995, blurb: "Kaeppel · DJROC weighted 11-month ROC turns up from -1, monthly" }, { id: 'sckcal', sourceIds: [], readerPaths: ["Trading Knowledge I/Stocks & Commodities DVD 11.26 (1982 - 2008) (traders.com)/VOLUMES/V20/C12/164stoc.pdf"], title: "The Stock Market, The Calendar, And You (Kaeppel, S&C V.20:12)", year: 2002, blurb: "Kaeppel \u00b7 seasonal index: long when two or more of four calendar tendencies are bullish; US only" }] },
  identities: ["Jay Kaeppel","Marvin Appel"],
  profile: {
    "kind": "person",
    "mastery": [
      "Seasonality",
      "Market timing",
      "Options"
    ],
    "summary": "Jay Kaeppel is a technical analyst, former head trader at a CTA and former Optionetics strategist; his Seasonal Stock Market Trends tests calendar-based stock-market patterns. Dr. Marvin Appel, CEO of Appel Asset Management and editor of Systems and Forecasts, writes on ETF selection and active asset allocation.",
    "known_for": [
      "Seasonal Stock Market Trends (Kaeppel)",
      "Investing with ETFs Made Easy (Appel)",
      "The Four Biggest Mistakes in Option Trading (Kaeppel)"
    ],
    "sources": [
      "https://cmtassociation.org/presenter/jay-kaeppel/",
      "https://www.wiley.com/en-us/Seasonal+Stock+Market+Trends:+The+Definitive+Guide+to+Calendar+Based+Stock+Market+Trading-p-9780470270431",
      "https://www.oreilly.com/library/view/investing-with-exchange-traded/9780137003716/pr04.html"
    ],
    "confidence": "high"
  },
  sourceReview: {
    "sckjb": {"type":"article","people":["Jay Kaeppel"]},
    "kpss": {"type":"book","people":["Jay Kaeppel"]},
    "maetf": {"type":"book","people":["Marvin Appel"]},
    "mahrs": {"type":"book","people":["Marvin Appel"]},
    "jkofc": {"type":"course_manual","people":["Brad Zigler","Jay Kaeppel","Robin Lofton"]},
    "sckdjroc": {"type":"article","people":["Jay Kaeppel"]},
    "sckcal": {"type":"article","people":["Jay Kaeppel"]},
  },
} satisfies AuthorFile;
