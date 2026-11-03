import type {AuthorFile} from '../author-files';

export default {
  order: 490,
  author: { id: 'lukac', name: "Lukac, Conway & Hamon", tagline: "Twelve-system study, stock-system design, Spike 35", books: [{ id: 'lbi12', title: "A Comparison of Twelve Technical Trading Systems (Lukac, Brorsen & Irwin)", year: 1985, blurb: "Lukac et al., 1985 · all 12 systems with the book’s yearly three-year re-optimisation; fixed parameters are an option" }, { id: 'cbpst', title: "Professional Stock Trading: System Design and Automation (Conway & Behle)", year: 2003, blurb: "Conway & Behle, 2003 · M, N, R and V systems from their TradeStation code, also on NSE stocks" }, { id: 'jhs35', title: "Spike 35 Trader’s Manual (Hamon)", year: null, blurb: "Hamon, 2nd ed. · about 60 trades a year, so costs matter" }, { id: 'jhbct', title: "Breakthroughs in Commodity Technical Analysis (Hamon)", year: null, blurb: "Hamon · weekly two-week rule variants" }] },
  identities: [],
  profile: {
    "kind": "collection",
    "mastery": [
      "System design & testing",
      "Trend following",
      "Futures"
    ],
    "summary": "System-testing research on futures. Lukac, Brorsen and Irwin's 1988 study simulated twelve technical systems on twelve commodities (1978-84) and found four with significant net, risk-adjusted returns. The group also covers Conway & Behle on automated stock-system design and Hamon's commodity-trading manuals.",
    "known_for": [
      "A Comparison of Twelve Technical Trading Systems",
      "Professional Stock Trading: System Design and Automation",
      "Spike 35 Trader's Manual"
    ],
    "sources": [
      "https://doi.org/10.1080/00036848800000113"
    ],
    "confidence": "medium"
  },
  sourceReview: {
    "lbi12": {"type":"book","people":["B. Wade Brorsen","Louis P. Lukac","Scott H. Irwin"]},
    "cbpst": {"type":"book","people":["Aaron N. Behle","Mark R. Conway"]},
    "jhs35": {"type":"code_package","people":["J. D. Hamon"]},
    "jhbct": {"type":"book","people":["J. D. Hamon"]},
  },
} satisfies AuthorFile;
