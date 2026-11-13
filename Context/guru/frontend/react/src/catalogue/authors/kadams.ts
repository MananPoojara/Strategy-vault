import type {AuthorFile} from '../author-files';

export default {
  order: 2220,
  author: { id: "kadams", name: "Kevin Adams", tagline: "Cracking The Forex Code · two moving-average forex systems; CFC2 (EMA 5/12 cross + Parabolic SAR) runs here on daily bars, CFC1 held (pip exits)", books: [
    { id: "kacfc", sourceIds: [], readerPaths: ["USB HDD 2/tRADING KNO-2/Kevin Adams - Cracking The Forex Code (crackingforexcode.com)/Cracking The Forex Code(Kevin Adams nd).pdf"], title: "Cracking The Forex Code", year: null, blurb: "Adams e-book (crackingforexcode.com, no date) · CFC1 EMA 5/15/60 pullback with pip stops and trailing, CFC2 EMA 5/12 cross with the Parabolic SAR exit; 1-hour charts, 'also 4-hour and daily'; chart examples only" }] },
  identities: ["Kevin Adams"],
  sourceReview: {
    "kacfc": {"type":"book","people":["Kevin Adams"]},
  },
} satisfies AuthorFile;
