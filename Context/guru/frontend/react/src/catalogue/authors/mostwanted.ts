import type {AuthorFile} from '../author-files';

export default {
  order: 2702,
  author: { id: "mostwanted", name: "Most Wanted Forex Robot", tagline: "MetaTrader forex systems · the Profit Maximiser runs here on index daily bars as a declared adaptation", books: [
    { id: "mwpm", sourceIds: [], readerPaths: ["Trading Knowledge III/Profit Maximiser System (mostwantedforexrobot.com)/Profit Maximiser System.pdf"], title: "Profit Maximiser System (Most Wanted Forex Robot)", year: 2008, blurb: "Most Wanted Forex Robot · close beyond the SMA 21 with a green or red Awesome Oscillator, enter the next candle, exit on the close back through the SMA 21, stop 50 pips beyond the signal candle; runs on NIFTY 50 / NIFTY BANK daily bars (pips scaled to a percentage) as a declared adaptation, no results printed, unverified" }] },
  identities: ["Most Wanted Forex Robot"],
  profile: {
    "kind": "collection",
    "mastery": ["MetaTrader forex systems", "Moving average and Awesome Oscillator rules"],
    "summary": "Most Wanted Forex Robot published short MetaTrader system guides for forex. This profile rests on the Profit Maximiser guide (read in full); only that system is coded here, on index daily bars.",
    "known_for": ["Profit Maximiser System"],
    "sources": [],
    "confidence": "low"
  },
  sourceReview: {
    "mwpm": {"type":"course_manual","people":["Most Wanted Forex Robot"]},
  },
} satisfies AuthorFile;
