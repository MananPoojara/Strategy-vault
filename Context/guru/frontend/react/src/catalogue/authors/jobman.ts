import type {AuthorFile} from '../author-files';

export default {
  order: 660,
  author: { id: "jobman", name: "Darrell R. Jobman", tagline: "Former Futures magazine editor · anthology of technical-analysis articles; Dennis Dunn's seven-week moving average", books: [
    { id: "djhta", sourceIds: ["title-ed296147ee941ead"], title: "The Handbook of Technical Analysis", year: 1995, blurb: "Jobman (ed.), 1995 · Dunn's seven-week Thursday moving average tested; indicator, chart, sentiment and cycle chapters have no complete rules" }] },
  identities: ["Darrell R. Jobman"],
  profile: {
    "kind": "person",
    "mastery": [
      "Futures",
      "Technical analysis",
      "Financial journalism"
    ],
    "summary": "Darrell R. Jobman was editor-in-chief of Futures magazine and edited handbooks on technical analysis and alternative investments. The Handbook of Technical Analysis collects magazine articles by Wilder, Lane, Elder, Bill Williams, Bernstein and others; only one complete rule (Dennis Dunn's seven-week moving average) is stated. The Handbook of Alternative Investments covers hedge funds, managed futures and private equity without trading rules. This profile rests on the books themselves.",
    "known_for": [
      "The Handbook of Technical Analysis",
      "The Handbook of Alternative Investments"
    ],
    "sources": [],
    "confidence": "low"
  },
  sourceReview: {
    "djhta": {"type":"book","people":["Darrell R. Jobman"]},
  },
} satisfies AuthorFile;
