import type {AuthorFile} from '../author-files';

export default {
  order: 1400,
  author: { id: "murphy", name: "John J. Murphy", tagline: "Author of the standard technical-analysis textbook and of intermarket analysis · the rules he states are other traders' systems already in the vault", books: [
    { id: "jjmtafm", sourceIds: ["title-914d25db99b52ecb", "title-856fb06d7e5732f2", "title-977f3288cea7ffaa", "title-03f1bb61d63b1719"], title: "Technical Analysis of the Financial Markets", year: 1999, blurb: "Murphy, 1999 · Donchian 4-week rule, 4-9-18 and double crossovers (run as Kaufman's kftsm rules); oscillator signals described without exits" },
    { id: "jjmita", sourceIds: ["title-692f33c949c4235a", "title-7779e08101a2372f", "title-c7ff6d3530b5a55b"], title: "Intermarket Technical Analysis", year: 1991, blurb: "Murphy, 1991 · commodities, bonds, dollar and stocks; needs CRB/bond/dollar data, chart reading" },
    { id: "jjmvi", sourceIds: ["title-687035ca0b822f2f"], title: "The Visual Investor", year: 2009, blurb: "Murphy, 2nd ed. 2009 · the textbook signals on ETF charts" }] },
  identities: ["John J. Murphy"],
  profile: {
    "kind": "person",
    "mastery": [
      "Technical analysis",
      "Intermarket analysis"
    ],
    "summary": "John J. Murphy wrote Technical Analysis of the Financial Markets, the standard textbook of the field, and pioneered intermarket analysis. His books describe the common indicators and other traders' systems (Donchian's 4-week rule, the 4-9-18 averages) rather than systems of his own, so his complete rules run under the vault's existing strategies. This profile rests on the books themselves.",
    "known_for": [
      "Technical Analysis of the Financial Markets",
      "Intermarket Technical Analysis",
      "The Visual Investor"
    ],
    "sources": [],
    "confidence": "low"
  },
  sourceReview: {
    "jjmtafm": {"type":"book","people":["John J. Murphy"]},
    "jjmita": {"type":"book","people":["John J. Murphy"]},
    "jjmvi": {"type":"book","people":["John J. Murphy"]},
  },
} satisfies AuthorFile;
