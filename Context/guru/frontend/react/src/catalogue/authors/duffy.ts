import type {AuthorFile} from '../author-files';

export default {
  order: 720,
  author: { id: "duffy", name: "Joseph T. Duffy", tagline: "Trading-championship finisher · Gann, Fibonacci and pivot confluence for turning points (held: techniques, not systems)", books: [
    { id: "jtdbet", sourceIds: ["source-6714799e3e56b338"], title: "Bull's Eye Trading", year: 2001, blurb: "Duffy, 2001 · pivots, pivot channel, 3x5ATR; held: 'techniques, not systems'" },
    { id: "jtdta", sourceIds: ["source-e1128f42baabdf39"], title: "The Trading Advantage", year: 1991, blurb: "Duffy, 1991 · Gann trend indicator, Fibonacci time, astronomy; held: judgement and forecasting" },
    { id: "jtdtpa", sourceIds: ["source-595fd2bac63f1e05"], title: "Turning Point Analysis in Price and Time", year: null, blurb: "Duffy · Square of Nine and multi-time-frame pivots; held: confluence by judgement" }] },
  identities: ["Joseph T. Duffy"],
  profile: {
    "kind": "person",
    "mastery": [
      "Gann analysis",
      "Fibonacci analysis"
    ],
    "summary": "Joseph T. Duffy is a former broker and proprietary trader who finished in the top ten of the U.S. Trading and Investing Championship three times. His books in the library combine Gann, Fibonacci, pivot and geometric techniques to locate turning points in price and time; he calls them techniques, not systems. This profile rests on the books themselves.",
    "known_for": [
      "The Trading Advantage",
      "Turning Point Analysis in Price and Time",
      "Bull's Eye Trading"
    ],
    "sources": [],
    "confidence": "low"
  },
  sourceReview: {
    "jtdbet": {"type":"book","people":["Joseph T. Duffy"]},
    "jtdta": {"type":"book","people":["Joseph T. Duffy"]},
    "jtdtpa": {"type":"book","people":["Joseph T. Duffy"]},
  },
} satisfies AuthorFile;
