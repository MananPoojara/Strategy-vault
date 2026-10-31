import type {AuthorFile} from '../author-files';

export default {
  order: 650,
  author: { id: "kroll", name: "Stanley Kroll", tagline: "Long-term futures trend trader · 50-day trend filter with moving-average entries and stop-outs", books: [
    { id: "skdab", sourceIds: ["title-1a375bb2a1b292dd"], title: "Dragons and Bulls", year: 1994, blurb: "Kroll, 1994 · 50-day trend, 10/20/50-day entries, 4/9/18-day stop-outs and re-entries; the 9/18/50 NIKKEI example is held (exit not exact)" },
    { id: "skfts", sourceIds: ["title-b57e56c915e14bbf"], title: "Kroll on Futures Trading Strategy", year: 1988, blurb: "Kroll, 1988 · strategy and money management; its moving-average examples are already in the vault" }] },
  identities: ["Stanley Kroll"],
  profile: {
    "kind": "person",
    "mastery": [
      "Long-term futures trend trading",
      "Moving-average trend strategies"
    ],
    "summary": "Stanley Kroll traded futures on Wall Street from 1960 and wrote several books on futures strategy. Dragons and Bulls gives a complete moving-average trend strategy (50-day trend filter, 10/20/50-day entries, 4/9/18-day stop-outs and re-entries), which is run here; his other chapters are money management and trading discipline. He co-wrote The New Technical Trader with Tushar Chande. This profile rests on the books themselves.",
    "known_for": [
      "Dragons and Bulls",
      "Kroll on Futures Trading Strategy",
      "The New Technical Trader (with Tushar Chande)"
    ],
    "sources": [],
    "confidence": "low"
  },
  sourceReview: {
    "skdab": {"type":"book","people":["Stanley Kroll"]},
    "skfts": {"type":"book","people":["Stanley Kroll"]},
  },
} satisfies AuthorFile;
