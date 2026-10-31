import type {AuthorFile} from '../author-files';

export default {
  order: 1260,
  author: { id: "arms", name: "Richard W. Arms Jr.", tagline: "Inventor of Equivolume, Ease of Movement and the Arms Index · the two EMV rules run here", books: [
    { id: "ravcs", sourceIds: ["title-bdb6dd066565a040"], title: "Volume Cycles in the Stock Market", year: 1994, blurb: "Arms · 13-day Ease of Movement zero cross and the weekly four-week reversal of the 13 run; shorter EMVs and cycle reading held" },
    { id: "rasmm", sourceIds: ["title-97aa0acf12e5e0cb"], title: "Stop and Make Money", year: 2008, blurb: "Arms, 2008 · power boxes, EMV trend lines, volume-adjusted averages and Arms Index extremes; held: chart judgement" },
    { id: "rateq", sourceIds: ["title-717397b8dab6f36d", "title-99621cc8c1953233"], title: "Trading with Equivolume", year: 2001, blurb: "Arms, 2001 · Equivolume chart patterns; held" }] },
  identities: ["Richard W. Arms Jr."],
  profile: {
    "kind": "person",
    "mastery": [
      "Volume analysis",
      "Equivolume charting",
      "Market breadth (Arms Index)"
    ],
    "summary": "Richard W. Arms Jr. created Equivolume charting, the Ease of Movement indicator and the Arms Index (TRIN). Volume Cycles in the Stock Market gives two complete Ease of Movement rules - the 13-day average crossing zero and the weekly 'four-week reversal of the 13' - which run here on indices, ETFs and stocks with volume as a declared application; his chart-pattern and Arms Index methods are judgement. This profile rests on the books themselves.",
    "known_for": [
      "Volume Cycles in the Stock Market",
      "Stop and Make Money",
      "The Arms Index"
    ],
    "sources": [],
    "confidence": "low"
  },
  sourceReview: {
    "ravcs": {"type":"book","people":["Richard W. Arms Jr."]},
    "rasmm": {"type":"book","people":["Richard W. Arms Jr."]},
    "rateq": {"type":"book","people":["Richard W. Arms Jr."]},
  },
} satisfies AuthorFile;
