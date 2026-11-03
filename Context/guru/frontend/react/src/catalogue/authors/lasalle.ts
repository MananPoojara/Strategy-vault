import type {AuthorFile} from '../author-files';

export default {
  order: 1750,
  author: { id: "lasalle", name: "LaSalle Street Trading Group", tagline: "Vendor TradeStation system pack \u00b7 the RemaBands reversal system runs here", books: [
    { id: "lsrema", sourceIds: [], readerPaths: ["USB HDD 1/TradeStation Systems & Indicators/LaSalle Street Trading Group System Pack/LaSalle_RemaBands_Strategy.txt"], title: "RemaBands strategy (System Pack)", blurb: "Regularized-EMA bands, always-in reversal on the close; the readme's vendor profit claim is not shown" }, { id: "lspack", sourceIds: [], readerPaths: ["USB HDD 1/TradeStation Systems & Indicators/LaSalle Street Trading Group System Pack/LaSalleRsiSystem_Strategy.txt", "USB HDD 1/TradeStation Systems & Indicators/LaSalle Street Trading Group System Pack/LaSalle_AtrVolRev_Strategy.txt"], title: "RSI System and ATR Volatility Reversal (System Pack)", year: null, blurb: "Laguerre-type RSI 0.18 / 0.75 reversals at the close; ATR(5) stop breakout from the open (long only as printed, reversing as an option)" }] },
  sourceReview: {
    "lsrema": {"type":"code_package","people":[]},
    "lspack": {"type":"code_package","people":[]},
  },
} satisfies AuthorFile;
