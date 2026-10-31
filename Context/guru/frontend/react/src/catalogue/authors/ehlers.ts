import type {AuthorFile} from '../author-files';

export default {
  order: 190,
  author: { id: 'ehlers', name: 'John Ehlers', tagline: 'Cycle analysis and DSP filters · MESA, MAMA, SineTrend', books: [{ id: 'ehrs', title: 'Rocket Science for Traders', year: 2001, blurb: 'Ehlers, 2001 · SineTrend and CycPart systems from the book’s code' }, { id: 'ehmd', title: 'MESA articles and MAMA', year: 2001, blurb: 'Ehlers · MAMA/FAMA crossover, Laguerre RSI, LMS predictor' }, { id: 'ehmtc', title: 'MESA and Trading Market Cycles', year: 1992, blurb: 'Ehlers, 1992 · needs the MESA program’s dominant cycle; no code given' }, { id: 'ehpred', sourceIds: [], readerPaths: ['USB HDD 1/TradeStation Systems & Indicators/R-MESA 5 (mesasoftware.com)/_Old Mesa versions/Mesa 98  4.1 for TS & Manual/Mesa98 v4.1 - Powerpoint Tutorial & Word documents/Powerpoint Tuturiol/PREDICT.ppt'], title: 'Predictive Indicators (Mesa98 seminar slides)', year: null, blurb: 'Ehlers · the simple predictive system: close + (close − EMA) crossing the EMA, stop and reverse (his own warning: too many whipsaws)' }] },
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
  },
} satisfies AuthorFile;
