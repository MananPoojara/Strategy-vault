import type {AuthorFile} from '../author-files';

export default {
  order: 340,
  author: { id: 'meyers', name: 'Dennis Meyers', tagline: 'Mathematical short-term systems · Parabolic, MultiBar, Kalman filters', books: [{ id: 'dmast', title: 'Advanced Short Term Systems & Indicators', year: 1999, blurb: 'Meyers · walk-forward-parameter systems; the 5-minute system needs intraday bars' }, { id: 'dmst2', title: 'Short Term Systems & Indicators 2.0', year: 1998, blurb: 'Meyers · Parabolic reproduces 102 vs 103 book trades' }, { id: 'dmtad', title: "Turbo A/D, NH, NL Market System (S&C)", year: 1997, blurb: "Meyers, 1997 · NYSE A/D ratios with smoothed new highs / new lows, stop and reverse on the S&P 500 close" }, { id: 'dmibm', title: "IBM Cubed", year: 1999, readerPaths: ["Trading Knowledge III/Dennis Meyers Publications/Dennis Meyers - Ibm Cubed.pdf"], blurb: "Meyers \u00b7 least-squares cubic forecast of (H+L)/2, always in, walk-forward IBM parameters; Figure 7 lists all 26 IBM trades 1993-98" }, { id: "dmtfly", readerPaths: ["Trading Knowledge I/Stocks & Commodities DVD 11.26 (1982 - 2008) (traders.com)/VOLUMES/V13/C05/MAKING.PDF", "Trading Knowledge I/Stocks & Commodities DVD 11.26 (1982 - 2008) (traders.com)/VOLUMES/V13/C05/SIDETRD.PDF"], title: "Making The Titanic Fly (S&C)", year: 1995, blurb: "Meyers \u00b7 flying Titanic: NYSE breadth thrusts after a 9% DJIA drop, new-low divergence after a 52-week high; sidebar TradeStation code" }, { id: "dmutil", readerPaths: ["Trading Knowledge I/Stocks & Commodities DVD 11.26 (1982 - 2008) (traders.com)/VOLUMES/V13/C08/UTILITY.pdf"], title: "The Utility Average Stock Market Indicator (S&C)", year: 1995, blurb: "Meyers \u00b7 weekly Dow Jones utilities: the UD% switch (5% / 14%) and the utility/S&P relative-strength system (19 weeks, 1.5 / -7, 11 weeks)" }, { id: "dmvrs", readerPaths: ["Trading Knowledge I/Stocks & Commodities DVD 11.26 (1982 - 2008) (traders.com)/VOLUMES/V14/C07/THEADVO.pdf"], title: "A/D Volume, New-High, New-Low System (S&C)", year: 1996, blurb: "Meyers \u00b7 daily NYSE advance/decline volume strength vs the Dow with new-high/new-low strengths, stop and reverse; Figure 4 lists every 1978-96 signal" }, { id: "dmadr", readerPaths: ["Trading Knowledge I/Stocks & Commodities DVD 11.26 (1982 - 2008) (traders.com)/VOLUMES/V15/C01/ADAILYA.pdf"], title: "A Daily A-D New High-New Low Market System (S&C)", year: 1997, blurb: "Meyers \u00b7 daily NYSE advance/decline issue strength vs the Dow with new-high/new-low strengths, stop and reverse; Figure 7 lists every 1978-96 signal" }, { id: "dmlut", readerPaths: ["Trading Knowledge I/Stocks & Commodities DVD 11.26 (1982 - 2008) (traders.com)/VOLUMES/V15/C03/ANEWUTI.pdf"], title: "A New Utility Average Stock Market System (S&C)", year: 1997, blurb: "Meyers \u00b7 daily log strength of the Dow Jones utilities vs the S&P 500 over separate buy (65-day) and sell (100-day) lags; Figure 6 lists every 1953-96 signal" }, { id: "dmtbb", readerPaths: ["Trading Knowledge I/Stocks & Commodities DVD 11.26 (1982 - 2008) (traders.com)/VOLUMES/V15/C11/THETBON.pdf"], title: "The T-Bond Futures & Stock Market Breadth System (S&C)", year: 1997, blurb: "Meyers \u00b7 T-bond futures direction with NYSE advance/decline thrusts and a 1.75% confirmation, stop and reverse on the S&P 500 close" }] },
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
    "dmvrs": {"type":"article","people":["Dennis Meyers"]},
    "dmadr": {"type":"article","people":["Dennis Meyers"]},
    "dmlut": {"type":"article","people":["Dennis Meyers"]},
    "dmtbb": {"type":"article","people":["Dennis Meyers"]},
  },
} satisfies AuthorFile;
