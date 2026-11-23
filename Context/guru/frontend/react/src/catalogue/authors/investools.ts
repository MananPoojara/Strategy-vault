import type {AuthorFile} from '../author-files';

export default {
  order: 1807,
  author: { id: "investools", name: "INVESTools", tagline: "Investor-education courses (2005) · the three-arrow buy and sell rules and the workshop's MACD backtest example run here", books: [
    { id: "it5sf", sourceIds: [], readerPaths: ["USB HDD 1/TRADING KNOWLEDGE-2/_InvesTools.com/5-Step Online Investing Formula DVD Course & Manual (Video 3.91 GB)/Manual/6.- Step 4 - Technical Analysis.pdf"], title: "The 5-Step Online Investing Formula, Step 4: Technical Analysis (course manual)", year: 2005, blurb: "INVESTools · buy when the 30-day average, MACD and stochastics arrows are all green, stop 3% under the two-week low, sell on three reds" },
    { id: "itatw", sourceIds: [], readerPaths: ["USB HDD 1/TRADING KNOWLEDGE-2/_InvesTools.com/Advanced Technical Analysis. Know Exactly What the Market is Telling You (Video & Manuals 1.06 GB)/4A - INVESTools Advanced Technical Analysis - Workshop Manuals.pdf"], title: "Advanced Technical Analysis Workshop Manual, ch.6 backtesting example", year: 2005, blurb: "INVESTools · MACD 8/17/9 cross in the direction of the 30-day average, next-open entry, $1.00 trailing stop (GE 2004 example)" },
    { id: "invcr", sourceIds: [], readerPaths: ["Trading Knowledge I/@investools.com/Advanced Currency Trader Manual/INVESTools ADVANCED_CURRENCY_TRADER_MANUAL.pdf"], title: "Advanced Currency Trader Course Manual: the trending system (INVESTools, 2006)", year: 2006, blurb: "INVESTools \u00b7 MACD 8-17-9 cross with the 21-day average direction, exit on the opposite cross, 100-pip backup stop; runs on NIFTY 50 / NIFTY BANK daily bars (1 pip = 0.01 percent) as a declared adaptation, no results printed, unverified; the manual's RSI bracket system is not published (stop and limit share a daily bar too often on NIFTY BANK)" }] },
  sourceReview: {
    "it5sf": {"type":"other","people":[]},
    "itatw": {"type":"other","people":[]},
    "invcr": {"type":"course_manual","people":[]},
  },
} satisfies AuthorFile;
