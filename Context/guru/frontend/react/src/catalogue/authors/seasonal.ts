import type {AuthorFile} from '../author-files';

export default {
  order: 460,
  author: { id: 'seasonal', name: "Jay Kaeppel & Marvin Appel", tagline: "Calendar models and fund-switching timing", books: [{ id: 'kpss', title: "Seasonal Stock Market Trends (Kaeppel)", year: 2009, blurb: "Kaeppel, 2009 · 56 calendar models; 404 of 453 printed Dow rows reproduce" }, { id: 'maetf', title: "Investing with ETFs Made Easy (M. Appel)", year: 2007, blurb: "M. Appel, 2007 · every timing rule needs bond, style-index or fund data" }, { id: 'mahrs', title: "Higher Returns from Safe Investments (M. Appel)", year: 2010, blurb: "M. Appel, 2010 · bond-fund and yield models need data" }, { id: 'jkofc', title: "Optionetics Futures & Commodity Course (Kaeppel et al.)", year: 2007, blurb: "Kaeppel, Lofton & Zigler, 2007 · 4-day average stop-and-reverse" }] },
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
    "kpss": {"type":"book","people":["Jay Kaeppel"]},
    "maetf": {"type":"book","people":["Marvin Appel"]},
    "mahrs": {"type":"book","people":["Marvin Appel"]},
    "jkofc": {"type":"course_manual","people":["Brad Zigler","Jay Kaeppel","Robin Lofton"]},
  },
} satisfies AuthorFile;
