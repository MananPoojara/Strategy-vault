import type {AuthorFile} from '../author-files';

export default {
  order: 1990,
  author: { id: "tscourses", name: "TradeStation and MetaStock course examples", tagline: "Vendor manuals and courses (Equis, Omega Research, TradeStation, NewMarket Software) · their worked strategies run as labelled teaching examples", books: [
    { id: "eqfp", sourceIds: [], readerPaths: ["USB HDD 1/MetaStock/@Knowledge/Equis International - Formula Primer Book/Formula Primer Metastock.pdf"], title: "MetaStock Formula Primer (Equis International)", year: 2002, blurb: "System Tester example · 21/40-day average cross in the direction of the 200-day average, long and short, at the close" },
    { id: "omelgs", sourceIds: [], readerPaths: ["USB HDD 1/TradeStation/_Knowleage/Easy Language Getting Started.pdf"], title: "EasyLanguage for TradeStation, Getting Started (Omega Research)", year: null, blurb: "Key Reversal LE with the library LX stop · the ch.7 9/18 average strategy is the same rule as atm00_ma_9_18, not coded twice" },
    { id: "tselhs", sourceIds: [], readerPaths: ["USB HDD 1/TradeStation/_Knowleage/EasyLanguage Home Study Course, $149 (tradestation.com)/EasyLanguage Home Study Course.pdf"], title: "EasyLanguage Home Study Course (TradeStation)", year: 2007, blurb: "Exercises #24-#27 · 8-bar breakout stops, 9/18 average cross with a stale-trade exit (US), real-body average, momentum cross with limits" },
    { id: "ttelru", sourceIds: [], readerPaths: ["USB HDD 1/TradeStation/Omega Research Prosuite 2000i/Utilites/ProSuite doc/EasyLanguage.doc"], title: "EasyLanguage course, Russian translation (Omega Research / TELETRADE)", year: null, blurb: "KeyReversal, XBarHighLow and Momentum System, as printed" },
    { id: "nmstc", sourceIds: [], readerPaths: ["Trading Courses,Seminars,Videos/Trading Systems Course (tradingalliance.com)/ts/DM.pps", "Trading Courses,Seminars,Videos/Trading Systems Course (tradingalliance.com)/ts/PARABS.pps"], title: "How to build your own Trading System (NewMarket Software, tradingalliance.com)", year: null, blurb: "NAVIGATOR · DM Trading System (TD-line close breaks) and the simple Parabolic System (SAR against a 5-day EMA)" }] },
  sourceReview: {
    "eqfp": {"type":"course_manual","people":[]},
    "omelgs": {"type":"course_manual","people":[]},
    "tselhs": {"type":"course_manual","people":[]},
    "ttelru": {"type":"course_manual","people":[]},
    "nmstc": {"type":"course_manual","people":[]},
  },
} satisfies AuthorFile;
