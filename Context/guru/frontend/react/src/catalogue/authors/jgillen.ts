import type {AuthorFile} from '../author-files';

export default {
  order: 2200,
  author: { id: "jgillen", name: "Jack Gillen", tagline: "Astro Stats for the NYSE · close-to-close DJIA trades on astrological dates and the book's tables to 2050; 6 methods run here (the three Sun-stats windows are one method, the three retrograde tables another), 3 held", books: [
    { id: "jgasn", sourceIds: [], readerPaths: ["USB HDD 2/tRADING KNO-2/Jack Gillen - Astro Stats for the New York Stock Exchange/ASNYSE.pdf"], title: "Astro Stats for the New York Stock Exchange", year: 2002, blurb: "Gillen, Real-Time Market Forecast 2002 · Sun, Moon, Mercury, Saturn and Jupiter date windows on the Dow (enter at one date's close, exit at another's), tables to 2050; Venus, Mars and the top-20 stock picks held" }] },
  identities: ["Jack Gillen"],
  sourceReview: {
    "jgasn": {"type":"book","people":["Jack Gillen"]},
  },
} satisfies AuthorFile;
