import type {AuthorFile} from '../author-files';

export default {
  order: 280,
  author: { id: 'bwilliams', name: 'Bill Williams', tagline: 'Profitunity · Alligator, fractals, Awesome and Accelerator oscillators', books: [{ id: 'bwntd', title: 'New Trading Dimensions', year: 1998, blurb: 'Williams, 1998 · fractal breakout with the Alligator, AO/AC/zone add-ons, AO system' }, { id: 'bwtc1', title: 'Trading Chaos (1st ed.)', year: 1995, blurb: 'Williams, 1995 · fractal stop-and-reverse, 5/13/34 long/short/out' }, { id: 'bwtc2', title: 'Trading Chaos (2nd ed.)', year: 2004, blurb: 'Bill & Justine Williams, 2004 · fractal and Super AO entries with the trailing stop' }] },
  identities: ["Bill Williams"],
  profile: {
    "kind": "person",
    "mastery": [
      "Indicators",
      "Trend following",
      "Trading psychology"
    ],
    "summary": "Bill M. Williams (1932-2019) was an American trader and author who applied ideas from chaos theory to stocks, commodities and forex. He developed widely used indicators that identify trends and momentum shifts, and he also wrote about trading psychology.",
    "known_for": [
      "Alligator",
      "Awesome Oscillator and Accelerator/Decelerator",
      "Fractals",
      "Trading Chaos"
    ],
    "sources": [
      "https://en.wikipedia.org/wiki/Bill_Williams_(trader)"
    ],
    "confidence": "high"
  },
  sourceReview: {
    "bwntd": {"type":"book","people":["Bill Williams"]},
    "bwtc1": {"type":"book","people":["Bill Williams"]},
    "bwtc2": {"type":"book","sameWorkAs":"bwtc1","people":["Bill Williams","Justine Williams"]},
  },
} satisfies AuthorFile;
