import type {AuthorFile} from '../author-files';

export default {
  order: 1790,
  author: { id: "kalosz", name: "K.A.L.O.S.Z. MetaStock formula archive", tagline: "Polish web archive of MetaStock list posts \u00b7 Lewandowski's MACDhistogram system and Lindstedt's 'No Name 1' run here", books: [
    { id: "kzmsf", sourceIds: [], readerPaths: ["USB HDD 1/MetaStock/@Systems & Indicators/K.A.L.O.S.Z Archive of Metastock Formulas/K.A.L.O.S.Z Archive of Metastock Formulas.pdf"], title: "K.A.L.O.S.Z. Archive of MetaStock Formulas", year: 2001, blurb: "Marek Lewandowski \u00b7 MACD histogram sign with a Stoch(5,3) filter, reverse at the open; Chris Lindstedt (for Svein Andersen) \u00b7 close above yesterday's high, out below the low of the highest bar" },
    { id: "kzmzn", sourceIds: [], readerPaths: ["USB HDD 1/MetaStock/@Systems & Indicators/K.A.L.O.S.Z Archive of Metastock Formulas/Metastock Zone.pdf"], title: "Metastock Zone (System Tester collection)", year: null, blurb: "Key Reversals (Scorpio), StochRSI (Baude), StoRSI (Karnish), Onno's Binary Wave, Wallace's Psychological Index, SectorBets' Coppock signals, Rejczak's WIG 20 system, the Simianer stochastic test, Ken's DeMark setup test, Estes' Momentum Time and Thamrin's 1-2-3 reversal, run as printed with close fills" }] },
  identities: ["Marek Lewandowski", "Chris Lindstedt"],
  sourceReview: {
    "kzmsf": {"type":"code_package","people":["Marek Lewandowski", "Chris Lindstedt"]},
    "kzmzn": {"type":"code_package","people":["Massimiliano Scorpio", "Christian Baude", "Steve Karnish", "Onno Goedknegt", "Glen Wallace", "Pawel Rejczak", "Richard Estes", "Yuwono Thamrin"]},
  },
} satisfies AuthorFile;
