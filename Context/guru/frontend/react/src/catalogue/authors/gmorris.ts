import type {AuthorFile} from '../author-files';

export default {
  order: 1820,
  author: { id: "gmorris", name: "Gregory L. Morris", tagline: "Technical analyst and software developer \u00b7 the 1987 'Trend of the trend' oscillator runs here with its DJIA settings", books: [
    { id: "gmtot", sourceIds: [], readerPaths: ["USB HDD 2/tRADING KNO-2/Barbara Star - Detecting Trend Direction & Strength (Article)/Barbara Star - Detecting Trend Direction & Strength (Article).pdf"], title: "Trend of the trend (Stocks & Commodities V.5:2, 1987)", year: 1987, blurb: "Morris \u00b7 EMA(7) - EMA(42) crossing its 21-day average, stop-and-reverse at the next open; the 4.5% band filter is not coded" }, { id: "gmins", sourceIds: [], readerPaths: ["USB HDD 1/TradeStation Systems & Indicators/_Misc/Greg Morris - Insync Index for ts/Insync Index G. Morris.rar"], title: "InSync Index trader (G. Morris Corp. / TradeWind, 1995)", year: 1995, blurb: "Ten overbought / oversold readings summed around 50; short when it falls back under 95, buy when it rises back over 5" }] },
  identities: ["Gregory L. Morris"],
  sourceReview: {
    "gmtot": {"type":"article","people":["Gregory L. Morris"]},
    "gmins": {"type":"code_package","people":["Gregory L. Morris"]},
  },
} satisfies AuthorFile;
