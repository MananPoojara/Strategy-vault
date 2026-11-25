import type {AuthorFile} from '../author-files';

export default {
  order: 2820,
  author: { id: "jcorona", name: "Joe Corona", tagline: "Professional Options Trading College · volatility-index signals for direction", books: [
    { id: "jcvix", sourceIds: [], readerPaths: ["Trading Courses,Seminars,Videos/Joe Corona - Professional Options Trading College (Videos & Manuals 11.2 GB), $2995 (tradingmarkets.com)/Week 5/Week 5 Notes.pdf"], title: "Professional Options Trading College, Week 5 (VIX, VXN and direction/volatility combinations)", year: 2005, blurb: "TradingMarkets course notes · buy the index when the VIX closes 10% above its 10-day SMA and exit when it crosses back below, sell on the mirror; four unstated details labelled, the course's option spreads not modelled (no option prices held; the card trades the underlying); SPX/SPY with the VIX and NIFTY 50/NIFTY BANK with India VIX (declared adaptation); the slides' backtest tables carry no dates, so nothing is compared; unverified" }] },
  identities: ["Joe Corona"],
  sourceReview: {
    "jcvix": {"type":"course","people":["Joe Corona"]},
  },
} satisfies AuthorFile;
