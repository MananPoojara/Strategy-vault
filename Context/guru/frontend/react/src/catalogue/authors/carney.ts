import type {AuthorFile} from '../author-files';

export default {
  order: 1340,
  author: { id: "carney", name: "Scott M. Carney", tagline: "Harmonic trading · Fibonacci-measured price patterns (Gartley, Bat, Crab, Butterfly) and the RSI BAMM", books: [
    { id: "smcht", sourceIds: ["title-6e05f4762b245a85", "title-64a46c5083c15151"], title: "The Harmonic Trader", year: 1999, blurb: "Carney, 1999 · AB=CD, Gartley 222, Butterfly, Three Drives; held: patterns picked on the chart" },
    { id: "smcv1", sourceIds: ["title-88f9c015c806d62e"], title: "Harmonic Trading, Volume One", year: 2010, blurb: "Carney, 2010 · pattern ratios and the trade-management system; held: swing points are chosen by eye" },
    { id: "smcv2", sourceIds: ["title-c40a0a5184e86423", "title-7b5b4bdbea2bc568"], title: "Harmonic Trading, Volume Two", year: 2010, blurb: "Carney, 2010 · new patterns, BAMM and RSI BAMM; held: needs judgement ('a degree of flexibility')" }] },
  identities: ["Scott M. Carney"],
  profile: {
    "kind": "person",
    "mastery": [
      "Harmonic price patterns",
      "Fibonacci ratios"
    ],
    "summary": "Scott M. Carney founded HarmonicTrader.com and named the Bat, Crab and Shark patterns. His books define each pattern by exact Fibonacci ratios between swing points, but the swing points themselves are picked on the chart and the trade management allows discretion, so no complete mechanical rule is given. This profile rests on the books themselves.",
    "known_for": [
      "The Harmonic Trader",
      "Harmonic Trading, Volumes One and Two"
    ],
    "sources": [],
    "confidence": "low"
  },
  sourceReview: {
    "smcht": {"type":"book","people":["Scott M. Carney"]},
    "smcv1": {"type":"book","people":["Scott M. Carney"]},
    "smcv2": {"type":"book","people":["Scott M. Carney"]},
  },
} satisfies AuthorFile;
