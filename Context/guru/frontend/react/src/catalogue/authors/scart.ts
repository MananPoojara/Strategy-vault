import type {AuthorFile} from '../author-files';

export default {
  order: 1450,
  author: { id: 'scart', name: "Stocks & Commodities authors", tagline: "Single-article systems", books: [{ id: 'ddrvi', title: "Refining the RVI (Dorsey)", year: null, blurb: "Dorsey · inertia system; S&P win rate 52% vs the article’s 53%" }, { id: 'aatt', title: "Trading the Trend (Abraham)", year: 1998, blurb: "Abraham, S&C 1998 · ATR trailing trend system" }, { id: 'rhlss', title: "LSS 3-day cycle (Angell)", year: null, blurb: "Level formulas missing; day type is judgement" }, { id: 'cmeod', title: "End of Day Trading System (Mathews)", year: null, blurb: "Judgement terms throughout" }, { id: 'mjzz', title: "Zigzag Signpost (M.S. Johnson)", year: null, blurb: "Exits left to the trader" }, { id: "sctraub", sourceIds: [], readerPaths: ["Trading Knowledge I/Stocks & Commodities DVD 11.26 (1982 - 2008) (traders.com)/VOLUMES/V12/C12/THREE.PDF"], title: "Three Market Timing Strategies (Traub, S&C V.12:12)", year: 1994, blurb: "Traub · weekly trend timing; the prime-rate and point-oscillator strategies are held" }, { id: "sctoombs", sourceIds: [], readerPaths: ["Trading Knowledge I/Stocks & Commodities DVD 11.26 (1982 - 2008) (traders.com)/VOLUMES/V08/C07/SHORT.pdf"], title: "A Short-Term Buy Signal For The DJIA (Toombs, S&C V.8:7)", year: 1990, blurb: "Toombs · slow 20-day stochastic dip buy on the DJIA; 20 DJI trades 1984-1990 against the article's 20" }, { id: "scdrvi", sourceIds: [], readerPaths: ["Trading Knowledge I/Stocks & Commodities DVD 11.26 (1982 - 2008) (traders.com)/VOLUMES/V11/C06/THERELA.pdf"], title: "The Relative Volatility Index (Dorsey, S&C V.11:6)", year: 1993, blurb: "Dorsey · 10/20 SMA crossover confirmed by the RVI, daily declared application" }, { id: "scevdmi", sourceIds: [], readerPaths: ["Trading Knowledge I/Stocks & Commodities DVD 11.26 (1982 - 2008) (traders.com)/VOLUMES/V17/C02/014DIR.pdf"], title: "Directional Movement (Evens, S&C V.17:2)", year: 1999, blurb: "Evens · DI lead with ADX rising and ADXR over 25, long and short" }, { id: "scvkvs", sourceIds: [], readerPaths: ["Trading Knowledge I/Stocks & Commodities DVD 11.26 (1982 - 2008) (traders.com)/VOLUMES/V17/C10/075VOL.pdf"], title: "The Volatility Stop System (Vakkur, S&C V.17:10)", year: 1999, blurb: "Vakkur · weekly breakout over the 12-week EMA with a ratcheting ATR stop" }, { id: "scevkc", sourceIds: [], readerPaths: ["Trading Knowledge I/Stocks & Commodities DVD 11.26 (1982 - 2008) (traders.com)/VOLUMES/V17/C12/095KELT.pdf"], title: "Keltner Channel (Evens, S&C V.17:12)", year: 1999, blurb: "Evens · Keltner channel methods A-D, two with an RSI filter" }, { id: "scboot", sourceIds: [], readerPaths: ["Trading Knowledge I/Stocks & Commodities DVD 11.26 (1982 - 2008) (traders.com)/VOLUMES/V18/C09/084ADX.pdf"], title: "Trend-Following With The ADX (Boot, S&C V.18:9)", year: 2000, blurb: "Boot · buy when ADX under 21 turns up, out at 35; the multiple-entry versions are not coded" }] },
  identities: [],
  profile: {
    "kind": "collection",
    "mastery": [
      "Indicators",
      "Short-term swing",
      "Cycles"
    ],
    "summary": "Systems and indicators from Technical Analysis of Stocks & Commodities magazine articles. They include Dorsey's refined Relative Volatility Index, Abraham's trend system, Angell's LSS 3-day cycle, a swing indicator, an end-of-day system and a zigzag signpost.",
    "known_for": [
      "Refining the RVI (Dorsey)",
      "LSS 3-day cycle (Angell)",
      "Trading the Trend (Abraham)"
    ],
    "sources": [],
    "confidence": "medium"
  },
  sourceReview: {
    "ddrvi": {"type":"article","people":["Donald Dorsey"]},
    "aatt": {"type":"article","people":["Andrew Abraham"]},
    "rhlss": {"type":"article","people":["George Angell"]},
    "cmeod": {"type":"book","people":["Chris Mathews"]},
    "mjzz": {"type":"book","people":["Michael S Johnson"]},
    "sctraub": {"type":"article","people":["Daniel J. Traub"]},
    "sctoombs": {"type":"article","people":["John Toombs"]},
    "scdrvi": {"type":"article","people":["Donald Dorsey"]},
    "scevdmi": {"type":"article","people":["Stuart Evens"]},
    "scvkvs": {"type":"article","people":["Mark Vakkur"]},
    "scevkc": {"type":"article","people":["Stuart Evens"]},
    "scboot": {"type":"article","people":["Martin Boot"]},
  },
} satisfies AuthorFile;
