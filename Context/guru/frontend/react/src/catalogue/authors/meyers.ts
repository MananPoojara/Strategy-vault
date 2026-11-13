import type {AuthorFile} from '../author-files';

export default {
  order: 340,
  author: { id: 'meyers', name: 'Dennis Meyers', tagline: 'Mathematical short-term systems · Parabolic, MultiBar, Kalman filters', books: [{ id: 'dmast', title: 'Advanced Short Term Systems & Indicators', year: 1999, blurb: 'Meyers · walk-forward-parameter systems; the 5-minute system needs intraday bars' }, { id: 'dmst2', title: 'Short Term Systems & Indicators 2.0', year: 1998, blurb: 'Meyers · Parabolic reproduces 102 vs 103 book trades' }, { id: 'dmtad', title: "Turbo A/D, NH, NL Market System (S&C)", year: 1997, blurb: "Meyers, 1997 · NYSE A/D ratios with smoothed new highs / new lows, stop and reverse on the S&P 500 close" }, { id: 'dmibm', title: "IBM Cubed", year: 1999, readerPaths: ["Trading Knowledge III/Dennis Meyers Publications/Dennis Meyers - Ibm Cubed.pdf"], blurb: "Meyers \u00b7 least-squares cubic forecast of (H+L)/2, always in, walk-forward IBM parameters; Figure 7 lists all 26 IBM trades 1993-98" }, { id: "dmtfly", readerPaths: ["Trading Knowledge I/Stocks & Commodities DVD 11.26 (1982 - 2008) (traders.com)/VOLUMES/V13/C05/MAKING.PDF", "Trading Knowledge I/Stocks & Commodities DVD 11.26 (1982 - 2008) (traders.com)/VOLUMES/V13/C05/SIDETRD.PDF"], title: "Making The Titanic Fly (S&C)", year: 1995, blurb: "Meyers \u00b7 flying Titanic: NYSE breadth thrusts after a 9% DJIA drop, new-low divergence after a 52-week high; sidebar TradeStation code" }, { id: "dmutil", readerPaths: ["Trading Knowledge I/Stocks & Commodities DVD 11.26 (1982 - 2008) (traders.com)/VOLUMES/V13/C08/UTILITY.pdf"], title: "The Utility Average Stock Market Indicator (S&C)", year: 1995, blurb: "Meyers \u00b7 weekly Dow Jones utilities: the UD% switch (5% / 14%) and the utility/S&P relative-strength system (19 weeks, 1.5 / -7, 11 weeks)" }] },
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
    "dmibm": {"type":"article","people":["Dennis Meyers"]},
    "dmtfly": {"type":"article","people":["Dennis Meyers"]},
    "dmutil": {"type":"article","people":["Dennis Meyers"]},
  },
} satisfies AuthorFile;
