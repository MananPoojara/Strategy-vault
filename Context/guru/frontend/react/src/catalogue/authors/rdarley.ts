import type {AuthorFile} from '../author-files';

export default {
  order: 1740,
  author: { id: "rdarley", name: "Roger Darley", tagline: "Stocks & Commodities author \u00b7 the Hilbert Channel Breakout runs here with its printed 15-bar inputs and with the article's adaptive Hilbert-period inputs", books: [
    { id: "rdhcb", sourceIds: [], readerPaths: ["USB HDD 1/MetaStock/@Systems & Indicators/@Misc/- Metastock Systems & Indicators/Hilbert Indicators + Info/hillbert_metastock.doc", "Trading Knowledge I/Stocks & Commodities DVD 11.26 (1982 - 2008) (traders.com)/VOLUMES/V18/C11/100Opt.pdf", "Trading Knowledge I/Stocks & Commodities DVD 11.26 (1982 - 2008) (traders.com)/VOLUMES/V18/C11/109TIPS.pdf"], title: "Optimizing With Hilbert Indicators (S&C, Nov 2000) and Traders' Tips", year: 2000, blurb: "Darley \u00b7 buy above the 15-bar high, out below the 15-bar low at the next open; adaptive form: 0.1 x / 3.4 x Ehlers' Hilbert cycle period (the article's GE inputs)" }] },
  identities: ["Roger Darley"],
  sourceReview: {
    "rdhcb": {"type":"article","people":["Roger Darley"]},
  },
} satisfies AuthorFile;
