import type {AuthorFile} from '../author-files';

export default {
  order: 1860,
  author: { id: "bcollette", name: "Barry Collette", tagline: "Bollinger Bands course author \u00b7 the Squeeze Breakout with a Parabolic SAR stop", books: [
    { id: "bcbbc", sourceIds: [], readerPaths: ["USB HDD 2/tRADING KNO-2/Barry Collette Bollinger Bands Course (114 MB)/Bollinger Bands.pdf"], title: "Bollinger Bands (course manual)", year: 2003, blurb: "Collette \u00b7 BandWidth under 10, close outside the band, next open; Parabolic SAR trailing stop" }] },
  identities: ["Barry Collette"],
  sourceReview: {
    "bcbbc": {"type":"other","people":["Barry Collette"]},
  },
} satisfies AuthorFile;
