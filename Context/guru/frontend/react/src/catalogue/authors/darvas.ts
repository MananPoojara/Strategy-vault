import type {AuthorFile} from '../author-files';

export default {
  order: 1560,
  author: { id: 'darvas', name: 'Nicolas Darvas', tagline: 'Box theory · boxes judged by eye; the DAR-CARD box rules need an undefined topmost box (held)', books: [{ id: 'darvas2m', sourceIds: ["source-1916d685a21701c5", "source-8da9573d054a76cb"], title: 'How I Made $2,000,000 in the Stock Market', year: 1960, blurb: "Darvas, 1960 · box theory and techno-fundamental selection; held: 'I did not find any fixed rule'" }, { id: "dvycs", sourceIds: ["source-8bdf5ba98301902c"], title: "You Can Still Make It in the Market", year: 1977, blurb: "Darvas, 1977 · DAR-CARD: three-day box top/bottom, 10% first stop, 5% danger zone; held: 'topmost box' undefined" }] },
  identities: ["Nicolas Darvas"],
  profile: {
    "kind": "person",
    "mastery": [
      "Breakout",
      "Momentum",
      "Trend following"
    ],
    "summary": "Nicolas Darvas (1920-1977) was a Hungarian-born professional dancer and self-taught investor. He reported making over $2 million during the 1957-58 bull market. His box method buys stocks breaking above price 'boxes' near new highs on rising volume, with trailing stop-losses.",
    "known_for": [
      "Darvas Box",
      "How I Made $2,000,000 in the Stock Market"
    ],
    "sources": [
      "https://en.wikipedia.org/wiki/Nicolas_Darvas"
    ],
    "confidence": "high"
  },
  sourceReview: {
    "darvas2m": {"type":"book","people":["Nicolas Darvas"]},
    "dvycs": {"type":"book","people":["Nicolas Darvas"]},
  },
} satisfies AuthorFile;
