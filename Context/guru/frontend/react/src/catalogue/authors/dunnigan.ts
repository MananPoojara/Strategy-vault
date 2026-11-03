import type {AuthorFile} from '../author-files';

export default {
  order: 250,
  author: { id: 'dunnigan', name: 'William Dunnigan', tagline: '1950s mechanical methods · One-Way Formula, Dow Barometer, Thrust', books: [{ id: 'wdowf', title: 'One-Way Formula', year: 1957, blurb: 'Dunnigan, 1957 · weekly stock procedure with the square-root Operating Plan' }, { id: 'wdnbp', title: 'New Blueprints for Gains in Stocks and Grains', year: 1956, blurb: 'Dunnigan, 1956 · the New Dow Barometer (Rules 1+2 and Rule 2 alone)' }, { id: 'wdgig', title: 'Gains in Grains', year: 1954, blurb: 'Dunnigan, 1954 · 2½%-3-day swings with 1% stop-and-reverse; grain rules need futures' }] },
  identities: ["William Dunnigan"],
  profile: {
    "kind": "person",
    "mastery": [
      "Trend following",
      "Breakout"
    ],
    "summary": "William Dunnigan was a mid-20th-century American market writer whose 1950s booklets set out mechanical swing-based rules for stocks and grain futures. His One-Way Formula (1957) aimed to be a single 'universal' method that applies the same rules to any actively traded stock or commodity, trading only in the direction of the main trend.",
    "known_for": [
      "One-Way Formula",
      "New Blueprints for Gains in Stocks and Grains",
      "Gains in Grains"
    ],
    "sources": [
      "https://www.panmacmillan.com/authors/william-dunnigan/new-blueprints-for-gains-in-stocks-and-grains-one-way-formula-for-trading-in-stocks-commodities/9781897597576",
      "https://www.alanpuritrading.com/product/one-way-formula-trading-stocks-commodities-1957-william-dunnigan/"
    ],
    "confidence": "medium"
  },
  sourceReview: {
    "wdowf": {"type":"other","people":["William Dunnigan"]},
    "wdnbp": {"type":"book","people":["William Dunnigan"]},
    "wdgig": {"type":"other","sameWorkAs":"wdnbp","people":["William Dunnigan"]},
  },
} satisfies AuthorFile;
