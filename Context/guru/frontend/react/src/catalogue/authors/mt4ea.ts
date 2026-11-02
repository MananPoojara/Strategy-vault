import type {AuthorFile} from '../author-files';

export default {
  order: 1514,
  author: { id: 'mt4ea', name: "MetaTrader 4 expert advisors", tagline: "Forex EAs read from their MQL4 code, run on daily bars as declared applications", books: [{ id: 'mtasc', title: "ASC Trend EA (Forexmentor Metatrader Toolkit)", year: 2006, blurb: "ASCTrend arrows, always in · pip stops, targets and hour filter held" }] },
  identities: [],
  profile: {
    "kind": "collection",
    "mastery": ["Forex expert advisors", "Indicator systems"],
    "summary": "MetaTrader 4 expert advisors (MQL4 code) found in course and toolkit folders of the research share. The code is the rule; pip stops, targets and clock-time filters are held, and the EAs run on daily index, ETF and stock bars as declared applications.",
    "known_for": ["ASC Trend EA"],
    "sources": [],
    "confidence": "low"
  },
  sourceReview: {
    "mtasc": {"type":"code_package","people":[]},
  },
} satisfies AuthorFile;
