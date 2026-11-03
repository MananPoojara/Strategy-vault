import type {AuthorFile} from '../author-files';

export default {
  order: 1000,
  author: { id: "schutzman", name: "Fred G. Schutzman", tagline: "CMT, system developer and money manager · two printed TradeStation systems run here", books: [
    { id: "fsdps", sourceIds: [], title: "Developing Profitable Trading Systems: from Discretionary to Mechanical", year: 1997, blurb: "Schutzman (TAG seminar workbook) · Zweig-style reversal on a 2% rise from the lowest close or a 6% fall from the highest close; MACD rising with a close under the 5-day Bollinger band buys, MACD falling with a close over the upper band sells (stop and reverse). The counter-trend stochastic system is held (TradeStation's SlowD smoothing not printed)" }] },
  identities: ["Fred G. Schutzman"],
  profile: {
    "kind": "person",
    "mastery": [
      "System development",
      "EasyLanguage"
    ],
    "summary": "Fred G. Schutzman, CMT, presented Developing Profitable Trading Systems: from Discretionary to Mechanical at the 1997 TAG seminar; the workbook prints three TradeStation EasyLanguage systems (trend-following, counter-trend and combination) with their test summaries on futures. His EasyLanguage also appears in other vault sources (Elder, LeBeau). This profile rests on the workbook itself.",
    "known_for": [
      "Developing Profitable Trading Systems (TAG 1997)"
    ],
    "sources": [],
    "confidence": "low"
  },
  sourceReview: {
    "fsdps": {"type":"book","people":["Fred G. Schutzman"]},
  },
} satisfies AuthorFile;
