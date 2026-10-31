import type {AuthorFile} from '../author-files';

export default {
  order: 1790,
  author: { id: "kalosz", name: "K.A.L.O.S.Z. MetaStock formula archive", tagline: "Polish web archive of MetaStock list posts \u00b7 Lewandowski's MACDhistogram system and Lindstedt's 'No Name 1' run here", books: [
    { id: "kzmsf", sourceIds: [], readerPaths: ["USB HDD 1/MetaStock/@Systems & Indicators/K.A.L.O.S.Z Archive of Metastock Formulas/K.A.L.O.S.Z Archive of Metastock Formulas.pdf"], title: "K.A.L.O.S.Z. Archive of MetaStock Formulas", year: 2001, blurb: "Marek Lewandowski \u00b7 MACD histogram sign with a Stoch(5,3) filter, reverse at the open; Chris Lindstedt (for Svein Andersen) \u00b7 close above yesterday's high, out below the low of the highest bar" }] },
  identities: ["Marek Lewandowski", "Chris Lindstedt"],
  sourceReview: {
    "kzmsf": {"type":"code_package","people":["Marek Lewandowski", "Chris Lindstedt"]},
  },
} satisfies AuthorFile;
