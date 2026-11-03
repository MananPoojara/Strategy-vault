import type {AuthorFile} from '../author-files';

export default {
  order: 390,
  author: { id: 'primo', name: 'Steven Primo', tagline: 'TradingMarkets swing-trading courses · R3, RSI 20, Raptor', books: [{ id: 'pstc', title: 'Professional Swing Trading College 2005', year: 2005, blurb: 'Primo, 2005 · R3 and RSI 20 match the course’s own TradeStation trade lists (e.g. 70/74, 94/96 entries)' }, { id: 'prap2', title: 'Raptor-II Trading System', year: 2006, blurb: 'Primo, 2006 · runs on NSE basket stocks' }, { id: 'prap1', title: 'Raptor-10 Momentum Methodology', year: 2005, blurb: 'Needs the S&P 500’s volatility next to each US stock' }, { id: 'pstc08', title: "Swing Trading College 2008", year: 2008, blurb: "Primo/Connors, 2008 · R4 and R5, RS Meteors, RSI 25 scale-in" }, { id: 'prpr', title: "PowerRatings course", year: 2006, blurb: "Primo/TradingMarkets, 2006 · every system needs the proprietary PowerRatings score" }] },
  identities: ["Steven Primo"],
  profile: {
    "kind": "person",
    "mastery": [
      "Short-term swing",
      "Mean reversion",
      "Momentum"
    ],
    "summary": "Steven Primo traded on the floor of the Pacific Stock Exchange for 16 years and later became Director of Education at TradingMarkets. He teaches short-term stock swing trading, including pullback strategies that use PowerRatings, and his own Raptor momentum methods.",
    "known_for": [
      "Professional Swing Trading College",
      "Raptor-10 Momentum Methodology",
      "PowerRatings pullback trading"
    ],
    "sources": [
      "https://tradingmarkets.com/recent/one-on-one_with_steven_primo-675278"
    ],
    "confidence": "medium"
  },
  sourceReview: {
    "pstc": {"type":"course_manual","people":["Steve Primo"]},
    "prap2": {"type":"course_manual","people":["Steve Primo"]},
    "prap1": {"type":"course_manual","people":["Steve Primo"]},
    "pstc08": {"type":"course_manual","people":["Steve Primo"]},
    "prpr": {"type":"course_manual","people":[]},
  },
} satisfies AuthorFile;
