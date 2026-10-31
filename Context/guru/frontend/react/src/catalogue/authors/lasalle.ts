import type {AuthorFile} from '../author-files';

export default {
  order: 1750,
  author: { id: "lasalle", name: "LaSalle Street Trading Group", tagline: "Vendor TradeStation system pack \u00b7 the RemaBands reversal system runs here", books: [
    { id: "lsrema", sourceIds: [], readerPaths: ["USB HDD 1/TradeStation Systems & Indicators/LaSalle Street Trading Group System Pack/LaSalle_RemaBands_Strategy.txt"], title: "RemaBands strategy (System Pack)", blurb: "Regularized-EMA bands, always-in reversal on the close; the readme's vendor profit claim is not shown" }] },
  sourceReview: {
    "lsrema": {"type":"code_package","people":[]},
  },
} satisfies AuthorFile;
