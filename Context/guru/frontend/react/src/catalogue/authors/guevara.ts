import type {AuthorFile} from '../author-files';

export default {
  order: 1091,
  author: { id: "guevara", name: "Alexandra Guevara", tagline: "TradeStation market technician · the RSI Volatility Spread strategy runs here", books: [
    { id: "tslrvs", sourceIds: [], readerPaths: ["USB HDD 1/2011/_TradeStation/RSI Volatility Spread/TradeStation Labs.mht", "USB HDD 1/2011/_TradeStation/RSI Volatility Spread/AG.ATR RSI Strategy.txt"], title: "RSI Volatility Spread (TradeStation Labs)", year: 2010, blurb: "Guevara · RSI of the range average of a security against the S&P 500's (Data2); buy lower-than-average relative volatility, sell higher. Tested by the author on EWA and other ETFs" }] },
  identities: ["Alexandra Guevara"],
  sourceReview: {
    "tslrvs": {"type":"article","people":["Alexandra Guevara"]},
  },
} satisfies AuthorFile;
