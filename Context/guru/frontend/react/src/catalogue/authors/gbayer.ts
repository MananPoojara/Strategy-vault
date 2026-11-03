import type {AuthorFile} from '../author-files';

export default {
  order: 1370,
  author: { id: "gbayer", name: "George Bayer", tagline: "Astrological market timing · planetary rules for wheat, hides and stocks", books: [
    { id: "gbywol", sourceIds: ["title-5353892c4d912829", "title-78c52b8279f8a517"], title: "George Wollsten, Expert Stock and Grain Trader", year: null, blurb: "Bayer · a novel teaching planetary timing; held: no defined trades" },
    { id: "gbygn", sourceIds: ["title-a80c67d2ea166ca3"], title: "Gold Nuggets for Stock and Commodity Traders", year: null, blurb: "Bayer · essay; no trading rule" },
    { id: "gbytd", sourceIds: ["title-583c1f5a443d98a1"], title: "Stock and Commodity Traders' Hand-Book of Trend Determination", year: null, blurb: "Bayer · 59 planetary rules; held: needs an ephemeris, no entry or exit" },
    { id: "gbytf", sourceIds: ["title-f64006520fcb6e51"], title: "Time Factors in the Stock Market", year: 1937, blurb: "Bayer, 1937 · planetary theory; no trading rule" }] },
  identities: ["George Bayer"],
  profile: {
    "kind": "person",
    "mastery": [
      "Astrological market timing"
    ],
    "summary": "George Bayer was a 1930s-1950s grain and stock trader who timed markets by planetary positions. His books state rules as planetary configurations that mark trend changes, without entries, stops or exits, so nothing is tested. This profile rests on the books themselves.",
    "known_for": [
      "Stock and Commodity Traders' Hand-Book of Trend Determination",
      "Time Factors in the Stock Market"
    ],
    "sources": [],
    "confidence": "low"
  },
  sourceReview: {
    "gbywol": {"type":"book","people":["George Bayer"]},
    "gbygn": {"type":"book","people":["George Bayer"]},
    "gbytd": {"type":"book","people":["George Bayer"]},
    "gbytf": {"type":"book","people":["George Bayer"]},
  },
} satisfies AuthorFile;
