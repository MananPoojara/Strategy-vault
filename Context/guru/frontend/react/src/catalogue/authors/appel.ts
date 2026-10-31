import type {AuthorFile} from '../author-files';

export default {
  order: 130,
  author: { id: 'appel', name: 'Gerald Appel', tagline: 'MACD inventor · market-timing models on moving averages, MACD and breadth', books: [{ id: 'apta', title: 'Technical Analysis: Power Tools for Active Investors', year: 2005, blurb: 'Appel, 2005 · 200-day Dow rule, breadth impulse, MACD timing, best six months · Nasdaq/NYSE ratio and yield models need data' }, { id: 'apmacd', title: 'Construction & Application of the MACD Indicator', year: 1996, blurb: 'Appel, 1996 TAG booklet · MACD buy/sell rules, including MACD of the advance-decline line' }, { id: 'apwms', title: 'Winning Market Systems: 83 Ways to Beat the Market', year: null, blurb: 'Appel · TRIN, up/down volume, Shadow Line, A-D zero crossings, Haurlan indices (NYSE and NSE breadth)' }, { id: 'apsmts', title: 'Stock Market Trading Systems (with Hitschler)', year: 1980, blurb: 'Appel & Hitschler, 1980 · MA penetration, MMACDTS, lead time, A-D ±2,000, 25-day channel' }, { id: 'apoi', title: 'Opportunity Investing', year: 2007, blurb: 'Appel, 2007 · presidential-cycle strategies 1 and 2' }] },
  identities: ["Gerald Appel"],
  profile: {
    "kind": "person",
    "mastery": [
      "Indicators",
      "Market timing",
      "Momentum"
    ],
    "summary": "Gerald Appel invented the MACD indicator in the late 1970s and founded the newsletter Systems and Forecasts in 1973. His approach is quantitative and systematic, validated by long historical studies; he managed client money for over 35 years through Signalert Asset Management.",
    "known_for": [
      "MACD",
      "Systems and Forecasts newsletter",
      "Technical Analysis: Power Tools for Active Investors"
    ],
    "sources": [
      "https://cmtassociation.org/presenter/gerald-appel/",
      "https://en.wikipedia.org/wiki/MACD"
    ],
    "confidence": "high"
  },
  sourceReview: {
    "apta": {"type":"book","people":["Gerald Appel"]},
    "apmacd": {"type":"course_manual","people":["Gerald Appel"]},
    "apwms": {"type":"book","people":["Gerald Appel"]},
    "apsmts": {"type":"book","people":["Gerald Appel","W. Frederick Hitschler"]},
    "apoi": {"type":"book","people":["Gerald Appel"]},
  },
} satisfies AuthorFile;
