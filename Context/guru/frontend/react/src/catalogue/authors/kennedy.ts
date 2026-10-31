import type {AuthorFile} from '../author-files';

export default {
  order: 700,
  author: { id: "kennedy", name: "Jeffrey Kennedy", tagline: "Elliott Wave International analyst · wave-count trades, MACD hooks and bar set-ups offered as alerts; no complete rule (held)", books: [
    { id: "jfkbotc", sourceIds: ["title-45784acb4b404ed6"], title: "The Best of Trader's Classroom", year: null, blurb: "Kennedy, EWI · Elliott wave lessons; held: wave counts by hand" },
    { id: "jfktcc", sourceIds: ["title-2b97623dd448a721", "title-470e0981aec6d750", "title-d3800718e33a5c10", "title-f9eaf33fa949e888"], title: "Trader's Classroom Collection (Vol.1-4)", year: null, blurb: "Kennedy, EWI · 10-day reversal, MACD hooks, three-in-one bar; held: alerts without exits, judged signals" },
    { id: "jfkttl", sourceIds: ["title-afb3f8d044fea3fa"], title: "Trading the Line", year: 2009, blurb: "Kennedy, EWI 2009 · trendline set-ups; held: lines drawn by the trader" }] },
  identities: ["Jeffrey Kennedy"],
  profile: {
    "kind": "person",
    "mastery": [
      "Elliott wave analysis",
      "Technical analysis"
    ],
    "summary": "Jeffrey Kennedy is an analyst at Elliott Wave International and edited its futures letters. His e-books in the library teach Elliott wave counting, trendlines and a few bar and MACD set-ups, which he presents as alerts to be combined with a wave count rather than as mechanical systems. This profile rests on the books themselves.",
    "known_for": [
      "Trader's Classroom",
      "Trading the Line",
      "Elliott Wave Junctures"
    ],
    "sources": [],
    "confidence": "low"
  },
  sourceReview: {
    "jfkbotc": {"type":"book","people":["Jeffrey Kennedy"]},
    "jfktcc": {"type":"book","people":["Jeffrey Kennedy"]},
    "jfkttl": {"type":"book","people":["Jeffrey Kennedy"]},
  },
} satisfies AuthorFile;
