import type {AuthorFile} from '../author-files';

export default {
  order: 780,
  author: { id: "lien", name: "Kathy Lien", tagline: "Currency strategist · FX day- and swing-trading setups; the CCI trade and the moving-average perfect order run here", books: [
    { id: "klhpt", sourceIds: ["title-366bcc25c9890ea7", "title-db8a0baccba990b1", "title-0cf286a11dbd5ddf"], title: "High Probability Trading Setups for the Currency Market (with Boris Schlossberg)", year: 2006, blurb: "Lien & Schlossberg, 2006 · the 'Do the Right Thing' CCI trade runs on daily bars; the other setups use pips, intraday charts or hand-picked swings" },
    { id: "kldst", sourceIds: ["title-27677f9123fd300f"], title: "Day Trading and Swing Trading the Currency Market", year: 2008, blurb: "Lien, 2008 · moving-average perfect order runs; pip-based, news and carry strategies held" },
    { id: "kldtc", sourceIds: ["title-21d3c35d6c2c02be", "title-39a12d5916d733f7"], title: "Day Trading the Currency Market", year: 2005, blurb: "Lien, 2005 · first edition of the same strategies (perfect order counted under the 2008 edition)" }] },
  identities: ["Kathy Lien"],
  profile: {
    "kind": "person",
    "mastery": [
      "Currency market strategy",
      "FX technical and fundamental setups"
    ],
    "summary": "Kathy Lien was chief currency strategist at FXCM and co-founded BK Asset Management with Boris Schlossberg. Her books give FX setups; most use pip-sized stops or intraday charts, but the 'Do the Right Thing' CCI trade and the moving-average perfect order are complete daily rules and run here on indices, ETFs and stocks as a declared application. This profile rests on the books themselves.",
    "known_for": [
      "Day Trading and Swing Trading the Currency Market",
      "High Probability Trading Setups for the Currency Market"
    ],
    "sources": [],
    "confidence": "low"
  },
  sourceReview: {
    "klhpt": {"type":"book","people":["Kathy Lien","Boris Schlossberg"]},
    "kldst": {"type":"book","people":["Kathy Lien"]},
    "kldtc": {"type":"book","people":["Kathy Lien"]},
  },
} satisfies AuthorFile;
