import type {AuthorFile} from '../author-files';

export default {
  order: 420,
  author: { id: 'haggerty', name: 'Kevin Haggerty & Cynthia Kase', tagline: 'Nothing testable · setups are intraday or judgement-based', books: [{ id: 'khtwg', title: 'Trading with the Generals III (Haggerty)', year: 2003, blurb: 'All 18 setups need chart judgement or intraday bars' }, { id: 'cktwo', title: 'Trading with the Odds (Kase)', year: 1996, blurb: 'Kase’s formulas are proprietary or have no exit' }] },
  identities: ["Cynthia Kase","Kevin Haggerty"],
  profile: {
    "kind": "person",
    "mastery": [
      "Day trading",
      "Short-term swing",
      "Volatility"
    ],
    "summary": "Kevin Haggerty was head of equity trading at Fidelity Capital Markets (1990-1997) and co-founded TradingMarkets with Larry Connors; his courses teach professional short-term equity tactics. Cynthia Kase, CMT, is an energy-markets technician whose Trading with the Odds applies probability and statistics to futures trading, including the Kase DevStop volatility stop.",
    "known_for": [
      "Trading with the Generals (Haggerty)",
      "Trading with the Odds (Kase)",
      "Kase DevStop"
    ],
    "sources": [
      "https://tradingmarkets.com/recent/kevin_haggerty-669173",
      "https://cmtassociation.org/presenter/kevin-haggerty/",
      "https://www.kaseco.com/kase-learning-and-support-center/kase-premium-books-and-videos/"
    ],
    "confidence": "medium"
  },
  sourceReview: {
    "khtwg": {"type":"course_manual","people":["Kevin Haggerty"]},
    "cktwo": {"type":"book","people":["Cynthia A. Kase"]},
  },
} satisfies AuthorFile;
