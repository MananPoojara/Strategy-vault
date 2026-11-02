import type {AuthorFile} from '../author-files';

export default {
  order: 190,
  author: { id: 'ehlers', name: 'John Ehlers', tagline: 'Cycle analysis and DSP filters · MESA, MAMA, SineTrend', books: [{ id: 'ehrs', title: 'Rocket Science for Traders', year: 2001, blurb: 'Ehlers, 2001 · SineTrend and CycPart systems from the book’s code' }, { id: 'ehmd', title: 'MESA articles and MAMA', year: 2001, blurb: 'Ehlers · MAMA/FAMA crossover, Laguerre RSI, LMS predictor' }, { id: 'ehmtc', title: 'MESA and Trading Market Cycles', year: 1992, blurb: 'Ehlers, 1992 · needs the MESA program’s dominant cycle; no code given' }, { id: 'ehpred', sourceIds: [], readerPaths: ['USB HDD 1/TradeStation Systems & Indicators/R-MESA 5 (mesasoftware.com)/_Old Mesa versions/Mesa 98  4.1 for TS & Manual/Mesa98 v4.1 - Powerpoint Tutorial & Word documents/Powerpoint Tuturiol/PREDICT.ppt'], title: 'Predictive Indicators (Mesa98 seminar slides)', year: null, blurb: 'Ehlers · the simple predictive system: close + (close − EMA) crossing the EMA, stop and reverse (his own warning: too many whipsaws)' }, { id: "scehmacd", sourceIds: [], readerPaths: ["Trading Knowledge I/Stocks & Commodities DVD 11.26 (1982 - 2008) (traders.com)/VOLUMES/V09/C10/MACDREV.PDF"], title: "The MACD Indicator Revisited (Ehlers, S&C V.9:10)", year: 1991, blurb: "Ehlers · inverse MACD 4/12 (EMA12 - EMA4) crossing its 12-day signal, always in" }, { id: "scehzl", sourceIds: [], readerPaths: ["Trading Knowledge I/Stocks & Commodities 2010 -2011 (traders.com)/SC2010-11.pdf"], title: "Zero Lag (Well, Almost) (Ehlers & Way, S&C Nov 2010)", year: 2010, blurb: "Ehlers & Way · error-correcting EMA crossing the EMA with a least-error threshold, always in" }] },
  identities: ["John Ehlers"],
  profile: {
    "kind": "person",
    "mastery": [
      "Cycles",
      "Indicators"
    ],
    "summary": "John Ehlers is an electrical engineer who retired from Raytheon as a Senior Engineering Fellow and applies digital signal processing to trading. He adapted Maximum Entropy spectrum analysis into MESA to measure market cycles and has written four Wiley books and over 75 Stocks & Commodities articles.",
    "known_for": [
      "MESA (Maximum Entropy Spectrum Analysis)",
      "MAMA adaptive moving average",
      "Rocket Science for Traders"
    ],
    "sources": [
      "https://www.mesasoftware.com/"
    ],
    "confidence": "high"
  },
  sourceReview: {
    "ehrs": {"type":"book","people":["John F. Ehlers"]},
    "ehmd": {"type":"article","people":["John F. Ehlers"]},
    "ehmtc": {"type":"book","people":["John F. Ehlers"]},
    "ehpred": {"type":"other","people":["John F. Ehlers"]},
    "scehmacd": {"type":"article","people":["John F. Ehlers"]},
    "scehzl": {"type":"article","people":["John F. Ehlers", "Ric Way"]},
  },
} satisfies AuthorFile;
