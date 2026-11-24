import type {AuthorFile} from '../author-files';

export default {
  order: 2753,
  author: { id: "capstone", name: "David Bean (CapStone Portfolio)", tagline: "TradeStation systems from the CapStone portfolio · the daily Situational System", books: [
    { id: "cpsit", sourceIds: [], readerPaths: ["USB HDD 1/TradeStation Systems & Indicators/_David Bean - CapStone Portfolio (All Systems) (sp-trader.com)"], title: "CapStone Portfolio: Situational System (EasyLanguage page)", year: 2005, blurb: "Printed EasyLanguage for a daily S&P system, run on the NSE indices with four declared readings, no printed results, unverified" }] },
  identities: ["David Bean"],
  profile: {
    "kind": "person",
    "mastery": ["Trading systems"],
    "summary": "David Bean's CapStone Portfolio collects TradeStation systems; this profile rests on the Situational System page alone.",
    "known_for": ["CapStone Portfolio TradeStation systems"],
    "sources": [],
    "confidence": "low"
  },
  sourceReview: {
    "cpsit": {"type":"system_listing","people":["David Bean"]},
  },
} satisfies AuthorFile;
