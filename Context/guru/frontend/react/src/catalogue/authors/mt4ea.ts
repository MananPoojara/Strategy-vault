import type {AuthorFile} from '../author-files';

export default {
  order: 1514,
  author: { id: 'mt4ea', name: "MetaTrader 4 expert advisors", tagline: "Forex EAs read from their MQL4 code, run on daily bars as declared applications", books: [{ id: 'mtasc', title: "ASC Trend EA (Forexmentor Metatrader Toolkit)", year: 2006, blurb: "ASCTrend arrows, always in · pip stops, targets and hour filter held" }, { id: 'mtsvos', title: "SVOS EURJPY D1 (Belkhayate course CD)", year: 2009, blurb: "VHF-switched OsMA / Stochastic EA · pip exits and the 0.3 StdDev filter held" }, { id: 'mtbbm', title: "Bollinger Master (TamFx, 2008 MQL Championship)", year: 2008, blurb: "Band fade with band-width stop and target · point clamps held" }, { id: 'mtenl', title: "enLight DX II Trader (MT4, D1)", year: 2009, blurb: "Shifted-EMA day breakout stops with ATR target, trail and break-even · point offsets held" }] },
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
    "mtsvos": {"type":"code_package","people":["Brian Dee"]},
    "mtbbm": {"type":"code_package","people":[]},
    "mtenl": {"type":"code_package","people":[]},
  },
} satisfies AuthorFile;
