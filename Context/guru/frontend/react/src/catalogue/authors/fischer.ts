import type {AuthorFile} from '../author-files';

export default {
  order: 620,
  author: { id: "fischer", name: "Robert Fischer", tagline: "Fibonacci and PHI-ellipse trader · swing-size Fibonacci systems, held until a swing size for non-futures markets is chosen", books: [
    { id: "rfcfcp", sourceIds: ["title-64beb6bfbd4c1c7b", "title-005448e05b8851b6"], title: "Candlesticks, Fibonacci, and Chart Pattern Trading Tools", year: 2003, blurb: "Robert & Jens Fischer, 2003 · double-top breakout and 61.8% correction systems need a swing size in points; PHI tools are judgement" },
    { id: "rffaps", sourceIds: ["title-7b3c3e3546fff1e7"], title: "Fibonacci Applications and Strategies for Traders", year: 1993, blurb: "Fischer, 1993 · swing breakout, wave-3 extension and time-goal-day systems, all on fixed futures-point swings" },
    { id: "rfnft", sourceIds: ["title-7cd59c28e9db4089", "title-70142b228b7b27d6"], title: "The New Fibonacci Trader (and Workbook)", year: 2001, blurb: "Fischer, 2001 · the 61.8% correction system again; extensions and time goals have no entry/exit rules" }] },
  identities: ["Robert Fischer"],
  sourceReview: {
    "rfcfcp": {"type":"book","people":["Robert Fischer"]},
    "rffaps": {"type":"book","people":["Robert Fischer"]},
    "rfnft": {"type":"book","people":["Robert Fischer"]},
  },
} satisfies AuthorFile;
