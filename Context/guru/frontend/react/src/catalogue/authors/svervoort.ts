import type {AuthorFile} from '../author-files';

export default {
  order: 2060,
  author: { id: "svervoort", name: "Sylvain Vervoort", tagline: "MetaStock system tests from the LOCKIT package · zero-lag TEMA crossovers", books: [
    { id: "svlock", sourceIds: [], readerPaths: ["USB HDD 1/MetaStock/@Systems & Indicators/Sylvain Vervoort's LOCKIT System (traderslibrary.com)/Vervoort.exe"], title: "LOCKIT System for MetaStock: SVE_ST_Cross_ZLCl_ZLHa", year: null, blurb: "Long when the zero-lag 65 TEMA of the typical price crosses above that of the Heikin-Ashi close, out on the cross back; close fills" }] },
  identities: ["Sylvain Vervoort"],
  sourceReview: {
    "svlock": {"type":"code_package","people":["Sylvain Vervoort"]},
  },
} satisfies AuthorFile;
