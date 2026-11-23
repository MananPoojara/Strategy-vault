import type {AuthorFile} from '../author-files';

export default {
  order: 2700,
  author: { id: "poulos", name: "Bill Poulos", tagline: "Profits Run · Market Mastery Protégé Program swing methods", books: [
    { id: "pfpd", sourceIds: [], readerPaths: ["Trading Courses,Seminars,Videos/Bill Poulos - ETF Profit Driver Course, $1947 (etfprofitdriver.com)/Module 2/data/presentation.xml"], title: "ETF Profit Driver", year: 2008, blurb: "Poulos · four long-only ETF swing methods (Breakaway, Momentum Maximizer, Trend Rebound, Trend Recovery) with 10% half scale-out and LL5/LL6 trailing stops; rules from the course slide text, the 10-position risk layer is not coded" },
    { id: "mmpoulos", sourceIds: [], readerPaths: ["Trading Courses,Seminars,Videos/Bill Poulos - Market Mastery Course (marketmastery.com)/Module 5/Method 4.pdf"], title: "Market Mastery Protégé Program (Method 4)", year: 2007, blurb: "Poulos · Method 4: a bullish or bearish stochastic divergence at a 40-day extreme is entered on a limit at the 20-day average, stopped beyond the extreme, half covered at the 50-day average and the rest on a 2-bar trail (stocks only; chart examples, no printed results)" }] },
  identities: ["Bill Poulos"],
  profile: {
    "kind": "person",
    "mastery": ["Swing trading", "Stock setups with stochastics and moving averages"],
    "summary": "Bill Poulos (Profits Run) teaches the Market Mastery Protégé Program: swing methods for liquid US stocks with numeric setup, limit-entry, stop and scale-out rules. This profile rests on the course transcripts (text); Method 4 is coded, Methods 1, 2 and 3 are held (too many unstated details).",
    "known_for": ["Market Mastery Protégé Program"],
    "sources": [],
    "confidence": "low"
  },
  sourceReview: {
    "mmpoulos": {"type":"course_manual","people":["Bill Poulos"]},
    "pfpd": {"type":"course_manual","people":["Bill Poulos"]},
  },
} satisfies AuthorFile;
