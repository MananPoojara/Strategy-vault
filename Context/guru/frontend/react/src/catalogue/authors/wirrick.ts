import type {AuthorFile} from '../author-files';

export default {
  order: 730,
  author: { id: "wirrick", name: "Steve Wirrick", tagline: "Options and swing-trading educator · the two-day pull-back swing set-up above the 50-day average (backtested); option plays held", books: [
    { id: "swmfst", sourceIds: ["title-ce524e2efcd0c7e6"], title: "Make a Fortune Swing Trading!", year: 2003, blurb: "Wirrick, 2003 · swing trade set-up: two-day pull-back above the 50-day average, break of the prior high, 20-day trailing exit" },
    { id: "swhoi", sourceIds: ["title-ed1441777051b0e2"], title: "High Octane Investing", year: null, blurb: "Wirrick, Collette & Christensen · option plays and a Bollinger squeeze; held: option prices, no squeeze threshold" },
    { id: "swhot", sourceIds: ["title-f0c83eda5e62b954"], title: "High Octane Trading", year: 2003, blurb: "Wirrick, 2003 · options primer and chart reading; held: no complete rule" }] },
  identities: ["Steve Wirrick"],
  profile: {
    "kind": "person",
    "mastery": [
      "Swing trading",
      "Stock options"
    ],
    "summary": "Steve Wirrick ran the My Trading Diary newsletter and taught swing trading of stocks with options. His books in the library combine trend reading with moving averages and a two-day pull-back swing set-up, traded through calls and puts. This profile rests on the books themselves.",
    "known_for": [
      "Make a Fortune Swing Trading!",
      "High Octane Trading"
    ],
    "sources": [],
    "confidence": "low"
  },
  sourceReview: {
    "swmfst": {"type":"book","people":["Steve Wirrick"]},
    "swhoi": {"type":"book","people":["Steve Wirrick"]},
    "swhot": {"type":"book","people":["Steve Wirrick"]},
  },
} satisfies AuthorFile;
