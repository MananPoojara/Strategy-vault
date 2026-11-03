import type {AuthorFile} from '../author-files';

export default {
  order: 1807,
  author: { id: "investools", name: "INVESTools", tagline: "Investor-education courses (2005) · the three-arrow buy and sell rules and the workshop's MACD backtest example run here", books: [
    { id: "it5sf", sourceIds: [], readerPaths: ["USB HDD 1/TRADING KNOWLEDGE-2/_InvesTools.com/5-Step Online Investing Formula DVD Course & Manual (Video 3.91 GB)/Manual/6.- Step 4 - Technical Analysis.pdf"], title: "The 5-Step Online Investing Formula, Step 4: Technical Analysis (course manual)", year: 2005, blurb: "INVESTools · buy when the 30-day average, MACD and stochastics arrows are all green, stop 3% under the two-week low, sell on three reds" },
    { id: "itatw", sourceIds: [], readerPaths: ["USB HDD 1/TRADING KNOWLEDGE-2/_InvesTools.com/Advanced Technical Analysis. Know Exactly What the Market is Telling You (Video & Manuals 1.06 GB)/4A - INVESTools Advanced Technical Analysis - Workshop Manuals.pdf"], title: "Advanced Technical Analysis Workshop Manual, ch.6 backtesting example", year: 2005, blurb: "INVESTools · MACD 8/17/9 cross in the direction of the 30-day average, next-open entry, $1.00 trailing stop (GE 2004 example)" }] },
  sourceReview: {
    "it5sf": {"type":"other","people":[]},
    "itatw": {"type":"other","people":[]},
  },
} satisfies AuthorFile;
