import type {AuthorFile} from '../author-files';

export default {
  order: 1360,
  author: { id: "mdarcher", name: "Michael Duane Archer", tagline: "Forex trading · the Goodman Swing Count System, codex method and reversal charts", books: [
    { id: "mdagsc", sourceIds: ["title-d84072c1830c9e4d", "title-50d8b2eaa6ec4acc"], title: "Getting Started in Currency Trading", year: 2010, blurb: "Archer (earlier edition with Bickford) · toolbox: Goodman swings, Nofri congestion, Pugh formations; held: no settings or exits" },
    { id: "mdafts", sourceIds: ["title-fd672d7519f339c9"], title: "Getting Started in Forex Trading Strategies", year: 2007, blurb: "Archer, 2007 · codex method with Goodman swing entries; held: set-ups chosen by eye" },
    { id: "mdafcc", sourceIds: ["title-5e4b16762691197e"], title: "The Forex Chartist Companion", year: 2007, blurb: "Archer & Bickford, 2007 · P&F, swing, kagi and line-break charts; held: pip box sizes, no exits" }] },
  identities: ["Michael Duane Archer"],
  profile: {
    "kind": "person",
    "mastery": [
      "Forex trading",
      "Goodman Swing Count System"
    ],
    "summary": "Michael Duane Archer is a commodity and forex trader who learned the Goodman Swing Count System from Charles B. Goodman and teaches it in his forex books. The set-ups are mapped by eye and the toolbox methods lack settings or exits, so no complete mechanical rule is given. This profile rests on the books themselves.",
    "known_for": [
      "Getting Started in Currency Trading",
      "The Forex Chartist Companion"
    ],
    "sources": [],
    "confidence": "low"
  },
  sourceReview: {
    "mdagsc": {"type":"book","people":["Michael Duane Archer"]},
    "mdafts": {"type":"book","people":["Michael Duane Archer"]},
    "mdafcc": {"type":"book","people":["Michael Duane Archer"]},
  },
} satisfies AuthorFile;
