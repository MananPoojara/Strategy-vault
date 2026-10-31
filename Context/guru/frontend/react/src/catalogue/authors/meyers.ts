import type {AuthorFile} from '../author-files';

export default {
  order: 340,
  author: { id: 'meyers', name: 'Dennis Meyers', tagline: 'Mathematical short-term systems · Parabolic, MultiBar, Kalman filters', books: [{ id: 'dmast', title: 'Advanced Short Term Systems & Indicators', year: 1999, blurb: 'Meyers · walk-forward-parameter systems; the 5-minute system needs intraday bars' }, { id: 'dmst2', title: 'Short Term Systems & Indicators 2.0', year: 1998, blurb: 'Meyers · Parabolic reproduces 102 vs 103 book trades' }, { id: 'dmtad', title: "Turbo A/D, NH, NL Market System (S&C)", year: 1997, blurb: "Meyers, 1997 · needs NYSE new highs and new lows" }] },
  identities: ["Dennis Meyers"],
  profile: {
    "kind": "person",
    "mastery": [
      "System design & testing",
      "Short-term swing",
      "Indicators"
    ],
    "summary": "Dennis Meyers, PhD, runs Meyers Analytics, which sells TradeStation/MultiCharts trading systems and walk-forward optimisation tools. A long-time Stocks & Commodities contributor, he writes mathematically based short-term and intraday systems, such as a modified parabolic SAR and breadth-based market systems, and he stresses out-of-sample testing over curve fitting.",
    "known_for": [
      "Power Walk-Forward Optimizer",
      "Turbo A/D, NH, NL Market System (S&C)",
      "Short-term systems & indicators"
    ],
    "sources": [
      "https://meyersanalytics.com/",
      "https://store.traders.com/-v15-c08-theturb-pdf.html",
      "https://store.traders.com/stcov34245tr.html"
    ],
    "confidence": "medium"
  },
  sourceReview: {
    "dmast": {"type":"code_package","people":["Dennis Meyers"]},
    "dmst2": {"type":"code_package","people":["Dennis Meyers"]},
    "dmtad": {"type":"article","people":["Dennis Meyers"]},
  },
} satisfies AuthorFile;
