import type {AuthorFile} from '../author-files';

export default {
  order: 370,
  author: { id: 'bollinger', name: 'John Bollinger', tagline: 'Bollinger Bands · Methods II and III', books: [{ id: 'bobb', title: 'Bollinger on Bollinger Bands', year: 2001, blurb: 'Bollinger, 2001 · Method II with band and Parabolic exits, Method III buy; the Squeeze has no defined breakout window' }] },
  identities: ["John Bollinger"],
  profile: {
    "kind": "person",
    "mastery": [
      "Volatility",
      "Indicators",
      "Mean reversion"
    ],
    "summary": "John Bollinger (born 1950), CFA and CMT, developed Bollinger Bands in the early 1980s: trading bands whose width adapts to volatility through the standard deviation. He was chief market analyst at the Financial News Network and runs Bollinger Capital Management.",
    "known_for": [
      "Bollinger Bands",
      "Bollinger on Bollinger Bands (2001)",
      "%b and BandWidth"
    ],
    "sources": [
      "https://en.wikipedia.org/wiki/John_Bollinger",
      "https://www.bollingerbands.com/about-us"
    ],
    "confidence": "high"
  },
  sourceReview: {
    "bobb": {"type":"book","people":["John Bollinger"]},
  },
} satisfies AuthorFile;
