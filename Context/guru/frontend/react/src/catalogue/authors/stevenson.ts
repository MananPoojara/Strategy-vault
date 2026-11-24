import type {AuthorFile} from '../author-files';

export default {
  order: 2743,
  author: { id: "stevenson", name: "Richard Stevenson", tagline: "Forex Impulse · 4-hour EMA cross with a daily stochastic filter, run here on index minute data as a declared adaptation", books: [
    { id: "fxim", sourceIds: [], readerPaths: ["Trading Knowledge I/Richard Stevenson - The Forex Impulse Trading System (theforexmagicmachine.com)/foreximpulse.pdf"], title: "The Forex Impulse Trading System (Richard Stevenson)", year: 2009, blurb: "Stevenson, 2009 · daily Slow Stochastic (13,5,5) %K above %D allows longs; on the 4-hour chart EMA4 then EMA13 cross above EMA50 to enter, stop 50 pips, exit when EMA4 crosses back through EMA13; runs on SPX / NIFTY 50 / NIFTY BANK 4-hour bars from 1-minute data (50 pips = 0.271 percent) as a declared adaptation, two GBP/USD examples only, unverified" }] },
  identities: ["Richard Stevenson"],
  profile: {
    "kind": "person",
    "mastery": ["Forex trend systems", "Moving average cross rules"],
    "summary": "Richard Stevenson published The Forex Impulse Trading System (2009), a short forex system guide: a daily stochastic filter and a 4-hour EMA 4/13/50 cross with a 50-pip stop. This profile rests on that guide (rules and both examples read in full).",
    "known_for": ["The Forex Impulse Trading System"],
    "sources": [],
    "confidence": "low"
  },
  sourceReview: {
    "fxim": {"type":"course_manual","people":["Richard Stevenson"]},
  },
} satisfies AuthorFile;
